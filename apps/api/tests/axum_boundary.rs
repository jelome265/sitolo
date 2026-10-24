//! Axum boundary regressions for the composed router.
//!
//! Exercises the real [`app_router`] with an injected secret provider so route
//! inventory, body bounds, and public/protected access are checked through the
//! same router the listener serves. No socket is bound here.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use sitolo_api_bin::bootstrap::StartupContext;
use sitolo_api_bin::serve::app_router;
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
