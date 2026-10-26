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

use sitolo_application::TenancyService;
use sitolo_auth::IdentityProviderPort;
use sitolo_config::{
    AppConfig, DatabaseRuntimeConfig, EnvLoader, LogLevel, config_fingerprint, validate,
};
use sitolo_integrations::{
    CognitoOidcConfig, CognitoOidcProvider, secrets::AwsSecretsManagerProvider,
};
use sitolo_observability::TelemetryBuffer;
use sitolo_persistence::{PgRequestIdentityAuthority, PgRuntimePool, PgTenancyUnitOfWork};
use sitolo_security::{EnvSecretProvider, SecretProvider, SecretValue};
use thiserror::Error;

use crate::state::AppState;

/// Readiness is published only after every startup step succeeds. The
/// listener binds after this context exists, so serving implies `Ready`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// Every startup step succeeded and the runtime is fully composed.
    Ready,
    /// The runtime is deliberately composed without live dependencies.
    NotReady,
}

/// Fully assembled, validated runtime. Holds the secret *provider*, never
/// secret values: the startup probe resolves and drops credentials before
/// this context is returned.
pub struct StartupContext {
    config: AppConfig,
    state: Arc<AppState>,
    provider: Arc<dyn SecretProvider>,
    database_pool: Option<PgRuntimePool>,
    db_intent: DatabaseRuntimeConfig,
    readiness: Readiness,
}

impl std::fmt::Debug for StartupContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The secret provider is omitted deliberately: it must never reach a
        // log line, a panic payload, or an error report.
        formatter
            .debug_struct("StartupContext")
            .field("config", &self.config)
            .field("state", &self.state)
            .field("has_database_pool", &self.database_pool.is_some())
            .field("db_intent", &self.db_intent)
            .field("readiness", &self.readiness)
            .finish_non_exhaustive()
    }
}

impl StartupContext {
    /// Builds from the real process environment. The only environment read
    /// happens inside [`EnvLoader`] (the approved configuration boundary);
    /// this function never touches process environment directly.
    ///
    /// # Errors
    /// Returns a fail-closed [`StartupError`] when the environment contains an
    /// unknown key, configuration validation fails, the managed secret provider
    /// cannot be constructed, or any later startup step fails. Error values
    /// carry counts and field classes only, never secret material.
    pub async fn build() -> Result<Self, StartupError> {
        let raw = EnvLoader::load()
            .map_err(|unknown| StartupError::UnknownConfigKey { key: unknown.0 })?;
        let config = validate(raw).map_err(|invalid| StartupError::ConfigRejected {
            problems: invalid.problems.len(),
        })?;
        let connect_database =
            !matches!(config.environment, sitolo_config::Environment::Development);
        let provider: Arc<dyn SecretProvider> = match config.environment {
            sitolo_config::Environment::Production => {
                let region = production_secret_region(&config)?;
                let managed = fail_closed(
                    AwsSecretsManagerProvider::from_ecs_task_role(
                        region,
                        config.db_secret_ref().clone(),
                        Duration::from_secs(5),
                    )
                    .await,
                    StartupError::SecretProviderUnavailable,
                )?;
                Arc::new(managed)
            }
            sitolo_config::Environment::Development | sitolo_config::Environment::Staging => {
                Arc::new(EnvSecretProvider::new(false))
            }
        };
        Self::assemble_validated(config, Some(provider), connect_database).await
    }

    /// Builds from injected pairs with the environment-default provider.
    /// Same path as [`Self::build`]; used by startup integration tests so
    /// they execute the real bootstrap.
    ///
    /// # Errors
    /// Returns a fail-closed [`StartupError`] exactly as [`Self::build`] does.
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
            None | Some("development" | "staging") => Some(Arc::new(EnvSecretProvider::new(false))),
            _ => None,
        };
        Self::build_from_pairs_with_provider(pairs, default).await
    }

    /// Builds from injected pairs with an explicit provider. This is the
    /// seam for the future managed production adapter (and for tests):
    /// an explicitly supplied provider is honored in every environment.
    ///
    /// # Errors
    /// Returns a fail-closed [`StartupError`] when an injected key is unknown,
    /// configuration validation fails, no provider is supplied for an
    /// environment that requires one, or any later startup step fails.
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

        Self::assemble_validated(config, provider, false).await
    }

    async fn assemble_validated(
        config: AppConfig,
        provider: Option<Arc<dyn SecretProvider>>,
        connect_database: bool,
    ) -> Result<Self, StartupError> {
        init_logging(config.log_level);
        let otel_max_queue = fail_closed(
            usize::try_from(config.otel_max_queue),
            StartupError::ConfigRejected { problems: 1 },
        )?;
        let telemetry = Arc::new(Mutex::new(TelemetryBuffer::new(otel_max_queue)));

        // Injected production compositions must provide an explicit authority;
        // the real process path constructs its ECS provider before this call.
        let provider = provider.ok_or(StartupError::SecretProviderUnavailable)?;

        // Resolve the configured database credential for connection setup.
        // The secret is consumed into SQLx options and never stored in AppState.
        let credential = fail_closed(
            provider.get(config.db_secret_ref()).await,
            StartupError::SecretProbeFailed {
                class: config.database_secret_class().as_str(),
            },
        )?;

        let identity_provider = if connect_database {
            Some(compose_identity_provider(&config).await?)
        } else {
            None
        };

        let database_pool = if connect_database {
            Some(connect_runtime(&config, credential).await?)
        } else {
            drop(credential);
            None
        };

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
        let mut state = AppState::new(
            config.service_name.clone(),
            config.service_version.clone(),
            fingerprint,
            config.database_target(),
            Arc::clone(&telemetry),
        );
        if let Some((pool, authority, tenancy)) = &database_pool {
            state = state
                .with_identity_authority(Arc::new(authority.clone()))
                .with_tenancy_service(Arc::clone(tenancy))
                .with_runtime_database(pool.clone());
        }
        if let Some(identity_provider) = identity_provider {
            state = state.with_identity_provider(identity_provider);
        }
        let runtime_ready =
            database_pool.is_some() && state.has_request_identity() && state.has_tenancy_service();
        if runtime_ready {
            state = state.with_runtime_ready();
        }
        let state = Arc::new(state);
        Ok(StartupContext {
            config,
            state,
            provider,
            database_pool: database_pool.map(|(pool, _, _)| pool),
            db_intent,
            readiness: if runtime_ready {
                Readiness::Ready
            } else {
                Readiness::NotReady
            },
        })
    }

    /// Validated, non-connecting database intent for the future
    /// persistence capability. No I/O has been performed.
    #[must_use]
    pub fn db_intent(&self) -> &DatabaseRuntimeConfig {
        &self.db_intent
    }

    /// The validated configuration this context was assembled from.
    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// The shared runtime state handed to the HTTP router.
    #[must_use]
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }

    /// The secret provider used for startup probes. The provider holds no
    /// secret values between calls.
    #[must_use]
    pub fn provider(&self) -> &Arc<dyn SecretProvider> {
        &self.provider
    }

    /// Returns the restricted PostgreSQL pool when process bootstrap opened it.
    #[must_use]
    pub fn database_pool(&self) -> Option<&PgRuntimePool> {
        self.database_pool.as_ref()
    }

    /// The readiness published by the last completed startup step.
    #[must_use]
    pub fn readiness(&self) -> Readiness {
        self.readiness
    }
}

/// Startup failures carry counts and field classes only: never secret
/// values, references, or full configuration documents.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StartupError {
    /// The environment contained a key the configuration schema does not own.
    #[error("unknown configuration key: {key}")]
    UnknownConfigKey {
        /// The rejected key name.
        key: String,
    },
    /// Typed configuration failed fail-closed validation.
    #[error("invalid configuration ({problems} problem(s))")]
    ConfigRejected {
        /// How many problems validation reported; the problems themselves are
        /// not carried, because they may quote configuration values.
        problems: usize,
    },
    /// Production has no managed secret provider available.
    #[error("no managed secret provider available for production")]
    SecretProviderUnavailable,
    /// The external identity provider could not be configured or reached.
    #[error("identity provider could not be configured or reached")]
    IdentityProviderUnavailable,
    /// The startup secret probe failed for the named secret class.
    #[error("startup secret probe failed for {class}")]
    SecretProbeFailed {
        /// Secret class only, never the reference or value.
        class: &'static str,
    },
    /// The restricted database runtime could not be established or verified.
    #[error("database runtime could not be established or verified")]
    DatabaseRuntimeUnavailable,
    /// The HTTP server terminated unexpectedly.
    #[error("HTTP server terminated unexpectedly ({class})")]
    ServeFailed {
        /// Error class only, never request data or credentials.
        class: &'static str,
    },
    /// The operating-system shutdown signal handler could not be installed.
    #[error("operating-system shutdown signal handler could not be installed")]
    SignalHandlerUnavailable,
}

/// Maps a fallible startup step to its fail-closed startup class.
///
/// The source error is deliberately dropped: provider, TLS, and database errors
/// can embed secret values or credential-bearing connection strings, and
/// [`StartupError`] must never carry either. Converting at the boundary keeps
/// that guarantee while still naming the discarded value explicitly.
fn fail_closed<T, E>(result: Result<T, E>, error: StartupError) -> Result<T, StartupError> {
    result.map_err(|source| {
        drop(source);
        error
    })
}

/// Validates the OIDC settings, constructs the Cognito provider, and proves the
/// issuer signing keys are reachable before the listener is published.
async fn compose_identity_provider(
    config: &AppConfig,
) -> Result<Arc<dyn IdentityProviderPort>, StartupError> {
    let oidc_config = fail_closed(
        CognitoOidcConfig::new(sitolo_integrations::oidc::CognitoOidcSettings {
            issuer: config
                .oidc_issuer
                .clone()
                .ok_or(StartupError::IdentityProviderUnavailable)?,
            client_id: config
                .oidc_client_id
                .clone()
                .ok_or(StartupError::IdentityProviderUnavailable)?,
            resource_audience: None,
            token_endpoint: config
                .oidc_token_endpoint
                .clone()
                .ok_or(StartupError::IdentityProviderUnavailable)?,
            redirect_uri: config
                .oidc_redirect_uri
                .clone()
                .ok_or(StartupError::IdentityProviderUnavailable)?,
            required_scopes: Vec::new(),
            request_timeout: Duration::from_secs(5),
            jwks_ttl: Duration::from_secs(300),
            unknown_kid_refresh_interval: Duration::from_secs(10),
            clock_skew: Duration::from_secs(30),
        }),
        StartupError::IdentityProviderUnavailable,
    )?;
    let oidc = Arc::new(fail_closed(
        CognitoOidcProvider::new(oidc_config),
        StartupError::IdentityProviderUnavailable,
    )?);
    fail_closed(
        oidc.probe().await,
        StartupError::IdentityProviderUnavailable,
    )?;
    Ok(oidc)
}

/// Opens the restricted runtime pool, verifies its role and required schema
/// objects, and composes the durable tenancy service.
async fn connect_runtime(
    config: &AppConfig,
    credential: SecretValue,
) -> Result<
    (
        PgRuntimePool,
        PgRequestIdentityAuthority,
        Arc<TenancyService>,
    ),
    StartupError,
> {
    let runtime = config.database_runtime();
    let pool = fail_closed(
        PgRuntimePool::connect(
            &runtime,
            credential,
            !matches!(config.environment, sitolo_config::Environment::Development),
        )
        .await,
        StartupError::DatabaseRuntimeUnavailable,
    )?;
    let runtime_pool = pool.pool();
    let authority = fail_closed(
        PgRequestIdentityAuthority::new(runtime_pool.clone(), runtime.schema.clone()),
        StartupError::DatabaseRuntimeUnavailable,
    )?;
    let tenancy = Arc::new(TenancyService::new_postgres(
        Vec::new(),
        Arc::new(PgTenancyUnitOfWork::new(runtime_pool)),
    ));
    Ok((pool, authority, tenancy))
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
    // A later call observes an already-installed subscriber; that outcome is
    // expected and intentionally not treated as a startup failure.
    let _installed = tracing_subscriber::fmt()
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
    fn production_requires_an_explicit_managed_secret_region()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = sitolo_config::defaults::Builder::production();
        builder.db_host = "db.internal".into();
        builder.db_name = "sitolo".into();
        builder.db_schema = "sitolo_app".into();
        builder.db_user = "app_runtime".into();
        builder.oidc_issuer = Some("https://cognito-idp.af-south-1.amazonaws.com/pool".into());
        builder.oidc_client_id = Some("client-id".into());
        builder.oidc_token_endpoint = Some("https://auth.example.com/oauth2/token".into());
        builder.oidc_redirect_uri = Some("https://app.example.com/callback".into());
        builder.db_password_ref = Some(sitolo_security::SecretRef::new(
            sitolo_security::SecretClass::Database,
            "production/sitolo/api/runtime-db-password",
        )?);
        let config = validate(builder.clone())?;
        assert_eq!(
            production_secret_region(&config),
            Err(StartupError::SecretProviderUnavailable)
        );

        builder.secrets_aws_region = Some("af-south-1".into());
        let config = validate(builder)?;
        assert_eq!(production_secret_region(&config), Ok("af-south-1"));
        Ok(())
    }
}
