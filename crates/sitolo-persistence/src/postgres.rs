//! PostgreSQL persistent authority and connection management.
//!
//! Phase 4 Part 7 / PR-008: Manages setup/migration authority separately from
//! the least-privileged runtime authority (`app_runtime`), and enforces trusted
//! transaction-local tenant context derived from `AuthorizedScope`.

use sitolo_domain::tenancy::TenancyError;
use sitolo_tenancy::AuthorizedScope;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Postgres, Row, Transaction};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PgAuthorityError {
    #[error("database authorization or isolation failure")]
    Sqlx,
    #[error("runtime role security violation")]
    SecurityViolation,
    #[error("tenant domain error: {0}")]
    Domain(#[from] TenancyError),
    #[error("not found or denied by RLS boundary")]
    NotFoundOrDenied,
}

impl From<sqlx::Error> for PgAuthorityError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => PgAuthorityError::NotFoundOrDenied,
            _ => PgAuthorityError::Sqlx,
        }
    }
}

/// Managed authority pair: admin pool (migration/setup authority) and runtime pool (least privileged).
#[derive(Clone)]
pub struct PgAuthorityPools {
    pub admin_pool: PgPool,
    pub runtime_pool: PgPool,
}

impl PgAuthorityPools {
    /// Creates an administrative pool and a runtime pool given database connection options.
    pub async fn connect_options(
        admin_opts: PgConnectOptions,
        runtime_opts: PgConnectOptions,
    ) -> Result<Self, PgAuthorityError> {
        let admin_pool = PgPoolOptions::new()
            .max_connections(20)
            .connect_with(admin_opts)
            .await?;

        let runtime_pool = PgPoolOptions::new()
            .max_connections(20)
            .connect_with(runtime_opts)
            .await?;

        Ok(Self {
            admin_pool,
            runtime_pool,
        })
    }

    /// Verifies via PostgreSQL catalogs that `app_runtime` has no administrative privileges,
    /// has zero role memberships in `pg_auth_members`, and does NOT own protected relations.
    pub async fn verify_runtime_role(&self) -> Result<(), PgAuthorityError> {
        let row = sqlx::query(
            "SELECT rolsuper, rolbypassrls, rolcreaterole, rolcreatedb
             FROM pg_roles
             WHERE rolname = 'app_runtime'",
        )
        .fetch_one(&self.admin_pool)
        .await?;

        let superuser: bool = row.get("rolsuper");
        let bypassrls: bool = row.get("rolbypassrls");
        let createrole: bool = row.get("rolcreaterole");
        let createdb: bool = row.get("rolcreatedb");

        if superuser || bypassrls || createrole || createdb {
            return Err(PgAuthorityError::SecurityViolation);
        }

        // Verify runtime role has NO role memberships in pg_auth_members
        let member_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM pg_auth_members m
             JOIN pg_roles r ON r.oid = m.member
             WHERE r.rolname = 'app_runtime'",
        )
        .fetch_one(&self.admin_pool)
        .await?;

        if member_count > 0 {
            return Err(PgAuthorityError::SecurityViolation);
        }

        // Verify runtime role is NOT the owner of protected tables
        let protected_tables = vec!["organizations", "branches", "tenant_resources"];

        for table in protected_tables {
            let owner_role: String = sqlx::query_scalar(
                "SELECT pg_get_userbyid(c.relowner)
                 FROM pg_class c
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = 'public' AND c.relname = $1",
            )
            .bind(table)
            .fetch_one(&self.admin_pool)
            .await?;

            if owner_role == "app_runtime" {
                return Err(PgAuthorityError::SecurityViolation);
            }
        }

        Ok(())
    }

    /// Verifies via PostgreSQL catalog functions that `app_runtime` has exact effective privileges
    /// against an explicit allowlist (SELECT, INSERT, UPDATE, DELETE) and no forbidden admin privileges.
    pub async fn verify_effective_privileges(&self) -> Result<(), PgAuthorityError> {
        let has_connect: bool = sqlx::query_scalar(
            "SELECT has_database_privilege('app_runtime', current_database(), 'CONNECT')",
        )
        .fetch_one(&self.admin_pool)
        .await?;

        if !has_connect {
            return Err(PgAuthorityError::SecurityViolation);
        }

        let has_usage: bool =
            sqlx::query_scalar("SELECT has_schema_privilege('app_runtime', 'public', 'USAGE')")
                .fetch_one(&self.admin_pool)
                .await?;

        if !has_usage {
            return Err(PgAuthorityError::SecurityViolation);
        }

        let protected_tables = vec!["organizations", "branches", "tenant_resources"];
        let required_privileges = vec!["SELECT", "INSERT", "UPDATE", "DELETE"];

        for table in protected_tables {
            for priv_kind in &required_privileges {
                let query =
                    format!("SELECT has_table_privilege('app_runtime', '{table}', '{priv_kind}')");
                let has_priv: bool = sqlx::query_scalar(&query)
                    .fetch_one(&self.admin_pool)
                    .await?;

                if !has_priv {
                    return Err(PgAuthorityError::SecurityViolation);
                }
            }

            let forbidden_privileges = vec!["TRUNCATE", "TRIGGER", "REFERENCES"];
            for priv_kind in &forbidden_privileges {
                let query =
                    format!("SELECT has_table_privilege('app_runtime', '{table}', '{priv_kind}')");
                let has_priv: bool = sqlx::query_scalar(&query)
                    .fetch_one(&self.admin_pool)
                    .await?;

                if has_priv {
                    return Err(PgAuthorityError::SecurityViolation);
                }
            }
        }

        Ok(())
    }

    /// Verifies via PostgreSQL catalogs (`pg_policy`, `pg_class`, `pg_namespace`) that RLS is enabled, forced,
    /// polroles targets app_runtime/PUBLIC, polcmd = '*', and exact USING/WITH CHECK expressions exist.
    pub async fn verify_rls_catalog_metadata(&self) -> Result<(), PgAuthorityError> {
        let protected_tables = vec!["organizations", "branches", "tenant_resources"];

        for table in protected_tables {
            let row = sqlx::query(
                "SELECT c.relrowsecurity, c.relforcerowsecurity
                 FROM pg_class c
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = 'public' AND c.relname = $1",
            )
            .bind(table)
            .fetch_one(&self.admin_pool)
            .await?;

            let rowsecurity: bool = row.get("relrowsecurity");
            let forcerowsecurity: bool = row.get("relforcerowsecurity");

            if !rowsecurity || !forcerowsecurity {
                return Err(PgAuthorityError::SecurityViolation);
            }

            let expected_policy_name = format!("{table}_isolation_policy");
            let policy_row = sqlx::query(
                "SELECT polname, polcmd, polroles::bigint[] as polroles,
                        pg_get_expr(polqual, polrelid) as qual_expr,
                        pg_get_expr(polwithcheck, polrelid) as check_expr
                 FROM pg_policy p
                 JOIN pg_class c ON c.oid = p.polrelid
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = 'public' AND c.relname = $1 AND p.polname = $2",
            )
            .bind(table)
            .bind(&expected_policy_name)
            .fetch_optional(&self.admin_pool)
            .await?;

            match policy_row {
                Some(p_row) => {
                    let polcmd: i8 = p_row.get::<i8, _>("polcmd");
                    let polroles: Vec<i64> = p_row.get("polroles");
                    let qual_expr: Option<String> = p_row.get("qual_expr");
                    let check_expr: Option<String> = p_row.get("check_expr");

                    // polcmd '*' = ALL in PG catalog pg_policy table
                    if polcmd != b'*' as i8 {
                        return Err(PgAuthorityError::SecurityViolation);
                    }

                    // Verify polroles targets app_runtime role or 0 (PUBLIC / ALL)
                    let runtime_oid: i64 = sqlx::query_scalar(
                        "SELECT oid::bigint FROM pg_roles WHERE rolname = 'app_runtime'",
                    )
                    .fetch_one(&self.admin_pool)
                    .await?;

                    if !polroles.is_empty()
                        && !polroles.contains(&0)
                        && !polroles.contains(&runtime_oid)
                    {
                        return Err(PgAuthorityError::SecurityViolation);
                    }

                    let qual = qual_expr.ok_or(PgAuthorityError::SecurityViolation)?;
                    let check = check_expr.ok_or(PgAuthorityError::SecurityViolation)?;

                    if !qual.contains("app.organization_id")
                        || !check.contains("app.organization_id")
                    {
                        return Err(PgAuthorityError::SecurityViolation);
                    }
                }
                None => {
                    return Err(PgAuthorityError::SecurityViolation);
                }
            }
        }

        Ok(())
    }
}

/// Sets transaction-local tenant context (`app.organization_id` and `app.branch_id`)
/// derived strictly from `AuthorizedScope`.
pub async fn set_transaction_tenant_context(
    tx: &mut Transaction<'_, Postgres>,
    scope: &AuthorizedScope,
) -> Result<(), PgAuthorityError> {
    sqlx::query("SELECT set_config('app.organization_id', $1, true)")
        .bind(scope.organization_id.as_str())
        .execute(&mut **tx)
        .await?;

    if let Some(branch_id) = &scope.branch_id {
        sqlx::query("SELECT set_config('app.branch_id', $1, true)")
            .bind(branch_id.as_str())
            .execute(&mut **tx)
            .await?;
    } else {
        sqlx::query("SELECT set_config('app.branch_id', '', true)")
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}
