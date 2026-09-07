//! Identity provider port.
//!
//! Phase 3 specification, §6.2, §65. The authentication boundary exposes
//! application-level methods; implementations validate provider assertions
//! and return normalized identity facts. Tenant membership and permissions
//! remain outside this boundary (§1.2, §69).

use async_trait::async_trait;

use sitolo_security::SecretValue;

use crate::error::AuthError;
use crate::id::{Assurance, AuthenticationMethod};

/// External identity fact (§65).
#[derive(Debug, Clone)]
pub struct ExternalIdentity {
    pub subject: String,
    pub authenticated_at: std::time::SystemTime,
    pub assurance_hint: Assurance,
    pub method: AuthenticationMethod,
}

/// Token set returned by an identity provider (§65).
#[derive(Debug, Clone)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_in_secs: Option<u64>,
}

/// Identity provider port (§6.2, §65). Implementations are protocol-specific
/// (OIDC, etc.); the rest of Sitolo depends only on this neutral boundary.
#[async_trait]
pub trait IdentityProviderPort: Send + Sync {
    /// Validates an identity assertion (e.g., an ID token or provider-specific
    /// credential) and returns the normalized identity fact.
    async fn validate_assertion(&self, assertion: &str) -> Result<ExternalIdentity, AuthError>;

    /// Exchanges an authorization code for a token set (§7, §65).
    async fn exchange_code(
        &self,
        code: SecretValue,
        pkce_verifier: &str,
    ) -> Result<TokenSet, AuthError>;
}

/// Test-only identity provider that accepts any assertion and returns a
/// fixed identity. Production deployments must use a real OIDC or similar
/// provider.
pub struct TestIdentityProvider {
    subject: String,
    assurance: Assurance,
}

impl TestIdentityProvider {
    pub fn new(subject: impl Into<String>, assurance: Assurance) -> Self {
        TestIdentityProvider {
            subject: subject.into(),
            assurance,
        }
    }
}

#[async_trait]
impl IdentityProviderPort for TestIdentityProvider {
    async fn validate_assertion(&self, _assertion: &str) -> Result<ExternalIdentity, AuthError> {
        Ok(ExternalIdentity {
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
            access_token: "test-access-token".into(),
            refresh_token: Some("test-refresh-token".into()),
            id_token: None,
            expires_in_secs: Some(3600),
        })
    }
}
