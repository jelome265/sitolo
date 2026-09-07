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

    #[test]
    fn catalogue_and_parser_key_contracts_are_identical() {
        let mut catalogue_keys: Vec<_> = catalogue().iter().map(|field| field.key).collect();
        let mut parser_keys = ACCEPTED_KEYS.to_vec();
        catalogue_keys.sort_unstable();
        parser_keys.sort_unstable();
        assert_eq!(catalogue_keys, parser_keys);
    }
}
