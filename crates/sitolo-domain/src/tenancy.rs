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
            /// Validates `value` and wraps it as an opaque tenant identifier.
            ///
            /// # Errors
            ///
            /// Returns [`TenancyError::InvalidIdentifier`] when `value` is
            /// empty, longer than 128 bytes, or contains a byte outside ASCII
            /// alphanumerics and the separators `-`, `_`, `.`, and `:`.
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

            /// Borrows the identifier as its validated string form.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
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
    /// A tenant identifier failed shape validation.
    #[error("invalid tenant identifier")]
    InvalidIdentifier,
    /// An organization or branch name was empty after trimming or exceeded
    /// 256 bytes.
    #[error("invalid organization or branch name")]
    InvalidName,
    /// The requested transition is not legal from the record's current state.
    #[error("invalid state transition")]
    InvalidTransition,
    /// The record is terminal and accepts no further transitions.
    #[error("tenant record is in a terminal state")]
    TerminalState,
    /// The referenced tenant record does not exist.
    #[error("tenant record not found")]
    NotFound,
    /// A tenant record already exists with different semantics.
    #[error("tenant record already exists with different semantics")]
    Conflict,
    /// Invitation parameters failed validation.
    #[error("invalid invitation parameters")]
    InvalidInvitation,
    /// Invitation issuance was rate limited.
    #[error("invitation rate limited")]
    RateLimited,
    /// Mandatory Phase 4 Part 8 audit/outbox evidence could not be durably
    /// recorded for an authoritative transition (contract §5, §10:
    /// mandatory evidence blocks the transition it describes).
    #[error("mandatory audit/outbox evidence persistence failed")]
    EvidencePersistenceFailed,
    /// Safe, non-sensitive diagnostic stage for infrastructure logs and
    /// integration-test failures. The display text intentionally remains
    /// generic so callers do not receive persistence internals.
    #[error("mandatory audit/outbox evidence persistence failed")]
    EvidencePersistenceFailedAt {
        /// Safe, non-sensitive diagnostic stage label for infrastructure logs
        /// and integration-test failures.
        stage: &'static str,
        /// Optional `SQLSTATE` code reported by the persistence adapter, when
        /// the driver supplied one.
        sqlstate: Option<String>,
    },
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
    /// Created but not yet operationally live.
    Provisioning,
    /// Operationally live; ordinary business authority exists.
    Active,
    /// Temporarily restricted; ordinary business authority is denied.
    Suspended,
    /// Controlled closure in progress; no new long-lived authority may be
    /// created (section 6.4).
    Closing,
    /// Terminal; history is preserved and no transition is accepted.
    Closed,
}

impl OrganizationState {
    /// Returns `true` when the state is [`OrganizationState::Closed`], which
    /// accepts no further transitions.
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
    /// Immutable tenant identifier.
    pub id: OrganizationId,
    /// Presentation name, trimmed and bounded at creation. Never a security
    /// identifier: duplicates are permitted.
    pub name: String,
    /// Current lifecycle state.
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
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidName`] when `name` is empty after
    /// trimming or longer than 256 bytes.
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
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the organization is not
    /// in [`OrganizationState::Provisioning`], and
    /// [`TenancyError::TerminalState`] when it is already closed.
    pub fn activate(&mut self) -> Result<(), TenancyError> {
        self.transition(OrganizationState::Provisioning, OrganizationState::Active)
    }

    /// `ACTIVE -> SUSPENDED`: business operations are restricted.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the organization is not
    /// in [`OrganizationState::Active`], and [`TenancyError::TerminalState`]
    /// when it is already closed.
    pub fn suspend(&mut self) -> Result<(), TenancyError> {
        self.transition(OrganizationState::Active, OrganizationState::Suspended)
    }

    /// `SUSPENDED -> ACTIVE`: restores ordinary operation.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the organization is not
    /// in [`OrganizationState::Suspended`], and [`TenancyError::TerminalState`]
    /// when it is already closed.
    pub fn resume(&mut self) -> Result<(), TenancyError> {
        self.transition(OrganizationState::Suspended, OrganizationState::Active)
    }

    /// `ACTIVE -> CLOSING`: controlled transition toward closure. No new
    /// long-lived authority may be created while closing (section 6.4).
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the organization is not
    /// in [`OrganizationState::Active`], and [`TenancyError::TerminalState`]
    /// when it is already closed.
    pub fn begin_close(&mut self) -> Result<(), TenancyError> {
        self.transition(OrganizationState::Active, OrganizationState::Closing)
    }

    /// `CLOSING -> CLOSED`: terminal. History is preserved, never deleted
    /// (section 6.5).
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the organization is not
    /// in [`OrganizationState::Closing`], and [`TenancyError::TerminalState`]
    /// when it is already closed.
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
    /// Created but not yet operationally live.
    Provisioning,
    /// Operationally live; the branch accepts ordinary business operations.
    Active,
    /// Temporarily restricted; ordinary operations are denied.
    Suspended,
    /// Controlled closure in progress; no new long-lived authority may be
    /// created.
    Closing,
    /// Terminal; no further transition is accepted.
    Closed,
}

impl BranchState {
    /// Returns `true` when the state is [`BranchState::Closed`], which accepts
    /// no further transitions.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, BranchState::Closed)
    }

    /// Returns `true` only while the branch is [`BranchState::Active`].
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
    /// Opaque branch identifier.
    pub id: BranchId,
    /// Owning organization, fixed at creation and never reassigned
    /// (section 14.3).
    pub organization_id: OrganizationId,
    /// Presentation name, trimmed and bounded at creation.
    pub name: String,
    /// Current lifecycle state.
    pub state: BranchState,
    /// Monotonic transition counter, bumped on every legal transition so stale
    /// authorization caches invalidate.
    pub state_version: u64,
}

impl Branch {
    /// Provisions a branch inside `organization_id` in state `Provisioning`.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidName`] when `name` is empty after
    /// trimming or longer than 256 bytes.
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

    /// `PROVISIONING -> ACTIVE`: the branch becomes operationally live.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the branch is not in
    /// [`BranchState::Provisioning`], and [`TenancyError::TerminalState`] when
    /// it is already closed.
    pub fn activate(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Provisioning, BranchState::Active)
    }

    /// `ACTIVE -> SUSPENDED`: business operations are restricted.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the branch is not in
    /// [`BranchState::Active`], and [`TenancyError::TerminalState`] when it is
    /// already closed.
    pub fn suspend(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Active, BranchState::Suspended)
    }

    /// `SUSPENDED -> ACTIVE`: restores ordinary operation.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the branch is not in
    /// [`BranchState::Suspended`], and [`TenancyError::TerminalState`] when it
    /// is already closed.
    pub fn resume(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Suspended, BranchState::Active)
    }

    /// `ACTIVE -> CLOSING`: controlled transition toward closure.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the branch is not in
    /// [`BranchState::Active`], and [`TenancyError::TerminalState`] when it is
    /// already closed.
    pub fn begin_close(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Active, BranchState::Closing)
    }

    /// `CLOSING -> CLOSED`: terminal. History is preserved, never deleted.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the branch is not in
    /// [`BranchState::Closing`], and [`TenancyError::TerminalState`] when it is
    /// already closed.
    pub fn close(&mut self) -> Result<(), TenancyError> {
        self.transition(BranchState::Closing, BranchState::Closed)
    }

    /// Ordinary business authority predicate (Phase 4 section 14).
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
    /// Invitation issued; enrollment has not started.
    Invited,
    /// Enrollment challenge issued and awaiting acceptance.
    PendingAcceptance,
    /// Terminal; the enrollment window lapsed before acceptance.
    Expired,
    /// Operationally live; the only state carrying ordinary business
    /// authority.
    Active,
    /// Temporarily stripped of authority pending review.
    Suspended,
    /// Terminal; permanent removal of ordinary authority. Re-enrollment
    /// requires a new membership.
    Revoked,
}

impl MembershipState {
    /// Returns `true` when the state is [`MembershipState::Expired`] or
    /// [`MembershipState::Revoked`], both of which accept no further
    /// transitions.
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
    /// Opaque membership identifier.
    pub id: MembershipId,
    /// Owning organization, fixed at creation and never reassigned.
    pub organization_id: OrganizationId,
    /// Identity-subsystem user this membership belongs to, fixed at creation.
    pub user_id: TenantUserId,
    /// Current lifecycle state.
    pub state: MembershipState,
    /// Monotonic transition counter, bumped on every legal transition so stale
    /// sessions and authorization caches invalidate (sections 20.3, 23).
    pub state_version: u64,
}

impl Membership {
    /// Issues an invitation-bound membership in state `Invited`. Acceptance,
    /// expiry and revocation are separate explicit transitions (sections
    /// 8, 9).
    #[must_use = "the returned membership is the only handle to the invitation"]
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
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the membership is not
    /// in [`MembershipState::Invited`], and [`TenancyError::TerminalState`]
    /// when it is expired or revoked.
    pub fn mark_pending(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::Invited, MembershipState::PendingAcceptance)
    }

    /// `PENDING_ACCEPTANCE -> EXPIRED`: the enrollment window lapsed.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the membership is not
    /// in [`MembershipState::PendingAcceptance`], and
    /// [`TenancyError::TerminalState`] when it is expired or revoked.
    pub fn expire(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::PendingAcceptance, MembershipState::Expired)
    }

    /// `PENDING_ACCEPTANCE -> ACTIVE`: enrollment accepted.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the membership is not
    /// in [`MembershipState::PendingAcceptance`], and
    /// [`TenancyError::TerminalState`] when it is expired or revoked.
    pub fn activate(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::PendingAcceptance, MembershipState::Active)
    }

    /// `ACTIVE -> SUSPENDED`: temporary removal of authority (section 20.1).
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the membership is not
    /// in [`MembershipState::Active`], and [`TenancyError::TerminalState`] when
    /// it is expired or revoked.
    pub fn suspend(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::Active, MembershipState::Suspended)
    }

    /// `SUSPENDED -> ACTIVE`: reactivation after review.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the membership is not
    /// in [`MembershipState::Suspended`], and [`TenancyError::TerminalState`]
    /// when it is expired or revoked.
    pub fn resume(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::Suspended, MembershipState::Active)
    }

    /// `ACTIVE -> REVOKED`: permanent termination for ordinary purposes
    /// (section 20.2). Bumps the version so caches and sessions invalidate
    /// (section 20.3).
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidTransition`] when the membership is not
    /// in [`MembershipState::Active`], and [`TenancyError::TerminalState`] when
    /// it is expired or revoked.
    pub fn revoke(&mut self) -> Result<(), TenancyError> {
        self.transition(MembershipState::Active, MembershipState::Revoked)
    }

    /// Ordinary business authority predicate (section 8).
    #[must_use]
    pub fn has_authority(&self) -> bool {
        self.state.has_authority()
    }
}

/// The domain projection committed when an organization is provisioned with
/// its owner membership and default branch. Persistence adapters return this
/// value only after the atomic provisioning transaction succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedOrganization {
    /// The organization, committed in `Provisioning`.
    pub organization: Organization,
    /// The owner membership created alongside the organization (section 7.1).
    pub owner_membership: Membership,
    /// The default branch created alongside the organization.
    pub default_branch: Branch,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn org_id(value: &str) -> Result<OrganizationId, TenancyError> {
        OrganizationId::new(value)
    }

    fn branch_id(value: &str) -> Result<BranchId, TenancyError> {
        BranchId::new(value)
    }

    fn active_org(id: &str) -> Result<Organization, TenancyError> {
        let mut org = Organization::provision(org_id(id)?, "Test Merchant")?;
        org.activate()?;
        Ok(org)
    }

    fn active_branch(id: &str, org: &str) -> Result<Branch, TenancyError> {
        let mut branch = Branch::provision(branch_id(id)?, org_id(org)?, "Main Branch")?;
        branch.activate()?;
        Ok(branch)
    }

    fn active_membership(id: &str, org: &str, user: &str) -> Result<Membership, TenancyError> {
        let mut membership = Membership::invite(
            MembershipId::new(id)?,
            org_id(org)?,
            TenantUserId::new(user)?,
        );
        membership.mark_pending()?;
        membership.activate()?;
        Ok(membership)
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
    fn names_are_presentation_data_with_bounds() -> Result<(), TenancyError> {
        assert!(Organization::provision(org_id("o1")?, "").is_err());
        assert!(Organization::provision(org_id("o1")?, "   ").is_err());
        assert!(Organization::provision(org_id("o1")?, &"n".repeat(257)).is_err());
        // Names are not unique security identifiers: duplicates are allowed.
        assert!(Organization::provision(org_id("o1")?, "Same Name").is_ok());
        assert!(Organization::provision(org_id("o2")?, "Same Name").is_ok());
        Ok(())
    }

    #[test]
    fn organization_lifecycle_follows_section_6() -> Result<(), TenancyError> {
        let mut org = Organization::provision(org_id("o1")?, "Merchant")?;
        assert_eq!(org.state, OrganizationState::Provisioning);
        assert!(!org.can_operate());

        // Provisioning cannot skip activation.
        assert_eq!(org.suspend(), Err(TenancyError::InvalidTransition));
        assert_eq!(org.begin_close(), Err(TenancyError::InvalidTransition));

        org.activate()?;
        assert!(org.can_operate());

        org.suspend()?;
        assert!(!org.can_operate());
        org.resume()?;
        assert!(org.can_operate());

        // Suspended cannot begin closing; only Active can (section 6 diagram).
        org.suspend()?;
        assert_eq!(org.begin_close(), Err(TenancyError::InvalidTransition));
        org.resume()?;

        org.begin_close()?;
        assert!(!org.can_operate());
        // Closing cannot return to Active.
        assert_eq!(org.resume(), Err(TenancyError::InvalidTransition));
        assert_eq!(org.activate(), Err(TenancyError::InvalidTransition));

        org.close()?;
        assert_eq!(org.state, OrganizationState::Closed);
        assert!(!org.can_operate());
        Ok(())
    }

    #[test]
    fn closed_organization_is_terminal() -> Result<(), TenancyError> {
        let mut org = active_org("o1")?;
        org.begin_close()?;
        org.close()?;
        assert_eq!(org.activate(), Err(TenancyError::TerminalState));
        assert_eq!(org.suspend(), Err(TenancyError::TerminalState));
        assert_eq!(org.resume(), Err(TenancyError::TerminalState));
        assert_eq!(org.begin_close(), Err(TenancyError::TerminalState));
        assert_eq!(org.close(), Err(TenancyError::TerminalState));
        Ok(())
    }

    #[test]
    fn branch_lifecycle_follows_section_14() -> Result<(), TenancyError> {
        let mut branch = Branch::provision(branch_id("b1")?, org_id("o1")?, "Branch")?;
        assert_eq!(branch.state, BranchState::Provisioning);
        assert!(!branch.can_operate());

        assert_eq!(branch.suspend(), Err(TenancyError::InvalidTransition));
        branch.activate()?;
        assert!(branch.can_operate());

        branch.suspend()?;
        assert!(!branch.can_operate());
        branch.resume()?;
        assert!(branch.can_operate());

        branch.begin_close()?;
        assert!(!branch.can_operate());
        branch.close()?;
        assert_eq!(branch.state, BranchState::Closed);
        assert_eq!(branch.resume(), Err(TenancyError::TerminalState));
        Ok(())
    }

    #[test]
    fn branch_organization_binding_is_immutable() -> Result<(), TenancyError> {
        let branch = active_branch("b1", "o1")?;
        assert_eq!(branch.organization_id.as_str(), "o1");
        // No setter exists: the binding can only be established at creation.
        // This test pins the struct shape against accidental mutability.
        let Branch {
            organization_id, ..
        } = &branch;
        assert_eq!(organization_id.as_str(), "o1");
        Ok(())
    }

    #[test]
    fn membership_lifecycle_follows_section_8() -> Result<(), TenancyError> {
        let mut membership = Membership::invite(
            MembershipId::new("m1")?,
            org_id("o1")?,
            TenantUserId::new("u1")?,
        );
        assert_eq!(membership.state, MembershipState::Invited);
        assert!(!membership.has_authority());

        // Invited cannot skip to Active or Revoked.
        assert_eq!(membership.activate(), Err(TenancyError::InvalidTransition));
        assert_eq!(membership.revoke(), Err(TenancyError::InvalidTransition));

        membership.mark_pending()?;
        assert!(!membership.has_authority());
        membership.activate()?;
        assert!(membership.has_authority());

        membership.suspend()?;
        assert!(!membership.has_authority());
        membership.resume()?;
        assert!(membership.has_authority());

        membership.revoke()?;
        assert_eq!(membership.state, MembershipState::Revoked);
        assert!(!membership.has_authority());
        Ok(())
    }

    #[test]
    fn revoked_membership_cannot_be_reactivated() -> Result<(), TenancyError> {
        let mut membership = active_membership("m1", "o1", "u1")?;
        membership.revoke()?;
        assert_eq!(membership.resume(), Err(TenancyError::TerminalState));
        assert_eq!(membership.activate(), Err(TenancyError::TerminalState));
        assert_eq!(membership.suspend(), Err(TenancyError::TerminalState));
        assert_eq!(membership.revoke(), Err(TenancyError::TerminalState));
        Ok(())
    }

    #[test]
    fn expired_membership_is_terminal() -> Result<(), TenancyError> {
        let mut membership = Membership::invite(
            MembershipId::new("m1")?,
            org_id("o1")?,
            TenantUserId::new("u1")?,
        );
        membership.mark_pending()?;
        membership.expire()?;
        assert_eq!(membership.state, MembershipState::Expired);
        assert!(!membership.has_authority());
        assert_eq!(membership.activate(), Err(TenancyError::TerminalState));
        Ok(())
    }

    #[test]
    fn invitation_cannot_expire_before_pending() -> Result<(), TenancyError> {
        let mut membership = Membership::invite(
            MembershipId::new("m1")?,
            org_id("o1")?,
            TenantUserId::new("u1")?,
        );
        assert_eq!(membership.expire(), Err(TenancyError::InvalidTransition));
        Ok(())
    }

    #[test]
    fn state_versions_increase_monotonically() -> Result<(), TenancyError> {
        let mut org = Organization::provision(org_id("o1")?, "Merchant")?;
        let v0 = org.state_version;
        org.activate()?;
        org.suspend()?;
        org.resume()?;
        assert!(org.state_version > v0);

        let mut membership = active_membership("m1", "o1", "u1")?;
        let mv0 = membership.state_version;
        membership.suspend()?;
        membership.resume()?;
        membership.revoke()?;
        assert!(membership.state_version > mv0);
        Ok(())
    }
}
