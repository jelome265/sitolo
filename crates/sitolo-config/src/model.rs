//! Typed canonical configuration model (Phase 2 Appendix A).

use std::fmt;
use std::net::SocketAddr;

use sitolo_security::{SecretClass, SecretRef};

/// Hard ceilings that no override may bypass (Phase 2 §6, §38).
///
/// These are architecture policy, not tuning suggestions. An operator value
/// above a ceiling is an invalid configuration, never new policy.
pub mod ceilings {
    /// 25 MiB maximum request-body size ceiling (Phase 2 §6 example).
    pub const MAX_REQUEST_BODY_CEILING_BYTES: u64 = 26_214_400;
    pub const REQUEST_HEADER_TIMEOUT_CEILING_MS: u64 = 30_000;
    pub const KEEPALIVE_TIMEOUT_CEILING_MS: u64 = 300_000;
    pub const DB_ACQUIRE_TIMEOUT_CEILING_MS: u64 = 60_000;
    pub const OTEL_EXPORT_TIMEOUT_CEILING_MS: u64 = 60_000;
    pub const OTEL_MAX_QUEUE_CEILING: u32 = 1_048_576;
    pub const DB_POOL_MIN_CEILING: u32 = 256;
    pub const DB_POOL_MAX_CEILING: u32 = 512;
    /// Bounded string lengths protect against pathological configuration sizes
    /// (Phase 2 §47: bounded validation failure, never a parser blowup).
    pub const MAX_BOUNDED_STRING_BYTES: usize = 512;
    /// Schema version handled by this build.
    pub const CONFIG_SCHEMA_VERSION: u32 = 2;
}

/// Deployment environment for a process.
///
/// Unknown values fail startup (Appendix A: `environment`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Staging => "staging",
            Environment::Production => "production",
        }
    }

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
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
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
    pub environment: Environment,
    pub service_name: String,
    /// Provenance field; immutable at runtime (Appendix A).
    pub service_version: String,
    pub bind_address: SocketAddr,
    pub max_request_body_bytes: u64,
    pub request_header_timeout_ms: u64,
    pub keepalive_timeout_ms: u64,
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user: String,
    /// Secret reference; a raw database password is prohibited (Appendix A).
    pub db_password_ref: SecretRef,
    pub db_pool_min: u32,
    pub db_pool_max: u32,
    pub db_acquire_timeout_ms: u64,
    /// `None` disables OTLP export. TLS is required for remote endpoints.
    pub otel_endpoint: Option<String>,
    pub otel_export_timeout_ms: u64,
    pub otel_max_queue: u32,
    /// Sampling ratio in `0.0..=1.0`.
    pub trace_sample_ratio: f64,
    pub log_level: LogLevel,
    pub allow_local_secret_provider: bool,
    pub config_schema_version: u32,
}

impl AppConfig {
    /// Matches a `db_password_ref` to the database secret class.
    pub fn db_secret_ref(&self) -> &SecretRef {
        &self.db_password_ref
    }

    /// The classified secret reference for the database.
    pub fn database_secret_class(&self) -> SecretClass {
        SecretClass::Database
    }
}

/// Preset used by tests and local development. Still validated by
/// [`super::validate`]; this is a *default layer*, never an authority.
pub mod defaults {
    use std::net::SocketAddr;

    use sitolo_security::{SecretClass, SecretRef};

    use super::*;

    /// Compiled safe defaults for the catalogue (Appendix A examples).
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// A mutable default layer; layers only ever firm up these values.
    #[derive(Clone)]
    pub struct Builder {
        pub environment: Environment,
        pub service_name: String,
        pub service_version: String,
        pub bind_address: SocketAddr,
        pub max_request_body_bytes: u64,
        pub request_header_timeout_ms: u64,
        pub keepalive_timeout_ms: u64,
        pub db_host: String,
        pub db_port: u16,
        pub db_name: String,
        pub db_user: String,
        pub db_pool_min: u32,
        pub db_pool_max: u32,
        pub db_acquire_timeout_ms: u64,
        pub otel_endpoint: Option<String>,
        pub otel_export_timeout_ms: u64,
        pub otel_max_queue: u32,
        pub trace_sample_ratio: f64,
        pub log_level: LogLevel,
        pub allow_local_secret_provider: bool,
        pub config_schema_version: u32,
        /// `None` until a secret reference is supplied from an upper layer.
        pub db_password_ref: Option<SecretRef>,
    }

    impl Default for Builder {
        fn default() -> Self {
            Builder {
                environment: Environment::Development,
                service_name: "sitolo".to_string(),
                service_version: "0.0.0+dev".to_string(),
                bind_address: "0.0.0.0:8080".parse().expect("static default"),
                max_request_body_bytes: 2_097_152,
                request_header_timeout_ms: 5_000,
                keepalive_timeout_ms: 30_000,
                db_host: "localhost".to_string(),
                db_port: 5432,
                db_name: "sitolo".to_string(),
                db_user: "sitolo_api".to_string(),
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
                db_password_ref: None,
            }
        }
    }

    impl Builder {
        /// A development-oriented preset with a local secret reference, so the
        /// catalogue has a complete, valid development profile.
        pub fn development() -> Self {
            Builder {
                db_password_ref: Some(
                    SecretRef::new(SecretClass::Database, "development/sitolo/db")
                        .expect("static development reference"),
                ),
                ..Builder::default()
            }
        }

        /// A production-safe preset: everything required, no local provider,
        /// no telemetry unless explicitly configured.
        pub fn production() -> Self {
            Builder {
                environment: Environment::Production,
                log_level: LogLevel::Info,
                allow_local_secret_provider: false,
                otel_endpoint: Some("https://collector.sitolo.internal".to_string()),
                ..Builder::development()
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
