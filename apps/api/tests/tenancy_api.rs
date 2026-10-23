//! HTTP boundary regressions. The authenticated principal is established by
//! the same live Axum handler used by the listener; a PostgreSQL-backed
//! branch-create route exercises the full authenticated authorization path.

use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use sitolo_api_bin::bootstrap::StartupContext;
use sitolo_api_bin::serve::app_router;
use sitolo_api_bin::state::AppState;
use sitolo_application::{TenancyEvidenceSink, TenancyService};
use sitolo_auth::{
    Assurance, AuthenticationMethod, ClientPlatform, IdentityProviderPort, SessionClass,
    SessionLifetime, SessionPolicySet, TestIdentityProvider, UserId,
};
use sitolo_persistence::audit_repository::PostgresAuditWriter;
use sitolo_persistence::outbox_repository::PostgresOutboxRepository;
use sitolo_persistence::{IdentityDatabase, IdentityStores, RequestIdentityAuthority};
use sqlx_core::query::query;
use sqlx_core::raw_sql::raw_sql;
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use tower::ServiceExt;

struct StubSecretProvider;

#[async_trait::async_trait]
impl sitolo_security::SecretProvider for StubSecretProvider {
    async fn get(
        &self,
        _reference: &sitolo_security::SecretRef,
    ) -> Result<sitolo_security::SecretValue, sitolo_security::SecretError> {
        Ok(sitolo_security::SecretValue::new("test-only"))
    }
}

#[allow(clippy::expect_used)] // Test bootstrap is a fixed, validated fixture.
async fn base_state() -> AppState {
    let context = StartupContext::build_from_pairs_with_provider(
        Vec::<(String, String)>::new(),
        Some(Arc::new(StubSecretProvider)),
    )
    .await
    .expect("test bootstrap succeeds");
    context.state().as_ref().clone()
}

fn identity_database() -> Arc<IdentityDatabase> {
    let lifetime = SessionLifetime {
        idle: Duration::from_secs(30 * 60),
        absolute: Duration::from_secs(8 * 60 * 60),
    };
    Arc::new(IdentityDatabase::new(
        SessionPolicySet {
            merchant: lifetime,
            administrator: lifetime,
            break_glass: lifetime,
        },
        Duration::from_secs(300),
    ))
}

#[allow(clippy::expect_used)] // Fixed fixture IDs and in-memory identity setup are asserted test preconditions.
async fn provision_identity(
    database: &IdentityDatabase,
    user: &str,
    link_external_identity: bool,
) -> sitolo_auth::SessionId {
    let user_id = UserId::new(user.to_owned()).expect("test user id is valid");
    database
        .create_user(user_id.clone(), None, None)
        .await
        .expect("test user can be created");
    if link_external_identity {
        database
            .bind_external_identity("test-issuer".to_owned(), "subject-1".to_owned(), &user_id)
            .await
            .expect("test identity can be linked");
    }
    database
        .establish_session(
            user_id,
            None,
            SessionClass::Merchant,
            AuthenticationMethod::OidcAssertion,
            ClientPlatform::Android,
            Assurance::A1,
            SystemTime::now(),
        )
        .await
        .expect("test session can be established")
        .session
        .id
}

#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn router_with_identity(database: Arc<IdentityDatabase>) -> Router {
    let provider: Arc<dyn IdentityProviderPort> =
        Arc::new(TestIdentityProvider::new("subject-1", Assurance::A1));
    let authority: Arc<dyn RequestIdentityAuthority> = database;
    let state = Arc::new(
        base_state()
            .await
            .with_request_identity(provider, authority),
    );
    app_router(state).expect("route inventory validates")
}

#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn router_with_identity_and_tenancy(
    database: Arc<IdentityDatabase>,
    tenancy: Arc<TenancyService>,
) -> Router {
    router_with_identity_and_tenancy_subject(database, tenancy, "subject-1").await
}

#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn router_with_identity_and_tenancy_subject(
    database: Arc<IdentityDatabase>,
    tenancy: Arc<TenancyService>,
    subject_id: &str,
) -> Router {
    let provider: Arc<dyn IdentityProviderPort> =
        Arc::new(TestIdentityProvider::new(subject_id, Assurance::A1));
    let authority: Arc<dyn RequestIdentityAuthority> = database;
    let state = Arc::new(
        base_state()
            .await
            .with_request_identity(provider, authority)
            .with_tenancy_service(tenancy),
    );
    app_router(state).expect("route inventory validates")
}

async fn post_organization(
    app: Router,
    authorization: Option<&str>,
    session_id: Option<&str>,
    body: &str,
) -> StatusCode {
    post_organization_response(app, authorization, session_id, body)
        .await
        .status()
}

#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn post_organization_response(
    app: Router,
    authorization: Option<&str>,
    session_id: Option<&str>,
    body: &str,
) -> Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/v1/organizations")
        .header("content-type", "application/json");
    if let Some(value) = authorization {
        builder = builder.header("authorization", value);
    }
    if let Some(value) = session_id {
        builder = builder.header("session-id", value);
    }
    app.oneshot(
        builder
            .body(Body::from(body.to_owned()))
            .expect("test HTTP request is valid"),
    )
    .await
    .expect("Axum returns an HTTP response")
}

#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn post_branch(
    app: Router,
    organization_id: &str,
    authorization: Option<&str>,
    session_id: Option<&str>,
    body: &str,
) -> Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/v1/organizations/{organization_id}/branches"))
        .header("content-type", "application/json");
    if let Some(value) = authorization {
        builder = builder.header("authorization", value);
    }
    if let Some(value) = session_id {
        builder = builder.header("session-id", value);
    }
    app.oneshot(
        builder
            .body(Body::from(body.to_owned()))
            .expect("test HTTP request is valid"),
    )
    .await
    .expect("Axum returns an HTTP response")
}

#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn post_branch_activation(
    app: Router,
    organization_id: &str,
    branch_id: &str,
    authorization: Option<&str>,
    session_id: Option<&str>,
) -> Response {
    let mut builder = Request::builder().method("POST").uri(format!(
        "/v1/organizations/{organization_id}/branches/{branch_id}/activate"
    ));
    if let Some(value) = authorization {
        builder = builder.header("authorization", value);
    }
    if let Some(value) = session_id {
        builder = builder.header("session-id", value);
    }
    app.oneshot(builder.body(Body::empty()).expect("request is valid"))
        .await
        .expect("Axum returns an HTTP response")
}

#[allow(clippy::expect_used)] // Fixed HTTP fixture identifiers make the request deterministic.
async fn post_member_revoke(
    app: Router,
    organization_id: &str,
    membership_id: &str,
    authorization: Option<&str>,
    session_id: Option<&str>,
) -> Response {
    let mut builder = Request::builder().method("POST").uri(format!(
        "/v1/organizations/{organization_id}/members/{membership_id}/revoke"
    ));
    if let Some(value) = authorization {
        builder = builder.header("authorization", value);
    }
    if let Some(value) = session_id {
        builder = builder.header("session-id", value);
    }
    app.oneshot(builder.body(Body::empty()).expect("request is valid"))
        .await
        .expect("Axum returns an HTTP response")
}

#[allow(clippy::expect_used)] // Fixed HTTP fixture identifiers make the request deterministic.
async fn post_member_suspend(
    app: Router,
    organization_id: &str,
    membership_id: &str,
    authorization: Option<&str>,
    session_id: Option<&str>,
) -> Response {
    let mut builder = Request::builder().method("POST").uri(format!(
        "/v1/organizations/{organization_id}/members/{membership_id}/suspend"
    ));
    if let Some(value) = authorization {
        builder = builder.header("authorization", value);
    }
    if let Some(value) = session_id {
        builder = builder.header("session-id", value);
    }
    app.oneshot(builder.body(Body::empty()).expect("request is valid"))
        .await
        .expect("Axum returns an HTTP response")
}

#[tokio::test]
#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn live_route_rejects_missing_credentials() {
    let app = app_router(Arc::new(base_state().await)).expect("route inventory validates");
    assert_eq!(
        post_organization(app, None, None, "{}").await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn protected_branch_route_authenticates_before_json_extraction() {
    let app = app_router(Arc::new(base_state().await)).expect("route inventory validates");
    let response = post_branch(app, "org-a", None, None, "{").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn live_route_requires_a_well_formed_session_id_header() {
    let database = identity_database();
    let session_id = provision_identity(&database, "user-1", true).await;
    let before = database
        .session_snapshot(&session_id)
        .await
        .expect("session exists");
    let app = router_with_identity(Arc::clone(&database)).await;

    assert_eq!(
        post_organization(
            app.clone(),
            Some("Bearer syntactically-valid-token"),
            None,
            "{}",
        )
        .await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post_organization(
            app,
            Some("Bearer syntactically-valid-token"),
            Some("bad session selector"),
            "{}",
        )
        .await,
        StatusCode::UNAUTHORIZED
    );
    let after = database
        .session_snapshot(&session_id)
        .await
        .expect("session remains present");
    assert_eq!(
        after.session.last_seen_at, before.session.last_seen_at,
        "denied requests without a usable selector do not touch a session"
    );
}

#[tokio::test]
#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn live_route_rejects_unknown_external_identity_without_mutating_tenancy() {
    let database = identity_database();
    let session_id = provision_identity(&database, "user-1", false).await;
    let app = router_with_identity(Arc::clone(&database)).await;
    let status = post_organization(
        app,
        Some("Bearer syntactically-valid-token"),
        Some(session_id.as_str()),
        r#"{"organization_id":"org-a","organization_name":"Org A","owner_membership_id":"member-a","default_branch_id":"branch-a","default_branch_name":"Main"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn live_route_rejects_session_owned_by_another_mapped_user() {
    let database = identity_database();
    let _mapped_session = provision_identity(&database, "user-1", true).await;
    let other_user_session = provision_identity(&database, "user-2", false).await;
    let app = router_with_identity(Arc::clone(&database)).await;
    let status = post_organization(
        app,
        Some("Bearer syntactically-valid-token"),
        Some(other_user_session.as_str()),
        r#"{"organization_id":"org-a","organization_name":"Org A","owner_membership_id":"member-a","default_branch_id":"branch-a","default_branch_name":"Main"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn live_organization_route_fails_closed_without_provision_command() {
    let database = identity_database();
    let session_id = provision_identity(&database, "user-1", true).await;
    let before = database
        .session_snapshot(&session_id)
        .await
        .expect("session exists");
    let app = router_with_identity(Arc::clone(&database)).await;
    let status = post_organization(
        app,
        Some("Bearer syntactically-valid-token"),
        Some(session_id.as_str()),
        r#"{"organization_id":"org-a","organization_name":"Org A","owner_membership_id":"member-a","default_branch_id":"branch-a","default_branch_name":"Main"}"#,
    )
    .await;

    // Authentication succeeds, but this build has no authoritative tenant
    // permission/command service, so no business mutation is allowed.
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let after = database
        .session_snapshot(&session_id)
        .await
        .expect("session remains present");
    assert!(after.session.last_seen_at >= before.session.last_seen_at);
}

#[tokio::test]
#[allow(clippy::expect_used)] // Fixed HTTP and identity fixtures assert harness invariants.
async fn health_probes_are_public_through_the_router() {
    let app = app_router(Arc::new(base_state().await)).expect("route inventory validates");
    for path in ["/process/live", "/process/ready"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(path)
                    .body(Body::empty())
                    .expect("test request is valid"),
            )
            .await
            .expect("Axum returns a response");
        let expected = if path == "/process/live" {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };
        assert_eq!(response.status(), expected);
    }
}

struct ApiTenancyDb {
    admin: PgPool,
    runtime: PgPool,
    schema: String,
}

async fn api_tenancy_db() -> Result<ApiTenancyDb, Box<dyn std::error::Error>> {
    let admin_url = env::var("ADMIN_DATABASE_URL")?;
    let runtime_url = env::var("RUNTIME_DATABASE_URL")?;
    let schema = format!("test_api_tenancy_{}", uuid::Uuid::new_v4().simple());
    let bootstrap = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await?;
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "CREATE SCHEMA \"{schema}\""
    )))
    .execute(&bootstrap)
    .await?;
    bootstrap.close().await;

    let admin_options =
        PgConnectOptions::from_str(&admin_url)?.options([("search_path", schema.as_str())]);
    let admin = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(admin_options)
        .await?;
    let audit_schema = include_str!(
        "../../../crates/sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql"
    )
    .replace("__SCHEMA__", &format!("\"{schema}\""));
    raw_sql(sqlx_core::sql_str::AssertSqlSafe(audit_schema.as_str()))
        .execute(&admin)
        .await?;
    raw_sql(include_str!(
        "../../../crates/sitolo-persistence/schema/phase4_part8_tenancy_provisioning.sql"
    ))
    .execute(&admin)
    .await?;

    let runtime_options =
        PgConnectOptions::from_str(&runtime_url)?.options([("search_path", schema.as_str())]);
    let runtime = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(runtime_options)
        .await?;
    let role: String = sqlx_core::query_scalar::query_scalar("SELECT session_user")
        .fetch_one(&runtime)
        .await?;
    if role != "app_runtime" {
        return Err(format!("expected app_runtime, got {role}").into());
    }
    Ok(ApiTenancyDb {
        admin,
        runtime,
        schema,
    })
}

#[tokio::test]
async fn live_organization_route_derives_owner_and_audit_actor_from_authenticated_subject()
-> Result<(), Box<dyn std::error::Error>> {
    let db = api_tenancy_db().await?;
    let tenancy = Arc::new(TenancyService::new_postgres(
        Vec::new(),
        TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(PostgresOutboxRepository::new(db.runtime.clone())),
        ),
    ));
    let identities = identity_database();
    let session_id = provision_identity(&identities, "user-1", true).await;
    let app = router_with_identity_and_tenancy(Arc::clone(&identities), tenancy).await;
    let authorization = Some("Bearer syntactically-valid-token");
    let selector = Some(session_id.as_str());

    let spoofed = post_organization_response(
        app.clone(),
        authorization,
        selector,
        r#"{"organization_id":"org-http-spoof","organization_name":"Spoof","owner_membership_id":"member-http-spoof","owner_user_id":"user-admin","actor_id":"user-admin","default_branch_id":"branch-http-spoof","default_branch_name":"Main"}"#,
    )
    .await;
    assert_eq!(spoofed.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let denied_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM organizations WHERE id = 'org-http-spoof'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(denied_count, 0);

    let created = post_organization_response(
        app,
        authorization,
        selector,
        r#"{"organization_id":"org-http","organization_name":"HTTP Org","owner_membership_id":"member-http","default_branch_id":"branch-http","default_branch_name":"Main"}"#,
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);

    let owner_user: String = sqlx_core::query_scalar::query_scalar(
        "SELECT user_id FROM organization_memberships WHERE organization_id = 'org-http' AND id = 'member-http'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(owner_user, "user-1");
    let actor: (String, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT actor_subject_ref, actor_membership_ref FROM audit_events WHERE organization_id = 'org-http' AND event_name = 'iam.organization.created'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(actor, ("user-1".to_owned(), Some("member-http".to_owned())));

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    db.runtime.close().await;
    db.admin.close().await;
    Ok(())
}

#[tokio::test]
async fn live_branch_route_authorizes_and_mutates_only_the_verified_tenant()
-> Result<(), Box<dyn std::error::Error>> {
    let db = api_tenancy_db().await?;
    query("INSERT INTO organizations (id, name, state, state_version) VALUES ('org-a', 'Org A', 'ACTIVE', 1), ('org-b', 'Org B', 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ('member-a', 'org-a', 'user-1', 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ('role-a', 'org-a', 'member-a', 'OWNER', 'EFFECTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO scope_grants (id, organization_id, membership_id, scope_kind, branch_id, state, state_version) VALUES ('grant-a', 'org-a', 'member-a', 'ORGANIZATION', NULL, 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;

    let tenancy = Arc::new(TenancyService::new_postgres(
        Vec::new(),
        TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(PostgresOutboxRepository::new(db.runtime.clone())),
        ),
    ));
    let identities = identity_database();
    let session_id = provision_identity(&identities, "user-1", true).await;
    let app = router_with_identity_and_tenancy(Arc::clone(&identities), tenancy).await;
    let authorization = Some("Bearer syntactically-valid-token");
    let selector = Some(session_id.as_str());

    let allowed = post_branch(
        app.clone(),
        "org-a",
        authorization,
        selector,
        r#"{"branch_id":"branch-a1","name":"Branch A1"}"#,
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::OK);
    let branch_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM branches WHERE organization_id = 'org-a' AND id = 'branch-a1'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(branch_count, 1);
    let evidence: (i64, i64, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*) FROM audit_events WHERE organization_id = 'org-a' AND event_name = 'iam.branch.created'), \
                (SELECT count(*) FROM outbox_events WHERE organization_id = 'org-a' AND event_name = 'iam.branch.created'), \
                (SELECT actor_subject_ref FROM audit_events WHERE organization_id = 'org-a' AND event_name = 'iam.branch.created'), \
                (SELECT actor_membership_ref FROM audit_events WHERE organization_id = 'org-a' AND event_name = 'iam.branch.created')",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(
        evidence,
        (1, 1, Some("user-1".to_owned()), Some("member-a".to_owned()))
    );

    let cross_tenant = post_branch(
        app.clone(),
        "org-b",
        authorization,
        selector,
        r#"{"branch_id":"branch-b1","name":"Forged Tenant Branch"}"#,
    )
    .await;
    assert_eq!(cross_tenant.status(), StatusCode::FORBIDDEN);
    let cross_tenant_count: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM branches WHERE organization_id = 'org-b'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(cross_tenant_count, 0);

    let spoofed_authority = post_branch(
        app.clone(),
        "org-a",
        authorization,
        selector,
        r#"{"branch_id":"branch-spoof","name":"Spoof","owner_user_id":"user-admin","actor_id":"user-admin"}"#,
    )
    .await;
    assert_eq!(spoofed_authority.status(), StatusCode::UNPROCESSABLE_ENTITY);

    query("UPDATE organization_memberships SET state = 'SUSPENDED', state_version = state_version + 1 WHERE organization_id = 'org-a' AND id = 'member-a'")
        .execute(&db.admin)
        .await?;
    let suspended = post_branch(
        app.clone(),
        "org-a",
        authorization,
        selector,
        r#"{"branch_id":"branch-suspended","name":"Suspended"}"#,
    )
    .await;
    assert_eq!(suspended.status(), StatusCode::FORBIDDEN);
    query("UPDATE organization_memberships SET state = 'REVOKED', state_version = state_version + 1 WHERE organization_id = 'org-a' AND id = 'member-a'")
        .execute(&db.admin)
        .await?;
    let revoked = post_branch(
        app,
        "org-a",
        authorization,
        selector,
        r#"{"branch_id":"branch-revoked","name":"Revoked"}"#,
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::FORBIDDEN);
    let denied_mutations: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM branches WHERE id IN ('branch-b1', 'branch-spoof', 'branch-suspended', 'branch-revoked')",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(denied_mutations, 0);
    let denied_evidence: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT (SELECT count(*) FROM audit_events WHERE event_name = 'iam.branch.created') + (SELECT count(*) FROM outbox_events WHERE event_name = 'iam.branch.created')",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(denied_evidence, 2);

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    db.runtime.close().await;
    db.admin.close().await;
    Ok(())
}

#[tokio::test]
async fn live_branch_activation_authenticates_and_commits_authorized_transition()
-> Result<(), Box<dyn std::error::Error>> {
    let db = api_tenancy_db().await?;
    query("INSERT INTO organizations (id, name, state, state_version) VALUES ('org-activate', 'Org', 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ('member-activate', 'org-activate', 'user-1', 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ('member-cashier', 'org-activate', 'user-2', 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ('role-activate', 'org-activate', 'member-activate', 'OWNER', 'EFFECTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ('role-cashier', 'org-activate', 'member-cashier', 'CASHIER', 'EFFECTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO scope_grants (id, organization_id, membership_id, scope_kind, branch_id, state, state_version) VALUES ('grant-activate', 'org-activate', 'member-activate', 'ORGANIZATION', NULL, 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ('branch-pending', 'org-activate', 'Pending', 'PROVISIONING', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ('branch-denied', 'org-activate', 'Denied', 'PROVISIONING', 1)")
        .execute(&db.admin)
        .await?;
    let tenancy = Arc::new(TenancyService::new_postgres(
        Vec::new(),
        TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(PostgresOutboxRepository::new(db.runtime.clone())),
        ),
    ));
    let identities = identity_database();
    let session_id = provision_identity(&identities, "user-1", true).await;
    let denied_session = provision_identity(&identities, "user-2", false).await;
    assert_ne!(
        session_id, denied_session,
        "test identities must receive distinct session IDs"
    );
    identities
        .bind_external_identity(
            "test-issuer".to_owned(),
            "subject-2".to_owned(),
            &UserId::new("user-2")?,
        )
        .await?;
    let app = router_with_identity_and_tenancy(Arc::clone(&identities), Arc::clone(&tenancy)).await;

    let anonymous =
        post_branch_activation(app.clone(), "org-activate", "branch-pending", None, None).await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    let activated = post_branch_activation(
        app,
        "org-activate",
        "branch-pending",
        Some("Bearer syntactically-valid-token"),
        Some(session_id.as_str()),
    )
    .await;
    assert_eq!(activated.status(), StatusCode::OK);
    let outcome: (String, i64, i64, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT b.state, \
         (SELECT count(*) FROM audit_events WHERE organization_id = 'org-activate' AND event_name = 'iam.branch.activated'), \
         (SELECT count(*) FROM outbox_events WHERE organization_id = 'org-activate' AND event_name = 'iam.branch.activated'), \
         (SELECT actor_subject_ref FROM audit_events WHERE organization_id = 'org-activate' AND event_name = 'iam.branch.activated') \
         FROM branches b WHERE b.organization_id = 'org-activate' AND b.id = 'branch-pending'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(
        outcome,
        ("ACTIVE".to_owned(), 1, 1, Some("user-1".to_owned()))
    );

    let denied_app =
        router_with_identity_and_tenancy_subject(identities, tenancy, "subject-2").await;
    let denied = post_branch_activation(
        denied_app,
        "org-activate",
        "branch-denied",
        Some("Bearer syntactically-valid-token"),
        Some(denied_session.as_str()),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let (denied_state, denied_evidence): (String, i64) = sqlx_core::query_as::query_as(
        "SELECT b.state, (SELECT count(*) FROM audit_events WHERE organization_id = 'org-activate' AND target_ref = 'branch-denied') + (SELECT count(*) FROM outbox_events WHERE organization_id = 'org-activate' AND aggregate_id = 'branch-denied') FROM branches b WHERE b.organization_id = 'org-activate' AND b.id = 'branch-denied'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(denied_state, "PROVISIONING");
    assert_eq!(denied_evidence, 0);

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    db.runtime.close().await;
    db.admin.close().await;
    Ok(())
}

#[tokio::test]
async fn live_membership_revoke_authorizes_tenant_and_records_verified_actor()
-> Result<(), Box<dyn std::error::Error>> {
    let db = api_tenancy_db().await?;
    query("INSERT INTO organizations (id, name, state, state_version) VALUES ('org-revoke', 'Org', 'ACTIVE', 1), ('org-revoke-other', 'Other', 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ('member-owner', 'org-revoke', 'user-1', 'ACTIVE', 1), ('member-cashier', 'org-revoke', 'user-2', 'ACTIVE', 1), ('member-target', 'org-revoke', 'user-3', 'ACTIVE', 1), ('member-denied', 'org-revoke', 'user-4', 'ACTIVE', 1), ('member-suspend-target', 'org-revoke', 'user-6', 'ACTIVE', 1), ('member-other-tenant', 'org-revoke-other', 'user-5', 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO role_assignments (id, organization_id, membership_id, role, state, state_version) VALUES ('role-owner', 'org-revoke', 'member-owner', 'OWNER', 'EFFECTIVE', 1), ('role-cashier', 'org-revoke', 'member-cashier', 'CASHIER', 'EFFECTIVE', 1)")
        .execute(&db.admin)
        .await?;
    query("INSERT INTO scope_grants (id, organization_id, membership_id, scope_kind, branch_id, state, state_version) VALUES ('grant-owner', 'org-revoke', 'member-owner', 'ORGANIZATION', NULL, 'ACTIVE', 1), ('grant-cashier', 'org-revoke', 'member-cashier', 'ORGANIZATION', NULL, 'ACTIVE', 1)")
        .execute(&db.admin)
        .await?;

    let tenancy = Arc::new(TenancyService::new_postgres(
        Vec::new(),
        TenancyEvidenceSink::new(
            db.runtime.clone(),
            Arc::new(PostgresAuditWriter::new()),
            Arc::new(PostgresOutboxRepository::new(db.runtime.clone())),
        ),
    ));
    let identities = identity_database();
    let owner_session = provision_identity(&identities, "user-1", true).await;
    let cashier_session = provision_identity(&identities, "user-2", false).await;
    assert_ne!(
        owner_session, cashier_session,
        "test identities must receive distinct session IDs"
    );
    identities
        .bind_external_identity(
            "test-issuer".to_owned(),
            "subject-2".to_owned(),
            &UserId::new("user-2")?,
        )
        .await?;
    let app = router_with_identity_and_tenancy(Arc::clone(&identities), Arc::clone(&tenancy)).await;

    let anonymous =
        post_member_revoke(app.clone(), "org-revoke", "member-target", None, None).await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    let revoked = post_member_revoke(
        app.clone(),
        "org-revoke",
        "member-target",
        Some("Bearer syntactically-valid-token"),
        Some(owner_session.as_str()),
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
    let outcome: (String, i64, i64, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT m.state, \
         (SELECT count(*) FROM audit_events WHERE organization_id = 'org-revoke' AND event_name = 'iam.membership.revoked' AND target_ref = 'member-target'), \
         (SELECT count(*) FROM outbox_events WHERE organization_id = 'org-revoke' AND event_name = 'iam.membership.revoked' AND aggregate_id = 'member-target'), \
         (SELECT actor_subject_ref FROM audit_events WHERE organization_id = 'org-revoke' AND event_name = 'iam.membership.revoked' AND target_ref = 'member-target') \
         FROM organization_memberships m WHERE m.organization_id = 'org-revoke' AND m.id = 'member-target'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(
        outcome,
        ("REVOKED".to_owned(), 1, 1, Some("user-1".to_owned()))
    );

    let suspended = post_member_suspend(
        app.clone(),
        "org-revoke",
        "member-suspend-target",
        Some("Bearer syntactically-valid-token"),
        Some(owner_session.as_str()),
    )
    .await;
    assert_eq!(suspended.status(), StatusCode::NO_CONTENT);
    let suspend_evidence: (String, i64, i64, Option<String>) =
        sqlx_core::query_as::query_as(
            "SELECT m.state, \
             (SELECT count(*) FROM audit_events WHERE organization_id = 'org-revoke' AND event_name = 'iam.membership.suspended' AND target_ref = 'member-suspend-target'), \
             (SELECT count(*) FROM outbox_events WHERE organization_id = 'org-revoke' AND event_name = 'iam.membership.suspended' AND aggregate_id = 'member-suspend-target'), \
             (SELECT actor_subject_ref FROM audit_events WHERE organization_id = 'org-revoke' AND event_name = 'iam.membership.suspended' AND target_ref = 'member-suspend-target') \
             FROM organization_memberships m WHERE m.organization_id = 'org-revoke' AND m.id = 'member-suspend-target'",
        )
        .fetch_one(&db.admin)
        .await?;
    assert_eq!(
        suspend_evidence,
        ("SUSPENDED".to_owned(), 1, 1, Some("user-1".to_owned()))
    );

    let denied_app =
        router_with_identity_and_tenancy_subject(identities, tenancy, "subject-2").await;
    let denied = post_member_revoke(
        denied_app.clone(),
        "org-revoke",
        "member-denied",
        Some("Bearer syntactically-valid-token"),
        Some(cashier_session.as_str()),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let cross_tenant = post_member_revoke(
        denied_app,
        "org-revoke-other",
        "member-other-tenant",
        Some("Bearer syntactically-valid-token"),
        Some(cashier_session.as_str()),
    )
    .await;
    assert_eq!(cross_tenant.status(), StatusCode::FORBIDDEN);
    let (denied_state, denied_evidence): (String, i64) = sqlx_core::query_as::query_as(
        "SELECT m.state, (SELECT count(*) FROM audit_events WHERE organization_id = 'org-revoke' AND target_ref = 'member-denied') + (SELECT count(*) FROM outbox_events WHERE organization_id = 'org-revoke' AND aggregate_id = 'member-denied') FROM organization_memberships m WHERE m.organization_id = 'org-revoke' AND m.id = 'member-denied'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(denied_state, "ACTIVE");
    assert_eq!(denied_evidence, 0);
    let other_tenant_state: String = sqlx_core::query_scalar::query_scalar(
        "SELECT state FROM organization_memberships WHERE organization_id = 'org-revoke-other' AND id = 'member-other-tenant'",
    )
    .fetch_one(&db.admin)
    .await?;
    assert_eq!(other_tenant_state, "ACTIVE");

    raw_sql(sqlx_core::sql_str::AssertSqlSafe(format!(
        "DROP SCHEMA \"{}\" CASCADE",
        db.schema
    )))
    .execute(&db.admin)
    .await?;
    db.runtime.close().await;
    db.admin.close().await;
    Ok(())
}
