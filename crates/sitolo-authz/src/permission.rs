//! Atomic permissions and role definition maps.

use crate::id::RoleId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Permission {
    // Organization
    OrgView,
    OrgUpdate,
    OrgClose,
    OrgSuspend,

    // Membership & IAM
    MembershipView,
    MembershipInvite,
    MembershipSuspend,
    MembershipRevoke,
    RoleAssign,
    ScopeGrant,

    // Branch & Infrastructure
    BranchCreate,
    BranchUpdate,
    BranchClose,
    WarehouseCreate,
    RegisterCreate,

    // Device
    DeviceRegister,
    DeviceRevoke,

    // Sales & POS
    SaleCreate,
    SaleView,
    SaleVoid,
    RefundCreate,
    RefundApprove,

    // Inventory
    InventoryView,
    InventoryAdjust,
    InventoryAdjustApprove,
    InventoryTransfer,

    // Reporting & Export
    ReportView,
    ExportCreate,
    AuditView,

    // Payment & Reconciliation
    PaymentView,
    PaymentReconcile,

    // Tax & Regulatory
    EisSubmit,
    EisConfigure,
}

impl Permission {
    pub fn as_str(self) -> &'static str {
        match self {
            Permission::OrgView => "org:view",
            Permission::OrgUpdate => "org:update",
            Permission::OrgClose => "org:close",
            Permission::OrgSuspend => "org:suspend",
            Permission::MembershipView => "membership:view",
            Permission::MembershipInvite => "membership:invite",
            Permission::MembershipSuspend => "membership:suspend",
            Permission::MembershipRevoke => "membership:revoke",
            Permission::RoleAssign => "role:assign",
            Permission::ScopeGrant => "scope:grant",
            Permission::BranchCreate => "branch:create",
            Permission::BranchUpdate => "branch:update",
            Permission::BranchClose => "branch:close",
            Permission::WarehouseCreate => "warehouse:create",
            Permission::RegisterCreate => "register:create",
            Permission::DeviceRegister => "device:register",
            Permission::DeviceRevoke => "device:revoke",
            Permission::SaleCreate => "sale:create",
            Permission::SaleView => "sale:view",
            Permission::SaleVoid => "sale:void",
            Permission::RefundCreate => "refund:create",
            Permission::RefundApprove => "refund:approve",
            Permission::InventoryView => "inventory:view",
            Permission::InventoryAdjust => "inventory:adjust",
            Permission::InventoryAdjustApprove => "inventory:adjust_approve",
            Permission::InventoryTransfer => "inventory:transfer",
            Permission::ReportView => "report:view",
            Permission::ExportCreate => "export:create",
            Permission::AuditView => "audit:view",
            Permission::PaymentView => "payment:view",
            Permission::PaymentReconcile => "payment:reconcile",
            Permission::EisSubmit => "eis:submit",
            Permission::EisConfigure => "eis:configure",
        }
    }
}

/// Built-in role constants.
pub const ROLE_OWNER: &str = "OWNER";
pub const ROLE_ORG_ADMIN: &str = "ORG_ADMIN";
pub const ROLE_BRANCH_MANAGER: &str = "BRANCH_MANAGER";
pub const ROLE_CASHIER: &str = "CASHIER";
pub const ROLE_INVENTORY_CLERK: &str = "INVENTORY_CLERK";
pub const ROLE_PROCUREMENT_CLERK: &str = "PROCUREMENT_CLERK";
pub const ROLE_ACCOUNTING_USER: &str = "ACCOUNTING_USER";
pub const ROLE_VIEWER: &str = "VIEWER";
pub const ROLE_AUDITOR: &str = "AUDITOR";

/// Get the atomic permission list for a built-in role.
pub fn role_permissions(role: &RoleId) -> &'static [Permission] {
    match role.as_str() {
        ROLE_OWNER => &[
            Permission::OrgView,
            Permission::OrgUpdate,
            Permission::OrgClose,
            Permission::OrgSuspend,
            Permission::MembershipView,
            Permission::MembershipInvite,
            Permission::MembershipSuspend,
            Permission::MembershipRevoke,
            Permission::RoleAssign,
            Permission::ScopeGrant,
            Permission::BranchCreate,
            Permission::BranchUpdate,
            Permission::BranchClose,
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
        ROLE_ORG_ADMIN => &[
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
        ],
        ROLE_BRANCH_MANAGER => &[
            Permission::OrgView,
            Permission::MembershipView,
            Permission::BranchUpdate,
            Permission::RegisterCreate,
            Permission::DeviceRegister,
            Permission::SaleCreate,
            Permission::SaleView,
            Permission::SaleVoid,
            Permission::RefundCreate,
            Permission::RefundApprove,
            Permission::InventoryView,
            Permission::InventoryAdjust,
            Permission::InventoryTransfer,
            Permission::ReportView,
            Permission::PaymentView,
        ],
        ROLE_CASHIER => &[
            Permission::OrgView,
            Permission::SaleCreate,
            Permission::SaleView,
            Permission::RefundCreate,
            Permission::InventoryView,
            Permission::PaymentView,
        ],
        ROLE_INVENTORY_CLERK => &[
            Permission::OrgView,
            Permission::InventoryView,
            Permission::InventoryAdjust,
            Permission::InventoryTransfer,
        ],
        ROLE_PROCUREMENT_CLERK => &[
            Permission::OrgView,
            Permission::InventoryView,
            Permission::InventoryAdjust,
        ],
        ROLE_ACCOUNTING_USER => &[
            Permission::OrgView,
            Permission::SaleView,
            Permission::ReportView,
            Permission::PaymentView,
            Permission::PaymentReconcile,
            Permission::ExportCreate,
        ],
        ROLE_VIEWER => &[
            Permission::OrgView,
            Permission::SaleView,
            Permission::InventoryView,
            Permission::ReportView,
        ],
        ROLE_AUDITOR => &[
            Permission::OrgView,
            Permission::MembershipView,
            Permission::SaleView,
            Permission::InventoryView,
            Permission::ReportView,
            Permission::AuditView,
            Permission::PaymentView,
            Permission::ExportCreate,
        ],
        _ => &[],
    }
}
