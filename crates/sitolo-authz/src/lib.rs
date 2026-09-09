//! Authorization.
//!
//! Owns permissions, scope evaluation, resource authorization, and step-up
//! requirements (Phase 1 specification, §5.5; Phase 4 specification, §10-§11).
#![forbid(unsafe_code)]

pub mod context;
pub mod error;
pub mod id;
pub mod permission;
pub mod scope;

pub use context::EffectiveIamContext;
pub use error::AuthzError;
pub use id::{
    BranchId, BusinessEntityId, InvitationId, MembershipId, OrganizationId, OwnershipTransferId,
    RegisterId, RoleId, ScopeGrantId, WarehouseId,
};
pub use permission::{
    Permission, ROLE_ACCOUNTING_USER, ROLE_AUDITOR, ROLE_BRANCH_MANAGER, ROLE_CASHIER,
    ROLE_INVENTORY_CLERK, ROLE_ORG_ADMIN, ROLE_OWNER, ROLE_PROCUREMENT_CLERK, ROLE_VIEWER,
    role_permissions,
};
pub use scope::Scope;
