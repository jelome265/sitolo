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

/// Telemetry importance.
///
/// Declaration order is significance order: `P0` is most important. Do not
/// reorder without reviewing shedding semantics, because the derived `Ord`
/// implementation decides which queued class is displaced first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Security-relevant records; never shed to admit a lower class.
    P0Security,
    /// Business-critical records.
    P1Business,
    /// Ordinary operational records.
    P2Normal,
    /// Diagnostic records, shed first under pressure.
    P3Debug,
}

/// Occupancy band reported for a [`TelemetryBuffer`].
///
/// Bands are computed from the queued record count as an integer percentage
/// of capacity: `Healthy` below 50%, `Degraded` from 50% to 74%, `Pressured`
/// from 75% to 89%, `Dropping` from 90% to 99%, and `Blackout` at or above
/// capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryState {
    /// Below 50% occupancy; all admissions are expected to succeed.
    Healthy,
    /// 50% to 74% occupancy; the buffer is filling but not yet under pressure.
    Degraded,
    /// 75% to 89% occupancy; shedding is likely for lower classes.
    Pressured,
    /// 90% to 99% occupancy; most lower-class admissions will displace records.
    Dropping,
    /// At or above capacity; every admission displaces or is dropped.
    Blackout,
}

/// Per-priority record counters.
///
/// A named-field layout makes every class addressable without an index, so
/// accounting cannot panic and cannot silently drift from the public class
/// set.
#[derive(Debug, Clone, Copy, Default)]
struct Counters {
    security: u64,
    business: u64,
    normal: u64,
    debug: u64,
}

impl Counters {
    /// Reads the count for `priority`.
    const fn get(&self, priority: Priority) -> u64 {
        match priority {
            Priority::P0Security => self.security,
            Priority::P1Business => self.business,
            Priority::P2Normal => self.normal,
            Priority::P3Debug => self.debug,
        }
    }

    /// Returns the mutable slot for `priority`.
    fn slot_mut(&mut self, priority: Priority) -> &mut u64 {
        match priority {
            Priority::P0Security => &mut self.security,
            Priority::P1Business => &mut self.business,
            Priority::P2Normal => &mut self.normal,
            Priority::P3Debug => &mut self.debug,
        }
    }

    /// Adds one record to `priority`, saturating at [`u64::MAX`].
    fn increment(&mut self, priority: Priority) {
        let slot = self.slot_mut(priority);
        *slot = slot.saturating_add(1);
    }

    /// Removes one record from `priority`, saturating at zero.
    fn decrement(&mut self, priority: Priority) {
        let slot = self.slot_mut(priority);
        *slot = slot.saturating_sub(1);
    }

    /// Total records across every class.
    ///
    /// Saturation is unreachable while [`TelemetryBuffer`]'s capacity
    /// invariant holds; it exists so accounting stays panic-free even if the
    /// invariant is ever broken.
    fn total(&self) -> u64 {
        self.security
            .saturating_add(self.business)
            .saturating_add(self.normal)
            .saturating_add(self.debug)
    }
}

/// Fixed-capacity admission model for telemetry records.
///
/// The buffer tracks per-priority occupancy rather than payloads. Invariant:
/// the sum of all queued counts never exceeds `capacity`, and each queued
/// record belongs to exactly one class. [`TelemetryBuffer::push`] therefore
/// either admits a record, displaces a less important queued class, or counts
/// the incoming record as dropped.
#[derive(Debug)]
pub struct TelemetryBuffer {
    capacity: u64,
    queued: Counters,
    dropped: Counters,
}

impl TelemetryBuffer {
    /// Creates an empty buffer that admits at most `capacity` records.
    ///
    /// # Panics
    ///
    /// Panics when `capacity` is zero. A zero-capacity buffer could never
    /// admit a record, so every push would be shed and the occupancy bands
    /// would carry no information.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "telemetry buffer capacity must be positive");
        Self {
            // `usize` is at most 64 bits on every supported target, so this
            // conversion is lossless; the fallback only guards a hypothetical
            // wider pointer.
            capacity: u64::try_from(capacity).unwrap_or(u64::MAX),
            queued: Counters::default(),
            dropped: Counters::default(),
        }
    }

    fn total_queued(&self) -> u64 {
        self.queued.total()
    }

    /// Reports the occupancy band for the currently queued records.
    ///
    /// The percentage is integer-truncated, so only a completely full buffer
    /// reports [`TelemetryState::Blackout`]; every lower occupancy reports one
    /// of the remaining bands.
    #[must_use]
    pub fn state(&self) -> TelemetryState {
        let queued = u128::from(self.total_queued());
        let capacity = u128::from(self.capacity);
        // `capacity` is non-zero by construction. A hypothetical zero capacity
        // is reported as full instead of dividing by zero.
        let percent = queued
            .saturating_mul(100)
            .checked_div(capacity)
            .unwrap_or(100);
        match percent {
            0..=49 => TelemetryState::Healthy,
            50..=74 => TelemetryState::Degraded,
            75..=89 => TelemetryState::Pressured,
            90..=99 => TelemetryState::Dropping,
            _ => TelemetryState::Blackout,
        }
    }

    /// Admits a record of priority `p`.
    ///
    /// Returns `true` when the record is queued. When the buffer is full, the
    /// least important non-empty class below `p` is displaced and counted as
    /// dropped; if no such class exists, `p` itself is counted as dropped and
    /// `false` is returned.
    pub fn push(&mut self, p: Priority) -> bool {
        if self.total_queued() < self.capacity {
            self.queued.increment(p);
            return true;
        }
        // Least important (highest discriminant) non-empty class first.
        for victim in [Priority::P3Debug, Priority::P2Normal, Priority::P1Business] {
            if victim > p && self.queued.get(victim) > 0 {
                self.queued.decrement(victim);
                self.dropped.increment(victim);
                self.queued.increment(p);
                return true;
            }
        }
        self.dropped.increment(p);
        false
    }

    /// Exports one queued record, most important class first.
    ///
    /// Queue accounting stays exact: one export removes exactly one queued
    /// record. Exporting an empty buffer is a silent no-op.
    pub fn export_one(&mut self) {
        for class in [
            Priority::P0Security,
            Priority::P1Business,
            Priority::P2Normal,
            Priority::P3Debug,
        ] {
            if self.queued.get(class) > 0 {
                self.queued.decrement(class);
                return;
            }
        }
    }

    /// Number of records currently queued for priority `p`.
    #[must_use]
    pub fn queued(&self, p: Priority) -> u64 {
        self.queued.get(p)
    }

    /// Number of records of priority `p` shed since construction.
    ///
    /// Counters saturate at [`u64::MAX`] instead of overflowing.
    #[must_use]
    pub fn dropped(&self, p: Priority) -> u64 {
        self.dropped.get(p)
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
