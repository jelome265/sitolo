//! Phase 4 Part 8 (PR-009) integration tests for
//! `TenancyService::provision_organization` evidence emission.
//!
//! Exercises the audit+outbox atomicity slice described in
//! `crates/sitolo-application/src/tenancy.rs` module docs and required by
//! `docs/phase4_part8_audit_outbox_implementation_contract.md` §29
//! (atomicity matrix) and §61 (required atomicity tests).
//!
//! Requires a real PostgreSQL instance (contract §59: "No security test may
//! silently skip because PostgreSQL is unavailable"). Set
//! `ADMIN_DATABASE_URL` to a database the connecting
//! role can create/drop schemas in. These tests intentionally panic rather
//! than skip when that variable is absent.
//!
//! ## What these tests prove, and what they do not
//!
//! Success and failure cases exercise the same PostgreSQL transaction for
//! the organization, owner membership, default branch, audit row and outbox
//! event. They intentionally panic rather than skip without PostgreSQL.

use std::env;
use std::sync::Arc;

use sitolo_application::{TenancyEvidenceSink, TenancyService};
use sitolo_domain::tenancy::{BranchId, MembershipId, OrganizationId, TenancyError, TenantUserId};
use sitolo_persistence::TenancyDatabase;
use sitolo_persistence::audit_repository::PostgresAuditWriter;
use sitolo_persistence::outbox_repository::PostgresOutboxRepository;
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::raw_sql::raw_sql;
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use tokio::sync::Barrier;
use uuid::Uuid;

struct TestContext {
    admin_pool: PgPool,
    runtime_pool: PgPool,
    schema_name: String,
}

async fn setup() -> TestContext {
    let admin_url = env::var("ADMIN_DATABASE_URL").expect(
        "ADMIN_DATABASE_URL must be set to run Part 8 PostgreSQL integration \
         tests; these tests do not skip silently (contract §59)",
    );

    let schema_name = format!("test_part8_app_{}", Uuid::new_v4().simple());

    // Set `search_path` at connection-establishment time (not via a runtime
    // `SET` on the pool, which does not reliably survive across pooled
    // connections) so every connection this pool opens resolves the
    // unqualified table names the production code uses ("audit_events",
    // "outbox_events") against our isolated test schema. Same approach as
    // crates/sitolo-persistence/tests/rls_security_tests.rs.
    let admin_opts: PgConnectOptions = admin_url
        .parse::<PgConnectOptions>()
        .expect("invalid ADMIN_DATABASE_URL")
        .options([("search_path", schema_name.as_str())]);

    let admin_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(admin_opts)
        .await
        .expect("connect to Postgres");

    raw_sql(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\""))
        .execute(&admin_pool)
        .await
        .expect("create test schema");

    let schema_sql =
        include_str!("../../sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql")
            .replace("__SCHEMA__", &format!("\"{schema_name}\""));
    raw_sql(&schema_sql)
        .execute(&admin_pool)
        .await
        .expect("apply Part 8 audit/outbox schema fixture");

    let tenancy_sql =
        include_str!("../../sitolo-persistence/schema/phase4_part8_tenancy_provisioning.sql");
    raw_sql(tenancy_sql)
        .execute(&admin_pool)
        .await
        .expect("apply PostgreSQL tenancy provisioning schema");

    let runtime_url = env::var("RUNTIME_DATABASE_URL")
        .expect("RUNTIME_DATABASE_URL must be set for PostgreSQL provisioning tests");
    let runtime_opts: PgConnectOptions = runtime_url
        .parse::<PgConnectOptions>()
        .expect("invalid RUNTIME_DATABASE_URL")
        .options([("search_path", schema_name.as_str())]);
    let runtime_pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(|conn, _| {
            Box::pin(async move {
                query("SET ROLE app_runtime").execute(conn).await?;
                Ok(())
            })
        })
        .connect_with(runtime_opts)
        .await
        .expect("connect runtime test pool");
    let identity: String = query_scalar("SELECT current_user")
        .fetch_one(&runtime_pool)
        .await
        .expect("read runtime identity");
    assert_eq!(identity, "app_runtime");

    TestContext {
        admin_pool,
        runtime_pool,
        schema_name,
    }
}

async fn teardown(ctx: &TestContext) {
    let _ = raw_sql(&format!(
        "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
        ctx.schema_name
    ))
    .execute(&ctx.admin_pool)
    .await;
}

fn service_with_evidence(pool: PgPool) -> TenancyService {
    let audit = Arc::new(PostgresAuditWriter::new());
    let outbox = Arc::new(PostgresOutboxRepository::new(pool.clone()));
    let sink = TenancyEvidenceSink::new(pool, audit, outbox);
    TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new()).with_evidence(sink)
}

async fn row_count(pool: &PgPool, table: &str, org_id: &str) -> i64 {
    query_scalar(&format!(
        "SELECT COUNT(*) FROM {table} WHERE organization_id = $1"
    ))
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("counting {table} for {org_id}: {e}"))
}

async fn organization_row_count(pool: &PgPool, organization_id: &str) -> i64 {
    query_scalar("SELECT COUNT(*) FROM organizations WHERE id = $1")
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("counting organization {organization_id}: {e}"))
}

#[derive(Clone)]
struct ProvisionArgs {
    org_id: OrganizationId,
    membership_id: MembershipId,
    user_id: TenantUserId,
    branch_id: BranchId,
}

fn fresh_provision_args() -> ProvisionArgs {
    ProvisionArgs {
        org_id: OrganizationId::new(format!("org-{}", Uuid::new_v4().simple())).unwrap(),
        membership_id: MembershipId::new(format!("mem-{}", Uuid::new_v4().simple())).unwrap(),
        user_id: TenantUserId::new(format!("user-{}", Uuid::new_v4().simple())).unwrap(),
        branch_id: BranchId::new(format!("branch-{}", Uuid::new_v4().simple())).unwrap(),
    }
}

/// Test C (§61): all writes succeed -> business, audit, and outbox present;
/// exactly one audit event and one outbox event per successful command
/// (§38 — no duplicate semantic events for one command).
#[tokio::test]
async fn provision_organization_emits_audit_and_outbox_on_success() {
    let ctx = setup().await;

    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args();

    let provisioned = service
        .provision_organization(
            args.org_id.clone(),
            "Acme Corp",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await;

    let audit_count = row_count(&ctx.admin_pool, "audit_events", args.org_id.as_str()).await;
    let outbox_count = row_count(&ctx.admin_pool, "outbox_events", args.org_id.as_str()).await;
    let organization_count = organization_row_count(&ctx.admin_pool, args.org_id.as_str()).await;
    let membership_count = row_count(
        &ctx.admin_pool,
        "organization_memberships",
        args.org_id.as_str(),
    )
    .await;
    let branch_count = row_count(&ctx.admin_pool, "branches", args.org_id.as_str()).await;

    teardown(&ctx).await;

    let provisioned = provisioned.expect("provision_organization should succeed");
    assert_eq!(provisioned.organization.id, args.org_id);
    assert_eq!(organization_count, 1);
    assert_eq!(membership_count, 1);
    assert_eq!(branch_count, 1);
    assert_eq!(
        audit_count, 1,
        "exactly one audit event for one successful command (§38)"
    );
    assert_eq!(
        outbox_count, 1,
        "exactly one outbox event for one successful command (§38)"
    );
}

/// A repeated command with the same complete topology payload returns the
/// existing result and does not emit a second semantic audit/outbox event.
#[tokio::test]
async fn provision_organization_retry_is_idempotent_and_divergence_conflicts() {
    let ctx = setup().await;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args();

    let first = service
        .provision_organization(
            args.org_id.clone(),
            "Acme Corp",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await;
    let retry = service
        .provision_organization(
            args.org_id.clone(),
            "Acme Corp",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await;
    let divergent = service
        .provision_organization(
            args.org_id.clone(),
            "Different name",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await;
    let audit_count = row_count(&ctx.admin_pool, "audit_events", args.org_id.as_str()).await;
    let outbox_count = row_count(&ctx.admin_pool, "outbox_events", args.org_id.as_str()).await;

    teardown(&ctx).await;

    assert!(first.is_ok());
    assert_eq!(retry, first);
    assert_eq!(divergent, Err(TenancyError::Conflict));
    assert_eq!(audit_count, 1);
    assert_eq!(outbox_count, 1);
}

/// Concurrent identical requests converge on one complete provisioning
/// bundle and one mandatory audit/outbox pair.
#[tokio::test]
async fn provision_organization_concurrent_same_payload_is_idempotent() {
    let ctx = setup().await;
    let service = Arc::new(service_with_evidence(ctx.runtime_pool.clone()));
    let args = fresh_provision_args();
    let barrier = Arc::new(Barrier::new(2));

    let first_service = Arc::clone(&service);
    let first_barrier = Arc::clone(&barrier);
    let first_args = args.clone();
    let first = tokio::spawn(async move {
        first_barrier.wait().await;
        first_service
            .provision_organization(
                first_args.org_id,
                "Acme Corp",
                first_args.membership_id,
                first_args.user_id,
                first_args.branch_id,
                "HQ",
            )
            .await
    });

    let second_barrier = Arc::clone(&barrier);
    let second_args = args.clone();
    let second = tokio::spawn(async move {
        second_barrier.wait().await;
        service
            .provision_organization(
                second_args.org_id,
                "Acme Corp",
                second_args.membership_id,
                second_args.user_id,
                second_args.branch_id,
                "HQ",
            )
            .await
    });

    let first_result = first.await.expect("first provisioning task did not panic");
    let second_result = second
        .await
        .expect("second provisioning task did not panic");
    let audit_count = row_count(&ctx.admin_pool, "audit_events", args.org_id.as_str()).await;
    let outbox_count = row_count(&ctx.admin_pool, "outbox_events", args.org_id.as_str()).await;
    let organization_count = organization_row_count(&ctx.admin_pool, args.org_id.as_str()).await;
    let membership_count = row_count(
        &ctx.admin_pool,
        "organization_memberships",
        args.org_id.as_str(),
    )
    .await;
    let branch_count = row_count(&ctx.admin_pool, "branches", args.org_id.as_str()).await;

    teardown(&ctx).await;

    assert!(first_result.is_ok());
    assert!(second_result.is_ok());
    assert_eq!(organization_count, 1);
    assert_eq!(membership_count, 1);
    assert_eq!(branch_count, 1);
    assert_eq!(audit_count, 1);
    assert_eq!(outbox_count, 1);
}

/// Test A (§61): outbox failure rolls back business and audit rows already
/// staged in the same transaction.
#[tokio::test]
async fn provision_organization_rolls_back_audit_when_outbox_write_fails() {
    let ctx = setup().await;

    // Break the outbox half of the shared transaction only.
    raw_sql("ALTER TABLE outbox_events RENAME TO outbox_events_disabled")
        .execute(&ctx.admin_pool)
        .await
        .expect("rename outbox_events to induce a deterministic outbox write failure");

    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args();

    let result = service
        .provision_organization(
            args.org_id.clone(),
            "Acme Corp",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await;

    let audit_count = row_count(&ctx.admin_pool, "audit_events", args.org_id.as_str()).await;
    let organization_count = organization_row_count(&ctx.admin_pool, args.org_id.as_str()).await;
    let membership_count = row_count(
        &ctx.admin_pool,
        "organization_memberships",
        args.org_id.as_str(),
    )
    .await;
    let branch_count = row_count(&ctx.admin_pool, "branches", args.org_id.as_str()).await;

    teardown(&ctx).await;

    assert!(
        matches!(result, Err(TenancyError::EvidencePersistenceFailed)),
        "expected evidence persistence failure, got {result:?}"
    );
    assert_eq!(
        audit_count, 0,
        "audit insert must roll back with the outbox insert in the same transaction (§5, §29)"
    );
    assert_eq!(organization_count, 0, "business row must roll back");
    assert_eq!(membership_count, 0, "business row must roll back");
    assert_eq!(branch_count, 0, "business row must roll back");
}

/// Mandatory audit failure also rolls back business state. The renamed table
/// causes the audit insert to fail after the three business inserts.
#[tokio::test]
async fn provision_organization_audit_failure_rolls_back_business_state() {
    let ctx = setup().await;

    raw_sql("ALTER TABLE audit_events RENAME TO audit_events_disabled")
        .execute(&ctx.admin_pool)
        .await
        .expect("rename audit_events to induce a deterministic audit write failure");

    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args();

    let result = service
        .provision_organization(
            args.org_id.clone(),
            "Acme Corp",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await;

    let organization_count = organization_row_count(&ctx.admin_pool, args.org_id.as_str()).await;
    let membership_count = row_count(
        &ctx.admin_pool,
        "organization_memberships",
        args.org_id.as_str(),
    )
    .await;
    let branch_count = row_count(&ctx.admin_pool, "branches", args.org_id.as_str()).await;
    teardown(&ctx).await;

    assert!(result.is_err(), "evidence write was induced to fail");
    assert_eq!(organization_count, 0);
    assert_eq!(membership_count, 0);
    assert_eq!(branch_count, 0);
}
