//! Password reset artifacts.
//!
//! Phase 3 specification, §21. A reset is an authentication path and must be
//! protected like login: single-use, short-lived, bound to the intended
//! account, stored as a hash, invalidated after use, unusable after a
//! security-version change (§21.1).

use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use crate::error::AuthError;
use crate::id::{ResetArtifactId, SecurityVersion, UserId};

/// Lifecycle state of a password reset artifact (§21.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetArtifactState {
    /// The artifact may still be claimed.
    Pending,
    /// The artifact was successfully redeemed.
    Consumed,
    /// The artifact passed its expiry before redemption.
    Expired,
    /// The artifact was revoked before redemption.
    Revoked,
}

/// A password reset artifact (§21.1). The token hash is stored; the raw
/// token is returned to the caller exactly once and never persisted.
#[derive(Debug, Clone)]
pub struct PasswordResetArtifact {
    /// Identifier of the artifact.
    pub id: ResetArtifactId,
    /// User the artifact is bound to.
    pub user_id: UserId,
    /// SHA-256 hash of the raw reset token.
    pub token_hash: [u8; 32],
    /// When the artifact was issued.
    pub created_at: SystemTime,
    /// Hard expiry of the artifact.
    pub expires_at: SystemTime,
    /// Current lifecycle state.
    pub state: ResetArtifactState,
    /// Security version the artifact is bound to (§21.1, §31).
    pub bound_security_version: SecurityVersion,
}

impl PasswordResetArtifact {
    /// Hashes a raw token into the stored representation (§21.1).
    #[must_use]
    pub fn hash_token(raw: &str) -> [u8; 32] {
        Sha256::digest(raw.as_bytes()).into()
    }

    /// Creates a pending artifact bound to the current security version.
    ///
    /// The expiry is the sum of `now` and `ttl`. Both are server-controlled,
    /// and the constructor cannot report an unrepresentable expiry, so the
    /// deadline is computed with the same panicking addition used by the rest
    /// of the crate rather than silently fabricating a different deadline.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "The published constructor returns the artifact, not a Result, and the \
                  operands are server-controlled; a checked fallback would silently move the \
                  reset window instead of failing loudly."
    )]
    #[must_use]
    pub fn issue(
        id: ResetArtifactId,
        user_id: UserId,
        raw_token: &str,
        now: SystemTime,
        ttl: Duration,
        bound_version: SecurityVersion,
    ) -> Self {
        PasswordResetArtifact {
            id,
            user_id,
            token_hash: Self::hash_token(raw_token),
            created_at: now,
            expires_at: now + ttl,
            state: ResetArtifactState::Pending,
            bound_security_version: bound_version,
        }
    }

    /// Atomically claims the artifact for redemption (§21.2). A second
    /// redemption attempt returns `RecoveryArtifactUsed` (§21.2, Appendix B).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::RecoveryArtifactInvalid`] when the presented token
    /// hash does not match, when `now` is at or past `expires_at` (the state
    /// is also moved to `Expired`), when the artifact is expired or revoked,
    /// or when `current_version` differs from the bound security version.
    /// Returns [`AuthError::RecoveryArtifactUsed`] when the artifact was
    /// already consumed.
    pub fn claim(
        &mut self,
        raw_token: &str,
        now: SystemTime,
        current_version: SecurityVersion,
    ) -> Result<(), AuthError> {
        let presented = Self::hash_token(raw_token);
        if presented != self.token_hash {
            return Err(AuthError::RecoveryArtifactInvalid);
        }
        if now >= self.expires_at {
            self.state = ResetArtifactState::Expired;
            return Err(AuthError::RecoveryArtifactInvalid);
        }
        if current_version != self.bound_security_version {
            return Err(AuthError::RecoveryArtifactInvalid);
        }
        match self.state {
            ResetArtifactState::Pending => {
                self.state = ResetArtifactState::Consumed;
                Ok(())
            }
            ResetArtifactState::Consumed => Err(AuthError::RecoveryArtifactUsed),
            ResetArtifactState::Expired | ResetArtifactState::Revoked => {
                Err(AuthError::RecoveryArtifactInvalid)
            }
        }
    }

    /// Revokes a pending artifact. Consumed, expired, and already-revoked
    /// artifacts are left unchanged.
    pub fn revoke(&mut self) {
        if self.state == ResetArtifactState::Pending {
            self.state = ResetArtifactState::Revoked;
        }
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

    fn artifact() -> Result<PasswordResetArtifact, AuthError> {
        let now = at(1_000);
        Ok(PasswordResetArtifact::issue(
            ResetArtifactId::new("reset-1")?,
            UserId::new("user-1")?,
            "raw-token-abc",
            now,
            Duration::from_secs(600),
            SecurityVersion(3),
        ))
    }

    #[test]
    fn single_use_redemption() -> Result<(), AuthError> {
        let now = at(1_100);
        let mut a = artifact()?;
        assert!(a.claim("raw-token-abc", now, SecurityVersion(3)).is_ok());
        assert_eq!(a.state, ResetArtifactState::Consumed);
        assert_eq!(
            a.claim("raw-token-abc", now, SecurityVersion(3)),
            Err(AuthError::RecoveryArtifactUsed)
        );
        Ok(())
    }

    #[test]
    fn wrong_token_is_rejected() -> Result<(), AuthError> {
        let now = at(1_100);
        let mut a = artifact()?;
        assert!(matches!(
            a.claim("wrong", now, SecurityVersion(3)),
            Err(AuthError::RecoveryArtifactInvalid)
        ));
        Ok(())
    }

    #[test]
    fn expired_artifact_is_rejected() -> Result<(), AuthError> {
        let now = at(2_000);
        let mut a = artifact()?;
        assert!(matches!(
            a.claim("raw-token-abc", now, SecurityVersion(3)),
            Err(AuthError::RecoveryArtifactInvalid)
        ));
        assert_eq!(a.state, ResetArtifactState::Expired);
        Ok(())
    }

    #[test]
    fn security_version_mismatch_blocks_use() -> Result<(), AuthError> {
        let now = at(1_100);
        let mut a = artifact()?;
        assert!(matches!(
            a.claim("raw-token-abc", now, SecurityVersion(4)),
            Err(AuthError::RecoveryArtifactInvalid)
        ));
        Ok(())
    }
}
