//! PostgreSQL persistent authority and connection management.
//!
//! Phase 4 Part 7 / PR-008: Manages setup/migration authority separately from
//! the least-privileged runtime authority (`app_runtime`), and enforces trusted
//! transaction-local tenant context derived from `AuthorizedScope`.

use sitolo_domain::tenancy::{Branch, BranchId, Organization, OrganizationId, TenancyError};
use sitolo_tenancy::AuthorizedScope;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Postgres, Row, Transaction};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PgAuthorityError {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("runtime role security violation: {0}")]
    SecurityViolation(String),
    #[error("tenant domain error: {0}")]
    Domain(#[from] TenancyError),
    #[error("not found or denied by RLS boundary")]
    NotFoundOrDenied,
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
            .max_connections(2)
            .connect_with(admin_opts)
            .await?;

        let runtime_pool = PgPoolOptions::new()
            .max_connections(3)
            .connect_with(runtime_opts)
            .await?;

        Ok(Self {
            admin_pool,
            runtime_pool,
        })
    }

    /// Creates an administrative pool and a runtime pool given database URLs.
    pub async fn connect(admin_url: &str, runtime_url: &str) -> Result<Self, PgAuthorityError> {
        let admin_opts: PgConnectOptions = admin_url.parse()?;
        let runtime_opts: PgConnectOptions = runtime_url.parse()?;
        Self::connect_options(admin_opts, runtime_opts).await
    }

    /// Verifies via PostgreSQL catalogs that `app_runtime` has no administrative privileges
    /// and does NOT own protected relations.
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

        if superuser {
            return Err(PgAuthorityError::SecurityViolation(
                "app_runtime role must not be SUPERUSER".to_string(),
            ));
        }
        if bypassrls {
            return Err(PgAuthorityError::SecurityViolation(
                "app_runtime role must not have BYPASSRLS".to_string(),
            ));
        }
        if createrole {
            return Err(PgAuthorityError::SecurityViolation(
                "app_runtime role must not have CREATEROLE".to_string(),
            ));
        }
        if createdb {
            return Err(PgAuthorityError::SecurityViolation(
                "app_runtime role must not have CREATEDB".to_string(),
            ));
        }

        // Verify runtime role is NOT the owner of protected tables (table ownership bypasses RLS in PG)
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
                return Err(PgAuthorityError::SecurityViolation(format!(
                    "Protected relation {table} must NOT be owned by app_runtime"
                )));
            }
        }

        Ok(())
    }

    /// Verifies via PostgreSQL catalogs that RLS is enabled, forced, and expected policies exist.
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

            if !rowsecurity {
                return Err(PgAuthorityError::SecurityViolation(format!(
                    "Table {table} does not have ROW LEVEL SECURITY enabled"
                )));
            }
            if !forcerowsecurity {
                return Err(PgAuthorityError::SecurityViolation(format!(
                    "Table {table} does not have FORCE ROW LEVEL SECURITY enabled"
                )));
            }

            // Verify policy exists with expected policy name
            let expected_policy_name = format!("{table}_isolation_policy");
            let policy_row = sqlx::query(
                "SELECT polname, pg_get_expr(polqual, polrelid) as qual_expr, pg_get_expr(polwithcheck, polrelid) as check_expr
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
                    let qual_expr: Option<String> = p_row.get("qual_expr");
                    let check_expr: Option<String> = p_row.get("check_expr");

                    let qual = qual_expr.unwrap_or_default();
                    let check = check_expr.unwrap_or_default();

                    if !qual.contains("app.organization_id") {
                        return Err(PgAuthorityError::SecurityViolation(format!(
                            "Policy {expected_policy_name} on {table} USING expression does not reference app.organization_id"
                        )));
                    }
                    if !check.contains("app.organization_id") {
                        return Err(PgAuthorityError::SecurityViolation(format!(
                            "Policy {expected_policy_name} on {table} WITH CHECK expression does not reference app.organization_id"
                        )));
                    }
                }
                None => {
                    return Err(PgAuthorityError::SecurityViolation(format!(
                        "Table {table} missing expected policy {expected_policy_name}"
                    )));
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

/// Tenant Resource struct for Part 7 RLS boundary testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantResource {
    pub id: String,
    pub organization_id: String,
    pub branch_id: String,
    pub data: String,
}

/// PostgreSQL repository implementation utilizing runtime authority and RLS.
pub struct PgTenantRepository {
    runtime_pool: PgPool,
}

impl PgTenantRepository {
    pub fn new(runtime_pool: PgPool) -> Self {
        Self { runtime_pool }
    }

    /// Begins a transaction using the runtime pool and sets the trusted tenant context.
    pub async fn begin_tx(
        &self,
        scope: &AuthorizedScope,
    ) -> Result<Transaction<'_, Postgres>, PgAuthorityError> {
        let mut tx = self.runtime_pool.begin().await?;
        set_transaction_tenant_context(&mut tx, scope).await?;
        Ok(tx)
    }

    // --- Organization Operations ---

    pub async fn create_organization(
        &self,
        scope: &AuthorizedScope,
        org: &Organization,
    ) -> Result<(), PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        sqlx::query(
            "INSERT INTO organizations (id, name, state, state_version)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(org.id.as_str())
        .bind(&org.name)
        .bind("ACTIVE")
        .bind(org.state_version as i64)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_organization(
        &self,
        scope: &AuthorizedScope,
        org_id: &OrganizationId,
    ) -> Result<Organization, PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        let row = sqlx::query(
            "SELECT id, name, state, state_version
             FROM organizations
             WHERE id = $1 AND id = $2",
        )
        .bind(org_id.as_str())
        .bind(scope.organization_id.as_str())
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        match row {
            Some(row) => {
                let id_str: String = row.get("id");
                let name: String = row.get("name");
                let id = OrganizationId::new(id_str)?;
                let mut org = Organization::provision(id, &name)?;
                org.activate()?;
                Ok(org)
            }
            None => Err(PgAuthorityError::NotFoundOrDenied),
        }
    }

    // --- Branch Operations ---

    pub async fn create_branch(
        &self,
        scope: &AuthorizedScope,
        branch: &Branch,
    ) -> Result<(), PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        sqlx::query(
            "INSERT INTO branches (id, organization_id, name, state, state_version)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(branch.id.as_str())
        .bind(branch.organization_id.as_str())
        .bind(&branch.name)
        .bind("ACTIVE")
        .bind(branch.state_version as i64)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_branch(
        &self,
        scope: &AuthorizedScope,
        branch_id: &BranchId,
    ) -> Result<Branch, PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        let row = sqlx::query(
            "SELECT id, organization_id, name, state, state_version
             FROM branches
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(branch_id.as_str())
        .bind(scope.organization_id.as_str())
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        match row {
            Some(row) => {
                let id_str: String = row.get("id");
                let org_str: String = row.get("organization_id");
                let name: String = row.get("name");
                let id = BranchId::new(id_str)?;
                let org_id = OrganizationId::new(org_str)?;
                let mut b = Branch::provision(id, org_id, &name)?;
                b.activate()?;
                Ok(b)
            }
            None => Err(PgAuthorityError::NotFoundOrDenied),
        }
    }

    // --- Tenant Resource Operations ---

    pub async fn create_tenant_resource(
        &self,
        scope: &AuthorizedScope,
        resource: &TenantResource,
    ) -> Result<(), PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        sqlx::query(
            "INSERT INTO tenant_resources (id, organization_id, branch_id, data)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&resource.id)
        .bind(&resource.organization_id)
        .bind(&resource.branch_id)
        .bind(&resource.data)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_tenant_resource(
        &self,
        scope: &AuthorizedScope,
        resource_id: &str,
    ) -> Result<TenantResource, PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        let row = sqlx::query(
            "SELECT id, organization_id, branch_id, data
             FROM tenant_resources
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(resource_id)
        .bind(scope.organization_id.as_str())
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        match row {
            Some(row) => Ok(TenantResource {
                id: row.get("id"),
                organization_id: row.get("organization_id"),
                branch_id: row.get("branch_id"),
                data: row.get("data"),
            }),
            None => Err(PgAuthorityError::NotFoundOrDenied),
        }
    }

    pub async fn update_tenant_resource(
        &self,
        scope: &AuthorizedScope,
        resource_id: &str,
        new_data: &str,
    ) -> Result<(), PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        let result = sqlx::query(
            "UPDATE tenant_resources
             SET data = $1
             WHERE id = $2 AND organization_id = $3",
        )
        .bind(new_data)
        .bind(resource_id)
        .bind(scope.organization_id.as_str())
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(PgAuthorityError::NotFoundOrDenied);
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_tenant_resource_ownership(
        &self,
        scope: &AuthorizedScope,
        resource_id: &str,
        new_org_id: &str,
    ) -> Result<(), PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        let result = sqlx::query(
            "UPDATE tenant_resources
             SET organization_id = $1
             WHERE id = $2 AND organization_id = $3",
        )
        .bind(new_org_id)
        .bind(resource_id)
        .bind(scope.organization_id.as_str())
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(PgAuthorityError::NotFoundOrDenied);
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_tenant_resource(
        &self,
        scope: &AuthorizedScope,
        resource_id: &str,
    ) -> Result<(), PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        let result = sqlx::query(
            "DELETE FROM tenant_resources
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(resource_id)
        .bind(scope.organization_id.as_str())
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(PgAuthorityError::NotFoundOrDenied);
        }

        tx.commit().await?;
        Ok(())
    }
}
