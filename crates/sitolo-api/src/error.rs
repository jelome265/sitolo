//! Total mapping from semantic failures to RFC 9457-compatible safe problems.

use sitolo_authz::AuthzError;
use sitolo_observability::RequestId;
use sitolo_tenancy::TenancyError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retryability {
    NotRetryable,
    Retryable,
    RetryAfter(u64),
    UnknownOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorFamily {
    Validation,
    Authentication,
    Authorization,
    NotFound,
    Conflict,
    Idempotency,
    RateLimit,
    DependencyUnavailable,
    ExternalRejected,
    UnknownOutcome,
    Configuration,
    Secret,
    Telemetry,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicError {
    pub code: &'static str,
    pub status: u16,
    pub family: ErrorFamily,
    pub retryability: Retryability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Validation,
    Authentication,
    Authorization,
    NotFound,
    Conflict,
    IdempotencyConflict,
    RateLimited,
    DependencyUnavailable,
    UpstreamInvalidResponse,
    UpstreamTimeout,
    UnknownOutcome,
    Internal,

    // IAM Error variants (§30)
    OrganizationNotFound,
    OrganizationSuspended,
    OrganizationClosed,
    BranchNotFound,
    BranchClosed,
    MembershipNotFound,
    MembershipInactive,
    MembershipAlreadyExists,
    InvitationExpired,
    InvitationAlreadyAccepted,
    InvitationInvalid,
    RoleAssignmentForbidden,
    ScopeAssignmentForbidden,
    OwnershipTransferInvalidState,
    IamConcurrencyConflict,
}

impl AppError {
    pub fn public(&self) -> PublicError {
        match self {
            Self::Validation => PublicError {
                code: "VALIDATION_ERROR",
                status: 422,
                family: ErrorFamily::Validation,
                retryability: Retryability::NotRetryable,
            },
            Self::Authentication => PublicError {
                code: "AUTHENTICATION_FAILED",
                status: 401,
                family: ErrorFamily::Authentication,
                retryability: Retryability::NotRetryable,
            },
            Self::Authorization => PublicError {
                code: "AUTHORIZATION_DENIED",
                status: 403,
                family: ErrorFamily::Authorization,
                retryability: Retryability::NotRetryable,
            },
            Self::NotFound => PublicError {
                code: "RESOURCE_NOT_FOUND",
                status: 404,
                family: ErrorFamily::NotFound,
                retryability: Retryability::NotRetryable,
            },
            Self::Conflict => PublicError {
                code: "CONFLICT",
                status: 409,
                family: ErrorFamily::Conflict,
                retryability: Retryability::NotRetryable,
            },
            Self::IdempotencyConflict => PublicError {
                code: "IDEMPOTENCY_KEY_REUSED_WITH_DIFFERENT_REQUEST",
                status: 409,
                family: ErrorFamily::Idempotency,
                retryability: Retryability::NotRetryable,
            },
            Self::RateLimited => PublicError {
                code: "RATE_LIMITED",
                status: 429,
                family: ErrorFamily::RateLimit,
                retryability: Retryability::RetryAfter(1),
            },
            Self::DependencyUnavailable => PublicError {
                code: "DEPENDENCY_UNAVAILABLE",
                status: 503,
                family: ErrorFamily::DependencyUnavailable,
                retryability: Retryability::Retryable,
            },
            Self::UpstreamInvalidResponse => PublicError {
                code: "UPSTREAM_INVALID_RESPONSE",
                status: 502,
                family: ErrorFamily::ExternalRejected,
                retryability: Retryability::NotRetryable,
            },
            Self::UpstreamTimeout => PublicError {
                code: "PAYMENT_PROVIDER_UNAVAILABLE",
                status: 504,
                family: ErrorFamily::DependencyUnavailable,
                retryability: Retryability::UnknownOutcome,
            },
            Self::UnknownOutcome => PublicError {
                code: "PAYMENT_UNKNOWN_OUTCOME",
                status: 409,
                family: ErrorFamily::UnknownOutcome,
                retryability: Retryability::UnknownOutcome,
            },
            Self::Internal => PublicError {
                code: "INTERNAL_ERROR",
                status: 500,
                family: ErrorFamily::Internal,
                retryability: Retryability::NotRetryable,
            },

            // IAM mappings (§30)
            Self::OrganizationNotFound => PublicError {
                code: "ORGANIZATION_NOT_FOUND",
                status: 404,
                family: ErrorFamily::NotFound,
                retryability: Retryability::NotRetryable,
            },
            Self::OrganizationSuspended => PublicError {
                code: "ORGANIZATION_SUSPENDED",
                status: 403,
                family: ErrorFamily::Authorization,
                retryability: Retryability::NotRetryable,
            },
            Self::OrganizationClosed => PublicError {
                code: "ORGANIZATION_CLOSED",
                status: 403,
                family: ErrorFamily::Authorization,
                retryability: Retryability::NotRetryable,
            },
            Self::BranchNotFound => PublicError {
                code: "BRANCH_NOT_FOUND",
                status: 404,
                family: ErrorFamily::NotFound,
                retryability: Retryability::NotRetryable,
            },
            Self::BranchClosed => PublicError {
                code: "BRANCH_CLOSED",
                status: 400,
                family: ErrorFamily::Validation,
                retryability: Retryability::NotRetryable,
            },
            Self::MembershipNotFound => PublicError {
                code: "MEMBERSHIP_NOT_FOUND",
                status: 404,
                family: ErrorFamily::NotFound,
                retryability: Retryability::NotRetryable,
            },
            Self::MembershipInactive => PublicError {
                code: "MEMBERSHIP_INACTIVE",
                status: 403,
                family: ErrorFamily::Authorization,
                retryability: Retryability::NotRetryable,
            },
            Self::MembershipAlreadyExists => PublicError {
                code: "MEMBERSHIP_ALREADY_EXISTS",
                status: 409,
                family: ErrorFamily::Conflict,
                retryability: Retryability::NotRetryable,
            },
            Self::InvitationExpired => PublicError {
                code: "INVITATION_EXPIRED",
                status: 410,
                family: ErrorFamily::Validation,
                retryability: Retryability::NotRetryable,
            },
            Self::InvitationAlreadyAccepted => PublicError {
                code: "INVITATION_ALREADY_ACCEPTED",
                status: 409,
                family: ErrorFamily::Conflict,
                retryability: Retryability::NotRetryable,
            },
            Self::InvitationInvalid => PublicError {
                code: "INVITATION_INVALID",
                status: 400,
                family: ErrorFamily::Validation,
                retryability: Retryability::NotRetryable,
            },
            Self::RoleAssignmentForbidden => PublicError {
                code: "ROLE_ASSIGNMENT_FORBIDDEN",
                status: 403,
                family: ErrorFamily::Authorization,
                retryability: Retryability::NotRetryable,
            },
            Self::ScopeAssignmentForbidden => PublicError {
                code: "SCOPE_ASSIGNMENT_FORBIDDEN",
                status: 403,
                family: ErrorFamily::Authorization,
                retryability: Retryability::NotRetryable,
            },
            Self::OwnershipTransferInvalidState => PublicError {
                code: "OWNERSHIP_TRANSFER_INVALID_STATE",
                status: 409,
                family: ErrorFamily::Conflict,
                retryability: Retryability::NotRetryable,
            },
            Self::IamConcurrencyConflict => PublicError {
                code: "IAM_CONCURRENCY_CONFLICT",
                status: 409,
                family: ErrorFamily::Conflict,
                retryability: Retryability::Retryable,
            },
        }
    }
}

impl From<TenancyError> for AppError {
    fn from(err: TenancyError) -> Self {
        match err {
            TenancyError::InvalidIdentifier => AppError::Validation,
            TenancyError::OrganizationNotFound { .. } => AppError::OrganizationNotFound,
            TenancyError::OrganizationSuspended { .. } => AppError::OrganizationSuspended,
            TenancyError::OrganizationClosed { .. } => AppError::OrganizationClosed,
            TenancyError::BranchNotFound { .. } => AppError::BranchNotFound,
            TenancyError::BranchClosed { .. } => AppError::BranchClosed,
            TenancyError::MembershipNotFound { .. } => AppError::MembershipNotFound,
            TenancyError::MembershipInactive { .. } => AppError::MembershipInactive,
            TenancyError::MembershipAlreadyExists => AppError::MembershipAlreadyExists,
            TenancyError::InvitationNotFound { .. } => AppError::InvitationInvalid,
            TenancyError::InvitationExpired => AppError::InvitationExpired,
            TenancyError::InvitationAlreadyAccepted => AppError::InvitationAlreadyAccepted,
            TenancyError::RoleAssignmentForbidden => AppError::RoleAssignmentForbidden,
            TenancyError::ScopeAssignmentForbidden => AppError::ScopeAssignmentForbidden,
            TenancyError::OwnershipTransferInvalidState => AppError::OwnershipTransferInvalidState,
            TenancyError::ConcurrencyConflict => AppError::IamConcurrencyConflict,
            TenancyError::Unauthorized { .. } => AppError::Authorization,
            TenancyError::InvalidStatusTransition { .. } => AppError::Validation,
        }
    }
}

impl From<AuthzError> for AppError {
    fn from(err: AuthzError) -> Self {
        match err {
            AuthzError::InvalidIdentifier => AppError::Validation,
            AuthzError::PermissionDenied { .. } => AppError::Authorization,
            AuthzError::ScopeViolation => AppError::Authorization,
            AuthzError::InsufficientAssurance => AppError::Authorization,
            AuthzError::SecurityVersionMismatch => AppError::Authorization,
            AuthzError::RoleAssignmentForbidden => AppError::RoleAssignmentForbidden,
            AuthzError::ScopeOutsideAuthority => AppError::ScopeAssignmentForbidden,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemDetails {
    pub problem_type: String,
    pub title: &'static str,
    pub status: u16,
    pub code: &'static str,
    pub request_id: String,
}

impl ProblemDetails {
    pub fn from_error(error: &AppError, id: &RequestId) -> Self {
        let p = error.public();
        Self {
            problem_type: format!(
                "https://sitolo.example/problems/{}",
                p.code.to_ascii_lowercase()
            ),
            title: "Request could not be completed",
            status: p.status,
            code: p.code,
            request_id: id.as_str().into(),
        }
    }

    pub fn json(&self) -> String {
        format!(
            "{{\"type\":\"{}\",\"title\":\"{}\",\"status\":{},\"code\":\"{}\",\"request_id\":\"{}\"}}",
            self.problem_type, self.title, self.status, self.code, self.request_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_never_serializes_source() {
        let p = ProblemDetails::from_error(&AppError::Internal, &RequestId::new_server()).json();
        assert!(!p.contains("password"));
        assert!(!p.contains("stack"));
        assert!(p.contains("INTERNAL_ERROR"));
    }

    #[test]
    fn tenancy_error_maps_to_public_codes() {
        let err: AppError = TenancyError::RoleAssignmentForbidden.into();
        let pub_err = err.public();
        assert_eq!(pub_err.code, "ROLE_ASSIGNMENT_FORBIDDEN");
        assert_eq!(pub_err.status, 403);
    }
}
