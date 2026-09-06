//! Total mapping from semantic failures to RFC 9457-compatible safe problems.
use sitolo_observability::RequestId;
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
            "{{\\\"type\\\":\\\"{}\\\",\\\"title\\\":\\\"{}\\\",\\\"status\\\":{},\\\"code\\\":\\\"{}\\\",\\\"request_id\\\":\\\"{}\\\"}}",
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
    fn unknown_outcome_is_not_retryable() {
        assert_eq!(
            AppError::UnknownOutcome.public().retryability,
            Retryability::UnknownOutcome
        );
    }
}
