//! Scope model, scope grants, and scope authorization.
//!
//! Phase 4 specification, sections 3.12, 11 and 54. A scope names where a
//! permission applies: organization-wide or narrowed to one branch. Grants
//! narrow; they never widen. With no active grants a membership's role
//! authority applies organization-wide by default; the first grant switches
//! that membership to allow-list narrowing:
//!
//! ```text
//! ROLE PERMISSIONS
//!         ∩
//! MEMBERSHIP SCOPE (grants; default = whole organization)
//!         =
//! AUTHORIZED SCOPE
//! ```
//!
//! (Further intersection with resource state, assurance, policy, and
//! entitlement arrives with Phase 6. Warehouse, register, and device levels
//! extend `Scope` additively once those entities exist.)

use sitolo_domain::tenancy::{BranchId, MembershipId, OrganizationId, ScopeGrantId, TenancyError};
use thiserror::Error;

/// Where a permission applies (Phase 4 section 11 hierarchy, organization
/// and branch levels).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    Organization {
        organization_id: OrganizationId,
    },
    Branch {
        organization_id: OrganizationId,
        branch_id: BranchId,
    },
}

impl Scope {
    /// The owning organization of this scope.
    #[must_use]
    pub fn organization_id(&self) -> &OrganizationId {
        match self {
            Scope::Organization { organization_id } => organization_id,
            Scope::Branch {
                organization_id, ..
            } => organization_id,
        }
    }

    /// Scope containment (section 11.1). A broader scope covers narrower
    /// descendants in the same organization; a branch scope covers only
    /// itself — `BRANCH A` authority never implies `BRANCH B`.
    #[must_use]
    pub fn covers(&self, other: &Scope) -> bool {
        match (self, other) {
            (
                Scope::Organization { organization_id },
                Scope::Organization {
                    organization_id: other_org,
                },
            ) => organization_id == other_org,
            (
                Scope::Organization { organization_id },
                Scope::Branch {
                    organization_id: other_org,
                    ..
                },
            ) => organization_id == other_org,
            (
                Scope::Branch {
                    organization_id,
                    branch_id,
                },
                Scope::Branch {
                    organization_id: other_org,
                    branch_id: other_branch,
                },
            ) => organization_id == other_org && branch_id == other_branch,
            (Scope::Branch { .. }, Scope::Organization { .. }) => false,
        }
    }
}

/// Failures for scope-grant transitions.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GrantError {
    #[error("invalid grant transition")]
    InvalidTransition,
    #[error("grant is in a terminal state")]
    TerminalState,
}

impl From<GrantError> for TenancyError {
    fn from(value: GrantError) -> Self {
        match value {
            GrantError::InvalidTransition => TenancyError::InvalidTransition,
            GrantError::TerminalState => TenancyError::TerminalState,
        }
    }
}

/// Scope-grant lifecycle. Grants are allow-list entries: `Active` narrows,
/// `Revoked` is terminal and preserved as evidence. Re-narrowing creates a
/// new record rather than reviving a revoked one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeGrantState {
    Active,
    Revoked,
}

impl ScopeGrantState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, ScopeGrantState::Revoked)
    }

    /// Only `Active` grants narrow authority.
    #[must_use]
    pub fn narrows(self) -> bool {
        matches!(self, ScopeGrantState::Active)
    }
}

/// One scope narrowing entry for one membership in one organization
/// (Phase 4 section 3.12). Bindings are immutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeGrant {
    pub id: ScopeGrantId,
    pub organization_id: OrganizationId,
    pub membership_id: MembershipId,
    pub scope: Scope,
    pub state: ScopeGrantState,
    pub state_version: u64,
}

impl ScopeGrant {
    /// Issues an active grant. Structural validation (grant scope inside the
    /// granting organization) is enforced by the repository, which loads the
    /// referenced records.
    pub fn grant(
        id: ScopeGrantId,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        scope: Scope,
    ) -> Self {
        ScopeGrant {
            id,
            organization_id,
            membership_id,
            scope,
            state: ScopeGrantState::Active,
            state_version: 1,
        }
    }

    /// `ACTIVE -> REVOKED`: terminal. Lifts the narrowing this entry
    /// imposed; remaining active grants still apply.
    pub fn revoke(&mut self) -> Result<(), GrantError> {
        if self.state.is_terminal() {
            return Err(GrantError::TerminalState);
        }
        if self.state != ScopeGrantState::Active {
            return Err(GrantError::InvalidTransition);
        }
        self.state = ScopeGrantState::Revoked;
        self.state_version = self.state_version.saturating_add(1);
        Ok(())
    }
}

/// Authorizes a requested scope against active grants. With no active
/// grants, organization-wide role authority applies unchanged; otherwise the
/// requested scope must fall inside at least one grant. Revoked grants are
/// ignored. Pure function over already-loaded records — existence and
/// operating-state checks belong to the scope-resolution layer.
#[must_use]
pub fn authorize_scope(grants: &[ScopeGrant], requested: &Scope) -> bool {
    let mut narrowing = false;
    for grant in grants.iter().filter(|grant| grant.state.narrows()) {
        narrowing = true;
        if grant.scope.covers(requested) {
            return true;
        }
    }
    !narrowing
}

#[cfg(test)]
mod tests {
    use super::*;

    fn org(value: &str) -> OrganizationId {
        OrganizationId::new(value).unwrap()
    }

    fn branch(value: &str) -> BranchId {
        BranchId::new(value).unwrap()
    }

    fn grant(scope: Scope) -> ScopeGrant {
        ScopeGrant::grant(
            ScopeGrantId::new("g1").unwrap(),
            org("o1"),
            MembershipId::new("m1").unwrap(),
            scope,
        )
    }

    #[test]
    fn covers_follows_section_11_inheritance() {
        let org_a = Scope::Organization {
            organization_id: org("o-a"),
        };
        let branch_a1 = Scope::Branch {
            organization_id: org("o-a"),
            branch_id: branch("b1"),
        };
        let branch_a2 = Scope::Branch {
            organization_id: org("o-a"),
            branch_id: branch("b2"),
        };
        let org_b = Scope::Organization {
            organization_id: org("o-b"),
        };
        // Broad implies narrower descendants in the same organization.
        assert!(org_a.covers(&org_a));
        assert!(org_a.covers(&branch_a1));
        assert!(branch_a1.covers(&branch_a1));
        // Branch A never implies Branch B, another organization, or the
        // organization scope itself.
        assert!(!branch_a1.covers(&branch_a2));
        assert!(!branch_a1.covers(&org_a));
        assert!(!org_a.covers(&org_b));
        assert!(!branch_a1.covers(&org_b));
    }

    #[test]
    fn no_grants_means_organization_wide_authority() {
        let requested = Scope::Branch {
            organization_id: org("o1"),
            branch_id: branch("b1"),
        };
        assert!(authorize_scope(&[], &requested));
    }

    #[test]
    fn grants_narrow_to_allow_list() {
        let branch_a1 = Scope::Branch {
            organization_id: org("o1"),
            branch_id: branch("b1"),
        };
        let branch_a2 = Scope::Branch {
            organization_id: org("o1"),
            branch_id: branch("b2"),
        };
        let org_scope = Scope::Organization {
            organization_id: org("o1"),
        };
        let grants = vec![grant(branch_a1.clone())];
        assert!(authorize_scope(&grants, &branch_a1));
        // Sibling branch denied.
        assert!(!authorize_scope(&grants, &branch_a2));
        // A branch grant does not cover organization-wide operations.
        assert!(!authorize_scope(&grants, &org_scope));

        // Revoking the only grant restores organization-wide authority.
        let mut revoked = grant(branch_a1.clone());
        revoked.revoke().unwrap();
        assert!(authorize_scope(&[revoked], &branch_a2));

        // An organization grant covers everything inside the organization.
        let org_grants = vec![grant(org_scope.clone())];
        assert!(authorize_scope(&org_grants, &branch_a2));
        assert!(authorize_scope(&org_grants, &org_scope));
    }

    #[test]
    fn revocation_is_terminal() {
        let mut grant_record = grant(Scope::Organization {
            organization_id: org("o1"),
        });
        grant_record.revoke().unwrap();
        assert_eq!(grant_record.revoke(), Err(GrantError::TerminalState));
    }

    #[test]
    fn grant_errors_map_to_tenancy_errors() {
        assert_eq!(
            TenancyError::from(GrantError::InvalidTransition),
            TenancyError::InvalidTransition
        );
        assert_eq!(
            TenancyError::from(GrantError::TerminalState),
            TenancyError::TerminalState
        );
    }
}
