//! Outbox Relay Worker and Event Dispatcher boundary.
//!
//! Phase 4 Part 8 (§23, §24, §25, §44). Processes claimed outbox records,
//! dispatches to external/derived listeners through safe abstractions, and
//! records bounded metrics without sensitive labels.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;

use crate::{OutboxEvent, RetryClassification};

/// Error returned by an [`EventDispatcher`].
#[derive(Debug, Clone)]
pub struct DispatchError {
    pub classification: RetryClassification,
    pub error_class: String,
}

impl DispatchError {
    pub fn retryable(class: impl Into<String>) -> Self {
        Self {
            classification: RetryClassification::Retryable,
            error_class: class.into(),
        }
    }

    pub fn terminal(class: impl Into<String>) -> Self {
        Self {
            classification: RetryClassification::Terminal,
            error_class: class.into(),
        }
    }
}

/// Port for publishing outbox events to external/derived subscribers (§23).
#[async_trait]
pub trait EventDispatcher: Send + Sync {
    async fn dispatch(&self, event: &OutboxEvent) -> Result<(), DispatchError>;
}

/// In-memory mock dispatcher for testing relay delivery.
#[derive(Default)]
pub struct InMemoryDispatcher {
    dispatched: std::sync::Mutex<Vec<OutboxEvent>>,
    fail_next: std::sync::Mutex<Option<DispatchError>>,
}

impl InMemoryDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dispatched_events(&self) -> Vec<OutboxEvent> {
        self.dispatched.lock().unwrap().clone()
    }

    pub fn set_fail_next(&self, err: DispatchError) {
        *self.fail_next.lock().unwrap() = Some(err);
    }
}

#[async_trait]
impl EventDispatcher for InMemoryDispatcher {
    async fn dispatch(&self, event: &OutboxEvent) -> Result<(), DispatchError> {
        let mut fail_guard = self.fail_next.lock().unwrap();
        if let Some(err) = fail_guard.take() {
            return Err(err);
        }
        self.dispatched.lock().unwrap().push(event.clone());
        Ok(())
    }
}

/// Bounded operational metrics for outbox relay (§44).
/// High-cardinality fields (org_id, user_id, event payloads) are forbidden.
#[derive(Debug, Default)]
pub struct OutboxRelayMetrics {
    pub claimed_count: AtomicU64,
    pub published_count: AtomicU64,
    pub retry_count: AtomicU64,
    pub quarantine_count: AtomicU64,
    pub duplicate_delivery_count: AtomicU64,
}

impl OutboxRelayMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_claim(&self, count: u64) {
        self.claimed_count.fetch_add(count, Ordering::Relaxed);
    }

    pub fn record_published(&self) {
        self.published_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_retry(&self) {
        self.retry_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_quarantine(&self) {
        self.quarantine_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_duplicate(&self) {
        self.duplicate_delivery_count
            .fetch_add(1, Ordering::Relaxed);
    }
}

/// Configuration parameters for outbox relay runner (§23).
#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub batch_size: u32,
    pub lease_duration: Duration,
    pub max_attempts: u32,
    pub backoff_duration: Duration,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            lease_duration: Duration::from_secs(30),
            max_attempts: 5,
            backoff_duration: Duration::from_secs(5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutboxStatus;
    use std::sync::Arc;
    use std::time::SystemTime;

    #[tokio::test]
    async fn dispatcher_records_dispatched_event() {
        let dispatcher = InMemoryDispatcher::new();
        let event = OutboxEvent {
            event_id: crate::OutboxEventId::new("evt-1").unwrap(),
            aggregate_type: "org".into(),
            aggregate_id: "org-1".into(),
            event_name: "iam.organization.created".into(),
            event_version: 1,
            organization_id: Some("org-1".into()),
            branch_id: None,
            occurred_at: SystemTime::UNIX_EPOCH,
            payload: "{}".into(),
            status: OutboxStatus::Pending,
            available_at: SystemTime::UNIX_EPOCH,
            attempt_count: 0,
            locked_at: None,
            published_at: None,
            last_error_class: None,
            deduplication_key: "dedup-1".into(),
            schema_version: 1,
        };

        dispatcher.dispatch(&event).await.unwrap();
        assert_eq!(dispatcher.dispatched_events().len(), 1);
    }

    #[test]
    fn metrics_counters_increment() {
        let metrics = Arc::new(OutboxRelayMetrics::new());
        metrics.record_claim(5);
        metrics.record_published();
        metrics.record_retry();
        metrics.record_quarantine();

        assert_eq!(metrics.claimed_count.load(Ordering::Relaxed), 5);
        assert_eq!(metrics.published_count.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.retry_count.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.quarantine_count.load(Ordering::Relaxed), 1);
    }
}
