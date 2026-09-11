//! Typed canonical configuration model (Phase 2 Appendix A).

use std::fmt;
use std::net::SocketAddr;
use std::time::Duration;

use sitolo_security::{SecretClass, SecretRef};

pub mod ceilings {
    pub const MAX_REQUEST_BODY_CEILING_BYTES: u64 = 26_214_400;
    pub const REQUEST_HEADER_TIMEOUT_CEILING_MS: u64 = 30_000;
    pub const KEEPALIVE_TIMEOUT_CEILING_MS: u64 = 300_000;
    pub const DB_ACQUIRE_TIMEOUT_CEILING_MS: u64 = 60_000;
    pub const OTEL_EXPORT_TIMEOUT_CEILING_MS: u64 = 60_000;
    pub const OTEL_MAX_QUEUE_CEILING: u32 = 1_048_576;
    pub const DB_POOL_MIN_CEILING: u32 = 256;
    pub const DB_POOL_MAX_CEILING: u32 = 512;
    pub const MAX_BOUNDED_STRING_BYTES: usize = 512;
    pub const CONFIG_SCHEMA_VERSION: u32 = 2;
}

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

    pub const PRODUCTION_CEILING: LogLevel = LogLevel::Info;
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppConfig {
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
    pub db_password_ref: SecretRef,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseRuntimeConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password_ref: SecretRef,
    pub pool_min: u32,
    pub pool_max: u32,
    pub acquire_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseTarget {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
}

impl AppConfig {
    pub fn db_secret_ref(&self) -> &SecretRef {
        &self.db_password_ref
    }

    pub fn database_secret_class(&self) -> SecretClass {
        SecretClass::Database
    }

    pub fn database_runtime(&self) -> DatabaseRuntimeConfig {
        DatabaseRuntimeConfig {
            host: self.db_host.clone(),
            port: self.db_port,
            database: self.db_name.clone(),
            username: self.db_user.clone(),
            password_ref: self.db_password_ref.clone(),
            pool_min: self.db_pool_min,
            pool_max: self.db_pool_max,
            acquire_timeout: Duration::from_millis(self.db_acquire_timeout_ms),
        }
    }

    pub fn database_target(&self) -> DatabaseTarget {
        DatabaseTarget {
            host: self.db_host.clone(),
            port: self.db_port,
            database: self.db_name.clone(),
            username: self.db_user.clone(),
        }
    }
}

pub mod defaults {
    use std::net::SocketAddr;

    use sitolo_security::{SecretClass, SecretRef};

    use super::*;

    pub fn builder() -> Builder {
        Builder::default()
    }

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
        pub db_password_ref: Option<SecretRef>,
    }

    impl Default for Builder {
        fn default() -> Self {
            Builder {
                environment: Environment::Development,
                service_name: "sitolo".to_string(),
                service_version: "0.0.0+dev".to_string(),
                bind_address: SocketAddr::from(([0, 0, 0, 0], 8080)),
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
        pub fn development() -> Self {
            Builder {
                db_password_ref: Some(
                    SecretRef::new(SecretClass::Database, "development/sitolo/db")
                        .unwrap_or_else(|_| std::process::abort()),
                ),
                ..Builder::default()
            }
        }

        pub fn staging() -> Self {
            Builder {
                environment: Environment::Staging,
                log_level: LogLevel::Info,
                allow_local_secret_provider: false,
                otel_endpoint: None,
                db_host: String::new(),
                db_name: String::new(),
                db_user: String::new(),
                db_password_ref: None,
                ..Builder::default()
            }
        }

        pub fn production() -> Self {
            Builder {
                environment: Environment::Production,
                log_level: LogLevel::Info,
                allow_local_secret_provider: false,
                otel_endpoint: Some("https://collector.sitolo.internal".to_string()),
                db_host: String::new(),
                db_name: String::new(),
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
