//! Stable, secret-free configuration drift representation.

use crate::AppConfig;
use sha2::{Digest, Sha256};

/// Changes only through an explicit fingerprint compatibility decision.
pub const FINGERPRINT_FORMAT_VERSION: u8 = 1;

/// Canonical, length-prefixed, non-secret effective configuration.
///
/// Length prefixes make Unicode, delimiters, and newline characters
/// unambiguous without relying on debug formatting or source ordering.
pub fn canonical_non_secret_config(config: &AppConfig) -> String {
    let mut output = String::with_capacity(1024);
    line(
        &mut output,
        "fingerprint_format",
        &FINGERPRINT_FORMAT_VERSION.to_string(),
    );
    line(
        &mut output,
        "config_schema_version",
        &config.config_schema_version.to_string(),
    );
    line(&mut output, "environment", config.environment.as_str());
    line(&mut output, "service_name", &config.service_name);
    line(&mut output, "service_version", &config.service_version);
    line(
        &mut output,
        "bind_address",
        &config.bind_address.to_string(),
    );
    line(
        &mut output,
        "max_request_body_bytes",
        &config.max_request_body_bytes.to_string(),
    );
    line(
        &mut output,
        "request_header_timeout_ms",
        &config.request_header_timeout_ms.to_string(),
    );
    line(
        &mut output,
        "keepalive_timeout_ms",
        &config.keepalive_timeout_ms.to_string(),
    );
    line(&mut output, "db_host", &config.db_host);
    line(&mut output, "db_port", &config.db_port.to_string());
    line(&mut output, "db_name", &config.db_name);
    line(&mut output, "db_user", &config.db_user);
    line(
        &mut output,
        "db_password_ref",
        &config.db_password_ref.to_string(),
    );
    line(&mut output, "db_pool_min", &config.db_pool_min.to_string());
    line(&mut output, "db_pool_max", &config.db_pool_max.to_string());
    line(
        &mut output,
        "db_acquire_timeout_ms",
        &config.db_acquire_timeout_ms.to_string(),
    );
    line(
        &mut output,
        "otel_endpoint",
        config.otel_endpoint.as_deref().unwrap_or("<none>"),
    );
    line(
        &mut output,
        "otel_export_timeout_ms",
        &config.otel_export_timeout_ms.to_string(),
    );
    line(
        &mut output,
        "otel_max_queue",
        &config.otel_max_queue.to_string(),
    );
    line(
        &mut output,
        "trace_sample_ratio",
        &config.trace_sample_ratio.to_string(),
    );
    line(&mut output, "log_level", config.log_level.as_str());
    line(
        &mut output,
        "allow_local_secret_provider",
        &config.allow_local_secret_provider.to_string(),
    );
    output
}

fn line(output: &mut String, key: &str, value: &str) {
    output.push_str(key);
    output.push('=');
    output.push_str(&value.len().to_string());
    output.push(':');
    output.push_str(value);
    output.push('\n');
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

/// SHA-256 over [`canonical_non_secret_config`].
pub fn config_fingerprint(config: &AppConfig) -> String {
    let digest = Sha256::digest(canonical_non_secret_config(config).as_bytes());
    format!("sha256:{}", lower_hex(digest.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LogLevel, SecretClass, SecretRef, defaults, validate};

    fn base() -> crate::AppConfig {
        validate(defaults::Builder::development()).unwrap()
    }

    #[test]
    fn digest_is_lowercase_sha256() {
        let fingerprint = config_fingerprint(&base());
        assert!(fingerprint.starts_with("sha256:"));
        let digest = &fingerprint[7..];
        assert_eq!(digest.len(), 64);
        assert!(
            digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
    }

    #[test]
    fn lower_hex_has_known_vector() {
        assert_eq!(
            lower_hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn every_effective_non_secret_field_changes_fingerprint() {
        let original = base();
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
        changed!(bind_address, "127.0.0.1:8081".parse().unwrap());
        changed!(max_request_body_bytes, 3_000_000);
        changed!(request_header_timeout_ms, 6_000);
        changed!(keepalive_timeout_ms, 31_000);
        changed!(db_host, "db.example".into());
        changed!(db_port, 5433);
        changed!(db_name, "other".into());
        changed!(db_user, "other_user".into());
        changed!(db_pool_min, 6);
        changed!(db_pool_max, 21);
        changed!(db_acquire_timeout_ms, 3_000);
        changed!(otel_endpoint, Some("https://collector.example".into()));
        changed!(otel_export_timeout_ms, 4_000);
        changed!(otel_max_queue, 2_049);
        changed!(trace_sample_ratio, 0.2);
        changed!(log_level, LogLevel::Warn);
        changed!(allow_local_secret_provider, false);
        changed!(config_schema_version, 3);
    }

    #[test]
    fn reference_changes_but_secret_value_does_not_enter_canonical_input() {
        let mut config = base();
        let initial = config_fingerprint(&config);
        config.db_password_ref =
            SecretRef::new(SecretClass::Database, "development/rotated/db").unwrap();
        assert_ne!(initial, config_fingerprint(&config));
        assert!(!canonical_non_secret_config(&config).contains("TEST_ONLY_DATABASE_SECRET"));
    }
}
