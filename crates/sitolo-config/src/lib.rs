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

/// Canonical environment variable namespace prefix for configuration.
///
/// Development secret *values* use the disjoint `SITOLO_SECRET__` namespace
/// (`sitolo_security::SECRET_ENV_PREFIX`) and are never parsed here.
pub const ENV_PREFIX: &str = "SITOLO__";

/// Separator between configuration sections in environment variable names.
pub const ENV_SEPARATOR: &str = "__";

/// Config-schema versions this binary understands. Unknown schema fails
/// startup (Appendix A: `config_schema_version`).
pub const KNOWN_SCHEMA_VERSIONS: &[u32] = &[2, 3];

#[cfg(test)]
mod contract_tests {
    use super::*;
    use std::error::Error;
    use std::io;

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
    fn fingerprint_is_deterministic_and_excludes_secret_value() -> Result<(), Box<dyn Error>> {
        let first = validate(defaults::Builder::development())?;
        let second = validate(defaults::Builder::development())?;
        assert_eq!(config_fingerprint(&first), config_fingerprint(&second));
        Ok(())
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
    fn environment_selector_chooses_baseline_before_overlays() -> Result<(), Box<dyn Error>> {
        let staging = EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "staging")])
            .map_err(|error| io::Error::other(error.0))?;
        assert_eq!(staging.environment, Environment::Staging);
        let production = EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "production")])
            .map_err(|error| io::Error::other(error.0))?;
        assert_eq!(production.environment, Environment::Production);
        assert!(EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "qa")]).is_err());
        Ok(())
    }

    #[test]
    fn secret_value_namespace_is_outside_configuration_parsing() -> Result<(), Box<dyn Error>> {
        // The development provider owns `SITOLO_SECRET__`; a value there must
        // coexist with ordinary `SITOLO__` configuration instead of being
        // rejected as an unknown configuration key.
        let builder = EnvLoader::from_pairs([
            pair("RUNTIME__ENVIRONMENT", "development"),
            (
                "SITOLO_SECRET__DATABASE__PASSWORD".to_string(),
                "TEST_ONLY_SECRET_001".to_string(),
            ),
        ])
        .map_err(|error| io::Error::other(error.0))?;
        assert_eq!(builder.environment, Environment::Development);
        assert_eq!(builder.db_host, "localhost");
        Ok(())
    }

    #[test]
    fn former_password_key_is_unknown_configuration() -> Result<(), Box<dyn Error>> {
        let error = EnvLoader::from_pairs([pair("DATABASE__PASSWORD", "TEST_ONLY_SECRET_001")])
            .err()
            .ok_or_else(|| io::Error::other("the former password key must fail closed"))?;
        assert_eq!(error.0, "DATABASE__PASSWORD");
        Ok(())
    }

    #[test]
    fn secret_value_namespace_does_not_mask_unknown_configuration() -> Result<(), Box<dyn Error>> {
        let error = EnvLoader::from_pairs([
            (
                "SITOLO_SECRET__DATABASE__PASSWORD".to_string(),
                "TEST_ONLY_SECRET_001".to_string(),
            ),
            pair("DATABASE__PASSWORD", "TEST_ONLY_SECRET_001"),
        ])
        .err()
        .ok_or_else(|| io::Error::other("an unknown configuration key must still fail closed"))?;
        assert_eq!(error.0, "DATABASE__PASSWORD");
        Ok(())
    }

    #[test]
    fn production_baseline_requires_explicit_database_identity() -> Result<(), Box<dyn Error>> {
        // F-002: no implicit development database or secret namespace.
        let builder = EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "production")])
            .map_err(|error| io::Error::other(error.0))?;
        assert!(validate(builder).is_err());
        let staging = EnvLoader::from_pairs([pair("RUNTIME__ENVIRONMENT", "staging")])
            .map_err(|error| io::Error::other(error.0))?;
        assert!(validate(staging).is_err());
        Ok(())
    }

    #[test]
    fn production_requires_restricted_database_role_and_complete_oidc_config()
    -> Result<(), Box<dyn Error>> {
        let mut config = defaults::Builder::production();
        config.db_host = "db.internal".into();
        config.db_name = "sitolo".into();
        config.db_schema = "sitolo_app".into();
        config.db_user = "app_runtime".into();
        config.db_password_ref = Some(SecretRef::new(
            SecretClass::Database,
            "production/sitolo/db",
        )?);
        config.oidc_issuer = Some("https://cognito-idp.af-south-1.amazonaws.com/pool".into());
        config.oidc_client_id = Some("client-id".into());
        config.oidc_token_endpoint = Some("https://auth.example.com/oauth2/token".into());
        config.oidc_redirect_uri = Some("https://app.example.com/callback".into());
        assert!(validate(config.clone()).is_ok());

        config.db_user = "sitolo_admin".into();
        assert!(validate(config.clone()).is_err());
        config.db_user = "app_runtime".into();
        config.oidc_issuer = None;
        assert!(validate(config).is_err());
        Ok(())
    }

    #[test]
    fn production_rejects_development_secret_namespace() -> Result<(), Box<dyn Error>> {
        // F-003: a fully explicit production config with a development
        // secret namespace is rejected; a production namespace is accepted.
        let mut dev_ns = defaults::Builder::production();
        dev_ns.db_host = "db.internal".into();
        dev_ns.db_name = "sitolo".into();
        dev_ns.db_schema = "sitolo_app".into();
        dev_ns.db_user = "app_runtime".into();
        dev_ns.db_password_ref = Some(SecretRef::new(
            SecretClass::Database,
            "development/sitolo/db",
        )?);
        assert!(validate(dev_ns).is_err());

        let mut prod_ns = defaults::Builder::production();
        prod_ns.db_host = "db.internal".into();
        prod_ns.db_name = "sitolo".into();
        prod_ns.db_schema = "sitolo_app".into();
        prod_ns.db_user = "app_runtime".into();
        prod_ns.db_password_ref = Some(SecretRef::new(
            SecretClass::Database,
            "production/sitolo/db",
        )?);
        prod_ns.oidc_issuer = Some("https://cognito-idp.af-south-1.amazonaws.com/pool".into());
        prod_ns.oidc_client_id = Some("client-id".into());
        prod_ns.oidc_token_endpoint = Some("https://auth.example.com/oauth2/token".into());
        prod_ns.oidc_redirect_uri = Some("https://app.example.com/callback".into());
        assert!(validate(prod_ns).is_ok());
        Ok(())
    }

    #[test]
    fn telemetry_endpoint_transport_is_explicit() -> Result<(), Box<dyn Error>> {
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
                builder.db_password_ref = Some(SecretRef::new(
                    SecretClass::Database,
                    "production/sitolo/db",
                )?);
                builder.db_host = "db.internal".into();
                builder.db_name = "sitolo".into();
                builder.db_schema = "sitolo_app".into();
                builder.db_user = "app_runtime".into();
                builder.oidc_issuer =
                    Some("https://cognito-idp.af-south-1.amazonaws.com/pool".into());
                builder.oidc_client_id = Some("client-id".into());
                builder.oidc_token_endpoint = Some("https://auth.example.com/oauth2/token".into());
                builder.oidc_redirect_uri = Some("https://app.example.com/callback".into());
            }
            builder.otel_endpoint = Some(endpoint.into());
            assert_eq!(
                validate(builder).is_ok(),
                valid,
                "endpoint={endpoint} production={production}"
            );
        }
        Ok(())
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

    #[test]
    fn database_schema_is_a_bounded_identifier_and_is_required_outside_development()
    -> Result<(), Box<dyn Error>> {
        let mut staging = defaults::Builder::staging();
        staging.db_host = "db.internal".into();
        staging.db_name = "sitolo".into();
        staging.db_user = "app_runtime".into();
        staging.db_schema = "sitolo_app".into();
        staging.db_password_ref = Some(SecretRef::new(SecretClass::Database, "staging/sitolo/db")?);
        staging.oidc_issuer = Some("https://cognito-idp.af-south-1.amazonaws.com/pool".into());
        staging.oidc_client_id = Some("client-id".into());
        staging.oidc_token_endpoint = Some("https://auth.example.com/oauth2/token".into());
        staging.oidc_redirect_uri = Some("https://app.example.com/callback".into());
        assert!(validate(staging.clone()).is_ok());

        for schema in ["", "Upper", "a-b", "0prefix", &"a".repeat(64)] {
            let mut invalid = staging.clone();
            invalid.db_schema = schema.to_owned();
            assert!(validate(invalid).is_err(), "schema={schema:?}");
        }

        let runtime = validate(staging)?.database_runtime();
        assert_eq!(runtime.schema, "sitolo_app");
        Ok(())
    }

    #[test]
    fn relay_settings_are_typed_and_invalid_values_fail_closed() -> Result<(), Box<dyn Error>> {
        let builder = EnvLoader::from_pairs([
            (
                "SITOLO__EVENTS__RELAY__AWS_REGION".into(),
                "af-south-1".into(),
            ),
            (
                "SITOLO__EVENTS__RELAY__SNS_TOPIC_ARN".into(),
                "arn:aws:sns:af-south-1:123456789012:sitolo-iam-events-prod.fifo".into(),
            ),
            ("SITOLO__EVENTS__RELAY__BATCH_SIZE".into(), "10".into()),
            ("SITOLO__EVENTS__RELAY__MAX_ATTEMPTS".into(), "10".into()),
            (
                "SITOLO__EVENTS__RELAY__BACKOFF_MAX_MS".into(),
                "3600000".into(),
            ),
        ])
        .map_err(|error| io::Error::other(error.0))?;
        let config = validate(builder)?;
        assert_eq!(config.relay_aws_region.as_deref(), Some("af-south-1"));
        assert_eq!(
            config.relay_sns_topic_arn.as_deref(),
            Some("arn:aws:sns:af-south-1:123456789012:sitolo-iam-events-prod.fifo")
        );
        assert_eq!(config.relay_batch_size, 10);
        assert_eq!(config.relay_max_attempts, 10);
        assert_eq!(config.relay_backoff_max_ms, 3_600_000);

        let invalid =
            EnvLoader::from_pairs([("SITOLO__EVENTS__RELAY__BATCH_SIZE".into(), "0".into())])
                .map_err(|error| io::Error::other(error.0))?;
        assert!(validate(invalid).is_err());
        let incomplete = EnvLoader::from_pairs([(
            "SITOLO__EVENTS__RELAY__AWS_REGION".into(),
            "af-south-1".into(),
        )])
        .map_err(|error| io::Error::other(error.0))?;
        assert!(validate(incomplete).is_err());

        let mismatched = EnvLoader::from_pairs([
            (
                "SITOLO__EVENTS__RELAY__AWS_REGION".into(),
                "af-south-1".into(),
            ),
            (
                "SITOLO__EVENTS__RELAY__SNS_TOPIC_ARN".into(),
                "arn:aws:sns:eu-west-1:123456789012:sitolo-iam-events-prod.fifo".into(),
            ),
        ])
        .map_err(|error| io::Error::other(error.0))?;
        assert!(validate(mismatched).is_err());
        Ok(())
    }

    #[test]
    fn managed_secret_region_is_typed_bounded_and_fingerprinted() -> Result<(), Box<dyn Error>> {
        let builder =
            EnvLoader::from_pairs([("SITOLO__SECRETS__AWS_REGION".into(), "af-south-1".into())])
                .map_err(|error| io::Error::other(error.0))?;
        let config = validate(builder)?;
        assert_eq!(config.secrets_aws_region.as_deref(), Some("af-south-1"));

        let invalid = EnvLoader::from_pairs([(
            "SITOLO__SECRETS__AWS_REGION".into(),
            "https://aws.example".into(),
        )])
        .map_err(|error| io::Error::other(error.0))?;
        assert!(validate(invalid).is_err());
        Ok(())
    }
}
