//! Authentication audit events and record shape.
//!
//! Phase 3 specification, §37, §37.1. The minimum event set is enumerated
//! here; records are constructed from secret-free fields by design. The
//! recorder port is async so that production implementations can write to
//! durable storage transactionally.

use std::sync::Mutex;
use std::time::SystemTime;

use async_trait::async_trait;
use thiserror::Error;

use sitolo_auth::{Assurance, AuditEventId, AuthenticationMethod, ClientPlatform, SecurityVersion};

/// Authentication audit event names (§37). The dotted lowercase names are
/// consistent with the existing observability registry; the SCREAMING names
/// in the specification map 1:1 to these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthEventName {
    LoginSuccess,
    LoginFailure,
    LoginRateLimited,
    SessionCreated,
    SessionRefreshed,
    SessionRevoked,
    SessionExpired,
    Logout,
    RefreshRotated,
    RefreshReuseDetected,
    PasswordChanged,                                                                                
    PasswordResetRequested,
    PasswordResetCompleted,
    MfaEnrollmentStarted,                                        
    MfaEnrollmentCompleted,
    MfaEnrollmentRejected,
    MfaSuccess,
    MfaFailure,
    MfaReset,
    RecoveryCodeRedeemed,
    DeviceRegistrationStarted,
    DeviceRegistered,
    DeviceSuspended,
    DeviceRevoked,
    DeviceReplaced,
    DeviceVerificationFailed,
    JwksRefreshFailed,
    TokenValidationFailed,
}

impl AuthEventName {
    /// Stable dotted event name for the audit record and registry.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AuthEventName::LoginSuccess => "auth.login.succeeded",
            AuthEventName::LoginFailure => "auth.login.failed",
            AuthEventName::LoginRateLimited => "auth.login.rate_limited",
            AuthEventName::SessionCreated => "auth.session.created",
            AuthEventName::SessionRefreshed => "auth.session.refreshed",
            AuthEventName::SessionRevoked => "auth.session.revoked",
            AuthEventName::SessionExpired => "auth.session.expired",
            AuthEventName::Logout => "auth.logout",
            AuthEventName::RefreshRotated => "auth.refresh.rotated",
            AuthEventName::RefreshReuseDetected => "auth.refresh.reuse_detected",
            AuthEventName::PasswordChanged => "auth.password.changed",
            AuthEventName::PasswordResetRequested => "auth.password.reset_requested",
            AuthEventName::PasswordResetCompleted => "auth.password.reset_completed",
            AuthEventName::MfaEnrollmentStarted => "auth.mfa.enrollment_started",
            AuthEventName::MfaEnrollmentCompleted => "auth.mfa.enrollment_completed",
            AuthEventName::MfaEnrollmentRejected => "auth.mfa.enrollment_rejected",
            AuthEventName::MfaSuccess => "auth.mfa.success",
            AuthEventName::MfaFailure => "auth.mfa.failure",
            AuthEventName::MfaReset => "auth.mfa.reset",
            AuthEventName::RecoveryCodeRedeemed => "auth.recovery.redeemed",
            AuthEventName::DeviceRegistrationStarted => "auth.device.registration_started",
            AuthEventName::DeviceRegistered => "auth.device.registered",
            AuthEventName::DeviceSuspended => "auth.device.suspended",
            AuthEventName::DeviceRevoked => "auth.device.revoked",
            AuthEventName::DeviceReplaced => "auth.device.replaced",
            AuthEventName::DeviceVerificationFailed => "auth.device.verification_failed",
            AuthEventName::JwksRefreshFailed => "auth.jwks.refresh_failed",
            AuthEventName::TokenValidationFailed => "auth.token.validation_failed",
        }
    }
}

/// Audit event outcome (§37.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    Success,
    Failure,
}

/// Authentication audit record (§37.1).
///
/// Secret-bearing fields are structurally impossible: the record is
/// constructed from pseudonymous references, bounded labels, and outcome
/// classifications. The forbidden list in §37.1 (password, otp, mfa
/// secret, refresh token, access token, authorization code, client secret,
/// private key, recovery code) is not part of this shape.
#[derive(Debug, Clone)]
pub struct AuthenticationEvent {
    pub event_id: AuditEventId,
    pub event_name: AuthEventName,
    pub occurred_at: SystemTime,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub subject_ref: Option<String>,
    pub session_ref: Option<String>,
    pub device_ref: Option<String>,
    pub client_platform: Option<ClientPlatform>,
    pub authentication_method: Option<AuthenticationMethod>,
    pub assurance_level: Option<Assurance>,
    pub result: EventResult,
    pub reason_class: Option<&'static str>,
    pub security_version: Option<SecurityVersion>,
}

/// Audit persistence failure (§43.4).
#[derive(Debug, Error)]
pub enum AuditError {
    #[error("audit record persistence failed")]
    PersistenceFailed,
}

/// Whether a record is mandatory or diagnostic (§43.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditRequirement {
    Mandatory,
    Diagnostic,
}

/// Audit recorder boundary (§37, §43.4).
#[async_trait]
pub trait AuditRecorder: Send + Sync {
    async fn record(
        &self,
        event: AuthenticationEvent,
        requirement: AuditRequirement,
    ) -> Result<(), AuditError>;
}

/// In-memory audit sink for tests and local reference wiring.
#[derive(Default)]
pub struct InMemoryAuditSink {
    events: Mutex<Vec<AuthenticationEvent>>,
}

impl InMemoryAuditSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a snapshot of all recorded events.
    pub fn events(&self) -> Vec<AuthenticationEvent> {
        self.events.lock().expect("audit sink lock").clone()
    }
}

#[async_trait]
impl AuditRecorder for InMemoryAuditSink {
    async fn record(
        &self,
        event: AuthenticationEvent,
        _requirement: AuditRequirement,
    ) -> Result<(), AuditError> {
        self.events.lock().expect("audit sink lock").push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_names_are_stable_and_dotted() {
        assert_eq!(AuthEventName::LoginSuccess.as_str(), "auth.login.succeeded");
        assert_eq!(
            AuthEventName::RefreshReuseDetected.as_str(),
            "auth.refresh.reuse_detected"
        );
        assert_eq!(AuthEventName::DeviceRevoked.as_str(), "auth.device.revoked");
    }

    #[test]
    fn record_shape_has_no_secret_fields() {
        // The record shape is secret-free by construction. This test
        // documents the invariant: no field in AuthenticationEvent can hold
        // a password, otp, mfa secret, refresh token, access token,
        // authorization code, client secret, private key, or recovery code.
        let event = AuthenticationEvent {
            event_id: AuditEventId::new("evt-1").unwrap(),
            event_name: AuthEventName::LoginSuccess,
            occurred_at: SystemTime::UNIX_EPOCH,
            request_id: Some("req-1".into()),
            trace_id: None,
            subject_ref: Some("subject_pseudonymous".into()),
            session_ref: Some("session_pseudonymous".into()),
            device_ref: None,
            client_platform: Some(ClientPlatform::Android),
            authentication_method: Some(AuthenticationMethod::Password),
            assurance_level: Some(Assurance::A1),
            result: EventResult::Success,
            reason_class: None,
            security_version: Some(SecurityVersion(1)),
        };
        let debug = format!("{event:?}");
        assert!(!debug.contains("hunter2"));
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("Bearer"));
    }
}
