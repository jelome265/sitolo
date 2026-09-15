//! Listener lifecycle: bind-gated serving, bounded probes, ordered shutdown
//! and tenancy HTTP routing (§28 PR-006).

use std::sync::Arc;
use std::time::Duration;

use sitolo_api::tenancy::{
    CreateBranchRequest, CreateOrganizationRequest, handle_activate_branch,
    handle_activate_organization, handle_begin_close_branch, handle_begin_close_organization,
    handle_close_branch, handle_close_organization, handle_create_branch,
    handle_provision_organization, handle_resume_branch, handle_resume_organization,
    handle_suspend_branch, handle_suspend_organization,
};
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
    let body_part = if header_end < n {
        String::from_utf8_lossy(&raw[header_end..]).into_owned()
    } else {
        String::new()
    };
    let first_line = headers_part.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let path = parts[1];
    let (status_str, body) = dispatch_request(method, path, &body_part, state).await;
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

    // Tenancy APIs (§28 PR-006). All mutations are bounded (§10.2) and
    // validated before domain work (§39); tenant binding is enforced
    // server-side (§27.1).
    if method == "POST" {
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() >= 3 && segments[1] == "v1" && segments[2] == "organizations" {
            let svc = state.tenancy_service();
            if segments.len() == 3 {
                // POST /v1/organizations
                if body.len() > sitolo_api::tenancy::bounds::MAX_TENANCY_BODY_BYTES {
                    return format_error_response(&AppError::Validation);
                }
                let req: Result<CreateOrganizationRequest, _> = serde_json::from_str(body);
                match req {
                    Ok(parsed) => match handle_provision_organization(svc, parsed).await {
                        Ok(res) => {
                            return (
                                "201 Created",
                                serde_json::to_string(&res).unwrap_or_default(),
                            );
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    Err(_) => return format_error_response(&AppError::Validation),
                }
            } else if segments.len() == 5 && segments[4] == "branches" {
                // POST /v1/organizations/{org_id}/branches
                if body.len() > sitolo_api::tenancy::bounds::MAX_TENANCY_BODY_BYTES {
                    return format_error_response(&AppError::Validation);
                }
                let org_id = segments[3];
                let req: Result<CreateBranchRequest, _> = serde_json::from_str(body);
                match req {
                    Ok(parsed) => match handle_create_branch(svc, org_id, parsed).await {
                        Ok(res) => {
                            return (
                                "201 Created",
                                serde_json::to_string(&res).unwrap_or_default(),
                            );
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    Err(_) => return format_error_response(&AppError::Validation),
                }
            } else if segments.len() == 5 {
                let org_id = segments[3];
                let action = segments[4];
                match action {
                    "activate" => match handle_activate_organization(svc, org_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    "suspend" => match handle_suspend_organization(svc, org_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    "resume" => match handle_resume_organization(svc, org_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    "begin_close" => match handle_begin_close_organization(svc, org_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    "close" => match handle_close_organization(svc, org_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    _ => {}
                }
            } else if segments.len() == 7 && segments[4] == "branches" {
                let org_id = segments[3];
                let branch_id = segments[5];
                let action = segments[6];
                match action {
                    "activate" => match handle_activate_branch(svc, org_id, branch_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    "suspend" => match handle_suspend_branch(svc, org_id, branch_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    "resume" => match handle_resume_branch(svc, org_id, branch_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    "begin_close" => {
                        match handle_begin_close_branch(svc, org_id, branch_id).await {
                            Ok(res) => {
                                return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                            }
                            Err(e) => return format_error_response(&e),
                        }
                    }
                    "close" => match handle_close_branch(svc, org_id, branch_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    _ => {}
                }
            }
        }
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
