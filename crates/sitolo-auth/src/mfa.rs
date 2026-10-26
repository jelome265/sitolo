//! MFA authenticator, enrollment, challenges, TOTP, and recovery codes.
//!
//! Phase 3 specification, §17-§20, Appendix A.3. MFA is not a UI flag; it is
//! a stateful workflow with distinct enrollment, challenge, verification,
//! replay, and recovery semantics. TOTP protocol machinery is delegated to a
//! maintained adapter; this module owns the lifecycle, replay guard, and
//! assurance mapping.
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use thiserror::Error;

use sitolo_security::SealedRef;

use crate::error::AuthError;
use crate::id::{
    ChallengeId, MfaAuthenticatorId, RecoveryCodeId, SecurityVersion, SessionId, UserId,
};

/// Failures raised by the MFA lifecycle (§17-§20).
#[derive(Debug, Error)]
pub enum MfaError {
    /// An active authenticator already exists for the user.
    #[error("mfa enrollment already active")]
    AlreadyActive,
    /// An enrollment is already in progress for the user.
    #[error("mfa enrollment in progress")]
    EnrollmentInProgress,
    /// The challenge window elapsed before consumption.
    #[error("mfa challenge expired")]
    ChallengeExpired,
    /// The challenge was already consumed.
    #[error("mfa challenge already consumed")]
    ChallengeConsumed,
    /// The challenge is bound to a different session.
    #[error("mfa challenge bound to different session")]
    ChallengeSessionMismatch,
}

/// MFA factor kind (§17.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaKind {
    /// Time-based one-time password (§18).
    Totp,
    /// Platform or roaming passkey.
    Passkey,
    /// Factor managed entirely by an external provider.
    ProviderManaged,
}

/// MFA authenticator state machine (§17.2, A.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaAuthenticatorState {
    /// Enrollment started but no proof has been verified.
    EnrollmentStarted,
    /// Enrollment needs a verification step before activation.
    VerificationRequired,
    /// The authenticator may satisfy MFA challenges.
    Active,
    /// The authenticator lapsed and must be re-enrolled.
    Expired,
    /// The authenticator was revoked; this state is terminal.
    Revoked,
}

/// An MFA authenticator record (§17).
#[derive(Debug, Clone)]
pub struct MfaAuthenticator {
    /// Identifier of the authenticator.
    pub id: MfaAuthenticatorId,
    /// User the authenticator belongs to.
    pub user_id: UserId,
    /// Factor kind.
    pub kind: MfaKind,
    /// Current lifecycle state.
    pub state: MfaAuthenticatorState,
    /// When enrollment started.
    pub created_at: SystemTime,
    /// When verification completed, if it did.
    pub verified_at: Option<SystemTime>,
    /// When the authenticator was revoked, if it was.
    pub revoked_at: Option<SystemTime>,
    /// Security version the authenticator is bound to (§31).
    pub security_version: SecurityVersion,
    /// Reference to sealed TOTP secret (§18, §34.2). None for non-TOTP kinds.
    pub secret_reference: Option<SealedRef>,
    /// Last accepted TOTP step for replay guard (§18).
    pub last_accepted_step: Option<u64>,
}

/// Challenge purpose (§7.1, §19.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengePurpose {
    /// Bind a new authenticator to the user.
    MfaEnrollment,
    /// Satisfy an MFA challenge during login.
    MfaVerification,
    /// Authorize a device registration.
    DeviceRegistration,
    /// Raise assurance for a sensitive operation.
    StepUp,
}

/// A single-use challenge (§7.1, §19.1).
#[derive(Debug, Clone)]
pub struct Challenge {
    /// Identifier of the challenge.
    pub id: ChallengeId,
    /// Why the challenge was issued.
    pub purpose: ChallengePurpose,
    /// User the challenge is bound to.
    pub user_id: UserId,
    /// Session the challenge is bound to.
    pub session_id: SessionId,
    /// When the challenge was issued.
    pub created_at: SystemTime,
    /// Hard expiry of the challenge.
    pub expires_at: SystemTime,
    /// Whether the challenge has already been consumed.
    pub consumed: bool,
}

impl Challenge {
    /// Atomically consumes the challenge (§7.1, §19.1). A second consumption
    /// returns `ChallengeConsumed`; expired challenges return `ChallengeExpired`.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::MfaEnrollmentExpired`] when `now` is at or past
    /// `expires_at`, [`AuthError::SessionInvalid`] when `session_id` does not
    /// match the challenge binding, and [`AuthError::RecoveryArtifactUsed`]
    /// when the challenge was already consumed.
    pub fn consume(&mut self, now: SystemTime, session_id: &SessionId) -> Result<(), AuthError> {
        if now >= self.expires_at {
            return Err(AuthError::MfaEnrollmentExpired);
        }
        if &self.session_id != session_id {
            return Err(AuthError::SessionInvalid);
        }
        if self.consumed {
            return Err(AuthError::RecoveryArtifactUsed);
        }
        self.consumed = true;
        Ok(())
    }
}

/// TOTP verification policy (§18).
#[derive(Debug, Clone, Copy)]
pub struct TotpPolicy {
    /// Length of one TOTP time step in seconds.
    pub period_secs: u64,
    /// Number of digits the code carries.
    pub digits: u32,
    /// Steps accepted on either side of the current step.
    pub window: u32,
}

/// TOTP verification port (§18, §6.1). Protocol machinery is delegated to a
/// maintained RFC 6238 implementation; this module owns the replay guard and
/// attempt counter.
#[async_trait]
pub trait TotpVerifier: Send + Sync {
    /// Verifies a TOTP code against the sealed secret. The `step` is the
    /// current time step; the implementation must check the window
    /// [step-window, step+window].
    ///
    /// # Errors
    ///
    /// Returns an error when the adapter cannot evaluate the code, for
    /// example because the sealed secret is unusable.
    async fn verify(
        &self,
        secret: &[u8],
        code: &str,
        step: u64,
        policy: &TotpPolicy,
    ) -> Result<bool, AuthError>;
}

/// Test-only TOTP verifier. Accepts codes of the form `{step:06}` zero-padded
/// within the window. Clearly labeled as a test double; production adapters
/// must use a maintained RFC 6238 implementation.
#[derive(Debug)]
pub struct TestTotpVerifier;

#[async_trait]
impl TotpVerifier for TestTotpVerifier {
    async fn verify(
        &self,
        _secret: &[u8],
        code: &str,
        step: u64,
        policy: &TotpPolicy,
    ) -> Result<bool, AuthError> {
        let window = u64::from(policy.window);
        for s in step.saturating_sub(window)..=step.saturating_add(window) {
            let expected = format!("{s:06}");
            if code == expected {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Recovery code record (§20).
#[derive(Debug, Clone)]
pub struct RecoveryCodeRecord {
    /// Identifier of the recovery code.
    pub id: RecoveryCodeId,
    /// User the code belongs to.
    pub user_id: UserId,
    /// Hash of the normalized recovery code.
    pub code_hash: [u8; 32],
    /// When the code was issued.
    pub created_at: SystemTime,
    /// When the code was consumed, if it was.
    pub consumed_at: Option<SystemTime>,
}

impl RecoveryCodeRecord {
    /// Normalizes a recovery code for comparison (§20).
    #[must_use]
    pub fn normalize(raw: &str) -> String {
        raw.trim()
            .to_uppercase()
            .replace(|c: char| !c.is_ascii_alphanumeric(), "")
    }

    /// Hashes a normalized recovery code (§20).
    #[must_use]
    pub fn hash_code(raw: &str) -> [u8; 32] {
        let normalized = Self::normalize(raw);
        Sha256::digest(normalized.as_bytes()).into()
    }

    /// Atomically consumes the recovery code (§20). A second redemption
    /// returns `RecoveryArtifactUsed`.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::RecoveryArtifactInvalid`] when the presented code
    /// hash does not match, and [`AuthError::RecoveryArtifactUsed`] when the
    /// code was already consumed.
    pub fn consume(&mut self, raw: &str, now: SystemTime) -> Result<(), AuthError> {
        let presented = Self::hash_code(raw);
        if presented != self.code_hash {
            return Err(AuthError::RecoveryArtifactInvalid);
        }
        if self.consumed_at.is_some() {
            return Err(AuthError::RecoveryArtifactUsed);
        }
        self.consumed_at = Some(now);
        Ok(())
    }
}

/// In-memory reference state for MFA authenticators, challenges, and recovery
/// codes. Production persistence replaces this with a transactional
/// repository.
#[derive(Debug, Default)]
pub struct MfaState {
    authenticators: BTreeMap<MfaAuthenticatorId, MfaAuthenticator>,
    by_user: BTreeMap<UserId, Vec<MfaAuthenticatorId>>,
    challenges: BTreeMap<ChallengeId, Challenge>,
    recovery_codes: BTreeMap<RecoveryCodeId, RecoveryCodeRecord>,
    recovery_by_user: BTreeMap<UserId, Vec<RecoveryCodeId>>,
}

impl MfaState {
    /// Creates empty MFA state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begins MFA enrollment (§18.1). Rejects if an active authenticator
    /// exists (must reset first) or an enrollment is in progress.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::MfaEnrollmentConflict`] when the user already has
    /// an active authenticator or an enrollment in progress, and
    /// [`AuthError::IdentityAuthorityUnavailable`] when an indexed
    /// authenticator record is missing.
    pub fn begin_enrollment(
        &mut self,
        id: MfaAuthenticatorId,
        user_id: UserId,
        kind: MfaKind,
        secret_reference: Option<SealedRef>,
        now: SystemTime,
        security_version: SecurityVersion,
    ) -> Result<(), AuthError> {
        let existing = self.by_user.get(&user_id).cloned().unwrap_or_default();
        for auth_id in existing {
            let auth = self
                .authenticators
                .get(&auth_id)
                .ok_or(AuthError::IdentityAuthorityUnavailable)?;
            if auth.state == MfaAuthenticatorState::Active {
                return Err(AuthError::MfaEnrollmentConflict);
            }
            if matches!(
                auth.state,
                MfaAuthenticatorState::EnrollmentStarted
                    | MfaAuthenticatorState::VerificationRequired
            ) {
                return Err(AuthError::MfaEnrollmentConflict);
            }
        }
        let auth = MfaAuthenticator {
            id: id.clone(),
            user_id: user_id.clone(),
            kind,
            state: MfaAuthenticatorState::EnrollmentStarted,
            created_at: now,
            verified_at: None,
            revoked_at: None,
            security_version,
            secret_reference,
            last_accepted_step: None,
        };
        self.authenticators.insert(id.clone(), auth);
        self.by_user.entry(user_id).or_default().push(id);
        Ok(())
    }

    /// Advances enrollment to verification-required (§17.2, A.3).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidIdentifier`] when the authenticator is
    /// unknown, and [`AuthError::InvalidTransition`] when it is not in
    /// `EnrollmentStarted`.
    pub fn require_verification(&mut self, id: &MfaAuthenticatorId) -> Result<(), AuthError> {
        let auth = self
            .authenticators
            .get_mut(id)
            .ok_or(AuthError::InvalidIdentifier)?;
        if auth.state != MfaAuthenticatorState::EnrollmentStarted {
            return Err(AuthError::InvalidTransition);
        }
        auth.state = MfaAuthenticatorState::VerificationRequired;
        Ok(())
    }

    /// Completes enrollment after successful verification (§18.1).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidIdentifier`] when the authenticator is
    /// unknown, and [`AuthError::InvalidTransition`] when it is not in
    /// `VerificationRequired`.
    pub fn complete_enrollment(
        &mut self,
        id: &MfaAuthenticatorId,
        now: SystemTime,
    ) -> Result<(), AuthError> {
        let auth = self
            .authenticators
            .get_mut(id)
            .ok_or(AuthError::InvalidIdentifier)?;
        if auth.state != MfaAuthenticatorState::VerificationRequired {
            return Err(AuthError::InvalidTransition);
        }
        auth.state = MfaAuthenticatorState::Active;
        auth.verified_at = Some(now);
        Ok(())
    }

    /// Revokes an authenticator (§17.2).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidIdentifier`] when the authenticator is
    /// unknown, and [`AuthError::InvalidTransition`] when it is already
    /// revoked.
    pub fn revoke(&mut self, id: &MfaAuthenticatorId, now: SystemTime) -> Result<(), AuthError> {
        let auth = self
            .authenticators
            .get_mut(id)
            .ok_or(AuthError::InvalidIdentifier)?;
        if auth.state == MfaAuthenticatorState::Revoked {
            return Err(AuthError::InvalidTransition);
        }
        auth.state = MfaAuthenticatorState::Revoked;
        auth.revoked_at = Some(now);
        Ok(())
    }

    /// Returns the active authenticator for a user, if any.
    #[must_use]
    pub fn active_authenticator(&self, user_id: &UserId) -> Option<&MfaAuthenticator> {
        let ids = self.by_user.get(user_id)?;
        for id in ids {
            let auth = self.authenticators.get(id)?;
            if auth.state == MfaAuthenticatorState::Active {
                return Some(auth);
            }
        }
        None
    }

    /// Records a successful TOTP verification and updates the replay guard
    /// (§18). Returns the updated authenticator.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidIdentifier`] when the authenticator is
    /// unknown, and [`AuthError::MfaFailed`] when `step` is not greater than
    /// the last accepted step (a replay).
    pub fn record_totp_success(
        &mut self,
        id: &MfaAuthenticatorId,
        step: u64,
    ) -> Result<(), AuthError> {
        let auth = self
            .authenticators
            .get_mut(id)
            .ok_or(AuthError::InvalidIdentifier)?;
        if let Some(last) = auth.last_accepted_step
            && step <= last
        {
            return Err(AuthError::MfaFailed);
        }
        auth.last_accepted_step = Some(step);
        Ok(())
    }

    /// Issues a challenge (§7.1, §19.1).
    ///
    /// The expiry is the sum of `now` and `ttl`. The method returns `()`, and
    /// both operands are server-controlled, so an unrepresentable expiry fails
    /// loudly rather than silently fabricating a different deadline.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "The published method returns (), and the TTL is server configuration; a \
                  checked fallback would silently change the challenge window."
    )]
    pub fn issue_challenge(
        &mut self,
        id: ChallengeId,
        purpose: ChallengePurpose,
        user_id: UserId,
        session_id: SessionId,
        now: SystemTime,
        ttl: Duration,
    ) {
        let challenge = Challenge {
            id: id.clone(),
            purpose,
            user_id,
            session_id,
            created_at: now,
            expires_at: now + ttl,
            consumed: false,
        };
        self.challenges.insert(id, challenge);
    }

    /// Consumes a challenge (§7.1, §19.1).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidIdentifier`] when the challenge is unknown,
    /// plus the failures documented by [`Challenge::consume`]:
    /// [`AuthError::MfaEnrollmentExpired`], [`AuthError::SessionInvalid`], and
    /// [`AuthError::RecoveryArtifactUsed`].
    pub fn consume_challenge(
        &mut self,
        id: &ChallengeId,
        now: SystemTime,
        session_id: &SessionId,
    ) -> Result<(), AuthError> {
        let challenge = self
            .challenges
            .get_mut(id)
            .ok_or(AuthError::InvalidIdentifier)?;
        challenge.consume(now, session_id)
    }

    /// Adds a recovery code (§20).
    pub fn add_recovery_code(&mut self, record: RecoveryCodeRecord) {
        let id = record.id.clone();
        let user_id = record.user_id.clone();
        self.recovery_codes.insert(id.clone(), record);
        self.recovery_by_user.entry(user_id).or_default().push(id);
    }

    /// Consumes a recovery code (§20).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::RecoveryArtifactInvalid`] when no stored code for
    /// the user matches `raw`, [`AuthError::RecoveryArtifactUsed`] when the
    /// matching code was already consumed, and
    /// [`AuthError::IdentityAuthorityUnavailable`] when an indexed recovery
    /// record is missing.
    pub fn consume_recovery_code(
        &mut self,
        user_id: &UserId,
        raw: &str,
        now: SystemTime,
    ) -> Result<RecoveryCodeId, AuthError> {
        let ids = self
            .recovery_by_user
            .get(user_id)
            .cloned()
            .unwrap_or_default();
        let hash = RecoveryCodeRecord::hash_code(raw);
        for id in ids {
            let record = self
                .recovery_codes
                .get_mut(&id)
                .ok_or(AuthError::IdentityAuthorityUnavailable)?;
            if hash == record.code_hash {
                if record.consumed_at.is_some() {
                    return Err(AuthError::RecoveryArtifactUsed);
                }
                record.consume(raw, now)?;
                return Ok(id);
            }
        }
        Err(AuthError::RecoveryArtifactInvalid)
    }

    /// Resets all MFA state for a user (§17.2). Revokes all authenticators
    /// and recovery codes; returns the list of revoked authenticator IDs.
    pub fn reset_all(&mut self, user_id: &UserId, now: SystemTime) -> Vec<MfaAuthenticatorId> {
        let ids = self.by_user.remove(user_id).unwrap_or_default();
        let mut revoked = Vec::new();
        for id in ids {
            if let Some(auth) = self.authenticators.get_mut(&id)
                && auth.state != MfaAuthenticatorState::Revoked
            {
                auth.state = MfaAuthenticatorState::Revoked;
                auth.revoked_at = Some(now);
                revoked.push(id);
            }
        }
        let recovery_ids = self.recovery_by_user.remove(user_id).unwrap_or_default();
        for id in recovery_ids {
            if let Some(record) = self.recovery_codes.get_mut(&id) {
                record.consumed_at = Some(now);
            }
        }
        revoked
    }

    /// Returns the authenticator with the given identifier, if it exists.
    #[must_use]
    pub fn authenticator(&self, id: &MfaAuthenticatorId) -> Option<&MfaAuthenticator> {
        self.authenticators.get(id)
    }

    /// Returns a mutable authenticator with the given identifier, if it
    /// exists.
    pub fn authenticator_mut(&mut self, id: &MfaAuthenticatorId) -> Option<&mut MfaAuthenticator> {
        self.authenticators.get_mut(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }

    fn setup() -> Result<(MfaState, SystemTime, MfaAuthenticatorId, UserId), AuthError> {
        let mut state = MfaState::new();
        let now = at(1_000);
        let user = UserId::new("user-1")?;
        let id = MfaAuthenticatorId::new("mfa-1")?;
        state.begin_enrollment(
            id.clone(),
            user.clone(),
            MfaKind::Totp,
            None,
            now,
            SecurityVersion(1),
        )?;
        Ok((state, now, id, user))
    }

    #[test]
    fn enrollment_lifecycle() -> Result<(), AuthError> {
        let (mut state, now, id, _) = setup()?;
        assert_eq!(
            state
                .authenticator(&id)
                .ok_or(AuthError::InvalidIdentifier)?
                .state,
            MfaAuthenticatorState::EnrollmentStarted
        );
        state.require_verification(&id)?;
        assert_eq!(
            state
                .authenticator(&id)
                .ok_or(AuthError::InvalidIdentifier)?
                .state,
            MfaAuthenticatorState::VerificationRequired
        );
        state.complete_enrollment(&id, now)?;
        assert_eq!(
            state
                .authenticator(&id)
                .ok_or(AuthError::InvalidIdentifier)?
                .state,
            MfaAuthenticatorState::Active
        );
        Ok(())
    }

    #[test]
    fn duplicate_enrollment_is_rejected() -> Result<(), AuthError> {
        let (mut state, now, _, user) = setup()?;
        let id2 = MfaAuthenticatorId::new("mfa-2")?;
        assert!(matches!(
            state.begin_enrollment(id2, user, MfaKind::Totp, None, now, SecurityVersion(1)),
            Err(AuthError::MfaEnrollmentConflict)
        ));
        Ok(())
    }

    #[test]
    fn totp_replay_guard() -> Result<(), AuthError> {
        let (mut state, _, id, _) = setup()?;
        state.require_verification(&id)?;
        state.complete_enrollment(&id, at(1_000))?;
        state.record_totp_success(&id, 100)?;
        assert!(matches!(
            state.record_totp_success(&id, 100),
            Err(AuthError::MfaFailed)
        ));
        assert!(state.record_totp_success(&id, 101).is_ok());
        Ok(())
    }

    #[test]
    fn recovery_code_single_use() -> Result<(), AuthError> {
        let mut state = MfaState::new();
        let now = at(1_000);
        let user = UserId::new("user-1")?;
        let id = RecoveryCodeId::new("rec-1")?;
        let record = RecoveryCodeRecord {
            id,
            user_id: user.clone(),
            code_hash: RecoveryCodeRecord::hash_code("ABCD-1234"),
            created_at: now,
            consumed_at: None,
        };
        state.add_recovery_code(record);
        assert!(state.consume_recovery_code(&user, "ABCD-1234", now).is_ok());
        assert!(matches!(
            state.consume_recovery_code(&user, "ABCD-1234", now),
            Err(AuthError::RecoveryArtifactUsed)
        ));
        Ok(())
    }

    #[test]
    fn reset_all_revokes_authenticators_and_codes() -> Result<(), AuthError> {
        let (mut state, now, id, user) = setup()?;
        state.require_verification(&id)?;
        state.complete_enrollment(&id, now)?;
        let rec_id = RecoveryCodeId::new("rec-1")?;
        state.add_recovery_code(RecoveryCodeRecord {
            id: rec_id,
            user_id: user.clone(),
            code_hash: RecoveryCodeRecord::hash_code("ABCD"),
            created_at: now,
            consumed_at: None,
        });
        let revoked = state.reset_all(&user, now);
        assert_eq!(revoked, vec![id.clone()]);
        assert_eq!(
            state
                .authenticator(&id)
                .ok_or(AuthError::InvalidIdentifier)?
                .state,
            MfaAuthenticatorState::Revoked
        );
        Ok(())
    }
}
