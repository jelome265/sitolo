//! API telemetry export: the typed record, its sink, and the drain loop.
//!
//! Request handling only *offers* a record to the bounded, priority-aware
//! [`TelemetryExporter`] (a short, non-blocking critical section); emission to
//! the sink happens on a background drain loop. Under pressure the exporter
//! sheds the least important records first, which is the behavior
//! `docs/runbooks/telemetry-blackout.md` requires: telemetry loss must never
//! block business operations, and queues must never be unbounded.
//!
//! The sink here is structured `tracing` output, the only exporter that
//! exists today. The exporter's sink is a plain closure, so a collector-backed
//! sink can replace it without touching admission, shedding or the request
//! path.
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use sitolo_observability::{DrainReport, Priority, TelemetryExporter, TelemetryState};
use tokio::sync::watch;
use tokio::time::MissedTickBehavior;

/// How often the background loop drains the queue.
pub const DRAIN_INTERVAL: Duration = Duration::from_millis(250);

/// Upper bound on records emitted per tick, so one tick can never starve the
/// runtime however deep the queue is.
pub const DRAIN_BATCH: usize = 512;

/// Bound on the final flush at shutdown. The queue itself is already bounded
/// by `otel_max_queue`; this guards against a stuck sink.
pub const FINAL_FLUSH_DEADLINE: Duration = Duration::from_secs(2);

/// Priority class labels, most important first (the `priority` label of
/// `sitolo_telemetry_dropped_records_total` in `docs/telemetry/metrics.yaml`).
const PRIORITY_LABELS: [&str; 4] = ["p0_security", "p1_business", "p2_normal", "p3_debug"];

/// The registered `http.request.completed` event
/// (`docs/telemetry/events.yaml`). Every field is low-cardinality or a
/// correlation id; never a raw path, header, body or credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestCompleted {
    pub method: String,
    /// Matched route template, or `unmatched`. Never the raw path.
    pub route_template: String,
    pub status: u16,
    pub status_class: &'static str,
    pub latency_ms: u64,
    pub request_id: String,
    /// A single, semantically valid W3C `traceparent`, when one was supplied.
    pub traceparent: Option<String>,
}

impl HttpRequestCompleted {
    /// Liveness/readiness probes are polled continuously by load balancers
    /// and are the first thing to shed; all other requests are normal traffic.
    pub fn priority(&self) -> Priority {
        if matches!(
            self.route_template.as_str(),
            "/process/live" | "/process/ready"
        ) {
            Priority::P3Debug
        } else {
            Priority::P2Normal
        }
    }
}

pub type ApiTelemetry = TelemetryExporter<HttpRequestCompleted>;

/// The `tracing` sink. Infallible: a log write cannot be rejected here.
pub fn emit(record: HttpRequestCompleted) -> Result<(), Infallible> {
    tracing::info!(
        event = "http.request.completed",
        method = %record.method,
        route_template = %record.route_template,
        request_id = %record.request_id,
        status = record.status,
        status_class = %record.status_class,
        latency_ms = record.latency_ms,
        traceparent = record.traceparent.as_deref(),
        "request completed"
    );
    Ok(())
}

/// Emits everything currently queued. Bounded, because the queue is.
pub fn flush(exporter: &ApiTelemetry) -> DrainReport {
    exporter.drain_into(usize::MAX, emit)
}

/// What the loop last reported, so each tick logs only what changed.
struct Observed {
    state: TelemetryState,
    dropped: [u64; 4],
    export_failures: u64,
}

/// Reports operator-facing signals from the runbook (dropped-record counters,
/// queue pressure, export failures) at most once per tick per signal. These
/// are plain `tracing` events and are deliberately never fed back into the
/// exporter, so telemetry about telemetry cannot amplify itself.
fn observe(exporter: &ApiTelemetry, observed: &mut Observed) {
    let snapshot = exporter.snapshot();

    for (index, label) in PRIORITY_LABELS.iter().enumerate() {
        let newly_dropped = snapshot.dropped[index].saturating_sub(observed.dropped[index]);
        if newly_dropped > 0 {
            tracing::warn!(
                event = "telemetry.record.dropped",
                priority = *label,
                dropped = newly_dropped,
                total_dropped = snapshot.dropped[index],
                "telemetry records shed under queue pressure"
            );
        }
    }

    let newly_failed = snapshot
        .export_failures
        .saturating_sub(observed.export_failures);
    if newly_failed > 0 {
        tracing::error!(
            event = "telemetry.export.failed",
            failed = newly_failed,
            "telemetry sink rejected records"
        );
    }

    if snapshot.state != observed.state {
        match snapshot.state {
            TelemetryState::Healthy | TelemetryState::Degraded => tracing::info!(
                from = ?observed.state,
                to = ?snapshot.state,
                "telemetry queue state changed"
            ),
            _ => tracing::warn!(
                from = ?observed.state,
                to = ?snapshot.state,
                "telemetry queue state changed"
            ),
        }
    }

    *observed = Observed {
        state: snapshot.state,
        dropped: snapshot.dropped,
        export_failures: snapshot.export_failures,
    };
}

/// Drains the exporter every `interval` until `stop` fires (or its sender is
/// dropped), then performs one final flush so records admitted before shutdown
/// are not lost.
pub async fn run_drain_loop(
    exporter: Arc<ApiTelemetry>,
    interval: Duration,
    mut stop: watch::Receiver<bool>,
) {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let initial = exporter.snapshot();
    let mut observed = Observed {
        state: initial.state,
        dropped: initial.dropped,
        export_failures: initial.export_failures,
    };

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                exporter.drain_into(DRAIN_BATCH, emit);
                observe(&exporter, &mut observed);
            }
            _ = stop.changed() => break,
        }
    }

    flush(&exporter);
    observe(&exporter, &mut observed);
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Mutex;

    use sitolo_observability::TelemetryBuffer;

    use super::*;

    fn record(route: &str, id: u32) -> HttpRequestCompleted {
        HttpRequestCompleted {
            method: "GET".into(),
            route_template: route.into(),
            status: 200,
            status_class: "2xx",
            latency_ms: 1,
            request_id: format!("req-{id}"),
            traceparent: None,
        }
    }

    fn exporter(capacity: usize) -> Arc<ApiTelemetry> {
        Arc::new(TelemetryExporter::new(Arc::new(Mutex::new(
            TelemetryBuffer::new(capacity),
        ))))
    }

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Capture {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().expect("lock").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Capture {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().expect("lock")).into_owned()
        }
    }

    fn capture_tracing() -> (Capture, tracing::subscriber::DefaultGuard) {
        let capture = Capture::default();
        let writer = capture.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .finish();
        (capture, tracing::subscriber::set_default(subscriber))
    }

    #[test]
    fn probes_are_debug_priority_and_everything_else_is_normal() {
        assert_eq!(record("/process/live", 0).priority(), Priority::P3Debug);
        assert_eq!(record("/process/ready", 0).priority(), Priority::P3Debug);
        assert_eq!(
            record("/v1/organizations", 0).priority(),
            Priority::P2Normal
        );
        assert_eq!(record("unmatched", 0).priority(), Priority::P2Normal);
    }

    #[test]
    fn flush_emits_every_admitted_record_and_empties_the_queue() {
        let (capture, _guard) = capture_tracing();
        let exporter = exporter(8);
        for id in 0..3 {
            let rec = record("/v1/organizations", id);
            assert!(exporter.record(rec.priority(), rec));
        }

        let report = flush(&exporter);
        assert_eq!(report.exported, 3);
        assert_eq!(report.failed, 0);
        assert_eq!(exporter.snapshot().total_queued(), 0);

        let log = capture.text();
        assert_eq!(log.matches("http.request.completed").count(), 3, "{log}");
        assert!(log.contains("request_id=req-2"), "{log}");
    }

    #[test]
    fn shedding_is_reported_per_priority_and_recovery_is_reported() {
        let (capture, _guard) = capture_tracing();
        let exporter = exporter(4);
        let initial = exporter.snapshot();
        let mut observed = Observed {
            state: initial.state,
            dropped: initial.dropped,
            export_failures: initial.export_failures,
        };

        // Fill with debug records, then overflow with normal ones: each
        // evicts a debug record (shed, counted against P3).
        for id in 0..4 {
            let rec = record("/process/live", id);
            exporter.record(rec.priority(), rec);
        }
        for id in 4..8 {
            let rec = record("/v1/organizations", id);
            exporter.record(rec.priority(), rec);
        }
        observe(&exporter, &mut observed);

        let log = capture.text();
        assert!(log.contains("telemetry.record.dropped"), "{log}");
        assert!(log.contains("priority=\"p3_debug\""), "{log}");
        assert!(log.contains("dropped=4"), "{log}");
        assert!(log.contains("telemetry queue state changed"), "{log}");
        assert!(log.contains("to=Blackout"), "{log}");

        // Draining reports the recovery and nothing is re-reported as shed.
        flush(&exporter);
        observe(&exporter, &mut observed);
        let log = capture.text();
        assert!(log.contains("to=Healthy"), "{log}");
        assert_eq!(log.matches("telemetry.record.dropped").count(), 1, "{log}");
    }

    #[tokio::test]
    async fn drain_loop_exports_on_its_own_and_flushes_on_stop() {
        let exporter = exporter(8);
        let (stop_tx, stop_rx) = watch::channel(false);
        let task = tokio::spawn(run_drain_loop(
            Arc::clone(&exporter),
            Duration::from_millis(10),
            stop_rx,
        ));

        let rec = record("/v1/organizations", 1);
        assert!(exporter.record(rec.priority(), rec));
        tokio::time::timeout(Duration::from_secs(2), async {
            while exporter.snapshot().total_queued() > 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the loop drains without any caller flushing");

        // Records admitted just before stop are not lost: the loop flushes.
        let rec = record("/v1/organizations", 2);
        assert!(exporter.record(rec.priority(), rec));
        stop_tx.send(true).expect("loop is listening");
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("loop stops promptly")
            .expect("loop does not panic");
        assert_eq!(exporter.snapshot().total_queued(), 0);
    }
}
