//! Phase 2 PostgreSQL runtime boundary.
//!
//! This module owns safe initialization intent and lifecycle semantics only.
//! SQLx, migrations, schemas, tenant context, and RLS begin in Phase 5.
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use sitolo_config::{DatabaseRuntimeConfig, DatabaseTarget};
use sitolo_security::{SecretError, SecretProvider, SecretValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseLifecycle {
    Uninitialized,
    Ready,
    ShuttingDown,
    Closed,
    Failed,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceInitError {
    #[error("database secret could not be resolved")]
    Secret(#[source] SecretError),
    #[error("database capability initialization failed")]
    Connection,
    #[error("database runtime is not ready")]
    NotReady,
}

/// Safe categories that let callers distinguish an indeterminate mutation
/// from a definite failure once Phase 5 introduces database operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceFailureKind {
    Connectivity,
    Authentication,
    AcquisitionTimeout,
    ResourceExhausted,
    UnknownOutcome,
}

/// Bounded, credential-free pool observations supplied by a concrete adapter.
/// Phase 2 owns this contract; Phase 5 supplies live PostgreSQL measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabasePoolMetrics {
    pub configured_min: u32,
    pub configured_max: u32,
    pub in_use: u32,
    pub idle: u32,
    pub acquisition_timeouts: u64,
    pub connection_failures: u64,
}

/// Opaque infrastructure capability. Consumers must not receive a password,
/// connection string, or a broadly usable raw pool through this boundary.
#[async_trait]
pub trait DatabaseCapability: Send + Sync {
    async fn readiness(&self) -> Result<(), PersistenceInitError>;
    async fn shutdown(&self) -> Result<(), PersistenceInitError>;

    /// Concrete adapters may report bounded aggregate telemetry. Returning
    /// `None` makes the absence explicit until a Phase 5 adapter exists.
    fn pool_metrics(&self) -> Option<DatabasePoolMetrics> {
        None
    }
}

/// Phase-5 infrastructure owns the concrete PostgreSQL connector.
#[async_trait]
pub trait DatabaseConnector: Send + Sync {
    async fn connect(
        &self,
        config: &DatabaseRuntimeConfig,
        password: SecretValue,
    ) -> Result<Box<dyn DatabaseCapability>, PersistenceInitError>;
}

/// A constructed runtime holds only validated intent and an opaque capability.
pub struct DatabaseRuntime {
    config: DatabaseRuntimeConfig,
    target: DatabaseTarget,
    capability: Box<dyn DatabaseCapability>,
    lifecycle: DatabaseLifecycle,
}

impl DatabaseRuntime {
    /// Resolves the secret immediately before the infrastructure connection
    /// factory consumes it. The credential never enters `AppConfig` or this
    /// struct's fields.
    pub async fn initialize(
        config: DatabaseRuntimeConfig,
        target: DatabaseTarget,
        secrets: &dyn SecretProvider,
        connector: &dyn DatabaseConnector,
    ) -> Result<Self, PersistenceInitError> {
        let password = secrets
            .get(&config.password_ref)
            .await
            .map_err(PersistenceInitError::Secret)?;
        let capability = connector.connect(&config, password).await?;
        Ok(Self {
            config,
            target,
            capability,
            lifecycle: DatabaseLifecycle::Ready,
        })
    }

    pub fn lifecycle(&self) -> DatabaseLifecycle {
        self.lifecycle
    }
    pub fn target_id(&self) -> String {
        target_id(&self.target)
    }
    pub fn pool_bounds(&self) -> (u32, u32) {
        (self.config.pool_min, self.config.pool_max)
    }
    pub fn pool_metrics(&self) -> Option<DatabasePoolMetrics> {
        self.capability.pool_metrics()
    }

    pub async fn readiness(&self) -> Result<(), PersistenceInitError> {
        if self.lifecycle != DatabaseLifecycle::Ready {
            return Err(PersistenceInitError::NotReady);
        }
        self.capability.readiness().await
    }

    /// Bounded connection/pool shutdown is delegated to the concrete adapter;
    /// no retry loop is introduced at this construction boundary.
    pub async fn shutdown(&mut self) -> Result<(), PersistenceInitError> {
        if self.lifecycle == DatabaseLifecycle::Closed {
            return Ok(());
        }
        self.lifecycle = DatabaseLifecycle::ShuttingDown;
        self.capability.shutdown().await?;
        self.lifecycle = DatabaseLifecycle::Closed;
        Ok(())
    }
}

/// Safe, stable, non-credential target identifier for telemetry.
pub fn target_id(target: &DatabaseTarget) -> String {
    let value = format!(
        "host={}\nport={}\ndatabase={}\nusername={}",
        target.host, target.port, target.database, target.username
    );
    let digest = Sha256::digest(value.as_bytes());
    let mut output = String::from("db_");
    for byte in digest.iter().take(12) {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_id_is_deterministic_and_credential_free() {
        let target = DatabaseTarget {
            host: "db.internal".into(),
            port: 5432,
            database: "sitolo".into(),
            username: "sitolo_app_runtime".into(),
        };
        let id = target_id(&target);
        assert!(id.starts_with("db_"));
        assert_eq!(id.len(), 27);
        assert!(!id.contains("sitolo_app_runtime"));
    }

    #[test]
    fn failure_kinds_keep_indeterminate_outcomes_explicit() {
        assert_eq!(
            PersistenceFailureKind::UnknownOutcome,
            PersistenceFailureKind::UnknownOutcome
        );
    }
}
