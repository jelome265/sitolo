//! Tenancy HTTP API integration tests (§28 PR-006, Phase 4 contract §14).
//!
//! Tests route dispatch, transport validation, error mapping, body limits,
//! unknown-field rejection, DTO deserialization, and cross-tenant repository scope isolation.

use sitolo_api::{BranchResponse, OrganizationResponse, ProvisionedOrganizationResponse};
use sitolo_api_bin::bootstrap::StartupContext;
use sitolo_api_bin::serve::dispatch_request;
use std::sync::Arc;

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
    .expect("bootstrap succeeds");
    Arc::clone(ctx.state())
}

#[tokio::test]
async fn provision_organization_route_success_and_conflict() {
    let state = test_app_state().await;

    let body = r#"{
        "organization_id": "org-prod-001",
        "organization_name": "SME Retail Store",
        "owner_membership_id": "mem-owner-001",
        "owner_user_id": "usr-owner-001",
        "default_branch_id": "br-main-001",
        "default_branch_name": "Main Store Branch"
    }"#;

    let (status, resp_body) = dispatch_request("POST", "/v1/organizations", body, &state).await;
    assert_eq!(status, "201 Created");

    // Deserialize into ProvisionedOrganizationResponse DTO to verify typed response contract
    let prov_res: ProvisionedOrganizationResponse = serde_json::from_str(&resp_body)
        .expect("must deserialize into ProvisionedOrganizationResponse");
    assert_eq!(prov_res.organization.id, "org-prod-001");
    assert_eq!(prov_res.organization.name, "SME Retail Store");
    assert_eq!(prov_res.organization.state, "ACTIVE");
    assert_eq!(prov_res.owner_membership_id, "mem-owner-001");
    assert_eq!(prov_res.default_branch.id, "br-main-001");
    assert_eq!(prov_res.default_branch.organization_id, "org-prod-001");
    assert_eq!(prov_res.default_branch.name, "Main Store Branch");
    assert_eq!(prov_res.default_branch.state, "ACTIVE");

    // Duplicate provisioning with conflicting default branch ID returns 409 Conflict
    let conflicting_body = r#"{
        "organization_id": "org-prod-001",
        "organization_name": "SME Retail Store",
        "owner_membership_id": "mem-owner-002",
        "owner_user_id": "usr-owner-001",
        "default_branch_id": "br-main-002",
        "default_branch_name": "Main Store Branch"
    }"#;

    let (status_conflict, _) =
        dispatch_request("POST", "/v1/organizations", conflicting_body, &state).await;
    assert_eq!(status_conflict, "409 Conflict");
}

#[tokio::test]
async fn branch_creation_and_full_lifecycle_routes() {
    let state = test_app_state().await;

    // First provision org
    let prov_body = r#"{
        "organization_id": "org-branch-test",
        "organization_name": "Multi Branch SME",
        "owner_membership_id": "mem-owner-bt",
        "owner_user_id": "usr-owner-bt",
        "default_branch_id": "br-default-bt",
        "default_branch_name": "Default Branch"
    }"#;
    let (status, _) = dispatch_request("POST", "/v1/organizations", prov_body, &state).await;
    assert_eq!(status, "201 Created");

    // Create a second branch
    let branch_body = r#"{
        "branch_id": "br-second-bt",
        "name": "Downtown Branch"
    }"#;
    let (b_status, b_resp) = dispatch_request(
        "POST",
        "/v1/organizations/org-branch-test/branches",
        branch_body,
        &state,
    )
    .await;
    assert_eq!(b_status, "201 Created");

    let branch_created: BranchResponse =
        serde_json::from_str(&b_resp).expect("must deserialize into BranchResponse");
    assert_eq!(branch_created.id, "br-second-bt");
    assert_eq!(branch_created.organization_id, "org-branch-test");
    assert_eq!(branch_created.name, "Downtown Branch");
    assert_eq!(branch_created.state, "PROVISIONING");

    // Activate second branch
    let (act_status, act_resp) = dispatch_request(
        "POST",
        "/v1/organizations/org-branch-test/branches/br-second-bt/activate",
        "",
        &state,
    )
    .await;
    assert_eq!(act_status, "200 OK");
    let branch_act: BranchResponse =
        serde_json::from_str(&act_resp).expect("must deserialize into BranchResponse");
    assert_eq!(branch_act.state, "ACTIVE");

    // Suspend second branch
    let (sus_status, sus_resp) = dispatch_request(
        "POST",
        "/v1/organizations/org-branch-test/branches/br-second-bt/suspend",
        "",
        &state,
    )
    .await;
    assert_eq!(sus_status, "200 OK");
    let branch_sus: BranchResponse =
        serde_json::from_str(&sus_resp).expect("must deserialize into BranchResponse");
    assert_eq!(branch_sus.state, "SUSPENDED");

    // Resume second branch
    let (res_status, res_resp) = dispatch_request(
        "POST",
        "/v1/organizations/org-branch-test/branches/br-second-bt/resume",
        "",
        &state,
    )
    .await;
    assert_eq!(res_status, "200 OK");
    let branch_res: BranchResponse =
        serde_json::from_str(&res_resp).expect("must deserialize into BranchResponse");
    assert_eq!(branch_res.state, "ACTIVE");

    // Begin close second branch
    let (bc_status, bc_resp) = dispatch_request(
        "POST",
        "/v1/organizations/org-branch-test/branches/br-second-bt/begin_close",
        "",
        &state,
    )
    .await;
    assert_eq!(bc_status, "200 OK");
    let branch_bc: BranchResponse =
        serde_json::from_str(&bc_resp).expect("must deserialize into BranchResponse");
    assert_eq!(branch_bc.state, "CLOSING");

    // Close second branch
    let (c_status, c_resp) = dispatch_request(
        "POST",
        "/v1/organizations/org-branch-test/branches/br-second-bt/close",
        "",
        &state,
    )
    .await;
    assert_eq!(c_status, "200 OK");
    let branch_c: BranchResponse =
        serde_json::from_str(&c_resp).expect("must deserialize into BranchResponse");
    assert_eq!(branch_c.state, "CLOSED");

    // Invalid lifecycle transition on closed branch returns 409 Conflict
    let (ill_act_status, _) = dispatch_request(
        "POST",
        "/v1/organizations/org-branch-test/branches/br-second-bt/activate",
        "",
        &state,
    )
    .await;
    assert_eq!(
        ill_act_status, "409 Conflict",
        "activating a closed branch must return 409 Conflict"
    );

    let (ill_sus_status, _) = dispatch_request(
        "POST",
        "/v1/organizations/org-branch-test/branches/br-second-bt/suspend",
        "",
        &state,
    )
    .await;
    assert_eq!(
        ill_sus_status, "409 Conflict",
        "suspending a closed branch must return 409 Conflict"
    );
}

#[tokio::test]
async fn organization_full_lifecycle_routes() {
    let state = test_app_state().await;

    let prov_body = r#"{
        "organization_id": "org-lc-test",
        "organization_name": "Lifecycle SME",
        "owner_membership_id": "mem-owner-lc",
        "owner_user_id": "usr-owner-lc",
        "default_branch_id": "br-default-lc",
        "default_branch_name": "Default Branch"
    }"#;
    let (status, _) = dispatch_request("POST", "/v1/organizations", prov_body, &state).await;
    assert_eq!(status, "201 Created");

    // Calling activate on an already active org returns 409 Conflict
    let (act_status, _) =
        dispatch_request("POST", "/v1/organizations/org-lc-test/activate", "", &state).await;
    assert_eq!(
        act_status, "409 Conflict",
        "activating an already ACTIVE org must return 409 Conflict"
    );

    // Calling activate on non-existent org returns 404 Not Found
    let (act_missing_status, _) =
        dispatch_request("POST", "/v1/organizations/org-missing/activate", "", &state).await;
    assert_eq!(act_missing_status, "404 Not Found");

    // Suspend org (ACTIVE -> SUSPENDED)
    let (s_status, s_resp) =
        dispatch_request("POST", "/v1/organizations/org-lc-test/suspend", "", &state).await;
    assert_eq!(s_status, "200 OK");
    let org_sus: OrganizationResponse =
        serde_json::from_str(&s_resp).expect("must deserialize into OrganizationResponse");
    assert_eq!(org_sus.state, "SUSPENDED");

    // Resume org (SUSPENDED -> ACTIVE)
    let (r_status, r_resp) =
        dispatch_request("POST", "/v1/organizations/org-lc-test/resume", "", &state).await;
    assert_eq!(r_status, "200 OK");
    let org_res: OrganizationResponse =
        serde_json::from_str(&r_resp).expect("must deserialize into OrganizationResponse");
    assert_eq!(org_res.state, "ACTIVE");

    // Begin close org (ACTIVE -> CLOSING)
    let (bc_status, bc_resp) = dispatch_request(
        "POST",
        "/v1/organizations/org-lc-test/begin_close",
        "",
        &state,
    )
    .await;
    assert_eq!(bc_status, "200 OK");
    let org_bc: OrganizationResponse =
        serde_json::from_str(&bc_resp).expect("must deserialize into OrganizationResponse");
    assert_eq!(org_bc.state, "CLOSING");

    // Close org (CLOSING -> CLOSED)
    let (c_status, c_resp) =
        dispatch_request("POST", "/v1/organizations/org-lc-test/close", "", &state).await;
    assert_eq!(c_status, "200 OK");
    let org_c: OrganizationResponse =
        serde_json::from_str(&c_resp).expect("must deserialize into OrganizationResponse");
    assert_eq!(org_c.state, "CLOSED");

    // Illegal state transition on closed org returns 409 Conflict
    let (ill_status, _) =
        dispatch_request("POST", "/v1/organizations/org-lc-test/suspend", "", &state).await;
    assert_eq!(ill_status, "409 Conflict");
}

#[tokio::test]
async fn rejects_unknown_fields_and_malformed_json() {
    let state = test_app_state().await;

    // Unknown field in CreateOrganizationRequest -> 422
    let unknown_field_body = r#"{
        "organization_id": "org-uf-001",
        "organization_name": "SME",
        "owner_membership_id": "mem-001",
        "owner_user_id": "usr-001",
        "default_branch_id": "br-001",
        "default_branch_name": "Branch",
        "unexpected": "property"
    }"#;
    let (status_uf, _) =
        dispatch_request("POST", "/v1/organizations", unknown_field_body, &state).await;
    assert_eq!(status_uf, "422 Unprocessable Entity");

    // Malformed JSON -> 422
    let malformed_body = r#"{ "organization_id": "org-bad", "#;
    let (status_mf, _) =
        dispatch_request("POST", "/v1/organizations", malformed_body, &state).await;
    assert_eq!(status_mf, "422 Unprocessable Entity");

    // Missing required field -> 422
    let missing_field_body = r#"{ "organization_id": "org-missing" }"#;
    let (status_mf2, _) =
        dispatch_request("POST", "/v1/organizations", missing_field_body, &state).await;
    assert_eq!(status_mf2, "422 Unprocessable Entity");
}

#[tokio::test]
async fn rejects_oversized_payload_and_invalid_identifiers() {
    let state = test_app_state().await;

    // Oversized body (> 32 KiB) -> Axum body-limit rejection (413)
    let huge_name = "x".repeat(33 * 1024);
    let huge_body = format!(
        r#"{{
            "organization_id": "org-huge",
            "organization_name": "{huge_name}",
            "owner_membership_id": "mem-huge",
            "owner_user_id": "usr-huge",
            "default_branch_id": "br-huge",
            "default_branch_name": "Branch"
        }}"#
    );
    let (status_huge, _) = dispatch_request("POST", "/v1/organizations", &huge_body, &state).await;
    assert_eq!(status_huge, "413 Payload Too Large");

    // Hostile identifier with CR/LF injection -> 422
    let hostile_id_body = r#"{
        "organization_id": "org-001\r\nInject",
        "organization_name": "SME",
        "owner_membership_id": "mem-001",
        "owner_user_id": "usr-001",
        "default_branch_id": "br-001",
        "default_branch_name": "Branch"
    }"#;
    let (status_hostile, _) =
        dispatch_request("POST", "/v1/organizations", hostile_id_body, &state).await;
    assert_eq!(status_hostile, "422 Unprocessable Entity");
}

#[tokio::test]
async fn rejects_unknown_routes_and_actions() {
    let state = test_app_state().await;

    // Unknown path -> 404
    let (status_p, _) =
        dispatch_request("POST", "/v1/organizations/org-1/unknown_action", "", &state).await;
    assert_eq!(status_p, "404 Not Found");

    // Unknown branch action -> 404
    let (status_b, _) = dispatch_request(
        "POST",
        "/v1/organizations/org-1/branches/br-1/unknown",
        "",
        &state,
    )
    .await;
    assert_eq!(status_b, "404 Not Found");
}

/// Demonstrates and verifies server-side organization/branch repository ownership
/// binding (scope enforcement: `WHERE organization_id = $1 AND id = $2`).
///
/// **What this test proves:**
/// A client supplying Organization A in the URL path selector (`/v1/organizations/org-tenant-a/...`)
/// and Branch B (`br-b1`, which belongs to Organization B) cannot mutate Branch B. The repository
/// scope predicate rejects the cross-tenant mismatch as absent, returning a generic `404 Not Found`.
/// This prevents cross-tenant existence disclosure or cross-tenant mutation.
///
/// **What this test DOES NOT claim:**
/// This test verifies tenant repository scope enforcement at the transport/repository boundary. It
/// does NOT simulate full authenticated-principal authorization (which belongs to authentication
/// middleware introduced in later phases). No fake auth middleware is invented in Part 6.
#[tokio::test]
async fn cross_tenant_branch_repository_ownership_isolation_denied() {
    let state = test_app_state().await;

    // Provision Tenant A
    let org_a_body = r#"{
        "organization_id": "org-tenant-a",
        "organization_name": "Tenant A",
        "owner_membership_id": "mem-a",
        "owner_user_id": "usr-a",
        "default_branch_id": "br-a1",
        "default_branch_name": "Branch A1"
    }"#;
    let (status_a, _) = dispatch_request("POST", "/v1/organizations", org_a_body, &state).await;
    assert_eq!(status_a, "201 Created");

    // Provision Tenant B
    let org_b_body = r#"{
        "organization_id": "org-tenant-b",
        "organization_name": "Tenant B",
        "owner_membership_id": "mem-b",
        "owner_user_id": "usr-b",
        "default_branch_id": "br-b1",
        "default_branch_name": "Branch B1"
    }"#;
    let (status_b, _) = dispatch_request("POST", "/v1/organizations", org_b_body, &state).await;
    assert_eq!(status_b, "201 Created");

    // Org A selector + Org B's branch -> 404 Not Found (denied without existence disclosure)
    let (cross_status, _) = dispatch_request(
        "POST",
        "/v1/organizations/org-tenant-a/branches/br-b1/activate",
        "",
        &state,
    )
    .await;
    assert_eq!(
        cross_status, "404 Not Found",
        "cross-tenant branch activation must be denied with 404"
    );

    let (cross_suspend, _) = dispatch_request(
        "POST",
        "/v1/organizations/org-tenant-a/branches/br-b1/suspend",
        "",
        &state,
    )
    .await;
    assert_eq!(
        cross_suspend, "404 Not Found",
        "cross-tenant branch suspension must be denied with 404"
    );

    let (cross_close, _) = dispatch_request(
        "POST",
        "/v1/organizations/org-tenant-a/branches/br-b1/close",
        "",
        &state,
    )
    .await;
    assert_eq!(
        cross_close, "404 Not Found",
        "cross-tenant branch close must be denied with 404"
    );
}
