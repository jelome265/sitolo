//! Tenancy service — Phase 4 organization, membership, and invitation use cases.
//!
//! Orchestrates organization provisioning, branch lifecycle, membership
//! lifecycle, role/scope grants, and the token-bound invitation workflow
//! against the tenancy persistence port. Each use case applies its domain
//! transitions atomically; the repository owns the transaction boundary
//! (Phase 4 section 21).
//!
//! Invitation issuance and acceptance are abuse-controlled (Phase 4 section
//! 38) through the shared rate limiter: creation throttles per organization,
//! acceptance throttles per token hash. Rules are caller-supplied policy
//! with documented moderate fallbacks.
//!
//! Explicitly deferred to later Phase 4 PRs: invitation delivery (outbox,
//! PR-009), HTTP routes and DTOs (PR-006). These methods enforce
//! state-machine and scope correctness, not caller authorization — actor
//! authority (who may invite, grant, or revoke) is enforced by the
//! authorization engine (Phase 6) once issuer scope exists.
//!
//! ## Phase 4 Part 8 evidence (PR-009)
//!
//! Durable business-state/audit/outbox atomicity is implemented for
//! organization provisioning and lifecycle transitions, branch creation and
//! lifecycle transitions, direct membership invitation and transitions,
//! membership pending/expiry transitions, role assignment/revocation, scope
//! grant/revocation, token-bound invitation issue/acceptance/withdrawal, and
//! invitation expiry sweeping.
//!
//! Most request-driven methods still take an `actor: ActorRef`; production
//! HTTP authority is not yet composed. Token-bound acceptance derives its
//! actor from the user bound to the locked invitation membership.
//!
//! Owner-approved invitation issue, withdrawal, and expiry sweeping are
//! coupled to their state changes; expiry sweeping is tenant-scoped and
//! bounded per organization.
//!
//! Organization provisioning uses PostgreSQL for its business bundle when
//! the durable evidence sink is configured: organization, owner membership,
//! default branch, audit, and outbox share one application-owned transaction.
//! Role/scope grant and revocation plus invitation acceptance use
//! PostgreSQL transactions when the durable evidence sink is configured.
//! The API composition root must wire verified authority and the durable
//! database path before it can be considered a production tenancy authority.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat, Utc};
use sitolo_audit::{ActorRef, IamAuditEvent, IamEventName, TargetRef};
use sitolo_auth::{
    AbuseClass, AuditEventId, RateLimitDecision, RateLimitRule, RateLimiter, SecurityContext,
};
use sitolo_authz::{
    Invitation, InvitationContact, InvitationError, InvitationState, Permission, Role,
    RoleAssignment, RoleAssignmentState, Scope, ScopeGrant, ScopeGrantState, authorize_scope,
    resolve_permissions,
};
use sitolo_domain::tenancy::{
    Branch, BranchId, BranchState, InvitationId, Membership, MembershipId, MembershipState,
    Organization, OrganizationId, OrganizationState, RoleAssignmentId, ScopeGrantId, TenancyError,
    TenantUserId,
};
use sitolo_events::OutboxEvent;
use sitolo_persistence::{
    AcceptedInvitation, CreateInvitationInput, CreatedInvitation, ProvisionedOrganization,
    TenancyDatabase, TenancyStores, audit_repository::AuditPersistenceError,
    audit_repository::AuditWriter, outbox_repository::OutboxPersistenceError,
    outbox_repository::OutboxWriter, tenancy_postgres::insert_provisioned_organization,
};
use sitolo_tenancy::{EffectiveScope, ScopeError, resolve_effective_scope};
use sqlx_core::error::Error as SqlxError;
use sqlx_core::query::query;
use sqlx_core::transaction::Transaction;
use sqlx_postgres::{PgPool, Postgres};

type PersistedInvitationRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
    DateTime<Utc>,
    i64,
);

type PersistedInvitationAcceptanceRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
    DateTime<Utc>,
);

fn persisted_role_assignment_state(value: &str) -> Option<RoleAssignmentState> {
    match value {
        "REQUESTED" => Some(RoleAssignmentState::Requested),
        "VALIDATING" => Some(RoleAssignmentState::Validating),
        "REJECTED" => Some(RoleAssignmentState::Rejected),
        "APPROVAL_REQUIRED" => Some(RoleAssignmentState::ApprovalRequired),
        "APPROVED" => Some(RoleAssignmentState::Approved),
        "EFFECTIVE" => Some(RoleAssignmentState::Effective),
        "REVOKED" => Some(RoleAssignmentState::Revoked),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct LifecycleEvidenceSpec {
    from: &'static str,
    to: &'static str,
    event_name: IamEventName,
    action: &'static str,
}

/// Durable audit/outbox evidence sink for Phase 4 Part 8 mandatory IAM
/// events (contract §5, §29). The audit insert and the outbox insert run in
/// one PostgreSQL transaction: either both land, or neither does.
///
/// Organization and branch lifecycle, direct membership lifecycle, and
/// invitation issue/withdrawal use this pool for the business mutation and
/// evidence transaction.
/// Unsupported mutations fail closed while this sink is configured.
pub struct TenancyEvidenceSink {
    pool: PgPool,
    audit: Arc<dyn AuditWriter>,
    outbox: Arc<dyn OutboxWriter>,
}

fn audit_sqlstate(error: &AuditPersistenceError) -> Option<String> {
    match error {
        AuditPersistenceError::Database(SqlxError::Database(database_error)) => {
            database_error.code().map(|code| code.into_owned())
        }
        _ => None,
    }
}

fn outbox_sqlstate(error: &OutboxPersistenceError) -> Option<String> {
    match error {
        OutboxPersistenceError::Database(SqlxError::Database(database_error)) => {
            database_error.code().map(|code| code.into_owned())
        }
        _ => None,
    }
}

fn system_time_to_utc(value: SystemTime) -> Result<DateTime<Utc>, TenancyError> {
    let duration = value
        .duration_since(UNIX_EPOCH)
        .map_err(|_| TenancyError::InvalidInvitation)?;
    let seconds = i64::try_from(duration.as_secs()).map_err(|_| TenancyError::InvalidInvitation)?;
    DateTime::from_timestamp(seconds, duration.subsec_nanos())
        .ok_or(TenancyError::InvalidInvitation)
}

fn utc_to_system_time(value: DateTime<Utc>) -> Result<SystemTime, TenancyError> {
    let seconds = value.timestamp();
    let nanos = value.timestamp_subsec_nanos();
    if seconds >= 0 {
        UNIX_EPOCH
            .checked_add(Duration::new(
                u64::try_from(seconds).map_err(|_| TenancyError::InvalidInvitation)?,
                nanos,
            ))
            .ok_or(TenancyError::InvalidInvitation)
    } else {
        UNIX_EPOCH
            .checked_sub(Duration::new(seconds.unsigned_abs(), 0))
            .and_then(|time| time.checked_add(Duration::new(0, nanos)))
            .ok_or(TenancyError::InvalidInvitation)
    }
}

impl TenancyEvidenceSink {
    #[must_use]
    pub fn new(pool: PgPool, audit: Arc<dyn AuditWriter>, outbox: Arc<dyn OutboxWriter>) -> Self {
        TenancyEvidenceSink {
            pool,
            audit,
            outbox,
        }
    }
}

/// The tenancy service (Phase 4 use cases).
pub struct TenancyService {
    db: Arc<TenancyDatabase>,
    limiter: Mutex<RateLimiter>,
    abuse_rules: Vec<(AbuseClass, RateLimitRule)>,
    /// Phase 4 Part 8 evidence sink. `None` in deployments/tests that have
    /// not wired PostgreSQL audit/outbox persistence yet — those callers
    /// intentionally see the pre-Part-8 behavior (no evidence emitted)
    /// rather than a hard failure, so this stays additive.
    evidence: Option<TenancyEvidenceSink>,
}

/// Combined failure from an authenticated tenancy command.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AuthorizedTenancyError {
    /// Persisted membership, permission, or scope denied the command.
    #[error(transparent)]
    Authorization(#[from] ScopeError),
    /// The domain command or mandatory evidence transaction failed.
    #[error(transparent)]
    Tenancy(#[from] TenancyError),
}

/// Fallback abuse rules when the deployment supplies none. These are
/// flood guards, not tuned policy: production limits are environment
/// configuration, load-tested per Phase 4 section 38.
fn fallback_rule() -> RateLimitRule {
    RateLimitRule {
        max_attempts: 10,
        window: Duration::from_secs(60),
        lockout: Duration::from_secs(300),
    }
}

impl TenancyService {
    /// Constructs a tenancy service with required PostgreSQL persistence and
    /// transactional audit/outbox writers. The in-memory store supplied by
    /// [`Self::new_reference`] is retained only as an inaccessible reference backing;
    /// every supported operation selects the configured durable path and
    /// unsupported mutations fail closed.
    ///
    /// # Errors
    /// This constructor does not perform I/O and cannot fail. The runtime
    /// must establish database readiness before publishing the service.
    ///
    /// # Panics
    /// This constructor does not panic.
    pub fn new_postgres(
        abuse_rules: Vec<(AbuseClass, RateLimitRule)>,
        sink: TenancyEvidenceSink,
    ) -> Self {
        Self {
            db: Arc::new(TenancyDatabase::new()),
            limiter: Mutex::new(RateLimiter::new()),
            abuse_rules,
            evidence: Some(sink),
        }
    }

    /// Changes one membership state and writes its mandatory evidence using
    /// the same PostgreSQL transaction. This is the authoritative path when
    /// the evidence sink is configured; the in-memory store remains a test
    /// reference implementation only.
    #[expect(
        clippy::too_many_arguments,
        reason = "the transition descriptor must keep the expected/next state and event provenance explicit"
    )]
    async fn transition_membership_with_evidence(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        expected_state: &'static str,
        next_state: &'static str,
        event_name: IamEventName,
        actor: ActorRef,
        action: &'static str,
    ) -> Result<Membership, TenancyError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let mut tx =
            sink.pool
                .begin()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "membership_transition_begin",
                    sqlstate: None,
                })?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "membership_transition_tenant_context",
                sqlstate: None,
            })?;
        let changed: Option<(String, i64)> = sqlx_core::query_as::query_as(
            "UPDATE organization_memberships SET state = $3, state_version = state_version + 1 WHERE organization_id = $1 AND id = $2 AND state = $4 RETURNING user_id, state_version",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(next_state)
        .bind(expected_state)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
            stage: "membership_transition_update",
            sqlstate: None,
        })?;
        let Some((user_id, state_version)) = changed else {
            return Err(TenancyError::InvalidTransition);
        };
        let membership = Membership {
            id: membership_id.clone(),
            organization_id: organization_id.clone(),
            user_id: TenantUserId::new(user_id).map_err(|_| {
                TenancyError::EvidencePersistenceFailedAt {
                    stage: "membership_transition_user_id",
                    sqlstate: None,
                }
            })?,
            state: match next_state {
                "ACTIVE" => MembershipState::Active,
                "SUSPENDED" => MembershipState::Suspended,
                "REVOKED" => MembershipState::Revoked,
                _ => return Err(TenancyError::InvalidTransition),
            },
            state_version: u64::try_from(state_version).map_err(|_| {
                TenancyError::EvidencePersistenceFailedAt {
                    stage: "membership_transition_state_version",
                    sqlstate: None,
                }
            })?,
        };
        self.record_evidence_in_transaction(
            &mut tx,
            event_name,
            organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor,
            action,
            "membership",
            membership_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": membership_id.as_str(),
                "from_state": expected_state,
                "to_state": next_state,
                "state_version": membership.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "membership_transition_commit",
                sqlstate: None,
            })?;
        Ok(membership)
    }

    async fn assign_role_with_evidence(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        assignment_id: &RoleAssignmentId,
        role: Role,
        actor: ActorRef,
    ) -> Result<RoleAssignment, TenancyError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let mut tx =
            sink.pool
                .begin()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "role_assignment_begin",
                    sqlstate: None,
                })?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "role_assignment_tenant_context",
                sqlstate: None,
            })?;
        query("SELECT lock_organization_mutations($1)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "role_assignment_organization_lock",
                sqlstate: None,
            })?;

        let membership_state: Option<String> = sqlx_core::query_scalar::query_scalar(
            "SELECT state FROM organization_memberships \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
            stage: "role_assignment_membership_read",
            sqlstate: None,
        })?;
        let Some(membership_state) = membership_state else {
            return Err(TenancyError::NotFound);
        };
        if matches!(membership_state.as_str(), "EXPIRED" | "REVOKED") {
            return Err(TenancyError::InvalidTransition);
        }

        let existing: Option<(String, String, String, String, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT organization_id, membership_id, role, state, state_version \
                 FROM role_assignments WHERE id = $1",
            )
            .bind(assignment_id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "role_assignment_idempotency_read",
                sqlstate: None,
            })?;
        if let Some((
            existing_org,
            existing_membership,
            existing_role,
            existing_state,
            state_version,
        )) = existing
        {
            if existing_org != organization_id.as_str()
                || existing_membership != membership_id.as_str()
                || existing_role != role.as_str()
            {
                return Err(TenancyError::Conflict);
            }
            let state_version = u64::try_from(state_version).map_err(|_| {
                TenancyError::EvidencePersistenceFailedAt {
                    stage: "role_assignment_state_version",
                    sqlstate: None,
                }
            })?;
            let state = persisted_role_assignment_state(&existing_state).ok_or(
                TenancyError::EvidencePersistenceFailedAt {
                    stage: "role_assignment_idempotency_state",
                    sqlstate: None,
                },
            )?;
            return Ok(RoleAssignment {
                id: assignment_id.clone(),
                organization_id: organization_id.clone(),
                membership_id: membership_id.clone(),
                role,
                state,
                state_version,
            });
        }

        let state_version: i64 = sqlx_core::query_scalar::query_scalar(
            "INSERT INTO role_assignments \
             (id, organization_id, membership_id, role, state, state_version) \
             VALUES ($1, $2, $3, $4, 'EFFECTIVE', 3) \
             RETURNING state_version",
        )
        .bind(assignment_id.as_str())
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(role.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| {
            let sqlstate = match &error {
                SqlxError::Database(database_error) => {
                    database_error.code().map(|code| code.into_owned())
                }
                _ => None,
            };
            if sqlstate.as_deref() == Some("23505") {
                TenancyError::Conflict
            } else {
                TenancyError::EvidencePersistenceFailedAt {
                    stage: "role_assignment_insert",
                    sqlstate,
                }
            }
        })?;
        let state_version = u64::try_from(state_version).map_err(|_| {
            TenancyError::EvidencePersistenceFailedAt {
                stage: "role_assignment_state_version",
                sqlstate: None,
            }
        })?;
        let assignment = RoleAssignment {
            id: assignment_id.clone(),
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            role,
            state: RoleAssignmentState::Effective,
            state_version,
        };
        self.record_evidence_in_transaction(
            &mut tx,
            IamEventName::RoleAssigned,
            organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor,
            "assign_role",
            "role_assignment",
            assignment_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": membership_id.as_str(),
                "assignment_id": assignment_id.as_str(),
                "role": role.as_str(),
                "state": "EFFECTIVE",
                "state_version": assignment.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "role_assignment_commit",
                sqlstate: None,
            })?;
        Ok(assignment)
    }

    async fn revoke_role_with_evidence(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
        actor: ActorRef,
    ) -> Result<RoleAssignment, TenancyError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let mut tx =
            sink.pool
                .begin()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "role_revocation_begin",
                    sqlstate: None,
                })?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "role_revocation_tenant_context",
                sqlstate: None,
            })?;
        query("SELECT lock_organization_mutations($1)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "role_revocation_organization_lock",
                sqlstate: None,
            })?;

        let changed: Option<(String, i64)> = sqlx_core::query_as::query_as(
            "UPDATE role_assignments \
             SET state = 'REVOKED', state_version = state_version + 1 \
             WHERE organization_id = $1 AND membership_id = $2 \
               AND role = $3 AND state = 'EFFECTIVE' \
             RETURNING id, state_version",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(role.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
            stage: "role_revocation_update",
            sqlstate: None,
        })?;
        let Some((assignment_id, version)) = changed else {
            return Err(TenancyError::NotFound);
        };
        let assignment_id = RoleAssignmentId::new(assignment_id).map_err(|_| {
            TenancyError::EvidencePersistenceFailedAt {
                stage: "role_revocation_assignment_id",
                sqlstate: None,
            }
        })?;
        let state_version =
            u64::try_from(version).map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "role_revocation_state_version",
                sqlstate: None,
            })?;
        let assignment = RoleAssignment {
            id: assignment_id.clone(),
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            role,
            state: RoleAssignmentState::Revoked,
            state_version,
        };
        self.record_evidence_in_transaction(
            &mut tx,
            IamEventName::RoleRemoved,
            organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor,
            "revoke_role",
            "role_assignment",
            assignment_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": membership_id.as_str(),
                "assignment_id": assignment_id.as_str(),
                "role": role.as_str(),
                "state": "REVOKED",
                "state_version": assignment.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "role_revocation_commit",
                sqlstate: None,
            })?;
        Ok(assignment)
    }

    async fn grant_scope_with_evidence(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        grant_id: &ScopeGrantId,
        scope: Scope,
        actor: ActorRef,
    ) -> Result<ScopeGrant, TenancyError> {
        if scope.organization_id() != organization_id {
            return Err(TenancyError::InvalidTransition);
        }
        let (scope_kind, branch_id) = match &scope {
            Scope::Organization { .. } => ("ORGANIZATION", None),
            Scope::Branch { branch_id, .. } => ("BRANCH", Some(branch_id.as_str().to_owned())),
        };
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let mut tx =
            sink.pool
                .begin()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "scope_grant_begin",
                    sqlstate: None,
                })?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "scope_grant_tenant_context",
                sqlstate: None,
            })?;
        query("SELECT lock_organization_mutations($1)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "scope_grant_organization_lock",
                sqlstate: None,
            })?;
        let membership_state: Option<String> = sqlx_core::query_scalar::query_scalar(
            "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
            stage: "scope_grant_membership_read",
            sqlstate: None,
        })?;
        let Some(membership_state) = membership_state else {
            return Err(TenancyError::NotFound);
        };
        if matches!(membership_state.as_str(), "EXPIRED" | "REVOKED") {
            return Err(TenancyError::InvalidTransition);
        }

        let existing: Option<(String, String, String, Option<String>, String, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT organization_id, membership_id, scope_kind, branch_id, state, state_version FROM scope_grants WHERE id = $1",
            ).bind(grant_id.as_str()).fetch_optional(&mut *tx).await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt { stage: "scope_grant_idempotency_read", sqlstate: None })?;
        if let Some((org, member, kind, branch, state, version)) = existing {
            if org != organization_id.as_str()
                || member != membership_id.as_str()
                || kind != scope_kind
                || branch != branch_id
            {
                return Err(TenancyError::Conflict);
            }
            let state = match state.as_str() {
                "ACTIVE" => ScopeGrantState::Active,
                "REVOKED" => ScopeGrantState::Revoked,
                _ => return Err(TenancyError::EvidencePersistenceFailed),
            };
            return Ok(ScopeGrant {
                id: grant_id.clone(),
                organization_id: organization_id.clone(),
                membership_id: membership_id.clone(),
                scope,
                state,
                state_version: u64::try_from(version)
                    .map_err(|_| TenancyError::EvidencePersistenceFailed)?,
            });
        }
        let version: i64 = sqlx_core::query_scalar::query_scalar(
            "INSERT INTO scope_grants (id, organization_id, membership_id, scope_kind, branch_id, state, state_version) VALUES ($1, $2, $3, $4, $5, 'ACTIVE', 1) RETURNING state_version",
        ).bind(grant_id.as_str()).bind(organization_id.as_str()).bind(membership_id.as_str()).bind(scope_kind).bind(branch_id.as_deref())
            .fetch_one(&mut *tx).await.map_err(|error| {
                let sqlstate = match &error { SqlxError::Database(db_error) => db_error.code().map(|code| code.into_owned()), _ => None };
                if sqlstate.as_deref() == Some("23505") { TenancyError::Conflict } else { TenancyError::EvidencePersistenceFailedAt { stage: "scope_grant_insert", sqlstate } }
            })?;
        let grant = ScopeGrant {
            id: grant_id.clone(),
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            scope,
            state: ScopeGrantState::Active,
            state_version: u64::try_from(version)
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?,
        };
        self.record_evidence_in_transaction(&mut tx, IamEventName::ScopeGranted, organization_id, None,
            TargetRef::Membership(membership_id.clone()), actor, "grant_scope", "scope_grant", grant_id.to_string(),
            serde_json::json!({ "organization_id": organization_id.as_str(), "membership_id": membership_id.as_str(), "grant_id": grant_id.as_str(), "scope_kind": scope_kind, "branch_id": branch_id, "state": "ACTIVE", "state_version": grant.state_version })).await?;
        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "scope_grant_commit",
                sqlstate: None,
            })?;
        Ok(grant)
    }

    async fn revoke_scope_with_evidence(
        &self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
        actor: ActorRef,
    ) -> Result<ScopeGrant, TenancyError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let mut tx =
            sink.pool
                .begin()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "scope_revoke_begin",
                    sqlstate: None,
                })?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "scope_revoke_tenant_context",
                sqlstate: None,
            })?;
        query("SELECT lock_organization_mutations($1)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "scope_revoke_organization_lock",
                sqlstate: None,
            })?;
        let row: Option<(String, String, Option<String>, String, i64)> = sqlx_core::query_as::query_as(
            "UPDATE scope_grants SET state = 'REVOKED', state_version = state_version + 1 WHERE organization_id = $1 AND id = $2 AND state = 'ACTIVE' RETURNING membership_id, scope_kind, branch_id, state, state_version",
        ).bind(organization_id.as_str()).bind(grant_id.as_str()).fetch_optional(&mut *tx).await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt { stage: "scope_revoke_update", sqlstate: None })?;
        let Some((membership_raw, kind, branch, state, version)) = row else {
            return Err(TenancyError::NotFound);
        };
        let membership_id = MembershipId::new(membership_raw)
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        let scope = match (kind.as_str(), branch) {
            ("ORGANIZATION", None) => Scope::Organization {
                organization_id: organization_id.clone(),
            },
            ("BRANCH", Some(branch_id)) => Scope::Branch {
                organization_id: organization_id.clone(),
                branch_id: BranchId::new(branch_id)
                    .map_err(|_| TenancyError::EvidencePersistenceFailed)?,
            },
            _ => return Err(TenancyError::EvidencePersistenceFailed),
        };
        let grant = ScopeGrant {
            id: grant_id.clone(),
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            scope,
            state: if state == "REVOKED" {
                ScopeGrantState::Revoked
            } else {
                return Err(TenancyError::EvidencePersistenceFailed);
            },
            state_version: u64::try_from(version)
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?,
        };
        self.record_evidence_in_transaction(&mut tx, IamEventName::ScopeRevoked, organization_id, None,
            TargetRef::Membership(membership_id.clone()), actor, "revoke_scope_grant", "scope_grant", grant_id.to_string(),
            serde_json::json!({ "organization_id": organization_id.as_str(), "membership_id": membership_id.as_str(), "grant_id": grant_id.as_str(), "state": "REVOKED", "state_version": grant.state_version })).await?;
        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "scope_revoke_commit",
                sqlstate: None,
            })?;
        Ok(grant)
    }

    async fn accept_invitation_with_evidence(
        &self,
        token_hash: [u8; 32],
        assignment_id: RoleAssignmentId,
        grant_id: Option<ScopeGrantId>,
        now: SystemTime,
    ) -> Result<AcceptedInvitation, InvitationError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(InvitationError::EvidencePersistenceFailed)?;
        let mut tx = sink
            .pool
            .begin()
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        let organization_raw: Option<String> =
            sqlx_core::query_scalar::query_scalar("SELECT resolve_invitation_organization($1)")
                .bind(token_hash.to_vec())
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        let Some(organization_raw) = organization_raw else {
            return Err(InvitationError::Invalid);
        };
        let organization_id = OrganizationId::new(organization_raw)
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        query("SELECT lock_organization_mutations($1)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;

        let row: Option<PersistedInvitationAcceptanceRow> = sqlx_core::query_as::query_as(
            "SELECT id, membership_id, proposed_role, scope_kind, branch_id, issued_at, expires_at FROM invitations WHERE organization_id = $1 AND token_hash = $2 AND state = 'ISSUED' FOR UPDATE",
        )
        .bind(organization_id.as_str())
        .bind(token_hash.to_vec())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        let Some((
            invitation_raw,
            membership_raw,
            role_raw,
            scope_kind,
            branch_raw,
            issued_at,
            expires_at,
        )) = row
        else {
            return Err(InvitationError::Invalid);
        };
        let now_utc =
            system_time_to_utc(now).map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        if now_utc >= expires_at {
            return Err(InvitationError::Expired);
        }
        let invitation_id = InvitationId::new(invitation_raw)
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        let membership_id = MembershipId::new(membership_raw)
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        let role = Role::all()
            .iter()
            .copied()
            .find(|candidate| candidate.as_str() == role_raw)
            .ok_or(InvitationError::EvidencePersistenceFailed)?;
        let proposed_scope = match (scope_kind.as_deref(), branch_raw) {
            (None, None) => None,
            (Some("ORGANIZATION"), None) => Some(Scope::Organization {
                organization_id: organization_id.clone(),
            }),
            (Some("BRANCH"), Some(branch)) => Some(Scope::Branch {
                organization_id: organization_id.clone(),
                branch_id: BranchId::new(branch)
                    .map_err(|_| InvitationError::EvidencePersistenceFailed)?,
            }),
            _ => return Err(InvitationError::EvidencePersistenceFailed),
        };
        if proposed_scope.is_some() != grant_id.is_some() {
            return Err(InvitationError::Invalid);
        }

        let membership: Option<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT user_id, state FROM organization_memberships WHERE organization_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        let Some((user_raw, membership_state)) = membership else {
            return Err(InvitationError::Invalid);
        };
        let user_id =
            TenantUserId::new(user_raw).map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        let actor = ActorRef {
            subject_ref: user_id.as_str().to_owned(),
            membership_ref: Some(membership_id.as_str().to_owned()),
            device_ref: None,
        };
        if membership_state == "INVITED" {
            let version: Option<i64> = sqlx_core::query_scalar::query_scalar(
                "UPDATE organization_memberships SET state = 'PENDING_ACCEPTANCE', state_version = state_version + 1 WHERE organization_id = $1 AND id = $2 AND state = 'INVITED' RETURNING state_version",
            )
            .bind(organization_id.as_str())
            .bind(membership_id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            let Some(version) = version else {
                return Err(InvitationError::Invalid);
            };
            self.record_evidence_in_transaction(
                &mut tx,
                IamEventName::MembershipPendingAcceptance,
                &organization_id,
                None,
                TargetRef::Membership(membership_id.clone()),
                actor.clone(),
                "accept_invitation_pending",
                "membership",
                membership_id.to_string(),
                serde_json::json!({ "membership_id": membership_id.as_str(), "from_state": "INVITED", "to_state": "PENDING_ACCEPTANCE", "state_version": version }),
            )
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        } else if membership_state != "PENDING_ACCEPTANCE" {
            return Err(InvitationError::Invalid);
        }
        let active_version: Option<i64> = sqlx_core::query_scalar::query_scalar(
            "UPDATE organization_memberships SET state = 'ACTIVE', state_version = state_version + 1 WHERE organization_id = $1 AND id = $2 AND state = 'PENDING_ACCEPTANCE' RETURNING state_version",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        let Some(active_version) = active_version else {
            return Err(InvitationError::Invalid);
        };
        self.record_evidence_in_transaction(
            &mut tx,
            IamEventName::MembershipActivated,
            &organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor.clone(),
            "accept_invitation_activate",
            "membership",
            membership_id.to_string(),
            serde_json::json!({ "organization_id": organization_id.as_str(), "membership_id": membership_id.as_str(), "state": "ACTIVE", "state_version": active_version }),
        )
        .await
        .map_err(|_| InvitationError::EvidencePersistenceFailed)?;

        let assignment_version: i64 = sqlx_core::query_scalar::query_scalar(
            "INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ($1, $2, $3, $4, 'EFFECTIVE', 3) RETURNING state_version",
        )
        .bind(assignment_id.as_str())
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(role.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| {
            let sqlstate = match &error {
                SqlxError::Database(database_error) => {
                    database_error.code().map(|code| code.into_owned())
                }
                _ => None,
            };
            if matches!(sqlstate.as_deref(), Some("23505" | "23503")) {
                InvitationError::Invalid
            } else {
                InvitationError::EvidencePersistenceFailed
            }
        })?;
        let assignment = RoleAssignment {
            id: assignment_id,
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            role,
            state: RoleAssignmentState::Effective,
            state_version: u64::try_from(assignment_version)
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?,
        };
        self.record_evidence_in_transaction(
            &mut tx,
            IamEventName::RoleAssigned,
            &organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor.clone(),
            "accept_invitation_assign_role",
            "role_assignment",
            assignment.id.to_string(),
            serde_json::json!({ "organization_id": organization_id.as_str(), "membership_id": membership_id.as_str(), "assignment_id": assignment.id.as_str(), "role": role.as_str(), "state": "EFFECTIVE", "state_version": assignment.state_version }),
        )
        .await
        .map_err(|_| InvitationError::EvidencePersistenceFailed)?;

        let scope_grant = if let (Some(scope), Some(grant_id)) = (proposed_scope.clone(), grant_id)
        {
            let (kind, branch_id) = match &scope {
                Scope::Organization { .. } => ("ORGANIZATION", None),
                Scope::Branch { branch_id, .. } => ("BRANCH", Some(branch_id.as_str().to_owned())),
            };
            sqlx_core::query::query(
                "INSERT INTO scope_grants (id, organization_id, membership_id, scope_kind, branch_id, state, state_version) VALUES ($1, $2, $3, $4, $5, 'ACTIVE', 1)",
            )
            .bind(grant_id.as_str())
            .bind(organization_id.as_str())
            .bind(membership_id.as_str())
            .bind(kind)
            .bind(branch_id.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(|error| {
                let sqlstate = match &error {
                    SqlxError::Database(database_error) => {
                        database_error.code().map(|code| code.into_owned())
                    }
                    _ => None,
                };
                if matches!(sqlstate.as_deref(), Some("23505" | "23503")) {
                    InvitationError::Invalid
                } else {
                    InvitationError::EvidencePersistenceFailed
                }
            })?;
            let grant = ScopeGrant::grant(
                grant_id.clone(),
                organization_id.clone(),
                membership_id.clone(),
                scope,
            );
            self.record_evidence_in_transaction(
                &mut tx,
                IamEventName::ScopeGranted,
                &organization_id,
                None,
                TargetRef::Membership(membership_id.clone()),
                actor.clone(),
                "accept_invitation_grant_scope",
                "scope_grant",
                grant_id.to_string(),
                serde_json::json!({ "organization_id": organization_id.as_str(), "membership_id": membership_id.as_str(), "grant_id": grant_id.as_str(), "scope_kind": kind, "branch_id": branch_id, "state": "ACTIVE", "state_version": grant.state_version }),
            )
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            Some(grant)
        } else {
            None
        };

        let invitation_version: Option<i64> = sqlx_core::query_scalar::query_scalar(
            "UPDATE invitations SET state = 'ACCEPTED', state_version = state_version + 1, contact_kind = NULL, contact_value = NULL, token_hash = NULL WHERE organization_id = $1 AND id = $2 AND state = 'ISSUED' RETURNING state_version",
        )
        .bind(organization_id.as_str())
        .bind(invitation_id.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        let Some(invitation_version) = invitation_version else {
            return Err(InvitationError::Invalid);
        };
        let invitation = Invitation {
            id: invitation_id.clone(),
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            proposed_role: role,
            proposed_scope,
            contact: None,
            token_hash: None,
            issued_at: utc_to_system_time(issued_at)
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?,
            expires_at: utc_to_system_time(expires_at)
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?,
            state: InvitationState::Accepted,
            state_version: u64::try_from(invitation_version)
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?,
        };
        self.record_evidence_in_transaction(
            &mut tx,
            IamEventName::MembershipInvitationAccepted,
            &organization_id,
            None,
            TargetRef::Invitation(invitation_id.clone()),
            actor.clone(),
            "accept_invitation",
            "invitation",
            invitation_id.to_string(),
            serde_json::json!({ "organization_id": organization_id.as_str(), "invitation_id": invitation_id.as_str(), "membership_id": membership_id.as_str(), "state": "ACCEPTED", "state_version": invitation.state_version }),
        )
        .await
        .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        tx.commit()
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;

        let mut membership = Membership::invite(membership_id, organization_id, user_id);
        if membership_state == "INVITED" {
            membership
                .mark_pending()
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        }
        membership
            .activate()
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        Ok(AcceptedInvitation {
            invitation,
            membership,
            assignment,
            scope_grant,
        })
    }

    async fn invite_member_with_evidence(
        &self,
        membership_id: MembershipId,
        organization_id: OrganizationId,
        user_id: TenantUserId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let membership =
            Membership::invite(membership_id.clone(), organization_id.clone(), user_id);
        let mut tx =
            sink.pool
                .begin()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "membership_invite_begin",
                    sqlstate: None,
                })?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "membership_invite_tenant_context",
                sqlstate: None,
            })?;
        let organization_state: Option<String> =
            sqlx_core::query_scalar::query_scalar("SELECT lock_organization_mutations($1)")
                .bind(organization_id.as_str())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "membership_invite_organization_check",
                    sqlstate: None,
                })?;
        if organization_state.as_deref() != Some("ACTIVE") {
            return Err(TenancyError::InvalidTransition);
        }
        query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ($1, $2, $3, 'INVITED', 1)")
            .bind(membership_id.as_str())
            .bind(organization_id.as_str())
            .bind(membership.user_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|error| {
                let sqlstate = match &error {
                    SqlxError::Database(database_error) => {
                        database_error.code().map(|code| code.into_owned())
                    }
                    _ => None,
                };
                if sqlstate.as_deref() == Some("23505") {
                    TenancyError::Conflict
                } else {
                    TenancyError::EvidencePersistenceFailedAt {
                        stage: "membership_invite_insert",
                        sqlstate,
                    }
                }
            })?;
        self.record_evidence_in_transaction(
            &mut tx,
            IamEventName::MembershipInvited,
            &organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor,
            "invite_member",
            "membership",
            membership_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": membership_id.as_str(),
                "state": "INVITED",
                "state_version": membership.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "membership_invite_commit",
                sqlstate: None,
            })?;
        Ok(membership)
    }

    /// Constructs the in-memory reference implementation for tests and local
    /// domain examples. It does not persist mandatory evidence and must not
    /// be used by a served production runtime. Use [`Self::new_postgres`] for
    /// production composition.
    ///
    /// # Errors
    /// This constructor has no fallible I/O and cannot fail.
    ///
    /// # Panics
    /// This constructor does not panic.
    pub fn new_reference(
        db: Arc<TenancyDatabase>,
        abuse_rules: Vec<(AbuseClass, RateLimitRule)>,
    ) -> Self {
        TenancyService {
            db,
            limiter: Mutex::new(RateLimiter::new()),
            abuse_rules,
            evidence: None,
        }
    }

    /// Adds durable evidence to the explicit in-memory reference service.
    /// Production composition must use [`Self::new_postgres`] so the
    /// persistent capability is mandatory at construction time.
    ///
    /// # Errors
    /// This builder does not perform I/O and cannot fail.
    ///
    /// # Panics
    /// This builder does not panic.
    #[must_use]
    pub fn for_reference_with_evidence(mut self, sink: TenancyEvidenceSink) -> Self {
        self.evidence = Some(sink);
        self
    }

    /// Prevents unsupported in-memory mutations from being mistaken for
    /// authoritative writes when durable evidence is configured.
    fn require_authoritative_mutation_path(&self) -> Result<(), TenancyError> {
        if self.evidence.is_some() {
            Err(TenancyError::EvidencePersistenceFailedAt {
                stage: "authoritative_mutation_path_unavailable",
                sqlstate: None,
            })
        } else {
            Ok(())
        }
    }

    /// Records one mandatory Phase 4 IAM audit event and its paired outbox
    /// event atomically (contract §5, §29). No-op success when no evidence
    /// sink is configured (see the `evidence` field doc).
    ///
    /// `aggregate_id`/`payload` describe the outbox envelope (contract
    /// §14, §19): bounded, versioned, secret-free, independent of Rust
    /// debug formatting. Callers must not pass raw domain-object debug
    /// output as `payload`.
    #[allow(clippy::too_many_arguments)]
    async fn record_evidence(
        &self,
        event_name: IamEventName,
        organization_id: &OrganizationId,
        branch_id: Option<&BranchId>,
        target: TargetRef,
        actor: ActorRef,
        action: &'static str,
        aggregate_type: &'static str,
        aggregate_id: String,
        payload: serde_json::Value,
    ) -> Result<(), TenancyError> {
        let Some(sink) = self.evidence.as_ref() else {
            return Ok(());
        };

        // Globally unique event identity (contract §6.1) — not derived from
        // timestamps, names, or client-supplied sequential values.
        let event_id_raw = uuid::Uuid::new_v4().to_string();
        let event_id = AuditEventId::new(event_id_raw.clone())
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;

        let audit_event = IamAuditEvent::success(
            event_id,
            event_name,
            organization_id.clone(),
            branch_id.cloned(),
            actor,
            target,
            action,
            "tenancy_service",
        );

        let payload_str =
            serde_json::to_string(&payload).map_err(|_| TenancyError::EvidencePersistenceFailed)?;

        let outbox_event = OutboxEvent::new(
            event_id_raw,
            aggregate_type.to_string(),
            aggregate_id,
            event_name.as_str().to_string(),
            1,
            organization_id.clone(),
            branch_id.cloned(),
            payload_str,
        )
        .map_err(|_| TenancyError::EvidencePersistenceFailed)?;

        // Audit insert + outbox insert share one transaction: forbidden
        // per contract §5 to commit the business mutation and then try
        // each of these best-effort. Here they are already coupled to
        // each other; see the module doc for the business-mutation leg.
        let mut tx = sink
            .pool
            .begin()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;

        sink.audit
            .record_required(&mut tx, &audit_event)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;

        sink.outbox
            .enqueue(&mut tx, &outbox_event)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;

        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_evidence_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event_name: IamEventName,
        organization_id: &OrganizationId,
        branch_id: Option<&BranchId>,
        target: TargetRef,
        actor: ActorRef,
        action: &'static str,
        aggregate_type: &'static str,
        aggregate_id: String,
        payload: serde_json::Value,
    ) -> Result<(), TenancyError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let event_id_raw = uuid::Uuid::new_v4().to_string();
        let event_id = AuditEventId::new(event_id_raw.clone())
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        let audit_event = IamAuditEvent::success(
            event_id,
            event_name,
            organization_id.clone(),
            branch_id.cloned(),
            actor,
            target,
            action,
            "tenancy_service",
        );
        let payload_str =
            serde_json::to_string(&payload).map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        let outbox_event = OutboxEvent::new(
            event_id_raw,
            aggregate_type.to_string(),
            aggregate_id,
            event_name.as_str().to_string(),
            1,
            organization_id.clone(),
            branch_id.cloned(),
            payload_str,
        )
        .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        sink.audit
            .record_required(tx, &audit_event)
            .await
            .map_err(|error| TenancyError::EvidencePersistenceFailedAt {
                stage: "audit_write",
                sqlstate: audit_sqlstate(&error),
            })?;
        sink.outbox
            .enqueue(tx, &outbox_event)
            .await
            .map_err(|error| TenancyError::EvidencePersistenceFailedAt {
                stage: "outbox_enqueue",
                sqlstate: outbox_sqlstate(&error),
            })?;
        Ok(())
    }

    fn check_rate(&self, class: AbuseClass, key: &str) -> RateLimitDecision {
        let mut limiter = self
            .limiter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let rule = self
            .abuse_rules
            .iter()
            .find(|(c, _)| *c == class)
            .map(|(_, r)| *r)
            .unwrap_or_else(fallback_rule);
        limiter.check(class, key, &rule, SystemTime::now())
    }

    /// Atomically provisions an organization with its owner membership and
    /// default branch (Phase 4 section 7.1). Either the whole triple commits
    /// or nothing does — there is no observable ownerless organization.
    pub async fn provision_organization(
        &self,
        organization_id: OrganizationId,
        organization_name: &str,
        owner_membership_id: MembershipId,
        owner_user_id: TenantUserId,
        branch_id: BranchId,
        branch_name: &str,
    ) -> Result<ProvisionedOrganization, TenancyError> {
        if let Some(sink) = self.evidence.as_ref() {
            let mut organization =
                Organization::provision(organization_id.clone(), organization_name)?;
            organization.activate()?;
            let mut owner_membership = Membership::invite(
                owner_membership_id.clone(),
                organization_id.clone(),
                owner_user_id.clone(),
            );
            owner_membership.mark_pending()?;
            owner_membership.activate()?;
            let mut default_branch =
                Branch::provision(branch_id.clone(), organization_id.clone(), branch_name)?;
            default_branch.activate()?;
            let provisioned = ProvisionedOrganization {
                organization,
                owner_membership,
                default_branch,
            };

            let mut tx =
                sink.pool
                    .begin()
                    .await
                    .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                        stage: "membership_begin",
                        sqlstate: None,
                    })?;
            query("SELECT set_config('app.organization_id', $1, true)")
                .bind(organization_id.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "membership_tenant_context",
                    sqlstate: None,
                })?;
            let newly_created = insert_provisioned_organization(
                &mut tx,
                &provisioned.organization,
                &provisioned.owner_membership,
                &provisioned.default_branch,
            )
            .await
            .map_err(|error| match error {
                sitolo_persistence::tenancy_postgres::TenancyProvisioningError::Conflict => {
                    TenancyError::Conflict
                }
                sitolo_persistence::tenancy_postgres::TenancyProvisioningError::Database(_) => {
                    TenancyError::EvidencePersistenceFailed
                }
            })?;
            if !newly_created {
                tx.commit()
                    .await
                    .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
                return Ok(provisioned);
            }
            self.record_evidence_in_transaction(
                &mut tx,
                IamEventName::OrganizationCreated,
                &organization_id,
                Some(&branch_id),
                TargetRef::Organization(organization_id.clone()),
                ActorRef {
                    subject_ref: owner_user_id.to_string(),
                    membership_ref: Some(owner_membership_id.to_string()),
                    device_ref: None,
                },
                "provision_organization",
                "organization",
                organization_id.to_string(),
                serde_json::json!({
                    "organization_id": organization_id.as_str(),
                    "organization_name": provisioned.organization.name,
                    "owner_membership_id": owner_membership_id.as_str(),
                    "branch_id": branch_id.as_str(),
                }),
            )
            .await?;
            tx.commit()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
            return Ok(provisioned);
        }

        let provisioned = self
            .db
            .provision_organization(
                organization_id.clone(),
                organization_name,
                owner_membership_id.clone(),
                owner_user_id.clone(),
                branch_id.clone(),
                branch_name,
            )
            .await?;

        // Phase 4 Part 8 (§5, §29, §38): mandatory `iam.organization.created`
        // audit evidence + paired outbox event. Bootstrap actor is the
        // organization's own owner — there is no separate issuing principal
        // for self-serve provisioning.
        self.record_evidence(
            IamEventName::OrganizationCreated,
            &organization_id,
            Some(&branch_id),
            TargetRef::Organization(organization_id.clone()),
            ActorRef {
                subject_ref: owner_user_id.to_string(),
                membership_ref: Some(owner_membership_id.to_string()),
                device_ref: None,
            },
            "provision_organization",
            "organization",
            organization_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "organization_name": provisioned.organization.name,
                "owner_membership_id": owner_membership_id.as_str(),
                "branch_id": branch_id.as_str(),
            }),
        )
        .await?;

        Ok(provisioned)
    }

    /// Creates an additional branch under an operating organization.
    pub async fn create_branch(
        &self,
        id: BranchId,
        organization_id: OrganizationId,
        name: &str,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if let Some(sink) = self.evidence.as_ref() {
            let mut branch = Branch::provision(id.clone(), organization_id.clone(), name)?;
            branch.activate()?;
            let mut tx =
                sink.pool
                    .begin()
                    .await
                    .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                        stage: "branch_create_begin",
                        sqlstate: None,
                    })?;
            query("SELECT set_config('app.organization_id', $1, true)")
                .bind(organization_id.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "branch_create_tenant_context",
                    sqlstate: None,
                })?;
            let organization_state: Option<String> =
                sqlx_core::query_scalar::query_scalar("SELECT lock_organization_mutations($1)")
                    .bind(organization_id.as_str())
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                        stage: "branch_create_organization_check",
                        sqlstate: None,
                    })?;
            if organization_state.as_deref() != Some("ACTIVE") {
                return Err(TenancyError::InvalidTransition);
            }
            query("INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ($1, $2, $3, 'ACTIVE', 2)")
                .bind(id.as_str())
                .bind(organization_id.as_str())
                .bind(&branch.name)
                .execute(&mut *tx)
                .await
                .map_err(|error| {
                    let sqlstate = match &error {
                        SqlxError::Database(database_error) => {
                            database_error.code().map(|code| code.into_owned())
                        }
                        _ => None,
                    };
                    if sqlstate.as_deref() == Some("23505") {
                        TenancyError::Conflict
                    } else {
                        TenancyError::EvidencePersistenceFailedAt {
                            stage: "branch_create_insert",
                            sqlstate,
                        }
                    }
                })?;
            self.record_evidence_in_transaction(
                &mut tx,
                IamEventName::BranchCreated,
                &organization_id,
                Some(&id),
                TargetRef::Branch(id.clone()),
                actor,
                "create_branch",
                "branch",
                id.to_string(),
                serde_json::json!({
                    "organization_id": organization_id.as_str(),
                    "branch_id": id.as_str(),
                    "branch_name": branch.name,
                }),
            )
            .await?;
            tx.commit()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "branch_create_commit",
                    sqlstate: None,
                })?;
            return Ok(branch);
        }
        let branch = self
            .db
            .create_branch(id.clone(), organization_id.clone(), name)
            .await?;
        self.record_evidence(
            IamEventName::BranchCreated,
            &organization_id,
            Some(&id),
            TargetRef::Branch(id.clone()),
            actor,
            "create_branch",
            "branch",
            id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "branch_id": id.as_str(),
                "branch_name": branch.name,
            }),
        )
        .await?;
        Ok(branch)
    }

    /// Creates a branch after authorizing the verified principal against
    /// current PostgreSQL authority while holding the organization's mutation
    /// lock.
    /// Business state, audit evidence and outbox intent commit together.
    ///
    /// # Errors
    /// Returns [`AuthorizedTenancyError::Authorization`] when current
    /// persisted membership, role, permission or scope denies the request;
    /// returns [`AuthorizedTenancyError::Tenancy`] for invalid or conflicting
    /// branch state and mandatory evidence failures.
    ///
    /// # Panics
    /// This method does not panic.
    pub async fn create_branch_for_authenticated_user(
        &self,
        security_context: &SecurityContext,
        id: BranchId,
        organization_id: OrganizationId,
        name: &str,
    ) -> Result<Branch, AuthorizedTenancyError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(ScopeError::AuthorityUnavailable)?;
        let user_id = TenantUserId::new(security_context.principal.subject_id.as_str())?;
        let mut branch = Branch::provision(id.clone(), organization_id.clone(), name)?;
        branch.activate()?;

        let mut tx = sink.pool.begin().await.map_err(|_| {
            AuthorizedTenancyError::Tenancy(TenancyError::EvidencePersistenceFailedAt {
                stage: "authorized_branch_create_begin",
                sqlstate: None,
            })
        })?;
        establish_authorized_mutation_transaction(&mut tx, &organization_id).await?;
        let locked_organization_state: Option<String> =
            sqlx_core::query_scalar::query_scalar("SELECT lock_organization_mutations($1)")
                .bind(organization_id.as_str())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| ScopeError::AuthorityUnavailable)?;
        if locked_organization_state.is_none() {
            return Err(ScopeError::OrganizationDenied.into());
        }
        let organization_state: Option<(String,)> =
            sqlx_core::query_as::query_as("SELECT state FROM organizations WHERE id = $1")
                .bind(organization_id.as_str())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| ScopeError::AuthorityUnavailable)?;
        if organization_state.as_ref().map(|(state,)| state.as_str()) != Some("ACTIVE") {
            return Err(ScopeError::OrganizationNotActive.into());
        }
        let scope = authorize_postgres_operation_in_transaction(
            &mut tx,
            &organization_id,
            MembershipSelector::User(&user_id),
            None,
            Permission::BranchCreate,
        )
        .await?;

        query("INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ($1, $2, $3, 'ACTIVE', 2)")
            .bind(id.as_str())
            .bind(organization_id.as_str())
            .bind(&branch.name)
            .execute(&mut *tx)
            .await
            .map_err(|error| {
                let sqlstate = match &error {
                    SqlxError::Database(database_error) => {
                        database_error.code().map(|code| code.into_owned())
                    }
                    _ => None,
                };
                if sqlstate.as_deref() == Some("23505") {
                    AuthorizedTenancyError::Tenancy(TenancyError::Conflict)
                } else {
                    AuthorizedTenancyError::Tenancy(
                        TenancyError::EvidencePersistenceFailedAt {
                            stage: "authorized_branch_create_insert",
                            sqlstate,
                        },
                    )
                }
            })?;
        self.record_evidence_in_transaction(
            &mut tx,
            IamEventName::BranchCreated,
            &organization_id,
            Some(&id),
            TargetRef::Branch(id.clone()),
            ActorRef {
                subject_ref: security_context.principal.subject_id.as_str().to_owned(),
                membership_ref: Some(scope.membership_id.as_str().to_owned()),
                device_ref: security_context
                    .principal
                    .device_id
                    .as_ref()
                    .map(|device_id| device_id.as_str().to_owned()),
            },
            "create_branch",
            "branch",
            id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "branch_id": id.as_str(),
                "branch_name": branch.name,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "authorized_branch_create_commit",
                sqlstate: None,
            })?;
        Ok(branch)
    }

    /// Invites a user into an organization (section 8): records the
    /// membership in state `Invited` and enforces one non-terminal
    /// membership per (organization, user). Token-bound enrollment with
    /// proposed role/scope goes through [`Self::create_invitation`].
    pub async fn invite_member(
        &self,
        id: MembershipId,
        organization_id: OrganizationId,
        user_id: TenantUserId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.evidence.is_some() {
            return self
                .invite_member_with_evidence(id, organization_id, user_id, actor)
                .await;
        }
        let membership = self
            .db
            .invite_membership(id.clone(), organization_id.clone(), user_id.clone())
            .await?;
        self.record_evidence(
            IamEventName::MembershipInvited,
            &organization_id,
            None,
            TargetRef::Membership(id.clone()),
            actor,
            "invite_member",
            "membership",
            id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": id.as_str(),
                "user_id": user_id.as_str(),
            }),
        )
        .await?;
        Ok(membership)
    }

    /// Advances an invitation to pending acceptance.
    ///
    /// When the durable sink is configured, the PostgreSQL state transition,
    /// mandatory audit event, and outbox intent share one transaction.
    /// Without that sink, this method uses the in-memory reference store,
    /// which is not an authoritative production persistence path.
    pub async fn mark_member_pending(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if let Some(sink) = self.evidence.as_ref() {
            let mut tx = sink
                .pool
                .begin()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
            query("SELECT set_config('app.organization_id', $1, true)")
                .bind(organization_id.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
            let changed: Option<(String, i64)> = sqlx_core::query_as::query_as(
                "UPDATE organization_memberships SET state = 'PENDING_ACCEPTANCE', state_version = state_version + 1 WHERE organization_id = $1 AND id = $2 AND state = 'INVITED' RETURNING user_id, state_version",
            )
            .bind(organization_id.as_str())
            .bind(id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt { stage: "membership_update", sqlstate: None })?;
            let Some((user_id, state_version)) = changed else {
                return Err(TenancyError::InvalidTransition);
            };
            let membership = Membership {
                id: id.clone(),
                organization_id: organization_id.clone(),
                user_id: TenantUserId::new(user_id).map_err(|_| {
                    TenancyError::EvidencePersistenceFailedAt {
                        stage: "membership_user_id",
                        sqlstate: None,
                    }
                })?,
                state: MembershipState::PendingAcceptance,
                state_version: u64::try_from(state_version).map_err(|_| {
                    TenancyError::EvidencePersistenceFailedAt {
                        stage: "membership_state_version",
                        sqlstate: None,
                    }
                })?,
            };
            self.record_evidence_in_transaction(
                &mut tx,
                IamEventName::MembershipPendingAcceptance,
                organization_id,
                None,
                TargetRef::Membership(id.clone()),
                actor,
                "mark_member_pending",
                "membership",
                id.to_string(),
                serde_json::json!({
                    "membership_id": id.as_str(),
                    "from_state": "INVITED",
                    "to_state": "PENDING_ACCEPTANCE",
                    "state_version": membership.state_version,
                }),
            )
            .await?;
            tx.commit()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "membership_commit",
                    sqlstate: None,
                })?;
            return Ok(membership);
        }
        self.db.advance_membership(organization_id, id).await
    }

    /// Accepts enrollment: `PENDING_ACCEPTANCE -> ACTIVE`.
    pub async fn accept_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_membership_with_evidence(
                    organization_id,
                    id,
                    "PENDING_ACCEPTANCE",
                    "ACTIVE",
                    IamEventName::MembershipActivated,
                    actor,
                    "accept_member",
                )
                .await;
        }
        let membership = self.db.activate_membership(organization_id, id).await?;
        self.record_evidence(
            IamEventName::MembershipActivated,
            organization_id,
            None,
            TargetRef::Membership(id.clone()),
            actor,
            "accept_member",
            "membership",
            id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": id.as_str(),
            }),
        )
        .await?;
        Ok(membership)
    }

    /// Temporarily removes authority with reactivation possible (20.1).
    pub async fn suspend_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_membership_with_evidence(
                    organization_id,
                    id,
                    "ACTIVE",
                    "SUSPENDED",
                    IamEventName::MembershipSuspended,
                    actor,
                    "suspend_member",
                )
                .await;
        }
        let membership = self.db.suspend_membership(organization_id, id).await?;
        self.record_evidence(
            IamEventName::MembershipSuspended,
            organization_id,
            None,
            TargetRef::Membership(id.clone()),
            actor,
            "suspend_member",
            "membership",
            id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": id.as_str(),
            }),
        )
        .await?;
        Ok(membership)
    }

    /// Restores a suspended membership.
    ///
    /// Emitted as [`IamEventName::MembershipActivated`]: the contract's
    /// event vocabulary has no separate "resumed" event for memberships
    /// (unlike organizations/branches, which do have a dedicated
    /// `...Resumed` name) — reactivation and initial acceptance share the
    /// same terminal state and the same named event.
    pub async fn resume_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_membership_with_evidence(
                    organization_id,
                    id,
                    "SUSPENDED",
                    "ACTIVE",
                    IamEventName::MembershipActivated,
                    actor,
                    "resume_member",
                )
                .await;
        }
        let membership = self.db.resume_membership(organization_id, id).await?;
        self.record_evidence(
            IamEventName::MembershipActivated,
            organization_id,
            None,
            TargetRef::Membership(id.clone()),
            actor,
            "resume_member",
            "membership",
            id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": id.as_str(),
            }),
        )
        .await?;
        Ok(membership)
    }

    /// Permanently terminates the membership for ordinary purposes (20.2).
    /// Triggers session and cache invalidation downstream via the bumped
    /// state version (20.3); invalidation consumers arrive in later PRs.
    pub async fn revoke_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_membership_with_evidence(
                    organization_id,
                    id,
                    "ACTIVE",
                    "REVOKED",
                    IamEventName::MembershipRevoked,
                    actor,
                    "revoke_member",
                )
                .await;
        }
        let membership = self.db.revoke_membership(organization_id, id).await?;
        self.record_evidence(
            IamEventName::MembershipRevoked,
            organization_id,
            None,
            TargetRef::Membership(id.clone()),
            actor,
            "revoke_member",
            "membership",
            id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": id.as_str(),
            }),
        )
        .await?;
        Ok(membership)
    }

    /// Expires a lapsed enrollment window with the approved system actor.
    ///
    /// With the durable sink configured, state, audit, and outbox are written
    /// in one PostgreSQL transaction.
    pub async fn expire_invitation(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        if let Some(sink) = self.evidence.as_ref() {
            let mut tx =
                sink.pool
                    .begin()
                    .await
                    .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                        stage: "expiry_begin",
                        sqlstate: None,
                    })?;
            query("SELECT set_config('app.organization_id', $1, true)")
                .bind(organization_id.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "expiry_tenant_context",
                    sqlstate: None,
                })?;
            let changed: Option<(String, i64)> = sqlx_core::query_as::query_as(
                "UPDATE organization_memberships SET state = 'EXPIRED', state_version = state_version + 1 WHERE organization_id = $1 AND id = $2 AND state = 'PENDING_ACCEPTANCE' RETURNING user_id, state_version",
            )
            .bind(organization_id.as_str())
            .bind(id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt { stage: "expiry_update", sqlstate: None })?;
            let Some((user_id, state_version)) = changed else {
                return Err(TenancyError::InvalidTransition);
            };
            let membership = Membership {
                id: id.clone(),
                organization_id: organization_id.clone(),
                user_id: TenantUserId::new(user_id).map_err(|_| {
                    TenancyError::EvidencePersistenceFailedAt {
                        stage: "expiry_user_id",
                        sqlstate: None,
                    }
                })?,
                state: MembershipState::Expired,
                state_version: u64::try_from(state_version).map_err(|_| {
                    TenancyError::EvidencePersistenceFailedAt {
                        stage: "expiry_state_version",
                        sqlstate: None,
                    }
                })?,
            };
            self.record_evidence_in_transaction(
                &mut tx,
                IamEventName::MembershipExpired,
                organization_id,
                None,
                TargetRef::Membership(id.clone()),
                ActorRef {
                    subject_ref: "system:invitation-expiry".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
                "expire_invitation",
                "membership",
                id.to_string(),
                serde_json::json!({
                    "membership_id": id.as_str(),
                    "from_state": "PENDING_ACCEPTANCE",
                    "to_state": "EXPIRED",
                    "state_version": membership.state_version,
                    "expiry_reason": "enrollment_window_elapsed",
                }),
            )
            .await?;
            tx.commit()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "expiry_commit",
                    sqlstate: None,
                })?;
            return Ok(membership);
        }
        self.db.expire_membership(organization_id, id).await
    }

    /// Creates a token-bound invitation (section 9): mints the `Invited`
    /// membership and the `Issued` invitation atomically. The raw token is
    /// caller-minted high-entropy material — hashed on entry, never stored,
    /// never logged. Throttled per organization (section 38).
    ///
    /// With the durable sink configured, membership, invitation, audit, and
    /// outbox rows commit together. The token hash is stored only while the
    /// invitation is issued; terminal transitions must erase it and contact.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_invitation(
        &self,
        organization_id: OrganizationId,
        invitation_id: InvitationId,
        membership_id: MembershipId,
        user_id: TenantUserId,
        proposed_role: Role,
        proposed_scope: Option<Scope>,
        contact: InvitationContact,
        raw_token: &str,
        ttl: Duration,
        actor: ActorRef,
    ) -> Result<CreatedInvitation, TenancyError> {
        if matches!(
            self.check_rate(AbuseClass::InvitationCreate, organization_id.as_str()),
            RateLimitDecision::Locked { .. }
        ) {
            return Err(TenancyError::RateLimited);
        }
        let now = SystemTime::now();
        if now.checked_add(ttl).is_none() {
            return Err(TenancyError::InvalidInvitation);
        }
        if let Some(sink) = self.evidence.as_ref() {
            let membership = Membership::invite(
                membership_id.clone(),
                organization_id.clone(),
                user_id.clone(),
            );
            let invitation = Invitation::issue(
                invitation_id.clone(),
                organization_id.clone(),
                membership_id.clone(),
                proposed_role,
                proposed_scope.clone(),
                contact.clone(),
                raw_token,
                now,
                ttl,
            )?;
            let expires_at = now
                .checked_add(ttl)
                .ok_or(TenancyError::InvalidInvitation)?;
            if proposed_scope
                .as_ref()
                .is_some_and(|scope| scope.organization_id() != &organization_id)
            {
                return Err(TenancyError::InvalidInvitation);
            }
            let (scope_kind, branch_id) = match proposed_scope.as_ref() {
                None => (None, None),
                Some(Scope::Organization { .. }) => (Some("ORGANIZATION"), None),
                Some(Scope::Branch { branch_id, .. }) => (Some("BRANCH"), Some(branch_id.as_str())),
            };
            let contact_kind = match contact.kind {
                sitolo_authz::ContactKind::Email => "EMAIL",
                sitolo_authz::ContactKind::Phone => "PHONE",
            };
            let token_hash = invitation
                .token_hash
                .ok_or(TenancyError::InvalidInvitation)?;
            let mut tx =
                sink.pool
                    .begin()
                    .await
                    .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                        stage: "invitation_create_begin",
                        sqlstate: None,
                    })?;
            query("SELECT set_config('app.organization_id', $1, true)")
                .bind(organization_id.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "invitation_create_tenant_context",
                    sqlstate: None,
                })?;
            let organization_state: Option<String> =
                sqlx_core::query_scalar::query_scalar("SELECT lock_organization_mutations($1)")
                    .bind(organization_id.as_str())
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                        stage: "invitation_create_organization_check",
                        sqlstate: None,
                    })?;
            if organization_state.as_deref() != Some("ACTIVE") {
                return Err(TenancyError::InvalidTransition);
            }
            query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ($1, $2, $3, 'INVITED', 1)")
                .bind(membership_id.as_str())
                .bind(organization_id.as_str())
                .bind(user_id.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|error| {
                    let sqlstate = match &error {
                        SqlxError::Database(database_error) => {
                            database_error.code().map(|code| code.into_owned())
                        }
                        _ => None,
                    };
                    if sqlstate.as_deref() == Some("23505") {
                        TenancyError::Conflict
                    } else {
                        TenancyError::EvidencePersistenceFailedAt {
                            stage: "invitation_create_membership_insert",
                            sqlstate,
                        }
                    }
                })?;
            query("INSERT INTO invitations (id, organization_id, membership_id, proposed_role, scope_kind, branch_id, contact_kind, contact_value, token_hash, state, issued_at, expires_at, state_version) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'ISSUED', $10, $11, 1)")
                .bind(invitation_id.as_str())
                .bind(organization_id.as_str())
                .bind(membership_id.as_str())
                .bind(proposed_role.as_str())
                .bind(scope_kind)
                .bind(branch_id)
                .bind(contact_kind)
                .bind(&contact.value)
                .bind(token_hash.to_vec())
                .bind(system_time_to_utc(now)?)
                .bind(system_time_to_utc(expires_at)?)
                .execute(&mut *tx)
                .await
                .map_err(|error| {
                    let sqlstate = match &error {
                        SqlxError::Database(database_error) => {
                            database_error.code().map(|code| code.into_owned())
                        }
                        _ => None,
                    };
                    if sqlstate.as_deref() == Some("23505") {
                        TenancyError::Conflict
                    } else {
                        TenancyError::EvidencePersistenceFailedAt {
                            stage: "invitation_create_insert",
                            sqlstate,
                        }
                    }
                })?;
            self.record_evidence_in_transaction(
                &mut tx,
                IamEventName::InvitationIssued,
                &organization_id,
                None,
                TargetRef::Invitation(invitation_id.clone()),
                actor,
                "create_invitation",
                "invitation",
                invitation_id.to_string(),
                serde_json::json!({
                    "invitation_id": invitation_id.as_str(),
                    "membership_id": membership_id.as_str(),
                    "expires_at": system_time_to_utc(expires_at)?.to_rfc3339_opts(SecondsFormat::Millis, true),
                    "state_version": invitation.state_version,
                }),
            )
            .await?;
            tx.commit()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "invitation_create_commit",
                    sqlstate: None,
                })?;
            return Ok(CreatedInvitation {
                membership,
                invitation,
            });
        }
        self.require_authoritative_mutation_path()?;
        self.db
            .create_invitation(CreateInvitationInput {
                organization_id,
                invitation_id,
                membership_id,
                user_id,
                proposed_role,
                proposed_scope,
                contact,
                raw_token: raw_token.to_string(),
                ttl,
                now,
            })
            .await
    }

    /// Accepts an invitation by raw token: claims the single-use token and
    /// drives membership activation plus the record's proposed role/scope
    /// grants atomically (sections 9.2, 9.3, T4). The token carries no role
    /// or scope — both materialize from the server-loaded record, so a
    /// tampered client payload grants nothing. Throttled per token hash
    /// (section 38). Unknown or withdrawn tokens share `Invalid`.
    pub async fn accept_invitation(
        &self,
        raw_token: &str,
        assignment_id: RoleAssignmentId,
        grant_id: Option<ScopeGrantId>,
        actor: ActorRef,
    ) -> Result<AcceptedInvitation, InvitationError> {
        Invitation::validate_raw_token(raw_token).map_err(|_| InvitationError::Invalid)?;
        let token_hash = Invitation::hash_token(raw_token);
        if matches!(
            self.check_rate(AbuseClass::InvitationAccept, &sitolo_auth::hex(&token_hash),),
            RateLimitDecision::Locked { .. }
        ) {
            return Err(InvitationError::RateLimited);
        }
        let now = SystemTime::now();
        if self.evidence.is_some() {
            return self
                .accept_invitation_with_evidence(token_hash, assignment_id, grant_id, now)
                .await;
        }
        let accepted = self
            .db
            .accept_invitation(token_hash, assignment_id, grant_id, now)
            .await?;
        self.record_evidence(
            IamEventName::MembershipInvitationAccepted,
            &accepted.membership.organization_id,
            None,
            TargetRef::Membership(accepted.membership.id.clone()),
            actor,
            "accept_invitation",
            "membership",
            accepted.membership.id.to_string(),
            serde_json::json!({
                "organization_id": accepted.membership.organization_id.as_str(),
                "membership_id": accepted.membership.id.as_str(),
                "invitation_id": accepted.invitation.id.as_str(),
            }),
        )
        .await
        .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        Ok(accepted)
    }

    /// Withdraws an issued invitation and releases the linked membership
    /// for re-enrollment.
    ///
    /// The invitation terminal transition, linked membership release, audit
    /// record, and outbox event share one PostgreSQL transaction.
    pub async fn revoke_invitation(
        &self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
        actor: ActorRef,
    ) -> Result<Invitation, InvitationError> {
        if let Some(sink) = self.evidence.as_ref() {
            let mut tx = sink
                .pool
                .begin()
                .await
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            query("SELECT set_config('app.organization_id', $1, true)")
                .bind(organization_id.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            let row: Option<PersistedInvitationRow> = sqlx_core::query_as::query_as(
                "SELECT state, membership_id, proposed_role, scope_kind, issued_at, expires_at, state_version FROM invitations WHERE organization_id = $1 AND id = $2 FOR UPDATE",
            )
            .bind(organization_id.as_str())
            .bind(invitation_id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            let Some((state, membership_id_raw, role_raw, scope_kind, issued_at, expires_at, _)) =
                row
            else {
                return Err(InvitationError::Invalid);
            };
            match state.as_str() {
                "ISSUED" => {}
                "ACCEPTED" => return Err(InvitationError::AlreadyAccepted),
                "EXPIRED" => return Err(InvitationError::Expired),
                _ => return Err(InvitationError::Invalid),
            }
            let membership_id = MembershipId::new(membership_id_raw)
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            let role = Role::all()
                .iter()
                .copied()
                .find(|role| Some(role.as_str()) == role_raw.as_deref())
                .ok_or(InvitationError::EvidencePersistenceFailed)?;
            let branch_id: Option<String> = sqlx_core::query_scalar::query_scalar(
                "SELECT branch_id FROM invitations WHERE organization_id = $1 AND id = $2",
            )
            .bind(organization_id.as_str())
            .bind(invitation_id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?
            .flatten();
            let proposed_scope = match scope_kind.as_deref() {
                None => None,
                Some("ORGANIZATION") => Some(Scope::Organization {
                    organization_id: organization_id.clone(),
                }),
                Some("BRANCH") => Some(Scope::Branch {
                    organization_id: organization_id.clone(),
                    branch_id: BranchId::new(
                        branch_id.ok_or(InvitationError::EvidencePersistenceFailed)?,
                    )
                    .map_err(|_| InvitationError::EvidencePersistenceFailed)?,
                }),
                _ => return Err(InvitationError::EvidencePersistenceFailed),
            };
            let invitation: Option<(i64,)> = sqlx_core::query_as::query_as(
                "UPDATE invitations SET state = 'WITHDRAWN', state_version = state_version + 1, contact_kind = NULL, contact_value = NULL, token_hash = NULL WHERE organization_id = $1 AND id = $2 AND state = 'ISSUED' RETURNING state_version",
            )
            .bind(organization_id.as_str())
            .bind(invitation_id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            let Some((state_version,)) = invitation else {
                return Err(InvitationError::Invalid);
            };
            query("UPDATE organization_memberships SET state = 'EXPIRED', state_version = state_version + 2 WHERE organization_id = $1 AND id = $2 AND state = 'INVITED'")
                .bind(organization_id.as_str())
                .bind(membership_id.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            let version = u64::try_from(state_version)
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            let issued_at = utc_to_system_time(issued_at)
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            let expires_at = utc_to_system_time(expires_at)
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            self.record_evidence_in_transaction(
                &mut tx,
                IamEventName::InvitationWithdrawn,
                organization_id,
                None,
                TargetRef::Invitation(invitation_id.clone()),
                actor,
                "revoke_invitation",
                "invitation",
                invitation_id.to_string(),
                serde_json::json!({
                    "invitation_id": invitation_id.as_str(),
                    "membership_id": membership_id.as_str(),
                    "from_state": "ISSUED",
                    "to_state": "WITHDRAWN",
                    "state_version": version,
                }),
            )
            .await
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            tx.commit()
                .await
                .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
            return Ok(Invitation {
                id: invitation_id.clone(),
                organization_id: organization_id.clone(),
                membership_id,
                proposed_role: role,
                proposed_scope,
                contact: None,
                token_hash: None,
                issued_at,
                expires_at,
                state: InvitationState::Revoked,
                state_version: version,
            });
        }
        self.require_authoritative_mutation_path()
            .map_err(|_| InvitationError::EvidencePersistenceFailed)?;
        self.db
            .revoke_invitation(organization_id, invitation_id)
            .await
    }

    /// Sweeps lapsed issued invitations to expired, releasing their linked
    /// memberships. Returns the count transitioned.
    ///
    /// The current PostgreSQL sweep is fail-closed until a bounded
    /// cross-tenant worker contract is available.
    pub async fn expire_invitations(&self) -> Result<u64, TenancyError> {
        self.require_authoritative_mutation_path()?;
        Ok(self.db.expire_invitations(SystemTime::now()).await)
    }

    /// Expires one bounded batch of issued invitations for a single tenant.
    ///
    /// Callers enumerate organizations through their trusted scheduler and
    /// invoke this method once per organization. Row locks with `SKIP LOCKED`
    /// make overlapping sweepers safe. Invitation state, linked membership
    /// state, audit, and outbox intent share one PostgreSQL transaction.
    pub async fn expire_invitations_for_organization(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<u64, TenancyError> {
        const BATCH_LIMIT: i64 = 100;
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let mut tx =
            sink.pool
                .begin()
                .await
                .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                    stage: "invitation_expiry_sweep_begin",
                    sqlstate: None,
                })?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "invitation_expiry_sweep_tenant_context",
                sqlstate: None,
            })?;

        let candidates: Vec<(String, String, DateTime<Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, membership_id, expires_at FROM invitations \
             WHERE organization_id = $1 AND state = 'ISSUED' AND expires_at <= now() \
             ORDER BY expires_at, id LIMIT $2 FOR UPDATE SKIP LOCKED",
        )
        .bind(organization_id.as_str())
        .bind(BATCH_LIMIT)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
            stage: "invitation_expiry_sweep_select",
            sqlstate: None,
        })?;

        let mut transitioned = 0_u64;
        for (invitation_raw, membership_raw, expires_at) in candidates {
            let invitation_id = InvitationId::new(invitation_raw)
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
            let membership_id = MembershipId::new(membership_raw)
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
            let invitation: Option<(i64,)> = sqlx_core::query_as::query_as(
                "UPDATE invitations SET state = 'EXPIRED', state_version = state_version + 1, \
                 contact_kind = NULL, contact_value = NULL, token_hash = NULL \
                 WHERE organization_id = $1 AND id = $2 AND state = 'ISSUED' \
                   AND expires_at <= now() RETURNING state_version",
            )
            .bind(organization_id.as_str())
            .bind(invitation_id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "invitation_expiry_sweep_update_invitation",
                sqlstate: None,
            })?;
            let Some((invitation_version,)) = invitation else {
                continue;
            };
            let invitation_version = u64::try_from(invitation_version)
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?;

            let membership: Option<(i64,)> = sqlx_core::query_as::query_as(
                "UPDATE organization_memberships SET state = 'EXPIRED', \
                 state_version = state_version + 1 \
                 WHERE organization_id = $1 AND id = $2 AND state = 'INVITED' \
                 RETURNING state_version",
            )
            .bind(organization_id.as_str())
            .bind(membership_id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "invitation_expiry_sweep_update_membership",
                sqlstate: None,
            })?;
            if let Some((membership_version,)) = membership {
                let membership_version = u64::try_from(membership_version)
                    .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
                self.record_evidence_in_transaction(
                    &mut tx,
                    IamEventName::MembershipExpired,
                    organization_id,
                    None,
                    TargetRef::Membership(membership_id.clone()),
                    ActorRef {
                        subject_ref: "system:invitation-expiry-sweeper".to_owned(),
                        membership_ref: None,
                        device_ref: None,
                    },
                    "expire_invitations",
                    "membership",
                    membership_id.to_string(),
                    serde_json::json!({
                        "membership_id": membership_id.as_str(),
                        "from_state": "INVITED",
                        "to_state": "EXPIRED",
                        "state_version": membership_version,
                        "expiry_reason": "invitation_expired",
                    }),
                )
                .await?;
            }

            self.record_evidence_in_transaction(
                &mut tx,
                IamEventName::InvitationExpired,
                organization_id,
                None,
                TargetRef::Invitation(invitation_id.clone()),
                ActorRef {
                    subject_ref: "system:invitation-expiry-sweeper".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
                "expire_invitations",
                "invitation",
                invitation_id.to_string(),
                serde_json::json!({
                    "invitation_id": invitation_id.as_str(),
                    "membership_id": membership_id.as_str(),
                    "from_state": "ISSUED",
                    "to_state": "EXPIRED",
                    "expires_at": expires_at.to_rfc3339_opts(SecondsFormat::Millis, true),
                    "state_version": invitation_version,
                }),
            )
            .await?;
            transitioned = transitioned
                .checked_add(1)
                .ok_or(TenancyError::EvidencePersistenceFailed)?;
        }

        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailedAt {
                stage: "invitation_expiry_sweep_commit",
                sqlstate: None,
            })?;
        Ok(transitioned)
    }

    /// Grants an organization-wide role to a live membership. Who may grant
    /// which role is an actor-authority question answered by Phase 6 once
    /// issuer scope is available (sections 8.1.8, 8.1.9, 11.2); this method
    /// enforces target-state and duplicate-grant correctness only.
    pub async fn assign_role(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        assignment_id: RoleAssignmentId,
        role: Role,
        actor: ActorRef,
    ) -> Result<RoleAssignment, TenancyError> {
        if self.evidence.is_some() {
            return self
                .assign_role_with_evidence(
                    &organization_id,
                    &membership_id,
                    &assignment_id,
                    role,
                    actor,
                )
                .await;
        }
        let assignment = self
            .db
            .assign_role(
                organization_id.clone(),
                membership_id.clone(),
                assignment_id.clone(),
                role,
            )
            .await?;
        self.record_evidence(
            IamEventName::RoleAssigned,
            &organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor,
            "assign_role",
            "role_assignment",
            assignment_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": membership_id.as_str(),
                "assignment_id": assignment_id.as_str(),
            }),
        )
        .await?;
        Ok(assignment)
    }

    /// Ends an effective grant. History is preserved; re-granting creates a
    /// new record (section 8.1.10).
    pub async fn revoke_role(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
        actor: ActorRef,
    ) -> Result<RoleAssignment, TenancyError> {
        if self.evidence.is_some() {
            return self
                .revoke_role_with_evidence(organization_id, membership_id, role, actor)
                .await;
        }
        let assignment = self
            .db
            .revoke_role(organization_id, membership_id, role)
            .await?;
        self.record_evidence(
            IamEventName::RoleRemoved,
            organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor,
            "revoke_role",
            "role_assignment",
            assignment.id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": membership_id.as_str(),
                "assignment_id": assignment.id.as_str(),
            }),
        )
        .await?;
        Ok(assignment)
    }

    /// Full grant history for a membership, all states.
    pub async fn member_roles(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<RoleAssignment> {
        if let Some(sink) = self.evidence.as_ref() {
            return persisted_member_roles(&sink.pool, organization_id, membership_id)
                .await
                .unwrap_or_default();
        }
        self.db
            .role_assignments_for(organization_id, membership_id)
            .await
    }

    /// Narrows a membership's authority to an explicit scope. Structural
    /// validity (scope inside the organization, branch existence) is enforced
    /// by the repository; issuer authority (the widening rule, section 11.2)
    /// is enforced by Phase 6 once actor scope exists.
    pub async fn grant_scope(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        grant_id: ScopeGrantId,
        scope: Scope,
        actor: ActorRef,
    ) -> Result<ScopeGrant, TenancyError> {
        if self.evidence.is_some() {
            return self
                .grant_scope_with_evidence(
                    &organization_id,
                    &membership_id,
                    &grant_id,
                    scope,
                    actor,
                )
                .await;
        }
        let grant = self
            .db
            .grant_scope(
                organization_id.clone(),
                membership_id.clone(),
                grant_id.clone(),
                scope,
            )
            .await?;
        self.record_evidence(
            IamEventName::ScopeGranted,
            &organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor,
            "grant_scope",
            "scope_grant",
            grant_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": membership_id.as_str(),
                "grant_id": grant_id.as_str(),
            }),
        )
        .await?;
        Ok(grant)
    }

    /// Lifts one narrowing entry. Remaining active grants still apply.
    pub async fn revoke_scope_grant(
        &self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
        actor: ActorRef,
    ) -> Result<ScopeGrant, TenancyError> {
        if self.evidence.is_some() {
            return self
                .revoke_scope_with_evidence(organization_id, grant_id, actor)
                .await;
        }
        let grant = self
            .db
            .revoke_scope_grant(organization_id, grant_id)
            .await?;
        self.record_evidence(
            IamEventName::ScopeRevoked,
            organization_id,
            None,
            TargetRef::Membership(grant.membership_id.clone()),
            actor,
            "revoke_scope_grant",
            "scope_grant",
            grant_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "grant_id": grant_id.as_str(),
            }),
        )
        .await?;
        Ok(grant)
    }

    /// Active and revoked narrowing entries for a membership.
    pub async fn member_grants(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<ScopeGrant> {
        if let Some(sink) = self.evidence.as_ref() {
            return persisted_member_grants(&sink.pool, organization_id, membership_id)
                .await
                .unwrap_or_default();
        }
        self.db
            .scope_grants_for(organization_id, membership_id)
            .await
    }

    /// Resolves the membership's effective organization-wide permissions:
    /// the union over `Effective` grants, gated on an `Active` membership.
    /// Unknown or non-active memberships resolve to the empty set — deny by
    /// default with no existence disclosure. Scope narrowing (branch and
    /// below) arrives with scope grants in PR-004.
    pub async fn member_permissions(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> BTreeSet<Permission> {
        if let Some(sink) = self.evidence.as_ref() {
            return persisted_member_permissions(&sink.pool, organization_id, membership_id)
                .await
                .unwrap_or_default();
        }
        let membership = self
            .db
            .membership_snapshot(organization_id, membership_id)
            .await
            .filter(|membership| membership.has_authority());
        let Some(membership) = membership else {
            return BTreeSet::new();
        };
        let assignments = self
            .db
            .role_assignments_for(&membership.organization_id, &membership.id)
            .await;
        resolve_permissions(&assignments)
    }

    /// Records evidence for an organization-lifecycle transition whose
    /// result is the organization itself (contract §5, §29).
    async fn record_organization_evidence(
        &self,
        event_name: IamEventName,
        organization: &Organization,
        actor: ActorRef,
        action: &'static str,
    ) -> Result<(), TenancyError> {
        self.record_evidence(
            event_name,
            &organization.id,
            None,
            TargetRef::Organization(organization.id.clone()),
            actor,
            action,
            "organization",
            organization.id.to_string(),
            serde_json::json!({
                "organization_id": organization.id.as_str(),
                "state": format!("{:?}", organization.state),
            }),
        )
        .await
    }

    async fn transition_organization_with_evidence(
        &self,
        id: &OrganizationId,
        next: OrganizationState,
        spec: LifecycleEvidenceSpec,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let mut tx = sink
            .pool
            .begin()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        let changed: Option<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT name, state_version FROM transition_organization_state($1, $2)",
        )
        .bind(spec.from)
        .bind(spec.to)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        let Some((name, state_version)) = changed else {
            return Err(TenancyError::InvalidTransition);
        };
        let organization = Organization {
            id: id.clone(),
            name,
            state: next,
            state_version: u64::try_from(state_version)
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?,
        };
        self.record_evidence_in_transaction(
            &mut tx,
            spec.event_name,
            id,
            None,
            TargetRef::Organization(id.clone()),
            actor,
            spec.action,
            "organization",
            id.to_string(),
            serde_json::json!({
                "organization_id": id.as_str(),
                "state": spec.to,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        Ok(organization)
    }

    async fn transition_branch_with_evidence(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        next: BranchState,
        spec: LifecycleEvidenceSpec,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        let mut tx = sink
            .pool
            .begin()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        query("SELECT set_config('app.organization_id', $1, true)")
            .bind(organization_id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        let changed: Option<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT name, state_version FROM transition_branch_state($1, $2, $3)",
        )
        .bind(id.as_str())
        .bind(spec.from)
        .bind(spec.to)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        let Some((name, state_version)) = changed else {
            return Err(TenancyError::InvalidTransition);
        };
        let branch = Branch {
            id: id.clone(),
            organization_id: organization_id.clone(),
            name,
            state: next,
            state_version: u64::try_from(state_version)
                .map_err(|_| TenancyError::EvidencePersistenceFailed)?,
        };
        self.record_evidence_in_transaction(
            &mut tx,
            spec.event_name,
            organization_id,
            Some(id),
            TargetRef::Branch(id.clone()),
            actor,
            spec.action,
            "branch",
            id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "branch_id": id.as_str(),
                "state": spec.to,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| TenancyError::EvidencePersistenceFailed)?;
        Ok(branch)
    }

    /// Records evidence for a branch-lifecycle transition whose result is
    /// the branch itself (contract §5, §29).
    async fn record_branch_evidence(
        &self,
        event_name: IamEventName,
        branch: &Branch,
        actor: ActorRef,
        action: &'static str,
    ) -> Result<(), TenancyError> {
        self.record_evidence(
            event_name,
            &branch.organization_id,
            Some(&branch.id),
            TargetRef::Branch(branch.id.clone()),
            actor,
            action,
            "branch",
            branch.id.to_string(),
            serde_json::json!({
                "organization_id": branch.organization_id.as_str(),
                "branch_id": branch.id.as_str(),
                "state": format!("{:?}", branch.state),
            }),
        )
        .await
    }

    /// Organization lifecycle transitions (section 6).
    pub async fn activate_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_organization_with_evidence(
                    id,
                    OrganizationState::Active,
                    LifecycleEvidenceSpec {
                        from: "PROVISIONING",
                        to: "ACTIVE",
                        event_name: IamEventName::OrganizationActivated,
                        action: "activate_organization",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let organization = self.db.activate_organization(id).await?;
        self.record_organization_evidence(
            IamEventName::OrganizationActivated,
            &organization,
            actor,
            "activate_organization",
        )
        .await?;
        Ok(organization)
    }

    pub async fn suspend_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_organization_with_evidence(
                    id,
                    OrganizationState::Suspended,
                    LifecycleEvidenceSpec {
                        from: "ACTIVE",
                        to: "SUSPENDED",
                        event_name: IamEventName::OrganizationSuspended,
                        action: "suspend_organization",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let organization = self.db.suspend_organization(id).await?;
        self.record_organization_evidence(
            IamEventName::OrganizationSuspended,
            &organization,
            actor,
            "suspend_organization",
        )
        .await?;
        Ok(organization)
    }

    pub async fn resume_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_organization_with_evidence(
                    id,
                    OrganizationState::Active,
                    LifecycleEvidenceSpec {
                        from: "SUSPENDED",
                        to: "ACTIVE",
                        event_name: IamEventName::OrganizationResumed,
                        action: "resume_organization",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let organization = self.db.resume_organization(id).await?;
        self.record_organization_evidence(
            IamEventName::OrganizationResumed,
            &organization,
            actor,
            "resume_organization",
        )
        .await?;
        Ok(organization)
    }

    pub async fn begin_close_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_organization_with_evidence(
                    id,
                    OrganizationState::Closing,
                    LifecycleEvidenceSpec {
                        from: "ACTIVE",
                        to: "CLOSING",
                        event_name: IamEventName::OrganizationClosingStarted,
                        action: "begin_close_organization",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let organization = self.db.begin_close_organization(id).await?;
        self.record_organization_evidence(
            IamEventName::OrganizationClosingStarted,
            &organization,
            actor,
            "begin_close_organization",
        )
        .await?;
        Ok(organization)
    }

    pub async fn close_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_organization_with_evidence(
                    id,
                    OrganizationState::Closed,
                    LifecycleEvidenceSpec {
                        from: "CLOSING",
                        to: "CLOSED",
                        event_name: IamEventName::OrganizationClosed,
                        action: "close_organization",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let organization = self.db.close_organization(id).await?;
        self.record_organization_evidence(
            IamEventName::OrganizationClosed,
            &organization,
            actor,
            "close_organization",
        )
        .await?;
        Ok(organization)
    }

    /// Branch lifecycle transitions (section 14.1). All scope-carrying: a
    /// branch is only addressable through its owning organization.
    pub async fn activate_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_branch_with_evidence(
                    organization_id,
                    id,
                    BranchState::Active,
                    LifecycleEvidenceSpec {
                        from: "PROVISIONING",
                        to: "ACTIVE",
                        event_name: IamEventName::BranchActivated,
                        action: "activate_branch",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let branch = self.db.activate_branch(organization_id, id).await?;
        self.record_branch_evidence(
            IamEventName::BranchActivated,
            &branch,
            actor,
            "activate_branch",
        )
        .await?;
        Ok(branch)
    }

    pub async fn suspend_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_branch_with_evidence(
                    organization_id,
                    id,
                    BranchState::Suspended,
                    LifecycleEvidenceSpec {
                        from: "ACTIVE",
                        to: "SUSPENDED",
                        event_name: IamEventName::BranchSuspended,
                        action: "suspend_branch",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let branch = self.db.suspend_branch(organization_id, id).await?;
        self.record_branch_evidence(
            IamEventName::BranchSuspended,
            &branch,
            actor,
            "suspend_branch",
        )
        .await?;
        Ok(branch)
    }

    pub async fn resume_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_branch_with_evidence(
                    organization_id,
                    id,
                    BranchState::Active,
                    LifecycleEvidenceSpec {
                        from: "SUSPENDED",
                        to: "ACTIVE",
                        event_name: IamEventName::BranchResumed,
                        action: "resume_branch",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let branch = self.db.resume_branch(organization_id, id).await?;
        self.record_branch_evidence(IamEventName::BranchResumed, &branch, actor, "resume_branch")
            .await?;
        Ok(branch)
    }

    pub async fn begin_close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_branch_with_evidence(
                    organization_id,
                    id,
                    BranchState::Closing,
                    LifecycleEvidenceSpec {
                        from: "ACTIVE",
                        to: "CLOSING",
                        event_name: IamEventName::BranchClosingStarted,
                        action: "begin_close_branch",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let branch = self.db.begin_close_branch(organization_id, id).await?;
        self.record_branch_evidence(
            IamEventName::BranchClosingStarted,
            &branch,
            actor,
            "begin_close_branch",
        )
        .await?;
        Ok(branch)
    }

    pub async fn close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.evidence.is_some() {
            return self
                .transition_branch_with_evidence(
                    organization_id,
                    id,
                    BranchState::Closed,
                    LifecycleEvidenceSpec {
                        from: "CLOSING",
                        to: "CLOSED",
                        event_name: IamEventName::BranchClosed,
                        action: "close_branch",
                    },
                    actor,
                )
                .await;
        }
        self.require_authoritative_mutation_path()?;
        let branch = self.db.close_branch(organization_id, id).await?;
        self.record_branch_evidence(IamEventName::BranchClosed, &branch, actor, "close_branch")
            .await?;
        Ok(branch)
    }

    /// Resolves the effective scope for persisted records (sections 5.1, 54).
    /// Missing records deny exactly like non-operating records — existence
    /// is never disclosed through this path.
    pub async fn effective_scope(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        branch_id: Option<&BranchId>,
    ) -> Result<EffectiveScope, ScopeError> {
        if let Some(sink) = self.evidence.as_ref() {
            let mut tx = sink
                .pool
                .begin()
                .await
                .map_err(|_| ScopeError::AuthorityUnavailable)?;
            establish_scope_transaction(&mut tx, organization_id).await?;
            let scope = load_effective_scope(
                &mut tx,
                organization_id,
                MembershipSelector::Id(membership_id),
                branch_id,
            )
            .await?;
            tx.commit()
                .await
                .map_err(|_| ScopeError::AuthorityUnavailable)?;
            return Ok(scope);
        }

        let organization = self
            .db
            .organization_snapshot(organization_id)
            .await
            .ok_or(ScopeError::OrganizationDenied)?;
        let membership = self
            .db
            .membership_snapshot(organization_id, membership_id)
            .await
            .ok_or(ScopeError::MembershipNotActive)?;
        let branch = match branch_id {
            Some(id) => Some(
                self.db
                    .branch_snapshot(organization_id, id)
                    .await
                    .ok_or(ScopeError::BranchDenied)?,
            ),
            None => None,
        };
        resolve_effective_scope(&membership, &organization, branch.as_ref())
    }

    /// Authorizes one tenancy-resource operation: the section-11.3
    /// intersection of role permissions and membership scope, over an
    /// operating organization, an active membership, and an operating branch
    /// where branch-scoped.
    ///
    /// ```text
    /// operating records -> EffectiveScope
    ///         +
    /// required permission in resolved role permissions
    ///         +
    /// requested scope covered by active grants (or none narrow)
    ///         =
    /// EffectiveScope with populated permissions
    /// ```
    ///
    /// Approvals, separation of duties, property/state authorization, and
    /// entitlement checks are Phase 6 concerns and are not evaluated here.
    pub async fn authorize_operation(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        branch_id: Option<&BranchId>,
        permission: Permission,
    ) -> Result<EffectiveScope, ScopeError> {
        if let Some(sink) = self.evidence.as_ref() {
            return authorize_postgres_operation(
                &sink.pool,
                organization_id,
                MembershipSelector::Id(membership_id),
                branch_id,
                permission,
            )
            .await;
        }

        let mut scope = self
            .effective_scope(organization_id, membership_id, branch_id)
            .await?;
        let assignments = self
            .db
            .role_assignments_for(&scope.organization_id, &scope.membership_id)
            .await;
        let permissions = resolve_permissions(&assignments);
        if !permissions.contains(&permission) {
            return Err(ScopeError::PermissionDenied);
        }
        let requested = match &scope.branch_id {
            Some(branch) => Scope::Branch {
                organization_id: scope.organization_id.clone(),
                branch_id: branch.clone(),
            },
            None => Scope::Organization {
                organization_id: scope.organization_id.clone(),
            },
        };
        let grants = self
            .db
            .scope_grants_for(&scope.organization_id, &scope.membership_id)
            .await;
        if !authorize_scope(&grants, &requested) {
            return Err(ScopeError::ScopeDenied);
        }
        scope.permissions = permissions;
        Ok(scope)
    }

    /// Resolves and authorizes an operation from the authenticated Sitolo user
    /// ID. The organization and branch values are selectors only; membership,
    /// operating state, role permissions, and scope grants are loaded from the
    /// configured authority store.
    ///
    /// # Errors
    /// Returns a scope denial when no active membership or permission covers
    /// the request, or [`ScopeError::AuthorityUnavailable`] when the durable
    /// authority cannot be read.
    pub async fn authorize_user_operation(
        &self,
        user_id: &TenantUserId,
        organization_id: &OrganizationId,
        branch_id: Option<&BranchId>,
        permission: Permission,
    ) -> Result<EffectiveScope, ScopeError> {
        let sink = self
            .evidence
            .as_ref()
            .ok_or(ScopeError::AuthorityUnavailable)?;
        authorize_postgres_operation(
            &sink.pool,
            organization_id,
            MembershipSelector::User(user_id),
            branch_id,
            permission,
        )
        .await
    }
}

enum MembershipSelector<'a> {
    Id(&'a MembershipId),
    User(&'a TenantUserId),
}

type PersistedScopeRow = (
    String,
    String,
    i64,
    String,
    String,
    String,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
);

type PersistedRoleRow = (String, String, String, i64);
type PersistedGrantRow = (String, String, Option<String>, String, i64);

async fn persisted_role_assignments(
    tx: &mut Transaction<'_, Postgres>,
    organization_id: &OrganizationId,
    membership_id: &MembershipId,
) -> Result<Vec<RoleAssignment>, ScopeError> {
    let rows: Vec<PersistedRoleRow> = sqlx_core::query_as::query_as(
        "SELECT id, role, state, state_version FROM role_assignments \
         WHERE organization_id = $1 AND membership_id = $2 ORDER BY id",
    )
    .bind(organization_id.as_str())
    .bind(membership_id.as_str())
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| ScopeError::AuthorityUnavailable)?;

    rows.into_iter()
        .map(|(id, role, state, state_version)| {
            let id = RoleAssignmentId::new(id).map_err(|_| ScopeError::AuthorityUnavailable)?;
            let role = Role::all()
                .iter()
                .copied()
                .find(|candidate| candidate.as_str() == role)
                .ok_or(ScopeError::AuthorityUnavailable)?;
            let state =
                persisted_role_assignment_state(&state).ok_or(ScopeError::AuthorityUnavailable)?;
            let state_version =
                u64::try_from(state_version).map_err(|_| ScopeError::AuthorityUnavailable)?;
            Ok(RoleAssignment {
                id,
                organization_id: organization_id.clone(),
                membership_id: membership_id.clone(),
                role,
                state,
                state_version,
            })
        })
        .collect()
}

async fn persisted_scope_grants(
    tx: &mut Transaction<'_, Postgres>,
    organization_id: &OrganizationId,
    membership_id: &MembershipId,
) -> Result<Vec<ScopeGrant>, ScopeError> {
    let rows: Vec<PersistedGrantRow> = sqlx_core::query_as::query_as(
        "SELECT id, scope_kind, branch_id, state, state_version FROM scope_grants \
         WHERE organization_id = $1 AND membership_id = $2 ORDER BY id",
    )
    .bind(organization_id.as_str())
    .bind(membership_id.as_str())
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| ScopeError::AuthorityUnavailable)?;

    rows.into_iter()
        .map(|(id, scope_kind, branch, state, state_version)| {
            let id = ScopeGrantId::new(id).map_err(|_| ScopeError::AuthorityUnavailable)?;
            let scope = match (scope_kind.as_str(), branch) {
                ("ORGANIZATION", None) => Scope::Organization {
                    organization_id: organization_id.clone(),
                },
                ("BRANCH", Some(id)) => Scope::Branch {
                    organization_id: organization_id.clone(),
                    branch_id: BranchId::new(id).map_err(|_| ScopeError::AuthorityUnavailable)?,
                },
                _ => return Err(ScopeError::AuthorityUnavailable),
            };
            let state = match state.as_str() {
                "ACTIVE" => ScopeGrantState::Active,
                "REVOKED" => ScopeGrantState::Revoked,
                _ => return Err(ScopeError::AuthorityUnavailable),
            };
            let state_version =
                u64::try_from(state_version).map_err(|_| ScopeError::AuthorityUnavailable)?;
            Ok(ScopeGrant {
                id,
                organization_id: organization_id.clone(),
                membership_id: membership_id.clone(),
                scope,
                state,
                state_version,
            })
        })
        .collect()
}

async fn persisted_member_roles(
    pool: &PgPool,
    organization_id: &OrganizationId,
    membership_id: &MembershipId,
) -> Result<Vec<RoleAssignment>, ScopeError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    establish_scope_transaction(&mut tx, organization_id).await?;
    let roles = persisted_role_assignments(&mut tx, organization_id, membership_id).await?;
    tx.commit()
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    Ok(roles)
}

async fn persisted_member_grants(
    pool: &PgPool,
    organization_id: &OrganizationId,
    membership_id: &MembershipId,
) -> Result<Vec<ScopeGrant>, ScopeError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    establish_scope_transaction(&mut tx, organization_id).await?;
    let grants = persisted_scope_grants(&mut tx, organization_id, membership_id).await?;
    tx.commit()
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    Ok(grants)
}

async fn persisted_member_permissions(
    pool: &PgPool,
    organization_id: &OrganizationId,
    membership_id: &MembershipId,
) -> Result<BTreeSet<Permission>, ScopeError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    establish_scope_transaction(&mut tx, organization_id).await?;
    let state: Option<(String,)> = sqlx_core::query_as::query_as(
        "SELECT state FROM organization_memberships \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(organization_id.as_str())
    .bind(membership_id.as_str())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| ScopeError::AuthorityUnavailable)?;
    let permissions = if matches!(state, Some((ref state,)) if state == "ACTIVE") {
        resolve_permissions(
            &persisted_role_assignments(&mut tx, organization_id, membership_id).await?,
        )
    } else {
        BTreeSet::new()
    };
    tx.commit()
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    Ok(permissions)
}

async fn establish_scope_transaction(
    tx: &mut Transaction<'_, Postgres>,
    organization_id: &OrganizationId,
) -> Result<(), ScopeError> {
    query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut **tx)
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    query("SELECT set_config('app.organization_id', $1, true)")
        .bind(organization_id.as_str())
        .execute(&mut **tx)
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    Ok(())
}

async fn establish_authorized_mutation_transaction(
    tx: &mut Transaction<'_, Postgres>,
    organization_id: &OrganizationId,
) -> Result<(), ScopeError> {
    // READ COMMITTED gives each statement a fresh snapshot after the advisory
    // lock is acquired. This matters when a request waits behind membership,
    // role, or scope revocation: its subsequent authority reads must see the
    // revocation before the command proceeds.
    query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut **tx)
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    query("SELECT set_config('app.organization_id', $1, true)")
        .bind(organization_id.as_str())
        .execute(&mut **tx)
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    Ok(())
}

async fn load_effective_scope(
    tx: &mut Transaction<'_, Postgres>,
    organization_id: &OrganizationId,
    selector: MembershipSelector<'_>,
    branch_id: Option<&BranchId>,
) -> Result<EffectiveScope, ScopeError> {
    let (membership_id, user_id) = match selector {
        MembershipSelector::Id(id) => (Some(id.as_str()), None),
        MembershipSelector::User(id) => (None, Some(id.as_str())),
    };
    let row: Option<PersistedScopeRow> = sqlx_core::query_as::query_as(
        "SELECT o.name, o.state, o.state_version, m.id, m.user_id, m.state, m.state_version, \
         b.id, b.name, b.state, b.state_version \
         FROM organizations o \
         JOIN organization_memberships m ON m.organization_id = o.id \
         LEFT JOIN branches b ON b.organization_id = o.id AND b.id = $3 \
         WHERE o.id = $1 AND \
         (($2::TEXT IS NOT NULL AND m.id = $2) OR ($4::TEXT IS NOT NULL AND m.user_id = $4))",
    )
    .bind(organization_id.as_str())
    .bind(membership_id)
    .bind(branch_id.map(BranchId::as_str))
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ScopeError::AuthorityUnavailable)?;
    let Some((
        organization_name,
        organization_state,
        organization_version,
        membership_id,
        membership_user_id,
        membership_state,
        membership_version,
        branch_id_raw,
        branch_name,
        branch_state,
        branch_version,
    )) = row
    else {
        return Err(ScopeError::OrganizationDenied);
    };

    let organization = Organization {
        id: organization_id.clone(),
        name: organization_name,
        state: match organization_state.as_str() {
            "PROVISIONING" => OrganizationState::Provisioning,
            "ACTIVE" => OrganizationState::Active,
            "SUSPENDED" => OrganizationState::Suspended,
            "CLOSING" => OrganizationState::Closing,
            "CLOSED" => OrganizationState::Closed,
            _ => return Err(ScopeError::AuthorityUnavailable),
        },
        state_version: u64::try_from(organization_version)
            .map_err(|_| ScopeError::AuthorityUnavailable)?,
    };
    let membership = Membership {
        id: MembershipId::new(membership_id).map_err(|_| ScopeError::AuthorityUnavailable)?,
        organization_id: organization_id.clone(),
        user_id: TenantUserId::new(membership_user_id)
            .map_err(|_| ScopeError::AuthorityUnavailable)?,
        state: match membership_state.as_str() {
            "INVITED" => MembershipState::Invited,
            "PENDING_ACCEPTANCE" => MembershipState::PendingAcceptance,
            "EXPIRED" => MembershipState::Expired,
            "ACTIVE" => MembershipState::Active,
            "SUSPENDED" => MembershipState::Suspended,
            "REVOKED" => MembershipState::Revoked,
            _ => return Err(ScopeError::AuthorityUnavailable),
        },
        state_version: u64::try_from(membership_version)
            .map_err(|_| ScopeError::AuthorityUnavailable)?,
    };
    let branch = match (
        branch_id,
        branch_id_raw,
        branch_name,
        branch_state,
        branch_version,
    ) {
        (None, None, None, None, None) => None,
        (Some(_), Some(id), Some(name), Some(state), Some(version)) => Some(Branch {
            id: BranchId::new(id).map_err(|_| ScopeError::AuthorityUnavailable)?,
            organization_id: organization_id.clone(),
            name,
            state: match state.as_str() {
                "PROVISIONING" => BranchState::Provisioning,
                "ACTIVE" => BranchState::Active,
                "SUSPENDED" => BranchState::Suspended,
                "CLOSING" => BranchState::Closing,
                "CLOSED" => BranchState::Closed,
                _ => return Err(ScopeError::AuthorityUnavailable),
            },
            state_version: u64::try_from(version).map_err(|_| ScopeError::AuthorityUnavailable)?,
        }),
        (Some(_), None, None, None, None) => return Err(ScopeError::BranchDenied),
        _ => return Err(ScopeError::AuthorityUnavailable),
    };
    resolve_effective_scope(&membership, &organization, branch.as_ref())
}

async fn authorize_postgres_operation(
    pool: &PgPool,
    organization_id: &OrganizationId,
    selector: MembershipSelector<'_>,
    branch_id: Option<&BranchId>,
    permission: Permission,
) -> Result<EffectiveScope, ScopeError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    establish_scope_transaction(&mut tx, organization_id).await?;
    let scope = authorize_postgres_operation_in_transaction(
        &mut tx,
        organization_id,
        selector,
        branch_id,
        permission,
    )
    .await?;
    tx.commit()
        .await
        .map_err(|_| ScopeError::AuthorityUnavailable)?;
    Ok(scope)
}

async fn authorize_postgres_operation_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    organization_id: &OrganizationId,
    selector: MembershipSelector<'_>,
    branch_id: Option<&BranchId>,
    permission: Permission,
) -> Result<EffectiveScope, ScopeError> {
    let mut scope = load_effective_scope(tx, organization_id, selector, branch_id).await?;
    let assignments: Vec<(String, String, i64)> = sqlx_core::query_as::query_as(
        "SELECT id, role, state_version FROM role_assignments \
         WHERE organization_id = $1 AND membership_id = $2 AND state = 'EFFECTIVE'",
    )
    .bind(organization_id.as_str())
    .bind(scope.membership_id.as_str())
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| ScopeError::AuthorityUnavailable)?;
    let assignments = assignments
        .into_iter()
        .map(|(id, role, state_version)| {
            let id = RoleAssignmentId::new(id).map_err(|_| ScopeError::AuthorityUnavailable)?;
            let role = Role::all()
                .iter()
                .copied()
                .find(|candidate| candidate.as_str() == role)
                .ok_or(ScopeError::AuthorityUnavailable)?;
            let state_version =
                u64::try_from(state_version).map_err(|_| ScopeError::AuthorityUnavailable)?;
            Ok(RoleAssignment {
                id,
                organization_id: organization_id.clone(),
                membership_id: scope.membership_id.clone(),
                role,
                state: RoleAssignmentState::Effective,
                state_version,
            })
        })
        .collect::<Result<Vec<_>, ScopeError>>()?;
    let permissions = resolve_permissions(&assignments);
    if !permissions.contains(&permission) {
        return Err(ScopeError::PermissionDenied);
    }

    let grants: Vec<(String, String, Option<String>, i64)> = sqlx_core::query_as::query_as(
        "SELECT id, scope_kind, branch_id, state_version FROM scope_grants \
         WHERE organization_id = $1 AND membership_id = $2 AND state = 'ACTIVE'",
    )
    .bind(organization_id.as_str())
    .bind(scope.membership_id.as_str())
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| ScopeError::AuthorityUnavailable)?;
    let grants = grants
        .into_iter()
        .map(|(id, scope_kind, branch, state_version)| {
            let id = ScopeGrantId::new(id).map_err(|_| ScopeError::AuthorityUnavailable)?;
            let grant_scope = match (scope_kind.as_str(), branch) {
                ("ORGANIZATION", None) => Scope::Organization {
                    organization_id: organization_id.clone(),
                },
                ("BRANCH", Some(id)) => Scope::Branch {
                    organization_id: organization_id.clone(),
                    branch_id: BranchId::new(id).map_err(|_| ScopeError::AuthorityUnavailable)?,
                },
                _ => return Err(ScopeError::AuthorityUnavailable),
            };
            Ok(ScopeGrant {
                id,
                organization_id: organization_id.clone(),
                membership_id: scope.membership_id.clone(),
                scope: grant_scope,
                state: ScopeGrantState::Active,
                state_version: u64::try_from(state_version)
                    .map_err(|_| ScopeError::AuthorityUnavailable)?,
            })
        })
        .collect::<Result<Vec<_>, ScopeError>>()?;
    let requested_scope = match &scope.branch_id {
        Some(branch_id) => Scope::Branch {
            organization_id: organization_id.clone(),
            branch_id: branch_id.clone(),
        },
        None => Scope::Organization {
            organization_id: organization_id.clone(),
        },
    };
    if !authorize_scope(&grants, &requested_scope) {
        return Err(ScopeError::ScopeDenied);
    }
    scope.permissions = permissions;
    Ok(scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_authz::ContactKind;

    fn org_id(value: &str) -> OrganizationId {
        OrganizationId::new(value).unwrap()
    }

    fn user(value: &str) -> TenantUserId {
        TenantUserId::new(value).unwrap()
    }

    #[test]
    fn persisted_role_assignment_state_rejects_unknown_values() {
        assert_eq!(
            persisted_role_assignment_state("EFFECTIVE"),
            Some(RoleAssignmentState::Effective)
        );
        assert_eq!(
            persisted_role_assignment_state("REVOKED"),
            Some(RoleAssignmentState::Revoked)
        );
        assert_eq!(persisted_role_assignment_state("effective"), None);
        assert_eq!(persisted_role_assignment_state("UNKNOWN"), None);
    }

    /// Test-only actor identity. No evidence sink is wired in these unit
    /// tests (`service()`/`strict_service()` never call `.with_evidence`),
    /// so this value is never persisted or asserted on here — it exists
    /// only to satisfy the `actor: ActorRef` parameter these methods now
    /// require. Postgres-backed evidence assertions live in
    /// `tests/part8_provision_organization_evidence_tests.rs`.
    fn actor(subject: &str) -> ActorRef {
        ActorRef {
            subject_ref: subject.to_string(),
            membership_ref: None,
            device_ref: None,
        }
    }

    fn service() -> TenancyService {
        TenancyService::new_reference(Arc::new(TenancyDatabase::new()), Vec::new())
    }

    fn strict_service() -> TenancyService {
        // Single-attempt rules: the second identical call locks (section 38).
        let rule = RateLimitRule {
            max_attempts: 1,
            window: Duration::from_secs(3_600),
            lockout: Duration::from_secs(3_600),
        };
        TenancyService::new_reference(
            Arc::new(TenancyDatabase::new()),
            vec![
                (AbuseClass::InvitationCreate, rule),
                (AbuseClass::InvitationAccept, rule),
            ],
        )
    }

    #[tokio::test]
    async fn unsupported_organization_mutation_fails_before_reference_store_write() {
        let pool = sqlx_postgres::PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy("postgres://app_runtime@127.0.0.1:1/sitolo_test")
            .unwrap();
        let db = Arc::new(TenancyDatabase::new());
        let service = TenancyService::new_reference(db.clone(), Vec::new())
            .for_reference_with_evidence(TenancyEvidenceSink::new(
                pool.clone(),
                Arc::new(sitolo_persistence::audit_repository::PostgresAuditWriter::new()),
                Arc::new(
                    sitolo_persistence::outbox_repository::PostgresOutboxRepository::new(pool),
                ),
            ));
        let organization_id = org_id("org-fail-closed");
        let result = service
            .activate_organization(&organization_id, actor("operator"))
            .await;
        assert!(result.is_err());
        assert!(db.organization_snapshot(&organization_id).await.is_none());
    }

    #[tokio::test]
    async fn authenticated_user_authorization_never_falls_back_to_reference_store() {
        let service = service();
        let bundle = provisioned(&service).await;

        let result = service
            .authorize_user_operation(
                &bundle.owner_membership.user_id,
                &bundle.organization.id,
                Some(&bundle.default_branch.id),
                Permission::SaleCreate,
            )
            .await;

        assert_eq!(result, Err(ScopeError::AuthorityUnavailable));
    }

    async fn provisioned(service: &TenancyService) -> ProvisionedOrganization {
        service
            .provision_organization(
                org_id("o1"),
                "Merchant",
                MembershipId::new("m-owner").unwrap(),
                user("owner"),
                BranchId::new("b1").unwrap(),
                "Main Branch",
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn provision_creates_operating_triple() {
        let service = service();
        let bundle = provisioned(&service).await;
        assert!(bundle.organization.can_operate());
        assert!(bundle.owner_membership.has_authority());
        assert!(bundle.default_branch.can_operate());
    }

    #[tokio::test]
    async fn member_lifecycle_round_trip() {
        let service = service();
        let bundle = provisioned(&service).await;
        let org = &bundle.organization.id;

        let invited = service
            .invite_member(
                MembershipId::new("m-cashier").unwrap(),
                org.clone(),
                user("cashier"),
                actor("test"),
            )
            .await
            .unwrap();
        assert!(!invited.has_authority());
        // Cannot skip to Active.
        assert_eq!(
            service.accept_member(org, &invited.id, actor("test")).await,
            Err(TenancyError::InvalidTransition)
        );
        service
            .mark_member_pending(org, &invited.id, actor("issuer"))
            .await
            .unwrap();
        let active = service
            .accept_member(org, &invited.id, actor("test"))
            .await
            .unwrap();
        assert!(active.has_authority());

        let scope = service
            .effective_scope(org, &active.id, Some(&bundle.default_branch.id))
            .await
            .unwrap();
        assert_eq!(scope.organization_id, *org);

        service
            .suspend_member(org, &active.id, actor("test"))
            .await
            .unwrap();
        assert_eq!(
            service.effective_scope(org, &active.id, None).await,
            Err(ScopeError::MembershipNotActive)
        );
        service
            .resume_member(org, &active.id, actor("test"))
            .await
            .unwrap();
        assert!(service.effective_scope(org, &active.id, None).await.is_ok());
        service
            .revoke_member(org, &active.id, actor("test"))
            .await
            .unwrap();
        assert_eq!(
            service.effective_scope(org, &active.id, None).await,
            Err(ScopeError::MembershipNotActive)
        );
    }

    #[tokio::test]
    async fn persisted_cross_tenant_scope_is_denied() {
        let service = service();
        let first = provisioned(&service).await;
        service
            .provision_organization(
                org_id("o2"),
                "Other Merchant",
                MembershipId::new("m-owner-2").unwrap(),
                user("owner-2"),
                BranchId::new("b2").unwrap(),
                "Other Branch",
            )
            .await
            .unwrap();
        // Tenant A membership against Tenant B organization: denied. The
        // scoped membership lookup misses first, so the denial surfaces as
        // MembershipNotActive — still fully non-disclosing.
        assert_eq!(
            service
                .effective_scope(&org_id("o2"), &first.owner_membership.id, None)
                .await,
            Err(ScopeError::MembershipNotActive)
        );
        // Tenant A membership with Tenant B branch: denied.
        assert_eq!(
            service
                .effective_scope(
                    &first.organization.id,
                    &first.owner_membership.id,
                    Some(&BranchId::new("b2").unwrap())
                )
                .await,
            Err(ScopeError::BranchDenied)
        );
    }

    #[tokio::test]
    async fn role_grants_resolve_to_least_privilege_permissions() {
        let service = service();
        let bundle = provisioned(&service).await;
        let org = &bundle.organization.id;

        // Owner holds the full catalog.
        service
            .assign_role(
                org.clone(),
                bundle.owner_membership.id.clone(),
                RoleAssignmentId::new("ra-owner").unwrap(),
                Role::Owner,
                actor("test"),
            )
            .await
            .unwrap();
        let owner_permissions = service
            .member_permissions(org, &bundle.owner_membership.id)
            .await;
        assert_eq!(owner_permissions.len(), Permission::all().len());

        // A cashier resolves to exactly sale capabilities.
        let cashier = service
            .invite_member(
                MembershipId::new("m-cashier").unwrap(),
                org.clone(),
                user("cashier"),
                actor("test"),
            )
            .await
            .unwrap();
        service
            .mark_member_pending(org, &cashier.id, actor("issuer"))
            .await
            .unwrap();
        service
            .accept_member(org, &cashier.id, actor("test"))
            .await
            .unwrap();
        service
            .assign_role(
                org.clone(),
                cashier.id.clone(),
                RoleAssignmentId::new("ra-cashier").unwrap(),
                Role::Cashier,
                actor("test"),
            )
            .await
            .unwrap();
        assert_eq!(
            service.member_permissions(org, &cashier.id).await,
            BTreeSet::from([Permission::SaleCreate, Permission::SaleView])
        );

        // Revocation drains authority without deleting history.
        service
            .revoke_role(org, &cashier.id, Role::Cashier, actor("test"))
            .await
            .unwrap();
        assert!(
            service
                .member_permissions(org, &cashier.id)
                .await
                .is_empty()
        );
        assert_eq!(service.member_roles(org, &cashier.id).await.len(), 1);

        // Suspended memberships resolve to nothing while suspended.
        service
            .assign_role(
                org.clone(),
                cashier.id.clone(),
                RoleAssignmentId::new("ra-cashier-2").unwrap(),
                Role::Cashier,
                actor("test"),
            )
            .await
            .unwrap();
        service
            .suspend_member(org, &cashier.id, actor("test"))
            .await
            .unwrap();
        assert!(
            service
                .member_permissions(org, &cashier.id)
                .await
                .is_empty()
        );

        // Unknown memberships resolve to nothing without disclosure.
        assert!(
            service
                .member_permissions(org, &MembershipId::new("m-ghost").unwrap())
                .await
                .is_empty()
        );
    }

    const INVITE_TOKEN: &str = "tok-invite-accept-0123456789abcdef";
    const INVITE_TOKEN_2: &str = "tok-invite-second-0123456789abcdef";

    fn mailto(value: &str) -> InvitationContact {
        InvitationContact::new(ContactKind::Email, value).unwrap()
    }

    async fn invited(
        service: &TenancyService,
        org: &OrganizationId,
        token: &str,
    ) -> CreatedInvitation {
        service
            .create_invitation(
                org.clone(),
                InvitationId::new("inv-1").unwrap(),
                MembershipId::new("m-invited").unwrap(),
                user("invitee"),
                Role::Cashier,
                None,
                mailto("invitee@example.com"),
                token,
                Duration::from_secs(3_600),
                actor("test"),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn invitation_accept_activates_with_record_authority() {
        let service = service();
        let bundle = provisioned(&service).await;
        let org = &bundle.organization.id;
        let created = invited(&service, org, INVITE_TOKEN).await;
        assert!(!created.membership.has_authority());

        // Acceptance takes no role or scope parameters: authority comes
        // from the server-loaded record only (sections 9.2, T4). A tampered
        // client cannot smuggle OWNER through any argument.
        let accepted = service
            .accept_invitation(
                INVITE_TOKEN,
                RoleAssignmentId::new("ra-inv").unwrap(),
                None,
                actor("test"),
            )
            .await
            .unwrap();
        assert!(accepted.membership.has_authority());
        assert_eq!(accepted.assignment.role, Role::Cashier);
        assert_eq!(accepted.scope_grant, None);
        assert_eq!(
            accepted.invitation.state,
            sitolo_authz::InvitationState::Accepted
        );
        assert!(accepted.invitation.contact.is_none());
        assert!(accepted.invitation.token_hash.is_none());

        // The persistence boundary deletes the consumed token hash, so replay
        // is indistinguishable from an unknown token; no grant is duplicated.
        assert_eq!(
            service
                .accept_invitation(
                    INVITE_TOKEN,
                    RoleAssignmentId::new("ra-replay").unwrap(),
                    None,
                    actor("test")
                )
                .await,
            Err(InvitationError::Invalid)
        );
        assert_eq!(
            service
                .member_roles(org, &created.membership.id)
                .await
                .len(),
            1
        );

        // Unknown tokens are generically invalid.
        assert_eq!(
            service
                .accept_invitation(
                    "tok-unknown-0123456789abcdefghijkl",
                    RoleAssignmentId::new("ra-ghost").unwrap(),
                    None,
                    actor("test")
                )
                .await,
            Err(InvitationError::Invalid)
        );
        // Malformed tokens never reach the store.
        assert_eq!(
            service
                .accept_invitation(
                    "short",
                    RoleAssignmentId::new("ra-x").unwrap(),
                    None,
                    actor("test")
                )
                .await,
            Err(InvitationError::Invalid)
        );
    }

    #[tokio::test]
    async fn invitation_applies_proposed_scope_grant() {
        let service = service();
        let bundle = provisioned(&service).await;
        let org = &bundle.organization.id;
        let branch = &bundle.default_branch.id;

        service
            .create_invitation(
                org.clone(),
                InvitationId::new("inv-scoped").unwrap(),
                MembershipId::new("m-scoped").unwrap(),
                user("scoped"),
                Role::BranchManager,
                Some(Scope::Branch {
                    organization_id: org.clone(),
                    branch_id: branch.clone(),
                }),
                mailto("scoped@example.com"),
                INVITE_TOKEN_2,
                Duration::from_secs(3_600),
                actor("test"),
            )
            .await
            .unwrap();
        // Accepting without the accompanying grant id is rejected: the
        // record demands a scope grant the caller did not mint.
        assert_eq!(
            service
                .accept_invitation(
                    INVITE_TOKEN_2,
                    RoleAssignmentId::new("ra-scoped").unwrap(),
                    None,
                    actor("test")
                )
                .await,
            Err(InvitationError::Invalid)
        );
        let accepted = service
            .accept_invitation(
                INVITE_TOKEN_2,
                RoleAssignmentId::new("ra-scoped").unwrap(),
                Some(ScopeGrantId::new("g-scoped").unwrap()),
                actor("test"),
            )
            .await
            .unwrap();
        let grant = accepted.scope_grant.expect("proposed scope granted");
        assert_eq!(
            grant.scope,
            Scope::Branch {
                organization_id: org.clone(),
                branch_id: branch.clone(),
            }
        );
        // The narrowed member authorizes inside the grant only.
        service
            .authorize_operation(
                org,
                &accepted.membership.id,
                Some(branch),
                Permission::SaleCreate,
            )
            .await
            .unwrap();
        assert_eq!(
            service
                .authorize_operation(org, &accepted.membership.id, None, Permission::SaleCreate)
                .await,
            Err(ScopeError::ScopeDenied)
        );
    }

    #[tokio::test]
    async fn revoked_invitation_releases_membership_for_reinvite() {
        let service = service();
        let bundle = provisioned(&service).await;
        let org = &bundle.organization.id;

        let created = invited(&service, org, INVITE_TOKEN).await;
        let withdrawn = service
            .revoke_invitation(org, &created.invitation.id, actor("test"))
            .await
            .unwrap();
        assert!(withdrawn.contact.is_none());
        assert!(withdrawn.token_hash.is_none());
        // The withdrawn token is generically invalid.
        assert_eq!(
            service
                .accept_invitation(
                    INVITE_TOKEN,
                    RoleAssignmentId::new("ra-dead").unwrap(),
                    None,
                    actor("test")
                )
                .await,
            Err(InvitationError::Invalid)
        );
        // The linked membership expired, freeing re-enrollment.
        let membership = service
            .db
            .membership_snapshot(org, &created.membership.id)
            .await
            .unwrap();
        assert!(!membership.has_authority());
        service
            .create_invitation(
                org.clone(),
                InvitationId::new("inv-2").unwrap(),
                MembershipId::new("m-reinvited").unwrap(),
                user("invitee"),
                Role::Cashier,
                None,
                mailto("invitee@example.com"),
                INVITE_TOKEN_2,
                Duration::from_secs(3_600),
                actor("test"),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn invitation_paths_are_rate_limited() {
        let service = strict_service();
        let bundle = provisioned(&service).await;
        let org = &bundle.organization.id;

        service
            .create_invitation(
                org.clone(),
                InvitationId::new("inv-a").unwrap(),
                MembershipId::new("m-a").unwrap(),
                user("a"),
                Role::Viewer,
                None,
                mailto("a@example.com"),
                INVITE_TOKEN,
                Duration::from_secs(3_600),
                actor("test"),
            )
            .await
            .unwrap();
        // Second creation for the organization locks (section 38).
        assert_eq!(
            service
                .create_invitation(
                    org.clone(),
                    InvitationId::new("inv-b").unwrap(),
                    MembershipId::new("m-b").unwrap(),
                    user("b"),
                    Role::Viewer,
                    None,
                    mailto("b@example.com"),
                    INVITE_TOKEN_2,
                    Duration::from_secs(3_600),
                    actor("test"),
                )
                .await,
            Err(TenancyError::RateLimited)
        );

        // Acceptance throttle precedes redemption: the first unknown-token
        // attempt consumes the single attempt, and the retry reports
        // throttling instead of reaching the store.
        let throttle = strict_service();
        assert_eq!(
            throttle
                .accept_invitation(
                    INVITE_TOKEN,
                    RoleAssignmentId::new("ra-t").unwrap(),
                    None,
                    actor("test")
                )
                .await,
            Err(InvitationError::Invalid)
        );
        assert_eq!(
            throttle
                .accept_invitation(
                    INVITE_TOKEN,
                    RoleAssignmentId::new("ra-t").unwrap(),
                    None,
                    actor("test")
                )
                .await,
            Err(InvitationError::RateLimited)
        );
    }

    #[tokio::test]
    async fn authorize_operation_intersects_permission_and_scope() {
        use sitolo_domain::tenancy::ScopeGrantId;

        let service = service();
        let bundle = provisioned(&service).await;
        let org = &bundle.organization.id;
        let owner = &bundle.owner_membership.id;
        let main_branch = &bundle.default_branch.id;

        service
            .assign_role(
                org.clone(),
                owner.clone(),
                RoleAssignmentId::new("ra-owner").unwrap(),
                Role::Owner,
                actor("test"),
            )
            .await
            .unwrap();
        let second_branch = service
            .create_branch(
                BranchId::new("b2").unwrap(),
                org.clone(),
                "Second Branch",
                actor("test"),
            )
            .await
            .unwrap();
        service
            .activate_branch(org, &second_branch.id, actor("test"))
            .await
            .unwrap();

        // No grants narrow: organization-wide and branch operations allowed.
        let access = service
            .authorize_operation(org, owner, None, Permission::SaleCreate)
            .await
            .unwrap();
        assert!(access.permissions.contains(&Permission::SaleCreate));
        service
            .authorize_operation(org, owner, Some(main_branch), Permission::SaleCreate)
            .await
            .unwrap();

        // Narrow the owner to the main branch.
        service
            .grant_scope(
                org.clone(),
                owner.clone(),
                ScopeGrantId::new("g-main").unwrap(),
                Scope::Branch {
                    organization_id: org.clone(),
                    branch_id: main_branch.clone(),
                },
                actor("test"),
            )
            .await
            .unwrap();
        assert!(
            service
                .authorize_operation(org, owner, None, Permission::SaleCreate)
                .await
                .is_err_and(|err| err == ScopeError::ScopeDenied)
        );
        service
            .authorize_operation(org, owner, Some(main_branch), Permission::SaleCreate)
            .await
            .unwrap();
        assert!(
            service
                .authorize_operation(org, owner, Some(&second_branch.id), Permission::SaleCreate)
                .await
                .is_err_and(|err| err == ScopeError::ScopeDenied)
        );

        // A cashier holds the scope but lacks the permission.
        let cashier = service
            .invite_member(
                MembershipId::new("m-cashier").unwrap(),
                org.clone(),
                user("cashier"),
                actor("test"),
            )
            .await
            .unwrap();
        service
            .mark_member_pending(org, &cashier.id, actor("issuer"))
            .await
            .unwrap();
        service
            .accept_member(org, &cashier.id, actor("test"))
            .await
            .unwrap();
        service
            .assign_role(
                org.clone(),
                cashier.id.clone(),
                RoleAssignmentId::new("ra-cashier").unwrap(),
                Role::Cashier,
                actor("test"),
            )
            .await
            .unwrap();
        service
            .grant_scope(
                org.clone(),
                cashier.id.clone(),
                ScopeGrantId::new("g-cashier").unwrap(),
                Scope::Branch {
                    organization_id: org.clone(),
                    branch_id: main_branch.clone(),
                },
                actor("test"),
            )
            .await
            .unwrap();
        assert!(
            service
                .authorize_operation(org, &cashier.id, Some(main_branch), Permission::SaleVoid)
                .await
                .is_err_and(|err| err == ScopeError::PermissionDenied)
        );
        service
            .authorize_operation(org, &cashier.id, Some(main_branch), Permission::SaleCreate)
            .await
            .unwrap();

        // Lifting the narrowing restores organization-wide authority.
        service
            .revoke_scope_grant(org, &ScopeGrantId::new("g-main").unwrap(), actor("test"))
            .await
            .unwrap();
        service
            .authorize_operation(org, owner, None, Permission::SaleCreate)
            .await
            .unwrap();
    }
}
