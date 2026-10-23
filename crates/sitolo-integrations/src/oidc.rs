//! Cognito access-token adapter for the protocol-neutral Sitolo identity port.
//!
//! This adapter authenticates an external identity only. It does not create
//! Sitolo sessions or establish tenant membership, roles, scopes, or
//! permissions; those remain server-side application responsibilities.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{Method, Request, Response, StatusCode, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use jsonwebtoken::jwk::{AlgorithmParameters, JwkSet, KeyAlgorithm, PublicKeyUse};
use jsonwebtoken::{Algorithm, DecodingKey, Header, Validation, decode, decode_header};
use serde::Deserialize;
use sitolo_auth::{
    Assurance, AuthError, AuthenticationMethod, ExternalIdentity, IdentityProviderPort, TokenSet,
};
use sitolo_security::SecretValue;
use tokio::sync::{Mutex, RwLock};
use url::Url;

const MAX_ACCESS_TOKEN_BYTES: usize = 16 * 1024;
const MAX_PROVIDER_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_JWKS_KEYS: usize = 64;
const MAX_SCOPES: usize = 64;
const MAX_SCOPE_BYTES: usize = 128;

/// Raw provider settings parsed at the application configuration boundary.
#[derive(Clone)]
pub struct CognitoOidcSettings {
    pub issuer: String,
    pub client_id: String,
    pub resource_audience: Option<String>,
    pub token_endpoint: String,
    pub redirect_uri: String,
    pub required_scopes: Vec<String>,
    pub request_timeout: Duration,
    pub jwks_ttl: Duration,
    pub unknown_kid_refresh_interval: Duration,
    pub clock_skew: Duration,
}

/// Validated static Cognito/OIDC settings.
#[derive(Clone)]
pub struct CognitoOidcConfig {
    issuer: Url,
    jwks_url: Url,
    client_id: String,
    resource_audience: Option<String>,
    token_endpoint: Url,
    redirect_uri: Url,
    required_scopes: BTreeSet<String>,
    request_timeout: Duration,
    jwks_ttl: Duration,
    unknown_kid_refresh_interval: Duration,
    clock_skew: Duration,
}

impl CognitoOidcConfig {
    /// Creates validated configuration and derives the JWKS URL from the
    /// trusted issuer. Callers cannot supply an independent key URL.
    pub fn new(settings: CognitoOidcSettings) -> Result<Self, AuthError> {
        let CognitoOidcSettings {
            issuer: issuer_raw,
            client_id,
            resource_audience,
            token_endpoint: token_endpoint_raw,
            redirect_uri: redirect_uri_raw,
            required_scopes,
            request_timeout,
            jwks_ttl,
            unknown_kid_refresh_interval,
            clock_skew,
        } = settings;
        let issuer = parse_https_url(&issuer_raw)?;
        if issuer.query().is_some() || issuer.fragment().is_some() || issuer.path() == "/" {
            return Err(AuthError::ConfigurationInvalid);
        }
        if client_id.is_empty() || client_id.len() > 256 || !is_safe_identifier(&client_id) {
            return Err(AuthError::ConfigurationInvalid);
        }
        if let Some(audience) = &resource_audience
            && (audience.is_empty() || audience.len() > 256 || !is_safe_identifier(audience))
        {
            return Err(AuthError::ConfigurationInvalid);
        }
        let token_endpoint = parse_https_url(&token_endpoint_raw)?;
        if token_endpoint.query().is_some() || token_endpoint.fragment().is_some() {
            return Err(AuthError::ConfigurationInvalid);
        }
        let redirect_uri =
            Url::parse(&redirect_uri_raw).map_err(|_| AuthError::ConfigurationInvalid)?;
        if !matches!(redirect_uri.scheme(), "https" | "sitolo")
            || redirect_uri.host_str().is_none()
            || !redirect_uri.username().is_empty()
            || redirect_uri.password().is_some()
            || redirect_uri.fragment().is_some()
        {
            return Err(AuthError::ConfigurationInvalid);
        }
        if request_timeout.is_zero() || request_timeout > Duration::from_secs(10) {
            return Err(AuthError::ConfigurationInvalid);
        }
        if jwks_ttl.is_zero() || jwks_ttl > Duration::from_secs(3_600) {
            return Err(AuthError::ConfigurationInvalid);
        }
        if unknown_kid_refresh_interval.is_zero()
            || unknown_kid_refresh_interval > Duration::from_secs(60)
        {
            return Err(AuthError::ConfigurationInvalid);
        }
        if clock_skew > Duration::from_secs(60) {
            return Err(AuthError::ConfigurationInvalid);
        }

        let required_scopes: BTreeSet<String> = required_scopes.into_iter().collect();
        if required_scopes.len() > MAX_SCOPES
            || required_scopes.iter().any(|scope| !is_valid_scope(scope))
        {
            return Err(AuthError::ConfigurationInvalid);
        }

        let jwks_url = Url::parse(&format!(
            "{}/.well-known/jwks.json",
            issuer.as_str().trim_end_matches('/')
        ))
        .map_err(|_| AuthError::ConfigurationInvalid)?;

        Ok(Self {
            issuer,
            jwks_url,
            client_id,
            resource_audience,
            token_endpoint,
            redirect_uri,
            required_scopes,
            request_timeout,
            jwks_ttl,
            unknown_kid_refresh_interval,
            clock_skew,
        })
    }
}

impl fmt::Debug for CognitoOidcConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CognitoOidcConfig")
            .field("issuer", &self.issuer)
            .field("jwks_url", &self.jwks_url)
            .field("client_id", &"[configured]")
            .field("resource_audience", &self.resource_audience.is_some())
            .field("token_endpoint", &self.token_endpoint)
            .field("redirect_uri", &self.redirect_uri)
            .field("required_scope_count", &self.required_scopes.len())
            .field("request_timeout", &self.request_timeout)
            .field("jwks_ttl", &self.jwks_ttl)
            .field(
                "unknown_kid_refresh_interval",
                &self.unknown_kid_refresh_interval,
            )
            .field("clock_skew", &self.clock_skew)
            .finish()
    }
}

struct CachedJwks {
    keys: HashMap<String, DecodingKey>,
    expires_at: SystemTime,
    last_forced_refresh_at: Option<SystemTime>,
}

/// Cognito adapter implementing the existing identity-provider port.
///
/// JWKS refreshes are bounded, serialized to prevent stampedes, derived only
/// from the configured issuer, and never follow redirects. Access tokens are
/// checked for RS256 signature, exact issuer, client, token use, times, and
/// configured resource audience/scopes.
pub struct CognitoOidcProvider {
    config: CognitoOidcConfig,
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
    jwks: RwLock<Option<CachedJwks>>,
    refresh_lock: Mutex<()>,
}

impl CognitoOidcProvider {
    /// Constructs the provider using TLS certificate verification and a
    /// bounded request timeout. Redirect following is disabled.
    pub fn new(config: CognitoOidcConfig) -> Result<Self, AuthError> {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        }
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            return Err(AuthError::ConfigurationInvalid);
        }
        let connector = HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|_| AuthError::ConfigurationInvalid)?
            .https_only()
            .enable_http1()
            .build();
        let client = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(8)
            .build(connector);
        Ok(Self {
            config,
            client,
            jwks: RwLock::new(None),
            refresh_lock: Mutex::new(()),
        })
    }

    /// Fetches and validates the trusted issuer's signing keys for startup
    /// readiness. The URL is derived from the validated issuer and redirects
    /// remain disabled by the existing HTTPS client.
    ///
    /// # Errors
    /// Returns an identity-provider error if the JWKS endpoint is unavailable
    /// or its bounded document is invalid.
    pub async fn probe(&self) -> Result<(), AuthError> {
        self.refresh_jwks().await
    }

    async fn decoding_key(&self, kid: &str) -> Result<DecodingKey, AuthError> {
        let now = SystemTime::now();
        if let Some(key) = self.cached_key(kid, now).await {
            return Ok(key);
        }

        let _refresh = self.refresh_lock.lock().await;
        let now = SystemTime::now();
        if let Some(key) = self.cached_key(kid, now).await {
            return Ok(key);
        }

        {
            let cache = self.jwks.read().await;
            if let Some(cache) = cache.as_ref()
                && now < cache.expires_at
                && cache
                    .last_forced_refresh_at
                    .is_some_and(|last| now < last + self.config.unknown_kid_refresh_interval)
            {
                return Err(AuthError::IdentityKeyUnavailable);
            }
        }

        self.refresh_jwks().await?;
        self.cached_key(kid, SystemTime::now())
            .await
            .ok_or(AuthError::IdentityKeyUnavailable)
    }

    async fn cached_key(&self, kid: &str, now: SystemTime) -> Option<DecodingKey> {
        let cache = self.jwks.read().await;
        cache
            .as_ref()
            .filter(|cache| now < cache.expires_at)
            .and_then(|cache| cache.keys.get(kid))
            .cloned()
    }

    async fn refresh_jwks(&self) -> Result<(), AuthError> {
        let uri = self
            .config
            .jwks_url
            .as_str()
            .parse::<Uri>()
            .map_err(|_| AuthError::ConfigurationInvalid)?;
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Full::new(Bytes::new()))
            .map_err(|_| AuthError::ConfigurationInvalid)?;
        let response =
            tokio::time::timeout(self.config.request_timeout, self.client.request(request))
                .await
                .map_err(|_| AuthError::IdentityProviderUnavailable)?
                .map_err(|_| AuthError::IdentityProviderUnavailable)?;
        let status = response.status();
        if !status.is_success() {
            return Err(map_provider_status(status));
        }
        let body = tokio::time::timeout(
            self.config.request_timeout,
            read_bounded(response, MAX_PROVIDER_RESPONSE_BYTES),
        )
        .await
        .map_err(|_| AuthError::IdentityProviderUnavailable)??;
        let keys = parse_jwks(&body)?;
        let now = SystemTime::now();
        let mut cache = self.jwks.write().await;
        let last_forced_refresh_at = cache
            .as_ref()
            .and_then(|previous| previous.last_forced_refresh_at);
        *cache = Some(CachedJwks {
            keys,
            expires_at: now + self.config.jwks_ttl,
            last_forced_refresh_at: last_forced_refresh_at.or(Some(now)),
        });
        Ok(())
    }

    async fn verify_access_token(&self, raw: &str) -> Result<ExternalIdentity, AuthError> {
        if raw.is_empty()
            || raw.len() > MAX_ACCESS_TOKEN_BYTES
            || raw.bytes().any(|byte| byte.is_ascii_whitespace())
        {
            return Err(AuthError::AuthenticationFailed);
        }
        let header: Header = decode_header(raw).map_err(|_| AuthError::AuthenticationFailed)?;
        if header.alg != Algorithm::RS256
            || header.typ.as_deref() != Some("JWT")
            || header.kid.as_deref().is_none_or(str::is_empty)
        {
            return Err(AuthError::AuthenticationFailed);
        }
        let kid = header
            .kid
            .as_deref()
            .ok_or(AuthError::IdentityKeyUnavailable)?;
        let key = self.decoding_key(kid).await?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.algorithms = vec![Algorithm::RS256];
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.validate_aud = false;
        validation.leeway = self.config.clock_skew.as_secs();
        validation.required_spec_claims = HashSet::from(["exp".to_owned(), "sub".to_owned()]);
        validation.set_issuer(&[self.config.issuer.as_str()]);

        let claims = decode::<CognitoAccessClaims>(raw, &key, &validation)
            .map_err(|_| AuthError::AuthenticationFailed)?
            .claims;
        if claims.iss != self.config.issuer.as_str()
            || claims.sub.is_empty()
            || claims.sub.len() > 256
            || claims.token_use != "access"
            || claims.client_id != self.config.client_id
            || claims.auth_time > claims.iat
            || claims.exp <= claims.iat
        {
            return Err(AuthError::AuthenticationFailed);
        }
        if let Some(audience) = claims.aud.as_ref() {
            let Some(expected) = self.config.resource_audience.as_deref() else {
                return Err(AuthError::AuthenticationFailed);
            };
            if !audience.contains(expected) {
                return Err(AuthError::AuthenticationFailed);
            }
        }
        let granted_scopes: BTreeSet<&str> = claims
            .scope
            .as_deref()
            .unwrap_or_default()
            .split_ascii_whitespace()
            .collect();
        if !self
            .config
            .required_scopes
            .iter()
            .all(|scope| granted_scopes.contains(scope.as_str()))
        {
            return Err(AuthError::AuthenticationFailed);
        }
        let authenticated_at = UNIX_EPOCH
            .checked_add(Duration::from_secs(claims.auth_time))
            .ok_or(AuthError::AuthenticationFailed)?;
        Ok(ExternalIdentity {
            issuer: claims.iss,
            subject: claims.sub,
            authenticated_at,
            assurance_hint: Assurance::A1,
            method: AuthenticationMethod::OidcAssertion,
        })
    }
}

fn parse_jwks(body: &[u8]) -> Result<HashMap<String, DecodingKey>, AuthError> {
    let jwks: JwkSet =
        serde_json::from_slice(body).map_err(|_| AuthError::IdentityKeyUnavailable)?;
    if jwks.keys.is_empty() || jwks.keys.len() > MAX_JWKS_KEYS {
        return Err(AuthError::IdentityKeyUnavailable);
    }

    let mut keys = HashMap::with_capacity(jwks.keys.len());
    for jwk in &jwks.keys {
        let Some(kid) = jwk.common.key_id.as_deref() else {
            continue;
        };
        if kid.is_empty() || kid.len() > 128 {
            continue;
        }
        if jwk.common.key_algorithm != Some(KeyAlgorithm::RS256)
            || !matches!(jwk.algorithm, AlgorithmParameters::RSA(_))
            || jwk
                .common
                .public_key_use
                .as_ref()
                .is_some_and(|key_use| *key_use != PublicKeyUse::Signature)
        {
            continue;
        }
        let Ok(key) = DecodingKey::from_jwk(jwk) else {
            continue;
        };
        // A duplicated key identifier makes selection ambiguous. Reject the
        // whole document instead of letting JSON order select the verifier.
        if keys.insert(kid.to_owned(), key).is_some() {
            return Err(AuthError::IdentityKeyUnavailable);
        }
    }
    if keys.is_empty() {
        return Err(AuthError::IdentityKeyUnavailable);
    }
    Ok(keys)
}

#[async_trait]
impl IdentityProviderPort for CognitoOidcProvider {
    async fn validate_assertion(&self, assertion: &str) -> Result<ExternalIdentity, AuthError> {
        self.verify_access_token(assertion).await
    }

    async fn exchange_code(
        &self,
        code: SecretValue,
        pkce_verifier: &str,
    ) -> Result<TokenSet, AuthError> {
        if pkce_verifier.is_empty() || pkce_verifier.len() > 128 {
            return Err(AuthError::IdentityProviderRejected);
        }
        let code = code.into_string();
        if code.is_empty() || code.len() > 4_096 {
            return Err(AuthError::IdentityProviderRejected);
        }
        let form = serde_urlencoded::to_string([
            ("grant_type", "authorization_code"),
            ("client_id", self.config.client_id.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", self.config.redirect_uri.as_str()),
            ("code_verifier", pkce_verifier),
        ])
        .map_err(|_| AuthError::IdentityProviderRejected)?;
        let uri = self
            .config
            .token_endpoint
            .as_str()
            .parse::<Uri>()
            .map_err(|_| AuthError::ConfigurationInvalid)?;
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Full::new(Bytes::from(form)))
            .map_err(|_| AuthError::ConfigurationInvalid)?;
        let response =
            tokio::time::timeout(self.config.request_timeout, self.client.request(request))
                .await
                .map_err(|_| AuthError::IdentityProviderUnavailable)?
                .map_err(|_| AuthError::IdentityProviderUnavailable)?;
        let status = response.status();
        if !status.is_success() {
            return Err(map_provider_status(status));
        }
        let body = tokio::time::timeout(
            self.config.request_timeout,
            read_bounded(response, MAX_PROVIDER_RESPONSE_BYTES),
        )
        .await
        .map_err(|_| AuthError::IdentityProviderUnavailable)??;
        let token_response: CognitoTokenResponse =
            serde_json::from_slice(&body).map_err(|_| AuthError::IdentityProviderRejected)?;
        if token_response.access_token.is_empty()
            || token_response.access_token.len() > MAX_ACCESS_TOKEN_BYTES
            || token_response.token_type != "Bearer"
            || token_response.expires_in == 0
            || token_response.expires_in > 86_400
        {
            return Err(AuthError::IdentityProviderRejected);
        }
        Ok(TokenSet {
            access_token: SecretValue::new(token_response.access_token),
            refresh_token: token_response.refresh_token.map(SecretValue::new),
            id_token: token_response.id_token.map(SecretValue::new),
            expires_in_secs: Some(token_response.expires_in),
        })
    }
}

impl fmt::Debug for CognitoOidcProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CognitoOidcProvider")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
struct CognitoAccessClaims {
    iss: String,
    sub: String,
    exp: u64,
    iat: u64,
    auth_time: u64,
    token_use: String,
    client_id: String,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    aud: Option<Audience>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

impl Audience {
    fn contains(&self, expected: &str) -> bool {
        match self {
            Audience::Single(value) => value == expected,
            Audience::Multiple(values) => values.iter().any(|value| value == expected),
        }
    }
}

#[derive(Deserialize)]
struct CognitoTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
}

async fn read_bounded(
    mut response: Response<Incoming>,
    limit: usize,
) -> Result<Vec<u8>, AuthError> {
    if response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > u64::try_from(limit).unwrap_or(u64::MAX))
    {
        return Err(AuthError::IdentityProviderRejected);
    }
    let mut body = Vec::new();
    while let Some(frame) = response.frame().await {
        let frame = frame.map_err(|_| AuthError::IdentityProviderUnavailable)?;
        let Ok(chunk) = frame.into_data() else {
            continue;
        };
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(AuthError::IdentityProviderRejected);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn map_provider_status(status: StatusCode) -> AuthError {
    if status.is_server_error() {
        AuthError::IdentityProviderUnavailable
    } else {
        AuthError::IdentityProviderRejected
    }
}

fn parse_https_url(raw: &str) -> Result<Url, AuthError> {
    let url = Url::parse(raw).map_err(|_| AuthError::ConfigurationInvalid)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AuthError::ConfigurationInvalid);
    }
    Ok(url)
}

fn is_safe_identifier(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn is_valid_scope(scope: &str) -> bool {
    !scope.is_empty()
        && scope.len() <= MAX_SCOPE_BYTES
        && scope.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'/' | b'.' | b'_' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, encode};
    use serde::Serialize;

    fn settings() -> CognitoOidcSettings {
        CognitoOidcSettings {
            issuer: "https://cognito-idp.af-south-1.amazonaws.com/pool-id".to_owned(),
            client_id: "sitolo-mobile-client".to_owned(),
            resource_audience: None,
            token_endpoint: "https://auth.example.test/oauth2/token".to_owned(),
            redirect_uri: "sitolo://auth/callback".to_owned(),
            required_scopes: vec!["sitolo/read".to_owned()],
            request_timeout: Duration::from_secs(3),
            jwks_ttl: Duration::from_secs(300),
            unknown_kid_refresh_interval: Duration::from_secs(5),
            clock_skew: Duration::from_secs(30),
        }
    }

    fn config() -> CognitoOidcConfig {
        CognitoOidcConfig::new(settings()).unwrap()
    }

    #[derive(Serialize)]
    struct TestAccessClaims {
        iss: String,
        sub: String,
        exp: u64,
        iat: u64,
        auth_time: u64,
        token_use: String,
        client_id: String,
        scope: String,
    }

    async fn provider_with_fixture_key() -> CognitoOidcProvider {
        let provider = CognitoOidcProvider::new(config()).unwrap();
        let jwks: JwkSet =
            serde_json::from_slice(include_bytes!("../tests/fixtures/cognito-test-jwks.json"))
                .unwrap();
        let key = DecodingKey::from_jwk(&jwks.keys[0]).unwrap();
        *provider.jwks.write().await = Some(CachedJwks {
            keys: HashMap::from([("sitolo-test-key".to_owned(), key)]),
            expires_at: SystemTime::now() + Duration::from_secs(300),
            last_forced_refresh_at: None,
        });
        provider
    }

    fn signed_access_token(claims: TestAccessClaims) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("sitolo-test-key".to_owned());
        header.typ = Some("JWT".to_owned());
        encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_der(include_bytes!(
                "../tests/fixtures/cognito-test-private.der"
            )),
        )
        .unwrap()
    }

    fn valid_claims() -> TestAccessClaims {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        TestAccessClaims {
            iss: "https://cognito-idp.af-south-1.amazonaws.com/pool-id".to_owned(),
            sub: "provider-subject-123".to_owned(),
            exp: now + 300,
            iat: now - 30,
            auth_time: now - 60,
            token_use: "access".to_owned(),
            client_id: "sitolo-mobile-client".to_owned(),
            scope: "sitolo/read".to_owned(),
        }
    }

    #[test]
    fn jwks_location_is_derived_from_pinned_https_issuer() {
        let config = config();
        assert_eq!(
            config.jwks_url.as_str(),
            "https://cognito-idp.af-south-1.amazonaws.com/pool-id/.well-known/jwks.json"
        );
    }

    #[test]
    fn duplicate_jwks_key_ids_are_rejected() {
        let mut document: serde_json::Value =
            serde_json::from_slice(include_bytes!("../tests/fixtures/cognito-test-jwks.json"))
                .unwrap();
        let keys = document
            .get_mut("keys")
            .and_then(serde_json::Value::as_array_mut)
            .unwrap();
        keys.push(keys[0].clone());
        let body = serde_json::to_vec(&document).unwrap();

        assert!(matches!(
            parse_jwks(&body),
            Err(AuthError::IdentityKeyUnavailable)
        ));
    }

    #[test]
    fn configuration_rejects_http_and_unsafe_client_identifiers() {
        let mut http_issuer_settings = settings();
        http_issuer_settings.issuer = "http://identity.example.test/pool".to_owned();
        let http_issuer = CognitoOidcConfig::new(http_issuer_settings);
        assert!(matches!(http_issuer, Err(AuthError::ConfigurationInvalid)));

        let mut unsafe_client_settings = settings();
        unsafe_client_settings.client_id = "client\r\nforged".to_owned();
        let unsafe_client_id = CognitoOidcConfig::new(unsafe_client_settings);
        assert!(matches!(
            unsafe_client_id,
            Err(AuthError::ConfigurationInvalid)
        ));
    }

    #[tokio::test]
    async fn malformed_and_oversized_access_tokens_are_rejected_before_fetch() {
        let provider = CognitoOidcProvider::new(config()).unwrap();
        assert_eq!(
            provider.validate_assertion("not-a-jwt").await.unwrap_err(),
            AuthError::AuthenticationFailed
        );
        let oversized = "a".repeat(MAX_ACCESS_TOKEN_BYTES + 1);
        assert_eq!(
            provider.validate_assertion(&oversized).await.unwrap_err(),
            AuthError::AuthenticationFailed
        );
    }

    #[tokio::test]
    async fn valid_signed_access_token_returns_only_the_external_identity_fact() {
        let provider = provider_with_fixture_key().await;
        let token = signed_access_token(valid_claims());
        let identity = provider.validate_assertion(&token).await.unwrap();
        assert_eq!(
            identity.issuer,
            "https://cognito-idp.af-south-1.amazonaws.com/pool-id"
        );
        assert_eq!(identity.subject, "provider-subject-123");
        assert_eq!(identity.method, AuthenticationMethod::OidcAssertion);
        assert_eq!(identity.assurance_hint, Assurance::A1);
    }

    #[tokio::test]
    async fn wrong_issuer_client_token_use_or_scope_is_denied() {
        let provider = provider_with_fixture_key().await;
        let mut cases = Vec::new();
        let mut wrong_issuer = valid_claims();
        wrong_issuer.iss = "https://attacker.example.test/pool".to_owned();
        cases.push(wrong_issuer);
        let mut wrong_client = valid_claims();
        wrong_client.client_id = "another-client".to_owned();
        cases.push(wrong_client);
        let mut wrong_token_use = valid_claims();
        wrong_token_use.token_use = "id".to_owned();
        cases.push(wrong_token_use);
        let mut insufficient_scope = valid_claims();
        insufficient_scope.scope = "sitolo/write".to_owned();
        cases.push(insufficient_scope);

        for claims in cases {
            let token = signed_access_token(claims);
            assert_eq!(
                provider.validate_assertion(&token).await.unwrap_err(),
                AuthError::AuthenticationFailed
            );
        }
    }

    #[tokio::test]
    async fn invalid_rsa_signature_is_denied() {
        let provider = provider_with_fixture_key().await;
        let mut token = signed_access_token(valid_claims());
        let signature_index = token.rfind('.').unwrap() + 1;
        let replacement = if token.as_bytes()[signature_index] == b'A' {
            'B'
        } else {
            'A'
        };
        token.replace_range(
            signature_index..signature_index + 1,
            &replacement.to_string(),
        );
        assert_eq!(
            provider.validate_assertion(&token).await.unwrap_err(),
            AuthError::AuthenticationFailed
        );
    }

    #[test]
    fn audience_handles_string_and_array_encodings() {
        assert!(Audience::Single("resource-a".to_owned()).contains("resource-a"));
        assert!(!Audience::Single("resource-a".to_owned()).contains("resource-b"));
        assert!(Audience::Multiple(vec!["resource-a".to_owned()]).contains("resource-a"));
    }
}
