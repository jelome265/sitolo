//! Sitolo worker library.
//!
//! Owns outbox relay processing and execution boundaries.

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::SystemTime;

use sitolo_events::{EventDispatcher, OutboxRelayMetrics, RelayConfig};
use sitolo_persistence::AuditOutboxStore;

/// Outbox Relay Batch Processor (§23).
pub struct OutboxRelayRunner<S: AuditOutboxStore, D: EventDispatcher> {
    store: Arc<S>,
    dispatcher: Arc<D>,
    metrics: Arc<OutboxRelayMetrics>,
    config: RelayConfig,
}

impl<S: AuditOutboxStore, D: EventDispatcher> OutboxRelayRunner<S, D> {
    pub fn new(
        store: Arc<S>,
        dispatcher: Arc<D>,
        metrics: Arc<OutboxRelayMetrics>,
        config: RelayConfig,
    ) -> Self {
        Self {
            store,
            dispatcher,
            metrics,
            config,
        }
    }

    pub fn metrics(&self) -> &Arc<OutboxRelayMetrics> {
        &self.metrics
    }

    /// Executes one tick of outbox claiming, dispatching, and status recording.
    pub async fn process_batch(&self, now: SystemTime) -> Result<usize, String> {
        let events = self
            .store
            .claim_outbox_events(self.config.batch_size, self.config.lease_duration, now)
            .await
            .map_err(|e| format!("Failed to claim outbox events: {e:?}"))?;

        let count = events.len();
        if count == 0 {
            return Ok(0);
        }

        self.metrics.record_claim(count as u64);

        for event in events {
            match self.dispatcher.dispatch(&event).await {
                Ok(()) => {
                    self.store
                        .mark_published(&event.event_id, now)
                        .await
                        .map_err(|e| format!("Failed to mark outbox event published: {e:?}"))?;
                    self.metrics.record_published();
                }
                Err(err) => {
                    let status = self
                        .store
                        .mark_failed_or_quarantined(
                            &event.event_id,
                            err.classification,
                            &err.error_class,
                            now,
                            self.config.max_attempts,
                            self.config.backoff_duration,
                        )
                        .await
                        .map_err(|e| format!("Failed to mark outbox event error: {e:?}"))?;

                    if status == sitolo_events::OutboxStatus::Quarantined {
                        self.metrics.record_quarantine();
                    } else {
                        self.metrics.record_retry();
                    }
                }
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_events::{
        DispatchError, InMemoryDispatcher, OutboxEvent, OutboxEventId, OutboxStatus,
    };
    use sitolo_persistence::AuditOutboxDatabase;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    #[tokio::test]
    async fn runner_processes_outbox_batch_successfully() {
        let store = Arc::new(AuditOutboxDatabase::new());
        let dispatcher = Arc::new(InMemoryDispatcher::new());
        let metrics = Arc::new(OutboxRelayMetrics::new());
        let config = RelayConfig::default();

        let runner =
            OutboxRelayRunner::new(store.clone(), dispatcher.clone(), metrics.clone(), config);
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        let event = OutboxEvent {
            event_id: OutboxEventId::new("evt-runner-1").unwrap(),
            aggregate_type: "membership".into(),
            aggregate_id: "mem-1".into(),
            event_name: "iam.membership.invited".into(),
            event_version: 1,
            organization_id: Some("org-1".into()),
            branch_id: None,
            occurred_at: now,
            payload: "{}".into(),
            status: OutboxStatus::Pending,
            available_at: now,
            attempt_count: 0,
            locked_at: None,
            published_at: None,
            last_error_class: None,
            deduplication_key: "dedup-runner-1".into(),
            schema_version: 1,
        };
        store.enqueue_outbox(event).unwrap();

        let count = runner.process_batch(now).await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(dispatcher.dispatched_events().len(), 1);
        assert_eq!(metrics.published_count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn runner_handles_retryable_error() {
        let store = Arc::new(AuditOutboxDatabase::new());
        let dispatcher = Arc::new(InMemoryDispatcher::new());
        let metrics = Arc::new(OutboxRelayMetrics::new());
        let config = RelayConfig {
            max_attempts: 3,
            ..Default::default()
        };

        let runner =
            OutboxRelayRunner::new(store.clone(), dispatcher.clone(), metrics.clone(), config);
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        let event = OutboxEvent {
            event_id: OutboxEventId::new("evt-runner-retry").unwrap(),
            aggregate_type: "organization".into(),
            aggregate_id: "org-1".into(),
            event_name: "iam.organization.created".into(),
            event_version: 1,
            organization_id: Some("org-1".into()),
            branch_id: None,
            occurred_at: now,
            payload: "{}".into(),
            status: OutboxStatus::Pending,
            available_at: now,
            attempt_count: 0,
            locked_at: None,
            published_at: None,
            last_error_class: None,
            deduplication_key: "dedup-runner-retry".into(),
            schema_version: 1,
        };
        store.enqueue_outbox(event).unwrap();

        dispatcher.set_fail_next(DispatchError::retryable("network_503"));

        let count = runner.process_batch(now).await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(metrics.retry_count.load(Ordering::Relaxed), 1);

        let records = store.outbox_records();
        assert_eq!(records[0].status, OutboxStatus::Pending);
        assert_eq!(records[0].attempt_count, 1);
        assert_eq!(records[0].last_error_class.as_deref(), Some("network_503"));
    }
}
