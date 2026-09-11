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

use async_trait::async_trait;
use sitolo_authz::{Role, RoleAssignment, RoleAssignmentState, Scope, ScopeGrant};
use sitolo_domain::tenancy::{
    Branch, BranchId, Membership, MembershipId, Organization, OrganizationId, RoleAssignmentId,
    ScopeGrantId, TenancyError, TenantUserId,
};

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
        // At most one non-terminal membership per (organization, user)
        // (section 8.1.3). The single lock serializes concurrent invites so
        // exactly one wins (section 22).
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
        // One effective grant per (membership, role): a retry mints the same
        // assignment id and hits the path above; a genuinely new grant for an
        // already-held role conflicts instead of duplicating authority.
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
        // Drive the section-19 machine to Effective: validation here is the
        // membership/catalog checks above. Approval-gated grants will pause
        // at ApprovalRequired once Phase 6 classifies high-risk transitions.
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
        // The grant scope must sit inside the granting organization
        // (section 25.2 cross-tenant consistency).
        if scope.organization_id() != &organization_id {
            return Err(TenancyError::InvalidTransition);
        }
        let mut state = self.lock();
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
        // One active narrowing per (membership, scope): a retry reuses the
        // grant id above; a genuinely new narrowing of an already-narrowed
        // scope conflicts instead of stacking duplicates.
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
