//! Transactional durable inbox for at-least-once message consumers.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx_core::{
    error::Error, query::query, query_scalar::query_scalar, row::Row, transaction::Transaction,
};
use sqlx_postgres::{PgRow, Postgres};
use thiserror::Error as ThisError;

const MAX_INBOX_PAYLOAD_BYTES: usize = 256 * 1024;

/// Immutable identity and payload metadata for one validated integration event.
#[derive(Debug, Clone, Copy)]
pub struct ConsumerInboxRecord<'a> {
    /// Stable consumer identity that owns this idempotency namespace.
    pub consumer_name: &'a str,
    /// Producer-assigned event identifier.
    pub event_id: &'a str,
    /// Canonical event name.
    pub event_name: &'a str,
    /// Schema version of the event payload.
    pub event_version: i32,
    /// Tenant that owns the event.
    pub organization_id: &'a str,
    /// Optional branch scope of the event.
    pub branch_id: Option<&'a str>,
    /// Aggregate family the event describes.
    pub aggregate_type: &'a str,
    /// Aggregate instance the event describes.
    pub aggregate_id: &'a str,
    /// Monotonic per-aggregate event sequence.
    pub aggregate_sequence: i64,
    /// Time the producer recorded the event.
    pub occurred_at: DateTime<Utc>,
    /// Serialized event payload used for duplicate-content comparison.
    pub payload: &'a [u8],
}

/// Inbox persistence failure. Details are deliberately not retained in logs.
#[derive(Debug, ThisError)]
pub enum ConsumerInboxError {
    /// The database rejected or could not complete the inbox operation.
    #[error("consumer inbox database operation failed")]
    Database(#[from] Error),
    /// The supplied record failed local validation.
    #[error("consumer inbox record is invalid")]
    InvalidRecord,
    /// The event identifier was reused with different content.
    #[error("event identifier was reused with different content")]
    EventIdentityConflict,
    /// A persisted row violated the inbox invariants.
    #[error("persisted consumer inbox row violates its invariant")]
    InvalidPersistedRecord,
}

/// Transactional consumer inbox port.
#[async_trait]
pub trait ConsumerInboxWriter: Send + Sync {
    /// Inserts an invisible, uncommitted idempotency marker.
    ///
    /// The consumer must run this in the same transaction as its business
    /// effect, then commit both together. A `false` result is a previously
    /// committed duplicate with identical event identity and content. The
    /// caller may acknowledge that duplicate after committing the no-op
    /// transaction. If processing fails, dropping/rolling back the transaction
    /// removes the marker and permits a safe redelivery.
    async fn begin_processing(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: ConsumerInboxRecord<'_>,
    ) -> Result<bool, ConsumerInboxError>;
}

/// PostgreSQL inbox implementation using a unique consumer/event key.
#[derive(Debug, Default, Clone, Copy)]
pub struct PostgresConsumerInbox;

#[async_trait]
impl ConsumerInboxWriter for PostgresConsumerInbox {
    async fn begin_processing(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: ConsumerInboxRecord<'_>,
    ) -> Result<bool, ConsumerInboxError> {
        if !valid_component(event.consumer_name, 64)
            || !valid_component(event.event_id, 128)
            || !valid_component(event.event_name, 128)
            || event.event_version <= 0
            || !valid_component(event.organization_id, 128)
            || event
                .branch_id
                .is_some_and(|branch| !valid_component(branch, 128))
            || !valid_component(event.aggregate_type, 128)
            || !valid_component(event.aggregate_id, 128)
            || event.aggregate_sequence <= 0
            || event.payload.is_empty()
            || event.payload.len() > MAX_INBOX_PAYLOAD_BYTES
        {
            return Err(ConsumerInboxError::InvalidRecord);
        }

        // RLS scopes inbox access to the validated event tenant for this
        // transaction only. This setting cannot escape the transaction.
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(event.organization_id)
            .execute(&mut **tx)
            .await?;

        let payload_hash = Sha256::digest(event.payload).to_vec();
        let inserted: Option<String> = query_scalar(
            "INSERT INTO consumer_inbox \
             (consumer_name, event_id, event_name, event_version, organization_id, branch_id, \
              aggregate_type, aggregate_id, aggregate_sequence, occurred_at, payload_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
             ON CONFLICT (consumer_name, event_id) DO NOTHING RETURNING event_id",
        )
        .bind(event.consumer_name)
        .bind(event.event_id)
        .bind(event.event_name)
        .bind(event.event_version)
        .bind(event.organization_id)
        .bind(event.branch_id)
        .bind(event.aggregate_type)
        .bind(event.aggregate_id)
        .bind(event.aggregate_sequence)
        .bind(event.occurred_at)
        .bind(&payload_hash)
        .fetch_optional(&mut **tx)
        .await?;
        if inserted.is_some() {
            return Ok(true);
        }

        let existing: Option<PgRow> = query(
            "SELECT event_name, event_version, organization_id, branch_id, aggregate_type, \
             aggregate_id, aggregate_sequence, occurred_at, payload_sha256 \
             FROM consumer_inbox WHERE consumer_name = $1 AND event_id = $2",
        )
        .bind(event.consumer_name)
        .bind(event.event_id)
        .fetch_optional(&mut **tx)
        .await?;
        let Some(row) = existing else {
            return Err(ConsumerInboxError::InvalidPersistedRecord);
        };
        let same = row.try_get::<String, _>("event_name")? == event.event_name
            && row.try_get::<i32, _>("event_version")? == event.event_version
            && row.try_get::<String, _>("organization_id")? == event.organization_id
            && row.try_get::<Option<String>, _>("branch_id")? == event.branch_id.map(str::to_owned)
            && row.try_get::<String, _>("aggregate_type")? == event.aggregate_type
            && row.try_get::<String, _>("aggregate_id")? == event.aggregate_id
            && row.try_get::<i64, _>("aggregate_sequence")? == event.aggregate_sequence
            && row.try_get::<DateTime<Utc>, _>("occurred_at")? == event.occurred_at
            && row.try_get::<Vec<u8>, _>("payload_sha256")? == payload_hash;
        if same {
            Ok(false)
        } else {
            Err(ConsumerInboxError::EventIdentityConflict)
        }
    }
}

fn valid_component(value: &str, max_len: usize) -> bool {
    !value.is_empty() && value.len() <= max_len && value.bytes().all(|byte| byte.is_ascii_graphic())
}
