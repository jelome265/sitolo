use std::env;
use std::str::FromStr;
use std::sync::Arc;

use sitolo_application::{TenancyEvidenceSink, TenancyService};
use sitolo_audit::ActorRef;
use sitolo_authz::{ContactKind, InvitationContact, Role};
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

async fn seed_active_branch(
    db: &TestDb,
    organization_id: &str,
    branch_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    query("INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ($1, $2, 'Main', 'ACTIVE', 2)")
        .bind(branch_id)
        .bind(organization_id)
        .execute(&db.admin)
        .await?;
    Ok(())
}

async fn seed_provisioning_branch(
    db: &TestDb,
    organization_id: &str,
    branch_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    query("INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ($1, $2, 'Main', 'PROVISIONING', 1)")
        .bind(branch_id)
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

async fn issue_invitation(
    tenancy: &TenancyService,
    organization_id: &OrganizationId,
    invitation_id: &str,
    membership_id: &str,
) {
    tenancy
        .create_invitation(
            organization_id.clone(),
            sitolo_domain::tenancy::InvitationId::new(invitation_id).unwrap(),
            MembershipId::new(membership_id).unwrap(),
            sitolo_domain::tenancy::TenantUserId::new(format!("user-{membership_id}")).unwrap(),
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "sweep@example.test").unwrap(),
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await
        .unwrap();
}

async fn make_invitation_expired(db: &TestDb, organization_id: &str, invitation_id: &str) {
    query(
        "UPDATE invitations SET issued_at = now() - interval '2 minutes', \
         expires_at = now() - interval '1 second' \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(organization_id)
    .bind(invitation_id)
    .execute(&db.admin)
    .await
    .unwrap();
}

#[tokio::test]
async fn organization_and_branch_lifecycle_changes_commit_with_evidence() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-lifecycle-atomicity").unwrap();
    let branch = sitolo_domain::tenancy::BranchId::new("branch-lifecycle-atomicity").unwrap();
    query("INSERT INTO organizations (id, name, state, state_version) VALUES ($1, 'Org', 'PROVISIONING', 1)")
        .bind(org.as_str())
        .execute(&db.admin)
        .await
        .unwrap();
    seed_provisioning_branch(&db, org.as_str(), branch.as_str())
        .await
        .unwrap();
    let tenancy = service(&db, Arc::new(PostgresAuditWriter::new()));
    let actor = |subject: &str| ActorRef {
        subject_ref: subject.to_owned(),
        membership_ref: None,
        device_ref: None,
    };

    tenancy
        .activate_organization(&org, actor("org-operator"))
        .await
        .unwrap();
    tenancy
        .suspend_organization(&org, actor("org-operator"))
        .await
        .unwrap();
    tenancy
        .resume_organization(&org, actor("org-operator"))
        .await
        .unwrap();
    tenancy
        .activate_branch(&org, &branch, actor("branch-operator"))
        .await
        .unwrap();
    tenancy
        .suspend_branch(&org, &branch, actor("branch-operator"))
        .await
        .unwrap();
    tenancy
        .resume_branch(&org, &branch, actor("branch-operator"))
        .await
        .unwrap();
    tenancy
        .begin_close_branch(&org, &branch, actor("branch-operator"))
        .await
        .unwrap();
    tenancy
        .close_branch(&org, &branch, actor("branch-operator"))
        .await
        .unwrap();
    tenancy
        .begin_close_organization(&org, actor("org-operator"))
        .await
        .unwrap();
    tenancy
        .close_organization(&org, actor("org-operator"))
        .await
        .unwrap();

    let persisted: (String, i64, String, i64) = sqlx_core::query_as::query_as(
        "SELECT organizations.state, organizations.state_version, branches.state, branches.state_version FROM organizations JOIN branches ON branches.organization_id = organizations.id WHERE organizations.id = $1 AND branches.id = $2",
    )
    .bind(org.as_str())
    .bind(branch.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(persisted, ("CLOSED".to_owned(), 6, "CLOSED".to_owned(), 6));
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(evidence_count, 20);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn organization_and_branch_lifecycle_failures_roll_back_state_and_evidence() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-lifecycle-rollback").unwrap();
    let branch = sitolo_domain::tenancy::BranchId::new("branch-lifecycle-rollback").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();
    seed_active_branch(&db, org.as_str(), branch.as_str())
        .await
        .unwrap();
    let actor = || ActorRef {
        subject_ref: "lifecycle-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };

    let audit_failure = service(&db, Arc::new(FailingAuditWriter));
    assert!(
        audit_failure
            .suspend_organization(&org, actor())
            .await
            .is_err()
    );
    let outbox_failure = TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new())
        .with_evidence(TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(FailingOutboxWriter),
        ));
    assert!(
        outbox_failure
            .suspend_branch(&org, &branch, actor())
            .await
            .is_err()
    );

    let state: (String, i64, String, i64) = sqlx_core::query_as::query_as(
        "SELECT organizations.state, organizations.state_version, branches.state, branches.state_version FROM organizations JOIN branches ON branches.organization_id = organizations.id WHERE organizations.id = $1 AND branches.id = $2",
    )
    .bind(org.as_str())
    .bind(branch.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(state, ("ACTIVE".to_owned(), 2, "ACTIVE".to_owned(), 2));
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

#[tokio::test]
async fn competing_organization_transitions_and_cross_tenant_branch_update_are_denied() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-lifecycle-race").unwrap();
    let other_org = OrganizationId::new("org-lifecycle-other").unwrap();
    let branch = sitolo_domain::tenancy::BranchId::new("branch-lifecycle-cross-tenant").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();
    seed_organization(&db, other_org.as_str()).await.unwrap();
    seed_active_branch(&db, org.as_str(), branch.as_str())
        .await
        .unwrap();
    let tenancy = service(&db, Arc::new(PostgresAuditWriter::new()));

    let (suspend, close) = tokio::join!(
        tenancy.suspend_organization(
            &org,
            ActorRef {
                subject_ref: "operator-a".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        ),
        tenancy.begin_close_organization(
            &org,
            ActorRef {
                subject_ref: "operator-b".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        ),
    );
    assert_ne!(suspend.is_ok(), close.is_ok());

    assert!(
        tenancy
            .suspend_branch(
                &other_org,
                &branch,
                ActorRef {
                    subject_ref: "cross-tenant-operator".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await
            .is_err()
    );
    let branch_state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM branches WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(branch.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(branch_state, ("ACTIVE".to_owned(), 2));
    let event_counts: (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1), (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(event_counts, (1, 1));
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
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
async fn token_invitation_issue_persists_only_hash_and_emits_approved_event() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-token-invitation").unwrap();
    let invitation = sitolo_domain::tenancy::InvitationId::new("inv-token-invitation").unwrap();
    let membership = MembershipId::new("member-token-invitation").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();

    let tenancy = service(&db, Arc::new(PostgresAuditWriter::new()));
    let created = tenancy
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-token-invitation").unwrap(),
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "invitee@example.test").unwrap(),
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-operator".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        created.membership.state,
        sitolo_domain::tenancy::MembershipState::Invited
    );
    let persisted: (String, bool, Option<String>, i64) = sqlx_core::query_as::query_as(
        "SELECT state, token_hash IS NOT NULL, contact_value, state_version FROM invitations WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(invitation.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(persisted.0, "ISSUED");
    assert!(persisted.1);
    assert_eq!(persisted.2.as_deref(), Some("invitee@example.test"));
    assert_eq!(persisted.3, 1);
    let audit_target: (String, String) = sqlx_core::query_as::query_as(
        "SELECT target_type, target_ref FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.invitation.issued'",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(
        audit_target,
        ("Invitation".to_owned(), invitation.to_string())
    );
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.invitation.issued') + (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.invitation.issued')",
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

async fn assert_invitation_rows_absent(
    db: &TestDb,
    organization_id: &OrganizationId,
) -> Result<(), Box<dyn std::error::Error>> {
    let persisted_rows: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM organization_memberships WHERE organization_id = $1) + (SELECT count(*) FROM invitations WHERE organization_id = $1)",
    )
    .bind(organization_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(persisted_rows, 0);
    Ok(())
}

fn token_invitation_arguments(
    org: OrganizationId,
    invitation_id: &str,
    membership_id: &str,
) -> (
    OrganizationId,
    sitolo_domain::tenancy::InvitationId,
    MembershipId,
    sitolo_domain::tenancy::TenantUserId,
    InvitationContact,
) {
    (
        org,
        sitolo_domain::tenancy::InvitationId::new(invitation_id).unwrap(),
        MembershipId::new(membership_id).unwrap(),
        sitolo_domain::tenancy::TenantUserId::new("user-invitation-failure").unwrap(),
        InvitationContact::new(ContactKind::Email, "invitee@example.test").unwrap(),
    )
}

#[tokio::test]
async fn token_invitation_issue_rolls_back_when_audit_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-invitation-audit-rollback").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();
    let tenancy = service(&db, Arc::new(FailingAuditWriter));
    let (org, invitation, membership, user, contact) =
        token_invitation_arguments(org.clone(), "inv-audit-rollback", "member-audit-rollback");
    assert!(
        tenancy
            .create_invitation(
                org.clone(),
                invitation,
                membership,
                user,
                Role::Cashier,
                None,
                contact,
                "0123456789abcdef0123456789abcdef",
                std::time::Duration::from_secs(3_600),
                ActorRef {
                    subject_ref: "user-operator".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await
            .is_err()
    );
    assert_invitation_rows_absent(&db, &org).await.unwrap();
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn token_invitation_issue_rolls_back_when_outbox_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-invitation-outbox-rollback").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();
    let tenancy = TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new()).with_evidence(
        TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(FailingOutboxWriter),
        ),
    );
    let (org, invitation, membership, user, contact) =
        token_invitation_arguments(org.clone(), "inv-outbox-rollback", "member-outbox-rollback");
    assert!(
        tenancy
            .create_invitation(
                org.clone(),
                invitation,
                membership,
                user,
                Role::Cashier,
                None,
                contact,
                "0123456789abcdef0123456789abcdef",
                std::time::Duration::from_secs(3_600),
                ActorRef {
                    subject_ref: "user-operator".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await
            .is_err()
    );
    assert_invitation_rows_absent(&db, &org).await.unwrap();
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn token_invitation_withdrawal_erases_contact_and_token_hash_atomically() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-invitation-withdrawal").unwrap();
    let invitation = sitolo_domain::tenancy::InvitationId::new("inv-withdrawal").unwrap();
    let membership = MembershipId::new("member-withdrawal").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();
    let tenancy = service(&db, Arc::new(PostgresAuditWriter::new()));
    tenancy
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-withdrawal").unwrap(),
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "withdrawal@example.test").unwrap(),
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await
        .unwrap();

    let withdrawn = tenancy
        .revoke_invitation(
            &org,
            &invitation,
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(withdrawn.state, sitolo_authz::InvitationState::Revoked);
    assert!(withdrawn.contact.is_none());
    assert!(withdrawn.token_hash.is_none());
    let stored: (String, bool, Option<String>, i64) = sqlx_core::query_as::query_as(
        "SELECT state, token_hash IS NOT NULL, contact_value, state_version FROM invitations WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(invitation.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(stored.0, "WITHDRAWN");
    assert!(!stored.1);
    assert!(stored.2.is_none());
    assert_eq!(stored.3, 2);
    let membership_state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(membership_state, ("EXPIRED".to_owned(), 3));
    let event_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(event_count, 2);
    let event_payloads: Vec<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT payload::text FROM outbox_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_all(&db.admin)
    .await
    .unwrap();
    assert!(
        event_payloads
            .iter()
            .all(|payload| !payload.contains("withdrawal@example.test")
                && !payload.contains("0123456789abcdef0123456789abcdef"))
    );
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn concurrent_token_invitation_withdrawals_emit_one_transition() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-withdrawal-concurrent").unwrap();
    let invitation =
        sitolo_domain::tenancy::InvitationId::new("inv-withdrawal-concurrent").unwrap();
    let membership = MembershipId::new("member-withdrawal-concurrent").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();
    let tenancy = service(&db, Arc::new(PostgresAuditWriter::new()));
    tenancy
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership,
            sitolo_domain::tenancy::TenantUserId::new("user-withdrawal-concurrent").unwrap(),
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "concurrent@example.test").unwrap(),
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await
        .unwrap();

    let (first, second) = tokio::join!(
        tenancy.revoke_invitation(
            &org,
            &invitation,
            ActorRef {
                subject_ref: "user-issuer-a".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        ),
        tenancy.revoke_invitation(
            &org,
            &invitation,
            ActorRef {
                subject_ref: "user-issuer-b".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        ),
    );
    assert_ne!(first.is_ok(), second.is_ok());

    let event_counts: (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.invitation.withdrawn'), (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.invitation.withdrawn')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(event_counts, (1, 1));
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn token_invitation_withdrawal_rolls_back_when_audit_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-withdrawal-audit-rollback").unwrap();
    let invitation = sitolo_domain::tenancy::InvitationId::new("inv-withdrawal-audit").unwrap();
    let membership = MembershipId::new("member-withdrawal-audit").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();
    let create_service = service(&db, Arc::new(PostgresAuditWriter::new()));
    create_service
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-withdrawal-audit").unwrap(),
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "audit@example.test").unwrap(),
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await
        .unwrap();

    let revoke_service = service(&db, Arc::new(FailingAuditWriter));
    assert!(
        revoke_service
            .revoke_invitation(
                &org,
                &invitation,
                ActorRef {
                    subject_ref: "user-issuer".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await
            .is_err()
    );
    let invitation_state: (String, bool, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT state, token_hash IS NOT NULL, contact_value FROM invitations WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(invitation.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(invitation_state.0, "ISSUED");
    assert!(invitation_state.1);
    assert_eq!(invitation_state.2.as_deref(), Some("audit@example.test"));
    let membership_state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(membership_state, ("INVITED".to_owned(), 1));
    let event_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(event_count, 1);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn token_invitation_withdrawal_rolls_back_when_outbox_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-withdrawal-outbox-rollback").unwrap();
    let invitation = sitolo_domain::tenancy::InvitationId::new("inv-withdrawal-outbox").unwrap();
    let membership = MembershipId::new("member-withdrawal-outbox").unwrap();
    seed_organization(&db, org.as_str()).await.unwrap();
    let create_service = service(&db, Arc::new(PostgresAuditWriter::new()));
    create_service
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-withdrawal-outbox").unwrap(),
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "outbox@example.test").unwrap(),
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await
        .unwrap();

    let revoke_service = TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new())
        .with_evidence(TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(FailingOutboxWriter),
        ));
    assert!(
        revoke_service
            .revoke_invitation(
                &org,
                &invitation,
                ActorRef {
                    subject_ref: "user-issuer".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await
            .is_err()
    );

    let invitation_state: (String, bool, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT state, token_hash IS NOT NULL, contact_value FROM invitations WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(invitation.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(invitation_state.0, "ISSUED");
    assert!(invitation_state.1);
    assert_eq!(invitation_state.2.as_deref(), Some("outbox@example.test"));
    let membership_state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(membership_state, ("INVITED".to_owned(), 1));
    let event_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(event_count, 1);
    let audit_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM audit_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(audit_count, 1);
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

#[tokio::test]
async fn invitation_expiry_sweep_commits_both_transitions_and_is_duplicate_safe() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-invitation-expiry-sweep").unwrap();
    let invitation = "inv-expiry-sweep";
    let membership = "member-expiry-sweep";
    seed_organization(&db, org.as_str()).await.unwrap();
    let tenancy = service(&db, Arc::new(PostgresAuditWriter::new()));
    issue_invitation(&tenancy, &org, invitation, membership).await;
    make_invitation_expired(&db, org.as_str(), invitation).await;

    let (first, second) = tokio::join!(
        tenancy.expire_invitations_for_organization(&org),
        tenancy.expire_invitations_for_organization(&org),
    );
    assert_eq!(first.unwrap() + second.unwrap(), 1);
    assert_eq!(
        tenancy
            .expire_invitations_for_organization(&org)
            .await
            .unwrap(),
        0
    );

    let invitation_row: (String, Option<String>, Option<String>, Option<Vec<u8>>, i64) =
        sqlx_core::query_as::query_as(
            "SELECT state, contact_kind, contact_value, token_hash, state_version \
             FROM invitations WHERE organization_id = $1 AND id = $2",
        )
        .bind(org.as_str())
        .bind(invitation)
        .fetch_one(&db.admin)
        .await
        .unwrap();
    assert_eq!(invitation_row, ("EXPIRED".to_owned(), None, None, None, 2));
    let membership_state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership)
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(membership_state, ("EXPIRED".to_owned(), 2));
    let event_names: Vec<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT event_name FROM outbox_events WHERE organization_id = $1 ORDER BY event_name",
    )
    .bind(org.as_str())
    .fetch_all(&db.admin)
    .await
    .unwrap();
    assert_eq!(
        event_names,
        vec![
            "iam.invitation.expired".to_owned(),
            "iam.invitation.issued".to_owned(),
            "iam.membership.expired".to_owned(),
        ]
    );
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn invitation_expiry_sweep_rolls_back_when_audit_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-invitation-expiry-audit-rollback").unwrap();
    let invitation = "inv-expiry-audit-rollback";
    let membership = "member-expiry-audit-rollback";
    seed_organization(&db, org.as_str()).await.unwrap();
    let issuer = service(&db, Arc::new(PostgresAuditWriter::new()));
    issue_invitation(&issuer, &org, invitation, membership).await;
    make_invitation_expired(&db, org.as_str(), invitation).await;
    let sweeper = service(&db, Arc::new(FailingAuditWriter));

    assert!(
        sweeper
            .expire_invitations_for_organization(&org)
            .await
            .is_err()
    );
    let row: (String, Option<String>, Option<Vec<u8>>) = sqlx_core::query_as::query_as(
        "SELECT state, contact_value, token_hash FROM invitations \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(invitation)
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(row.0, "ISSUED");
    assert!(row.1.is_some() && row.2.is_some());
    let membership_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership)
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(membership_state, "INVITED");
    let expiry_events: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE organization_id = $1 \
         AND event_name IN ('iam.invitation.expired', 'iam.membership.expired')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(expiry_events, 0);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn invitation_expiry_sweep_rolls_back_when_outbox_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-invitation-expiry-outbox-rollback").unwrap();
    let invitation = "inv-expiry-outbox-rollback";
    let membership = "member-expiry-outbox-rollback";
    seed_organization(&db, org.as_str()).await.unwrap();
    let issuer = service(&db, Arc::new(PostgresAuditWriter::new()));
    issue_invitation(&issuer, &org, invitation, membership).await;
    make_invitation_expired(&db, org.as_str(), invitation).await;
    let sweeper = TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new()).with_evidence(
        TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(FailingOutboxWriter),
        ),
    );

    assert!(
        sweeper
            .expire_invitations_for_organization(&org)
            .await
            .is_err()
    );
    let row: (String, Option<String>, Option<Vec<u8>>) = sqlx_core::query_as::query_as(
        "SELECT state, contact_value, token_hash FROM invitations \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(invitation)
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(row.0, "ISSUED");
    assert!(row.1.is_some() && row.2.is_some());
    let membership_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership)
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(membership_state, "INVITED");
    let expiry_audits: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM audit_events WHERE organization_id = $1 \
         AND event_name IN ('iam.invitation.expired', 'iam.membership.expired')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(expiry_audits, 0);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

async fn seed_pending_membership(db: &TestDb, organization_id: &str, membership_id: &str) {
    seed_organization(db, organization_id).await.unwrap();
    query(
        "INSERT INTO organization_memberships \
         (id, organization_id, user_id, state, state_version) \
         VALUES ($1, $2, 'user-expiring', 'PENDING_ACCEPTANCE', 2)",
    )
    .bind(membership_id)
    .bind(organization_id)
    .execute(&db.admin)
    .await
    .unwrap();
}

#[tokio::test]
async fn membership_expiry_rolls_back_when_audit_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-membership-expiry-audit-rollback").unwrap();
    let membership = MembershipId::new("member-expiry-audit-rollback").unwrap();
    seed_pending_membership(&db, org.as_str(), membership.as_str()).await;
    let tenancy = service(&db, Arc::new(FailingAuditWriter));

    assert!(tenancy.expire_invitation(&org, &membership).await.is_err());
    let state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(state, ("PENDING_ACCEPTANCE".to_owned(), 2));
    let expiry_events: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 \
                 AND event_name = 'iam.membership.expired') + \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 \
                 AND event_name = 'iam.membership.expired')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(expiry_events, 0);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}

#[tokio::test]
async fn membership_expiry_rolls_back_when_outbox_write_fails() {
    let db = test_db().await.unwrap();
    let org = OrganizationId::new("org-membership-expiry-outbox-rollback").unwrap();
    let membership = MembershipId::new("member-expiry-outbox-rollback").unwrap();
    seed_pending_membership(&db, org.as_str(), membership.as_str()).await;
    let tenancy = TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new()).with_evidence(
        TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(FailingOutboxWriter),
        ),
    );

    assert!(tenancy.expire_invitation(&org, &membership).await.is_err());
    let state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(state, ("PENDING_ACCEPTANCE".to_owned(), 2));
    let expiry_audits: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM audit_events WHERE organization_id = $1 \
         AND event_name = 'iam.membership.expired'",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    .unwrap();
    assert_eq!(expiry_audits, 0);
    raw_sql(&format!("DROP SCHEMA \"{}\" CASCADE", db.schema))
        .execute(&db.admin)
        .await
        .unwrap();
}
