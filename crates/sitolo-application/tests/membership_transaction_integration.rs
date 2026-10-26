//! PostgreSQL-backed integration tests for [`sitolo_application::TenancyService`].
//!
//! Every test here exercises the real durable `TenancyUnitOfWork` adapter
//! against an isolated schema: authorization, tenant scope, state transitions,
//! audit/outbox evidence, and transaction rollback. Evidence-failure cases use
//! PostgreSQL triggers selected by runtime connection identity, so failures
//! occur on actual audit/outbox inserts inside the authoritative transaction.
//! The suites fail rather than skip when PostgreSQL is unavailable, so a green
//! local unit run is never mistaken for database proof; CI supplies the three
//! DSNs.

use std::env;
use std::sync::Arc;
use std::time::SystemTime;

use sitolo_application::TenancyService;
use sitolo_audit::ActorRef;
use sitolo_auth::{
    Assurance, AuthenticatedPrincipal, SecurityContext, SecurityVersion, SessionClass, SessionId,
    SessionSecurityState, SessionState, UserId,
};
use sitolo_authz::{ContactKind, InvitationContact, Role, RoleAssignmentState, Scope};
use sitolo_domain::tenancy::{
    BranchId, InvitationId, Membership, RoleAssignmentId, ScopeGrantId, TenancyError,
};
use sitolo_domain::tenancy::{MembershipId, OrganizationId};
use sitolo_persistence::PgTenancyUnitOfWork;
use sitolo_tenancy::ScopeError;
use sitolo_testkit::postgres::{parse_disposable_targets, validate_generated_schema_name};
use sqlx_core::query::query;
use sqlx_core::raw_sql::raw_sql;
use sqlx_postgres::{PgPool, PgPoolOptions};
use tokio::sync::Barrier;
use uuid::Uuid;

fn authenticated_context(user_id: &str) -> Result<SecurityContext, Box<dyn std::error::Error>> {
    let session_id = SessionId::new("test-session")?;
    let assurance = Assurance::A1;
    let security_version = SecurityVersion(1);
    Ok(SecurityContext {
        principal: AuthenticatedPrincipal {
            subject_id: UserId::new(user_id.to_owned())?,
            session_id: session_id.clone(),
            device_id: None,
            authenticated_at: SystemTime::now(),
            assurance,
            security_version,
        },
        session: SessionSecurityState {
            session_id,
            state: SessionState::Active,
            assurance,
            security_version,
            class: SessionClass::Merchant,
        },
        device: None,
        assurance,
        request_id: sitolo_observability::RequestId::new_server(),
        trace: None,
    })
}

struct TestDb {
    admin: PgPool,
    runtime: PgPool,
    audit_failure_runtime: PgPool,
    outbox_failure_runtime: PgPool,
    schema: String,
}

async fn install_postgres_evidence_failure_triggers(
    admin: &PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    raw_sql(
        r"
        CREATE FUNCTION sitolo_test_reject_audit_insert() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
            IF current_setting('application_name', true) = 'sitolo_test_fail_audit' THEN
                RAISE EXCEPTION 'test audit insert failure' USING ERRCODE = '23514';
            END IF;
            RETURN NEW;
        END;
        $$;
        CREATE TRIGGER sitolo_test_reject_audit_insert
        BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION sitolo_test_reject_audit_insert();

        CREATE FUNCTION sitolo_test_reject_outbox_insert() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
            IF current_setting('application_name', true) = 'sitolo_test_fail_outbox' THEN
                RAISE EXCEPTION 'test outbox insert failure' USING ERRCODE = '23514';
            END IF;
            RETURN NEW;
        END;
        $$;
        CREATE TRIGGER sitolo_test_reject_outbox_insert
        BEFORE INSERT ON outbox_events
        FOR EACH ROW EXECUTE FUNCTION sitolo_test_reject_outbox_insert();
        ",
    )
    .execute(admin)
    .await?;
    Ok(())
}

async fn failure_runtime_pool(
    target: &sqlx_postgres::PgConnectOptions,
    schema: &str,
    application_name: &str,
) -> Result<PgPool, Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(target.clone().options([
            ("search_path", schema),
            ("application_name", application_name),
        ]))
        .await?;
    let actual_application_name: String =
        sqlx_core::query_scalar::query_scalar("SELECT current_setting('application_name')")
            .fetch_one(&pool)
            .await?;
    if actual_application_name != application_name {
        return Err("PostgreSQL evidence-failure session identity was not configured".into());
    }
    Ok(pool)
}

#[expect(
    clippy::disallowed_methods,
    reason = "FW-CLP-02: the integration-test harness reads the CI-provided DSNs; production configuration is parsed at the binary boundary."
)]
async fn test_db() -> Result<TestDb, Box<dyn std::error::Error>> {
    let schema = format!("test_membership_tx_{}", Uuid::new_v4().simple());
    validate_generated_schema_name(&schema).map_err(std::io::Error::other)?;
    let disposable = env::var("SITOLO_TEST_DATABASE_DISPOSABLE").unwrap_or_default();
    let admin_url = env::var("ADMIN_DATABASE_URL")?;
    let runtime_url = env::var("RUNTIME_DATABASE_URL")?;
    let worker_url = env::var("WORKER_DATABASE_URL")?;
    let targets = parse_disposable_targets(&disposable, &admin_url, &runtime_url, &worker_url)
        .map_err(std::io::Error::other)?;
    let bootstrap = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(targets.admin.clone())
        .await?;
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "CREATE SCHEMA \"{schema}\""
    )))
    .execute(&bootstrap)
    .await?;
    bootstrap.close().await;

    let admin_options = targets.admin.options([("search_path", schema.as_str())]);
    let admin = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(admin_options)
        .await?;
    raw_sql(include_str!(
        "../../sitolo-persistence/tests/fixtures/role_bootstrap.sql"
    ))
    .execute(&admin)
    .await?;
    let audit_outbox_schema = include_str!(
        "../../sitolo-persistence/schema/migrations/0002_phase4_part8_audit_outbox.sql"
    )
    .replace("__SCHEMA__", &format!("\"{schema}\""));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(
        audit_outbox_schema.as_str(),
    ))
    .execute(&admin)
    .await?;
    let tenancy_schema = include_str!(
        "../../sitolo-persistence/schema/migrations/0003_phase4_part8_tenancy_provisioning.sql"
    );
    raw_sql(tenancy_schema).execute(&admin).await?;

    install_postgres_evidence_failure_triggers(&admin).await?;

    let runtime_options = targets
        .runtime
        .clone()
        .options([("search_path", schema.as_str())]);
    let runtime = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(runtime_options.clone())
        .await?;
    let runtime_identity: String = sqlx_core::query_scalar::query_scalar("SELECT session_user")
        .fetch_one(&runtime)
        .await?;
    if runtime_identity != "app_runtime" {
        return Err(format!("expected app_runtime, got {runtime_identity}").into());
    }
    let audit_failure_runtime =
        failure_runtime_pool(&targets.runtime, &schema, "sitolo_test_fail_audit").await?;
    let outbox_failure_runtime =
        failure_runtime_pool(&targets.runtime, &schema, "sitolo_test_fail_outbox").await?;
    Ok(TestDb {
        admin,
        runtime,
        audit_failure_runtime,
        outbox_failure_runtime,
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

/// Seeds one active membership holding a cashier role narrowed to
/// `branch-authority-main`, with `branch-authority-other` outside the scope.
async fn seed_authority_snapshot(
    db: &TestDb,
    organization_id: &OrganizationId,
    membership_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    seed_active_membership(db, organization_id.as_str(), membership_id).await?;
    seed_active_branch(db, organization_id.as_str(), "branch-authority-main").await?;
    seed_active_branch(db, organization_id.as_str(), "branch-authority-other").await?;
    query("INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ('role-authority-read', $1, $2, 'CASHIER', 'EFFECTIVE', 1)")
        .bind(organization_id.as_str())
        .bind(membership_id)
        .execute(&db.admin)
        .await?;
    query("INSERT INTO scope_grants (id, organization_id, membership_id, scope_kind, branch_id, state, state_version) VALUES ('scope-authority-read', $1, $2, 'BRANCH', 'branch-authority-main', 'ACTIVE', 1)")
        .bind(organization_id.as_str())
        .bind(membership_id)
        .execute(&db.admin)
        .await?;
    Ok(())
}

#[tokio::test]
async fn operation_authorization_uses_one_persisted_tenant_authority_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let organization_id = OrganizationId::new("org-authority-read")?;
    let membership_id = "member-authority-read";
    seed_authority_snapshot(&db, &organization_id, membership_id).await?;

    // The reference store is deliberately empty. A successful result proves
    // the request authorization path reads the same PostgreSQL authority that
    // owns the mutation and evidence records.
    let tenancy = service(&db);
    let user_id = sitolo_domain::tenancy::TenantUserId::new("user-1")?;
    let main_branch = BranchId::new("branch-authority-main")?;
    let other_branch = BranchId::new("branch-authority-other")?;
    let roles = tenancy
        .member_roles(&organization_id, &MembershipId::new(membership_id)?)
        .await;
    assert_eq!(roles.len(), 1);
    assert_eq!(
        roles.first().map(|assignment| assignment.role),
        Some(Role::Cashier)
    );
    let grants = tenancy
        .member_grants(&organization_id, &MembershipId::new(membership_id)?)
        .await;
    assert_eq!(grants.len(), 1);
    assert_eq!(
        tenancy
            .member_permissions(&organization_id, &MembershipId::new(membership_id)?,)
            .await,
        [
            sitolo_authz::Permission::SaleCreate,
            sitolo_authz::Permission::SaleView,
        ]
        .into_iter()
        .collect()
    );
    let authorized = tenancy
        .authorize_user_operation(
            &user_id,
            &organization_id,
            Some(&main_branch),
            sitolo_authz::Permission::SaleCreate,
        )
        .await?;
    assert_eq!(authorized.membership_id.as_str(), membership_id);
    assert_eq!(
        tenancy
            .authorize_user_operation(
                &user_id,
                &organization_id,
                Some(&main_branch),
                sitolo_authz::Permission::SaleVoid,
            )
            .await,
        Err(ScopeError::PermissionDenied)
    );
    assert_eq!(
        tenancy
            .authorize_user_operation(
                &user_id,
                &organization_id,
                Some(&other_branch),
                sitolo_authz::Permission::SaleCreate,
            )
            .await,
        Err(ScopeError::ScopeDenied)
    );

    query("UPDATE organization_memberships SET state = 'SUSPENDED' WHERE organization_id = $1 AND id = $2")
        .bind(organization_id.as_str())
        .bind(membership_id)
        .execute(&db.admin)
        .await
        ?;
    assert!(
        tenancy
            .member_permissions(&organization_id, &MembershipId::new(membership_id)?,)
            .await
            .is_empty(),
        "a suspended persisted membership has no effective permissions"
    );
    assert_eq!(
        tenancy
            .authorize_user_operation(
                &user_id,
                &organization_id,
                Some(&main_branch),
                sitolo_authz::Permission::SaleCreate,
            )
            .await,
        Err(ScopeError::MembershipNotActive)
    );
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[derive(Clone, Copy)]
enum EvidenceFailure {
    Audit,
    Outbox,
}

fn service(db: &TestDb) -> TenancyService {
    TenancyService::new_postgres(
        Vec::new(),
        Arc::new(PgTenancyUnitOfWork::new(db.runtime.clone())),
    )
}

fn failing_service(db: &TestDb, mode: EvidenceFailure) -> TenancyService {
    let pool = match mode {
        EvidenceFailure::Audit => db.audit_failure_runtime.clone(),
        EvidenceFailure::Outbox => db.outbox_failure_runtime.clone(),
    };
    TenancyService::new_postgres(Vec::new(), Arc::new(PgTenancyUnitOfWork::new(pool)))
}

#[tokio::test]
async fn authenticated_membership_suspend_authorizes_and_records_server_actor()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let organization_id = OrganizationId::new("org-authenticated-suspend")?;
    seed_active_membership(&db, organization_id.as_str(), "member-admin").await?;
    query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ('member-target', $1, 'user-target', 'ACTIVE', 1), ('member-denied-target', $1, 'user-denied-target', 'ACTIVE', 1)")
        .bind(organization_id.as_str())
        .execute(&db.admin)
        .await?;
    query("INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ('role-admin-suspend', $1, 'member-admin', 'OWNER', 'EFFECTIVE', 1)")
        .bind(organization_id.as_str())
        .execute(&db.admin)
        .await?;
    let tenancy = service(&db);

    let suspended = tenancy
        .suspend_member_for_authenticated_user(
            &authenticated_context("user-1")?,
            &organization_id,
            &MembershipId::new("member-target")?,
        )
        .await?;
    assert_eq!(
        suspended.state,
        sitolo_domain::tenancy::MembershipState::Suspended
    );
    let actor: String = sqlx_core::query_scalar::query_scalar(
        "SELECT actor_subject_ref FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.suspended'",
    )
    .bind(organization_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(actor, "user-1");

    let denied = tenancy
        .suspend_member_for_authenticated_user(
            &authenticated_context("user-denied-target")?,
            &organization_id,
            &MembershipId::new("member-denied-target")?,
        )
        .await;
    assert!(matches!(
        denied,
        Err(sitolo_application::AuthorizedTenancyError::Authorization(
            ScopeError::PermissionDenied
        ))
    ));
    let (target_state, denied_evidence): (String, i64) = sqlx_core::query_as::query_as(
        "SELECT m.state, (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND target_ref = 'member-denied-target') + (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND aggregate_id = 'member-denied-target') FROM organization_memberships m WHERE m.organization_id = $1 AND m.id = 'member-denied-target'",
    )
    .bind(organization_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(target_state, "ACTIVE");
    assert_eq!(denied_evidence, 0);

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn authenticated_membership_revoke_authorizes_and_records_server_actor()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let organization_id = OrganizationId::new("org-authenticated-revoke")?;
    seed_active_membership(&db, organization_id.as_str(), "member-admin").await?;
    query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ('member-revoke-target', $1, 'user-revoke-target', 'ACTIVE', 1), ('member-revoke-denied', $1, 'user-revoke-denied', 'ACTIVE', 1)")
        .bind(organization_id.as_str())
        .execute(&db.admin)
        .await?;
    query("INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ('role-admin-revoke', $1, 'member-admin', 'OWNER', 'EFFECTIVE', 1)")
        .bind(organization_id.as_str())
        .execute(&db.admin)
        .await?;
    let tenancy = service(&db);

    let revoked = tenancy
        .revoke_member_for_authenticated_user(
            &authenticated_context("user-1")?,
            &organization_id,
            &MembershipId::new("member-revoke-target")?,
        )
        .await?;
    assert_eq!(
        revoked.state,
        sitolo_domain::tenancy::MembershipState::Revoked
    );
    let actor: String = sqlx_core::query_scalar::query_scalar(
        "SELECT actor_subject_ref FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.revoked'",
    )
    .bind(organization_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(actor, "user-1");

    let denied = tenancy
        .revoke_member_for_authenticated_user(
            &authenticated_context("user-revoke-denied")?,
            &organization_id,
            &MembershipId::new("member-revoke-denied")?,
        )
        .await;
    assert!(matches!(
        denied,
        Err(sitolo_application::AuthorizedTenancyError::Authorization(
            ScopeError::PermissionDenied
        ))
    ));
    let (target_state, denied_evidence): (String, i64) = sqlx_core::query_as::query_as(
        "SELECT m.state, (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND target_ref = 'member-revoke-denied') + (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND aggregate_id = 'member-revoke-denied') FROM organization_memberships m WHERE m.organization_id = $1 AND m.id = 'member-revoke-denied'",
    )
    .bind(organization_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(target_state, "ACTIVE");
    assert_eq!(denied_evidence, 0);

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

/// Seeds an active owner member with organization-wide scope, plus the role
/// target and the denied caller used by the authenticated assignment tests.
async fn seed_role_assignment_authority(
    db: &TestDb,
    organization_id: &OrganizationId,
) -> Result<(), Box<dyn std::error::Error>> {
    seed_active_membership(db, organization_id.as_str(), "member-admin").await?;
    query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ('member-role-target', $1, 'user-role-target', 'ACTIVE', 1), ('member-role-denied', $1, 'user-role-denied', 'ACTIVE', 1)")
        .bind(organization_id.as_str())
        .execute(&db.admin)
        .await?;
    query("INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ('role-admin-assignment', $1, 'member-admin', 'OWNER', 'EFFECTIVE', 1)")
        .bind(organization_id.as_str())
        .execute(&db.admin)
        .await?;
    query("INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ('branch-role-scope', $1, 'Main', 'ACTIVE', 1)")
        .bind(organization_id.as_str())
        .execute(&db.admin)
        .await?;
    query("INSERT INTO scope_grants (id, organization_id, membership_id, scope_kind, branch_id, state, state_version) VALUES ('scope-admin-assignment', $1, 'member-admin', 'ORGANIZATION', NULL, 'ACTIVE', 1)")
        .bind(organization_id.as_str())
        .execute(&db.admin)
        .await?;
    Ok(())
}

/// Proves the authenticated scope-grant path authorizes against the persisted
/// snapshot: the owner may grant and revoke, the unprivileged caller may not,
/// and the denied request leaves no grant behind.
async fn assert_authenticated_scope_grant_authorization(
    tenancy: &TenancyService,
    db: &TestDb,
    organization_id: &OrganizationId,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope_grant = tenancy
        .grant_scope_for_authenticated_user(
            &authenticated_context("user-1")?,
            organization_id.clone(),
            MembershipId::new("member-role-target")?,
            ScopeGrantId::new("scope-target-branch")?,
            Scope::Branch {
                organization_id: organization_id.clone(),
                branch_id: BranchId::new("branch-role-scope")?,
            },
        )
        .await?;
    assert_eq!(scope_grant.state, sitolo_authz::ScopeGrantState::Active);

    let denied_scope = tenancy
        .grant_scope_for_authenticated_user(
            &authenticated_context("user-role-target")?,
            organization_id.clone(),
            MembershipId::new("member-role-denied")?,
            ScopeGrantId::new("scope-denied")?,
            Scope::Organization {
                organization_id: organization_id.clone(),
            },
        )
        .await;
    assert!(matches!(
        denied_scope,
        Err(sitolo_application::AuthorizedTenancyError::Authorization(
            ScopeError::PermissionDenied
        ))
    ));

    tenancy
        .revoke_scope_grant_for_authenticated_user(
            &authenticated_context("user-1")?,
            organization_id,
            &ScopeGrantId::new("scope-target-branch")?,
        )
        .await?;

    let denied_scope_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM scope_grants WHERE organization_id = $1 AND id = 'scope-denied'",
    )
    .bind(organization_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(denied_scope_count, 0);
    Ok(())
}

#[tokio::test]
async fn authenticated_role_assignment_authorizes_and_records_server_actor()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let organization_id = OrganizationId::new("org-authenticated-role-assignment")?;
    seed_role_assignment_authority(&db, &organization_id).await?;
    let tenancy = service(&db);
    let assigned = tenancy
        .assign_role_for_authenticated_user(
            &authenticated_context("user-1")?,
            organization_id.clone(),
            MembershipId::new("member-role-target")?,
            RoleAssignmentId::new("assignment-target")?,
            Role::Cashier,
        )
        .await?;
    assert_eq!(assigned.state, RoleAssignmentState::Effective);
    let audit_target: (String, String, String) = sqlx_core::query_as::query_as(
        "SELECT target_type, target_ref, actor_subject_ref FROM audit_events \
         WHERE organization_id = $1 AND event_name = 'iam.role.assigned'",
    )
    .bind(organization_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(
        audit_target,
        (
            "Membership".to_owned(),
            "member-role-target".to_owned(),
            "user-1".to_owned(),
        )
    );
    let outbox_aggregate: (String, String) = sqlx_core::query_as::query_as(
        "SELECT aggregate_type, aggregate_id FROM outbox_events \
         WHERE organization_id = $1 AND event_name = 'iam.role.assigned'",
    )
    .bind(organization_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(
        outbox_aggregate,
        ("role_assignment".to_owned(), "assignment-target".to_owned())
    );

    let denied = tenancy
        .assign_role_for_authenticated_user(
            &authenticated_context("user-role-target")?,
            organization_id.clone(),
            MembershipId::new("member-role-denied")?,
            RoleAssignmentId::new("assignment-denied")?,
            Role::Viewer,
        )
        .await;
    assert!(matches!(
        denied,
        Err(sitolo_application::AuthorizedTenancyError::Authorization(
            ScopeError::PermissionDenied
        ))
    ));
    let (assignment_count, denied_evidence): (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM role_assignments WHERE organization_id = $1 AND id = 'assignment-denied'), \
         (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.role.assigned' AND target_ref = 'member-role-denied') + \
         (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND aggregate_id = 'assignment-denied')",
    )
    .bind(organization_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(assignment_count, 0);
    assert_eq!(denied_evidence, 0);

    assert_authenticated_scope_grant_authorization(&tenancy, &db, &organization_id).await?;
    tenancy
        .revoke_role_for_authenticated_user(
            &authenticated_context("user-1")?,
            &organization_id,
            &MembershipId::new("member-role-target")?,
            Role::Cashier,
        )
        .await?;

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

async fn issue_invitation(
    tenancy: &TenancyService,
    organization_id: &OrganizationId,
    invitation_id: &str,
    membership_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tenancy
        .create_invitation(
            organization_id.clone(),
            InvitationId::new(invitation_id)?,
            MembershipId::new(membership_id)?,
            sitolo_domain::tenancy::TenantUserId::new(format!("user-{membership_id}"))?,
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "sweep@example.test")?,
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;
    Ok(())
}

async fn make_invitation_expired(
    db: &TestDb,
    organization_id: &str,
    invitation_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    query(
        "UPDATE invitations SET issued_at = now() - interval '2 minutes', \
         expires_at = now() - interval '1 second' \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(organization_id)
    .bind(invitation_id)
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn organization_and_branch_lifecycle_changes_commit_with_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-lifecycle-atomicity")?;
    let branch = BranchId::new("branch-lifecycle-atomicity")?;
    query("INSERT INTO organizations (id, name, state, state_version) VALUES ($1, 'Org', 'PROVISIONING', 1)")
        .bind(org.as_str())
        .execute(&db.admin)
        .await
        ?;
    seed_provisioning_branch(&db, org.as_str(), branch.as_str()).await?;
    let tenancy = service(&db);
    let actor = |subject: &str| ActorRef {
        subject_ref: subject.to_owned(),
        membership_ref: None,
        device_ref: None,
    };

    tenancy
        .activate_organization(&org, actor("org-operator"))
        .await?;
    tenancy
        .suspend_organization(&org, actor("org-operator"))
        .await?;
    tenancy
        .resume_organization(&org, actor("org-operator"))
        .await?;
    tenancy
        .activate_branch(&org, &branch, actor("branch-operator"))
        .await?;
    tenancy
        .suspend_branch(&org, &branch, actor("branch-operator"))
        .await?;
    tenancy
        .resume_branch(&org, &branch, actor("branch-operator"))
        .await?;
    tenancy
        .begin_close_branch(&org, &branch, actor("branch-operator"))
        .await?;
    tenancy
        .close_branch(&org, &branch, actor("branch-operator"))
        .await?;
    tenancy
        .begin_close_organization(&org, actor("org-operator"))
        .await?;
    tenancy
        .close_organization(&org, actor("org-operator"))
        .await?;

    let persisted: (String, i64, String, i64) = sqlx_core::query_as::query_as(
        "SELECT organizations.state, organizations.state_version, branches.state, branches.state_version FROM organizations JOIN branches ON branches.organization_id = organizations.id WHERE organizations.id = $1 AND branches.id = $2",
    )
    .bind(org.as_str())
    .bind(branch.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(persisted, ("CLOSED".to_owned(), 6, "CLOSED".to_owned(), 6));
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(evidence_count, 20);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn organization_and_branch_lifecycle_failures_roll_back_state_and_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-lifecycle-rollback")?;
    let branch = BranchId::new("branch-lifecycle-rollback")?;
    seed_organization(&db, org.as_str()).await?;
    seed_active_branch(&db, org.as_str(), branch.as_str()).await?;
    let actor = || ActorRef {
        subject_ref: "lifecycle-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };

    let audit_failure = failing_service(&db, EvidenceFailure::Audit);
    assert!(
        audit_failure
            .suspend_organization(&org, actor())
            .await
            .is_err()
    );
    let outbox_failure = failing_service(&db, EvidenceFailure::Outbox);
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
    ?;
    assert_eq!(state, ("ACTIVE".to_owned(), 2, "ACTIVE".to_owned(), 2));
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(evidence_count, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

/// One lifecycle transition exercised by the evidence-failure rollback matrix.
#[derive(Clone, Copy)]
enum Transition {
    ActivateOrganization,
    SuspendOrganization,
    ResumeOrganization,
    BeginCloseOrganization,
    CloseOrganization,
    ActivateBranch,
    SuspendBranch,
    ResumeBranch,
    BeginCloseBranch,
    CloseBranch,
}

impl Transition {
    /// Applies this transition through the service, returning the domain error
    /// the caller observes when evidence insertion fails.
    async fn execute(
        self,
        tenancy: &TenancyService,
        organization_id: &OrganizationId,
        branch_id: &BranchId,
        actor: ActorRef,
    ) -> Result<(), TenancyError> {
        match self {
            Self::ActivateOrganization => {
                tenancy
                    .activate_organization(organization_id, actor)
                    .await?;
            }
            Self::SuspendOrganization => {
                tenancy.suspend_organization(organization_id, actor).await?;
            }
            Self::ResumeOrganization => {
                tenancy.resume_organization(organization_id, actor).await?;
            }
            Self::BeginCloseOrganization => {
                tenancy
                    .begin_close_organization(organization_id, actor)
                    .await?;
            }
            Self::CloseOrganization => {
                tenancy.close_organization(organization_id, actor).await?;
            }
            Self::ActivateBranch => {
                tenancy
                    .activate_branch(organization_id, branch_id, actor)
                    .await?;
            }
            Self::SuspendBranch => {
                tenancy
                    .suspend_branch(organization_id, branch_id, actor)
                    .await?;
            }
            Self::ResumeBranch => {
                tenancy
                    .resume_branch(organization_id, branch_id, actor)
                    .await?;
            }
            Self::BeginCloseBranch => {
                tenancy
                    .begin_close_branch(organization_id, branch_id, actor)
                    .await?;
            }
            Self::CloseBranch => {
                tenancy
                    .close_branch(organization_id, branch_id, actor)
                    .await?;
            }
        }
        Ok(())
    }
}

/// `(case name, transition, organization state, optional branch state)`.
const TRANSITION_CASES: [(&str, Transition, &str, Option<&str>); 10] = [
    (
        "activate-organization",
        Transition::ActivateOrganization,
        "PROVISIONING",
        None,
    ),
    (
        "suspend-organization",
        Transition::SuspendOrganization,
        "ACTIVE",
        None,
    ),
    (
        "resume-organization",
        Transition::ResumeOrganization,
        "SUSPENDED",
        None,
    ),
    (
        "begin-close-organization",
        Transition::BeginCloseOrganization,
        "ACTIVE",
        None,
    ),
    (
        "close-organization",
        Transition::CloseOrganization,
        "CLOSING",
        None,
    ),
    (
        "activate-branch",
        Transition::ActivateBranch,
        "ACTIVE",
        Some("PROVISIONING"),
    ),
    (
        "suspend-branch",
        Transition::SuspendBranch,
        "ACTIVE",
        Some("ACTIVE"),
    ),
    (
        "resume-branch",
        Transition::ResumeBranch,
        "ACTIVE",
        Some("SUSPENDED"),
    ),
    (
        "begin-close-branch",
        Transition::BeginCloseBranch,
        "ACTIVE",
        Some("ACTIVE"),
    ),
    (
        "close-branch",
        Transition::CloseBranch,
        "ACTIVE",
        Some("CLOSING"),
    ),
];

#[tokio::test]
async fn every_organization_and_branch_transition_rolls_back_on_audit_and_outbox_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    for (name, transition, organization_state, branch_state) in TRANSITION_CASES {
        let audit_org = OrganizationId::new(format!("org-{name}-audit"))?;
        let outbox_org = OrganizationId::new(format!("org-{name}-outbox"))?;
        let audit_branch = BranchId::new(format!("branch-{name}-audit"))?;
        let outbox_branch = BranchId::new(format!("branch-{name}-outbox"))?;
        for (organization_id, branch_id) in
            [(&audit_org, &audit_branch), (&outbox_org, &outbox_branch)]
        {
            query("INSERT INTO organizations (id, name, state, state_version) VALUES ($1, 'Org', $2, 3)")
                .bind(organization_id.as_str())
                .bind(organization_state)
                .execute(&db.admin)
                .await
                ?;
            if let Some(branch_state) = branch_state {
                query("INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ($1, $2, 'Main', $3, 4)")
                    .bind(branch_id.as_str())
                    .bind(organization_id.as_str())
                    .bind(branch_state)
                    .execute(&db.admin)
                    .await
                    ?;
            }
        }

        let audit_service = failing_service(&db, EvidenceFailure::Audit);
        let outbox_service = failing_service(&db, EvidenceFailure::Outbox);
        let actor = || ActorRef {
            subject_ref: "lifecycle-operator".to_owned(),
            membership_ref: None,
            device_ref: None,
        };

        assert!(
            transition
                .execute(&audit_service, &audit_org, &audit_branch, actor())
                .await
                .is_err(),
            "{name} must abort when audit insertion fails"
        );
        assert!(
            transition
                .execute(&outbox_service, &outbox_org, &outbox_branch, actor())
                .await
                .is_err(),
            "{name} must abort when outbox insertion fails"
        );

        for organization_id in [&audit_org, &outbox_org] {
            let persisted: (String, i64, Option<String>, Option<i64>) = sqlx_core::query_as::query_as(
                "SELECT organizations.state, organizations.state_version, branches.state, branches.state_version \
                 FROM organizations LEFT JOIN branches ON branches.organization_id = organizations.id \
                 WHERE organizations.id = $1",
            )
            .bind(organization_id.as_str())
            .fetch_one(&db.admin)
            .await
            ?;
            assert_eq!(
                persisted,
                (
                    organization_state.to_owned(),
                    3,
                    branch_state.map(str::to_owned),
                    branch_state.map(|_| 4),
                ),
                "{name} must preserve business state and version on failure for {}",
                organization_id.as_str()
            );
            let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
                "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + \
                        (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
            )
            .bind(organization_id.as_str())
            .fetch_one(&db.admin)
            .await?;
            assert_eq!(evidence_count, 0, "{name} must not leave partial evidence");
        }
    }
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn branch_creation_rolls_back_state_and_evidence_on_write_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-branch-create-rollback")?;
    seed_organization(&db, org.as_str()).await?;
    let actor = || ActorRef {
        subject_ref: "user-branch-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };

    let audit_failure = failing_service(&db, EvidenceFailure::Audit)
        .create_branch(
            BranchId::new("branch-create-audit-failure")?,
            org.clone(),
            "Audit Failure",
            actor(),
        )
        .await;
    assert!(audit_failure.is_err());

    let outbox_failure = failing_service(&db, EvidenceFailure::Outbox)
        .create_branch(
            BranchId::new("branch-create-outbox-failure")?,
            org.clone(),
            "Outbox Failure",
            actor(),
        )
        .await;
    assert!(outbox_failure.is_err());

    let counts: (i64, i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM branches WHERE organization_id = $1), \
                (SELECT count(*) FROM audit_events WHERE organization_id = $1), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(counts, (0, 0, 0));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn competing_organization_transitions_and_cross_tenant_branch_update_are_denied()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-lifecycle-race")?;
    let other_org = OrganizationId::new("org-lifecycle-other")?;
    let branch = BranchId::new("branch-lifecycle-cross-tenant")?;
    seed_organization(&db, org.as_str()).await?;
    seed_organization(&db, other_org.as_str()).await?;
    seed_active_branch(&db, org.as_str(), branch.as_str()).await?;
    let tenancy = service(&db);

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
    .await?;
    assert_eq!(branch_state, ("ACTIVE".to_owned(), 2));
    let event_counts: (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1), (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(event_counts, (1, 1));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn member_invitation_and_required_evidence_commit_together()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-member-invitation-atomicity")?;
    let membership = MembershipId::new("member-invitation-atomicity")?;
    seed_organization(&db, org.as_str()).await?;

    let tenancy = service(&db);
    let created = tenancy
        .invite_member(
            membership.clone(),
            org.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-invited")?,
            ActorRef {
                subject_ref: "user-operator".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;
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
    .await?;
    assert_eq!(persisted_state, "INVITED");
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.invited') + (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.membership.invited')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(evidence_count, 2);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn member_invitation_rolls_back_state_and_evidence_on_write_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-member-invitation-rollback")?;
    seed_organization(&db, org.as_str()).await?;
    let actor = || ActorRef {
        subject_ref: "user-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };

    let audit_failure = failing_service(&db, EvidenceFailure::Audit)
        .invite_member(
            MembershipId::new("member-invitation-audit-failure")?,
            org.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-invitation-audit-failure")?,
            actor(),
        )
        .await;
    assert!(audit_failure.is_err());

    let outbox_failure = failing_service(&db, EvidenceFailure::Outbox)
        .invite_member(
            MembershipId::new("member-invitation-outbox-failure")?,
            org.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-invitation-outbox-failure")?,
            actor(),
        )
        .await;
    assert!(outbox_failure.is_err());

    let counts: (i64, i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM organization_memberships WHERE organization_id = $1), \
                (SELECT count(*) FROM audit_events WHERE organization_id = $1), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(counts, (0, 0, 0));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn token_invitation_issue_persists_only_hash_and_emits_approved_event()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-token-invitation")?;
    let invitation = InvitationId::new("inv-token-invitation")?;
    let membership = MembershipId::new("member-token-invitation")?;
    seed_organization(&db, org.as_str()).await?;

    let tenancy = service(&db);
    let created = tenancy
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-token-invitation")?,
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "invitee@example.test")?,
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-operator".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;
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
    ?;
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
    ?;
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
    ?;
    assert_eq!(evidence_count, 2);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
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

type TokenInvitationArguments = (
    OrganizationId,
    InvitationId,
    MembershipId,
    sitolo_domain::tenancy::TenantUserId,
    InvitationContact,
);

fn token_invitation_arguments(
    org: OrganizationId,
    invitation_id: &str,
    membership_id: &str,
) -> Result<TokenInvitationArguments, Box<dyn std::error::Error>> {
    Ok((
        org,
        InvitationId::new(invitation_id)?,
        MembershipId::new(membership_id)?,
        sitolo_domain::tenancy::TenantUserId::new("user-invitation-failure")?,
        InvitationContact::new(ContactKind::Email, "invitee@example.test")?,
    ))
}

#[tokio::test]
async fn token_invitation_issue_rolls_back_when_audit_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-invitation-audit-rollback")?;
    seed_organization(&db, org.as_str()).await?;
    let tenancy = failing_service(&db, EvidenceFailure::Audit);
    let (org, invitation, membership, user, contact) =
        token_invitation_arguments(org.clone(), "inv-audit-rollback", "member-audit-rollback")?;
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
    assert_invitation_rows_absent(&db, &org).await?;
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn token_invitation_issue_rolls_back_when_outbox_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-invitation-outbox-rollback")?;
    seed_organization(&db, org.as_str()).await?;
    let tenancy = failing_service(&db, EvidenceFailure::Outbox);
    let (org, invitation, membership, user, contact) =
        token_invitation_arguments(org.clone(), "inv-outbox-rollback", "member-outbox-rollback")?;
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
    assert_invitation_rows_absent(&db, &org).await?;
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn token_invitation_withdrawal_erases_contact_and_token_hash_atomically()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-invitation-withdrawal")?;
    let invitation = InvitationId::new("inv-withdrawal")?;
    let membership = MembershipId::new("member-withdrawal")?;
    seed_organization(&db, org.as_str()).await?;
    let tenancy = service(&db);
    tenancy
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-withdrawal")?,
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "withdrawal@example.test")?,
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;

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
        .await?;
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
    ?;
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
    ?;
    assert_eq!(membership_state, ("EXPIRED".to_owned(), 3));
    let event_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(event_count, 2);
    let event_payloads: Vec<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT payload::text FROM outbox_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_all(&db.admin)
    .await?;
    assert!(
        event_payloads
            .iter()
            .all(|payload| !payload.contains("withdrawal@example.test")
                && !payload.contains("0123456789abcdef0123456789abcdef"))
    );
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn concurrent_token_invitation_withdrawals_emit_one_transition()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-withdrawal-concurrent")?;
    let invitation = InvitationId::new("inv-withdrawal-concurrent")?;
    let membership = MembershipId::new("member-withdrawal-concurrent")?;
    seed_organization(&db, org.as_str()).await?;
    let tenancy = service(&db);
    tenancy
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership,
            sitolo_domain::tenancy::TenantUserId::new("user-withdrawal-concurrent")?,
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "concurrent@example.test")?,
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;

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
    ?;
    assert_eq!(event_counts, (1, 1));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn token_invitation_withdrawal_rolls_back_when_audit_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-withdrawal-audit-rollback")?;
    let invitation = InvitationId::new("inv-withdrawal-audit")?;
    let membership = MembershipId::new("member-withdrawal-audit")?;
    seed_organization(&db, org.as_str()).await?;
    let create_service = service(&db);
    create_service
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-withdrawal-audit")?,
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "audit@example.test")?,
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;

    let revoke_service = failing_service(&db, EvidenceFailure::Audit);
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
    ?;
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
    ?;
    assert_eq!(membership_state, ("INVITED".to_owned(), 1));
    let event_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(event_count, 1);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn token_invitation_withdrawal_rolls_back_when_outbox_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-withdrawal-outbox-rollback")?;
    let invitation = InvitationId::new("inv-withdrawal-outbox")?;
    let membership = MembershipId::new("member-withdrawal-outbox")?;
    seed_organization(&db, org.as_str()).await?;
    let create_service = service(&db);
    create_service
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-withdrawal-outbox")?,
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "outbox@example.test")?,
            "0123456789abcdef0123456789abcdef",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;

    let revoke_service = failing_service(&db, EvidenceFailure::Outbox);
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
    ?;
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
    ?;
    assert_eq!(membership_state, ("INVITED".to_owned(), 1));
    let event_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(event_count, 1);
    let audit_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM audit_events WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(audit_count, 1);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn membership_transition_and_audit_outbox_commit_together()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-membership-atomicity")?;
    let membership = MembershipId::new("member-membership-atomicity")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;

    let tenancy = service(&db);
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
        .await?;
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
    .await?;
    assert_eq!(persisted_state, "SUSPENDED");
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(evidence_count, 2);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn membership_transition_rolls_back_when_audit_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-membership-rollback")?;
    let membership = MembershipId::new("member-membership-rollback")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;

    let tenancy = failing_service(&db, EvidenceFailure::Audit);
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
    .await?;
    assert_eq!(persisted_state, "ACTIVE");
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(evidence_count, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn authenticated_membership_suspend_and_revoke_roll_back_on_evidence_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let cases = [
        ("org-auth-suspend-audit", true, true),
        ("org-auth-suspend-outbox", true, false),
        ("org-auth-revoke-audit", false, true),
        ("org-auth-revoke-outbox", false, false),
    ];

    for (organization_raw, suspend, fail_audit) in cases {
        let organization_id = OrganizationId::new(organization_raw)?;
        let owner_membership_id = format!("{organization_raw}-owner");
        let target_membership_id = format!("{organization_raw}-target");
        seed_active_membership(&db, organization_id.as_str(), &owner_membership_id).await?;
        query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ($1, $2, 'user-target', 'ACTIVE', 1)")
            .bind(&target_membership_id)
            .bind(organization_id.as_str())
            .execute(&db.admin)
            .await?;
        query("INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ($1, $2, $3, 'OWNER', 'EFFECTIVE', 1)")
            .bind(format!("{organization_raw}-role"))
            .bind(organization_id.as_str())
            .bind(&owner_membership_id)
            .execute(&db.admin)
            .await?;

        let tenancy = if fail_audit {
            failing_service(&db, EvidenceFailure::Audit)
        } else {
            failing_service(&db, EvidenceFailure::Outbox)
        };
        let result = if suspend {
            tenancy
                .suspend_member_for_authenticated_user(
                    &authenticated_context("user-1")?,
                    &organization_id,
                    &MembershipId::new(target_membership_id.clone())?,
                )
                .await
                .map(|_| ())
        } else {
            tenancy
                .revoke_member_for_authenticated_user(
                    &authenticated_context("user-1")?,
                    &organization_id,
                    &MembershipId::new(target_membership_id.clone())?,
                )
                .await
                .map(|_| ())
        };
        assert!(
            result.is_err(),
            "expected evidence failure for {organization_raw}"
        );

        let (state, evidence_count): (String, i64) = sqlx_core::query_as::query_as(
            "SELECT m.state, \
             (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND target_ref = $2) + \
             (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND aggregate_id = $2) \
             FROM organization_memberships m WHERE m.organization_id = $1 AND m.id = $2",
        )
        .bind(organization_id.as_str())
        .bind(&target_membership_id)
        .fetch_one(&db.admin)
        .await?;
        assert_eq!(state, "ACTIVE", "membership changed for {organization_raw}");
        assert_eq!(
            evidence_count, 0,
            "evidence remained for {organization_raw}"
        );
    }

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn membership_transition_rolls_back_when_outbox_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-membership-outbox-rollback")?;
    let membership = MembershipId::new("member-membership-outbox-rollback")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;

    let tenancy = failing_service(&db, EvidenceFailure::Outbox);
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
    .await?;
    assert_eq!(persisted_state, "ACTIVE");
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(evidence_count, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn invitation_expiry_sweep_commits_both_transitions_and_is_duplicate_safe()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-invitation-expiry-sweep")?;
    let invitation = "inv-expiry-sweep";
    let membership = "member-expiry-sweep";
    seed_organization(&db, org.as_str()).await?;
    let tenancy = service(&db);
    issue_invitation(&tenancy, &org, invitation, membership).await?;
    make_invitation_expired(&db, org.as_str(), invitation).await?;

    let (first, second) = tokio::join!(
        tenancy.expire_invitations_for_organization(&org),
        tenancy.expire_invitations_for_organization(&org),
    );
    assert_eq!(first?.saturating_add(second?), 1);
    assert_eq!(tenancy.expire_invitations_for_organization(&org).await?, 0);

    let invitation_row: (String, Option<String>, Option<String>, Option<Vec<u8>>, i64) =
        sqlx_core::query_as::query_as(
            "SELECT state, contact_kind, contact_value, token_hash, state_version \
             FROM invitations WHERE organization_id = $1 AND id = $2",
        )
        .bind(org.as_str())
        .bind(invitation)
        .fetch_one(&db.admin)
        .await?;
    assert_eq!(invitation_row, ("EXPIRED".to_owned(), None, None, None, 2));
    let membership_state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership)
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(membership_state, ("EXPIRED".to_owned(), 2));
    let event_names: Vec<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT event_name FROM outbox_events WHERE organization_id = $1 ORDER BY event_name",
    )
    .bind(org.as_str())
    .fetch_all(&db.admin)
    .await?;
    assert_eq!(
        event_names,
        vec![
            "iam.invitation.expired".to_owned(),
            "iam.invitation.issued".to_owned(),
            "iam.membership.expired".to_owned(),
        ]
    );
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn invitation_accept_withdraw_and_expiry_race_commits_one_terminal_transition()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-invitation-terminal-race")?;
    let invitation = "inv-terminal-race";
    let membership = "member-terminal-race";
    let token = "0123456789abcdef0123456789abcdef";
    seed_organization(&db, org.as_str()).await?;
    let tenancy = Arc::new(service(&db));
    tenancy
        .create_invitation(
            org.clone(),
            InvitationId::new(invitation)?,
            MembershipId::new(membership)?,
            sitolo_domain::tenancy::TenantUserId::new("user-terminal-race")?,
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "terminal-race@example.test")?,
            token,
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "user-issuer".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;
    make_invitation_expired(&db, org.as_str(), invitation).await?;
    let invitation_id = InvitationId::new(invitation)?;

    let (accepted, withdrawn, expired) = tokio::join!(
        tenancy.accept_invitation(
            token,
            RoleAssignmentId::new("assignment-terminal-race")?,
            None,
            ActorRef {
                subject_ref: "caller-accept".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        ),
        tenancy.revoke_invitation(
            &org,
            &invitation_id,
            ActorRef {
                subject_ref: "caller-withdraw".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        ),
        tenancy.expire_invitations_for_organization(&org),
    );

    assert!(accepted.is_err());
    let withdrawn = withdrawn.is_ok();
    let expired = expired? == 1;
    assert_ne!(withdrawn, expired);

    let invitation_row: (String, Option<String>, Option<Vec<u8>>) = sqlx_core::query_as::query_as(
        "SELECT state, contact_value, token_hash FROM invitations \
             WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(invitation)
    .fetch_one(&db.admin)
    .await?;
    assert!(matches!(invitation_row.0.as_str(), "WITHDRAWN" | "EXPIRED"));
    assert!(invitation_row.1.is_none() && invitation_row.2.is_none());

    let terminal_events: Vec<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT event_name FROM outbox_events WHERE organization_id = $1 \
         AND event_name IN ('iam.invitation.withdrawn', 'iam.invitation.expired')",
    )
    .bind(org.as_str())
    .fetch_all(&db.admin)
    .await?;
    assert_eq!(terminal_events.len(), 1);
    let terminal_event = terminal_events
        .first()
        .ok_or("expected one terminal invitation event")?;
    assert_eq!(
        terminal_event.as_str(),
        if expired {
            "iam.invitation.expired"
        } else {
            "iam.invitation.withdrawn"
        }
    );

    let membership_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership)
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(membership_state, "EXPIRED");

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn invitation_expiry_sweep_rolls_back_when_audit_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-invitation-expiry-audit-rollback")?;
    let invitation = "inv-expiry-audit-rollback";
    let membership = "member-expiry-audit-rollback";
    seed_organization(&db, org.as_str()).await?;
    let issuer = service(&db);
    issue_invitation(&issuer, &org, invitation, membership).await?;
    make_invitation_expired(&db, org.as_str(), invitation).await?;
    let sweeper = failing_service(&db, EvidenceFailure::Audit);

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
    .await?;
    assert_eq!(row.0, "ISSUED");
    assert!(row.1.is_some() && row.2.is_some());
    let membership_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership)
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(membership_state, "INVITED");
    let expiry_events: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE organization_id = $1 \
         AND event_name IN ('iam.invitation.expired', 'iam.membership.expired')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(expiry_events, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn invitation_expiry_sweep_rolls_back_when_outbox_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-invitation-expiry-outbox-rollback")?;
    let invitation = "inv-expiry-outbox-rollback";
    let membership = "member-expiry-outbox-rollback";
    seed_organization(&db, org.as_str()).await?;
    let issuer = service(&db);
    issue_invitation(&issuer, &org, invitation, membership).await?;
    make_invitation_expired(&db, org.as_str(), invitation).await?;
    let sweeper = failing_service(&db, EvidenceFailure::Outbox);

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
    .await?;
    assert_eq!(row.0, "ISSUED");
    assert!(row.1.is_some() && row.2.is_some());
    let membership_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership)
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(membership_state, "INVITED");
    let expiry_audits: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM audit_events WHERE organization_id = $1 \
         AND event_name IN ('iam.invitation.expired', 'iam.membership.expired')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(expiry_audits, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

async fn seed_pending_membership(
    db: &TestDb,
    organization_id: &str,
    membership_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    seed_organization(db, organization_id).await?;
    query(
        "INSERT INTO organization_memberships \
         (id, organization_id, user_id, state, state_version) \
         VALUES ($1, $2, 'user-expiring', 'PENDING_ACCEPTANCE', 2)",
    )
    .bind(membership_id)
    .bind(organization_id)
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn membership_expiry_rolls_back_when_audit_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-membership-expiry-audit-rollback")?;
    let membership = MembershipId::new("member-expiry-audit-rollback")?;
    seed_pending_membership(&db, org.as_str(), membership.as_str()).await?;
    let tenancy = failing_service(&db, EvidenceFailure::Audit);

    assert!(tenancy.expire_invitation(&org, &membership).await.is_err());
    let state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(state, ("PENDING_ACCEPTANCE".to_owned(), 2));
    let expiry_events: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 \
                 AND event_name = 'iam.membership.expired') + \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 \
                 AND event_name = 'iam.membership.expired')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(expiry_events, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn membership_expiry_rolls_back_when_outbox_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-membership-expiry-outbox-rollback")?;
    let membership = MembershipId::new("member-expiry-outbox-rollback")?;
    seed_pending_membership(&db, org.as_str(), membership.as_str()).await?;
    let tenancy = failing_service(&db, EvidenceFailure::Outbox);

    assert!(tenancy.expire_invitation(&org, &membership).await.is_err());
    let state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM organization_memberships \
         WHERE organization_id = $1 AND id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(state, ("PENDING_ACCEPTANCE".to_owned(), 2));
    let expiry_audits: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM audit_events WHERE organization_id = $1 \
         AND event_name = 'iam.membership.expired'",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(expiry_audits, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

/// One membership transition exercised by the evidence-failure rollback matrix.
#[derive(Clone, Copy)]
enum MembershipOperation {
    Accept,
    Resume,
    Revoke,
}

impl MembershipOperation {
    /// Applies this transition through `service`, returning the error the caller
    /// observes when evidence insertion fails.
    async fn execute(
        self,
        service: &TenancyService,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        actor: ActorRef,
    ) -> Result<Membership, TenancyError> {
        match self {
            Self::Accept => {
                service
                    .accept_member(organization_id, membership_id, actor)
                    .await
            }
            Self::Resume => {
                service
                    .resume_member(organization_id, membership_id, actor)
                    .await
            }
            Self::Revoke => {
                service
                    .revoke_member(organization_id, membership_id, actor)
                    .await
            }
        }
    }
}

/// Seeds one audit-failure and one outbox-failure target per membership
/// transition, all owned by `organization_id`.
async fn seed_membership_failure_fixtures(
    db: &TestDb,
    organization_id: &OrganizationId,
) -> Result<(), Box<dyn std::error::Error>> {
    seed_organization(db, organization_id.as_str()).await?;
    for (membership_id, user_id, state) in [
        (
            "member-accept-audit",
            "user-accept-audit",
            "PENDING_ACCEPTANCE",
        ),
        (
            "member-accept-outbox",
            "user-accept-outbox",
            "PENDING_ACCEPTANCE",
        ),
        ("member-resume-audit", "user-resume-audit", "SUSPENDED"),
        ("member-resume-outbox", "user-resume-outbox", "SUSPENDED"),
        ("member-revoke-audit", "user-revoke-audit", "ACTIVE"),
        ("member-revoke-outbox", "user-revoke-outbox", "ACTIVE"),
    ] {
        query(
            "INSERT INTO organization_memberships \
             (id, organization_id, user_id, state, state_version) \
             VALUES ($1, $2, $3, $4, 3)",
        )
        .bind(membership_id)
        .bind(organization_id.as_str())
        .bind(user_id)
        .bind(state)
        .execute(&db.admin)
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn membership_accept_resume_and_revoke_roll_back_on_audit_and_outbox_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-membership-transition-failures")?;
    seed_membership_failure_fixtures(&db, &org).await?;

    let actor = || ActorRef {
        subject_ref: "user-membership-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };
    let audit_service = failing_service(&db, EvidenceFailure::Audit);
    let outbox_service = failing_service(&db, EvidenceFailure::Outbox);

    for (audit_id, outbox_id, operation) in [
        (
            "member-accept-audit",
            "member-accept-outbox",
            MembershipOperation::Accept,
        ),
        (
            "member-resume-audit",
            "member-resume-outbox",
            MembershipOperation::Resume,
        ),
        (
            "member-revoke-audit",
            "member-revoke-outbox",
            MembershipOperation::Revoke,
        ),
    ] {
        let audit_membership = MembershipId::new(audit_id)?;
        let outbox_membership = MembershipId::new(outbox_id)?;
        let audit_result = operation
            .execute(&audit_service, &org, &audit_membership, actor())
            .await;
        assert!(audit_result.is_err(), "{audit_id} audit failure must abort");
        let outbox_result = operation
            .execute(&outbox_service, &org, &outbox_membership, actor())
            .await;
        assert!(
            outbox_result.is_err(),
            "{outbox_id} outbox failure must abort"
        );
    }

    let rows: Vec<(String, String, i64)> = sqlx_core::query_as::query_as(
        "SELECT id, state, state_version FROM organization_memberships \
         WHERE organization_id = $1 ORDER BY id",
    )
    .bind(org.as_str())
    .fetch_all(&db.admin)
    .await?;
    assert_eq!(
        rows,
        vec![
            (
                "member-accept-audit".to_owned(),
                "PENDING_ACCEPTANCE".to_owned(),
                3
            ),
            (
                "member-accept-outbox".to_owned(),
                "PENDING_ACCEPTANCE".to_owned(),
                3
            ),
            ("member-resume-audit".to_owned(), "SUSPENDED".to_owned(), 3),
            ("member-resume-outbox".to_owned(), "SUSPENDED".to_owned(), 3),
            ("member-revoke-audit".to_owned(), "ACTIVE".to_owned(), 3),
            ("member-revoke-outbox".to_owned(), "ACTIVE".to_owned(), 3),
        ]
    );
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(evidence_count, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn role_assignment_commits_with_evidence_and_is_semantically_idempotent()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-role-assignment-atomicity")?;
    let membership = MembershipId::new("member-role-assignment-atomicity")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let tenancy = service(&db);
    let assignment_id = RoleAssignmentId::new("assignment-atomicity")?;
    let actor = ActorRef {
        subject_ref: "user-role-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };

    let assignment = tenancy
        .assign_role(
            org.clone(),
            membership.clone(),
            assignment_id.clone(),
            Role::Cashier,
            actor.clone(),
        )
        .await?;
    assert_eq!(assignment.state, RoleAssignmentState::Effective);
    let persisted: (String, String, String, i64) = sqlx_core::query_as::query_as(
        "SELECT organization_id, membership_id, role, state_version \
         FROM role_assignments WHERE id = $1",
    )
    .bind(assignment_id.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(
        persisted,
        (
            org.to_string(),
            membership.to_string(),
            "CASHIER".to_owned(),
            3,
        )
    );

    let retry = tenancy
        .assign_role(
            org.clone(),
            membership.clone(),
            assignment_id.clone(),
            Role::Cashier,
            actor.clone(),
        )
        .await?;
    assert_eq!(retry, assignment);
    assert!(
        tenancy
            .assign_role(
                org.clone(),
                membership.clone(),
                assignment_id,
                Role::Viewer,
                actor.clone(),
            )
            .await
            .is_err()
    );
    assert!(
        tenancy
            .assign_role(
                org.clone(),
                membership.clone(),
                RoleAssignmentId::new("assignment-duplicate-role")?,
                Role::Cashier,
                actor,
            )
            .await
            .is_err()
    );

    let counts: (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 \
                 AND event_name = 'iam.role.assigned'), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 \
                 AND event_name = 'iam.role.assigned')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(counts, (1, 1));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
#[expect(
    clippy::disallowed_methods,
    reason = "FW-TOK-03: the two concurrent calls are a bounded test harness and every handle is awaited."
)]
async fn concurrent_role_assignment_retries_emit_one_semantic_event()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-role-assignment-concurrent")?;
    let membership = MembershipId::new("member-role-assignment-concurrent")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let tenancy = Arc::new(service(&db));
    let barrier = Arc::new(Barrier::new(2));
    let assignment_id = RoleAssignmentId::new("assignment-concurrent")?;
    let run = |tenancy: Arc<TenancyService>, barrier: Arc<Barrier>| {
        let org = org.clone();
        let membership = membership.clone();
        let assignment_id = assignment_id.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            tenancy
                .assign_role(
                    org,
                    membership,
                    assignment_id,
                    Role::Cashier,
                    ActorRef {
                        subject_ref: "user-role-operator".to_owned(),
                        membership_ref: None,
                        device_ref: None,
                    },
                )
                .await
        })
    };
    let first = run(Arc::clone(&tenancy), Arc::clone(&barrier));
    let second = run(Arc::clone(&tenancy), barrier);
    let first = first.await??;
    let second = second.await??;
    assert_eq!(first, second);
    let counts: (i64, i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM role_assignments WHERE organization_id = $1), \
                (SELECT count(*) FROM audit_events WHERE organization_id = $1 \
                 AND event_name = 'iam.role.assigned'), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 \
                 AND event_name = 'iam.role.assigned')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(counts, (1, 1, 1));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn role_assignment_rolls_back_when_audit_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-role-assignment-audit-failure")?;
    let membership = MembershipId::new("member-role-assignment-audit-failure")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let tenancy = failing_service(&db, EvidenceFailure::Audit);
    assert!(
        tenancy
            .assign_role(
                org.clone(),
                membership,
                RoleAssignmentId::new("assignment-audit-failure")?,
                Role::Cashier,
                ActorRef {
                    subject_ref: "user-role-operator".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await
            .is_err()
    );
    let role_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM role_assignments WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(role_count, 0);
    let evidence_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1) + \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(evidence_count, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn role_assignment_rolls_back_when_outbox_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-role-assignment-outbox-failure")?;
    let membership = MembershipId::new("member-role-assignment-outbox-failure")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let tenancy = failing_service(&db, EvidenceFailure::Outbox);
    assert!(
        tenancy
            .assign_role(
                org.clone(),
                membership,
                RoleAssignmentId::new("assignment-outbox-failure")?,
                Role::Cashier,
                ActorRef {
                    subject_ref: "user-role-operator".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await
            .is_err()
    );
    let counts: (i64, i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM role_assignments WHERE organization_id = $1), \
                (SELECT count(*) FROM audit_events WHERE organization_id = $1), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(counts, (0, 0, 0));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn role_revocation_preserves_history_and_regrant_is_duplicate_safe()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-role-revocation")?;
    let membership = MembershipId::new("member-role-revocation")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let tenancy = service(&db);
    let assignment_id = RoleAssignmentId::new("assignment-role-revocation")?;
    let actor = ActorRef {
        subject_ref: "user-role-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };
    tenancy
        .assign_role(
            org.clone(),
            membership.clone(),
            assignment_id.clone(),
            Role::Cashier,
            actor.clone(),
        )
        .await?;

    let revoked = tenancy
        .revoke_role(&org, &membership, Role::Cashier, actor.clone())
        .await?;
    assert_eq!(revoked.id, assignment_id);
    assert_eq!(revoked.state, RoleAssignmentState::Revoked);
    assert_eq!(revoked.state_version, 4);
    assert!(
        tenancy
            .revoke_role(&org, &membership, Role::Cashier, actor.clone())
            .await
            .is_err()
    );

    let replay = tenancy
        .assign_role(
            org.clone(),
            membership.clone(),
            revoked.id.clone(),
            Role::Cashier,
            actor.clone(),
        )
        .await?;
    assert_eq!(replay, revoked);

    let regrant = tenancy
        .assign_role(
            org.clone(),
            membership,
            RoleAssignmentId::new("assignment-role-regrant")?,
            Role::Cashier,
            actor,
        )
        .await?;
    assert_eq!(regrant.state, RoleAssignmentState::Effective);
    let counts: (i64, i64, i64, i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM role_assignments WHERE organization_id = $1), \
                (SELECT count(*) FROM role_assignments WHERE organization_id = $1 AND state = 'REVOKED'), \
                (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.role.assigned'), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.role.assigned'), \
                (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.role.removed')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(counts, (2, 1, 2, 2, 1));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn role_revocation_rolls_back_when_audit_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-role-revoke-audit-failure")?;
    let membership = MembershipId::new("member-role-revoke-audit-failure")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let actor = ActorRef {
        subject_ref: "user-role-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };
    service(&db)
        .assign_role(
            org.clone(),
            membership.clone(),
            RoleAssignmentId::new("assignment-revoke-audit-failure")?,
            Role::Cashier,
            actor.clone(),
        )
        .await?;

    assert!(
        failing_service(&db, EvidenceFailure::Audit)
            .revoke_role(&org, &membership, Role::Cashier, actor)
            .await
            .is_err()
    );
    let state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM role_assignments \
         WHERE organization_id = $1 AND membership_id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(state, ("EFFECTIVE".to_owned(), 3));
    let removal_events: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.role.removed') + \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.role.removed')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(removal_events, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn role_revocation_rolls_back_when_outbox_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-role-revoke-outbox-failure")?;
    let membership = MembershipId::new("member-role-revoke-outbox-failure")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let actor = ActorRef {
        subject_ref: "user-role-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };
    service(&db)
        .assign_role(
            org.clone(),
            membership.clone(),
            RoleAssignmentId::new("assignment-revoke-outbox-failure")?,
            Role::Cashier,
            actor.clone(),
        )
        .await?;
    let failing_outbox = failing_service(&db, EvidenceFailure::Outbox);
    assert!(
        failing_outbox
            .revoke_role(&org, &membership, Role::Cashier, actor)
            .await
            .is_err()
    );
    let state: (String, i64) = sqlx_core::query_as::query_as(
        "SELECT state, state_version FROM role_assignments \
         WHERE organization_id = $1 AND membership_id = $2",
    )
    .bind(org.as_str())
    .bind(membership.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(state, ("EFFECTIVE".to_owned(), 3));
    let removal_events: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.role.removed') + \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.role.removed')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(removal_events, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
#[expect(
    clippy::disallowed_methods,
    reason = "FW-TOK-03: the two concurrent calls are a bounded test harness and every handle is awaited."
)]
async fn concurrent_role_revocations_emit_one_semantic_event()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-role-revoke-concurrent")?;
    let membership = MembershipId::new("member-role-revoke-concurrent")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    service(&db)
        .assign_role(
            org.clone(),
            membership.clone(),
            RoleAssignmentId::new("assignment-revoke-concurrent")?,
            Role::Cashier,
            ActorRef {
                subject_ref: "user-role-operator".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;
    let tenancy = Arc::new(service(&db));
    let barrier = Arc::new(Barrier::new(2));
    let run = |tenancy: Arc<TenancyService>, barrier: Arc<Barrier>| {
        let org = org.clone();
        let membership = membership.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            tenancy
                .revoke_role(
                    &org,
                    &membership,
                    Role::Cashier,
                    ActorRef {
                        subject_ref: "user-role-operator".to_owned(),
                        membership_ref: None,
                        device_ref: None,
                    },
                )
                .await
        })
    };
    let first = run(Arc::clone(&tenancy), Arc::clone(&barrier));
    let second = run(tenancy, barrier);
    let first = first.await?;
    let second = second.await?;
    let successful = usize::from(first.is_ok()) + usize::from(second.is_ok());
    assert_eq!(successful, 1);
    assert_eq!(
        first.err().or_else(|| second.err()),
        Some(TenancyError::NotFound)
    );
    let counts: (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.role.removed'), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.role.removed')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(counts, (1, 1));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn scope_grant_and_revocation_commit_with_single_evidence_pair()
-> Result<(), Box<dyn std::error::Error>> {
    use sitolo_authz::ScopeGrantState;

    let db = test_db().await?;
    let org = OrganizationId::new("org-scope-persist")?;
    let membership = MembershipId::new("member-scope-persist")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let tenancy = service(&db);
    let grant_id = ScopeGrantId::new("grant-scope-persist")?;
    let scope = Scope::Organization {
        organization_id: org.clone(),
    };
    let actor = ActorRef {
        subject_ref: "user-scope-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };

    let grant = tenancy
        .grant_scope(
            org.clone(),
            membership.clone(),
            grant_id.clone(),
            scope.clone(),
            actor.clone(),
        )
        .await?;
    assert_eq!(grant.state, ScopeGrantState::Active);
    assert_eq!(grant.state_version, 1);
    assert_eq!(
        tenancy
            .grant_scope(
                org.clone(),
                membership.clone(),
                grant_id.clone(),
                scope,
                actor.clone(),
            )
            .await?,
        grant
    );
    let revoked = tenancy.revoke_scope_grant(&org, &grant_id, actor).await?;
    assert_eq!(revoked.state, ScopeGrantState::Revoked);
    assert_eq!(revoked.state_version, 2);
    assert!(
        tenancy
            .revoke_scope_grant(
                &org,
                &grant_id,
                ActorRef {
                    subject_ref: "user-scope-operator".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                }
            )
            .await
            .is_err()
    );
    let counts: (i64, i64, i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM scope_grants WHERE organization_id = $1), \
                (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.scope.granted'), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.scope.granted'), \
                (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.scope.revoked')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(counts, (1, 1, 1, 1));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn scope_grant_rolls_back_when_audit_or_outbox_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-scope-rollback")?;
    let membership = MembershipId::new("member-scope-rollback")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let scope = Scope::Organization {
        organization_id: org.clone(),
    };
    let actor = || ActorRef {
        subject_ref: "user-scope-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };
    let input = |grant_id: &str| -> Result<_, Box<dyn std::error::Error>> {
        Ok((
            org.clone(),
            membership.clone(),
            ScopeGrantId::new(grant_id)?,
            scope.clone(),
            actor(),
        ))
    };
    let audit_input = input("grant-scope-audit-failure")?;
    let audit_failure = failing_service(&db, EvidenceFailure::Audit)
        .grant_scope(
            audit_input.0,
            audit_input.1,
            audit_input.2,
            audit_input.3,
            audit_input.4,
        )
        .await;
    assert!(audit_failure.is_err());
    let failing_outbox = failing_service(&db, EvidenceFailure::Outbox);
    let outbox_input = input("grant-scope-outbox-failure")?;
    assert!(
        failing_outbox
            .grant_scope(
                outbox_input.0,
                outbox_input.1,
                outbox_input.2,
                outbox_input.3,
                outbox_input.4
            )
            .await
            .is_err()
    );
    let remaining: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM scope_grants WHERE organization_id = $1",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(remaining, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn scope_revocation_rolls_back_when_audit_or_outbox_write_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-scope-revoke-rollback")?;
    let audit_membership = MembershipId::new("member-scope-revoke-audit")?;
    let outbox_membership = MembershipId::new("member-scope-revoke-outbox")?;
    seed_active_membership(&db, org.as_str(), audit_membership.as_str()).await?;
    query(
        "INSERT INTO organization_memberships \
         (id, organization_id, user_id, state, state_version) \
         VALUES ($1, $2, 'user-scope-revoke-outbox', 'ACTIVE', 3)",
    )
    .bind(outbox_membership.as_str())
    .bind(org.as_str())
    .execute(&db.admin)
    .await?;

    let scope = Scope::Organization {
        organization_id: org.clone(),
    };
    let actor = || ActorRef {
        subject_ref: "user-scope-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };
    let audit_grant = ScopeGrantId::new("grant-scope-revoke-audit")?;
    let outbox_grant = ScopeGrantId::new("grant-scope-revoke-outbox")?;
    let tenancy = service(&db);
    tenancy
        .grant_scope(
            org.clone(),
            audit_membership,
            audit_grant.clone(),
            scope.clone(),
            actor(),
        )
        .await?;
    tenancy
        .grant_scope(
            org.clone(),
            outbox_membership,
            outbox_grant.clone(),
            scope,
            actor(),
        )
        .await?;

    let audit_failure = failing_service(&db, EvidenceFailure::Audit)
        .revoke_scope_grant(&org, &audit_grant, actor())
        .await;
    assert!(audit_failure.is_err());
    let outbox_failure = failing_service(&db, EvidenceFailure::Outbox)
        .revoke_scope_grant(&org, &outbox_grant, actor())
        .await;
    assert!(outbox_failure.is_err());

    let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
        "SELECT id, state FROM scope_grants WHERE organization_id = $1 ORDER BY id",
    )
    .bind(org.as_str())
    .fetch_all(&db.admin)
    .await?;
    assert_eq!(
        rows,
        vec![
            (audit_grant.to_string(), "ACTIVE".to_owned()),
            (outbox_grant.to_string(), "ACTIVE".to_owned()),
        ]
    );
    let revoked_evidence: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.scope.revoked') + \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.scope.revoked')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(revoked_evidence, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn concurrent_identical_scope_grants_emit_one_evidence_pair()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-scope-concurrent")?;
    let membership = MembershipId::new("member-scope-concurrent")?;
    seed_active_membership(&db, org.as_str(), membership.as_str()).await?;
    let tenancy = service(&db);
    let scope = Scope::Organization {
        organization_id: org.clone(),
    };
    let grant_id = ScopeGrantId::new("grant-scope-concurrent")?;
    let actor = || ActorRef {
        subject_ref: "user-scope-operator".to_owned(),
        membership_ref: None,
        device_ref: None,
    };
    let (first, second) = tokio::join!(
        tenancy.grant_scope(
            org.clone(),
            membership.clone(),
            grant_id.clone(),
            scope.clone(),
            actor(),
        ),
        tenancy.grant_scope(org.clone(), membership, grant_id, scope, actor(),),
    );
    assert_eq!(first?, second?);
    let counts: (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.scope.granted'), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.scope.granted')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(counts, (1, 1));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn invitation_acceptance_activates_member_and_erases_terminal_pii_atomically()
-> Result<(), Box<dyn std::error::Error>> {
    use sitolo_authz::InvitationState;

    let db = test_db().await?;
    let org = OrganizationId::new("org-accept-persist")?;
    let invitation = InvitationId::new("inv-accept-persist")?;
    let membership = MembershipId::new("member-accept-persist")?;
    let user = sitolo_domain::tenancy::TenantUserId::new("user-accept-persist")?;
    seed_organization(&db, org.as_str()).await?;
    let tenancy = service(&db);
    tenancy
        .create_invitation(
            org.clone(),
            invitation.clone(),
            membership.clone(),
            user,
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "invitee-accept@example.test")?,
            "token-for-acceptance-persist-test",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;

    let accepted = tenancy
        .accept_invitation(
            "token-for-acceptance-persist-test",
            RoleAssignmentId::new("assignment-accept-persist")?,
            None,
            ActorRef {
                subject_ref: "forged-caller-actor".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;
    assert_eq!(accepted.invitation.state, InvitationState::Accepted);
    assert!(accepted.invitation.contact.is_none());
    assert!(accepted.invitation.token_hash.is_none());
    assert_eq!(
        accepted.membership.state,
        sitolo_domain::tenancy::MembershipState::Active
    );
    assert_eq!(accepted.assignment.state, RoleAssignmentState::Effective);

    let stored: (String, Option<String>, bool, String, i64) = sqlx_core::query_as::query_as(
        "SELECT invitation.state, invitation.contact_value, invitation.token_hash IS NOT NULL, membership.state, membership.state_version FROM invitations invitation JOIN organization_memberships membership ON membership.organization_id = invitation.organization_id AND membership.id = invitation.membership_id WHERE invitation.organization_id = $1 AND invitation.id = $2",
    )
    .bind(org.as_str())
    .bind(invitation.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(
        stored,
        ("ACCEPTED".to_owned(), None, false, "ACTIVE".to_owned(), 3)
    );
    let accepted_audit: (i64, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT count(*), min(actor_subject_ref), min(actor_membership_ref) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.invitation_accepted'",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(
        accepted_audit,
        (
            1,
            Some("user-accept-persist".to_owned()),
            Some(membership.to_string())
        )
    );
    let pii_outbox: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND payload::text LIKE ANY(ARRAY['%invitee-accept@example.test%', '%token-for-acceptance-persist-test%'])",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(pii_outbox, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn invitation_acceptance_rolls_back_on_audit_and_outbox_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-accept-rollback")?;
    seed_organization(&db, org.as_str()).await?;
    let create_service = service(&db);
    for (suffix, raw_token) in [
        ("audit", "token-for-audit-failure-case-12345678"),
        ("outbox", "token-for-outbox-failure-case-12345678"),
    ] {
        let contact = format!("{suffix}@example.test");
        create_service
            .create_invitation(
                org.clone(),
                InvitationId::new(format!("inv-accept-{suffix}"))?,
                MembershipId::new(format!("member-accept-{suffix}"))?,
                sitolo_domain::tenancy::TenantUserId::new(format!("user-accept-{suffix}"))?,
                Role::Cashier,
                None,
                InvitationContact::new(ContactKind::Email, &contact)?,
                raw_token,
                std::time::Duration::from_secs(3_600),
                ActorRef {
                    subject_ref: "issuer-user".to_owned(),
                    membership_ref: None,
                    device_ref: None,
                },
            )
            .await?;
    }
    let audit_failure = failing_service(&db, EvidenceFailure::Audit)
        .accept_invitation(
            "token-for-audit-failure-case-12345678",
            RoleAssignmentId::new("assignment-accept-audit-failure")?,
            None,
            ActorRef {
                subject_ref: "caller".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await;
    assert!(audit_failure.is_err());
    let outbox_failure = failing_service(&db, EvidenceFailure::Outbox)
        .accept_invitation(
            "token-for-outbox-failure-case-12345678",
            RoleAssignmentId::new("assignment-accept-outbox-failure")?,
            None,
            ActorRef {
                subject_ref: "caller".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await;
    assert!(outbox_failure.is_err());
    let state: (i64, i64, i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM invitations WHERE organization_id = $1 AND state = 'ISSUED'), (SELECT count(*) FROM organization_memberships WHERE organization_id = $1 AND state = 'INVITED'), (SELECT count(*) FROM role_assignments WHERE organization_id = $1), (SELECT count(*) FROM scope_grants WHERE organization_id = $1)",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(state, (2, 2, 0, 0));
    let accepted_events: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name IN ('iam.membership.invitation_accepted', 'iam.membership.pending_acceptance', 'iam.membership.activated')) + (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name IN ('iam.membership.invitation_accepted', 'iam.membership.pending_acceptance', 'iam.membership.activated'))",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(accepted_events, 0);
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}

#[tokio::test]
async fn concurrent_invitation_acceptance_claims_token_once()
-> Result<(), Box<dyn std::error::Error>> {
    let db = test_db().await?;
    let org = OrganizationId::new("org-accept-concurrent")?;
    let invitation = InvitationId::new("inv-accept-concurrent")?;
    let membership = MembershipId::new("member-accept-concurrent")?;
    seed_organization(&db, org.as_str()).await?;
    let tenancy = Arc::new(service(&db));
    tenancy
        .create_invitation(
            org.clone(),
            invitation,
            membership.clone(),
            sitolo_domain::tenancy::TenantUserId::new("user-accept-concurrent")?,
            Role::Cashier,
            None,
            InvitationContact::new(ContactKind::Email, "accept-race@example.test")?,
            "token-for-concurrent-acceptance-12345678",
            std::time::Duration::from_secs(3_600),
            ActorRef {
                subject_ref: "issuer-user".to_owned(),
                membership_ref: None,
                device_ref: None,
            },
        )
        .await?;
    let actor = || ActorRef {
        subject_ref: "caller".to_owned(),
        membership_ref: None,
        device_ref: None,
    };
    let (first, second) = tokio::join!(
        tenancy.accept_invitation(
            "token-for-concurrent-acceptance-12345678",
            RoleAssignmentId::new("assignment-accept-race-a")?,
            None,
            actor(),
        ),
        tenancy.accept_invitation(
            "token-for-concurrent-acceptance-12345678",
            RoleAssignmentId::new("assignment-accept-race-b")?,
            None,
            actor(),
        ),
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let counts: (i64, i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM organization_memberships WHERE organization_id = $1 AND state = 'ACTIVE'), \
                (SELECT count(*) FROM audit_events WHERE organization_id = $1 AND event_name = 'iam.membership.invitation_accepted'), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = $1 AND event_name = 'iam.membership.invitation_accepted')",
    )
    .bind(org.as_str())
    .fetch_one(&db.admin)
    .await
    ?;
    assert_eq!(counts, (1, 1, 1));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    Ok(())
}
