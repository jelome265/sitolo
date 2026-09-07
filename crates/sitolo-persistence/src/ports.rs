//! Repository ports (§51).
//!
//! Repositories expose semantic operations rather than generic database
//! mutation. The IdentityStores trait bundles all Phase 3 identity
//! persistence; implementations own transaction boundaries (§52).

use async_trait::async_trait;

use sitolo_audit::{AuditRecorder, AuthenticationEvent};
use sitolo_auth::{
    Assurance, AuthenticationMethod, ClientPlatform, Device, DeviceId, MfaAuthenticatorId, MfaKind,
    RecoveryCodeId, SecurityVersion, SessionClass, SessionId, UserId,
};
use sitolo_security::SealedRef;

use crate::memory::{
    DeviceRegistrationInput, DeviceRevocationEffect, EstablishedSession, MfaEnrollmentResult,
    PasswordResetResult, RefreshRotation, SessionSnapshot, UserSnapshot,
};

/// Bundled identity persistence port (§51).
#[async_trait]
pub trait IdentityStores: Send + Sync + AuditRecorder {
    // --- users ---
    async fn create_user(
        &self,
        id: UserId,
        password_version: Option<u32>,
        password_verifier: Option<String>,
    ) -> Result<UserSnapshot, sitolo_auth::AuthError>;
    async fn user_snapshot(&self, id: &UserId) -> Option<UserSnapshot>;
    async fn set_password_verifier(
        &self,
        id: &UserId,
        verifier: String,
        policy_version: u32,
    ) -> Result<(), sitolo_auth::AuthError>;
    async fn bump_user_security_version(
        &self,
        id: &UserId,
    ) -> Result<SecurityVersion, sitolo_auth::AuthError>;
    async fn suspend_user(&self, id: &UserId) -> Result<(), sitolo_auth::AuthError>;

    // --- sessions ---
    // Explicit session-establishment port; bundling would obscure the §52
    // transaction boundary implementors must enforce.
    #[allow(clippy::too_many_arguments)]
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
    async fn session_snapshot(&self, id: &SessionId) -> Option<SessionSnapshot>;
    async fn accept_session(
        &self,
        id: &SessionId,
        now: std::time::SystemTime,
    ) -> Result<SessionSnapshot, sitolo_auth::AuthError>;
    async fn revoke_session(
        &self,
        id: &SessionId,
        trigger: sitolo_auth::RevocationTrigger,
    ) -> Result<(), sitolo_auth::AuthError>;
    async fn revoke_sessions_by_scope(
        &self,
        scope: sitolo_auth::RevocationScope,
        trigger: sitolo_auth::RevocationTrigger,
        user_id: Option<&UserId>,
        device_id: Option<&DeviceId>,
    ) -> Result<u32, sitolo_auth::AuthError>;
    async fn elevate_session_assurance(
        &self,
        id: &SessionId,
        to: Assurance,
    ) -> Result<(), sitolo_auth::AuthError>;

    // --- refresh ---
    async fn rotate_refresh(
        &self,
        raw: &str,
        now: std::time::SystemTime,
    ) -> Result<RefreshRotation, sitolo_auth::AuthError>;

    // --- devices ---
    async fn begin_device_registration(
        &self,
        input: DeviceRegistrationInput,
    ) -> Result<Device, sitolo_auth::AuthError>;
    async fn complete_device_registration(
        &self,
        device_id: &DeviceId,
        now: std::time::SystemTime,
    ) -> Result<Device, sitolo_auth::AuthError>;
    async fn device_snapshot(&self, id: &DeviceId) -> Option<Device>;
    async fn revoke_device(
        &self,
        id: &DeviceId,
        now: std::time::SystemTime,
    ) -> Result<DeviceRevocationEffect, sitolo_auth::AuthError>;
    async fn replace_device(
        &self,
        old: &DeviceId,
        now: std::time::SystemTime,
    ) -> Result<Device, sitolo_auth::AuthError>;

    // --- mfa ---
    async fn begin_mfa_enrollment(
        &self,
        user_id: UserId,
        kind: MfaKind,
        secret_reference: Option<SealedRef>,
        now: std::time::SystemTime,
    ) -> Result<MfaEnrollmentResult, sitolo_auth::AuthError>;
    async fn complete_mfa_enrollment(
        &self,
        authenticator_id: &MfaAuthenticatorId,
        now: std::time::SystemTime,
    ) -> Result<Vec<String>, sitolo_auth::AuthError>;
    async fn record_totp_success(
        &self,
        authenticator_id: &MfaAuthenticatorId,
        step: u64,
    ) -> Result<(), sitolo_auth::AuthError>;
    async fn active_authenticator(&self, user_id: &UserId)
    -> Option<sitolo_auth::MfaAuthenticator>;
    async fn consume_recovery_code(
        &self,
        user_id: &UserId,
        raw: &str,
        now: std::time::SystemTime,
    ) -> Result<RecoveryCodeId, sitolo_auth::AuthError>;
    async fn reset_mfa(
        &self,
        target: &UserId,
        actor: &UserId,
        now: std::time::SystemTime,
    ) -> Result<Vec<MfaAuthenticatorId>, sitolo_auth::AuthError>;

    // --- password reset ---
    async fn request_password_reset(
        &self,
        user_id: &UserId,
        raw_token: &str,
        now: std::time::SystemTime,
        ttl: std::time::Duration,
    ) -> Result<PasswordResetResult, sitolo_auth::AuthError>;
    async fn redeem_password_reset(
        &self,
        raw_token: &str,
        new_verifier: String,
        new_policy_version: u32,
        now: std::time::SystemTime,
    ) -> Result<UserId, sitolo_auth::AuthError>;

    // --- audit ---
    async fn append_audit(
        &self,
        event: AuthenticationEvent,
    ) -> Result<(), sitolo_audit::AuditError>;
    async fn audit_trail(&self) -> Vec<AuthenticationEvent>;
}
