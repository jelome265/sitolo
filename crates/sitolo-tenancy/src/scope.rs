//! Effective tenant-scope resolution.
//!
//! Phase 4 specification, sections 0, 5, 15, 16, 54 and 55. Resolves the
//! server-authoritative scope under which a domain operation executes from
//! already-loaded records:
//!
//! ```text
//! AUTHENTICATED PRINCIPAL (Phase 3 SecurityContext)
//!         |
//!         v
//! ACTIVE MEMBERSHIP + OPERATING ORGANIZATION [+ OPERATING BRANCH]
//!         |
//!         v
//! EFFECTIVE SCOPE -> APPLICATION COMMAND (Phase 6 authorization consumes this)
//! ```
//!
//! The central rule: a client-supplied tenant identifier is a selector, never
//! proof of authority. Requested and trusted identifiers are distinct types
//! (section 55) so authority confusion cannot be expressed accidentally.

use sitolo_domain::tenancy::{
    Branch, BranchId, Membership, MembershipId, Organization, OrganizationId,
};
use thiserror::Error;

/// A client-supplied organization selector (section 55). Untrusted input:
/// it names what the caller wants, never what the caller may access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedOrganizationId(pub OrganizationId);

/// A client-supplied branch selector (section 55). Untrusted input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedBranchId(pub BranchId);

/// A server-resolved organization binding: the requested selector matched the
/// caller's active membership organization. Only this type enters effective
/// scope construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedOrganizationId(pub OrganizationId);

/// Scope resolution failures. Every variant denies the operation; public
/// transport mapping stays generic (a later Phase 4 PR owns the error
/// registry) so tenant existence is never enumerable.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScopeError {
    #[error("no authority for the requested organization")]
    OrganizationDenied,
    #[error("no authority for the requested branch")]
    BranchDenied,
    #[error("membership lacks ordinary business authority")]
    MembershipNotActive,
    #[error("organization is not operating")]
    OrganizationNotActive,
    #[error("branch is not operating")]
    BranchNotActive,
}

/// Binds a requested organization selector to the caller's membership
/// (sections 5, 55). Succeeds only when the selector names the membership's
/// own organization — a Tenant A member requesting Tenant B is denied without
/// disclosing whether Tenant B exists.
pub fn bind_organization(
    requested: &RequestedOrganizationId,
    membership: &Membership,
) -> Result<TrustedOrganizationId, ScopeError> {
    if requested.0 != membership.organization_id {
        return Err(ScopeError::OrganizationDenied);
    }
    Ok(TrustedOrganizationId(requested.0.clone()))
}

/// The server-authoritative scope for one operation (section 54, reduced to
/// the PR-001 vocabulary: identity, membership, organization, branch).
/// Immutable after construction; downstream handlers cannot widen it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveScope {
    pub organization_id: OrganizationId,
    pub membership_id: MembershipId,
    pub branch_id: Option<BranchId>,
    pub organization_version: u64,
    pub membership_version: u64,
}

/// Resolves the effective scope from server-loaded records (sections 5.1,
/// 15, 16).
///
/// Required facts, all server-side:
/// - `membership` is the caller's membership, loaded for the session user;
/// - `organization` is loaded by the trusted organization binding, never by
///   a raw client identifier;
/// - `branch`, when the operation is branch-scoped, is loaded by the
///   requested branch selector and must belong to the trusted organization.
///
/// Denies when any fact is missing or non-operating: unknown membership
/// state, suspended/revoked membership, non-operating organization or branch,
/// branch owned by another organization, or a device/branch mismatch (branch
/// binding arrives in a later Phase 4 PR).
pub fn resolve_effective_scope(
    membership: &Membership,
    organization: &Organization,
    branch: Option<&Branch>,
) -> Result<EffectiveScope, ScopeError> {
    // Membership must belong to the resolved organization. A caller cannot
    // combine Membership(U, A) with Organization B (section 5.1).
    if membership.organization_id != organization.id {
        return Err(ScopeError::OrganizationDenied);
    }
    if !organization.can_operate() {
        return Err(ScopeError::OrganizationNotActive);
    }
    if !membership.has_authority() {
        return Err(ScopeError::MembershipNotActive);
    }
    let branch_id = match branch {
        None => None,
        Some(branch) => {
            // Branch A authority never implies Branch B authority, even
            // within one organization (sections 14, 15).
            if branch.organization_id != organization.id {
                return Err(ScopeError::BranchDenied);
            }
            if !branch.can_operate() {
                return Err(ScopeError::BranchNotActive);
            }
            Some(branch.id.clone())
        }
    };
    Ok(EffectiveScope {
        organization_id: organization.id.clone(),
        membership_id: membership.id.clone(),
        branch_id,
        organization_version: organization.state_version,
        membership_version: membership.state_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_domain::tenancy::{
        BranchId, MembershipId, MembershipState, OrganizationId, TenantUserId,
    };

    fn org(id: &str) -> Organization {
        let mut organization =
            Organization::provision(OrganizationId::new(id).unwrap(), "Merchant").unwrap();
        organization.activate().unwrap();
        organization
    }

    fn branch(id: &str, org: &str) -> Branch {
        let mut branch = Branch::provision(
            BranchId::new(id).unwrap(),
            OrganizationId::new(org).unwrap(),
            "Branch",
        )
        .unwrap();
        branch.activate().unwrap();
        branch
    }

    fn membership(id: &str, org: &str, user: &str) -> Membership {
        let mut membership = Membership::invite(
            MembershipId::new(id).unwrap(),
            OrganizationId::new(org).unwrap(),
            TenantUserId::new(user).unwrap(),
        );
        membership.mark_pending().unwrap();
        membership.activate().unwrap();
        membership
    }

    #[test]
    fn org_wide_scope_resolves_for_active_membership() {
        let membership = membership("m1", "org-a", "u1");
        let organization = org("org-a");
        let scope = resolve_effective_scope(&membership, &organization, None).unwrap();
        assert_eq!(scope.organization_id.as_str(), "org-a");
        assert_eq!(scope.membership_id.as_str(), "m1");
        assert_eq!(scope.branch_id, None);
    }

    #[test]
    fn branch_scoped_operation_resolves_within_organization() {
        let membership = membership("m1", "org-a", "u1");
        let organization = org("org-a");
        let branch = branch("branch-a1", "org-a");
        let scope = resolve_effective_scope(&membership, &organization, Some(&branch)).unwrap();
        assert_eq!(
            scope.branch_id.as_ref().map(BranchId::as_str),
            Some("branch-a1")
        );
    }

    #[test]
    fn cross_tenant_access_is_denied() {
        // Membership(U, A) = ACTIVE, object owned by B: READ/WRITE denied
        // (section 5.1 negative invariant).
        let membership = membership("m1", "org-a", "u1");
        let other = org("org-b");
        assert_eq!(
            resolve_effective_scope(&membership, &other, None),
            Err(ScopeError::OrganizationDenied)
        );
        // Binding a foreign requested selector is denied without disclosure.
        let requested = RequestedOrganizationId(OrganizationId::new("org-b").unwrap());
        assert_eq!(
            bind_organization(&requested, &membership),
            Err(ScopeError::OrganizationDenied)
        );
        let own = RequestedOrganizationId(OrganizationId::new("org-a").unwrap());
        assert!(bind_organization(&own, &membership).is_ok());
    }

    #[test]
    fn cross_branch_access_is_denied() {
        // Branch B1 (org A) authority never implies Branch B2 (org B).
        let membership = membership("m1", "org-a", "u1");
        let organization = org("org-a");
        let foreign = branch("branch-b1", "org-b");
        assert_eq!(
            resolve_effective_scope(&membership, &organization, Some(&foreign)),
            Err(ScopeError::BranchDenied)
        );
    }

    #[test]
    fn non_operating_records_deny() {
        let membership = membership("m1", "org-a", "u1");
        let organization = org("org-a");
        let branch = branch("branch-a1", "org-a");

        let mut suspended_org = organization.clone();
        suspended_org.suspend().unwrap();
        assert_eq!(
            resolve_effective_scope(&membership, &suspended_org, None),
            Err(ScopeError::OrganizationNotActive)
        );

        let mut suspended_member = membership.clone();
        suspended_member.suspend().unwrap();
        assert_eq!(
            resolve_effective_scope(&suspended_member, &organization, None),
            Err(ScopeError::MembershipNotActive)
        );
        let mut revoked_member = membership.clone();
        revoked_member.revoke().unwrap();
        assert_eq!(
            resolve_effective_scope(&revoked_member, &organization, None),
            Err(ScopeError::MembershipNotActive)
        );
        // A revoked membership cannot launder authority through a live branch.
        assert_eq!(
            resolve_effective_scope(&revoked_member, &organization, Some(&branch)),
            Err(ScopeError::MembershipNotActive)
        );

        let mut suspended_branch = branch.clone();
        suspended_branch.suspend().unwrap();
        assert_eq!(
            resolve_effective_scope(&membership, &organization, Some(&suspended_branch)),
            Err(ScopeError::BranchNotActive)
        );

        // Expired invitations carry no authority.
        let mut expired = Membership::invite(
            MembershipId::new("m9").unwrap(),
            OrganizationId::new("org-a").unwrap(),
            TenantUserId::new("u9").unwrap(),
        );
        expired.mark_pending().unwrap();
        expired.expire().unwrap();
        assert_eq!(expired.state, MembershipState::Expired);
        assert_eq!(
            resolve_effective_scope(&expired, &organization, None),
            Err(ScopeError::MembershipNotActive)
        );
    }

    #[test]
    fn closed_branch_cannot_authorize() {
        let membership = membership("m1", "org-a", "u1");
        let organization = org("org-a");
        let mut branch = branch("branch-a1", "org-a");
        branch.begin_close().unwrap();
        assert_eq!(
            resolve_effective_scope(&membership, &organization, Some(&branch)),
            Err(ScopeError::BranchNotActive)
        );
        branch.close().unwrap();
        assert_eq!(
            resolve_effective_scope(&membership, &organization, Some(&branch)),
            Err(ScopeError::BranchNotActive)
        );
    }

    #[test]
    fn scope_versions_track_authority_state() {
        let membership = membership("m1", "org-a", "u1");
        let organization = org("org-a");
        let scope = resolve_effective_scope(&membership, &organization, None).unwrap();
        assert_eq!(scope.organization_version, organization.state_version);
        assert_eq!(scope.membership_version, membership.state_version);
    }
}
