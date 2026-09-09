//! Authorization errors.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AuthzError {
    #[error("invalid identifier format")]
    InvalidIdentifier,

    #[error("authorization denied: missing required permission {permission:?}")]
    PermissionDenied { permission: &'static str },

    #[error("authorization denied: scope restriction violated")]
    ScopeViolation,

    #[error("authorization denied: insufficient assurance level")]
    InsufficientAssurance,

    #[error("authorization denied: security version mismatch")]
    SecurityVersionMismatch,

    #[error("role assignment forbidden: cannot grant role higher than actor authority")]
    RoleAssignmentForbidden,

    #[error("scope grant forbidden: scope is outside actor authority")]
    ScopeOutsideAuthority,
}
