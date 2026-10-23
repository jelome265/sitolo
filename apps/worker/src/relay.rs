//! Bounded, destination-neutral outbox processing.
//!
//! The relay engine owns claim handling, event/version validation, retry and
//! quarantine transitions. A transport adapter must preserve `event_id` as
//! the idempotency key at the destination. This module does not select a
//! transport or endpoint.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use sitolo_audit::IamEventName;
use sitolo_events::{
    MAX_BACKOFF, OutboxClaimPolicy, OutboxErrorClass, OutboxEvent, OutboxStatus, RetryClass,
    validate_iam_payload,
};
use sitolo_persistence::outbox_repository::{OutboxPersistenceError, OutboxReader};
use thiserror::Error;
use uuid::Uuid;

const MIN_POLL_INTERVAL: Duration = Duration::from_millis(100);
const MAX_POLL_INTERVAL: Duration = Duration::from_secs(60);
const MIN_RETRY_DELAY: Duration = Duration::from_millis(1);
const SUPPORTED_EVENT_VERSION: u32 = 1;

/// A classified failure returned by the concrete transport adapter.
#[derive(Debug)]
pub struct DeliveryFailure {
    class: OutboxErrorClass,
    retry_class: RetryClass,
    worker_unavailable: bool,
}

impl DeliveryFailure {
    /// Constructs a delivery failure with a bounded persisted error class.
    #[must_use]
    pub fn new(class: OutboxErrorClass, retry_class: RetryClass) -> Self {
        Self {
            class,
            retry_class,
            worker_unavailable: false,
        }
    }

    /// Marks a destination-wide permission or configuration failure. The
    /// claimed event is released for retry and the worker then stops; it is
    /// never quarantined as an individually poisonous event.
    #[must_use]
    pub fn unavailable(class: OutboxErrorClass) -> Self {
        Self {
            class,
            retry_class: RetryClass::Retryable,
            worker_unavailable: true,
        }
    }
}

/// Port implemented by the approved transport adapter.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publishes an event and uses `idempotency_key` to make retries safe at
    /// the destination. Implementations must not report success until the
    /// destination has accepted the event under its documented acknowledgement
    /// contract.
    async fn publish(
        &self,
        event: &OutboxEvent,
        idempotency_key: &str,
    ) -> Result<(), DeliveryFailure>;
}

/// Validated controls for one worker process.
#[derive(Clone, Copy)]
pub struct RelayPolicy {
    claim: OutboxClaimPolicy,
    poll_interval: Duration,
    retry_base: Duration,
    retry_max: Duration,
}

impl RelayPolicy {
    /// Builds a policy with bounded polling and exponential retry limits.
    pub fn new(
        claim: OutboxClaimPolicy,
        poll_interval: Duration,
        retry_base: Duration,
        retry_max: Duration,
    ) -> Result<Self, WorkerError> {
        if !(MIN_POLL_INTERVAL..=MAX_POLL_INTERVAL).contains(&poll_interval) {
            return Err(WorkerError::InvalidPolicy);
        }
        if retry_base < MIN_RETRY_DELAY || retry_max < retry_base || retry_max > MAX_BACKOFF {
            return Err(WorkerError::InvalidPolicy);
        }
        Ok(Self {
            claim,
            poll_interval,
            retry_base,
            retry_max,
        })
    }
}

/// Aggregate result for one claim/publish pass. Suitable for structured logs
/// or metrics without exposing event payloads or claim tokens.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BatchReport {
    pub recovered: u32,
    pub claimed: usize,
    pub published: usize,
    pub retried: usize,
    pub quarantined: usize,
}

/// Relay runtime failure. Persistence and lease errors are never swallowed.
#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("invalid worker relay policy")]
    InvalidPolicy,
    #[error("claimed outbox event has invalid lease state")]
    InvalidClaim,
    #[error("event destination is unavailable or misconfigured")]
    PublisherUnavailable,
    #[error(transparent)]
    Persistence(#[from] OutboxPersistenceError),
}

/// Processes claimed events using one approved publisher.
pub struct RelayWorker {
    reader: Arc<dyn OutboxReader>,
    publisher: Arc<dyn EventPublisher>,
    policy: RelayPolicy,
}

impl RelayWorker {
    #[must_use]
    pub fn new(
        reader: Arc<dyn OutboxReader>,
        publisher: Arc<dyn EventPublisher>,
        policy: RelayPolicy,
    ) -> Self {
        Self {
            reader,
            publisher,
            policy,
        }
    }

    /// Claims and processes one bounded batch.
    pub async fn process_batch(&self) -> Result<BatchReport, WorkerError> {
        let recovered = self
            .reader
            .recover_stale_claims(self.policy.claim.lease_duration())
            .await?;
        let events = self.reader.claim_batch(self.policy.claim).await?;
        let mut report = BatchReport {
            recovered,
            claimed: events.len(),
            ..BatchReport::default()
        };

        for event in events {
            if event.status != OutboxStatus::Claimed {
                return Err(WorkerError::InvalidClaim);
            }
            let token = event
                .claim_token
                .as_ref()
                .ok_or(WorkerError::InvalidClaim)?;

            if !is_registered_event(&event) {
                let class = OutboxErrorClass::new("unsupported_event_schema")
                    .expect("static error class is valid");
                self.reader
                    .quarantine(&event.event_id, token, &class)
                    .await?;
                report.quarantined += 1;
                continue;
            }

            match self.publisher.publish(&event, &event.event_id).await {
                Ok(()) => {
                    self.reader.mark_published(&event.event_id, token).await?;
                    report.published += 1;
                }
                Err(failure) if failure.retry_class == RetryClass::Retryable => {
                    let backoff = retry_backoff_with_random(
                        self.policy.retry_base,
                        self.policy.retry_max,
                        event.attempt_count,
                        fresh_jitter_entropy(),
                    );
                    self.reader
                        .release_for_retry(&event.event_id, token, &failure.class, backoff)
                        .await?;
                    report.retried += 1;
                    if failure.worker_unavailable {
                        return Err(WorkerError::PublisherUnavailable);
                    }
                }
                Err(failure) => {
                    self.reader
                        .quarantine(&event.event_id, token, &failure.class)
                        .await?;
                    report.quarantined += 1;
                }
            }
        }

        tracing::info!(
            recovered = report.recovered,
            claimed = report.claimed,
            published = report.published,
            retried = report.retried,
            quarantined = report.quarantined,
            "outbox relay batch completed"
        );
        Ok(report)
    }

    /// Runs bounded claim passes until shutdown is signalled. The first pass
    /// runs immediately; persistence failures terminate the process path.
    pub async fn run_until_shutdown(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), WorkerError> {
        loop {
            if *shutdown.borrow() {
                return Ok(());
            }
            self.process_batch().await?;
            tokio::select! {
                _ = tokio::time::sleep(self.policy.poll_interval) => {}
                result = shutdown.changed() => {
                    if result.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }
}

fn is_registered_event(event: &OutboxEvent) -> bool {
    let valid_payload = serde_json::from_str::<serde_json::Value>(&event.payload)
        .is_ok_and(|payload| payload.is_object());
    valid_payload
        && event.event_version == SUPPORTED_EVENT_VERSION
        && event.schema_version == event.event_version
        && IamEventName::ALL
            .iter()
            .any(|registered| registered.as_str() == event.event_name)
        && validate_iam_payload(&event.event_name, event.event_version, &event.payload).is_ok()
}

fn retry_backoff_with_random(
    base: Duration,
    ceiling: Duration,
    attempt_count: u32,
    random: u64,
) -> Duration {
    let exponent = attempt_count.saturating_sub(1).min(31);
    let cap = base.saturating_mul(1_u32 << exponent).min(ceiling);
    let cap_millis = u64::try_from(cap.as_millis()).unwrap_or(u64::MAX);
    // PostgreSQL stores timestamps at microsecond precision. A 1ms floor
    // preserves a measurable positive delay while keeping full jitter bounded
    // by the exponential cap.
    let jittered_millis = ((u128::from(random) * u128::from(cap_millis)) >> 64) + 1;
    Duration::from_millis(u64::try_from(jittered_millis).unwrap_or(cap_millis))
}

fn fresh_jitter_entropy() -> u64 {
    let [_, _, _, _, _, _, _, _, b8, b9, b10, b11, b12, b13, b14, b15] =
        Uuid::new_v4().as_u128().to_le_bytes();
    u64::from_le_bytes([b8, b9, b10, b11, b12, b13, b14, b15])
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use sitolo_domain::tenancy::OrganizationId;
    use sitolo_events::{ClaimToken, RetryClass};

    use super::*;

    #[derive(Default)]
    struct RecordedTransitions {
        published: Vec<String>,
        retried: Vec<(String, Duration)>,
        quarantined: Vec<String>,
    }

    struct FakeReader {
        events: Mutex<Vec<OutboxEvent>>,
        transitions: Mutex<RecordedTransitions>,
        recovered_leases: Mutex<Vec<Duration>>,
        fail_recovery: Mutex<bool>,
    }

    #[async_trait]
    impl OutboxReader for FakeReader {
        async fn claim_batch(
            &self,
            _policy: OutboxClaimPolicy,
        ) -> Result<Vec<OutboxEvent>, OutboxPersistenceError> {
            Ok(std::mem::take(
                &mut *self.events.lock().expect("event lock"),
            ))
        }

        async fn mark_published(
            &self,
            event_id: &str,
            _claim_token: &ClaimToken,
        ) -> Result<(), OutboxPersistenceError> {
            self.transitions
                .lock()
                .expect("transition lock")
                .published
                .push(event_id.to_owned());
            Ok(())
        }

        async fn release_for_retry(
            &self,
            event_id: &str,
            _claim_token: &ClaimToken,
            _error_class: &OutboxErrorClass,
            backoff: Duration,
        ) -> Result<(), OutboxPersistenceError> {
            self.transitions
                .lock()
                .expect("transition lock")
                .retried
                .push((event_id.to_owned(), backoff));
            Ok(())
        }

        async fn quarantine(
            &self,
            event_id: &str,
            _claim_token: &ClaimToken,
            _error_class: &OutboxErrorClass,
        ) -> Result<(), OutboxPersistenceError> {
            self.transitions
                .lock()
                .expect("transition lock")
                .quarantined
                .push(event_id.to_owned());
            Ok(())
        }

        async fn recover_stale_claims(
            &self,
            lease_timeout: Duration,
        ) -> Result<u32, OutboxPersistenceError> {
            if *self.fail_recovery.lock().expect("recovery failure lock") {
                return Err(OutboxPersistenceError::InvalidRow("recovery"));
            }
            self.recovered_leases
                .lock()
                .expect("recovery lock")
                .push(lease_timeout);
            Ok(1)
        }
    }

    struct FakePublisher {
        failure: Option<DeliveryFailure>,
        idempotency_keys: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl EventPublisher for FakePublisher {
        async fn publish(
            &self,
            _event: &OutboxEvent,
            idempotency_key: &str,
        ) -> Result<(), DeliveryFailure> {
            self.idempotency_keys
                .lock()
                .expect("publisher lock")
                .push(idempotency_key.to_owned());
            match &self.failure {
                Some(failure) => {
                    let class = OutboxErrorClass::new(failure.class.as_str().to_owned())
                        .expect("copied class is valid");
                    if failure.worker_unavailable {
                        Err(DeliveryFailure::unavailable(class))
                    } else {
                        Err(DeliveryFailure::new(class, failure.retry_class))
                    }
                }
                None => Ok(()),
            }
        }
    }

    fn policy() -> RelayPolicy {
        RelayPolicy::new(
            OutboxClaimPolicy::new(8, Duration::from_secs(30), 5).expect("claim policy"),
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(20),
        )
        .expect("relay policy")
    }

    fn claimed_event(event_id: &str, event_name: &str, version: u32, attempt: u32) -> OutboxEvent {
        let mut event = OutboxEvent::new(
            event_id.to_owned(),
            "Organization".to_owned(),
            "org-1".to_owned(),
            event_name.to_owned(),
            version,
            OrganizationId::new("org-1").expect("organization id"),
            None,
            "{}".to_owned(),
        )
        .expect("event");
        event.status = OutboxStatus::Claimed;
        event.claim_token = Some(ClaimToken::new("claim-1").expect("claim token"));
        event.attempt_count = attempt;
        event
    }

    fn worker(
        events: Vec<OutboxEvent>,
        failure: Option<DeliveryFailure>,
    ) -> (RelayWorker, Arc<FakeReader>, Arc<FakePublisher>) {
        let reader = Arc::new(FakeReader {
            events: Mutex::new(events),
            transitions: Mutex::new(RecordedTransitions::default()),
            recovered_leases: Mutex::new(Vec::new()),
            fail_recovery: Mutex::new(false),
        });
        let publisher = Arc::new(FakePublisher {
            failure,
            idempotency_keys: Mutex::new(Vec::new()),
        });
        (
            RelayWorker::new(reader.clone(), publisher.clone(), policy()),
            reader,
            publisher,
        )
    }

    #[tokio::test]
    async fn success_uses_event_id_as_destination_idempotency_key() {
        let (worker, reader, publisher) = worker(
            vec![claimed_event("evt-1", "iam.organization.created", 1, 1)],
            None,
        );

        let report = worker.process_batch().await.expect("batch succeeds");
        assert_eq!(
            report,
            BatchReport {
                recovered: 1,
                claimed: 1,
                published: 1,
                retried: 0,
                quarantined: 0
            }
        );
        assert_eq!(
            reader
                .recovered_leases
                .lock()
                .expect("recovery lock")
                .as_slice(),
            [Duration::from_secs(30)]
        );
        assert_eq!(
            *publisher.idempotency_keys.lock().expect("publisher lock"),
            ["evt-1"]
        );
        assert_eq!(
            reader
                .transitions
                .lock()
                .expect("transition lock")
                .published,
            ["evt-1"]
        );
    }

    #[tokio::test]
    async fn stale_claim_recovery_failure_aborts_before_claim_or_publish() {
        let (worker, reader, publisher) = worker(
            vec![claimed_event("evt-8", "iam.organization.created", 1, 1)],
            None,
        );
        *reader.fail_recovery.lock().expect("recovery failure lock") = true;

        assert!(matches!(
            worker.process_batch().await,
            Err(WorkerError::Persistence(_))
        ));
        assert!(
            publisher
                .idempotency_keys
                .lock()
                .expect("publisher lock")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn retryable_failure_uses_bounded_full_jitter_backoff() {
        let failure = DeliveryFailure::new(
            OutboxErrorClass::new("transport_timeout").expect("error class"),
            RetryClass::Retryable,
        );
        let (worker, reader, _) = worker(
            vec![claimed_event("evt-2", "iam.organization.created", 1, 3)],
            Some(failure),
        );

        let report = worker.process_batch().await.expect("batch succeeds");
        assert_eq!(report.retried, 1);
        let transitions = reader.transitions.lock().expect("transition lock");
        assert_eq!(transitions.retried.len(), 1);
        let (event_id, backoff) = transitions.retried.first().expect("retry transition");
        assert_eq!(event_id, "evt-2");
        assert!(*backoff > Duration::ZERO);
        assert!(*backoff <= Duration::from_secs(8));
    }

    #[tokio::test]
    async fn destination_wide_failure_releases_event_and_stops_without_quarantine() {
        let failure = DeliveryFailure::unavailable(
            OutboxErrorClass::new("sns_destination_unavailable").expect("error class"),
        );
        let (worker, reader, _) = worker(
            vec![claimed_event("evt-10", "iam.organization.created", 1, 1)],
            Some(failure),
        );

        let result = worker.process_batch().await;
        assert!(
            matches!(result, Err(WorkerError::PublisherUnavailable)),
            "unexpected destination failure handling: {result:?}"
        );
        let transitions = reader.transitions.lock().expect("transition lock");
        assert_eq!(transitions.retried.len(), 1);
        assert_eq!(
            transitions
                .retried
                .first()
                .map(|(event_id, _)| event_id.as_str()),
            Some("evt-10")
        );
        assert!(transitions.quarantined.is_empty());
    }

    #[tokio::test]
    async fn unknown_and_terminal_failures_are_quarantined() {
        for retry_class in [RetryClass::Unknown, RetryClass::Terminal] {
            let failure = DeliveryFailure::new(
                OutboxErrorClass::new("permanent_rejection").expect("error class"),
                retry_class,
            );
            let (worker, reader, _) = worker(
                vec![claimed_event("evt-3", "iam.organization.created", 1, 1)],
                Some(failure),
            );
            let report = worker.process_batch().await.expect("batch succeeds");
            assert_eq!(report.quarantined, 1);
            assert_eq!(
                reader
                    .transitions
                    .lock()
                    .expect("transition lock")
                    .quarantined,
                ["evt-3"]
            );
        }
    }

    #[tokio::test]
    async fn unsupported_event_or_version_is_quarantined_without_publish() {
        let mut invalid_payload = claimed_event("evt-6", "iam.organization.created", 1, 1);
        invalid_payload.payload = "[]".to_owned();
        let mut invalid_lifecycle_payload =
            claimed_event("evt-9", "iam.organization.created", 1, 1);
        invalid_lifecycle_payload.event_name = "iam.membership.pending_acceptance".to_owned();
        invalid_lifecycle_payload.payload = r#"{"membership_id":"member-1","from_state":"INVITED","to_state":"PENDING_ACCEPTANCE","state_version":2,"email":"person@example.test"}"#.to_owned();
        let mut mismatched_schema = claimed_event("evt-7", "iam.organization.created", 1, 1);
        mismatched_schema.schema_version = 2;
        let (worker, reader, publisher) = worker(
            vec![
                claimed_event("evt-4", "iam.unknown.created", 1, 1),
                claimed_event("evt-5", "iam.organization.created", 2, 1),
                invalid_payload,
                invalid_lifecycle_payload,
                mismatched_schema,
            ],
            None,
        );

        let report = worker.process_batch().await.expect("batch succeeds");
        assert_eq!(report.quarantined, 5);
        assert!(
            publisher
                .idempotency_keys
                .lock()
                .expect("publisher lock")
                .is_empty()
        );
        assert_eq!(
            reader
                .transitions
                .lock()
                .expect("transition lock")
                .quarantined,
            ["evt-4", "evt-5", "evt-6", "evt-9", "evt-7"]
        );
    }

    #[test]
    fn policy_rejects_unbounded_poll_and_retry_values() {
        let claim = OutboxClaimPolicy::new(1, Duration::from_secs(1), 1).expect("claim policy");
        assert!(
            RelayPolicy::new(
                claim,
                Duration::ZERO,
                Duration::from_secs(1),
                Duration::from_secs(1)
            )
            .is_err()
        );
        assert!(
            RelayPolicy::new(
                claim,
                Duration::from_secs(1),
                Duration::ZERO,
                Duration::from_secs(1)
            )
            .is_err()
        );
        assert!(
            RelayPolicy::new(
                claim,
                Duration::from_secs(1),
                Duration::from_nanos(999_999),
                Duration::from_secs(1)
            )
            .is_err()
        );
        assert!(
            RelayPolicy::new(
                claim,
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(1)
            )
            .is_err()
        );
        assert!(
            RelayPolicy::new(
                claim,
                Duration::from_secs(1),
                Duration::from_secs(1),
                MAX_BACKOFF + Duration::from_secs(1)
            )
            .is_err()
        );
    }

    #[test]
    fn retry_backoff_uses_full_jitter_within_exponential_ceiling() {
        let base = Duration::from_secs(2);
        let ceiling = Duration::from_secs(20);

        assert_eq!(
            retry_backoff_with_random(base, ceiling, 1, 0),
            Duration::from_millis(1)
        );
        assert_eq!(
            retry_backoff_with_random(base, ceiling, 1, u64::MAX),
            Duration::from_secs(2)
        );
        assert_eq!(
            retry_backoff_with_random(base, ceiling, 4, u64::MAX),
            Duration::from_secs(16)
        );
        assert_eq!(
            retry_backoff_with_random(base, ceiling, 5, u64::MAX),
            Duration::from_secs(20)
        );
        assert_eq!(
            retry_backoff_with_random(base, ceiling, u32::MAX, u64::MAX),
            Duration::from_secs(20)
        );
    }
}
