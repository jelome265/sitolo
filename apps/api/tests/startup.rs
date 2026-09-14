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
use sitolo_api_bin::serve::serve;
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

    // Live serving: ephemeral port, real socket, bounded probes.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let serve = tokio::spawn(serve(listener, Arc::clone(dev.state()), shutdown_rx));

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
