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

impl std::fmt::Debug for PkceVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PkceVerifier(redacted)")
    }
}

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
    /// Opaque one-time state value echoed by the authorization server.
    pub state: String,
    /// S256 challenge bound to this transaction.
    pub challenge: String,
    /// When the transaction was created.
    pub created_at: SystemTime,
    /// Hard expiry of the transaction.
    pub expires_at: SystemTime,
    /// Whether the callback has already been completed.
    pub completed: bool,
}

/// Authorization transaction log (§7.1, §7.4).
#[derive(Debug, Default)]
pub struct AuthorizationTransactionLog {
    transactions: std::collections::BTreeMap<String, AuthorizationTransaction>,
}

impl AuthorizationTransactionLog {
    /// Creates an empty transaction log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begins an authorization transaction (§7.1, §7.4).
    ///
    /// The transaction expiry is the sum of `now` and `ttl`. Both are
    /// server-controlled values, and the signature cannot report an
    /// unrepresentable expiry, so the deadline is computed with the same
    /// panicking addition used by the rest of the crate rather than silently
    /// fabricating a different deadline.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "The published signature returns a transaction, not a Result, and the \
                  operands are server-controlled; a checked fallback would silently move \
                  the deadline instead of failing loudly."
    )]
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
        self.transactions.insert(state, txn.clone());
        txn
    }

    /// Completes a callback (§7.1, §7.4). Rejects unsolicited callbacks
    /// (unknown state), expired transactions, and replays.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::AuthenticationFailed`] when `state` is unknown,
    /// when the transaction has expired at `now`, or when it has already been
    /// completed.
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

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }

    #[test]
    fn pkce_s256_roundtrip() {
        let rng = DeterministicRandom::test_only([1u8; 32]);
        let (verifier, challenge) = PkceVerifier::generate(&rng);
        assert!(PkceVerifier::verify(&verifier.expose(), &challenge));
        assert!(!PkceVerifier::verify("wrong", &challenge));
    }

    #[test]
    fn authorization_transaction_single_use() -> Result<(), AuthError> {
        let rng = DeterministicRandom::test_only([2u8; 32]);
        let mut log = AuthorizationTransactionLog::new();
        let now = at(1_000);
        let txn = log.begin(&rng, "challenge".into(), now, Duration::from_secs(60));
        let completed = log.complete(&txn.state, now)?;
        assert!(completed.completed);
        // Replay rejected.
        assert!(log.complete(&txn.state, now).is_err());
        Ok(())
    }

    #[test]
    fn unsolicited_callback_rejected() {
        let mut log = AuthorizationTransactionLog::new();
        let now = at(1_000);
        assert!(log.complete("unknown", now).is_err());
    }

    #[test]
    fn expired_transaction_rejected() {
        let rng = DeterministicRandom::test_only([3u8; 32]);
        let mut log = AuthorizationTransactionLog::new();
        let now = at(1_000);
        let txn = log.begin(&rng, "challenge".into(), now, Duration::from_secs(60));
        let later = at(1_061);
        assert!(log.complete(&txn.state, later).is_err());
    }
}
