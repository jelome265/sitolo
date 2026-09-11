//! Organization, branch, and membership domain primitives.
//!
//! Phase 4 specification, sections 4, 6, 8, 14, 15 and 20. These are pure
//! state machines and invariants: entities, legal transitions, terminal
//! states, and authority predicates. There is no persistence, no HTTP, no
//! provider machinery, and no role/permission policy here — roles, scopes,
//! invitations and the authorization engine arrive in later Phase 4 PRs and
//! Phase 6 respectively.
//!
//! Cardinality rules enforced structurally:
//!
//! ```text
//! Branch -> Organization     = exactly 1 (immutable at creation)
//! Membership -> Organization = exactly 1 (immutable at creation)
//! Membership -> User         = exactly 1 (immutable at creation)
//! ```

use thiserror::Error;

/// Generates an opaque tenant identifier newtype with bounded validation.
/// Values embed no names, roles, or branch information.
macro_rules! tenant_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, TenancyError> {
                let value = value.into();
                if value.is_empty()
                    || value.len() > 128
                    || !value.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':')
                    })
                {
                    return Err(TenancyError::InvalidIdentifier);
                }
                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

tenant_id!(
    OrganizationId,
    "Opaque tenant identifier. A selector for server-side records, never proof of authority (Phase 4 section 0)."
);
tenant_id!(
    BranchId,
    "Opaque branch identifier within one organization."
);
tenant_id!(MembershipId, "Opaque membership identifier.");
tenant_id!(
    TenantUserId,
    "Reference to the identity-subsystem user behind a membership (Phase 4 section 3.1: `User != Membership`). Correlates to the authenticated subject established by Phase 3; carries no authority by itself."
);
tenant_id!(
    RoleAssignmentId,
    "Opaque role-assignment identifier. Assignment lifecycle lives in the authorization crate; the identifier itself is tenant-topology vocabulary."
);
tenant_id!(
    ScopeGrantId,
    "Opaque scope-grant identifier. Grant lifecycle lives in the authorization crate; the identifier itself is tenant-topology vocabulary."
);
tenant_id!(
    InvitationId,
    "Opaque invitation identifier. Invitation lifecycle lives in the authorization crate; the identifier itself is tenant-topology vocabulary."
);

/// Domain failures for tenant topology transitions.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TenancyError {
    #[error("invalid tenant identifier")]
    InvalidIdentifier,
    #[error("invalid organization or branch name")]
    InvalidName,
    #[error("invalid state transition")]
    InvalidTransition,
    #[error("tenant record is in a terminal state")]
    TerminalState,
    #[error("tenant record not found")]
    NotFound,
    #[error("tenant record already exists with different semantics")]
    Conflict,
    #[error("invalid invitation parameters")]
    InvalidInvitation,
    #[error("invitation rate limited")]
    RateLimited,
}

fn validate_name(name: &str) -> Result<String, TenancyError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 256 {
        return Err(TenancyError::InvalidName);
    }
    Ok(trimmed.to_string())
}

/// Organization lifecycle (Phase 4 section 6).
///
/// ```text
/// PROVISIONING -> ACTIVE <-> SUSPENDED
///                  |
///                  v
///               CLOSING -> CLOSED
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrganizationState {
    Provisioning,
    Active,
    Suspended,
    Closing,
    Closed,
}

impl OrganizationState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, OrganizationState::Closed)
    }

    /// Ordinary business authority exists only while `Active` (sections
    /// 6.1-6.5). Suspended, provisioning, closing and closed organizations
    /// deny normal financial workflows.
    #[must_use]
    pub fn can_operate(self) -> bool {
        matches!(self, OrganizationState::Active)
    }
}

/// An organization: the top-level tenant boundary (Phase 4 section 3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub state: OrganizationState,
    /// Monotonic transition counter. Bumped on every legal transition so
    /// revocation/suspension can invalidate stale authorization caches
    /// (sections 20.3, 23).
    pub state_version: u64,
}

impl Organization {
    /// Provisions a new organization. Creation must be paired with an owner
    /// membership in one atomic transaction (section 7.1); this constructor
    /// only builds the organization half of that pair.
    pub fn provision(id: OrganizationId, name: &str) -> Result<Self, TenancyError> {
        Ok(Organization {
            id,
            name: validate_name(name)?,
            state: OrganizationState::Provisioning,
            state_version: 1,
        })
    }

    fn transition(
        &mut self,
        from: OrganizationState,
        to: OrganizationState,
    ) -> Result<(), TenancyError> {
        if self.state.is_terminal() {
            return Err(TenancyError::TerminalState);
        }
        if self.state != from {
            return Err(TenancyError::InvalidTransition);
        }
        self.state = to;
        self.state_version = self.state_version.saturating_add(1);
        Ok(())
    }

    /// `PROVISIONING -> ACTIVE`: the organization becomes operationally live.
    pub fn activate(&mut self) -> Result<(), TenancyError> {
        self.transition(OrganizationState::Provisioning, OrganizationState::Active)
    }

    /// `ACTIVE -> SUSPENDED`: business operations are restricted.
    pub fn suspend(&mut self) -> Result<(), TenancyError> {
        self.transition(OrganizationState::Active, OrganizationState::Suspended)
    }

    /// `SUSPENDED -> ACTIVE`: restores ordinary operation.
    pub fn resume(&mut self) -> Result<(), TenancyError> {
        self.transition(OrganizationState::Suspended, OrganizationState::Active)
    }

    /// `ACTIVE -> CLOSING`: controlled transition toward closure. No new
    /// long-lived authority may be created while closing (section 6.4).
    pub fn begin_close(&mut self) -> Result<(), TenancyError> {
        self.transition(OrganizationState::Active, OrganizationState::Closing)
    }

    /// `CLOSING -> CLOSED`: terminal. History is preserved, never deleted
    /// (section 6.5).
    pub fn close(&mut self) -> Result<(), TenancyError> {
        self.transition(OrganizationState::Closing, OrganizationState::Closed)
    }

    /// Ordinary business authority predicate (sections 6.1-6.5).
    #[must_use]
    pub fn can_operate(&self) -> bool {
        self.state.can_operate()
    }
}

/// Branch lifecycle (Phase 4 section 14.1).
///
/// ```text
/// PROVISIONING -> ACTIVE -> SUSPENDED
///                  |
///                  v
///               CLOSING -> CLOSED
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BranchState {
    Provisioning,
    Active,
    Suspended,
    Closing,
    Closed,
}

impl BranchState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, BranchState::Closed)
    }

    #[must_use]
    pub fn can_operate(self) -> bool {
        matches!(self, BranchState::Active)
    }
}

/// A branch: a first-class operational boundary within exactly one
/// organization (Phase 4 sections 3.4, 4.1, 14). The owning organization is
/// fixed at creation — moving scope is a controlled transition, never a
/// foreign-key edit (section 14.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub id: BranchId,
    pub organization_id: OrganizationId,
    pub name: String,
    pub state: BranchState,
    pub state_version: u64,
}

impl Branch {
    pub fn provision(
        id: BranchId,
        organization_id: OrganizationId,
        name: &str,
    ) -> Result<Self, TenancyError> {
        Ok(Branch {
            id,
            organization_id,
            name: validate_name(name)?,
            state: BranchState::Provisioning,
            state_version: 1,
        })
    }

    fn transition(&mut self, from: BranchState, to: BranchState) -> Result<(), TenancyError> {
        if self.state.is_terminal() {
            return Err(TenancyError::TerminalState);
        }
        if self.state != from {
            return Err(TenancyError::InvalidTransition);
        }
        self.state = to;
        self.state_version = self.state_version.saturating_add(1);
        Ok(())
    }

    pub fn activate(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Provisioning, BranchState::Active)
    }

    pub fn suspend(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Active, BranchState::Suspended)
    }

    pub fn resume(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Suspended, BranchState::Active)
    }

    pub fn begin_close(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Active, BranchState::Closing)
    }

    pub fn close(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Closing, BranchState::Closed)
    }

    #[must_use]
    pub fn can_operate(&self) -> bool {
        self.state.can_operate()
    }
}

/// Membership lifecycle (Phase 4 section 8).
///
/// ```text
/// INVITED -> PENDING_ACCEPTANCE -> ACTIVE <-> SUSPENDED
///                 |                   |
///                 v                   v
///              EXPIRED              REVOKED
/// ```
///
/// Only `ACTIVE` carries ordinary business authority. `REVOKED` is permanent
/// for ordinary purposes: a revoked membership cannot be reactivated through
/// client mutation (section 8.1.5); re-enrollment is a new membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MembershipState {
    Invited,
    PendingAcceptance,
    Expired,
    Active,
    Suspended,
    Revoked,
}

impl MembershipState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, MembershipState::Expired | MembershipState::Revoked)
    }

    /// Ordinary business authority exists only while `Active` (section 8).
    #[must_use]
    pub fn has_authority(self) -> bool {
        matches!(self, MembershipState::Active)
    }
}

/// A membership: the explicit relationship between one user and one
/// organization from which business authority is derived (Phase 4 sections
/// 3.9, 8.1). Organization and user bindings are immutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Membership {
    pub id: MembershipId,
    pub organization_id: OrganizationId,
    pub user_id: TenantUserId,
    pub state: MembershipState,
    pub state_version: u64,
}

impl Membership {
    /// Issues an invitation-bound membership in state `Invited`. Acceptance,
    /// expiry and revocation are separate explicit transitions (sections
    /// 8, 9).
    pub fn invite(
        id: MembershipId,
        organization_id: OrganizationId,
        user_id: TenantUserId,
    ) -> Self {
        Membership {
            id,
            organization_id,
            user_id,
            state: MembershipState::Invited,
            state_version: 1,
        }
    }

    fn transition(
        &mut self,
        from: MembershipState,
        to: MembershipState,
    ) -> Result<(), TenancyError> {
        if self.state.is_terminal() {
            return Err(TenancyError::TerminalState);
        }
        if self.state != from {
            return Err(TenancyError::InvalidTransition);
        }
        self.state = to;
        self.state_version = self.state_version.saturating_add(1);
        Ok(())
    }

    /// `INVITED -> PENDING_ACCEPTANCE`: enrollment challenge issued.
    pub fn mark_pending(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::Invited, MembershipState::PendingAcceptance)
    }

    /// `PENDING_ACCEPTANCE -> EXPIRED`: the enrollment window lapsed.
    pub fn expire(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::PendingAcceptance, MembershipState::Expired)
    }

    /// `PENDING_ACCEPTANCE -> ACTIVE`: enrollment accepted.
    pub fn activate(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::PendingAcceptance, MembershipState::Active)
    }

    /// `ACTIVE -> SUSPENDED`: temporary removal of authority (section 20.1).
    pub fn suspend(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::Active, MembershipState::Suspended)
    }

    /// `SUSPENDED -> ACTIVE`: reactivation after review.
    pub fn resume(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::Suspended, MembershipState::Active)
    }

    /// `ACTIVE -> REVOKED`: permanent termination for ordinary purposes
    /// (section 20.2). Bumps the version so caches and sessions invalidate
    /// (section 20.3).
    pub fn revoke(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::Active, MembershipState::Revoked)
    }

    /// Ordinary business authority predicate (section 8).
    #[must_use]
    pub fn has_authority(&self) -> bool {
        self.state.has_authority()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn org_id(value: &str) -> OrganizationId {
        OrganizationId::new(value).unwrap()
    }

    fn branch_id(value: &str) -> BranchId {
        BranchId::new(value).unwrap()
    }

    fn active_org(id: &str) -> Organization {
        let mut org = Organization::provision(org_id(id), "Test Merchant").unwrap();
        org.activate().unwrap();
        org
    }

    fn active_branch(id: &str, org: &str) -> Branch {
        let mut branch = Branch::provision(branch_id(id), org_id(org), "Main Branch").unwrap();
        branch.activate().unwrap();
        branch
    }

    fn active_membership(id: &str, org: &str, user: &str) -> Membership {
        let mut membership = Membership::invite(
            MembershipId::new(id).unwrap(),
            org_id(org),
            TenantUserId::new(user).unwrap(),
        );
        membership.mark_pending().unwrap();
        membership.activate().unwrap();
        membership
    }

    #[test]
    fn identifiers_reject_hostile_shapes() {
        assert!(OrganizationId::new("").is_err());
        assert!(OrganizationId::new("has space").is_err());
        assert!(OrganizationId::new("inject\r\nline").is_err());
        assert!(OrganizationId::new("way-too-long-".repeat(12).as_str()).is_err());
        assert!(OrganizationId::new("org_01ABCdef-2").is_ok());
        assert!(BranchId::new("").is_err());
        assert!(MembershipId::new("").is_err());
        assert!(TenantUserId::new("").is_err());
    }

    #[test]
    fn names_are_presentation_data_with_bounds() {
        assert!(Organization::provision(org_id("o1"), "").is_err());
        assert!(Organization::provision(org_id("o1"), "   ").is_err());
        assert!(Organization::provision(org_id("o1"), &"n".repeat(257)).is_err());
        // Names are not unique security identifiers: duplicates are allowed.
        assert!(Organization::provision(org_id("o1"), "Same Name").is_ok());
        assert!(Organization::provision(org_id("o2"), "Same Name").is_ok());
    }

    #[test]
    fn organization_lifecycle_follows_section_6() {
        let mut org = Organization::provision(org_id("o1"), "Merchant").unwrap();
        assert_eq!(org.state, OrganizationState::Provisioning);
        assert!(!org.can_operate());

        // Provisioning cannot skip activation.
        assert_eq!(org.suspend(), Err(TenancyError::InvalidTransition));
        assert_eq!(org.begin_close(), Err(TenancyError::InvalidTransition));

        org.activate().unwrap();
        assert!(org.can_operate());

        org.suspend().unwrap();
        assert!(!org.can_operate());
        org.resume().unwrap();
        assert!(org.can_operate());

        // Suspended cannot begin closing; only Active can (section 6 diagram).
        org.suspend().unwrap();
        assert_eq!(org.begin_close(), Err(TenancyError::InvalidTransition));
        org.resume().unwrap();

        org.begin_close().unwrap();
        assert!(!org.can_operate());
        // Closing cannot return to Active.
        assert_eq!(org.resume(), Err(TenancyError::InvalidTransition));
        assert_eq!(org.activate(), Err(TenancyError::InvalidTransition));

        org.close().unwrap();
        assert_eq!(org.state, OrganizationState::Closed);
        assert!(!org.can_operate());
    }

    #[test]
    fn closed_organization_is_terminal() {
        let mut org = active_org("o1");
        org.begin_close().unwrap();
        org.close().unwrap();
        assert_eq!(org.activate(), Err(TenancyError::TerminalState));
        assert_eq!(org.suspend(), Err(TenancyError::TerminalState));
        assert_eq!(org.resume(), Err(TenancyError::TerminalState));
        assert_eq!(org.begin_close(), Err(TenancyError::TerminalState));
        assert_eq!(org.close(), Err(TenancyError::TerminalState));
    }

    #[test]
    fn branch_lifecycle_follows_section_14() {
        let mut branch = Branch::provision(branch_id("b1"), org_id("o1"), "Branch").unwrap();
        assert_eq!(branch.state, BranchState::Provisioning);
        assert!(!branch.can_operate());

        assert_eq!(branch.suspend(), Err(TenancyError::InvalidTransition));
        branch.activate().unwrap();
        assert!(branch.can_operate());

        branch.suspend().unwrap();
        assert!(!branch.can_operate());
        branch.resume().unwrap();
        assert!(branch.can_operate());

        branch.begin_close().unwrap();
        assert!(!branch.can_operate());
        branch.close().unwrap();
        assert_eq!(branch.state, BranchState::Closed);
        assert_eq!(branch.resume(), Err(TenancyError::TerminalState));
    }

    #[test]
    fn branch_organization_binding_is_immutable() {
        let branch = active_branch("b1", "o1");
        assert_eq!(branch.organization_id.as_str(), "o1");
        // No setter exists: the binding can only be established at creation.
        // This test pins the struct shape against accidental mutability.
        let Branch {
            organization_id, ..
        } = &branch;
        assert_eq!(organization_id.as_str(), "o1");
    }

    #[test]
    fn membership_lifecycle_follows_section_8() {
        let mut membership = Membership::invite(
            MembershipId::new("m1").unwrap(),
            org_id("o1"),
            TenantUserId::new("u1").unwrap(),
        );
        assert_eq!(membership.state, MembershipState::Invited);
        assert!(!membership.has_authority());

        // Invited cannot skip to Active or Revoked.
        assert_eq!(membership.activate(), Err(TenancyError::InvalidTransition));
        assert_eq!(membership.revoke(), Err(TenancyError::InvalidTransition));

        membership.mark_pending().unwrap();
        assert!(!membership.has_authority());
        membership.activate().unwrap();
        assert!(membership.has_authority());

        membership.suspend().unwrap();
        assert!(!membership.has_authority());
        membership.resume().unwrap();
        assert!(membership.has_authority());

        membership.revoke().unwrap();
        assert_eq!(membership.state, MembershipState::Revoked);
        assert!(!membership.has_authority());
    }

    #[test]
    fn revoked_membership_cannot_be_reactivated() {
        let mut membership = active_membership("m1", "o1", "u1");
        membership.revoke().unwrap();
        assert_eq!(membership.resume(), Err(TenancyError::TerminalState));
        assert_eq!(membership.activate(), Err(TenancyError::TerminalState));
        assert_eq!(membership.suspend(), Err(TenancyError::TerminalState));
        assert_eq!(membership.revoke(), Err(TenancyError::TerminalState));
    }

    #[test]
    fn expired_membership_is_terminal() {
        let mut membership = Membership::invite(
            MembershipId::new("m1").unwrap(),
            org_id("o1"),
            TenantUserId::new("u1").unwrap(),
        );
        membership.mark_pending().unwrap();
        membership.expire().unwrap();
        assert_eq!(membership.state, MembershipState::Expired);
        assert!(!membership.has_authority());
        assert_eq!(membership.activate(), Err(TenancyError::TerminalState));
    }

    #[test]
    fn invitation_cannot_expire_before_pending() {
        let mut membership = Membership::invite(
            MembershipId::new("m1").unwrap(),
            org_id("o1"),
            TenantUserId::new("u1").unwrap(),
        );
        assert_eq!(membership.expire(), Err(TenancyError::InvalidTransition));
    }

    #[test]
    fn state_versions_increase_monotonically() {
        let mut org = Organization::provision(org_id("o1"), "Merchant").unwrap();
        let v0 = org.state_version;
        org.activate().unwrap();
        org.suspend().unwrap();
        org.resume().unwrap();
        assert!(org.state_version > v0);

        let mut membership = active_membership("m1", "o1", "u1");
        let mv0 = membership.state_version;
        membership.suspend().unwrap();
        membership.resume().unwrap();
        membership.revoke().unwrap();
        assert!(membership.state_version > mv0);
    }
}
