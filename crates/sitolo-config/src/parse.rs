//! Deterministic environment layer parsing. Environment is an input at startup,
//! never an authority consulted during request handling.

use std::collections::BTreeMap;

use crate::{ACCEPTED_KEYS, ENV_PREFIX, Environment, LogLevel, SecretClass, SecretRef, defaults};

/// A configuration key rejected during environment ingestion.
///
/// The payload is the canonical key name (for example `DATABASE__PORT`), never
/// the environment value: values can carry secret references and must not be
/// echoed into logs or error surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKey(
    /// Canonical key name, relative to the `SITOLO__` prefix.
    pub String,
);

/// Loads only the canonical `SITOLO__` configuration namespace and rejects
/// unknown keys.
///
/// The disjoint `SITOLO_SECRET__` namespace belongs to the development secret
/// provider and is deliberately not parsed here. It cannot match the
/// `SITOLO__` prefix because the character after `SITOLO_` differs.
#[derive(Debug, Clone, Copy)]
pub struct EnvLoader;

impl EnvLoader {
    /// Reads configuration from the current process environment.
    ///
    /// # Errors
    ///
    /// Returns [`UnknownKey`] when a `SITOLO__` key is not listed in
    /// [`ACCEPTED_KEYS`], when `RUNTIME__ENVIRONMENT` is not one of
    /// `development`, `staging`, or `production`, or when an accepted key's
    /// value does not parse into its typed representation (socket address,
    /// integer, boolean, float, or secret reference).
    pub fn load() -> Result<defaults::Builder, UnknownKey> {
        Self::from_pairs(std::env::vars())
    }

    /// Builds the default layer from an explicit set of key/value pairs.
    ///
    /// Keys outside the `SITOLO__` namespace are ignored, which keeps the
    /// `SITOLO_SECRET__` value namespace disjoint from configuration. Duplicate
    /// keys resolve to the last occurrence in input order.
    ///
    /// # Errors
    ///
    /// Returns [`UnknownKey`] under the same conditions as [`EnvLoader::load`]:
    /// an unaccepted `SITOLO__` key, an unknown environment selector, or a
    /// value that does not parse into its typed representation.
    pub fn from_pairs(
        pairs: impl IntoIterator<Item = (String, String)>,
    ) -> Result<defaults::Builder, UnknownKey> {
        // Duplicate keys: last occurrence in input order wins. The map
        // iteration itself is sorted and therefore deterministic.
        let mut values = BTreeMap::new();
        for (key, value) in pairs {
            if let Some(name) = key.strip_prefix(ENV_PREFIX) {
                values.insert(name.to_string(), value);
            }
        }
        // F-002: the environment selector runs before any defaults are
        // chosen, so production/staging never inherit development-only
        // values. An absent selector keeps the development baseline.
        let mut b = match values.get("RUNTIME__ENVIRONMENT").map(String::as_str) {
            None | Some("development") => defaults::Builder::development(),
            Some("staging") => defaults::Builder::staging(),
            Some("production") => defaults::Builder::production(),
            Some(_) => return Err(UnknownKey("RUNTIME__ENVIRONMENT".to_string())),
        };
        for (key, value) in values {
            apply(&mut b, &key, &value)?;
        }
        Ok(b)
    }
}

fn apply(b: &mut defaults::Builder, key: &str, value: &str) -> Result<(), UnknownKey> {
    if !ACCEPTED_KEYS.contains(&key) {
        return Err(UnknownKey(key.to_string()));
    }
    match key {
        "RUNTIME__ENVIRONMENT" => {
            b.environment = match value {
                "development" => Environment::Development,
                "staging" => Environment::Staging,
                "production" => Environment::Production,
                _ => return Err(UnknownKey(key.to_string())),
            };
        }
        "RUNTIME__SERVICE_NAME" => b.service_name = value.into(),
        "RUNTIME__SERVICE_VERSION" => b.service_version = value.into(),
        "HTTP__BIND_ADDRESS" => {
            b.bind_address = parse_value(key, value)?;
        }
        "HTTP__MAX_BODY_BYTES" => b.max_request_body_bytes = parse_value(key, value)?,
        "HTTP__REQUEST_HEADER_TIMEOUT_MS" => b.request_header_timeout_ms = parse_value(key, value)?,
        "HTTP__KEEPALIVE_TIMEOUT_MS" => b.keepalive_timeout_ms = parse_value(key, value)?,
        "DATABASE__HOST" => b.db_host = value.into(),
        "DATABASE__PORT" => b.db_port = parse_value(key, value)?,
        "DATABASE__NAME" => b.db_name = value.into(),
        "DATABASE__SCHEMA" => b.db_schema = value.into(),
        "DATABASE__USER" => b.db_user = value.into(),
        "IDENTITY__OIDC__ISSUER" => b.oidc_issuer = Some(value.into()),
        "IDENTITY__OIDC__CLIENT_ID" => b.oidc_client_id = Some(value.into()),
        "IDENTITY__OIDC__TOKEN_ENDPOINT" => b.oidc_token_endpoint = Some(value.into()),
        "IDENTITY__OIDC__REDIRECT_URI" => b.oidc_redirect_uri = Some(value.into()),
        "DATABASE__PASSWORD_REF" => {
            b.db_password_ref = Some(secret_ref(key, value)?);
        }
        "WORKER__DATABASE__PASSWORD_REF" => {
            b.worker_db_password_ref = Some(secret_ref(key, value)?);
        }
        "DATABASE__POOL_MIN" => b.db_pool_min = parse_value(key, value)?,
        "DATABASE__POOL_MAX" => b.db_pool_max = parse_value(key, value)?,
        "DATABASE__ACQUIRE_TIMEOUT_MS" => b.db_acquire_timeout_ms = parse_value(key, value)?,
        "TELEMETRY__OTLP_ENDPOINT" => {
            b.otel_endpoint = if value.is_empty() {
                None
            } else {
                Some(value.into())
            };
        }
        "TELEMETRY__EXPORT_TIMEOUT_MS" => b.otel_export_timeout_ms = parse_value(key, value)?,
        "TELEMETRY__MAX_QUEUE" => b.otel_max_queue = parse_value(key, value)?,
        "TELEMETRY__TRACE_SAMPLE_RATIO" => {
            b.trace_sample_ratio = parse_value(key, value)?;
        }
        "LOG__LEVEL" => {
            b.log_level = match value {
                "trace" => LogLevel::Trace,
                "debug" => LogLevel::Debug,
                "info" => LogLevel::Info,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                _ => return Err(UnknownKey(key.to_string())),
            };
        }
        "SECRETS__ALLOW_LOCAL_PROVIDER" => {
            b.allow_local_secret_provider = parse_value(key, value)?;
        }
        "RUNTIME__CONFIG_SCHEMA_VERSION" => b.config_schema_version = parse_value(key, value)?,
        "EVENTS__RELAY__AWS_REGION" => b.relay_aws_region = Some(value.into()),
        "EVENTS__RELAY__SNS_TOPIC_ARN" => b.relay_sns_topic_arn = Some(value.into()),
        "EVENTS__RELAY__BATCH_SIZE" => b.relay_batch_size = parse_value(key, value)?,
        "EVENTS__RELAY__MAX_ATTEMPTS" => b.relay_max_attempts = parse_value(key, value)?,
        "EVENTS__RELAY__BACKOFF_MAX_MS" => b.relay_backoff_max_ms = parse_value(key, value)?,
        "SECRETS__AWS_REGION" => b.secrets_aws_region = Some(value.into()),
        // Defense in depth: `ACCEPTED_KEYS` is checked above, so this arm is
        // not expected to be reached. It is intentionally a structured
        // error rather than `unreachable!()` so that a future maintenance
        // drift between the key catalogue and this match (e.g. a new key
        // added to one but not the other) fails closed as a normal
        // configuration error at startup instead of an unhandled panic.
        _ => return Err(UnknownKey(key.to_string())),
    }
    Ok(())
}

/// Parses a typed value, attributing any failure to `key`.
///
/// The concrete parse error is deliberately collapsed into [`UnknownKey`]:
/// startup diagnostics report the canonical key only, so a malformed value
/// (which may contain a secret reference) is never echoed back.
fn parse_value<T: std::str::FromStr>(key: &str, value: &str) -> Result<T, UnknownKey> {
    match value.parse() {
        Ok(parsed) => Ok(parsed),
        Err(_) => Err(UnknownKey(key.to_string())),
    }
}

/// Builds a database secret reference, attributing any failure to `key`.
fn secret_ref(key: &str, value: &str) -> Result<SecretRef, UnknownKey> {
    match SecretRef::new(SecretClass::Database, value) {
        Ok(reference) => Ok(reference),
        Err(_) => Err(UnknownKey(key.to_string())),
    }
}
