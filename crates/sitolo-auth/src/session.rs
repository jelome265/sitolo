//! Session model, lifecycle, and revocation semantics.
//!
//! Phase 3 specification, §10, §11, §10.3, Appendix A.1. A session is an
//! active authenticated relationship between a principal and Sitolo. Only
//! `ACTIVE` sessions may be accepted for normal authenticated requests, and
//! all expiry decisions use server-controlled time (§11.2).

use std::time::{Duration, SystemTime};

use crate::error::AuthError;
use crate::id::{Assurance, DeviceId, SecurityVersion, SessionClass, SessionId, UserId};

/// Session state machine states (§10.2). `Created` exists only until the
/// authoritative transaction commits; a failed creation produces no session
/// (Appendix A.1), so a `Created` row is never observable to request handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    Created,
    Active,
    IdleExpired,
    AbsoluteExpired,
    Revoked,
    SecurityRevoked,
}

impl SessionState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        !matches!(self, SessionState::Created | SessionState::Active)
    }
}

/// Class-based lifetime policy (§11.1). Durations are configuration evidence,
/// never hard-coded domain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLifetime {
    pub idle: Duration,
    pub absolute: Duration,
}

/// The policy set for all session classes (§11.1, §63).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionPolicySet {
    pub merchant: SessionLifetime,
    pub administrator: SessionLifetime,
    pub break_glass: SessionLifetime,
}

impl SessionPolicySet {
    /// Selects the class policy (§11.1).
    #[must_use]
    pub fn for_class(&self, class: SessionClass) -> SessionLifetime {
        match class {
            SessionClass::Merchant => self.merchant,
            SessionClass::Administrator => self.administrator,
            SessionClass::BreakGlass => self.break_glass,
        }
    }
}

/// Security events that invalidate sessions (§10.3). Each event has explicit,
/// centrally defined scope semantics; individual handlers must never guess
/// which sessions die.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationTrigger {
    Logout,
    AdministrativeRevocation,
    PasswordChange,
    PasswordReset,
    MfaReset,
    DeviceRevocation,
    SecurityCompromise,
    UserSuspension,
    MembershipLoss,
    ProviderRevocation,
    SecurityVersionMismatch,
}

/// How wide a revocation reaches (§10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationScope {
    CurrentSession,
    AllSessionsOnCurrentDevice,
    AllUserSessions,
    AuthenticatorFamily,
}

impl RevocationTrigger {
    /// The mandated scope for each trigger (§10.3). Semantics are central and
    /// reviewable; handlers cannot widen or narrow them locally.
    #[must_use]
    pub fn default_scope(self) -> RevocationScope {
        match self {
            RevocationTrigger::Logout => RevocationScope::CurrentSession,
            RevocationTrigger::AdministrativeRevocation => RevocationScope::AllUserSessions,
            RevocationTrigger::PasswordChange => RevocationScope::AllUserSessions,
            RevocationTrigger::PasswordReset => RevocationScope::AllUserSessions,
            RevocationTrigger::MfaReset => RevocationScope::AllUserSessions,
            RevocationTrigger::DeviceRevocation => RevocationScope::AllSessionsOnCurrentDevice,
            RevocationTrigger::SecurityCompromise => RevocationScope::AllUserSessions,
            RevocationTrigger::UserSuspension => RevocationScope::AllUserSessions,
            RevocationTrigger::MembershipLoss => RevocationScope::CurrentSession,
            RevocationTrigger::ProviderRevocation => RevocationScope::AuthenticatorFamily,
            RevocationTrigger::SecurityVersionMismatch => RevocationScope::CurrentSession,
        }
    }

    /// Whether the trigger marks the session `SecurityRevoked` (security
    /// evidence) rather than ordinary `Revoked` (§10.2, A.1).
    #[must_use]
    pub fn is_security_event(self) -> bool {
        matches!(
            self,
            RevocationTrigger::SecurityCompromise
                | RevocationTrigger::SecurityVersionMismatch
                | RevocationTrigger::DeviceRevocation
                | RevocationTrigger::ProviderRevocation
                | RevocationTrigger::MfaReset
        )
    }
}

/// An authenticated session (§10).
#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub device_id: Option<DeviceId>,
    pub class: SessionClass,
    pub state: SessionState,
    pub created_at: SystemTime,
    pub authenticated_at: SystemTime,
    pub last_seen_at: SystemTime,
    pub idle_expires_at: SystemTime,
    pub absolute_expires_at: SystemTime,
    pub assurance: Assurance,
    pub authentication_method: crate::id::AuthenticationMethod,
    pub security_version: SecurityVersion,
}

impl Session {
    /// Creates an authoritative session inside its class lifetime. Creation is
    /// transactional at the repository boundary; a partial activation never
    /// becomes observable (Appendix A.1).
    // Explicit 9-field domain constructor; bundling into a params struct would
    // obscure the Appendix A.1 creation invariant at call sites.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn create(
        id: SessionId,
        user_id: UserId,
        device_id: Option<DeviceId>,
        class: SessionClass,
        authentication_method: crate::id::AuthenticationMethod,
        assurance: Assurance,
        security_version: SecurityVersion,
        lifetime: SessionLifetime,
        now: SystemTime,
    ) -> Session {
        Session {
            id,
            user_id,
            device_id,
            class,
            state: SessionState::Active,
            created_at: now,
            authenticated_at: now,
            last_seen_at: now,
            idle_expires_at: now + lifetime.idle,
            absolute_expires_at: now + lifetime.absolute,
            assurance,
            authentication_method,
            security_version,
        }
    }

    /// Validates the session for one authenticated request (§10.2, §11, §31).
    /// Checks security-version first, then device state, then absolute and
    /// idle expiry with server-controlled time, then state.
    pub fn accept(
        &mut self,
        now: SystemTime,
        user_security_version: SecurityVersion,
        device_state: Option<crate::device::DeviceState>,
    ) -> Result<(), AuthError> {
        if self.security_version != user_security_version {
            self.state = SessionState::SecurityRevoked;
            return Err(AuthError::SecurityVersionMismatch);
        }
        if let Some(device) = device_state
            && device != crate::device::DeviceState::Active
        {
            return Err(match device {
                crate::device::DeviceState::Revoked => AuthError::DeviceRevoked,
                crate::device::DeviceState::Suspended => AuthError::DeviceSuspended,
                _ => AuthError::DeviceVerificationRequired,
            });
        }
        if self.state == SessionState::Revoked {
            return Err(AuthError::SessionRevoked);
        }
        if self.state == SessionState::SecurityRevoked {
            return Err(AuthError::SessionRevoked);
        }
        if self.state != SessionState::Active {
            return Err(AuthError::SessionExpired);
        }
        if now >= self.absolute_expires_at {
            self.state = SessionState::AbsoluteExpired;
            return Err(AuthError::SessionExpired);
        }
        if now >= self.idle_expires_at {
            self.state = SessionState::IdleExpired;
            return Err(AuthError::SessionExpired);
        }
        self.last_seen_at = now;
        Ok(())
    }

    /// Applies the idle slide after an accepted request (§11).
    pub fn slide_idle(&mut self, lifetime: SessionLifetime, now: SystemTime) {
        self.idle_expires_at = now + lifetime.idle;
    }

    /// Revokes the session with the trigger-classified reason (§10.3). Once
    /// revoked, a session can never return to `Active` (Appendix A.1).
    pub fn revoke(&mut self, trigger: RevocationTrigger) {
        if !self.state.is_terminal() {
            self.state = if trigger.is_security_event() {
                SessionState::SecurityRevoked
            } else {
                SessionState::Revoked
            };
        }
    }

    /// Elevates assurance after a successful step-up/MFA verification (§16,
    /// §17). Elevation never lowers existing assurance.
    pub fn elevate_assurance(&mut self, to: Assurance) {
        if to > self.assurance {
            self.assurance = to;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DeviceState;
    use crate::id::AuthenticationMethod;

    fn session(lifetime: SessionLifetime) -> (SystemTime, Session) {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let s = Session::create(
            SessionId::new("sess-1").unwrap(),
            UserId::new("user-1").unwrap(),
            None,
            SessionClass::Merchant,
            AuthenticationMethod::Password,
            Assurance::A1,
            SecurityVersion(3),
            lifetime,
            now,
        );
        (now, s)
    }

    fn lifetime() -> SessionLifetime {
        SessionLifetime {
            idle: Duration::from_secs(30),
            absolute: Duration::from_secs(60),
        }
    }

    #[test]
    fn active_session_is_accepted_and_slides() {
        let (now, mut s) = session(lifetime());
        let later = now + Duration::from_secs(20);
        assert!(s.accept(later, SecurityVersion(3), None).is_ok());
        s.slide_idle(lifetime(), later);
        assert_eq!(s.last_seen_at, later);
        // Idle expiry must follow the slide, not the original issue time.
        assert!(
            s.accept(later + Duration::from_secs(25), SecurityVersion(3), None)
                .is_ok()
        );
    }

    #[test]
    fn idle_timeout_expires_server_side() {
        let (now, mut s) = session(lifetime());
        let err = s
            .accept(now + Duration::from_secs(31), SecurityVersion(3), None)
            .unwrap_err();
        assert_eq!(err, AuthError::SessionExpired);
        assert_eq!(s.state, SessionState::IdleExpired);
    }

    #[test]
    fn absolute_timeout_wins_over_idle_activity() {
        let (now, mut s) = session(lifetime());
        s.accept(now + Duration::from_secs(10), SecurityVersion(3), None)
            .unwrap();
        let err = s
            .accept(now + Duration::from_secs(61), SecurityVersion(3), None)
            .unwrap_err();
        assert_eq!(err, AuthError::SessionExpired);
        assert_eq!(s.state, SessionState::AbsoluteExpired);
    }

    #[test]
    fn security_version_mismatch_security_revokes() {
        let (now, mut s) = session(lifetime());
        let err = s.accept(now, SecurityVersion(4), None).unwrap_err();
        assert_eq!(err, AuthError::SecurityVersionMismatch);
        assert_eq!(s.state, SessionState::SecurityRevoked);
    }

    #[test]
    fn revoked_device_blocks_device_bound_session() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let mut s = Session::create(
            SessionId::new("sess-1").unwrap(),
            UserId::new("user-1").unwrap(),
            Some(DeviceId::new("dev-1").unwrap()),
            SessionClass::Merchant,
            AuthenticationMethod::Password,
            Assurance::A1,
            SecurityVersion(1),
            lifetime(),
            now,
        );
        let err = s
            .accept(now, SecurityVersion(1), Some(DeviceState::Revoked))
            .unwrap_err();
        assert_eq!(err, AuthError::DeviceRevoked);
    }

    #[test]
    fn revoked_session_never_returns_to_active() {
        let (now, mut s) = session(lifetime());
        s.revoke(RevocationTrigger::Logout);
        assert_eq!(s.state, SessionState::Revoked);
        assert_eq!(
            s.accept(now, SecurityVersion(3), None).unwrap_err(),
            AuthError::SessionRevoked
        );
        s.revoke(RevocationTrigger::Logout);
        assert_eq!(s.state, SessionState::Revoked);
    }

    #[test]
    fn triggers_map_to_mandated_scopes() {
        assert_eq!(
            RevocationTrigger::Logout.default_scope(),
            RevocationScope::CurrentSession
        );
        assert_eq!(
            RevocationTrigger::PasswordReset.default_scope(),
            RevocationScope::AllUserSessions
        );
        assert_eq!(
            RevocationTrigger::DeviceRevocation.default_scope(),
            RevocationScope::AllSessionsOnCurrentDevice
        );
    }

    #[test]
    fn elevation_never_downgrades_assurance() {
        let (now, mut s) = session(lifetime());
        let _ = now;
        s.elevate_assurance(Assurance::A3);
        s.elevate_assurance(Assurance::A2);
        assert_eq!(s.assurance, Assurance::A3);
    }
}
