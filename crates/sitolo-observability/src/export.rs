//! Bounded, priority-aware telemetry export queue.
//!
//! [`TelemetryBuffer`] is an admission and shedding *model*: it counts queued
//! records per priority class and decides what to admit, evict or drop, but it
//! holds no payloads. Its own documentation assigns record payloads, ordering
//! and drain to the export integration. This module is that integration.
//!
//! [`TelemetryExporter`] keeps one FIFO payload queue per priority class and
//! keeps it in lockstep with the buffer's decisions, so the invariant
//! `queue[class].len() == buffer.queued(class)` always holds:
//!
//! - `record` asks the buffer to admit. If the buffer drops the record, its
//!   payload is dropped. If the buffer admitted it by evicting a less
//!   important class, the oldest payload of that class is evicted too.
//! - `drain_into` pops the most important payload first (the same order as
//!   [`TelemetryBuffer::export_one`]) and releases its slot.
//!
//! Operational rules from `docs/runbooks/telemetry-blackout.md` are enforced
//! by construction: the queue is bounded by the buffer's capacity (never
//! unbounded), the producer path only takes short, non-blocking critical
//! sections (telemetry loss must never block business operations), a failed
//! export releases its slot rather than wedging the queue, and a poisoned lock
//! is recovered rather than propagated into the request path.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::buffer::{Priority, TelemetryBuffer, TelemetryState};

/// Priority classes in significance order; index matches queue index.
const CLASSES: [Priority; 4] = [
    Priority::P0Security,
    Priority::P1Business,
    Priority::P2Normal,
    Priority::P3Debug,
];

fn class_index(priority: Priority) -> usize {
    priority as usize
}

/// Locks, recovering from poisoning: a panic elsewhere must never turn
/// telemetry into a failure of the request path.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Outcome of one [`TelemetryExporter::drain_into`] call.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DrainReport {
    /// Records the sink accepted.
    pub exported: usize,
    /// Records the sink rejected. Their slots are released; they are not
    /// retried, so a failing sink can never wedge the queue.
    pub failed: usize,
}

/// Point-in-time view of the export queue, for operators and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExporterSnapshot {
    pub state: TelemetryState,
    /// Queued payloads per class, most important first.
    pub queued: [u64; 4],
    /// Cumulative records dropped per class, most important first.
    pub dropped: [u64; 4],
    /// Cumulative records the sink rejected.
    pub export_failures: u64,
}

impl ExporterSnapshot {
    pub fn total_queued(&self) -> u64 {
        self.queued.iter().sum()
    }

    pub fn dropped(&self, priority: Priority) -> u64 {
        self.dropped[class_index(priority)]
    }
}

/// Payload queues kept in lockstep with a [`TelemetryBuffer`].
pub struct TelemetryExporter<T> {
    buffer: Arc<Mutex<TelemetryBuffer>>,
    queues: Mutex<[VecDeque<T>; 4]>,
    export_failures: AtomicU64,
}

impl<T> TelemetryExporter<T> {
    /// Wraps a buffer. Lock order is always `queues` then `buffer`.
    pub fn new(buffer: Arc<Mutex<TelemetryBuffer>>) -> Self {
        Self {
            buffer,
            queues: Mutex::new(std::array::from_fn(|_| VecDeque::new())),
            export_failures: AtomicU64::new(0),
        }
    }

    /// Offers a record. Returns whether it was admitted. A `false` result means
    /// the record was shed (counted by the buffer) and its payload dropped;
    /// callers must not treat that as an error.
    pub fn record(&self, priority: Priority, payload: T) -> bool {
        let mut queues = lock(&self.queues);
        let mut buffer = lock(&self.buffer);

        let before = CLASSES.map(|class| buffer.queued(class));
        if !buffer.push(priority) {
            return false;
        }
        // The buffer admitted by evicting a less important class: drop the
        // oldest payload of whichever class shrank to stay in lockstep.
        for (index, class) in CLASSES.iter().enumerate() {
            if index != class_index(priority) && buffer.queued(*class) < before[index] {
                queues[index].pop_front();
            }
        }
        queues[class_index(priority)].push_back(payload);
        true
    }

    /// Exports up to `max` records, most important first and FIFO within a
    /// class. The sink runs with no lock held. A sink error releases the
    /// record's slot and is counted; it is not retried.
    pub fn drain_into<E>(
        &self,
        max: usize,
        mut export: impl FnMut(T) -> Result<(), E>,
    ) -> DrainReport {
        let mut report = DrainReport::default();
        for _ in 0..max {
            let Some(record) = self.pop_most_important() else {
                break;
            };
            match export(record) {
                Ok(()) => report.exported += 1,
                Err(_) => {
                    report.failed += 1;
                    self.export_failures.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        report
    }

    fn pop_most_important(&self) -> Option<T> {
        let mut queues = lock(&self.queues);
        let mut buffer = lock(&self.buffer);
        for queue in queues.iter_mut() {
            if let Some(record) = queue.pop_front() {
                buffer.export_one();
                return Some(record);
            }
        }
        None
    }

    pub fn snapshot(&self) -> ExporterSnapshot {
        let _queues = lock(&self.queues);
        let buffer = lock(&self.buffer);
        ExporterSnapshot {
            state: buffer.state(),
            queued: CLASSES.map(|class| buffer.queued(class)),
            dropped: CLASSES.map(|class| buffer.dropped(class)),
            export_failures: self.export_failures.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exporter(capacity: usize) -> TelemetryExporter<u32> {
        TelemetryExporter::new(Arc::new(Mutex::new(TelemetryBuffer::new(capacity))))
    }

    fn drain_all(exporter: &TelemetryExporter<u32>) -> Vec<u32> {
        let mut out = Vec::new();
        exporter.drain_into(usize::MAX, |record| {
            out.push(record);
            Ok::<(), ()>(())
        });
        out
    }

    /// The core invariant: each class's payload queue is exactly as long as
    /// the buffer says it is, and the total never exceeds capacity.
    fn assert_lockstep(exporter: &TelemetryExporter<u32>, capacity: usize) {
        let queues = lock(&exporter.queues);
        let buffer = lock(&exporter.buffer);
        let mut total = 0;
        for (index, class) in CLASSES.iter().enumerate() {
            assert_eq!(
                queues[index].len() as u64,
                buffer.queued(*class),
                "class {class:?} payload queue diverged from the buffer"
            );
            total += queues[index].len();
        }
        assert!(total <= capacity, "queue exceeded capacity: {total}");
    }

    #[test]
    fn drains_most_important_first_and_fifo_within_a_class() {
        let exporter = exporter(16);
        assert!(exporter.record(Priority::P3Debug, 30));
        assert!(exporter.record(Priority::P2Normal, 20));
        assert!(exporter.record(Priority::P0Security, 1));
        assert!(exporter.record(Priority::P2Normal, 21));
        assert!(exporter.record(Priority::P1Business, 10));
        assert!(exporter.record(Priority::P0Security, 2));

        assert_eq!(drain_all(&exporter), vec![1, 2, 10, 20, 21, 30]);
        assert_eq!(exporter.snapshot().total_queued(), 0);
    }

    #[test]
    fn a_more_important_record_evicts_the_oldest_payload_of_the_least_important_class() {
        let exporter = exporter(3);
        assert!(exporter.record(Priority::P3Debug, 31)); // oldest debug
        assert!(exporter.record(Priority::P3Debug, 32));
        assert!(exporter.record(Priority::P2Normal, 20));

        // Full. A P1 record evicts the least important class (P3) first, and
        // within it the oldest payload.
        assert!(exporter.record(Priority::P1Business, 10));
        assert_lockstep(&exporter, 3);
        let snapshot = exporter.snapshot();
        assert_eq!(snapshot.dropped(Priority::P3Debug), 1);

        assert_eq!(drain_all(&exporter), vec![10, 20, 32]);
    }

    #[test]
    fn an_unimportant_record_is_dropped_when_nothing_less_important_can_yield() {
        let exporter = exporter(2);
        assert!(exporter.record(Priority::P0Security, 1));
        assert!(exporter.record(Priority::P0Security, 2));

        // Full of P0: a P3 record has nothing to evict and is shed itself.
        assert!(!exporter.record(Priority::P3Debug, 99));
        // Even another P0 cannot displace an equal-priority record.
        assert!(!exporter.record(Priority::P0Security, 3));

        let snapshot = exporter.snapshot();
        assert_eq!(snapshot.dropped(Priority::P3Debug), 1);
        assert_eq!(snapshot.dropped(Priority::P0Security), 1);
        assert_eq!(drain_all(&exporter), vec![1, 2]);
    }

    #[test]
    fn stays_in_lockstep_and_within_capacity_under_mixed_load() {
        let capacity = 7;
        let exporter = exporter(capacity);
        // Deterministic pseudo-random operation stream (no external crate).
        let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for step in 0..20_000u32 {
            let roll = next();
            if roll % 3 == 0 {
                exporter.drain_into((roll as usize >> 8) % 3 + 1, |_| Ok::<(), ()>(()));
            } else {
                let class = CLASSES[(roll as usize >> 4) % 4];
                exporter.record(class, step);
            }
            assert_lockstep(&exporter, capacity);
        }
    }

    #[test]
    fn a_failing_sink_releases_capacity_and_never_wedges_the_queue() {
        let exporter = exporter(4);
        for id in 0..4 {
            assert!(exporter.record(Priority::P2Normal, id));
        }
        assert_eq!(exporter.snapshot().state, TelemetryState::Blackout);

        let report = exporter.drain_into(usize::MAX, |_| Err::<(), &str>("collector down"));
        assert_eq!(
            report,
            DrainReport {
                exported: 0,
                failed: 4
            }
        );
        let snapshot = exporter.snapshot();
        assert_eq!(snapshot.export_failures, 4);
        assert_eq!(snapshot.total_queued(), 0);
        assert!(exporter.record(Priority::P2Normal, 100));
        assert_lockstep(&exporter, 4);
    }

    #[test]
    fn returns_to_healthy_after_pressure_is_drained() {
        // docs/runbooks/telemetry-blackout.md step 5: recovery must return the
        // buffer to a healthy state.
        let exporter = exporter(10);
        for id in 0..10 {
            exporter.record(Priority::P2Normal, id);
        }
        assert_eq!(exporter.snapshot().state, TelemetryState::Blackout);
        drain_all(&exporter);
        assert_eq!(exporter.snapshot().state, TelemetryState::Healthy);
    }

    #[test]
    fn a_poisoned_lock_does_not_propagate_into_the_producer() {
        let exporter = Arc::new(exporter(4));
        let poisoner = Arc::clone(&exporter);
        let joined = std::thread::spawn(move || {
            let _guard = poisoner.queues.lock().expect("lock");
            panic!("simulated panic while holding the telemetry lock");
        })
        .join();
        assert!(joined.is_err());

        // Telemetry must never take the request path down with it.
        assert!(exporter.record(Priority::P2Normal, 7));
        assert_eq!(drain_all(&exporter), vec![7]);
    }
}
