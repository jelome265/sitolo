//! Identity persistence ports owned by the application layer.
//!
//! The application layer defines what it needs from identity storage; the
//! persistence adapter implements these contracts. Keeping the port here
//! means the dependency edge points from persistence to application, never
//! the reverse (Phase 1 section 5.2, section 51).

use async_trait::async_trait;

use sitolo_audit::{AuditRecorder, AuthenticationEvent};
use sitolo_auth::{
    Assurance, AuthenticationMethod, ClientPlatform, Device, DeviceId, MfaAuthenticatorId, MfaKind,
    RecoveryCodeId, SecurityVersion, SessionClass, SessionId, UserId,
};
use sitolo_security::SealedRef;

pub use sitolo_auth::SessionSnapshot;

/// User snapshot returned to the application layer.
#[derive(Debug, Clone)]
pub struct UserSnapshot {
    /// Stable user identifier.
    pub id: UserId,
    /// Monotonic security version used to invalidate outstanding credentials.
    pub security_version: SecurityVersion,
    /// Whether the account is administratively suspended.
    pub suspended: bool,
    /// Whether a password verifier is configured.
    pub has_password: bool,
    /// Whether an active MFA authenticator exists.
    pub mfa_active: bool,
    /// Opaque password verifier; never logged or returned to clients.
    pub password_verifier: Option<String>,
    /// Password policy version that produced the verifier.
    pub password_policy_version: u32,
}

/// The outcome of establishing a session (Phase 3 section 52.1).
#[derive(Debug, Clone)]
pub struct EstablishedSession {
    /// The persisted session record.
    pub session: sitolo_auth::Session,
    /// Raw refresh token; returned to the caller once and never persisted.
    pub refresh_token: String,
    /// Refresh-family identifier for rotation and revocation.
    pub family_id: sitolo_auth::RefreshFamilyId,
}

/// The outcome of a successful refresh rotation (Phase 3 section 12.1).
#[derive(Debug, Clone)]
pub struct RefreshRotation {
    /// Refresh-family identifier.
    pub family_id: sitolo_auth::RefreshFamilyId,
    /// User that owns the family.
    pub user_id: UserId,
    /// Session bound to the rotated credential.
    pub session_id: SessionId,
    /// Successor raw refresh token.
    pub successor_token: String,
}

/// The outcome of MFA enrollment completion (Phase 3 section 18.1).
#[derive(Debug, Clone)]
pub struct MfaEnrollmentResult {
    /// The newly active authenticator.
    pub authenticator_id: MfaAuthenticatorId,
    /// Challenge that must be satisfied to finish enrollment.
    pub challenge_id: sitolo_auth::ChallengeId,
}

/// The outcome of a password reset request (Phase 3 section 21).
#[derive(Debug, Clone)]
pub struct PasswordResetResult {
    /// Resolved user, or `None` when the request must remain enumeration-safe.
    pub user_id: Option<UserId>,
}

/// The outcome of device revocation (Phase 3 sections 27 and 52.3).
#[derive(Debug, Clone)]
pub struct DeviceRevocationEffect {
    /// The revoked device record.
    pub device: Device,
    /// Sessions revoked as a consequence.
    pub revoked_sessions: Vec<SessionId>,
}

/// Input for device registration (Phase 3 section 25).
#[derive(Debug, Clone)]
pub struct DeviceRegistrationInput {
    /// User registering the device.
    pub user_id: UserId,
    /// Client platform reported by the device.
    pub platform: ClientPlatform,
    /// Security version the device is bound to.
    pub security_version: SecurityVersion,
    /// Registration timestamp.
    pub now: std::time::SystemTime,
}

/// Bundled identity persistence port (Phase 1 section 51).
///
/// Implementations own the transaction boundary for every operation
/// (Phase 1 section 52).
#[async_trait]
pub trait IdentityStores: Send + Sync + AuditRecorder {
    /// Resolves an exact, pre-linked external identity. This operation must
    /// never create or link a user based on token claims such as email.
    async fn resolve_external_identity(&self, issuer: &str, subject: &str) -> Option<UserId>;

    /// Explicit administrative/onboarding binding. Implementations must
    /// reject missing users and any attempt to reassign an existing pair.
    async fn bind_external_identity(
        &self,
        issuer: String,
        subject: String,
        user_id: &UserId,
    ) -> Result<(), sitolo_auth::AuthError>;

    // --- users ---

    /// Creates a user record with an optional password verifier.
    async fn create_user(
        &self,
        id: UserId,
        password_version: Option<u32>,
        password_verifier: Option<String>,
    ) -> Result<UserSnapshot, sitolo_auth::AuthError>;

    /// Returns the user snapshot when the user exists.
    async fn user_snapshot(&self, id: &UserId) -> Option<UserSnapshot>;

    /// Replaces the password verifier and records the producing policy version.
    async fn set_password_verifier(
        &self,
        id: &UserId,
        verifier: String,
        policy_version: u32,
    ) -> Result<(), sitolo_auth::AuthError>;

    /// Increments the user's security version.
    async fn bump_user_security_version(
        &self,
        id: &UserId,
    ) -> Result<SecurityVersion, sitolo_auth::AuthError>;

    /// Marks the user as suspended.
    async fn suspend_user(&self, id: &UserId) -> Result<(), sitolo_auth::AuthError>;

    // --- sessions ---

    /// Explicit session-establishment port; bundling would obscure the
    /// transaction boundary implementors must enforce.
    #[expect(
        clippy::too_many_arguments,
        reason = "The explicit session-establishment fields expose the transaction contract at call sites."
    )]
    async fn establish_session(
        &self,
        user_id: UserId,
        device_id: Option<DeviceId>,
        class: SessionClass,
        method: AuthenticationMethod,
        platform: ClientPlatform,
        assurance: Assurance,
        now: std::time::SystemTime,
    ) -> Result<EstablishedSession, sitolo_auth::AuthError>;

    /// Returns the session snapshot when the session exists.
    async fn session_snapshot(&self, id: &SessionId) -> Option<SessionSnapshot>;

    /// Marks a session accepted.
    async fn accept_session(
        &self,
        id: &SessionId,
        now: std::time::SystemTime,
    ) -> Result<SessionSnapshot, sitolo_auth::AuthError>;

    /// Accepts a session only when it belongs to the already-resolved user.
    /// The ownership check and session-state update are one repository
    /// operation; the caller must not trust a session ID as identity proof.
    async fn accept_session_for_user(
        &self,
        id: &SessionId,
        expected_user_id: &UserId,
        now: std::time::SystemTime,
    ) -> Result<SessionSnapshot, sitolo_auth::AuthError>;

    /// Revokes one session.
    async fn revoke_session(
        &self,
        id: &SessionId,
        trigger: sitolo_auth::RevocationTrigger,
    ) -> Result<(), sitolo_auth::AuthError>;

    /// Revokes every session matching the scope.
    async fn revoke_sessions_by_scope(
        &self,
        scope: sitolo_auth::RevocationScope,
        trigger: sitolo_auth::RevocationTrigger,
        user_id: Option<&UserId>,
        device_id: Option<&DeviceId>,
    ) -> Result<u32, sitolo_auth::AuthError>;

    /// Raises the assurance level of a session.
    async fn elevate_session_assurance(
        &self,
        id: &SessionId,
        to: Assurance,
    ) -> Result<(), sitolo_auth::AuthError>;

    // --- refresh ---

    /// Rotates a refresh credential and returns the successor.
    async fn rotate_refresh(
        &self,
        raw: &str,
        now: std::time::SystemTime,
    ) -> Result<RefreshRotation, sitolo_auth::AuthError>;

    // --- devices ---

    /// Begins device registration.
    async fn begin_device_registration(
        &self,
        input: DeviceRegistrationInput,
    ) -> Result<Device, sitolo_auth::AuthError>;

    /// Completes device registration.
    async fn complete_device_registration(
        &self,
        device_id: &DeviceId,
        now: std::time::SystemTime,
    ) -> Result<Device, sitolo_auth::AuthError>;

    /// Returns the device when it exists.
    async fn device_snapshot(&self, id: &DeviceId) -> Option<Device>;

    /// Revokes a device and its sessions.
    async fn revoke_device(
        &self,
        id: &DeviceId,
        now: std::time::SystemTime,
    ) -> Result<DeviceRevocationEffect, sitolo_auth::AuthError>;

    /// Replaces a device with a fresh registration.
    async fn replace_device(
        &self,
        old: &DeviceId,
        now: std::time::SystemTime,
    ) -> Result<Device, sitolo_auth::AuthError>;

    // --- mfa ---

    /// Begins MFA enrollment.
    async fn begin_mfa_enrollment(
        &self,
        user_id: UserId,
        kind: MfaKind,
        secret_reference: Option<SealedRef>,
        now: std::time::SystemTime,
    ) -> Result<MfaEnrollmentResult, sitolo_auth::AuthError>;

    /// Completes MFA enrollment and returns recovery codes.
    async fn complete_mfa_enrollment(
        &self,
        authenticator_id: &MfaAuthenticatorId,
        now: std::time::SystemTime,
    ) -> Result<Vec<String>, sitolo_auth::AuthError>;

    /// Records a successful TOTP step to prevent replay.
    async fn record_totp_success(
        &self,
        authenticator_id: &MfaAuthenticatorId,
        step: u64,
    ) -> Result<(), sitolo_auth::AuthError>;

    /// Returns the active authenticator for a user.
    async fn active_authenticator(&self, user_id: &UserId)
    -> Option<sitolo_auth::MfaAuthenticator>;

    /// Consumes a recovery code.
    async fn consume_recovery_code(
        &self,
        user_id: &UserId,
        raw: &str,
        now: std::time::SystemTime,
    ) -> Result<RecoveryCodeId, sitolo_auth::AuthError>;

    /// Resets every authenticator for a user.
    async fn reset_mfa(
        &self,
        target: &UserId,
        actor: &UserId,
        now: std::time::SystemTime,
    ) -> Result<Vec<MfaAuthenticatorId>, sitolo_auth::AuthError>;

    // --- password reset ---

    /// Creates a password reset artifact.
    async fn request_password_reset(
        &self,
        user_id: &UserId,
        raw_token: &str,
        now: std::time::SystemTime,
        ttl: std::time::Duration,
    ) -> Result<PasswordResetResult, sitolo_auth::AuthError>;

    /// Redeems a password reset artifact.
    async fn redeem_password_reset(
        &self,
        raw_token: &str,
        new_verifier: String,
        new_policy_version: u32,
        now: std::time::SystemTime,
    ) -> Result<UserId, sitolo_auth::AuthError>;

    // --- audit ---

    /// Appends an authentication audit event.
    async fn append_audit(
        &self,
        event: AuthenticationEvent,
    ) -> Result<(), sitolo_audit::AuditError>;

    /// Returns the authentication audit trail.
    async fn audit_trail(&self) -> Vec<AuthenticationEvent>;
}
