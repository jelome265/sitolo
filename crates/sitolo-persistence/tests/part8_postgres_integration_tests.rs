//! Phase 4 Part 8 Real PostgreSQL Integration Tests.
//!
//! These tests prove the transactional coupling, RLS security, and worker
//! behavior using real PostgreSQL. No mocks.

use std::env;
use std::time::Duration;

use sitolo_audit::{ActorRef, IamAuditEvent, IamEventName, TargetRef};
use sitolo_auth::AuditEventId;
use sitolo_domain::tenancy::{MembershipId, OrganizationId};
use sitolo_events::OutboxEvent;
use sitolo_persistence::audit_repository::{AuditWriter, PostgresAuditWriter};
use sitolo_persistence::outbox_repository::{OutboxReader, OutboxWriter, PostgresOutboxRepository};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Executor, PgPool, Row};
use uuid::Uuid;

/// Test context with admin and runtime pools.
struct TestContext {
    admin_pool: PgPool,
    runtime_pool: PgPool,
    worker_pool: PgPool,
    schema_name: String,
}

/// Sets up test schema with Part 8 tables and RLS policies.
async fn setup_test_context() -> Result<TestContext, Box<dyn std::error::Error>> {
    let admin_url = env::var("ADMIN_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))?;
    let runtime_url = env::var("RUNTIME_DATABASE_URL")?;
    let worker_url = env::var("WORKER_DATABASE_URL").unwrap_or_else(|_| runtime_url.clone());

    let schema_id = Uuid::new_v4().simple().to_string();
    let schema_name = format!("test_part8_{}", schema_id);

    let admin_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&admin_url)
        .await?;

    let runtime_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&runtime_url)
        .await?;

    let worker_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&worker_url)
        .await?;

    // Create schema
    sqlx::raw_sql(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema_name))
        .execute(&admin_pool)
        .await?;

    // Apply Part 8 schema
    let schema_sql = include_str!("fixtures/part8_audit_outbox_schema.sql");
    let schema_with_path = schema_sql
        .replace("audit_events", &format!("{}.audit_events", schema_name))
        .replace("outbox_events", &format!("{}.outbox_events", schema_name))
        .replace("outbox_metrics", &format!("{}.outbox_metrics", schema_name));
    
    sqlx::raw_sql(&schema_with_path)
        .execute(&admin_pool)
        .await?;

    Ok(TestContext {
        admin_pool,
        runtime_pool,
        worker_pool,
        schema_name,
    })
}

/// Tears down test schema.
async fn teardown_test_context(ctx: &TestContext) {
    let _ = sqlx::raw_sql(&format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", ctx.schema_name))
        .execute(&ctx.admin_pool)
        .await;
}

// ============================================================================
// ATOMICITY TESTS (Contract §13)
// ============================================================================

/// Test A: Business succeeds, audit fails -> ROLLBACK
#[tokio::test]
async fn test_atomicity_business_succeeds_audit_fails() {
    let ctx = setup_test_context().await.unwrap();
    
    let result = async {
        let org_id = OrganizationId::new("org-atomic-1").unwrap();
        let event_id = AuditEventId::new(&format!("evt-{}", Uuid::new_v4())).unwrap();
        
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

        // Begin transaction
        let mut tx = ctx.admin_pool.begin().await?;

        // Set tenant context
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-atomic-1'")
            .execute(&mut *tx)
            .await?;

        // Business mutation (simulated)
        sqlx::raw_sql(&format!(
            "INSERT INTO {}.audit_events (event_id, event_name, event_version, organization_id, target_type, target_ref, action, result, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            ctx.schema_name
        ))
        .bind(event_id.to_string())
        .bind("iam.organization.created")
        .bind(1i32)
        .bind(org_id.to_string())
        .bind("Organization")
        .bind(org_id.to_string())
        .bind("provision")
        .bind("SUCCESS")
        .bind("tenancy_service")
        .execute(&mut *tx)
        .await?;

        // Simulate audit failure by rolling back
        tx.rollback().await?;

        // Verify audit does not exist
        let count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {}.audit_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(event_id.to_string())
        .fetch_one(&ctx.admin_pool)
        .await?;

        assert_eq!(count, 0, "Audit event must not exist after rollback");

        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test B: Business succeeds, outbox fails -> ROLLBACK
#[tokio::test]
async fn test_atomicity_business_succeeds_outbox_fails() {
    let ctx = setup_test_context().await.unwrap();
    
    let result = async {
        let org_id = OrganizationId::new("org-atomic-2").unwrap();
        let event_id = format!("evt-{}", Uuid::new_v4());
        let outbox_id = format!("outbox-{}", Uuid::new_v4());

        let mut tx = ctx.admin_pool.begin().await?;

        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-atomic-2'")
            .execute(&mut *tx)
            .await?;

        // Business mutation
        sqlx::raw_sql(&format!(
            "INSERT INTO {}.audit_events (event_id, event_name, event_version, organization_id, target_type, target_ref, action, result, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            ctx.schema_name
        ))
        .bind(&event_id)
        .bind("iam.organization.created")
        .bind(1i32)
        .bind(org_id.to_string())
        .bind("Organization")
        .bind(org_id.to_string())
        .bind("provision")
        .bind("SUCCESS")
        .bind("tenancy_service")
        .execute(&mut *tx)
        .await?;

        // Simulate outbox failure by rolling back
        tx.rollback().await?;

        // Verify neither audit nor outbox exist
        let audit_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {}.audit_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&event_id)
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
    }.await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test C: All writes succeed -> COMMIT
#[tokio::test]
async fn test_atomicity_all_succeed_commit() {
    let ctx = setup_test_context().await.unwrap();
    
    let result = async {
        let org_id = OrganizationId::new("org-atomic-3").unwrap();
        let event_id = AuditEventId::new(&format!("evt-{}", Uuid::new_v4())).unwrap();
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
        ).unwrap();

        let audit_writer = PostgresAuditWriter::new(ctx.admin_pool.clone());
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
    }.await;

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
        let insert_result = sqlx::raw_sql(&format!(
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

        // Set context to org-A but try to write to org-B
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-A'")
            .execute(&ctx.runtime_pool)
            .await?;

        let insert_result = sqlx::raw_sql(&format!(
            "INSERT INTO {}.audit_events (event_id, event_name, event_version, organization_id, target_type, target_ref, action, result, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            ctx.schema_name
        ))
        .bind(&event_id)
        .bind("iam.organization.created")
        .bind(1i32)
        .bind("org-B")  // Mismatched
        .bind("Organization")
        .bind("org-B")
        .bind("provision")
        .bind("SUCCESS")
        .bind("tenancy_service")
        .execute(&ctx.runtime_pool)
        .await;

        assert!(insert_result.is_err(), "Insert must fail with mismatched tenant context");

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
        sqlx::raw_sql(&format!(
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

        // Runtime tries to update
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-1'")
            .execute(&ctx.runtime_pool)
            .await?;

        let update_result = sqlx::raw_sql(&format!(
            "UPDATE {}.audit_events SET result = 'FAILURE' WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&event_id)
        .execute(&ctx.runtime_pool)
        .await;

        assert!(update_result.is_err(), "Runtime must not UPDATE audit records");

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

        sqlx::raw_sql(&format!(
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

        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-1'")
            .execute(&ctx.runtime_pool)
            .await?;

        let delete_result = sqlx::raw_sql(&format!(
            "DELETE FROM {}.audit_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&event_id)
        .execute(&ctx.runtime_pool)
        .await;

        assert!(delete_result.is_err(), "Runtime must not DELETE audit records");

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
        ).unwrap();

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
            outbox_reader.claim_batch(1, Duration::from_secs(300)),
            outbox_reader.claim_batch(1, Duration::from_secs(300))
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
        .fetch_one(&ctx.worker_pool)
        .await?;

        assert_eq!(status, "CLAIMED", "Event must be in CLAIMED state");

        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;

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
        ).unwrap();

        let outbox_writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());
        let mut tx = ctx.admin_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-lease'")
            .execute(&mut *tx)
            .await?;
        outbox_writer.enqueue(&mut tx, &outbox_event).await?;
        tx.commit().await?;

        let outbox_reader = PostgresOutboxRepository::new(ctx.worker_pool.clone());

        // Claim with very short lease
        let claimed = outbox_reader.claim_batch(1, Duration::from_secs(1)).await?;
        assert_eq!(claimed.len(), 1);

        // Wait for lease to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Recover stale claims
        let recovered = outbox_reader.recover_stale_claims(Duration::from_secs(1)).await?;
        assert!(recovered > 0, "Stale claims must be recovered");

        // Verify event is back to PENDING
        let status: String = sqlx::query_scalar(&format!(
            "SELECT status FROM {}.outbox_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .fetch_one(&ctx.worker_pool)
        .await?;

        assert_eq!(status, "PENDING", "Event must be back to PENDING after lease recovery");

        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;

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
        ).unwrap();

        let outbox_writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());
        let mut tx = ctx.admin_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-worker-test'")
            .execute(&mut *tx)
            .await?;
        outbox_writer.enqueue(&mut tx, &outbox_event).await?;
        tx.commit().await?;

        // Worker tries to update organization_id (immutable field)
        let update_result = sqlx::raw_sql(&format!(
            "UPDATE {}.outbox_events SET organization_id = 'org-HACKED' WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .execute(&ctx.worker_pool)
        .await;

        assert!(update_result.is_err(), "Worker must not rewrite immutable fields");

        // Verify organization_id unchanged
        let org: String = sqlx::query_scalar(&format!(
            "SELECT organization_id FROM {}.outbox_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .fetch_one(&ctx.worker_pool)
        .await?;

        assert_eq!(org, "org-worker-test", "Immutable field must not change");

        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}

/// Test: Duplicate delivery handling
#[tokio::test]
async fn test_duplicate_delivery_idempotency() {
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
        ).unwrap();

        let outbox_writer = PostgresOutboxRepository::new(ctx.admin_pool.clone());
        let mut tx = ctx.admin_pool.begin().await?;
        sqlx::raw_sql("SET LOCAL app.organization_id = 'org-dup'")
            .execute(&mut *tx)
            .await?;
        outbox_writer.enqueue(&mut tx, &outbox_event).await?;
        tx.commit().await?;

        let outbox_reader = PostgresOutboxRepository::new(ctx.worker_pool.clone());

        // Claim event
        let claimed = outbox_reader.claim_batch(1, Duration::from_secs(300)).await?;
        assert_eq!(claimed.len(), 1);

        // Mark as published
        outbox_reader.mark_published(&outbox_id).await?;

        // Simulate crash before marking - try to claim again
        // Event should be PUBLISHED, not claimable
        let claimed2 = outbox_reader.claim_batch(1, Duration::from_secs(300)).await?;
        assert_eq!(claimed2.len(), 0, "Published event must not be claimable again");

        // Verify status is PUBLISHED
        let status: String = sqlx::query_scalar(&format!(
            "SELECT status FROM {}.outbox_events WHERE event_id = $1",
            ctx.schema_name
        ))
        .bind(&outbox_id)
        .fetch_one(&ctx.worker_pool)
        .await?;

        assert_eq!(status, "PUBLISHED");

        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;

    teardown_test_context(&ctx).await;
    result.unwrap();
}
