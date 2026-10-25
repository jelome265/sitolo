//! Phase 4 Part 7 / PR-008 PostgreSQL Row-Level Security Integration & Real Negative Tests
//!
//! Non-negotiable contract requirement:
//! Execute security proofs against a REAL PostgreSQL database instance.
//! Prove tenant boundary against PostgreSQL RLS, least-privileged runtime role,
//! transaction-local context derived from `AuthorizedScope`, and negative tests.

use sitolo_domain::tenancy::{
    Branch, BranchId, Membership, MembershipId, Organization, OrganizationId, TenantUserId,
};
use sitolo_persistence::postgres::{
    PgAuthorityError, PgAuthorityPools, set_transaction_tenant_context,
};
use sitolo_tenancy::{
    AuthorizedScope, RequestedOrganizationId, bind_organization, resolve_effective_scope,
};
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::raw_sql::raw_sql;
use sqlx_core::row::Row;
use sqlx_core::transaction::Transaction;
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions, Postgres};
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::OnceCell;
use tokio::task::JoinSet;

mod common;

static MIGRATIONS_INIT: OnceCell<Result<(), String>> = OnceCell::const_new();

tokio::task_local! {
    static INJECT_SETUP_FAILURE: bool;
}

/// Test-harness tenant resource entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantResource {
    /// Resource identifier.
    pub id: String,
    /// Owning organization identifier.
    pub organization_id: String,
    /// Owning branch identifier.
    pub branch_id: String,
    /// Opaque payload used to prove row-level isolation.
    pub data: String,
}

/// Test-harness repository helper for Part 7 security proof operations with invocation tracking.
#[derive(Debug)]
pub struct PgTestTenantRepository {
    /// Least-privileged runtime pool used for every statement.
    pub runtime_pool: PgPool,
    /// Number of transactions opened through this helper.
    pub invocation_count: Arc<AtomicUsize>,
}

impl PgTestTenantRepository {
    /// Builds a repository over the supplied runtime pool.
    #[must_use]
    pub fn new(runtime_pool: PgPool) -> Self {
        Self {
            runtime_pool,
            invocation_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Opens a transaction with the tenant context derived from `scope`.
    ///
    /// # Errors
    /// Returns [`PgAuthorityError::Sqlx`] when the transaction cannot start or
    /// the tenant context cannot be applied.
    pub async fn begin_tx(
        &self,
        scope: &AuthorizedScope,
    ) -> Result<Transaction<'_, Postgres>, PgAuthorityError> {
        self.invocation_count.fetch_add(1, Ordering::SeqCst);
        let mut tx = self.runtime_pool.begin().await?;
        set_transaction_tenant_context(&mut tx, scope).await?;
        Ok(tx)
    }

    /// Inserts a tenant resource under the tenant context derived from `scope`.
    ///
    /// # Errors
    /// Returns [`PgAuthorityError::Sqlx`] when the insert or commit fails.
    pub async fn create_tenant_resource(
        &self,
        scope: &AuthorizedScope,
        resource: &TenantResource,
    ) -> Result<(), PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        query(
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

    /// Reads a tenant resource inside the tenant context derived from `scope`.
    ///
    /// # Errors
    /// Returns [`PgAuthorityError::NotFoundOrDenied`] when no visible row
    /// matches, and [`PgAuthorityError::Sqlx`] when the query fails.
    pub async fn get_tenant_resource(
        &self,
        scope: &AuthorizedScope,
        resource_id: &str,
    ) -> Result<TenantResource, PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        let row = query(
            "SELECT id, organization_id, branch_id, data
             FROM tenant_resources
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(resource_id)
        .bind(scope.organization_id().as_str())
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

    /// Updates a tenant resource inside the tenant context derived from `scope`.
    ///
    /// # Errors
    /// Returns [`PgAuthorityError::NotFoundOrDenied`] when no row matches, and
    /// [`PgAuthorityError::Sqlx`] when the update or commit fails.
    pub async fn update_tenant_resource(
        &self,
        scope: &AuthorizedScope,
        resource_id: &str,
        new_data: &str,
    ) -> Result<(), PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        let result = query(
            "UPDATE tenant_resources
             SET data = $1
             WHERE id = $2 AND organization_id = $3",
        )
        .bind(new_data)
        .bind(resource_id)
        .bind(scope.organization_id().as_str())
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(PgAuthorityError::NotFoundOrDenied);
        }

        tx.commit().await?;
        Ok(())
    }

    /// Deletes a tenant resource inside the tenant context derived from `scope`.
    ///
    /// # Errors
    /// Returns [`PgAuthorityError::NotFoundOrDenied`] when no row matches, and
    /// [`PgAuthorityError::Sqlx`] when the delete or commit fails.
    pub async fn delete_tenant_resource(
        &self,
        scope: &AuthorizedScope,
        resource_id: &str,
    ) -> Result<(), PgAuthorityError> {
        let mut tx = self.begin_tx(scope).await?;
        let result = query(
            "DELETE FROM tenant_resources
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(result_id_str(resource_id))
        .bind(scope.organization_id().as_str())
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

fn result_id_str(s: &str) -> &str {
    s
}

/// Minimal test-only application orchestration seam verifying persistence invocation bounds.
///
/// # Errors
/// Returns [`PgAuthorityError::Domain`] when the requested organization binding
/// or effective-scope resolution is rejected, and propagates read failures from
/// [`PgTestTenantRepository::get_tenant_resource`].
pub async fn execute_application_tenant_operation(
    membership: &Membership,
    organization: &Organization,
    requested_org_id: &RequestedOrganizationId,
    repo: &PgTestTenantRepository,
    resource_id: &str,
) -> Result<TenantResource, PgAuthorityError> {
    let _trusted_org = bind_organization(requested_org_id, membership)?;

    let eff_scope = resolve_effective_scope(membership, organization, None)?;

    let authorized_scope = AuthorizedScope::from_effective(&eff_scope);

    repo.get_tenant_resource(&authorized_scope, resource_id)
        .await
}

/// Test context holding admin and runtime pools, plus pre-seeded fixture IDs.
struct SchemaGuard {
    schema_name: String,
    admin_pool: PgPool,
}

impl SchemaGuard {
    async fn teardown(&self) -> Result<(), PgAuthorityError> {
        let drop_sql = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", self.schema_name);
        raw_sql(sqlx_core::sql_str::AssertSqlSafe(drop_sql.as_str()))
            .execute(&self.admin_pool)
            .await?;
        Ok(())
    }
}

struct TestContext {
    pools: PgAuthorityPools,
    repo: PgTestTenantRepository,
    guard: SchemaGuard,
    schema_name: String,
    org_a: Organization,
    org_b: Organization,
    mem_a: Membership,
    _mem_b: Membership,
    org_a_scope: AuthorizedScope,
    org_b_scope: AuthorizedScope,
    org_a_b1_scope: AuthorizedScope,
    res_a1_id: String,
    res_a2_id: String,
    res_b1_id: String,
    org_a_id_str: String,
    org_b_id_str: String,
    branch_a1_id_str: String,
    _branch_a2_id_str: String,
    branch_b1_id_str: String,
}

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[expect(
    clippy::disallowed_methods,
    reason = "FW-TOK-03 / BP-AS-04: the task isolates an unwind for teardown and its handle is awaited immediately."
)]
async fn run_test_with_teardown<F, Fut>(test_fn: F) -> TestResult
where
    F: FnOnce(Arc<TestContext>) -> Fut,
    Fut: Future<Output = TestResult> + Send + 'static,
{
    let ctx = Arc::new(setup_test_context(None).await?);

    let current_user: String = query_scalar("SELECT current_user")
        .fetch_one(ctx.pools.runtime_pool())
        .await?;
    if current_user != "app_runtime" {
        let identity_error = std::io::Error::other(format!(
            "runtime pool connection identity must be app_runtime, got {current_user}"
        ));
        return match ctx.guard.teardown().await {
            Ok(()) => Err(identity_error.into()),
            Err(teardown_error) => Err(std::io::Error::other(format!(
                "{identity_error}; schema teardown also failed: {teardown_error}"
            ))
            .into()),
        };
    }

    let test_result = tokio::spawn(AssertUnwindSafe(test_fn(Arc::clone(&ctx))))
        .await
        .map_err(|error| std::io::Error::other(format!("test task failed: {error}")))?;
    let teardown_result = ctx.guard.teardown().await;
    match (test_result, teardown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(test_error), Ok(())) => Err(test_error),
        (Ok(()), Err(teardown_error)) => Err(teardown_error.into()),
        (Err(test_error), Err(teardown_error)) => Err(std::io::Error::other(format!(
            "test failed: {test_error}; schema teardown also failed: {teardown_error}"
        ))
        .into()),
    }
}

async fn setup_test_context(
    schema_name_override: Option<&str>,
) -> Result<TestContext, Box<dyn std::error::Error + Send + Sync>> {
    let schema_name = if let Some(name) = schema_name_override {
        name.to_string()
    } else {
        let schema_id = uuid::Uuid::new_v4().simple().to_string();
        format!("test_schema_{schema_id}")
    };

    let targets = common::targets_from_env().map_err(std::io::Error::other)?;
    let mut admin_opts: PgConnectOptions = targets.admin;
    let mut runtime_opts: PgConnectOptions = targets.runtime;

    admin_opts = admin_opts.options([("search_path", schema_name.as_str())]);
    runtime_opts = runtime_opts.options([("search_path", schema_name.as_str())]);

    let admin_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(admin_opts.clone())
        .await?;

    MIGRATIONS_INIT
        .get_or_init(|| async {
            let role_sql = "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_runtime') THEN CREATE ROLE app_runtime WITH LOGIN NOSUPERUSER NOINHERIT NOCREATEDB NOCREATEROLE NOBYPASSRLS; END IF; END $$; ALTER ROLE app_runtime WITH LOGIN NOSUPERUSER NOINHERIT NOCREATEDB NOCREATEROLE NOBYPASSRLS;";
            raw_sql(role_sql)
                .execute(&admin_pool)
                .await
                .map(|_| ())
                .map_err(|error| format!("failed to initialize app_runtime role: {error}"))
        })
        .await
        .as_ref()
        .map_err(|error| std::io::Error::other(error.clone()))?;

    // Create isolated schema
    let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\"");
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(
        create_schema_sql.as_str(),
    ))
    .execute(&admin_pool)
    .await?;

    let guard = SchemaGuard {
        schema_name: schema_name.clone(),
        admin_pool: admin_pool.clone(),
    };

    let result =
        setup_test_context_inner(&schema_name, &admin_opts, &runtime_opts, &admin_pool).await;

    match result {
        Ok(ctx) => Ok(ctx),
        Err(setup_err) => match guard.teardown().await {
            Ok(()) => Err(Box::new(setup_err)),
            Err(td_err) => Err(Box::new(PgAuthorityError::ComposedSetupAndTeardownError {
                setup_error: Box::new(setup_err),
                teardown_error: Box::new(td_err),
            })),
        },
    }
}

async fn setup_test_context_inner(
    schema_name: &str,
    admin_opts: &PgConnectOptions,
    runtime_opts: &PgConnectOptions,
    admin_pool: &PgPool,
) -> Result<TestContext, PgAuthorityError> {
    // Apply Part 7 schema inside search_path
    let schema_sql = include_str!("fixtures/rls_schema.sql");
    raw_sql(schema_sql).execute(admin_pool).await?;

    // Grant schema USAGE and table privileges on isolated schema to app_runtime
    let grant_schema_sql = format!(
        "GRANT USAGE ON SCHEMA \"{schema_name}\" TO app_runtime; GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA \"{schema_name}\" TO app_runtime;"
    );
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(grant_schema_sql.as_str()))
        .execute(admin_pool)
        .await?;

    let pools = PgAuthorityPools::connect_options(admin_opts.clone(), runtime_opts.clone()).await?;

    let unique_id = uuid::Uuid::new_v4().simple().to_string();
    let org_a_id_str = format!("org_A_{unique_id}");
    let org_b_id_str = format!("org_B_{unique_id}");
    let branch_a1_id_str = format!("branch_A1_{unique_id}");
    let branch_a2_id_str = format!("branch_A2_{unique_id}");
    let branch_b1_id_str = format!("branch_B1_{unique_id}");

    let res_a1_id = format!("res_A1_{unique_id}");
    let res_a2_id = format!("res_A2_{unique_id}");
    let res_b1_id = format!("res_B1_{unique_id}");

    // Build domain objects
    let org_a_id = OrganizationId::new(&org_a_id_str)?;
    let org_b_id = OrganizationId::new(&org_b_id_str)?;

    let mut org_a = Organization::provision(org_a_id.clone(), "Organization A")?;
    org_a.activate()?;

    let mut org_b = Organization::provision(org_b_id.clone(), "Organization B")?;
    org_b.activate()?;

    let branch_a1_id = BranchId::new(&branch_a1_id_str)?;
    let branch_a2_id = BranchId::new(&branch_a2_id_str)?;
    let branch_b1_id = BranchId::new(&branch_b1_id_str)?;

    let mut branch_a1 = Branch::provision(branch_a1_id.clone(), org_a_id.clone(), "Branch A1")?;
    branch_a1.activate()?;

    let mut branch_a2 = Branch::provision(branch_a2_id.clone(), org_a_id.clone(), "Branch A2")?;
    branch_a2.activate()?;

    let mut branch_b1 = Branch::provision(branch_b1_id.clone(), org_b_id.clone(), "Branch B1")?;
    branch_b1.activate()?;

    let mem_a_id = MembershipId::new(format!("mem_A_{unique_id}"))?;
    let mem_b_id = MembershipId::new(format!("mem_B_{unique_id}"))?;
    let user_a_id = TenantUserId::new(format!("user_A_{unique_id}"))?;
    let user_b_id = TenantUserId::new(format!("user_B_{unique_id}"))?;

    let mut mem_a = Membership::invite(mem_a_id.clone(), org_a_id.clone(), user_a_id);
    mem_a.mark_pending()?;
    mem_a.activate()?;

    let mut mem_b = Membership::invite(mem_b_id.clone(), org_b_id.clone(), user_b_id);
    mem_b.mark_pending()?;
    mem_b.activate()?;

    // Derive AuthorizedScope via full Phase 4 scope resolution pipeline
    let req_a = RequestedOrganizationId(org_a_id.clone());
    let _trusted_a = bind_organization(&req_a, &mem_a)?;
    let eff_a = resolve_effective_scope(&mem_a, &org_a, None)?;
    let org_a_scope = AuthorizedScope::from_effective(&eff_a);

    let req_b = RequestedOrganizationId(org_b_id.clone());
    let _trusted_b = bind_organization(&req_b, &mem_b)?;
    let eff_b = resolve_effective_scope(&mem_b, &org_b, None)?;
    let org_b_scope = AuthorizedScope::from_effective(&eff_b);

    let eff_a_b1 = resolve_effective_scope(&mem_a, &org_a, Some(&branch_a1))?;
    let org_a_b1_scope = AuthorizedScope::from_effective(&eff_a_b1);

    let repo = PgTestTenantRepository::new(pools.runtime_pool().clone());

    // Seed the organizations, branches, and tenant resources the proofs need.
    let fixtures = FixtureIds {
        org_a: &org_a_id_str,
        org_b: &org_b_id_str,
        branch_a1: &branch_a1_id_str,
        branch_a2: &branch_a2_id_str,
        branch_b1: &branch_b1_id_str,
        res_a1: &res_a1_id,
        res_a2: &res_a2_id,
        res_b1: &res_b1_id,
    };
    seed_fixtures(admin_pool, &fixtures).await?;

    if INJECT_SETUP_FAILURE.try_with(|b| *b).unwrap_or(false) {
        return Err(PgAuthorityError::SecurityViolation);
    }

    Ok(TestContext {
        pools,
        repo,
        guard: SchemaGuard {
            schema_name: schema_name.to_string(),
            admin_pool: admin_pool.clone(),
        },
        schema_name: schema_name.to_string(),
        org_a,
        org_b,
        mem_a,
        _mem_b: mem_b,
        org_a_scope,
        org_b_scope,
        org_a_b1_scope,
        res_a1_id,
        res_a2_id,
        res_b1_id,
        org_a_id_str,
        org_b_id_str,
        branch_a1_id_str,
        _branch_a2_id_str: branch_a2_id_str,
        branch_b1_id_str,
    })
}

/// Identifiers of the rows seeded by the Part 7 fixture.
struct FixtureIds<'a> {
    org_a: &'a str,
    org_b: &'a str,
    branch_a1: &'a str,
    branch_a2: &'a str,
    branch_b1: &'a str,
    res_a1: &'a str,
    res_a2: &'a str,
    res_b1: &'a str,
}

/// Seeds every row the Part 7 RLS proofs read: two organizations, three
/// branches, and three tenant resources.
async fn seed_fixtures(admin_pool: &PgPool, ids: &FixtureIds<'_>) -> Result<(), PgAuthorityError> {
    seed_organizations(admin_pool, ids.org_a, ids.org_b).await?;
    seed_branches(
        admin_pool,
        ids.org_a,
        ids.org_b,
        ids.branch_a1,
        ids.branch_a2,
        ids.branch_b1,
    )
    .await?;
    seed_tenant_resources(admin_pool, ids).await
}

/// Seeds the two organizations used by the Part 7 fixture.
async fn seed_organizations(
    admin_pool: &PgPool,
    org_a_id: &str,
    org_b_id: &str,
) -> Result<(), PgAuthorityError> {
    query(
        "INSERT INTO organizations (id, name, state, state_version) VALUES ($1, $2, $3, $4), ($5, $6, $7, $8)",
    )
    .bind(org_a_id)
    .bind("Organization A")
    .bind("ACTIVE")
    .bind(1i64)
    .bind(org_b_id)
    .bind("Organization B")
    .bind("ACTIVE")
    .bind(1i64)
    .execute(admin_pool)
    .await?;
    Ok(())
}

/// Seeds the active branches used by the Part 7 fixture.
async fn seed_branches(
    admin_pool: &PgPool,
    org_a_id: &str,
    org_b_id: &str,
    branch_a1_id: &str,
    branch_a2_id: &str,
    branch_b1_id: &str,
) -> Result<(), PgAuthorityError> {
    query(
        "INSERT INTO branches (id, organization_id, name, state, state_version) VALUES
         ($1, $2, 'Branch A1', 'ACTIVE', 1),
         ($3, $4, 'Branch A2', 'ACTIVE', 1),
         ($5, $6, 'Branch B1', 'ACTIVE', 1)",
    )
    .bind(branch_a1_id)
    .bind(org_a_id)
    .bind(branch_a2_id)
    .bind(org_a_id)
    .bind(branch_b1_id)
    .bind(org_b_id)
    .execute(admin_pool)
    .await?;
    Ok(())
}

/// Seeds the tenant resources used by the Part 7 fixture.
async fn seed_tenant_resources(
    admin_pool: &PgPool,
    ids: &FixtureIds<'_>,
) -> Result<(), PgAuthorityError> {
    query(
        "INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES
         ($1, $2, $3, 'Secret Data A1'),
         ($4, $5, $6, 'Secret Data A2'),
         ($7, $8, $9, 'Secret Data B1')",
    )
    .bind(ids.res_a1)
    .bind(ids.org_a)
    .bind(ids.branch_a1)
    .bind(ids.res_a2)
    .bind(ids.org_a)
    .bind(ids.branch_a2)
    .bind(ids.res_b1)
    .bind(ids.org_b)
    .bind(ids.branch_b1)
    .execute(admin_pool)
    .await?;
    Ok(())
}

// ============================================================================
// 1. CATALOG & ROLE PRIVILEGE ASSERTIONS
// ============================================================================

#[tokio::test]
async fn test_catalog_runtime_role_privileges() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        ctx.pools.verify_runtime_role(&ctx.schema_name).await?;

        ctx.pools
            .verify_effective_privileges(&ctx.schema_name)
            .await?;

        Ok(())
    })
    .await?;
    Ok(())
}

/// The runtime pool must authenticate as `app_runtime`. Privilege reduction
/// reached through a reversible `SET ROLE` from an administrative login is not
/// a boundary, so `RESET ROLE` must not change the identity and `SET ROLE
/// postgres` must fail.
#[tokio::test]
async fn test_runtime_pool_identity_cannot_escalate() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let runtime_pool = ctx.pools.runtime_pool();

        let current: String = query_scalar("SELECT current_user")
            .fetch_one(runtime_pool)
            .await?;
        let session: String = query_scalar("SELECT session_user")
            .fetch_one(runtime_pool)
            .await?;

        assert_eq!(current, "app_runtime");
        assert_eq!(session, "app_runtime");

        // No role was assumed, so RESET ROLE is a no-op.
        query("RESET ROLE").execute(runtime_pool).await?;
        let after_reset: String = query_scalar("SELECT current_user")
            .fetch_one(runtime_pool)
            .await?;
        assert_eq!(after_reset, "app_runtime");

        // app_runtime is not a member of postgres and must not be able to
        // assume it. Run inside a transaction so a failed statement cannot
        // leave a pooled connection in an aborted state.
        let mut tx = runtime_pool.begin().await?;
        let escalation = query("SET ROLE postgres").execute(&mut *tx).await;
        tx.rollback().await?;

        assert!(
            escalation.is_err(),
            "runtime identity must not be able to SET ROLE postgres"
        );

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_catalog_rls_policy_metadata() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        ctx.pools
            .verify_part7_rls_catalog_metadata(&ctx.schema_name)
            .await?;

        Ok(())
    })
    .await?;
    Ok(())
}

// ============================================================================
// 2. POSITIVE ISOLATION TESTS & SYMMETRIC B OPERATIONS
// ============================================================================

#[tokio::test]
async fn test_positive_tenant_a_reads_a() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await?;

        assert_eq!(res.id, ctx.res_a1_id);
        assert_eq!(res.organization_id, ctx.org_a_id_str);
        assert_eq!(res.data, "Secret Data A1");

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_positive_tenant_b_reads_b() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await?;

        assert_eq!(res.id, ctx.res_b1_id);
        assert_eq!(res.organization_id, ctx.org_b_id_str);
        assert_eq!(res.data, "Secret Data B1");

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_positive_tenant_a_creates_a() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let new_res_id = format!("res_A_created_{}", uuid::Uuid::new_v4().simple());
        let new_resource = TenantResource {
            id: new_res_id.clone(),
            organization_id: ctx.org_a_id_str.clone(),
            branch_id: ctx.branch_a1_id_str.clone(),
            data: "Newly Created Data A".to_string(),
        };

        ctx.repo
            .create_tenant_resource(&ctx.org_a_scope, &new_resource)
            .await?;

        let fetched = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &new_res_id)
            .await?;

        assert_eq!(fetched.data, "Newly Created Data A");

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_positive_tenant_b_creates_b() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let new_res_id = format!("res_B_created_{}", uuid::Uuid::new_v4().simple());
        let new_resource = TenantResource {
            id: new_res_id.clone(),
            organization_id: ctx.org_b_id_str.clone(),
            branch_id: ctx.branch_b1_id_str.clone(),
            data: "Newly Created Data B".to_string(),
        };

        ctx.repo
            .create_tenant_resource(&ctx.org_b_scope, &new_resource)
            .await?;

        let fetched = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &new_res_id)
            .await?;

        assert_eq!(fetched.data, "Newly Created Data B");

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_positive_tenant_a_updates_a() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        ctx.repo
            .update_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id, "Updated Data A1")
            .await?;

        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await?;

        assert_eq!(res.data, "Updated Data A1");

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_positive_tenant_b_updates_b() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        ctx.repo
            .update_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id, "Updated Data B1")
            .await?;

        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await?;

        assert_eq!(res.data, "Updated Data B1");

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_positive_tenant_a_deletes_a() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        ctx.repo
            .delete_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await?;

        let fetch_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await;

        assert!(matches!(fetch_res, Err(PgAuthorityError::NotFoundOrDenied)));

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_positive_tenant_b_deletes_b() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        ctx.repo
            .delete_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await?;

        let fetch_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await;

        assert!(matches!(fetch_res, Err(PgAuthorityError::NotFoundOrDenied)));

        Ok(())
    })
    .await?;
    Ok(())
}

// ============================================================================
// 3. NEGATIVE CROSS-TENANT ISOLATION & WITH CHECK PROOFS
// ============================================================================

#[tokio::test]
async fn test_negative_tenant_a_cannot_read_b() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_b1_id)
            .await;

        assert!(
            matches!(res, Err(PgAuthorityError::NotFoundOrDenied)),
            "Tenant A must not read Tenant B resource"
        );

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_negative_unknown_resource_does_not_bypass_scope() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let unknown_res_id = format!("res_NONEXISTENT_{}", uuid::Uuid::new_v4().simple());
        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &unknown_res_id)
            .await;

        assert!(
            matches!(res, Err(PgAuthorityError::NotFoundOrDenied)),
            "Unknown resource query must return NotFoundOrDenied without revealing existence"
        );

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_negative_unknown_resource_update_fails_closed() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let unknown_res_id = format!("res_NONEXISTENT_{}", uuid::Uuid::new_v4().simple());
        let update_res = ctx
            .repo
            .update_tenant_resource(&ctx.org_a_scope, &unknown_res_id, "Hacked Data")
            .await;

        assert!(
            matches!(update_res, Err(PgAuthorityError::NotFoundOrDenied)),
            "Unknown resource update must return NotFoundOrDenied without mutating state"
        );

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_negative_unknown_resource_delete_fails_closed() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let unknown_res_id = format!("res_NONEXISTENT_{}", uuid::Uuid::new_v4().simple());
        let delete_res = ctx
            .repo
            .delete_tenant_resource(&ctx.org_a_scope, &unknown_res_id)
            .await;

        assert!(
            matches!(delete_res, Err(PgAuthorityError::NotFoundOrDenied)),
            "Unknown resource delete must return NotFoundOrDenied without mutating state"
        );

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_update_b() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let update_res = ctx
            .repo
            .update_tenant_resource(&ctx.org_a_scope, &ctx.res_b1_id, "Hacked Data")
            .await;

        assert!(
            matches!(update_res, Err(PgAuthorityError::NotFoundOrDenied)),
            "Tenant A must not update Tenant B resource"
        );

        let b_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await?;

        assert_eq!(b_res.data, "Secret Data B1");

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_delete_b() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let delete_res = ctx
            .repo
            .delete_tenant_resource(&ctx.org_a_scope, &ctx.res_b1_id)
            .await;

        assert!(
            matches!(delete_res, Err(PgAuthorityError::NotFoundOrDenied)),
            "Tenant A must not delete Tenant B resource"
        );

        let b_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await;

        assert!(b_res.is_ok());

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_insert_b_owned_row_relationally_valid() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let res_malicious_id = format!("res_malicious_{}", uuid::Uuid::new_v4().simple());

        let mut tx = ctx.repo.begin_tx(&ctx.org_a_scope).await?;

        let raw_res = query(
            "INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES ($1, $2, $3, $4)",
        )
        .bind(&res_malicious_id)
        .bind(&ctx.org_b_id_str)
        .bind(&ctx.branch_b1_id_str)
        .bind("Malicious Insert Data")
        .execute(&mut *tx)
        .await;

        assert!(raw_res.is_err(), "Raw INSERT must fail");
        let err = raw_res.err().ok_or_else(|| std::io::Error::other("RLS insert must be denied"))?;
        let pg_err = err.as_database_error().ok_or_else(|| std::io::Error::other("PostgreSQL error expected"))?;

        let code = pg_err.code().unwrap_or_default();
        assert!(
            code == "42501" || code == "44000",
            "Must be SQLSTATE 42501 (RLS policy violation) or 44000 from PostgreSQL RLS WITH CHECK, observed: {code}"
        );

        tx.rollback().await?;

        let fetch_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &res_malicious_id)
            .await;

        assert!(matches!(fetch_res, Err(PgAuthorityError::NotFoundOrDenied)));

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_negative_ownership_changing_update_relationally_valid() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let mut tx = ctx.repo.begin_tx(&ctx.org_a_scope).await?;

        let update_res = query(
            "UPDATE tenant_resources
             SET organization_id = $1, branch_id = $2
             WHERE id = $3",
        )
        .bind(&ctx.org_b_id_str)
        .bind(&ctx.branch_b1_id_str)
        .bind(&ctx.res_a1_id)
        .execute(&mut *tx)
        .await;

        assert!(update_res.is_err(), "Ownership update must fail");
        let err = update_res.err().ok_or_else(|| std::io::Error::other("RLS ownership update must be denied"))?;
        let pg_err = err.as_database_error().ok_or_else(|| std::io::Error::other("PostgreSQL error expected"))?;

        let code = pg_err.code().unwrap_or_default();
        assert!(
            code == "42501" || code == "44000",
            "Must be SQLSTATE 42501 (RLS policy violation) or 44000 from PostgreSQL RLS WITH CHECK, observed: {code}"
        );

        tx.rollback().await?;

        let fetch_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await
            ?;

        assert_eq!(fetch_res.organization_id, ctx.org_a_id_str);

        Ok(())
    })
    .await?;
    Ok(())
}

// ============================================================================
// 4. END-TO-END APPLICATION + DB COMPOSITION TEST
// ============================================================================

#[tokio::test]
async fn test_end_to_end_application_and_db_composition() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let initial_invocations = ctx.repo.invocation_count.load(Ordering::SeqCst);

        // 1. AUTHORIZED PATH via application orchestration seam
        let req_org_a = RequestedOrganizationId(OrganizationId::new(&ctx.org_a_id_str)?);
        let res_a = execute_application_tenant_operation(
            &ctx.mem_a,
            &ctx.org_a,
            &req_org_a,
            &ctx.repo,
            &ctx.res_a1_id,
        )
        .await
        ?;
        assert_eq!(res_a.data, "Secret Data A1");

        // 2. APPLICATION TAMPER PATH (Zero persistence invocation proof)
        let count_before_tamper = ctx.repo.invocation_count.load(Ordering::SeqCst);
        let req_org_b = RequestedOrganizationId(OrganizationId::new(&ctx.org_b_id_str)?);
        let tamper_res = execute_application_tenant_operation(
            &ctx.mem_a,
            &ctx.org_b,
            &req_org_b,
            &ctx.repo,
            &ctx.res_b1_id,
        )
        .await;

        assert!(
            tamper_res.is_err(),
            "Application layer must reject cross-tenant binding attempt before DB layer"
        );

        // Prove invocation count did NOT increase on application authorization rejection
        let count_after_tamper = ctx.repo.invocation_count.load(Ordering::SeqCst);
        assert_eq!(
            count_before_tamper, count_after_tamper,
            "Persistence layer MUST NOT be invoked when application authorization fails"
        );

        // 3. DATABASE INDEPENDENCE PATH
        let eff_scope_a = resolve_effective_scope(&ctx.mem_a, &ctx.org_a, None)?;
        let authorized_scope_a = AuthorizedScope::from_effective(&eff_scope_a);
        let mut tx = ctx.repo.begin_tx(&authorized_scope_a).await?;
        let row_b = query("SELECT id FROM tenant_resources WHERE id = $1")
            .bind(&ctx.res_b1_id)
            .fetch_optional(&mut *tx)
            .await
            ?;

        assert!(
            row_b.is_none(),
            "PostgreSQL RLS independently blocks Tenant B row even when raw query lacks WHERE organization_id predicate"
        );
        tx.commit().await?;

        assert!(
            ctx.repo.invocation_count.load(Ordering::SeqCst) > initial_invocations,
            "Invocation counter accurately recorded operations"
        );

        Ok(())
    })
    .await?;
    Ok(())
}

// ============================================================================
// 5. DIRECT DB BOUNDARY PROOF
// ============================================================================

#[tokio::test]
async fn test_direct_db_query_without_application_predicate() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let mut tx = ctx.repo.begin_tx(&ctx.org_a_scope).await?;

        let row = query("SELECT id, organization_id FROM tenant_resources WHERE id = $1")
            .bind(&ctx.res_b1_id)
            .fetch_optional(&mut *tx)
            .await
            ?;

        assert!(
            row.is_none(),
            "PostgreSQL RLS must independently hide res_B1 even when application query lacks a tenant predicate"
        );

        tx.commit().await?;

        Ok(())
    })
    .await?;
    Ok(())
}

// ============================================================================
// 6. BRANCH ISOLATION TESTS
// ============================================================================

#[tokio::test]
async fn test_branch_scoped_read_isolation() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let res1 = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_b1_scope, &ctx.res_a1_id)
            .await;
        assert!(res1.is_ok());

        let res2 = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_b1_scope, &ctx.res_a2_id)
            .await;
        assert!(matches!(res2, Err(PgAuthorityError::NotFoundOrDenied)));

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_cross_tenant_branch_binding_denial() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let invalid_resource = TenantResource {
            id: format!("res_cross_branch_{}", uuid::Uuid::new_v4().simple()),
            organization_id: ctx.org_b_id_str.clone(),
            branch_id: ctx.branch_a1_id_str.clone(),
            data: "Cross Branch Invalid".to_string(),
        };

        let mut tx = ctx.repo.begin_tx(&ctx.org_b_scope).await?;

        let insert_res = query(
            "INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES ($1, $2, $3, $4)",
        )
        .bind(&invalid_resource.id)
        .bind(&invalid_resource.organization_id)
        .bind(&invalid_resource.branch_id)
        .bind(&invalid_resource.data)
        .execute(&mut *tx)
        .await;

        assert!(insert_res.is_err(), "Cross branch insert must fail");
        let err = insert_res.err().ok_or_else(|| std::io::Error::other("invalid RLS insert must fail"))?;
        let pg_err = err.as_database_error().ok_or_else(|| std::io::Error::other("PostgreSQL error expected"))?;

        // Assert SQLSTATE 23503 (foreign_key_violation)
        assert_eq!(
            pg_err.code().unwrap_or_default(),
            "23503",
            "Must be SQLSTATE 23503 (foreign_key_violation) for invalid composite branch reference"
        );

        tx.rollback().await?;

        Ok(())
    })
    .await?;
    Ok(())
}

// ============================================================================
// 7. MISSING & INVALID CONTEXT TESTS
// ============================================================================

#[tokio::test]
async fn test_missing_tenant_context_fails_closed() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let mut tx = ctx.pools.runtime_pool().begin().await?;

        // 1. Read denial proof
        let count: i64 = query_scalar("SELECT COUNT(*) FROM tenant_resources")
            .fetch_one(&mut *tx)
            .await
            ?;

        assert_eq!(
            count, 0,
            "Unset / missing tenant context must return 0 rows (fail closed)"
        );

        // 2. Write denial proof
        let insert_res = query(
            "INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES ($1, $2, $3, $4)",
        )
        .bind("res_no_context")
        .bind(&ctx.org_a_id_str)
        .bind(&ctx.branch_a1_id_str)
        .bind("Data Without Context")
        .execute(&mut *tx)
        .await;

        assert!(
            insert_res.is_err(),
            "INSERT without tenant context must fail closed at RLS WITH CHECK"
        );
        let err = insert_res.err().ok_or_else(|| std::io::Error::other("invalid RLS insert must fail"))?;
        let pg_err = err.as_database_error().ok_or_else(|| std::io::Error::other("PostgreSQL error expected"))?;
        let code = pg_err.code().unwrap_or_default();
        assert!(
            code == "42501" || code == "44000",
            "Must be SQLSTATE 42501 or 44000 from PostgreSQL RLS WITH CHECK, observed: {code}"
        );

        tx.rollback().await?;

        Ok(())
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_invalid_tenant_context_fails_closed() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let req_org_nonexistent = RequestedOrganizationId(OrganizationId::new("org_NONEXISTENT")?);

        let bind_res = bind_organization(&req_org_nonexistent, &ctx.mem_a);
        assert!(bind_res.is_err(), "Binding nonexistent org must fail");

        let mut tx = ctx.pools.runtime_pool().begin().await?;
        query("SELECT set_config('app.organization_id', 'org_NONEXISTENT', true)")
            .execute(&mut *tx)
            .await?;

        let count: i64 = query_scalar("SELECT COUNT(*) FROM tenant_resources")
            .fetch_one(&mut *tx)
            .await?;

        assert_eq!(
            count, 0,
            "Nonexistent tenant GUC context must return 0 rows (fail closed)"
        );

        Ok(())
    })
    .await?;
    Ok(())
}

// ============================================================================
// 8. CONNECTION POOL LEAKAGE & ROLLBACK SAFETY
// ============================================================================

#[tokio::test]
async fn test_connection_pool_context_leakage_and_rollback_safety() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        for _ in 0..10 {
            {
                let mut tx = ctx.pools.runtime_pool().begin().await?;
                set_transaction_tenant_context(&mut tx, &ctx.org_a_scope).await?;

                let count: i64 =
                    query_scalar("SELECT COUNT(*) FROM tenant_resources WHERE id = $1")
                        .bind(&ctx.res_a1_id)
                        .fetch_one(&mut *tx)
                        .await?;

                assert_eq!(count, 1);
                tx.rollback().await?;
            }

            {
                let mut tx = ctx.pools.runtime_pool().begin().await?;
                set_transaction_tenant_context(&mut tx, &ctx.org_b_scope).await?;

                let count_b: i64 =
                    query_scalar("SELECT COUNT(*) FROM tenant_resources WHERE id = $1")
                        .bind(&ctx.res_b1_id)
                        .fetch_one(&mut *tx)
                        .await?;
                assert_eq!(count_b, 1);

                let count_a: i64 =
                    query_scalar("SELECT COUNT(*) FROM tenant_resources WHERE id = $1")
                        .bind(&ctx.res_a1_id)
                        .fetch_one(&mut *tx)
                        .await?;
                assert_eq!(
                    count_a, 0,
                    "Tenant A context must not leak into Tenant B transaction"
                );

                tx.commit().await?;
            }
        }

        Ok(())
    })
    .await?;
    Ok(())
}

// ============================================================================
// 9. CONCURRENT TENANT A & B READ & WRITE ISOLATION
// ============================================================================

#[tokio::test]
async fn test_concurrent_tenant_isolation_reads_and_writes() -> TestResult {
    run_test_with_teardown(|ctx| async move {
        let mut set = JoinSet::new();

        for i in 0..8 {
            let ctx_clone = Arc::clone(&ctx);
            set.spawn(async move {
                let item_id = format!("res_concurrent_{i}_{}", uuid::Uuid::new_v4().simple());
                if i % 2 == 0 {
                    let res_a = TenantResource {
                        id: item_id.clone(),
                        organization_id: ctx_clone.org_a_id_str.clone(),
                        branch_id: ctx_clone.branch_a1_id_str.clone(),
                        data: format!("Concurrent Data A {i}"),
                    };
                    ctx_clone
                        .repo
                        .create_tenant_resource(&ctx_clone.org_a_scope, &res_a)
                        .await?;

                    let read_a = ctx_clone
                        .repo
                        .get_tenant_resource(&ctx_clone.org_a_scope, &item_id)
                        .await;
                    assert!(read_a.is_ok());

                    let read_b_denied = ctx_clone
                        .repo
                        .get_tenant_resource(&ctx_clone.org_b_scope, &item_id)
                        .await;
                    assert!(matches!(
                        read_b_denied,
                        Err(PgAuthorityError::NotFoundOrDenied)
                    ));
                } else {
                    let res_b = TenantResource {
                        id: item_id.clone(),
                        organization_id: ctx_clone.org_b_id_str.clone(),
                        branch_id: ctx_clone.branch_b1_id_str.clone(),
                        data: format!("Concurrent Data B {i}"),
                    };
                    ctx_clone
                        .repo
                        .create_tenant_resource(&ctx_clone.org_b_scope, &res_b)
                        .await?;

                    let read_b = ctx_clone
                        .repo
                        .get_tenant_resource(&ctx_clone.org_b_scope, &item_id)
                        .await;
                    assert!(read_b.is_ok());

                    let read_a_denied = ctx_clone
                        .repo
                        .get_tenant_resource(&ctx_clone.org_a_scope, &item_id)
                        .await;
                    assert!(matches!(
                        read_a_denied,
                        Err(PgAuthorityError::NotFoundOrDenied)
                    ));
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    })
    .await?;
    Ok(())
}

// ============================================================================
// 10. SCHEMA ISOLATION & TEARDOWN REGRESSION ASSERTIONS
// ============================================================================

#[tokio::test]
async fn test_setup_failure_injection_cleans_up_schema() -> TestResult {
    let schema_id = uuid::Uuid::new_v4().simple().to_string();
    let schema_name = format!("test_schema_fail_inject_{schema_id}");

    let targets = common::targets_from_env().map_err(std::io::Error::other)?;
    let admin_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(targets.admin)
        .await?;

    // Verify schema does not exist prior to setup
    let exists_before: bool =
        query_scalar("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1)")
            .bind(&schema_name)
            .fetch_one(&admin_pool)
            .await?;
    assert!(!exists_before, "Schema must not exist before setup");

    // Execute real setup_test_context with INJECT_SETUP_FAILURE seam active
    let setup_res = INJECT_SETUP_FAILURE
        .scope(true, async { setup_test_context(Some(&schema_name)).await })
        .await;

    assert!(
        setup_res.is_err(),
        "Failing setup context must return setup error"
    );

    // Prove schema no longer exists in PostgreSQL catalog after setup_test_context error handling
    let exists_after: bool =
        query_scalar("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1)")
            .bind(&schema_name)
            .fetch_one(&admin_pool)
            .await?;
    assert!(
        !exists_after,
        "Schema must be CASCADE dropped by setup_test_context error handling"
    );
    Ok(())
}
