//! PostgreSQL outbox persistence.
//!
//! Implements durable outbox event storage with claim/lease/retry/quarantine
//! semantics. Uses narrow SQL functions for state transitions to enforce
//! least-privilege worker authority.

use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;

use sitolo_events::{OutboxError, OutboxEvent, OutboxStatus};

/// Outbox persistence error.
#[derive(Debug, Error)]
pub enum OutboxPersistenceError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("event not found")]
    NotFound,
    #[error("invalid state transition")]
    InvalidStateTransition,
    #[error("payload too large")]
    PayloadTooLarge,
}

/// Outbox writer port for transactional persistence.
#[async_trait]
pub trait OutboxWriter: Send + Sync {
    /// Enqueues an outbox event within the same transaction as the business mutation.
    async fn enqueue(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &OutboxEvent,
    ) -> Result<(), OutboxPersistenceError>;
}

/// Outbox reader port for worker claim/processing.
#[async_trait]
pub trait OutboxReader: Send + Sync {
    /// Claims a batch of PENDING events with lease.
    ///
    /// Uses row locks and SKIP LOCKED for concurrent worker safety.
    async fn claim_batch(
        &self,
        batch_size: usize,
        lease_duration: Duration,
    ) -> Result<Vec<OutboxEvent>, OutboxPersistenceError>;

    /// Marks an event as successfully published.
    async fn mark_published(&self, event_id: &str) -> Result<(), OutboxPersistenceError>;

    /// Releases an event for retry with backoff.
    async fn release_for_retry(
        &self,
        event_id: &str,
        error_class: &str,
        backoff: Duration,
    ) -> Result<(), OutboxPersistenceError>;

    /// Quarantines an event (terminal failure).
    async fn quarantine(
        &self,
        event_id: &str,
        error_class: &str,
    ) -> Result<(), OutboxPersistenceError>;

    /// Recovers stale claims (lease expired).
    async fn recover_stale_claims(&self, lease_timeout: Duration) -> Result<u32, OutboxPersistenceError>;
}

/// PostgreSQL outbox repository implementation.
pub struct PostgresOutboxRepository {
    pool: PgPool,
}

impl PostgresOutboxRepository {
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
            return Err(OutboxPersistenceError::PayloadTooLarge);
        }

        let branch_id = event.branch_id.as_ref().map(|b| b.to_string());
        let payload_json: serde_json::Value = serde_json::from_str(&event.payload)
            .map_err(|_| OutboxPersistenceError::Database(sqlx::Error::Decode("Invalid JSON".into())))?;

        // Get or compute aggregate sequence for ordering
        let aggregate_sequence: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(aggregate_sequence), 0) + 1
            FROM outbox_events
            WHERE aggregate_type = $1 AND aggregate_id = $2
            "#,
        )
        .bind(&event.aggregate_type)
        .bind(&event.aggregate_id)
        .fetch_one(&mut **tx)
        .await
        .unwrap_or(1);

        sqlx::query(
            r#"
            INSERT INTO outbox_events (
                event_id, aggregate_type, aggregate_id, event_name, event_version,
                organization_id, branch_id, occurred_at, payload,
                status, available_at, attempt_count, locked_at, published_at,
                last_error_class, deduplication_key, schema_version, aggregate_sequence
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#,
        )
        .bind(&event.event_id)
        .bind(&event.aggregate_type)
        .bind(&event.aggregate_id)
        .bind(&event.event_name)
        .bind(event.event_version as i32)
        .bind(event.organization_id.to_string())
        .bind(branch_id)
        .bind(event.occurred_at)
        .bind(&payload_json)
        .bind(event.status.as_str())
        .bind(event.available_at)
        .bind(event.attempt_count as i32)
        .bind(event.locked_at)
        .bind(event.published_at)
        .bind(&event.last_error_class)
        .bind(&event.deduplication_key)
        .bind(event.schema_version as i32)
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
        batch_size: usize,
        lease_duration: Duration,
    ) -> Result<Vec<OutboxEvent>, OutboxPersistenceError> {
        let lease_interval = chrono::Duration::from_std(lease_duration)
            .map_err(|_| OutboxPersistenceError::Database(sqlx::Error::Decode("Invalid duration".into())))?;

        let rows = sqlx::query_as::<_, OutboxEventRow>(
            r#"
            SELECT * FROM outbox_claim_events($1, $2)
            "#,
        )
        .bind(batch_size as i32)
        .bind(lease_interval)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn mark_published(&self, event_id: &str) -> Result<(), OutboxPersistenceError> {
        sqlx::query("SELECT outbox_mark_published($1)")
            .bind(event_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn release_for_retry(
        &self,
        event_id: &str,
        error_class: &str,
        backoff: Duration,
    ) -> Result<(), OutboxPersistenceError> {
        let backoff_interval = chrono::Duration::from_std(backoff)
            .map_err(|_| OutboxPersistenceError::Database(sqlx::Error::Decode("Invalid duration".into())))?;

        sqlx::query("SELECT outbox_release_for_retry($1, $2, $3)")
            .bind(event_id)
            .bind(error_class)
            .bind(backoff_interval)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn quarantine(
        &self,
        event_id: &str,
        error_class: &str,
    ) -> Result<(), OutboxPersistenceError> {
        sqlx::query("SELECT outbox_quarantine($1, $2)")
            .bind(event_id)
            .bind(error_class)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn recover_stale_claims(&self, lease_timeout: Duration) -> Result<u32, OutboxPersistenceError> {
        let timeout_interval = chrono::Duration::from_std(lease_timeout)
            .map_err(|_| OutboxPersistenceError::Database(sqlx::Error::Decode("Invalid duration".into())))?;

        let count: i32 = sqlx::query_scalar("SELECT outbox_recover_stale_claims($1)")
            .bind(timeout_interval)
            .fetch_one(&self.pool)
            .await?;

        Ok(count as u32)
    }
}

/// Database row representation.
#[derive(Debug, sqlx::FromRow)]
struct OutboxEventRow {
    event_id: String,
    aggregate_type: String,
    aggregate_id: String,
    event_name: String,
    event_version: i32,
    organization_id: String,
    branch_id: Option<String>,
    occurred_at: SystemTime,
    payload: serde_json::Value,
    status: String,
    available_at: SystemTime,
    attempt_count: i32,
    locked_at: Option<SystemTime>,
    published_at: Option<SystemTime>,
    last_error_class: Option<String>,
    deduplication_key: String,
    schema_version: i32,
    aggregate_sequence: Option<i64>,
    created_at: SystemTime,
}

impl From<OutboxEventRow> for OutboxEvent {
    fn from(row: OutboxEventRow) -> Self {
        let status = match row.status.as_str() {
            "PENDING" => OutboxStatus::Pending,
            "CLAIMED" => OutboxStatus::Claimed,
            "PUBLISHED" => OutboxStatus::Published,
            "QUARANTINED" => OutboxStatus::Quarantined,
            _ => OutboxStatus::Pending,
        };

        OutboxEvent {
            event_id: row.event_id,
            aggregate_type: row.aggregate_type,
            aggregate_id: row.aggregate_id,
            event_name: row.event_name,
            event_version: row.event_version as u32,
            organization_id: sitolo_domain::tenancy::OrganizationId::new(&row.organization_id).unwrap(),
            branch_id: row.branch_id.and_then(|b| sitolo_domain::tenancy::BranchId::new(&b).ok()),
            occurred_at: row.occurred_at,
            payload: serde_json::to_string(&row.payload).unwrap_or_default(),
            status,
            available_at: row.available_at,
            attempt_count: row.attempt_count as u32,
            locked_at: row.locked_at,
            published_at: row.published_at,
            last_error_class: row.last_error_class,
            deduplication_key: row.deduplication_key,
            schema_version: row.schema_version as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_domain::tenancy::OrganizationId;

    #[test]
    fn outbox_event_payload_validation() {
        let valid = OutboxEvent::new(
            "evt-1".into(),
            "Organization".into(),
            "org-1".into(),
            "iam.organization.created".into(),
            1,
            OrganizationId::new("org-1").unwrap(),
            None,
            r#"{"name":"test"}"#.into(),
        );
        assert!(valid.is_ok());

        let oversized = "x".repeat(OutboxEvent::MAX_PAYLOAD_SIZE + 1);
        let invalid = OutboxEvent::new(
            "evt-2".into(),
            "Organization".into(),
            "org-1".into(),
            "iam.organization.created".into(),
            1,
            OrganizationId::new("org-1").unwrap(),
            None,
            oversized,
        );
        assert!(matches!(invalid, Err(OutboxError::PayloadTooLarge)));
    }
}
