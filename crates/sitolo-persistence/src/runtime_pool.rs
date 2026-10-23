//! Least-privilege PostgreSQL pool construction for an API runtime.

use sitolo_config::DatabaseRuntimeConfig;
use sitolo_security::SecretValue;
use sqlx_core::query::query;
use sqlx_core::row::Row;
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgSslMode};
use thiserror::Error;

/// Startup failures for the API runtime pool. Database errors and connection
/// details are deliberately not retained because driver diagnostics can expose
/// connection configuration.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePoolError {
    /// The configured role did not authenticate directly as `app_runtime`.
    #[error("database runtime role is not authorized")]
    UnauthorizedRole,
    /// The configured schema is not present in the selected database.
    #[error("database runtime schema is unavailable")]
    SchemaUnavailable,
    /// PostgreSQL could not be connected to or the role could not be verified.
    #[error("database runtime is unavailable")]
    Unavailable,
}

/// A PostgreSQL pool authenticated directly as the restricted API role.
#[derive(Clone)]
pub struct PgRuntimePool {
    pool: PgPool,
}

impl PgRuntimePool {
    /// Opens a bounded pool and verifies its login role and configured schema.
    ///
    /// Non-development environments require TLS with full certificate and host
    /// verification. The configured schema is pinned as the connection's
    /// `search_path`, which keeps unqualified tenancy statements within the
    /// selected schema.
    ///
    /// # Errors
    /// Returns a generic fail-closed error if connection, role, or schema
    /// verification fails. Driver details are not included in the error.
    pub async fn connect(
        config: &DatabaseRuntimeConfig,
        password: SecretValue,
        require_tls: bool,
    ) -> Result<Self, RuntimePoolError> {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        }
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            return Err(RuntimePoolError::Unavailable);
        }
        if config.username != "app_runtime" || !valid_schema_name(&config.schema) {
            return Err(RuntimePoolError::UnauthorizedRole);
        }

        // SQLx reads PG* environment values when creating options. All
        // connection identity, credential, TLS mode, and search_path fields
        // that affect this runtime boundary are overwritten from validated
        // application configuration before a connection is attempted.
        let password = password.into_string();
        let options = PgConnectOptions::new_without_pgpass()
            .host(&config.host)
            .port(config.port)
            .username(&config.username)
            .password(&password)
            .database(&config.database)
            .application_name("sitolo-api")
            .ssl_mode(if require_tls {
                PgSslMode::VerifyFull
            } else {
                PgSslMode::Prefer
            })
            .options([("search_path", config.schema.as_str())]);
        drop(password);

        let pool = PgPoolOptions::new()
            .min_connections(config.pool_min)
            .max_connections(config.pool_max)
            .acquire_timeout(config.acquire_timeout)
            .max_lifetime(Some(std::time::Duration::from_secs(1_800)))
            .idle_timeout(Some(std::time::Duration::from_secs(600)))
            .connect_with(options)
            .await
            .map_err(|_| RuntimePoolError::Unavailable)?;

        let identity = query(
            "SELECT current_user::text AS current_user, session_user::text AS session_user, \
             EXISTS (SELECT 1 FROM pg_roles WHERE rolname = current_user AND NOT rolsuper \
                 AND NOT rolbypassrls AND NOT rolcreaterole AND NOT rolcreatedb \
                 AND NOT rolinherit AND NOT rolreplication) AS role_restricted, \
             NOT EXISTS (SELECT 1 FROM pg_auth_members m JOIN pg_roles r ON r.oid = m.member \
                 WHERE r.rolname = current_user) AS has_no_role_memberships, \
             to_regnamespace($1) IS NOT NULL AS schema_exists, \
             to_regclass(format('%I.%I', $1, 'identity_users')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'external_identities')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'identity_devices')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'identity_sessions')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'organizations')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'organization_memberships')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'branches')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'role_assignments')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'scope_grants')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'invitations')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'audit_events')) IS NOT NULL \
               AND to_regclass(format('%I.%I', $1, 'outbox_events')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'resolve_external_identity(text,text)')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'accept_sitolo_session(text,text,timestamp with time zone)')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'load_identity_device(text,text)')) IS NOT NULL \
               AS required_schema_present",
        )
        .bind(config.schema.as_str())
        .fetch_one(&pool)
        .await
        .map_err(|_| RuntimePoolError::Unavailable)?;
        let current_user: String = identity
            .try_get("current_user")
            .map_err(|_| RuntimePoolError::Unavailable)?;
        let session_user: String = identity
            .try_get("session_user")
            .map_err(|_| RuntimePoolError::Unavailable)?;
        let role_restricted: bool = identity
            .try_get("role_restricted")
            .map_err(|_| RuntimePoolError::Unavailable)?;
        let no_memberships: bool = identity
            .try_get("has_no_role_memberships")
            .map_err(|_| RuntimePoolError::Unavailable)?;
        let schema_exists: bool = identity
            .try_get("schema_exists")
            .map_err(|_| RuntimePoolError::Unavailable)?;
        let required_schema_present: bool = identity
            .try_get("required_schema_present")
            .map_err(|_| RuntimePoolError::Unavailable)?;

        if current_user != "app_runtime"
            || session_user != "app_runtime"
            || !role_restricted
            || !no_memberships
        {
            pool.close().await;
            return Err(RuntimePoolError::UnauthorizedRole);
        }
        if !schema_exists || !required_schema_present {
            pool.close().await;
            return Err(RuntimePoolError::SchemaUnavailable);
        }

        Ok(Self { pool })
    }

    /// Returns a clone of the pool for adapters that share the runtime identity.
    #[must_use]
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    /// Closes the underlying pool and waits for checked-out connections.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

fn valid_schema_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte == b'_')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
