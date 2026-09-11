//! In-memory tenant topology database.
//!
//! Reference implementation of [`TenancyStores`] for tests and local
//! development. Each method holds a single lock for the duration of the
//! operation, simulating one serializable transaction: concurrent callers
//! serialize, so the "one legal transition" invariant (Phase 4 section 21)
//! is provable with threads. Production implementations replace this with
//! PostgreSQL-backed repositories enforcing the same semantics through row
//! locks, uniqueness constraints, and RLS defense in depth (Phase 4 sections
//! 25, 26; Phase 5 owns the schema).
//!
//! Identifier minting belongs to callers: every creation method takes an
//! explicit id, so retries with the same id and payload return the existing
//! record while divergent payloads conflict (Phase 4 section 56, record
//! level; command idempotency keys arrive with the API layer).

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use sitolo_authz::{
    Invitation, InvitationContact, InvitationError, InvitationState, Role, RoleAssignment,
    RoleAssignmentState, Scope, ScopeGrant,
};
use sitolo_domain::tenancy::{
    Branch, BranchId, InvitationId, Membership, MembershipId, MembershipState, Organization,
    OrganizationId, RoleAssignmentId, ScopeGrantId, TenancyError, TenantUserId,
};

/// The atomic result of accepting an invitation: the claimed invitation,
/// the activated membership, the effective role grant, and the scope grant
/// when the invitation proposed a narrowed scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedInvitation {
    pub invitation: Invitation,
    pub membership: Membership,
    pub assignment: RoleAssignment,
    pub scope_grant: Option<ScopeGrant>,
}

/// The atomic result of creating an invitation: the `Invited` membership
/// plus the `Issued` invitation. The raw token is held by the caller only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedInvitation {
    pub membership: Membership,
    pub invitation: Invitation,
}

/// Creation parameters for [`TenancyStores::create_invitation`]. Grouped so
/// the port keeps an explicit, reviewable parameter list instead of eleven
/// positional arguments.
#[derive(Debug, Clone)]
pub struct CreateInvitationInput {
    pub organization_id: OrganizationId,
    pub invitation_id: InvitationId,
    pub membership_id: MembershipId,
    pub user_id: TenantUserId,
    pub proposed_role: Role,
    pub proposed_scope: Option<Scope>,
    pub contact: InvitationContact,
    pub raw_token: String,
    pub ttl: Duration,
    pub now: SystemTime,
}

/// The atomic result of provisioning an organization (Phase 4 section 7.1):
/// organization, owner membership, and default branch committed together.
/// There is no observable state with an organization but no owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedOrganization {
    pub organization: Organization,
    pub owner_membership: Membership,
    pub default_branch: Branch,
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
    async fn create_organization(
        &self,
        id: OrganizationId,
        name: &str,
    ) -> Result<Organization, TenancyError>;
    async fn organization_snapshot(&self, id: &OrganizationId) -> Option<Organization>;
    async fn activate_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError>;
    async fn suspend_organization(&self, id: &OrganizationId)
    -> Result<Organization, TenancyError>;
    async fn resume_organization(&self, id: &OrganizationId) -> Result<Organization, TenancyError>;
    async fn begin_close_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError>;
    async fn close_organization(&self, id: &OrganizationId) -> Result<Organization, TenancyError>;

    // --- branches (scope-carrying) ---
    async fn create_branch(
        &self,
        id: BranchId,
        organization_id: OrganizationId,
        name: &str,
    ) -> Result<Branch, TenancyError>;
    async fn branch_snapshot(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Option<Branch>;
    async fn activate_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;
    async fn suspend_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;
    async fn resume_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;
    async fn begin_close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;
    async fn close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError>;

    // --- memberships (scope-carrying) ---
    async fn invite_membership(
        &self,
        id: MembershipId,
        organization_id: OrganizationId,
        user_id: TenantUserId,
    ) -> Result<Membership, TenancyError>;
    async fn membership_snapshot(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Option<Membership>;
    async fn membership_for_user(
        &self,
        organization_id: &OrganizationId,
        user_id: &TenantUserId,
    ) -> Option<Membership>;
    async fn advance_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;
    async fn expire_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;
    async fn activate_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;
    async fn suspend_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;
    async fn resume_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;
    async fn revoke_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError>;

    // --- atomic provisioning bundle (section 7.1) ---
    //
    // All identifiers are caller-minted: the repository never invents ids,
    // so retries with the same ids are unambiguous. A bundle whose
    // organization id already exists conflicts rather than forking a second
    // owner (command idempotency keys arrive with the API layer).
    async fn provision_organization(
        &self,
        organization_id: OrganizationId,
        organization_name: &str,
        owner_membership_id: MembershipId,
        owner_user_id: TenantUserId,
        branch_id: BranchId,
        branch_name: &str,
    ) -> Result<ProvisionedOrganization, TenancyError>;

    // --- role assignments (organization-wide grants; PR-004 narrows scope) ---
    //
    // Grants are organization-wide until scope grants arrive: assignment
    // records carry the owning organization and membership bindings, and
    // every method enforces them. Authority flows only from `Effective`
    // assignments held by `Active` memberships; resolution lives in the
    // authorization crate and the application service.
    async fn assign_role(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        assignment_id: RoleAssignmentId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError>;
    async fn revoke_role(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError>;
    async fn role_assignments_for(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<RoleAssignment>;

    // --- scope grants (narrowing; PR-004) ---
    //
    // Grants narrow organization-wide role authority to explicit scopes.
    // Structural rules enforced here: the grant scope must sit inside the
    // granting organization (section 25.2), and branch scopes must name an
    // existing branch of that organization. Issuer authority (the widening
    // rule, section 11.2) is enforced by Phase 6 once actor scope exists.
    async fn grant_scope(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        grant_id: ScopeGrantId,
        scope: Scope,
    ) -> Result<ScopeGrant, TenancyError>;
    async fn revoke_scope_grant(
        &self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
    ) -> Result<ScopeGrant, TenancyError>;
    async fn scope_grants_for(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<ScopeGrant>;

    // --- invitations (section 9) ---
    //
    // Token-bound enrollment: creation mints the `Invited` membership and the
    // `Issued` invitation atomically; acceptance claims the token and drives
    // membership activation plus the proposed role/scope grants atomically.
    // The raw token is a caller-minted high-entropy value that is hashed on
    // entry and never persisted (section 9.1).
    async fn create_invitation(
        &self,
        input: CreateInvitationInput,
    ) -> Result<CreatedInvitation, TenancyError>;
    async fn accept_invitation(
        &self,
        token_hash: [u8; 32],
        assignment_id: RoleAssignmentId,
        grant_id: Option<ScopeGrantId>,
        now: SystemTime,
    ) -> Result<AcceptedInvitation, InvitationError>;
    async fn revoke_invitation(
        &self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<Invitation, InvitationError>;
    async fn expire_invitations(&self, now: SystemTime) -> u64;
}

struct TenancyState {
    organizations: BTreeMap<OrganizationId, Organization>,
    branches: BTreeMap<BranchId, Branch>,
    memberships: BTreeMap<MembershipId, Membership>,
    /// Latest membership per (organization, user). Overwritten only when the
    /// previous record reached a terminal state, enforcing at most one
    /// non-terminal membership per pair (Phase 4 section 8.1.3).
    by_org_user: BTreeMap<(OrganizationId, TenantUserId), MembershipId>,
    assignments: BTreeMap<RoleAssignmentId, RoleAssignment>,
    by_membership: BTreeMap<MembershipId, Vec<RoleAssignmentId>>,
    grants: BTreeMap<ScopeGrantId, ScopeGrant>,
    grants_by_membership: BTreeMap<MembershipId, Vec<ScopeGrantId>>,
    invitations: BTreeMap<InvitationId, Invitation>,
    /// Token-hash index: at most one invitation per hash (replay safety,
    /// Phase 5 future unique constraint).
    by_token_hash: BTreeMap<[u8; 32], InvitationId>,
}

/// In-memory tenant topology database.
pub struct TenancyDatabase {
    state: Mutex<TenancyState>,
}

impl TenancyDatabase {
    pub fn new() -> Self {
        TenancyDatabase {
            state: Mutex::new(TenancyState {
                organizations: BTreeMap::new(),
                branches: BTreeMap::new(),
                memberships: BTreeMap::new(),
                by_org_user: BTreeMap::new(),
                assignments: BTreeMap::new(),
                by_membership: BTreeMap::new(),
                grants: BTreeMap::new(),
                grants_by_membership: BTreeMap::new(),
                invitations: BTreeMap::new(),
                by_token_hash: BTreeMap::new(),
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, TenancyState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for TenancyDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TenancyStores for TenancyDatabase {
    async fn create_organization(
        &self,
        id: OrganizationId,
        name: &str,
    ) -> Result<Organization, TenancyError> {
        let mut state = self.lock();
        if let Some(existing) = state.organizations.get(&id) {
            // Record-level idempotency: same id plus same name returns the
            // existing record; divergent semantics conflict (section 56).
            if existing.name == name.trim() {
                return Ok(existing.clone());
            }
            return Err(TenancyError::Conflict);
        }
        let organization = Organization::provision(id.clone(), name)?;
        state.organizations.insert(id, organization.clone());
        Ok(organization)
    }

    async fn organization_snapshot(&self, id: &OrganizationId) -> Option<Organization> {
        self.lock().organizations.get(id).cloned()
    }

    async fn activate_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError> {
        let mut state = self.lock();
        let organization = state
            .organizations
            .get_mut(id)
            .ok_or(TenancyError::NotFound)?;
        organization.activate()?;
        Ok(organization.clone())
    }

    async fn suspend_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError> {
        let mut state = self.lock();
        let organization = state
            .organizations
            .get_mut(id)
            .ok_or(TenancyError::NotFound)?;
        organization.suspend()?;
        Ok(organization.clone())
    }

    async fn resume_organization(&self, id: &OrganizationId) -> Result<Organization, TenancyError> {
        let mut state = self.lock();
        let organization = state
            .organizations
            .get_mut(id)
            .ok_or(TenancyError::NotFound)?;
        organization.resume()?;
        Ok(organization.clone())
    }

    async fn begin_close_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Organization, TenancyError> {
        let mut state = self.lock();
        let organization = state
            .organizations
            .get_mut(id)
            .ok_or(TenancyError::NotFound)?;
        organization.begin_close()?;
        Ok(organization.clone())
    }

    async fn close_organization(&self, id: &OrganizationId) -> Result<Organization, TenancyError> {
        let mut state = self.lock();
        let organization = state
            .organizations
            .get_mut(id)
            .ok_or(TenancyError::NotFound)?;
        organization.close()?;
        Ok(organization.clone())
    }

    async fn create_branch(
        &self,
        id: BranchId,
        organization_id: OrganizationId,
        name: &str,
    ) -> Result<Branch, TenancyError> {
        let mut state = self.lock();
        if let Some(existing) = state.branches.get(&id) {
            if existing.organization_id == organization_id && existing.name == name.trim() {
                return Ok(existing.clone());
            }
            return Err(TenancyError::Conflict);
        }
        // Branch creation is an operating-organization action. The
        // provisioning flow creates the default branch through the atomic
        // bundle instead (section 7.2).
        let organization = state
            .organizations
            .get(&organization_id)
            .ok_or(TenancyError::NotFound)?;
        if !organization.can_operate() {
            return Err(TenancyError::InvalidTransition);
        }
        let branch = Branch::provision(id.clone(), organization_id, name)?;
        state.branches.insert(id, branch.clone());
        Ok(branch)
    }

    async fn branch_snapshot(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Option<Branch> {
        // Tenant predicate: the binding mismatch behaves as absence, so a
        // cross-tenant read discloses nothing (section 27.1).
        self.lock()
            .branches
            .get(id)
            .and_then(|branch| (branch.organization_id == *organization_id).then(|| branch.clone()))
    }

    async fn activate_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        let mut state = self.lock();
        let branch = scoped_branch(&mut state, organization_id, id)?;
        branch.activate()?;
        Ok(branch.clone())
    }

    async fn suspend_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        let mut state = self.lock();
        let branch = scoped_branch(&mut state, organization_id, id)?;
        branch.suspend()?;
        Ok(branch.clone())
    }

    async fn resume_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        let mut state = self.lock();
        let branch = scoped_branch(&mut state, organization_id, id)?;
        branch.resume()?;
        Ok(branch.clone())
    }

    async fn begin_close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        let mut state = self.lock();
        let branch = scoped_branch(&mut state, organization_id, id)?;
        branch.begin_close()?;
        Ok(branch.clone())
    }

    async fn close_branch(
        &self,
        organization_id: &OrganizationId,
        id: &BranchId,
    ) -> Result<Branch, TenancyError> {
        let mut state = self.lock();
        let branch = scoped_branch(&mut state, organization_id, id)?;
        branch.close()?;
        Ok(branch.clone())
    }

    async fn invite_membership(
        &self,
        id: MembershipId,
        organization_id: OrganizationId,
        user_id: TenantUserId,
    ) -> Result<Membership, TenancyError> {
        let mut state = self.lock();
        invite_membership_locked(&mut state, id, organization_id, user_id)
    }

    async fn membership_snapshot(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Option<Membership> {
        self.lock().memberships.get(id).and_then(|membership| {
            (membership.organization_id == *organization_id).then(|| membership.clone())
        })
    }

    async fn membership_for_user(
        &self,
        organization_id: &OrganizationId,
        user_id: &TenantUserId,
    ) -> Option<Membership> {
        let state = self.lock();
        let id = state
            .by_org_user
            .get(&(organization_id.clone(), user_id.clone()))?;
        state.memberships.get(id).cloned()
    }

    async fn advance_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        let mut state = self.lock();
        let membership = scoped_membership(&mut state, organization_id, id)?;
        membership.mark_pending()?;
        Ok(membership.clone())
    }

    async fn expire_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        let mut state = self.lock();
        let membership = scoped_membership(&mut state, organization_id, id)?;
        membership.expire()?;
        Ok(membership.clone())
    }

    async fn activate_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        let mut state = self.lock();
        let membership = scoped_membership(&mut state, organization_id, id)?;
        membership.activate()?;
        Ok(membership.clone())
    }

    async fn suspend_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        let mut state = self.lock();
        let membership = scoped_membership(&mut state, organization_id, id)?;
        membership.suspend()?;
        Ok(membership.clone())
    }

    async fn resume_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        let mut state = self.lock();
        let membership = scoped_membership(&mut state, organization_id, id)?;
        membership.resume()?;
        Ok(membership.clone())
    }

    async fn revoke_membership(
        &self,
        organization_id: &OrganizationId,
        id: &MembershipId,
    ) -> Result<Membership, TenancyError> {
        let mut state = self.lock();
        let membership = scoped_membership(&mut state, organization_id, id)?;
        membership.revoke()?;
        Ok(membership.clone())
    }

    async fn provision_organization(
        &self,
        organization_id: OrganizationId,
        organization_name: &str,
        owner_membership_id: MembershipId,
        owner_user_id: TenantUserId,
        branch_id: BranchId,
        branch_name: &str,
    ) -> Result<ProvisionedOrganization, TenancyError> {
        // Validate everything before mutating: a failure leaves no partial
        // organization behind (section 7.1).
        let mut organization = Organization::provision(organization_id.clone(), organization_name)?;
        let mut branch =
            Branch::provision(branch_id.clone(), organization_id.clone(), branch_name)?;
        let mut owner = Membership::invite(
            owner_membership_id.clone(),
            organization_id.clone(),
            owner_user_id.clone(),
        );

        let mut state = self.lock();
        if state.organizations.contains_key(&organization_id) {
            return Err(TenancyError::Conflict);
        }
        if state.branches.contains_key(&branch_id) {
            return Err(TenancyError::Conflict);
        }
        if state.memberships.contains_key(&owner_membership_id) {
            return Err(TenancyError::Conflict);
        }

        // Drive the legal transitions: setup completes inside the bundle, so
        // the organization and branch become operating and the owner active.
        organization.activate()?;
        branch.activate()?;
        owner.mark_pending()?;
        owner.activate()?;

        state
            .organizations
            .insert(organization_id.clone(), organization.clone());
        state.branches.insert(branch_id, branch.clone());
        state.memberships.insert(owner.id.clone(), owner.clone());
        state
            .by_org_user
            .insert((organization_id, owner_user_id), owner.id.clone());
        Ok(ProvisionedOrganization {
            organization,
            owner_membership: owner,
            default_branch: branch,
        })
    }

    async fn assign_role(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        assignment_id: RoleAssignmentId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError> {
        let mut state = self.lock();
        assign_role_locked(
            &mut state,
            organization_id,
            membership_id,
            assignment_id,
            role,
        )
    }

    async fn revoke_role(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError> {
        let mut state = self.lock();
        // Scope the membership first: foreign grants behave as absent.
        state
            .memberships
            .get(membership_id)
            .filter(|membership| membership.organization_id == *organization_id)
            .ok_or(TenancyError::NotFound)?;
        let target = state
            .by_membership
            .get(membership_id)
            .into_iter()
            .flatten()
            .filter_map(|id| state.assignments.get(id))
            .find(|assignment| {
                assignment.role == role && assignment.state == RoleAssignmentState::Effective
            })
            .map(|assignment| assignment.id.clone())
            .ok_or(TenancyError::NotFound)?;
        let assignment = state
            .assignments
            .get_mut(&target)
            .expect("indexed assignment exists");
        assignment.revoke()?;
        Ok(assignment.clone())
    }

    async fn role_assignments_for(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<RoleAssignment> {
        let state = self.lock();
        // Read path: a binding mismatch yields history for nobody — an empty
        // set, never a cross-tenant record.
        let bound = state
            .memberships
            .get(membership_id)
            .is_some_and(|membership| membership.organization_id == *organization_id);
        if !bound {
            return Vec::new();
        }
        state
            .by_membership
            .get(membership_id)
            .into_iter()
            .flatten()
            .filter_map(|id| state.assignments.get(id).cloned())
            .collect()
    }

    async fn grant_scope(
        &self,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        grant_id: ScopeGrantId,
        scope: Scope,
    ) -> Result<ScopeGrant, TenancyError> {
        let mut state = self.lock();
        grant_scope_locked(&mut state, organization_id, membership_id, grant_id, scope)
    }

    async fn revoke_scope_grant(
        &self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
    ) -> Result<ScopeGrant, TenancyError> {
        let mut state = self.lock();
        let grant = state
            .grants
            .get_mut(grant_id)
            .filter(|grant| grant.organization_id == *organization_id)
            .ok_or(TenancyError::NotFound)?;
        grant.revoke()?;
        Ok(grant.clone())
    }

    async fn scope_grants_for(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Vec<ScopeGrant> {
        let state = self.lock();
        let bound = state
            .memberships
            .get(membership_id)
            .is_some_and(|membership| membership.organization_id == *organization_id);
        if !bound {
            return Vec::new();
        }
        state
            .grants_by_membership
            .get(membership_id)
            .into_iter()
            .flatten()
            .filter_map(|id| state.grants.get(id).cloned())
            .collect()
    }

    async fn create_invitation(
        &self,
        input: CreateInvitationInput,
    ) -> Result<CreatedInvitation, TenancyError> {
        let CreateInvitationInput {
            organization_id,
            invitation_id,
            membership_id,
            user_id,
            proposed_role,
            proposed_scope,
            contact,
            raw_token,
            ttl,
            now,
        } = input;
        // Validate token shape and scope structure before touching state.
        // Contact is revalidated here as well: the repository never trusts
        // caller-built records.
        Invitation::validate_raw_token(&raw_token)?;
        InvitationContact::new(contact.kind, &contact.value)?;
        if let Some(scope) = &proposed_scope
            && scope.organization_id() != &organization_id
        {
            return Err(TenancyError::InvalidTransition);
        }
        let mut state = self.lock();
        // Token-hash uniqueness: one invitation per hash, so a raw token can
        // never enroll twice even across re-issued invitations (section 25.1
        // replay constraints).
        let token_hash = Invitation::hash_token(&raw_token);
        if state.by_token_hash.contains_key(&token_hash) {
            return Err(TenancyError::Conflict);
        }
        // Invitation idempotency mirrors record creation: same id plus same
        // payload returns the existing pair.
        if let Some(existing) = state.invitations.get(&invitation_id) {
            let same = existing.organization_id == organization_id
                && existing.membership_id == membership_id
                && existing.proposed_role == proposed_role
                && existing.proposed_scope == proposed_scope
                && existing.contact == contact
                && existing.token_hash == token_hash;
            if same {
                let membership = state
                    .memberships
                    .get(&membership_id)
                    .cloned()
                    .ok_or(TenancyError::NotFound)?;
                return Ok(CreatedInvitation {
                    membership,
                    invitation: existing.clone(),
                });
            }
            return Err(TenancyError::Conflict);
        }
        // Proposed branch scopes must name an existing branch now; operating
        // state is rechecked at acceptance.
        if let Some(Scope::Branch { branch_id, .. }) = &proposed_scope {
            let branch = state
                .branches
                .get(branch_id)
                .filter(|branch| branch.organization_id == organization_id);
            if branch.is_none() {
                return Err(TenancyError::NotFound);
            }
        }
        // The membership and invitation commit together: no dangling
        // invitation without a membership to activate.
        let membership = invite_membership_locked(
            &mut state,
            membership_id.clone(),
            organization_id.clone(),
            user_id,
        )?;
        let invitation = Invitation::issue(
            invitation_id.clone(),
            organization_id,
            membership_id,
            proposed_role,
            proposed_scope,
            contact,
            &raw_token,
            now,
            ttl,
        )?;
        state
            .invitations
            .insert(invitation_id.clone(), invitation.clone());
        state.by_token_hash.insert(token_hash, invitation_id);
        Ok(CreatedInvitation {
            membership,
            invitation,
        })
    }

    async fn accept_invitation(
        &self,
        token_hash: [u8; 32],
        assignment_id: RoleAssignmentId,
        grant_id: Option<ScopeGrantId>,
        now: SystemTime,
    ) -> Result<AcceptedInvitation, InvitationError> {
        let mut state = self.lock();
        // The token identifies the invitation; everything authoritative
        // comes from the loaded record (sections 9.2, T4).
        let invitation_id = state
            .by_token_hash
            .get(&token_hash)
            .cloned()
            .ok_or(InvitationError::Invalid)?;
        // Read-only gate first: state and expiry decide the outcome before
        // any mutation, so concurrent attempts serialize to exactly one
        // success (sections 9.3, 22.3).
        let gate = {
            let invitation = state
                .invitations
                .get(&invitation_id)
                .ok_or(InvitationError::Invalid)?;
            match invitation.state {
                InvitationState::Accepted => return Err(InvitationError::AlreadyAccepted),
                InvitationState::Revoked => return Err(InvitationError::Invalid),
                InvitationState::Expired => return Err(InvitationError::Expired),
                InvitationState::Issued => {}
            }
            if now >= invitation.expires_at {
                None
            } else {
                Some((
                    invitation.organization_id.clone(),
                    invitation.membership_id.clone(),
                    invitation.proposed_role,
                    invitation.proposed_scope.clone(),
                ))
            }
        };
        let Some((organization_id, membership_id, proposed_role, proposed_scope)) = gate else {
            // Lapsed while issued: expire the invitation and release the
            // linked membership for re-enrollment.
            {
                let invitation = state
                    .invitations
                    .get_mut(&invitation_id)
                    .ok_or(InvitationError::Invalid)?;
                let _ = invitation.accept(now);
            }
            if let Some(invitation) = state.invitations.get(&invitation_id).cloned() {
                expire_linked_membership(&mut state, &invitation);
            }
            return Err(InvitationError::Expired);
        };
        // The grant id must accompany a proposed scope exactly: role and
        // scope materialize from the record, never from caller input (T4).
        if proposed_scope.is_some() != grant_id.is_some() {
            return Err(InvitationError::Invalid);
        }
        // Pre-validate every mutation before applying any: membership must
        // still be Invited, and neither the role grant nor the scope grant
        // may already exist effectively.
        {
            let pending = state
                .memberships
                .get(&membership_id)
                .filter(|membership| {
                    membership.organization_id == organization_id
                        && membership.state == MembershipState::Invited
                })
                .is_some();
            if !pending {
                return Err(InvitationError::Invalid);
            }
            if state.assignments.contains_key(&assignment_id) {
                return Err(InvitationError::Invalid);
            }
            let role_taken = state
                .by_membership
                .get(&membership_id)
                .into_iter()
                .flatten()
                .filter_map(|id| state.assignments.get(id))
                .any(|assignment| {
                    assignment.role == proposed_role
                        && assignment.state == RoleAssignmentState::Effective
                });
            if role_taken {
                return Err(InvitationError::Invalid);
            }
            if let Some(grant_id) = &grant_id
                && state.grants.contains_key(grant_id)
            {
                return Err(InvitationError::Invalid);
            }
        }
        // All checks passed: drive membership activation, the role grant,
        // the scope grant, and the invitation claim together.
        let membership =
            activate_invited_membership_locked(&mut state, &organization_id, &membership_id)
                .map_err(|_| InvitationError::Invalid)?;
        let assignment = assign_role_locked(
            &mut state,
            organization_id.clone(),
            membership_id.clone(),
            assignment_id,
            proposed_role,
        )
        .map_err(|_| InvitationError::Invalid)?;
        let scope_grant = match (proposed_scope, grant_id) {
            (Some(scope), Some(grant_id)) => Some(
                grant_scope_locked(
                    &mut state,
                    organization_id.clone(),
                    membership_id.clone(),
                    grant_id,
                    scope,
                )
                .map_err(|_| InvitationError::Invalid)?,
            ),
            (None, None) => None,
            _ => return Err(InvitationError::Invalid),
        };
        let invitation = state
            .invitations
            .get_mut(&invitation_id)
            .ok_or(InvitationError::Invalid)?;
        invitation.accept(now)?;
        let invitation = invitation.clone();
        Ok(AcceptedInvitation {
            invitation,
            membership,
            assignment,
            scope_grant,
        })
    }

    async fn revoke_invitation(
        &self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<Invitation, InvitationError> {
        let mut state = self.lock();
        let target = state
            .invitations
            .get(invitation_id)
            .filter(|invitation| invitation.organization_id == *organization_id)
            .map(|invitation| invitation.id.clone())
            .ok_or(InvitationError::Invalid)?;
        {
            let invitation = state
                .invitations
                .get_mut(&target)
                .ok_or(InvitationError::Invalid)?;
            invitation.revoke()?;
        }
        // Withdrawing the invitation releases the linked membership for
        // re-enrollment: Invited -> Pending -> Expired.
        if let Some(invitation) = state.invitations.get(&target).cloned() {
            expire_linked_membership(&mut state, &invitation);
        }
        state
            .invitations
            .get(&target)
            .cloned()
            .ok_or(InvitationError::Invalid)
    }

    async fn expire_invitations(&self, now: SystemTime) -> u64 {
        let mut state = self.lock();
        // Collect first: the sweep mutates records it inspects.
        let lapsed: Vec<InvitationId> = state
            .invitations
            .values()
            .filter(|invitation| {
                invitation.state == InvitationState::Issued && now >= invitation.expires_at
            })
            .map(|invitation| invitation.id.clone())
            .collect();
        let mut expired = 0u64;
        for id in lapsed {
            if let Some(invitation) = state.invitations.get_mut(&id) {
                let _ = invitation.accept(now);
            }
            if let Some(invitation) = state.invitations.get(&id).cloned() {
                expire_linked_membership(&mut state, &invitation);
                expired += 1;
            }
        }
        expired
    }
}

/// Scope-enforcing branch access: the binding mismatch behaves as absence.
fn scoped_branch<'a>(
    state: &'a mut TenancyState,
    organization_id: &OrganizationId,
    id: &BranchId,
) -> Result<&'a mut Branch, TenancyError> {
    let branch = state.branches.get_mut(id).ok_or(TenancyError::NotFound)?;
    if branch.organization_id != *organization_id {
        return Err(TenancyError::NotFound);
    }
    Ok(branch)
}

/// Scope-enforcing membership access: the binding mismatch behaves as absence.
fn scoped_membership<'a>(
    state: &'a mut TenancyState,
    organization_id: &OrganizationId,
    id: &MembershipId,
) -> Result<&'a mut Membership, TenancyError> {
    let membership = state
        .memberships
        .get_mut(id)
        .ok_or(TenancyError::NotFound)?;
    if membership.organization_id != *organization_id {
        return Err(TenancyError::NotFound);
    }
    Ok(membership)
}

/// Locked-handle membership enrollment shared by the public invite path and
/// the atomic invitation-creation path below. Enforces the organization
/// gate and the one-non-terminal-membership invariant (section 8.1.3).
fn invite_membership_locked(
    state: &mut TenancyState,
    id: MembershipId,
    organization_id: OrganizationId,
    user_id: TenantUserId,
) -> Result<Membership, TenancyError> {
    if let Some(existing) = state.memberships.get(&id) {
        if existing.organization_id == organization_id && existing.user_id == user_id {
            return Ok(existing.clone());
        }
        return Err(TenancyError::Conflict);
    }
    let organization = state
        .organizations
        .get(&organization_id)
        .ok_or(TenancyError::NotFound)?;
    // Enrollment stays open while the organization operates or is
    // suspended; provisioning, closing and closed organizations cannot
    // gain members.
    if !matches!(
        organization.state,
        sitolo_domain::tenancy::OrganizationState::Active
            | sitolo_domain::tenancy::OrganizationState::Suspended
    ) {
        return Err(TenancyError::InvalidTransition);
    }
    let incumbent = state
        .by_org_user
        .get(&(organization_id.clone(), user_id.clone()))
        .cloned()
        .and_then(|current_id| state.memberships.get(&current_id))
        .filter(|current| !current.state.is_terminal());
    if incumbent.is_some() {
        return Err(TenancyError::Conflict);
    }
    let membership = Membership::invite(id.clone(), organization_id.clone(), user_id.clone());
    state.memberships.insert(id.clone(), membership.clone());
    state.by_org_user.insert((organization_id, user_id), id);
    Ok(membership)
}

/// Locked-handle role grant shared by the public path and invitation
/// acceptance. Drives the section-19 machine to Effective.
fn assign_role_locked(
    state: &mut TenancyState,
    organization_id: OrganizationId,
    membership_id: MembershipId,
    assignment_id: RoleAssignmentId,
    role: Role,
) -> Result<RoleAssignment, TenancyError> {
    // The grant target must be a live membership of this organization.
    // Terminal memberships cannot receive authority.
    let membership = state
        .memberships
        .get(&membership_id)
        .filter(|membership| membership.organization_id == organization_id)
        .ok_or(TenancyError::NotFound)?;
    if membership.state.is_terminal() {
        return Err(TenancyError::InvalidTransition);
    }
    // Record-level idempotency on the assignment id.
    if let Some(existing) = state.assignments.get(&assignment_id) {
        if existing.organization_id == organization_id
            && existing.membership_id == membership_id
            && existing.role == role
        {
            return Ok(existing.clone());
        }
        return Err(TenancyError::Conflict);
    }
    // One effective grant per (membership, role).
    let duplicate = state
        .by_membership
        .get(&membership_id)
        .into_iter()
        .flatten()
        .filter_map(|id| state.assignments.get(id))
        .any(|assignment| {
            assignment.role == role && assignment.state == RoleAssignmentState::Effective
        });
    if duplicate {
        return Err(TenancyError::Conflict);
    }
    let mut assignment = RoleAssignment::request(
        assignment_id.clone(),
        organization_id,
        membership_id.clone(),
        role,
    );
    assignment.begin_validation()?;
    assignment.mark_effective()?;
    state
        .assignments
        .insert(assignment_id.clone(), assignment.clone());
    state
        .by_membership
        .entry(membership_id)
        .or_default()
        .push(assignment_id);
    Ok(assignment)
}

/// Locked-handle scope narrowing shared by the public path and invitation
/// acceptance.
fn grant_scope_locked(
    state: &mut TenancyState,
    organization_id: OrganizationId,
    membership_id: MembershipId,
    grant_id: ScopeGrantId,
    scope: Scope,
) -> Result<ScopeGrant, TenancyError> {
    // The grant scope must sit inside the granting organization
    // (section 25.2 cross-tenant consistency).
    if scope.organization_id() != &organization_id {
        return Err(TenancyError::InvalidTransition);
    }
    // Branch scopes must name an existing branch of this organization.
    if let Scope::Branch { branch_id, .. } = &scope {
        let branch = state
            .branches
            .get(branch_id)
            .filter(|branch| branch.organization_id == organization_id);
        if branch.is_none() {
            return Err(TenancyError::NotFound);
        }
    }
    // The grant target must be a live membership of this organization.
    let membership = state
        .memberships
        .get(&membership_id)
        .filter(|membership| membership.organization_id == organization_id)
        .ok_or(TenancyError::NotFound)?;
    if membership.state.is_terminal() {
        return Err(TenancyError::InvalidTransition);
    }
    // Record-level idempotency on the grant id.
    if let Some(existing) = state.grants.get(&grant_id) {
        if existing.organization_id == organization_id
            && existing.membership_id == membership_id
            && existing.scope == scope
        {
            return Ok(existing.clone());
        }
        return Err(TenancyError::Conflict);
    }
    // One active narrowing per (membership, scope).
    let duplicate = state
        .grants_by_membership
        .get(&membership_id)
        .into_iter()
        .flatten()
        .filter_map(|id| state.grants.get(id))
        .any(|grant| grant.scope == scope && grant.state.narrows());
    if duplicate {
        return Err(TenancyError::Conflict);
    }
    let grant = ScopeGrant::grant(
        grant_id.clone(),
        organization_id,
        membership_id.clone(),
        scope,
    );
    state.grants.insert(grant_id.clone(), grant.clone());
    state
        .grants_by_membership
        .entry(membership_id)
        .or_default()
        .push(grant_id);
    Ok(grant)
}

/// Drives a freshly invited membership to `Active` inside a composite
/// operation (provisioning bundle, invitation acceptance).
fn activate_invited_membership_locked(
    state: &mut TenancyState,
    organization_id: &OrganizationId,
    membership_id: &MembershipId,
) -> Result<Membership, TenancyError> {
    let membership = scoped_membership(state, organization_id, membership_id)?;
    membership.mark_pending()?;
    membership.activate()?;
    Ok(membership.clone())
}

/// Releases an invitation-linked membership for re-enrollment after the
/// invitation lapses or is withdrawn: `Invited -> Pending -> Expired`.
/// Best-effort cleanup — the invitation terminal state is authoritative;
/// a membership that already advanced keeps its state.
fn expire_linked_membership(state: &mut TenancyState, invitation: &Invitation) {
    if let Ok(membership) = scoped_membership(
        state,
        &invitation.organization_id,
        &invitation.membership_id,
    ) && membership.state == MembershipState::Invited
        && membership.mark_pending().is_ok()
    {
        let _ = membership.expire();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_authz::ContactKind;
    use sitolo_domain::tenancy::{BranchState, MembershipState, OrganizationState};

    fn org_id(value: &str) -> OrganizationId {
        OrganizationId::new(value).unwrap()
    }

    fn user(value: &str) -> TenantUserId {
        TenantUserId::new(value).unwrap()
    }

    async fn active_org(db: &TenancyDatabase, id: &str) -> Organization {
        let org = db
            .create_organization(org_id(id), "Merchant")
            .await
            .unwrap();
        db.activate_organization(&org.id).await.unwrap()
    }

    #[test]
    fn database_constructs_default() {
        let _ = TenancyDatabase::default();
    }

    #[tokio::test]
    async fn organization_create_is_idempotent_on_same_payload() {
        let db = TenancyDatabase::new();
        let first = db
            .create_organization(org_id("o1"), "Merchant")
            .await
            .unwrap();
        let retry = db
            .create_organization(org_id("o1"), "Merchant")
            .await
            .unwrap();
        assert_eq!(first, retry);
        // Same id with divergent semantics conflicts (section 56).
        assert_eq!(
            db.create_organization(org_id("o1"), "Other Name").await,
            Err(TenancyError::Conflict)
        );
    }

    #[tokio::test]
    async fn organization_lifecycle_persists() {
        let db = TenancyDatabase::new();
        let org = active_org(&db, "o1").await;
        assert!(org.can_operate());
        let suspended = db.suspend_organization(&org.id).await.unwrap();
        assert!(!suspended.can_operate());
        assert_eq!(
            db.organization_snapshot(&org.id).await.unwrap().state,
            OrganizationState::Suspended
        );
        assert_eq!(
            db.activate_organization(&org.id).await,
            Err(TenancyError::InvalidTransition)
        );
        assert_eq!(
            db.suspend_organization(&org_id("missing")).await,
            Err(TenancyError::NotFound)
        );
    }

    #[tokio::test]
    async fn branch_requires_operating_organization() {
        let db = TenancyDatabase::new();
        // Missing organization behaves as absent.
        assert_eq!(
            db.create_branch(BranchId::new("b1").unwrap(), org_id("o-missing"), "Branch")
                .await,
            Err(TenancyError::NotFound)
        );
        // Provisioning organizations cannot gain branches outside the bundle.
        db.create_organization(org_id("o1"), "Merchant")
            .await
            .unwrap();
        assert_eq!(
            db.create_branch(BranchId::new("b1").unwrap(), org_id("o1"), "Branch")
                .await,
            Err(TenancyError::InvalidTransition)
        );
        // Active organizations can.
        db.activate_organization(&org_id("o1")).await.unwrap();
        let branch = db
            .create_branch(BranchId::new("b1").unwrap(), org_id("o1"), "Branch")
            .await
            .unwrap();
        assert_eq!(branch.state, BranchState::Provisioning);
        // Closed organizations cannot.
        db.begin_close_organization(&org_id("o1")).await.unwrap();
        assert_eq!(
            db.create_branch(BranchId::new("b2").unwrap(), org_id("o1"), "Other")
                .await,
            Err(TenancyError::InvalidTransition)
        );
    }

    #[tokio::test]
    async fn branch_reads_enforce_tenant_predicate() {
        let db = TenancyDatabase::new();
        active_org(&db, "o1").await;
        active_org(&db, "o2").await;
        let branch = db
            .create_branch(BranchId::new("b1").unwrap(), org_id("o1"), "Branch")
            .await
            .unwrap();
        // Cross-tenant read behaves as absence (section 27.1).
        assert!(
            db.branch_snapshot(&org_id("o2"), &branch.id)
                .await
                .is_none()
        );
        assert!(
            db.branch_snapshot(&org_id("o1"), &branch.id)
                .await
                .is_some()
        );
        // Cross-tenant transitions behave as absence.
        assert_eq!(
            db.suspend_branch(&org_id("o2"), &branch.id).await,
            Err(TenancyError::NotFound)
        );
    }

    #[tokio::test]
    async fn membership_uniqueness_holds_per_org_user() {
        let db = TenancyDatabase::new();
        active_org(&db, "o1").await;
        let first = db
            .invite_membership(MembershipId::new("m1").unwrap(), org_id("o1"), user("u1"))
            .await
            .unwrap();
        assert_eq!(first.state, MembershipState::Invited);
        // A second live membership for the same pair conflicts (8.1.3).
        assert_eq!(
            db.invite_membership(MembershipId::new("m2").unwrap(), org_id("o1"), user("u1"))
                .await,
            Err(TenancyError::Conflict)
        );
        // Same membership id with the same payload is a safe retry.
        let retry = db
            .invite_membership(MembershipId::new("m1").unwrap(), org_id("o1"), user("u1"))
            .await
            .unwrap();
        assert_eq!(first, retry);
        // Same membership id with divergent payload conflicts.
        assert_eq!(
            db.invite_membership(MembershipId::new("m1").unwrap(), org_id("o1"), user("u2"))
                .await,
            Err(TenancyError::Conflict)
        );
        // After revocation the user may be re-enrolled with a new record.
        db.advance_membership(&org_id("o1"), &first.id)
            .await
            .unwrap();
        db.activate_membership(&org_id("o1"), &first.id)
            .await
            .unwrap();
        db.revoke_membership(&org_id("o1"), &first.id)
            .await
            .unwrap();
        let second = db
            .invite_membership(MembershipId::new("m2").unwrap(), org_id("o1"), user("u1"))
            .await
            .unwrap();
        assert_eq!(second.state, MembershipState::Invited);
    }

    #[tokio::test]
    async fn membership_reads_are_scope_aware() {
        let db = TenancyDatabase::new();
        active_org(&db, "o1").await;
        active_org(&db, "o2").await;
        let membership = db
            .invite_membership(MembershipId::new("m1").unwrap(), org_id("o1"), user("u1"))
            .await
            .unwrap();
        assert!(
            db.membership_snapshot(&org_id("o2"), &membership.id)
                .await
                .is_none()
        );
        assert!(
            db.membership_for_user(&org_id("o2"), &user("u1"))
                .await
                .is_none()
        );
        assert!(
            db.membership_for_user(&org_id("o1"), &user("u1"))
                .await
                .is_some()
        );
        assert_eq!(
            db.revoke_membership(&org_id("o2"), &membership.id).await,
            Err(TenancyError::NotFound)
        );
    }

    #[tokio::test]
    async fn concurrent_invites_allow_exactly_one() {
        let db = std::sync::Arc::new(TenancyDatabase::new());
        // Already inside the test runtime: seed directly without nesting.
        active_org(&db, "o1").await;
        let outcomes: Vec<_> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|i| {
                    let db = std::sync::Arc::clone(&db);
                    scope.spawn(move || {
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();
                        runtime.block_on(db.invite_membership(
                            MembershipId::new(format!("m-race-{i}")).unwrap(),
                            org_id("o1"),
                            user("u-race"),
                        ))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("thread"))
                .collect()
        });
        assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
        assert!(
            outcomes
                .iter()
                .all(|r| r.is_ok() || *r == Err(TenancyError::Conflict))
        );
    }

    #[tokio::test]
    async fn provision_bundle_is_atomic() {
        let db = TenancyDatabase::new();
        let bundle = db
            .provision_organization(
                org_id("o1"),
                "Merchant",
                MembershipId::new("m-owner").unwrap(),
                user("owner"),
                BranchId::new("b1").unwrap(),
                "Main Branch",
            )
            .await
            .unwrap();
        assert!(bundle.organization.can_operate());
        assert!(bundle.owner_membership.has_authority());
        assert!(bundle.default_branch.can_operate());
        assert_eq!(
            bundle.owner_membership.organization_id,
            bundle.organization.id
        );
        assert_eq!(
            bundle.default_branch.organization_id,
            bundle.organization.id
        );
        // Re-provisioning the same organization conflicts: no forked owners.
        assert_eq!(
            db.provision_organization(
                org_id("o1"),
                "Merchant",
                MembershipId::new("m-owner-2").unwrap(),
                user("owner"),
                BranchId::new("b2").unwrap(),
                "Other Branch",
            )
            .await,
            Err(TenancyError::Conflict)
        );
    }

    #[tokio::test]
    async fn provision_bundle_leaves_no_partial_state() {
        let db = TenancyDatabase::new();
        // Invalid branch name fails validation before any mutation (7.1).
        assert_eq!(
            db.provision_organization(
                org_id("o1"),
                "Merchant",
                MembershipId::new("m-owner").unwrap(),
                user("owner"),
                BranchId::new("b1").unwrap(),
                "",
            )
            .await,
            Err(TenancyError::InvalidName)
        );
        assert!(db.organization_snapshot(&org_id("o1")).await.is_none());
        assert!(
            db.branch_snapshot(&org_id("o1"), &BranchId::new("b1").unwrap())
                .await
                .is_none()
        );
        assert!(
            db.membership_for_user(&org_id("o1"), &user("owner"))
                .await
                .is_none()
        );
    }

    async fn active_member(
        db: &TenancyDatabase,
        org: &str,
        member: &str,
        user_id: &str,
    ) -> Membership {
        let membership = db
            .invite_membership(
                MembershipId::new(member).unwrap(),
                org_id(org),
                user(user_id),
            )
            .await
            .unwrap();
        db.advance_membership(&org_id(org), &membership.id)
            .await
            .unwrap();
        db.activate_membership(&org_id(org), &membership.id)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn role_grant_lifecycle_preserves_history() {
        use sitolo_authz::RoleAssignmentState;

        let db = TenancyDatabase::new();
        active_org(&db, "o1").await;
        let membership = active_member(&db, "o1", "m1", "u1").await;

        let grant = db
            .assign_role(
                org_id("o1"),
                membership.id.clone(),
                RoleAssignmentId::new("ra-1").unwrap(),
                Role::Cashier,
            )
            .await
            .unwrap();
        assert_eq!(grant.state, RoleAssignmentState::Effective);

        // Same assignment id with the same payload is a safe retry.
        let retry = db
            .assign_role(
                org_id("o1"),
                membership.id.clone(),
                RoleAssignmentId::new("ra-1").unwrap(),
                Role::Cashier,
            )
            .await
            .unwrap();
        assert_eq!(grant, retry);

        // A second live grant of the same role conflicts rather than
        // duplicating authority.
        assert_eq!(
            db.assign_role(
                org_id("o1"),
                membership.id.clone(),
                RoleAssignmentId::new("ra-2").unwrap(),
                Role::Cashier,
            )
            .await,
            Err(TenancyError::Conflict)
        );

        let revoked = db
            .revoke_role(&org_id("o1"), &membership.id, Role::Cashier)
            .await
            .unwrap();
        assert_eq!(revoked.state, RoleAssignmentState::Revoked);
        // Revoking again finds no effective grant.
        assert_eq!(
            db.revoke_role(&org_id("o1"), &membership.id, Role::Cashier)
                .await,
            Err(TenancyError::NotFound)
        );

        // Re-granting creates a new record; the revoked one stays revoked.
        let second = db
            .assign_role(
                org_id("o1"),
                membership.id.clone(),
                RoleAssignmentId::new("ra-3").unwrap(),
                Role::Cashier,
            )
            .await
            .unwrap();
        assert_ne!(second.id, revoked.id);
        let history = db.role_assignments_for(&org_id("o1"), &membership.id).await;
        assert_eq!(history.len(), 2);
    }

    #[tokio::test]
    async fn role_grants_enforce_membership_scope() {
        let db = TenancyDatabase::new();
        active_org(&db, "o1").await;
        active_org(&db, "o2").await;
        let membership = active_member(&db, "o1", "m1", "u1").await;

        // A grant naming a foreign organization is denied as absent.
        assert_eq!(
            db.assign_role(
                org_id("o2"),
                membership.id.clone(),
                RoleAssignmentId::new("ra-x").unwrap(),
                Role::Cashier,
            )
            .await,
            Err(TenancyError::NotFound)
        );
        // Reads across the boundary expose nothing.
        assert!(
            db.role_assignments_for(&org_id("o2"), &membership.id)
                .await
                .is_empty()
        );
        assert_eq!(
            db.revoke_role(&org_id("o2"), &membership.id, Role::Cashier)
                .await,
            Err(TenancyError::NotFound)
        );

        // Terminal memberships cannot receive authority.
        db.revoke_membership(&org_id("o1"), &membership.id)
            .await
            .unwrap();
        assert_eq!(
            db.assign_role(
                org_id("o1"),
                membership.id.clone(),
                RoleAssignmentId::new("ra-y").unwrap(),
                Role::Cashier,
            )
            .await,
            Err(TenancyError::InvalidTransition)
        );
    }

    const INVITE_TOKEN: &str = "tok-invite-repo-0123456789abcdef";
    const INVITE_TOKEN_2: &str = "tok-invite-repo-second-0123456789";

    fn mailto(value: &str) -> InvitationContact {
        InvitationContact::new(ContactKind::Email, value).unwrap()
    }

    fn invite_input(token: &str) -> CreateInvitationInput {
        CreateInvitationInput {
            organization_id: org_id("o1"),
            invitation_id: InvitationId::new("inv-1").unwrap(),
            membership_id: MembershipId::new("m-invited").unwrap(),
            user_id: user("invitee"),
            proposed_role: Role::Cashier,
            proposed_scope: None,
            contact: mailto("invitee@example.com"),
            raw_token: token.to_string(),
            ttl: Duration::from_secs(3_600),
            now: SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
        }
    }

    #[tokio::test]
    async fn invitation_create_validates_before_mutating() {
        let db = TenancyDatabase::new();
        active_org(&db, "o1").await;

        let created = db
            .create_invitation(invite_input(INVITE_TOKEN))
            .await
            .unwrap();
        assert!(!created.membership.has_authority());
        assert_eq!(
            created.invitation.token_hash,
            Invitation::hash_token(INVITE_TOKEN)
        );

        // Raw-token reuse across invitations conflicts (replay safety).
        let mut clash = invite_input(INVITE_TOKEN);
        clash.invitation_id = InvitationId::new("inv-2").unwrap();
        clash.membership_id = MembershipId::new("m-other").unwrap();
        clash.user_id = user("other");
        assert_eq!(
            db.create_invitation(clash).await,
            Err(TenancyError::Conflict)
        );

        // Invalid contact leaves no membership behind. The repository
        // revalidates caller-built records instead of trusting them.
        let mut bad_contact = invite_input(INVITE_TOKEN_2);
        bad_contact.membership_id = MembershipId::new("m-bad").unwrap();
        bad_contact.user_id = user("bad");
        bad_contact.contact = InvitationContact {
            kind: ContactKind::Email,
            value: "not-an-email".into(),
        };
        assert_eq!(
            db.create_invitation(bad_contact).await,
            Err(TenancyError::InvalidInvitation)
        );
        assert!(
            db.membership_for_user(&org_id("o1"), &user("bad"))
                .await
                .is_none()
        );
        // Invalid token shape likewise mutates nothing.
        let mut bad_token = invite_input("short");
        bad_token.membership_id = MembershipId::new("m-bad2").unwrap();
        bad_token.user_id = user("bad2");
        assert_eq!(
            db.create_invitation(bad_token).await,
            Err(TenancyError::InvalidInvitation)
        );
        assert!(
            db.membership_for_user(&org_id("o1"), &user("bad2"))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn invitation_accept_is_atomic_and_single_use() {
        let db = TenancyDatabase::new();
        active_org(&db, "o1").await;
        let created = db
            .create_invitation(invite_input(INVITE_TOKEN))
            .await
            .unwrap();
        let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_001);

        let accepted = db
            .accept_invitation(
                Invitation::hash_token(INVITE_TOKEN),
                RoleAssignmentId::new("ra-inv").unwrap(),
                None,
                at,
            )
            .await
            .unwrap();
        assert!(accepted.membership.has_authority());
        assert_eq!(accepted.assignment.role, Role::Cashier);
        assert_eq!(accepted.scope_grant, None);

        // Replay reports prior acceptance; no duplicate grant exists.
        assert_eq!(
            db.accept_invitation(
                Invitation::hash_token(INVITE_TOKEN),
                RoleAssignmentId::new("ra-replay").unwrap(),
                None,
                at,
            )
            .await,
            Err(InvitationError::AlreadyAccepted)
        );
        assert_eq!(
            db.role_assignments_for(&org_id("o1"), &created.membership.id)
                .await
                .len(),
            1
        );
        // Unknown hashes are generically invalid.
        assert_eq!(
            db.accept_invitation(
                Invitation::hash_token("tok-unknown-0123456789abcdefghijkl"),
                RoleAssignmentId::new("ra-ghost").unwrap(),
                None,
                at,
            )
            .await,
            Err(InvitationError::Invalid)
        );
    }

    #[tokio::test]
    async fn invitation_expiry_releases_membership() {
        let db = TenancyDatabase::new();
        active_org(&db, "o1").await;
        let mut input = invite_input(INVITE_TOKEN);
        input.ttl = Duration::from_secs(60);
        let created = db.create_invitation(input).await.unwrap();

        // Acceptance after expiry fails and cascades membership expiry.
        let late = SystemTime::UNIX_EPOCH + Duration::from_secs(1_061);
        assert_eq!(
            db.accept_invitation(
                Invitation::hash_token(INVITE_TOKEN),
                RoleAssignmentId::new("ra-late").unwrap(),
                None,
                late,
            )
            .await,
            Err(InvitationError::Expired)
        );
        let membership = db
            .membership_snapshot(&org_id("o1"), &created.membership.id)
            .await
            .unwrap();
        assert!(!membership.has_authority());

        // The sweeper collects lapsed invitations without acceptance attempts.
        let mut second = invite_input(INVITE_TOKEN_2);
        second.invitation_id = InvitationId::new("inv-2").unwrap();
        second.membership_id = MembershipId::new("m-2").unwrap();
        second.user_id = user("u2");
        second.ttl = Duration::from_secs(60);
        db.create_invitation(second).await.unwrap();
        assert_eq!(db.expire_invitations(late).await, 1);
        assert_eq!(db.expire_invitations(late).await, 0);
    }

    #[tokio::test]
    async fn concurrent_accepts_allow_exactly_one() {
        let db = std::sync::Arc::new(TenancyDatabase::new());
        // Already inside the test runtime: seed directly without nesting.
        active_org(&db, "o1").await;
        db.create_invitation(invite_input(INVITE_TOKEN))
            .await
            .unwrap();
        let hash = Invitation::hash_token(INVITE_TOKEN);
        let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_001);
        let outcomes: Vec<_> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|i| {
                    let db = std::sync::Arc::clone(&db);
                    scope.spawn(move || {
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();
                        runtime.block_on(db.accept_invitation(
                            hash,
                            RoleAssignmentId::new(format!("ra-race-{i}")).unwrap(),
                            None,
                            at,
                        ))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("thread"))
                .collect()
        });
        assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
        assert!(
            outcomes
                .iter()
                .all(|r| r.is_ok() || *r == Err(InvitationError::AlreadyAccepted))
        );
    }
}
