//! Axum/Tokio HTTP serving boundary.
//!
//! Production ingress is implemented exclusively through Axum's router and
//! Tokio's asynchronous runtime. The API process owns lifecycle and graceful
//! shutdown here; request routing and transport decoding stay inside Axum.
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, Path, State};
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use sitolo_api::tenancy::{
    BranchResponse, CreateBranchRequest, CreateOrganizationRequest, OrganizationResponse,
    handle_activate_branch, handle_activate_organization, handle_begin_close_branch,
    handle_begin_close_organization, handle_close_branch, handle_close_organization,
    handle_create_branch, handle_provision_organization, handle_resume_branch,
    handle_resume_organization, handle_suspend_branch, handle_suspend_organization,
};
use sitolo_api::{AppError, ProblemDetails};
use sitolo_observability::RequestId;
use tokio::sync::oneshot;
use tower::ServiceExt;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::{RequestBodyTimeoutLayer, TimeoutLayer};

use crate::shutdown::{MAX_IN_FLIGHT_CONNECTIONS, SHUTDOWN_DRAIN_DEADLINE_SECS, Subsystem};
use crate::state::AppState;

const REQUEST_TIMEOUT_SECS: u64 = 30;
const REQUEST_BODY_IDLE_TIMEOUT_SECS: u64 = 5;
const COMPAT_RESPONSE_BODY_MAX_BYTES: usize = 64 * 1024;

/// Builds the production HTTP application.
///
/// Every request enters through this Axum router. Body size, body-idle,
/// request wall-clock, and concurrency controls are enforced by Tower/Axum
/// middleware before application handlers run.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/process/live", get(live))
        .route("/process/ready", get(ready))
        .route("/v1/organizations", post(provision_organization))
        .route(
            "/v1/organizations/{organization_id}/branches",
            post(create_branch),
        )
        .route(
            "/v1/organizations/{organization_id}/{action}",
            post(organization_action),
        )
        .route(
            "/v1/organizations/{organization_id}/branches/{branch_id}/{action}",
            post(branch_action),
        )
        .layer(RequestBodyLimitLayer::new(
            sitolo_api::tenancy::bounds::MAX_TENANCY_BODY_BYTES,
        ))
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(
            REQUEST_BODY_IDLE_TIMEOUT_SECS,
        )))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
        ))
        .layer(ConcurrencyLimitLayer::new(MAX_IN_FLIGHT_CONNECTIONS))
        .with_state(state)
}

/// Serves the Axum application until shutdown, then waits for in-flight
/// requests for at most the configured drain deadline.
pub async fn serve(
    listener: tokio::net::TcpListener,
    state: Arc<AppState>,
    shutdown: oneshot::Receiver<()>,
) -> Vec<Subsystem> {
    let app = router(state);
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        let _ = shutdown.await;
    });

    let _ = tokio::time::timeout(Duration::from_secs(SHUTDOWN_DRAIN_DEADLINE_SECS), server).await;

    let mut coordinator = crate::shutdown::ShutdownCoordinator::new();
    coordinator.shutdown().to_vec()
}

async fn live(State(state): State<Arc<AppState>>) -> Response {
    let body = format!(
        "{{\"status\":\"live\",\"service\":\"{}\",\"version\":\"{}\"}}",
        json_escape(state.service_name()),
        json_escape(state.service_version())
    );
    json_body(StatusCode::OK, body)
}

async fn ready() -> Response {
    json_body(StatusCode::OK, "{\"status\":\"ready\"}".to_string())
}

async fn provision_organization(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<CreateOrganizationRequest>, JsonRejection>,
) -> Response {
    let Json(req) = match payload {
        Ok(value) => value,
        Err(_) => return format_error_response(&AppError::Validation),
    };

    match handle_provision_organization(state.tenancy_service(), req).await {
        Ok(response) => json_response(StatusCode::CREATED, response),
        Err(error) => format_error_response(&error),
    }
}

async fn create_branch(
    State(state): State<Arc<AppState>>,
    Path(organization_id): Path<String>,
    payload: Result<Json<CreateBranchRequest>, JsonRejection>,
) -> Response {
    let Json(req) = match payload {
        Ok(value) => value,
        Err(_) => return format_error_response(&AppError::Validation),
    };

    match handle_create_branch(state.tenancy_service(), &organization_id, req).await {
        Ok(response) => json_response(StatusCode::CREATED, response),
        Err(error) => format_error_response(&error),
    }
}

async fn organization_action(
    State(state): State<Arc<AppState>>,
    Path((organization_id, action)): Path<(String, String)>,
) -> Response {
    let result = match action.as_str() {
        "activate" => handle_activate_organization(state.tenancy_service(), &organization_id).await,
        "suspend" => handle_suspend_organization(state.tenancy_service(), &organization_id).await,
        "resume" => handle_resume_organization(state.tenancy_service(), &organization_id).await,
        "begin_close" => {
            handle_begin_close_organization(state.tenancy_service(), &organization_id).await
        }
        "close" => handle_close_organization(state.tenancy_service(), &organization_id).await,
        _ => return format_error_response(&AppError::NotFound),
    };

    match result {
        Ok(response) => json_response(StatusCode::OK, response),
        Err(error) => format_error_response(&error),
    }
}

async fn branch_action(
    State(state): State<Arc<AppState>>,
    Path((organization_id, branch_id, action)): Path<(String, String, String)>,
) -> Response {
    let result = match action.as_str() {
        "activate" => {
            handle_activate_branch(state.tenancy_service(), &organization_id, &branch_id).await
        }
        "suspend" => {
            handle_suspend_branch(state.tenancy_service(), &organization_id, &branch_id).await
        }
        "resume" => {
            handle_resume_branch(state.tenancy_service(), &organization_id, &branch_id).await
        }
        "begin_close" => {
            handle_begin_close_branch(state.tenancy_service(), &organization_id, &branch_id).await
        }
        "close" => handle_close_branch(state.tenancy_service(), &organization_id, &branch_id).await,
        _ => return format_error_response(&AppError::NotFound),
    };

    match result {
        Ok(response) => json_response(StatusCode::OK, response),
        Err(error) => format_error_response(&error),
    }
}
fn json_response<T: serde::Serialize>(status: StatusCode, value: T) -> Response {
    (status, Json(value)).into_response()
}

fn json_body(status: StatusCode, body: String) -> Response {
    let mut response = (status, Body::from(body)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    response
}

fn format_error_response(err: &AppError) -> Response {
    let status =
        StatusCode::from_u16(err.public().status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let problem = ProblemDetails::from_error(err, &RequestId::new_server());
    let mut response = (status, Body::from(problem.json())).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/problem+json"),
    );
    response
}

/// Test-only compatibility adapter. It intentionally exercises the same
/// production Axum router rather than maintaining a second request parser.
pub async fn dispatch_request(
    method: &str,
    path: &str,
    body: &str,
    state: &AppState,
) -> (&'static str, String) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, body.as_bytes().len().to_string())
        .body(Body::from(body.to_owned()))
        .expect("test request construction must succeed");

    let response = router(Arc::new(state.clone()))
        .oneshot(request)
        .await
        .expect("Axum router is infallible");
    let status = response.status();
    let body = to_bytes(response.into_body(), COMPAT_RESPONSE_BODY_MAX_BYTES)
        .await
        .expect("test response body must be bounded and readable");
    let body = String::from_utf8_lossy(&body).into_owned();
    (status_line(status), body)
}

fn status_line(status: StatusCode) -> &'static str {
    match status {
        StatusCode::OK => "200 OK",
        StatusCode::CREATED => "201 Created",
        StatusCode::UNPROCESSABLE_ENTITY => "422 Unprocessable Entity",
        StatusCode::NOT_FOUND => "404 Not Found",
        StatusCode::CONFLICT => "409 Conflict",
        StatusCode::UNAUTHORIZED => "401 Unauthorized",
        StatusCode::FORBIDDEN => "403 Forbidden",
        StatusCode::PAYLOAD_TOO_LARGE => "413 Payload Too Large",
        StatusCode::TOO_MANY_REQUESTS => "429 Too Many Requests",
        StatusCode::REQUEST_TIMEOUT => "408 Request Timeout",
        StatusCode::BAD_GATEWAY => "502 Bad Gateway",
        StatusCode::SERVICE_UNAVAILABLE => "503 Service Unavailable",
        StatusCode::GATEWAY_TIMEOUT => "504 Gateway Timeout",
        _ => "500 Internal Server Error",
    }
}

/// Minimal JSON escaping for operator-controlled identity fields used by the
/// liveness projection. Application responses use serde_json/Axum directly.
fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escape_neutralizes_quotes_and_controls() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
        assert_eq!(json_escape("plain"), "plain");
        assert!(json_escape("\u{0001}").contains("\\u0001"));
    }

    #[test]
    fn production_router_is_axum_composed() {
        let state = AppState::new(
            "sitolo".into(),
            "test".into(),
            "fingerprint".into(),
            sitolo_config::DatabaseTarget {
                host: "localhost".into(),
                port: 5432,
                database: "sitolo".into(),
                username: "sitolo".into(),
            },
            Arc::new(std::sync::Mutex::new(
                sitolo_observability::TelemetryBuffer::new(8),
            )),
        );
        let _router = router(Arc::new(state));
    }
}
