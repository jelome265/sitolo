//! Phase 4 Part 8 Real PostgreSQL Integration Tests.
//!
//! These tests prove the transactional coupling, RLS security, and worker
//! behavior using real PostgreSQL. No mocks.
//!
//! Prerequisites:
//! - `ADMIN_DATABASE_URL` (or `DATABASE_URL`) connects as a role that can
//!   create schemas and roles, transfer table/view ownership to
//!   `app_outbox_owner`, and grant privileges.
//! - `RUNTIME_DATABASE_URL` and `WORKER_DATABASE_URL` connect as login roles
//!   that may `SET ROLE app_runtime` and `SET ROLE app_worker` respectively
//!   (role membership or superuser). The fixture creates the NOLOGIN
//!   application roles. Tests fail rather than skip when a variable is absent
//!   or an identity differs.

use std::env;
use std::str::FromStr;
use std::time::Duration;

use sitolo_audit::{ActorRef, IamAuditEvent, IamEventName, TargetRef};
use sitolo_auth::AuditEventId;
use sitolo_domain::tenancy::OrganizationId;
use sitolo_events::{MAX_ATTEMPTS, OutboxClaimPolicy, OutboxErrorClass, OutboxEvent};
use sitolo_persistence::audit_repository::{AuditWriter, PostgresAuditWriter};
use sitolo_persistence::outbox_repository::{
    OutboxPersistenceError, OutboxReader, OutboxWriter, PostgresOutboxRepository,
};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

/// Test context with admin, runtime, and worker pools.
struct TestContext {
    admin_pool: PgPool,
    runtime_pool: PgPool,
    worker_pool: PgPool,
    schema_name: String,
}

/// Catalog row proving the security-definer hardening of one function.
#[derive(Debug, sqlx::FromRow)]
struct FunctionSecurityRow {
    proname: String,
    owner: String,
    prosecdef: bool,
    proconfig: Option<Vec<String>>,
    acl_is_explicit: bool,
    worker_can_execute: bool,
    runtime_can_execute: bool,
    public_can_execute: bool,
}

/// Catalog row proving ownership and RLS posture of one relation.
#[derive(Debug, sqlx::FromRow)]
struct RelationSecurityRow {
    relname: String,
    owner: String,
    relrowsecurity: bool,
    relforcerowsecurity: bool,
}

/// Sets up an isolated schema with Part 8 tables, roles, and RLS policies.
///
/// `RUNTIME_DATABASE_URL` and `WORKER_DATABASE_URL` are both required. The
/// worker identity is deliberately not allowed to fall back to the runtime
/// identity: the tests must prove the narrow worker role, not the superuser.
async fn setup_test_context() -> Result<TestContext, Box<dyn std::error::Error>> {
    let admin_url = env::var("ADMIN_DATABASE_URL").or_else(|_| env::var("DATABASE_URL"))?;
    let runtime_url = env::var("RUNTIME_DATABASE_URL")?;
    let worker_url = env::var("WORKER_DATABASE_URL")?;

    let schema_id = Uuid::new_v4().simple().to_string();
    let schema_name = format!("test_part8_{}", schema_id);

    // The schema must exist before a pool can set it as its search_path.
    let bootstrap = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await?;
    sqlx::raw_sql(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema_name))
        .execute(&bootstrap)
        .await?;
    bootstrap.close().await;

    let admin_opts =
        PgConnectOptions::from_str(&admin_url)?.options([("search_path", schema_name.as_str())]);
    let admin_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(admin_opts)
        .await?;

    // Apply the complete Part 8 schema. The fixture creates the roles before
    // the runtime and worker pools attempt SET ROLE.
    let schema_sql = include_str!("fixtures/part8_audit_outbox_schema.sql")
        .replace("__SCHEMA__", &format!("\"{}\"", schema_name));
    sqlx::raw_sql(&schema_sql).execute(&admin_pool).await?;

    let runtime_opts =
        PgConnectOptions::from_str(&runtime_url)?.options([("search_path", schema_name.as_str())]);
    let runtime_pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE app_runtime").execute(conn).await?;
                Ok(())
            })
        })
        .connect_with(runtime_opts)
        .await?;

    let worker_opts =
        PgConnectOptions::from_str(&worker_url)?.options([("search_path", schema_name.as_str())]);
    let worker_pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE app_worker").execute(conn).await?;
                Ok(())
            })
        })
        .connect_with(worker_opts)
        .await?;

    // Prove the pools execute as the intended roles. A worker test that
    // silently runs as app_runtime (or as the migration superuser) would not
    // prove the worker privilege boundary.
    let runtime_identity: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&runtime_pool)
        .await?;
    if runtime_identity != "app_runtime" {
        return Err(
            format!("runtime pool must execute as app_runtime, got {runtime_identity}").into(),
        );
    }
    let worker_identity: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&worker_pool)
        .await?;
    if worker_identity != "app_worker" {
        return Err(
            format!("worker pool must execute as app_worker, got {worker_identity}").into(),
        );
    }

    Ok(TestContext {
        admin_pool,
        runtime_pool,
        worker_pool,
        schema_name,
    })
}

/// Builds the bounded claim policy used by the worker tests.
fn claim_policy(batch_size: usize, lease_secs: u64) -> OutboxClaimPolicy {
    OutboxClaimPolicy::new(batch_size, Duration::from_secs(lease_secs), MAX_ATTEMPTS)
        .expect("test claim policy is within the configured bounds")
}

/// Enqueues one pending event through the request transaction path.
async fn enqueue_pending_event(
    ctx: &TestContext,
    organization_id: &OrganizationId,
    event_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let event = OutboxEvent::new(
        event_id.to_owned(),
        "Organization".to_owned(),
        organization_id.to_string(),
        "iam.organization.created".to_owned(),
        1,
        organization_id.clone(),
        None,
        r#"{}"#.to_owned(),
    )?;
    let writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());
    let mut tx = ctx.admin_pool.begin().await?;
    sqlx::query("SELECT set_config('app.organization_id', $1, true)")
        .bind(organization_id.to_string())
        .execute(&mut *tx)
        .await?;
    writer.enqueue(&mut tx, &event).await?;
    tx.commit().await?;
    Ok(())
}

/// Tears down test schema.
async fn teardown_test_context(ctx: &TestContext) {
    let _ = sqlx::raw_sql(&format!(
        "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
        ctx.schema_name
    ))
    .execute(&ctx.admin_pool)
    .await;
}

// ============================================================================
// ATOMICITY TESTS (Contract §13)
// ============================================================================

/// Test A: A mandatory audit write failure rolls back the transaction.
#[tokio::test]
async fn test_atomicity_business_succeeds_audit_fails() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-atomic-1").unwrap();
        let event_id = AuditEventId::new(format!("evt-{}", Uuid::new_v4())).unwrap();

        let audit_event = IamAuditEvent::success(
            event_id.clone(),
            IamEventName::OrganizationCreated,
            org_id.clone(),
            None,
            ActorRef {
                subject_ref: "subject-1".into(),
                membership_ref: None,
                device_ref: None,
            },
            TargetRef::Organization(org_id.clone()),
            "provision",
            "tenancy_service",
        );

        let audit_writer = PostgresAuditWriter::new();
        let mut tx = ctx.admin_pool.begin().await?;

        // Set tenant context
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-atomic-1'")
            .execute(&mut *tx)
            .await?;

        // The first evidence write succeeds.
        audit_writer.record_required(&mut tx, &audit_event).await?;

        // Inject a deterministic audit failure: the same primary key twice.
        let duplicate = audit_writer.record_required(&mut tx, &audit_event).await;
        assert!(
            duplicate.is_err(),
            "duplicate audit event must fail inside the transaction"
        );

        tx.rollback().await?;

        // Verify the first audit write did not survive the failure.
        let count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {}.audit_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(event_id.to_string())
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(count, 0, "Audit event must not exist after rollback");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test B: An outbox write failure rolls back the audit and outbox writes.
#[tokio::test]
async fn test_atomicity_business_succeeds_outbox_fails() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-atomic-2").unwrap();
        let event_id = AuditEventId::new(format!("evt-{}", Uuid::new_v4())).unwrap();
        let outbox_id = format!("outbox-{}", Uuid::new_v4());

        let audit_event = IamAuditEvent::success(
            event_id.clone(),
            IamEventName::OrganizationCreated,
            org_id.clone(),
            None,
            ActorRef {
                subject_ref: "subject-1".into(),
                membership_ref: None,
                device_ref: None,
            },
            TargetRef::Organization(org_id.clone()),
            "provision",
            "tenancy_service",
        );
        let outbox_event = OutboxEvent::new(
            outbox_id.clone(),
            "Organization".into(),
            org_id.to_string(),
            "iam.organization.created".into(),
            1,
            org_id.clone(),
            None,
            r#"{"organization_id":"org-atomic-2","name":"Test Org"}"#.into(),
        )
        .unwrap();

        let audit_writer = PostgresAuditWriter::new();
        let outbox_writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());
        let mut tx = ctx.admin_pool.begin().await?;

        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-atomic-2'")
            .execute(&mut *tx)
            .await?;

        // Both writes succeed before the injected failure.
        audit_writer.record_required(&mut tx, &audit_event).await?;
        outbox_writer.enqueue(&mut tx, &outbox_event).await?;

        // Inject a deterministic outbox failure: duplicate primary key.
        let duplicate = outbox_writer.enqueue(&mut tx, &outbox_event).await;
        assert!(
            duplicate.is_err(),
            "duplicate outbox event must fail inside the transaction"
        );

        tx.rollback().await?;

        // Verify neither the audit nor the successful outbox write survived.
        let audit_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {}.audit_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(event_id.to_string())
        .fetch_one(&ctx.admin_pool)
        .await?;

        let outbox_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {}.outbox_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(audit_count, 0, "Audit must not exist after rollback");
        assert_eq!(outbox_count, 0, "Outbox must not exist after rollback");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test C: All writes succeed -> COMMIT
#[tokio::test]
async fn test_atomicity_all_succeed_commit() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-atomic-3").unwrap();
        let event_id = AuditEventId::new(format!("evt-{}", Uuid::new_v4())).unwrap();
        let outbox_id = format!("outbox-{}", Uuid::new_v4());

        let audit_event = IamAuditEvent::success(
            event_id.clone(),
            IamEventName::OrganizationCreated,
            org_id.clone(),
            None,
            ActorRef {
                subject_ref: "subject-1".into(),
                membership_ref: None,
                device_ref: None,
            },
            TargetRef::Organization(org_id.clone()),
            "provision",
            "tenancy_service",
        );

        let outbox_event = OutboxEvent::new(
            outbox_id.clone(),
            "Organization".into(),
            org_id.to_string(),
            "iam.organization.created".into(),
            1,
            org_id.clone(),
            None,
            r#"{"organization_id":"org-atomic-3","name":"Test Org"}"#.into(),
        )
        .unwrap();

        let audit_writer = PostgresAuditWriter::new();
        let outbox_writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());

        let mut tx = ctx.admin_pool.begin().await?;

        // Set tenant context
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-atomic-3'")
            .execute(&mut *tx)
            .await?;

        // Record audit
        audit_writer.record_required(&mut tx, &audit_event).await?;

        // Enqueue outbox
        outbox_writer.enqueue(&mut tx, &outbox_event).await?;

        // Commit
        tx.commit().await?;

        // Verify both exist
        let audit_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {}.audit_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(event_id.to_string())
        .fetch_one(&ctx.admin_pool)
        .await?;

        let outbox_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {}.outbox_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(audit_count, 1, "Audit must exist after commit");
        assert_eq!(outbox_count, 1, "Outbox must exist after commit");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

// ============================================================================
// SECURITY TESTS (Contract §14, §62)
// ============================================================================

/// Test: Missing tenant context fails closed
#[tokio::test]
async fn test_missing_tenant_context_fails_closed() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let event_id = format!("evt-{}", Uuid::new_v4());

        // No SET LOCAL app.organization_id
        let insert_result = sqlx::query(&format!(
            "INSERT INTO {}.audit_events (event_id, event_name, event_version, organization_id, target_type, target_ref, action, result, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            ctx.schema_name
        ))
        .bind(&event_id)
        .bind("iam.organization.created")
        .bind(1i32)
        .bind("org-1")
        .bind("Organization")
        .bind("org-1")
        .bind("provision")
        .bind("SUCCESS")
        .bind("tenancy_service")
        .execute(&ctx.runtime_pool)
        .await;

        assert!(insert_result.is_err(), "Insert must fail without tenant context");

        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: Invalid tenant context fails closed
#[tokio::test]
async fn test_invalid_tenant_context_fails_closed() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let event_id = format!("evt-{}", Uuid::new_v4());
        let mut tx = ctx.runtime_pool.begin().await?;

        // Set context to org-A on the same connection, then write org-B.
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-A'")
            .execute(&mut *tx)
            .await?;

        let insert_result = sqlx::query(&format!(
            "INSERT INTO {}.audit_events (event_id, event_name, event_version, organization_id, target_type, target_ref, action, result, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            ctx.schema_name
        ))
        .bind(&event_id)
        .bind("iam.organization.created")
        .bind(1i32)
        .bind("org-B") // Mismatched
        .bind("Organization")
        .bind("org-B")
        .bind("provision")
        .bind("SUCCESS")
        .bind("tenancy_service")
        .execute(&mut *tx)
        .await;

        assert!(
            insert_result.is_err(),
            "Insert must fail with mismatched tenant context"
        );
        tx.rollback().await?;

        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: Runtime cannot UPDATE audit records
#[tokio::test]
async fn test_runtime_cannot_update_audit() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let event_id = format!("evt-{}", Uuid::new_v4());

        // Admin inserts audit record
        sqlx::query(&format!(
            "INSERT INTO {}.audit_events (event_id, event_name, event_version, organization_id, target_type, target_ref, action, result, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            ctx.schema_name
        ))
        .bind(&event_id)
        .bind("iam.organization.created")
        .bind(1i32)
        .bind("org-1")
        .bind("Organization")
        .bind("org-1")
        .bind("provision")
        .bind("SUCCESS")
        .bind("tenancy_service")
        .execute(&ctx.admin_pool)
        .await?;

        // Runtime tries to update within a valid tenant context.
        let mut tx = ctx.runtime_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-1'")
            .execute(&mut *tx)
            .await?;

        let update_result = sqlx::query(&format!(
            "UPDATE {}.audit_events SET result = 'FAILURE' WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&event_id)
        .execute(&mut *tx)
        .await;

        assert!(
            update_result.is_err(),
            "Runtime must not UPDATE audit records"
        );
        tx.rollback().await?;

        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: Runtime cannot DELETE audit records
#[tokio::test]
async fn test_runtime_cannot_delete_audit() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let event_id = format!("evt-{}", Uuid::new_v4());

        sqlx::query(&format!(
            "INSERT INTO {}.audit_events (event_id, event_name, event_version, organization_id, target_type, target_ref, action, result, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            ctx.schema_name
        ))
        .bind(&event_id)
        .bind("iam.organization.created")
        .bind(1i32)
        .bind("org-1")
        .bind("Organization")
        .bind("org-1")
        .bind("provision")
        .bind("SUCCESS")
        .bind("tenancy_service")
        .execute(&ctx.admin_pool)
        .await?;

        // Runtime tries to delete within a valid tenant context.
        let mut tx = ctx.runtime_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-1'")
            .execute(&mut *tx)
            .await?;

        let delete_result = sqlx::query(&format!(
            "DELETE FROM {}.audit_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&event_id)
        .execute(&mut *tx)
        .await;

        assert!(
            delete_result.is_err(),
            "Runtime must not DELETE audit records"
        );
        tx.rollback().await?;

        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

// ============================================================================
// CONCURRENCY TESTS (Contract §15, §63)
// ============================================================================

/// Test: Two workers claim same event - only one succeeds
#[tokio::test]
async fn test_concurrent_worker_claim() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-concurrent").unwrap();
        let outbox_id = format!("outbox-{}", Uuid::new_v4());

        // Insert PENDING event
        let outbox_event = OutboxEvent::new(
            outbox_id.clone(),
            "Organization".into(),
            org_id.to_string(),
            "iam.organization.created".into(),
            1,
            org_id,
            None,
            r#"{}"#.into(),
        )
        .unwrap();

        let outbox_writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());
        let mut tx = ctx.admin_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-concurrent'")
            .execute(&mut *tx)
            .await?;
        outbox_writer.enqueue(&mut tx, &outbox_event).await?;
        tx.commit().await?;

        // Two workers try to claim simultaneously
        let outbox_reader = PostgresOutboxRepository::new(ctx.worker_pool.clone());

        let (result1, result2) = tokio::join!(
            outbox_reader.claim_batch(claim_policy(1, 300)),
            outbox_reader.claim_batch(claim_policy(1, 300))
        );

        let claimed1 = result1.unwrap();
        let claimed2 = result2.unwrap();

        // Exactly one worker should claim the event
        let total_claims = claimed1.len() + claimed2.len();
        assert_eq!(total_claims, 1, "Exactly one worker must claim the event");

        // Verify event is now CLAIMED
        let status: String = sqlx::query_scalar(&format!(
            "SELECT status FROM {}.outbox_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(status, "CLAIMED", "Event must be in CLAIMED state");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: Lease recovery after worker crash
#[tokio::test]
async fn test_lease_recovery() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-lease").unwrap();
        let outbox_id = format!("outbox-{}", Uuid::new_v4());

        let outbox_event = OutboxEvent::new(
            outbox_id.clone(),
            "Organization".into(),
            org_id.to_string(),
            "iam.organization.created".into(),
            1,
            org_id,
            None,
            r#"{}"#.into(),
        )
        .unwrap();

        let outbox_writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());
        let mut tx = ctx.admin_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-lease'")
            .execute(&mut *tx)
            .await?;
        outbox_writer.enqueue(&mut tx, &outbox_event).await?;
        tx.commit().await?;

        let outbox_reader = PostgresOutboxRepository::new(ctx.worker_pool.clone());

        // Claim with very short lease
        let claimed = outbox_reader.claim_batch(claim_policy(1, 1)).await?;
        assert_eq!(claimed.len(), 1);
        let stale_token = claimed
            .first()
            .and_then(|event| event.claim_token.as_ref())
            .expect("claimed event carries a claim token")
            .clone();

        // Wait for lease to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Recover stale claims
        let recovered = outbox_reader
            .recover_stale_claims(Duration::from_secs(1))
            .await?;
        assert!(recovered > 0, "Stale claims must be recovered");

        // A worker that stalled before recovery cannot publish afterwards.
        let stale_publish = outbox_reader.mark_published(&outbox_id, &stale_token).await;
        assert!(
            matches!(stale_publish, Err(OutboxPersistenceError::ClaimLost)),
            "stale claim token must be fenced after lease recovery"
        );

        // Verify event is back to PENDING
        let status: String = sqlx::query_scalar(&format!(
            "SELECT status FROM {}.outbox_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(
            status, "PENDING",
            "Event must be back to PENDING after lease recovery"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: Worker cannot rewrite immutable event fields
#[tokio::test]
async fn test_worker_cannot_rewrite_immutable_fields() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-worker-test").unwrap();
        let outbox_id = format!("outbox-{}", Uuid::new_v4());

        let outbox_event = OutboxEvent::new(
            outbox_id.clone(),
            "Organization".into(),
            org_id.to_string(),
            "iam.organization.created".into(),
            1,
            org_id.clone(),
            None,
            r#"{}"#.into(),
        )
        .unwrap();

        let outbox_writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());
        let mut tx = ctx.admin_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-worker-test'")
            .execute(&mut *tx)
            .await?;
        outbox_writer.enqueue(&mut tx, &outbox_event).await?;
        tx.commit().await?;

        // Worker tries to update organization_id (immutable field)
        let update_result = sqlx::query(&format!(
            "UPDATE {}.outbox_events SET organization_id = 'org-HACKED' WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .execute(&ctx.worker_pool)
        .await;

        assert!(
            update_result.is_err(),
            "Worker must not rewrite immutable fields"
        );

        // Verify organization_id unchanged
        let org: String = sqlx::query_scalar(&format!(
            "SELECT organization_id FROM {}.outbox_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(org, "org-worker-test", "Immutable field must not change");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: A published event is terminal and cannot be claimed again.
///
/// This is not the duplicate-delivery/idempotent-consumer proof: that requires
/// the worker relay and a real consumer (blocked on the delivery destination).
#[tokio::test]
async fn test_published_event_is_not_reclaimable() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-dup").unwrap();
        let outbox_id = format!("outbox-{}", Uuid::new_v4());

        let outbox_event = OutboxEvent::new(
            outbox_id.clone(),
            "Organization".into(),
            org_id.to_string(),
            "iam.organization.created".into(),
            1,
            org_id,
            None,
            r#"{}"#.into(),
        )
        .unwrap();

        let outbox_writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());
        let mut tx = ctx.admin_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-dup'")
            .execute(&mut *tx)
            .await?;
        outbox_writer.enqueue(&mut tx, &outbox_event).await?;
        tx.commit().await?;

        let outbox_reader = PostgresOutboxRepository::new(ctx.worker_pool.clone());

        // Claim event
        let claimed = outbox_reader.claim_batch(claim_policy(1, 300)).await?;
        assert_eq!(claimed.len(), 1);
        let claim_token = claimed
            .first()
            .and_then(|event| event.claim_token.as_ref())
            .expect("claimed event carries a claim token");

        // Mark as published
        outbox_reader
            .mark_published(&outbox_id, claim_token)
            .await?;

        // Simulate crash before marking - try to claim again
        // Event should be PUBLISHED, not claimable
        let claimed2 = outbox_reader.claim_batch(claim_policy(1, 300)).await?;
        assert_eq!(
            claimed2.len(),
            0,
            "Published event must not be claimable again"
        );

        // Verify status is PUBLISHED
        let status: String = sqlx::query_scalar(&format!(
            "SELECT status FROM {}.outbox_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(status, "PUBLISHED");

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: concurrent enqueues for one aggregate receive a unique, gap-free
/// database-assigned sequence. This is the regression test for the former
/// `MAX(aggregate_sequence) + 1` race.
#[tokio::test]
async fn test_concurrent_aggregate_sequence_assignment() {
    const EVENT_COUNT: usize = 8;

    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-sequence").unwrap();
        let aggregate_id = org_id.to_string();
        let mut tasks = tokio::task::JoinSet::new();

        for _ in 0..EVENT_COUNT {
            let pool = ctx.runtime_pool.clone();
            let org_id = org_id.clone();
            let aggregate_id = aggregate_id.clone();
            tasks.spawn(async move {
                let event = OutboxEvent::new(
                    format!("outbox-seq-{}", Uuid::new_v4()),
                    "Organization".to_owned(),
                    aggregate_id,
                    "iam.organization.created".to_owned(),
                    1,
                    org_id,
                    None,
                    r#"{}"#.to_owned(),
                )?;
                let organization_id = event.organization_id.to_string();
                let writer = PostgresOutboxRepository::new(pool.clone());
                let mut tx = pool.begin().await?;
                sqlx::query("SELECT set_config('app.organization_id', $1, true)")
                    .bind(organization_id)
                    .execute(&mut *tx)
                    .await?;
                writer.enqueue(&mut tx, &event).await?;
                tx.commit().await?;
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            });
        }

        while let Some(joined) = tasks.join_next().await {
            joined??;
        }

        let sequences: Vec<i64> = sqlx::query_scalar(
            "SELECT aggregate_sequence \
             FROM outbox_events \
             WHERE aggregate_type = $1 AND aggregate_id = $2 \
             ORDER BY aggregate_sequence",
        )
        .bind("Organization")
        .bind(&aggregate_id)
        .fetch_all(&ctx.admin_pool)
        .await?;

        let upper_bound = i64::try_from(EVENT_COUNT)?;
        let expected: Vec<i64> = (1..=upper_bound).collect();
        assert_eq!(
            sequences, expected,
            "concurrent enqueues must produce unique contiguous sequences"
        );

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: same-aggregate claims follow the database-assigned sequence. Only the
/// aggregate head is claimable, so delivery order cannot be inverted by
/// wall-clock availability.
#[tokio::test]
async fn test_same_aggregate_claims_follow_sequence() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-order").unwrap();
        let first_id = format!("outbox-{}", Uuid::new_v4());
        let second_id = format!("outbox-{}", Uuid::new_v4());
        enqueue_pending_event(&ctx, &org_id, &first_id).await?;
        enqueue_pending_event(&ctx, &org_id, &second_id).await?;

        let reader = PostgresOutboxRepository::new(ctx.worker_pool.clone());

        let first_claim = reader.claim_batch(claim_policy(10, 300)).await?;
        assert_eq!(
            first_claim.len(),
            1,
            "only the aggregate head may be claimed"
        );
        assert_eq!(first_claim[0].event_id, first_id);
        assert_eq!(
            first_claim[0].aggregate_sequence,
            Some(1),
            "the first event carries aggregate sequence one"
        );
        let first_token = first_claim[0]
            .claim_token
            .as_ref()
            .expect("claimed event carries a claim token")
            .clone();
        reader.mark_published(&first_id, &first_token).await?;

        let second_claim = reader.claim_batch(claim_policy(10, 300)).await?;
        assert_eq!(
            second_claim.len(),
            1,
            "the successor becomes claimable after publication"
        );
        assert_eq!(second_claim[0].event_id, second_id);
        assert_eq!(
            second_claim[0].aggregate_sequence,
            Some(2),
            "the successor carries aggregate sequence two"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

// ============================================================================
// RETRY AND QUARANTINE TESTS (Contract §23, §24, §40)
// ============================================================================

/// Test: A retryable failure reschedules the event and fences the old token.
#[tokio::test]
async fn test_release_for_retry_reschedules_and_fences() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-retry").unwrap();
        let outbox_id = format!("outbox-{}", Uuid::new_v4());
        enqueue_pending_event(&ctx, &org_id, &outbox_id).await?;

        let reader = PostgresOutboxRepository::new(ctx.worker_pool.clone());
        let claimed = reader.claim_batch(claim_policy(1, 300)).await?;
        let token = claimed
            .first()
            .and_then(|event| event.claim_token.as_ref())
            .expect("claimed event carries a claim token")
            .clone();
        let error_class = OutboxErrorClass::new("http_5xx").expect("valid error class");

        reader
            .release_for_retry(&outbox_id, &token, &error_class, Duration::from_secs(60))
            .await?;

        let (status, attempt_count, claim_token, last_error_class): (
            String,
            i32,
            Option<String>,
            Option<String>,
        ) = sqlx::query_as(
            "SELECT status, attempt_count, claim_token, last_error_class \
             FROM outbox_events WHERE event_id = $1",
        )
        .bind(&outbox_id)
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(status, "PENDING", "released event returns to PENDING");
        assert_eq!(attempt_count, 1, "the failed attempt is still counted");
        assert!(claim_token.is_none(), "release clears the claim token");
        assert_eq!(
            last_error_class.as_deref(),
            Some("http_5xx"),
            "release records the bounded error class"
        );

        let reclaimed = reader.claim_batch(claim_policy(1, 300)).await?;
        assert!(
            reclaimed.is_empty(),
            "event must not be claimable before available_at"
        );

        assert!(
            matches!(
                reader.mark_published(&outbox_id, &token).await,
                Err(OutboxPersistenceError::ClaimLost)
            ),
            "the released claim token must be fenced"
        );

        // Repository-side bounds are enforced before SQL is reached.
        let zero_backoff = reader
            .release_for_retry(&outbox_id, &token, &error_class, Duration::ZERO)
            .await;
        assert!(
            matches!(
                zero_backoff,
                Err(OutboxPersistenceError::InvalidRequest(
                    sitolo_events::OutboxError::InvalidBackoff
                ))
            ),
            "zero backoff must be rejected before reaching PostgreSQL"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: Quarantine is terminal, and the abandoned lease cannot transition.
#[tokio::test]
async fn test_quarantine_is_terminal() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-quarantine").unwrap();
        let outbox_id = format!("outbox-{}", Uuid::new_v4());
        enqueue_pending_event(&ctx, &org_id, &outbox_id).await?;

        let reader = PostgresOutboxRepository::new(ctx.worker_pool.clone());
        let claimed = reader.claim_batch(claim_policy(1, 300)).await?;
        let token = claimed
            .first()
            .and_then(|event| event.claim_token.as_ref())
            .expect("claimed event carries a claim token")
            .clone();
        let error_class = OutboxErrorClass::new("schema_invalid").expect("valid error class");

        reader.quarantine(&outbox_id, &token, &error_class).await?;

        let (status, claim_token, locked_at, last_error_class): (
            String,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
        ) = sqlx::query_as(
            "SELECT status, claim_token, locked_at, last_error_class \
             FROM outbox_events WHERE event_id = $1",
        )
        .bind(&outbox_id)
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(status, "QUARANTINED");
        assert!(claim_token.is_none(), "quarantine clears the claim token");
        assert!(locked_at.is_none(), "quarantine clears the lease time");
        assert_eq!(last_error_class.as_deref(), Some("schema_invalid"));

        let reclaimed = reader.claim_batch(claim_policy(1, 300)).await?;
        assert!(reclaimed.is_empty(), "quarantined events are not claimable");

        assert!(
            matches!(
                reader.mark_published(&outbox_id, &token).await,
                Err(OutboxPersistenceError::ClaimLost)
            ),
            "publish after quarantine must be fenced"
        );
        assert!(
            matches!(
                reader
                    .release_for_retry(&outbox_id, &token, &error_class, Duration::from_secs(1))
                    .await,
                Err(OutboxPersistenceError::ClaimLost)
            ),
            "release after quarantine must be fenced"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: an event that exhausts its attempt budget is quarantined by the claim
/// function and never handed to a worker again.
#[tokio::test]
async fn test_attempt_budget_exhaustion_quarantines() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-budget").unwrap();
        let outbox_id = format!("outbox-{}", Uuid::new_v4());
        enqueue_pending_event(&ctx, &org_id, &outbox_id).await?;

        let reader = PostgresOutboxRepository::new(ctx.worker_pool.clone());
        let single_attempt =
            OutboxClaimPolicy::new(1, Duration::from_secs(300), 1).expect("valid policy");
        let claimed = reader.claim_batch(single_attempt).await?;
        let token = claimed
            .first()
            .and_then(|event| event.claim_token.as_ref())
            .expect("claimed event carries a claim token")
            .clone();

        reader
            .release_for_retry(
                &outbox_id,
                &token,
                &OutboxErrorClass::new("destination_unavailable").expect("valid error class"),
                Duration::from_millis(10),
            )
            .await?;
        tokio::time::sleep(Duration::from_millis(50)).await;

        let exhausted = reader.claim_batch(single_attempt).await?;
        assert!(
            exhausted.is_empty(),
            "an exhausted event must not be claimed again"
        );

        let status: String =
            sqlx::query_scalar("SELECT status FROM outbox_events WHERE event_id = $1")
                .bind(&outbox_id)
                .fetch_one(&ctx.admin_pool)
                .await?;
        assert_eq!(
            status, "QUARANTINED",
            "claim must quarantine an event that exhausted its attempt budget"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

// ============================================================================
// ROLE-BOUNDARY TESTS (Contract §62)
// ============================================================================

/// Extracts the SQLSTATE from a database error, failing loudly when the error
/// is not a PostgreSQL error at all.
fn sqlstate(error: &sqlx::Error) -> String {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .expect("expected a PostgreSQL error with a SQLSTATE")
        .into_owned()
}

/// Test: every SECURITY DEFINER function is owned by the non-login owner,
/// pins a trusted `search_path`, and is executable only by `app_worker`.
#[tokio::test]
async fn test_security_definer_functions_are_hardened() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let rows: Vec<FunctionSecurityRow> = sqlx::query_as(
            "SELECT
                 p.proname,
                 r.rolname AS owner,
                 p.prosecdef,
                 p.proconfig,
                 (p.proacl IS NOT NULL) AS acl_is_explicit,
                 has_function_privilege('app_worker', p.oid, 'EXECUTE') AS worker_can_execute,
                 has_function_privilege('app_runtime', p.oid, 'EXECUTE') AS runtime_can_execute,
                 COALESCE((
                     SELECT bool_or(acl.grantee = 0 AND acl.privilege_type = 'EXECUTE')
                     FROM aclexplode(p.proacl) AS acl
                 ), false) AS public_can_execute
             FROM pg_proc AS p
             JOIN pg_namespace AS n ON n.oid = p.pronamespace
             JOIN pg_roles AS r ON r.oid = p.proowner
             WHERE n.nspname = $1
               AND p.proname IN (
                   'outbox_claim_events',
                   'outbox_mark_published',
                   'outbox_release_for_retry',
                   'outbox_quarantine',
                   'outbox_recover_stale_claims'
               )
             ORDER BY p.proname",
        )
        .bind(&ctx.schema_name)
        .fetch_all(&ctx.admin_pool)
        .await?;

        assert_eq!(
            rows.len(),
            5,
            "all five worker functions must be catalogued"
        );
        for row in &rows {
            assert_eq!(
                row.owner, "app_outbox_owner",
                "{} must be owned by the dedicated non-login owner",
                row.proname
            );
            assert!(row.prosecdef, "{} must be SECURITY DEFINER", row.proname);
            assert!(
                row.acl_is_explicit,
                "{} must have an explicit ACL (PUBLIC revoke materialised)",
                row.proname
            );
            assert!(
                row.worker_can_execute,
                "{} must be executable by app_worker",
                row.proname
            );
            assert!(
                !row.runtime_can_execute,
                "{} must not be executable by app_runtime",
                row.proname
            );
            assert!(
                !row.public_can_execute,
                "{} must not be executable by PUBLIC",
                row.proname
            );

            let config = row
                .proconfig
                .as_ref()
                .unwrap_or_else(|| panic!("{} must pin search_path", row.proname));
            assert!(
                config.iter().any(|entry| {
                    entry.starts_with("search_path=")
                        && entry.contains("pg_catalog")
                        && entry.contains(&ctx.schema_name)
                        && !entry.contains("public")
                        && !entry.contains("pg_temp")
                }),
                "{} must pin search_path to pg_catalog plus the schema, got {config:?}",
                row.proname
            );
        }

        let roles: Vec<(String, bool, bool, bool, bool, bool)> = sqlx::query_as(
            "SELECT rolname, rolsuper, rolbypassrls, rolcanlogin, rolinherit, rolreplication
             FROM pg_roles
             WHERE rolname IN ('app_runtime', 'app_worker')
             ORDER BY rolname",
        )
        .fetch_all(&ctx.admin_pool)
        .await?;
        assert_eq!(roles.len(), 2, "both application roles must exist");
        for (name, is_superuser, bypass_rls, can_login, inherits, replication) in roles {
            assert!(!is_superuser, "{name} must not be a superuser");
            assert!(!bypass_rls, "{name} must not bypass RLS");
            assert!(!can_login, "{name} must be NOLOGIN");
            assert!(!inherits, "{name} must be NOINHERIT");
            assert!(!replication, "{name} must not have replication");
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: Part 8 relations are owned by the dedicated non-login owner, the
/// evidence tables force RLS, and the policies name exact roles only.
#[tokio::test]
async fn test_part8_relation_ownership_and_rls_metadata() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let relations: Vec<RelationSecurityRow> = sqlx::query_as(
            "SELECT
                 c.relname,
                 pg_get_userbyid(c.relowner) AS owner,
                 c.relrowsecurity,
                 c.relforcerowsecurity
             FROM pg_class AS c
             JOIN pg_namespace AS n ON n.oid = c.relnamespace
             WHERE n.nspname = $1
               AND c.relname IN (
                   'audit_events',
                   'outbox_events',
                   'outbox_aggregate_sequences',
                   'outbox_metrics'
               )
             ORDER BY c.relname",
        )
        .bind(&ctx.schema_name)
        .fetch_all(&ctx.admin_pool)
        .await?;
        assert_eq!(relations.len(), 4, "all Part 8 relations must exist");
        for relation in &relations {
            assert_eq!(
                relation.owner, "app_outbox_owner",
                "{} must be owned by the dedicated non-login owner",
                relation.relname
            );
        }

        let audit = relations
            .iter()
            .find(|relation| relation.relname == "audit_events")
            .expect("audit table is catalogued");
        assert!(audit.relrowsecurity, "audit_events must enable RLS");
        assert!(audit.relforcerowsecurity, "audit_events must force RLS");

        let outbox = relations
            .iter()
            .find(|relation| relation.relname == "outbox_events")
            .expect("outbox table is catalogued");
        assert!(outbox.relrowsecurity, "outbox_events must enable RLS");
        assert!(outbox.relforcerowsecurity, "outbox_events must force RLS");

        let policies: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT tablename, policyname, array_to_string(roles, ',') AS roles, cmd
             FROM pg_policies
             WHERE schemaname = $1
             ORDER BY tablename, policyname",
        )
        .bind(&ctx.schema_name)
        .fetch_all(&ctx.admin_pool)
        .await?;
        assert_eq!(policies.len(), 3, "exactly three Part 8 policies expected");
        assert!(
            policies.iter().any(|(table, policy, roles, command)| {
                table == "audit_events"
                    && policy == "audit_events_insert_policy"
                    && roles == "app_runtime"
                    && command == "INSERT"
            }),
            "audit insert policy must target app_runtime only: {policies:?}"
        );
        assert!(
            policies.iter().any(|(table, policy, roles, command)| {
                table == "outbox_events"
                    && policy == "outbox_events_insert_policy"
                    && roles == "app_runtime"
                    && command == "INSERT"
            }),
            "outbox insert policy must target app_runtime only: {policies:?}"
        );
        assert!(
            policies.iter().any(|(table, policy, roles, command)| {
                table == "outbox_events"
                    && policy == "outbox_events_owner_policy"
                    && roles == "app_outbox_owner"
                    && command == "ALL"
            }),
            "outbox owner policy must target app_outbox_owner only: {policies:?}"
        );
        assert!(
            !policies
                .iter()
                .any(|(_, _, roles, _)| roles.contains("app_worker")),
            "no table policy may target the worker role: {policies:?}"
        );

        let memberships: Vec<(String, String)> = sqlx::query_as(
            "SELECT member.rolname, granted.rolname
             FROM pg_auth_members AS membership
             JOIN pg_roles AS member ON member.oid = membership.member
             JOIN pg_roles AS granted ON granted.oid = membership.roleid
             WHERE member.rolname IN ('app_runtime', 'app_worker')",
        )
        .fetch_all(&ctx.admin_pool)
        .await?;
        assert!(
            memberships.is_empty(),
            "application roles must hold no unintended memberships: {memberships:?}"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: the runtime role cannot read evidence tables or truncate them.
#[tokio::test]
async fn test_runtime_cannot_read_or_truncate_evidence() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let audit_read = sqlx::query("SELECT event_id FROM audit_events LIMIT 1")
            .fetch_all(&ctx.runtime_pool)
            .await
            .expect_err("runtime must not read audit events");
        assert_eq!(
            sqlstate(&audit_read),
            "42501",
            "expected insufficient_privilege, got: {audit_read}"
        );

        let outbox_read = sqlx::query("SELECT event_id FROM outbox_events LIMIT 1")
            .fetch_all(&ctx.runtime_pool)
            .await
            .expect_err("runtime must not read outbox events");
        assert_eq!(
            sqlstate(&outbox_read),
            "42501",
            "expected insufficient_privilege, got: {outbox_read}"
        );

        let truncate = sqlx::query("TRUNCATE audit_events")
            .execute(&ctx.runtime_pool)
            .await
            .expect_err("runtime must not truncate audit events");
        assert_eq!(
            sqlstate(&truncate),
            "42501",
            "expected insufficient_privilege, got: {truncate}"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: RLS rejects an outbox insert whose organization does not match the
/// transaction tenant.
#[tokio::test]
async fn test_outbox_cross_tenant_insert_is_denied() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let mut tx = ctx.runtime_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-A'")
            .execute(&mut *tx)
            .await?;

        let insert = sqlx::query(
            "INSERT INTO outbox_events (
                 event_id, aggregate_type, aggregate_id, event_name, event_version,
                 organization_id, payload, deduplication_key, schema_version,
                 aggregate_sequence
             )
             VALUES ($1, 'Organization', $2, 'iam.organization.created', 1, $2,
                     '{}'::jsonb, $3, 1, 1)",
        )
        .bind(format!("outbox-{}", Uuid::new_v4()))
        .bind("org-B")
        .bind(format!("dedup-{}", Uuid::new_v4()))
        .execute(&mut *tx)
        .await;

        let error = insert.expect_err("cross-tenant outbox insert must be denied");
        assert_eq!(
            sqlstate(&error),
            "42501",
            "expected RLS violation, got: {error}"
        );
        tx.rollback().await?;

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: the worker role cannot read the outbox table directly, so one replica
/// cannot observe another replica's claim tokens or payloads.
#[tokio::test]
async fn test_worker_cannot_select_outbox_table() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let error = sqlx::query("SELECT event_id FROM outbox_events LIMIT 1")
            .fetch_one(&ctx.worker_pool)
            .await
            .expect_err("worker must not read the outbox table directly");

        assert_eq!(
            sqlstate(&error),
            "42501",
            "expected insufficient_privilege, got: {error}"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: the runtime request role cannot invoke worker state transitions.
#[tokio::test]
async fn test_runtime_cannot_execute_worker_transitions() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let mark_error = sqlx::query_scalar::<_, bool>("SELECT outbox_mark_published($1, $2)")
            .bind("any-event")
            .bind("any-token")
            .fetch_one(&ctx.runtime_pool)
            .await
            .expect_err("runtime must not execute worker transitions");
        assert_eq!(
            sqlstate(&mark_error),
            "42501",
            "expected insufficient_privilege, got: {mark_error}"
        );

        let claim_error = sqlx::query("SELECT * FROM outbox_claim_events($1, $2, $3)")
            .bind(1i32)
            .bind(chrono::Duration::seconds(60))
            .bind(10i32)
            .fetch_all(&ctx.runtime_pool)
            .await
            .expect_err("runtime must not claim outbox events");
        assert_eq!(
            sqlstate(&claim_error),
            "42501",
            "expected insufficient_privilege, got: {claim_error}"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: the worker metrics view exposes bounded aggregate columns only, never
/// payloads, actor data, or claim tokens.
#[tokio::test]
async fn test_worker_metrics_view_is_bounded() {
    let ctx = setup_test_context().await.unwrap();

    let result = async {
        let org_id = OrganizationId::new("org-metrics").unwrap();
        enqueue_pending_event(&ctx, &org_id, &format!("outbox-{}", Uuid::new_v4())).await?;

        let counts: Vec<(String, i64)> =
            sqlx::query_as("SELECT status, event_count FROM outbox_metrics ORDER BY status")
                .fetch_all(&ctx.worker_pool)
                .await?;
        assert_eq!(counts, vec![("PENDING".to_owned(), 1)]);

        let columns: Vec<String> = sqlx::query_scalar(
            "SELECT column_name FROM information_schema.columns \
             WHERE table_schema = $1 AND table_name = 'outbox_metrics' \
             ORDER BY column_name",
        )
        .bind(&ctx.schema_name)
        .fetch_all(&ctx.worker_pool)
        .await?;
        assert_eq!(
            columns,
            vec![
                "event_count".to_owned(),
                "oldest_pending".to_owned(),
                "oldest_pending_age_seconds".to_owned(),
                "status".to_owned(),
            ],
            "metrics view must expose only bounded aggregate columns"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}
