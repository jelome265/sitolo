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

use sitolo_auth::{
    AuthError, CredentialMaterial, Device, IdentityProviderPort, RequestIdentityAuthority,
    SecurityContext, Session, SessionId,
};
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
///
/// # Errors
/// Returns [`AppError::Authentication`] when the identifier is empty or over
/// [`bounds::MAX_IDENTIFIER_LEN`], or when the password exceeds
/// [`bounds::MAX_PASSWORD_LEN`].
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
///
/// # Errors
/// Returns [`AppError::Authentication`] when the code is empty or longer than
/// [`bounds::MAX_OTP_LEN`].
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
///
/// # Errors
/// Returns [`AppError::Authentication`] when the header is absent, is not a
/// `Bearer` header, is empty, exceeds [`bounds::MAX_CREDENTIAL_LEN`], or
/// carries a non-opaque character.
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
        AuthError::IdentityAuthorityUnavailable => AppError::DependencyUnavailable,
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
///
/// # Errors
/// Returns the mapped public error when the session is not valid, when a bound
/// device record is missing or inactive, or when the device fails the
/// established session's binding checks.
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

/// Validates the upstream bearer token, resolves only a pre-linked exact
/// `(issuer, subject)` pair, and accepts a server-side session belonging to
/// that mapped user. The session selector is an index only; it never selects
/// the principal. Tenant, role, and permission resolution remain a separate
/// authorization step and must run before protected commands.
///
/// # Errors
/// Returns [`AppError::Authentication`] when the session selector or bearer
/// credential is absent or malformed, or the session cannot be established.
/// Returns [`AppError::DependencyUnavailable`] when the identity provider or
/// identity authority is unavailable.
pub async fn authenticate_request(
    authorization: Option<&str>,
    session_selector: Option<&str>,
    identity_provider: &dyn IdentityProviderPort,
    identity_stores: &dyn RequestIdentityAuthority,
    now: SystemTime,
    request_id: RequestId,
    trace: Option<TraceParent>,
) -> Result<SecurityContext, AppError> {
    // Reject malformed selectors before performing provider network work.
    let session_selector = session_selector.ok_or(AppError::Authentication)?;
    if session_selector.len() > bounds::MAX_IDENTIFIER_LEN {
        return Err(AppError::Authentication);
    }
    let session_id =
        SessionId::new(session_selector.to_owned()).map_err(|error| map_auth_error(&error))?;
    let credential = extract_bearer(authorization)?;
    let raw_assertion = credential.expose();
    let external_identity = identity_provider
        .validate_assertion(&raw_assertion)
        .await
        .map_err(|error| map_auth_error(&error))?;
    drop(raw_assertion);

    // An unknown provider identity is not auto-provisioned or linked by email.
    let user_id = identity_stores
        .resolve_external_identity(&external_identity.issuer, &external_identity.subject)
        .await
        .map_err(|error| map_auth_error(&error))?
        .ok_or(AppError::Authentication)?;
    let mut session_snapshot = identity_stores
        .accept_session_for_user(&session_id, &user_id, now)
        .await
        .map_err(|error| map_auth_error(&error))?;
    let device = match session_snapshot.session.device_id.as_ref() {
        Some(device_id) => Some(
            identity_stores
                .device_snapshot(device_id, &user_id)
                .await
                .map_err(|error| map_auth_error(&error))?
                .ok_or_else(|| map_auth_error(&AuthError::DeviceNotRegistered))?,
        ),
        None => None,
    };
    let accepted_lifetime = session_lifetime_for_accepted(&session_snapshot.session);
    establish_context(
        &mut session_snapshot.session,
        device.as_ref(),
        session_snapshot.user_security_version,
        // A session's configured policy is applied by the authoritative store
        // when it is accepted. This value is used only for the in-memory
        // context consistency check after that operation.
        accepted_lifetime,
        now,
        request_id,
        trace,
    )
}

fn session_lifetime_for_accepted(session: &Session) -> sitolo_auth::SessionLifetime {
    let absolute = session
        .absolute_expires_at
        .duration_since(session.created_at)
        .unwrap_or_default();
    let idle = session
        .idle_expires_at
        .duration_since(session.last_seen_at)
        .unwrap_or_default();
    sitolo_auth::SessionLifetime { idle, absolute }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::time::Duration;

    use sitolo_auth::{
        Assurance, AuthenticationMethod, SecurityVersion, SessionClass, SessionId, SessionLifetime,
        SessionPolicySet, TestIdentityProvider, UserId,
    };
    use sitolo_persistence::{IdentityDatabase, IdentityStores};

    fn lifetime() -> SessionLifetime {
        SessionLifetime {
            idle: Duration::from_secs(30),
            absolute: Duration::from_secs(60),
        }
    }

    /// A fixed, non-zero timestamp for deterministic session tests.
    fn test_now() -> Result<SystemTime, Box<dyn Error>> {
        match SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(1_000)) {
            Some(now) => Ok(now),
            None => Err("fixed test timestamp overflowed".into()),
        }
    }

    fn session() -> Result<Session, Box<dyn Error>> {
        let now = test_now()?;
        Ok(Session::create(
            SessionId::new("sess-1")?,
            UserId::new("user-1")?,
            None,
            SessionClass::Merchant,
            AuthenticationMethod::Password,
            Assurance::A1,
            SecurityVersion(1),
            lifetime(),
            now,
        ))
    }

    fn identity_database() -> IdentityDatabase {
        let lifetime = lifetime();
        IdentityDatabase::new(
            SessionPolicySet {
                merchant: lifetime,
                administrator: lifetime,
                break_glass: lifetime,
            },
            Duration::from_secs(300),
        )
    }

    async fn user_with_session(
        database: &IdentityDatabase,
        user_id: &str,
        bind_provider_identity: bool,
    ) -> Result<SessionId, Box<dyn Error>> {
        let user_id = UserId::new(user_id)?;
        database.create_user(user_id.clone(), None, None).await?;
        if bind_provider_identity {
            database
                .bind_external_identity("test-issuer".into(), "subject-1".into(), &user_id)
                .await?;
        }
        let created = database
            .establish_session(
                user_id,
                None,
                SessionClass::Merchant,
                AuthenticationMethod::OidcAssertion,
                sitolo_auth::ClientPlatform::Android,
                Assurance::A1,
                SystemTime::now(),
            )
            .await?;
        Ok(created.session.id)
    }

    #[tokio::test]
    async fn verified_identity_and_owned_active_session_establish_context()
    -> Result<(), Box<dyn Error>> {
        let database = identity_database();
        let session_id = user_with_session(&database, "user-1", true).await?;
        let provider = TestIdentityProvider::new("subject-1", Assurance::A1);
        let context = authenticate_request(
            Some("Bearer signed-token"),
            Some(session_id.as_str()),
            &provider,
            &database,
            SystemTime::now(),
            RequestId::new_server(),
            None,
        )
        .await
        .map_err(|error| std::io::Error::other(format!("authentication failed: {error:?}")))?;
        assert_eq!(context.principal.subject_id.as_str(), "user-1");
        assert_eq!(context.principal.session_id, session_id);
        Ok(())
    }

    #[tokio::test]
    async fn session_selector_cannot_cross_the_mapped_user_boundary() -> Result<(), Box<dyn Error>>
    {
        let database = identity_database();
        let _mapped_user_session = user_with_session(&database, "user-1", true).await?;
        let other_user_session = user_with_session(&database, "user-2", false).await?;
        let provider = TestIdentityProvider::new("subject-1", Assurance::A1);
        let result = authenticate_request(
            Some("Bearer signed-token"),
            Some(other_user_session.as_str()),
            &provider,
            &database,
            SystemTime::now(),
            RequestId::new_server(),
            None,
        )
        .await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn unknown_external_identity_is_not_auto_linked() -> Result<(), Box<dyn Error>> {
        let database = identity_database();
        let session_id = user_with_session(&database, "user-1", false).await?;
        let provider = TestIdentityProvider::new("subject-1", Assurance::A1);
        let result = authenticate_request(
            Some("Bearer signed-token"),
            Some(session_id.as_str()),
            &provider,
            &database,
            SystemTime::now(),
            RequestId::new_server(),
            None,
        )
        .await;
        assert!(matches!(result, Err(AppError::Authentication)));
        Ok(())
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
    fn bearer_material_never_renders() -> Result<(), Box<dyn Error>> {
        let cred = extract_bearer(Some("Bearer secret-token")).map_err(|error| {
            std::io::Error::other(format!("bearer extraction failed: {error:?}"))
        })?;
        assert_eq!(format!("{cred:?}"), "CredentialMaterial(redacted)");
        assert_eq!(cred.to_string(), "<credential>");
        Ok(())
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
            map_auth_error(&AuthError::IdentityAuthorityUnavailable)
                .public()
                .code,
            "DEPENDENCY_UNAVAILABLE"
        );
        assert_eq!(
            map_auth_error(&AuthError::RecoveryArtifactUsed)
                .public()
                .status,
            409
        );
    }

    #[test]
    fn bound_device_requires_device_record() -> Result<(), Box<dyn Error>> {
        use sitolo_auth::DeviceId;
        let now = test_now()?;
        let device_id = DeviceId::new("dev-1")?;
        let mut sess = Session::create(
            SessionId::new("sess-2")?,
            UserId::new("user-1")?,
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
        let mut free = session()?;
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
        .map_err(|error| {
            std::io::Error::other(format!("context establishment failed: {error:?}"))
        })?;
        assert_eq!(ctx.assurance, Assurance::A1);
        Ok(())
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
