//! PKCE and authorization transaction state.
//!
//! Phase 3 specification, §7.2, §7.4. PKCE uses S256; the verifier is never
//! transmitted until code redemption. Authorization state is one-time and
//! short-lived.

use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use sitolo_security::RandomSource;

use crate::error::AuthError;
use crate::id::{base64url, hex};

/// PKCE verifier (§7.2).
pub struct PkceVerifier(String);

impl PkceVerifier {
    /// Generates a PKCE verifier and challenge (§7.2).
    #[must_use]
    pub fn generate(rng: &dyn RandomSource) -> (Self, String) {
        let bytes = rng.bytes(32);
        let verifier = base64url(&bytes);
        let challenge = Self::challenge(&verifier);
        (PkceVerifier(verifier), challenge)
    }

    /// Computes the S256 challenge from a verifier (§7.2).
    #[must_use]
    pub fn challenge(verifier: &str) -> String {
        let digest = Sha256::digest(verifier.as_bytes());
        base64url(&digest)
    }

    /// Verifies a presented verifier against a stored challenge (§7.2).
    #[must_use]
    pub fn verify(verifier: &str, challenge: &str) -> bool {
        Self::challenge(verifier) == challenge
    }

    /// Exposes the verifier for code redemption (§7.2).
    #[must_use]
    pub fn expose(self) -> String {
        self.0
    }
}

/// Authorization transaction state (§7.1, §7.4).
#[derive(Debug, Clone)]
pub struct AuthorizationTransaction {
    pub state: String,
    pub challenge: String,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub completed: bool,
}

/// Authorization transaction log (§7.1, §7.4).
#[derive(Debug, Default)]
pub struct AuthorizationTransactionLog {
    transactions: std::collections::BTreeMap<String, AuthorizationTransaction>,
}

impl AuthorizationTransactionLog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begins an authorization transaction (§7.1, §7.4).
    #[must_use]
    pub fn begin(
        &mut self,
        rng: &dyn RandomSource,
        challenge: String,
        now: SystemTime,
        ttl: Duration,
    ) -> AuthorizationTransaction {
        let state_bytes = rng.bytes(16);
        let state = hex(&state_bytes);
        let txn = AuthorizationTransaction {
            state: state.clone(),
            challenge,
            created_at: now,
            expires_at: now + ttl,
            completed: false,
        };
        self.transactions.insert(state.clone(), txn.clone());
        txn
    }

    /// Completes a callback (§7.1, §7.4). Rejects unsolicited callbacks
    /// (unknown state), expired transactions, and replays.
    pub fn complete(
        &mut self,
        state: &str,
        now: SystemTime,
    ) -> Result<AuthorizationTransaction, AuthError> {
        let txn = self
            .transactions
            .get_mut(state)
            .ok_or(AuthError::AuthenticationFailed)?;
        if now >= txn.expires_at {
            return Err(AuthError::AuthenticationFailed);
        }
        if txn.completed {
            return Err(AuthError::AuthenticationFailed);
        }
        txn.completed = true;
        Ok(txn.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_security::DeterministicRandom;

    #[test]
    fn pkce_s256_roundtrip() {
        let rng = DeterministicRandom::test_only([1u8; 32]);
        let (verifier, challenge) = PkceVerifier::generate(&rng);
        assert!(PkceVerifier::verify(&verifier.expose(), &challenge));
        assert!(!PkceVerifier::verify("wrong", &challenge));
    }

    #[test]
    fn authorization_transaction_single_use() {
        let rng = DeterministicRandom::test_only([2u8; 32]);
        let mut log = AuthorizationTransactionLog::new();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let txn = log.begin(&rng, "challenge".into(), now, Duration::from_secs(60));
        let completed = log.complete(&txn.state, now).unwrap();
        assert!(completed.completed);
        // Replay rejected.
        assert!(log.complete(&txn.state, now).is_err());
    }

    #[test]
    fn unsolicited_callback_rejected() {
        let mut log = AuthorizationTransactionLog::new();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        assert!(log.complete("unknown", now).is_err());
    }

    #[test]
    fn expired_transaction_rejected() {
        let rng = DeterministicRandom::test_only([3u8; 32]);
        let mut log = AuthorizationTransactionLog::new();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let txn = log.begin(&rng, "challenge".into(), now, Duration::from_secs(60));
        let later = now + Duration::from_secs(61);
        assert!(log.complete(&txn.state, later).is_err());
    }
}
