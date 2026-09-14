//! Listener lifecycle: bind-gated serving, bounded probes, ordered shutdown,
//! and tenancy HTTP routing (§28).

use std::sync::Arc;
use std::time::Duration;

use sitolo_api::tenancy::{
    CreateBranchRequest, CreateOrganizationRequest, handle_activate_branch,
    handle_activate_organization, handle_close_branch, handle_close_organization,
    handle_create_branch, handle_provision_organization, handle_resume_branch,
    handle_resume_organization, handle_suspend_branch, handle_suspend_organization,
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

/// Serves probes and API requests until `shutdown` fires, then drains in-flight
/// connections within [`SHUTDOWN_DRAIN_DEADLINE_SECS`] and stops subsystems in
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

    if method == "POST" {
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() >= 3 && segments[1] == "v1" && segments[2] == "organizations" {
            let svc = state.tenancy_service();
            if segments.len() == 3 {
                // POST /v1/organizations
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
            } else if segments.len() == 5 && segments[2] == "organizations" {
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
                    "close" => match handle_close_organization(svc, org_id).await {
                        Ok(res) => {
                            return ("200 OK", serde_json::to_string(&res).unwrap_or_default());
                        }
                        Err(e) => return format_error_response(&e),
                    },
                    _ => {}
                }
            } else if segments.len() == 7 && segments[4] == "branches" {
                // /v1/organizations/{org_id}/branches/{branch_id}/{action}
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

fn format_error_response(e: &AppError) -> (&'static str, String) {
    let pub_err = e.public();
    let status_str = match pub_err.status {
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        403 => "403 Forbidden",
        404 => "404 Not Found",
        409 => "409 Conflict",
        422 => "422 Unprocessable Entity",
        429 => "429 Too Many Requests",
        503 => "503 Service Unavailable",
        _ => "500 Internal Server Error",
    };
    let problem = ProblemDetails::from_error(e, &RequestId::new_server());
    (status_str, problem.json())
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
    use sitolo_config::DatabaseTarget;
    use sitolo_observability::TelemetryBuffer;
    use std::sync::Mutex;

    fn test_app_state() -> Arc<AppState> {
        Arc::new(AppState::new(
            "sitolo-api".into(),
            "0.1.0".into(),
            "test_fingerprint".into(),
            DatabaseTarget {
                host: "localhost".into(),
                port: 5432,
                database: "sitolo_test".into(),
                username: "sitolo".into(),
            },
            Arc::new(Mutex::new(TelemetryBuffer::new(100))),
        ))
    }

    #[tokio::test]
    async fn dispatch_probes_returns_200() {
        let state = test_app_state();
        let (status, body) = dispatch_request("GET", "/process/live", "", &state).await;
        assert_eq!(status, "200 OK");
        assert!(body.contains("sitolo-api"));

        let (status, body) = dispatch_request("GET", "/process/ready", "", &state).await;
        assert_eq!(status, "200 OK");
        assert_eq!(body, "{\"status\":\"ready\"}");
    }

    #[tokio::test]
    async fn dispatch_organization_provisioning_and_lifecycle() {
        let state = test_app_state();
        let prov_payload = r#"{
            "organization_id": "org-http-1",
            "organization_name": "HTTP Merchant",
            "owner_membership_id": "mem-owner-http",
            "owner_user_id": "usr-owner-http",
            "default_branch_id": "br-default-http",
            "default_branch_name": "Main HTTP Branch"
        }"#;

        let (status, body) =
            dispatch_request("POST", "/v1/organizations", prov_payload, &state).await;
        assert_eq!(status, "201 Created");
        assert!(body.contains("org-http-1"));
        assert!(body.contains("ACTIVE"));

        let (status, body) =
            dispatch_request("POST", "/v1/organizations/org-http-1/suspend", "", &state).await;
        assert_eq!(status, "200 OK");
        assert!(body.contains("SUSPENDED"));

        let (status, body) =
            dispatch_request("POST", "/v1/organizations/org-http-1/resume", "", &state).await;
        assert_eq!(status, "200 OK");
        assert!(body.contains("ACTIVE"));

        let (status, body) =
            dispatch_request("POST", "/v1/organizations/org-http-1/close", "", &state).await;
        assert_eq!(status, "200 OK");
        assert!(body.contains("CLOSED"));
    }

    #[tokio::test]
    async fn dispatch_branch_creation_and_lifecycle() {
        let state = test_app_state();
        let prov_payload = r#"{
            "organization_id": "org-http-2",
            "organization_name": "HTTP Merchant 2",
            "owner_membership_id": "mem-owner-http-2",
            "owner_user_id": "usr-owner-http-2",
            "default_branch_id": "br-default-http-2",
            "default_branch_name": "Main HTTP Branch 2"
        }"#;

        dispatch_request("POST", "/v1/organizations", prov_payload, &state).await;

        let branch_payload = r#"{
            "branch_id": "br-http-sub",
            "name": "Sub Branch"
        }"#;
        let (status, body) = dispatch_request(
            "POST",
            "/v1/organizations/org-http-2/branches",
            branch_payload,
            &state,
        )
        .await;
        assert_eq!(status, "201 Created");
        assert!(body.contains("br-http-sub"));
        assert!(body.contains("PROVISIONING"));

        let (status, body) = dispatch_request(
            "POST",
            "/v1/organizations/org-http-2/branches/br-http-sub/activate",
            "",
            &state,
        )
        .await;
        assert_eq!(status, "200 OK");
        assert!(body.contains("ACTIVE"));
    }

    #[tokio::test]
    async fn dispatch_invalid_inputs_returns_validation_error() {
        let state = test_app_state();
        let (status, body) =
            dispatch_request("POST", "/v1/organizations", "invalid json", &state).await;
        assert_eq!(status, "422 Unprocessable Entity");
        assert!(body.contains("VALIDATION_ERROR"));
    }

    #[test]
    fn json_escape_neutralizes_quotes() {
        assert_eq!(json_escape("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(json_escape("plain"), "plain");
    }
}
