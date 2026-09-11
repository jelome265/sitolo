//! Role catalog, permission catalog, assignment lifecycle, and permission
//! resolution.
//!
//! Phase 4 specification, sections 3.10, 3.11, 10 and 19. Roles are the
//! human vocabulary for authority; permissions are atomic capabilities; a
//! role resolves to permissions through this versioned server-side mapping
//! (section 10.2). Assignment state follows section 19:
//!
//! ```text
//! REQUESTED -> VALIDATING -> EFFECTIVE -> REVOKED
//!                  |
//!                  +-> REJECTED
//!                  |
//!                  +-> APPROVAL_REQUIRED -> APPROVED -> EFFECTIVE
//! ```
//!
//! The approval branch is modeled but not yet driven: high-risk transitions
//! require step-up/approval policy owned by Phase 6. Scope grants narrow
//! these organization-wide grants to branches and resources in PR-004; until
//! then every grant here is organization-wide by construction.

use std::collections::BTreeSet;

use sitolo_domain::tenancy::{MembershipId, OrganizationId, RoleAssignmentId, TenancyError};
use thiserror::Error;

/// Version of the built-in role definition mapping below. Bumped if and only
/// if a role gains or loses a permission; resolution consumers can detect
/// definition drift instead of silently applying a new policy (section 10.2).
pub const ROLE_DEFINITION_VERSION: u32 = 1;

/// Atomic operation capabilities (Phase 4 section 10.1). Codes match the
/// specification vocabulary exactly; authorization behavior must never depend
/// on string comparisons scattered through handlers (section 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    OrgView,
    OrgUpdate,
    OrgClose,
    MembershipView,
    MembershipInvite,
    MembershipSuspend,
    MembershipRevoke,
    RoleAssign,
    ScopeGrant,
    BranchCreate,
    BranchUpdate,
    BranchArchive,
    WarehouseCreate,
    RegisterCreate,
    DeviceRegister,
    DeviceRevoke,
    SaleCreate,
    SaleView,
    SaleVoid,
    RefundCreate,
    RefundApprove,
    InventoryView,
    InventoryAdjust,
    InventoryAdjustApprove,
    InventoryTransfer,
    ReportView,
    ExportCreate,
    AuditView,
    PaymentView,
    PaymentReconcile,
    EisSubmit,
    EisConfigure,
}

impl Permission {
    /// Stable machine-readable code from the specification vocabulary.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Permission::OrgView => "ORG_VIEW",
            Permission::OrgUpdate => "ORG_UPDATE",
            Permission::OrgClose => "ORG_CLOSE",
            Permission::MembershipView => "MEMBERSHIP_VIEW",
            Permission::MembershipInvite => "MEMBERSHIP_INVITE",
            Permission::MembershipSuspend => "MEMBERSHIP_SUSPEND",
            Permission::MembershipRevoke => "MEMBERSHIP_REVOKE",
            Permission::RoleAssign => "ROLE_ASSIGN",
            Permission::ScopeGrant => "SCOPE_GRANT",
            Permission::BranchCreate => "BRANCH_CREATE",
            Permission::BranchUpdate => "BRANCH_UPDATE",
            Permission::BranchArchive => "BRANCH_ARCHIVE",
            Permission::WarehouseCreate => "WAREHOUSE_CREATE",
            Permission::RegisterCreate => "REGISTER_CREATE",
            Permission::DeviceRegister => "DEVICE_REGISTER",
            Permission::DeviceRevoke => "DEVICE_REVOKE",
            Permission::SaleCreate => "SALE_CREATE",
            Permission::SaleView => "SALE_VIEW",
            Permission::SaleVoid => "SALE_VOID",
            Permission::RefundCreate => "REFUND_CREATE",
            Permission::RefundApprove => "REFUND_APPROVE",
            Permission::InventoryView => "INVENTORY_VIEW",
            Permission::InventoryAdjust => "INVENTORY_ADJUST",
            Permission::InventoryAdjustApprove => "INVENTORY_ADJUST_APPROVE",
            Permission::InventoryTransfer => "INVENTORY_TRANSFER",
            Permission::ReportView => "REPORT_VIEW",
            Permission::ExportCreate => "EXPORT_CREATE",
            Permission::AuditView => "AUDIT_VIEW",
            Permission::PaymentView => "PAYMENT_VIEW",
            Permission::PaymentReconcile => "PAYMENT_RECONCILE",
            Permission::EisSubmit => "EIS_SUBMIT",
            Permission::EisConfigure => "EIS_CONFIGURE",
        }
    }

    /// Every permission in the catalog, in canonical order.
    #[must_use]
    pub fn all() -> &'static [Permission] {
        &[
            Permission::OrgView,
            Permission::OrgUpdate,
            Permission::OrgClose,
            Permission::MembershipView,
            Permission::MembershipInvite,
            Permission::MembershipSuspend,
            Permission::MembershipRevoke,
            Permission::RoleAssign,
            Permission::ScopeGrant,
            Permission::BranchCreate,
            Permission::BranchUpdate,
            Permission::BranchArchive,
            Permission::WarehouseCreate,
            Permission::RegisterCreate,
            Permission::DeviceRegister,
            Permission::DeviceRevoke,
            Permission::SaleCreate,
            Permission::SaleView,
            Permission::SaleVoid,
            Permission::RefundCreate,
            Permission::RefundApprove,
            Permission::InventoryView,
            Permission::InventoryAdjust,
            Permission::InventoryAdjustApprove,
            Permission::InventoryTransfer,
            Permission::ReportView,
            Permission::ExportCreate,
            Permission::AuditView,
            Permission::PaymentView,
            Permission::PaymentReconcile,
            Permission::EisSubmit,
            Permission::EisConfigure,
        ]
    }
}

/// Built-in role vocabulary (Phase 4 section 10). Roles are bundles, never
/// complete authorization decisions (section 10.2): the effective decision
/// additionally requires scope, state, assurance, and policy checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    Owner,
    OrgAdmin,
    BranchManager,
    Cashier,
    InventoryClerk,
    ProcurementClerk,
    AccountingUser,
    Viewer,
    Auditor,
}

impl Role {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "OWNER",
            Role::OrgAdmin => "ORG_ADMIN",
            Role::BranchManager => "BRANCH_MANAGER",
            Role::Cashier => "CASHIER",
            Role::InventoryClerk => "INVENTORY_CLERK",
            Role::ProcurementClerk => "PROCUREMENT_CLERK",
            Role::AccountingUser => "ACCOUNTING_USER",
            Role::Viewer => "VIEWER",
            Role::Auditor => "AUDITOR",
        }
    }

    #[must_use]
    pub fn all() -> &'static [Role] {
        &[
            Role::Owner,
            Role::OrgAdmin,
            Role::BranchManager,
            Role::Cashier,
            Role::InventoryClerk,
            Role::ProcurementClerk,
            Role::AccountingUser,
            Role::Viewer,
            Role::Auditor,
        ]
    }

    /// Server-side role definition at [`ROLE_DEFINITION_VERSION`], least
    /// privilege by default:
    ///
    /// - `Owner` holds everything, including organization closure.
    /// - `OrgAdmin` delegates everything except closure.
    /// - `BranchManager` runs shop operations; cannot manage roles, close
    ///   the organization, approve refunds, or touch tax configuration.
    /// - `Cashier` sells; voids and refunds require approval elsewhere.
    /// - `InventoryClerk` moves and adjusts stock; approvals stay above.
    /// - `ProcurementClerk` holds read access until procurement workflows
    ///   (Phase 13) define dedicated capabilities.
    /// - `AccountingUser` reconciles and reports; cannot mutate membership
    ///   or roles.
    /// - `Viewer` and `Auditor` are read-only; only the auditor exports.
    #[must_use]
    pub fn permissions(self) -> &'static [Permission] {
        match self {
            Role::Owner => Permission::all(),
            Role::OrgAdmin => &[
                Permission::OrgView,
                Permission::OrgUpdate,
                Permission::MembershipView,
                Permission::MembershipInvite,
                Permission::MembershipSuspend,
                Permission::MembershipRevoke,
                Permission::RoleAssign,
                Permission::ScopeGrant,
                Permission::BranchCreate,
                Permission::BranchUpdate,
                Permission::BranchArchive,
                Permission::WarehouseCreate,
                Permission::RegisterCreate,
                Permission::DeviceRegister,
                Permission::DeviceRevoke,
                Permission::SaleCreate,
                Permission::SaleView,
                Permission::SaleVoid,
                Permission::RefundCreate,
                Permission::RefundApprove,
                Permission::InventoryView,
                Permission::InventoryAdjust,
                Permission::InventoryAdjustApprove,
                Permission::InventoryTransfer,
                Permission::ReportView,
                Permission::ExportCreate,
                Permission::AuditView,
                Permission::PaymentView,
                Permission::PaymentReconcile,
                Permission::EisSubmit,
                Permission::EisConfigure,
            ],
            Role::BranchManager => &[
                Permission::OrgView,
                Permission::MembershipView,
                Permission::MembershipInvite,
                Permission::BranchUpdate,
                Permission::RegisterCreate,
                Permission::DeviceRegister,
                Permission::SaleCreate,
                Permission::SaleView,
                Permission::SaleVoid,
                Permission::RefundCreate,
                Permission::InventoryView,
                Permission::InventoryAdjust,
                Permission::InventoryTransfer,
                Permission::ReportView,
                Permission::PaymentView,
            ],
            Role::Cashier => &[Permission::SaleCreate, Permission::SaleView],
            Role::InventoryClerk => &[
                Permission::InventoryView,
                Permission::InventoryAdjust,
                Permission::InventoryTransfer,
            ],
            Role::ProcurementClerk => &[Permission::InventoryView, Permission::ReportView],
            Role::AccountingUser => &[
                Permission::SaleView,
                Permission::RefundApprove,
                Permission::InventoryView,
                Permission::InventoryAdjustApprove,
                Permission::ReportView,
                Permission::ExportCreate,
                Permission::AuditView,
                Permission::PaymentView,
                Permission::PaymentReconcile,
            ],
            Role::Viewer => &[
                Permission::OrgView,
                Permission::MembershipView,
                Permission::SaleView,
                Permission::InventoryView,
                Permission::ReportView,
                Permission::PaymentView,
            ],
            Role::Auditor => &[
                Permission::OrgView,
                Permission::MembershipView,
                Permission::SaleView,
                Permission::InventoryView,
                Permission::ReportView,
                Permission::ExportCreate,
                Permission::AuditView,
                Permission::PaymentView,
            ],
        }
    }
}

/// Failures for role-assignment transitions.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssignmentError {
    #[error("invalid assignment transition")]
    InvalidTransition,
    #[error("assignment is in a terminal state")]
    TerminalState,
}

impl From<AssignmentError> for TenancyError {
    fn from(value: AssignmentError) -> Self {
        match value {
            AssignmentError::InvalidTransition => TenancyError::InvalidTransition,
            AssignmentError::TerminalState => TenancyError::TerminalState,
        }
    }
}

/// Role-assignment lifecycle (Phase 4 section 19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleAssignmentState {
    Requested,
    Validating,
    Rejected,
    ApprovalRequired,
    Approved,
    Effective,
    Revoked,
}

impl RoleAssignmentState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            RoleAssignmentState::Rejected | RoleAssignmentState::Revoked
        )
    }

    /// Only `Effective` assignments contribute permissions.
    #[must_use]
    pub fn grants_authority(self) -> bool {
        matches!(self, RoleAssignmentState::Effective)
    }
}

/// One role held by one membership in one organization. Bindings are
/// immutable; ending authority revokes the record (history is preserved per
/// Phase 4 section 8.1.10) and re-granting creates a new record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleAssignment {
    pub id: RoleAssignmentId,
    pub organization_id: OrganizationId,
    pub membership_id: MembershipId,
    pub role: Role,
    pub state: RoleAssignmentState,
    pub state_version: u64,
}

impl RoleAssignment {
    /// Begins an assignment in state `Requested`. Validation, approval, and
    /// activation are separate explicit transitions below.
    pub fn request(
        id: RoleAssignmentId,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        role: Role,
    ) -> Self {
        RoleAssignment {
            id,
            organization_id,
            membership_id,
            role,
            state: RoleAssignmentState::Requested,
            state_version: 1,
        }
    }

    fn transition(
        &mut self,
        from: RoleAssignmentState,
        to: RoleAssignmentState,
    ) -> Result<(), AssignmentError> {
        if self.state.is_terminal() {
            return Err(AssignmentError::TerminalState);
        }
        if self.state != from {
            return Err(AssignmentError::InvalidTransition);
        }
        self.state = to;
        self.state_version = self.state_version.saturating_add(1);
        Ok(())
    }

    /// `REQUESTED -> VALIDATING`: the grant is checked against membership
    /// state, catalog validity, and (later) issuer authority.
    pub fn begin_validation(&mut self) -> Result<(), AssignmentError> {
        self.transition(
            RoleAssignmentState::Requested,
            RoleAssignmentState::Validating,
        )
    }

    /// `VALIDATING -> REJECTED`: terminal refusal, preserved as evidence.
    pub fn reject(&mut self) -> Result<(), AssignmentError> {
        self.transition(
            RoleAssignmentState::Validating,
            RoleAssignmentState::Rejected,
        )
    }

    /// `VALIDATING -> APPROVAL_REQUIRED`: high-risk grants wait for step-up
    /// approval owned by Phase 6.
    pub fn require_approval(&mut self) -> Result<(), AssignmentError> {
        self.transition(
            RoleAssignmentState::Validating,
            RoleAssignmentState::ApprovalRequired,
        )
    }

    /// `APPROVAL_REQUIRED -> APPROVED`: the required assurance was met.
    pub fn approve(&mut self) -> Result<(), AssignmentError> {
        self.transition(
            RoleAssignmentState::ApprovalRequired,
            RoleAssignmentState::Approved,
        )
    }

    /// `VALIDATING|APPROVED -> EFFECTIVE`: the grant contributes authority.
    /// Low-risk grants transition straight from validation; approved grants
    /// complete the approval branch.
    pub fn mark_effective(&mut self) -> Result<(), AssignmentError> {
        if self.state.is_terminal() {
            return Err(AssignmentError::TerminalState);
        }
        match self.state {
            RoleAssignmentState::Validating | RoleAssignmentState::Approved => {
                self.state = RoleAssignmentState::Effective;
                self.state_version = self.state_version.saturating_add(1);
                Ok(())
            }
            _ => Err(AssignmentError::InvalidTransition),
        }
    }

    /// `EFFECTIVE -> REVOKED`: terminal. Re-granting the same role creates a
    /// new record rather than reviving this one.
    pub fn revoke(&mut self) -> Result<(), AssignmentError> {
        self.transition(RoleAssignmentState::Effective, RoleAssignmentState::Revoked)
    }
}

/// Resolves the union of permissions granted by `Effective` assignments
/// (Phase 4 section 10.2). Scope narrowing, state, assurance, and policy
/// intersection arrive with scope grants (PR-004) and Phase 6 — this is the
/// role layer only, never a complete authorization decision.
#[must_use]
pub fn resolve_permissions(assignments: &[RoleAssignment]) -> BTreeSet<Permission> {
    assignments
        .iter()
        .filter(|assignment| assignment.state.grants_authority())
        .flat_map(|assignment| assignment.role.permissions().iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assignment(role: Role) -> RoleAssignment {
        RoleAssignment::request(
            RoleAssignmentId::new("ra-1").unwrap(),
            OrganizationId::new("o1").unwrap(),
            MembershipId::new("m1").unwrap(),
            role,
        )
    }

    fn effective(role: Role) -> RoleAssignment {
        let mut assignment = assignment(role);
        assignment.begin_validation().unwrap();
        assignment.mark_effective().unwrap();
        assignment
    }

    #[test]
    fn catalog_codes_are_stable_and_unique() {
        let mut codes = BTreeSet::new();
        for permission in Permission::all() {
            assert!(codes.insert(permission.as_str()), "duplicate code");
        }
        assert_eq!(Permission::all().len(), 32);
        let mut roles = BTreeSet::new();
        for role in Role::all() {
            assert!(roles.insert(role.as_str()), "duplicate role");
        }
        assert_eq!(Role::all().len(), 9);
        // Every mapped permission is a catalog member.
        for role in Role::all() {
            for permission in role.permissions() {
                assert!(
                    Permission::all().contains(permission),
                    "{} maps unknown permission",
                    role.as_str()
                );
            }
        }
    }

    #[test]
    fn owner_holds_everything_admin_cannot_close() {
        let owner: BTreeSet<_> = Role::Owner.permissions().iter().copied().collect();
        assert_eq!(owner.len(), Permission::all().len());
        let admin: BTreeSet<_> = Role::OrgAdmin.permissions().iter().copied().collect();
        assert!(!admin.contains(&Permission::OrgClose));
        assert!(admin.contains(&Permission::RoleAssign));
        assert_eq!(
            ROLE_DEFINITION_VERSION, 1,
            "mapping changed without a version bump"
        );
    }

    #[test]
    fn frontline_roles_are_least_privilege() {
        let cashier: BTreeSet<_> = Role::Cashier.permissions().iter().copied().collect();
        assert_eq!(
            cashier,
            BTreeSet::from([Permission::SaleCreate, Permission::SaleView])
        );
        let viewer: BTreeSet<_> = Role::Viewer.permissions().iter().copied().collect();
        assert!(!viewer.contains(&Permission::SaleCreate));
        assert!(!viewer.contains(&Permission::ExportCreate));
        let auditor: BTreeSet<_> = Role::Auditor.permissions().iter().copied().collect();
        assert!(!auditor.contains(&Permission::RoleAssign));
        assert!(auditor.contains(&Permission::ExportCreate));
        let manager: BTreeSet<_> = Role::BranchManager.permissions().iter().copied().collect();
        assert!(!manager.contains(&Permission::RoleAssign));
        assert!(!manager.contains(&Permission::RefundApprove));
        assert!(!manager.contains(&Permission::EisConfigure));
    }

    #[test]
    fn assignment_lifecycle_follows_section_19() {
        let mut assignment = assignment(Role::Cashier);
        assert_eq!(assignment.state, RoleAssignmentState::Requested);
        // Cannot skip validation.
        assert_eq!(
            assignment.mark_effective(),
            Err(AssignmentError::InvalidTransition)
        );
        assignment.begin_validation().unwrap();
        assignment.mark_effective().unwrap();
        assert!(assignment.state.grants_authority());
        assignment.revoke().unwrap();
        assert_eq!(assignment.state, RoleAssignmentState::Revoked);
        assert!(!assignment.state.grants_authority());
    }

    #[test]
    fn approval_branch_requires_approval_before_effective() {
        let mut assignment = assignment(Role::OrgAdmin);
        assignment.begin_validation().unwrap();
        assignment.require_approval().unwrap();
        // Approval-gated grants cannot become effective early.
        assert_eq!(
            assignment.mark_effective(),
            Err(AssignmentError::InvalidTransition)
        );
        assignment.approve().unwrap();
        assignment.mark_effective().unwrap();
        assert!(assignment.state.grants_authority());
    }

    #[test]
    fn rejection_and_revocation_are_terminal() {
        let mut rejected = assignment(Role::Cashier);
        rejected.begin_validation().unwrap();
        rejected.reject().unwrap();
        assert_eq!(
            rejected.mark_effective(),
            Err(AssignmentError::TerminalState)
        );
        assert_eq!(rejected.revoke(), Err(AssignmentError::TerminalState));

        let mut revoked = effective(Role::Cashier);
        revoked.revoke().unwrap();
        assert_eq!(
            revoked.begin_validation(),
            Err(AssignmentError::TerminalState)
        );
    }

    #[test]
    fn resolution_counts_only_effective_assignments() {
        let active = effective(Role::Cashier);
        let mut pending = assignment(Role::BranchManager);
        pending.begin_validation().unwrap();
        let mut revoked = effective(Role::InventoryClerk);
        revoked.revoke().unwrap();
        let permissions = resolve_permissions(&[active, pending, revoked]);
        assert_eq!(
            permissions,
            BTreeSet::from([Permission::SaleCreate, Permission::SaleView])
        );
        assert!(resolve_permissions(&[]).is_empty());
    }

    #[test]
    fn assignment_errors_map_to_tenancy_errors() {
        assert_eq!(
            TenancyError::from(AssignmentError::InvalidTransition),
            TenancyError::InvalidTransition
        );
        assert_eq!(
            TenancyError::from(AssignmentError::TerminalState),
            TenancyError::TerminalState
        );
    }
}
