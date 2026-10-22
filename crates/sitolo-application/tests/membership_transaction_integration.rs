use std::env;
use std::str::FromStr;
use std::sync::Arc;

use sitolo_application::{TenancyEvidenceSink, TenancyService};
use sitolo_audit::ActorRef;
use sitolo_domain::tenancy::{MembershipId, OrganizationId};
use sitolo_events::{OutboxError, OutboxEvent};
use sitolo_persistence::TenancyDatabase;
use sitolo_persistence::audit_repository::{
    AuditPersistenceError, AuditWriter, PostgresAuditWriter,
};
use sitolo_persistence::outbox_repository::{
    OutboxPersistenceError, OutboxWriter, PostgresOutboxRepository,
};
use sqlx_core::query::query;
use sqlx_core::raw_sql::raw_sql;
use sqlx_core::transaction::Transaction;
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions, Postgres};
use uuid::Uuid;

struct TestDb {
    admin: PgPool,
    runtime: PgPool,
    schema: String,
}

async fn test_db() -> Result<TestDb, Box<dyn std::error::Error>> {
    let admin_url = env::var("ADMIN_DATABASE_URL")?;
    let runtime_url = env::var("RUNTIME_DATABASE_URL")?;
    let schema = format!("test_membership_tx_{}", Uuid::new_v4().simple());
    let bootstrap = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await?;
    raw_sql(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(&bootstrap)
        .await?;
    bootstrap.close().await;

    let admin_options =
        PgConnectOptions::from_str(&admin_url)?.options([("search_path", schema.as_str())]);
    let admin = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(admin_options)
        .await?;
    let audit_schema =
        include_str!("../../sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql")
            .replace("__SCHEMA__", &format!("\"{schema}\""));
    raw_sql(&audit_schema).execute(&admin).await?;
    let tenancy_schema =
        include_str!("../../sitolo-persistence/schema/phase4_part8_tenancy_provisioning.sql");
    raw_sql(tenancy_schema).execute(&admin).await?;

    let runtime_options =
        PgConnectOptions::from_str(&runtime_url)?.options([("search_path", schema.as_str())]);
    let runtime = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(runtime_options)
        .await?;
    let runtime_identity: String = sqlx_core::query_scalar::query_scalar("SELECT session_user")
        .fetch_one(&runtime)
        .await?;
    if runtime_identity != "app_runtime" {
        return Err(format!("expected app_runtime, got {runtime_identity}").into());
    }
    Ok(TestDb {
        admin,
        runtime,
        schema,
    })
}

async fn seed_active_membership(
    db: &TestDb,
    organization_id: &str,
    membership_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    query("INSERT INTO organizations (id, name, state, state_version) VALUES ($1, 'Org', 'ACTIVE', 2)")
        .bind(organization_id)
        .execute(&db.admin)
        .await?;
    query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ($1, $2, 'user-1', 'ACTIVE', 3)")
        .bind(membership_id)
        .bind(organization_id)
        .execute(&db.admin)
        .await?;
    Ok(())
}

async fn seed_organization(
    db: &TestDb,
    organization_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    query("INSERT INTO organizations (id, name, state, state_version) VALUES ($1, 'Org', 'ACTIVE', 2)")
        .bind(organization_id)
        .execute(&db.admin)
        .await?;
    Ok(())
}

fn service(db: &TestDb, audit: Arc<dyn AuditWriter>) -> TenancyService {
    let outbox = Arc::new(PostgresOutboxRepository::new(db.runtime.clone()));
    TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new())
        .with_evidence(TenancyEvidenceSink::new(db.runtime.clone(), audit, outbox))
}

#[tokio::test]
async fn member_invitation_and_required_evidence_commit_together() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-member-invitation-atomicity").unwrap();
    let membership = MembershipId::new("member-invitation-atomicity").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();

    let tenancy = service(&db, Arc::new(PostgresAuditWriter::new()));
    let created = tenancy
        .invite_member(
            membership.clone(),
            org.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-invited").unwrap(),
            ActorRef {
                subject_ref: "user-operator".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        created.state,
        sitolo_domain::tenancy::MembershipState::Invited
    );
    let persisted_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(persisted_state, "INVITED");
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.invited') + (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.membership.invited')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(evidence_count, 2);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn membership_transition_and_audit_outbox_commit_together() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-membership-atomicity").unwrap();
    let membership = MembershipId::new("member-membership-atomicity").unwrap();
    seed_active_membership(&db, org.as_str(), membership.as_str())
        .await
        .unwrap();

    let tenancy = service(&db, Arc::new(PostgresAuditWriter::new()));
    let changed = tenancy
        .suspend_member(
            &org,
            &membership,
            ActorRef {
                subject_ref: "user-operator".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        changed.state,
        sitolo_domain::tenancy::MembershipState::Suspended
    );
    let persisted_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(persisted_state, "SUSPENDED");
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(evidence_count, 2);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

struct FailingAuditWriter;

#[async_trait::async_trait]
impl AuditWriter for FailingAuditWriter {
    async fn record_required(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        _event: &sitolo_audit::IamAuditEvent,
    ) -> Result<(), AuditPersistenceError> {
        Err(AuditPersistenceError::Serialization)
    }
}

#[tokio::test]
async fn membership_transition_rolls_back_when_audit_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-membership-rollback").unwrap();
    let membership = MembershipId::new("member-membership-rollback").unwrap();
    seed_active_membership(&db, org.as_str(), membership.as_str())
        .await
        .unwrap();

    let tenancy = service(&db, Arc::new(FailingAuditWriter));
    assert!(
        tenancy
            .suspend_member(
                &org,
                &membership,
                ActorRef {
                    subject_ref: "user-operator".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await
            .is_err()
    );
    let persisted_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(persisted_state, "ACTIVE");
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(evidence_count, 0);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

struct FailingOutboxWriter;

#[async_trait::async_trait]
impl OutboxWriter for FailingOutboxWriter {
    async fn enqueue(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        _event: &OutboxEvent,
    ) -> Result<(), OutboxPersistenceError> {
        Err(OutboxPersistenceError::InvalidRequest(
            OutboxError::PersistenceFailed,
        ))
    }
}

#[tokio::test]
async fn membership_transition_rolls_back_when_outbox_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-membership-outbox-rollback").unwrap();
    let membership = MembershipId::new("member-membership-outbox-rollback").unwrap();
    seed_active_membership(&db, org.as_str(), membership.as_str())
        .await
        .unwrap();

    let tenancy = TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new()).with_evidence(
        TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(FailingOutboxWriter),
        ),
    );
    assert!(
        tenancy
            .suspend_member(
                &org,
                &membership,
                ActorRef {
                    subject_ref: "user-operator".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await
            .is_err()
    );
    let persisted_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(persisted_state, "ACTIVE");
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(evidence_count, 0);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}
