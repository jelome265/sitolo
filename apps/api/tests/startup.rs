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

fn stub() -> Arc<dyn sitolo_security::SecretProvider> {
    Arc::new(StubProvider {
        value: PROBE_SECRET.to_string(),
    })
}

async fn probe_once(
    addr: std::net::SocketAddr,
    path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream =
        tokio::time::timeout(Duration::from_secs(5), tokio::net::TcpStream::connect(addr))
            .await??;
    let request = format!("GET {path} HTTP/1.1\r\nHost: probe\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await?;
    let mut body = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut body)).await??;
    Ok(String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
#[expect(
    clippy::disallowed_methods,
    reason = "FW-TOK-03: the live serve task is owned by the test and awaited after the bounded probes."
)]
async fn development_boots_and_serves_bounded_probes() -> Result<(), Box<dyn std::error::Error>> {
    let dev = StartupContext::build_from_pairs_with_provider(
        Vec::<(String, String)>::new(),
        Some(stub()),
    )
    .await?;
    assert_eq!(dev.readiness(), Readiness::NotReady);
    assert!(dev.state().config_fingerprint().starts_with("sha256:"));

    // Live serving: ephemeral port, real socket, bounded probes.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let serve = tokio::spawn(serve(listener, Arc::clone(dev.state()), shutdown_rx));

    let live = probe_once(addr, "/process/live").await?;
    assert!(live.contains("200 OK"), "unexpected live response: {live}");
    assert!(live.contains("\"status\":\"live\""));
    assert!(!live.contains(PROBE_SECRET), "probe leaked secret material");
    let ready = probe_once(addr, "/process/ready").await?;
    assert!(
        ready.contains("503 Service Unavailable"),
        "unexpected ready response: {ready}"
    );
    let missing = probe_once(addr, "/no/such/path").await?;
    assert!(
        missing.contains("404"),
        "unexpected unknown-path response: {missing}"
    );

    let _sent = shutdown_tx.send(());
    let order = tokio::time::timeout(Duration::from_secs(15), serve).await??;
    let order = order?;
    assert_eq!(
        order,
        &[
            Subsystem::Listener,
            Subsystem::Telemetry,
            Subsystem::PersistenceIntent,
        ]
    );
    Ok(())
}

#[tokio::test]
async fn staging_boots_with_explicit_database_identity() -> Result<(), Box<dyn std::error::Error>> {
    let staging = StartupContext::build_from_pairs_with_provider(
        [
            pair("RUNTIME__ENVIRONMENT", "staging"),
            pair("DATABASE__HOST", "db.internal"),
            pair("DATABASE__PORT", "5432"),
            pair("DATABASE__NAME", "sitolo"),
            pair("DATABASE__SCHEMA", "sitolo_app"),
            pair("DATABASE__USER", "app_runtime"),
            pair("DATABASE__PASSWORD_REF", "staging/sitolo/db"),
            pair(
                "IDENTITY__OIDC__ISSUER",
                "https://cognito-idp.af-south-1.amazonaws.com/pool",
            ),
            pair("IDENTITY__OIDC__CLIENT_ID", "client-id"),
            pair(
                "IDENTITY__OIDC__TOKEN_ENDPOINT",
                "https://auth.example.com/oauth2/token",
            ),
            pair(
                "IDENTITY__OIDC__REDIRECT_URI",
                "https://app.example.com/callback",
            ),
        ],
        Some(stub()),
    )
    .await?;
    assert_eq!(staging.readiness(), Readiness::NotReady);
    Ok(())
}

#[tokio::test]
async fn injected_production_bootstrap_without_provider_fails_closed()
-> Result<(), Box<dyn std::error::Error>> {
    // Injected startup deliberately does not discover ECS task credentials.
    // Without an explicitly supplied provider it refuses rather than using
    // the local provider. No listener is ever bound on this path.
    let production = StartupContext::build_from_pairs([
        pair("RUNTIME__ENVIRONMENT", "production"),
        pair("DATABASE__HOST", "db.internal"),
        pair("DATABASE__PORT", "5432"),
        pair("DATABASE__NAME", "sitolo"),
        pair("DATABASE__SCHEMA", "sitolo_app"),
        pair("DATABASE__USER", "app_runtime"),
        pair("DATABASE__PASSWORD_REF", "production/sitolo/db"),
        pair(
            "IDENTITY__OIDC__ISSUER",
            "https://cognito-idp.af-south-1.amazonaws.com/pool",
        ),
        pair("IDENTITY__OIDC__CLIENT_ID", "client-id"),
        pair(
            "IDENTITY__OIDC__TOKEN_ENDPOINT",
            "https://auth.example.com/oauth2/token",
        ),
        pair(
            "IDENTITY__OIDC__REDIRECT_URI",
            "https://app.example.com/callback",
        ),
        pair("SECRETS__ALLOW_LOCAL_PROVIDER", "false"),
    ])
    .await;
    assert!(
        matches!(production, Err(StartupError::SecretProviderUnavailable)),
        "production must fail closed"
    );
    Ok(())
}

#[tokio::test]
async fn explicit_provider_is_honored_in_production() -> Result<(), Box<dyn std::error::Error>> {
    // The explicit injection seam is used by tests and controlled composition;
    // the process entry point constructs the ECS-backed provider itself.
    let production = StartupContext::build_from_pairs_with_provider(
        [
            pair("RUNTIME__ENVIRONMENT", "production"),
            pair("DATABASE__HOST", "db.internal"),
            pair("DATABASE__PORT", "5432"),
            pair("DATABASE__NAME", "sitolo"),
            pair("DATABASE__SCHEMA", "sitolo_app"),
            pair("DATABASE__USER", "app_runtime"),
            pair("DATABASE__PASSWORD_REF", "production/sitolo/db"),
            pair(
                "IDENTITY__OIDC__ISSUER",
                "https://cognito-idp.af-south-1.amazonaws.com/pool",
            ),
            pair("IDENTITY__OIDC__CLIENT_ID", "client-id"),
            pair(
                "IDENTITY__OIDC__TOKEN_ENDPOINT",
                "https://auth.example.com/oauth2/token",
            ),
            pair(
                "IDENTITY__OIDC__REDIRECT_URI",
                "https://app.example.com/callback",
            ),
            pair("SECRETS__ALLOW_LOCAL_PROVIDER", "false"),
        ],
        Some(stub()),
    )
    .await?;
    assert_eq!(production.readiness(), Readiness::NotReady);
    Ok(())
}
