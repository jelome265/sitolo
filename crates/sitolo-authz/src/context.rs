//! Immutable request-time IAM evaluation context.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use sitolo_auth::id::{Assurance, DeviceId, SecurityVersion, SessionId, UserId};

use crate::id::{MembershipId, OrganizationId, RoleId};
use crate::permission::{Permission, role_permissions};
use crate::scope::Scope;

/// Server-derived, immutable request-time IAM context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveIamContext {
    pub principal_id: UserId,
    pub session_id: SessionId,
    pub device_id: Option<DeviceId>,
    pub organization_id: OrganizationId,
    pub membership_id: MembershipId,
    pub roles: Vec<RoleId>,
    pub scope_grants: Vec<Scope>,
    pub assurance: Assurance,
    pub policy_version: u32,
    pub org_security_version: SecurityVersion,
    pub membership_security_version: SecurityVersion,
}

impl EffectiveIamContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        principal_id: UserId,
        session_id: SessionId,
        device_id: Option<DeviceId>,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        roles: Vec<RoleId>,
        scope_grants: Vec<Scope>,
        assurance: Assurance,
        policy_version: u32,
        org_security_version: SecurityVersion,
        membership_security_version: SecurityVersion,
    ) -> Self {
        Self {
            principal_id,
            session_id,
            device_id,
            organization_id,
            membership_id,
            roles,
            scope_grants,
            assurance,
            policy_version,
            org_security_version,
            membership_security_version,
        }
    }

    /// Set of effective atomic permissions collected from assigned roles.
    pub fn effective_permissions(&self) -> HashSet<Permission> {
        let mut perms = HashSet::new();
        for role in &self.roles {
            for perm in role_permissions(role) {
                perms.insert(*perm);
            }
        }
        perms
    }

    /// Evaluates if the context has the required permission for the specified target scope.
    pub fn is_authorized(&self, action: Permission, target_scope: &Scope) -> bool {
        let perms = self.effective_permissions();
        if !perms.contains(&action) {
            return false;
        }

        // Check if any granted scope covers the target scope
        if self.scope_grants.is_empty() {
            // Default to Organization scope if grants are unconstrained
            true
        } else {
            self.scope_grants
                .iter()
                .any(|grant| grant.covers(target_scope))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::BranchId;
    use crate::permission::{ROLE_CASHIER, ROLE_OWNER};

    #[test]
    fn owner_has_full_authority() {
        let ctx = EffectiveIamContext::new(
            UserId::new("u-1").unwrap(),
            SessionId::new("s-1").unwrap(),
            None,
            OrganizationId::new("org-1").unwrap(),
            MembershipId::new("m-1").unwrap(),
            vec![RoleId::new(ROLE_OWNER).unwrap()],
            vec![Scope::Organization],
            Assurance::A2,
            1,
            SecurityVersion(1),
            SecurityVersion(1),
        );

        assert!(ctx.is_authorized(Permission::OrgUpdate, &Scope::Organization));
        assert!(ctx.is_authorized(
            Permission::SaleCreate,
            &Scope::Branch(BranchId::new("b-1").unwrap())
        ));
    }

    #[test]
    fn cashier_authority_is_restricted() {
        let b1 = BranchId::new("b-1").unwrap();
        let ctx = EffectiveIamContext::new(
            UserId::new("u-2").unwrap(),
            SessionId::new("s-2").unwrap(),
            None,
            OrganizationId::new("org-1").unwrap(),
            MembershipId::new("m-2").unwrap(),
            vec![RoleId::new(ROLE_CASHIER).unwrap()],
            vec![Scope::Branch(b1.clone())],
            Assurance::A1,
            1,
            SecurityVersion(1),
            SecurityVersion(1),
        );

        assert!(ctx.is_authorized(Permission::SaleCreate, &Scope::Branch(b1.clone())));
        // Cannot update organization or assign roles
        assert!(!ctx.is_authorized(Permission::OrgUpdate, &Scope::Organization));
        assert!(!ctx.is_authorized(Permission::RoleAssign, &Scope::Organization));
        // Cannot act on another branch
        assert!(!ctx.is_authorized(
            Permission::SaleCreate,
            &Scope::Branch(BranchId::new("b-2").unwrap())
        ));
    }
}
