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
use sitolo_audit::ActorRef;
use sitolo_domain::tenancy::{BranchId, MembershipId, OrganizationId, TenancyError, TenantUserId};
use sitolo_persistence::TenancyDatabase;
use sitolo_persistence::audit_repository::PostgresAuditWriter;
use sitolo_persistence::outbox_repository::PostgresOutboxRepository;
use sitolo_persistence::postgres::verify_part8_provisioning_rls_metadata;
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

    verify_part8_provisioning_rls_metadata(&admin_pool, &schema_name)
        .await
        .expect("Part 8 provisioning RLS contract failed verification");

    let runtime_url = env::var("RUNTIME_DATABASE_URL")
        .expect("RUNTIME_DATABASE_URL must be set for PostgreSQL provisioning tests");
    let runtime_opts: PgConnectOptions = runtime_url
        .parse::<PgConnectOptions>()
        .expect("invalid RUNTIME_DATABASE_URL")
        .options([("search_path", schema_name.as_str())]);
    let runtime_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(runtime_opts)
        .await
        .expect("connect runtime test pool");
    let identity: String = query_scalar("SELECT current_user")
        .fetch_one(&runtime_pool)
        .await
        .expect("read runtime identity");
    let session_identity: String = query_scalar("SELECT session_user")
        .fetch_one(&runtime_pool)
        .await
        .expect("read runtime session identity");
    assert_eq!(identity, "app_runtime");
    assert_eq!(session_identity, "app_runtime");

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

async fn seed_invited_member(ctx: &TestContext, args: &ProvisionArgs) -> MembershipId {
    let member_id = MembershipId::new(format!("member-{}", Uuid::new_v4().simple())).unwrap();
    let user_id = TenantUserId::new(format!("user-{}", Uuid::new_v4().simple())).unwrap();
    query(
        "INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ($1, $2, $3, 'INVITED', 1)",
    )
    .bind(member_id.as_str())
    .bind(args.org_id.as_str())
    .bind(user_id.as_str())
    .execute(&ctx.admin_pool)
    .await
    .expect("seed invited member");
    member_id
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

/// R3/R4 regression: the approved pending-acceptance transition updates
/// authoritative PostgreSQL state in the same transaction as its mandatory
/// audit and outbox records.
#[tokio::test]
async fn mark_member_pending_commits_state_audit_and_outbox_together() {
    let ctx = setup().await;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args();
    service
        .provision_organization(
            args.org_id.clone(),
            "Acme Corp",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await
        .expect("provision the tenant before seeding the invited member");

    let member_id = seed_invited_member(&ctx, &args).await;

    let result = service
        .mark_member_pending(
            &args.org_id,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership_id.to_string()),
                device_ref: None,
            },
        )
        .await;
    let duplicate_result = service
        .mark_member_pending(
            &args.org_id,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership_id.to_string()),
                device_ref: None,
            },
        )
        .await;
    let membership: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(args.org_id.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    .expect("read transitioned membership");
    let event_count: i64 = query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.pending_acceptance' AND target_ref = $2",
    )
    .bind(args.org_id.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    .expect("count pending audit event");
    let outbox_payload: serde_json::Value = query_scalar(
        "SELECT payload FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.membership.pending_acceptance' AND aggregate_id = $2",
    )
    .bind(args.org_id.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    .expect("read pending outbox payload");
    teardown(&ctx).await;

    result.expect("membership transition should succeed");
    assert!(matches!(
        duplicate_result,
        Err(TenancyError::InvalidTransition)
    ));
    assert_eq!(membership, ("PENDING_ACCEPTANCE".to_owned(), 2));
    assert_eq!(event_count, 1);
    assert_eq!(outbox_payload["membership_id"], member_id.as_str());
    assert_eq!(outbox_payload["from_state"], "INVITED");
    assert_eq!(outbox_payload["to_state"], "PENDING_ACCEPTANCE");
    assert_eq!(outbox_payload["state_version"], 2);
}

#[tokio::test]
async fn mark_member_pending_rolls_back_state_and_audit_when_outbox_fails() {
    let ctx = setup().await;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args();
    service
        .provision_organization(
            args.org_id.clone(),
            "Acme Corp",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await
        .expect("provision tenant");
    let member_id = seed_invited_member(&ctx, &args).await;
    raw_sql("ALTER TABLE outbox_events RENAME TO outbox_events_disabled")
        .execute(&ctx.admin_pool)
        .await
        .expect("induce an outbox write failure");

    let result = service
        .mark_member_pending(
            &args.org_id,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership_id.to_string()),
                device_ref: None,
            },
        )
        .await;
    let membership: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(args.org_id.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    .expect("read membership after failed transition");
    let audit_count: i64 = query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.pending_acceptance' AND target_ref = $2",
    )
    .bind(args.org_id.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    .expect("count rolled-back audit events");
    teardown(&ctx).await;

    assert!(matches!(
        result,
        Err(TenancyError::EvidencePersistenceFailed)
    ));
    assert_eq!(membership, ("INVITED".to_owned(), 1));
    assert_eq!(audit_count, 0);
}

#[tokio::test]
async fn mark_member_pending_rolls_back_state_when_audit_fails() {
    let ctx = setup().await;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args();
    service
        .provision_organization(
            args.org_id.clone(),
            "Acme Corp",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await
        .expect("provision tenant");
    let member_id = seed_invited_member(&ctx, &args).await;
    raw_sql("ALTER TABLE audit_events RENAME TO audit_events_disabled")
        .execute(&ctx.admin_pool)
        .await
        .expect("induce an audit write failure");

    let result = service
        .mark_member_pending(
            &args.org_id,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership_id.to_string()),
                device_ref: None,
            },
        )
        .await;
    let membership: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(args.org_id.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    .expect("read membership after failed transition");
    teardown(&ctx).await;

    assert!(matches!(
        result,
        Err(TenancyError::EvidencePersistenceFailed)
    ));
    assert_eq!(membership, ("INVITED".to_owned(), 1));
}

#[tokio::test]
async fn expire_invitation_commits_membership_audit_and_outbox_together() {
    let ctx = setup().await;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args();
    service
        .provision_organization(
            args.org_id.clone(),
            "Acme Corp",
            args.membership_id.clone(),
            args.user_id.clone(),
            args.branch_id.clone(),
            "HQ",
        )
        .await
        .expect("provision tenant");
    let member_id = seed_invited_member(&ctx, &args).await;
    service
        .mark_member_pending(
            &args.org_id,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership_id.to_string()),
                device_ref: None,
            },
        )
        .await
        .expect("move membership to pending acceptance");

    let result = service.expire_invitation(&args.org_id, &member_id).await;
    let membership: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(args.org_id.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    .expect("read expired membership");
    let event: (String, String, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT event_name, actor_subject_ref, reason_class FROM audit_events WHERE organization_id = $1 AND target_ref = $2 AND event_name = 'iam.membership.expired'",
    )
    .bind(args.org_id.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    .expect("read expiry audit event");
    let payload: serde_json::Value = query_scalar(
        "SELECT payload FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.membership.expired' AND aggregate_id = $2",
    )
    .bind(args.org_id.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    .expect("read expiry payload");
    teardown(&ctx).await;

    result.expect("expiry should succeed");
    assert_eq!(membership, ("EXPIRED".to_owned(), 3));
    assert_eq!(event.0, "iam.membership.expired");
    assert_eq!(event.1, "system:invitation-expiry");
    assert_eq!(event.2, None);
    assert_eq!(payload["membership_id"], member_id.as_str());
    assert_eq!(payload["from_state"], "PENDING_ACCEPTANCE");
    assert_eq!(payload["to_state"], "EXPIRED");
    assert_eq!(payload["state_version"], 3);
    assert_eq!(payload["expiry_reason"], "enrollment_window_elapsed");
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

/// Negative regression for the Part 8 verifier: a SELECT policy that keeps the
/// table, role, and command but removes the tenant-context comparison must
/// fail. `id IS NOT NULL` / `organization_id IS NOT NULL` satisfied the old
/// substring helper, so these cases exercise the exact-predicate replacement.
#[tokio::test]
async fn part8_verifier_rejects_select_policies_without_tenant_context() {
    for (table, policy, expression) in [
        (
            "organizations",
            "organizations_provision_read",
            "id IS NOT NULL",
        ),
        (
            "organization_memberships",
            "memberships_provision_read",
            "organization_id IS NOT NULL",
        ),
        (
            "branches",
            "branches_provision_read",
            "organization_id IS NOT NULL",
        ),
    ] {
        let ctx = setup().await;
        raw_sql(&format!(
            "DROP POLICY {policy} ON {table}; \
             CREATE POLICY {policy} ON {table} FOR SELECT TO app_runtime USING ({expression})"
        ))
        .execute(&ctx.admin_pool)
        .await
        .expect("replace the SELECT policy with a tenant-blind expression");
        let verified =
            verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
        teardown(&ctx).await;
        assert!(
            verified.is_err(),
            "verifier accepted a SELECT policy without tenant context on {table}"
        );
    }
}

/// Negative regression for INSERT: the old helper accepted `id <> ''` because
/// it contained the tenant column and the non-empty guard. The canonical
/// predicate must reject it because it never compares against
/// `current_setting('app.organization_id', true)`.
#[tokio::test]
async fn part8_verifier_rejects_insert_policies_without_tenant_context() {
    for (table, policy, expression) in [
        (
            "organizations",
            "organizations_provision_insert",
            "id <> ''",
        ),
        (
            "organization_memberships",
            "memberships_provision_insert",
            "organization_id <> ''",
        ),
        (
            "branches",
            "branches_provision_insert",
            "organization_id <> ''",
        ),
    ] {
        let ctx = setup().await;
        raw_sql(&format!(
            "DROP POLICY {policy} ON {table}; \
             CREATE POLICY {policy} ON {table} FOR INSERT TO app_runtime WITH CHECK ({expression})"
        ))
        .execute(&ctx.admin_pool)
        .await
        .expect("replace the INSERT policy with a tenant-blind expression");
        let verified =
            verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
        teardown(&ctx).await;
        assert!(
            verified.is_err(),
            "verifier accepted an INSERT policy without tenant context on {table}"
        );
    }
}

#[tokio::test]
async fn part8_verifier_rejects_membership_update_policy_without_tenant_context() {
    let ctx = setup().await;
    raw_sql(
        "DROP POLICY memberships_transition_update ON organization_memberships; \
         CREATE POLICY memberships_transition_update ON organization_memberships \
         FOR UPDATE TO app_runtime USING (organization_id IS NOT NULL) \
         WITH CHECK (organization_id IS NOT NULL)",
    )
    .execute(&ctx.admin_pool)
    .await
    .expect("replace the UPDATE policy with a tenant-blind expression");
    let verified = verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
    teardown(&ctx).await;
    assert!(
        verified.is_err(),
        "verifier accepted a membership UPDATE policy without tenant context"
    );
}

#[tokio::test]
async fn part8_verifier_rejects_broad_membership_update_grant() {
    let ctx = setup().await;
    raw_sql("GRANT UPDATE ON organization_memberships TO app_runtime")
        .execute(&ctx.admin_pool)
        .await
        .expect("widen the runtime UPDATE grant");
    let verified = verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
    teardown(&ctx).await;
    assert!(
        verified.is_err(),
        "verifier accepted a table-wide membership UPDATE grant"
    );
}
