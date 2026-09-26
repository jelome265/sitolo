//! PostgreSQL audit persistence.
//!
//! Implements durable audit record storage with append-only semantics.
//! Runtime role has INSERT-only access scoped to transaction tenant context.

use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;

use sitolo_audit::{IamAuditEvent, IamEventResult};

/// Audit persistence error.
#[derive(Debug, Error)]
pub enum AuditPersistenceError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("serialization error")]
    Serialization,
}

/// Audit writer port for transactional persistence.
#[async_trait]
pub trait AuditWriter: Send + Sync {
    /// Records an audit event within the same transaction as the business mutation.
    ///
    /// This must be called before transaction commit to ensure mandatory audit
    /// evidence is coupled with authoritative state.
    async fn record_required(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &IamAuditEvent,
    ) -> Result<(), AuditPersistenceError>;
}

/// PostgreSQL audit writer implementation.
pub struct PostgresAuditWriter {
    pool: PgPool,
}

impl PostgresAuditWriter {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditWriter for PostgresAuditWriter {
    async fn record_required(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &IamAuditEvent,
    ) -> Result<(), AuditPersistenceError> {
        let result_str = match event.result {
            IamEventResult::Success => "SUCCESS",
            IamEventResult::Failure => "FAILURE",
        };

        let reason_class = event.reason_class.as_ref().map(|r| r.as_str().to_string());
        let branch_id = event.branch_id.as_ref().map(|b| b.to_string());

        let (target_type, target_ref) = match &event.target {
            sitolo_audit::TargetRef::Organization(id) => ("Organization", id.to_string()),
            sitolo_audit::TargetRef::Branch(id) => ("Branch", id.to_string()),
            sitolo_audit::TargetRef::Membership(id) => ("Membership", id.to_string()),
            sitolo_audit::TargetRef::Role(name) => ("Role", name.clone()),
            sitolo_audit::TargetRef::Scope(name) => ("Scope", name.clone()),
            sitolo_audit::TargetRef::Device(id) => ("Device", id.clone()),
        };

        sqlx::query(
            r#"
            INSERT INTO audit_events (
                event_id, event_name, event_version, occurred_at,
                organization_id, branch_id,
                actor_subject_ref, actor_membership_ref, actor_device_ref,
                request_id, trace_id,
                target_type, target_ref, action, result,
                reason_class, assurance_level, source
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#,
        )
        .bind(event.event_id.to_string())
        .bind(event.event_name.as_str())
        .bind(event.event_version.0 as i32)
        .bind(event.occurred_at)
        .bind(event.organization_id.to_string())
        .bind(branch_id)
        .bind(&event.actor.subject_ref)
        .bind(&event.actor.membership_ref)
        .bind(&event.actor.device_ref)
        .bind(&event.request_id)
        .bind(&event.trace_id)
        .bind(target_type)
        .bind(&target_ref)
        .bind(event.action)
        .bind(result_str)
        .bind(reason_class)
        .bind(&event.assurance_level)
        .bind(event.source)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_audit::{ActorRef, IamEventName, TargetRef};
    use sitolo_auth::AuditEventId;
    use sitolo_domain::tenancy::{MembershipId, OrganizationId};

    #[test]
    fn audit_event_serialization_is_secret_free() {
        let event = IamAuditEvent::success(
            AuditEventId::new("evt-test").unwrap(),
            IamEventName::MembershipRevoked,
            OrganizationId::new("org-1").unwrap(),
            None,
            ActorRef {
                subject_ref: "subject-1".into(),
                membership_ref: Some("mem-1".into()),
                device_ref: None,
            },
            TargetRef::Membership(MembershipId::new("mem-1").unwrap()),
            "revoke",
            "tenancy_service",
        );

        let debug = format!("{:?}", event);
        assert!(!debug.contains("password"));
        assert!(!debug.contains("Bearer"));
        assert!(!debug.contains("secret"));
    }
}
