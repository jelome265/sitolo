//! Deterministic, bounded process shutdown (F-020).
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subsystem { Listener, Telemetry, PersistenceIntent }

pub const SHUTDOWN_DRAIN_DEADLINE_SECS: u64 = 10;
pub const MAX_IN_FLIGHT_CONNECTIONS: usize = 128;
pub const MAX_IN_FLIGHT_REQUESTS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessState { Initializing, Ready, Draining, ShutDown }

#[derive(Clone)]
pub struct Readiness {
    state: Arc<std::sync::Mutex<ReadinessState>>,
    draining: Arc<AtomicBool>,
}

impl Readiness {
    pub fn new() -> Self {
        Self { state: Arc::new(std::sync::Mutex::new(ReadinessState::Initializing)), draining: Arc::new(AtomicBool::new(false)) }
    }
    pub fn mark_ready(&self) { *self.state.lock().unwrap() = ReadinessState::Ready; }
    pub fn mark_draining(&self) {
        *self.state.lock().unwrap() = ReadinessState::Draining;
        self.draining.store(true, Ordering::SeqCst);
    }
    pub fn is_draining(&self) -> bool { self.draining.load(Ordering::SeqCst) }
    pub fn get_state(&self) -> ReadinessState { *self.state.lock().unwrap() }
}

impl Default for Readiness { fn default() -> Self { Self::new() } }

pub struct ShutdownCoordinator { completed: Vec<Subsystem>, done: bool }
impl ShutdownCoordinator {
    pub fn new() -> Self { Self { completed: Vec::new(), done: false } }
    pub fn shutdown(&mut self) -> &[Subsystem] {
        if !self.done {
            for subsystem in [Subsystem::Listener, Subsystem::Telemetry, Subsystem::PersistenceIntent] { self.completed.push(subsystem); }
            self.done = true;
        }
        &self.completed
    }
    pub fn is_shut_down(&self) -> bool { self.done }
}
impl Default for ShutdownCoordinator { fn default() -> Self { Self::new() } }
