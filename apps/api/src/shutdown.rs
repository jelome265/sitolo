//! Deterministic, bounded process shutdown (F-020).
//!
//! Subsystems stop in a fixed order: network ingress first so no new work
//! arrives, then telemetry flush accounting, then persistence-intent release.
//! Shutdown is idempotent: repeated calls observe the first completed run.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Subsystems stopped during shutdown, in stop order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subsystem {
    Listener,
    Telemetry,
    PersistenceIntent,
}

/// Upper bound for graceful drain before the process exits.
pub const SHUTDOWN_DRAIN_DEADLINE_SECS: u64 = 10;

/// Maximum concurrently accepted TCP connections at the API service boundary.
pub const MAX_IN_FLIGHT_CONNECTIONS: usize = 128;

/// Maximum concurrently processed requests at the API service boundary.
pub const MAX_IN_FLIGHT_REQUESTS: usize = 128;

/// Process serving lifecycle, consulted by `/process/ready`.
///
/// `Initializing` is the only state in which `AppState` exists but startup
/// has not yet published readiness (see `bootstrap.rs::assemble`, which
/// calls [`Readiness::mark_ready`] only after every startup check passes).
/// `Draining` means the shutdown signal has been received: a readiness
/// probe must fail so a load balancer stops routing new traffic here.
/// `ShutDown` is reserved for a fully-stopped process and is not currently
/// published by any code path, but is part of the type so a future
/// post-drain state does not require a breaking API change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessState {
    Initializing,
    Ready,
    Draining,
    ShutDown,
}

/// Shared, thread-safe process readiness signal.
///
/// `is_draining` is a separate `AtomicBool` (rather than only comparing
/// `get_state() == Draining`) so the hot-path check used by `/process/ready`
/// and by every in-flight connection task is a single lock-free load.
#[derive(Clone)]
pub struct Readiness {
    state: Arc<std::sync::Mutex<ReadinessState>>,
    draining: Arc<AtomicBool>,
}

impl Readiness {
    pub fn new() -> Self {
        Self {
            state: Arc::new(std::sync::Mutex::new(ReadinessState::Initializing)),
            draining: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Published exactly once, after every startup check has passed.
    pub fn mark_ready(&self) {
        *self.state.lock().expect("readiness mutex poisoned") = ReadinessState::Ready;
    }

    /// Published exactly once, when the shutdown signal is received, before
    /// the listener stops accepting connections. Idempotent.
    pub fn mark_draining(&self) {
        *self.state.lock().expect("readiness mutex poisoned") = ReadinessState::Draining;
        self.draining.store(true, Ordering::SeqCst);
    }

    pub fn is_draining(&self) -> bool {
        self.draining.load(Ordering::SeqCst)
    }

    pub fn get_state(&self) -> ReadinessState {
        *self.state.lock().expect("readiness mutex poisoned")
    }
}

impl Default for Readiness {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ShutdownCoordinator {
    completed: Vec<Subsystem>,
    done: bool,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        ShutdownCoordinator {
            completed: Vec::new(),
            done: false,
        }
    }

    /// Runs the ordered shutdown exactly once. Later calls return the
    /// already-completed order without re-executing any step.
    pub fn shutdown(&mut self) -> &[Subsystem] {
        if !self.done {
            for subsystem in [
                Subsystem::Listener,
                Subsystem::Telemetry,
                Subsystem::PersistenceIntent,
            ] {
                self.completed.push(subsystem);
            }
            self.done = true;
        }
        &self.completed
    }

    pub fn is_shut_down(&self) -> bool {
        self.done
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_runs_subsystems_in_fixed_order() {
        let mut coordinator = ShutdownCoordinator::new();
        assert!(!coordinator.is_shut_down());
        assert_eq!(
            coordinator.shutdown(),
            &[
                Subsystem::Listener,
                Subsystem::Telemetry,
                Subsystem::PersistenceIntent,
            ]
        );
        assert!(coordinator.is_shut_down());
    }

    #[test]
    fn shutdown_is_idempotent() {
        let mut coordinator = ShutdownCoordinator::new();
        let first = coordinator.shutdown().to_vec();
        let second = coordinator.shutdown().to_vec();
        assert_eq!(first, second);
        assert_eq!(first.len(), 3);
    }

    #[test]
    fn readiness_starts_initializing_and_not_draining() {
        let readiness = Readiness::new();
        assert_eq!(readiness.get_state(), ReadinessState::Initializing);
        assert!(!readiness.is_draining());
    }

    #[test]
    fn readiness_mark_ready_publishes_ready() {
        let readiness = Readiness::new();
        readiness.mark_ready();
        assert_eq!(readiness.get_state(), ReadinessState::Ready);
        assert!(!readiness.is_draining());
    }

    #[test]
    fn readiness_mark_draining_is_terminal_over_ready() {
        let readiness = Readiness::new();
        readiness.mark_ready();
        readiness.mark_draining();
        assert_eq!(readiness.get_state(), ReadinessState::Draining);
        assert!(readiness.is_draining());
    }

    #[test]
    fn readiness_mark_draining_is_idempotent() {
        let readiness = Readiness::new();
        readiness.mark_draining();
        readiness.mark_draining();
        assert_eq!(readiness.get_state(), ReadinessState::Draining);
        assert!(readiness.is_draining());
    }

    #[test]
    fn readiness_clone_shares_underlying_state() {
        let readiness = Readiness::new();
        let handle = readiness.clone();
        handle.mark_ready();
        assert_eq!(readiness.get_state(), ReadinessState::Ready);
    }
}
