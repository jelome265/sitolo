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
    /// Creation is in progress and not yet committed.
    Created,
    /// The session may be accepted for authenticated requests.
    Active,
    /// The idle lifetime elapsed before the request.
    IdleExpired,
    /// The absolute lifetime elapsed before the request.
    AbsoluteExpired,
    /// The session was revoked for a non-security reason.
    Revoked,
    /// The session was revoked because security state changed (§31).
    SecurityRevoked,
}

impl SessionState {
    /// Whether this state can never return to `Active` (Appendix A.1).
    #[must_use]
    pub fn is_terminal(self) -> bool {
        !matches!(self, SessionState::Created | SessionState::Active)
    }
}

/// Class-based lifetime policy (§11.1). Durations are configuration evidence,
/// never hard-coded domain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLifetime {
    /// Maximum inactivity before the session expires.
    pub idle: Duration,
    /// Maximum total lifetime regardless of activity.
    pub absolute: Duration,
}

/// The policy set for all session classes (§11.1, §63).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionPolicySet {
    /// Lifetime policy for merchant sessions.
    pub merchant: SessionLifetime,
    /// Lifetime policy for administrative sessions.
    pub administrator: SessionLifetime,
    /// Lifetime policy for break-glass sessions.
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
    /// The user logged out of the current session.
    Logout,
    /// An administrator revoked access.
    AdministrativeRevocation,
    /// The user changed their password.
    PasswordChange,
    /// The user completed a password reset.
    PasswordReset,
    /// MFA state was reset for the user.
    MfaReset,
    /// A device bound to the session was revoked.
    DeviceRevocation,
    /// A security compromise was detected.
    SecurityCompromise,
    /// The user account was suspended.
    UserSuspension,
    /// The user lost a membership relevant to the session.
    MembershipLoss,
    /// The identity provider revoked the underlying grant.
    ProviderRevocation,
    /// The presented security version no longer matches authority (§31).
    SecurityVersionMismatch,
}

/// How wide a revocation reaches (§10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationScope {
    /// Only the session that observed the event.
    CurrentSession,
    /// Every session bound to the current device.
    AllSessionsOnCurrentDevice,
    /// Every session belonging to the user.
    AllUserSessions,
    /// Every session that used the affected authenticator family.
    AuthenticatorFamily,
}

impl RevocationTrigger {
    /// The mandated scope for each trigger (§10.3). Semantics are central and
    /// reviewable; handlers cannot widen or narrow them locally.
    #[must_use]
    pub fn default_scope(self) -> RevocationScope {
        match self {
            RevocationTrigger::Logout
            | RevocationTrigger::MembershipLoss
            | RevocationTrigger::SecurityVersionMismatch => RevocationScope::CurrentSession,
            RevocationTrigger::AdministrativeRevocation
            | RevocationTrigger::PasswordChange
            | RevocationTrigger::PasswordReset
            | RevocationTrigger::MfaReset
            | RevocationTrigger::SecurityCompromise
            | RevocationTrigger::UserSuspension => RevocationScope::AllUserSessions,
            RevocationTrigger::DeviceRevocation => RevocationScope::AllSessionsOnCurrentDevice,
            RevocationTrigger::ProviderRevocation => RevocationScope::AuthenticatorFamily,
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
    /// Server-issued session identifier.
    pub id: SessionId,
    /// The authenticated user.
    pub user_id: UserId,
    /// Device bound to the session, if any.
    pub device_id: Option<DeviceId>,
    /// Risk class that selected the lifetime policy (§11.1).
    pub class: SessionClass,
    /// Current lifecycle state.
    pub state: SessionState,
    /// When the session row was created.
    pub created_at: SystemTime,
    /// When the subject authenticated.
    pub authenticated_at: SystemTime,
    /// When the session was last accepted.
    pub last_seen_at: SystemTime,
    /// End of the idle window.
    pub idle_expires_at: SystemTime,
    /// End of the absolute lifetime.
    pub absolute_expires_at: SystemTime,
    /// Assurance level the session holds (§16).
    pub assurance: Assurance,
    /// How the subject authenticated.
    pub authentication_method: crate::id::AuthenticationMethod,
    /// User security version observed at authentication (§31).
    pub security_version: SecurityVersion,
}

impl Session {
    /// Creates an authoritative session inside its class lifetime. Creation is
    /// transactional at the repository boundary; a partial activation never
    /// becomes observable (Appendix A.1).
    // Explicit 9-field domain constructor; bundling into a params struct would
    // obscure the Appendix A.1 creation invariant at call sites.
    #[expect(
        clippy::too_many_arguments,
        reason = "The explicit session constructor keeps the Appendix A.1 creation invariant reviewable at call sites."
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "The published constructor returns the session, not a Result, and the lifetime \
                  comes from validated server configuration; a checked fallback would silently \
                  shorten or fabricate the mandated deadline."
    )]
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
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::SecurityVersionMismatch`] when the session and the
    /// user disagree on the security version (the session is moved to
    /// `SecurityRevoked`); [`AuthError::DeviceRevoked`],
    /// [`AuthError::DeviceSuspended`], or
    /// [`AuthError::DeviceVerificationRequired`] when a bound device is not
    /// active; [`AuthError::SessionRevoked`] when the session is already
    /// revoked; and [`AuthError::SessionExpired`] when the session is not
    /// active, has reached its absolute lifetime, or has reached its idle
    /// deadline at `now`.
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
                crate::device::DeviceState::RegistrationStarted
                | crate::device::DeviceState::VerificationRequired
                | crate::device::DeviceState::Registered
                | crate::device::DeviceState::Active
                | crate::device::DeviceState::Replaced
                | crate::device::DeviceState::Retired => AuthError::DeviceVerificationRequired,
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
    ///
    /// The new deadline is the sum of `now` and the configured idle lifetime.
    /// The method returns `()`, and the lifetime is validated server
    /// configuration, so an unrepresentable deadline fails loudly rather than
    /// silently shortening the window.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "The published method returns (), and the idle lifetime is validated server \
                  configuration; a checked fallback would silently change the session lifetime."
    )]
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

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }

    fn session(lifetime: SessionLifetime) -> Result<(SystemTime, Session), AuthError> {
        let now = at(1_000);
        let s = Session::create(
            SessionId::new("sess-1")?,
            UserId::new("user-1")?,
            None,
            SessionClass::Merchant,
            AuthenticationMethod::Password,
            Assurance::A1,
            SecurityVersion(3),
            lifetime,
            now,
        );
        Ok((now, s))
    }

    fn lifetime() -> SessionLifetime {
        SessionLifetime {
            idle: Duration::from_secs(30),
            absolute: Duration::from_secs(60),
        }
    }

    #[test]
    fn active_session_is_accepted_and_slides() -> Result<(), AuthError> {
        let (_now, mut s) = session(lifetime())?;
        let later = at(1_020);
        s.accept(later, SecurityVersion(3), None)?;
        s.slide_idle(lifetime(), later);
        assert_eq!(s.last_seen_at, later);
        // Idle expiry must follow the slide, not the original issue time.
        s.accept(at(1_045), SecurityVersion(3), None)?;
        Ok(())
    }

    #[test]
    fn idle_timeout_expires_server_side() -> Result<(), AuthError> {
        let (_now, mut s) = session(lifetime())?;
        assert!(matches!(
            s.accept(at(1_031), SecurityVersion(3), None),
            Err(AuthError::SessionExpired)
        ));
        assert_eq!(s.state, SessionState::IdleExpired);
        Ok(())
    }

    #[test]
    fn absolute_timeout_wins_over_idle_activity() -> Result<(), AuthError> {
        let (_now, mut s) = session(lifetime())?;
        s.accept(at(1_010), SecurityVersion(3), None)?;
        assert!(matches!(
            s.accept(at(1_061), SecurityVersion(3), None),
            Err(AuthError::SessionExpired)
        ));
        assert_eq!(s.state, SessionState::AbsoluteExpired);
        Ok(())
    }

    #[test]
    fn security_version_mismatch_security_revokes() -> Result<(), AuthError> {
        let (now, mut s) = session(lifetime())?;
        assert!(matches!(
            s.accept(now, SecurityVersion(4), None),
            Err(AuthError::SecurityVersionMismatch)
        ));
        assert_eq!(s.state, SessionState::SecurityRevoked);
        Ok(())
    }

    #[test]
    fn revoked_device_blocks_device_bound_session() -> Result<(), AuthError> {
        let now = at(1_000);
        let mut s = Session::create(
            SessionId::new("sess-1")?,
            UserId::new("user-1")?,
            Some(DeviceId::new("dev-1")?),
            SessionClass::Merchant,
            AuthenticationMethod::Password,
            Assurance::A1,
            SecurityVersion(1),
            lifetime(),
            now,
        );
        assert!(matches!(
            s.accept(now, SecurityVersion(1), Some(DeviceState::Revoked)),
            Err(AuthError::DeviceRevoked)
        ));
        Ok(())
    }

    #[test]
    fn revoked_session_never_returns_to_active() -> Result<(), AuthError> {
        let (now, mut s) = session(lifetime())?;
        s.revoke(RevocationTrigger::Logout);
        assert_eq!(s.state, SessionState::Revoked);
        assert!(matches!(
            s.accept(now, SecurityVersion(3), None),
            Err(AuthError::SessionRevoked)
        ));
        s.revoke(RevocationTrigger::Logout);
        assert_eq!(s.state, SessionState::Revoked);
        Ok(())
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
    fn elevation_never_downgrades_assurance() -> Result<(), AuthError> {
        let (_, mut s) = session(lifetime())?;
        s.elevate_assurance(Assurance::A3);
        s.elevate_assurance(Assurance::A2);
        assert_eq!(s.assurance, Assurance::A3);
        Ok(())
    }
}
