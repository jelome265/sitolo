//! Test kit.
//!
//! Owns reusable fixtures and test infrastructure (Phase 1 specification,
//! §5.14). Production crates MUST NOT depend on this crate.
#![forbid(unsafe_code)]

use sitolo_domain::tenancy::{
    Branch, BranchId, Membership, MembershipId, Organization, OrganizationId, TenancyError,
    TenantUserId,
};

/// Builds a validated organization identifier for tests.
///
/// # Errors
/// Returns [`TenancyError::InvalidIdentifier`] when `value` violates the
/// domain identifier contract.
pub fn organization_id(value: &str) -> Result<OrganizationId, TenancyError> {
    OrganizationId::new(value)
}

/// Builds a validated branch identifier for tests.
///
/// # Errors
/// Returns [`TenancyError::InvalidIdentifier`] when `value` violates the
/// domain identifier contract.
pub fn branch_id(value: &str) -> Result<BranchId, TenancyError> {
    BranchId::new(value)
}

/// Builds a validated membership identifier for tests.
///
/// # Errors
/// Returns [`TenancyError::InvalidIdentifier`] when `value` violates the
/// domain identifier contract.
pub fn membership_id(value: &str) -> Result<MembershipId, TenancyError> {
    MembershipId::new(value)
}

/// Builds a validated tenant-user identifier for tests.
///
/// # Errors
/// Returns [`TenancyError::InvalidIdentifier`] when `value` violates the
/// domain identifier contract.
pub fn tenant_user_id(value: &str) -> Result<TenantUserId, TenancyError> {
    TenantUserId::new(value)
}

/// Builds an active organization fixture through its legal domain lifecycle.
///
/// # Errors
/// Returns a domain error when the identifier or lifecycle transition is
/// invalid.
pub fn active_organization(id: &str) -> Result<Organization, TenancyError> {
    let mut organization = Organization::provision(organization_id(id)?, "Test merchant")?;
    organization.activate()?;
    Ok(organization)
}

/// Builds an active branch fixture belonging to `organization`.
///
/// # Errors
/// Returns a domain error when either identifier, name, or lifecycle
/// transition is invalid.
pub fn active_branch(id: &str, organization: &str) -> Result<Branch, TenancyError> {
    let mut branch = Branch::provision(
        branch_id(id)?,
        organization_id(organization)?,
        "Test branch",
    )?;
    branch.activate()?;
    Ok(branch)
}

/// Builds an active membership fixture through invitation and activation.
///
/// # Errors
/// Returns a domain error when an identifier or lifecycle transition is
/// invalid.
pub fn active_membership(
    id: &str,
    organization: &str,
    user: &str,
) -> Result<Membership, TenancyError> {
    let mut membership = Membership::invite(
        membership_id(id)?,
        organization_id(organization)?,
        tenant_user_id(user)?,
    );
    membership.mark_pending()?;
    membership.activate()?;
    Ok(membership)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_domain::tenancy::MembershipState;

    #[test]
    fn active_tenancy_builders_follow_domain_transitions() -> Result<(), TenancyError> {
        let organization = active_organization("org-test")?;
        let branch = active_branch("branch-test", organization.id.as_str())?;
        let membership = active_membership("member-test", organization.id.as_str(), "user-test")?;

        assert!(organization.can_operate());
        assert!(branch.can_operate());
        assert_eq!(membership.state, MembershipState::Active);
        Ok(())
    }

    #[test]
    fn identifiers_preserve_domain_validation() {
        assert!(organization_id("invalid id").is_err());
        assert!(branch_id("").is_err());
        assert!(membership_id("member:valid").is_ok());
        assert!(tenant_user_id("user/invalid").is_err());
    }
}
