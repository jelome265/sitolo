//! Transactional outbox events.
//!
//! The outbox stores durable event intent in the same database transaction as
//! the business mutation. A separate relay/worker eventually publishes these
//! events to external systems. Delivery is at-least-once; consumers must be
//! idempotent.
//!
//! Reference: https://microservices.io/patterns/data/transactional-outbox

use std::time::SystemTime;

use async_trait::async_trait;
use thiserror::Error;

use sitolo_domain::tenancy::{BranchId, OrganizationId};

/// Outbox event status state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxStatus {
    Pending,
    Claimed,
    Published,
    Quarantined,
}

impl OutboxStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            OutboxStatus::Pending => "PENDING",
            OutboxStatus::Claimed => "CLAIMED",
            OutboxStatus::Published => "PUBLISHED",
            OutboxStatus::Quarantined => "QUARANTINED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    Retryable,
    Terminal,
    Unknown,
}

#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("outbox persistence failed")]
    PersistenceFailed,
    #[error("event payload too large")]
    PayloadTooLarge,
    #[error("invalid event schema")]
    InvalidSchema,
    #[error("event already exists")]
    Duplicate,
}

#[derive(Debug, Clone)]
pub struct OutboxEvent {
    pub event_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_name: String,
    pub event_version: u32,
    pub organization_id: OrganizationId,
    pub branch_id: Option<BranchId>,
    pub occurred_at: SystemTime,
    pub payload: String,
    pub status: OutboxStatus,
    pub available_at: SystemTime,
    pub attempt_count: u32,
    pub locked_at: Option<SystemTime>,
    pub published_at: Option<SystemTime>,
    pub last_error_class: Option<String>,
    pub deduplication_key: String,
    pub schema_version: u32,
}

impl OutboxEvent {
    pub const MAX_PAYLOAD_SIZE: usize = 65_536;

    #[allow(clippy::too_many_arguments)]
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
        })
    }
}

#[async_trait]
pub trait OutboxWriter: Send + Sync {
    async fn enqueue(&self, event: OutboxEvent) -> Result<(), OutboxError>;
}

#[async_trait]
pub trait OutboxReader: Send + Sync {
    async fn claim_batch(
        &self,
        batch_size: usize,
        lease_duration: std::time::Duration,
    ) -> Result<Vec<OutboxEvent>, OutboxError>;

    async fn mark_published(&self, event_id: &str) -> Result<(), OutboxError>;

    async fn release(&self, event_id: &str, error_class: String) -> Result<(), OutboxError>;

    async fn quarantine(&self, event_id: &str, error_class: String) -> Result<(), OutboxError>;
}
