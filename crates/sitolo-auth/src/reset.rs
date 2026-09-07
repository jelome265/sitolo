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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetArtifactState {
    Pending,
    Consumed,
    Expired,
    Revoked,
}

/// A password reset artifact (§21.1). The token hash is stored; the raw
/// token is returned to the caller exactly once and never persisted.
#[derive(Debug, Clone)]
pub struct PasswordResetArtifact {
    pub id: ResetArtifactId,
    pub user_id: UserId,
    pub token_hash: [u8; 32],
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub state: ResetArtifactState,
    pub bound_security_version: SecurityVersion,
}

impl PasswordResetArtifact {
    /// Hashes a raw token into the stored representation (§21.1).
    #[must_use]
    pub fn hash_token(raw: &str) -> [u8; 32] {
        Sha256::digest(raw.as_bytes()).into()
    }

    /// Creates a pending artifact bound to the current security version.
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

    pub fn revoke(&mut self) {
        if self.state == ResetArtifactState::Pending {
            self.state = ResetArtifactState::Revoked;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact() -> PasswordResetArtifact {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        PasswordResetArtifact::issue(
            ResetArtifactId::new("reset-1").unwrap(),
            UserId::new("user-1").unwrap(),
            "raw-token-abc",
            now,
            Duration::from_secs(600),
            SecurityVersion(3),
        )
    }

    #[test]
    fn single_use_redemption() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_100);
        let mut a = artifact();
        assert!(a.claim("raw-token-abc", now, SecurityVersion(3)).is_ok());
        assert_eq!(a.state, ResetArtifactState::Consumed);
        assert_eq!(
            a.claim("raw-token-abc", now, SecurityVersion(3))
                .unwrap_err(),
            AuthError::RecoveryArtifactUsed
        );
    }

    #[test]
    fn wrong_token_is_rejected() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_100);
        let mut a = artifact();
        assert_eq!(
            a.claim("wrong", now, SecurityVersion(3)).unwrap_err(),
            AuthError::RecoveryArtifactInvalid
        );
    }

    #[test]
    fn expired_artifact_is_rejected() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000);
        let mut a = artifact();
        assert_eq!(
            a.claim("raw-token-abc", now, SecurityVersion(3))
                .unwrap_err(),
            AuthError::RecoveryArtifactInvalid
        );
        assert_eq!(a.state, ResetArtifactState::Expired);
    }

    #[test]
    fn security_version_mismatch_blocks_use() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_100);
        let mut a = artifact();
        assert_eq!(
            a.claim("raw-token-abc", now, SecurityVersion(4))
                .unwrap_err(),
            AuthError::RecoveryArtifactInvalid
        );
    }
}
