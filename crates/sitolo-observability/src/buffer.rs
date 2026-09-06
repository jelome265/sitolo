//! Bounded telemetry queue with explicit shedding states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    P0Security,
    P1Business,
    P2Normal,
    P3Debug,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryState {
    Healthy,
    Degraded,
    Pressured,
    Dropping,
    Blackout,
}
#[derive(Debug)]
pub struct TelemetryBuffer {
    capacity: usize,
    queued: usize,
    dropped: [u64; 4],
}
impl TelemetryBuffer {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            queued: 0,
            dropped: [0; 4],
        }
    }
    pub fn state(&self) -> TelemetryState {
        let p = self.queued * 100 / self.capacity;
        match p {
            0..=49 => TelemetryState::Healthy,
            50..=74 => TelemetryState::Degraded,
            75..=89 => TelemetryState::Pressured,
            90..=99 => TelemetryState::Dropping,
            _ => TelemetryState::Blackout,
        }
    }
    pub fn push(&mut self, p: Priority) -> bool {
        if self.queued < self.capacity {
            self.queued += 1;
            true
        } else {
            self.dropped[p as usize] += 1;
            false
        }
    }
    pub fn export_one(&mut self) {
        self.queued = self.queued.saturating_sub(1)
    }
    pub fn dropped(&self, p: Priority) -> u64 {
        self.dropped[p as usize]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_queue_drops_when_full() {
        let mut q = TelemetryBuffer::new(2);
        assert!(q.push(Priority::P2Normal));
        assert!(q.push(Priority::P2Normal));
        assert!(!q.push(Priority::P3Debug));
        assert_eq!(q.state(), TelemetryState::Blackout);
        assert_eq!(q.dropped(Priority::P3Debug), 1);
    }
}
