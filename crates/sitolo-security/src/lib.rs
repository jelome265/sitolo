//! Security: secrets and redaction.
//!
//! Owns typed secret references, the secret-provider boundary, secret-value
//! memory hygiene, and redaction primitives (Phase 2 specification, §12-§16,
//! §20, Appendix D).
//!
//! Secret values are distinct from ordinary configuration: they must never be
//! serialized, logged, or propagated accidentally. This crate never stores a
//! raw secret in a long-lived object; it only provides the narrow capability
//! handles consumers need.
#![forbid(unsafe_code)]

mod provider;
mod redact;
mod reference;
mod value;

pub use provider::{EnvSecretProvider, SecretError, SecretProvider};
pub use redact::{RedactBuf, RedactLimits, RedactionOutcome, SecretRedaction};
pub use reference::{SecretClass, SecretRef, SecretRefError};
pub use value::SecretValue;
