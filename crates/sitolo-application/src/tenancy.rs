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
//! Invitation delivery remains an outbox concern. Caller authorization is
//! implemented only by the explicit authenticated command methods; generic
//! actor-taking methods remain reference/evidence operations and must not be
//! wired directly to production HTTP requests.
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
//! Most request-driven methods still take an `actor: ActorRef`. The explicit
//! authenticated branch-create, branch-activate, and membership-suspend
//! commands instead derive actor provenance from `SecurityContext` and
//! authorize against persisted membership state in the mutation transaction.
//! Token-bound acceptance derives its actor from the user bound to the locked
//! invitation membership.
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
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat, Utc};
use sitolo_audit::{ActorRef, IamAuditEvent, IamEventName, TargetRef};
use sitolo_auth::{
    AbuseClass, AuditEventId, RateLimitDecision, RateLimitRule, RateLimiter, SecurityContext,
};
use sitolo_authz::{
    Invitation, InvitationContact, InvitationError, InvitationState, Permission, Role,
    RoleAssignment, Scope, ScopeGrant, authorize_scope, resolve_permissions,
};
use sitolo_domain::tenancy::{
    Branch, BranchId, BranchState, InvitationId, Membership, MembershipId, MembershipState,
    Organization, OrganizationId, OrganizationState, ProvisionedOrganization, RoleAssignmentId,
    ScopeGrantId, TenancyError, TenantUserId,
};
use sitolo_events::OutboxEvent;
use sitolo_tenancy::{EffectiveScope, ScopeError, resolve_effective_scope};

use crate::tenancy_ports::{
    AcceptedInvitation, CreateInvitationInput, CreatedInvitation, InvitationRecord,
    MembershipSelector, RoleAssignmentOutcome, TenancyStores, TenancyTransaction,
    TenancyUnitOfWork,
};

#[derive(Clone, Copy)]
struct LifecycleEvidenceSpec {
    from: &'static str,
    to: &'static str,
    event_name: IamEventName,
    action: &'static str,
}

/// Borrowed invitation-acceptance context shared by the phase helpers.
struct InvitationAcceptance<'a> {
    organization_id: &'a OrganizationId,
    record: &'a InvitationRecord,
    actor: &'a ActorRef,
    membership_state: MembershipState,
}

fn system_time_to_utc(value: SystemTime) -> Result<DateTime<Utc>, TenancyError> {
    let duration = value
        .duration_since(UNIX_EPOCH)
        .map_err(|_error| TenancyError::InvalidInvitation)?;
    let seconds =
        i64::try_from(duration.as_secs()).map_err(|_error| TenancyError::InvalidInvitation)?;
    DateTime::from_timestamp(seconds, duration.subsec_nanos())
        .ok_or(TenancyError::InvalidInvitation)
}

/// The tenancy service (Phase 4 use cases).
///
/// Production composition supplies a durable [`TenancyUnitOfWork`]; the
/// in-memory reference store is supplied only by local/test composition. The
/// two are mutually exclusive so an authoritative deployment cannot silently
/// fall back to non-durable state.
pub struct TenancyService {
    stores: Option<Arc<dyn TenancyStores>>,
    unit_of_work: Option<Arc<dyn TenancyUnitOfWork>>,
    /// Test-only evidence channel used by the in-memory reference service.
    /// It never mutates business state; it only proves that mandatory evidence
    /// writes can be exercised without a tenancy schema.
    evidence: Option<Arc<dyn TenancyUnitOfWork>>,
    limiter: Mutex<RateLimiter>,
    abuse_rules: Vec<(AbuseClass, RateLimitRule)>,
}

impl fmt::Debug for TenancyService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let backend = if self.unit_of_work.is_some() {
            "durable"
        } else if self.stores.is_some() {
            "reference"
        } else {
            "unconfigured"
        };
        f.debug_struct("TenancyService")
            .field("backend", &backend)
            .field("evidence_configured", &self.evidence.is_some())
            .field("abuse_rule_count", &self.abuse_rules.len())
            .finish_non_exhaustive()
    }
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
    /// Constructs a tenancy service backed by the durable transactional port.
    /// Production composition must use this constructor; there is no
    /// in-memory fallback behind it.
    ///
    /// # Errors
    /// This constructor does not perform I/O and cannot fail. The runtime
    /// must establish database readiness before publishing the service.
    ///
    /// # Panics
    /// This constructor does not panic.
    #[must_use]
    pub fn new_postgres(
        abuse_rules: Vec<(AbuseClass, RateLimitRule)>,
        unit_of_work: Arc<dyn TenancyUnitOfWork>,
    ) -> Self {
        Self {
            stores: None,
            unit_of_work: Some(unit_of_work),
            evidence: None,
            limiter: Mutex::new(RateLimiter::new()),
            abuse_rules,
        }
    }

    /// Returns the durable unit of work, or a fail-closed error when the
    /// service was composed with the in-memory reference store.
    fn durable(&self) -> Result<&Arc<dyn TenancyUnitOfWork>, TenancyError> {
        self.unit_of_work
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailedAt {
                stage: "durable_unit_of_work_unavailable",
                sqlstate: None,
            })
    }

    /// Returns the in-memory reference store, or a fail-closed error when the
    /// service was composed with the durable unit of work.
    fn reference(&self) -> Result<&Arc<dyn TenancyStores>, TenancyError> {
        self.stores
            .as_ref()
            .ok_or(TenancyError::EvidencePersistenceFailedAt {
                stage: "reference_store_unavailable",
                sqlstate: None,
            })
    }

    /// Changes one membership state and writes its mandatory evidence using
    /// the same durable transaction.
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
        let mut tx = self.durable()?.begin_mutation(organization_id).await?;
        let membership = tx
            .transition_membership(organization_id, membership_id, expected_state, next_state)
            .await?;
        self.record_evidence_in_transaction(
            tx.as_mut(),
            event_name,
            organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            actor,
            action,
            "membership",
            membership_id.to_string(),
            serde_json::json!({
                "membership_id": membership_id.as_str(),
                "from_state": expected_state,
                "to_state": next_state,
                "state_version": membership.state_version,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(membership)
    }

    async fn assign_role_with_evidence(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        assignment_id: &RoleAssignmentId,
        role: Role,
        actor: Option<ActorRef>,
        security_context: Option<&SecurityContext>,
    ) -> Result<RoleAssignment, AuthorizedTenancyError> {
        let mut tx = self.durable()?.begin_mutation(organization_id).await?;
        tx.lock_organization(organization_id).await?;
        let actor = if let Some(security_context) = security_context {
            let user_id =
                TenantUserId::new(security_context.principal.subject_id.as_str().to_owned())
                    .map_err(|_error| ScopeError::AuthorityUnavailable)?;
            let authority = authorize_operation_in_transaction(
                tx.as_mut(),
                organization_id,
                MembershipSelector::User(&user_id),
                None,
                Permission::RoleAssign,
            )
            .await?;
            ActorRef {
                subject_ref: security_context.principal.subject_id.as_str().to_owned(),
                membership_ref: Some(authority.membership_id.as_str().to_owned()),
                device_ref: security_context
                    .principal
                    .device_id
                    .as_ref()
                    .map(|device_id| device_id.as_str().to_owned()),
            }
        } else {
            actor.ok_or(TenancyError::EvidencePersistenceFailed)?
        };

        let Some(membership) = tx.membership_record(organization_id, membership_id).await? else {
            return Err(TenancyError::NotFound.into());
        };
        if matches!(
            membership.state,
            MembershipState::Expired | MembershipState::Revoked
        ) {
            return Err(TenancyError::InvalidTransition.into());
        }

        let outcome = tx
            .insert_role_assignment(organization_id, membership_id, assignment_id, role)
            .await?;
        let assignment = match outcome {
            RoleAssignmentOutcome::Created(assignment) => {
                self.record_evidence_in_transaction(
                    tx.as_mut(),
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
                assignment
            }
            RoleAssignmentOutcome::Replayed(assignment) => assignment,
        };
        tx.commit()
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
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
        actor: Option<ActorRef>,
        security_context: Option<&SecurityContext>,
    ) -> Result<RoleAssignment, AuthorizedTenancyError> {
        let mut tx = self.durable()?.begin_mutation(organization_id).await?;
        tx.lock_organization(organization_id).await?;
        let actor = if let Some(security_context) = security_context {
            let user_id =
                TenantUserId::new(security_context.principal.subject_id.as_str().to_owned())
                    .map_err(|_error| ScopeError::AuthorityUnavailable)?;
            let authority = authorize_operation_in_transaction(
                tx.as_mut(),
                organization_id,
                MembershipSelector::User(&user_id),
                None,
                Permission::RoleAssign,
            )
            .await?;
            ActorRef {
                subject_ref: security_context.principal.subject_id.as_str().to_owned(),
                membership_ref: Some(authority.membership_id.as_str().to_owned()),
                device_ref: security_context
                    .principal
                    .device_id
                    .as_ref()
                    .map(|device_id| device_id.as_str().to_owned()),
            }
        } else {
            actor.ok_or(TenancyError::EvidencePersistenceFailed)?
        };

        let assignment = tx
            .revoke_role_assignment(organization_id, membership_id, role)
            .await?;
        self.record_evidence_in_transaction(
            tx.as_mut(),
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
                "role": role.as_str(),
                "state": "REVOKED",
                "state_version": assignment.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
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
        actor: Option<ActorRef>,
        security_context: Option<&SecurityContext>,
    ) -> Result<ScopeGrant, AuthorizedTenancyError> {
        if scope.organization_id() != organization_id {
            return Err(TenancyError::InvalidTransition.into());
        }
        let (scope_kind, branch_id) = match &scope {
            Scope::Organization { .. } => ("ORGANIZATION", None),
            Scope::Branch { branch_id, .. } => ("BRANCH", Some(branch_id.as_str().to_owned())),
        };
        let mut tx = self.durable()?.begin_mutation(organization_id).await?;
        tx.lock_organization(organization_id).await?;
        let actor = if let Some(security_context) = security_context {
            let user_id =
                TenantUserId::new(security_context.principal.subject_id.as_str().to_owned())
                    .map_err(|_error| ScopeError::AuthorityUnavailable)?;
            let authority = authorize_operation_in_transaction(
                tx.as_mut(),
                organization_id,
                MembershipSelector::User(&user_id),
                None,
                Permission::ScopeGrant,
            )
            .await?;
            ActorRef {
                subject_ref: security_context.principal.subject_id.as_str().to_owned(),
                membership_ref: Some(authority.membership_id.as_str().to_owned()),
                device_ref: security_context
                    .principal
                    .device_id
                    .as_ref()
                    .map(|device_id| device_id.as_str().to_owned()),
            }
        } else {
            actor.ok_or(TenancyError::EvidencePersistenceFailed)?
        };
        let Some(membership) = tx.membership_record(organization_id, membership_id).await? else {
            return Err(TenancyError::NotFound.into());
        };
        if matches!(
            membership.state,
            MembershipState::Expired | MembershipState::Revoked
        ) {
            return Err(TenancyError::InvalidTransition.into());
        }

        if let Some(existing) = tx.scope_grant(grant_id).await? {
            if existing.organization_id != *organization_id
                || existing.membership_id != *membership_id
                || existing.scope != scope
            {
                return Err(TenancyError::Conflict.into());
            }
            return Ok(existing);
        }
        let grant = tx
            .insert_scope_grant(organization_id, membership_id, grant_id, &scope)
            .await?;
        self.record_evidence_in_transaction(
            tx.as_mut(),
            IamEventName::ScopeGranted,
            organization_id,
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
                "scope_kind": scope_kind,
                "branch_id": branch_id,
                "state": "ACTIVE",
                "state_version": grant.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
                stage: "scope_grant_commit",
                sqlstate: None,
            })?;
        Ok(grant)
    }

    async fn revoke_scope_with_evidence(
        &self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
        actor: Option<ActorRef>,
        security_context: Option<&SecurityContext>,
    ) -> Result<ScopeGrant, AuthorizedTenancyError> {
        let mut tx = self.durable()?.begin_mutation(organization_id).await?;
        tx.lock_organization(organization_id).await?;
        let actor = if let Some(security_context) = security_context {
            let user_id =
                TenantUserId::new(security_context.principal.subject_id.as_str().to_owned())
                    .map_err(|_error| ScopeError::AuthorityUnavailable)?;
            let authority = authorize_operation_in_transaction(
                tx.as_mut(),
                organization_id,
                MembershipSelector::User(&user_id),
                None,
                Permission::ScopeGrant,
            )
            .await?;
            ActorRef {
                subject_ref: security_context.principal.subject_id.as_str().to_owned(),
                membership_ref: Some(authority.membership_id.as_str().to_owned()),
                device_ref: security_context
                    .principal
                    .device_id
                    .as_ref()
                    .map(|device_id| device_id.as_str().to_owned()),
            }
        } else {
            actor.ok_or(TenancyError::EvidencePersistenceFailed)?
        };
        let grant = tx.revoke_scope_grant(organization_id, grant_id).await?;
        self.record_evidence_in_transaction(
            tx.as_mut(),
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
                "membership_id": grant.membership_id.as_str(),
                "grant_id": grant_id.as_str(),
                "state": "REVOKED",
                "state_version": grant.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
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
        let unit = self
            .unit_of_work
            .as_ref()
            .ok_or(InvitationError::EvidencePersistenceFailed)?;
        let mut tx = unit.begin_invitation().await?;
        let Some(organization_id) = tx.resolve_invitation_organization(&token_hash).await? else {
            return Err(InvitationError::Invalid);
        };
        tx.set_tenant_context(&organization_id)
            .await
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
        tx.lock_organization(&organization_id)
            .await
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;

        let Some(record) = tx
            .lock_issued_invitation(&organization_id, &token_hash)
            .await?
        else {
            return Err(InvitationError::Invalid);
        };
        if now >= record.expires_at {
            return Err(InvitationError::Expired);
        }
        if record.proposed_scope.is_some() != grant_id.is_some() {
            return Err(InvitationError::Invalid);
        }
        let Some(membership) = tx
            .membership_record(&organization_id, &record.membership_id)
            .await
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?
        else {
            return Err(InvitationError::Invalid);
        };
        let actor = ActorRef {
            subject_ref: membership.user_id.as_str().to_owned(),
            membership_ref: Some(record.membership_id.as_str().to_owned()),
            device_ref: None,
        };
        let acceptance = InvitationAcceptance {
            organization_id: &organization_id,
            record: &record,
            actor: &actor,
            membership_state: membership.state,
        };
        self.activate_invitation_membership(tx.as_mut(), &acceptance)
            .await?;
        let assignment = self
            .assign_invitation_role(tx.as_mut(), &acceptance, &assignment_id)
            .await?;
        let scope_grant = match grant_id {
            Some(grant_id) => Some(
                self.grant_invitation_scope(tx.as_mut(), &acceptance, &grant_id)
                    .await?,
            ),
            None => None,
        };
        let invitation = self.complete_invitation(tx.as_mut(), &acceptance).await?;
        tx.commit()
            .await
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;

        let mut accepted_membership = Membership::invite(
            record.membership_id.clone(),
            organization_id,
            membership.user_id,
        );
        if membership.state == MembershipState::Invited {
            accepted_membership
                .mark_pending()
                .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
        }
        accepted_membership
            .activate()
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
        Ok(AcceptedInvitation {
            invitation,
            membership: accepted_membership,
            assignment,
            scope_grant,
        })
    }

    async fn activate_invitation_membership(
        &self,
        tx: &mut dyn TenancyTransaction,
        acceptance: &InvitationAcceptance<'_>,
    ) -> Result<(), InvitationError> {
        let organization_id = acceptance.organization_id;
        let record = acceptance.record;
        if acceptance.membership_state == MembershipState::Invited {
            let version = tx
                .transition_invitation_membership(
                    organization_id,
                    &record.membership_id,
                    "INVITED",
                    "PENDING_ACCEPTANCE",
                )
                .await?;
            self.record_evidence_in_transaction(
                tx,
                IamEventName::MembershipPendingAcceptance,
                organization_id,
                None,
                TargetRef::Membership(record.membership_id.clone()),
                acceptance.actor.clone(),
                "accept_invitation_pending",
                "membership",
                record.membership_id.to_string(),
                serde_json::json!({
                    "membership_id": record.membership_id.as_str(),
                    "from_state": "INVITED",
                    "to_state": "PENDING_ACCEPTANCE",
                    "state_version": version,
                }),
            )
            .await
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
        } else if acceptance.membership_state != MembershipState::PendingAcceptance {
            return Err(InvitationError::Invalid);
        }
        let active_version = tx
            .transition_invitation_membership(
                organization_id,
                &record.membership_id,
                "PENDING_ACCEPTANCE",
                "ACTIVE",
            )
            .await?;
        self.record_evidence_in_transaction(
            tx,
            IamEventName::MembershipActivated,
            organization_id,
            None,
            TargetRef::Membership(record.membership_id.clone()),
            acceptance.actor.clone(),
            "accept_invitation_activate",
            "membership",
            record.membership_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": record.membership_id.as_str(),
                "state": "ACTIVE",
                "state_version": active_version,
            }),
        )
        .await
        .map_err(|_error| InvitationError::EvidencePersistenceFailed)
    }

    async fn assign_invitation_role(
        &self,
        tx: &mut dyn TenancyTransaction,
        acceptance: &InvitationAcceptance<'_>,
        assignment_id: &RoleAssignmentId,
    ) -> Result<RoleAssignment, InvitationError> {
        let organization_id = acceptance.organization_id;
        let record = acceptance.record;
        let assignment = tx
            .insert_invitation_role_assignment(
                organization_id,
                &record.membership_id,
                assignment_id,
                record.proposed_role,
            )
            .await?;
        self.record_evidence_in_transaction(
            tx,
            IamEventName::RoleAssigned,
            organization_id,
            None,
            TargetRef::Membership(record.membership_id.clone()),
            acceptance.actor.clone(),
            "accept_invitation_assign_role",
            "role_assignment",
            assignment.id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": record.membership_id.as_str(),
                "assignment_id": assignment.id.as_str(),
                "role": record.proposed_role.as_str(),
                "state": "EFFECTIVE",
                "state_version": assignment.state_version,
            }),
        )
        .await
        .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
        Ok(assignment)
    }

    async fn grant_invitation_scope(
        &self,
        tx: &mut dyn TenancyTransaction,
        acceptance: &InvitationAcceptance<'_>,
        grant_id: &ScopeGrantId,
    ) -> Result<ScopeGrant, InvitationError> {
        let organization_id = acceptance.organization_id;
        let record = acceptance.record;
        let Some(scope) = record.proposed_scope.clone() else {
            return Err(InvitationError::Invalid);
        };
        let (scope_kind, branch_id) = match &scope {
            Scope::Organization { .. } => ("ORGANIZATION", None),
            Scope::Branch { branch_id, .. } => ("BRANCH", Some(branch_id.as_str().to_owned())),
        };
        let grant = tx
            .insert_invitation_scope_grant(organization_id, &record.membership_id, grant_id, &scope)
            .await?;
        self.record_evidence_in_transaction(
            tx,
            IamEventName::ScopeGranted,
            organization_id,
            None,
            TargetRef::Membership(record.membership_id.clone()),
            acceptance.actor.clone(),
            "accept_invitation_grant_scope",
            "scope_grant",
            grant_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": record.membership_id.as_str(),
                "grant_id": grant_id.as_str(),
                "scope_kind": scope_kind,
                "branch_id": branch_id,
                "state": "ACTIVE",
                "state_version": grant.state_version,
            }),
        )
        .await
        .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
        Ok(grant)
    }

    async fn complete_invitation(
        &self,
        tx: &mut dyn TenancyTransaction,
        acceptance: &InvitationAcceptance<'_>,
    ) -> Result<Invitation, InvitationError> {
        let organization_id = acceptance.organization_id;
        let record = acceptance.record;
        let invitation_version = tx.accept_invitation(organization_id, &record.id).await?;
        let invitation = Invitation {
            id: record.id.clone(),
            organization_id: organization_id.clone(),
            membership_id: record.membership_id.clone(),
            proposed_role: record.proposed_role,
            proposed_scope: record.proposed_scope.clone(),
            contact: None,
            token_hash: None,
            issued_at: record.issued_at,
            expires_at: record.expires_at,
            state: InvitationState::Accepted,
            state_version: invitation_version,
        };
        self.record_evidence_in_transaction(
            tx,
            IamEventName::MembershipInvitationAccepted,
            organization_id,
            None,
            TargetRef::Invitation(record.id.clone()),
            acceptance.actor.clone(),
            "accept_invitation",
            "invitation",
            record.id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "invitation_id": record.id.as_str(),
                "membership_id": record.membership_id.as_str(),
                "state": "ACCEPTED",
                "state_version": invitation.state_version,
            }),
        )
        .await
        .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
        Ok(invitation)
    }

    async fn invite_member_with_evidence(
        &self,
        membership_id: MembershipId,
        organization_id: OrganizationId,
        user_id: TenantUserId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        let membership =
            Membership::invite(membership_id.clone(), organization_id.clone(), user_id);
        let mut tx = self.durable()?.begin_mutation(&organization_id).await?;
        let organization_state = tx.lock_organization(&organization_id).await?;
        if organization_state.as_deref() != Some("ACTIVE") {
            return Err(TenancyError::InvalidTransition);
        }
        tx.insert_membership(&membership).await?;
        self.record_evidence_in_transaction(
            tx.as_mut(),
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
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
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
        stores: Arc<dyn TenancyStores>,
        abuse_rules: Vec<(AbuseClass, RateLimitRule)>,
    ) -> Self {
        TenancyService {
            stores: Some(stores),
            unit_of_work: None,
            evidence: None,
            limiter: Mutex::new(RateLimiter::new()),
            abuse_rules,
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
    pub fn for_reference_with_evidence(mut self, unit_of_work: Arc<dyn TenancyUnitOfWork>) -> Self {
        self.evidence = Some(unit_of_work);
        self
    }

    /// Prevents unsupported in-memory mutations from being mistaken for
    /// authoritative writes when durable evidence is configured.
    fn require_authoritative_mutation_path(&self) -> Result<(), TenancyError> {
        if self.evidence.is_some() || self.unit_of_work.is_some() {
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
    #[expect(
        clippy::too_many_arguments,
        reason = "Invitation issuance keeps tenant, actor, and token material explicit at the application boundary."
    )]
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
        let Some(unit) = self.evidence.as_ref() else {
            return Ok(());
        };

        // Globally unique event identity (contract §6.1) — not derived from
        // timestamps, names, or client-supplied sequential values.
        let event_id_raw = uuid::Uuid::new_v4().to_string();
        let event_id = AuditEventId::new(event_id_raw.clone())
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;

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

        let payload_str = serde_json::to_string(&payload)
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;

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
        .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;

        unit.record_evidence(&audit_event, &outbox_event).await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the explicit audit and invitation fields keep tenant, actor, target, and provenance reviewable at call sites"
    )]
    async fn record_evidence_in_transaction(
        &self,
        tx: &mut dyn TenancyTransaction,
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
        let event_id_raw = uuid::Uuid::new_v4().to_string();
        let event_id = AuditEventId::new(event_id_raw.clone())
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
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
        let payload_str = serde_json::to_string(&payload)
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
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
        .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
        tx.record_evidence(&audit_event, &outbox_event).await
    }

    fn check_rate(&self, class: AbuseClass, key: &str) -> RateLimitDecision {
        let mut limiter = self
            .limiter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let rule = self
            .abuse_rules
            .iter()
            .find(|(c, _)| *c == class)
            .map_or_else(fallback_rule, |(_, r)| *r);
        limiter.check(class, key, &rule, SystemTime::now())
    }

    /// Atomically provisions an organization with its owner membership and
    /// default branch (Phase 4 section 7.1). Either the whole triple commits
    /// or nothing does — there is no observable ownerless organization.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn provision_organization(
        &self,
        organization_id: OrganizationId,
        organization_name: &str,
        owner_membership_id: MembershipId,
        owner_user_id: TenantUserId,
        branch_id: BranchId,
        branch_name: &str,
    ) -> Result<ProvisionedOrganization, TenancyError> {
        if self.unit_of_work.is_some() {
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

            let mut tx = self.durable()?.begin_mutation(&organization_id).await?;
            let newly_created = tx.provision_organization(&provisioned).await?;
            if !newly_created {
                tx.commit()
                    .await
                    .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
                return Ok(provisioned);
            }
            self.record_evidence_in_transaction(
                tx.as_mut(),
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
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
            return Ok(provisioned);
        }

        let provisioned = self
            .reference()?
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
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn create_branch(
        &self,
        id: BranchId,
        organization_id: OrganizationId,
        name: &str,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.unit_of_work.is_some() {
            let mut branch = Branch::provision(id.clone(), organization_id.clone(), name)?;
            branch.activate()?;
            let mut tx = self.durable()?.begin_mutation(&organization_id).await?;
            let organization_state = tx.lock_organization(&organization_id).await?;
            if organization_state.as_deref() != Some("ACTIVE") {
                return Err(TenancyError::InvalidTransition);
            }
            tx.insert_branch(&branch).await?;
            self.record_evidence_in_transaction(
                tx.as_mut(),
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
                .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
                    stage: "branch_create_commit",
                    sqlstate: None,
                })?;
            return Ok(branch);
        }
        let branch = self
            .reference()?
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
        let user_id = TenantUserId::new(security_context.principal.subject_id.as_str())?;
        let mut branch = Branch::provision(id.clone(), organization_id.clone(), name)?;
        branch.activate()?;

        let mut tx = self
            .durable()?
            .begin_authorized_mutation(&organization_id)
            .await?;
        let locked = tx
            .lock_organization(&organization_id)
            .await
            .map_err(|_error| ScopeError::AuthorityUnavailable)?;
        if locked.is_none() {
            return Err(ScopeError::OrganizationDenied.into());
        }
        let organization_state = tx.organization_state(&organization_id).await?;
        if organization_state.as_deref() != Some("ACTIVE") {
            return Err(ScopeError::OrganizationNotActive.into());
        }
        let scope = authorize_operation_in_transaction(
            tx.as_mut(),
            &organization_id,
            MembershipSelector::User(&user_id),
            None,
            Permission::BranchCreate,
        )
        .await?;

        tx.insert_branch(&branch).await?;
        self.record_evidence_in_transaction(
            tx.as_mut(),
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
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
                stage: "authorized_branch_create_commit",
                sqlstate: None,
            })?;
        Ok(branch)
    }

    /// Activates a provisioning branch after checking persisted
    /// `BRANCH_UPDATE` authority at the organization scope. Authorization,
    /// transition, audit, and outbox writes share one PostgreSQL transaction;
    /// actor provenance comes only from the authenticated context.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn activate_branch_for_authenticated_user(
        &self,
        security_context: &SecurityContext,
        organization_id: OrganizationId,
        branch_id: BranchId,
    ) -> Result<Branch, AuthorizedTenancyError> {
        let user_id = TenantUserId::new(security_context.principal.subject_id.as_str())?;
        let mut tx = self
            .durable()?
            .begin_authorized_mutation(&organization_id)
            .await?;
        let organization_state = tx
            .lock_organization(&organization_id)
            .await
            .map_err(|_error| ScopeError::AuthorityUnavailable)?;
        if organization_state.as_deref() != Some("ACTIVE") {
            return Err(ScopeError::OrganizationDenied.into());
        }
        let authorized_scope = authorize_operation_in_transaction(
            tx.as_mut(),
            &organization_id,
            MembershipSelector::User(&user_id),
            None,
            Permission::BranchUpdate,
        )
        .await?;
        // The adapter routes through the owner-owned SECURITY DEFINER
        // transition function: app_runtime holds no direct UPDATE privilege
        // on `branches`, and the function re-applies the tenant predicate plus
        // the advisory lock.
        let Some((name, state_version)) = tx
            .transition_branch(&branch_id, "PROVISIONING", "ACTIVE")
            .await?
        else {
            return Err(TenancyError::InvalidTransition.into());
        };
        let branch = Branch {
            id: branch_id.clone(),
            organization_id: organization_id.clone(),
            name,
            state: BranchState::Active,
            state_version,
        };
        self.record_evidence_in_transaction(
            tx.as_mut(),
            IamEventName::BranchActivated,
            &organization_id,
            Some(&branch_id),
            TargetRef::Branch(branch_id.clone()),
            ActorRef {
                subject_ref: security_context.principal.subject_id.as_str().to_owned(),
                membership_ref: Some(authorized_scope.membership_id.as_str().to_owned()),
                device_ref: security_context
                    .principal
                    .device_id
                    .as_ref()
                    .map(|device_id| device_id.as_str().to_owned()),
            },
            "activate_branch",
            "branch",
            branch_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "branch_id": branch_id.as_str(),
                "from_state": "PROVISIONING",
                "to_state": "ACTIVE",
                "state_version": branch.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
                stage: "authorized_branch_activate_commit",
                sqlstate: None,
            })?;
        Ok(branch)
    }

    /// Invites a user into an organization (section 8): records the
    /// membership in state `Invited` and enforces one non-terminal
    /// membership per (organization, user). Token-bound enrollment with
    /// proposed role/scope goes through [`Self::create_invitation`].
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn invite_member(
        &self,
        id: MembershipId,
        organization_id: OrganizationId,
        user_id: TenantUserId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.unit_of_work.is_some() {
            return self
                .invite_member_with_evidence(id, organization_id, user_id, actor)
                .await;
        }
        let membership = self
            .reference()?
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
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn mark_member_pending(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.unit_of_work.is_some() {
            return self
                .transition_membership_with_evidence(
                    organization_id,
                    id,
                    "INVITED",
                    "PENDING_ACCEPTANCE",
                    IamEventName::MembershipPendingAcceptance,
                    actor,
                    "mark_member_pending",
                )
                .await;
        }
        self.reference()?
            .advance_membership(organization_id, id)
            .await
    }

    /// Accepts enrollment: `PENDING_ACCEPTANCE -> ACTIVE`.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn accept_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let membership = self
            .reference()?
            .activate_membership(organization_id, id)
            .await?;
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
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn suspend_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let membership = self
            .reference()?
            .suspend_membership(organization_id, id)
            .await?;
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

    /// Suspends a membership only when the authenticated principal has the
    /// persisted `MEMBERSHIP_SUSPEND` permission in this organization. The
    /// authorization read, state change, audit row, and outbox event share
    /// one transaction; the audit actor is derived from `security_context`.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn suspend_member_for_authenticated_user(
        &self,
        security_context: &SecurityContext,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Membership, AuthorizedTenancyError> {
        let user_id = TenantUserId::new(security_context.principal.subject_id.as_str())?;
        let mut tx = self
            .durable()?
            .begin_authorized_mutation(organization_id)
            .await?;
        let organization_state = tx
            .lock_organization(organization_id)
            .await
            .map_err(|_error| ScopeError::AuthorityUnavailable)?;
        if organization_state.as_deref() != Some("ACTIVE") {
            return Err(ScopeError::OrganizationDenied.into());
        }
        let authorized_scope = authorize_operation_in_transaction(
            tx.as_mut(),
            organization_id,
            MembershipSelector::User(&user_id),
            None,
            Permission::MembershipSuspend,
        )
        .await?;
        let membership = tx
            .transition_membership(organization_id, membership_id, "ACTIVE", "SUSPENDED")
            .await?;
        self.record_evidence_in_transaction(
            tx.as_mut(),
            IamEventName::MembershipSuspended,
            organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            ActorRef {
                subject_ref: security_context.principal.subject_id.as_str().to_owned(),
                membership_ref: Some(authorized_scope.membership_id.as_str().to_owned()),
                device_ref: security_context
                    .principal
                    .device_id
                    .as_ref()
                    .map(|device_id| device_id.as_str().to_owned()),
            },
            "suspend_member",
            "membership",
            membership_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": membership_id.as_str(),
                "from_state": "ACTIVE",
                "to_state": "SUSPENDED",
                "state_version": membership.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
                stage: "authorized_membership_suspend_commit",
                sqlstate: None,
            })?;
        Ok(membership)
    }

    /// Revokes a membership only when the authenticated principal has the
    /// persisted `MEMBERSHIP_REVOKE` permission in this organization. The
    /// authorization read, state change, audit row, and outbox event share
    /// one transaction; the audit actor is derived from `security_context`.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn revoke_member_for_authenticated_user(
        &self,
        security_context: &SecurityContext,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Membership, AuthorizedTenancyError> {
        let user_id = TenantUserId::new(security_context.principal.subject_id.as_str())?;
        let mut tx = self
            .durable()?
            .begin_authorized_mutation(organization_id)
            .await?;
        let organization_state = tx
            .lock_organization(organization_id)
            .await
            .map_err(|_error| ScopeError::AuthorityUnavailable)?;
        if organization_state.as_deref() != Some("ACTIVE") {
            return Err(ScopeError::OrganizationDenied.into());
        }
        let authorized_scope = authorize_operation_in_transaction(
            tx.as_mut(),
            organization_id,
            MembershipSelector::User(&user_id),
            None,
            Permission::MembershipRevoke,
        )
        .await?;
        let membership = tx
            .transition_membership(organization_id, membership_id, "ACTIVE", "REVOKED")
            .await?;
        self.record_evidence_in_transaction(
            tx.as_mut(),
            IamEventName::MembershipRevoked,
            organization_id,
            None,
            TargetRef::Membership(membership_id.clone()),
            ActorRef {
                subject_ref: security_context.principal.subject_id.as_str().to_owned(),
                membership_ref: Some(authorized_scope.membership_id.as_str().to_owned()),
                device_ref: security_context
                    .principal
                    .device_id
                    .as_ref()
                    .map(|device_id| device_id.as_str().to_owned()),
            },
            "revoke_member",
            "membership",
            membership_id.to_string(),
            serde_json::json!({
                "organization_id": organization_id.as_str(),
                "membership_id": membership_id.as_str(),
                "from_state": "ACTIVE",
                "to_state": "REVOKED",
                "state_version": membership.state_version,
            }),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
                stage: "authorized_membership_revoke_commit",
                sqlstate: None,
            })?;
        Ok(membership)
    }

    /// Restores a suspended membership.
    ///
    /// Emitted as [`IamEventName::MembershipActivated`]: the contract's
    /// event vocabulary has no separate "resumed" event for memberships
    /// (unlike organizations/branches, which do have a dedicated
    /// `...Resumed` name) — reactivation and initial acceptance share the
    /// same terminal state and the same named event.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn resume_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let membership = self
            .reference()?
            .resume_membership(organization_id, id)
            .await?;
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
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn revoke_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let membership = self
            .reference()?
            .revoke_membership(organization_id, id)
            .await?;
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
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn expire_invitation(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        if self.unit_of_work.is_some() {
            let mut tx = self.durable()?.begin_mutation(organization_id).await?;
            let membership = tx
                .transition_membership(organization_id, id, "PENDING_ACCEPTANCE", "EXPIRED")
                .await?;
            self.record_evidence_in_transaction(
                tx.as_mut(),
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
                .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
                    stage: "expiry_commit",
                    sqlstate: None,
                })?;
            return Ok(membership);
        }
        self.reference()?
            .expire_membership(organization_id, id)
            .await
    }

    /// Creates a token-bound invitation (section 9): mints the `Invited`
    /// membership and the `Issued` invitation atomically. The raw token is
    /// caller-minted high-entropy material — hashed on entry, never stored,
    /// never logged. Throttled per organization (section 38).
    ///
    /// With the durable sink configured, membership, invitation, audit, and
    /// outbox rows commit together. The token hash is stored only while the
    /// invitation is issued; terminal transitions must erase it and contact.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    #[expect(
        clippy::too_many_arguments,
        reason = "the explicit audit and invitation fields keep tenant, actor, target, and provenance reviewable at call sites"
    )]
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
        if self.unit_of_work.is_some() {
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
            let mut tx = self.durable()?.begin_mutation(&organization_id).await?;
            let organization_state = tx.lock_organization(&organization_id).await?;
            if organization_state.as_deref() != Some("ACTIVE") {
                return Err(TenancyError::InvalidTransition);
            }
            tx.insert_membership(&membership).await?;
            tx.insert_invitation(&invitation).await?;
            self.record_evidence_in_transaction(
                tx.as_mut(),
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
                .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
                    stage: "invitation_create_commit",
                    sqlstate: None,
                })?;
            return Ok(CreatedInvitation {
                membership,
                invitation,
            });
        }
        self.require_authoritative_mutation_path()?;
        self.reference()?
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
    ///
    /// # Errors
    /// Returns [`InvitationError`] when the invitation is invalid, expired, already used, or evidence cannot be persisted.
    pub async fn accept_invitation(
        &self,
        raw_token: &str,
        assignment_id: RoleAssignmentId,
        grant_id: Option<ScopeGrantId>,
        actor: ActorRef,
    ) -> Result<AcceptedInvitation, InvitationError> {
        Invitation::validate_raw_token(raw_token).map_err(|_error| InvitationError::Invalid)?;
        let token_hash = Invitation::hash_token(raw_token);
        if matches!(
            self.check_rate(AbuseClass::InvitationAccept, &sitolo_auth::hex(&token_hash),),
            RateLimitDecision::Locked { .. }
        ) {
            return Err(InvitationError::RateLimited);
        }
        let now = SystemTime::now();
        if self.unit_of_work.is_some() {
            return self
                .accept_invitation_with_evidence(token_hash, assignment_id, grant_id, now)
                .await;
        }
        let accepted = self
            .reference()
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?
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
        .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
        Ok(accepted)
    }

    /// Withdraws an issued invitation and releases the linked membership
    /// for re-enrollment.
    ///
    /// The invitation terminal transition, linked membership release, audit
    /// record, and outbox event share one PostgreSQL transaction.
    ///
    /// # Errors
    /// Returns [`InvitationError`] when the invitation is invalid, expired, already used, or evidence cannot be persisted.
    pub async fn revoke_invitation(
        &self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
        actor: ActorRef,
    ) -> Result<Invitation, InvitationError> {
        if self.unit_of_work.is_some() {
            let mut tx = self
                .durable()
                .map_err(|_error| InvitationError::EvidencePersistenceFailed)?
                .begin_mutation(organization_id)
                .await
                .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
            let Some(record) = tx.lock_invitation(organization_id, invitation_id).await? else {
                return Err(InvitationError::Invalid);
            };
            match record.state {
                InvitationState::Issued => {}
                InvitationState::Accepted => return Err(InvitationError::AlreadyAccepted),
                InvitationState::Expired => return Err(InvitationError::Expired),
                InvitationState::Revoked => return Err(InvitationError::Invalid),
            }
            let version = tx
                .withdraw_invitation(organization_id, invitation_id)
                .await?;
            tx.expire_linked_membership(organization_id, &record.membership_id)
                .await?;
            self.record_evidence_in_transaction(
                tx.as_mut(),
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
                    "membership_id": record.membership_id.as_str(),
                    "from_state": "ISSUED",
                    "to_state": "WITHDRAWN",
                    "state_version": version,
                }),
            )
            .await
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
            tx.commit()
                .await
                .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
            return Ok(Invitation {
                id: invitation_id.clone(),
                organization_id: organization_id.clone(),
                membership_id: record.membership_id,
                proposed_role: record.proposed_role,
                proposed_scope: record.proposed_scope,
                contact: None,
                token_hash: None,
                issued_at: record.issued_at,
                expires_at: record.expires_at,
                state: InvitationState::Revoked,
                state_version: version,
            });
        }
        self.require_authoritative_mutation_path()
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?;
        self.reference()
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?
            .revoke_invitation(organization_id, invitation_id)
            .await
    }

    /// Sweeps lapsed issued invitations to expired, releasing their linked
    /// memberships. Returns the count transitioned.
    ///
    /// The current PostgreSQL sweep is fail-closed until a bounded
    /// cross-tenant worker contract is available.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn expire_invitations(&self) -> Result<u64, TenancyError> {
        self.require_authoritative_mutation_path()?;
        Ok(self
            .reference()?
            .expire_invitations(SystemTime::now())
            .await)
    }

    /// Expires one bounded batch of issued invitations for a single tenant.
    ///
    /// Callers enumerate organizations through their trusted scheduler and
    /// invoke this method once per organization. Row locks with `SKIP LOCKED`
    /// make overlapping sweepers safe. Invitation state, linked membership
    /// state, audit, and outbox intent share one PostgreSQL transaction.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn expire_invitations_for_organization(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<u64, TenancyError> {
        const BATCH_LIMIT: i64 = 100;
        let mut tx = self.durable()?.begin_mutation(organization_id).await?;
        let candidates = tx
            .expired_invitation_batch(organization_id, BATCH_LIMIT)
            .await?;

        let mut transitioned = 0_u64;
        for candidate in candidates {
            let Some(invitation_version) = tx
                .expire_invitation(organization_id, &candidate.invitation_id)
                .await?
            else {
                continue;
            };

            if let Some(membership_version) = tx
                .expire_invited_membership(organization_id, &candidate.membership_id)
                .await?
            {
                self.record_evidence_in_transaction(
                    tx.as_mut(),
                    IamEventName::MembershipExpired,
                    organization_id,
                    None,
                    TargetRef::Membership(candidate.membership_id.clone()),
                    ActorRef {
                        subject_ref: "system:invitation-expiry-sweeper".to_owned(),
                        membership_ref: None,
                        device_ref: None,
                    },
                    "expire_invitations",
                    "membership",
                    candidate.membership_id.to_string(),
                    serde_json::json!({
                        "membership_id": candidate.membership_id.as_str(),
                        "from_state": "INVITED",
                        "to_state": "EXPIRED",
                        "state_version": membership_version,
                        "expiry_reason": "invitation_expired",
                    }),
                )
                .await?;
            }

            self.record_evidence_in_transaction(
                tx.as_mut(),
                IamEventName::InvitationExpired,
                organization_id,
                None,
                TargetRef::Invitation(candidate.invitation_id.clone()),
                ActorRef {
                    subject_ref: "system:invitation-expiry-sweeper".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
                "expire_invitations",
                "invitation",
                candidate.invitation_id.to_string(),
                serde_json::json!({
                    "invitation_id": candidate.invitation_id.as_str(),
                    "membership_id": candidate.membership_id.as_str(),
                    "from_state": "ISSUED",
                    "to_state": "EXPIRED",
                    "expires_at": system_time_to_utc(candidate.expires_at)?.to_rfc3339_opts(SecondsFormat::Millis, true),
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
            .map_err(|_error| TenancyError::EvidencePersistenceFailedAt {
                stage: "invitation_expiry_sweep_commit",
                sqlstate: None,
            })?;
        Ok(transitioned)
    }

    /// Low-level role assignment using caller-supplied actor evidence. This is
    /// a reference/evidence API and does not authorize that actor; production
    /// requests must call [`Self::assign_role_for_authenticated_user`].
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn assign_role(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        assignment_id: RoleAssignmentId,
        role: Role,
        actor: ActorRef,
    ) -> Result<RoleAssignment, TenancyError> {
        if self.unit_of_work.is_some() {
            return self
                .assign_role_with_evidence(
                    &organization_id,
                    &membership_id,
                    &assignment_id,
                    role,
                    Some(actor),
                    None,
                )
                .await
                .map_err(|error| match error {
                    AuthorizedTenancyError::Tenancy(error) => error,
                    AuthorizedTenancyError::Authorization(_) => {
                        TenancyError::EvidencePersistenceFailed
                    }
                });
        }
        let assignment = self
            .reference()?
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

    /// Assigns an organization role after checking persisted `ROLE_ASSIGN`
    /// authority in the same PostgreSQL transaction as the state and evidence
    /// writes. Actor and membership provenance are derived from the request's
    /// authenticated security context.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn assign_role_for_authenticated_user(
        &self,
        security_context: &SecurityContext,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        assignment_id: RoleAssignmentId,
        role: Role,
    ) -> Result<RoleAssignment, AuthorizedTenancyError> {
        self.assign_role_with_evidence(
            &organization_id,
            &membership_id,
            &assignment_id,
            role,
            None,
            Some(security_context),
        )
        .await
    }

    /// Low-level role revocation using caller-supplied actor evidence. Use
    /// [`Self::revoke_role_for_authenticated_user`] for production requests.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn revoke_role(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
        actor: ActorRef,
    ) -> Result<RoleAssignment, TenancyError> {
        if self.unit_of_work.is_some() {
            return self
                .revoke_role_with_evidence(organization_id, membership_id, role, Some(actor), None)
                .await
                .map_err(|error| match error {
                    AuthorizedTenancyError::Tenancy(error) => error,
                    AuthorizedTenancyError::Authorization(_) => {
                        TenancyError::EvidencePersistenceFailed
                    }
                });
        }
        let assignment = self
            .reference()?
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

    /// Revokes an effective role after checking persisted `ROLE_ASSIGN`
    /// authority in the same transaction as the revocation and its evidence.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn revoke_role_for_authenticated_user(
        &self,
        security_context: &SecurityContext,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
    ) -> Result<RoleAssignment, AuthorizedTenancyError> {
        self.revoke_role_with_evidence(
            organization_id,
            membership_id,
            role,
            None,
            Some(security_context),
        )
        .await
    }

    /// Full grant history for a membership, all states.
    pub async fn member_roles(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<RoleAssignment> {
        if let Some(unit) = self.unit_of_work.as_ref() {
            return unit
                .read_member_roles(organization_id, membership_id)
                .await
                .unwrap_or_default();
        }
        let Some(stores) = self.stores.as_ref() else {
            return Vec::new();
        };
        stores
            .role_assignments_for(organization_id, membership_id)
            .await
    }

    /// Low-level scope grant using caller-supplied actor evidence. Use
    /// [`Self::grant_scope_for_authenticated_user`] for production requests.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn grant_scope(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        grant_id: ScopeGrantId,
        scope: Scope,
        actor: ActorRef,
    ) -> Result<ScopeGrant, TenancyError> {
        if self.unit_of_work.is_some() {
            return self
                .grant_scope_with_evidence(
                    &organization_id,
                    &membership_id,
                    &grant_id,
                    scope,
                    Some(actor),
                    None,
                )
                .await
                .map_err(|error| match error {
                    AuthorizedTenancyError::Tenancy(error) => error,
                    AuthorizedTenancyError::Authorization(_) => {
                        TenancyError::EvidencePersistenceFailed
                    }
                });
        }
        let grant = self
            .reference()?
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

    /// Adds a scope grant after checking persisted `SCOPE_GRANT` authority in
    /// the same PostgreSQL transaction as the grant and evidence writes.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn grant_scope_for_authenticated_user(
        &self,
        security_context: &SecurityContext,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        grant_id: ScopeGrantId,
        scope: Scope,
    ) -> Result<ScopeGrant, AuthorizedTenancyError> {
        self.grant_scope_with_evidence(
            &organization_id,
            &membership_id,
            &grant_id,
            scope,
            None,
            Some(security_context),
        )
        .await
    }

    /// Low-level scope revocation using caller-supplied actor evidence. Use
    /// [`Self::revoke_scope_grant_for_authenticated_user`] for production requests.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn revoke_scope_grant(
        &self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
        actor: ActorRef,
    ) -> Result<ScopeGrant, TenancyError> {
        if self.unit_of_work.is_some() {
            return self
                .revoke_scope_with_evidence(organization_id, grant_id, Some(actor), None)
                .await
                .map_err(|error| match error {
                    AuthorizedTenancyError::Tenancy(error) => error,
                    AuthorizedTenancyError::Authorization(_) => {
                        TenancyError::EvidencePersistenceFailed
                    }
                });
        }
        let grant = self
            .reference()?
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

    /// Revokes a scope grant after checking persisted `SCOPE_GRANT` authority
    /// in the same PostgreSQL transaction as the revocation/evidence writes.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn revoke_scope_grant_for_authenticated_user(
        &self,
        security_context: &SecurityContext,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
    ) -> Result<ScopeGrant, AuthorizedTenancyError> {
        self.revoke_scope_with_evidence(organization_id, grant_id, None, Some(security_context))
            .await
    }

    /// Active and revoked narrowing entries for a membership.
    pub async fn member_grants(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<ScopeGrant> {
        if let Some(unit) = self.unit_of_work.as_ref() {
            return unit
                .read_member_grants(organization_id, membership_id)
                .await
                .unwrap_or_default();
        }
        let Some(stores) = self.stores.as_ref() else {
            return Vec::new();
        };
        stores
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
        if let Some(unit) = self.unit_of_work.as_ref() {
            let Ok(mut tx) = unit.begin_scope(organization_id).await else {
                return BTreeSet::new();
            };
            let Ok(material) = tx
                .load_scope_material(organization_id, MembershipSelector::Id(membership_id), None)
                .await
            else {
                return BTreeSet::new();
            };
            let permissions = if material.membership.has_authority() {
                resolve_permissions(&material.role_assignments)
            } else {
                BTreeSet::new()
            };
            let _commit_result: Result<(), TenancyError> = tx.commit().await;
            return permissions;
        }
        let Some(stores) = self.stores.as_ref() else {
            return BTreeSet::new();
        };
        let membership = stores
            .membership_snapshot(organization_id, membership_id)
            .await
            .filter(Membership::has_authority);
        let Some(membership) = membership else {
            return BTreeSet::new();
        };
        let assignments = stores
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
        let mut tx = self.durable()?.begin_mutation(id).await?;
        let Some((name, state_version)) = tx.transition_organization(spec.from, spec.to).await?
        else {
            return Err(TenancyError::InvalidTransition);
        };
        let organization = Organization {
            id: id.clone(),
            name,
            state: next,
            state_version,
        };
        self.record_evidence_in_transaction(
            tx.as_mut(),
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
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
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
        let mut tx = self.durable()?.begin_mutation(organization_id).await?;
        let Some((name, state_version)) = tx.transition_branch(id, spec.from, spec.to).await?
        else {
            return Err(TenancyError::InvalidTransition);
        };
        let branch = Branch {
            id: id.clone(),
            organization_id: organization_id.clone(),
            name,
            state: next,
            state_version,
        };
        self.record_evidence_in_transaction(
            tx.as_mut(),
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
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
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
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn activate_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let organization = self.reference()?.activate_organization(id).await?;
        self.record_organization_evidence(
            IamEventName::OrganizationActivated,
            &organization,
            actor,
            "activate_organization",
        )
        .await?;
        Ok(organization)
    }

    /// Suspends an active organization.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn suspend_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let organization = self.reference()?.suspend_organization(id).await?;
        self.record_organization_evidence(
            IamEventName::OrganizationSuspended,
            &organization,
            actor,
            "suspend_organization",
        )
        .await?;
        Ok(organization)
    }

    /// Resumes a suspended organization.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn resume_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let organization = self.reference()?.resume_organization(id).await?;
        self.record_organization_evidence(
            IamEventName::OrganizationResumed,
            &organization,
            actor,
            "resume_organization",
        )
        .await?;
        Ok(organization)
    }

    /// Begins closing an active organization.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn begin_close_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let organization = self.reference()?.begin_close_organization(id).await?;
        self.record_organization_evidence(
            IamEventName::OrganizationClosingStarted,
            &organization,
            actor,
            "begin_close_organization",
        )
        .await?;
        Ok(organization)
    }

    /// Closes a closing organization.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn close_organization(
        &self,
        id: &OrganizationId,
        actor: ActorRef,
    ) -> Result<Organization, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let organization = self.reference()?.close_organization(id).await?;
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
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn activate_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let branch = self
            .reference()?
            .activate_branch(organization_id, id)
            .await?;
        self.record_branch_evidence(
            IamEventName::BranchActivated,
            &branch,
            actor,
            "activate_branch",
        )
        .await?;
        Ok(branch)
    }

    /// Suspends an active branch.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn suspend_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let branch = self
            .reference()?
            .suspend_branch(organization_id, id)
            .await?;
        self.record_branch_evidence(
            IamEventName::BranchSuspended,
            &branch,
            actor,
            "suspend_branch",
        )
        .await?;
        Ok(branch)
    }

    /// Resumes a suspended branch.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn resume_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let branch = self.reference()?.resume_branch(organization_id, id).await?;
        self.record_branch_evidence(IamEventName::BranchResumed, &branch, actor, "resume_branch")
            .await?;
        Ok(branch)
    }

    /// Begins closing an active branch.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn begin_close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let branch = self
            .reference()?
            .begin_close_branch(organization_id, id)
            .await?;
        self.record_branch_evidence(
            IamEventName::BranchClosingStarted,
            &branch,
            actor,
            "begin_close_branch",
        )
        .await?;
        Ok(branch)
    }

    /// Closes a closing branch.
    ///
    /// # Errors
    /// Returns [`TenancyError`] when the lifecycle transition is not permitted, a record is missing, or the durable transaction fails.
    pub async fn close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
        actor: ActorRef,
    ) -> Result<Branch, TenancyError> {
        if self.unit_of_work.is_some() {
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
        let branch = self.reference()?.close_branch(organization_id, id).await?;
        self.record_branch_evidence(IamEventName::BranchClosed, &branch, actor, "close_branch")
            .await?;
        Ok(branch)
    }

    /// Resolves the effective scope for persisted records (sections 5.1, 54).
    /// Missing records deny exactly like non-operating records — existence
    /// is never disclosed through this path.
    ///
    /// # Errors
    /// Returns [`ScopeError`] when membership, branch, or scope material cannot be resolved.
    pub async fn effective_scope(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        branch_id: Option<&BranchId>,
    ) -> Result<EffectiveScope, ScopeError> {
        if let Some(unit) = self.unit_of_work.as_ref() {
            let mut tx = unit.begin_scope(organization_id).await?;
            let material = tx
                .load_scope_material(
                    organization_id,
                    MembershipSelector::Id(membership_id),
                    branch_id,
                )
                .await?;
            let scope = resolve_effective_scope(
                &material.membership,
                &material.organization,
                material.branch.as_ref(),
            )?;
            tx.commit()
                .await
                .map_err(|_error| ScopeError::AuthorityUnavailable)?;
            return Ok(scope);
        }

        let Some(stores) = self.stores.as_ref() else {
            return Err(ScopeError::AuthorityUnavailable);
        };
        let organization = stores
            .organization_snapshot(organization_id)
            .await
            .ok_or(ScopeError::OrganizationDenied)?;
        let membership = stores
            .membership_snapshot(organization_id, membership_id)
            .await
            .ok_or(ScopeError::MembershipNotActive)?;
        let branch = match branch_id {
            Some(id) => Some(
                stores
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
    ///
    /// # Errors
    /// Returns [`ScopeError`] when membership, branch, or scope material cannot be resolved.
    pub async fn authorize_operation(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        branch_id: Option<&BranchId>,
        permission: Permission,
    ) -> Result<EffectiveScope, ScopeError> {
        if let Some(unit) = self.unit_of_work.as_ref() {
            let mut tx = unit.begin_scope(organization_id).await?;
            let scope = authorize_operation_in_transaction(
                tx.as_mut(),
                organization_id,
                MembershipSelector::Id(membership_id),
                branch_id,
                permission,
            )
            .await?;
            tx.commit()
                .await
                .map_err(|_error| ScopeError::AuthorityUnavailable)?;
            return Ok(scope);
        }

        let mut scope = self
            .effective_scope(organization_id, membership_id, branch_id)
            .await?;
        let Some(stores) = self.stores.as_ref() else {
            return Err(ScopeError::AuthorityUnavailable);
        };
        let assignments = stores
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
        let grants = stores
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
        let unit = self
            .unit_of_work
            .as_ref()
            .ok_or(ScopeError::AuthorityUnavailable)?;
        let mut tx = unit.begin_scope(organization_id).await?;
        let scope = authorize_operation_in_transaction(
            tx.as_mut(),
            organization_id,
            MembershipSelector::User(user_id),
            branch_id,
            permission,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_error| ScopeError::AuthorityUnavailable)?;
        Ok(scope)
    }
}

/// Authorizes one operation from the durable scope material loaded inside the
/// caller's transaction.
///
/// The transaction has already established tenant context. This function only
/// composes the pure authorization decision owned by the application layer;
/// the persistence adapter supplies the rows through [`TenancyTransaction`].
async fn authorize_operation_in_transaction(
    tx: &mut dyn TenancyTransaction,
    organization_id: &OrganizationId,
    selector: MembershipSelector<'_>,
    branch_id: Option<&BranchId>,
    permission: Permission,
) -> Result<EffectiveScope, ScopeError> {
    let material = tx
        .load_scope_material(organization_id, selector, branch_id)
        .await?;
    let mut scope = resolve_effective_scope(
        &material.membership,
        &material.organization,
        material.branch.as_ref(),
    )?;
    let permissions = resolve_permissions(&material.role_assignments);
    if !permissions.contains(&permission) {
        return Err(ScopeError::PermissionDenied);
    }
    let requested = match &scope.branch_id {
        Some(branch_id) => Scope::Branch {
            organization_id: organization_id.clone(),
            branch_id: branch_id.clone(),
        },
        None => Scope::Organization {
            organization_id: organization_id.clone(),
        },
    };
    if !authorize_scope(&material.scope_grants, &requested) {
        return Err(ScopeError::ScopeDenied);
    }
    scope.permissions = permissions;
    Ok(scope)
}
