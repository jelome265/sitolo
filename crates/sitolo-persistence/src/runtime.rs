//! Phase 2 PostgreSQL runtime boundary.
//!
//! This module owns safe initialization intent and lifecycle semantics only.
//! `SQLx`, migrations, schemas, tenant context, and RLS begin in Phase 5.
use std::fmt;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use sitolo_config::{DatabaseRuntimeConfig, DatabaseTarget};
use sitolo_security::{SecretError, SecretProvider, SecretValue};

/// Lifecycle position of a constructed database runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseLifecycle {
    /// No connection attempt has been made.
    Uninitialized,
    /// The capability was constructed and passed its readiness contract.
    Ready,
    /// Shutdown has started but the adapter has not confirmed completion.
    ShuttingDown,
    /// The capability was shut down and will not be reused.
    Closed,
    /// Initialization or readiness failed and the runtime must not be used.
    Failed,
}

/// Fail-closed initialization failure. No driver detail or credential is kept.
#[derive(Debug, thiserror::Error)]
pub enum PersistenceInitError {
    /// The configured secret reference could not be resolved.
    #[error("database secret could not be resolved")]
    Secret(#[source] SecretError),
    /// The concrete connector could not establish the capability.
    #[error("database capability initialization failed")]
    Connection,
    /// The runtime is not in a usable lifecycle state.
    #[error("database runtime is not ready")]
    NotReady,
}

/// Safe categories that let callers distinguish an indeterminate mutation
/// from a definite failure once Phase 5 introduces database operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceFailureKind {
    /// The database could not be reached.
    Connectivity,
    /// The configured role failed authentication or authorization.
    Authentication,
    /// A pooled connection could not be acquired within its timeout.
    AcquisitionTimeout,
    /// The server rejected the work because a resource limit was reached.
    ResourceExhausted,
    /// The mutation outcome cannot be determined and must be reconciled.
    UnknownOutcome,
}

/// Bounded, credential-free pool observations supplied by a concrete adapter.
/// Phase 2 owns this contract; Phase 5 supplies live PostgreSQL measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabasePoolMetrics {
    /// Configured minimum number of pooled connections.
    pub configured_min: u32,
    /// Configured maximum number of pooled connections.
    pub configured_max: u32,
    /// Connections currently checked out.
    pub in_use: u32,
    /// Idle connections available for checkout.
    pub idle: u32,
    /// Cumulative count of acquisition timeouts.
    pub acquisition_timeouts: u64,
    /// Cumulative count of failed connection attempts.
    pub connection_failures: u64,
}

/// Opaque infrastructure capability. Consumers must not receive a password,
/// connection string, or a broadly usable raw pool through this boundary.
#[async_trait]
pub trait DatabaseCapability: Send + Sync {
    /// Verifies that the capability can currently serve requests.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceInitError`] when the capability is not usable.
    async fn readiness(&self) -> Result<(), PersistenceInitError>;

    /// Releases the capability's bounded resources.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceInitError`] when shutdown could not be confirmed.
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
    /// Constructs a capability from validated intent and a resolved secret.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceInitError`] when the capability cannot be built.
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

impl fmt::Debug for DatabaseRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DatabaseRuntime")
            .field("config", &self.config)
            .field("target", &self.target)
            .field("capability", &"<opaque>")
            .field("lifecycle", &self.lifecycle)
            .finish()
    }
}

impl DatabaseRuntime {
    /// Resolves the secret immediately before the infrastructure connection
    /// factory consumes it. The credential never enters `AppConfig` or this
    /// struct's fields.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceInitError::Secret`] when the secret reference
    /// cannot be resolved and [`PersistenceInitError::Connection`] when the
    /// connector cannot construct the capability.
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

    /// Returns the current lifecycle position.
    #[must_use]
    pub fn lifecycle(&self) -> DatabaseLifecycle {
        self.lifecycle
    }

    /// Returns the stable, non-credential target identifier for telemetry.
    #[must_use]
    pub fn target_id(&self) -> String {
        target_id(&self.target)
    }

    /// Returns the configured minimum and maximum pool sizes.
    #[must_use]
    pub fn pool_bounds(&self) -> (u32, u32) {
        (self.config.pool_min, self.config.pool_max)
    }

    /// Returns bounded pool observations when the adapter provides them.
    #[must_use]
    pub fn pool_metrics(&self) -> Option<DatabasePoolMetrics> {
        self.capability.pool_metrics()
    }

    /// Verifies that the runtime and its capability are ready.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceInitError::NotReady`] when the runtime has left the
    /// ready lifecycle state, or the adapter's error when its probe fails.
    pub async fn readiness(&self) -> Result<(), PersistenceInitError> {
        if self.lifecycle != DatabaseLifecycle::Ready {
            return Err(PersistenceInitError::NotReady);
        }
        self.capability.readiness().await
    }

    /// Bounded connection/pool shutdown is delegated to the concrete adapter;
    /// no retry loop is introduced at this construction boundary.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceInitError`] when the adapter cannot confirm
    /// shutdown. The runtime then remains in the shutting-down state.
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
fn target_id(target: &DatabaseTarget) -> String {
    use fmt::Write as _;

    let value = format!(
        "host={}\nport={}\ndatabase={}\nusername={}",
        target.host, target.port, target.database, target.username
    );
    let digest = Sha256::digest(value.as_bytes());
    let mut output = String::from("db_");
    for byte in digest.iter().take(12) {
        // Writing to a `String` is infallible, so the result carries no failure.
        let _written = write!(output, "{byte:02x}");
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
