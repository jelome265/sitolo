//! Typed canonical configuration model (Phase 2 Appendix A).

use std::fmt;
use std::net::SocketAddr;
use std::time::Duration;

use sitolo_security::{SecretClass, SecretRef};

/// Hard ceilings that no override may bypass (Phase 2 §6, §38).
///
/// These are architecture policy, not tuning suggestions. An operator value
/// above a ceiling is an invalid configuration, never new policy.
pub mod ceilings {
    /// 25 MiB maximum request-body size ceiling (Phase 2 §6 example).
    pub const MAX_REQUEST_BODY_CEILING_BYTES: u64 = 26_214_400;
    /// Hard ceiling for the request-header read timeout, in milliseconds.
    pub const REQUEST_HEADER_TIMEOUT_CEILING_MS: u64 = 30_000;
    /// Hard ceiling for the HTTP keep-alive idle timeout, in milliseconds.
    pub const KEEPALIVE_TIMEOUT_CEILING_MS: u64 = 300_000;
    /// Hard ceiling for the database connection-acquire timeout, in
    /// milliseconds.
    pub const DB_ACQUIRE_TIMEOUT_CEILING_MS: u64 = 60_000;
    /// Hard ceiling for the OTLP export timeout, in milliseconds.
    pub const OTEL_EXPORT_TIMEOUT_CEILING_MS: u64 = 60_000;
    /// Hard ceiling for the OTLP batch queue depth, in spans.
    pub const OTEL_MAX_QUEUE_CEILING: u32 = 1_048_576;
    /// Hard ceiling for the configured database pool minimum.
    pub const DB_POOL_MIN_CEILING: u32 = 256;
    /// Hard ceiling for the configured database pool maximum.
    pub const DB_POOL_MAX_CEILING: u32 = 512;
    /// Bounded string lengths protect against pathological configuration sizes
    /// (Phase 2 §47: bounded validation failure, never a parser blowup).
    pub const MAX_BOUNDED_STRING_BYTES: usize = 512;
    /// Schema version handled by this build.
    pub const CONFIG_SCHEMA_VERSION: u32 = 3;
}

/// Deployment environment for a process.
///
/// Unknown values fail startup (Appendix A: `environment`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Environment {
    /// Local development posture: the local secret provider and the
    /// development secret namespace are permitted.
    Development,
    /// Pre-production posture: production security rules apply, but database
    /// identity and OIDC settings must still be supplied explicitly.
    Staging,
    /// Production posture: strictest secret-provider, transport, and logging
    /// rules.
    Production,
}

impl Environment {
    /// Stable lowercase identifier used in logs, fingerprints, and errors.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Staging => "staging",
            Environment::Production => "production",
        }
    }

    /// Whether this is the production environment.
    #[must_use]
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured-log severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Most verbose level; never permitted in production.
    Trace,
    /// Diagnostic level; never permitted in production.
    Debug,
    /// Standard operational level.
    Info,
    /// Recoverable-degradation level.
    Warn,
    /// Failure level.
    Error,
}

impl LogLevel {
    /// Stable lowercase identifier used in logs, fingerprints, and errors.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }

    /// Most verbose level production is permitted to run at. Values more
    /// verbose than this (trace/debug) are rejected in production
    /// (Appendix A: `log_level` production ceiling).
    pub const PRODUCTION_CEILING: LogLevel = LogLevel::Info;
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The validated, typed, effective configuration.
///
/// Constructed only through [`super::validate`]; never constructed directly.
/// Contains secret *references*, never secret values.
#[derive(Debug, Clone, PartialEq)]
pub struct AppConfig {
    /// Deployment environment selected before any defaults were layered.
    pub environment: Environment,
    /// Stable service name used in logs and telemetry.
    pub service_name: String,
    /// Provenance field; immutable at runtime (Appendix A).
    pub service_version: String,
    /// Socket address the HTTP listener binds to.
    pub bind_address: SocketAddr,
    /// Maximum accepted request-body size, in bytes.
    pub max_request_body_bytes: u64,
    /// Request-header read timeout, in milliseconds.
    pub request_header_timeout_ms: u64,
    /// HTTP keep-alive idle timeout, in milliseconds.
    pub keepalive_timeout_ms: u64,
    /// Database host name.
    pub db_host: String,
    /// Database TCP port; validation rejects zero.
    pub db_port: u16,
    /// Database (catalogue) name.
    pub db_name: String,
    /// PostgreSQL schema pinned as the runtime connection search path.
    pub db_schema: String,
    /// Database role the API pool authenticates as.
    pub db_user: String,
    /// Secret reference; a raw database password is prohibited (Appendix A).
    pub db_password_ref: SecretRef,
    /// Separate secret reference for the restricted outbox worker role.
    pub worker_db_password_ref: Option<SecretRef>,
    /// Trusted Cognito OIDC issuer used by the process runtime.
    pub oidc_issuer: Option<String>,
    /// Cognito public app-client identifier.
    pub oidc_client_id: Option<String>,
    /// OAuth token endpoint for authorization-code exchange.
    pub oidc_token_endpoint: Option<String>,
    /// Registered callback URI used for authorization-code exchange.
    pub oidc_redirect_uri: Option<String>,
    /// Minimum number of pooled database connections.
    pub db_pool_min: u32,
    /// Maximum number of pooled database connections.
    pub db_pool_max: u32,
    /// Database connection-acquire timeout, in milliseconds.
    pub db_acquire_timeout_ms: u64,
    /// `None` disables OTLP export. TLS is required for remote endpoints.
    pub otel_endpoint: Option<String>,
    /// OTLP export timeout, in milliseconds.
    pub otel_export_timeout_ms: u64,
    /// Maximum number of queued spans awaiting export.
    pub otel_max_queue: u32,
    /// Sampling ratio in `0.0..=1.0`.
    pub trace_sample_ratio: f64,
    /// Structured-log severity level.
    pub log_level: LogLevel,
    /// Whether the development secret provider may serve references.
    pub allow_local_secret_provider: bool,
    /// Schema version understood by this build.
    pub config_schema_version: u32,
    /// Optional SNS relay destination; credentials are always workload-provided.
    pub relay_aws_region: Option<String>,
    /// Optional SNS FIFO topic ARN paired with `relay_aws_region`.
    pub relay_sns_topic_arn: Option<String>,
    /// Maximum number of events per relay batch.
    pub relay_batch_size: u32,
    /// Maximum delivery attempts per relayed event.
    pub relay_max_attempts: u32,
    /// Maximum relay backoff delay, in milliseconds.
    pub relay_backoff_max_ms: u64,
    /// Explicit AWS region for managed production secret retrieval.
    pub secrets_aws_region: Option<String>,
}

/// Validated, non-connecting PostgreSQL runtime intent.
///
/// This is deliberately component-based: it carries a secret *reference*, not
/// a credential-bearing DSN. `sitolo-persistence` consumes this narrow view
/// and resolves the secret only while constructing a future database capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseRuntimeConfig {
    /// Database host name.
    pub host: String,
    /// Database TCP port.
    pub port: u16,
    /// Database (catalogue) name.
    pub database: String,
    /// PostgreSQL schema pinned as the connection search path.
    pub schema: String,
    /// Database role this connection authenticates as.
    pub username: String,
    /// Secret reference resolved only while constructing the capability.
    pub password_ref: SecretRef,
    /// Minimum number of pooled connections for this capability.
    pub pool_min: u32,
    /// Maximum number of pooled connections for this capability.
    pub pool_max: u32,
    /// Connection-acquire timeout.
    pub acquire_timeout: Duration,
}

/// Safe database target identity for diagnostics. It intentionally omits the
/// password reference as well as the resolved credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseTarget {
    /// Database host name.
    pub host: String,
    /// Database TCP port.
    pub port: u16,
    /// Database (catalogue) name.
    pub database: String,
    /// Database role, which is safe to expose in diagnostics.
    pub username: String,
}

impl AppConfig {
    /// Matches a `db_password_ref` to the database secret class.
    #[must_use]
    pub fn db_secret_ref(&self) -> &SecretRef {
        &self.db_password_ref
    }

    /// The classified secret reference for the database.
    #[must_use]
    pub fn database_secret_class(&self) -> SecretClass {
        SecretClass::Database
    }

    /// Produces validated database connection intent without network I/O.
    #[must_use]
    pub fn database_runtime(&self) -> DatabaseRuntimeConfig {
        DatabaseRuntimeConfig {
            host: self.db_host.clone(),
            port: self.db_port,
            database: self.db_name.clone(),
            schema: self.db_schema.clone(),
            username: self.db_user.clone(),
            password_ref: self.db_password_ref.clone(),
            pool_min: self.db_pool_min,
            pool_max: self.db_pool_max,
            acquire_timeout: Duration::from_millis(self.db_acquire_timeout_ms),
        }
    }

    /// Produces connection intent for the restricted outbox worker login.
    /// The credential reference is distinct from the API runtime credential.
    #[must_use]
    pub fn worker_database_runtime(&self) -> Option<DatabaseRuntimeConfig> {
        self.worker_db_password_ref
            .as_ref()
            .map(|password_ref| DatabaseRuntimeConfig {
                host: self.db_host.clone(),
                port: self.db_port,
                database: self.db_name.clone(),
                schema: self.db_schema.clone(),
                username: "app_worker".to_string(),
                password_ref: password_ref.clone(),
                pool_min: 1,
                pool_max: self.db_pool_max.min(8),
                acquire_timeout: Duration::from_millis(self.db_acquire_timeout_ms),
            })
    }

    /// Returns only safe database identity fields for diagnostics/telemetry.
    #[must_use]
    pub fn database_target(&self) -> DatabaseTarget {
        DatabaseTarget {
            host: self.db_host.clone(),
            port: self.db_port,
            database: self.db_name.clone(),
            username: self.db_user.clone(),
        }
    }
}

/// Preset used by tests and local development. Still validated by
/// [`super::validate`]; this is a *default layer*, never an authority.
pub mod defaults {
    use std::fmt;
    use std::net::SocketAddr;

    use sitolo_security::{SecretClass, SecretRef};

    use super::{Environment, LogLevel, ceilings};

    /// Compiled safe defaults for the catalogue (Appendix A examples).
    #[must_use]
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// A mutable default layer; layers only ever firm up these values.
    ///
    /// The debug representation redacts both secret reference paths: although
    /// references are not secret values, the builder is logged during startup
    /// diagnostics where even provider paths are need-to-know.
    #[derive(Clone)]
    pub struct Builder {
        /// Deployment environment selected for this builder.
        pub environment: Environment,
        /// Stable service name.
        pub service_name: String,
        /// Service version provenance string.
        pub service_version: String,
        /// Socket address the HTTP listener binds to.
        pub bind_address: SocketAddr,
        /// Maximum accepted request-body size, in bytes.
        pub max_request_body_bytes: u64,
        /// Request-header read timeout, in milliseconds.
        pub request_header_timeout_ms: u64,
        /// HTTP keep-alive idle timeout, in milliseconds.
        pub keepalive_timeout_ms: u64,
        /// Database host name.
        pub db_host: String,
        /// Database TCP port.
        pub db_port: u16,
        /// Database (catalogue) name.
        pub db_name: String,
        /// PostgreSQL schema used as the runtime search path.
        pub db_schema: String,
        /// Database role the API pool authenticates as.
        pub db_user: String,
        /// Trusted Cognito OIDC issuer.
        pub oidc_issuer: Option<String>,
        /// Cognito public app-client identifier.
        pub oidc_client_id: Option<String>,
        /// OAuth token endpoint for authorization-code exchange.
        pub oidc_token_endpoint: Option<String>,
        /// Registered callback URI for authorization-code exchange.
        pub oidc_redirect_uri: Option<String>,
        /// Minimum number of pooled database connections.
        pub db_pool_min: u32,
        /// Maximum number of pooled database connections.
        pub db_pool_max: u32,
        /// Database connection-acquire timeout, in milliseconds.
        pub db_acquire_timeout_ms: u64,
        /// OTLP endpoint; `None` disables export.
        pub otel_endpoint: Option<String>,
        /// OTLP export timeout, in milliseconds.
        pub otel_export_timeout_ms: u64,
        /// Maximum number of queued spans awaiting export.
        pub otel_max_queue: u32,
        /// Sampling ratio in `0.0..=1.0`.
        pub trace_sample_ratio: f64,
        /// Structured-log severity level.
        pub log_level: LogLevel,
        /// Whether the development secret provider may serve references.
        pub allow_local_secret_provider: bool,
        /// Schema version understood by this build.
        pub config_schema_version: u32,
        /// Optional SNS relay region.
        pub relay_aws_region: Option<String>,
        /// Optional SNS FIFO topic ARN.
        pub relay_sns_topic_arn: Option<String>,
        /// Maximum number of events per relay batch.
        pub relay_batch_size: u32,
        /// Maximum delivery attempts per relayed event.
        pub relay_max_attempts: u32,
        /// Maximum relay backoff delay, in milliseconds.
        pub relay_backoff_max_ms: u64,
        /// Explicit AWS region for managed secret retrieval.
        pub secrets_aws_region: Option<String>,
        /// `None` until a secret reference is supplied from an upper layer.
        pub db_password_ref: Option<SecretRef>,
        /// `None` until the restricted worker reference is supplied.
        pub worker_db_password_ref: Option<SecretRef>,
    }

    impl fmt::Debug for Builder {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Builder")
                .field("environment", &self.environment)
                .field("service_name", &self.service_name)
                .field("service_version", &self.service_version)
                .field("bind_address", &self.bind_address)
                .field("max_request_body_bytes", &self.max_request_body_bytes)
                .field("request_header_timeout_ms", &self.request_header_timeout_ms)
                .field("keepalive_timeout_ms", &self.keepalive_timeout_ms)
                .field("db_host", &self.db_host)
                .field("db_port", &self.db_port)
                .field("db_name", &self.db_name)
                .field("db_schema", &self.db_schema)
                .field("db_user", &self.db_user)
                .field("oidc_issuer", &self.oidc_issuer)
                .field("oidc_client_id", &self.oidc_client_id)
                .field("oidc_token_endpoint", &self.oidc_token_endpoint)
                .field("oidc_redirect_uri", &self.oidc_redirect_uri)
                .field("db_pool_min", &self.db_pool_min)
                .field("db_pool_max", &self.db_pool_max)
                .field("db_acquire_timeout_ms", &self.db_acquire_timeout_ms)
                .field("otel_endpoint", &self.otel_endpoint)
                .field("otel_export_timeout_ms", &self.otel_export_timeout_ms)
                .field("otel_max_queue", &self.otel_max_queue)
                .field("trace_sample_ratio", &self.trace_sample_ratio)
                .field("log_level", &self.log_level)
                .field(
                    "allow_local_secret_provider",
                    &self.allow_local_secret_provider,
                )
                .field("config_schema_version", &self.config_schema_version)
                .field("relay_aws_region", &self.relay_aws_region)
                .field("relay_sns_topic_arn", &self.relay_sns_topic_arn)
                .field("relay_batch_size", &self.relay_batch_size)
                .field("relay_max_attempts", &self.relay_max_attempts)
                .field("relay_backoff_max_ms", &self.relay_backoff_max_ms)
                .field("secrets_aws_region", &self.secrets_aws_region)
                .field(
                    "db_password_ref",
                    &self.db_password_ref.as_ref().map(|_| "<secret-reference>"),
                )
                .field(
                    "worker_db_password_ref",
                    &self
                        .worker_db_password_ref
                        .as_ref()
                        .map(|_| "<secret-reference>"),
                )
                .finish()
        }
    }

    impl Default for Builder {
        fn default() -> Self {
            Builder {
                environment: Environment::Development,
                service_name: "sitolo".to_string(),
                service_version: "0.0.0+dev".to_string(),
                bind_address: SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), 8080),
                max_request_body_bytes: 2_097_152,
                request_header_timeout_ms: 5_000,
                keepalive_timeout_ms: 30_000,
                db_host: "localhost".to_string(),
                db_port: 5432,
                db_name: "sitolo".to_string(),
                db_schema: "public".to_string(),
                db_user: "sitolo_api".to_string(),
                oidc_issuer: None,
                oidc_client_id: None,
                oidc_token_endpoint: None,
                oidc_redirect_uri: None,
                db_pool_min: 5,
                db_pool_max: 20,
                db_acquire_timeout_ms: 2_000,
                otel_endpoint: None,
                otel_export_timeout_ms: 3_000,
                otel_max_queue: 2_048,
                trace_sample_ratio: 0.1,
                log_level: LogLevel::Info,
                allow_local_secret_provider: true,
                config_schema_version: ceilings::CONFIG_SCHEMA_VERSION,
                relay_aws_region: None,
                relay_sns_topic_arn: None,
                relay_batch_size: 10,
                relay_max_attempts: 10,
                relay_backoff_max_ms: 3_600_000,
                secrets_aws_region: None,
                db_password_ref: None,
                worker_db_password_ref: None,
            }
        }
    }

    impl Builder {
        /// A development-oriented preset with a local secret reference, so the
        /// catalogue has a complete, valid development profile.
        ///
        /// # Panics
        ///
        /// Panics only if the fixed, compile-time development reference
        /// `development/sitolo/db` stops satisfying [`SecretRef::new`]'s
        /// grammar. That reference is a literal in this crate, so the panic is
        /// unreachable unless the literal is edited into an invalid form; the
        /// explicit `expect` keeps the invariant visible instead of silently
        /// downgrading to a missing reference.
        #[must_use]
        pub fn development() -> Self {
            // This built-in path is a fixed, validated development reference.
            #[expect(
                clippy::expect_used,
                reason = "the fixed development reference uses only allowed ASCII path characters"
            )]
            let db_password_ref = Some(
                SecretRef::new(SecretClass::Database, "development/sitolo/db")
                    .expect("static development reference"),
            );
            Builder {
                db_password_ref,
                worker_db_password_ref: None,
                ..Builder::default()
            }
        }

        /// A staging preset: production posture (no local provider, no
        /// development secret namespace), but database identity must still be
        /// supplied explicitly. There are no implicit staging credentials.
        #[must_use]
        pub fn staging() -> Self {
            Builder {
                environment: Environment::Staging,
                log_level: LogLevel::Info,
                allow_local_secret_provider: false,
                otel_endpoint: None,
                db_host: String::new(),
                db_name: String::new(),
                db_schema: String::new(),
                db_user: String::new(),
                db_password_ref: None,
                ..Builder::default()
            }
        }

        /// A production-safe baseline: no local provider, no development
        /// secret namespace, and no implicit database identity. Database
        /// host/name/user/reference must be supplied explicitly; validation
        /// rejects the empty placeholders (F-002: production must never
        /// inherit development-only defaults).
        #[must_use]
        pub fn production() -> Self {
            Builder {
                environment: Environment::Production,
                log_level: LogLevel::Info,
                allow_local_secret_provider: false,
                otel_endpoint: Some("https://collector.sitolo.internal".to_string()),
                db_host: String::new(),
                db_name: String::new(),
                db_schema: String::new(),
                db_user: String::new(),
                db_password_ref: None,
                ..Builder::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_display_is_stable() {
        assert_eq!(Environment::Production.as_str(), "production");
        assert!(Environment::Production.is_production());
        assert!(!Environment::Development.is_production());
    }

    #[test]
    fn log_level_ordering_matches_production_ceiling() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Error > LogLevel::Info);
        assert_eq!(LogLevel::PRODUCTION_CEILING, LogLevel::Info);
    }
}
