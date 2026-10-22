//! Listener lifecycle: bind-gated serving, bounded probes, ordered shutdown
//! and tenancy HTTP routing (§28 PR-006).

use std::sync::Arc;
use std::time::Duration;

use sitolo_api::{AppError, ProblemDetails};
use sitolo_observability::RequestId;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, oneshot};

use crate::shutdown::{MAX_IN_FLIGHT_CONNECTIONS, SHUTDOWN_DRAIN_DEADLINE_SECS, Subsystem};
use crate::state::AppState;

const PROBE_READ_TIMEOUT_SECS: u64 = 5;
const PROBE_MAX_BYTES: usize = 32 * 1024;

/// Serves probes and tenancy APIs until `shutdown` fires, then drains
/// in-flight connections within [`SHUTDOWN_DRAIN_DEADLINE_SECS`] and stops
/// subsystems in deterministic order. Returns the completed shutdown order.
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
                    handle_connection(stream, &state).await;
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

async fn handle_connection(mut stream: TcpStream, state: &AppState) {
    let mut buf = vec![0u8; PROBE_MAX_BYTES];
    let read = tokio::time::timeout(
        Duration::from_secs(PROBE_READ_TIMEOUT_SECS),
        stream.read(&mut buf),
    )
    .await;
    let n = match read {
        Ok(Ok(n)) if n > 0 => n,
        _ => return,
    };
    let raw = &buf[..n];
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .or_else(|| raw.windows(2).position(|w| w == b"\n\n").map(|p| p + 2))
        .unwrap_or(n);
    let headers_part = String::from_utf8_lossy(&raw[..header_end]);
    let authorization = headers_part
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| {
            name.eq_ignore_ascii_case("authorization")
                .then_some(value.trim())
        });
    let first_line = headers_part.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let path = parts[1];
    let (status_str, body) =
        dispatch_request_with_authorization(method, path, authorization, "", state).await;
    let response = format!(
        "HTTP/1.1 {status_str}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

pub async fn dispatch_request(
    method: &str,
    path: &str,
    body: &str,
    state: &AppState,
) -> (&'static str, String) {
    dispatch_request_with_authorization(method, path, None, body, state).await
}

/// Dispatches a request while preserving the Authorization header at the
/// HTTP boundary. The current runtime has no composed OIDC verifier, session
/// lookup, or membership/permission resolver. Consequently the presence of a
/// bearer header cannot establish authority and protected writes remain
/// fail-closed until that verified context is wired end-to-end.
pub async fn dispatch_request_with_authorization(
    method: &str,
    path: &str,
    _authorization: Option<&str>,
    _body: &str,
    state: &AppState,
) -> (&'static str, String) {
    // Liveness / readiness probes remain authoritative and unconditional.
    if method == "GET" {
        if path == "/process/live" {
            return (
                "200 OK",
                format!(
                    "{{\"status\":\"live\",\"service\":\"{}\",\"version\":\"{}\"}}",
                    json_escape(state.service_name()),
                    json_escape(state.service_version())
                ),
            );
        } else if path == "/process/ready" {
            return ("200 OK", "{\"status\":\"ready\"}".to_string());
        }
    }

    // The shipped HTTP runtime does not compose an OIDC verifier, validated
    // session lookup, or membership/permission resolver into AppState. A
    // syntactically present bearer therefore cannot establish authority.
    // Keep tenancy writes disabled and never call the pre-auth reference
    // handlers until the complete server-derived authority chain is wired.
    if method == "POST" && path.starts_with("/v1/organizations") {
        return format_error_response(&AppError::Authentication);
    }

    ("404 Not Found", "{\"status\":\"not_found\"}".to_string())
}

fn format_error_response(err: &AppError) -> (&'static str, String) {
    let status = match err {
        AppError::Validation => "422 Unprocessable Entity",
        AppError::Authentication => "401 Unauthorized",
        AppError::Authorization => "403 Forbidden",
        AppError::NotFound => "404 Not Found",
        AppError::Conflict | AppError::IdempotencyConflict => "409 Conflict",
        AppError::RateLimited => "429 Too Many Requests",
        AppError::DependencyUnavailable
        | AppError::UpstreamInvalidResponse
        | AppError::UpstreamTimeout
        | AppError::UnknownOutcome
        | AppError::Internal => "500 Internal Server Error",
    };
    let body = ProblemDetails::from_error(err, &RequestId::new_server()).json();
    (status, body)
}

/// Minimal JSON string escaping for operator-controlled identity fields.
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
