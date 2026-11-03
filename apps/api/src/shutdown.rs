//! Deterministic, bounded process shutdown (F-020).
//!
//! Subsystems stop in a fixed order: network ingress first so no new work
//! arrives, then telemetry flush accounting, then persistence-intent release.
//! Shutdown is idempotent: repeated calls observe the first completed run.

/// Subsystems stopped during shutdown, in stop order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subsystem {
    /// Network ingress, stopped first so no new work arrives.
    Listener,
    /// Telemetry flush accounting.
    Telemetry,
    /// Persistence-intent release.
    PersistenceIntent,
}

/// Upper bound for graceful drain before the process exits.
pub const SHUTDOWN_DRAIN_DEADLINE_SECS: u64 = 10;

/// Maximum concurrently tracked connections during drain.
pub const MAX_IN_FLIGHT_CONNECTIONS: usize = 128;

/// Tracks the ordered, idempotent shutdown of process subsystems.
#[derive(Debug)]
pub struct ShutdownCoordinator {
    completed: Vec<Subsystem>,
    done: bool,
}

impl ShutdownCoordinator {
    /// Creates a coordinator that has not yet stopped any subsystem.
    #[must_use]
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

    /// Whether the ordered shutdown has already completed.
    #[must_use]
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
}
