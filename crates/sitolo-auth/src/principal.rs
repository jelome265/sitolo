//! Principal and security context.
//!
//! Phase 3 specification, §15, §32. The authenticated principal is a typed
//! identity fact; the security context is the trusted handoff to downstream
//! authorization (§69). Raw tokens must not survive beyond the credential
//! extraction boundary (§32.1, §49).

use std::time::SystemTime;

use sitolo_observability::{RequestId, TraceParent};

use crate::device::{Device, DeviceState};
use crate::error::AuthError;
use crate::id::{Assurance, DeviceId, SecurityVersion, SessionId, UserId};
use crate::session::{Session, SessionLifetime, SessionState};

/// Normalized identity fact (§15). Deliberately excludes tenant, role, and
/// permission claims because those are current authorization facts, not
/// identity-provider facts (§1.2, §69).
#[derive(Debug, Clone)]
pub struct AuthenticatedPrincipal {
    /// The authenticated user.
    pub subject_id: UserId,
    /// Session the principal was authenticated on.
    pub session_id: SessionId,
    /// Device bound to the session, if any.
    pub device_id: Option<DeviceId>,
    /// When the subject authenticated.
    pub authenticated_at: SystemTime,
    /// Assurance level established at authentication (§16).
    pub assurance: Assurance,
    /// User security version observed at authentication (§31).
    pub security_version: SecurityVersion,
}

/// Session security state (§32.1).
#[derive(Debug, Clone)]
pub struct SessionSecurityState {
    /// Session the state describes.
    pub session_id: SessionId,
    /// Current lifecycle state of the session.
    pub state: SessionState,
    /// Assurance level the session currently holds.
    pub assurance: Assurance,
    /// Session security version (§31).
    pub security_version: SecurityVersion,
    /// Risk class that selected the session lifetime policy (§11.1).
    pub class: crate::id::SessionClass,
}

/// Device security state (§32.1).
#[derive(Debug, Clone)]
pub struct DeviceSecurityState {
    /// Device the state describes.
    pub device_id: DeviceId,
    /// Current lifecycle state of the device.
    pub state: DeviceState,
    /// Device security version (§31).
    pub security_version: SecurityVersion,
}

/// Trusted security context (§32.1). Constructed only through the
/// middleware pipeline; never contains raw bearer tokens or secrets after
/// the credential extraction boundary.
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// The authenticated identity fact.
    pub principal: AuthenticatedPrincipal,
    /// Session state observed while establishing the context.
    pub session: SessionSecurityState,
    /// Device state observed while establishing the context, if bound.
    pub device: Option<DeviceSecurityState>,
    /// Effective assurance of the request.
    pub assurance: Assurance,
    /// Correlation identifier for this request.
    pub request_id: RequestId,
    /// Incoming trace context, when present.
    pub trace: Option<TraceParent>,
}

impl SecurityContext {
    /// Establishes a security context from a session and optional device
    /// (§32). Validates security version, device state, and session
    /// expiration; denies if any check fails.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::SecurityVersionMismatch`] when the session and the
    /// user disagree on the security version (the session is also moved to
    /// `SecurityRevoked`), [`AuthError::DeviceRevoked`],
    /// [`AuthError::DeviceSuspended`], or
    /// [`AuthError::DeviceVerificationRequired`] when a bound device is not
    /// active, [`AuthError::SessionRevoked`] when the session is already
    /// revoked, and [`AuthError::SessionExpired`] when the absolute or idle
    /// lifetime has elapsed at `now`.
    pub fn establish(
        session: &mut Session,
        device: Option<&Device>,
        user_security_version: SecurityVersion,
        lifetime: SessionLifetime,
        now: SystemTime,
        request_id: RequestId,
        trace: Option<TraceParent>,
    ) -> Result<Self, AuthError> {
        let device_state = device.map(|d| d.state);
        session.accept(now, user_security_version, device_state)?;
        session.slide_idle(lifetime, now);
        let principal = AuthenticatedPrincipal {
            subject_id: session.user_id.clone(),
            session_id: session.id.clone(),
            device_id: session.device_id.clone(),
            authenticated_at: session.authenticated_at,
            assurance: session.assurance,
            security_version: session.security_version,
        };
        let session_state = SessionSecurityState {
            session_id: session.id.clone(),
            state: session.state,
            assurance: session.assurance,
            security_version: session.security_version,
            class: session.class,
        };
        let device_state = device.map(|d| DeviceSecurityState {
            device_id: d.id.clone(),
            state: d.state,
            security_version: d.security_version,
        });
        Ok(SecurityContext {
            principal,
            session: session_state,
            device: device_state,
            assurance: session.assurance,
            request_id,
            trace,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::id::{AuthenticationMethod, SessionClass};

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }

    #[test]
    fn security_context_construction() -> Result<(), AuthError> {
        let now = at(1_000);
        let mut session = Session::create(
            SessionId::new("sess-1")?,
            UserId::new("user-1")?,
            None,
            SessionClass::Merchant,
            AuthenticationMethod::Password,
            Assurance::A1,
            SecurityVersion(1),
            SessionLifetime {
                idle: Duration::from_secs(30),
                absolute: Duration::from_secs(60),
            },
            now,
        );
        let lifetime = SessionLifetime {
            idle: Duration::from_secs(30),
            absolute: Duration::from_secs(60),
        };
        let request_id = RequestId::new_server();
        let ctx = SecurityContext::establish(
            &mut session,
            None,
            SecurityVersion(1),
            lifetime,
            now,
            request_id.clone(),
            None,
        )?;
        assert_eq!(ctx.principal.subject_id, UserId::new("user-1")?);
        assert_eq!(ctx.assurance, Assurance::A1);
        assert_eq!(ctx.request_id, request_id);
        Ok(())
    }

    #[test]
    fn security_version_mismatch_denies() -> Result<(), AuthError> {
        let now = at(1_000);
        let mut session = Session::create(
            SessionId::new("sess-1")?,
            UserId::new("user-1")?,
            None,
            SessionClass::Merchant,
            AuthenticationMethod::Password,
            Assurance::A1,
            SecurityVersion(1),
            SessionLifetime {
                idle: Duration::from_secs(30),
                absolute: Duration::from_secs(60),
            },
            now,
        );
        let lifetime = SessionLifetime {
            idle: Duration::from_secs(30),
            absolute: Duration::from_secs(60),
        };
        let request_id = RequestId::new_server();
        assert!(matches!(
            SecurityContext::establish(
                &mut session,
                None,
                SecurityVersion(2),
                lifetime,
                now,
                request_id,
                None,
            ),
            Err(AuthError::SecurityVersionMismatch)
        ));
        Ok(())
    }
}
