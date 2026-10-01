//! Request telemetry contract tests.
//!
//! These drive the real production router through `tower::ServiceExt` and
//! capture the output of a real `tracing-subscriber` formatter, asserting on
//! what would actually be emitted. They pin the contract in
//! `docs/telemetry/{events,metrics,redaction}.yaml`: the registered
//! `http.request.completed` event, low-cardinality `route_template` /
//! `status_class` labels, no user-controlled identifiers in labels, and one
//! request identity shared by the log line and the error response body.

use std::io;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use sitolo_api_bin::serve::router;
use sitolo_api_bin::state::AppState;
use sitolo_config::DatabaseTarget;
use sitolo_observability::TelemetryBuffer;
use tower::ServiceExt;

/// In-memory sink for the formatter, so tests read real emitted output.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("capture lock").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Capture {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("capture lock")).into_owned()
    }
}

/// Installs a scoped, thread-local subscriber for the current test. The
/// `#[tokio::test]` runtime is single-threaded, so everything the router does
/// runs on this thread and is observed.
fn capture_tracing() -> (Capture, tracing::subscriber::DefaultGuard) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    (capture, tracing::subscriber::set_default(subscriber))
}

fn app() -> Router {
    let state = AppState::new(
        "sitolo".into(),
        "test".into(),
        "fingerprint".into(),
        DatabaseTarget {
            host: "localhost".into(),
            port: 5432,
            database: "sitolo".into(),
            username: "sitolo".into(),
        },
        Arc::new(Mutex::new(TelemetryBuffer::new(8))),
    );
    router(Arc::new(state), 32 * 1024)
}

async fn call(request: Request<Body>) -> axum::response::Response {
    app().oneshot(request).await.expect("router is infallible")
}

#[tokio::test]
async fn completed_request_emits_the_registered_event_with_registry_labels() {
    let (capture, _guard) = capture_tracing();

    let response = call(
        Request::builder()
            .uri("/process/live")
            .body(Body::empty())
            .expect("request builds"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let log = capture.text();
    assert!(log.contains("http.request.completed"), "{log}");
    assert!(log.contains("method=GET"), "{log}");
    assert!(log.contains("route_template=/process/live"), "{log}");
    assert!(log.contains("status=200"), "{log}");
    assert!(log.contains("status_class=2xx"), "{log}");
    assert!(log.contains("latency_ms="), "{log}");
}

#[tokio::test]
async fn labels_use_the_route_template_never_user_controlled_identifiers() {
    let (capture, _guard) = capture_tracing();

    let _ = call(
        Request::builder()
            .method("POST")
            .uri("/v1/organizations/org-secret-identifier-123/suspend")
            .body(Body::empty())
            .expect("request builds"),
    )
    .await;

    let log = capture.text();
    assert!(
        log.contains("route_template=/v1/organizations/{organization_id}/{action}"),
        "{log}"
    );
    // docs/telemetry/redaction.yaml: user-controlled identifiers are never
    // metric labels. The raw path must not appear in the emitted record.
    assert!(!log.contains("org-secret-identifier-123"), "{log}");
}

#[tokio::test]
async fn requests_rejected_by_the_body_limit_layer_are_still_observed() {
    let (capture, _guard) = capture_tracing();

    // The declared length exceeds the body limit, so `RequestBodyLimitLayer`
    // answers 413 itself and the handler never runs. The telemetry layer
    // wraps it, so the rejection must still produce the completion event.
    let oversized = 64 * 1024;
    let response = call(
        Request::builder()
            .method("POST")
            .uri("/v1/organizations")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONTENT_LENGTH, oversized.to_string())
            .body(Body::from(vec![b'a'; oversized]))
            .expect("request builds"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let log = capture.text();
    assert!(log.contains("http.request.completed"), "{log}");
    assert!(log.contains("route_template=/v1/organizations"), "{log}");
    assert!(log.contains("status=413"), "{log}");
    assert!(log.contains("status_class=4xx"), "{log}");
}

#[tokio::test]
async fn client_request_id_is_shared_by_the_log_and_the_error_body() {
    let (capture, _guard) = capture_tracing();

    let response = call(
        Request::builder()
            .method("POST")
            .uri("/v1/organizations")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-request-id", "client-correlation-0001")
            .body(Body::from("this is not json"))
            .expect("request builds"),
    )
    .await;
    assert!(response.status().is_client_error());

    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("bounded body");
    let problem: serde_json::Value = serde_json::from_slice(&body).expect("problem+json body");
    assert_eq!(problem["request_id"], "client-correlation-0001");
    assert!(
        capture
            .text()
            .contains("request_id=client-correlation-0001"),
        "{}",
        capture.text()
    );
}

#[tokio::test]
async fn server_minted_request_id_is_shared_by_the_log_and_the_error_body() {
    let (capture, _guard) = capture_tracing();

    // No client id: the server mints one. The id in the error body must be
    // the one the span recorded, not a second independently minted value.
    let response = call(
        Request::builder()
            .method("POST")
            .uri("/v1/organizations")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("this is not json"))
            .expect("request builds"),
    )
    .await;
    assert!(response.status().is_client_error());

    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("bounded body");
    let problem: serde_json::Value = serde_json::from_slice(&body).expect("problem+json body");
    let request_id = problem["request_id"].as_str().expect("request_id string");
    assert!(request_id.starts_with("req_"), "{request_id}");
    assert!(
        capture.text().contains(&format!("request_id={request_id}")),
        "{}",
        capture.text()
    );
}

const VALID_TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

fn live_request_with_traceparents(values: &[&str]) -> Request<Body> {
    let mut builder = Request::builder().uri("/process/live");
    for value in values {
        builder = builder.header("traceparent", *value);
    }
    builder.body(Body::empty()).expect("request builds")
}

#[tokio::test]
async fn valid_traceparent_is_recorded_on_the_request_span() {
    let (capture, _guard) = capture_tracing();

    let response = call(live_request_with_traceparents(&[VALID_TRACEPARENT])).await;
    assert_eq!(response.status(), StatusCode::OK);

    let log = capture.text();
    assert!(log.contains(VALID_TRACEPARENT), "{log}");
    assert!(log.contains("http.request.completed"), "{log}");
}

#[tokio::test]
async fn invalid_traceparent_values_are_ignored_not_echoed_into_telemetry() {
    // Each is rejected by `TraceParent::parse` (finding F-006): all-zero
    // trace id, reserved version, non-hex digits, wrong field length.
    let rejected = [
        "00-00000000000000000000000000000000-00f067aa0ba902b7-01",
        "ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        "00-4bf92f3577b34da6a3ce929d0e0e47zz-00f067aa0ba902b7-01",
        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b-01",
    ];
    for value in rejected {
        let (capture, _guard) = capture_tracing();
        let response = call(live_request_with_traceparents(&[value])).await;
        // A bad diagnostic header must never affect the request itself.
        assert_eq!(response.status(), StatusCode::OK, "{value}");

        let log = capture.text();
        assert!(log.contains("http.request.completed"), "{value}: {log}");
        assert!(!log.contains(value), "{value} leaked into telemetry: {log}");
    }
}

#[tokio::test]
async fn multiple_traceparent_headers_are_discarded() {
    let (capture, _guard) = capture_tracing();

    // Two individually valid headers: the spec forbids choosing one.
    let other = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let response = call(live_request_with_traceparents(&[VALID_TRACEPARENT, other])).await;
    assert_eq!(response.status(), StatusCode::OK);

    let log = capture.text();
    assert!(!log.contains(VALID_TRACEPARENT), "{log}");
    assert!(!log.contains(other), "{log}");
}
