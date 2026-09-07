//! JWT validation and JWKS cache policy.
//!
//! Phase 3 specification, §13, §14. JWT validation is explicit and
//! cryptographic; token claims are never treated as complete authorization
//! state (§0.5). JWKS cache must prevent stampede (§14.1, §67).

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use thiserror::Error;

use crate::error::AuthError;
use crate::id::Assurance;

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("algorithm not permitted")]
    AlgorithmNotPermitted,
    #[error("issuer not trusted")]
    IssuerNotTrusted,
    #[error("audience mismatch")]
    AudienceMismatch,
    #[error("token expired")]
    Expired,
    #[error("token not yet valid")]
    NotYetValid,
    #[error("key id missing")]
    KeyIdMissing,
    #[error("key not found")]
    KeyNotFound,
    #[error("signature invalid")]
    SignatureInvalid,
    #[error("token use not permitted")]
    TokenUseNotPermitted,
}

/// Allowed JWT algorithms (§13). `none` is never permitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenAlgorithm {
    Rs256,
    Es256,
    EdDsa,
}

impl TokenAlgorithm {
    /// Parses a declared algorithm string. Rejects `none` and unknown values
    /// (§13, §60).
    pub fn parse(declared: &str) -> Result<Self, TokenError> {
        match declared {
            "RS256" => Ok(TokenAlgorithm::Rs256),
            "ES256" => Ok(TokenAlgorithm::Es256),
            "EdDSA" => Ok(TokenAlgorithm::EdDsa),
            _ => Err(TokenError::AlgorithmNotPermitted),
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            TokenAlgorithm::Rs256 => "RS256",
            TokenAlgorithm::Es256 => "ES256",
            TokenAlgorithm::EdDsa => "EdDSA",
        }
    }
}

/// Token use claim (§13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenUse {
    Access,
    IdToken,
}

/// Parsed JWT claims (§13).
#[derive(Debug, Clone)]
pub struct TokenClaims {
    pub issuer: String,
    pub audience: String,
    pub subject: String,
    pub expires_at: SystemTime,
    pub not_before: Option<SystemTime>,
    pub issued_at: Option<SystemTime>,
    pub token_use: TokenUse,
    pub key_id: String,
    pub algorithm: TokenAlgorithm,
}

/// Validation policy (§13).
#[derive(Debug, Clone)]
pub struct TokenValidationPolicy {
    pub trusted_issuers: Vec<String>,
    pub expected_audience: String,
    pub allowed_algorithms: Vec<TokenAlgorithm>,
    pub clock_skew: Duration,
    pub require_key_id: bool,
}

/// Signature verification port (§13, §6.1). Protocol machinery is delegated
/// to a maintained JOSE implementation; this module owns the policy checks.
#[async_trait]
pub trait SignatureVerifier: Send + Sync {
    /// Verifies the signature over `signing_input` using the key identified
    /// by `key_id` and the declared algorithm.
    async fn verify(
        &self,
        key_id: &str,
        algorithm: TokenAlgorithm,
        signing_input: &[u8],
        signature: &[u8],
    ) -> Result<bool, AuthError>;

    /// Returns true if the key is known (for cache refresh decisions, §14.1).
    fn contains_key(&self, key_id: &str) -> bool;
}

/// Validated token (§13).
#[derive(Debug, Clone)]
pub struct ValidatedToken {
    pub subject: String,
    pub issuer: String,
    pub audience: String,
    pub expires_at: SystemTime,
    pub token_use: TokenUse,
    pub key_id: String,
    pub assurance_hint: Assurance,
}

/// Validates a token against the policy (§13). The order of checks is
/// explicit: algorithm allowlist, issuer, audience, time claims, key
/// availability, signature.
pub async fn validate_token(
    claims: &TokenClaims,
    signing_input: &[u8],
    signature: &[u8],
    policy: &TokenValidationPolicy,
    verifier: &dyn SignatureVerifier,
    now: SystemTime,
) -> Result<ValidatedToken, AuthError> {
    // Algorithm allowlist (§13, §60).
    if !policy.allowed_algorithms.contains(&claims.algorithm) {
        return Err(AuthError::AuthenticationFailed);
    }
    // Issuer (§13).
    if !policy.trusted_issuers.contains(&claims.issuer) {
        return Err(AuthError::AuthenticationFailed);
    }
    // Audience (§13).
    if claims.audience != policy.expected_audience {
        return Err(AuthError::AuthenticationFailed);
    }
    // Expiration with clock skew (§13).
    if now > claims.expires_at + policy.clock_skew {
        return Err(AuthError::SessionExpired);
    }
    // Not-before with clock skew (§13).
    if let Some(nbf) = claims.not_before {
        if now < nbf - policy.clock_skew {
            return Err(AuthError::AuthenticationFailed);
        }
    }
    // Key ID presence (§13).
    if policy.require_key_id && claims.key_id.is_empty() {
        return Err(AuthError::IdentityKeyUnavailable);
    }
    // Key availability (§14.1).
    if !verifier.contains_key(&claims.key_id) {
        return Err(AuthError::IdentityKeyUnavailable);
    }
    // Signature (§13).
    let valid = verifier
        .verify(&claims.key_id, claims.algorithm, signing_input, signature)
        .await?;
    if !valid {
        return Err(AuthError::AuthenticationFailed);
    }
    // Token use (§13).
    let assurance_hint = match claims.token_use {
        TokenUse::Access => Assurance::A1,
        TokenUse::IdToken => Assurance::A2,
    };
    Ok(ValidatedToken {
        subject: claims.subject.clone(),
        issuer: claims.issuer.clone(),
        audience: claims.audience.clone(),
        expires_at: claims.expires_at,
        token_use: claims.token_use,
        key_id: claims.key_id.clone(),
        assurance_hint,
    })
}

/// JWKS cache policy (§14.1).
#[derive(Debug, Clone)]
pub struct JwksCachePolicy {
    pub ttl: Duration,
    pub negative_ttl: Duration,
    pub max_refreshes_per_window: u32,
    pub window: Duration,
}

/// JWKS cache with stampede prevention (§14.1, §67).
#[derive(Debug)]
pub struct JwksCache {
    keys: BTreeMap<String, SystemTime>,
    policy: JwksCachePolicy,
    refresh_count: u32,
    window_start: SystemTime,
    last_negative: Option<SystemTime>,
}

impl JwksCache {
    #[must_use]
    pub fn new(policy: JwksCachePolicy, now: SystemTime) -> Self {
        JwksCache {
            keys: BTreeMap::new(),
            policy,
            refresh_count: 0,
            window_start: now,
            last_negative: None,
        }
    }

    /// Resolves a key ID (§14.1). Returns `RefreshNeeded` if the key is
    /// unknown and the refresh budget allows; `Backoff` if the budget is
    /// exhausted; `Trusted` if the key is fresh.
    #[must_use]
    pub fn resolve_key(&mut self, key_id: &str, now: SystemTime) -> KeyResolution {
        // Reset window if elapsed.
        if now > self.window_start + self.policy.window {
            self.window_start = now;
            self.refresh_count = 0;
        }
        // Negative cache (§14.1).
        if let Some(negative) = self.last_negative {
            if now < negative + self.policy.negative_ttl {
                return KeyResolution::Backoff;
            }
        }
        // Known key.
        if let Some(fetched_at) = self.keys.get(key_id) {
            if now < *fetched_at + self.policy.ttl {
                return KeyResolution::Trusted;
            }
        }
        // Unknown or stale; check refresh budget (§67).
        if self.refresh_count >= self.policy.max_refreshes_per_window {
            return KeyResolution::Backoff;
        }
        KeyResolution::RefreshNeeded
    }

    /// Records a successful refresh (§14.1).
    pub fn record_refresh_success(&mut self, key_ids: Vec<String>, now: SystemTime) {
        for kid in key_ids {
            self.keys.insert(kid, now);
        }
        self.refresh_count += 1;
        self.last_negative = None;
    }

    /// Records a failed refresh (§14.1).
    pub fn record_refresh_failure(&mut self, now: SystemTime) {
        self.last_negative = Some(now);
        self.refresh_count += 1;
    }
}

/// Key resolution decision (§14.1, §67).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyResolution {
    Trusted,
    RefreshNeeded,
    Backoff,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithm_parsing_rejects_none() {
        assert!(TokenAlgorithm::parse("none").is_err());
        assert!(TokenAlgorithm::parse("HS256").is_err());
        assert!(TokenAlgorithm::parse("RS256").is_ok());
    }

    #[test]
    fn jwks_cache_prevents_stampede() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let policy = JwksCachePolicy {
            ttl: Duration::from_secs(300),
            negative_ttl: Duration::from_secs(60),
            max_refreshes_per_window: 2,
            window: Duration::from_secs(60),
        };
        let mut cache = JwksCache::new(policy, now);
        assert_eq!(
            cache.resolve_key("unknown-1", now),
            KeyResolution::RefreshNeeded
        );
        cache.record_refresh_success(vec!["unknown-1".into()], now);
        assert_eq!(cache.resolve_key("unknown-1", now), KeyResolution::Trusted);
        assert_eq!(
            cache.resolve_key("unknown-2", now),
            KeyResolution::RefreshNeeded
        );
        cache.record_refresh_success(vec!["unknown-2".into()], now);
        // Budget exhausted.
        assert_eq!(cache.resolve_key("unknown-3", now), KeyResolution::Backoff);
    }

    #[test]
    fn jwks_negative_cache_blocks_refresh() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let policy = JwksCachePolicy {
            ttl: Duration::from_secs(300),
            negative_ttl: Duration::from_secs(60),
            max_refreshes_per_window: 5,
            window: Duration::from_secs(60),
        };
        let mut cache = JwksCache::new(policy, now);
        cache.record_refresh_failure(now);
        assert_eq!(cache.resolve_key("unknown", now), KeyResolution::Backoff);
        // Negative cache expires.
        let later = now + Duration::from_secs(61);
        assert_eq!(
            cache.resolve_key("unknown", later),
            KeyResolution::RefreshNeeded
        );
    }
}
