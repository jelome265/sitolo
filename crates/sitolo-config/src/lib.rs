//! Typed process configuration for Sitolo.
//!
//! Phase 2 specification §5-§11 and Appendices A-C. This crate owns the
//! configuration contract: a canonical namespace (`SITOLO__`), compiled safe
//! defaults layered under environment variables, explicit normalization, and a
//! layered validation pipeline (syntax → type → semantic → security →
//! environment → cross-field). It does NOT own secret *values*; it owns secret
//! *references* (class B configuration).
//!
//! Domain code must not call `std::env::var` directly (Phase 2 §3.1). All
//! process configuration flows through this crate.
//!
//! ## Security invariants (Appendix C, executable)
//!
//! - Production cannot use the development secret provider.
//! - Raw database-password environment variables are rejected in production.
//! - Required secret references must resolve; absence is a startup failure.
//! - Remote telemetry endpoints must use TLS.
//! - Effective configuration is fingerprintable over non-secret fields only;
//!   rotating a secret value does not change the fingerprint.
#![forbid(unsafe_code)]

mod field;
mod fingerprint;
mod model;
mod parse;
mod validate;

pub use field::{ACCEPTED_KEYS, ConfigClass, ConfigField, catalogue};
pub use fingerprint::{
    FINGERPRINT_FORMAT_VERSION, canonical_non_secret_config, config_fingerprint,
};
pub use model::{
    AppConfig, DatabaseRuntimeConfig, DatabaseTarget, Environment, LogLevel, ceilings, defaults,
};
pub use parse::{EnvLoader, UnknownKey};
pub use sitolo_security::{SecretClass, SecretRef};
pub use validate::{ConfigProblem, ConfigValidationError, ValidationLayer, validate};

/// Canonical environment variable namespace prefix.
pub const ENV_PREFIX: &str = "SITOLO__";

/// Separator between configuration sections in environment variable names.
pub const ENV_SEPARATOR: &str = "__";

/// Config-schema versions this binary understands. Unknown schema fails
/// startup (Appendix A: `config_schema_version`).
pub const KNOWN_SCHEMA_VERSIONS: &[u32] = &[2];

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn production_rejects_local_secret_provider() {
        let mut config = defaults::Builder::production();
        config.allow_local_secret_provider = true;
        assert!(validate(config).is_err());
    }

    #[test]
    fn remote_telemetry_requires_tls() {
        let mut config = defaults::Builder::development();
        config.otel_endpoint = Some("http://collector.example".into());
        assert!(validate(config).is_err());
    }

    #[test]
    fn fingerprint_is_deterministic_and_excludes_secret_value() {
        let first = validate(defaults::Builder::development()).unwrap();
        let second = validate(defaults::Builder::development()).unwrap();
        assert_eq!(config_fingerprint(&first), config_fingerprint(&second));
    }

    #[test]
    fn pool_inversion_is_rejected() {
        let mut config = defaults::Builder::development();
        config.db_pool_min = 21;
        config.db_pool_max = 20;
        assert!(validate(config).is_err());
    }

    #[test]
    fn zero_database_port_is_rejected() {
        let mut config = defaults::Builder::development();
        config.db_port = 0;
        assert!(validate(config).is_err());
    }

    fn pair(key: &str, value: &str) -> (String, String) {
        (format!("SITOLO__{key}"), value.to_string())
    }

    #[test]
    fn environment_selector_chooses_baseline_before_overlays() {
        let staging = EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "staging")]).unwrap();
        assert_eq!(staging.environment, Environment::Staging);
        let production =
            EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "production")]).unwrap();
        assert_eq!(production.environment, Environment::Production);
        assert!(EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "qa")]).is_err());
    }

    #[test]
    fn production_baseline_requires_explicit_database_identity() {
        // F-002: no implicit development database or secret namespace.
        let builder = EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "production")]).unwrap();
        assert!(validate(builder).is_err());
        let staging = EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "staging")]).unwrap();
        assert!(validate(staging).is_err());
    }

    #[test]
    fn production_rejects_development_secret_namespace() {
        // F-003: a fully explicit production config with a development
        // secret namespace is rejected; a production namespace is accepted.
        let mut dev_ns = defaults::Builder::production();
        dev_ns.db_host = "db.internal".into();
        dev_ns.db_name = "sitolo".into();
        dev_ns.db_user = "sitolo_api".into();
        dev_ns.db_password_ref =
            Some(SecretRef::new(SecretClass::Database, "development/sitolo/db").unwrap());
        assert!(validate(dev_ns).is_err());

        let mut prod_ns = defaults::Builder::production();
        prod_ns.db_host = "db.internal".into();
        prod_ns.db_name = "sitolo".into();
        prod_ns.db_user = "sitolo_api".into();
        prod_ns.db_password_ref =
            Some(SecretRef::new(SecretClass::Database, "production/sitolo/db").unwrap());
        assert!(validate(prod_ns).is_ok());
    }

    #[test]
    fn telemetry_endpoint_transport_is_explicit() {
        // F-004: (endpoint, production, expected valid).
        for (endpoint, production, valid) in [
            ("https://collector.example", false, true),
            ("https://collector.example", true, true),
            ("http://collector.example", false, false),
            ("http://collector.example", true, false),
            ("collector.example:4317", false, false),
            ("collector.example:4317", true, false),
            ("http://localhost:4317", false, true),
            ("http://localhost:4317", true, false),
            ("localhost:4317", false, true),
            ("localhost:4317", true, false),
        ] {
            let mut builder = defaults::Builder::development();
            if production {
                builder.environment = Environment::Production;
                builder.allow_local_secret_provider = false;
                // F-003: production rows need a production secret namespace
                // so only the endpoint rule is under test here.
                builder.db_password_ref =
                    Some(SecretRef::new(SecretClass::Database, "production/sitolo/db").unwrap());
            }
            builder.otel_endpoint = Some(endpoint.into());
            assert_eq!(
                validate(builder).is_ok(),
                valid,
                "endpoint={endpoint} production={production}"
            );
        }
    }

    #[test]
    fn service_identity_rejects_control_characters() {
        // F-005.
        let mut builder = defaults::Builder::development();
        builder.service_name = "sitolo\ninjected".into();
        assert!(validate(builder).is_err());
        let mut builder = defaults::Builder::development();
        builder.service_version = "1.0.0\r".into();
        assert!(validate(builder).is_err());
    }

    #[test]
    fn catalogue_and_parser_key_contracts_are_identical() {
        let mut catalogue_keys: Vec<_> = catalogue().iter().map(|field| field.key).collect();
        let mut parser_keys = ACCEPTED_KEYS.to_vec();
        catalogue_keys.sort_unstable();
        parser_keys.sort_unstable();
        assert_eq!(catalogue_keys, parser_keys);
    }
}
