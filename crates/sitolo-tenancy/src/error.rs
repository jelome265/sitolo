//! Tenancy domain and lifecycle errors.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TenancyError {
    #[error("invalid identifier")]
    InvalidIdentifier,

    #[error("organization not found: {id}")]
    OrganizationNotFound { id: String },

    #[error("organization is suspended: {id}")]
    OrganizationSuspended { id: String },

    #[error("organization is closed: {id}")]
    OrganizationClosed { id: String },

    #[error("branch not found: {id}")]
    BranchNotFound { id: String },

    #[error("branch is closed: {id}")]
    BranchClosed { id: String },

    #[error("membership not found: {id}")]
    MembershipNotFound { id: String },

    #[error("membership is inactive or revoked: {id}")]
    MembershipInactive { id: String },

    #[error("membership already exists for user in organization")]
    MembershipAlreadyExists,

    #[error("invitation not found or invalid: {id}")]
    InvitationNotFound { id: String },

    #[error("invitation expired")]
    InvitationExpired,

    #[error("invitation already accepted")]
    InvitationAlreadyAccepted,

    #[error("role assignment forbidden")]
    RoleAssignmentForbidden,

    #[error("scope grant forbidden")]
    ScopeAssignmentForbidden,

    #[error("ownership transfer invalid state transition")]
    OwnershipTransferInvalidState,

    #[error("concurrency conflict during tenancy transition")]
    ConcurrencyConflict,

    #[error("unauthorized tenancy operation: {reason}")]
    Unauthorized { reason: &'static str },

    #[error("invalid status transition from {from:?} to {to:?}")]
    InvalidStatusTransition { from: String, to: String },
}
