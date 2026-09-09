//! Authentication and IAM audit events and record shape.
//!
//! Phase 3 specification §37; Phase 4 specification §31.
//! Minimum event sets are enumerated here; records are constructed from secret-free fields by design.

use std::sync::Mutex;
use std::time::SystemTime;

use async_trait::async_trait;
use thiserror::Error;

use sitolo_auth::{Assurance, AuditEventId, AuthenticationMethod, ClientPlatform, SecurityVersion};

/// Authentication audit event names (§37).
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

/// IAM audit event names (Phase 4 §31).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IamEventName {
    OrganizationCreated,
    OrganizationUpdated,
    OrganizationSuspended,
    OrganizationReactivated,
    OrganizationClosed,

    InvitationCreated,
    InvitationAccepted,
    InvitationRevoked,
    InvitationExpired,

    MembershipCreated,
    MembershipRoleChanged,
    MembershipScopeChanged,
    MembershipSuspended,
    MembershipReactivated,
    MembershipRevoked,

    BranchCreated,
    BranchUpdated,
    BranchClosed,

    OwnershipTransferRequested,
    OwnershipTransferApproved,
    OwnershipTransferRejected,
}

impl IamEventName {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            IamEventName::OrganizationCreated => "iam.organization.created",
            IamEventName::OrganizationUpdated => "iam.organization.updated",
            IamEventName::OrganizationSuspended => "iam.organization.suspended",
            IamEventName::OrganizationReactivated => "iam.organization.reactivated",
            IamEventName::OrganizationClosed => "iam.organization.closed",

            IamEventName::InvitationCreated => "iam.invitation.created",
            IamEventName::InvitationAccepted => "iam.invitation.accepted",
            IamEventName::InvitationRevoked => "iam.invitation.revoked",
            IamEventName::InvitationExpired => "iam.invitation.expired",

            IamEventName::MembershipCreated => "iam.membership.created",
            IamEventName::MembershipRoleChanged => "iam.membership.role_changed",
            IamEventName::MembershipScopeChanged => "iam.membership.scope_changed",
            IamEventName::MembershipSuspended => "iam.membership.suspended",
            IamEventName::MembershipReactivated => "iam.membership.reactivated",
            IamEventName::MembershipRevoked => "iam.membership.revoked",

            IamEventName::BranchCreated => "iam.branch.created",
            IamEventName::BranchUpdated => "iam.branch.updated",
            IamEventName::BranchClosed => "iam.branch.closed",

            IamEventName::OwnershipTransferRequested => "iam.ownership.transfer_requested",
            IamEventName::OwnershipTransferApproved => "iam.ownership.transfer_approved",
            IamEventName::OwnershipTransferRejected => "iam.ownership.transfer_rejected",
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

/// IAM audit record (Phase 4 §31).
#[derive(Debug, Clone)]
pub struct IamAuditEvent {
    pub event_id: AuditEventId,
    pub event_name: IamEventName,
    pub occurred_at: SystemTime,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub actor_ref: Option<String>,
    pub organization_ref: Option<String>,
    pub target_ref: Option<String>,
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

    async fn record_iam(
        &self,
        event: IamAuditEvent,
        requirement: AuditRequirement,
    ) -> Result<(), AuditError> {
        let _ = (event, requirement);
        Ok(())
    }
}

/// In-memory audit sink for tests and local reference wiring.
#[derive(Default)]
pub struct InMemoryAuditSink {
    events: Mutex<Vec<AuthenticationEvent>>,
    iam_events: Mutex<Vec<IamAuditEvent>>,
}

impl InMemoryAuditSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a snapshot of all recorded authentication events.
    pub fn events(&self) -> Vec<AuthenticationEvent> {
        self.events.lock().expect("audit sink lock").clone()
    }

    /// Returns a snapshot of all recorded IAM events.
    pub fn iam_events(&self) -> Vec<IamAuditEvent> {
        self.iam_events.lock().expect("audit sink lock").clone()
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

    async fn record_iam(
        &self,
        event: IamAuditEvent,
        _requirement: AuditRequirement,
    ) -> Result<(), AuditError> {
        self.iam_events.lock().expect("audit sink lock").push(event);
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

        assert_eq!(
            IamEventName::OrganizationCreated.as_str(),
            "iam.organization.created"
        );
        assert_eq!(
            IamEventName::MembershipRoleChanged.as_str(),
            "iam.membership.role_changed"
        );
    }

    #[test]
    fn record_shape_has_no_secret_fields() {
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
