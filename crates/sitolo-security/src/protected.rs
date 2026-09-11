//! Secret-at-rest boundary.
//!
//! Phase 3 specification, §18 and §34.2: TOTP seeds and comparable high-value
//! material must be protected at rest and must never enter ordinary audit
//! rows. Domain code therefore stores a [`SealedRef`], not the material.
//! Production wiring uses KMS envelope encryption through the Phase 2 secret
//! authority; the in-memory store here models the capability contract only.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

use thiserror::Error;

/// Errors produced by secret-at-rest stores.
#[derive(Debug, Error)]
pub enum SecretAtRestError {
    #[error("sealed reference not found")]
    NotFound,
    #[error("secret-at-rest store unavailable")]
    Unavailable,
}

/// An opaque reference to protected material. Safe to persist alongside an
/// ordinary database row; the material itself is never embedded here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SealedRef(String);

impl SealedRef {
    /// Wraps an opaque reference string.
    pub fn new(value: impl Into<String>) -> Self {
        SealedRef(value.into())
    }
    /// The reference string. Not secret material; a storage pointer only.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SealedRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sealed:{}", self.0)
    }
}

/// Seal/open boundary for high-value authenticator material (§18, §34.2).
pub trait SecretAtRest: Send + Sync {
    /// Encrypts/protects `material` and returns its opaque reference.
    fn seal(&self, material: &[u8]) -> Result<SealedRef, SecretAtRestError>;
    /// Opens a sealed reference for a narrow verification operation.
    fn open(&self, reference: &SealedRef) -> Result<Vec<u8>, SecretAtRestError>;
    /// Permanently removes sealed material (authenticator revocation §17.2).
    fn destroy(&self, reference: &SealedRef);
}

/// Process-memory store for tests and local reference wiring.
///
/// The "sealing" here is storage separation, not encryption; production
/// deployments must replace it with a KMS-backed implementation.
#[derive(Default)]
pub struct InMemoryProtectedStore {
    items: Mutex<BTreeMap<String, Vec<u8>>>,
    next: Mutex<u64>,
}

impl InMemoryProtectedStore {
    /// Acquires the items lock, recovering from mutex poisoning.
    ///
    /// A panic in one caller while holding this lock must not cascade into
    /// permanent unavailability for every subsequent caller (fail-closed
    /// security decisions still occur upstream via typed `Result`s; a
    /// poisoned in-memory cache is not itself a security invariant).
    fn items_lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Vec<u8>>> {
        self.items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Acquires the id-counter lock, recovering from mutex poisoning.
    fn next_lock(&self) -> std::sync::MutexGuard<'_, u64> {
        self.next
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl SecretAtRest for InMemoryProtectedStore {
    fn seal(&self, material: &[u8]) -> Result<SealedRef, SecretAtRestError> {
        let mut id = self.next_lock();
        *id = id.wrapping_add(1);
        let reference = SealedRef::new(format!("sealed_{id:08}"));
        self.items_lock()
            .insert(reference.0.clone(), material.to_vec());
        Ok(reference)
    }

    fn open(&self, reference: &SealedRef) -> Result<Vec<u8>, SecretAtRestError> {
        self.items_lock()
            .get(&reference.0)
            .cloned()
            .ok_or(SecretAtRestError::NotFound)
    }

    fn destroy(&self, reference: &SealedRef) {
        self.items_lock().remove(&reference.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_destroy_roundtrip() {
        let store = InMemoryProtectedStore::default();
        let reference = store.seal(b"totp-seed-material").unwrap();
        assert_eq!(store.open(&reference).unwrap(), b"totp-seed-material");
        store.destroy(&reference);
        assert!(matches!(
            store.open(&reference),
            Err(SecretAtRestError::NotFound)
        ));
    }

    #[test]
    fn reference_display_never_carries_material() {
        let store = InMemoryProtectedStore::default();
        let reference = store.seal(b"super-secret-seed").unwrap();
        let text = reference.to_string();
        assert!(!text.contains("super-secret-seed"));
    }
}
