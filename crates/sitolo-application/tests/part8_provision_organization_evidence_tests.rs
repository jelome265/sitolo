//! Phase 4 Part 8 (PR-009) integration tests for
//! `TenancyService::provision_organization` evidence emission.
//!
//! Exercises the audit+outbox atomicity slice described in
//! `crates/sitolo-application/src/tenancy.rs` module docs and required by
//! `docs/phase4_part8_audit_outbox_implementation_contract.md` Â§29
//! (atomicity matrix) and Â§61 (required atomicity tests).
//!
//! Requires a real PostgreSQL instance (contract Â§59: "No security test may
//! silently skip because PostgreSQL is unavailable"). Set
//! `ADMIN_DATABASE_URL` to a database the connecting
//! role can create/drop schemas in. Setup errors propagate as test failures;
//! these tests do not silently skip when a database variable is absent.
//!
//! ## What these tests prove, and what they do not
//!
//! Success and failure cases exercise the same PostgreSQL transaction for
//! the organization, owner membership, default branch, audit row and outbox
//! event. They return setup and assertion errors rather than skipping without
//! PostgreSQL.

use std::sync::Arc;
use std::{env, error::Error};

use sitolo_application::TenancyService;
use sitolo_audit::ActorRef;
use sitolo_domain::tenancy::{BranchId, MembershipId, OrganizationId, TenancyError, TenantUserId};
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

#[expect(
    clippy::disallowed_methods,
    reason = "FW-CLP-02: the integration-test harness reads the CI-provided DSNs; production configuration is parsed at the binary boundary."
)]
async fn setup() -> Result<TestContext, Box<dyn Error>> {
    let admin_url = env::var("ADMIN_DATABASE_URL")?;

    let schema_name = format!("test_part8_app_{}", Uuid::new_v4().simple());

    // Set `search_path` at connection-establishment time (not via a runtime
    // `SET` on the pool, which does not reliably survive across pooled
    // connections) so every connection this pool opens resolves the
    // unqualified table names the production code uses ("audit_events",
    // "outbox_events") against our isolated test schema. Same approach as
    // crates/sitolo-persistence/tests/rls_security_tests.rs.
    let admin_opts: PgConnectOptions = admin_url
        .parse::<PgConnectOptions>()?
        .options([("search_path", schema_name.as_str())]);

    let admin_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(admin_opts)
        .await?;

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "CREATE SCHEMA IF NOT EXISTS \"{schema_name}\""
    )))
    .execute(&admin_pool)
    .await?;

    let schema_sql =
        include_str!("../../sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql")
            .replace("__SCHEMA__", &format!("\"{schema_name}\""));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(schema_sql.as_str()))
        .execute(&admin_pool)
        .await?;

    let tenancy_sql =
        include_str!("../../sitolo-persistence/schema/phase4_part8_tenancy_provisioning.sql");
    raw_sql(tenancy_sql).execute(&admin_pool).await?;

    verify_part8_provisioning_rls_metadata(&admin_pool, &schema_name).await?;

    let runtime_url = env::var("RUNTIME_DATABASE_URL")?;
    let runtime_opts: PgConnectOptions = runtime_url
        .parse::<PgConnectOptions>()?
        .options([("search_path", schema_name.as_str())]);
    let runtime_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(runtime_opts)
        .await?;
    let identity: String = query_scalar("SELECT current_user")
        .fetch_one(&runtime_pool)
        .await?;
    let session_identity: String = query_scalar("SELECT session_user")
        .fetch_one(&runtime_pool)
        .await?;
    assert_eq!(identity, "app_runtime");
    assert_eq!(session_identity, "app_runtime");

    Ok(TestContext {
        admin_pool,
        runtime_pool,
        schema_name,
    })
}

async fn teardown(ctx: &TestContext) {
    // Best-effort teardown: a drop failure must not mask the test's own result.
    let _teardown = raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
        ctx.schema_name
    )))
    .execute(&ctx.admin_pool)
    .await;
}

fn service_with_evidence(pool: PgPool) -> TenancyService {
    TenancyService::new_postgres(
        Vec::new(),
        Arc::new(sitolo_persistence::PgTenancyUnitOfWork::new(pool)),
    )
}

async fn row_count(pool: &PgPool, table: &str, org_id: &str) -> Result<i64, Box<dyn Error>> {
    query_scalar(sqlx_core::sql_str::AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM {table} WHERE organization_id = $1"
    )))
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

async fn organization_row_count(
    pool: &PgPool,
    organization_id: &str,
) -> Result<i64, Box<dyn Error>> {
    query_scalar("SELECT COUNT(*) FROM organizations WHERE id = $1")
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
}

#[derive(Clone)]
struct ProvisionArgs {
    organization: OrganizationId,
    membership: MembershipId,
    user: TenantUserId,
    branch: BranchId,
}

fn fresh_provision_args() -> Result<ProvisionArgs, Box<dyn Error>> {
    Ok(ProvisionArgs {
        organization: OrganizationId::new(format!("org-{}", Uuid::new_v4().simple()))?,
        membership: MembershipId::new(format!("mem-{}", Uuid::new_v4().simple()))?,
        user: TenantUserId::new(format!("user-{}", Uuid::new_v4().simple()))?,
        branch: BranchId::new(format!("branch-{}", Uuid::new_v4().simple()))?,
    })
}

async fn seed_invited_member(
    ctx: &TestContext,
    args: &ProvisionArgs,
) -> Result<MembershipId, Box<dyn Error>> {
    let member_id = MembershipId::new(format!("member-{}", Uuid::new_v4().simple()))?;
    let user_id = TenantUserId::new(format!("user-{}", Uuid::new_v4().simple()))?;
    query(
        "INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ($1, $2, $3, 'INVITED', 1)",
    )
    .bind(member_id.as_str())
    .bind(args.organization.as_str())
    .bind(user_id.as_str())
    .execute(&ctx.admin_pool)
    .await?;
    Ok(member_id)
}

/// Test C (Â§61): all writes succeed -> business, audit, and outbox present;
/// exactly one audit event and one outbox event per successful command
/// (Â§38 â€” no duplicate semantic events for one command).
#[tokio::test]
async fn provision_organization_emits_audit_and_outbox_on_success() -> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;

    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args()?;

    let provisioned = service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await;

    let audit_count =
        row_count(&ctx.admin_pool, "audit_events", args.organization.as_str()).await?;
    let outbox_count =
        row_count(&ctx.admin_pool, "outbox_events", args.organization.as_str()).await?;
    let organization_count =
        organization_row_count(&ctx.admin_pool, args.organization.as_str()).await?;
    let membership_count = row_count(
        &ctx.admin_pool,
        "organization_memberships",
        args.organization.as_str(),
    )
    .await?;
    let branch_count = row_count(&ctx.admin_pool, "branches", args.organization.as_str()).await?;

    teardown(&ctx).await;

    let provisioned = provisioned?;
    assert_eq!(provisioned.organization.id, args.organization);
    assert_eq!(organization_count, 1);
    assert_eq!(membership_count, 1);
    assert_eq!(branch_count, 1);
    assert_eq!(
        audit_count, 1,
        "exactly one audit event for one successful command (Â§38)"
    );
    assert_eq!(
        outbox_count, 1,
        "exactly one outbox event for one successful command (Â§38)"
    );
    Ok(())
}

/// R3/R4 regression: the approved pending-acceptance transition updates
/// authoritative PostgreSQL state in the same transaction as its mandatory
/// audit and outbox records.
#[tokio::test]
async fn mark_member_pending_commits_state_audit_and_outbox_together() -> Result<(), Box<dyn Error>>
{
    let ctx = setup().await?;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args()?;
    service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await?;

    let member_id = seed_invited_member(&ctx, &args).await?;

    let result = service
        .mark_member_pending(
            &args.organization,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership.to_string()),
                device_ref: None,
            },
        )
        .await;
    let duplicate_result = service
        .mark_member_pending(
            &args.organization,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership.to_string()),
                device_ref: None,
            },
        )
        .await;
    let membership: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    let event_count: i64 = query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.pending_acceptance' AND target_ref = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    let outbox_payload: serde_json::Value = query_scalar(
        "SELECT payload FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.membership.pending_acceptance' AND aggregate_id = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    teardown(&ctx).await;

    result?;
    assert!(matches!(
        duplicate_result,
        Err(TenancyError::InvalidTransition)
    ));
    assert_eq!(membership, ("PENDING_ACCEPTANCE".to_owned(), 2));
    assert_eq!(event_count, 1);
    assert_eq!(
        outbox_payload
            .get("membership_id")
            .and_then(serde_json::Value::as_str),
        Some(member_id.as_str())
    );
    assert_eq!(
        outbox_payload
            .get("from_state")
            .and_then(serde_json::Value::as_str),
        Some("INVITED")
    );
    assert_eq!(
        outbox_payload
            .get("to_state")
            .and_then(serde_json::Value::as_str),
        Some("PENDING_ACCEPTANCE")
    );
    assert_eq!(
        outbox_payload
            .get("state_version")
            .and_then(serde_json::Value::as_u64),
        Some(2)
    );
    Ok(())
}

#[tokio::test]
#[expect(
    clippy::disallowed_methods,
    reason = "FW-TOK-03: the two concurrent calls are a bounded test harness and every handle is awaited."
)]
async fn concurrent_mark_member_pending_emits_one_semantic_event() -> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    let service = Arc::new(service_with_evidence(ctx.runtime_pool.clone()));
    let args = fresh_provision_args()?;
    service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await?;
    let member_id = seed_invited_member(&ctx, &args).await?;
    let barrier = Arc::new(Barrier::new(3));
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let service = Arc::clone(&service);
        let barrier = Arc::clone(&barrier);
        let organization_id = args.organization.clone();
        let membership_id = member_id.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            service
                .mark_member_pending(
                    &organization_id,
                    &membership_id,
                    ActorRef {
                        subject_ref: "issuer-user".to_owned(),
                        membership_ref: Some("issuer-membership".to_owned()),
                        device_ref: None,
                    },
                )
                .await
        }));
    }
    barrier.wait().await;
    let first = tasks.remove(0).await?;
    let second = tasks.remove(0).await?;

    let event_count: i64 = query_scalar(
        "SELECT COUNT(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.membership.pending_acceptance' AND aggregate_id = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    let membership: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    teardown(&ctx).await;

    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert_eq!(event_count, 1);
    assert_eq!(membership, ("PENDING_ACCEPTANCE".to_owned(), 2));
    Ok(())
}

#[tokio::test]
async fn mark_member_pending_rolls_back_state_and_audit_when_outbox_fails()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args()?;
    service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await?;
    let member_id = seed_invited_member(&ctx, &args).await?;
    raw_sql("ALTER TABLE outbox_events RENAME TO outbox_events_disabled")
        .execute(&ctx.admin_pool)
        .await?;

    let result = service
        .mark_member_pending(
            &args.organization,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership.to_string()),
                device_ref: None,
            },
        )
        .await;
    let membership: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    let audit_count: i64 = query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.pending_acceptance' AND target_ref = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    teardown(&ctx).await;

    assert!(matches!(
        result,
        Err(TenancyError::EvidencePersistenceFailed
            | TenancyError::EvidencePersistenceFailedAt { .. })
    ));
    assert_eq!(membership, ("INVITED".to_owned(), 1));
    assert_eq!(audit_count, 0);
    Ok(())
}

#[tokio::test]
async fn mark_member_pending_rolls_back_state_when_audit_fails() -> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args()?;
    service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await?;
    let member_id = seed_invited_member(&ctx, &args).await?;
    raw_sql("ALTER TABLE audit_events RENAME TO audit_events_disabled")
        .execute(&ctx.admin_pool)
        .await?;

    let result = service
        .mark_member_pending(
            &args.organization,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership.to_string()),
                device_ref: None,
            },
        )
        .await;
    let membership: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    teardown(&ctx).await;

    assert!(matches!(
        result,
        Err(TenancyError::EvidencePersistenceFailed
            | TenancyError::EvidencePersistenceFailedAt { .. })
    ));
    assert_eq!(membership, ("INVITED".to_owned(), 1));
    Ok(())
}

#[tokio::test]
async fn expire_invitation_commits_membership_audit_and_outbox_together()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args()?;
    service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await?;
    let member_id = seed_invited_member(&ctx, &args).await?;
    service
        .mark_member_pending(
            &args.organization,
            &member_id,
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: Some(args.membership.to_string()),
                device_ref: None,
            },
        )
        .await?;

    let result = service
        .expire_invitation(&args.organization, &member_id)
        .await;
    let membership: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    let event: (String, String, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT event_name, actor_subject_ref, reason_class FROM audit_events WHERE organization_id = $1 AND target_ref = $2 AND event_name = 'iam.membership.expired'",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    let payload: serde_json::Value = query_scalar(
        "SELECT payload FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.membership.expired' AND aggregate_id = $2",
    )
    .bind(args.organization.as_str())
    .bind(member_id.as_str())
    .fetch_one(&ctx.admin_pool)
    .await
    ?;
    teardown(&ctx).await;

    result?;
    assert_eq!(membership, ("EXPIRED".to_owned(), 3));
    assert_eq!(event.0, "iam.membership.expired");
    assert_eq!(event.1, "system:invitation-expiry");
    assert_eq!(event.2, None);
    assert_eq!(
        payload
            .get("membership_id")
            .and_then(serde_json::Value::as_str),
        Some(member_id.as_str())
    );
    assert_eq!(
        payload
            .get("from_state")
            .and_then(serde_json::Value::as_str),
        Some("PENDING_ACCEPTANCE")
    );
    assert_eq!(
        payload.get("to_state").and_then(serde_json::Value::as_str),
        Some("EXPIRED")
    );
    assert_eq!(
        payload
            .get("state_version")
            .and_then(serde_json::Value::as_u64),
        Some(3)
    );
    assert_eq!(
        payload
            .get("expiry_reason")
            .and_then(serde_json::Value::as_str),
        Some("enrollment_window_elapsed")
    );
    Ok(())
}

/// A repeated command with the same complete topology payload returns the
/// existing result and does not emit a second semantic audit/outbox event.
#[tokio::test]
async fn provision_organization_retry_is_idempotent_and_divergence_conflicts()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args()?;

    let first = service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await;
    let retry = service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await;
    let divergent = service
        .provision_organization(
            args.organization.clone(),
            "Different name",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await;
    let audit_count =
        row_count(&ctx.admin_pool, "audit_events", args.organization.as_str()).await?;
    let outbox_count =
        row_count(&ctx.admin_pool, "outbox_events", args.organization.as_str()).await?;

    teardown(&ctx).await;

    assert!(first.is_ok());
    assert_eq!(retry, first);
    assert_eq!(divergent, Err(TenancyError::Conflict));
    assert_eq!(audit_count, 1);
    assert_eq!(outbox_count, 1);
    Ok(())
}

/// Concurrent identical requests converge on one complete provisioning
/// bundle and one mandatory audit/outbox pair.
#[tokio::test]
#[expect(
    clippy::disallowed_methods,
    reason = "FW-TOK-03: the two concurrent calls are a bounded test harness and every handle is awaited."
)]
async fn provision_organization_concurrent_same_payload_is_idempotent() -> Result<(), Box<dyn Error>>
{
    let ctx = setup().await?;
    let service = Arc::new(service_with_evidence(ctx.runtime_pool.clone()));
    let args = fresh_provision_args()?;
    let barrier = Arc::new(Barrier::new(2));

    let first_service = Arc::clone(&service);
    let first_barrier = Arc::clone(&barrier);
    let first_args = args.clone();
    let first = tokio::spawn(async move {
        first_barrier.wait().await;
        first_service
            .provision_organization(
                first_args.organization,
                "Acme Corp",
                first_args.membership,
                first_args.user,
                first_args.branch,
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
                second_args.organization,
                "Acme Corp",
                second_args.membership,
                second_args.user,
                second_args.branch,
                "HQ",
            )
            .await
    });

    let first_result = first.await?;
    let second_result = second.await?;
    let audit_count =
        row_count(&ctx.admin_pool, "audit_events", args.organization.as_str()).await?;
    let outbox_count =
        row_count(&ctx.admin_pool, "outbox_events", args.organization.as_str()).await?;
    let organization_count =
        organization_row_count(&ctx.admin_pool, args.organization.as_str()).await?;
    let membership_count = row_count(
        &ctx.admin_pool,
        "organization_memberships",
        args.organization.as_str(),
    )
    .await?;
    let branch_count = row_count(&ctx.admin_pool, "branches", args.organization.as_str()).await?;

    teardown(&ctx).await;

    assert!(first_result.is_ok());
    assert!(second_result.is_ok());
    assert_eq!(organization_count, 1);
    assert_eq!(membership_count, 1);
    assert_eq!(branch_count, 1);
    assert_eq!(audit_count, 1);
    assert_eq!(outbox_count, 1);
    Ok(())
}

/// Test A (Â§61): outbox failure rolls back business and audit rows already
/// staged in the same transaction.
#[tokio::test]
async fn provision_organization_rolls_back_audit_when_outbox_write_fails()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;

    // Break the outbox half of the shared transaction only.
    raw_sql("ALTER TABLE outbox_events RENAME TO outbox_events_disabled")
        .execute(&ctx.admin_pool)
        .await?;

    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args()?;

    let result = service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await;

    let audit_count =
        row_count(&ctx.admin_pool, "audit_events", args.organization.as_str()).await?;
    let organization_count =
        organization_row_count(&ctx.admin_pool, args.organization.as_str()).await?;
    let membership_count = row_count(
        &ctx.admin_pool,
        "organization_memberships",
        args.organization.as_str(),
    )
    .await?;
    let branch_count = row_count(&ctx.admin_pool, "branches", args.organization.as_str()).await?;

    teardown(&ctx).await;

    assert!(
        matches!(
            result,
            Err(TenancyError::EvidencePersistenceFailed
                | TenancyError::EvidencePersistenceFailedAt { .. })
        ),
        "expected evidence persistence failure, got {result:?}"
    );
    assert_eq!(
        audit_count, 0,
        "audit insert must roll back with the outbox insert in the same transaction (Â§5, Â§29)"
    );
    assert_eq!(organization_count, 0, "business row must roll back");
    assert_eq!(membership_count, 0, "business row must roll back");
    assert_eq!(branch_count, 0, "business row must roll back");
    Ok(())
}

/// Mandatory audit failure also rolls back business state. The renamed table
/// causes the audit insert to fail after the three business inserts.
#[tokio::test]
async fn provision_organization_audit_failure_rolls_back_business_state()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;

    raw_sql("ALTER TABLE audit_events RENAME TO audit_events_disabled")
        .execute(&ctx.admin_pool)
        .await?;

    let service = service_with_evidence(ctx.runtime_pool.clone());
    let args = fresh_provision_args()?;

    let result = service
        .provision_organization(
            args.organization.clone(),
            "Acme Corp",
            args.membership.clone(),
            args.user.clone(),
            args.branch.clone(),
            "HQ",
        )
        .await;

    let organization_count =
        organization_row_count(&ctx.admin_pool, args.organization.as_str()).await?;
    let membership_count = row_count(
        &ctx.admin_pool,
        "organization_memberships",
        args.organization.as_str(),
    )
    .await?;
    let branch_count = row_count(&ctx.admin_pool, "branches", args.organization.as_str()).await?;
    teardown(&ctx).await;

    assert!(result.is_err(), "evidence write was induced to fail");
    assert_eq!(organization_count, 0);
    assert_eq!(membership_count, 0);
    assert_eq!(branch_count, 0);
    Ok(())
}

/// Negative regression for the Part 8 verifier: a SELECT policy that keeps the
/// table, role, and command but removes the tenant-context comparison must
/// fail. `id IS NOT NULL` / `organization_id IS NOT NULL` satisfied the old
/// substring helper, so these cases exercise the exact-predicate replacement.
#[tokio::test]
async fn part8_verifier_rejects_select_policies_without_tenant_context()
-> Result<(), Box<dyn Error>> {
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
        let ctx = setup().await?;
        raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
            "DROP POLICY {policy} ON {table}; \
             CREATE POLICY {policy} ON {table} FOR SELECT TO app_runtime USING ({expression})"
        )))
        .execute(&ctx.admin_pool)
        .await?;
        let verified =
            verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
        teardown(&ctx).await;
        assert!(
            verified.is_err(),
            "verifier accepted a SELECT policy without tenant context on {table}"
        );
    }
    Ok(())
}

/// Negative regression for INSERT: the old helper accepted `id <> ''` because
/// it contained the tenant column and the non-empty guard. The canonical
/// predicate must reject it because it never compares against
/// `current_setting('app.organization_id', true)`.
#[tokio::test]
async fn part8_verifier_rejects_insert_policies_without_tenant_context()
-> Result<(), Box<dyn Error>> {
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
        let ctx = setup().await?;
        raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
            "DROP POLICY {policy} ON {table}; \
             CREATE POLICY {policy} ON {table} FOR INSERT TO app_runtime WITH CHECK ({expression})"
        )))
        .execute(&ctx.admin_pool)
        .await?;
        let verified =
            verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
        teardown(&ctx).await;
        assert!(
            verified.is_err(),
            "verifier accepted an INSERT policy without tenant context on {table}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn part8_verifier_rejects_membership_update_policy_without_tenant_context()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    raw_sql(
        "DROP POLICY memberships_transition_update ON organization_memberships; \
         CREATE POLICY memberships_transition_update ON organization_memberships \
         FOR UPDATE TO app_runtime USING (organization_id IS NOT NULL) \
         WITH CHECK (organization_id IS NOT NULL)",
    )
    .execute(&ctx.admin_pool)
    .await?;
    let verified = verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
    teardown(&ctx).await;
    assert!(
        verified.is_err(),
        "verifier accepted a membership UPDATE policy without tenant context"
    );
    Ok(())
}

#[tokio::test]
async fn part8_verifier_rejects_broad_membership_update_grant() -> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    raw_sql("GRANT UPDATE ON organization_memberships TO app_runtime")
        .execute(&ctx.admin_pool)
        .await?;
    let verified = verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
    teardown(&ctx).await;
    assert!(
        verified.is_err(),
        "verifier accepted a table-wide membership UPDATE grant"
    );
    Ok(())
}

#[tokio::test]
async fn part8_verifier_rejects_role_assignment_policy_without_tenant_context()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    raw_sql(
        "DROP POLICY role_assignments_tenant_all ON role_assignments; \
         CREATE POLICY role_assignments_tenant_all ON role_assignments \
         FOR ALL TO app_runtime USING (organization_id IS NOT NULL) \
         WITH CHECK (organization_id IS NOT NULL)",
    )
    .execute(&ctx.admin_pool)
    .await?;
    let verified = verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
    teardown(&ctx).await;
    assert!(
        verified.is_err(),
        "verifier accepted a role assignment policy without tenant context"
    );
    Ok(())
}

#[tokio::test]
async fn part8_verifier_rejects_table_wide_role_assignment_update_grant()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    raw_sql("GRANT UPDATE ON role_assignments TO app_runtime")
        .execute(&ctx.admin_pool)
        .await?;
    let verified = verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
    teardown(&ctx).await;
    assert!(
        verified.is_err(),
        "verifier accepted a table-wide role-assignment UPDATE grant"
    );
    Ok(())
}

#[tokio::test]
async fn part8_verifier_rejects_table_wide_scope_grant_update_privilege()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    raw_sql("GRANT UPDATE ON scope_grants TO app_runtime")
        .execute(&ctx.admin_pool)
        .await?;
    let verified = verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
    teardown(&ctx).await;
    assert!(
        verified.is_err(),
        "verifier accepted a table-wide scope-grant UPDATE grant"
    );
    Ok(())
}

#[tokio::test]
async fn part8_verifier_rejects_unrestricted_invitation_token_resolution_policy()
-> Result<(), Box<dyn Error>> {
    let ctx = setup().await?;
    raw_sql(
        "DROP POLICY invitations_token_resolution ON invitations; \
         CREATE POLICY invitations_token_resolution ON invitations \
         FOR SELECT TO app_outbox_owner USING (true)",
    )
    .execute(&ctx.admin_pool)
    .await?;
    let verified = verify_part8_provisioning_rls_metadata(&ctx.admin_pool, &ctx.schema_name).await;
    teardown(&ctx).await;
    assert!(
        verified.is_err(),
        "verifier accepted an unrestricted invitation-resolution policy"
    );
    Ok(())
}
