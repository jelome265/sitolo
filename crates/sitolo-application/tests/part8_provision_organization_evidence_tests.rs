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
//! `ADMIN_DATABASE_URL` (or `DATABASE_URL`) to a database the connecting
//! role can create/drop schemas in. These tests intentionally panic rather
//! than skip when that variable is absent.
//!
//! ## What these tests prove, and what they do not
//!
//! - `provision_organization_emits_audit_and_outbox_on_success` proves Test
//!   C (§61): one successful command produces exactly one audit event and
//!   one paired outbox event (§38), and the audit/outbox pair is written
//!   through one PostgreSQL transaction.
//! - `provision_organization_rolls_back_audit_when_outbox_write_fails`
//!   proves Test B (§61): if the outbox half of that shared transaction
//!   fails, the audit half that already ran in the same transaction is
//!   rolled back with it — the pair is atomic with each other.
//! - `provision_organization_known_gap_business_state_survives_evidence_failure`
//!   documents, rather than hides, the residual gap disclosed in the
//!   `sitolo-application` tenancy module docs: because `TenancyDatabase` is
//!   still in-memory pending Phase 5, the organization/membership/branch
//!   mutation itself is NOT rolled back when the evidence transaction
//!   fails. Full Test A (§61) — business mutation rolled back together with
//!   failed audit — remains blocked on Phase 5 and is not claimed here.

use std::env;
use std::sync::Arc;

use sitolo_application::{TenancyEvidenceSink, TenancyService};
use sitolo_domain::tenancy::{BranchId, MembershipId, OrganizationId, TenancyError, TenantUserId};
use sitolo_persistence::{TenancyDatabase, TenancyStores};
use sitolo_persistence::audit_repository::PostgresAuditWriter;
use sitolo_persistence::outbox_repository::PostgresOutboxRepository;
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

struct TestContext {
    admin_pool: PgPool,
    schema_name: String,
}

async fn setup() -> TestContext {
    let admin_url = env::var("ADMIN_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect(
            "ADMIN_DATABASE_URL or DATABASE_URL must be set to run Part 8 \
             PostgreSQL integration tests; these tests do not skip silently \
             (contract §59)",
        );

    let schema_name = format!("test_part8_app_{}", Uuid::new_v4().simple());

    // Set `search_path` at connection-establishment time (not via a runtime
    // `SET` on the pool, which does not reliably survive across pooled
    // connections) so every connection this pool opens resolves the
    // unqualified table names the production code uses ("audit_events",
    // "outbox_events") against our isolated test schema. Same approach as
    // crates/sitolo-persistence/tests/rls_security_tests.rs.
    let admin_opts: PgConnectOptions = admin_url
        .parse()
        .expect("invalid ADMIN_DATABASE_URL/DATABASE_URL")
        .options([("search_path", schema_name.as_str())]);

    let admin_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(admin_opts)
        .await
        .expect("connect to Postgres");

    sqlx::raw_sql(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\""))
        .execute(&admin_pool)
        .await
        .expect("create test schema");

    let schema_sql = include_str!("fixtures/part8_evidence_schema.sql");
    sqlx::raw_sql(schema_sql)
        .execute(&admin_pool)
        .await
        .expect("apply Part 8 evidence schema fixture");

    TestContext {
        admin_pool,
        schema_name,
    }
}

async fn teardown(ctx: &TestContext) {
    let _ = sqlx::raw_sql(&format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", ctx.schema_name))
        .execute(&ctx.admin_pool)
        .await;
}

fn service_with_evidence(pool: PgPool) -> TenancyService {
    let audit = Arc::new(PostgresAuditWriter::new(pool.clone()));
    let outbox = Arc::new(PostgresOutboxRepository::new(pool.clone()));
    let sink = TenancyEvidenceSink::new(pool, audit, outbox);
    TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new()).with_evidence(sink)
}

async fn row_count(pool: &PgPool, table: &str, org_id: &str) -> i64 {
    sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM {table} WHERE organization_id = $1"
    ))
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("counting {table} for {org_id}: {e}"))
}

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

    let service = service_with_evidence(ctx.admin_pool.clone());
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

    teardown(&ctx).await;

    let provisioned = provisioned.expect("provision_organization should succeed");
    assert_eq!(provisioned.organization.id, args.org_id);
    assert_eq!(
        audit_count, 1,
        "exactly one audit event for one successful command (§38)"
    );
    assert_eq!(
        outbox_count, 1,
        "exactly one outbox event for one successful command (§38)"
    );
}

/// Test B (§61): business mutation succeeds, outbox write fails -> the
/// audit write in the same transaction is rolled back with it. Failure is
/// induced by renaming `outbox_events` out from under the write after audit
/// would otherwise have already been staged in the same transaction, which
/// is deterministic and does not depend on guessing a generated event id.
#[tokio::test]
async fn provision_organization_rolls_back_audit_when_outbox_write_fails() {
    let ctx = setup().await;

    // Break the outbox half of the shared transaction only.
    sqlx::raw_sql("ALTER TABLE outbox_events RENAME TO outbox_events_disabled")
        .execute(&ctx.admin_pool)
        .await
        .expect("rename outbox_events to induce a deterministic outbox write failure");

    let service = service_with_evidence(ctx.admin_pool.clone());
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

    teardown(&ctx).await;

    assert!(
        matches!(result, Err(TenancyError::EvidencePersistenceFailed)),
        "expected evidence persistence failure, got {result:?}"
    );
    assert_eq!(
        audit_count, 0,
        "audit insert must roll back with the outbox insert in the same transaction (§5, §29)"
    );
}

/// Documents the disclosed residual gap (see the `sitolo-application`
/// tenancy module docs): `TenancyDatabase` is in-memory pending Phase 5, so
/// the business mutation is NOT part of the same transaction as the
/// evidence writes and is not rolled back when they fail. This is not
/// Test A passing — it is the opposite, made explicit rather than silently
/// true. Closing it requires Phase 5 (out of Part 8 scope, contract §51).
#[tokio::test]
async fn provision_organization_known_gap_business_state_survives_evidence_failure() {
    let ctx = setup().await;

    sqlx::raw_sql("ALTER TABLE outbox_events RENAME TO outbox_events_disabled")
        .execute(&ctx.admin_pool)
        .await
        .expect("rename outbox_events to induce a deterministic outbox write failure");

    let db = Arc::new(TenancyDatabase::new());
    let audit = Arc::new(PostgresAuditWriter::new(ctx.admin_pool.clone()));
    let outbox = Arc::new(PostgresOutboxRepository::new(ctx.admin_pool.clone()));
    let sink = TenancyEvidenceSink::new(ctx.admin_pool.clone(), audit, outbox);
    let service = TenancyService::new(db.clone(), Vec::new()).with_evidence(sink);
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

    let in_memory_snapshot = db.organization_snapshot(&args.org_id).await;

    teardown(&ctx).await;

    assert!(result.is_err(), "evidence write was induced to fail");
    assert!(
        in_memory_snapshot.is_some(),
        "known gap: in-memory business state is not rolled back by evidence \
         failure until Phase 5 moves tenancy state into PostgreSQL — this \
         assertion documents that gap rather than concealing it"
    );
}
