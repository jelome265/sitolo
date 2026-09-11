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
//! PR-009), audit emission (PR-009), HTTP routes and DTOs (PR-006). These
//! methods enforce state-machine and scope correctness, not caller
//! authorization — actor authority (who may invite, grant, or revoke) is
//! enforced by the authorization engine (Phase 6) once issuer scope exists.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use sitolo_auth::{AbuseClass, RateLimitDecision, RateLimitRule, RateLimiter};
use sitolo_authz::{
    Invitation, InvitationContact, InvitationError, Permission, Role, RoleAssignment, Scope,
    ScopeGrant, authorize_scope, resolve_permissions,
};
use sitolo_domain::tenancy::{
    Branch, BranchId, InvitationId, Membership, MembershipId, Organization, OrganizationId,
    RoleAssignmentId, ScopeGrantId, TenancyError, TenantUserId,
};
use sitolo_persistence::{
    AcceptedInvitation, CreateInvitationInput, CreatedInvitation, ProvisionedOrganization,
    TenancyDatabase, TenancyStores,
};
use sitolo_tenancy::{EffectiveScope, ScopeError, resolve_effective_scope};

/// The tenancy service (Phase 4 use cases).
pub struct TenancyService {
    db: Arc<TenancyDatabase>,
    limiter: Mutex<RateLimiter>,
    abuse_rules: Vec<(AbuseClass, RateLimitRule)>,
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
    pub fn new(db: Arc<TenancyDatabase>, abuse_rules: Vec<(AbuseClass, RateLimitRule)>) -> Self {
        TenancyService {
            db,
            limiter: Mutex::new(RateLimiter::new()),
            abuse_rules,
        }
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
        self.db
            .provision_organization(
                organization_id,
                organization_name,
                owner_membership_id,
                owner_user_id,
                branch_id,
                branch_name,
            )
            .await
    }

    /// Creates an additional branch under an operating organization.
    pub async fn create_branch(
        &self,
        id: BranchId,
        organization_id: OrganizationId,
        name: &str,
    ) -> Result<Branch, TenancyError> {
        self.db.create_branch(id, organization_id, name).await
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
    ) -> Result<Membership, TenancyError> {
        self.db
            .invite_membership(id, organization_id, user_id)
            .await
    }

    /// Advances an invitation to pending acceptance.
    pub async fn mark_member_pending(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        self.db.advance_membership(organization_id, id).await
    }

    /// Accepts enrollment: `PENDING_ACCEPTANCE -> ACTIVE`.
    pub async fn accept_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        self.db.activate_membership(organization_id, id).await
    }

    /// Temporarily removes authority with reactivation possible (20.1).
    pub async fn suspend_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        self.db.suspend_membership(organization_id, id).await
    }

    /// Restores a suspended membership.
    pub async fn resume_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        self.db.resume_membership(organization_id, id).await
    }

    /// Permanently terminates the membership for ordinary purposes (20.2).
    /// Triggers session and cache invalidation downstream via the bumped
    /// state version (20.3); invalidation consumers arrive in later PRs.
    pub async fn revoke_member(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        self.db.revoke_membership(organization_id, id).await
    }

    /// Expires a lapsed enrollment window.
    pub async fn expire_invitation(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        self.db.expire_membership(organization_id, id).await
    }

    /// Creates a token-bound invitation (section 9): mints the `Invited`
    /// membership and the `Issued` invitation atomically. The raw token is
    /// caller-minted high-entropy material — hashed on entry, never stored,
    /// never logged. Throttled per organization (section 38).
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
    ) -> Result<CreatedInvitation, TenancyError> {
        if matches!(
            self.check_rate(AbuseClass::InvitationCreate, organization_id.as_str()),
            RateLimitDecision::Locked { .. }
        ) {
            return Err(TenancyError::RateLimited);
        }
        let now = SystemTime::now();
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
        self.db
            .accept_invitation(token_hash, assignment_id, grant_id, now)
            .await
    }

    /// Withdraws an issued invitation and releases the linked membership
    /// for re-enrollment.
    pub async fn revoke_invitation(
        &self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<Invitation, InvitationError> {
        self.db
            .revoke_invitation(organization_id, invitation_id)
            .await
    }

    /// Sweeps lapsed issued invitations to expired, releasing their linked
    /// memberships. Returns the count transitioned.
    pub async fn expire_invitations(&self) -> u64 {
        self.db.expire_invitations(SystemTime::now()).await
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
    ) -> Result<RoleAssignment, TenancyError> {
        self.db
            .assign_role(organization_id, membership_id, assignment_id, role)
            .await
    }

    /// Ends an effective grant. History is preserved; re-granting creates a
    /// new record (section 8.1.10).
    pub async fn revoke_role(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError> {
        self.db
            .revoke_role(organization_id, membership_id, role)
            .await
    }

    /// Full grant history for a membership, all states.
    pub async fn member_roles(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<RoleAssignment> {
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
    ) -> Result<ScopeGrant, TenancyError> {
        self.db
            .grant_scope(organization_id, membership_id, grant_id, scope)
            .await
    }

    /// Lifts one narrowing entry. Remaining active grants still apply.
    pub async fn revoke_scope_grant(
        &self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
    ) -> Result<ScopeGrant, TenancyError> {
        self.db.revoke_scope_grant(organization_id, grant_id).await
    }

    /// Active and revoked narrowing entries for a membership.
    pub async fn member_grants(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<ScopeGrant> {
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

    /// Organization lifecycle transitions (section 6).
    pub async fn activate_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError> {
        self.db.activate_organization(id).await
    }

    pub async fn suspend_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError> {
        self.db.suspend_organization(id).await
    }

    pub async fn resume_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError> {
        self.db.resume_organization(id).await
    }

    pub async fn begin_close_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError> {
        self.db.begin_close_organization(id).await
    }

    pub async fn close_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError> {
        self.db.close_organization(id).await
    }

    /// Branch lifecycle transitions (section 14.1). All scope-carrying: a
    /// branch is only addressable through its owning organization.
    pub async fn activate_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        self.db.activate_branch(organization_id, id).await
    }

    pub async fn suspend_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        self.db.suspend_branch(organization_id, id).await
    }

    pub async fn resume_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        self.db.resume_branch(organization_id, id).await
    }

    pub async fn begin_close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        self.db.begin_close_branch(organization_id, id).await
    }

    pub async fn close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        self.db.close_branch(organization_id, id).await
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

    fn service() -> TenancyService {
        TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new())
    }

    fn strict_service() -> TenancyService {
        // Single-attempt rules: the second identical call locks (section 38).
        let rule = RateLimitRule {
            max_attempts: 1,
            window: Duration::from_secs(3_600),
            lockout: Duration::from_secs(3_600),
        };
        TenancyService::new(
            Arc::new(TenancyDatabase::new()),
            vec![
                (AbuseClass::InvitationCreate, rule),
                (AbuseClass::InvitationAccept, rule),
            ],
        )
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
            )
            .await
            .unwrap();
        assert!(!invited.has_authority());
        // Cannot skip to Active.
        assert_eq!(
            service.accept_member(org, &invited.id).await,
            Err(TenancyError::InvalidTransition)
        );
        service.mark_member_pending(org, &invited.id).await.unwrap();
        let active = service.accept_member(org, &invited.id).await.unwrap();
        assert!(active.has_authority());

        let scope = service
            .effective_scope(org, &active.id, Some(&bundle.default_branch.id))
            .await
            .unwrap();
        assert_eq!(scope.organization_id, *org);

        service.suspend_member(org, &active.id).await.unwrap();
        assert_eq!(
            service.effective_scope(org, &active.id, None).await,
            Err(ScopeError::MembershipNotActive)
        );
        service.resume_member(org, &active.id).await.unwrap();
        assert!(service.effective_scope(org, &active.id, None).await.is_ok());
        service.revoke_member(org, &active.id).await.unwrap();
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
            )
            .await
            .unwrap();
        service.mark_member_pending(org, &cashier.id).await.unwrap();
        service.accept_member(org, &cashier.id).await.unwrap();
        service
            .assign_role(
                org.clone(),
                cashier.id.clone(),
                RoleAssignmentId::new("ra-cashier").unwrap(),
                Role::Cashier,
            )
            .await
            .unwrap();
        assert_eq!(
            service.member_permissions(org, &cashier.id).await,
            BTreeSet::from([Permission::SaleCreate, Permission::SaleView])
        );

        // Revocation drains authority without deleting history.
        service
            .revoke_role(org, &cashier.id, Role::Cashier)
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
            )
            .await
            .unwrap();
        service.suspend_member(org, &cashier.id).await.unwrap();
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
            .accept_invitation(INVITE_TOKEN, RoleAssignmentId::new("ra-inv").unwrap(), None)
            .await
            .unwrap();
        assert!(accepted.membership.has_authority());
        assert_eq!(accepted.assignment.role, Role::Cashier);
        assert_eq!(accepted.scope_grant, None);
        assert_eq!(
            accepted.invitation.state,
            sitolo_authz::InvitationState::Accepted
        );

        // Replay of the consumed token reports prior acceptance, and no
        // second membership activation or grant occurs.
        assert_eq!(
            service
                .accept_invitation(
                    INVITE_TOKEN,
                    RoleAssignmentId::new("ra-replay").unwrap(),
                    None,
                )
                .await,
            Err(InvitationError::AlreadyAccepted)
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
                )
                .await,
            Err(InvitationError::Invalid)
        );
        // Malformed tokens never reach the store.
        assert_eq!(
            service
                .accept_invitation("short", RoleAssignmentId::new("ra-x").unwrap(), None)
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
                )
                .await,
            Err(InvitationError::Invalid)
        );
        let accepted = service
            .accept_invitation(
                INVITE_TOKEN_2,
                RoleAssignmentId::new("ra-scoped").unwrap(),
                Some(ScopeGrantId::new("g-scoped").unwrap()),
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
        service
            .revoke_invitation(org, &created.invitation.id)
            .await
            .unwrap();
        // The withdrawn token is generically invalid.
        assert_eq!(
            service
                .accept_invitation(
                    INVITE_TOKEN,
                    RoleAssignmentId::new("ra-dead").unwrap(),
                    None,
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
                .accept_invitation(INVITE_TOKEN, RoleAssignmentId::new("ra-t").unwrap(), None,)
                .await,
            Err(InvitationError::Invalid)
        );
        assert_eq!(
            throttle
                .accept_invitation(INVITE_TOKEN, RoleAssignmentId::new("ra-t").unwrap(), None,)
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
            )
            .await
            .unwrap();
        let second_branch = service
            .create_branch(BranchId::new("b2").unwrap(), org.clone(), "Second Branch")
            .await
            .unwrap();
        service
            .activate_branch(org, &second_branch.id)
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
            )
            .await
            .unwrap();
        service.mark_member_pending(org, &cashier.id).await.unwrap();
        service.accept_member(org, &cashier.id).await.unwrap();
        service
            .assign_role(
                org.clone(),
                cashier.id.clone(),
                RoleAssignmentId::new("ra-cashier").unwrap(),
                Role::Cashier,
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
            .revoke_scope_grant(org, &ScopeGrantId::new("g-main").unwrap())
            .await
            .unwrap();
        service
            .authorize_operation(org, owner, None, Permission::SaleCreate)
            .await
            .unwrap();
    }
}
