//! Tenancy persistence ports owned by the application layer.
//!
//! [`TenancyStores`] is the reference-store contract used by local and test
//! composition. [`TenancyUnitOfWork`] is the durable transactional contract
//! used by production composition: the application opens a transaction,
//! performs semantic operations, records mandatory audit/outbox evidence,
//! and commits, while the persistence adapter owns every SQL statement.
//!
//! The dependency edge points from persistence to application, never the
//! reverse (Phase 1 section 5.2; Phase 4 sections 21 and 27).

use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use sitolo_audit::IamAuditEvent;
use sitolo_authz::{
    Invitation, InvitationContact, InvitationError, Role, RoleAssignment, Scope, ScopeGrant,
};
use sitolo_domain::tenancy::{
    Branch, BranchId, InvitationId, Membership, MembershipId, Organization, OrganizationId,
    ProvisionedOrganization, RoleAssignmentId, ScopeGrantId, TenancyError, TenantUserId,
};
use sitolo_events::OutboxEvent;
use sitolo_tenancy::ScopeError;

/// The atomic result of accepting an invitation: the claimed invitation,
/// the activated membership, the effective role grant, and the scope grant
/// when the invitation proposed a narrowed scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedInvitation {
    /// The claimed invitation, in its terminal accepted state.
    pub invitation: Invitation,
    /// The activated membership.
    pub membership: Membership,
    /// The effective role assignment materialized from the record.
    pub assignment: RoleAssignment,
    /// The optional narrowing grant materialized from the record.
    pub scope_grant: Option<ScopeGrant>,
}

/// The atomic result of creating an invitation: the `Invited` membership
/// plus the `Issued` invitation. The raw token is held by the caller only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedInvitation {
    /// The invited membership.
    pub membership: Membership,
    /// The issued invitation.
    pub invitation: Invitation,
}

/// The outcome of one role-assignment insert attempt.
///
/// A repeated identical command is an idempotent replay, not a new state
/// transition. Mandatory audit/outbox evidence is written only for
/// [`RoleAssignmentOutcome::Created`]; a divergent request is
/// [`TenancyError::Conflict`] and writes no evidence at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleAssignmentOutcome {
    /// This command created the effective assignment.
    Created(RoleAssignment),
    /// An identical assignment already existed; no transition occurred.
    Replayed(RoleAssignment),
}

/// Creation parameters for [`TenancyStores::create_invitation`]. Grouped so
/// the port keeps an explicit, reviewable parameter list instead of eleven
/// positional arguments.
#[derive(Debug, Clone)]
pub struct CreateInvitationInput {
    /// Owning organization.
    pub organization_id: OrganizationId,
    /// Caller-minted invitation identifier.
    pub invitation_id: InvitationId,
    /// Caller-minted membership identifier.
    pub membership_id: MembershipId,
    /// Invited user.
    pub user_id: TenantUserId,
    /// Role proposed by the invitation record.
    pub proposed_role: Role,
    /// Optional narrowing scope proposed by the record.
    pub proposed_scope: Option<Scope>,
    /// Delivery contact; erased on terminal transitions.
    pub contact: InvitationContact,
    /// Caller-minted high-entropy token; hashed before storage.
    pub raw_token: String,
    /// Invitation lifetime.
    pub ttl: Duration,
    /// Issuance timestamp.
    pub now: SystemTime,
}

/// Tenant topology persistence port.
///
/// Every branch/membership operation carries the owning organization id and
/// enforces the binding server-side, mirroring the tenant-predicate queries
/// (`WHERE organization_id = $1 AND id = $2`) production repositories must
/// issue even where RLS exists (Phase 4 section 27.1). A binding mismatch
/// behaves as absence: [`TenancyError::NotFound`].
#[async_trait]
pub trait TenancyStores: Send + Sync {
    // --- organizations ---

    /// Creates an organization in the provisioning state.
    async fn create_organization(
        &self,
        id: OrganizationId,
        name: &str,
    ) -> Result<Organization, TenancyError>;

    /// Returns the organization when it exists.
    async fn organization_snapshot(&self, id: &OrganizationId) -> Option<Organization>;

    /// Activates a provisioning organization.
    async fn activate_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError>;

    /// Suspends an active organization.
    async fn suspend_organization(&self, id: &OrganizationId)
    -> Result<Organization, TenancyError>;

    /// Resumes a suspended organization.
    async fn resume_organization(&self, id: &OrganizationId) -> Result<Organization, TenancyError>;

    /// Begins closing an active organization.
    async fn begin_close_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError>;

    /// Closes a closing organization.
    async fn close_organization(&self, id: &OrganizationId) -> Result<Organization, TenancyError>;

    // --- branches (scope-carrying) ---

    /// Creates a branch under an operating organization.
    async fn create_branch(
        &self,
        id: BranchId,
        organization_id: OrganizationId,
        name: &str,
    ) -> Result<Branch, TenancyError>;

    /// Returns the branch when it belongs to the organization.
    async fn branch_snapshot(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Option<Branch>;

    /// Activates a provisioning branch.
    async fn activate_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;

    /// Suspends an active branch.
    async fn suspend_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;

    /// Resumes a suspended branch.
    async fn resume_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;

    /// Begins closing an active branch.
    async fn begin_close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;

    /// Closes a closing branch.
    async fn close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;

    // --- memberships (scope-carrying) ---

    /// Creates an invited membership.
    async fn invite_membership(
        &self,
        id: MembershipId,
        organization_id: OrganizationId,
        user_id: TenantUserId,
    ) -> Result<Membership, TenancyError>;

    /// Returns the membership when it belongs to the organization.
    async fn membership_snapshot(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Option<Membership>;

    /// Returns the latest membership for the user in the organization.
    async fn membership_for_user(
        &self,
        organization_id: &OrganizationId,
        user_id: &TenantUserId,
    ) -> Option<Membership>;

    /// Advances an invited membership to pending acceptance.
    async fn advance_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;

    /// Expires a pending membership.
    async fn expire_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;

    /// Activates a pending membership.
    async fn activate_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;

    /// Suspends an active membership.
    async fn suspend_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;

    /// Resumes a suspended membership.
    async fn resume_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;

    /// Revokes a membership.
    async fn revoke_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;

    // --- atomic provisioning bundle (Phase 4 section 7.1) ---

    /// Provisions an organization, its owner membership, and its default
    /// branch atomically.
    async fn provision_organization(
        &self,
        organization_id: OrganizationId,
        organization_name: &str,
        owner_membership_id: MembershipId,
        owner_user_id: TenantUserId,
        branch_id: BranchId,
        branch_name: &str,
    ) -> Result<ProvisionedOrganization, TenancyError>;

    // --- role assignments ---

    /// Assigns a role to a membership.
    async fn assign_role(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        assignment_id: RoleAssignmentId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError>;

    /// Revokes an effective role from a membership.
    async fn revoke_role(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError>;

    /// Returns the full role-assignment history for a membership.
    async fn role_assignments_for(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<RoleAssignment>;

    // --- scope grants ---

    /// Grants a narrowing scope to a membership.
    async fn grant_scope(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        grant_id: ScopeGrantId,
        scope: Scope,
    ) -> Result<ScopeGrant, TenancyError>;

    /// Revokes a scope grant.
    async fn revoke_scope_grant(
        &self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
    ) -> Result<ScopeGrant, TenancyError>;

    /// Returns the full scope-grant history for a membership.
    async fn scope_grants_for(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<ScopeGrant>;

    // --- invitations (Phase 4 section 9) ---

    /// Creates an invitation and its invited membership atomically.
    async fn create_invitation(
        &self,
        input: CreateInvitationInput,
    ) -> Result<CreatedInvitation, TenancyError>;

    /// Claims a token-bound invitation and materializes its role/scope.
    async fn accept_invitation(
        &self,
        token_hash: [u8; 32],
        assignment_id: RoleAssignmentId,
        grant_id: Option<ScopeGrantId>,
        now: SystemTime,
    ) -> Result<AcceptedInvitation, InvitationError>;

    /// Withdraws an issued invitation.
    async fn revoke_invitation(
        &self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<Invitation, InvitationError>;

    /// Expires every lapsed issued invitation.
    async fn expire_invitations(&self, now: SystemTime) -> u64;
}

/// Selects the membership used to resolve authorization material.
#[derive(Debug, Clone, Copy)]
pub enum MembershipSelector<'a> {
    /// Select by membership identifier.
    Id(&'a MembershipId),
    /// Select by the user bound to the membership.
    User(&'a TenantUserId),
}

/// Persisted records needed to decide whether an operation is authorized.
#[derive(Debug, Clone)]
pub struct ScopeMaterial {
    /// Owning organization.
    pub organization: Organization,
    /// Membership resolved by the selector.
    pub membership: Membership,
    /// Requested branch, when the operation is branch-scoped.
    pub branch: Option<Branch>,
    /// All role assignments for the membership, in any state.
    pub role_assignments: Vec<RoleAssignment>,
    /// All scope grants for the membership, in any state.
    pub scope_grants: Vec<ScopeGrant>,
}

/// Persisted membership state used by mutation flows.
#[derive(Debug, Clone)]
pub struct MembershipRecord {
    /// User bound to the membership.
    pub user_id: TenantUserId,
    /// Current membership state.
    pub state: sitolo_domain::tenancy::MembershipState,
    /// Optimistic concurrency version.
    pub state_version: u64,
}

/// Persisted invitation record used by acceptance and withdrawal flows.
#[derive(Debug, Clone)]
pub struct InvitationRecord {
    /// Invitation identifier.
    pub id: InvitationId,
    /// Linked membership.
    pub membership_id: MembershipId,
    /// Proposed role.
    pub proposed_role: Role,
    /// Optional proposed narrowing scope.
    pub proposed_scope: Option<Scope>,
    /// Current invitation state.
    pub state: sitolo_authz::InvitationState,
    /// Issuance timestamp.
    pub issued_at: SystemTime,
    /// Expiry timestamp.
    pub expires_at: SystemTime,
    /// Optimistic concurrency version.
    pub state_version: u64,
}

/// One lapsed invitation selected by the tenant-scoped expiry sweep.
#[derive(Debug, Clone)]
pub struct ExpiredInvitation {
    /// Invitation identifier.
    pub invitation_id: InvitationId,
    /// Linked membership.
    pub membership_id: MembershipId,
    /// Expiry timestamp carried into the audit payload.
    pub expires_at: SystemTime,
}

/// Durable transactional tenancy port.
///
/// The application owns the transaction boundary: it opens a transaction,
/// performs the semantic operations required by one use case, records the
/// mandatory audit/outbox evidence, and commits. The adapter owns the SQL and
/// the row-to-domain mapping.
#[async_trait]
pub trait TenancyUnitOfWork: Send + Sync {
    /// Begins a tenant-scoped mutation transaction.
    async fn begin_mutation(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Box<dyn TenancyTransaction>, TenancyError>;

    /// Begins a tenant-scoped mutation transaction for an authenticated
    /// command that must authorize before mutating.
    async fn begin_authorized_mutation(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Box<dyn TenancyTransaction>, ScopeError>;

    /// Begins a tenant-scoped repeatable-read transaction for authorization
    /// reads.
    async fn begin_scope(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Box<dyn TenancyTransaction>, ScopeError>;

    /// Begins a transaction that resolves its tenant from an invitation token
    /// before any tenant context is known.
    async fn begin_invitation(&self) -> Result<Box<dyn TenancyTransaction>, InvitationError>;

    /// Writes one audit row and its paired outbox row in a dedicated
    /// transaction.
    async fn record_evidence(
        &self,
        audit: &IamAuditEvent,
        outbox: &OutboxEvent,
    ) -> Result<(), TenancyError>;

    /// Returns the full role-assignment history for a membership.
    async fn read_member_roles(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Vec<RoleAssignment>, ScopeError>;

    /// Returns the full scope-grant history for a membership.
    async fn read_member_grants(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Vec<ScopeGrant>, ScopeError>;
}

/// One durable tenancy transaction.
///
/// Implementations must apply the tenant predicate to every operation even
/// where row-level security also applies (Phase 4 section 27.1).
#[async_trait]
pub trait TenancyTransaction: Send {
    /// Sets the transaction-local tenant context used by row-level security.
    async fn set_tenant_context(
        &mut self,
        organization_id: &OrganizationId,
    ) -> Result<(), ScopeError>;

    /// Takes the organization mutation advisory lock and returns its state.
    async fn lock_organization(
        &mut self,
        organization_id: &OrganizationId,
    ) -> Result<Option<String>, TenancyError>;

    /// Returns the organization state without taking the mutation lock.
    async fn organization_state(
        &mut self,
        organization_id: &OrganizationId,
    ) -> Result<Option<String>, ScopeError>;

    /// Loads the organization, membership, optional branch, role assignments,
    /// and scope grants needed for an authorization decision.
    async fn load_scope_material(
        &mut self,
        organization_id: &OrganizationId,
        selector: MembershipSelector<'_>,
        branch_id: Option<&BranchId>,
    ) -> Result<ScopeMaterial, ScopeError>;

    /// Returns the membership record when it belongs to the organization.
    async fn membership_record(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Option<MembershipRecord>, TenancyError>;

    /// Applies one membership state transition.
    async fn transition_membership(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        from: &str,
        to: &str,
    ) -> Result<Membership, TenancyError>;

    /// Inserts an invited membership.
    async fn insert_membership(&mut self, membership: &Membership) -> Result<(), TenancyError>;

    /// Returns an existing role assignment by identifier.
    async fn role_assignment(
        &mut self,
        assignment_id: &RoleAssignmentId,
    ) -> Result<Option<RoleAssignment>, TenancyError>;

    /// Inserts an effective role assignment and reports whether this command
    /// created it. Returns [`RoleAssignmentOutcome::Replayed`] for an
    /// identical existing record and [`TenancyError::Conflict`] for divergent
    /// semantics.
    async fn insert_role_assignment(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        assignment_id: &RoleAssignmentId,
        role: Role,
    ) -> Result<RoleAssignmentOutcome, TenancyError>;

    /// Revokes the effective assignment for a role.
    async fn revoke_role_assignment(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError>;

    /// Returns an existing scope grant by identifier.
    async fn scope_grant(
        &mut self,
        grant_id: &ScopeGrantId,
    ) -> Result<Option<ScopeGrant>, TenancyError>;

    /// Inserts an active scope grant, returning an identical existing record
    /// or conflicting on divergent semantics.
    async fn insert_scope_grant(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        grant_id: &ScopeGrantId,
        scope: &Scope,
    ) -> Result<ScopeGrant, TenancyError>;

    /// Revokes an active scope grant.
    async fn revoke_scope_grant(
        &mut self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
    ) -> Result<ScopeGrant, TenancyError>;

    /// Inserts an active branch.
    async fn insert_branch(&mut self, branch: &Branch) -> Result<(), TenancyError>;

    /// Inserts the provisioning bundle. Returns `false` when an identical
    /// bundle already exists.
    async fn provision_organization(
        &mut self,
        provisioned: &ProvisionedOrganization,
    ) -> Result<bool, TenancyError>;

    /// Applies one organization lifecycle transition.
    async fn transition_organization(
        &mut self,
        from: &str,
        to: &str,
    ) -> Result<Option<(String, u64)>, TenancyError>;

    /// Applies one branch lifecycle transition.
    async fn transition_branch(
        &mut self,
        branch_id: &BranchId,
        from: &str,
        to: &str,
    ) -> Result<Option<(String, u64)>, TenancyError>;

    /// Resolves the organization that owns an invitation token.
    async fn resolve_invitation_organization(
        &mut self,
        token_hash: &[u8; 32],
    ) -> Result<Option<OrganizationId>, InvitationError>;

    /// Locks the issued invitation addressed by a token hash.
    async fn lock_issued_invitation(
        &mut self,
        organization_id: &OrganizationId,
        token_hash: &[u8; 32],
    ) -> Result<Option<InvitationRecord>, InvitationError>;

    /// Locks an invitation by identifier.
    async fn lock_invitation(
        &mut self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<Option<InvitationRecord>, InvitationError>;

    /// Inserts an issued invitation.
    async fn insert_invitation(&mut self, invitation: &Invitation) -> Result<(), TenancyError>;

    /// Applies one membership transition during invitation acceptance.
    async fn transition_invitation_membership(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        from: &str,
        to: &str,
    ) -> Result<u64, InvitationError>;

    /// Inserts the role assignment materialized by an accepted invitation.
    async fn insert_invitation_role_assignment(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        assignment_id: &RoleAssignmentId,
        role: Role,
    ) -> Result<RoleAssignment, InvitationError>;

    /// Inserts the scope grant materialized by an accepted invitation.
    async fn insert_invitation_scope_grant(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        grant_id: &ScopeGrantId,
        scope: &Scope,
    ) -> Result<ScopeGrant, InvitationError>;

    /// Claims an issued invitation.
    async fn accept_invitation(
        &mut self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<u64, InvitationError>;

    /// Withdraws an issued invitation.
    async fn withdraw_invitation(
        &mut self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<u64, InvitationError>;

    /// Expires a membership still in the invited state.
    async fn expire_linked_membership(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<(), InvitationError>;

    /// Returns one bounded batch of lapsed invitations for the tenant.
    async fn expired_invitation_batch(
        &mut self,
        organization_id: &OrganizationId,
        limit: i64,
    ) -> Result<Vec<ExpiredInvitation>, TenancyError>;

    /// Expires one lapsed invitation.
    async fn expire_invitation(
        &mut self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<Option<u64>, TenancyError>;

    /// Expires the linked membership when it is still invited.
    async fn expire_invited_membership(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Option<u64>, TenancyError>;

    /// Writes the mandatory audit row and paired outbox row.
    async fn record_evidence(
        &mut self,
        audit: &IamAuditEvent,
        outbox: &OutboxEvent,
    ) -> Result<(), TenancyError>;

    /// Commits the transaction.
    async fn commit(self: Box<Self>) -> Result<(), TenancyError>;
}
