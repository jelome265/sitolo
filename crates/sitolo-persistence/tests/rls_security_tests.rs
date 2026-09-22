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
use std::sync::Arc;
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

/// Test-harness repository helper for Part 7 security proof operations.
pub struct PgTestTenantRepository {
    pub runtime_pool: sqlx::PgPool,
}

impl PgTestTenantRepository {
    pub fn new(runtime_pool: sqlx::PgPool) -> Self {
        Self { runtime_pool }
    }

    pub async fn begin_tx(
        &self,
        scope: &AuthorizedScope,
    ) -> Result<Transaction<'_, Postgres>, PgAuthorityError> {
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

/// Test context holding admin and runtime pools, plus pre-seeded fixture IDs.
struct SchemaGuard {
    schema_name: String,
    admin_pool: sqlx::PgPool,
}

impl SchemaGuard {
    async fn teardown(&self) {
        let drop_sql = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", self.schema_name);
        let _ = sqlx::raw_sql(&drop_sql).execute(&self.admin_pool).await;
    }
}

struct TestContext {
    pools: PgAuthorityPools,
    repo: PgTestTenantRepository,
    guard: SchemaGuard,
    schema_name: String,
    org_a: Organization,
    mem_a: Membership,
    org_a_scope: AuthorizedScope,
    org_b_scope: AuthorizedScope,
    org_a_b1_scope: AuthorizedScope,
    res_a1_id: String,
    res_a2_id: String,
    res_b1_id: String,
    org_a_id_str: String,
    org_b_id_str: String,
    branch_a1_id_str: String,
    branch_b1_id_str: String,
}

async fn setup_test_context() -> TestContext {
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
        .await
        .expect("FAIL-CLOSED: Real PostgreSQL database connection failed. Tests cannot run without PostgreSQL.");

    MIGRATIONS_INIT
        .get_or_init(|| async {
            if let Ok(runtime_pass) = env::var("APP_RUNTIME_PASSWORD") {
                let alter_sql = format!("ALTER ROLE app_runtime WITH PASSWORD '{runtime_pass}';");
                sqlx::raw_sql(&alter_sql).execute(&admin_pool).await.ok();
            }
        })
        .await;

    // Create isolated schema and set search path
    let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\"");
    sqlx::raw_sql(&create_schema_sql)
        .execute(&admin_pool)
        .await
        .expect("Failed to create isolated test schema");

    // Apply Part 7 schema inside search_path
    let schema_sql = include_str!("fixtures/rls_schema.sql");
    sqlx::raw_sql(schema_sql)
        .execute(&admin_pool)
        .await
        .expect("Failed to apply RLS fixture schema inside search_path");

    // Grant schema USAGE on isolated schema to app_runtime
    let grant_schema_sql = format!(
        "GRANT USAGE ON SCHEMA \"{schema_name}\" TO app_runtime; GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA \"{schema_name}\" TO app_runtime;"
    );
    sqlx::raw_sql(&grant_schema_sql)
        .execute(&admin_pool)
        .await
        .expect("Failed to grant schema privileges to app_runtime");

    let guard = SchemaGuard {
        schema_name: schema_name.clone(),
        admin_pool: admin_pool.clone(),
    };

    let pools = PgAuthorityPools::connect_options(admin_opts, runtime_opts)
        .await
        .expect("Failed to create authority pools");

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
    let org_a_id = OrganizationId::new(&org_a_id_str).unwrap();
    let org_b_id = OrganizationId::new(&org_b_id_str).unwrap();

    let mut org_a = Organization::provision(org_a_id.clone(), "Organization A").unwrap();
    org_a.activate().unwrap();

    let mut org_b = Organization::provision(org_b_id.clone(), "Organization B").unwrap();
    org_b.activate().unwrap();

    let branch_a1_id = BranchId::new(&branch_a1_id_str).unwrap();
    let branch_a2_id = BranchId::new(&branch_a2_id_str).unwrap();
    let branch_b1_id = BranchId::new(&branch_b1_id_str).unwrap();

    let mut branch_a1 =
        Branch::provision(branch_a1_id.clone(), org_a_id.clone(), "Branch A1").unwrap();
    branch_a1.activate().unwrap();

    let mut branch_a2 =
        Branch::provision(branch_a2_id.clone(), org_a_id.clone(), "Branch A2").unwrap();
    branch_a2.activate().unwrap();

    let mut branch_b1 =
        Branch::provision(branch_b1_id.clone(), org_b_id.clone(), "Branch B1").unwrap();
    branch_b1.activate().unwrap();

    let mem_a_id = MembershipId::new(format!("mem_A_{unique_id}")).unwrap();
    let mem_b_id = MembershipId::new(format!("mem_B_{unique_id}")).unwrap();
    let user_a_id = TenantUserId::new(format!("user_A_{unique_id}")).unwrap();
    let user_b_id = TenantUserId::new(format!("user_B_{unique_id}")).unwrap();

    let mut mem_a = Membership::invite(mem_a_id.clone(), org_a_id.clone(), user_a_id);
    mem_a.mark_pending().unwrap();
    mem_a.activate().unwrap();

    let mut mem_b = Membership::invite(mem_b_id.clone(), org_b_id.clone(), user_b_id);
    mem_b.mark_pending().unwrap();
    mem_b.activate().unwrap();

    // Derive AuthorizedScope via full Phase 4 scope resolution pipeline
    let req_a = RequestedOrganizationId(org_a_id.clone());
    let _trusted_a = bind_organization(&req_a, &mem_a).unwrap();
    let eff_a = resolve_effective_scope(&mem_a, &org_a, None).unwrap();
    let org_a_scope = AuthorizedScope::from_effective(&eff_a);

    let req_b = RequestedOrganizationId(org_b_id.clone());
    let _trusted_b = bind_organization(&req_b, &mem_b).unwrap();
    let eff_b = resolve_effective_scope(&mem_b, &org_b, None).unwrap();
    let org_b_scope = AuthorizedScope::from_effective(&eff_b);

    let eff_a_b1 = resolve_effective_scope(&mem_a, &org_a, Some(&branch_a1)).unwrap();
    let org_a_b1_scope = AuthorizedScope::from_effective(&eff_a_b1);

    let repo = PgTestTenantRepository::new(pools.runtime_pool.clone());

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
    .execute(&pools.admin_pool)
    .await
    .unwrap();

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
    .execute(&pools.admin_pool)
    .await
    .unwrap();

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
    .execute(&pools.admin_pool)
    .await
    .unwrap();

    TestContext {
        pools,
        repo,
        guard,
        schema_name,
        org_a,
        mem_a,
        org_a_scope,
        org_b_scope,
        org_a_b1_scope,
        res_a1_id,
        res_a2_id,
        res_b1_id,
        org_a_id_str,
        org_b_id_str,
        branch_a1_id_str,
        branch_b1_id_str,
    }
}

// ============================================================================
// 1. CATALOG & ROLE PRIVILEGE ASSERTIONS
// ============================================================================

#[tokio::test]
async fn test_catalog_runtime_role_privileges() {
    let ctx = setup_test_context().await;
    ctx.pools
        .verify_runtime_role(&ctx.schema_name)
        .await
        .expect("app_runtime role privileges failed catalog verification");

    ctx.pools
        .verify_effective_privileges(&ctx.schema_name)
        .await
        .expect("app_runtime effective privileges failed catalog verification");
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_catalog_rls_policy_metadata() {
    let ctx = setup_test_context().await;
    ctx.pools
        .verify_rls_catalog_metadata(&ctx.schema_name)
        .await
        .expect("RLS catalog metadata failed verification");
    ctx.guard.teardown().await;
}

// ============================================================================
// 2. POSITIVE ISOLATION TESTS
// ============================================================================

#[tokio::test]
async fn test_positive_tenant_a_reads_a() {
    let ctx = setup_test_context().await;
    let res = ctx
        .repo
        .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
        .await
        .expect("Tenant A should read its own resource res_A1");

    assert_eq!(res.id, ctx.res_a1_id);
    assert_eq!(res.organization_id, ctx.org_a_id_str);
    assert_eq!(res.data, "Secret Data A1");
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_positive_tenant_b_reads_b() {
    let ctx = setup_test_context().await;
    let res = ctx
        .repo
        .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
        .await
        .expect("Tenant B should read its own resource res_B1");

    assert_eq!(res.id, ctx.res_b1_id);
    assert_eq!(res.organization_id, ctx.org_b_id_str);
    assert_eq!(res.data, "Secret Data B1");
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_positive_tenant_a_updates_a() {
    let ctx = setup_test_context().await;
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
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_positive_tenant_a_deletes_a() {
    let ctx = setup_test_context().await;
    ctx.repo
        .delete_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
        .await
        .expect("Tenant A should delete its own resource res_A1");

    let fetch_res = ctx
        .repo
        .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
        .await;

    assert!(matches!(fetch_res, Err(PgAuthorityError::NotFoundOrDenied)));
    ctx.guard.teardown().await;
}

// ============================================================================
// 3. NEGATIVE CROSS-TENANT ISOLATION & WITH CHECK PROOFS
// ============================================================================

#[tokio::test]
async fn test_negative_tenant_a_cannot_read_b() {
    let ctx = setup_test_context().await;
    let res = ctx
        .repo
        .get_tenant_resource(&ctx.org_a_scope, &ctx.res_b1_id)
        .await;

    assert!(
        matches!(res, Err(PgAuthorityError::NotFoundOrDenied)),
        "Tenant A must not read Tenant B resource"
    );
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_update_b() {
    let ctx = setup_test_context().await;
    let update_res = ctx
        .repo
        .update_tenant_resource(&ctx.org_a_scope, &ctx.res_b1_id, "Hacked Data")
        .await;

    assert!(
        matches!(update_res, Err(PgAuthorityError::NotFoundOrDenied)),
        "Tenant A must not update Tenant B resource"
    );

    // Verify DB state remains unchanged via Tenant B read
    let b_res = ctx
        .repo
        .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
        .await
        .unwrap();

    assert_eq!(b_res.data, "Secret Data B1");
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_delete_b() {
    let ctx = setup_test_context().await;
    let delete_res = ctx
        .repo
        .delete_tenant_resource(&ctx.org_a_scope, &ctx.res_b1_id)
        .await;

    assert!(
        matches!(delete_res, Err(PgAuthorityError::NotFoundOrDenied)),
        "Tenant A must not delete Tenant B resource"
    );

    // Verify DB state remains unchanged via Tenant B read
    let b_res = ctx
        .repo
        .get_tenant_resource(&ctx.org_b_scope, &ctx.res_b1_id)
        .await;

    assert!(b_res.is_ok());
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_insert_b_owned_row_relationally_valid() {
    let ctx = setup_test_context().await;
    let res_malicious_id = format!("res_malicious_{}", uuid::Uuid::new_v4().simple());

    // Relationally valid fixture: Branch B1 belongs to Org B (FK constraint is 100% satisfied!)
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

    // Assert SQLSTATE 42501 (insufficient_privilege) or 44000 (check_violation) proving denial is strictly from RLS WITH CHECK, NOT FK
    let code = pg_err.code().unwrap_or_default();
    assert!(
        code == "42501" || code == "44000",
        "Must be SQLSTATE 42501 (RLS policy violation) or 44000 from PostgreSQL RLS WITH CHECK, observed: {code}"
    );

    tx.rollback().await.unwrap();

    // Verify denial was strictly caused by RLS WITH CHECK, and DB state is unchanged
    let fetch_res = ctx
        .repo
        .get_tenant_resource(&ctx.org_b_scope, &res_malicious_id)
        .await;

    assert!(matches!(fetch_res, Err(PgAuthorityError::NotFoundOrDenied)));
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_negative_ownership_changing_update_relationally_valid() {
    let ctx = setup_test_context().await;

    // Relationally valid mutation target: Change organization_id to Org B AND branch_id to Branch B1 (belonging to Org B).
    // The foreign key constraint is 100% valid! The ONLY constraint denying this update is PostgreSQL RLS WITH CHECK!
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

    // Verify DB state remains unchanged (res_A1 still owned by org_A)
    let fetch_res = ctx
        .repo
        .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
        .await
        .unwrap();

    assert_eq!(fetch_res.organization_id, ctx.org_a_id_str);
    ctx.guard.teardown().await;
}

// ============================================================================
// 4. END-TO-END APPLICATION + DB COMPOSITION TEST
// ============================================================================

#[tokio::test]
async fn test_end_to_end_application_and_db_composition() {
    let ctx = setup_test_context().await;

    // 1. AUTHORIZED PATH
    // Principal A -> Org A binding -> Scope Resolution -> AuthorizedScope -> DB Context -> DB Query
    let req_org_a = RequestedOrganizationId(OrganizationId::new(&ctx.org_a_id_str).unwrap());
    let _trusted_a = bind_organization(&req_org_a, &ctx.mem_a)
        .expect("Application binding succeeds for authorized membership");
    let eff_scope_a =
        resolve_effective_scope(&ctx.mem_a, &ctx.org_a, None).expect("Scope resolution succeeds");
    let authorized_scope_a = AuthorizedScope::from_effective(&eff_scope_a);

    let res_a = ctx
        .repo
        .get_tenant_resource(&authorized_scope_a, &ctx.res_a1_id)
        .await
        .expect("Authorized end-to-end operation succeeds");
    assert_eq!(res_a.data, "Secret Data A1");

    // 2. APPLICATION TAMPER PATH
    // Principal A attempts to bind to Org B without membership -> Application rejects before DB call
    let req_org_b = RequestedOrganizationId(OrganizationId::new(&ctx.org_b_id_str).unwrap());
    let tamper_res = bind_organization(&req_org_b, &ctx.mem_a);
    assert!(
        tamper_res.is_err(),
        "Application layer must reject cross-tenant binding attempt before DB layer"
    );

    // 3. DATABASE INDEPENDENCE PATH
    // Tenant A DB context active -> deliberately issue broad raw DB query without app tenant predicate -> PostgreSQL independently blocks Tenant B rows
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
    ctx.guard.teardown().await;
}

// ============================================================================
// 5. DIRECT DB BOUNDARY PROOF
// ============================================================================

#[tokio::test]
async fn test_direct_db_query_without_application_predicate() {
    let ctx = setup_test_context().await;

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
    ctx.guard.teardown().await;
}

// ============================================================================
// 6. BRANCH ISOLATION TESTS
// ============================================================================

#[tokio::test]
async fn test_branch_scoped_read_isolation() {
    let ctx = setup_test_context().await;

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
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_cross_tenant_branch_binding_denial() {
    let ctx = setup_test_context().await;

    let invalid_resource = TenantResource {
        id: format!("res_cross_branch_{}", uuid::Uuid::new_v4().simple()),
        organization_id: ctx.org_b_id_str.clone(),
        branch_id: ctx.branch_a1_id_str.clone(),
        data: "Cross Branch Invalid".to_string(),
    };

    let insert_res = ctx
        .repo
        .create_tenant_resource(&ctx.org_b_scope, &invalid_resource)
        .await;

    assert!(
        insert_res.is_err(),
        "Foreign key / RLS constraint must deny referencing a branch belonging to another organization"
    );
    ctx.guard.teardown().await;
}

// ============================================================================
// 7. MISSING & INVALID CONTEXT TESTS
// ============================================================================

#[tokio::test]
async fn test_missing_tenant_context_fails_closed() {
    let ctx = setup_test_context().await;

    let mut tx = ctx.pools.runtime_pool.begin().await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenant_resources")
        .fetch_one(&mut *tx)
        .await
        .unwrap();

    assert_eq!(
        count, 0,
        "Unset / missing tenant context must return 0 rows (fail closed)"
    );
    ctx.guard.teardown().await;
}

#[tokio::test]
async fn test_invalid_tenant_context_fails_closed() {
    let ctx = setup_test_context().await;

    let invalid_scope = AuthorizedScope {
        organization_id: OrganizationId::new("org_NONEXISTENT").unwrap(),
        membership_id: MembershipId::new("mem_NONEXISTENT").unwrap(),
        branch_id: None,
        organization_version: 1,
        membership_version: 1,
    };

    let res = ctx
        .repo
        .get_tenant_resource(&invalid_scope, &ctx.res_a1_id)
        .await;

    assert!(
        matches!(res, Err(PgAuthorityError::NotFoundOrDenied)),
        "Nonexistent tenant context must fail closed"
    );
    ctx.guard.teardown().await;
}

// ============================================================================
// 8. CONNECTION POOL LEAKAGE & ROLLBACK SAFETY
// ============================================================================

#[tokio::test]
async fn test_connection_pool_context_leakage_and_rollback_safety() {
    let ctx = setup_test_context().await;

    for _ in 0..10 {
        {
            let mut tx = ctx.pools.runtime_pool.begin().await.unwrap();
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
            let mut tx = ctx.pools.runtime_pool.begin().await.unwrap();
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
    ctx.guard.teardown().await;
}

// ============================================================================
// 9. CONCURRENT TENANT A & B READ & WRITE ISOLATION
// ============================================================================

#[tokio::test]
async fn test_concurrent_tenant_isolation_reads_and_writes() {
    let ctx = Arc::new(setup_test_context().await);
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
    ctx.guard.teardown().await;
}

// ============================================================================
// 10. SCHEMA ISOLATION & TEARDOWN REGRESSION ASSERTIONS
// ============================================================================

#[tokio::test]
async fn test_schema_isolation_and_teardown_regression() {
    let ctx = setup_test_context().await;
    let schema_name = ctx.schema_name.clone();

    // 1. Verify schema exists
    let schema_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1)")
            .bind(&schema_name)
            .fetch_one(&ctx.pools.admin_pool)
            .await
            .unwrap();
    assert!(schema_exists, "Isolated schema must exist");

    // 2. Verify organizations exists in isolated schema
    let org_table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = $1 AND c.relname = 'organizations')",
    )
    .bind(&schema_name)
    .fetch_one(&ctx.pools.admin_pool)
    .await
    .unwrap();
    assert!(
        org_table_exists,
        "organizations table must exist in isolated schema"
    );

    // 3. Perform teardown and assert schema no longer exists
    ctx.guard.teardown().await;

    let schema_exists_after: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1)")
            .bind(&schema_name)
            .fetch_one(&ctx.pools.admin_pool)
            .await
            .unwrap();
    assert!(
        !schema_exists_after,
        "Isolated schema must be completely dropped after teardown"
    );
}
