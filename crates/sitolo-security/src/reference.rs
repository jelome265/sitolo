//! Typed secret references.
//!
//! A `SecretRef` is *configuration*: it names where a secret lives. The secret
//! *value* is never part of a reference, so references are safe to serialise,
//! log, and fingerprint (Phase 2 specification, §5.B, §12, Appendix F).

use std::fmt;

/// Operational class of a secret. Used for least-privilege capability routing
/// and for safe audit metadata (never for the secret value itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SecretClass {
    /// PostgreSQL role credentials.
    Database,
    /// Identity/session signing or OIDC client secret material.
    Identity,
    /// Payment provider credentials.
    Payment,
    /// Webhook verification secrets.
    Webhook,
    /// MRA EIS terminal secret material.
    MraTerminal,
    /// Telemetry exporter credentials (e.g. OTLP bearer).
    Telemetry,
    /// Signing keys.
    SigningKey,
    /// Object storage credentials.
    ObjectStorage,
    /// Internal service credentials.
    Service,
}

impl SecretClass {
    /// A short, stable, lowercase identifier suitable for safe audit metadata
    /// and telemetry (bounded grammar).
    pub fn as_str(&self) -> &'static str {
        match self {
            SecretClass::Database => "database",
            SecretClass::Identity => "identity",
            SecretClass::Payment => "payment",
            SecretClass::Webhook => "webhook",
            SecretClass::MraTerminal => "mra_terminal",
            SecretClass::Telemetry => "telemetry",
            SecretClass::SigningKey => "signing_key",
            SecretClass::ObjectStorage => "object_storage",
            SecretClass::Service => "service",
        }
    }
}

impl fmt::Display for SecretClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed reference to a secret in a secret authority. This is the value that
/// flows through configuration and audit, never the secret itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretRef {
    class: SecretClass,
    /// Provider-scoped identifier, e.g. `prod/sitolo/db`. Bounded length.
    path: String,
}

/// Error produced while constructing a `SecretRef`.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SecretRefError {
    #[error("secret reference path is empty")]
    EmptyPath,
    #[error("secret reference path is too long (max {max} bytes)")]
    TooLong { max: usize },
    #[error("secret reference path contains forbidden characters")]
    ForbiddenChars,
}

const MAX_PATH_BYTES: usize = 256;

impl SecretRef {
    /// Creates a `SecretRef`. The reference itself carries no secret value and
    /// is safe to log and fingerprint.
    pub fn new(class: SecretClass, path: impl Into<String>) -> Result<Self, SecretRefError> {
        let path = path.into();
        if path.is_empty() {
            return Err(SecretRefError::EmptyPath);
        }
        if path.len() > MAX_PATH_BYTES {
            return Err(SecretRefError::TooLong {
                max: MAX_PATH_BYTES,
            });
        }
        // Restrict the grammar so references are bounded and cannot smuggle
        // control characters, whitespace, or PII into audit/telemetry fields.
        if path
            .bytes()
            .any(|b| !(b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b':')))
        {
            return Err(SecretRefError::ForbiddenChars);
        }
        Ok(SecretRef { class, path })
    }

    /// The secret class this reference belongs to.
    pub fn class(&self) -> SecretClass {
        self.class
    }

    /// A non-secret reference fingerprint for correlation in audit/telemetry,
    /// derived from the reference string, not the secret value.
    pub fn reference_fingerprint(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.path.hash(&mut h);
        format!("ref_{:016x}", h.finish())
    }

    /// The bounded provider path. Intended for internal adapter use only and
    /// never for broad logging.
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl fmt::Display for SecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.class, self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_reference_constructs() {
        let r = SecretRef::new(SecretClass::Database, "prod/sitolo/db").unwrap();
        assert_eq!(r.class(), SecretClass::Database);
        assert!(r.reference_fingerprint().starts_with("ref_"));
    }

    #[test]
    fn empty_reference_rejected() {
        assert!(matches!(
            SecretRef::new(SecretClass::Database, ""),
            Err(SecretRefError::EmptyPath)
        ));
    }

    #[test]
    fn forbidden_chars_rejected() {
        assert!(matches!(
            SecretRef::new(SecretClass::Database, "bad/secret with space"),
            Err(SecretRefError::ForbiddenChars)
        ));
        assert!(matches!(
            SecretRef::new(SecretClass::Database, "bad;sql"),
            Err(SecretRefError::ForbiddenChars)
        ));
    }
}
