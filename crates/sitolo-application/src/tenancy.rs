//! Tenancy and IAM application service.

use sitolo_auth::id::{Assurance, DeviceId, SecurityVersion, SessionId, UserId};
use sitolo_authz::{
    BranchId, EffectiveIamContext, InvitationId, MembershipId, OrganizationId, OwnershipTransferId,
    Permission, ROLE_OWNER, RoleId, Scope, role_permissions,
};
use sitolo_persistence::TenancyStores;
use sitolo_tenancy::{
    Branch, BranchStatus, Invitation, Membership, MembershipStatus, Organization,
    OrganizationStatus, OwnershipTransferRequest, TenancyError,
};
use std::sync::atomic::{AtomicU64, Ordering};

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Tenancy and IAM application service (§53).
pub struct TenancyService<T: TenancyStores> {
    store: T,
}

impl<T: TenancyStores> TenancyService<T> {
    pub fn new(store: T) -> Self {
        Self { store }
    }

    fn random_hex(len: usize) -> String {
        let count = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut out = vec![0u8; len];
        let bytes = count.to_le_bytes();
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = bytes[i % bytes.len()].wrapping_add((i as u8).wrapping_add(42));
        }
        sitolo_auth::hex(&out)
    }

    /// Provision a new Organization with default branch and owner membership (§7).
    pub async fn create_organization(
        &self,
        name: impl Into<String>,
        owner_user_id: UserId,
        now_epoch_secs: u64,
    ) -> Result<(Organization, Branch, Membership), TenancyError> {
        let org_id = OrganizationId::new(format!("org-{}", Self::random_hex(8)))
            .map_err(|_| TenancyError::InvalidIdentifier)?;
        let mut org = Organization::create(org_id.clone(), name, now_epoch_secs)?;
        org.activate(now_epoch_secs)?;
        let created_org = self.store.create_organization(org).await?;

        // Default Branch
        let branch_id = BranchId::new(format!("br-{}", Self::random_hex(8)))
            .map_err(|_| TenancyError::InvalidIdentifier)?;
        let branch = Branch::create(
            branch_id,
            org_id.clone(),
            "Main Branch",
            true,
            now_epoch_secs,
        )?;
        let created_branch = self.store.create_branch(branch).await?;

        // Owner Membership
        let mem_id = MembershipId::new(format!("mem-{}", Self::random_hex(8)))
            .map_err(|_| TenancyError::InvalidIdentifier)?;
        let owner_role = RoleId::new(ROLE_OWNER).map_err(|_| TenancyError::InvalidIdentifier)?;
        let membership = Membership::create_active(
            mem_id,
            org_id,
            owner_user_id,
            vec![owner_role],
            vec![Scope::Organization],
            now_epoch_secs,
        );
        let created_mem = self.store.create_membership(membership).await?;

        Ok((created_org, created_branch, created_mem))
    }

    pub async fn get_organization(
        &self,
        id: &OrganizationId,
        actor_ctx: &EffectiveIamContext,
    ) -> Result<Organization, TenancyError> {
        if &actor_ctx.organization_id != id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant organization access denied",
            });
        }
        if !actor_ctx.is_authorized(Permission::OrgView, &Scope::Organization) {
            return Err(TenancyError::Unauthorized {
                reason: "missing OrgView permission",
            });
        }
        self.store
            .get_organization(id)
            .await?
            .ok_or_else(|| TenancyError::OrganizationNotFound { id: id.to_string() })
    }

    pub async fn suspend_organization(
        &self,
        id: &OrganizationId,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
    ) -> Result<Organization, TenancyError> {
        if &actor_ctx.organization_id != id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant organization access denied",
            });
        }
        if !actor_ctx.is_authorized(Permission::OrgSuspend, &Scope::Organization) {
            return Err(TenancyError::Unauthorized {
                reason: "missing OrgSuspend permission",
            });
        }
        self.store
            .update_organization_status(id, OrganizationStatus::Suspended, now_epoch_secs)
            .await
    }

    pub async fn close_organization(
        &self,
        id: &OrganizationId,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
    ) -> Result<Organization, TenancyError> {
        if &actor_ctx.organization_id != id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant organization access denied",
            });
        }
        if !actor_ctx.is_authorized(Permission::OrgClose, &Scope::Organization) {
            return Err(TenancyError::Unauthorized {
                reason: "missing OrgClose permission",
            });
        }
        self.store
            .update_organization_status(id, OrganizationStatus::Closed, now_epoch_secs)
            .await
    }

    pub async fn create_branch(
        &self,
        org_id: &OrganizationId,
        name: impl Into<String>,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
    ) -> Result<Branch, TenancyError> {
        if &actor_ctx.organization_id != org_id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant branch creation denied",
            });
        }
        if !actor_ctx.is_authorized(Permission::BranchCreate, &Scope::Organization) {
            return Err(TenancyError::Unauthorized {
                reason: "missing BranchCreate permission",
            });
        }
        let branch_id = BranchId::new(format!("br-{}", Self::random_hex(8)))
            .map_err(|_| TenancyError::InvalidIdentifier)?;
        let branch = Branch::create(branch_id, org_id.clone(), name, false, now_epoch_secs)?;
        self.store.create_branch(branch).await
    }

    pub async fn close_branch(
        &self,
        org_id: &OrganizationId,
        branch_id: &BranchId,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
    ) -> Result<Branch, TenancyError> {
        if &actor_ctx.organization_id != org_id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant branch operation denied",
            });
        }
        if !actor_ctx.is_authorized(Permission::BranchClose, &Scope::Branch(branch_id.clone())) {
            return Err(TenancyError::Unauthorized {
                reason: "missing BranchClose permission",
            });
        }
        self.store
            .update_branch_status(org_id, branch_id, BranchStatus::Closed, now_epoch_secs)
            .await
    }

    /// Issue an invitation with role-escalation validation (§9, §18).
    #[allow(clippy::too_many_arguments)]
    pub async fn invite_member(
        &self,
        org_id: &OrganizationId,
        target_contact: impl Into<String>,
        proposed_role: RoleId,
        proposed_scope: Scope,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
        ttl_secs: u64,
    ) -> Result<(Invitation, String), TenancyError> {
        if &actor_ctx.organization_id != org_id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant invitation denied",
            });
        }
        if !actor_ctx.is_authorized(Permission::MembershipInvite, &Scope::Organization) {
            return Err(TenancyError::Unauthorized {
                reason: "missing MembershipInvite permission",
            });
        }

        // Anti-escalation check (§18.1): Actor cannot grant permissions higher than their own!
        let actor_perms = actor_ctx.effective_permissions();
        let proposed_perms = role_permissions(&proposed_role);
        for perm in proposed_perms {
            if !actor_perms.contains(perm) {
                return Err(TenancyError::RoleAssignmentForbidden);
            }
        }

        let inv_id = InvitationId::new(format!("inv-{}", Self::random_hex(8)))
            .map_err(|_| TenancyError::InvalidIdentifier)?;
        let raw_token = Self::random_hex(32);
        let token_hash = sitolo_auth::hex(&sitolo_auth::hash_raw(&raw_token));

        let invitation = Invitation::create(
            inv_id,
            org_id.clone(),
            actor_ctx.principal_id.clone(),
            target_contact,
            token_hash,
            proposed_role,
            proposed_scope,
            now_epoch_secs + ttl_secs,
            now_epoch_secs,
        )?;

        let created_inv = self.store.create_invitation(invitation).await?;
        Ok((created_inv, raw_token))
    }

    /// Redeem/accept an invitation (§9).
    pub async fn accept_invitation(
        &self,
        invitation_id: &InvitationId,
        user_id: UserId,
        now_epoch_secs: u64,
    ) -> Result<Membership, TenancyError> {
        let accepted_inv = self
            .store
            .accept_invitation(invitation_id, now_epoch_secs)
            .await?;

        let mem_id = MembershipId::new(format!("mem-{}", Self::random_hex(8)))
            .map_err(|_| TenancyError::InvalidIdentifier)?;
        let membership = Membership::create_active(
            mem_id,
            accepted_inv.organization_id,
            user_id,
            vec![accepted_inv.proposed_role],
            vec![accepted_inv.proposed_scope],
            now_epoch_secs,
        );

        self.store.create_membership(membership).await
    }

    pub async fn suspend_membership(
        &self,
        org_id: &OrganizationId,
        membership_id: &MembershipId,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
    ) -> Result<Membership, TenancyError> {
        if &actor_ctx.organization_id != org_id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant membership operation denied",
            });
        }
        if !actor_ctx.is_authorized(Permission::MembershipSuspend, &Scope::Organization) {
            return Err(TenancyError::Unauthorized {
                reason: "missing MembershipSuspend permission",
            });
        }
        self.store
            .update_membership_status(
                org_id,
                membership_id,
                MembershipStatus::Suspended,
                now_epoch_secs,
            )
            .await
    }

    pub async fn revoke_membership(
        &self,
        org_id: &OrganizationId,
        membership_id: &MembershipId,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
    ) -> Result<Membership, TenancyError> {
        if &actor_ctx.organization_id != org_id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant membership operation denied",
            });
        }
        if !actor_ctx.is_authorized(Permission::MembershipRevoke, &Scope::Organization) {
            return Err(TenancyError::Unauthorized {
                reason: "missing MembershipRevoke permission",
            });
        }
        self.store
            .update_membership_status(
                org_id,
                membership_id,
                MembershipStatus::Revoked,
                now_epoch_secs,
            )
            .await
    }

    pub async fn assign_role(
        &self,
        org_id: &OrganizationId,
        membership_id: &MembershipId,
        role: RoleId,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
    ) -> Result<Membership, TenancyError> {
        if &actor_ctx.organization_id != org_id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant role assignment denied",
            });
        }
        if !actor_ctx.is_authorized(Permission::RoleAssign, &Scope::Organization) {
            return Err(TenancyError::Unauthorized {
                reason: "missing RoleAssign permission",
            });
        }

        // Anti-escalation check (§18.1): Cannot grant roles with permissions exceeding actor's own!
        let actor_perms = actor_ctx.effective_permissions();
        for perm in role_permissions(&role) {
            if !actor_perms.contains(perm) {
                return Err(TenancyError::RoleAssignmentForbidden);
            }
        }

        self.store
            .assign_membership_role(org_id, membership_id, role, now_epoch_secs)
            .await
    }

    pub async fn request_ownership_transfer(
        &self,
        org_id: &OrganizationId,
        proposed_owner_user_id: UserId,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
        ttl_secs: u64,
    ) -> Result<OwnershipTransferRequest, TenancyError> {
        if &actor_ctx.organization_id != org_id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant ownership operation denied",
            });
        }
        // Step-up assurance check (§12.2)
        if !actor_ctx.assurance.meets(Assurance::A2) {
            return Err(TenancyError::Unauthorized {
                reason: "step-up assurance A2 required for ownership transfer",
            });
        }

        let transfer_id = OwnershipTransferId::new(format!("transfer-{}", Self::random_hex(8)))
            .map_err(|_| TenancyError::InvalidIdentifier)?;

        let req = OwnershipTransferRequest::create(
            transfer_id,
            org_id.clone(),
            actor_ctx.principal_id.clone(),
            proposed_owner_user_id,
            now_epoch_secs + ttl_secs,
            now_epoch_secs,
        );

        self.store.create_ownership_transfer(req).await
    }

    pub async fn approve_ownership_transfer(
        &self,
        org_id: &OrganizationId,
        transfer_id: &OwnershipTransferId,
        actor_ctx: &EffectiveIamContext,
        now_epoch_secs: u64,
    ) -> Result<OwnershipTransferRequest, TenancyError> {
        if &actor_ctx.organization_id != org_id {
            return Err(TenancyError::Unauthorized {
                reason: "cross-tenant ownership operation denied",
            });
        }
        if !actor_ctx.assurance.meets(Assurance::A2) {
            return Err(TenancyError::Unauthorized {
                reason: "step-up assurance A2 required for ownership transfer approval",
            });
        }

        let approved_req = self
            .store
            .approve_ownership_transfer(transfer_id, now_epoch_secs)
            .await?;

        let target_membership = self
            .store
            .get_membership_by_user(org_id, &approved_req.proposed_owner_user_id)
            .await?
            .ok_or_else(|| TenancyError::MembershipNotFound {
                id: approved_req.proposed_owner_user_id.to_string(),
            })?;

        let owner_role = RoleId::new(ROLE_OWNER).map_err(|_| TenancyError::InvalidIdentifier)?;
        self.store
            .assign_membership_role(org_id, &target_membership.id, owner_role, now_epoch_secs)
            .await?;

        Ok(approved_req)
    }

    /// Construct immutable request-time `EffectiveIamContext` for an active user in an organization (§17).
    pub async fn switch_organization_context(
        &self,
        user_id: UserId,
        target_org_id: OrganizationId,
        session_id: SessionId,
        device_id: Option<DeviceId>,
        assurance: Assurance,
        _user_security_version: SecurityVersion,
    ) -> Result<EffectiveIamContext, TenancyError> {
        let org = self
            .store
            .get_organization(&target_org_id)
            .await?
            .ok_or_else(|| TenancyError::OrganizationNotFound {
                id: target_org_id.to_string(),
            })?;

        if !org.is_active() {
            return Err(TenancyError::OrganizationSuspended {
                id: target_org_id.to_string(),
            });
        }

        let membership = self
            .store
            .get_membership_by_user(&target_org_id, &user_id)
            .await?
            .ok_or_else(|| TenancyError::MembershipInactive {
                id: user_id.to_string(),
            })?;

        if !membership.is_active() {
            return Err(TenancyError::MembershipInactive {
                id: membership.id.to_string(),
            });
        }

        Ok(EffectiveIamContext::new(
            user_id,
            session_id,
            device_id,
            target_org_id,
            membership.id,
            membership.roles,
            membership.scope_grants,
            assurance,
            1, // policy version
            org.security_version,
            membership.security_version,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_authz::{ROLE_CASHIER, ROLE_ORG_ADMIN, ROLE_OWNER};
    use sitolo_persistence::TenancyDatabase;

    #[tokio::test]
    async fn tenancy_service_happy_path_and_anti_escalation() {
        let store = TenancyDatabase::new();
        let service = TenancyService::new(store);

        let user_owner = UserId::new("user-owner").unwrap();
        let (org, _branch, _mem) = service
            .create_organization("Sitolo SME", user_owner.clone(), 1000)
            .await
            .unwrap();

        let ctx_owner = service
            .switch_organization_context(
                user_owner.clone(),
                org.id.clone(),
                SessionId::new("sess-1").unwrap(),
                None,
                Assurance::A2,
                SecurityVersion(1),
            )
            .await
            .unwrap();

        // Create new branch
        let new_branch = service
            .create_branch(&org.id, "Branch Lilongwe", &ctx_owner, 1010)
            .await
            .unwrap();

        assert_eq!(new_branch.name, "Branch Lilongwe");

        // Invite Cashier
        let cashier_role = RoleId::new(ROLE_CASHIER).unwrap();
        let (inv, _token) = service
            .invite_member(
                &org.id,
                "cashier@sitolo.mw",
                cashier_role,
                Scope::Branch(new_branch.id.clone()),
                &ctx_owner,
                1020,
                3600,
            )
            .await
            .unwrap();

        // Accept invitation as cashier
        let user_cashier = UserId::new("user-cashier").unwrap();
        let cashier_mem = service
            .accept_invitation(&inv.id, user_cashier.clone(), 1030)
            .await
            .unwrap();

        let ctx_cashier = service
            .switch_organization_context(
                user_cashier,
                org.id.clone(),
                SessionId::new("sess-2").unwrap(),
                None,
                Assurance::A1,
                SecurityVersion(1),
            )
            .await
            .unwrap();

        // Check 1: Cashier lacks MembershipInvite permission -> returns Unauthorized
        let owner_role = RoleId::new(ROLE_OWNER).unwrap();
        let err = service
            .invite_member(
                &org.id,
                "attacker@sitolo.mw",
                owner_role.clone(),
                Scope::Organization,
                &ctx_cashier,
                1040,
                3600,
            )
            .await
            .unwrap_err();

        assert!(matches!(err, TenancyError::Unauthorized { .. }));

        // Invite an OrgAdmin
        let admin_role = RoleId::new(ROLE_ORG_ADMIN).unwrap();
        let (inv_admin, _) = service
            .invite_member(
                &org.id,
                "admin@sitolo.mw",
                admin_role,
                Scope::Organization,
                &ctx_owner,
                1040,
                3600,
            )
            .await
            .unwrap();

        let user_admin = UserId::new("user-admin").unwrap();
        let _admin_mem = service
            .accept_invitation(&inv_admin.id, user_admin.clone(), 1050)
            .await
            .unwrap();

        let ctx_admin = service
            .switch_organization_context(
                user_admin,
                org.id.clone(),
                SessionId::new("sess-3").unwrap(),
                None,
                Assurance::A2,
                SecurityVersion(1),
            )
            .await
            .unwrap();

        // Check 2: OrgAdmin HAS MembershipInvite permission, but tries to invite an OWNER -> RoleAssignmentForbidden!
        let err2 = service
            .invite_member(
                &org.id,
                "superowner@sitolo.mw",
                owner_role.clone(),
                Scope::Organization,
                &ctx_admin,
                1060,
                3600,
            )
            .await
            .unwrap_err();

        assert_eq!(err2, TenancyError::RoleAssignmentForbidden);

        // Check 3: OrgAdmin HAS RoleAssign permission, but tries to assign OWNER role -> RoleAssignmentForbidden!
        let err3 = service
            .assign_role(&org.id, &cashier_mem.id, owner_role, &ctx_admin, 1070)
            .await
            .unwrap_err();

        assert_eq!(err3, TenancyError::RoleAssignmentForbidden);
    }
}
