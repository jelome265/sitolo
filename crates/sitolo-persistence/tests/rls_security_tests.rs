//! Phase 4 Part 7 / PR-008 PostgreSQL Row-Level Security Integration & Real Negative Tests
//!
//! Non-negotiable contract requirement:
//! Execute security proofs against a REAL PostgreSQL database instance.
//! Prove tenant boundary against PostgreSQL RLS, least-privileged runtime role,
//! transaction-local context derived from AuthorizedScope, and negative tests.

use sitolo_domain::tenancy::{BranchId, MembershipId, OrganizationId};
use sitolo_persistence::postgres::{
    PgAuthorityError, PgAuthorityPools, PgTenantRepository, TenantResource,
    set_transaction_tenant_context,
};
use sitolo_tenancy::AuthorizedScope;
use std::env;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tokio::task::JoinSet;

static POOLS_INIT: OnceCell<PgAuthorityPools> = OnceCell::const_new();

/// Test context holding admin and runtime pools, plus pre-seeded fixture IDs.
struct TestContext {
    pools: PgAuthorityPools,
    repo: PgTenantRepository,
    org_a_scope: AuthorizedScope,
    org_b_scope: AuthorizedScope,
    org_a_b1_scope: AuthorizedScope,
    res_a1_id: String,
    res_a2_id: String,
    res_b1_id: String,
    org_a_id_str: String,
    org_b_id_str: String,
    branch_a1_id_str: String,
}

async fn get_shared_pools() -> PgAuthorityPools {
    POOLS_INIT
        .get_or_init(|| async {
            let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@127.0.0.1:5432/sitolo_test".to_string()
            });

            let runtime_url = db_url
                .replace("postgres:postgres@", "app_runtime:app_runtime_pass@")
                .replace("postgres@", "app_runtime:app_runtime_pass@");

            let admin_pool = sqlx::PgPool::connect(&db_url).await.expect(
                "FAIL-CLOSED: Real PostgreSQL database connection failed. Tests cannot run without PostgreSQL.",
            );

            let migration_sql = include_str!("../../../migrations/0001_initial_rls_schema.sql");
            sqlx::raw_sql(migration_sql)
                .execute(&admin_pool)
                .await
                .expect("Failed to apply initial RLS schema migration");

            PgAuthorityPools::connect(&db_url, &runtime_url)
                .await
                .expect("Failed to create authority pools")
        })
        .await
        .clone()
}

async fn setup_test_context() -> TestContext {
    let pools = get_shared_pools().await;

    let unique_id = uuid::Uuid::new_v4().simple().to_string();
    let org_a_id_str = format!("org_A_{unique_id}");
    let org_b_id_str = format!("org_B_{unique_id}");
    let branch_a1_id_str = format!("branch_A1_{unique_id}");
    let branch_a2_id_str = format!("branch_A2_{unique_id}");
    let branch_b1_id_str = format!("branch_B1_{unique_id}");

    let res_a1_id = format!("res_A1_{unique_id}");
    let res_a2_id = format!("res_A2_{unique_id}");
    let res_b1_id = format!("res_B1_{unique_id}");

    // Build scopes
    let org_a_id = OrganizationId::new(&org_a_id_str).unwrap();
    let org_b_id = OrganizationId::new(&org_b_id_str).unwrap();
    let branch_a1_id = BranchId::new(&branch_a1_id_str).unwrap();

    let mem_a_id = MembershipId::new(format!("mem_A_{unique_id}")).unwrap();
    let mem_b_id = MembershipId::new(format!("mem_B_{unique_id}")).unwrap();

    let org_a_scope = AuthorizedScope {
        organization_id: org_a_id.clone(),
        membership_id: mem_a_id.clone(),
        branch_id: None,
        organization_version: 1,
        membership_version: 1,
    };

    let org_b_scope = AuthorizedScope {
        organization_id: org_b_id.clone(),
        membership_id: mem_b_id.clone(),
        branch_id: None,
        organization_version: 1,
        membership_version: 1,
    };

    let org_a_b1_scope = org_a_scope.clone().with_branch(branch_a1_id.clone());

    let repo = PgTenantRepository::new(pools.runtime_pool.clone());

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
        org_a_scope,
        org_b_scope,
        org_a_b1_scope,
        res_a1_id,
        res_a2_id,
        res_b1_id,
        org_a_id_str,
        org_b_id_str,
        branch_a1_id_str,
    }
}

// ============================================================================
// 1. CATALOG & ROLE PRIVILEGE ASSERTIONS
// ============================================================================

#[tokio::test]
async fn test_catalog_runtime_role_privileges() {
    let ctx = setup_test_context().await;
    ctx.pools
        .verify_runtime_role()
        .await
        .expect("app_runtime role privileges failed catalog verification");
}

#[tokio::test]
async fn test_catalog_rls_policy_metadata() {
    let ctx = setup_test_context().await;
    ctx.pools
        .verify_rls_catalog_metadata()
        .await
        .expect("RLS catalog metadata failed verification");
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
}

// ============================================================================
// 3. NEGATIVE CROSS-TENANT ISOLATION TESTS
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
}

#[tokio::test]
async fn test_negative_tenant_a_cannot_insert_b_owned_row() {
    let ctx = setup_test_context().await;
    let res_malicious_id = format!("res_malicious_{}", uuid::Uuid::new_v4().simple());
    let malicious_resource = TenantResource {
        id: res_malicious_id.clone(),
        organization_id: ctx.org_b_id_str.clone(), // Trying to insert for Org B
        branch_id: "branch_B1_fake".to_string(),
        data: "Malicious Insert".to_string(),
    };

    let insert_res = ctx
        .repo
        .create_tenant_resource(&ctx.org_a_scope, &malicious_resource)
        .await;

    assert!(
        insert_res.is_err(),
        "RLS WITH CHECK must deny inserting a row owned by Org B while operating under Org A context"
    );

    // Verify DB state remains unchanged via Tenant B read
    let fetch_res = ctx
        .repo
        .get_tenant_resource(&ctx.org_b_scope, &res_malicious_id)
        .await;

    assert!(matches!(fetch_res, Err(PgAuthorityError::NotFoundOrDenied)));
}

#[tokio::test]
async fn test_negative_ownership_changing_update() {
    let ctx = setup_test_context().await;
    let update_res = ctx
        .repo
        .update_tenant_resource_ownership(&ctx.org_a_scope, &ctx.res_a1_id, &ctx.org_b_id_str)
        .await;

    assert!(
        update_res.is_err(),
        "RLS WITH CHECK must deny changing organization_id of res_A1 to org_B"
    );

    // Verify DB state remains unchanged (res_A1 still owned by org_A)
    let fetch_res = ctx
        .repo
        .get_tenant_resource(&ctx.org_a_scope, &ctx.res_a1_id)
        .await
        .unwrap();

    assert_eq!(fetch_res.organization_id, ctx.org_a_id_str);
}

// ============================================================================
// 4. BRANCH ISOLATION TESTS
// ============================================================================

#[tokio::test]
async fn test_branch_scoped_read_isolation() {
    let ctx = setup_test_context().await;

    // Org A with Branch A1 scope reads res_A1 (branch_A1) -> success
    let res1 = ctx
        .repo
        .get_tenant_resource(&ctx.org_a_b1_scope, &ctx.res_a1_id)
        .await;
    assert!(res1.is_ok());

    // Org A with Branch A1 scope attempts to read res_A2 (branch_A2) -> denied by branch RLS
    let res2 = ctx
        .repo
        .get_tenant_resource(&ctx.org_a_b1_scope, &ctx.res_a2_id)
        .await;
    assert!(matches!(res2, Err(PgAuthorityError::NotFoundOrDenied)));
}

#[tokio::test]
async fn test_cross_tenant_branch_binding_denial() {
    let ctx = setup_test_context().await;

    // Org B attempting to use Org A's branch_A1 -> composite FK / RLS fails
    let invalid_resource = TenantResource {
        id: format!("res_cross_branch_{}", uuid::Uuid::new_v4().simple()),
        organization_id: ctx.org_b_id_str.clone(),
        branch_id: ctx.branch_a1_id_str.clone(), // Branch A1 belongs to Org A!
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
}

// ============================================================================
// 5. MISSING & INVALID CONTEXT TESTS
// ============================================================================

#[tokio::test]
async fn test_missing_tenant_context_fails_closed() {
    let ctx = setup_test_context().await;

    // Begin tx without setting tenant context (unset app.organization_id)
    let mut tx = ctx.pools.runtime_pool.begin().await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenant_resources")
        .fetch_one(&mut *tx)
        .await
        .unwrap();

    assert_eq!(
        count, 0,
        "Unset / missing tenant context must return 0 rows (fail closed)"
    );
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
}

// ============================================================================
// 6. CONNECTION POOL LEAKAGE & ROLLBACK SAFETY
// ============================================================================

#[tokio::test]
async fn test_connection_pool_context_leakage_and_rollback_safety() {
    let ctx = setup_test_context().await;

    for _ in 0..10 {
        // Step 1: Tx 1 sets Tenant A context, queries res_A1, then rolls back
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

        // Step 2: Tx 2 on same pool sets Tenant B context. Verify Tenant A context does NOT leak
        {
            let mut tx = ctx.pools.runtime_pool.begin().await.unwrap();
            set_transaction_tenant_context(&mut tx, &ctx.org_b_scope)
                .await
                .unwrap();

            // Tenant B should see res_B1
            let count_b: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM tenant_resources WHERE id = $1")
                    .bind(&ctx.res_b1_id)
                    .fetch_one(&mut *tx)
                    .await
                    .unwrap();
            assert_eq!(count_b, 1);

            // Tenant B must NOT see res_A1
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
}

// ============================================================================
// 7. CONCURRENT TENANT A & B ISOLATION
// ============================================================================

#[tokio::test]
async fn test_concurrent_tenant_isolation() {
    let ctx = Arc::new(setup_test_context().await);
    let mut set = JoinSet::new();

    for i in 0..8 {
        let ctx_clone = ctx.clone();
        set.spawn(async move {
            if i % 2 == 0 {
                // Task for Tenant A
                let res = ctx_clone
                    .repo
                    .get_tenant_resource(&ctx_clone.org_a_scope, &ctx_clone.res_a1_id)
                    .await;
                assert!(res.is_ok(), "Concurrent Tenant A read res_A1 must succeed");

                let res_b = ctx_clone
                    .repo
                    .get_tenant_resource(&ctx_clone.org_a_scope, &ctx_clone.res_b1_id)
                    .await;
                assert!(
                    matches!(res_b, Err(PgAuthorityError::NotFoundOrDenied)),
                    "Concurrent Tenant A read res_B1 must be denied"
                );
            } else {
                // Task for Tenant B
                let res = ctx_clone
                    .repo
                    .get_tenant_resource(&ctx_clone.org_b_scope, &ctx_clone.res_b1_id)
                    .await;
                assert!(res.is_ok(), "Concurrent Tenant B read res_B1 must succeed");

                let res_a = ctx_clone
                    .repo
                    .get_tenant_resource(&ctx_clone.org_b_scope, &ctx_clone.res_a1_id)
                    .await;
                assert!(
                    matches!(res_a, Err(PgAuthorityError::NotFoundOrDenied)),
                    "Concurrent Tenant B read res_A1 must be denied"
                );
            }
        });
    }

    while let Some(res) = set.join_next().await {
        res.expect("Concurrent task panicked");
    }
}
