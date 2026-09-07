//! Security: secrets and redaction.
//!
//! Owns typed secret references, the secret-provider boundary, secret-value
//! memory hygiene, secure randomness and secret-at-rest ports, and redaction
//! primitives (Phase 2 specification, §12-§16, §20, Appendix D; Phase 3
//! specification, §35, §40).
//!
//! Secret values are distinct from ordinary configuration: they must never be
//! serialized, logged, or propagated accidentally. This crate never stores a
//! raw secret in a long-lived object; it only provides the narrow capability
//! handles consumers need.
#![forbid(unsafe_code)]

mod protected;
mod provider;
mod random;
mod redact;
mod reference;
mod value;

pub use protected::{InMemoryProtectedStore, SealedRef, SecretAtRest, SecretAtRestError};
pub use provider::{EnvSecretProvider, SecretError, SecretProvider};
pub use random::{DeterministicRandom, RandomSource, RandomSourceError};
pub use redact::{
    AUTH_FORBIDDEN_FIELDS, RedactBuf, RedactLimits, RedactionOutcome, SecretRedaction,
    sanitize_authentication_text,
};
pub use reference::{SecretClass, SecretRef, SecretRefError};
pub use value::SecretValue;
