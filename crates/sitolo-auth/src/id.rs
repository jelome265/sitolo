//! Opaque identifiers and bounded security vocabularies.
//!
//! Phase 3 specification, §10.1, §16, §24: identifiers are indexes into
//! server state, never authority. They embed no email, phone, user, tenant,
//! role, branch, or revealing timestamp material.

/// Opaque server-issued identifier. Production values must come from a
/// cryptographically secure source through [`sitolo_security::RandomSource`].
macro_rules! opaque_id {
    ($name:ident) => {
        #[doc = concat!("Opaque server-issued `", stringify!($name), "` identifier.")]
        ///
        /// The value is an index into server state, never authority, and embeds
        /// no user, tenant, role, or timestamp material (§10.1, §16, §24).
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Validates and wraps an opaque identifier value.
            ///
            /// Accepts 1-128 ASCII alphanumeric bytes plus `-`, `_`, `.`, and
            /// `:`. Every other shape is rejected so identifiers cannot carry
            /// hostile bytes into logs or downstream protocols.
            ///
            /// # Errors
            ///
            /// Returns [`AuthError::InvalidIdentifier`] when the value is
            /// empty, longer than 128 bytes, or contains a byte outside the
            /// permitted set.
            pub fn new(value: impl Into<String>) -> Result<Self, AuthError> {
                let value = value.into();
                if value.is_empty()
                    || value.len() > 128
                    || !value.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':')
                    })
                {
                    return Err(AuthError::InvalidIdentifier);
                }
                Ok(Self(value))
            }

            /// Borrows the identifier as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

use thiserror::Error;

use crate::error::AuthError;

/// Lowercase hex encoding (stable, dependency-free).
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let high = usize::from(byte >> 4);
        let low = usize::from(byte & 0x0f);
        if let Some(&digit) = DIGITS.get(high) {
            out.push(char::from(digit));
        }
        if let Some(&digit) = DIGITS.get(low) {
            out.push(char::from(digit));
        }
    }
    out
}

/// Unpadded base64url encoding (RFC 4648 §5). Encoding only; verification
/// re-derives challenges rather than decoding attacker input (§7.2).
#[must_use]
pub fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk.first().copied().unwrap_or(0);
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        // Four six-bit groups, computed entirely in `u8` so no fallible cast
        // or slice index is needed.
        let sextets = [
            b0 >> 2,
            ((b0 & 0x03) << 4) | (b1 >> 4),
            ((b1 & 0x0f) << 2) | (b2 >> 6),
            b2 & 0x3f,
        ];
        // A one-byte chunk yields two symbols, two bytes yield three, and a
        // full chunk yields four.
        let emitted = chunk.len().saturating_add(1);
        for sextet in sextets.iter().take(emitted) {
            if let Some(&symbol) = ALPHABET.get(usize::from(*sextet)) {
                out.push(char::from(symbol));
            }
        }
    }
    out
}

opaque_id!(UserId);
opaque_id!(SessionId);
opaque_id!(DeviceId);
opaque_id!(InstallationId);
opaque_id!(RefreshFamilyId);
opaque_id!(RefreshCredentialId);
opaque_id!(RecoveryCodeId);
opaque_id!(MfaAuthenticatorId);
opaque_id!(ChallengeId);
opaque_id!(ResetArtifactId);
opaque_id!(AuditEventId);

/// Monotonic invalidation counter for stale authority (§31). It never
/// decreases; a mismatch denies or triggers controlled reauthentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecurityVersion(pub u64);

impl SecurityVersion {
    /// The next version for a security-state transition (§31).
    #[must_use]
    pub fn next(self) -> SecurityVersion {
        SecurityVersion(self.0.saturating_add(1))
    }
}

/// Authentication assurance levels (§16): A0 anonymous through A4 break-glass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Assurance {
    /// Anonymous: no authenticated subject.
    A0,
    /// Single-factor authentication with a password or equivalent secret.
    A1,
    /// Two independent factors, or an equivalent provider-managed assertion.
    A2,
    /// Phishing-resistant authentication, such as a passkey.
    A3,
    /// Break-glass access under explicit emergency procedure.
    A4,
}

impl Assurance {
    /// True when this assurance satisfies `required` (§16).
    #[must_use]
    pub fn meets(self, required: Assurance) -> bool {
        self >= required
    }

    /// Stable bounded label for telemetry (§38).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Assurance::A0 => "a0",
            Assurance::A1 => "a1",
            Assurance::A2 => "a2",
            Assurance::A3 => "a3",
            Assurance::A4 => "a4",
        }
    }
}

/// Session risk class (§11.1). Lifetime policy is class-based configuration;
/// domain code must not hard-code a single lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionClass {
    /// Ordinary merchant-facing session.
    Merchant,
    /// Administrative session with a stricter lifetime policy.
    Administrator,
    /// Emergency break-glass session with explicit review obligations.
    BreakGlass,
}

impl SessionClass {
    /// Stable bounded label for telemetry (§38).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SessionClass::Merchant => "merchant",
            SessionClass::Administrator => "admin",
            SessionClass::BreakGlass => "break_glass",
        }
    }
}

/// How an identity was authenticated (bounded metric label, §38).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthenticationMethod {
    /// Password verification against a stored verifier record.
    Password,
    /// Assertion validated by an external OIDC provider.
    OidcAssertion,
    /// Passkey or platform authenticator ceremony.
    Passkey,
    /// Authentication fully delegated to an external provider.
    ProviderManaged,
    /// Single-use recovery code redemption.
    RecoveryCode,
    /// Workload or service credential.
    ServiceCredential,
}

impl AuthenticationMethod {
    /// Stable bounded label for telemetry (§38).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AuthenticationMethod::Password => "password",
            AuthenticationMethod::OidcAssertion => "oidc",
            AuthenticationMethod::Passkey => "passkey",
            AuthenticationMethod::ProviderManaged => "provider_managed",
            AuthenticationMethod::RecoveryCode => "recovery_code",
            AuthenticationMethod::ServiceCredential => "service",
        }
    }
}

/// Client surface (bounded metric label, §38).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientPlatform {
    /// Android application.
    Android,
    /// iOS application.
    Ios,
    /// Desktop application.
    Desktop,
    /// Browser-based web application.
    Web,
    /// Server-to-server service client.
    Service,
}

impl ClientPlatform {
    /// Stable bounded label for telemetry (§38).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ClientPlatform::Android => "android",
            ClientPlatform::Ios => "ios",
            ClientPlatform::Desktop => "desktop",
            ClientPlatform::Web => "web",
            ClientPlatform::Service => "service",
        }
    }
}

/// Startup errors for impossible or refused security configuration (§64).
#[derive(Debug, Error)]
pub enum IdentityConfigError {
    /// The supplied identity configuration cannot be used safely.
    #[error("identity configuration is invalid: {reason}")]
    Invalid {
        /// Bounded, non-secret explanation of the rejected configuration.
        reason: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_reject_hostile_shapes() {
        assert!(SessionId::new("").is_err());
        assert!(SessionId::new("has space").is_err());
        assert!(SessionId::new("inject\r\nline").is_err());
        assert!(SessionId::new("way-too-long-".repeat(12).as_str()).is_err());
        assert!(SessionId::new("sess_01ABCdef-2").is_ok());
    }

    #[test]
    fn assurance_order_is_a_total_ladder() {
        assert!(Assurance::A3.meets(Assurance::A2));
        assert!(!Assurance::A1.meets(Assurance::A2));
    }

    #[test]
    fn security_version_never_decreases() {
        let v = SecurityVersion(5);
        assert!(v.next() > v);
    }

    #[test]
    fn base64url_is_unpadded_and_safe() {
        // RFC 4648 §5 known vectors.
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(&[0xfb, 0xff, 0xff]), "-___");
    }
}
