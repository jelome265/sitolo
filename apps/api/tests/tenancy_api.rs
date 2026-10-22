//! Protected tenancy HTTP routes fail closed until identity and authorization
//! are composed into the production runtime.

use sitolo_api_bin::bootstrap::StartupContext;
use sitolo_api_bin::serve::{dispatch_request_with_authorization, serve};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinSet;

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

async fn test_app_state() -> Arc<sitolo_api_bin::state::AppState> {
    let ctx = StartupContext::build_from_pairs_with_provider(
        Vec::<(String, String)>::new(),
        Some(Arc::new(StubProvider)),
    )
    .await
    .expect("test bootstrap succeeds");
    Arc::clone(ctx.state())
}

#[tokio::test]
async fn tenancy_mutations_deny_missing_and_unverified_bearer_authority() {
    let state = test_app_state().await;
    let routes = [
        ("/v1/organizations", r#"{"owner_user_id":"attacker"}"#),
        (
            "/v1/organizations/org-a/branches",
            r#"{"branch_id":"branch-a","name":"Branch"}"#,
        ),
        ("/v1/organizations/org-a/activate", "{}"),
        ("/v1/organizations/org-a/branches/branch-a/activate", "{}"),
    ];

    for (path, body) in routes {
        let (missing_status, _) =
            dispatch_request_with_authorization("POST", path, None, body, &state).await;
        assert_eq!(missing_status, "401 Unauthorized", "route {path}");

        // A parseable bearer value is not a verified OIDC assertion or
        // session. Until the verifier and authority resolver are composed,
        // it must not enable writes or trust owner/tenant selectors.
        let (unverified_status, _) = dispatch_request_with_authorization(
            "POST",
            path,
            Some("Bearer syntactically-valid-but-unverified"),
            body,
            &state,
        )
        .await;
        assert_eq!(unverified_status, "401 Unauthorized", "route {path}");
    }
}

#[tokio::test]
async fn caller_selected_owner_and_cross_tenant_paths_cannot_mutate() {
    let state = test_app_state().await;
    let spoofed_owner = r#"{
        "organization_id":"org-victim",
        "organization_name":"Victim",
        "owner_membership_id":"attacker-membership",
        "owner_user_id":"victim-owner",
        "default_branch_id":"branch-victim",
        "default_branch_name":"Main"
    }"#;

    for path in [
        "/v1/organizations",
        "/v1/organizations/org-victim/branches/branch-other/close",
    ] {
        let (status, response) = dispatch_request_with_authorization(
            "POST",
            path,
            Some("Bearer invalid"),
            spoofed_owner,
            &state,
        )
        .await;
        assert_eq!(status, "401 Unauthorized");
        assert!(response.contains("AUTHENTICATION_FAILED"));
    }
}

#[tokio::test]
async fn health_probes_remain_available_without_identity() {
    let state = test_app_state().await;
    let (live, _) =
        dispatch_request_with_authorization("GET", "/process/live", None, "", &state).await;
    let (ready, _) =
        dispatch_request_with_authorization("GET", "/process/ready", None, "", &state).await;
    assert_eq!(live, "200 OK");
    assert_eq!(ready, "200 OK");
}

#[tokio::test]
async fn live_http_mutation_route_rejects_missing_and_unverified_credentials() {
    let state = test_app_state().await;
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral test listener");
    let address = listener.local_addr().expect("read listener address");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let mut server = JoinSet::new();
    server.spawn(serve(listener, state, shutdown_rx));

    for authorization in [None, Some("Authorization: Bearer caller-chosen-token\r\n")] {
        let request_body = r#"{"owner_user_id":"attacker"}"#;
        let auth_header = authorization.unwrap_or("");
        let request = format!(
            "POST /v1/organizations HTTP/1.1\r\nHost: localhost\r\n{auth_header}Content-Length: {}\r\nConnection: close\r\n\r\n{request_body}",
            request_body.len()
        );
        let mut client = TcpStream::connect(address)
            .await
            .expect("connect to server");
        client
            .write_all(request.as_bytes())
            .await
            .expect("write HTTP request");
        client.shutdown().await.expect("finish request body");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read HTTP response");
        let response = String::from_utf8(response).expect("HTTP response is UTF-8");
        assert!(response.starts_with("HTTP/1.1 401 Unauthorized"));
        assert!(response.contains("AUTHENTICATION_FAILED"));
    }

    let _ = shutdown_tx.send(());
    server
        .join_next()
        .await
        .expect("server task exits")
        .unwrap();
}
