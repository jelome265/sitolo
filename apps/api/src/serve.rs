//! Axum/Tokio HTTP serving boundary.
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, Path, State};
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto::Builder;
use hyper_util::service::TowerToHyperService;
use sitolo_api::tenancy::{
    CreateBranchRequest, CreateOrganizationRequest, handle_activate_branch,
    handle_activate_organization, handle_begin_close_branch, handle_begin_close_organization,
    handle_close_branch, handle_close_organization, handle_create_branch,
    handle_provision_organization, handle_resume_branch, handle_resume_organization,
    handle_suspend_branch, handle_suspend_organization,
};
use sitolo_api::{AppError, ProblemDetails};
use sitolo_observability::RequestId;
use socket2::{SockRef, TcpKeepalive};
use tokio::sync::{Semaphore, oneshot, watch};
use tokio::task::JoinSet;
use tower::ServiceBuilder;
use tower::ServiceExt;
use tower::limit::GlobalConcurrencyLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::{RequestBodyTimeoutLayer, TimeoutLayer};
use tower_http::trace::TraceLayer;

use crate::shutdown::{
    MAX_IN_FLIGHT_CONNECTIONS, MAX_IN_FLIGHT_REQUESTS, SHUTDOWN_DRAIN_DEADLINE_SECS, Subsystem,
};
use crate::state::AppState;
use sitolo_config::AppConfig;

const REQUEST_BODY_IDLE_TIMEOUT_SECS: u64 = 5;
const REQUEST_TIMEOUT_SECS: u64 = 30;
const HTTP1_IDLE_TIMEOUT_SECS: u64 = 60;
const HTTP2_PING_INTERVAL_SECS: u64 = 30;
const RESPONSE_BODY_TIMEOUT_SECS: u64 = 60;
const COMPAT_RESPONSE_BODY_MAX_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct HttpTransportConfig {
    pub max_request_body_bytes: usize,
    pub request_header_timeout: Duration,
    pub keepalive_timeout: Duration,
    pub request_timeout: Duration,
    pub request_body_idle_timeout: Duration,
    pub http1_idle_timeout: Duration,
    pub http2_ping_interval: Duration,
    pub response_body_timeout: Duration,
}

impl HttpTransportConfig {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            max_request_body_bytes: config.max_request_body_bytes as usize,
            request_header_timeout: Duration::from_millis(config.request_header_timeout_ms),
            keepalive_timeout: Duration::from_millis(config.keepalive_timeout_ms),
            request_timeout: Duration::from_secs(REQUEST_TIMEOUT_SECS),
            request_body_idle_timeout: Duration::from_secs(REQUEST_BODY_IDLE_TIMEOUT_SECS),
            http1_idle_timeout: Duration::from_secs(HTTP1_IDLE_TIMEOUT_SECS),
            http2_ping_interval: Duration::from_secs(HTTP2_PING_INTERVAL_SECS),
            response_body_timeout: Duration::from_secs(RESPONSE_BODY_TIMEOUT_SECS),
        }
    }
}

pub fn router(state: Arc<AppState>, max_request_body_bytes: usize) -> Router {
    Router::new()
        .route("/process/live", get(live))
        .route("/process/ready", get(ready))
        .route("/v1/organizations", post(provision_organization))
        .route("/v1/organizations/{organization_id}/branches", post(create_branch))
        .route("/v1/organizations/{organization_id}/{action}", post(organization_action))
        .route("/v1/organizations/{organization_id}/branches/{branch_id}/{action}", post(branch_action))
        .layer(TraceLayer::new_for_http())
        .layer(
            ServiceBuilder::new()
                .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(REQUEST_TIMEOUT_SECS)))
                .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(REQUEST_BODY_IDLE_TIMEOUT_SECS)))
                .layer(RequestBodyLimitLayer::new(max_request_body_bytes.min(sitolo_api::tenancy::bounds::MAX_TENANCY_BODY_BYTES)))
                .layer(GlobalConcurrencyLimitLayer::new(MAX_IN_FLIGHT_REQUESTS)),
        )
        .with_state(state)
}

pub async fn serve(
    listener: tokio::net::TcpListener,
    state: Arc<AppState>,
    mut shutdown: oneshot::Receiver<()>,
    transport: HttpTransportConfig,
) -> Vec<Subsystem> {
    let app = router(Arc::clone(&state), transport.max_request_body_bytes);
    let mut connections = JoinSet::new();
    let connection_permits = Arc::new(Semaphore::new(MAX_IN_FLIGHT_CONNECTIONS));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accept = listener.accept() => {
                let (stream, peer) = match accept {
                    Ok(value) => value,
                    Err(error) => { tracing::error!(%error, "failed to accept TCP connection"); continue; }
                };
                let permit = match connection_permits.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => { tracing::warn!(?peer, "rejecting connection at concurrency ceiling"); drop(stream); continue; }
                };
                if let Err(error) = configure_tcp_keepalive(&stream, transport.keepalive_timeout) {
                    tracing::warn!(%error, ?peer, "failed to configure TCP keepalive");
                }
                let service = app.clone();
                let header_timeout = transport.request_header_timeout;
                let keepalive = transport.keepalive_timeout;
                let h2_ping = transport.http2_ping_interval;
                let mut conn_shutdown_rx = shutdown_rx.clone();
                
                connections.spawn(async move {
                    let _connection_permit = permit;
                    let mut builder = Builder::new(TokioExecutor::new());
                    builder.http1().timer(TokioTimer::new()).header_read_timeout(header_timeout);
                    builder.http2().timer(TokioTimer::new()).max_concurrent_streams(MAX_IN_FLIGHT_REQUESTS as u32)
                        .keep_alive_interval(h2_ping).keep_alive_timeout(keepalive);

                    let io = TokioIo::new(stream);
                    let hyper_service = TowerToHyperService::new(service);
                    tokio::select! {
                        res = builder.serve_connection(io, hyper_service) => {
                            if let Err(error) = res { tracing::debug!(%error, ?peer, "HTTP connection closed with error"); }
                        }
                        _ = conn_shutdown_rx.changed() => {
                            tracing::debug!(?peer, "connection draining initiated");
                        }
                    }
                });
            }
        }
    }

    let _ = shutdown_tx.send(true);
    let drain = async { while connections.join_next().await.is_some() {} };
    let _ = tokio::time::timeout(Duration::from_secs(SHUTDOWN_DRAIN_DEADLINE_SECS), drain).await;
    connections.abort_all();

    let mut coordinator = crate::shutdown::ShutdownCoordinator::new();
    coordinator.shutdown().to_vec()
}

fn configure_tcp_keepalive(stream: &tokio::net::TcpStream, keepalive_timeout: Duration) -> std::io::Result<()> {
    let keepalive = TcpKeepalive::new().with_time(keepalive_timeout);
    SockRef::from(stream).set_tcp_keepalive(&keepalive)
}

async fn live(State(state): State<Arc<AppState>>) -> Response {
    let body = format!("{{\"status\":\"live\",\"service\":\"{}\",\"version\":\"{}\"}}",
        json_escape(state.service_name()), json_escape(state.service_version()));
    json_body(StatusCode::OK, body)
}

async fn ready(State(state): State<Arc<AppState>>) -> Response {
    if state.readiness().is_draining() {
        return json_body(StatusCode::SERVICE_UNAVAILABLE, "{\"status\":\"draining\"}".to_string());
    }
    match state.readiness().get_state() {
        crate::shutdown::ReadinessState::Ready => json_body(StatusCode::OK, "{\"status\":\"ready\"}".to_string()),
        _ => json_body(StatusCode::SERVICE_UNAVAILABLE, "{\"status\":\"not_ready\"}".to_string()),
    }
}

async fn provision_organization(State(state): State<Arc<AppState>>, payload: Result<Json<CreateOrganizationRequest>, JsonRejection>) -> Response {
    let Json(req) = match payload {
        Ok(value) => value,
        Err(rejection) => {
            if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE { return format_error_response(&AppError::PayloadTooLarge); }
            return format_error_response(&AppError::Validation);
        }
    };
    match handle_provision_organization(state.tenancy_service(), req).await {
        Ok(response) => json_response(StatusCode::CREATED, response),
        Err(error) => format_error_response(&error),
    }
}

async fn create_branch(State(state): State<Arc<AppState>>, Path(organization_id): Path<String>, payload: Result<Json<CreateBranchRequest>, JsonRejection>) -> Response {
    let Json(req) = match payload {
        Ok(value) => value,
        Err(rejection) => {
            if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE { return format_error_response(&AppError::PayloadTooLarge); }
            return format_error_response(&AppError::Validation);
        }
    };
    match handle_create_branch(state.tenancy_service(), &organization_id, req).await {
        Ok(response) => json_response(StatusCode::CREATED, response),
        Err(error) => format_error_response(&error),
    }
}

async fn organization_action(State(state): State<Arc<AppState>>, Path((organization_id, action)): Path<(String, String)>) -> Response {
    let result = match action.as_str() {
        "activate" => handle_activate_organization(state.tenancy_service(), &organization_id).await,
        "suspend" => handle_suspend_organization(state.tenancy_service(), &organization_id).await,
        "resume" => handle_resume_organization(state.tenancy_service(), &organization_id).await,
        "begin_close" => handle_begin_close_organization(state.tenancy_service(), &organization_id).await,
        "close" => handle_close_organization(state.tenancy_service(), &organization_id).await,
        _ => return format_error_response(&AppError::NotFound),
    };
    match result { Ok(r) => json_response(StatusCode::OK, r), Err(e) => format_error_response(&e) }
}

async fn branch_action(State(state): State<Arc<AppState>>, Path((org_id, branch_id, action)): Path<(String, String, String)>) -> Response {
    let result = match action.as_str() {
        "activate" => handle_activate_branch(state.tenancy_service(), &org_id, &branch_id).await,
        "suspend" => handle_suspend_branch(state.tenancy_service(), &org_id, &branch_id).await,
        "resume" => handle_resume_branch(state.tenancy_service(), &org_id, &branch_id).await,
        "begin_close" => handle_begin_close_branch(state.tenancy_service(), &org_id, &branch_id).await,
        "close" => handle_close_branch(state.tenancy_service(), &org_id, &branch_id).await,
        _ => return format_error_response(&AppError::NotFound),
    };
    match result { Ok(r) => json_response(StatusCode::OK, r), Err(e) => format_error_response(&e) }
}

fn json_response<T: serde::Serialize>(status: StatusCode, value: T) -> Response { (status, Json(value)).into_response() }
fn json_body(status: StatusCode, body: String) -> Response {
    let mut r = (status, Body::from(body)).into_response();
    r.headers_mut().insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
    r
}
fn format_error_response(err: &AppError) -> Response {
    let status = StatusCode::from_u16(err.public().status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let problem = ProblemDetails::from_error(err, &RequestId::new_server());
    let mut r = (status, Body::from(problem.json())).into_response();
    r.headers_mut().insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/problem+json"));
    r
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""), '\\' => out.push_str("\\\\"), '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"), '\n' => out.push_str("\\n"), '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"), c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out
}
