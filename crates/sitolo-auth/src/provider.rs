//! Identity provider port.
//!
//! Phase 3 specification, §6.2, §65. The authentication boundary exposes
//! application-level methods; implementations validate provider assertions
//! and return normalized identity facts. Tenant membership and permissions
//! remain outside this boundary (§1.2, §69).

use std::fmt;

use async_trait::async_trait;

use sitolo_security::SecretValue;

use crate::error::AuthError;
use crate::id::{Assurance, AuthenticationMethod};

/// External identity fact (§65).
#[derive(Debug, Clone)]
pub struct ExternalIdentity {
    /// Exact provider issuer used with `subject` as the external identity key.
    pub issuer: String,
    /// Provider-local subject identifier, unique within `issuer`.
    pub subject: String,
    /// When the provider says the subject authenticated.
    pub authenticated_at: std::time::SystemTime,
    /// Assurance the provider claims, subject to local policy (§16).
    pub assurance_hint: Assurance,
    /// Authentication method the provider used.
    pub method: AuthenticationMethod,
}

/// Token set returned by an identity provider (§65).
pub struct TokenSet {
    /// Provider access token; secret material.
    pub access_token: SecretValue,
    /// Optional provider refresh token; secret material.
    pub refresh_token: Option<SecretValue>,
    /// Optional OIDC ID token; secret material.
    pub id_token: Option<SecretValue>,
    /// Access-token lifetime reported by the provider, if any.
    pub expires_in_secs: Option<u64>,
}

impl fmt::Debug for TokenSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenSet")
            .field("access_token", &"[redacted]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .field("id_token", &self.id_token.as_ref().map(|_| "[redacted]"))
            .field("expires_in_secs", &self.expires_in_secs)
            .finish()
    }
}

/// Identity provider port (§6.2, §65). Implementations are protocol-specific
/// (OIDC, etc.); the rest of Sitolo depends only on this neutral boundary.
#[async_trait]
pub trait IdentityProviderPort: Send + Sync {
    /// Checks whether the provider's verification material is available for
    /// serving authenticated requests. Implementations must bound external
    /// I/O and may reuse valid cached verification metadata.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::IdentityProviderUnavailable`] when verification
    /// material cannot be obtained within the adapter's I/O bounds.
    async fn readiness_probe(&self) -> Result<(), AuthError>;

    /// Validates an identity assertion (e.g., an ID token or provider-specific
    /// credential) and returns the normalized identity fact.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::IdentityProviderRejected`] when the assertion is
    /// malformed, expired, or fails signature or claim validation, and
    /// [`AuthError::IdentityProviderUnavailable`] when the provider cannot be
    /// reached to complete validation.
    async fn validate_assertion(&self, assertion: &str) -> Result<ExternalIdentity, AuthError>;

    /// Exchanges an authorization code for a token set (§7, §65).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::IdentityProviderRejected`] when the authorization
    /// code or PKCE verifier is rejected, and
    /// [`AuthError::IdentityProviderUnavailable`] on transport failure.
    async fn exchange_code(
        &self,
        code: SecretValue,
        pkce_verifier: &str,
    ) -> Result<TokenSet, AuthError>;
}

/// Test-only identity provider that accepts any assertion and returns a
/// fixed identity. Production deployments must use a real OIDC or similar
/// provider.
#[derive(Debug)]
pub struct TestIdentityProvider {
    subject: String,
    assurance: Assurance,
}

impl TestIdentityProvider {
    /// Creates a test provider that always returns `subject` with the claimed
    /// `assurance`.
    pub fn new(subject: impl Into<String>, assurance: Assurance) -> Self {
        TestIdentityProvider {
            subject: subject.into(),
            assurance,
        }
    }
}

#[async_trait]
impl IdentityProviderPort for TestIdentityProvider {
    async fn readiness_probe(&self) -> Result<(), AuthError> {
        Ok(())
    }

    async fn validate_assertion(&self, _assertion: &str) -> Result<ExternalIdentity, AuthError> {
        Ok(ExternalIdentity {
            issuer: "test-issuer".to_owned(),
            subject: self.subject.clone(),
            authenticated_at: std::time::SystemTime::now(),
            assurance_hint: self.assurance,
            method: AuthenticationMethod::OidcAssertion,
        })
    }

    async fn exchange_code(
        &self,
        _code: SecretValue,
        _pkce_verifier: &str,
    ) -> Result<TokenSet, AuthError> {
        Ok(TokenSet {
            access_token: SecretValue::new("test-access-token"),
            refresh_token: Some(SecretValue::new("test-refresh-token")),
            id_token: None,
            expires_in_secs: Some(3600),
        })
    }
}
