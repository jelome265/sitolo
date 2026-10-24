//! Least-privilege PostgreSQL pool construction for an API runtime.

use std::fmt;
use std::time::Duration;

use sitolo_config::DatabaseRuntimeConfig;
use sitolo_security::SecretValue;
use sqlx_core::query::query;
use sqlx_core::row::Row;
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgRow, PgSslMode};
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
    schema: String,
}

impl fmt::Debug for PgRuntimePool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PgRuntimePool")
            .field("schema", &self.schema)
            .finish_non_exhaustive()
    }
}

/// A pool authenticated directly as the restricted outbox worker identity.
#[derive(Clone)]
pub struct PgWorkerPool {
    pool: PgPool,
}

impl fmt::Debug for PgWorkerPool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PgWorkerPool")
            .finish_non_exhaustive()
    }
}

impl PgWorkerPool {
    /// Connects as `app_worker` and verifies the narrow outbox schema contract.
    ///
    /// # Errors
    ///
    /// Returns a generic fail-closed error when the TLS provider cannot be
    /// installed, the configured role is not the restricted worker identity,
    /// or the connection, role, or schema verification fails.
    pub async fn connect(
        config: &DatabaseRuntimeConfig,
        password: SecretValue,
        require_tls: bool,
    ) -> Result<Self, RuntimePoolError> {
        if !install_crypto_provider()
            || config.username != "app_worker"
            || !valid_schema_name(&config.schema)
        {
            return Err(RuntimePoolError::UnauthorizedRole);
        }
        let password = password.into_string();
        let options = PgConnectOptions::new_without_pgpass()
            .host(&config.host)
            .port(config.port)
            .username(&config.username)
            .password(&password)
            .database(&config.database)
            .application_name("sitolo-worker")
            .ssl_mode(tls_mode(require_tls))
            .options([("search_path", config.schema.as_str())]);
        drop(password);
        let pool = PgPoolOptions::new()
            .min_connections(config.pool_min)
            .max_connections(config.pool_max)
            .acquire_timeout(config.acquire_timeout)
            .max_lifetime(Some(Duration::from_mins(30)))
            .idle_timeout(Some(Duration::from_mins(10)))
            .connect_with(options)
            .await
            .map_err(|_error| RuntimePoolError::Unavailable)?;
        let identity = query(
            "SELECT current_user::text AS current_user, session_user::text AS session_user, \
             EXISTS (SELECT 1 FROM pg_roles WHERE rolname = current_user AND NOT rolsuper \
                 AND NOT rolbypassrls AND NOT rolcreaterole AND NOT rolcreatedb \
                 AND NOT rolinherit AND NOT rolreplication) AS role_restricted, \
             NOT EXISTS (SELECT 1 FROM pg_auth_members m JOIN pg_roles r ON r.oid = m.member \
                 WHERE r.rolname = current_user) AS has_no_role_memberships, \
             to_regnamespace($1) IS NOT NULL AS schema_exists, \
             to_regclass(format('%I.%I', $1, 'outbox_events')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'outbox_claim_events(integer,interval,integer)')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'outbox_mark_published(text,text)')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'outbox_release_for_retry(text,text,text,interval)')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'outbox_quarantine(text,text,text)')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'outbox_recover_stale_claims(interval)')) IS NOT NULL \
               AND has_function_privilege(current_user, to_regprocedure(format('%I.%s', $1, 'outbox_claim_events(integer,interval,integer)')), 'EXECUTE') \
               AND has_function_privilege(current_user, to_regprocedure(format('%I.%s', $1, 'outbox_mark_published(text,text)')), 'EXECUTE') \
               AND has_function_privilege(current_user, to_regprocedure(format('%I.%s', $1, 'outbox_release_for_retry(text,text,text,interval)')), 'EXECUTE') \
               AND has_function_privilege(current_user, to_regprocedure(format('%I.%s', $1, 'outbox_quarantine(text,text,text)')), 'EXECUTE') \
               AND has_function_privilege(current_user, to_regprocedure(format('%I.%s', $1, 'outbox_recover_stale_claims(interval)')), 'EXECUTE') \
               AS required_schema_present"
        ).bind(config.schema.as_str()).fetch_one(&pool).await.map_err(|_error| RuntimePoolError::Unavailable)?;
        let identity = IdentityRow::read(&identity)?;
        if let Err(error) = identity.verify("app_worker") {
            pool.close().await;
            return Err(error);
        }
        Ok(Self { pool })
    }

    /// Returns a clone of the verified worker pool.
    #[must_use]
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    /// Closes the underlying pool.
    pub async fn close(&self) {
        self.pool.close().await;
    }
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
    /// Returns a generic fail-closed error if the TLS provider cannot be
    /// installed or if connection, role, or schema verification fails. Driver
    /// details are not included in the error.
    pub async fn connect(
        config: &DatabaseRuntimeConfig,
        password: SecretValue,
        require_tls: bool,
    ) -> Result<Self, RuntimePoolError> {
        if !install_crypto_provider() {
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
            .ssl_mode(tls_mode(require_tls))
            .options([("search_path", config.schema.as_str())]);
        drop(password);

        let pool = PgPoolOptions::new()
            .min_connections(config.pool_min)
            .max_connections(config.pool_max)
            .acquire_timeout(config.acquire_timeout)
            .max_lifetime(Some(Duration::from_mins(30)))
            .idle_timeout(Some(Duration::from_mins(10)))
            .connect_with(options)
            .await
            .map_err(|_error| RuntimePoolError::Unavailable)?;

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
               AND to_regprocedure(format('%I.%s', $1, 'lock_organization_mutations(text)')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'transition_organization_state(text,text)')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'transition_branch_state(text,text,text)')) IS NOT NULL \
               AND to_regprocedure(format('%I.%s', $1, 'resolve_invitation_organization(bytea)')) IS NOT NULL \
               AS required_schema_present",
        )
        .bind(config.schema.as_str())
        .fetch_one(&pool)
        .await
        .map_err(|_error| RuntimePoolError::Unavailable)?;
        let identity = IdentityRow::read(&identity)?;

        if let Err(error) = identity.verify("app_runtime") {
            pool.close().await;
            return Err(error);
        }

        Ok(Self {
            pool,
            schema: config.schema.clone(),
        })
    }

    /// Verifies the live API connection identity and required Phase 4 schema.
    /// The probe is read-only and bounded so readiness cannot wait indefinitely.
    ///
    /// # Errors
    ///
    /// Returns a generic fail-closed error when the bounded probe times out,
    /// the connection fails, or the role or schema verification fails.
    pub async fn probe(&self) -> Result<(), RuntimePoolError> {
        let row = tokio::time::timeout(
            Duration::from_secs(1),
            query(
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
                   AND to_regprocedure(format('%I.%s', $1, 'lock_organization_mutations(text)')) IS NOT NULL \
                   AND to_regprocedure(format('%I.%s', $1, 'transition_organization_state(text,text)')) IS NOT NULL \
                   AND to_regprocedure(format('%I.%s', $1, 'transition_branch_state(text,text,text)')) IS NOT NULL \
                   AND to_regprocedure(format('%I.%s', $1, 'resolve_invitation_organization(bytea)')) IS NOT NULL \
                   AS required_schema_present",
            )
            .bind(self.schema.as_str())
            .fetch_one(&self.pool),
        )
        .await
        .map_err(|_error| RuntimePoolError::Unavailable)?
        .map_err(|_error| RuntimePoolError::Unavailable)?;
        IdentityRow::read(&row)?.verify("app_runtime")
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

/// Installs the process-wide rustls provider once, reporting whether one is
/// available. A concurrent install by another thread is treated as success.
fn install_crypto_provider() -> bool {
    if rustls::crypto::CryptoProvider::get_default().is_some() {
        return true;
    }
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .is_ok()
}

fn tls_mode(require_tls: bool) -> PgSslMode {
    if require_tls {
        PgSslMode::VerifyFull
    } else {
        PgSslMode::Prefer
    }
}

/// Identity and schema facts read from the live connection.
struct IdentityRow {
    current_user: String,
    session_user: String,
    role: RoleFacts,
    schema: SchemaFacts,
}

/// Privilege facts reported for the connected role.
struct RoleFacts {
    restricted: bool,
    has_no_role_memberships: bool,
}

/// Presence facts reported for the expected application schema.
struct SchemaFacts {
    exists: bool,
    required_present: bool,
}

impl IdentityRow {
    fn read(row: &PgRow) -> Result<Self, RuntimePoolError> {
        Ok(Self {
            current_user: read_column(row, "current_user")?,
            session_user: read_column(row, "session_user")?,
            role: RoleFacts {
                restricted: read_column(row, "role_restricted")?,
                has_no_role_memberships: read_column(row, "has_no_role_memberships")?,
            },
            schema: SchemaFacts {
                exists: read_column(row, "schema_exists")?,
                required_present: read_column(row, "required_schema_present")?,
            },
        })
    }

    fn verify(&self, expected_role: &str) -> Result<(), RuntimePoolError> {
        if self.current_user != expected_role
            || self.session_user != expected_role
            || !self.role.restricted
            || !self.role.has_no_role_memberships
        {
            return Err(RuntimePoolError::UnauthorizedRole);
        }
        if !self.schema.exists || !self.schema.required_present {
            return Err(RuntimePoolError::SchemaUnavailable);
        }
        Ok(())
    }
}

fn read_column<T>(row: &PgRow, column: &str) -> Result<T, RuntimePoolError>
where
    T: for<'r> sqlx_core::decode::Decode<'r, sqlx_postgres::Postgres>
        + sqlx_core::types::Type<sqlx_postgres::Postgres>,
{
    row.try_get(column)
        .map_err(|_error| RuntimePoolError::Unavailable)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn readiness_probe_fails_closed_for_a_closed_pool() -> Result<(), RuntimePoolError> {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://app_runtime:unused@127.0.0.1/sitolo")
            .map_err(|_error| RuntimePoolError::Unavailable)?;
        pool.close().await;
        let runtime = PgRuntimePool {
            pool,
            schema: "sitolo".to_owned(),
        };

        assert_eq!(runtime.probe().await, Err(RuntimePoolError::Unavailable));
        Ok(())
    }
}
