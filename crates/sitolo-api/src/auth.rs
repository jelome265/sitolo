//! Authentication transport boundary.
//!
//! Phase 3 specification, §36, §46-§47, §50, Appendix B-C. This module owns
//! the HTTP-facing authentication pipeline in the order mandated by §50:
//!
//! ```text
//! request size / parser limits
//!   -> rate / concurrency controls (enforced by caller with 429)
//!   -> credential extraction
//!   -> cryptographic validation (delegated to sitolo-auth token validation)
//!   -> session state
//!   -> device state
//!   -> principal construction
//!   -> next-stage tenant/authz (Phase 4/6, not invented here)
//! ```
//!
//! It deliberately does NOT invent tenant permissions (§69, §86). It maps
//! semantic [`sitolo_auth::AuthError`] values to stable public [`AppError`]
//! problems with generic detail (§36.1, Appendix B) so transport responses
//! never leak provider internals, tokens, or password material.

use std::time::SystemTime;

use sitolo_auth::{AuthError, CredentialMaterial, Device, SecurityContext, Session};
use sitolo_observability::{RequestId, TraceParent};

use crate::error::AppError;

/// Bounded authentication transport limits (§47).
///
/// Every externally reachable authentication path must be bounded before
/// expensive work (password hashing, signature verification). These are
/// structural ceilings, not product policy.
pub mod bounds {
    /// Maximum HTTP body accepted on authentication routes.
    pub const MAX_AUTH_BODY_BYTES: usize = 32 * 1024;
    /// Maximum identifier (username / user id) length.
    pub const MAX_IDENTIFIER_LEN: usize = 128;
    /// Maximum password length checked before hashing (§47).
    pub const MAX_PASSWORD_LEN: usize = 512;
    /// Maximum OTP / TOTP code length.
    pub const MAX_OTP_LEN: usize = 16;
    /// Maximum recovery-code length.
    pub const MAX_RECOVERY_CODE_LEN: usize = 64;
    /// Maximum raw bearer / refresh credential length.
    pub const MAX_CREDENTIAL_LEN: usize = 4096;
    /// Maximum OAuth callback parameter length.
    pub const MAX_CALLBACK_PARAM_LEN: usize = 2048;
}

/// Validates bounded authentication inputs before expensive work (§47).
///
/// Cheap structural rejection runs first; account-enumeration defenses (§22)
/// are preserved by mapping all credential failures to the same generic
/// [`AppError::Authentication`].
pub fn validate_login_input(identifier: &str, password_len: usize) -> Result<(), AppError> {
    if identifier.is_empty()
        || identifier.len() > bounds::MAX_IDENTIFIER_LEN
        || password_len > bounds::MAX_PASSWORD_LEN
    {
        return Err(AppError::Authentication);
    }
    Ok(())
}

/// Validates a one-time code shape before verification (§47).
pub fn validate_otp_input(code: &str) -> Result<(), AppError> {
    if code.is_empty() || code.len() > bounds::MAX_OTP_LEN {
        return Err(AppError::Authentication);
    }
    Ok(())
}

/// Extracts a bearer credential without logging it (§32.1, §40, §50).
///
/// The returned [`CredentialMaterial`] is consumed exactly once by the
/// verifier; `Debug`/`Display` never render the raw value. Missing,
/// malformed, or overlong headers map to generic authentication failure —
/// never to a distinct enumerable signal beyond 401.
pub fn extract_bearer(authorization: Option<&str>) -> Result<CredentialMaterial, AppError> {
    let header = authorization.ok_or(AppError::Authentication)?;
    if header.len() > bounds::MAX_CREDENTIAL_LEN + 7 {
        return Err(AppError::Authentication);
    }
    let raw = header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Authentication)?;
    if raw.is_empty() || raw.len() > bounds::MAX_CREDENTIAL_LEN {
        return Err(AppError::Authentication);
    }
    // Bearer material is opaque; only structural validation here.
    // Cryptographic validation happens in sitolo-auth token validation.
    if !raw
        .bytes()
        .all(|b| b.is_ascii_graphic() || b == b'-' || b == b'_' || b == b'.')
    {
        return Err(AppError::Authentication);
    }
    Ok(CredentialMaterial::new(raw.to_string()))
}

/// Maps semantic authentication failures to stable public errors
/// (Appendix B). Public detail stays generic; retryability follows the
/// taxonomy: rate limits are retry-after, dependencies are retryable or
/// unknown-outcome, everything credential-related is not retryable.
#[must_use]
pub fn map_auth_error(err: &AuthError) -> AppError {
    match err {
        AuthError::AuthenticationFailed
        | AuthError::SessionInvalid
        | AuthError::SessionExpired
        | AuthError::SessionRevoked
        | AuthError::RefreshTokenInvalid
        | AuthError::RefreshTokenReused
        | AuthError::MfaRequired
        | AuthError::MfaFailed
        | AuthError::MfaEnrollmentExpired
        | AuthError::RecoveryRequired
        | AuthError::RecoveryArtifactInvalid
        | AuthError::DeviceNotRegistered
        | AuthError::DeviceSuspended
        | AuthError::DeviceRevoked
        | AuthError::DeviceVerificationRequired
        | AuthError::IdentityProviderRejected
        | AuthError::InvalidIdentifier
        | AuthError::InvalidTransition
        | AuthError::SecurityVersionMismatch => AppError::Authentication,
        AuthError::AuthenticationRateLimited | AuthError::MfaRateLimited => AppError::RateLimited,
        AuthError::MfaEnrollmentConflict | AuthError::RecoveryArtifactUsed => AppError::Conflict,
        AuthError::IdentityProviderUnavailable | AuthError::IdentityKeyUnavailable => {
            AppError::DependencyUnavailable
        }
        AuthError::ConfigurationInvalid => AppError::Internal,
        AuthError::AuthorizationDenied => AppError::Authorization,
    }
}

/// Establishes a [`SecurityContext`] from already-loaded session and device
/// snapshots (§32, §50, §69).
///
/// Callers (application layer) load the authoritative records; this function
/// enforces the pipeline order and fail-closed device binding:
///
/// - session validity first (version, expiry, revocation via `accept`);
/// - device binding second: when `session.device_id` is bound, `device`
///   MUST be present and ACTIVE — a missing record fails closed with
///   `DeviceNotRegistered` rather than silently dropping the binding;
/// - a device identifier alone never authenticates (§50 wrong examples).
///
/// Raw credentials must not reach this function (§32.1, §49).
pub fn establish_context(
    session: &mut Session,
    device: Option<&Device>,
    user_security_version: sitolo_auth::SecurityVersion,
    lifetime: sitolo_auth::SessionLifetime,
    now: SystemTime,
    request_id: RequestId,
    trace: Option<TraceParent>,
) -> Result<SecurityContext, AppError> {
    if session.device_id.is_some() && device.is_none() {
        return Err(map_auth_error(&AuthError::DeviceNotRegistered));
    }
    SecurityContext::establish(
        session,
        device,
        user_security_version,
        lifetime,
        now,
        request_id,
        trace,
    )
    .map_err(|e| map_auth_error(&e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use sitolo_auth::{
        Assurance, AuthenticationMethod, SecurityVersion, SessionClass, SessionId, SessionLifetime,
        UserId,
    };

    fn lifetime() -> SessionLifetime {
        SessionLifetime {
            idle: Duration::from_secs(30),
            absolute: Duration::from_secs(60),
        }
    }

    fn session() -> Session {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        Session::create(
            SessionId::new("sess-1").unwrap(),
            UserId::new("user-1").unwrap(),
            None,
            SessionClass::Merchant,
            AuthenticationMethod::Password,
            Assurance::A1,
            SecurityVersion(1),
            lifetime(),
            now,
        )
    }

    #[test]
    fn bearer_extraction_rejects_missing_and_malformed() {
        assert!(extract_bearer(None).is_err());
        assert!(extract_bearer(Some("Basic abc")).is_err());
        assert!(extract_bearer(Some("Bearer ")).is_err());
        assert!(extract_bearer(Some("Bearer ok-token_01.02")).is_ok());
        // Control characters and spaces are rejected.
        assert!(extract_bearer(Some("Bearer has space")).is_err());
        assert!(extract_bearer(Some("Bearer inject\r\nline")).is_err());
    }

    #[test]
    fn bearer_material_never_renders() {
        let cred = extract_bearer(Some("Bearer secret-token")).unwrap();
        assert_eq!(format!("{cred:?}"), "CredentialMaterial(redacted)");
        assert_eq!(cred.to_string(), "<credential>");
    }

    #[test]
    fn error_mapping_is_generic_and_bounded() {
        // Credential failures collapse to generic 401 without internal detail.
        for err in [
            AuthError::AuthenticationFailed,
            AuthError::SessionRevoked,
            AuthError::RefreshTokenReused,
            AuthError::DeviceRevoked,
            AuthError::SecurityVersionMismatch,
            AuthError::InvalidIdentifier,
        ] {
            let public = map_auth_error(&err).public();
            assert_eq!(public.status, 401, "unexpected status for {err:?}");
            assert_eq!(public.code, "AUTHENTICATION_FAILED");
        }
        assert_eq!(
            map_auth_error(&AuthError::AuthenticationRateLimited)
                .public()
                .status,
            429
        );
        assert_eq!(
            map_auth_error(&AuthError::IdentityProviderUnavailable)
                .public()
                .status,
            503
        );
        assert_eq!(
            map_auth_error(&AuthError::RecoveryArtifactUsed)
                .public()
                .status,
            409
        );
    }

    #[test]
    fn bound_device_requires_device_record() {
        use sitolo_auth::{ClientPlatform, Device, DeviceId};
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let device_id = DeviceId::new("dev-1").unwrap();
        let mut sess = Session::create(
            SessionId::new("sess-2").unwrap(),
            UserId::new("user-1").unwrap(),
            Some(device_id),
            SessionClass::Merchant,
            AuthenticationMethod::Password,
            Assurance::A1,
            SecurityVersion(1),
            lifetime(),
            now,
        );
        let request_id = RequestId::new_server();
        // Omitting the bound device fails closed — the device id alone or its
        // absence never authenticates (§50).
        assert!(
            establish_context(
                &mut sess,
                None,
                SecurityVersion(1),
                lifetime(),
                now,
                request_id,
                None,
            )
            .is_err()
        );
        // An unbound session establishes normally.
        let mut free = session();
        let request_id = RequestId::new_server();
        let ctx = establish_context(
            &mut free,
            None,
            SecurityVersion(1),
            lifetime(),
            now,
            request_id,
            None,
        )
        .unwrap();
        assert_eq!(ctx.assurance, Assurance::A1);
        let _ = Device::begin_registration(
            DeviceId::new("dev-x").unwrap(),
            UserId::new("user-1").unwrap(),
            ClientPlatform::Android,
            SecurityVersion(1),
            now,
        );
    }

    #[test]
    fn login_input_bounds_reject_overlong_before_hashing() {
        let long_id = "u".repeat(bounds::MAX_IDENTIFIER_LEN + 1);
        assert!(validate_login_input(&long_id, 6).is_err());
        assert!(validate_login_input("", 6).is_err());
        assert!(validate_login_input("user-1", bounds::MAX_PASSWORD_LEN + 1).is_err());
        assert!(validate_login_input("user-1", 8).is_ok());
        assert!(validate_otp_input("").is_err());
        assert!(validate_otp_input(&"1".repeat(bounds::MAX_OTP_LEN + 1)).is_err());
    }
}
