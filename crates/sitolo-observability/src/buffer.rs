//! Bounded telemetry admission with priority-aware shedding (F-010).
//!
//! This is an admission and shedding *model*, not a record store: it tracks
//! per-priority occupancy so saturation decisions are executable without
//! pretending payloads are retained here. The export integration owns
//! record payloads, ordering, retry, and drain; this model tells it what
//! admission and shedding decisions to apply.
//!
//! Shedding order: when full, an incoming record displaces the least
//! important queued class (`P3` first). A `P0` security record is therefore
//! never dropped for debug noise; it is dropped only when the buffer holds
//! nothing but `P0` records. Export drains the most important class first.

/// Telemetry importance. Declaration order is significance order: `P0` is
/// most important. Do not reorder without reviewing shedding semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    P0Security,
    P1Business,
    P2Normal,
    P3Debug,
}

impl Priority {
    fn index(self) -> usize {
        self as usize
    }
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
    queued: [u64; 4],
    dropped: [u64; 4],
}

impl TelemetryBuffer {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            queued: [0; 4],
            dropped: [0; 4],
        }
    }

    fn total_queued(&self) -> u64 {
        self.queued.iter().sum()
    }

    pub fn state(&self) -> TelemetryState {
        let percent = self.total_queued() * 100 / self.capacity as u64;
        match percent {
            0..=49 => TelemetryState::Healthy,
            50..=74 => TelemetryState::Degraded,
            75..=89 => TelemetryState::Pressured,
            90..=99 => TelemetryState::Dropping,
            _ => TelemetryState::Blackout,
        }
    }

    /// Admits a record of priority `p`. When full, evicts one record from
    /// the least important queued class if `p` outranks it; otherwise the
    /// incoming record is dropped and counted.
    pub fn push(&mut self, p: Priority) -> bool {
        if self.total_queued() < self.capacity as u64 {
            self.queued[p.index()] += 1;
            return true;
        }
        // Least important (highest discriminant) non-empty class first.
        for victim in [Priority::P3Debug, Priority::P2Normal, Priority::P1Business] {
            if victim > p && self.queued[victim.index()] > 0 {
                self.queued[victim.index()] -= 1;
                self.dropped[victim.index()] += 1;
                self.queued[p.index()] += 1;
                return true;
            }
        }
        self.dropped[p.index()] += 1;
        false
    }

    /// Exports one record, most important class first. Queue accounting
    /// stays exact: one export removes exactly one queued record.
    pub fn export_one(&mut self) {
        for class in [
            Priority::P0Security,
            Priority::P1Business,
            Priority::P2Normal,
            Priority::P3Debug,
        ] {
            if self.queued[class.index()] > 0 {
                self.queued[class.index()] -= 1;
                return;
            }
        }
    }

    pub fn queued(&self, p: Priority) -> u64 {
        self.queued[p.index()]
    }

    pub fn dropped(&self, p: Priority) -> u64 {
        self.dropped[p.index()]
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

    #[test]
    fn saturation_matrix_is_priority_aware() {
        // F-010/§20: capacity N accepts N normal events.
        let mut q = TelemetryBuffer::new(2);
        assert!(q.push(Priority::P2Normal));
        assert!(q.push(Priority::P2Normal));
        // Debug is shed when full.
        assert!(!q.push(Priority::P3Debug));
        assert_eq!(q.dropped(Priority::P3Debug), 1);
        assert_eq!(q.queued(Priority::P2Normal), 2);
    }

    #[test]
    fn security_record_displaces_debug_noise() {
        let mut q = TelemetryBuffer::new(2);
        assert!(q.push(Priority::P3Debug));
        assert!(q.push(Priority::P3Debug));
        // P0 is preserved: it evicts debug instead of being dropped.
        assert!(q.push(Priority::P0Security));
        assert_eq!(q.queued(Priority::P0Security), 1);
        assert_eq!(q.queued(Priority::P3Debug), 1);
        assert_eq!(q.dropped(Priority::P3Debug), 1);
        assert_eq!(q.dropped(Priority::P0Security), 0);
    }

    #[test]
    fn security_record_is_dropped_only_when_nothing_evictable() {
        let mut q = TelemetryBuffer::new(1);
        assert!(q.push(Priority::P0Security));
        assert!(!q.push(Priority::P0Security));
        assert_eq!(q.dropped(Priority::P0Security), 1);
    }

    #[test]
    fn export_drains_most_important_first_with_exact_accounting() {
        let mut q = TelemetryBuffer::new(4);
        assert!(q.push(Priority::P3Debug));
        assert!(q.push(Priority::P0Security));
        assert!(q.push(Priority::P2Normal));
        q.export_one();
        assert_eq!(q.queued(Priority::P0Security), 0);
        assert_eq!(q.queued(Priority::P2Normal), 1);
        assert_eq!(q.queued(Priority::P3Debug), 1);
        q.export_one();
        q.export_one();
        assert_eq!(q.state(), TelemetryState::Healthy);
        // Exporting an empty buffer is a silent no-op; accounting holds.
        q.export_one();
        assert_eq!(q.state(), TelemetryState::Healthy);
    }
}
