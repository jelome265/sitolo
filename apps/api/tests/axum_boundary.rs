//! Axum boundary regressions for the composed router.
//!
//! Exercises the real [`app_router`] with an injected secret provider so route
//! inventory, body bounds, and public/protected access are checked through the
//! same router the listener serves. No socket is bound here.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use sitolo_api_bin::bootstrap::StartupContext;
use sitolo_api_bin::serve::{app_router, declared_routes};
use sitolo_api_bin::state::AppState;
use std::sync::Arc;

struct StubProvider;

#[async_trait::async_trait]
impl sitolo_security::SecretProvider for StubProvider {
    async fn get(
        &self,
        _reference: &sitolo_security::SecretRef,
    ) -> Result<sitolo_security::SecretValue, sitolo_security::SecretError> {
        Ok(sitolo_security::SecretValue::new("test-only"))
    }
}

async fn test_app_state() -> Result<Arc<AppState>, Box<dyn std::error::Error>> {
    let context = StartupContext::build_from_pairs_with_provider(
        Vec::<(String, String)>::new(),
        Some(Arc::new(StubProvider)),
    )
    .await
    .map_err(std::io::Error::other)?;
    Ok(Arc::clone(context.state()))
}

#[tokio::test]
async fn axum_protected_route_rejects_missing_credentials() -> Result<(), Box<dyn std::error::Error>>
{
    let app = app_router(test_app_state().await?).map_err(std::io::Error::other)?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/organizations")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .map_err(std::io::Error::other)?,
        )
        .await
        .map_err(std::io::Error::other)?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

/// Substitutes concrete identifiers for every path parameter so the test can
/// send a real request to each declared route.
fn concrete_uri(path: &str) -> String {
    path.replace("{organization_id}", "org-route-coverage")
        .replace("{branch_id}", "branch-route-coverage")
        .replace("{membership_id}", "membership-route-coverage")
}

/// R2 deny-by-default proof: every protected route declared in the reviewed
/// inventory rejects a request with no credentials. This is enumerated from the
/// inventory, so adding a route without authentication coverage fails here.
#[tokio::test]
async fn every_declared_protected_route_rejects_missing_credentials()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app_router(test_app_state().await?).map_err(std::io::Error::other)?;
    let mut protected = 0usize;
    for (method, path, permission) in declared_routes() {
        if permission.is_none() {
            continue;
        }
        protected = protected.saturating_add(1);
        let uri = concrete_uri(path);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(&uri)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .map_err(std::io::Error::other)?,
            )
            .await
            .map_err(std::io::Error::other)?;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must reject missing credentials"
        );
    }
    assert_eq!(
        protected, 5,
        "protected route count drifted from the reviewed inventory"
    );
    Ok(())
}

/// Organization activation is not an HTTP operation: provisioning commits a
/// complete active tenant atomically, while incomplete provisioning is not a
/// visible tenant. The stale route must therefore remain absent.
#[tokio::test]
async fn organization_activation_route_is_not_exposed() -> Result<(), Box<dyn std::error::Error>> {
    let app = app_router(test_app_state().await?).map_err(std::io::Error::other)?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/organizations/org-route-coverage/activate")
                .body(Body::empty())
                .map_err(std::io::Error::other)?,
        )
        .await
        .map_err(std::io::Error::other)?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(
        !declared_routes()
            .iter()
            .any(|(_, path, _)| { *path == "/v1/organizations/{organization_id}/activate" })
    );
    Ok(())
}
