//! Listener lifecycle: bind-gated serving, bounded probes, ordered shutdown.
//!
//! The listener socket is created by the caller only after
//! [`crate::bootstrap::StartupContext`] exists, so serving structurally
//! implies readiness. Probes are intentionally minimal (liveness/readiness
//! only); request routing arrives with later API phases.
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, oneshot};

use crate::shutdown::{MAX_IN_FLIGHT_CONNECTIONS, SHUTDOWN_DRAIN_DEADLINE_SECS, Subsystem};
use crate::state::AppState;

const PROBE_READ_TIMEOUT_SECS: u64 = 5;
const PROBE_MAX_BYTES: usize = 8 * 1024;

/// Serves probes until `shutdown` fires, then drains in-flight connections
/// within [`SHUTDOWN_DRAIN_DEADLINE_SECS`] and stops subsystems in
/// deterministic order. Returns the completed shutdown order.
pub async fn serve(
    listener: TcpListener,
    state: Arc<AppState>,
    shutdown: oneshot::Receiver<()>,
) -> Vec<Subsystem> {
    let mut shutdown = shutdown;
    let semaphore = Arc::new(Semaphore::new(MAX_IN_FLIGHT_CONNECTIONS));
    let mut in_flight = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else { continue };
                let Ok(permit) = semaphore.clone().try_acquire_owned() else { continue };
                let state = Arc::clone(&state);
                in_flight.spawn(async move {
                    let _permit = permit;
                    handle_probe(stream, &state).await;
                });
            }
        }
    }
    let _ = tokio::time::timeout(
        Duration::from_secs(SHUTDOWN_DRAIN_DEADLINE_SECS),
        in_flight.join_all(),
    )
    .await;
    let mut coordinator = crate::shutdown::ShutdownCoordinator::new();
    coordinator.shutdown().to_vec()
}

async fn handle_probe(mut stream: TcpStream, state: &AppState) {
    let mut buf = vec![0u8; PROBE_MAX_BYTES];
    let read = tokio::time::timeout(
        Duration::from_secs(PROBE_READ_TIMEOUT_SECS),
        stream.read(&mut buf),
    )
    .await;
    let request_line = match read {
        Ok(Ok(0)) => return,
        Ok(Ok(n)) => {
            let head = &buf[..n];
            let end = head
                .windows(2)
                .position(|w| w == b"\r\n")
                .or_else(|| head.iter().position(|&b| b == b'\n'))
                .unwrap_or(head.len());
            String::from_utf8_lossy(&head[..end]).into_owned()
        }
        _ => return,
    };
    let (status, body) =
        if request_line == "GET /process/live" || request_line.starts_with("GET /process/live ") {
            (
                "200 OK",
                format!(
                    "{{\"status\":\"live\",\"service\":\"{}\",\"version\":\"{}\"}}",
                    json_escape(state.service_name()),
                    json_escape(state.service_version())
                ),
            )
        } else if request_line == "GET /process/ready"
            || request_line.starts_with("GET /process/ready ")
        {
            ("200 OK", "{\"status\":\"ready\"}".to_string())
        } else {
            ("404 Not Found", "{\"status\":\"not_found\"}".to_string())
        };
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

/// Minimal JSON string escaping for operator-controlled identity fields.
/// Service name/version already reject control characters at validation.
fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escape_neutralizes_quotes() {
        assert_eq!(json_escape("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(json_escape("plain"), "plain");
    }
}
