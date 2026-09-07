//! Secure randomness boundary.
//!
//! Phase 3 specification, §6.1: secure randomness is *reused*, never invented.
//! Identity code issues session identifiers, refresh credentials, PKCE
//! verifiers, MFA enrollment artifacts, and recovery codes, so every credential
//! minting path receives randomness through this port. Production deployments
//! wire an OS CSPRNG-backed adapter; this crate provides only a deterministic
//! test double, hard-refusing production use (Phase 2 §16 fail-closed pattern).

use std::sync::Mutex;

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors produced by randomness sources.
#[derive(Debug, Error)]
pub enum RandomSourceError {
    #[error("deterministic randomness is forbidden in production")]
    ForbiddenInProduction,
}

/// The randomness capability boundary. Implementations must draw from a
/// cryptographically secure source in production deployments.
pub trait RandomSource: Send + Sync {
    /// Fills `dest` with random bytes.
    fn fill(&self, dest: &mut [u8]);

    /// Returns `len` random bytes. Bounded by callers at the use site (§47).
    fn bytes(&self, len: usize) -> Vec<u8> {
        let mut out = vec![0u8; len];
        self.fill(&mut out);
        out
    }
}

/// A deterministic SHA-256 counter-chain generator.
///
/// Test and fixture use only. Like [`crate::EnvSecretProvider`], it can be
/// constructed in a production posture but every read is refused, so a
/// misconfigured wiring fails at first use rather than minting predictable
/// credentials (§43 fail-closed rule).
pub struct DeterministicRandom {
    seed: [u8; 32],
    counter: Mutex<u64>,
    production: bool,
}

impl DeterministicRandom {
    /// Development/test construction path.
    pub fn test_only(seed: [u8; 32]) -> Self {
        DeterministicRandom {
            seed,
            counter: Mutex::new(0),
            production: false,
        }
    }

    /// Production-guarded construction: succeeds only when `production` is
    /// false, mirroring the local secret provider rule.
    pub fn new(seed: [u8; 32], production: bool) -> Result<Self, RandomSourceError> {
        if production {
            return Err(RandomSourceError::ForbiddenInProduction);
        }
        Ok(Self::test_only(seed))
    }

    fn stream_next(&self) -> [u8; 32] {
        let mut counter = self.counter.lock().expect("random counter lock");
        *counter = counter.wrapping_add(1);
        let mut h = Sha256::new();
        h.update(self.seed);
        h.update(counter.to_le_bytes());
        h.finalize().into()
    }
}

impl RandomSource for DeterministicRandom {
    fn fill(&self, dest: &mut [u8]) {
        if self.production {
            // Fail closed: predictable output must never reach a caller.
            panic!("deterministic randomness is forbidden in production");
        }
        let mut offset = 0usize;
        while offset < dest.len() {
            let block = self.stream_next();
            let take = (dest.len() - offset).min(block.len());
            dest[offset..offset + take].copy_from_slice(&block[..take]);
            offset += take;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_construction_is_refused() {
        assert!(matches!(
            DeterministicRandom::new([7u8; 32], true),
            Err(RandomSourceError::ForbiddenInProduction)
        ));
    }

    #[test]
    fn deterministic_stream_advances() {
        let r = DeterministicRandom::test_only([9u8; 32]);
        let a = r.bytes(16);
        let b = r.bytes(16);
        assert_ne!(a, b, "consecutive draws must differ");
        assert_eq!(a.len(), 16);
    }
}
