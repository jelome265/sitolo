//! Authorization.
//!
//! Owns role catalogs, permission catalogs, assignment lifecycle, permission
//! resolution, scope evaluation, resource authorization, and step-up
//! requirements (Phase 1 specification, §5.5; Phase 4 specification,
//! sections 10 and 19). Resolution here is the role layer only: scope
//! grants (PR-004) and the policy engine (Phase 6) complete the decision.
#![forbid(unsafe_code)]

mod roles;
mod scopes;

pub use roles::{
    AssignmentError, Permission, ROLE_DEFINITION_VERSION, Role, RoleAssignment,
    RoleAssignmentState, resolve_permissions,
};
pub use scopes::{GrantError, Scope, ScopeGrant, ScopeGrantState, authorize_scope};
