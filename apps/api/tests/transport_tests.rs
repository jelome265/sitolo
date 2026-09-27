//! Real-socket transport tests for the PR-68 remediation (Stage 07).
//!
//! Every test here spawns the actual production `serve()` future against a
//! real ephemeral TCP listener and talks HTTP/1 (or raw HTTP/2 preface
//! bytes) over a real socket. None of these assert on a placeholder; each
//! one proves a specific transport-policy claim from the Stage 06 audit.

use std::sync::Arc;
use std::time::Duration;

use sitolo_api_bin::bootstrap::StartupContext;
use sitolo_api_bin::serve::{HttpTransportConfig, serve};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

const PROBE_SECRET: &str = "TEST_ONLY_TRANSPORT_PROBE_001";

fn stub() -> Option<Arc<dyn sitolo_security::SecretProvider>> {
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

    Some(Arc::new(StubProvider {
        value: PROBE_SECRET.to_string(),
    }))
}

/// Boots a real `StartupContext` and spawns `serve()` on an ephemeral port
/// with the given transport overrides, returning the address, a shutdown
/// sender, and the `JoinHandle` so callers can trigger and observe an
/// orderly shutdown.
async fn spawn_server(
    transport: HttpTransportConfig,
) -> (
    std::net::SocketAddr,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<Vec<sitolo_api_bin::shutdown::Subsystem>>,
) {
    let dev = StartupContext::build_from_pairs_with_provider(Vec::<(String, String)>::new(), stub())
        .await
        .expect("development bootstrap succeeds");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let handle = tokio::spawn(serve(listener, Arc::clone(dev.state()), shutdown_rx, transport));
    (addr, shutdown_tx, handle)
}

async fn connect(addr: std::net::SocketAddr) -> TcpStream {
    tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(addr))
        .await
        .expect("connect does not hang")
        .expect("connect succeeds")
}

/// Reads exactly one HTTP/1 response (status line + headers + body) off a
/// socket, honoring `Content-Length` so a persistent connection's next
/// response is not consumed by mistake.
async fn read_one_http_response(stream: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut chunk))
            .await
            .expect("read does not hang")
            .expect("read succeeds");
        assert!(n > 0, "connection closed before a full response arrived");
        buf.extend_from_slice(&chunk[..n]);

        let text = String::from_utf8_lossy(&buf);
        let Some(header_end) = text.find("\r\n\r\n") else {
            continue;
        };
        let headers = &text[..header_end];
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .starts_with("content-length:")
                    .then(|| line["content-length:".len()..].trim().parse::<usize>().unwrap_or(0))
            })
            .unwrap_or(0);
        let body_so_far = buf.len() - (header_end + 4);
        if body_so_far >= content_length {
            return text.into_owned();
        }
    }
}

fn base_transport() -> HttpTransportConfig {
    HttpTransportConfig {
        max_request_body_bytes: 2_097_152,
        request_header_timeout: Duration::from_millis(5_000),
        keepalive_timeout: Duration::from_millis(30_000),
        request_timeout: Duration::from_millis(30_000),
        request_body_idle_timeout: Duration::from_millis(5_000),
        http1_idle_timeout: Duration::from_millis(60_000),
        http2_ping_interval: Duration::from_millis(30_000),
        http2_keep_alive_timeout: Duration::from_millis(10_000),
        response_body_timeout: Duration::from_millis(30_000),
    }
}

/// Stage 06 audit P0 #4: the only prior real-socket test always sent
/// `Connection: close`. This reuses one socket for two full request/
/// response cycles, which is the behavior actually relied upon in
/// production.
#[tokio::test]
async fn real_socket_http1_persistent_connection() {
    let (addr, shutdown_tx, handle) = spawn_server(base_transport()).await;
    let mut stream = connect(addr).await;

    stream
        .write_all(b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .expect("first request writes");
    let first = read_one_http_response(&mut stream).await;
    assert!(first.contains("200 OK"), "unexpected first response: {first}");
    assert!(
        !first.to_ascii_lowercase().contains("connection: close"),
        "server must not close a persistent connection after one request: {first}"
    );

    stream
        .write_all(b"GET /process/ready HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .expect("second request writes");
    let second = tokio::time::timeout(Duration::from_secs(5), async {
        let mut body = Vec::new();
        stream.read_to_end(&mut body).await.expect("second request reads");
        String::from_utf8_lossy(&body).into_owned()
    })
    .await
    .expect("second request on the same socket must complete without hanging");
    assert!(second.contains("200 OK"), "unexpected second response: {second}");

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(15), handle).await;
}

/// Proves the HTTP/1 header-read timeout actually bounds a slow-header
/// client: a connection that never finishes sending its request headers is
/// closed by the server rather than held open indefinitely. Uses a short
/// configured timeout so the test is fast and deterministic rather than
/// racing the compiled 5s default.
#[tokio::test]
async fn real_socket_slow_header_is_bounded() {
    let mut transport = base_transport();
    transport.request_header_timeout = Duration::from_millis(200);
    let (addr, shutdown_tx, handle) = spawn_server(transport).await;
    let mut stream = connect(addr).await;

    // Send a request line but withhold the header terminator.
    stream
        .write_all(b"GET /process/live HTTP/1.1\r\n")
        .await
        .expect("partial request writes");

    // The server must close the connection once `request_header_timeout`
    // elapses, without ever completing the headers. A bounded read that
    // observes EOF (0 bytes) within well under the drain deadline proves
    // the timeout is real, not merely configured and ignored.
    let outcome = tokio::time::timeout(Duration::from_secs(5), async {
        let mut buf = [0u8; 256];
        stream.read(&mut buf).await
    })
    .await
    .expect("server must close the slow-header connection, not hang forever");
    match outcome {
        Ok(0) => {}
        Ok(n) => panic!("expected EOF from header-timeout close, got {n} bytes"),
        Err(error) => panic!("unexpected read error: {error}"),
    }

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(15), handle).await;
}

/// D-1 closure: proves `http1_idle_timeout` actually evicts a connection
/// that has gone application-idle after successfully completing a request,
/// independent of TCP keepalive (which never fires here — the peer socket
/// stays healthy the whole time).
#[tokio::test]
async fn real_socket_idle_connection_is_evicted_after_configured_timeout() {
    let mut transport = base_transport();
    transport.http1_idle_timeout = Duration::from_millis(200);
    let (addr, shutdown_tx, handle) = spawn_server(transport).await;
    let mut stream = connect(addr).await;

    stream
        .write_all(b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .expect("request writes");
    let response = read_one_http_response(&mut stream).await;
    assert!(response.contains("200 OK"), "unexpected response: {response}");

    // Now go idle: send nothing further. The server must close this
    // connection on its own once `http1_idle_timeout` elapses.
    let outcome = tokio::time::timeout(Duration::from_secs(5), async {
        let mut buf = [0u8; 256];
        stream.read(&mut buf).await
    })
    .await
    .expect("server must evict the idle connection, not hold it open indefinitely");
    match outcome {
        Ok(0) => {}
        Ok(n) => panic!("expected EOF from idle eviction, got {n} bytes"),
        Err(error) => panic!("unexpected read error: {error}"),
    }

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(15), handle).await;
}

/// Counterpart to the idle-eviction test above: a connection that keeps
/// making requests well inside `http1_idle_timeout` between each one must
/// never be evicted, proving the deadline resets on activity rather than
/// firing on a fixed timer from connection open.
#[tokio::test]
async fn real_socket_active_connection_survives_past_idle_timeout() {
    let mut transport = base_transport();
    transport.http1_idle_timeout = Duration::from_millis(300);
    let (addr, shutdown_tx, handle) = spawn_server(transport).await;
    let mut stream = connect(addr).await;

    // Three requests, each well within the idle window, spanning more than
    // twice the configured idle timeout in total. If eviction were a bare
    // timer from connection open rather than an activity-reset deadline,
    // this connection would already be dead before the third request.
    for _ in 0..3 {
        stream
            .write_all(b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .expect("request writes");
        let response = read_one_http_response(&mut stream).await;
        assert!(response.contains("200 OK"), "unexpected response: {response}");
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(15), handle).await;
}

/// Proves oversized JSON request bodies are classified 413, not merely
/// rejected some other way, consistent across the transport-level body
/// limit (this exercises `RequestBodyLimitLayer`, upstream of any
/// per-handler `JsonRejection` classification).
#[tokio::test]
async fn real_socket_oversized_body_is_413() {
    let mut transport = base_transport();
    transport.max_request_body_bytes = 16;
    let (addr, shutdown_tx, handle) = spawn_server(transport).await;
    let mut stream = connect(addr).await;

    let body = "{\"padding\":\"this body is deliberately larger than the configured 16-byte limit\"}";
    let request = format!(
        "POST /v1/organizations HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(request.as_bytes()).await.expect("request writes");

    let response = tokio::time::timeout(Duration::from_secs(5), async {
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.expect("read succeeds");
        String::from_utf8_lossy(&buf).into_owned()
    })
    .await
    .expect("oversized-body request must not hang");
    assert!(response.contains("413"), "expected 413, got: {response}");

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(15), handle).await;
}

/// Proves `request_body_idle_timeout` bounds a body that stalls mid-stream
/// (headers complete, `Content-Length` promised, but the client never
/// finishes sending it) rather than holding the connection open forever.
#[tokio::test]
async fn real_socket_stalled_body_is_bounded() {
    let mut transport = base_transport();
    transport.request_body_idle_timeout = Duration::from_millis(200);
    let (addr, shutdown_tx, handle) = spawn_server(transport).await;
    let mut stream = connect(addr).await;

    stream
        .write_all(
            b"POST /v1/organizations HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 4096\r\n\r\n",
        )
        .await
        .expect("headers write");
    // Promise 4096 bytes of body, send none, and never send more.

    let outcome = tokio::time::timeout(Duration::from_secs(5), async {
        let mut buf = [0u8; 4096];
        stream.read(&mut buf).await
    })
    .await
    .expect("server must bound the stalled body, not hang forever");
    // Either the server sends a timeout/error response before closing, or
    // it closes without responding; both are acceptable terminal outcomes
    // for a stalled body. What matters, and what this test proves, is that
    // it terminates within the bound above instead of hanging.
    match outcome {
        Ok(0) => {}
        Ok(_) => {}
        Err(error) => panic!("unexpected read error: {error}"),
    }

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(15), handle).await;
}

/// Minimal HTTP/2 prior-knowledge (h2c) smoke test: `hyper_util`'s
/// `auto::Builder` detects the HTTP/2 connection preface on a plaintext
/// socket without ALPN/TLS. Sending the preface plus a valid empty SETTINGS
/// frame and observing a SETTINGS frame back proves the server actually
/// negotiates HTTP/2 on this listener, rather than only ever speaking
/// HTTP/1. A full request/response cycle over h2 would require a real h2
/// client stack, which is not available as a dependency in this crate; the
/// preface/SETTINGS exchange is the largest genuine claim provable without
/// adding one.
#[tokio::test]
async fn real_socket_http2_preface_is_recognized() {
    let (addr, shutdown_tx, handle) = spawn_server(base_transport()).await;
    let mut stream = connect(addr).await;

    // RFC 9113 §3.4 connection preface, followed by an empty SETTINGS
    // frame (9-byte header: length=0, type=0x4, flags=0, stream id=0).
    let mut preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n".to_vec();
    preface.extend_from_slice(&[0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00]);
    stream.write_all(&preface).await.expect("preface writes");

    let frame_header = tokio::time::timeout(Duration::from_secs(5), async {
        let mut buf = [0u8; 9];
        stream.read_exact(&mut buf).await.expect("reads a full frame header");
        buf
    })
    .await
    .expect("server must respond to the HTTP/2 preface, not hang");

    // Byte 3 of an HTTP/2 frame header is the frame type; 0x04 is SETTINGS.
    // A compliant HTTP/2 server responds to the client preface with its own
    // SETTINGS frame before anything else.
    assert_eq!(
        frame_header[3], 0x04,
        "expected a SETTINGS frame in response to the h2c preface, got frame type {:#x}",
        frame_header[3]
    );

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(15), handle).await;
}

/// Proves an already-established persistent connection is drained rather
/// than abruptly severed when shutdown begins: a request completed before
/// shutdown, followed by shutdown being signaled, must still leave the
/// connection in a well-defined terminal state (a further request is either
/// answered or the socket is closed cleanly) within a bounded time, never
/// hanging.
#[tokio::test]
async fn real_socket_open_connection_drains_cleanly_on_shutdown() {
    let (addr, shutdown_tx, handle) = spawn_server(base_transport()).await;
    let mut stream = connect(addr).await;

    stream
        .write_all(b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .expect("request writes");
    let response = read_one_http_response(&mut stream).await;
    assert!(response.contains("200 OK"), "unexpected response: {response}");

    let _ = shutdown_tx.send(());

    // A further request on the already-open socket must reach a
    // deterministic terminal state (a response, or a clean close) within a
    // bound well inside the drain deadline, never hanging past it.
    let _ = stream
        .write_all(b"GET /process/live HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await;
    let terminal = tokio::time::timeout(Duration::from_secs(8), async {
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf).await;
        buf
    })
    .await
    .expect("connection must reach a terminal state before the drain deadline");
    if !terminal.is_empty() {
        let text = String::from_utf8_lossy(&terminal);
        assert!(
            text.contains("200") || text.contains("503") || text.contains("HTTP/1.1"),
            "unexpected bytes on drain: {text}"
        );
    }

    let order = tokio::time::timeout(Duration::from_secs(15), handle)
        .await
        .expect("serve shuts down")
        .expect("serve joins");
    assert_eq!(
        order,
        &[
            sitolo_api_bin::shutdown::Subsystem::Listener,
            sitolo_api_bin::shutdown::Subsystem::Telemetry,
            sitolo_api_bin::shutdown::Subsystem::PersistenceIntent,
        ]
    );
}
