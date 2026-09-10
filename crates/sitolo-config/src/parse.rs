//! Deterministic environment layer parsing. Environment is an input at startup,
//! never an authority consulted during request handling.
use crate::{ACCEPTED_KEYS, ENV_PREFIX, defaults};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKey(pub String);

/// Loads only the canonical `SITOLO__` namespace and rejects unknown keys.
pub struct EnvLoader;
impl EnvLoader {
    pub fn load() -> Result<defaults::Builder, UnknownKey> {
        Self::from_pairs(std::env::vars())
    }
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
    use crate::{Environment, LogLevel, SecretClass, SecretRef};
    let bad = || Err(UnknownKey(key.to_string()));
    if !ACCEPTED_KEYS.contains(&key) {
        return bad();
    }
    match key {
        "RUNTIME__ENVIRONMENT" => {
            b.environment = match value {
                "development" => Environment::Development,
                "staging" => Environment::Staging,
                "production" => Environment::Production,
                _ => return bad(),
            }
        }
        "RUNTIME__SERVICE_NAME" => b.service_name = value.into(),
        "RUNTIME__SERVICE_VERSION" => b.service_version = value.into(),
        "HTTP__BIND_ADDRESS" => {
            b.bind_address = value.parse().map_err(|_| UnknownKey(key.into()))?
        }
        "HTTP__MAX_BODY_BYTES" => b.max_request_body_bytes = number(key, value)?,
        "HTTP__REQUEST_HEADER_TIMEOUT_MS" => b.request_header_timeout_ms = number(key, value)?,
        "HTTP__KEEPALIVE_TIMEOUT_MS" => b.keepalive_timeout_ms = number(key, value)?,
        "DATABASE__HOST" => b.db_host = value.into(),
        "DATABASE__PORT" => b.db_port = number(key, value)?,
        "DATABASE__NAME" => b.db_name = value.into(),
        "DATABASE__USER" => b.db_user = value.into(),
        "DATABASE__PASSWORD_REF" => {
            b.db_password_ref = Some(
                SecretRef::new(SecretClass::Database, value).map_err(|_| UnknownKey(key.into()))?,
            )
        }
        "DATABASE__POOL_MIN" => b.db_pool_min = number(key, value)?,
        "DATABASE__POOL_MAX" => b.db_pool_max = number(key, value)?,
        "DATABASE__ACQUIRE_TIMEOUT_MS" => b.db_acquire_timeout_ms = number(key, value)?,
        "TELEMETRY__OTLP_ENDPOINT" => {
            b.otel_endpoint = if value.is_empty() {
                None
            } else {
                Some(value.into())
            }
        }
        "TELEMETRY__EXPORT_TIMEOUT_MS" => b.otel_export_timeout_ms = number(key, value)?,
        "TELEMETRY__MAX_QUEUE" => b.otel_max_queue = number(key, value)?,
        "TELEMETRY__TRACE_SAMPLE_RATIO" => {
            b.trace_sample_ratio = value.parse().map_err(|_| UnknownKey(key.into()))?
        }
        "LOG__LEVEL" => {
            b.log_level = match value {
                "trace" => LogLevel::Trace,
                "debug" => LogLevel::Debug,
                "info" => LogLevel::Info,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                _ => return bad(),
            }
        }
        "SECRETS__ALLOW_LOCAL_PROVIDER" => {
            b.allow_local_secret_provider = value.parse().map_err(|_| UnknownKey(key.into()))?
        }
        "RUNTIME__CONFIG_SCHEMA_VERSION" => b.config_schema_version = number(key, value)?,
        // Defense in depth: `ACCEPTED_KEYS` is checked above, so this arm is
        // not expected to be reached. It is intentionally a structured
        // error rather than `unreachable!()` so that a future maintenance
        // drift between the key catalogue and this match (e.g. a new key
        // added to one but not the other) fails closed as a normal
        // configuration error at startup instead of an unhandled panic.
        _ => return bad(),
    };
    Ok(())
}
fn number<T: std::str::FromStr>(key: &str, value: &str) -> Result<T, UnknownKey> {
    value.parse().map_err(|_| UnknownKey(key.into()))
}
