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

/// Failures raised while validating a token (§13).
#[derive(Debug, Error)]
pub enum TokenError {
    /// The declared algorithm is not permitted by policy (§13, §60).
    #[error("algorithm not permitted")]
    AlgorithmNotPermitted,
    /// The issuer is not trusted by policy.
    #[error("issuer not trusted")]
    IssuerNotTrusted,
    /// The audience does not match the expected audience.
    #[error("audience mismatch")]
    AudienceMismatch,
    /// The token is expired at the evaluation time.
    #[error("token expired")]
    Expired,
    /// The token's not-before time is in the future.
    #[error("token not yet valid")]
    NotYetValid,
    /// The token carries no key identifier while policy requires one.
    #[error("key id missing")]
    KeyIdMissing,
    /// No verification key is available for the token's key identifier.
    #[error("key not found")]
    KeyNotFound,
    /// The signature does not verify.
    #[error("signature invalid")]
    SignatureInvalid,
    /// The token-use claim is not permitted for this operation.
    #[error("token use not permitted")]
    TokenUseNotPermitted,
}

/// Allowed JWT algorithms (§13). `none` is never permitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenAlgorithm {
    /// `RSASSA-PKCS1-v1_5` with SHA-256.
    Rs256,
    /// ECDSA with P-256 and SHA-256.
    Es256,
    /// `EdDSA` over a supported Edwards curve.
    EdDsa,
}

impl TokenAlgorithm {
    /// Parses a declared algorithm string. Rejects `none` and unknown values
    /// (§13, §60).
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::AlgorithmNotPermitted`] for `none` and for every
    /// value outside the `RS256`, `ES256`, and `EdDSA` allowlist.
    pub fn parse(declared: &str) -> Result<Self, TokenError> {
        match declared {
            "RS256" => Ok(TokenAlgorithm::Rs256),
            "ES256" => Ok(TokenAlgorithm::Es256),
            "EdDSA" => Ok(TokenAlgorithm::EdDsa),
            _ => Err(TokenError::AlgorithmNotPermitted),
        }
    }

    /// The exact JOSE algorithm name.
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
    /// Access token for calling resource servers.
    Access,
    /// OIDC ID token describing an authentication event.
    IdToken,
}

/// Parsed JWT claims (§13).
#[derive(Debug, Clone)]
pub struct TokenClaims {
    /// Issuer (`iss`) claim.
    pub issuer: String,
    /// Audience (`aud`) claim.
    pub audience: String,
    /// Subject (`sub`) claim.
    pub subject: String,
    /// Expiry (`exp`) claim.
    pub expires_at: SystemTime,
    /// Not-before (`nbf`) claim, when present.
    pub not_before: Option<SystemTime>,
    /// Issued-at (`iat`) claim, when present.
    pub issued_at: Option<SystemTime>,
    /// Whether this is an access or ID token.
    pub token_use: TokenUse,
    /// Key identifier (`kid`) used to select verification material.
    pub key_id: String,
    /// Algorithm declared in the JOSE header.
    pub algorithm: TokenAlgorithm,
}

/// Validation policy (§13).
#[derive(Debug, Clone)]
pub struct TokenValidationPolicy {
    /// Issuers the deployment trusts.
    pub trusted_issuers: Vec<String>,
    /// Audience this service expects.
    pub expected_audience: String,
    /// Algorithms the deployment accepts.
    pub allowed_algorithms: Vec<TokenAlgorithm>,
    /// Tolerance applied to time-based claims.
    pub clock_skew: Duration,
    /// Whether a key identifier is mandatory.
    pub require_key_id: bool,
}

/// Signature verification port (§13, §6.1). Protocol machinery is delegated
/// to a maintained JOSE implementation; this module owns the policy checks.
#[async_trait]
pub trait SignatureVerifier: Send + Sync {
    /// Verifies the signature over `signing_input` using the key identified
    /// by `key_id` and the declared algorithm.
    ///
    /// # Errors
    ///
    /// Returns an error when the verifier cannot evaluate the signature, for
    /// example because the selected key is unusable for the declared
    /// algorithm.
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
    /// Subject the token was issued for.
    pub subject: String,
    /// Issuer that signed the token.
    pub issuer: String,
    /// Audience the token was issued to.
    pub audience: String,
    /// Expiry carried by the token.
    pub expires_at: SystemTime,
    /// Whether this was an access or ID token.
    pub token_use: TokenUse,
    /// Key identifier that verified the signature.
    pub key_id: String,
    /// Assurance implied by the token use (§16).
    pub assurance_hint: Assurance,
}

/// Validates a token against the policy (§13). The order of checks is
/// explicit: algorithm allowlist, issuer, audience, time claims, key
/// availability, signature.
///
/// # Errors
///
/// Returns [`AuthError::AuthenticationFailed`] when the algorithm is not
/// allowed, the issuer is untrusted, the audience does not match, the
/// not-before claim is in the future beyond the clock skew, or the signature
/// does not verify. Returns [`AuthError::SessionExpired`] when the token is
/// expired beyond the clock skew, and [`AuthError::IdentityKeyUnavailable`]
/// when a required key identifier is missing or the verifier does not know the
/// key. Errors from the verifier are propagated unchanged.
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
    // Expiration with clock skew (§13). An unrepresentable skewed deadline
    // can only occur at the end of representable time, where the token is
    // treated as expired.
    if claims
        .expires_at
        .checked_add(policy.clock_skew)
        .is_none_or(|deadline| now > deadline)
    {
        return Err(AuthError::SessionExpired);
    }
    // Not-before with clock skew (§13). If the skewed lower bound precedes the
    // representable range, no lower bound applies.
    if let Some(nbf) = claims.not_before
        && nbf
            .checked_sub(policy.clock_skew)
            .is_some_and(|earliest| now < earliest)
    {
        return Err(AuthError::AuthenticationFailed);
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
    /// How long a fetched key remains trusted.
    pub ttl: Duration,
    /// How long a failed refresh suppresses further attempts.
    pub negative_ttl: Duration,
    /// Refresh budget per window (§67).
    pub max_refreshes_per_window: u32,
    /// Length of the refresh budget window.
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
    /// Creates an empty cache whose refresh window starts at `now`.
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
        // Reset window if elapsed. An unrepresentable window end means the
        // window cannot have elapsed, so the budget is retained.
        if self
            .window_start
            .checked_add(self.policy.window)
            .is_some_and(|end| now > end)
        {
            self.window_start = now;
            self.refresh_count = 0;
        }
        // Negative cache (§14.1). An unrepresentable end keeps the negative
        // entry in force.
        if let Some(negative) = self.last_negative
            && negative
                .checked_add(self.policy.negative_ttl)
                .is_none_or(|until| now < until)
        {
            return KeyResolution::Backoff;
        }
        // Known key. An unrepresentable expiry keeps the key trusted.
        if let Some(fetched_at) = self.keys.get(key_id)
            && fetched_at
                .checked_add(self.policy.ttl)
                .is_none_or(|expires| now < expires)
        {
            return KeyResolution::Trusted;
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
        self.refresh_count = self.refresh_count.saturating_add(1);
        self.last_negative = None;
    }

    /// Records a failed refresh (§14.1).
    pub fn record_refresh_failure(&mut self, now: SystemTime) {
        self.last_negative = Some(now);
        self.refresh_count = self.refresh_count.saturating_add(1);
    }
}

/// Key resolution decision (§14.1, §67).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyResolution {
    /// The key is cached and fresh.
    Trusted,
    /// A refresh may be attempted.
    RefreshNeeded,
    /// The refresh budget or negative cache forbids a refresh.
    Backoff,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }

    #[test]
    fn algorithm_parsing_rejects_none() {
        assert!(TokenAlgorithm::parse("none").is_err());
        assert!(TokenAlgorithm::parse("HS256").is_err());
        assert!(TokenAlgorithm::parse("RS256").is_ok());
    }

    #[test]
    fn jwks_cache_prevents_stampede() {
        let now = at(1_000);
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
        let now = at(1_000);
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
        let later = at(1_061);
        assert_eq!(
            cache.resolve_key("unknown", later),
            KeyResolution::RefreshNeeded
        );
    }
}
