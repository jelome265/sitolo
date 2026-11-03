//! Total mapping from semantic failures to RFC 9457-compatible safe problems.

use sitolo_observability::RequestId;

/// Whether a caller may repeat the request, and after what delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retryability {
    /// Repeating the same request cannot succeed.
    NotRetryable,
    /// The same request may be retried immediately.
    Retryable,
    /// The request may be retried after the given number of seconds.
    RetryAfter(u64),
    /// The outcome is unknown; retrying risks a duplicate effect.
    UnknownOutcome,
}

/// Stable problem family used for metrics and client branching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorFamily {
    /// The request was malformed or failed transport validation.
    Validation,
    /// The caller could not be authenticated.
    Authentication,
    /// The caller is authenticated but not permitted.
    Authorization,
    /// The addressed resource is absent or hidden.
    NotFound,
    /// The request conflicts with current state.
    Conflict,
    /// An idempotency key was reused with a different request.
    Idempotency,
    /// A rate limit was exceeded.
    RateLimit,
    /// A required dependency is unavailable.
    DependencyUnavailable,
    /// An upstream returned a response the caller must not accept.
    ExternalRejected,
    /// The effect of an upstream call is unknown.
    UnknownOutcome,
    /// The service is misconfigured.
    Configuration,
    /// A secret could not be resolved.
    Secret,
    /// Telemetry could not be emitted.
    Telemetry,
    /// An unexpected internal failure occurred.
    Internal,
}

/// Public, source-free description of one failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicError {
    /// Stable machine-readable problem code.
    pub code: &'static str,
    /// HTTP status code.
    pub status: u16,
    /// Problem family for metrics and client branching.
    pub family: ErrorFamily,
    /// Whether and when the caller may retry.
    pub retryability: Retryability,
}

/// Semantic application failure, before it is mapped to a public problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// The request failed validation.
    Validation,
    /// Authentication is missing or invalid.
    Authentication,
    /// The caller is not permitted to perform the operation.
    Authorization,
    /// The addressed resource is absent or hidden.
    NotFound,
    /// The request conflicts with current state.
    Conflict,
    /// An idempotency key was reused with a different request.
    IdempotencyConflict,
    /// A rate limit was exceeded.
    RateLimited,
    /// A required dependency is unavailable.
    DependencyUnavailable,
    /// An upstream returned an unusable response.
    UpstreamInvalidResponse,
    /// An upstream call timed out.
    UpstreamTimeout,
    /// The effect of an upstream call is unknown.
    UnknownOutcome,
    /// An unexpected internal failure occurred.
    Internal,
}

impl AppError {
    /// Maps this failure to its public, source-free representation.
    #[must_use]
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
/// RFC 9457-compatible problem document safe to return to a caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemDetails {
    /// Absolute problem type URI.
    pub problem_type: String,
    /// Human-readable summary that discloses no source detail.
    pub title: &'static str,
    /// HTTP status code.
    pub status: u16,
    /// Stable machine-readable problem code.
    pub code: &'static str,
    /// Correlation identifier for the failing request.
    pub request_id: String,
}

impl ProblemDetails {
    /// Builds the public problem document for `error` and the request's
    /// correlation id.
    #[must_use]
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
    /// Serializes the problem document to its JSON wire form.
    #[must_use]
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
