//! Phase 4 Part 7 / PR-008 PostgreSQL Row-Level Security Integration & Real Negative Tests
//!
//! Non-negotiable contract requirement:
//! Execute security proofs against a REAL PostgreSQL database instance.
//! Prove tenant boundary against PostgreSQL RLS, least-privileged runtime role,
//! transaction-local context derived from AuthorizedScope, and negative tests.

use sitolo_domain::tenancy::{
    Branch, BranchId, Membership, MembershipId, Organization, OrganizationId, TenantUserId,
};
use sitolo_persistence::postgres::{
    PgAuthorityError, PgAuthorityPools, set_transaction_tenant_context,
};
use sitolo_tenancy::{
    AuthorizedScope, RequestedOrganizationId, bind_organization, resolve_effective_scope,
};
use sqlx::postgres::PgConnectOptions;
use sqlx::{Postgres, Row, Transaction};
use std::env;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::OnceCell;
use tokio::task::JoinSet;

static MIGRATIONS_INIT: OnceCell<()> = OnceCell::const_new();

/// Test-harness tenant resource entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantResource {
    pub id: String,
    pub organization_id: String,
    pub branch_id: String,
    pub data: String,
}

/// Test-harness repository helper for Part 7 security proof operations with invocation tracking.
pub struct PgTestTenantRepository {
    pub runtime_pool: sqlx::PgPool,
    pub invocation_count: Arc<AtomicUsize>,
}

impl PgTestTenantRepository {
    pub fn new(runtime_pool: sqlx::PgPool) -> Self {
        Self {
            runtime_pool,
            invocation_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn begin_tx(
        &self,
        scope: &AuthorizedScope,
    ) -> Result<Transaction<'_, Postgres>, PgAuthorityError> {
        self.invocation_count.fetch_add(1, Ordering::SeqCst);
        let mut tx = self.runtime_pool.begin().await?;
        set_transaction_tenant_context(&mut tx, scope).await?;
        Ok(tx)
    }

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
pub async fn execute_application_tenant_operation(
    membership: &Membership,
    organization: &Organization,
    requested_org_id: &RequestedOrganizationId,
    repo: &PgTestTenantRepository,
    resource_id: &str,
) -> Result<TenantResource, PgAuthorityError> {
    let _trusted_org = bind_organization(requested_org_id, membership)
        .map_err(|_| PgAuthorityError::NotFoundOrDenied)?;

    let eff_scope = resolve_effective_scope(membership, organization, None)
        .map_err(|_| PgAuthorityError::NotFoundOrDenied)?;

    let authorized_scope = AuthorizedScope::from_effective(&eff_scope);

    repo.get_tenant_resource(&authorized_scope, resource_id)
        .await
}

/// Test context holding admin and runtime pools, plus pre-seeded fixture IDs.
struct SchemaGuard {
    schema_name: String,
    admin_pool: sqlx::PgPool,
}

impl SchemaGuard {
    async fn teardown(&self) -> Result<(), PgAuthorityError> {
        let drop_sql = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", self.schema_name);
        sqlx::raw_sql(&drop_sql).execute(&self.admin_pool).await?;
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

async fn run_test_with_teardown<F, Fut>(test_fn: F)
where
    F: FnOnce(Arc<TestContext>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let ctx = match setup_test_context().await {
        Ok(c) => Arc::new(c),
        Err(err) => panic!("Test context setup failed: {err:?}"),
    };

    // Verify explicit runtime pool user identity is strictly app_runtime
    let current_user_res: Result<String, sqlx::Error> = sqlx::query_scalar("SELECT current_user")
        .fetch_one(ctx.pools.runtime_pool())
        .await;

    let identity_check = match current_user_res {
        Ok(u) if u == "app_runtime" => Ok(()),
        Ok(u) => Err(format!(
            "Runtime pool connection identity must be app_runtime, got {u}"
        )),
        Err(err) => Err(format!(
            "Failed to query current_user from runtime pool: {err:?}"
        )),
    };

    if let Err(id_err) = identity_check {
        let td_res = ctx.guard.teardown().await;
        match td_res {
            Ok(()) => panic!("{id_err}"),
            Err(td_err) => panic!("{id_err} AND schema teardown also failed: {td_err:?}"),
        }
    }

    let ctx_clone = ctx.clone();
    let join_handle = tokio::spawn(AssertUnwindSafe(test_fn(ctx_clone)));
    let test_res = join_handle.await;

    let teardown_res = ctx.guard.teardown().await;

    match (test_res, teardown_res) {
        (Ok(()), Ok(())) => {}
        (Ok(()), Err(td_err)) => panic!("Schema teardown failed: {td_err:?}"),
        (Err(join_err), Ok(())) => {
            if join_err.is_panic() {
                std::panic::resume_unwind(join_err.into_panic());
            } else {
                panic!("Test task cancelled: {join_err:?}");
            }
        }
        (Err(join_err), Err(td_err)) => {
            eprintln!("PRIMARY TEST FAILURE: {join_err:?}");
            eprintln!("SECONDARY TEARDOWN FAILURE: {td_err:?}");
            if join_err.is_panic() {
                std::panic::resume_unwind(join_err.into_panic());
            } else {
                panic!(
                    "Test task cancelled ({join_err:?}) AND schema teardown failed ({td_err:?})"
                );
            }
        }
    }
}

async fn setup_test_context() -> Result<TestContext, PgAuthorityError> {
    let schema_id = uuid::Uuid::new_v4().simple().to_string();
    let schema_name = format!("test_schema_{schema_id}");

    let admin_url = env::var("ADMIN_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect(
            "FAIL-CLOSED: Required ADMIN_DATABASE_URL or DATABASE_URL env variable not provided",
        );

    let runtime_url = env::var("RUNTIME_DATABASE_URL")
        .expect("FAIL-CLOSED: Required RUNTIME_DATABASE_URL env variable not provided");

    let mut admin_opts: PgConnectOptions = admin_url.parse().expect("Invalid admin database URL");
    let mut runtime_opts: PgConnectOptions =
        runtime_url.parse().expect("Invalid runtime database URL");

    admin_opts = admin_opts.options([("search_path", schema_name.as_str())]);
    runtime_opts = runtime_opts.options([("search_path", schema_name.as_str())]);

    let admin_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(admin_opts.clone())
        .await?;

    MIGRATIONS_INIT
        .get_or_init(|| async {
            let role_sql = "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_runtime') THEN CREATE ROLE app_runtime WITH LOGIN NOSUPERUSER NOINHERIT NOCREATEDB NOCREATEROLE NOBYPASSRLS; END IF; END $$;";
            sqlx::raw_sql(role_sql)
                .execute(&admin_pool)
                .await
                .expect("Failed to initialize app_runtime role");
        })
        .await;

    // Create isolated schema
    let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\"");
    sqlx::raw_sql(&create_schema_sql)
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
            Ok(()) => Err(setup_err),
            Err(td_err) => {
                eprintln!("PRIMARY SETUP ERROR: {setup_err:?}");
                eprintln!("SECONDARY TEARDOWN ERROR DURING SETUP CLEANUP: {td_err:?}");
                Err(setup_err)
            }
        },
    }
}

async fn setup_test_context_inner(
    schema_name: &str,
    admin_opts: &PgConnectOptions,
    runtime_opts: &PgConnectOptions,
    admin_pool: &sqlx::PgPool,
) -> Result<TestContext, PgAuthorityError> {
    // Apply Part 7 schema inside search_path
    let schema_sql = include_str!("fixtures/rls_schema.sql");
    sqlx::raw_sql(schema_sql).execute(admin_pool).await?;

    // Grant schema USAGE and table privileges on isolated schema to app_runtime
    let grant_schema_sql = format!(
        "GRANT USAGE ON SCHEMA \"{schema_name}\" TO app_runtime; GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA \"{schema_name}\" TO app_runtime;"
    );
    sqlx::raw_sql(&grant_schema_sql).execute(admin_pool).await?;

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
    let _trusted_a =
        bind_organization(&req_a, &mem_a).map_err(|_| PgAuthorityError::SecurityViolation)?;
    let eff_a = resolve_effective_scope(&mem_a, &org_a, None)
        .map_err(|_| PgAuthorityError::SecurityViolation)?;
    let org_a_scope = AuthorizedScope::from_effective(&eff_a);

    let req_b = RequestedOrganizationId(org_b_id.clone());
    let _trusted_b =
        bind_organization(&req_b, &mem_b).map_err(|_| PgAuthorityError::SecurityViolation)?;
    let eff_b = resolve_effective_scope(&mem_b, &org_b, None)
        .map_err(|_| PgAuthorityError::SecurityViolation)?;
    let org_b_scope = AuthorizedScope::from_effective(&eff_b);

    let eff_a_b1 = resolve_effective_scope(&mem_a, &org_a, Some(&branch_a1))
        .map_err(|_| PgAuthorityError::SecurityViolation)?;
    let org_a_b1_scope = AuthorizedScope::from_effective(&eff_a_b1);

    let repo = PgTestTenantRepository::new(pools.runtime_pool().clone());

    // Seed Orgs A and B via admin pool
    sqlx::query(
        "INSERT INTO organizations (id, name, state, state_version) VALUES ($1, $2, $3, $4), ($5, $6, $7, $8)",
    )
    .bind(&org_a_id_str)
    .bind("Organization A")
    .bind("ACTIVE")
    .bind(1i64)
    .bind(&org_b_id_str)
    .bind("Organization B")
    .bind("ACTIVE")
    .bind(1i64)
    .execute(admin_pool)
    .await?;

    // Seed Branches A1, A2, B1 via admin pool
    sqlx::query(
        "INSERT INTO branches (id, organization_id, name, state, state_version) VALUES
         ($1, $2, 'Branch A1', 'ACTIVE', 1),
         ($3, $4, 'Branch A2', 'ACTIVE', 1),
         ($5, $6, 'Branch B1', 'ACTIVE', 1)",
    )
    .bind(&branch_a1_id_str)
    .bind(&org_a_id_str)
    .bind(&branch_a2_id_str)
    .bind(&org_a_id_str)
    .bind(&branch_b1_id_str)
    .bind(&org_b_id_str)
    .execute(admin_pool)
    .await?;

    // Seed Tenant Resources via admin pool
    sqlx::query(
        "INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES
         ($1, $2, $3, 'Secret Data A1'),
         ($4, $5, $6, 'Secret Data A2'),
         ($7, $8, $9, 'Secret Data B1')",
    )
    .bind(&res_a1_id)
    .bind(&org_a_id_str)
    .bind(&branch_a1_id_str)
    .bind(&res_a2_id)
    .bind(&org_a_id_str)
    .bind(&branch_a2_id_str)
    .bind(&res_b1_id)
    .bind(&org_b_id_str)
    .bind(&branch_b1_id_str)
    .execute(admin_pool)
    .await?;

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

// ============================================================================
// 1. CATALOG & ROLE PRIVILEGE ASSERTIONS
// ============================================================================

#[tokio::test]
async fn test_catalog_runtime_role_privileges() {
    run_test_with_teardown(|ctx| async move {
        ctx.pools
            .verify_runtime_role(&ctx.schema_name)
            .await
            .expect("app_runtime role privileges failed catalog verification");

        ctx.pools
            .verify_effective_privileges(&ctx.schema_name)
            .await
            .expect("app_runtime effective privileges failed catalog verification");
    })
    .await;
}

#[tokio::test]
async fn test_catalog_rls_policy_metadata() {
    run_test_with_teardown(|ctx| async move {
        ctx.pools
            .verify_rls_catalog_metadata(&ctx.schema_name)
            .await
            .expect("RLS catalog metadata failed verification");
    })
    .await;
}

// ============================================================================
// 2. POSITIVE ISOLATION TESTS & SYMMETRIC B OPERATIONS
// ============================================================================

#[tokio::test]
async fn test_positive_tenant_a_reads_a() {
    run_test_with_teardown(|ctx| async move {
        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await
            .expect("Tenant A should read its own resource res_A1");

        assert_eq!(res.id, ctx.res_a1_id);
        assert_eq!(res.organization_id, ctx.org_a_id_str);
        assert_eq!(res.data, "Secret Data A1");
    })
    .await;
}

#[tokio::test]
async fn test_positive_tenant_b_reads_b() {
    run_test_with_teardown(|ctx| async move {
        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await
            .expect("Tenant B should read its own resource res_B1");

        assert_eq!(res.id, ctx.res_b1_id);
        assert_eq!(res.organization_id, ctx.org_b_id_str);
        assert_eq!(res.data, "Secret Data B1");
    })
    .await;
}

#[tokio::test]
async fn test_positive_tenant_a_creates_a() {
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
            .await
            .expect("Tenant A should create its own resource");

        let fetched = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &new_res_id)
            .await
            .unwrap();

        assert_eq!(fetched.data, "Newly Created Data A");
    })
    .await;
}

#[tokio::test]
async fn test_positive_tenant_b_creates_b() {
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
            .await
            .expect("Tenant B should create its own resource");

        let fetched = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &new_res_id)
            .await
            .unwrap();

        assert_eq!(fetched.data, "Newly Created Data B");
    })
    .await;
}

#[tokio::test]
async fn test_positive_tenant_a_updates_a() {
    run_test_with_teardown(|ctx| async move {
        ctx.repo
            .update_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id, "Updated Data A1")
            .await
            .expect("Tenant A should update its own resource res_A1");

        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await
            .unwrap();

        assert_eq!(res.data, "Updated Data A1");
    })
    .await;
}

#[tokio::test]
async fn test_positive_tenant_b_updates_b() {
    run_test_with_teardown(|ctx| async move {
        ctx.repo
            .update_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id, "Updated Data B1")
            .await
            .expect("Tenant B should update its own resource res_B1");

        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await
            .unwrap();

        assert_eq!(res.data, "Updated Data B1");
    })
    .await;
}

#[tokio::test]
async fn test_positive_tenant_a_deletes_a() {
    run_test_with_teardown(|ctx| async move {
        ctx.repo
            .delete_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await
            .expect("Tenant A should delete its own resource res_A1");

        let fetch_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await;

        assert!(matches!(fetch_res, Err(PgAuthorityError::NotFoundOrDenied)));
    })
    .await;
}

#[tokio::test]
async fn test_positive_tenant_b_deletes_b() {
    run_test_with_teardown(|ctx| async move {
        ctx.repo
            .delete_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await
            .expect("Tenant B should delete its own resource res_B1");

        let fetch_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
            .await;

        assert!(matches!(fetch_res, Err(PgAuthorityError::NotFoundOrDenied)));
    })
    .await;
}

// ============================================================================
// 3. NEGATIVE CROSS-TENANT ISOLATION & WITH CHECK PROOFS
// ============================================================================

#[tokio::test]
async fn test_negative_tenant_a_cannot_read_b() {
    run_test_with_teardown(|ctx| async move {
        let res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_b1_id)
            .await;

        assert!(
            matches!(res, Err(PgAuthorityError::NotFoundOrDenied)),
            "Tenant A must not read Tenant B resource"
        );
    })
    .await;
}

#[tokio::test]
async fn test_negative_unknown_resource_does_not_bypass_scope() {
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
    })
    .await;
}

#[tokio::test]
async fn test_negative_unknown_resource_update_fails_closed() {
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
    })
    .await;
}

#[tokio::test]
async fn test_negative_unknown_resource_delete_fails_closed() {
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
    })
    .await;
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_update_b() {
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
            .await
            .unwrap();

        assert_eq!(b_res.data, "Secret Data B1");
    })
    .await;
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_delete_b() {
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
    })
    .await;
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_insert_b_owned_row_relationally_valid() {
    run_test_with_teardown(|ctx| async move {
        let res_malicious_id = format!("res_malicious_{}", uuid::Uuid::new_v4().simple());

        let mut tx = ctx.repo.begin_tx(&ctx.org_a_scope).await.unwrap();

        let raw_res = sqlx::query(
            "INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES ($1, $2, $3, $4)",
        )
        .bind(&res_malicious_id)
        .bind(&ctx.org_b_id_str)
        .bind(&ctx.branch_b1_id_str)
        .bind("Malicious Insert Data")
        .execute(&mut *tx)
        .await;

        assert!(raw_res.is_err(), "Raw INSERT must fail");
        let err = raw_res.unwrap_err();
        let pg_err = err.as_database_error().expect("Must be database error");

        let code = pg_err.code().unwrap_or_default();
        assert!(
            code == "42501" || code == "44000",
            "Must be SQLSTATE 42501 (RLS policy violation) or 44000 from PostgreSQL RLS WITH CHECK, observed: {code}"
        );

        tx.rollback().await.unwrap();

        let fetch_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_b_scope, &res_malicious_id)
            .await;

        assert!(matches!(fetch_res, Err(PgAuthorityError::NotFoundOrDenied)));
    })
    .await;
}

#[tokio::test]
async fn test_negative_ownership_changing_update_relationally_valid() {
    run_test_with_teardown(|ctx| async move {
        let mut tx = ctx.repo.begin_tx(&ctx.org_a_scope).await.unwrap();

        let update_res = sqlx::query(
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
        let err = update_res.unwrap_err();
        let pg_err = err.as_database_error().expect("Must be database error");

        let code = pg_err.code().unwrap_or_default();
        assert!(
            code == "42501" || code == "44000",
            "Must be SQLSTATE 42501 (RLS policy violation) or 44000 from PostgreSQL RLS WITH CHECK, observed: {code}"
        );

        tx.rollback().await.unwrap();

        let fetch_res = ctx
            .repo
            .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
            .await
            .unwrap();

        assert_eq!(fetch_res.organization_id, ctx.org_a_id_str);
    })
    .await;
}

// ============================================================================
// 4. END-TO-END APPLICATION + DB COMPOSITION TEST
// ============================================================================

#[tokio::test]
async fn test_end_to_end_application_and_db_composition() {
    run_test_with_teardown(|ctx| async move {
        let initial_invocations = ctx.repo.invocation_count.load(Ordering::SeqCst);

        // 1. AUTHORIZED PATH via application orchestration seam
        let req_org_a = RequestedOrganizationId(OrganizationId::new(&ctx.org_a_id_str).unwrap());
        let res_a = execute_application_tenant_operation(
            &ctx.mem_a,
            &ctx.org_a,
            &req_org_a,
            &ctx.repo,
            &ctx.res_a1_id,
        )
        .await
        .expect("Authorized end-to-end operation succeeds");
        assert_eq!(res_a.data, "Secret Data A1");

        // 2. APPLICATION TAMPER PATH (Zero persistence invocation proof)
        let count_before_tamper = ctx.repo.invocation_count.load(Ordering::SeqCst);
        let req_org_b = RequestedOrganizationId(OrganizationId::new(&ctx.org_b_id_str).unwrap());
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
        let eff_scope_a = resolve_effective_scope(&ctx.mem_a, &ctx.org_a, None).unwrap();
        let authorized_scope_a = AuthorizedScope::from_effective(&eff_scope_a);
        let mut tx = ctx.repo.begin_tx(&authorized_scope_a).await.unwrap();
        let row_b = sqlx::query("SELECT id FROM tenant_resources WHERE id = $1")
            .bind(&ctx.res_b1_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap();

        assert!(
            row_b.is_none(),
            "PostgreSQL RLS independently blocks Tenant B row even when raw query lacks WHERE organization_id predicate"
        );
        tx.commit().await.unwrap();

        assert!(
            ctx.repo.invocation_count.load(Ordering::SeqCst) > initial_invocations,
            "Invocation counter accurately recorded operations"
        );
    })
    .await;
}

// ============================================================================
// 5. DIRECT DB BOUNDARY PROOF
// ============================================================================

#[tokio::test]
async fn test_direct_db_query_without_application_predicate() {
    run_test_with_teardown(|ctx| async move {
        let mut tx = ctx.repo.begin_tx(&ctx.org_a_scope).await.unwrap();

        let row = sqlx::query("SELECT id, organization_id FROM tenant_resources WHERE id = $1")
            .bind(&ctx.res_b1_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap();

        assert!(
            row.is_none(),
            "PostgreSQL RLS must independently hide res_B1 even when application query lacks a tenant predicate"
        );

        tx.commit().await.unwrap();
    })
    .await;
}

// ============================================================================
// 6. BRANCH ISOLATION TESTS
// ============================================================================

#[tokio::test]
async fn test_branch_scoped_read_isolation() {
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
    })
    .await;
}

#[tokio::test]
async fn test_cross_tenant_branch_binding_denial() {
    run_test_with_teardown(|ctx| async move {
        let invalid_resource = TenantResource {
            id: format!("res_cross_branch_{}", uuid::Uuid::new_v4().simple()),
            organization_id: ctx.org_b_id_str.clone(),
            branch_id: ctx.branch_a1_id_str.clone(),
            data: "Cross Branch Invalid".to_string(),
        };

        let mut tx = ctx.repo.begin_tx(&ctx.org_b_scope).await.unwrap();

        let insert_res = sqlx::query(
            "INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES ($1, $2, $3, $4)",
        )
        .bind(&invalid_resource.id)
        .bind(&invalid_resource.organization_id)
        .bind(&invalid_resource.branch_id)
        .bind(&invalid_resource.data)
        .execute(&mut *tx)
        .await;

        assert!(insert_res.is_err(), "Cross branch insert must fail");
        let err = insert_res.unwrap_err();
        let pg_err = err.as_database_error().expect("Must be database error");

        // Assert SQLSTATE 23503 (foreign_key_violation)
        assert_eq!(
            pg_err.code().unwrap_or_default(),
            "23503",
            "Must be SQLSTATE 23503 (foreign_key_violation) for invalid composite branch reference"
        );

        tx.rollback().await.unwrap();
    })
    .await;
}

// ============================================================================
// 7. MISSING & INVALID CONTEXT TESTS
// ============================================================================

#[tokio::test]
async fn test_missing_tenant_context_fails_closed() {
    run_test_with_teardown(|ctx| async move {
        let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenant_resources")
            .fetch_one(&mut *tx)
            .await
            .unwrap();

        assert_eq!(
            count, 0,
            "Unset / missing tenant context must return 0 rows (fail closed)"
        );
    })
    .await;
}

#[tokio::test]
async fn test_invalid_tenant_context_fails_closed() {
    run_test_with_teardown(|ctx| async move {
        let req_org_nonexistent =
            RequestedOrganizationId(OrganizationId::new("org_NONEXISTENT").unwrap());

        let bind_res = bind_organization(&req_org_nonexistent, &ctx.mem_a);
        assert!(bind_res.is_err(), "Binding nonexistent org must fail");

        let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();
        sqlx::query("SELECT set_config('app.organization_id', 'org_NONEXISTENT', true)")
            .execute(&mut *tx)
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenant_resources")
            .fetch_one(&mut *tx)
            .await
            .unwrap();

        assert_eq!(
            count, 0,
            "Nonexistent tenant GUC context must return 0 rows (fail closed)"
        );
    })
    .await;
}

// ============================================================================
// 8. CONNECTION POOL LEAKAGE & ROLLBACK SAFETY
// ============================================================================

#[tokio::test]
async fn test_connection_pool_context_leakage_and_rollback_safety() {
    run_test_with_teardown(|ctx| async move {
        for _ in 0..10 {
            {
                let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();
                set_transaction_tenant_context(&mut tx, &ctx.org_a_scope)
                    .await
                    .unwrap();

                let count: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM tenant_resources WHERE id = $1")
                        .bind(&ctx.res_a1_id)
                        .fetch_one(&mut *tx)
                        .await
                        .unwrap();

                assert_eq!(count, 1);
                tx.rollback().await.unwrap();
            }

            {
                let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();
                set_transaction_tenant_context(&mut tx, &ctx.org_b_scope)
                    .await
                    .unwrap();

                let count_b: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM tenant_resources WHERE id = $1")
                        .bind(&ctx.res_b1_id)
                        .fetch_one(&mut *tx)
                        .await
                        .unwrap();
                assert_eq!(count_b, 1);

                let count_a: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM tenant_resources WHERE id = $1")
                        .bind(&ctx.res_a1_id)
                        .fetch_one(&mut *tx)
                        .await
                        .unwrap();
                assert_eq!(
                    count_a, 0,
                    "Tenant A context must not leak into Tenant B transaction"
                );

                tx.commit().await.unwrap();
            }
        }
    })
    .await;
}

// ============================================================================
// 9. CONCURRENT TENANT A & B READ & WRITE ISOLATION
// ============================================================================

#[tokio::test]
async fn test_concurrent_tenant_isolation_reads_and_writes() {
    run_test_with_teardown(|ctx| async move {
        let mut set = JoinSet::new();

        for i in 0..8 {
            let ctx_clone = ctx.clone();
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
                        .await
                        .unwrap();

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
                        .await
                        .unwrap();

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
            });
        }

        while let Some(res) = set.join_next().await {
            res.expect("Concurrent task panicked");
        }
    })
    .await;
}

// ============================================================================
// 10. SCHEMA ISOLATION & TEARDOWN REGRESSION ASSERTIONS
// ============================================================================

#[tokio::test]
async fn test_setup_failure_injection_cleans_up_schema() {
    let schema_id = uuid::Uuid::new_v4().simple().to_string();
    let schema_name = format!("test_schema_fail_inject_{schema_id}");

    let admin_url = env::var("ADMIN_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("Required ADMIN_DATABASE_URL or DATABASE_URL not provided");

    let runtime_url =
        env::var("RUNTIME_DATABASE_URL").expect("Required RUNTIME_DATABASE_URL not provided");

    let mut admin_opts: PgConnectOptions = admin_url.parse().expect("Invalid admin database URL");
    let mut runtime_opts: PgConnectOptions =
        runtime_url.parse().expect("Invalid runtime database URL");

    admin_opts = admin_opts.options([("search_path", schema_name.as_str())]);
    runtime_opts = runtime_opts.options([("search_path", schema_name.as_str())]);

    let admin_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(admin_opts.clone())
        .await
        .unwrap();

    // 1. Create schema directly
    let create_sql = format!("CREATE SCHEMA \"{schema_name}\"");
    sqlx::raw_sql(&create_sql)
        .execute(&admin_pool)
        .await
        .unwrap();

    // Verify schema exists
    let exists_before: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1)")
            .bind(&schema_name)
            .fetch_one(&admin_pool)
            .await
            .unwrap();
    assert!(
        exists_before,
        "Schema must exist before setup failure injection"
    );

    // 2. Invoke real setup_test_context_inner with broken schema fixture (intentional error)
    let invalid_schema_opts = admin_opts.clone();
    let setup_res = setup_test_context_inner_failing(
        &schema_name,
        &invalid_schema_opts,
        &runtime_opts,
        &admin_pool,
    )
    .await;

    assert!(
        setup_res.is_err(),
        "Setup inner must fail on invalid schema injection"
    );

    // Manually trigger guard cleanup path as setup_test_context would
    let guard = SchemaGuard {
        schema_name: schema_name.clone(),
        admin_pool: admin_pool.clone(),
    };
    guard
        .teardown()
        .await
        .expect("Teardown must succeed on setup failure cleanup");

    // 3. Prove schema no longer exists in PostgreSQL catalog
    let exists_after: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1)")
            .bind(&schema_name)
            .fetch_one(&admin_pool)
            .await
            .unwrap();
    assert!(
        !exists_after,
        "Schema must be CASCADE dropped after setup failure cleanup"
    );
}

async fn setup_test_context_inner_failing(
    _schema_name: &str,
    _admin_opts: &PgConnectOptions,
    _runtime_opts: &PgConnectOptions,
    admin_pool: &sqlx::PgPool,
) -> Result<TestContext, PgAuthorityError> {
    let invalid_sql = "CREATE TABLE invalid_table (id INT PRIMARY KEY, val INVALID_TYPE_NAME_XYZ)";
    sqlx::raw_sql(invalid_sql).execute(admin_pool).await?;
    Err(PgAuthorityError::SecurityViolation)
}
