//! HTTP boundary regressions. The authenticated principal is established by
//! the same live Axum handler used by the listener; protected commands remain
//! unavailable until persistent tenancy authorization is composed.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use sitolo_api_bin::bootstrap::StartupContext;
use sitolo_api_bin::serve::app_router;
use sitolo_api_bin::state::AppState;
use sitolo_auth::{
    Assurance, AuthenticationMethod, ClientPlatform, IdentityProviderPort, SessionClass,
    SessionLifetime, SessionPolicySet, TestIdentityProvider, UserId,
};
use sitolo_persistence::{IdentityDatabase, IdentityStores, RequestIdentityAuthority};
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

async fn base_state() -> AppState {
    StartupContext::build_from_pairs_with_provider(
        Vec::<(String, String)>::new(),
        Some(Arc::new(StubSecretProvider)),
    )
    .await
    .expect("test bootstrap succeeds")
    .state()
    .as_ref()
    .clone()
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

async fn provision_identity(
    database: &IdentityDatabase,
    user: &str,
    link_external_identity: bool,
) -> sitolo_auth::SessionId {
    let user_id = UserId::new(user.to_owned()).expect("test user id is valid");
    database
        .create_user(user_id.clone(), None, None)
        .await
        .expect("test user is created");
    if link_external_identity {
        database
            .bind_external_identity("test-issuer".to_owned(), "subject-1".to_owned(), &user_id)
            .await
            .expect("test external identity is linked");
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
        .expect("test session is established")
        .session
        .id
}

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

async fn post_organization(
    app: Router,
    authorization: Option<&str>,
    session_id: Option<&str>,
    body: &str,
) -> StatusCode {
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
    .status()
}

#[tokio::test]
async fn live_route_rejects_missing_credentials() {
    let app = app_router(Arc::new(base_state().await)).expect("route inventory validates");
    assert_eq!(
        post_organization(app, None, None, "{}").await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
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
async fn live_route_rejects_unknown_external_identity_without_mutating_tenancy() {
    let database = identity_database();
    let session_id = provision_identity(&database, "user-1", false).await;
    let app = router_with_identity(Arc::clone(&database)).await;
    let status = post_organization(
        app,
        Some("Bearer syntactically-valid-token"),
        Some(session_id.as_str()),
        r#"{"organization_id":"org-a","owner_user_id":"spoofed-owner"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn live_route_rejects_session_owned_by_another_mapped_user() {
    let database = identity_database();
    let _mapped_session = provision_identity(&database, "user-1", true).await;
    let other_user_session = provision_identity(&database, "user-2", false).await;
    let app = router_with_identity(Arc::clone(&database)).await;
    let status = post_organization(
        app,
        Some("Bearer syntactically-valid-token"),
        Some(other_user_session.as_str()),
        r#"{"organization_id":"org-a","owner_user_id":"spoofed-owner"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn live_route_authenticates_mapped_identity_but_stays_closed_without_authorizer() {
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
        r#"{"organization_id":"org-a","owner_user_id":"spoofed-owner"}"#,
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
