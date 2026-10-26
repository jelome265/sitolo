//! Validated SQS consumer contracts.
//!
//! The business effect is intentionally a port: owner approval is required
//! before a concrete effect or queue configuration can be supplied.

use std::time::{Duration, SystemTime};
use std::{future::Future, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sitolo_audit::IamEventName;
use sitolo_domain::tenancy::{BranchId, OrganizationId};
use sitolo_events::{OutboxErrorClass, validate_iam_payload};
use sitolo_persistence::consumer_inbox::{
    ConsumerInboxError, ConsumerInboxRecord, ConsumerInboxWriter,
};
use sqlx_core::error::Error as DatabaseError;
use sqlx_postgres::{PgPool, Postgres};
use thiserror::Error;
use tokio::sync::watch;

use crate::sns::aggregate_group_id;

const INITIAL_CONSUMER_BACKOFF: Duration = Duration::from_millis(250);
const MAX_CONSUMER_BACKOFF: Duration = Duration::from_secs(30);
const CONSUMER_VISIBILITY_HEARTBEAT: Duration = Duration::from_secs(30);
const CONSUMER_VISIBILITY_LEASE_SECONDS: u32 = 60;

const MAX_EVENT_ID: usize = 128;
const MAX_AGGREGATE_PART: usize = 128;
const MAX_CONSUMER_MESSAGE_BYTES: usize = 256 * 1024;
const EXPECTED_FIELDS: [&str; 10] = [
    "event_id",
    "event_name",
    "event_version",
    "occurred_at",
    "organization_id",
    "branch_id",
    "aggregate_type",
    "aggregate_id",
    "aggregate_sequence",
    "payload",
];

/// Collapses any parse, conversion, or schema failure into the single
/// externally visible [`ConsumerEventError::Invalid`] class.
///
/// The source is deliberately dropped: JSON and domain errors can quote the
/// rejected payload, and the consumer boundary must never propagate or log it.
fn invalid<T, E>(result: Result<T, E>) -> Result<T, ConsumerEventError> {
    result.map_err(|source| {
        drop(source);
        ConsumerEventError::Invalid
    })
}

/// A decoded event that passed all transport-envelope and IAM payload checks.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsumerEvent {
    event_id: String,
    event_name: String,
    event_version: u32,
    occurred_at: SystemTime,
    organization_id: OrganizationId,
    branch_id: Option<BranchId>,
    aggregate_type: String,
    aggregate_id: String,
    aggregate_sequence: u64,
    payload: Value,
}

impl ConsumerEvent {
    /// Parses and validates the canonical SNS-to-SQS event body.
    ///
    /// # Errors
    /// Returns [`ConsumerEventError::Invalid`] when the body is empty or
    /// oversized, is not canonical JSON, carries unknown or missing fields,
    /// names an unregistered event, or fails the registered payload schema.
    /// The error never quotes the rejected body.
    pub fn parse(body: &str) -> Result<Self, ConsumerEventError> {
        if body.is_empty() || body.len() > MAX_CONSUMER_MESSAGE_BYTES {
            return Err(ConsumerEventError::Invalid);
        }
        let value: Value = invalid(serde_json::from_str(body))?;
        let object = value.as_object().ok_or(ConsumerEventError::Invalid)?;
        if object.len() != EXPECTED_FIELDS.len()
            || object
                .keys()
                .any(|field| !EXPECTED_FIELDS.contains(&field.as_str()))
        {
            return Err(ConsumerEventError::Invalid);
        }

        let event_id = required_string(object, "event_id")?;
        if !valid_event_id(&event_id) {
            return Err(ConsumerEventError::Invalid);
        }
        let event_name = required_string(object, "event_name")?;
        if !IamEventName::ALL
            .iter()
            .any(|registered| registered.as_str() == event_name)
        {
            return Err(ConsumerEventError::Invalid);
        }
        let event_version = required_u32(object, "event_version")?;
        if event_version != 1 {
            return Err(ConsumerEventError::Invalid);
        }
        let occurred_at = invalid(DateTime::parse_from_rfc3339(&required_string(
            object,
            "occurred_at",
        )?))?
        .with_timezone(&Utc)
        .into();
        let organization_id = invalid(OrganizationId::new(required_string(
            object,
            "organization_id",
        )?))?;
        let branch_id = match object.get("branch_id") {
            Some(Value::Null) => None,
            Some(Value::String(value)) => Some(invalid(BranchId::new(value.clone()))?),
            _ => return Err(ConsumerEventError::Invalid),
        };
        let aggregate_type = required_component(object, "aggregate_type")?;
        let aggregate_id = required_component(object, "aggregate_id")?;
        let aggregate_sequence = object
            .get("aggregate_sequence")
            .and_then(Value::as_u64)
            .filter(|sequence| *sequence > 0 && i64::try_from(*sequence).is_ok())
            .ok_or(ConsumerEventError::Invalid)?;
        let payload = object
            .get("payload")
            .cloned()
            .ok_or(ConsumerEventError::Invalid)?;
        if !payload.is_object() {
            return Err(ConsumerEventError::Invalid);
        }
        let payload_json = invalid(serde_json::to_string(&payload))?;
        invalid(validate_iam_payload(
            &event_name,
            event_version,
            &payload_json,
        ))?;

        Ok(Self {
            event_id,
            event_name,
            event_version,
            occurred_at,
            organization_id,
            branch_id,
            aggregate_type,
            aggregate_id,
            aggregate_sequence,
            payload,
        })
    }

    /// Stable event identifier.
    #[must_use]
    pub fn event_id(&self) -> &str {
        &self.event_id
    }
    /// Registered IAM event name.
    #[must_use]
    pub fn event_name(&self) -> &str {
        &self.event_name
    }
    /// Registered event schema version.
    #[must_use]
    pub fn event_version(&self) -> u32 {
        self.event_version
    }
    /// Time the producing transaction recorded the event.
    #[must_use]
    pub fn occurred_at(&self) -> SystemTime {
        self.occurred_at
    }
    /// Tenant the event belongs to.
    #[must_use]
    pub fn organization_id(&self) -> &OrganizationId {
        &self.organization_id
    }
    /// Branch scope, when the event is branch-specific.
    #[must_use]
    pub fn branch_id(&self) -> Option<&BranchId> {
        self.branch_id.as_ref()
    }
    /// Aggregate family that produced the event.
    #[must_use]
    pub fn aggregate_type(&self) -> &str {
        &self.aggregate_type
    }
    /// Stable aggregate identifier.
    #[must_use]
    pub fn aggregate_id(&self) -> &str {
        &self.aggregate_id
    }
    /// Monotonic per-aggregate sequence number.
    #[must_use]
    pub fn aggregate_sequence(&self) -> u64 {
        self.aggregate_sequence
    }
    /// Validated event payload.
    #[must_use]
    pub fn payload(&self) -> &Value {
        &self.payload
    }
}

/// Event body is malformed, unsupported, or fails its registered schema.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ConsumerEventError {
    /// The body is malformed, unsupported, or fails its registered schema.
    #[error("consumer event is invalid or unsupported")]
    Invalid,
}

/// Message and receipt handle delivered by an SQS-compatible transport port.
#[derive(Clone, Debug)]
pub struct ReceivedMessage {
    /// Stable source transport identifier, used to make quarantine transfer idempotent.
    pub message_id: String,
    /// Canonical SNS-to-SQS event body, validated before any effect runs.
    pub body: String,
    /// Opaque source receipt handle used for acknowledgement.
    pub receipt_handle: String,
    /// FIFO group identifier returned as an SQS system attribute.
    pub message_group_id: Option<String>,
}

/// Boundary for the configured SQS receive, acknowledgement and quarantine/DLQ operations.
/// Implementations must preserve FIFO ordering within each publisher message
/// group and must not allow a later message in a group to overtake one whose
/// processing is still in flight. The exact queue and receive configuration
/// remain deployment inputs.
#[async_trait]
pub trait ConsumerTransport: Send + Sync {
    /// Receives one bounded batch in the transport's ordering order.
    async fn receive_batch(&self) -> Result<Vec<ReceivedMessage>, ConsumerTransportError>;

    /// Acknowledges a source message after its durable effect/inbox commit.
    async fn acknowledge(&self, receipt_handle: &str) -> Result<(), ConsumerTransportError>;

    /// Extends the source message lease while its transaction is processing.
    async fn extend_visibility(
        &self,
        receipt_handle: &str,
        visibility_timeout: Duration,
    ) -> Result<(), ConsumerTransportError>;

    /// Routes a permanently rejected message to the configured quarantine boundary.
    /// Implementations must ensure the quarantine operation is durable before
    /// removing the source message. No queue identity is selected here.
    async fn quarantine(
        &self,
        message: &ReceivedMessage,
        error_class: &OutboxErrorClass,
    ) -> Result<(), ConsumerTransportError>;
}

/// Owner-supplied business effect. The passed transaction must contain every
/// durable effect so it commits atomically with the inbox idempotency marker.
#[async_trait]
pub trait ConsumerBusinessEffect: Send + Sync {
    /// Applies the effect inside `tx`, which also carries the inbox marker.
    async fn apply(
        &self,
        tx: &mut sqlx_core::transaction::Transaction<'_, Postgres>,
        event: &ConsumerEvent,
    ) -> Result<(), ConsumerEffectFailure>;
}

/// Business-effect failure classification, without exposing payload or provider details.
#[derive(Debug, Clone)]
pub enum ConsumerEffectFailure {
    /// The effect may succeed on redelivery; leave the message unacknowledged.
    Retryable,
    /// The effect can never succeed; quarantine with the given error class.
    Permanent(OutboxErrorClass),
}

/// Consumer processor failure.
#[derive(Debug, Error)]
pub enum ConsumerProcessError {
    /// The event envelope is malformed, unsupported, or fails its schema.
    #[error("consumer event is invalid or unsupported")]
    InvalidEvent(#[from] ConsumerEventError),
    /// A database operation failed.
    #[error("consumer database operation failed")]
    Database(#[from] DatabaseError),
    /// The inbox idempotency marker could not be written or read.
    #[error("consumer inbox persistence failed")]
    Inbox(#[from] ConsumerInboxError),
    /// Acknowledgement or quarantine failed.
    #[error("consumer transport operation failed")]
    Transport(#[from] ConsumerTransportError),
    /// The business effect failed retryably; the message stays unacknowledged.
    #[error("consumer business effect failed and is retryable")]
    EffectRetryable,
}

/// Transport acknowledgement or quarantine failed.
#[derive(Debug, Error, Clone, Copy)]
#[error("consumer transport operation failed")]
pub struct ConsumerTransportError;

/// Transactional message handler. It has no default business effect or queue configuration.
pub struct ConsumerProcessor {
    pool: PgPool,
    consumer_name: String,
    inbox: Arc<dyn ConsumerInboxWriter>,
    effect: Arc<dyn ConsumerBusinessEffect>,
    transport: Arc<dyn ConsumerTransport>,
    invalid_event_class: OutboxErrorClass,
}

impl std::fmt::Debug for ConsumerProcessor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The pool and adapters are represented by presence flags only: their
        // internals must never reach a log line or a panic payload.
        formatter
            .debug_struct("ConsumerProcessor")
            .field("consumer_name", &self.consumer_name)
            .field("invalid_event_class", &self.invalid_event_class)
            .finish_non_exhaustive()
    }
}

impl ConsumerProcessor {
    /// Constructs an explicitly configured consumer from its approved adapters.
    ///
    /// # Errors
    /// Returns [`ConsumerEventError::Invalid`] when `consumer_name` is empty,
    /// longer than 64 bytes, or contains anything outside lowercase ASCII
    /// alphanumerics, `_`, and `-`.
    pub fn new(
        pool: PgPool,
        consumer_name: String,
        inbox: Arc<dyn ConsumerInboxWriter>,
        effect: Arc<dyn ConsumerBusinessEffect>,
        transport: Arc<dyn ConsumerTransport>,
        invalid_event_class: OutboxErrorClass,
    ) -> Result<Self, ConsumerEventError> {
        if !consumer_name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        }) || consumer_name.is_empty()
            || consumer_name.len() > 64
        {
            return Err(ConsumerEventError::Invalid);
        }
        Ok(Self {
            pool,
            consumer_name,
            inbox,
            effect,
            transport,
            invalid_event_class,
        })
    }

    /// Processes one received message. It acknowledges only after commit; a
    /// retryable error leaves the source message unacknowledged. Invalid
    /// envelopes are quarantined without being reported as successfully handled.
    ///
    /// # Errors
    /// Returns [`ConsumerProcessError`] when the inbox or business-effect
    /// transaction fails, or when acknowledgement or quarantine itself fails.
    /// A retryable effect failure surfaces as
    /// [`ConsumerProcessError::EffectRetryable`] and leaves the message
    /// unacknowledged for redelivery.
    pub async fn process(&self, message: &ReceivedMessage) -> Result<(), ConsumerProcessError> {
        let processing = Box::pin(self.process_without_lease(message));
        self.maintain_visibility(message, processing).await
    }

    async fn process_without_lease(
        &self,
        message: &ReceivedMessage,
    ) -> Result<(), ConsumerProcessError> {
        let Ok(event) = ConsumerEvent::parse(&message.body) else {
            self.transport
                .quarantine(message, &self.invalid_event_class)
                .await?;
            return Ok(());
        };
        let expected_group_id = aggregate_group_id(
            event.organization_id().as_str(),
            event.aggregate_type(),
            event.aggregate_id(),
        );
        match message.message_group_id.as_deref() {
            Some(actual) if actual == expected_group_id => {}
            Some(_) => {
                self.transport
                    .quarantine(message, &self.invalid_event_class)
                    .await?;
                return Ok(());
            }
            None => return Err(ConsumerTransportError.into()),
        }

        let mut tx = self.pool.begin().await?;
        let payload = invalid(serde_json::to_vec(event.payload()))?;
        let sequence = invalid(i64::try_from(event.aggregate_sequence()))?;
        let record = ConsumerInboxRecord {
            consumer_name: &self.consumer_name,
            event_id: event.event_id(),
            event_name: event.event_name(),
            event_version: invalid(i32::try_from(event.event_version()))?,
            organization_id: event.organization_id().as_str(),
            branch_id: event.branch_id().map(BranchId::as_str),
            aggregate_type: event.aggregate_type(),
            aggregate_id: event.aggregate_id(),
            aggregate_sequence: sequence,
            occurred_at: DateTime::<Utc>::from(event.occurred_at()),
            payload: &payload,
        };
        let is_new = self.inbox.begin_processing(&mut tx, record).await?;
        if is_new {
            match self.effect.apply(&mut tx, &event).await {
                Ok(()) => {}
                Err(ConsumerEffectFailure::Retryable) => {
                    tx.rollback().await?;
                    return Err(ConsumerProcessError::EffectRetryable);
                }
                Err(ConsumerEffectFailure::Permanent(class)) => {
                    tx.rollback().await?;
                    self.transport.quarantine(message, &class).await?;
                    return Ok(());
                }
            }
        }
        tx.commit().await?;
        self.transport.acknowledge(&message.receipt_handle).await?;
        Ok(())
    }

    async fn maintain_visibility<F>(
        &self,
        message: &ReceivedMessage,
        processing: F,
    ) -> Result<(), ConsumerProcessError>
    where
        F: Future<Output = Result<(), ConsumerProcessError>>,
    {
        maintain_visibility_lease(
            self.transport.as_ref(),
            &message.receipt_handle,
            CONSUMER_VISIBILITY_HEARTBEAT,
            Duration::from_secs(u64::from(CONSUMER_VISIBILITY_LEASE_SECONDS)),
            processing,
        )
        .await
    }

    /// Processes a received batch serially and stops on the first retryable
    /// failure. This preserves the ordering returned for FIFO message groups;
    /// callers must not process overlapping batches concurrently per queue.
    ///
    /// # Errors
    /// Returns the first [`ConsumerProcessError`] from [`Self::process`]; the
    /// remaining messages stay unacknowledged for redelivery.
    pub async fn process_batch(
        &self,
        messages: &[ReceivedMessage],
    ) -> Result<(), ConsumerProcessError> {
        for message in messages {
            Box::pin(self.process(message)).await?;
        }
        Ok(())
    }

    /// Runs the configured consumer until shutdown. Receive and processing
    /// failures leave messages unacknowledged and use bounded exponential
    /// backoff; queue redrive policy remains an external deployment input.
    pub async fn run_until_shutdown(&self, mut shutdown: watch::Receiver<bool>) {
        let mut backoff = INITIAL_CONSUMER_BACKOFF;
        loop {
            if *shutdown.borrow() {
                return;
            }
            let received = tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return;
                    }
                    continue;
                }
                result = self.transport.receive_batch() => result,
            };
            match received {
                Ok(messages) if messages.is_empty() => {
                    backoff = INITIAL_CONSUMER_BACKOFF;
                }
                Ok(messages) => match Box::pin(self.process_batch(&messages)).await {
                    Ok(()) => backoff = INITIAL_CONSUMER_BACKOFF,
                    Err(error) => {
                        tracing::warn!(error_class = %error, "consumer batch left unacknowledged");
                        if !wait_or_shutdown(&mut shutdown, backoff).await {
                            return;
                        }
                        backoff = backoff.saturating_mul(2).min(MAX_CONSUMER_BACKOFF);
                    }
                },
                Err(error) => {
                    tracing::warn!(error_class = %error, "consumer receive failed");
                    if !wait_or_shutdown(&mut shutdown, backoff).await {
                        return;
                    }
                    backoff = backoff.saturating_mul(2).min(MAX_CONSUMER_BACKOFF);
                }
            }
        }
    }
}

async fn maintain_visibility_lease<F>(
    transport: &dyn ConsumerTransport,
    receipt_handle: &str,
    heartbeat: Duration,
    lease: Duration,
    processing: F,
) -> Result<(), ConsumerProcessError>
where
    F: Future<Output = Result<(), ConsumerProcessError>>,
{
    let mut interval = tokio::time::interval(heartbeat);
    // The first tick is immediate; start renewing only after one heartbeat.
    interval.tick().await;
    tokio::pin!(processing);
    loop {
        tokio::select! {
            result = &mut processing => return result,
            _ = interval.tick() => {
                transport
                    .extend_visibility(receipt_handle, lease)
                    .await?;
            }
        }
    }
}

async fn wait_or_shutdown(shutdown: &mut watch::Receiver<bool>, delay: Duration) -> bool {
    tokio::select! {
        changed = shutdown.changed() => changed.is_ok() && !*shutdown.borrow(),
        () = tokio::time::sleep(delay) => !*shutdown.borrow(),
    }
}

fn required_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, ConsumerEventError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(ConsumerEventError::Invalid)
}

fn required_u32(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<u32, ConsumerEventError> {
    object
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok())
        .ok_or(ConsumerEventError::Invalid)
}

fn required_component(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, ConsumerEventError> {
    let value = required_string(object, field)?;
    if value.is_empty()
        || value.len() > MAX_AGGREGATE_PART
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(ConsumerEventError::Invalid);
    }
    Ok(value)
}

fn valid_event_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_EVENT_ID
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~".contains(&byte)
        })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use sqlx_postgres::PgPoolOptions;

    #[derive(Default)]
    struct RecordingTransport {
        acknowledged: Arc<AtomicUsize>,
        quarantined: Arc<AtomicUsize>,
        visibility_extensions: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ConsumerTransport for RecordingTransport {
        async fn receive_batch(&self) -> Result<Vec<ReceivedMessage>, ConsumerTransportError> {
            Ok(Vec::new())
        }

        async fn acknowledge(&self, _receipt_handle: &str) -> Result<(), ConsumerTransportError> {
            self.acknowledged.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn extend_visibility(
            &self,
            _receipt_handle: &str,
            _visibility_timeout: Duration,
        ) -> Result<(), ConsumerTransportError> {
            self.visibility_extensions.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn quarantine(
            &self,
            _message: &ReceivedMessage,
            _error_class: &OutboxErrorClass,
        ) -> Result<(), ConsumerTransportError> {
            self.quarantined.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct TestInbox;

    #[async_trait]
    impl ConsumerInboxWriter for TestInbox {
        async fn begin_processing(
            &self,
            _tx: &mut sqlx_core::transaction::Transaction<'_, Postgres>,
            _event: ConsumerInboxRecord<'_>,
        ) -> Result<bool, ConsumerInboxError> {
            Ok(true)
        }
    }

    struct TestEffect;

    #[async_trait]
    impl ConsumerBusinessEffect for TestEffect {
        async fn apply(
            &self,
            _tx: &mut sqlx_core::transaction::Transaction<'_, Postgres>,
            _event: &ConsumerEvent,
        ) -> Result<(), ConsumerEffectFailure> {
            Ok(())
        }
    }

    fn valid_body() -> Value {
        serde_json::json!({
            "event_id": "evt-1",
            "event_name": "iam.invitation.issued",
            "event_version": 1,
            "occurred_at": "2026-10-23T00:00:00Z",
            "organization_id": "org-1",
            "branch_id": null,
            "aggregate_type": "Invitation",
            "aggregate_id": "inv-1",
            "aggregate_sequence": 1,
            "payload": {
                "invitation_id": "inv-1",
                "membership_id": "mem-1",
                "expires_at": "2026-11-23T00:00:00Z",
                "state_version": 1
            }
        })
    }

    #[test]
    fn event_parser_accepts_valid_registered_event_and_preserves_scope()
    -> Result<(), Box<dyn std::error::Error>> {
        let body = valid_body().to_string();
        let event = ConsumerEvent::parse(&body)?;
        assert_eq!(event.event_id(), "evt-1");
        assert_eq!(event.event_name(), "iam.invitation.issued");
        assert_eq!(event.event_version(), 1);
        assert_eq!(event.organization_id().as_str(), "org-1");
        assert_eq!(event.branch_id(), None);
        assert_eq!(event.aggregate_sequence(), 1);
        Ok(())
    }

    #[test]
    fn event_parser_rejects_malformed_or_permanently_invalid_envelopes() {
        assert!(ConsumerEvent::parse("not-json").is_err());

        for mutate in [
            ("event_id", Value::String("bad id".to_owned())),
            (
                "event_name",
                Value::String("iam.unregistered.event".to_owned()),
            ),
            ("event_version", Value::from(2)),
            ("organization_id", Value::String(String::new())),
            ("aggregate_sequence", Value::from(0)),
        ] {
            let mut body = valid_body();
            if let Some(object) = body.as_object_mut() {
                object.insert(mutate.0.to_owned(), mutate.1);
            }
            assert!(ConsumerEvent::parse(&body.to_string()).is_err());
        }

        let mut unknown_field = valid_body();
        if let Some(object) = unknown_field.as_object_mut() {
            object.insert("actor_id".to_owned(), Value::String("spoofed".to_owned()));
        }
        assert!(ConsumerEvent::parse(&unknown_field.to_string()).is_err());

        let mut invalid_payload = valid_body();
        if let Some(object) = invalid_payload.as_object_mut() {
            object.insert(
                "payload".to_owned(),
                serde_json::json!({"membership_id": "mem-1"}),
            );
        }
        assert!(ConsumerEvent::parse(&invalid_payload.to_string()).is_err());

        let mut non_object_payload = valid_body();
        if let Some(object) = non_object_payload.as_object_mut() {
            object.insert(
                "event_name".to_owned(),
                Value::String("iam.organization.created".to_owned()),
            );
            object.insert("payload".to_owned(), Value::String("invalid".to_owned()));
        }
        assert!(ConsumerEvent::parse(&non_object_payload.to_string()).is_err());
    }

    #[tokio::test]
    async fn malformed_message_is_quarantined_without_success_ack()
    -> Result<(), Box<dyn std::error::Error>> {
        let pool =
            PgPoolOptions::new().connect_lazy("postgres://app_worker:unused@127.0.0.1/sitolo")?;
        let transport = Arc::new(RecordingTransport::default());
        let acknowledged = Arc::clone(&transport.acknowledged);
        let quarantined = Arc::clone(&transport.quarantined);
        let transport_port: Arc<dyn ConsumerTransport> = transport;
        let processor = ConsumerProcessor::new(
            pool,
            "iam-projection".to_owned(),
            Arc::new(TestInbox),
            Arc::new(TestEffect),
            transport_port,
            OutboxErrorClass::new("invalid_event")?,
        )?;
        processor
            .process(&ReceivedMessage {
                message_id: "source-id".to_owned(),
                body: "not-json".to_owned(),
                receipt_handle: "opaque-receipt".to_owned(),
                message_group_id: Some("group-1".to_owned()),
            })
            .await?;

        assert_eq!(quarantined.load(Ordering::SeqCst), 1);
        assert_eq!(acknowledged.load(Ordering::SeqCst), 0);
        Ok(())
    }

    #[tokio::test]
    async fn invalid_fifo_group_is_rejected_before_database_access()
    -> Result<(), Box<dyn std::error::Error>> {
        let pool =
            PgPoolOptions::new().connect_lazy("postgres://app_worker:unused@127.0.0.1/sitolo")?;
        let transport = Arc::new(RecordingTransport::default());
        let acknowledged = Arc::clone(&transport.acknowledged);
        let quarantined = Arc::clone(&transport.quarantined);
        let transport_port: Arc<dyn ConsumerTransport> = transport;
        let processor = ConsumerProcessor::new(
            pool,
            "iam-projection".to_owned(),
            Arc::new(TestInbox),
            Arc::new(TestEffect),
            transport_port,
            OutboxErrorClass::new("invalid_event")?,
        )?;

        let mismatched_group = ReceivedMessage {
            message_id: "source-id".to_owned(),
            body: valid_body().to_string(),
            receipt_handle: "opaque-receipt".to_owned(),
            message_group_id: Some("wrong-group".to_owned()),
        };
        processor.process(&mismatched_group).await?;

        let missing_group = ReceivedMessage {
            message_id: "source-id-2".to_owned(),
            body: valid_body().to_string(),
            receipt_handle: "opaque-receipt-2".to_owned(),
            message_group_id: None,
        };
        assert!(matches!(
            processor.process(&missing_group).await,
            Err(ConsumerProcessError::Transport(_))
        ));

        assert_eq!(quarantined.load(Ordering::SeqCst), 1);
        assert_eq!(acknowledged.load(Ordering::SeqCst), 0);
        Ok(())
    }

    #[tokio::test]
    async fn processing_renews_visibility_until_the_operation_finishes()
    -> Result<(), Box<dyn std::error::Error>> {
        let transport = RecordingTransport::default();
        let extensions = Arc::clone(&transport.visibility_extensions);
        let processing = async {
            tokio::time::sleep(Duration::from_millis(15)).await;
            Ok(())
        };
        maintain_visibility_lease(
            &transport,
            "opaque-receipt",
            Duration::from_millis(2),
            Duration::from_secs(4),
            processing,
        )
        .await?;
        assert!(extensions.load(Ordering::SeqCst) >= 2);
        Ok(())
    }
}
