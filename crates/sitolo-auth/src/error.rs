//! Semantic authentication error taxonomy.
//!
//! Phase 3 specification, §36 and Appendix B. These are internal semantic
//! failures; transport mapping lives in `sitolo-api` and must never expose
//! internal provider details. Public detail is always generic.

use thiserror::Error;

/// Every authentication-path failure (§36).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AuthError {
    /// Credential verification failed. Public detail is always generic so the
    /// failure cannot be attributed to a known account (§22).
    #[error("authentication failed")]
    AuthenticationFailed,
    /// Too many authentication attempts for the subject or source (§23).
    #[error("authentication rate limited")]
    AuthenticationRateLimited,
    /// The session does not exist or is not owned by the expected user (§10).
    #[error("session is invalid")]
    SessionInvalid,
    /// The session passed its idle or absolute lifetime (§11).
    #[error("session expired")]
    SessionExpired,
    /// The session was explicitly revoked (§10.3).
    #[error("session revoked")]
    SessionRevoked,
    /// The refresh credential is unknown, expired, or malformed (§12).
    #[error("refresh token invalid")]
    RefreshTokenInvalid,
    /// An already-consumed refresh credential was presented; the family is
    /// compromised (§12.2).
    #[error("refresh token reused")]
    RefreshTokenReused,
    /// A second factor is required before the login can complete (§17).
    #[error("mfa required")]
    MfaRequired,
    /// The presented MFA proof was rejected, including TOTP replay (§18).
    #[error("mfa verification failed")]
    MfaFailed,
    /// Too many MFA verification attempts for the subject or source (§23).
    #[error("mfa rate limited")]
    MfaRateLimited,
    /// The MFA enrollment or challenge window elapsed (§17.2, §19.1).
    #[error("mfa enrollment expired")]
    MfaEnrollmentExpired,
    /// An authenticator is already active or an enrollment is already in
    /// progress for the user (§17.2).
    #[error("mfa enrollment already in progress or active")]
    MfaEnrollmentConflict,
    /// Recovery is required before the account can authenticate (§20).
    #[error("recovery required")]
    RecoveryRequired,
    /// A recovery or reset artifact was unknown, expired, or bound to a stale
    /// security version (§20, §21).
    #[error("recovery artifact invalid")]
    RecoveryArtifactInvalid,
    /// A recovery or reset artifact had already been consumed (§20, §21.2).
    #[error("recovery artifact already used")]
    RecoveryArtifactUsed,
    /// The device has no registration record (§24).
    #[error("device not registered")]
    DeviceNotRegistered,
    /// The device is suspended and cannot participate in authenticated
    /// operations until reinstated (§27).
    #[error("device suspended")]
    DeviceSuspended,
    /// The device is revoked and can never return to service (§27).
    #[error("device revoked")]
    DeviceRevoked,
    /// The device must complete verification before it can be used (§24.1).
    #[error("device verification required")]
    DeviceVerificationRequired,
    /// The identity provider could not be reached or is not ready (§6.2, §65).
    #[error("identity provider unavailable")]
    IdentityProviderUnavailable,
    /// The identity provider rejected the assertion or code exchange (§65).
    #[error("identity provider rejected")]
    IdentityProviderRejected,
    /// The signing key needed to verify an assertion is unavailable (§14.1).
    #[error("identity signing key unavailable")]
    IdentityKeyUnavailable,
    /// Security configuration is impossible or internally inconsistent (§64).
    #[error("identity configuration invalid")]
    ConfigurationInvalid,
    /// A security identifier failed shape validation (§10.1).
    #[error("invalid security identifier")]
    InvalidIdentifier,
    /// The requested state transition is not permitted by the state machine
    /// (Appendix A).
    #[error("invalid state transition")]
    InvalidTransition,
    /// The presented security version does not match authoritative state
    /// (§31).
    #[error("security version mismatch")]
    SecurityVersionMismatch,
    /// Authorization policy denied the operation (§69).
    #[error("authorization denied")]
    AuthorizationDenied,
    /// Durable session authority could not be read (§32).
    #[error("identity authority unavailable")]
    IdentityAuthorityUnavailable,
}

impl AuthError {
    /// Stable machine-readable code. The public registry in `sitolo-api` is
    /// generated from this mapping; unknown codes are a release blocker
    /// (§78: undocumented new public authentication error codes).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            AuthError::AuthenticationFailed
            | AuthError::InvalidIdentifier
            | AuthError::InvalidTransition => "AUTHENTICATION_FAILED",
            AuthError::AuthenticationRateLimited => "AUTHENTICATION_RATE_LIMITED",
            AuthError::SessionInvalid => "SESSION_INVALID",
            AuthError::SessionExpired => "SESSION_EXPIRED",
            AuthError::SessionRevoked | AuthError::SecurityVersionMismatch => "SESSION_REVOKED",
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
            AuthError::AuthorizationDenied => "AUTHORIZATION_DENIED",
            AuthError::IdentityAuthorityUnavailable => "IDENTITY_AUTHORITY_UNAVAILABLE",
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
            AuthError::IdentityAuthorityUnavailable,
            AuthError::RecoveryArtifactUsed,
        ];
        assert_eq!(all[0].code(), "AUTHENTICATION_FAILED");
        assert_eq!(all[4].code(), "REFRESH_TOKEN_REUSED");
        assert_eq!(all[9].code(), "IDENTITY_AUTHORITY_UNAVAILABLE");
        assert_eq!(all[10].code(), "RECOVERY_ARTIFACT_USED");
    }

    #[test]
    fn credential_material_never_prints() {
        let c = CredentialMaterial::new("Bearer secret-token");
        assert_eq!(format!("{c:?}"), "CredentialMaterial(redacted)");
        assert_eq!(c.to_string(), "<credential>");
        assert_eq!(c.expose(), "Bearer secret-token");
    }
}
