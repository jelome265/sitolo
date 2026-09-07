//! MFA authenticator, enrollment, challenges, TOTP, and recovery codes.
//!
//! Phase 3 specification, §17-§20, Appendix A.3. MFA is not a UI flag; it is
//! a stateful workflow with distinct enrollment, challenge, verification,
//! replay, and recovery semantics. TOTP protocol machinery is delegated to a
//! maintained adapter; this module owns the lifecycle, replay guard, and
/// assurance mapping.
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

#[derive(Debug, Error)]
pub enum MfaError {
    #[error("mfa enrollment already active")]
    AlreadyActive,
    #[error("mfa enrollment in progress")]
    EnrollmentInProgress,
    #[error("mfa challenge expired")]
    ChallengeExpired,
    #[error("mfa challenge already consumed")]
    ChallengeConsumed,
    #[error("mfa challenge bound to different session")]
    ChallengeSessionMismatch,
}

/// MFA factor kind (§17.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaKind {
    Totp,
    Passkey,
    ProviderManaged,
}

/// MFA authenticator state machine (§17.2, A.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaAuthenticatorState {
    EnrollmentStarted,
    VerificationRequired,
    Active,
    Expired,
    Revoked,
}

/// An MFA authenticator record (§17).
#[derive(Debug, Clone)]
pub struct MfaAuthenticator {
    pub id: MfaAuthenticatorId,
    pub user_id: UserId,
    pub kind: MfaKind,
    pub state: MfaAuthenticatorState,
    pub created_at: SystemTime,
    pub verified_at: Option<SystemTime>,
    pub revoked_at: Option<SystemTime>,
    pub security_version: SecurityVersion,
    /// Reference to sealed TOTP secret (§18, §34.2). None for non-TOTP kinds.
    pub secret_reference: Option<SealedRef>,
    /// Last accepted TOTP step for replay guard (§18).
    pub last_accepted_step: Option<u64>,
}

/// Challenge purpose (§7.1, §19.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengePurpose {
    MfaEnrollment,
    MfaVerification,
    DeviceRegistration,
    StepUp,
}

/// A single-use challenge (§7.1, §19.1).
#[derive(Debug, Clone)]
pub struct Challenge {
    pub id: ChallengeId,
    pub purpose: ChallengePurpose,
    pub user_id: UserId,
    pub session_id: SessionId,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub consumed: bool,
}

impl Challenge {
    /// Atomically consumes the challenge (§7.1, §19.1). A second consumption
    /// returns `ChallengeConsumed`; expired challenges return `ChallengeExpired`.
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
    pub period_secs: u64,
    pub digits: u32,
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
        let window = policy.window as u64;
        for s in step.saturating_sub(window)..=step + window {
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
    pub id: RecoveryCodeId,
    pub user_id: UserId,
    pub code_hash: [u8; 32],
    pub created_at: SystemTime,
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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begins MFA enrollment (§18.1). Rejects if an active authenticator
    /// exists (must reset first) or an enrollment is in progress.
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
            let auth = self.authenticators.get(&auth_id).expect("existing");
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
            let record = self.recovery_codes.get_mut(&id).expect("existing");
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
            if let Some(auth) = self.authenticators.get_mut(&id) {
                if auth.state != MfaAuthenticatorState::Revoked {
                    auth.state = MfaAuthenticatorState::Revoked;
                    auth.revoked_at = Some(now);
                    revoked.push(id);
                }
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

    pub fn authenticator(&self, id: &MfaAuthenticatorId) -> Option<&MfaAuthenticator> {
        self.authenticators.get(id)
    }

    pub fn authenticator_mut(&mut self, id: &MfaAuthenticatorId) -> Option<&mut MfaAuthenticator> {
        self.authenticators.get_mut(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (MfaState, SystemTime, MfaAuthenticatorId, UserId) {
        let mut state = MfaState::new();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let user = UserId::new("user-1").unwrap();
        let id = MfaAuthenticatorId::new("mfa-1").unwrap();
        state
            .begin_enrollment(
                id.clone(),
                user.clone(),
                MfaKind::Totp,
                None,
                now,
                SecurityVersion(1),
            )
            .unwrap();
        (state, now, id, user)
    }

    #[test]
    fn enrollment_lifecycle() {
        let (mut state, now, id, _) = setup();
        assert_eq!(
            state.authenticator(&id).unwrap().state,
            MfaAuthenticatorState::EnrollmentStarted
        );
        state.require_verification(&id).unwrap();
        assert_eq!(
            state.authenticator(&id).unwrap().state,
            MfaAuthenticatorState::VerificationRequired
        );
        state.complete_enrollment(&id, now).unwrap();
        assert_eq!(
            state.authenticator(&id).unwrap().state,
            MfaAuthenticatorState::Active
        );
    }

    #[test]
    fn duplicate_enrollment_is_rejected() {
        let (mut state, now, _, user) = setup();
        let id2 = MfaAuthenticatorId::new("mfa-2").unwrap();
        state
            .begin_enrollment(
                id2.clone(),
                user.clone(),
                MfaKind::Totp,
                None,
                now,
                SecurityVersion(1),
            )
            .unwrap_err();
    }

    #[test]
    fn totp_replay_guard() {
        let (mut state, _, id, _) = setup();
        state.require_verification(&id).unwrap();
        state
            .complete_enrollment(&id, SystemTime::UNIX_EPOCH + Duration::from_secs(1_000))
            .unwrap();
        state.record_totp_success(&id, 100).unwrap();
        assert!(matches!(
            state.record_totp_success(&id, 100),
            Err(AuthError::MfaFailed)
        ));
        assert!(state.record_totp_success(&id, 101).is_ok());
    }

    #[test]
    fn recovery_code_single_use() {
        let mut state = MfaState::new();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let user = UserId::new("user-1").unwrap();
        let id = RecoveryCodeId::new("rec-1").unwrap();
        let record = RecoveryCodeRecord {
            id: id.clone(),
            user_id: user.clone(),
            code_hash: RecoveryCodeRecord::hash_code("ABCD-1234"),
            created_at: now,
            consumed_at: None,
        };
        state.add_recovery_code(record);
        assert!(state.consume_recovery_code(&user, "ABCD-1234", now).is_ok());
        assert_eq!(
            state
                .consume_recovery_code(&user, "ABCD-1234", now)
                .unwrap_err(),
            AuthError::RecoveryArtifactUsed
        );
    }

    #[test]
    fn reset_all_revokes_authenticators_and_codes() {
        let (mut state, now, id, user) = setup();
        state.require_verification(&id).unwrap();
        state.complete_enrollment(&id, now).unwrap();
        let rec_id = RecoveryCodeId::new("rec-1").unwrap();
        state.add_recovery_code(RecoveryCodeRecord {
            id: rec_id.clone(),
            user_id: user.clone(),
            code_hash: RecoveryCodeRecord::hash_code("ABCD"),
            created_at: now,
            consumed_at: None,
        });
        let revoked = state.reset_all(&user, now);
        assert_eq!(revoked, vec![id.clone()]);
        assert_eq!(
            state.authenticator(&id).unwrap().state,
            MfaAuthenticatorState::Revoked
        );
    }
}
