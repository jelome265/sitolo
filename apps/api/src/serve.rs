//! Axum HTTP boundary with a reviewed route inventory and bounded ingress.

use std::convert::Infallible;
use std::error::Error as StdError;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::error_handling::HandleErrorLayer;
use axum::extract::{DefaultBodyLimit, Extension, Path, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tower::ServiceBuilder;
use tower::limit::ConcurrencyLimitLayer;
use tower::load_shed::LoadShedLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::shutdown::{
    MAX_IN_FLIGHT_CONNECTIONS, SHUTDOWN_DRAIN_DEADLINE_SECS, ShutdownCoordinator, Subsystem,
};
use crate::state::AppState;
use sitolo_api::{AppError, ProblemDetails};
use sitolo_api::{
    BranchResponse, CreateBranchRequest, map_authorized_tenancy_error, validate_create_branch_input,
};
use sitolo_auth::SecurityContext;
use sitolo_domain::tenancy::{BranchId, OrganizationId};
use sitolo_observability::RequestId;

const MAX_INGRESS_BODY_BYTES: usize = 32 * 1024;
const MAX_PROBE_BODY_BYTES: usize = 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const PROBE_TIMEOUT: Duration = Duration::from_secs(1);

/// Whether a route is public or requires a permission decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    Public { reason: &'static str },
    Permission(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Method {
    Get,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RouteSpec {
    method: Method,
    path: &'static str,
    access: Access,
    body_limit: usize,
    timeout: Duration,
}

const ROUTES: &[RouteSpec] = &[
    RouteSpec {
        method: Method::Get,
        path: "/process/live",
        access: Access::Public {
            reason: "container liveness probe",
        },
        body_limit: MAX_PROBE_BODY_BYTES,
        timeout: PROBE_TIMEOUT,
    },
    RouteSpec {
        method: Method::Get,
        path: "/process/ready",
        access: Access::Public {
            reason: "container readiness probe",
        },
        body_limit: MAX_PROBE_BODY_BYTES,
        timeout: PROBE_TIMEOUT,
    },
    RouteSpec {
        method: Method::Post,
        path: "/v1/organizations",
        access: Access::Permission("organization.create"),
        body_limit: MAX_INGRESS_BODY_BYTES,
        timeout: REQUEST_TIMEOUT,
    },
    RouteSpec {
        method: Method::Post,
        path: "/v1/organizations/{organization_id}/branches",
        access: Access::Permission("branch.create"),
        body_limit: MAX_INGRESS_BODY_BYTES,
        timeout: REQUEST_TIMEOUT,
    },
    RouteSpec {
        method: Method::Post,
        path: "/v1/organizations/{organization_id}/activate",
        access: Access::Permission("organization.lifecycle"),
        body_limit: MAX_PROBE_BODY_BYTES,
        timeout: REQUEST_TIMEOUT,
    },
    RouteSpec {
        method: Method::Post,
        path: "/v1/organizations/{organization_id}/branches/{branch_id}/activate",
        access: Access::Permission("branch.lifecycle"),
        body_limit: MAX_PROBE_BODY_BYTES,
        timeout: REQUEST_TIMEOUT,
    },
];

/// Reviewed route snapshot. Update only with an intentional route change and its tests.
pub const ROUTE_INVENTORY_SNAPSHOT: &str = "GET /process/live access=public(container liveness probe) body<=1024 timeout=1000ms\nGET /process/ready access=public(container readiness probe) body<=1024 timeout=1000ms\nPOST /v1/organizations access=perm(organization.create) body<=32768 timeout=10000ms\nPOST /v1/organizations/{organization_id}/activate access=perm(organization.lifecycle) body<=1024 timeout=10000ms\nPOST /v1/organizations/{organization_id}/branches access=perm(branch.create) body<=32768 timeout=10000ms\nPOST /v1/organizations/{organization_id}/branches/{branch_id}/activate access=perm(branch.lifecycle) body<=1024 timeout=10000ms";

/// Errors returned while validating the static route inventory or serving HTTP.
#[derive(Debug, Error)]
pub enum ServeError {
    /// The route list is invalid or differs from its reviewed snapshot.
    #[error("HTTP route inventory is invalid")]
    InvalidRouteInventory,
    /// The HTTP listener terminated unexpectedly.
    #[error("HTTP listener failed")]
    Listener(#[source] std::io::Error),
    /// Graceful shutdown exceeded its configured drain deadline.
    #[error("HTTP listener drain deadline exceeded")]
    DrainDeadlineExceeded,
}

/// Builds the single live router from its validated permission/body/deadline inventory.
///
/// The branch-create route authenticates before typed request extraction and
/// invokes only the durable, transaction-authorized command when supplied.
/// Other protected routes remain closed until their command paths are composed.
///
/// # Errors
/// Returns [`ServeError::InvalidRouteInventory`] if declarations are invalid or the
/// reviewed snapshot does not match.
pub fn app_router(state: Arc<AppState>) -> Result<Router, ServeError> {
    let snapshot = validate_route_inventory()?;
    if snapshot != ROUTE_INVENTORY_SNAPSHOT {
        return Err(ServeError::InvalidRouteInventory);
    }

    let mut router = Router::new();
    for spec in ROUTES {
        let body_limit = DefaultBodyLimit::max(spec.body_limit);
        let timeout = TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, spec.timeout);
        router = match (spec.method, spec.path, spec.access) {
            (Method::Get, "/process/live", Access::Public { .. }) => router.route(
                spec.path,
                get(liveness)
                    .layer::<_, Infallible>(body_limit)
                    .layer::<_, Infallible>(timeout),
            ),
            (Method::Get, "/process/ready", Access::Public { .. }) => router.route(
                spec.path,
                get(readiness)
                    .layer::<_, Infallible>(body_limit)
                    .layer::<_, Infallible>(timeout),
            ),
            (
                Method::Post,
                "/v1/organizations/{organization_id}/branches",
                Access::Permission(_),
            ) => router.route(
                spec.path,
                post(create_branch_route)
                    .layer::<_, Infallible>(body_limit)
                    .layer::<_, Infallible>(timeout)
                    .route_layer(middleware::from_fn_with_state(
                        Arc::clone(&state),
                        authenticate_protected_request,
                    )),
            ),
            (Method::Post, _, Access::Permission(_)) => router.route(
                spec.path,
                post(protected_route)
                    .layer::<_, Infallible>(body_limit)
                    .layer::<_, Infallible>(timeout),
            ),
            _ => return Err(ServeError::InvalidRouteInventory),
        };
    }

    Ok(router.with_state(state).layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(ingress_error))
            .layer(LoadShedLayer::new())
            .layer(ConcurrencyLimitLayer::new(MAX_IN_FLIGHT_CONNECTIONS))
            .layer(RequestBodyLimitLayer::new(MAX_INGRESS_BODY_BYTES))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                REQUEST_TIMEOUT,
            )),
    ))
}

/// Serves the validated router and drains in-flight requests within the process deadline.
///
/// # Errors
/// Returns a listener error or drain timeout. In either case the error contains no
/// request data or credentials.
pub async fn serve(
    listener: TcpListener,
    state: Arc<AppState>,
    shutdown: oneshot::Receiver<()>,
) -> Result<Vec<Subsystem>, ServeError> {
    let router = app_router(state)?;
    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        let _ = shutdown.await;
    });
    match tokio::time::timeout(Duration::from_secs(SHUTDOWN_DRAIN_DEADLINE_SECS), server).await {
        Ok(Ok(())) => {
            let mut coordinator = ShutdownCoordinator::new();
            Ok(coordinator.shutdown().to_vec())
        }
        Ok(Err(error)) => Err(ServeError::Listener(error)),
        Err(_) => Err(ServeError::DrainDeadlineExceeded),
    }
}

fn validate_route_inventory() -> Result<String, ServeError> {
    let mut lines = Vec::with_capacity(ROUTES.len());
    for (index, spec) in ROUTES.iter().enumerate() {
        let access_ok = match spec.access {
            Access::Public { reason } => !reason.trim().is_empty(),
            Access::Permission(permission) => !permission.trim().is_empty(),
        };
        if !access_ok
            || spec.body_limit == 0
            || spec.body_limit > MAX_INGRESS_BODY_BYTES
            || spec.timeout.is_zero()
            || spec.timeout > Duration::from_secs(60)
            || ROUTES[..index]
                .iter()
                .any(|prior| prior.method == spec.method && prior.path == spec.path)
        {
            return Err(ServeError::InvalidRouteInventory);
        }
        lines.push(render_route(spec));
    }
    lines.sort_unstable();
    Ok(lines.join("\n"))
}

fn render_route(spec: &RouteSpec) -> String {
    let method = match spec.method {
        Method::Get => "GET",
        Method::Post => "POST",
    };
    let access = match spec.access {
        Access::Public { reason } => format!("public({reason})"),
        Access::Permission(permission) => format!("perm({permission})"),
    };
    format!(
        "{method} {} access={} body<={} timeout={}ms",
        spec.path,
        access,
        spec.body_limit,
        spec.timeout.as_millis()
    )
}

async fn liveness(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "status": "live",
        "service": state.service_name(),
        "version": state.service_version(),
    }))
}

async fn readiness(State(state): State<Arc<AppState>>) -> Response {
    if state.runtime_ready() {
        (StatusCode::OK, Json(json!({"status": "ready"}))).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status": "not_ready"})),
        )
            .into_response()
    }
}

async fn authenticate_protected_request(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let authorization = header_text(request.headers(), header::AUTHORIZATION);
    let session_selector = header_text(request.headers(), "session-id");
    match state
        .authenticate_request(authorization, session_selector)
        .await
    {
        Ok(security_context) => {
            request.extensions_mut().insert(security_context);
            next.run(request).await
        }
        Err(error) => app_error_response(error),
    }
}

async fn protected_route(State(state): State<Arc<AppState>>, request: Request<Body>) -> Response {
    let authorization = header_text(request.headers(), header::AUTHORIZATION);
    let session_selector = header_text(request.headers(), "session-id");
    match state
        .authenticate_request(authorization, session_selector)
        .await
    {
        Ok(_security_context) => app_error_response(AppError::DependencyUnavailable),
        Err(error) => app_error_response(error),
    }
}

async fn create_branch_route(
    State(state): State<Arc<AppState>>,
    Path(organization_id): Path<String>,
    Extension(security_context): Extension<SecurityContext>,
    axum::Json(request): axum::Json<CreateBranchRequest>,
) -> Response {
    if let Err(error) = validate_create_branch_input(&organization_id, &request) {
        return app_error_response(error);
    }
    let organization_id = match OrganizationId::new(organization_id) {
        Ok(value) => value,
        Err(error) => return app_error_response(sitolo_api::map_tenancy_error(&error)),
    };
    let branch_id = match BranchId::new(request.branch_id) {
        Ok(value) => value,
        Err(error) => return app_error_response(sitolo_api::map_tenancy_error(&error)),
    };
    let Some(service) = state.tenancy_service() else {
        return app_error_response(AppError::DependencyUnavailable);
    };
    match service
        .create_branch_for_authenticated_user(
            &security_context,
            branch_id,
            organization_id,
            &request.name,
        )
        .await
    {
        Ok(branch) => axum::Json(BranchResponse::from(&branch)).into_response(),
        Err(error) => app_error_response(map_authorized_tenancy_error(&error)),
    }
}

fn header_text(headers: &HeaderMap, name: impl axum::http::header::AsHeaderName) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

async fn ingress_error(error: Box<dyn StdError + Send + Sync>) -> Response {
    tracing::warn!(error_class = %std::any::type_name_of_val(error.as_ref()), "HTTP ingress rejected request");
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "service temporarily unavailable",
    )
        .into_response()
}

fn app_error_response(error: AppError) -> Response {
    let request_id = RequestId::new_server();
    let details = ProblemDetails::from_error(&error, &request_id);
    let status = StatusCode::from_u16(details.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        status,
        [(header::CONTENT_TYPE, "application/problem+json")],
        details.json(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_inventory_matches_reviewed_snapshot() {
        assert_eq!(
            validate_route_inventory().expect("route inventory validates"),
            ROUTE_INVENTORY_SNAPSHOT
        );
    }
}
