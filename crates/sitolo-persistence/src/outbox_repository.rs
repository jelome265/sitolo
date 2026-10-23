//! PostgreSQL outbox persistence.
//!
//! Implements durable outbox event storage with claim/lease/retry/quarantine
//! semantics. Aggregate ordering is assigned by PostgreSQL under a row lock;
//! every state transition requires the claim token issued by the claim
//! function. Uses narrow SQL functions so the worker role never receives
//! general table write authority.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx_core::error::Error;
use sqlx_core::from_row::FromRow;
use sqlx_core::query::query;
use sqlx_core::query_as::query_as;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::row::Row;
use sqlx_core::transaction::Transaction;
use sqlx_postgres::{PgPool, PgRow, Postgres};
use thiserror::Error;

use sitolo_events::{
    ClaimToken, MAX_LEASE_DURATION, OutboxClaimPolicy, OutboxError, OutboxErrorClass, OutboxEvent,
    OutboxStatus, validate_backoff, validate_iam_payload,
};

use crate::postgres_time::{from_utc, to_utc};

/// Outbox persistence error.
#[derive(Debug, Error)]
pub enum OutboxPersistenceError {
    /// PostgreSQL rejected or could not execute the operation.
    #[error("database error: {0}")]
    Database(#[from] Error),
    /// The caller supplied an invalid outbox request.
    #[error("invalid outbox request: {0}")]
    InvalidRequest(#[from] OutboxError),
    /// The claim token does not own the event lease.
    ///
    /// This deliberately covers both "unknown event" and "token mismatch" so
    /// an unprivileged worker cannot enumerate event identifiers.
    #[error("outbox claim is no longer valid")]
    ClaimLost,
    /// A duration could not be represented by PostgreSQL.
    #[error("duration outside the supported range: {0}")]
    InvalidDuration(#[source] chrono::OutOfRangeError),
    /// A persisted numeric value exceeds the Rust representation.
    #[error("persisted value out of range for {field}: {source}")]
    NumericRange {
        /// Column or field name.
        field: &'static str,
        /// Conversion failure.
        #[source]
        source: std::num::TryFromIntError,
    },
    /// The payload is not valid JSON.
    #[error("invalid JSON payload: {0}")]
    InvalidJson(#[from] serde_json::Error),
    /// A persisted row violates the outbox invariants.
    #[error("invalid persisted outbox event: {0}")]
    InvalidRow(&'static str),
    /// A timestamp cannot be represented by PostgreSQL.
    #[error("timestamp outside the supported range")]
    InvalidTimestamp,
}

/// Outbox writer port for transactional persistence.
#[async_trait]
pub trait OutboxWriter: Send + Sync {
    /// Enqueues an outbox event within the same transaction as the business
    /// mutation.
    ///
    /// # Errors
    ///
    /// Returns an error when the event is not a new pending event, the payload
    /// is too large or not JSON, or PostgreSQL rejects the insert.
    async fn enqueue(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &OutboxEvent,
    ) -> Result<(), OutboxPersistenceError>;
}

/// Outbox reader port for worker claim/processing.
#[async_trait]
pub trait OutboxReader: Send + Sync {
    /// Claims a batch of PENDING events with a lease.
    ///
    /// Uses row locks and `SKIP LOCKED` for concurrent worker safety. An event
    /// is eligible only when every earlier event for the same aggregate is
    /// published, so delivery order matches the database-assigned aggregate
    /// sequence. Unrelated aggregates proceed in parallel. The returned events
    /// carry the claim token required by every subsequent state transition.
    ///
    /// # Errors
    ///
    /// Returns an error when the claim policy cannot be bound, PostgreSQL
    /// rejects the claim, or a persisted row violates an invariant.
    async fn claim_batch(
        &self,
        policy: OutboxClaimPolicy,
    ) -> Result<Vec<OutboxEvent>, OutboxPersistenceError>;

    /// Marks an event as successfully published.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxPersistenceError::ClaimLost`] when the event is no
    /// longer claimed under `claim_token`.
    async fn mark_published(
        &self,
        event_id: &str,
        claim_token: &ClaimToken,
    ) -> Result<(), OutboxPersistenceError>;

    /// Releases an event for retry with backoff.
    ///
    /// # Errors
    ///
    /// Returns an error when the backoff is outside the configured bounds or
    /// the claim token no longer owns the event.
    async fn release_for_retry(
        &self,
        event_id: &str,
        claim_token: &ClaimToken,
        error_class: &OutboxErrorClass,
        backoff: Duration,
    ) -> Result<(), OutboxPersistenceError>;

    /// Quarantines an event after a terminal failure.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxPersistenceError::ClaimLost`] when the event is no
    /// longer claimed under `claim_token`.
    async fn quarantine(
        &self,
        event_id: &str,
        claim_token: &ClaimToken,
        error_class: &OutboxErrorClass,
    ) -> Result<(), OutboxPersistenceError>;

    /// Recovers claims whose lease expired.
    ///
    /// Recovery clears the old claim token and increments the claim
    /// generation, so a stalled worker cannot publish after recovery.
    ///
    /// # Errors
    ///
    /// Returns an error when `lease_timeout` is zero or above
    /// [`MAX_LEASE_DURATION`], or PostgreSQL rejects the update.
    async fn recover_stale_claims(
        &self,
        lease_timeout: Duration,
    ) -> Result<u32, OutboxPersistenceError>;
}

/// PostgreSQL outbox repository implementation.
pub struct PostgresOutboxRepository {
    pool: PgPool,
}

impl PostgresOutboxRepository {
    /// Creates a repository over an existing connection pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OutboxWriter for PostgresOutboxRepository {
    async fn enqueue(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &OutboxEvent,
    ) -> Result<(), OutboxPersistenceError> {
        if event.payload.len() > OutboxEvent::MAX_PAYLOAD_SIZE {
            return Err(OutboxError::PayloadTooLarge.into());
        }
        // `OutboxEvent` fields are public, so constructor validation is not a
        // persistence-boundary guarantee. Revalidate the approved lifecycle
        // schemas before sequence allocation or any other database write.
        validate_iam_payload(&event.event_name, event.event_version, &event.payload)?;
        // Enqueue accepts only a fresh event. Database-assigned state must not
        // be supplied by the caller, and a loaded event is a duplicate rather
        // than a new intent.
        if !event.is_fresh() {
            return Err(OutboxError::InvalidSchema.into());
        }

        let payload_json: serde_json::Value = serde_json::from_str(&event.payload)?;
        let occurred_at =
            to_utc(event.occurred_at).ok_or(OutboxPersistenceError::InvalidTimestamp)?;
        let available_at =
            to_utc(event.available_at).ok_or(OutboxPersistenceError::InvalidTimestamp)?;
        let event_version = i32::try_from(event.event_version).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "event_version",
                source,
            }
        })?;
        let attempt_count = i32::try_from(event.attempt_count).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "attempt_count",
                source,
            }
        })?;
        let schema_version = i32::try_from(event.schema_version).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "schema_version",
                source,
            }
        })?;
        let locked_at = match event.locked_at {
            Some(value) => Some(to_utc(value).ok_or(OutboxPersistenceError::InvalidTimestamp)?),
            None => None,
        };
        let published_at = match event.published_at {
            Some(value) => Some(to_utc(value).ok_or(OutboxPersistenceError::InvalidTimestamp)?),
            None => None,
        };
        let last_error_class = event
            .last_error_class
            .as_deref()
            .map(OutboxErrorClass::new)
            .transpose()?;

        // Assign the aggregate sequence under a row lock. `ON CONFLICT DO
        // UPDATE` serialises concurrent writers for the same aggregate, so
        // there is no MAX()+1 race.
        let aggregate_sequence: i64 = query_scalar(
            r#"
            INSERT INTO outbox_aggregate_sequences (
                aggregate_type, aggregate_id, next_sequence
            )
            VALUES ($1, $2, 2)
            ON CONFLICT (aggregate_type, aggregate_id)
            DO UPDATE SET next_sequence = outbox_aggregate_sequences.next_sequence + 1
            RETURNING next_sequence - 1
            "#,
        )
        .bind(&event.aggregate_type)
        .bind(&event.aggregate_id)
        .fetch_one(&mut **tx)
        .await?;

        if aggregate_sequence <= 0 {
            return Err(OutboxPersistenceError::InvalidRow("aggregate_sequence"));
        }

        query(
            r#"
            INSERT INTO outbox_events (
                event_id, aggregate_type, aggregate_id, event_name, event_version,
                organization_id, branch_id, occurred_at, payload,
                status, available_at, attempt_count, locked_at, published_at,
                last_error_class, deduplication_key, schema_version,
                aggregate_sequence
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15, $16, $17, $18
            )
            "#,
        )
        .bind(&event.event_id)
        .bind(&event.aggregate_type)
        .bind(&event.aggregate_id)
        .bind(&event.event_name)
        .bind(event_version)
        .bind(event.organization_id.to_string())
        .bind(event.branch_id.as_ref().map(ToString::to_string))
        .bind(occurred_at)
        .bind(&payload_json)
        .bind(event.status.as_str())
        .bind(available_at)
        .bind(attempt_count)
        .bind(locked_at)
        .bind(published_at)
        .bind(last_error_class.as_ref().map(OutboxErrorClass::as_str))
        .bind(&event.deduplication_key)
        .bind(schema_version)
        .bind(aggregate_sequence)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl OutboxReader for PostgresOutboxRepository {
    async fn claim_batch(
        &self,
        policy: OutboxClaimPolicy,
    ) -> Result<Vec<OutboxEvent>, OutboxPersistenceError> {
        let batch_size = i32::try_from(policy.batch_size()).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "batch_size",
                source,
            }
        })?;
        let max_attempts = i32::try_from(policy.max_attempts()).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "max_attempts",
                source,
            }
        })?;
        let lease_interval = chrono::Duration::from_std(policy.lease_duration())
            .map_err(OutboxPersistenceError::InvalidDuration)?;

        let rows = query_as::<_, OutboxEventRow>(
            r#"
            SELECT
                event_id, aggregate_type, aggregate_id, event_name, event_version,
                organization_id, branch_id, occurred_at, payload::text AS payload,
                status, available_at, attempt_count, locked_at, published_at,
                last_error_class, deduplication_key, schema_version,
                aggregate_sequence, claim_token, claim_generation
            FROM outbox_claim_events($1, $2, $3)
            "#,
        )
        .bind(batch_size)
        .bind(lease_interval)
        .bind(max_attempts)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(OutboxEvent::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn mark_published(
        &self,
        event_id: &str,
        claim_token: &ClaimToken,
    ) -> Result<(), OutboxPersistenceError> {
        let updated: bool = query_scalar("SELECT outbox_mark_published($1, $2)")
            .bind(event_id)
            .bind(claim_token.as_str())
            .fetch_one(&self.pool)
            .await?;
        if updated {
            Ok(())
        } else {
            Err(OutboxPersistenceError::ClaimLost)
        }
    }

    async fn release_for_retry(
        &self,
        event_id: &str,
        claim_token: &ClaimToken,
        error_class: &OutboxErrorClass,
        backoff: Duration,
    ) -> Result<(), OutboxPersistenceError> {
        validate_backoff(backoff)?;
        let backoff_interval =
            chrono::Duration::from_std(backoff).map_err(OutboxPersistenceError::InvalidDuration)?;

        let updated: bool = query_scalar("SELECT outbox_release_for_retry($1, $2, $3, $4)")
            .bind(event_id)
            .bind(claim_token.as_str())
            .bind(error_class.as_str())
            .bind(backoff_interval)
            .fetch_one(&self.pool)
            .await?;
        if updated {
            Ok(())
        } else {
            Err(OutboxPersistenceError::ClaimLost)
        }
    }

    async fn quarantine(
        &self,
        event_id: &str,
        claim_token: &ClaimToken,
        error_class: &OutboxErrorClass,
    ) -> Result<(), OutboxPersistenceError> {
        let updated: bool = query_scalar("SELECT outbox_quarantine($1, $2, $3)")
            .bind(event_id)
            .bind(claim_token.as_str())
            .bind(error_class.as_str())
            .fetch_one(&self.pool)
            .await?;
        if updated {
            Ok(())
        } else {
            Err(OutboxPersistenceError::ClaimLost)
        }
    }

    async fn recover_stale_claims(
        &self,
        lease_timeout: Duration,
    ) -> Result<u32, OutboxPersistenceError> {
        if lease_timeout.is_zero() || lease_timeout > MAX_LEASE_DURATION {
            return Err(OutboxError::InvalidLeaseDuration.into());
        }
        let timeout_interval = chrono::Duration::from_std(lease_timeout)
            .map_err(OutboxPersistenceError::InvalidDuration)?;

        let recovered: i32 = query_scalar("SELECT outbox_recover_stale_claims($1)")
            .bind(timeout_interval)
            .fetch_one(&self.pool)
            .await?;
        u32::try_from(recovered).map_err(|source| OutboxPersistenceError::NumericRange {
            field: "recovered_count",
            source,
        })
    }
}

/// Database row representation.
#[derive(Debug)]
struct OutboxEventRow {
    event_id: String,
    aggregate_type: String,
    aggregate_id: String,
    event_name: String,
    event_version: i32,
    organization_id: String,
    branch_id: Option<String>,
    occurred_at: DateTime<Utc>,
    payload: String,
    status: String,
    available_at: DateTime<Utc>,
    attempt_count: i32,
    locked_at: Option<DateTime<Utc>>,
    published_at: Option<DateTime<Utc>>,
    last_error_class: Option<String>,
    deduplication_key: String,
    schema_version: i32,
    aggregate_sequence: i64,
    claim_token: Option<String>,
    claim_generation: i64,
}

impl<'r> FromRow<'r, PgRow> for OutboxEventRow {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        Ok(Self {
            event_id: row.try_get("event_id")?,
            aggregate_type: row.try_get("aggregate_type")?,
            aggregate_id: row.try_get("aggregate_id")?,
            event_name: row.try_get("event_name")?,
            event_version: row.try_get("event_version")?,
            organization_id: row.try_get("organization_id")?,
            branch_id: row.try_get("branch_id")?,
            occurred_at: row.try_get("occurred_at")?,
            payload: row.try_get("payload")?,
            status: row.try_get("status")?,
            available_at: row.try_get("available_at")?,
            attempt_count: row.try_get("attempt_count")?,
            locked_at: row.try_get("locked_at")?,
            published_at: row.try_get("published_at")?,
            last_error_class: row.try_get("last_error_class")?,
            deduplication_key: row.try_get("deduplication_key")?,
            schema_version: row.try_get("schema_version")?,
            aggregate_sequence: row.try_get("aggregate_sequence")?,
            claim_token: row.try_get("claim_token")?,
            claim_generation: row.try_get("claim_generation")?,
        })
    }
}

impl TryFrom<OutboxEventRow> for OutboxEvent {
    type Error = OutboxPersistenceError;

    fn try_from(row: OutboxEventRow) -> Result<Self, Self::Error> {
        let status = match row.status.as_str() {
            "PENDING" => OutboxStatus::Pending,
            "CLAIMED" => OutboxStatus::Claimed,
            "PUBLISHED" => OutboxStatus::Published,
            "QUARANTINED" => OutboxStatus::Quarantined,
            _ => return Err(OutboxPersistenceError::InvalidRow("status")),
        };
        let event_version = u32::try_from(row.event_version).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "event_version",
                source,
            }
        })?;
        let attempt_count = u32::try_from(row.attempt_count).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "attempt_count",
                source,
            }
        })?;
        let schema_version = u32::try_from(row.schema_version).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "schema_version",
                source,
            }
        })?;
        let aggregate_sequence = u64::try_from(row.aggregate_sequence).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "aggregate_sequence",
                source,
            }
        })?;
        if aggregate_sequence == 0 {
            return Err(OutboxPersistenceError::InvalidRow("aggregate_sequence"));
        }
        let claim_generation = u64::try_from(row.claim_generation).map_err(|source| {
            OutboxPersistenceError::NumericRange {
                field: "claim_generation",
                source,
            }
        })?;
        let claim_token = row
            .claim_token
            .map(ClaimToken::new)
            .transpose()
            .map_err(|_| OutboxPersistenceError::InvalidRow("claim_token"))?;
        if matches!(status, OutboxStatus::Claimed) && claim_token.is_none() {
            return Err(OutboxPersistenceError::InvalidRow("claim_token"));
        }
        if !matches!(status, OutboxStatus::Claimed) && claim_token.is_some() {
            return Err(OutboxPersistenceError::InvalidRow(
                "claim_token_state_mismatch",
            ));
        }
        if matches!(status, OutboxStatus::Claimed) && row.locked_at.is_none() {
            return Err(OutboxPersistenceError::InvalidRow("locked_at"));
        }
        if matches!(status, OutboxStatus::Published) && row.published_at.is_none() {
            return Err(OutboxPersistenceError::InvalidRow("published_at"));
        }
        let organization_id = sitolo_domain::tenancy::OrganizationId::new(&row.organization_id)
            .map_err(|_| OutboxPersistenceError::InvalidRow("organization_id"))?;
        let branch_id = row
            .branch_id
            .map(|value| {
                sitolo_domain::tenancy::BranchId::new(&value)
                    .map_err(|_| OutboxPersistenceError::InvalidRow("branch_id"))
            })
            .transpose()?;
        let occurred_at =
            from_utc(row.occurred_at).ok_or(OutboxPersistenceError::InvalidTimestamp)?;
        let available_at =
            from_utc(row.available_at).ok_or(OutboxPersistenceError::InvalidTimestamp)?;
        let locked_at = match row.locked_at {
            Some(value) => Some(from_utc(value).ok_or(OutboxPersistenceError::InvalidTimestamp)?),
            None => None,
        };
        let published_at = match row.published_at {
            Some(value) => Some(from_utc(value).ok_or(OutboxPersistenceError::InvalidTimestamp)?),
            None => None,
        };

        Ok(OutboxEvent {
            event_id: row.event_id,
            aggregate_type: row.aggregate_type,
            aggregate_id: row.aggregate_id,
            event_name: row.event_name,
            event_version,
            organization_id,
            branch_id,
            occurred_at,
            payload: row.payload,
            status,
            available_at,
            attempt_count,
            locked_at,
            published_at,
            last_error_class: row.last_error_class,
            deduplication_key: row.deduplication_key,
            schema_version,
            aggregate_sequence: Some(aggregate_sequence),
            claim_token,
            claim_generation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> OutboxEventRow {
        OutboxEventRow {
            event_id: "evt-1".to_owned(),
            aggregate_type: "Organization".to_owned(),
            aggregate_id: "org-1".to_owned(),
            event_name: "iam.organization.created".to_owned(),
            event_version: 1,
            organization_id: "org-1".to_owned(),
            branch_id: None,
            occurred_at: Utc::now(),
            payload: r#"{"name":"test"}"#.to_owned(),
            status: "CLAIMED".to_owned(),
            available_at: Utc::now(),
            attempt_count: 1,
            locked_at: Some(Utc::now()),
            published_at: None,
            last_error_class: None,
            deduplication_key: "evt-1".to_owned(),
            schema_version: 1,
            aggregate_sequence: 1,
            claim_token: Some("8d3a4c1e-3f8f-4f37-9d0e-2a4c9e5b7a11".to_owned()),
            claim_generation: 1,
        }
    }

    #[test]
    fn claimed_row_requires_claim_token() {
        let mut value = row();
        value.claim_token = None;
        assert!(matches!(
            OutboxEvent::try_from(value),
            Err(OutboxPersistenceError::InvalidRow("claim_token"))
        ));
    }

    #[test]
    fn unclaimed_row_rejects_a_claim_token() {
        let mut value = row();
        value.status = "PENDING".to_owned();
        value.locked_at = None;
        assert!(matches!(
            OutboxEvent::try_from(value),
            Err(OutboxPersistenceError::InvalidRow(
                "claim_token_state_mismatch"
            ))
        ));
    }

    #[test]
    fn claimed_row_requires_lease_time() {
        let mut value = row();
        value.locked_at = None;
        assert!(matches!(
            OutboxEvent::try_from(value),
            Err(OutboxPersistenceError::InvalidRow("locked_at"))
        ));
    }

    #[test]
    fn published_row_requires_publication_time() {
        let mut value = row();
        value.status = "PUBLISHED".to_owned();
        value.claim_token = None;
        value.locked_at = None;
        assert!(matches!(
            OutboxEvent::try_from(value),
            Err(OutboxPersistenceError::InvalidRow("published_at"))
        ));
    }

    #[test]
    fn row_rejects_zero_aggregate_sequence() {
        let mut value = row();
        value.aggregate_sequence = 0;
        assert!(matches!(
            OutboxEvent::try_from(value),
            Err(OutboxPersistenceError::InvalidRow("aggregate_sequence"))
        ));
    }

    #[test]
    fn row_rejects_unknown_status() {
        let mut value = row();
        value.status = "DONE".to_owned();
        assert!(matches!(
            OutboxEvent::try_from(value),
            Err(OutboxPersistenceError::InvalidRow("status"))
        ));
    }

    #[test]
    fn row_preserves_database_sequence_and_generation() -> Result<(), Box<dyn std::error::Error>> {
        let event = OutboxEvent::try_from(row())?;
        assert_eq!(event.aggregate_sequence, Some(1));
        assert_eq!(event.claim_generation, 1);
        assert!(event.claim_token.is_some());
        Ok(())
    }

    #[test]
    fn valid_organization_is_required() {
        let mut value = row();
        value.organization_id = String::new();
        assert!(matches!(
            OutboxEvent::try_from(value),
            Err(OutboxPersistenceError::InvalidRow("organization_id"))
        ));
    }
}
