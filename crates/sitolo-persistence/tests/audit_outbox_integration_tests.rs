//! Phase 4 Part 8 / PR-009 Audit + Outbox Atomicity, Rollback, Worker Concurrency, and Isolation Tests.

use std::env;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use sitolo_audit::{EventResult, IamAuditEvent, IamAuditEventName};
use sitolo_auth::{Assurance, AuditEventId};
use sitolo_domain::tenancy::{
    Membership, MembershipId, Organization, OrganizationId, TenantUserId,
};
use sitolo_events::{OutboxEvent, OutboxEventId, OutboxStatus};
use sitolo_persistence::audit_outbox::{AuditOutboxStore, PgAuditOutboxStore};
use sitolo_persistence::postgres::{
    PgAuthorityError, PgAuthorityPools, set_transaction_tenant_context,
};
use sitolo_tenancy::{
    AuthorizedScope, RequestedOrganizationId, bind_organization, resolve_effective_scope,
};
use sqlx::PgPool;
use sqlx::postgres::PgConnectOptions;
use tokio::sync::OnceCell;
use tokio::task::JoinSet;

static MIGRATIONS_INIT: OnceCell<()> = OnceCell::const_new();

struct SchemaGuard {
    schema_name: String,
    admin_pool: PgPool,
}

impl SchemaGuard {
    async fn teardown(&self) -> Result<(), PgAuthorityError> {
        let drop_sql = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", self.schema_name);
        sqlx::raw_sql(&drop_sql).execute(&self.admin_pool).await?;
        Ok(())
    }
}

struct AuditOutboxTestContext {
    pools: PgAuthorityPools,
    guard: SchemaGuard,
    _schema_name: String,
    store: PgAuditOutboxStore,
    worker_pool: PgPool,
    org_a_scope: AuthorizedScope,
    _org_b_scope: AuthorizedScope,
    org_a_id_str: String,
    _org_b_id_str: String,
}

async fn run_audit_outbox_test<F, Fut>(test_fn: F)
where
    F: FnOnce(Arc<AuditOutboxTestContext>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let ctx = match setup_audit_outbox_context().await {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => return,
        Err(err) => panic!("Audit/Outbox test context setup failed: {err:?}"),
    };

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
            panic!("Test task cancelled ({join_err:?}) AND teardown failed ({td_err:?})");
        }
    }
}

async fn setup_audit_outbox_context() -> Result<Option<AuditOutboxTestContext>, PgAuthorityError> {
    let admin_url = match env::var("ADMIN_DATABASE_URL").or_else(|_| env::var("DATABASE_URL")) {
        Ok(url) => url,
        Err(_) => {
            eprintln!(
                "Skipping PostgreSQL integration test: ADMIN_DATABASE_URL / DATABASE_URL not set"
            );
            return Ok(None);
        }
    };

    let runtime_url = match env::var("RUNTIME_DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("Skipping PostgreSQL integration test: RUNTIME_DATABASE_URL not set");
            return Ok(None);
        }
    };

    let schema_id = uuid::Uuid::new_v4().simple().to_string();
    let schema_name = format!("test_audit_outbox_{schema_id}");

    let mut admin_opts: PgConnectOptions = admin_url.parse().expect("Invalid admin database URL");
    let mut runtime_opts: PgConnectOptions =
        runtime_url.parse().expect("Invalid runtime database URL");

    admin_opts = admin_opts.options([("search_path", schema_name.as_str())]);
    runtime_opts = runtime_opts.options([("search_path", schema_name.as_str())]);

    let mut worker_opts: sqlx::postgres::PgConnectOptions = runtime_url.parse().expect("Invalid runtime database URL");
    worker_opts = worker_opts
        .username("app_worker")
        .password("test")
        .options([("search_path", schema_name.as_str())]);


    let admin_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(admin_opts.clone())
        .await?;

    MIGRATIONS_INIT
        .get_or_init(|| async {
            let role_sql = "DO $$ BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_runtime') THEN
        CREATE ROLE app_runtime WITH LOGIN NOSUPERUSER NOINHERIT NOCREATEDB NOCREATEROLE NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_worker') THEN
        CREATE ROLE app_worker WITH LOGIN NOSUPERUSER NOINHERIT NOCREATEDB NOCREATEROLE NOBYPASSRLS;
    END IF;
END $$;";
            sqlx::raw_sql(role_sql)
                .execute(&admin_pool)
                .await
                .expect("Failed to initialize app_runtime role");
        })
        .await;

    let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\"");
    sqlx::raw_sql(&create_schema_sql)
        .execute(&admin_pool)
        .await?;

    let schema_sql = include_str!("fixtures/rls_schema.sql");
    sqlx::raw_sql(schema_sql).execute(&admin_pool).await?;

    let grant_schema_sql = format!(
        "GRANT USAGE ON SCHEMA \"{schema_name}\" TO app_runtime; GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA \"{schema_name}\" TO app_runtime;"
    );
    sqlx::raw_sql(&grant_schema_sql)
        .execute(&admin_pool)
        .await?;

    
    let worker_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(worker_opts.clone())
        .await?;

    let pools = PgAuthorityPools::connect_options(admin_opts.clone(), runtime_opts.clone()).await?;

    let unique_id = uuid::Uuid::new_v4().simple().to_string();
    let org_a_id_str = format!("org_A_{unique_id}");
    let org_b_id_str = format!("org_B_{unique_id}");
    let branch_a1_id_str = format!("branch_A1_{unique_id}");
    let branch_b1_id_str = format!("branch_B1_{unique_id}");

    let org_a_id = OrganizationId::new(&org_a_id_str)?;
    let org_b_id = OrganizationId::new(&org_b_id_str)?;

    let mut org_a = Organization::provision(org_a_id.clone(), "Organization A")?;
    org_a.activate()?;

    let mut org_b = Organization::provision(org_b_id.clone(), "Organization B")?;
    org_b.activate()?;

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
    .execute(&admin_pool)
    .await?;

    sqlx::query(
        "INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ($1, $2, 'Branch A1', 'ACTIVE', 1), ($3, $4, 'Branch B1', 'ACTIVE', 1)",
    )
    .bind(&branch_a1_id_str)
    .bind(&org_a_id_str)
    .bind(&branch_b1_id_str)
    .bind(&org_b_id_str)
    .execute(&admin_pool)
    .await?;

    let store = PgAuditOutboxStore::new(pools.runtime_pool().clone(), worker_pool.clone());

    Ok(Some(AuditOutboxTestContext {
        pools,
        guard: SchemaGuard {
            schema_name: schema_name.clone(),
            admin_pool: admin_pool.clone(),
        },
        _schema_name: schema_name,
        store,
        org_a_scope,
        _org_b_scope: org_b_scope,
        org_a_id_str,
        _org_b_id_str: org_b_id_str,
    }))
}

fn sample_audit_event(id: &str, org_id: &str) -> IamAuditEvent {
    IamAuditEvent {
        event_id: AuditEventId::new(id).unwrap(),
        event_name: IamAuditEventName::OrganizationCreated,
        event_version: 1,
        occurred_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
        organization_id: Some(org_id.to_string()),
        branch_id: None,
        actor_subject_ref: Some("user-1".into()),
        actor_membership_ref: None,
        actor_device_ref: None,
        request_id: Some("req-1".into()),
        trace_id: None,
        target_type: Some("organization".into()),
        target_ref: Some(org_id.to_string()),
        action: "iam.organization.created".into(),
        result: EventResult::Success,
        reason_class: None,
        assurance_level: Some(Assurance::A1),
        source: "api".into(),
        metadata: None,
    }
}


async fn application_activate_organization(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    store: &PgAuditOutboxStore,
    org_id: &str,
    audit_id: &str,
    outbox_id: &str,
) -> Result<(), PgAuthorityError> {
    sqlx::query("UPDATE organizations SET state = 'ACTIVE', state_version = state_version + 1 WHERE id = $1")
        .bind(org_id)
        .execute(&mut **tx)
        .await?;

    let audit = sample_audit_event(audit_id, org_id);
    store.record_iam_audit_tx(tx, &audit).await?;

    let dedup_key = format!("dedup_activate_{}", audit_id);
    let outbox = sample_outbox_event(outbox_id, &dedup_key, org_id);
    store.enqueue_outbox_tx(tx, &outbox).await?;

    Ok(())
}

fn sample_outbox_event(id: &str, dedup_key: &str, org_id: &str) -> OutboxEvent {
    OutboxEvent {
        event_id: OutboxEventId::new(id).unwrap(),
        aggregate_type: "organization".into(),
        aggregate_id: org_id.to_string(),
        aggregate_sequence: 1,
        event_name: "iam.organization.created".into(),
        event_version: 1,
        organization_id: Some(org_id.to_string()),
        branch_id: None,
        occurred_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
        payload: "{}".into(),
        status: OutboxStatus::Pending,
        available_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
        attempt_count: 0,
        locked_at: None,
        published_at: None,
        last_error_class: None,
        deduplication_key: dedup_key.to_string(),
        schema_version: 1,
        claim_token: None,
    }
}

// ============================================================================
// 1. TRANSACTION ATOMICITY & ROLLBACK TESTS (§29, §61, §64)
// ============================================================================

#[tokio::test]
async fn test_application_command_atomicity() {
    run_audit_outbox_test(|ctx| async move {
        let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut tx, &ctx.org_a_scope)
            .await
            .unwrap();

        // 1. Business state mutation
        let res_id = format!("res_atomicity_ok_{}", uuid::Uuid::new_v4().simple());
        sqlx::query("INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES ($1, $2, $3, $4)")
            .bind(&res_id)
            .bind(&ctx.org_a_id_str)
            .bind(format!("branch_A1_{}", ctx.org_a_id_str.trim_start_matches("org_A_")))
            .bind("Atomicity Data OK")
            .execute(&mut *tx)
            .await
            .unwrap();

        // 2. Audit write
        let audit_id = format!("evt_audit_ok_{}", uuid::Uuid::new_v4().simple());
        let audit = sample_audit_event(&audit_id, &ctx.org_a_id_str);
        ctx.store
            .record_iam_audit_tx(&mut tx, &audit)
            .await
            .unwrap();

        // 3. Outbox write
        let outbox_id = format!("evt_outbox_ok_{}", uuid::Uuid::new_v4().simple());
        let dedup_key = format!("dedup_ok_{}", uuid::Uuid::new_v4().simple());
        let outbox = sample_outbox_event(&outbox_id, &dedup_key, &ctx.org_a_id_str);
        ctx.store.enqueue_outbox_tx(&mut tx, &outbox).await.unwrap();

        // Commit transaction
        tx.commit().await.unwrap();

        // Verify all 3 records are durably present
        let mut read_tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut read_tx, &ctx.org_a_scope)
            .await
            .unwrap();

        let resource_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tenant_resources WHERE id = $1)")
                .bind(&res_id)
                .fetch_one(&mut *read_tx)
                .await
                .unwrap();
        assert!(resource_exists);

        let audit_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM iam_audit_records WHERE event_id = $1)")
                .bind(&audit_id)
                .fetch_one(&mut *read_tx)
                .await
                .unwrap();
        assert!(audit_exists);

        let outbox_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM outbox_events WHERE event_id = $1)")
                .bind(&outbox_id)
                .fetch_one(&mut *read_tx)
                .await
                .unwrap();
        assert!(outbox_exists);

        read_tx.commit().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn test_atomicity_rollback_when_audit_fails() {
    run_audit_outbox_test(|ctx| async move {
        let duplicate_audit_id = format!("evt_audit_dup_{}", uuid::Uuid::new_v4().simple());

        // Pre-seed the audit ID
        let mut seed_tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut seed_tx, &ctx.org_a_scope)
            .await
            .unwrap();
        let seed_audit = sample_audit_event(&duplicate_audit_id, &ctx.org_a_id_str);
        ctx.store
            .record_iam_audit_tx(&mut seed_tx, &seed_audit)
            .await
            .unwrap();
        seed_tx.commit().await.unwrap();

        // Now start authoritative transaction that tries to re-insert the same audit ID
        let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut tx, &ctx.org_a_scope)
            .await
            .unwrap();

        let res_id = format!("res_atomicity_fail_{}", uuid::Uuid::new_v4().simple());
        sqlx::query("INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES ($1, $2, $3, $4)")
            .bind(&res_id)
            .bind(&ctx.org_a_id_str)
            .bind(format!("branch_A1_{}", ctx.org_a_id_str.trim_start_matches("org_A_")))
            .bind("Will Rollback")
            .execute(&mut *tx)
            .await
            .unwrap();

        let dup_audit = sample_audit_event(&duplicate_audit_id, &ctx.org_a_id_str);
        let audit_res = ctx.store.record_iam_audit_tx(&mut tx, &dup_audit).await;

        assert!(audit_res.is_err(), "Duplicate audit insert must fail");

        // Rollback transaction
        tx.rollback().await.unwrap();

        // Verify business resource was NOT committed
        let mut check_tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut check_tx, &ctx.org_a_scope)
            .await
            .unwrap();

        let resource_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tenant_resources WHERE id = $1)")
                .bind(&res_id)
                .fetch_one(&mut *check_tx)
                .await
                .unwrap();

        assert!(
            !resource_exists,
            "Business mutation MUST roll back when audit write fails"
        );
        check_tx.commit().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn test_atomicity_rollback_when_outbox_fails() {
    run_audit_outbox_test(|ctx| async move {
        let duplicate_dedup_key = format!("dedup_dup_{}", uuid::Uuid::new_v4().simple());

        // Pre-seed an outbox event with this deduplication key
        let mut seed_tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut seed_tx, &ctx.org_a_scope)
            .await
            .unwrap();
        let seed_outbox = sample_outbox_event(
            &format!("evt_seed_{}", uuid::Uuid::new_v4().simple()),
            &duplicate_dedup_key,
            &ctx.org_a_id_str,
        );
        ctx.store
            .enqueue_outbox_tx(&mut seed_tx, &seed_outbox)
            .await
            .unwrap();
        seed_tx.commit().await.unwrap();

        // Transaction attempts business mutation + audit + duplicate outbox deduplication key
        let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut tx, &ctx.org_a_scope)
            .await
            .unwrap();

        let res_id = format!("res_outbox_fail_{}", uuid::Uuid::new_v4().simple());
        sqlx::query("INSERT INTO tenant_resources (id, organization_id, branch_id, data) VALUES ($1, $2, $3, $4)")
            .bind(&res_id)
            .bind(&ctx.org_a_id_str)
            .bind(format!("branch_A1_{}", ctx.org_a_id_str.trim_start_matches("org_A_")))
            .bind("Will Rollback Outbox")
            .execute(&mut *tx)
            .await
            .unwrap();

        let audit_id = format!("evt_audit_outbox_fail_{}", uuid::Uuid::new_v4().simple());
        let audit = sample_audit_event(&audit_id, &ctx.org_a_id_str);
        ctx.store
            .record_iam_audit_tx(&mut tx, &audit)
            .await
            .unwrap();

        let dup_outbox = sample_outbox_event(
            &format!("evt_outbox_fail_{}", uuid::Uuid::new_v4().simple()),
            &duplicate_dedup_key,
            &ctx.org_a_id_str,
        );
        let outbox_res = ctx.store.enqueue_outbox_tx(&mut tx, &dup_outbox).await;

        assert!(
            outbox_res.is_err(),
            "Duplicate deduplication key must fail outbox enqueue"
        );

        tx.rollback().await.unwrap();

        // Verify business resource AND audit record are both absent
        let mut check_tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut check_tx, &ctx.org_a_scope)
            .await
            .unwrap();

        let res_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tenant_resources WHERE id = $1)")
                .bind(&res_id)
                .fetch_one(&mut *check_tx)
                .await
                .unwrap();
        assert!(
            !res_exists,
            "Business mutation MUST roll back when outbox write fails"
        );

        let audit_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM iam_audit_records WHERE event_id = $1)")
                .bind(&audit_id)
                .fetch_one(&mut *check_tx)
                .await
                .unwrap();
        assert!(
            !audit_exists,
            "Audit write MUST roll back when outbox write fails"
        );

        check_tx.commit().await.unwrap();
    })
    .await;
}

// ============================================================================
// 2. WORKER CONCURRENCY & LEASE RECOVERY TESTS (§24, §39, §63)
// ============================================================================

#[tokio::test]
async fn test_worker_claim_concurrency() {
    run_audit_outbox_test(|ctx| async move {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        // Seed 10 pending outbox events
        let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut tx, &ctx.org_a_scope)
            .await
            .unwrap();

        for i in 0..10 {
            let id = format!("evt_concur_{i}_{}", uuid::Uuid::new_v4().simple());
            let dedup = format!("dedup_concur_{i}_{}", uuid::Uuid::new_v4().simple());
            let outbox = sample_outbox_event(&id, &dedup, &ctx.org_a_id_str);
            ctx.store.enqueue_outbox_tx(&mut tx, &outbox).await.unwrap();
        }
        tx.commit().await.unwrap();

        // Spawn 4 concurrent worker claim tasks
        let mut set = JoinSet::new();
        for _ in 0..4 {
            let store = ctx.store.clone();
            set.spawn(async move {
                store
                    .claim_outbox_events(5, Duration::from_secs(30), now)
                    .await
                    .unwrap()
            });
        }

        let mut total_claimed = 0;
        let mut claimed_ids = std::collections::HashSet::new();

        while let Some(res) = set.join_next().await {
            let events = res.unwrap();
            total_claimed += events.len();
            for ev in events {
                assert!(
                    claimed_ids.insert(ev.event_id.as_str().to_string()),
                    "Outbox event claimed twice simultaneously by concurrent workers"
                );
            }
        }

        assert_eq!(
            total_claimed, 10,
            "All 10 events claimed exactly once across workers"
        );
    })
    .await;
}

#[tokio::test]
async fn test_worker_lease_recovery() {
    run_audit_outbox_test(|ctx| async move {
        let t1 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut tx, &ctx.org_a_scope)
            .await
            .unwrap();

        let id = format!("evt_lease_{}", uuid::Uuid::new_v4().simple());
        let dedup = format!("dedup_lease_{}", uuid::Uuid::new_v4().simple());
        let outbox = sample_outbox_event(&id, &dedup, &ctx.org_a_id_str);
        ctx.store.enqueue_outbox_tx(&mut tx, &outbox).await.unwrap();
        tx.commit().await.unwrap();

        // Worker A claims event at t1 with 10 second lease
        let claimed_a = ctx
            .store
            .claim_outbox_events(10, Duration::from_secs(10), t1)
            .await
            .unwrap();
        assert_eq!(claimed_a.len(), 1);

        // Worker B tries to claim at t1 + 5s (lease NOT expired) -> claims 0
        let t2 = t1 + Duration::from_secs(5);
        let claimed_b_too_early = ctx
            .store
            .claim_outbox_events(10, Duration::from_secs(10), t2)
            .await
            .unwrap();
        assert_eq!(claimed_b_too_early.len(), 0);

        // Worker B claims at t1 + 15s (lease IS expired) -> Worker B re-claims event
        let t3 = t1 + Duration::from_secs(15);
        let claimed_b_after_lease = ctx
            .store
            .claim_outbox_events(10, Duration::from_secs(10), t3)
            .await
            .unwrap();
        assert_eq!(claimed_b_after_lease.len(), 1);
        assert_eq!(claimed_b_after_lease[0].event_id, claimed_a[0].event_id);
        assert_eq!(claimed_b_after_lease[0].attempt_count, 2);
    })
    .await;
}

// ============================================================================
// 3. CROSS-TENANT ISOLATION TESTS FOR AUDIT & OUTBOX (§12, §37)
// ============================================================================

#[tokio::test]
async fn test_cross_tenant_audit_isolation() {
    run_audit_outbox_test(|ctx| async move {
        // Insert audit event under Tenant A
        let mut tx_a = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut tx_a, &ctx.org_a_scope)
            .await
            .unwrap();
        let audit_a_id = format!("evt_audit_A_{}", uuid::Uuid::new_v4().simple());
        let audit_a = sample_audit_event(&audit_a_id, &ctx.org_a_id_str);
        ctx.store
            .record_iam_audit_tx(&mut tx_a, &audit_a)
            .await
            .unwrap();
        tx_a.commit().await.unwrap();

        // Tenant B attempts to read Tenant A's audit event
        let mut tx_b = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut tx_b, &ctx._org_b_scope)
            .await
            .unwrap();

        let row = sqlx::query("SELECT event_id FROM iam_audit_records WHERE event_id = $1")
            .bind(&audit_a_id)
            .fetch_optional(&mut *tx_b)
            .await
            .unwrap();

        assert!(
            row.is_none(),
            "Tenant B must not see Tenant A audit record under RLS"
        );
        tx_b.commit().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn test_lease_race_and_reclaim() {
    run_audit_outbox_test(|ctx| async move {
        // Enqueue an event
        let mut tx = ctx.pools.runtime_pool().begin().await.unwrap();
        set_transaction_tenant_context(&mut tx, &ctx.org_a_scope).await.unwrap();

        let audit_id = format!("evt_race_{}", uuid::Uuid::new_v4().simple());
        let outbox_id = format!("evt_outbox_race_{}", uuid::Uuid::new_v4().simple());
        
        application_activate_organization(&mut tx, &ctx.store, &ctx.org_a_id_str, &audit_id, &outbox_id)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // Worker A claims the event
        let now = std::time::SystemTime::now();
        let claimed = ctx.store.claim_outbox_events(1, std::time::Duration::from_secs(1), now).await.unwrap();
        assert_eq!(claimed.len(), 1);
        let token_a = claimed[0].claim_token.clone().unwrap();

        // Wait for lease to expire
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Worker B claims the same event (stale lease recovery)
        let now2 = std::time::SystemTime::now();
        let claimed_b = ctx.store.claim_outbox_events(1, std::time::Duration::from_secs(60), now2).await.unwrap();
        assert_eq!(claimed_b.len(), 1);
        let token_b = claimed_b[0].claim_token.clone().unwrap();
        
        assert_ne!(token_a, token_b, "Worker B should get a new claim token");

        // Worker A tries to mark published with stale token
        let stale_res = ctx.store.mark_published(&claimed[0].event_id, &token_a, now2).await;
        assert!(stale_res.is_err(), "Stale worker A should be rejected");

        // Worker B marks published with valid token
        ctx.store.mark_published(&claimed_b[0].event_id, &token_b, now2).await.unwrap();
    }).await;
}
