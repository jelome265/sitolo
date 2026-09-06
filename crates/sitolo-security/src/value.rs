//! Secret value containment.
//!
//! A `SecretValue` holds access material with deliberate memory hygiene:
//! it is not `Debug`, not `Display`, not `Copy`, and not `Serialize`. It can be
//! consumed exactly once to produce an owned byte/string value for a narrow
//! consumer (Phase 2 specification, §14, Appendix D).

use std::fmt;

/// A one-shot container for secret access material.
///
/// Deliberately does NOT implement `Debug`, `Display`, `PartialEq` that prints
/// contents, or any serialization trait. Attempting to print it yields a fixed
/// redacted marker.
pub struct SecretValue {
    value: String,
}

impl SecretValue {
    /// Wraps a raw secret value. The caller must treat `value` as sensitive and
    /// not retain other copies.
    pub fn new(value: impl Into<String>) -> Self {
        SecretValue {
            value: value.into(),
        }
    }

    /// Consumes the secret and returns the raw bytes to the calling capability.
    /// This is the only way to obtain the material.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.value.into_bytes()
    }

    /// Consumes the secret and returns the raw UTF-8 string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.value
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue(redacted)")
    }
}

impl fmt::Display for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let s = SecretValue::new("supersecret");
        assert_eq!(format!("{s:?}"), "SecretValue(redacted)");
        assert_eq!(format!("{s}"), "<redacted>");
    }

    #[test]
    fn value_consumed_into_string() {
        let s = SecretValue::new("my-db-password");
        assert_eq!(s.into_string(), "my-db-password");
    }

    #[test]
    fn value_consumed_into_bytes() {
        let s = SecretValue::new("abc");
        assert_eq!(s.into_bytes(), b"abc");
    }
}
