//! Axum/Tokio HTTP serving boundary.
//!
//! Production ingress is implemented exclusively through Axum's router and
//! Tokio's asynchronous runtime. The API process owns lifecycle and graceful
//! shutdown here; request routing and transport decoding stay inside Axum.
#![forbid(unsafe_code)]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Json, MatchedPath, Path, State};
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
use sitolo_observability::{RequestId, TraceParent};
use socket2::{SockRef, TcpKeepalive};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{Semaphore, oneshot, watch};
use tokio::task::JoinSet;
use tower::ServiceBuilder;
use tower::ServiceExt;
use tower::limit::GlobalConcurrencyLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::{RequestBodyTimeoutLayer, ResponseBodyTimeoutLayer, TimeoutLayer};
use tower_http::trace::TraceLayer;

use crate::shutdown::{
    MAX_IN_FLIGHT_CONNECTIONS, MAX_IN_FLIGHT_REQUESTS, ReadinessState,
    SHUTDOWN_DRAIN_DEADLINE_SECS, Subsystem,
};
use crate::state::AppState;
use sitolo_config::AppConfig;

const COMPAT_RESPONSE_BODY_MAX_BYTES: usize = 64 * 1024;

/// Compiled safe-default transport timeouts, used only by the CI/test-pinned
/// [`router`] compatibility shim below. Production serving always goes
/// through [`serve`], which projects the real validated `AppConfig` via
/// `HttpTransportConfig::from_config` instead of these fallbacks.
fn compiled_default_transport() -> HttpTransportConfig {
    let config = sitolo_config::validate(sitolo_config::defaults::Builder::development())
        .expect("compiled development defaults are always valid");
    HttpTransportConfig::from_config(&config)
}

/// Validated transport controls projected from the process configuration.
///
/// Every duration here has a single, distinct meaning and traces to a typed
/// configuration value (Stage 07 remediation of the Stage 06 transport
/// audit; see `crates/sitolo-config` for the validated source fields):
///
/// - `keepalive_timeout` is the OS-level TCP keepalive probe interval only.
///   It says nothing about how long an idle HTTP/1 keep-alive connection or
///   an HTTP/2 session is allowed to sit unused.
/// - `http1_idle_timeout` is the application-level policy that closes an
///   HTTP/1 persistent connection that has gone quiet, independent of TCP.
///   Operational note: hyper's header-read timer (`request_header_timeout`)
///   also runs while a keep-alive connection waits for its *next* request's
///   headers, so the effective idle bound for HTTP/1 is the smaller of the
///   two. `http1_idle_timeout` is the binding limit only when it is shorter
///   than `request_header_timeout`, and is what bounds a connection that
///   trickles bytes without ever completing a request.
/// - `http2_ping_interval`/`http2_keep_alive_timeout` are the HTTP/2
///   protocol PING liveness policy, independent of both of the above.
/// - `request_timeout` is the total wall-clock deadline for a request.
/// - `request_body_idle_timeout` is an *idle* deadline between request-body
///   chunks, not a total body-transfer deadline; `request_timeout` remains
///   the outer bound that catches a peer trickling bytes forever.
/// - `response_body_timeout` is the equivalent idle deadline for the
///   *response* body, so a slow/stalled consumer cannot hold a connection
///   (and its concurrency slot) open indefinitely either.
#[derive(Debug, Clone, Copy)]
pub struct HttpTransportConfig {
    pub max_request_body_bytes: usize,
    pub request_header_timeout: Duration,
    pub keepalive_timeout: Duration,
    pub request_timeout: Duration,
    pub request_body_idle_timeout: Duration,
    pub http1_idle_timeout: Duration,
    pub http2_ping_interval: Duration,
    pub http2_keep_alive_timeout: Duration,
    pub response_body_timeout: Duration,
}

impl HttpTransportConfig {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            max_request_body_bytes: config.max_request_body_bytes as usize,
            request_header_timeout: Duration::from_millis(config.request_header_timeout_ms),
            keepalive_timeout: Duration::from_millis(config.keepalive_timeout_ms),
            request_timeout: Duration::from_millis(config.request_timeout_ms),
            request_body_idle_timeout: Duration::from_millis(config.request_body_idle_timeout_ms),
            http1_idle_timeout: Duration::from_millis(config.http1_idle_timeout_ms),
            http2_ping_interval: Duration::from_millis(config.http2_ping_interval_ms),
            http2_keep_alive_timeout: Duration::from_millis(config.http2_keep_alive_timeout_ms),
            response_body_timeout: Duration::from_millis(config.response_body_timeout_ms),
        }
    }
}

/// Builds the production HTTP application with the compiled safe-default
/// transport timeouts.
///
/// This exact signature is relied upon by `scripts/ci/check-api-architecture`
/// and by the test-only dispatch adapters below (and, transitively, by
/// `apps/api/tests/tenancy_api.rs`), none of which carry a full
/// `HttpTransportConfig`. Production serving goes through [`serve`], which
/// uses the real, config-provenanced durations instead of these compiled
/// defaults.
pub fn router(state: Arc<AppState>, max_request_body_bytes: usize) -> Router {
    router_with_transport(state, max_request_body_bytes, compiled_default_transport())
}

fn router_with_transport(
    state: Arc<AppState>,
    max_request_body_bytes: usize,
    transport: HttpTransportConfig,
) -> Router {
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
        .layer(
            // Order matters (first added = outermost). `TimeoutLayer` builds
            // its 408 with `ResBody::default()`, and tower-http's
            // `TimeoutBody` (produced by `ResponseBodyTimeoutLayer`) has no
            // `Default` impl, so the response-body timeout must wrap the
            // request deadline, never sit inside it. Keeping `TimeoutLayer`
            // outside the body limit and concurrency gate also means the
            // wall-clock deadline covers time spent queued for a permit.
            ServiceBuilder::new()
                .layer(ResponseBodyTimeoutLayer::new(
                    transport.response_body_timeout,
                ))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    transport.request_timeout,
                ))
                .layer(RequestBodyTimeoutLayer::new(
                    transport.request_body_idle_timeout,
                ))
                .layer(RequestBodyLimitLayer::new(
                    max_request_body_bytes.min(sitolo_api::tenancy::bounds::MAX_TENANCY_BODY_BYTES),
                ))
                .layer(GlobalConcurrencyLimitLayer::new(MAX_IN_FLIGHT_REQUESTS)),
        )
        // Outermost (added last): request identity and the completion event.
        // Every request that reaches a route passes through here *around* the
        // timeout, body-limit and concurrency layers above, so requests those
        // layers reject (413, 408, ...) are observed too rather than silently
        // producing no span. Each `Router::layer` call erases the body type,
        // so this ordering is independent of the inner layers' body types.
        .layer(
            ServiceBuilder::new()
                // Assigns the request-scoped identity exactly once, before
                // anything downstream runs: reuse a well-formed client
                // `x-request-id` if the peer supplied one, otherwise mint a
                // server identity. Stored in request extensions so the span
                // below, every handler, and `format_error_response` all
                // observe the *same* identity rather than each minting its
                // own (the Stage 06 audit's specific finding: "creating a
                // new request ID inside an error formatter is not
                // equivalent to propagating the original request identity
                // through the complete lifecycle").
                .map_request(|mut request: Request<Body>| {
                    let request_id = request
                        .headers()
                        .get("x-request-id")
                        .and_then(|value| value.to_str().ok())
                        .and_then(RequestId::parse_client)
                        .unwrap_or_else(RequestId::new_server);
                    request.extensions_mut().insert(request_id);
                    // W3C trace context: accept only a single, semantically
                    // valid `traceparent`. More than one header is discarded
                    // outright (the spec forbids guessing which to trust) and
                    // anything `TraceParent::parse` rejects is ignored rather
                    // than echoed into telemetry. Diagnostic correlation only;
                    // never authorization evidence.
                    let trace_parent = {
                        let mut values = request.headers().get_all("traceparent").iter();
                        match (values.next(), values.next()) {
                            (Some(value), None) => value
                                .to_str()
                                .ok()
                                .and_then(TraceParent::parse),
                            _ => None,
                        }
                    };
                    if let Some(trace_parent) = trace_parent {
                        request.extensions_mut().insert(trace_parent);
                    }
                    request
                })
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|request: &Request<Body>| {
                            let request_id = request
                                .extensions()
                                .get::<RequestId>()
                                .map(|id| id.as_str().to_string())
                                .unwrap_or_else(|| "unassigned".to_string());
                            // Fields follow docs/telemetry/metrics.yaml
                            // (`method`, `route_template`): the matched route
                            // template is low-cardinality, whereas the raw URI
                            // embeds user-controlled identifiers (organization
                            // and branch ids) that the redaction rules forbid
                            // as metric labels.
                            let trace_parent = request
                                .extensions()
                                .get::<TraceParent>()
                                .map(|value| value.as_str().to_string());
                            tracing::info_span!(
                                "http_request",
                                method = %request.method(),
                                route_template = %route_template(request),
                                request_id = %request_id,
                                // Omitted entirely when absent or invalid.
                                traceparent = trace_parent.as_deref(),
                            )
                        })
                        .on_response(
                            |response: &Response, latency: Duration, _span: &tracing::Span| {
                                // Registered event `http.request.completed`
                                // (docs/telemetry/events.yaml).
                                tracing::info!(
                                    event = "http.request.completed",
                                    status = response.status().as_u16(),
                                    status_class = %status_class(response.status()),
                                    latency_ms = latency.as_millis(),
                                    "request completed"
                                );
                            },
                        ),
                ),
        )
        .with_state(state)
}

/// Serves the Axum application until shutdown, then waits for in-flight
/// requests for at most the configured drain deadline.
///
/// Shutdown sequence:
///
/// 1. The shutdown signal fires. `state.readiness().mark_draining()` flips
///    `/process/ready` to fail *before* anything else happens, so a load
///    balancer can stop routing new traffic here immediately.
/// 2. The accept loop stops accepting new TCP connections.
/// 3. Every in-flight Hyper connection is told to gracefully shut down via
///    a real `hyper_util` `Connection::graceful_shutdown()` call (HTTP/1:
///    finish the in-flight response, then refuse further keep-alive reuse;
///    HTTP/2: GOAWAY-style draining), raced against the connection future
///    itself so the drain and the wait below proceed concurrently.
/// 4. The drain is bounded by `SHUTDOWN_DRAIN_DEADLINE_SECS`. Connections
///    that do not cooperate within the deadline are aborted so shutdown
///    always terminates.
pub async fn serve(
    listener: tokio::net::TcpListener,
    state: Arc<AppState>,
    mut shutdown: oneshot::Receiver<()>,
    transport: HttpTransportConfig,
) -> Vec<Subsystem> {
    let app = router_with_transport(
        Arc::clone(&state),
        transport.max_request_body_bytes,
        transport,
    );
    let mut connections = JoinSet::new();
    let connection_permits = Arc::new(Semaphore::new(MAX_IN_FLIGHT_CONNECTIONS));
    // Fires exactly once, when this loop stops accepting connections, so
    // every live connection task can initiate protocol-aware graceful
    // shutdown concurrently with the bounded wait below.
    let (graceful_tx, graceful_rx) = watch::channel(false);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                // Published *before* the loop breaks: a readiness probe
                // observed from this instant on must fail.
                state.readiness().mark_draining();
                break;
            }
            accept = listener.accept() => {
                let (stream, peer) = match accept {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::error!(%error, "failed to accept TCP connection");
                        continue;
                    }
                };

                let permit = match connection_permits.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => {
                        tracing::warn!(?peer, "rejecting connection at concurrency ceiling");
                        drop(stream);
                        continue;
                    }
                };

                if let Err(error) = configure_tcp_keepalive(&stream, transport.keepalive_timeout) {
                    tracing::warn!(%error, ?peer, "failed to configure TCP keepalive");
                }

                let service = app.clone();
                let header_timeout = transport.request_header_timeout;
                let mut graceful_rx = graceful_rx.clone();
                connections.spawn(async move {
                    let _connection_permit = permit;
                    let mut builder = Builder::new(TokioExecutor::new());
                    builder
                        .http1()
                        .timer(TokioTimer::new())
                        .header_read_timeout(header_timeout)
                        .keep_alive(true);

                    builder
                        .http2()
                        .timer(TokioTimer::new())
                        .max_concurrent_streams(MAX_IN_FLIGHT_REQUESTS as u32)
                        .keep_alive_interval(transport.http2_ping_interval)
                        .keep_alive_timeout(transport.http2_keep_alive_timeout);

                    // Distinct from TCP keepalive: this evicts a connection
                    // that is idle at the *application* protocol level (no
                    // bytes read for `http1_idle_timeout`), even while the
                    // OS-level TCP session is still healthy. The deadline
                    // resets on every successful read, so a connection that
                    // is actively serving requests is never evicted for
                    // merely staying open past `http1_idle_timeout`.
                    let io = IdleTimeoutIo::new(stream, transport.http1_idle_timeout);
                    let conn = builder
                        .serve_connection(TokioIo::new(io), TowerToHyperService::new(service));
                    let mut conn = std::pin::pin!(conn);

                    loop {
                        tokio::select! {
                            result = conn.as_mut() => {
                                if let Err(error) = result {
                                    tracing::debug!(%error, ?peer, "HTTP connection closed with error");
                                }
                                break;
                            }
                            changed = graceful_rx.changed() => {
                                if changed.is_ok() {
                                    tracing::debug!(?peer, "connection draining initiated");
                                    conn.as_mut().graceful_shutdown();
                                }
                            }
                        }
                    }
                });
            }
        }
    }

    // Close the listening socket immediately, rather than merely stopping
    // our own accept() calls: the OS keeps completing TCP handshakes into
    // the backlog for as long as the listening fd stays open, even once
    // nothing in this process ever calls accept() on it again. Left open,
    // a client connecting during the drain window would succeed at the TCP
    // layer and then hang forever waiting for bytes that will never come.
    // Dropping the listener here makes that fail fast (connection refused)
    // instead, which is the correct, bounded failure mode for new traffic
    // arriving after this process has started draining.
    drop(listener);

    let _ = graceful_tx.send(true);

    let drain = async { while connections.join_next().await.is_some() {} };
    let _ = tokio::time::timeout(Duration::from_secs(SHUTDOWN_DRAIN_DEADLINE_SECS), drain).await;

    // Bounded hard-stop fallback: any connection that did not cooperate
    // with graceful shutdown within the deadline is aborted so process
    // shutdown is still bounded.
    connections.abort_all();

    let mut coordinator = crate::shutdown::ShutdownCoordinator::new();
    coordinator.shutdown().to_vec()
}

fn configure_tcp_keepalive(
    stream: &tokio::net::TcpStream,
    keepalive_timeout: Duration,
) -> std::io::Result<()> {
    let keepalive = TcpKeepalive::new().with_time(keepalive_timeout);
    SockRef::from(stream).set_tcp_keepalive(&keepalive)
}

/// Wraps a raw Tokio I/O type with an application-level idle-connection
/// timeout, independent of TCP keepalive.
///
/// TCP keepalive proves the *peer socket* is alive, but says nothing about
/// whether an HTTP/1 persistent connection has gone application-idle. A
/// client can keep a keep-alive connection open indefinitely without ever
/// sending another request, holding a connection-concurrency slot forever;
/// TCP keepalive alone will not reclaim it as long as the peer's OS answers
/// probes.
///
/// The deadline resets on every successful *read* (new request bytes
/// arriving), not on writes: "idle" here means "no new request activity",
/// matching how HTTP/1 keep-alive idle policy is conventionally defined by
/// reverse proxies (e.g. nginx `keepalive_timeout`). A connection that is
/// continuously serving requests never trips this, unlike a bare timer
/// started once at connection open.
struct IdleTimeoutIo<T> {
    inner: T,
    idle_timeout: Duration,
    idle_deadline: Pin<Box<tokio::time::Sleep>>,
}

impl<T> IdleTimeoutIo<T> {
    fn new(inner: T, idle_timeout: Duration) -> Self {
        Self {
            inner,
            idle_timeout,
            idle_deadline: Box::pin(tokio::time::sleep(idle_timeout)),
        }
    }

    fn reset_idle_deadline(&mut self) {
        self.idle_deadline
            .as_mut()
            .reset(tokio::time::Instant::now() + self.idle_timeout);
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for IdleTimeoutIo<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.idle_deadline.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "HTTP/1 keep-alive connection idle timeout",
            )));
        }
        let before = buf.filled().len();
        let inner = Pin::new(&mut self.inner);
        let result = AsyncRead::poll_read(inner, cx, buf);
        if matches!(result, Poll::Ready(Ok(()))) && buf.filled().len() > before {
            self.reset_idle_deadline();
        }
        result
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for IdleTimeoutIo<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        AsyncWrite::poll_write(Pin::new(&mut self.get_mut().inner), cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        AsyncWrite::poll_flush(Pin::new(&mut self.get_mut().inner), cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        AsyncWrite::poll_shutdown(Pin::new(&mut self.get_mut().inner), cx)
    }
}

async fn live(State(state): State<Arc<AppState>>) -> Response {
    let body = format!(
        "{{\"status\":\"live\",\"service\":\"{}\",\"version\":\"{}\"}}",
        json_escape(state.service_name()),
        json_escape(state.service_version())
    );
    json_body(StatusCode::OK, body)
}

/// Matched route template for telemetry (`/v1/organizations/{organization_id}/...`),
/// never the raw path. Requests that match no route report `unmatched`.
fn route_template(request: &Request<Body>) -> &str {
    request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or("unmatched")
}

/// Low-cardinality HTTP status class label (`2xx`, `4xx`, ...).
fn status_class(status: StatusCode) -> &'static str {
    match status.as_u16() {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "other",
    }
}

/// A genuine readiness probe, consulting [`AppState::readiness`], the
/// single authoritative process lifecycle signal.
async fn ready(State(state): State<Arc<AppState>>) -> Response {
    if state.readiness().is_draining() {
        return json_body(
            StatusCode::SERVICE_UNAVAILABLE,
            "{\"status\":\"draining\"}".to_string(),
        );
    }
    match state.readiness().get_state() {
        ReadinessState::Ready => json_body(StatusCode::OK, "{\"status\":\"ready\"}".to_string()),
        _ => json_body(
            StatusCode::SERVICE_UNAVAILABLE,
            "{\"status\":\"not_ready\"}".to_string(),
        ),
    }
}

async fn provision_organization(
    State(state): State<Arc<AppState>>,
    Extension(request_id): Extension<RequestId>,
    payload: Result<Json<CreateOrganizationRequest>, JsonRejection>,
) -> Response {
    let Json(req) = match payload {
        Ok(value) => value,
        Err(rejection) => {
            if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
                return format_error_response(&AppError::PayloadTooLarge, &request_id);
            }
            return format_error_response(&AppError::Validation, &request_id);
        }
    };

    match handle_provision_organization(state.tenancy_service(), req).await {
        Ok(response) => json_response(StatusCode::CREATED, response),
        Err(error) => format_error_response(&error, &request_id),
    }
}

async fn create_branch(
    State(state): State<Arc<AppState>>,
    Extension(request_id): Extension<RequestId>,
    Path(organization_id): Path<String>,
    payload: Result<Json<CreateBranchRequest>, JsonRejection>,
) -> Response {
    let Json(req) = match payload {
        Ok(value) => value,
        Err(rejection) => {
            // Oversized JSON bodies must map to 413 the same way across
            // every JSON endpoint, matching `provision_organization`.
            if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
                return format_error_response(&AppError::PayloadTooLarge, &request_id);
            }
            return format_error_response(&AppError::Validation, &request_id);
        }
    };

    match handle_create_branch(state.tenancy_service(), &organization_id, req).await {
        Ok(response) => json_response(StatusCode::CREATED, response),
        Err(error) => format_error_response(&error, &request_id),
    }
}

async fn organization_action(
    State(state): State<Arc<AppState>>,
    Extension(request_id): Extension<RequestId>,
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
        _ => return format_error_response(&AppError::NotFound, &request_id),
    };

    match result {
        Ok(response) => json_response(StatusCode::OK, response),
        Err(error) => format_error_response(&error, &request_id),
    }
}

async fn branch_action(
    State(state): State<Arc<AppState>>,
    Extension(request_id): Extension<RequestId>,
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
        _ => return format_error_response(&AppError::NotFound, &request_id),
    };

    match result {
        Ok(response) => json_response(StatusCode::OK, response),
        Err(error) => format_error_response(&error, &request_id),
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

fn format_error_response(err: &AppError, request_id: &RequestId) -> Response {
    let status =
        StatusCode::from_u16(err.public().status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let problem = ProblemDetails::from_error(err, request_id);
    let mut response = (status, Body::from(problem.json())).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/problem+json"),
    );
    response
}

/// Test-only compatibility adapter, relied on by `apps/api/tests/tenancy_api.rs`.
/// It intentionally exercises the same production Axum router rather than
/// maintaining a second request parser. This variant intentionally omits
/// Content-Length so the bounded body wrapper itself must surface an
/// oversized request as 413.
pub async fn dispatch_request_without_content_length(
    method: &str,
    path: &str,
    body: &str,
    state: &AppState,
) -> (&'static str, String) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("test request construction must succeed");

    let response = router(Arc::new(state.clone()), 2 * 1024)
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

/// Test-only compatibility adapter, relied on by `apps/api/tests/tenancy_api.rs`.
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
        .header(header::CONTENT_LENGTH, body.len().to_string())
        .body(Body::from(body.to_owned()))
        .expect("test request construction must succeed");

    let response = router(Arc::new(state.clone()), 32 * 1024)
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
    fn status_class_buckets_every_range() {
        assert_eq!(status_class(StatusCode::CONTINUE), "1xx");
        assert_eq!(status_class(StatusCode::OK), "2xx");
        assert_eq!(status_class(StatusCode::NO_CONTENT), "2xx");
        assert_eq!(status_class(StatusCode::FOUND), "3xx");
        assert_eq!(status_class(StatusCode::NOT_FOUND), "4xx");
        assert_eq!(status_class(StatusCode::PAYLOAD_TOO_LARGE), "4xx");
        assert_eq!(status_class(StatusCode::SERVICE_UNAVAILABLE), "5xx");
    }

    #[test]
    fn route_template_defaults_to_unmatched_outside_a_route() {
        let request = Request::builder()
            .uri("/v1/organizations/org-123/branches")
            .body(Body::empty())
            .expect("request builds");
        // Never falls back to the raw path: it embeds user-controlled ids.
        assert_eq!(route_template(&request), "unmatched");
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
        let _router = router(Arc::new(state), 32 * 1024);
    }
}
