//! Transactional outbox events.
//!
//! The outbox stores durable event intent in the same database transaction as
//! the business mutation. A separate relay/worker eventually publishes these
//! events to external systems. Delivery is at-least-once; consumers must be
//! idempotent.
//!
//! Reference: <https://microservices.io/patterns/data/transactional-outbox>

use std::time::{Duration, SystemTime};

use thiserror::Error;

use sitolo_domain::tenancy::{BranchId, OrganizationId};

/// Maximum number of events a worker may claim in one batch.
pub const MAX_BATCH_SIZE: usize = 1_000;

/// Maximum lease a worker may hold for a claimed event.
pub const MAX_LEASE_DURATION: Duration = Duration::from_secs(3_600);

/// Maximum retry backoff a worker may request.
pub const MAX_BACKOFF: Duration = Duration::from_secs(3_600);

/// Maximum number of delivery attempts before an event is quarantined.
pub const MAX_ATTEMPTS: u32 = 10;

/// Maximum accepted length of a bounded error class.
pub const MAX_ERROR_CLASS_LEN: usize = 64;

/// Maximum accepted length of a claim token.
pub const MAX_CLAIM_TOKEN_LEN: usize = 128;

/// Outbox event status state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxStatus {
    /// Ready to be claimed by a worker.
    Pending,
    /// Leased to a worker.
    Claimed,
    /// Delivered to the destination.
    Published,
    /// Terminal failure; requires an operator.
    Quarantined,
}

impl OutboxStatus {
    /// Stable database representation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            OutboxStatus::Pending => "PENDING",
            OutboxStatus::Claimed => "CLAIMED",
            OutboxStatus::Published => "PUBLISHED",
            OutboxStatus::Quarantined => "QUARANTINED",
        }
    }

    /// Whether no further delivery attempt is permitted.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, OutboxStatus::Published | OutboxStatus::Quarantined)
    }
}

/// Delivery classification for a failed attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    /// The same event may be delivered again after a bounded backoff.
    Retryable,
    /// The destination rejected the event permanently.
    Terminal,
    /// The failure classification is not known; quarantine rather than loop.
    Unknown,
}

impl RetryClass {
    /// Whether a retry is permitted for this classification.
    #[must_use]
    pub fn is_retryable(self) -> bool {
        matches!(self, RetryClass::Retryable)
    }
}

/// Outbox validation error.
#[derive(Debug, Error)]
pub enum OutboxError {
    /// Persistence failed.
    #[error("outbox persistence failed")]
    PersistenceFailed,
    /// Payload exceeds the per-event limit.
    #[error("event payload too large")]
    PayloadTooLarge,
    /// Event schema is invalid.
    #[error("invalid event schema")]
    InvalidSchema,
    /// Deduplication key already exists.
    #[error("event already exists")]
    Duplicate,
    /// Batch size is zero or above the configured ceiling.
    #[error("invalid outbox claim batch size")]
    InvalidBatchSize,
    /// Lease duration is zero or above the configured ceiling.
    #[error("invalid outbox lease duration")]
    InvalidLeaseDuration,
    /// Maximum attempts is zero or above the configured ceiling.
    #[error("invalid outbox maximum attempts")]
    InvalidMaxAttempts,
    /// Retry backoff is zero or above the configured ceiling.
    #[error("invalid outbox retry backoff")]
    InvalidBackoff,
    /// Error class is empty, too long, or contains disallowed characters.
    #[error("invalid outbox error class")]
    InvalidErrorClass,
    /// Claim token is empty, too long, or contains disallowed characters.
    #[error("invalid outbox claim token")]
    InvalidClaimToken,
    /// The claim token no longer matches the event lease.
    #[error("outbox claim is no longer valid")]
    ClaimLost,
    /// Aggregate sequence exceeds the supported range.
    #[error("outbox aggregate sequence overflow")]
    SequenceOverflow,
}

/// Validated limits for one worker claim request.
///
/// Constructing this type is the only way to pass limits to the repository,
/// so an unbounded or zero value cannot reach SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboxClaimPolicy {
    batch_size: usize,
    lease_duration: Duration,
    max_attempts: u32,
}

impl OutboxClaimPolicy {
    /// Builds a validated claim policy.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidBatchSize`] when `batch_size` is zero or
    /// above [`MAX_BATCH_SIZE`], [`OutboxError::InvalidLeaseDuration`] when the
    /// lease is zero or above [`MAX_LEASE_DURATION`], and
    /// [`OutboxError::InvalidMaxAttempts`] when `max_attempts` is zero or
    /// above [`MAX_ATTEMPTS`].
    pub fn new(
        batch_size: usize,
        lease_duration: Duration,
        max_attempts: u32,
    ) -> Result<Self, OutboxError> {
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(OutboxError::InvalidBatchSize);
        }
        if lease_duration.is_zero() || lease_duration > MAX_LEASE_DURATION {
            return Err(OutboxError::InvalidLeaseDuration);
        }
        if max_attempts == 0 || max_attempts > MAX_ATTEMPTS {
            return Err(OutboxError::InvalidMaxAttempts);
        }
        Ok(Self {
            batch_size,
            lease_duration,
            max_attempts,
        })
    }

    /// Number of events to claim.
    #[must_use]
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Lease duration for the claimed events.
    #[must_use]
    pub fn lease_duration(&self) -> Duration {
        self.lease_duration
    }

    /// Delivery attempts permitted before an event is quarantined.
    #[must_use]
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

/// A claim token proves that the caller still owns the event lease.
///
/// The token is opaque to callers and is redacted from `Debug` output so it
/// cannot reach a log through an accidental formatting call.
#[derive(Clone, PartialEq, Eq)]
pub struct ClaimToken(String);

impl ClaimToken {
    /// Validates and wraps a claim token.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidClaimToken`] when the value is empty,
    /// longer than [`MAX_CLAIM_TOKEN_LEN`], or contains a character outside
    /// ASCII alphanumerics and `-`.
    pub fn new(value: impl Into<String>) -> Result<Self, OutboxError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_CLAIM_TOKEN_LEN
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(OutboxError::InvalidClaimToken);
        }
        Ok(Self(value))
    }

    /// Borrows the token for a bound SQL parameter.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ClaimToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ClaimToken(***)")
    }
}

/// A bounded, low-cardinality classification of a delivery failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxErrorClass(String);

impl OutboxErrorClass {
    /// Validates and wraps an error class.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidErrorClass`] when the value is empty,
    /// longer than [`MAX_ERROR_CLASS_LEN`], does not start with an ASCII
    /// lowercase letter, or contains a character outside ASCII lowercase
    /// letters, digits, `_`, `.`, and `-`.
    pub fn new(value: impl Into<String>) -> Result<Self, OutboxError> {
        let value = value.into();
        let mut bytes = value.bytes();
        let Some(first) = bytes.next() else {
            return Err(OutboxError::InvalidErrorClass);
        };
        if value.len() > MAX_ERROR_CLASS_LEN
            || !first.is_ascii_lowercase()
            || !bytes.all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'_' | b'.' | b'-')
            })
        {
            return Err(OutboxError::InvalidErrorClass);
        }
        Ok(Self(value))
    }

    /// Borrows the error class for a bound SQL parameter.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validates a retry backoff before it reaches SQL.
///
/// # Errors
///
/// Returns [`OutboxError::InvalidBackoff`] when `backoff` is zero or greater
/// than [`MAX_BACKOFF`].
pub fn validate_backoff(backoff: Duration) -> Result<(), OutboxError> {
    if backoff.is_zero() || backoff > MAX_BACKOFF {
        return Err(OutboxError::InvalidBackoff);
    }
    Ok(())
}

/// A durable outbox event.
///
/// `aggregate_sequence`, `claim_token`, and `claim_generation` are assigned by
/// PostgreSQL. A new event carries `None`/zero for those fields; a claimed
/// event carries the lease token that the worker must present for every state
/// transition.
#[derive(Debug, Clone)]
pub struct OutboxEvent {
    /// Stable event identifier.
    pub event_id: String,
    /// Aggregate family used for ordering.
    pub aggregate_type: String,
    /// Aggregate identifier used for ordering.
    pub aggregate_id: String,
    /// Versioned event name.
    pub event_name: String,
    /// Event schema version.
    pub event_version: u32,
    /// Owning organization.
    pub organization_id: OrganizationId,
    /// Optional branch.
    pub branch_id: Option<BranchId>,
    /// Time the domain event occurred.
    pub occurred_at: SystemTime,
    /// JSON payload.
    pub payload: String,
    /// Current delivery state.
    pub status: OutboxStatus,
    /// Earliest time the next attempt may run.
    pub available_at: SystemTime,
    /// Number of attempts already made.
    pub attempt_count: u32,
    /// Lease start time for a claimed event.
    pub locked_at: Option<SystemTime>,
    /// Publication time for a published event.
    pub published_at: Option<SystemTime>,
    /// Last bounded failure class.
    pub last_error_class: Option<String>,
    /// Deduplication key.
    pub deduplication_key: String,
    /// Persisted row schema version.
    pub schema_version: u32,
    /// Database-assigned sequence within the aggregate.
    pub aggregate_sequence: Option<u64>,
    /// Lease token assigned by the claim operation.
    pub claim_token: Option<ClaimToken>,
    /// Number of times the event has been claimed.
    pub claim_generation: u64,
}

impl OutboxEvent {
    /// Maximum accepted JSON payload size in bytes.
    pub const MAX_PAYLOAD_SIZE: usize = 65_536;

    /// Builds a new pending event.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::PayloadTooLarge`] when the payload exceeds
    /// [`OutboxEvent::MAX_PAYLOAD_SIZE`].
    /// Returns [`OutboxError::InvalidSchema`] when a mapped lifecycle event
    /// does not match its approved versioned payload schema.
    #[allow(
        clippy::too_many_arguments,
        reason = "constructor mirrors the persisted event columns before database assignment"
    )]
    pub fn new(
        event_id: String,
        aggregate_type: String,
        aggregate_id: String,
        event_name: String,
        event_version: u32,
        organization_id: OrganizationId,
        branch_id: Option<BranchId>,
        payload: String,
    ) -> Result<Self, OutboxError> {
        if payload.len() > Self::MAX_PAYLOAD_SIZE {
            return Err(OutboxError::PayloadTooLarge);
        }
        crate::validate_iam_payload(&event_name, event_version, &payload)?;

        let now = SystemTime::now();
        Ok(OutboxEvent {
            event_id: event_id.clone(),
            aggregate_type,
            aggregate_id,
            event_name,
            event_version,
            organization_id,
            branch_id,
            occurred_at: now,
            payload,
            status: OutboxStatus::Pending,
            available_at: now,
            attempt_count: 0,
            locked_at: None,
            published_at: None,
            last_error_class: None,
            deduplication_key: event_id,
            schema_version: event_version,
            aggregate_sequence: None,
            claim_token: None,
            claim_generation: 0,
        })
    }

    /// Returns `true` when this is a new client-created intent with no
    /// database-assigned delivery state.
    ///
    /// The repository rejects loaded or partially transitioned events on
    /// enqueue so a caller cannot forge a claim, publication, or backoff state.
    #[must_use]
    pub fn is_fresh(&self) -> bool {
        self.status == OutboxStatus::Pending
            && self.aggregate_sequence.is_none()
            && self.claim_token.is_none()
            && self.claim_generation == 0
            && self.attempt_count == 0
            && self.locked_at.is_none()
            && self.published_at.is_none()
            && self.last_error_class.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn organization() -> Result<OrganizationId, sitolo_domain::tenancy::TenancyError> {
        OrganizationId::new("org-1")
    }

    fn event() -> Result<OutboxEvent, Box<dyn Error>> {
        Ok(OutboxEvent::new(
            "evt-1".to_owned(),
            "Organization".to_owned(),
            "org-1".to_owned(),
            "iam.organization.created".to_owned(),
            1,
            organization()?,
            None,
            r#"{"name":"test"}"#.to_owned(),
        )?)
    }

    #[test]
    fn claim_policy_accepts_inclusive_bounds() -> Result<(), OutboxError> {
        let policy = OutboxClaimPolicy::new(MAX_BATCH_SIZE, MAX_LEASE_DURATION, MAX_ATTEMPTS)?;
        assert_eq!(policy.batch_size(), MAX_BATCH_SIZE);
        assert_eq!(policy.lease_duration(), MAX_LEASE_DURATION);
        assert_eq!(policy.max_attempts(), MAX_ATTEMPTS);
        Ok(())
    }

    #[test]
    fn claim_policy_rejects_zero_and_above_ceiling() {
        assert!(matches!(
            OutboxClaimPolicy::new(0, Duration::from_secs(1), 1),
            Err(OutboxError::InvalidBatchSize)
        ));
        assert!(matches!(
            OutboxClaimPolicy::new(MAX_BATCH_SIZE + 1, Duration::from_secs(1), 1),
            Err(OutboxError::InvalidBatchSize)
        ));
        assert!(matches!(
            OutboxClaimPolicy::new(1, Duration::ZERO, 1),
            Err(OutboxError::InvalidLeaseDuration)
        ));
        assert!(matches!(
            OutboxClaimPolicy::new(1, MAX_LEASE_DURATION + Duration::from_secs(1), 1),
            Err(OutboxError::InvalidLeaseDuration)
        ));
        assert!(matches!(
            OutboxClaimPolicy::new(1, Duration::from_secs(1), 0),
            Err(OutboxError::InvalidMaxAttempts)
        ));
        assert!(matches!(
            OutboxClaimPolicy::new(1, Duration::from_secs(1), MAX_ATTEMPTS + 1),
            Err(OutboxError::InvalidMaxAttempts)
        ));
    }

    #[test]
    fn error_class_rejects_unbounded_or_injectable_values() {
        assert!(OutboxErrorClass::new("http_5xx").is_ok());
        assert!(OutboxErrorClass::new("").is_err());
        assert!(OutboxErrorClass::new("HTTP_5XX").is_err());
        assert!(OutboxErrorClass::new("http 5xx").is_err());
        assert!(OutboxErrorClass::new("http\n5xx").is_err());
        assert!(OutboxErrorClass::new("x".repeat(MAX_ERROR_CLASS_LEN + 1)).is_err());
    }

    #[test]
    fn claim_token_is_bounded_and_redacted() -> Result<(), OutboxError> {
        let token = ClaimToken::new("8d3a4c1e-3f8f-4f37-9d0e-2a4c9e5b7a11")?;
        assert!(!format!("{token:?}").contains(token.as_str()));
        assert!(ClaimToken::new("").is_err());
        assert!(ClaimToken::new("has space").is_err());
        assert!(ClaimToken::new("x".repeat(MAX_CLAIM_TOKEN_LEN + 1)).is_err());
        Ok(())
    }

    #[test]
    fn backoff_is_bounded() {
        assert!(validate_backoff(Duration::from_secs(1)).is_ok());
        assert!(matches!(
            validate_backoff(Duration::ZERO),
            Err(OutboxError::InvalidBackoff)
        ));
        assert!(matches!(
            validate_backoff(MAX_BACKOFF + Duration::from_secs(1)),
            Err(OutboxError::InvalidBackoff)
        ));
    }

    #[test]
    fn new_event_has_no_database_assigned_state() -> Result<(), Box<dyn Error>> {
        let event = event()?;
        assert!(event.aggregate_sequence.is_none());
        assert!(event.claim_token.is_none());
        assert_eq!(event.claim_generation, 0);
        assert_eq!(event.status, OutboxStatus::Pending);
        assert!(event.is_fresh());
        Ok(())
    }

    #[test]
    fn transitioned_event_is_not_fresh() -> Result<(), Box<dyn Error>> {
        let mut claimed = event()?;
        claimed.claim_generation = 1;
        assert!(!claimed.is_fresh());

        let mut locked = event()?;
        locked.locked_at = Some(SystemTime::now());
        assert!(!locked.is_fresh());

        let mut published = event()?;
        published.published_at = Some(SystemTime::now());
        assert!(!published.is_fresh());

        let mut failed = event()?;
        failed.last_error_class = Some("http_5xx".to_owned());
        assert!(!failed.is_fresh());
        Ok(())
    }

    #[test]
    fn payload_validation() -> Result<(), Box<dyn Error>> {
        let oversized = "x".repeat(OutboxEvent::MAX_PAYLOAD_SIZE + 1);
        let invalid = OutboxEvent::new(
            "evt-2".to_owned(),
            "Organization".to_owned(),
            "org-1".to_owned(),
            "iam.organization.created".to_owned(),
            1,
            organization()?,
            None,
            oversized,
        );
        assert!(matches!(invalid, Err(OutboxError::PayloadTooLarge)));
        Ok(())
    }

    #[test]
    fn approved_lifecycle_payload_rejects_unapproved_sensitive_fields() -> Result<(), Box<dyn Error>>
    {
        let result = OutboxEvent::new(
            "evt-pending".to_owned(),
            "membership".to_owned(),
            "membership-1".to_owned(),
            "iam.membership.pending_acceptance".to_owned(),
            1,
            organization()?,
            None,
            r#"{"membership_id":"membership-1","from_state":"INVITED","to_state":"PENDING_ACCEPTANCE","state_version":2,"email":"person@example.test"}"#.to_owned(),
        );

        assert!(matches!(result, Err(OutboxError::InvalidSchema)));
        Ok(())
    }

    #[test]
    fn terminal_statuses_are_not_claimable() {
        assert!(!OutboxStatus::Pending.is_terminal());
        assert!(!OutboxStatus::Claimed.is_terminal());
        assert!(OutboxStatus::Published.is_terminal());
        assert!(OutboxStatus::Quarantined.is_terminal());
    }
}
