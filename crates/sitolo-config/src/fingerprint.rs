//! Stable, secret-free configuration drift representation.

use sha2::{Digest, Sha256};

use crate::AppConfig;

/// Changes only through an explicit fingerprint compatibility decision.
pub const FINGERPRINT_FORMAT_VERSION: u8 = 2;

/// Canonical, length-prefixed, non-secret effective configuration.
///
/// Length prefixes make Unicode, delimiters, and newline characters
/// unambiguous without relying on debug formatting or source ordering. Secret
/// *values* never enter this representation; secret *references* do, so
/// rotating a reference is a visible configuration change while rotating the
/// value behind it is not.
#[must_use]
pub fn canonical_non_secret_config(config: &AppConfig) -> String {
    let mut output = String::with_capacity(1024);
    write_runtime_and_http(&mut output, config);
    write_database_identity(&mut output, config);
    write_identity(&mut output, config);
    write_secrets_and_pool(&mut output, config);
    write_telemetry(&mut output, config);
    write_relay(&mut output, config);
    output
}

/// Fingerprint format, schema, environment, service identity, and HTTP limits.
fn write_runtime_and_http(output: &mut String, config: &AppConfig) {
    line(
        output,
        "fingerprint_format",
        &FINGERPRINT_FORMAT_VERSION.to_string(),
    );
    line(
        output,
        "config_schema_version",
        &config.config_schema_version.to_string(),
    );
    line(output, "environment", config.environment.as_str());
    line(output, "service_name", &config.service_name);
    line(output, "service_version", &config.service_version);
    line(output, "bind_address", &config.bind_address.to_string());
    line(
        output,
        "max_request_body_bytes",
        &config.max_request_body_bytes.to_string(),
    );
    line(
        output,
        "request_header_timeout_ms",
        &config.request_header_timeout_ms.to_string(),
    );
    line(
        output,
        "keepalive_timeout_ms",
        &config.keepalive_timeout_ms.to_string(),
    );
}

/// Database host, port, catalogue, schema, and role.
fn write_database_identity(output: &mut String, config: &AppConfig) {
    line(output, "db_host", &config.db_host);
    line(output, "db_port", &config.db_port.to_string());
    line(output, "db_name", &config.db_name);
    line(output, "db_schema", &config.db_schema);
    line(output, "db_user", &config.db_user);
}

/// OIDC identity settings.
fn write_identity(output: &mut String, config: &AppConfig) {
    optional_line(output, "oidc_issuer", config.oidc_issuer.as_deref());
    optional_line(output, "oidc_client_id", config.oidc_client_id.as_deref());
    optional_line(
        output,
        "oidc_token_endpoint",
        config.oidc_token_endpoint.as_deref(),
    );
    optional_line(
        output,
        "oidc_redirect_uri",
        config.oidc_redirect_uri.as_deref(),
    );
}

/// Secret references (never values) and database pool bounds.
fn write_secrets_and_pool(output: &mut String, config: &AppConfig) {
    line(
        output,
        "db_password_ref",
        &config.db_password_ref.to_string(),
    );
    optional_line(
        output,
        "worker_db_password_ref",
        config
            .worker_db_password_ref
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
    );
    line(output, "db_pool_min", &config.db_pool_min.to_string());
    line(output, "db_pool_max", &config.db_pool_max.to_string());
    line(
        output,
        "db_acquire_timeout_ms",
        &config.db_acquire_timeout_ms.to_string(),
    );
}

/// Telemetry endpoint, limits, sampling, and logging posture.
fn write_telemetry(output: &mut String, config: &AppConfig) {
    optional_line(output, "otel_endpoint", config.otel_endpoint.as_deref());
    line(
        output,
        "otel_export_timeout_ms",
        &config.otel_export_timeout_ms.to_string(),
    );
    line(output, "otel_max_queue", &config.otel_max_queue.to_string());
    line(
        output,
        "trace_sample_ratio",
        &config.trace_sample_ratio.to_string(),
    );
    line(output, "log_level", config.log_level.as_str());
    line(
        output,
        "allow_local_secret_provider",
        &config.allow_local_secret_provider.to_string(),
    );
}

/// Event relay destination and the managed-secret provider region.
fn write_relay(output: &mut String, config: &AppConfig) {
    optional_line(
        output,
        "relay_aws_region",
        config.relay_aws_region.as_deref(),
    );
    optional_line(
        output,
        "relay_sns_topic_arn",
        config.relay_sns_topic_arn.as_deref(),
    );
    line(
        output,
        "relay_batch_size",
        &config.relay_batch_size.to_string(),
    );
    line(
        output,
        "relay_max_attempts",
        &config.relay_max_attempts.to_string(),
    );
    line(
        output,
        "relay_backoff_max_ms",
        &config.relay_backoff_max_ms.to_string(),
    );
    optional_line(
        output,
        "secrets_aws_region",
        config.secrets_aws_region.as_deref(),
    );
}

/// Appends one length-prefixed `key=len:value` line.
fn line(output: &mut String, key: &str, value: &str) {
    output.push_str(key);
    output.push('=');
    output.push_str(&value.len().to_string());
    output.push(':');
    output.push_str(value);
    output.push('\n');
}

/// Appends an optional value with a distinct presence marker.
fn optional_line(output: &mut String, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        output.push_str(key);
        output.push_str("=some:");
        output.push_str(&value.len().to_string());
        output.push(':');
        output.push_str(value);
        output.push('\n');
    } else {
        output.push_str(key);
        output.push_str("=none\n");
    }
}

/// Lowercase hexadecimal encoding of `bytes`.
fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for &byte in bytes {
        for nibble in [byte >> 4, byte & 0x0f] {
            if let Some(&digit) = HEX.get(usize::from(nibble)) {
                output.push(char::from(digit));
            }
        }
    }
    output
}

/// SHA-256 over [`canonical_non_secret_config`].
#[must_use]
pub fn config_fingerprint(config: &AppConfig) -> String {
    let digest = Sha256::digest(canonical_non_secret_config(config).as_bytes());
    format!("sha256:{}", lower_hex(digest.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LogLevel, SecretClass, SecretRef, defaults, validate};
    use std::error::Error;

    fn base() -> Result<AppConfig, Box<dyn Error>> {
        Ok(validate(defaults::Builder::development())?)
    }

    /// Exhaustively names every `AppConfig` field so adding one forces this
    /// fingerprint test module to review its serialization and change test.
    fn assert_fingerprint_field_inventory_is_exhaustive(config: &AppConfig) {
        let AppConfig {
            environment: _,
            service_name: _,
            service_version: _,
            bind_address: _,
            max_request_body_bytes: _,
            request_header_timeout_ms: _,
            keepalive_timeout_ms: _,
            db_host: _,
            db_port: _,
            db_name: _,
            db_schema: _,
            db_user: _,
            db_password_ref: _,
            worker_db_password_ref: _,
            oidc_issuer: _,
            oidc_client_id: _,
            oidc_token_endpoint: _,
            oidc_redirect_uri: _,
            db_pool_min: _,
            db_pool_max: _,
            db_acquire_timeout_ms: _,
            otel_endpoint: _,
            otel_export_timeout_ms: _,
            otel_max_queue: _,
            trace_sample_ratio: _,
            log_level: _,
            allow_local_secret_provider: _,
            config_schema_version: _,
            relay_aws_region: _,
            relay_sns_topic_arn: _,
            relay_batch_size: _,
            relay_max_attempts: _,
            relay_backoff_max_ms: _,
            secrets_aws_region: _,
        } = config;
    }

    #[test]
    fn fingerprint_tests_inventory_every_configuration_field() -> Result<(), Box<dyn Error>> {
        assert_fingerprint_field_inventory_is_exhaustive(&base()?);
        Ok(())
    }

    #[test]
    fn digest_is_lowercase_sha256() -> Result<(), Box<dyn Error>> {
        let fingerprint = config_fingerprint(&base()?);
        let digest = fingerprint
            .strip_prefix("sha256:")
            .ok_or_else(|| std::io::Error::other("fingerprint must use the sha256 prefix"))?;
        assert_eq!(digest.len(), 64);
        assert!(
            digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        Ok(())
    }

    #[test]
    fn lower_hex_has_known_vector() {
        assert_eq!(
            lower_hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn every_effective_non_secret_field_changes_fingerprint() -> Result<(), Box<dyn Error>> {
        let original = base()?;
        let baseline = config_fingerprint(&original);
        macro_rules! changed {
            ($field:ident, $value:expr) => {{
                let mut config = original.clone();
                config.$field = $value;
                assert_ne!(baseline, config_fingerprint(&config), stringify!($field));
            }};
        }
        changed!(environment, crate::Environment::Staging);
        changed!(service_name, "other".into());
        changed!(service_version, "1.0.0".into());
        changed!(bind_address, "127.0.0.1:8081".parse()?);
        changed!(max_request_body_bytes, 3_000_000);
        changed!(request_header_timeout_ms, 6_000);
        changed!(keepalive_timeout_ms, 31_000);
        changed!(db_host, "db.example".into());
        changed!(db_port, 5433);
        changed!(db_name, "other".into());
        changed!(db_schema, "tenant_data".into());
        changed!(db_user, "other_user".into());
        changed!(oidc_issuer, Some("https://identity.example/issuer".into()));
        changed!(oidc_client_id, Some("other-client".into()));
        changed!(
            oidc_token_endpoint,
            Some("https://identity.example/token".into())
        );
        changed!(
            oidc_redirect_uri,
            Some("https://sitolo.example/callback".into())
        );
        changed!(
            worker_db_password_ref,
            Some(
                SecretRef::new(SecretClass::Database, "worker/rotated")
                    .map_err(std::io::Error::other)?
            )
        );
        changed!(db_pool_min, 6);
        changed!(db_pool_max, 21);
        changed!(db_acquire_timeout_ms, 3_000);
        changed!(otel_endpoint, Some("https://collector.example".into()));
        changed!(otel_export_timeout_ms, 4_000);
        changed!(otel_max_queue, 2_049);
        changed!(trace_sample_ratio, 0.2);
        changed!(log_level, LogLevel::Warn);
        changed!(allow_local_secret_provider, false);
        changed!(config_schema_version, 2);
        changed!(relay_aws_region, Some("af-south-1".into()));
        changed!(
            relay_sns_topic_arn,
            Some("arn:aws:sns:af-south-1:123456789012:events.fifo".into())
        );
        changed!(relay_batch_size, 11);
        changed!(relay_max_attempts, 9);
        changed!(relay_backoff_max_ms, 3_000_000);
        changed!(secrets_aws_region, Some("af-south-1".into()));
        Ok(())
    }

    #[test]
    fn optional_fields_distinguish_absent_from_present_sentinel_values()
    -> Result<(), Box<dyn Error>> {
        let mut absent = base()?;
        absent.oidc_client_id = None;
        let mut present = absent.clone();
        present.oidc_client_id = Some("<none>".into());
        assert_ne!(
            canonical_non_secret_config(&absent),
            canonical_non_secret_config(&present)
        );
        assert_ne!(config_fingerprint(&absent), config_fingerprint(&present));
        Ok(())
    }

    #[test]
    fn reference_changes_but_secret_value_does_not_enter_canonical_input()
    -> Result<(), Box<dyn Error>> {
        let mut config = base()?;
        let initial = config_fingerprint(&config);
        config.db_password_ref = SecretRef::new(SecretClass::Database, "development/rotated/db")?;
        assert_ne!(initial, config_fingerprint(&config));
        assert!(!canonical_non_secret_config(&config).contains("TEST_ONLY_DATABASE_SECRET"));
        Ok(())
    }
}
