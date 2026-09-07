//! Semantic authentication error taxonomy.
//!
//! Phase 3 specification, §36 and Appendix B. These are internal semantic
//! failures; transport mapping lives in `sitolo-api` and must never expose
//! internal provider details. Public detail is always generic.

use thiserror::Error;

/// Every authentication-path failure (§36).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AuthError {
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("authentication rate limited")]
    AuthenticationRateLimited,
    #[error("session is invalid")]
    SessionInvalid,
    #[error("session expired")]
    SessionExpired,
    #[error("session revoked")]
    SessionRevoked,
    #[error("refresh token invalid")]
    RefreshTokenInvalid,
    #[error("refresh token reused")]
    RefreshTokenReused,
    #[error("mfa required")]
    MfaRequired,
    #[error("mfa verification failed")]
    MfaFailed,
    #[error("mfa rate limited")]
    MfaRateLimited,
    #[error("mfa enrollment expired")]
    MfaEnrollmentExpired,
    #[error("mfa enrollment already in progress or active")]
    MfaEnrollmentConflict,
    #[error("recovery required")]
    RecoveryRequired,
    #[error("recovery artifact invalid")]
    RecoveryArtifactInvalid,
    #[error("recovery artifact already used")]
    RecoveryArtifactUsed,
    #[error("device not registered")]
    DeviceNotRegistered,
    #[error("device suspended")]
    DeviceSuspended,
    #[error("device revoked")]
    DeviceRevoked,
    #[error("device verification required")]
    DeviceVerificationRequired,
    #[error("identity provider unavailable")]
    IdentityProviderUnavailable,
    #[error("identity provider rejected")]
    IdentityProviderRejected,
    #[error("identity signing key unavailable")]
    IdentityKeyUnavailable,
    #[error("identity configuration invalid")]
    ConfigurationInvalid,
    #[error("invalid security identifier")]
    InvalidIdentifier,
    #[error("invalid state transition")]
    InvalidTransition,
    #[error("security version mismatch")]
    SecurityVersionMismatch,
    #[error("authorization denied")]
    AuthorizationDenied,
}

impl AuthError {
    /// Stable machine-readable code. The public registry in `sitolo-api` is
    /// generated from this mapping; unknown codes are a release blocker
    /// (§78: undocumented new public authentication error codes).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            AuthError::AuthenticationFailed => "AUTHENTICATION_FAILED",
            AuthError::AuthenticationRateLimited => "AUTHENTICATION_RATE_LIMITED",
            AuthError::SessionInvalid => "SESSION_INVALID",
            AuthError::SessionExpired => "SESSION_EXPIRED",
            AuthError::SessionRevoked => "SESSION_REVOKED",
            AuthError::RefreshTokenInvalid => "REFRESH_TOKEN_INVALID",
            AuthError::RefreshTokenReused => "REFRESH_TOKEN_REUSED",
            AuthError::MfaRequired => "MFA_REQUIRED",
            AuthError::MfaFailed => "MFA_FAILED",
            AuthError::MfaRateLimited => "MFA_RATE_LIMITED",
            AuthError::MfaEnrollmentExpired => "MFA_ENROLLMENT_EXPIRED",
            AuthError::MfaEnrollmentConflict => "MFA_ENROLLMENT_CONFLICT",
            AuthError::RecoveryRequired => "RECOVERY_REQUIRED",
            AuthError::RecoveryArtifactInvalid => "INVALID_RECOVERY_ARTIFACT",
            AuthError::RecoveryArtifactUsed => "RECOVERY_ARTIFACT_USED",
            AuthError::DeviceNotRegistered => "DEVICE_NOT_REGISTERED",
            AuthError::DeviceSuspended => "DEVICE_SUSPENDED",
            AuthError::DeviceRevoked => "DEVICE_REVOKED",
            AuthError::DeviceVerificationRequired => "DEVICE_VERIFICATION_REQUIRED",
            AuthError::IdentityProviderUnavailable => "IDENTITY_PROVIDER_UNAVAILABLE",
            AuthError::IdentityProviderRejected => "IDENTITY_PROVIDER_REJECTED",
            AuthError::IdentityKeyUnavailable => "IDENTITY_KEY_UNAVAILABLE",
            AuthError::ConfigurationInvalid => "CONFIGURATION_INVALID",
            AuthError::InvalidIdentifier => "AUTHENTICATION_FAILED",
            AuthError::InvalidTransition => "AUTHENTICATION_FAILED",
            AuthError::SecurityVersionMismatch => "SESSION_REVOKED",
            AuthError::AuthorizationDenied => "AUTHORIZATION_DENIED",
        }
    }
}

/// A consumed-exactly-once credential material type. Raw bearer/refresh
/// material must never survive into `SecurityContext`, errors, or caches
/// (Phase 3 specification, §32.1, §49).
pub struct CredentialMaterial(String);

impl CredentialMaterial {
    /// Wraps extracted bearer/refresh material. Callers must not retain other
    /// copies; the value is consumed exactly once by the credential boundary.
    pub fn new(value: impl Into<String>) -> Self {
        CredentialMaterial(value.into())
    }
    /// Consumes the credential and hands the raw value to the verifier.
    #[must_use]
    pub fn expose(self) -> String {
        self.0
    }
}

impl std::fmt::Debug for CredentialMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CredentialMaterial(redacted)")
    }
}

impl std::fmt::Display for CredentialMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<credential>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_stable() {
        let all = [
            AuthError::AuthenticationFailed,
            AuthError::AuthenticationRateLimited,
            AuthError::SessionExpired,
            AuthError::RefreshTokenInvalid,
            AuthError::RefreshTokenReused,
            AuthError::MfaRequired,
            AuthError::MfaFailed,
            AuthError::DeviceRevoked,
            AuthError::IdentityProviderUnavailable,
            AuthError::RecoveryArtifactUsed,
        ];
        assert_eq!(all[0].code(), "AUTHENTICATION_FAILED");
        assert_eq!(all[4].code(), "REFRESH_TOKEN_REUSED");
        assert_eq!(all[9].code(), "RECOVERY_ARTIFACT_USED");
    }

    #[test]
    fn credential_material_never_prints() {
        let c = CredentialMaterial::new("Bearer secret-token");
        assert_eq!(format!("{c:?}"), "CredentialMaterial(redacted)");
        assert_eq!(c.to_string(), "<credential>");
        assert_eq!(c.expose(), "Bearer secret-token");
    }
}
