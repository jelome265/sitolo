//! Startup integration tests (audit F-001/F-020, §21).
//!
//! These execute the real [`sitolo_api_bin::bootstrap::StartupContext`]
//! composition path: valid config boots and serves probes, production
//! without a managed provider fails closed, and shutdown is ordered.
//! A stub provider stands in for the future managed adapter, so no test
//! mutates process-global environment.

use std::sync::Arc;
use std::time::Duration;

use sitolo_api_bin::bootstrap::{Readiness, StartupContext, StartupError};
use sitolo_api_bin::serve::{HttpTransportConfig, serve};
use sitolo_api_bin::shutdown::Subsystem;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

const PROBE_SECRET: &str = "TEST_ONLY_STARTUP_PROBE_001";

fn pair(key: &str, value: &str) -> (String, String) {
    (format!("SITOLO__{key}"), value.to_string())
}

/// Test double for the future managed secret adapter.
struct StubProvider {
    value: String,
}

#[async_trait::async_trait]
impl sitolo_security::SecretProvider for StubProvider {
    async fn get(
        &self,
        _reference: &sitolo_security::SecretRef,
    ) -> Result<sitolo_security::SecretValue, sitolo_security::SecretError> {
        Ok(sitolo_security::SecretValue::new(self.value.clone()))
    }
}

fn stub() -> Option<Arc<dyn sitolo_security::SecretProvider>> {
    Some(Arc::new(StubProvider {
        value: PROBE_SECRET.to_string(),
    }))
}

async fn probe_once(addr: std::net::SocketAddr, path: &str) -> String {
    let mut stream =
        tokio::time::timeout(Duration::from_secs(5), tokio::net::TcpStream::connect(addr))
            .await
            .expect("probe connects")
            .expect("probe connects");
    let request = format!("GET {path} HTTP/1.1\r\nHost: probe\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("probe writes");
    let mut body = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut body))
        .await
        .expect("probe reads")
        .expect("probe reads");
    String::from_utf8_lossy(&body).into_owned()
}

#[tokio::test]
async fn development_boots_and_serves_bounded_probes() {
    let dev =
        StartupContext::build_from_pairs_with_provider(Vec::<(String, String)>::new(), stub())
            .await
            .expect("development bootstrap succeeds");
    assert_eq!(dev.readiness(), Readiness::Ready);
    assert!(dev.state().config_fingerprint().starts_with("sha256:"));

    // Capture real sink output. The server tasks run on this test's
    // single-threaded runtime, so this scoped subscriber observes them.
    let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let writer = std::sync::Arc::clone(&sink);
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_writer(move || SinkWriter(std::sync::Arc::clone(&writer)))
            .with_ansi(false)
            .finish(),
    );

    // Live serving: ephemeral port, real socket, bounded probes.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let serve = tokio::spawn(serve(
        listener,
        Arc::clone(dev.state()),
        shutdown_rx,
        HttpTransportConfig::from_config(dev.config()),
    ));

    let live = probe_once(addr, "/process/live").await;
    assert!(live.contains("200 OK"), "unexpected live response: {live}");
    assert!(live.contains("\"status\":\"live\""));
    assert!(!live.contains(PROBE_SECRET), "probe leaked secret material");
    let ready = probe_once(addr, "/process/ready").await;
    assert!(
        ready.contains("200 OK"),
        "unexpected ready response: {ready}"
    );
    let missing = probe_once(addr, "/no/such/path").await;
    assert!(
        missing.contains("404"),
        "unexpected unknown-path response: {missing}"
    );

    let _ = shutdown_tx.send(());
    let order = tokio::time::timeout(Duration::from_secs(15), serve)
        .await
        .expect("serve shuts down")
        .expect("serve joins");
    assert_eq!(
        order,
        &[
            Subsystem::Listener,
            Subsystem::Telemetry,
            Subsystem::PersistenceIntent,
        ]
    );

    // End to end through the real serving path: each request was offered to
    // the bounded exporter by the Axum middleware and emitted by the drain
    // loop or the final shutdown flush, and nothing admitted is left queued.
    assert_eq!(dev.state().telemetry_exporter().snapshot().total_queued(), 0);
    let log = String::from_utf8_lossy(&sink.lock().expect("sink lock")).into_owned();
    assert_eq!(log.matches("http.request.completed").count(), 3, "{log}");
    assert!(log.contains("route_template=/process/live"), "{log}");
    assert!(log.contains("route_template=/process/ready"), "{log}");
    assert!(log.contains("route_template=unmatched"), "{log}");
    assert!(log.contains("status_class=4xx"), "{log}");
    // The unknown path is reported as `unmatched`, never as the raw path.
    assert!(!log.contains("/no/such/path"), "{log}");
}

/// Writer handing formatter output to a shared buffer.
struct SinkWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for SinkWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("sink lock").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn ready_reports_unavailable_while_draining_and_shutdown_still_completes() {
    // Proof for Stage 06 audit finding P0 #1 ("/process/ready is not
    // genuine readiness") and P0 #3 ("shutdown is bounded but not
    // protocol-graceful"): readiness must flip to unavailable at the
    // moment shutdown is signaled, not merely after the process has
    // finished exiting, and the drain must still terminate.
    let dev =
        StartupContext::build_from_pairs_with_provider(Vec::<(String, String)>::new(), stub())
            .await
            .expect("development bootstrap succeeds");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let state = Arc::clone(dev.state());
    let serve = tokio::spawn(serve(
        listener,
        Arc::clone(&state),
        shutdown_rx,
        HttpTransportConfig::from_config(dev.config()),
    ));

    let ready_before = probe_once(addr, "/process/ready").await;
    assert!(
        ready_before.contains("200 OK"),
        "unexpected pre-shutdown ready response: {ready_before}"
    );
    assert!(!state.readiness().is_draining());

    let _ = shutdown_tx.send(());

    // Readiness must flip the instant shutdown is signaled, not merely
    // once the process has finished exiting. Checked directly against the
    // shared state handle rather than by racing a fresh HTTP connection
    // against it: the production listener is intentionally dropped as
    // soon as this process stops accepting (see `serve()`), so a new
    // connection attempt during the drain window is expected to fail fast
    // rather than be served — proven separately below — and is not a
    // reliable way to observe this specific state transition's timing.
    let became_draining = tokio::time::timeout(Duration::from_secs(1), async {
        while !state.readiness().is_draining() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(
        became_draining.is_ok(),
        "readiness did not flip to draining promptly after the shutdown signal"
    );

    // A connection attempt during the drain window must fail fast
    // (the listener is dropped, not merely un-accepted) rather than hang:
    // that is the real bug this test originally caught (see remediation
    // notes) — new connections succeeding at the TCP layer and then
    // waiting forever for a response that would never come.
    let refused =
        tokio::time::timeout(Duration::from_secs(5), tokio::net::TcpStream::connect(addr))
            .await
            .expect("a connection attempt during drain must resolve quickly, not hang");
    assert!(
        refused.is_err(),
        "expected the drained listener to refuse new connections, got a live socket"
    );

    let order = tokio::time::timeout(Duration::from_secs(15), serve)
        .await
        .expect("serve shuts down")
        .expect("serve joins");
    assert_eq!(
        order,
        &[
            Subsystem::Listener,
            Subsystem::Telemetry,
            Subsystem::PersistenceIntent,
        ]
    );
}

#[tokio::test]
async fn staging_boots_with_explicit_database_identity() {
    let staging = StartupContext::build_from_pairs_with_provider(
        [
            pair("RUNTIME__ENVIRONMENT", "staging"),
            pair("DATABASE__HOST", "db.internal"),
            pair("DATABASE__PORT", "5432"),
            pair("DATABASE__NAME", "sitolo"),
            pair("DATABASE__USER", "sitolo_api"),
            pair("DATABASE__PASSWORD_REF", "staging/sitolo/db"),
        ],
        stub(),
    )
    .await
    .expect("staging bootstrap succeeds");
    assert_eq!(staging.readiness(), Readiness::Ready);
}

#[tokio::test]
async fn production_fails_closed_without_managed_provider() {
    // Validation passes, but no managed secret adapter exists yet, so
    // startup refuses instead of using the local provider. No listener is
    // ever bound on this path.
    let production = StartupContext::build_from_pairs([
        pair("RUNTIME__ENVIRONMENT", "production"),
        pair("DATABASE__HOST", "db.internal"),
        pair("DATABASE__PORT", "5432"),
        pair("DATABASE__NAME", "sitolo"),
        pair("DATABASE__USER", "sitolo_api"),
        pair("DATABASE__PASSWORD_REF", "production/sitolo/db"),
        pair("SECRETS__ALLOW_LOCAL_PROVIDER", "false"),
    ])
    .await;
    assert!(
        matches!(production, Err(StartupError::SecretProviderUnavailable)),
        "production must fail closed"
    );
}

#[tokio::test]
async fn explicit_provider_is_honored_in_production() {
    // The seam for the future managed adapter: an explicitly supplied
    // provider is used even in production.
    let production = StartupContext::build_from_pairs_with_provider(
        [
            pair("RUNTIME__ENVIRONMENT", "production"),
            pair("DATABASE__HOST", "db.internal"),
            pair("DATABASE__PORT", "5432"),
            pair("DATABASE__NAME", "sitolo"),
            pair("DATABASE__USER", "sitolo_api"),
            pair("DATABASE__PASSWORD_REF", "production/sitolo/db"),
            pair("SECRETS__ALLOW_LOCAL_PROVIDER", "false"),
        ],
        stub(),
    )
    .await
    .expect("explicit provider boots production");
    assert_eq!(production.readiness(), Readiness::Ready);
}
