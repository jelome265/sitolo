//! Password policy and hashing boundary.
//!
//! Phase 3 specification, §9. Passwords are never stored in plaintext or
//! reversible form (§0.2); hashing is delegated to a maintained Argon2id
//! adapter at deployment time (§6.1). This crate owns the policy, the port,
//! and the rehash decision; production cryptographic work happens in the
//! adapter.

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use thiserror::Error;

use sitolo_security::SecretValue;

use crate::error::AuthError;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("password too short")]
    TooShort,
    #[error("password too long")]
    TooLong,
    #[error("password hash failed")]
    HashFailed,
    #[error("password verifier malformed")]
    VerifierMalformed,
}

/// Policy version plus bounded length constraints (§9.2, §47).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordPolicy {
    pub policy_version: u32,
    pub min_length: usize,
    pub max_length: usize,
    pub rehash_enabled: bool,
}

impl PasswordPolicy {
    /// Validates a candidate against the bounded policy (§47). The check is
    /// cheap and must run before any expensive hashing work.
    pub fn validate(&self, candidate: &str) -> Result<(), AuthError> {
        if candidate.len() < self.min_length || candidate.len() > self.max_length {
            return Err(AuthError::AuthenticationFailed);
        }
        Ok(())
    }
}

/// The opaque verifier record stored alongside a user (§9.1). The hash
/// string is opaque to the domain; only the hasher adapter knows its format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordVerifierRecord {
    /// Opaque verifier string. Not `Debug`-printed in its raw form.
    pub verifier: String,
    pub policy_version: u32,
}

/// The password hashing boundary (§9.1, §6.1).
///
/// Production deployments wire in a maintained Argon2id implementation; the
/// reference `TestPasswordHasher` below is a deterministic test double
/// clearly labeled as such and must never be selected in production.
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    /// Hashes a password into a verifier record.
    async fn hash(&self, password: SecretValue) -> Result<PasswordVerifierRecord, AuthError>;
    /// Verifies a password against a verifier record.
    async fn verify(
        &self,
        password: SecretValue,
        record: &PasswordVerifierRecord,
    ) -> Result<bool, AuthError>;
    /// The policy version this adapter currently produces.
    fn current_policy_version(&self) -> u32;
}

/// Whether a successful verification should trigger a rehash (§9.3).
///
/// Rehashing is never triggered on a failed verification; a failed
/// authentication must never cause additional CPU work (§9.3).
#[must_use]
pub fn needs_rehash(record: &PasswordVerifierRecord, policy: &PasswordPolicy) -> bool {
    policy.rehash_enabled && record.policy_version < policy.policy_version
}

/// Test-only deterministic password hasher.
///
/// Uses iterated SHA-256 over a tagged prefix. This is explicitly **not**
/// Argon2id and must never be selected in production; it exists so the
/// lifecycle and rehash policy can be tested without a real KDF dependency.
/// The verifier format is `testv{version}${hex}`.
pub struct TestPasswordHasher {
    policy_version: u32,
}

impl TestPasswordHasher {
    pub fn new(policy_version: u32) -> Self {
        TestPasswordHasher { policy_version }
    }
}

#[async_trait]
impl PasswordHasher for TestPasswordHasher {
    async fn hash(&self, password: SecretValue) -> Result<PasswordVerifierRecord, AuthError> {
        let bytes = password.into_bytes();
        let mut h = Sha256::new();
        h.update(b"sitolo-test-password-v");
        h.update(self.policy_version.to_le_bytes());
        h.update(&bytes);
        let digest = h.finalize();
        Ok(PasswordVerifierRecord {
            verifier: format!("testv{}${}", self.policy_version, crate::id::hex(&digest)),
            policy_version: self.policy_version,
        })
    }

    async fn verify(
        &self,
        password: SecretValue,
        record: &PasswordVerifierRecord,
    ) -> Result<bool, AuthError> {
        let Some(version) = parse_test_version(&record.verifier) else {
            return Err(AuthError::ConfigurationInvalid);
        };
        let bytes = password.into_bytes();
        let mut h = Sha256::new();
        h.update(b"sitolo-test-password-v");
        h.update(version.to_le_bytes());
        h.update(&bytes);
        let digest = h.finalize();
        let expected = format!("testv{version}${}", crate::id::hex(&digest));
        Ok(expected == record.verifier)
    }

    fn current_policy_version(&self) -> u32 {
        self.policy_version
    }
}

fn parse_test_version(verifier: &str) -> Option<u32> {
    let rest = verifier.strip_prefix("testv")?;
    let (version, tail) = rest.split_once('$')?;
    if tail.is_empty() {
        return None;
    }
    version.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_rejects_too_short_and_too_long() {
        let policy = PasswordPolicy {
            policy_version: 1,
            min_length: 8,
            max_length: 64,
            rehash_enabled: true,
        };
        assert!(policy.validate("short").is_err());
        assert!(policy.validate("exactly8").is_ok());
        assert!(policy.validate(&"x".repeat(65)).is_err());
    }

    #[test]
    fn rehash_triggers_on_older_version_only() {
        let policy = PasswordPolicy {
            policy_version: 3,
            min_length: 8,
            max_length: 64,
            rehash_enabled: true,
        };
        assert!(needs_rehash(
            &PasswordVerifierRecord {
                verifier: "testv2$abc".into(),
                policy_version: 2
            },
            &policy
        ));
        assert!(!needs_rehash(
            &PasswordVerifierRecord {
                verifier: "testv3$abc".into(),
                policy_version: 3
            },
            &policy
        ));
    }

    #[test]
    fn rehash_disabled_suppresses_upgrade() {
        let policy = PasswordPolicy {
            policy_version: 3,
            min_length: 8,
            max_length: 64,
            rehash_enabled: false,
        };
        assert!(!needs_rehash(
            &PasswordVerifierRecord {
                verifier: "testv2$abc".into(),
                policy_version: 2
            },
            &policy
        ));
    }
}
