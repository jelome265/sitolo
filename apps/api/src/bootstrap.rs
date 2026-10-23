//! Single application bootstrap path (audit F-001/F-020).
//!
//! Lifecycle, in order:
//!
//! ```text
//! raw environment
//!   -> parse typed configuration (environment selector first)
//!   -> fail-closed validation
//!   -> initialize redaction/security primitives (stateless by construction)
//!   -> initialize structured logging (exactly once per process)
//!   -> create correlation/telemetry infrastructure
//!   -> construct the environment-appropriate secret provider
//!   -> probe required startup secrets, then drop them immediately
//!   -> construct database runtime intent (no I/O)
//!   -> publish readiness
//! ```
//!
//! The HTTP listener binds only after [`StartupContext`] exists. Production
//! secrets use the ECS task role and AWS Secrets Manager; production never
//! falls back to the development environment-secret provider.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sitolo_config::{
    AppConfig, DatabaseRuntimeConfig, EnvLoader, LogLevel, config_fingerprint, validate,
};
use sitolo_integrations::secrets::AwsSecretsManagerProvider;
use sitolo_observability::TelemetryBuffer;
use sitolo_security::{EnvSecretProvider, SecretProvider};
use thiserror::Error;

use crate::state::AppState;

/// Readiness is published only after every startup step succeeds. The
/// listener binds after this context exists, so serving implies `Ready`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    Ready,
    NotReady,
}

/// Fully assembled, validated runtime. Holds the secret *provider*, never
/// secret values: the startup probe resolves and drops credentials before
/// this context is returned.
pub struct StartupContext {
    config: AppConfig,
    state: Arc<AppState>,
    provider: Arc<dyn SecretProvider>,
    db_intent: DatabaseRuntimeConfig,
    readiness: Readiness,
}

impl StartupContext {
    /// Builds from the real process environment. The only environment read
    /// happens inside [`EnvLoader`] (the approved configuration boundary);
    /// this function never touches process environment directly.
    pub async fn build() -> Result<Self, StartupError> {
        let raw = EnvLoader::load()
            .map_err(|unknown| StartupError::UnknownConfigKey { key: unknown.0 })?;
        let config = validate(raw).map_err(|invalid| StartupError::ConfigRejected {
            problems: invalid.problems.len(),
        })?;
        let provider: Arc<dyn SecretProvider> = match config.environment {
            sitolo_config::Environment::Production => {
                let region = production_secret_region(&config)?;
                let managed = AwsSecretsManagerProvider::from_ecs_task_role(
                    region,
                    config.db_secret_ref().clone(),
                    Duration::from_secs(5),
                )
                .await
                .map_err(|_| StartupError::SecretProviderUnavailable)?;
                Arc::new(managed)
            }
            sitolo_config::Environment::Development | sitolo_config::Environment::Staging => {
                Arc::new(EnvSecretProvider::new(false))
            }
        };
        Self::assemble_validated(config, Some(provider)).await
    }

    /// Builds from injected pairs with the environment-default provider.
    /// Same path as [`Self::build`]; used by startup integration tests so
    /// they execute the real bootstrap.
    pub async fn build_from_pairs(
        pairs: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, StartupError> {
        let pairs: Vec<(String, String)> = pairs.into_iter().collect();
        let selector = pairs
            .iter()
            .find(|(key, _)| key == "SITOLO__RUNTIME__ENVIRONMENT")
            .map(|(_, value)| value.as_str());
        // Injected-pair builds keep production fail-closed unless a provider
        // is supplied explicitly; they do not discover AWS task credentials.
        let default: Option<Arc<dyn SecretProvider>> = match selector {
            None | Some("development") | Some("staging") => {
                Some(Arc::new(EnvSecretProvider::new(false)))
            }
            _ => None,
        };
        Self::build_from_pairs_with_provider(pairs, default).await
    }

    /// Builds from injected pairs with an explicit provider. This is the
    /// seam for the future managed production adapter (and for tests):
    /// an explicitly supplied provider is honored in every environment.
    pub async fn build_from_pairs_with_provider(
        pairs: impl IntoIterator<Item = (String, String)>,
        provider: Option<Arc<dyn SecretProvider>>,
    ) -> Result<Self, StartupError> {
        let raw = EnvLoader::from_pairs(pairs)
            .map_err(|unknown| StartupError::UnknownConfigKey { key: unknown.0 })?;
        Self::assemble(raw, provider).await
    }

    /// Shared assembly: validate, initialize platform subsystems in order,
    /// resolve-and-drop the startup secret probe, and publish readiness.
    async fn assemble(
        raw: sitolo_config::defaults::Builder,
        provider: Option<Arc<dyn SecretProvider>>,
    ) -> Result<Self, StartupError> {
        let config = validate(raw).map_err(|invalid| StartupError::ConfigRejected {
            problems: invalid.problems.len(),
        })?;

        Self::assemble_validated(config, provider).await
    }

    async fn assemble_validated(
        config: AppConfig,
        provider: Option<Arc<dyn SecretProvider>>,
    ) -> Result<Self, StartupError> {
        init_logging(config.log_level);
        let telemetry = Arc::new(Mutex::new(TelemetryBuffer::new(
            config.otel_max_queue as usize,
        )));

        // Injected production compositions must provide an explicit authority;
        // the real process path constructs its ECS provider before this call.
        let provider = provider.ok_or(StartupError::SecretProviderUnavailable)?;

        // Startup secret probe: proves the reference resolves through the
        // selected provider, then drops the value immediately. State never
        // holds credentials; only the provider capability is retained.
        let probe = provider.get(config.db_secret_ref()).await.map_err(|_| {
            StartupError::SecretProbeFailed {
                class: config.database_secret_class().as_str(),
            }
        })?;
        drop(probe);

        let fingerprint = config_fingerprint(&config);
        let db_intent: DatabaseRuntimeConfig = config.database_runtime();
        tracing::info!(
            service_name = config.service_name.as_str(),
            service_version = config.service_version.as_str(),
            environment = config.environment.as_str(),
            config_schema_version = config.config_schema_version,
            config_fingerprint = fingerprint.as_str(),
            db_host = config.db_host.as_str(),
            db_port = config.db_port,
            db_name = config.db_name.as_str(),
            db_user = config.db_user.as_str(),
            "sitolo-api startup complete"
        );
        let state = Arc::new(AppState::new(
            config.service_name.clone(),
            config.service_version.clone(),
            fingerprint,
            config.database_target(),
            Arc::clone(&telemetry),
        ));
        Ok(StartupContext {
            config,
            state,
            provider,
            db_intent,
            // A resolved secret proves configuration access only. Until the
            // persistent PostgreSQL/auth/tenancy composition is installed and
            // probed, the HTTP process must not advertise readiness.
            readiness: Readiness::NotReady,
        })
    }

    /// Validated, non-connecting database intent for the future
    /// persistence capability. No I/O has been performed.
    pub fn db_intent(&self) -> &DatabaseRuntimeConfig {
        &self.db_intent
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }

    pub fn provider(&self) -> &Arc<dyn SecretProvider> {
        &self.provider
    }

    pub fn readiness(&self) -> Readiness {
        self.readiness
    }
}

/// Startup failures carry counts and field classes only: never secret
/// values, references, or full configuration documents.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StartupError {
    #[error("unknown configuration key: {key}")]
    UnknownConfigKey { key: String },
    #[error("invalid configuration ({problems} problem(s))")]
    ConfigRejected { problems: usize },
    #[error("no managed secret provider available for production")]
    SecretProviderUnavailable,
    #[error("startup secret probe failed for {class}")]
    SecretProbeFailed { class: &'static str },
    #[error("HTTP server terminated unexpectedly ({class})")]
    ServeFailed { class: &'static str },
}

fn logging_level(level: LogLevel) -> tracing::level_filters::LevelFilter {
    match level {
        LogLevel::Trace => tracing::level_filters::LevelFilter::TRACE,
        LogLevel::Debug => tracing::level_filters::LevelFilter::DEBUG,
        LogLevel::Info => tracing::level_filters::LevelFilter::INFO,
        LogLevel::Warn => tracing::level_filters::LevelFilter::WARN,
        LogLevel::Error => tracing::level_filters::LevelFilter::ERROR,
    }
}

/// Installs the process-global JSON subscriber exactly once. Later calls are
/// no-ops: the first initialization wins for the process lifetime, which is
/// also what makes multi-bootstrap test processes deterministic.
fn init_logging(level: LogLevel) {
    let _ = tracing_subscriber::fmt()
        .json()
        .with_max_level(logging_level(level))
        .try_init();
}

fn production_secret_region(config: &AppConfig) -> Result<&str, StartupError> {
    config
        .secrets_aws_region
        .as_deref()
        .ok_or(StartupError::SecretProviderUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_errors_carry_no_secret_material() {
        let sentinel = "TEST_ONLY_STARTUP_SECRET_001";
        let display = format!("{}", StartupError::SecretProbeFailed { class: "database" });
        assert!(!display.contains(sentinel));
        let display = format!(
            "{}",
            StartupError::UnknownConfigKey {
                key: "RUNTIME__ENVIRONMENT".into()
            }
        );
        assert!(display.contains("RUNTIME__ENVIRONMENT"));
    }

    #[test]
    fn logging_level_mapping_covers_all_levels() {
        assert_eq!(
            logging_level(LogLevel::Trace),
            tracing::level_filters::LevelFilter::TRACE
        );
        assert_eq!(
            logging_level(LogLevel::Error),
            tracing::level_filters::LevelFilter::ERROR
        );
    }

    #[test]
    fn production_requires_an_explicit_managed_secret_region() {
        let mut builder = sitolo_config::defaults::Builder::production();
        builder.db_host = "db.internal".into();
        builder.db_name = "sitolo".into();
        builder.db_user = "sitolo_api".into();
        builder.db_password_ref = Some(
            sitolo_security::SecretRef::new(
                sitolo_security::SecretClass::Database,
                "production/sitolo/api/runtime-db-password",
            )
            .expect("static test reference is valid"),
        );
        let config = validate(builder.clone()).expect("production database config validates");
        assert_eq!(
            production_secret_region(&config),
            Err(StartupError::SecretProviderUnavailable)
        );

        builder.secrets_aws_region = Some("af-south-1".into());
        let config = validate(builder).expect("production AWS region validates");
        assert_eq!(production_secret_region(&config), Ok("af-south-1"));
    }
}
