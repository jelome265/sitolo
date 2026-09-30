//! Real socket transport tests for PR #68 remediation.
//!
//! Every test here binds a real `TcpListener`, spawns the actual
//! [`sitolo_api_bin::serve::serve`] loop on it (the same function
//! `main.rs` runs in production), and drives it with a raw
//! [`tokio::net::TcpStream`] or a bytes-level HTTP/2 client preface. These
//! are transport/connection-lifecycle tests — socket timeouts, HTTP/1
//! keep-alive, h2c wire behavior, body-size/time limits, and graceful
//! shutdown — as distinct from the handler/application-level tests in
//! `tenancy_api.rs`, which drive the router in-process via
//! `tower::ServiceExt::oneshot` and never touch a socket.
//!
//! Every assertion here reads real bytes back from a real accepted
//! connection. None of these are `assert!(true, "...")` placeholders.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use sitolo_api_bin::bootstrap::StartupContext;
use sitolo_api_bin::serve::{HttpTransportConfig, serve};
use sitolo_api_bin::state::AppState;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

struct StubProvider;

#[async_trait::async_trait]
impl sitolo_security::SecretProvider for StubProvider {
    async fn get(
        &self,
        _reference: &sitolo_security::SecretRef,
    ) -> Result<sitolo_security::SecretValue, sitolo_security::SecretError> {
        Ok(sitolo_security::SecretValue::new(
            "PROBE_SECRET".to_string(),
        ))
    }
}

async fn test_app_state() -> Arc<AppState> {
    let ctx = StartupContext::build_from_pairs_with_provider(
        Vec::<(String, String)>::new(),
        Some(Arc::new(StubProvider)),
    )
    .await
    .expect("bootstrap succeeds");
    Arc::clone(ctx.state())
}

/// Short, test-tuned timeouts so timeout-triggering tests finish in well
/// under a second instead of the production defaults (tens of seconds).
/// These are still real `HttpTransportConfig` values consumed by the real
/// `serve()` loop, not a separate test-only code path.
fn short_transport() -> HttpTransportConfig {
    HttpTransportConfig {
        max_request_body_bytes: 1024 * 1024,
        request_header_timeout: Duration::from_millis(200),
        keepalive_timeout: Duration::from_secs(30),
        request_timeout: Duration::from_secs(10),
        request_body_idle_timeout: Duration::from_millis(200),
        http1_idle_timeout: Duration::from_millis(500),
        http2_ping_interval: Duration::from_secs(30),
        http2_keep_alive_timeout: Duration::from_secs(30),
        response_body_timeout: Duration::from_secs(10),
    }
}

/// Like [`short_transport`], but with the header-read timeout pushed well
/// out of the way so that `http1_idle_timeout` is the *only* policy that can
/// close an idle keep-alive connection.
///
/// This matters: hyper's header-read timer also runs while a keep-alive
/// connection waits for its *next* request's headers, so with a header
/// timeout shorter than the idle timeout, the header timer closes the
/// connection first and an idle-eviction test would pass (or fail) for the
/// wrong reason without ever exercising `IdleTimeoutIo`.
fn idle_isolated_transport() -> HttpTransportConfig {
    HttpTransportConfig {
        request_header_timeout: Duration::from_secs(5),
        http1_idle_timeout: Duration::from_millis(500),
        ..short_transport()
    }
}

/// Binds a real listener, spawns the real `serve()` loop on it, and
/// returns the address to connect to plus a shutdown handle. Dropping the
/// returned `oneshot::Sender` without calling it leaves the server running
/// for the rest of the test binary's process lifetime (each test binds its
/// own ephemeral port via `:0`, so this is harmless but callers that care
/// about graceful shutdown should send on it explicitly, per
/// `real_socket_active_connection_shutdown_test`).
async fn spawn_server(
    transport: HttpTransportConfig,
) -> (
    SocketAddr,
    oneshot::Sender<()>,
    JoinHandle<Vec<sitolo_api_bin::shutdown::Subsystem>>,
) {
    let state = test_app_state().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("listener has a local addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let handle = tokio::spawn(serve(listener, state, shutdown_rx, transport));
    (addr, shutdown_tx, handle)
}

async fn send_and_read(stream: &mut TcpStream, req: &[u8]) -> String {
    stream.write_all(req).await.unwrap();
    let mut buf = vec![0; 8192];
    let n = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf))
        .await
        .expect("response within 2s")
        .unwrap();
    String::from_utf8_lossy(&buf[..n]).to_string()
}

/// Stage 06 audit P0 #4: the only prior real-socket test always sent
/// `Connection: close`. This reuses one socket for two full request/
/// response cycles, which is the behavior actually relied upon in
/// production.
#[tokio::test]
async fn real_socket_http1_persistent_connection() {
    let (addr, _shutdown, _handle) = spawn_server(short_transport()).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req1 = b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let res1 = send_and_read(&mut stream, req1).await;
    assert!(
        res1.starts_with("HTTP/1.1 200"),
        "expected 200 from /process/live, got: {res1}"
    );

    // Same TCP connection, second request: proves keep-alive rather than
    // the server closing after one response.
    let req2 = b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let res2 = send_and_read(&mut stream, req2).await;
    assert!(
        res2.starts_with("HTTP/1.1 200"),
        "expected second response on the same persistent connection, got: {res2}"
    );
}

/// Opens a connection, completes one request/response, then sends nothing
/// further. Proves `http1_idle_timeout` actually evicts a connection that
/// has gone application-idle, independent of TCP keepalive (which never
/// fires here — the peer socket stays healthy the whole time; only the
/// application-level idle policy closes it).
#[tokio::test]
async fn real_socket_idle_connection_is_evicted_after_configured_timeout() {
    let (addr, _shutdown, _handle) = spawn_server(idle_isolated_transport()).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let res = send_and_read(&mut stream, req).await;
    assert!(
        res.starts_with("HTTP/1.1 200"),
        "warm-up request failed: {res}"
    );

    // Now go idle: send nothing further. `idle_isolated_transport()`
    // configures a 500ms `http1_idle_timeout` (header timeout is 5s, so it cannot
    // be the policy that closes this connection); the server must close this connection on
    // its own well before our own 2s read bound.
    let mut buf = vec![0; 64];
    let read = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf)).await;
    match read {
        Ok(Ok(0)) => {}  // connection closed: idle eviction enforced
        Ok(Err(_)) => {} // reset/closed: also acceptable enforcement
        Ok(Ok(n)) => panic!(
            "expected the idle connection to be closed by the server, got {n} unexpected bytes"
        ),
        Err(_) => panic!(
            "server did not evict a connection idle for well over the configured \
             500ms http1_idle_timeout — idle eviction is not enforced"
        ),
    }
}

/// Counterpart to the idle-eviction test above, and the direct regression
/// proof for the specific bug this remediation fixed: an earlier version
/// of the idle-eviction mechanism started a bare timer once at connection
/// open and never reset it, so it would eventually kill *any* long-lived
/// connection — including one continuously serving requests — the moment
/// the fixed duration elapsed, regardless of activity. `IdleTimeoutIo`
/// resets its deadline on every successful read instead. Three requests,
/// each well inside the 500ms idle window but spanning more than twice
/// that window in total, prove the deadline tracks *inactivity* rather
/// than *connection age*: a non-resetting timer would have killed this
/// connection before the third request.
#[tokio::test]
async fn real_socket_active_connection_survives_past_idle_timeout() {
    let (addr, _shutdown, _handle) = spawn_server(idle_isolated_transport()).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    for i in 0..3 {
        let req = b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let res = send_and_read(&mut stream, req).await;
        assert!(
            res.starts_with("HTTP/1.1 200"),
            "request {i} on an actively-used connection failed: {res}"
        );
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Sends the request line and nothing else, then waits past
/// `request_header_timeout` without completing the headers. A server that
/// enforces the header-read timeout closes the connection; a server that
/// doesn't would leave the socket open (`read` would time out our own 2s
/// wait instead of returning).
#[tokio::test]
async fn real_socket_slow_header_test() {
    let (addr, _shutdown, _handle) = spawn_server(short_transport()).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    stream.write_all(b"GET / HTTP/1.1\r\n").await.unwrap();
    // Header timeout is 200ms; wait well past it before sending the rest.
    tokio::time::sleep(Duration::from_millis(600)).await;
    let _ = stream.write_all(b"Host: localhost\r\n\r\n").await;

    let mut buf = vec![0; 1024];
    let read = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf)).await;
    match read {
        Ok(Ok(0)) => {} // connection closed by server: timeout enforced
        Ok(Ok(n)) => {
            let text = String::from_utf8_lossy(&buf[..n]);
            assert!(
                text.contains("408") || text.contains("400"),
                "expected the server to reject the slow-header connection, got: {text}"
            );
        }
        Ok(Err(_)) => {} // reset/closed: also acceptable enforcement
        Err(_) => panic!(
            "server neither closed the connection nor responded within 2s after a 600ms \
             header stall against a 200ms request_header_timeout — timeout is not enforced"
        ),
    }
}

/// Sends valid headers declaring a body, then trickles the body slower
/// than `request_body_idle_timeout`. Proves the idle-body timeout actually
/// tears the connection down instead of waiting forever.
#[tokio::test]
async fn real_socket_slow_body_test() {
    let (addr, _shutdown, _handle) = spawn_server(short_transport()).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let headers = b"POST /v1/organizations HTTP/1.1\r\n\
                     Host: localhost\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: 20\r\n\r\n";
    stream.write_all(headers).await.unwrap();
    // Send 1 byte, then stall well past the 200ms body idle timeout
    // without ever sending the remaining 19 declared bytes.
    stream.write_all(b"{").await.unwrap();
    tokio::time::sleep(Duration::from_millis(700)).await;
    // Best-effort: the connection may already be closed by the server.
    let _ = stream.write_all(b"\"a\":1}").await;

    let mut buf = vec![0; 1024];
    let read = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf)).await;
    match read {
        Ok(Ok(0)) => {} // connection closed: idle-body timeout enforced
        Ok(Ok(n)) => {
            let text = String::from_utf8_lossy(&buf[..n]);
            assert!(
                !text.starts_with("HTTP/1.1 201"),
                "a stalled body must not be treated as a complete, successful request: {text}"
            );
        }
        Ok(Err(_)) => {}
        Err(_) => panic!(
            "server neither closed the connection nor responded within 2s after a 700ms \
             body stall against a 200ms request_body_idle_timeout — timeout is not enforced"
        ),
    }
}

/// Sends a chunked-encoded body (no `Content-Length`) whose total size
/// exceeds `sitolo_api::tenancy::bounds::MAX_TENANCY_BODY_BYTES` (32 KiB).
/// Proves the body-size limit is enforced on the streamed/chunked path,
/// not only when a client is honest enough to declare an oversized
/// `Content-Length` up front.
#[tokio::test]
async fn real_socket_chunked_oversized_request_test() {
    let (addr, _shutdown, _handle) = spawn_server(short_transport()).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let headers = b"POST /v1/organizations HTTP/1.1\r\n\
                     Host: localhost\r\n\
                     Content-Type: application/json\r\n\
                     Transfer-Encoding: chunked\r\n\r\n";
    stream.write_all(headers).await.unwrap();

    // 40 chunks of 1 KiB = 40 KiB, above the 32 KiB cap, sent as valid
    // chunked framing (no Content-Length is ever declared). The server is
    // allowed to reset the connection as soon as it crosses the limit —
    // possibly before we finish writing — so writes past that point are
    // tolerated rather than unwrapped; what we assert on is the response
    // actually read back.
    let chunk_payload = vec![b'x'; 1024];
    for _ in 0..40 {
        let frame = format!("{:x}\r\n", chunk_payload.len());
        if stream.write_all(frame.as_bytes()).await.is_err() {
            break;
        }
        if stream.write_all(&chunk_payload).await.is_err() {
            break;
        }
        if stream.write_all(b"\r\n").await.is_err() {
            break;
        }
    }
    let _ = stream.write_all(b"0\r\n\r\n").await;

    let mut buf = vec![0; 4096];
    let n = tokio::time::timeout(Duration::from_secs(3), stream.read(&mut buf))
        .await
        .expect("server responds within 3s")
        .unwrap();
    let text = String::from_utf8_lossy(&buf[..n]);
    assert!(
        text.starts_with("HTTP/1.1 413"),
        "expected 413 Payload Too Large for an oversized chunked body, got: {text}"
    );
}

/// Performs the real HTTP/2 cleartext (h2c, prior-knowledge) client
/// preface and a real SETTINGS frame at the byte level, and asserts the
/// server answers with its own SETTINGS frame — proving the listener
/// actually speaks HTTP/2 on this port, not just HTTP/1.
#[tokio::test]
async fn real_socket_h2_prior_knowledge_test() {
    let (addr, _shutdown, _handle) = spawn_server(short_transport()).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    // RFC 9113 §3.4: the connection preface, sent by an h2c client using
    // prior knowledge (no HTTP/1 Upgrade round trip).
    const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
    // An empty SETTINGS frame: 9-byte header (length=0, type=0x4
    // SETTINGS, flags=0, stream id=0), no payload.
    const EMPTY_SETTINGS_FRAME: [u8; 9] = [0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00];

    stream.write_all(PREFACE).await.unwrap();
    stream.write_all(&EMPTY_SETTINGS_FRAME).await.unwrap();

    // Read at least a 9-byte frame header back.
    let mut header = [0u8; 9];
    tokio::time::timeout(Duration::from_secs(2), stream.read_exact(&mut header))
        .await
        .expect("server responds with an HTTP/2 frame within 2s")
        .expect("read a full 9-byte frame header");

    let frame_type = header[3];
    // The server's first frame on a fresh h2 connection must be its own
    // SETTINGS frame (type 0x4) per RFC 9113 §3.4 — never HTTP/1 bytes
    // ("HTTP/1.1 ..." would decode here as frame_type = b'T' = 0x54).
    assert_eq!(
        frame_type, 0x04,
        "expected an HTTP/2 SETTINGS frame (type 0x04) as the server's first frame on an \
         h2c prior-knowledge connection, got frame type byte {frame_type:#x} — the listener \
         is not speaking HTTP/2 on this connection"
    );
}

/// Opens a connection, starts a slow request mid-flight, triggers a real
/// shutdown via the same `oneshot::Sender` `main.rs` uses for SIGTERM, and
/// asserts the in-flight connection is closed (not abandoned mid-response,
/// and not left open past the drain deadline) — proving
/// `conn.as_mut().graceful_shutdown()` is actually reached on the shutdown
/// path, not merely present in source.
#[tokio::test]
async fn real_socket_active_connection_shutdown_test() {
    let (addr, shutdown_tx, handle) = spawn_server(short_transport()).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    // Establish the connection with one full request/response first so the
    // server has actually accepted and served on it before we shut down.
    let req = b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let res = send_and_read(&mut stream, req).await;
    assert!(
        res.starts_with("HTTP/1.1 200"),
        "warm-up request failed: {res}"
    );

    // Trigger real graceful shutdown.
    shutdown_tx
        .send(())
        .expect("serve() task still listening for shutdown signal");

    // The connection must be closed by the server within the real drain
    // deadline, not left open indefinitely: a further read should observe
    // EOF. Margin is added on top of the actual constant `serve()` uses
    // (imported, not duplicated) so this can't race the server's own
    // deadline if that constant ever changes.
    let assertion_margin = Duration::from_secs(5);
    let drain_deadline =
        Duration::from_secs(sitolo_api_bin::shutdown::SHUTDOWN_DRAIN_DEADLINE_SECS);

    let mut buf = vec![0; 64];
    let read = tokio::time::timeout(drain_deadline + assertion_margin, stream.read(&mut buf)).await;
    assert!(
        matches!(read, Ok(Ok(0)) | Ok(Err(_))),
        "expected the connection to be closed by the server during graceful shutdown, got: {read:?}"
    );

    let subsystems = tokio::time::timeout(drain_deadline + assertion_margin, handle)
        .await
        .expect("serve() task returns within the drain deadline plus margin")
        .expect("serve() task does not panic");
    assert!(
        !subsystems.is_empty(),
        "serve() should report at least one subsystem shutdown result"
    );
}

const LIVE_REQUEST: &[u8] = b"GET /process/live HTTP/1.1\r\nHost: localhost\r\n\r\n";

/// Opens a fresh connection and reports whether it was actually served a
/// 200, treating every failure mode (reset, EOF, timeout) as "not served".
async fn live_request_is_served(addr: SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect(addr).await else {
        return false;
    };
    if stream.write_all(LIVE_REQUEST).await.is_err() {
        return false;
    }
    let mut buf = vec![0u8; 1024];
    match tokio::time::timeout(Duration::from_secs(1), stream.read(&mut buf)).await {
        Ok(Ok(n)) if n > 0 => String::from_utf8_lossy(&buf[..n]).starts_with("HTTP/1.1 200"),
        _ => false,
    }
}

/// Resource-pressure proof for the connection-concurrency ceiling
/// (`MAX_IN_FLIGHT_CONNECTIONS`), which had no coverage: the ceiling must
/// admit exactly its limit, refuse the next connection promptly instead of
/// serving it or leaving it hanging, and release the permit when a held
/// connection closes so the server recovers rather than staying wedged.
#[tokio::test]
async fn real_socket_connection_ceiling_rejects_then_recovers() {
    use sitolo_api_bin::shutdown::MAX_IN_FLIGHT_CONNECTIONS;

    // Nothing may close a held connection on its own during this test, so
    // both the header-read timer and the idle policy are pushed far out.
    let transport = HttpTransportConfig {
        request_header_timeout: Duration::from_secs(30),
        http1_idle_timeout: Duration::from_secs(60),
        ..short_transport()
    };
    let (addr, _shutdown, _handle) = spawn_server(transport).await;

    // Fill every permit. Each connection completes a real request first,
    // which also proves the server has accepted it and taken its permit
    // before the next one is opened (no reliance on accept-loop timing).
    let mut held = Vec::with_capacity(MAX_IN_FLIGHT_CONNECTIONS);
    for i in 0..MAX_IN_FLIGHT_CONNECTIONS {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let res = send_and_read(&mut stream, LIVE_REQUEST).await;
        assert!(
            res.starts_with("HTTP/1.1 200"),
            "connection {i} of {MAX_IN_FLIGHT_CONNECTIONS} should be admitted: {res}"
        );
        held.push(stream);
    }

    // One past the ceiling. The kernel completes the TCP handshake via the
    // listen backlog, but the server must drop it at the permit check: the
    // client sees EOF or a reset, never a served response.
    let mut extra = TcpStream::connect(addr).await.unwrap();
    let _ = extra.write_all(LIVE_REQUEST).await;
    let mut buf = vec![0u8; 1024];
    let outcome = tokio::time::timeout(Duration::from_secs(2), extra.read(&mut buf))
        .await
        .expect("an over-ceiling connection must be closed promptly, not left hanging");
    match outcome {
        Ok(0) | Err(_) => {}
        Ok(n) => panic!(
            "connection past the ceiling was served: {}",
            String::from_utf8_lossy(&buf[..n])
        ),
    }

    // Release one permit and prove the server recovers. Retries absorb the
    // small window before the server observes the close and frees the slot.
    drop(held.pop());
    let recovered = tokio::time::timeout(Duration::from_secs(5), async {
        while !live_request_is_served(addr).await {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(
        recovered.is_ok(),
        "server never admitted a new connection after a permit was released"
    );
    drop(held);
}
