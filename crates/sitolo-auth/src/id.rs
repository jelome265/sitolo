//! Opaque identifiers and bounded security vocabularies.
//!
//! Phase 3 specification, §10.1, §16, §24: identifiers are indexes into
//! server state, never authority. They embed no email, phone, user, tenant,
//! role, branch, or revealing timestamp material.

/// Opaque server-issued identifier. Production values must come from a
/// cryptographically secure source through [`sitolo_security::RandomSource`].
macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
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
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

use thiserror::Error;

use crate::error::AuthError;

/// Lowercase hex encoding (stable, dependency-free).
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Unpadded base64url encoding (RFC 4648 §5). Encoding only; verification
/// re-derives challenges rather than decoding attacker input (§7.2).
pub fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(ALPHABET[(n >> 18) as usize & 0x3f] as char);
        out.push(ALPHABET[(n >> 12) as usize & 0x3f] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 0x3f] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 0x3f] as char);
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
    A0,
    A1,
    A2,
    A3,
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
    Merchant,
    Administrator,
    BreakGlass,
}

impl SessionClass {
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
    Password,
    OidcAssertion,
    Passkey,
    ProviderManaged,
    RecoveryCode,
    ServiceCredential,
}

impl AuthenticationMethod {
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
    Android,
    Ios,
    Desktop,
    Web,
    Service,
}

impl ClientPlatform {
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
    #[error("identity configuration is invalid: {reason}")]
    Invalid { reason: &'static str },
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
