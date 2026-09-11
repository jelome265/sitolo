//! Tenancy service — Phase 4 organization and membership use cases.
//!
//! Orchestrates organization provisioning, branch lifecycle, and membership
//! lifecycle against the tenancy persistence port. Each use case applies one
//! domain transition per call; the repository owns the transaction boundary
//! (Phase 4 section 21).
//!
//! Explicitly deferred to later Phase 4 PRs: role/permission checks (PR-003),
//! invitation delivery (PR-005), audit/outbox emission (PR-009). These
//! methods enforce state-machine and scope correctness, not caller
//! authorization — actor authority is enforced by the authorization engine
//! (Phase 6) once roles exist. Public invitation acceptance and other
//! caller-authenticated flows arrive with the API layer (PR-006).

use std::sync::Arc;

use sitolo_domain::tenancy::{
    Branch, BranchId, Membership, MembershipId, Organization, OrganizationId, TenancyError,
    TenantUserId,
};
use sitolo_persistence::{ProvisionedOrganization, TenancyDatabase, TenancyStores};
use sitolo_tenancy::{EffectiveScope, ScopeError, resolve_effective_scope};

/// The tenancy service (Phase 4 use cases).
pub struct TenancyService {
    db: Arc<TenancyDatabase>,
}

impl TenancyService {
    pub fn new(db: Arc<TenancyDatabase>) -> Self {
        TenancyService { db }
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

    /// Invites a user into an organization (section 8). The invitation
    /// workflow (delivery, acceptance tokens) arrives in PR-005; this records
    /// the membership in state `Invited` and enforces one non-terminal
    /// membership per (organization, user).
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn org_id(value: &str) -> OrganizationId {
        OrganizationId::new(value).unwrap()
    }

    fn user(value: &str) -> TenantUserId {
        TenantUserId::new(value).unwrap()
    }

    fn service() -> TenancyService {
        TenancyService::new(Arc::new(TenancyDatabase::new()))
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
}
