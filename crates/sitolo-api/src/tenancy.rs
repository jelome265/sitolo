//! Tenancy API transport boundary and DTO definitions.
//!
//! Phase 4 specification, §28-§30, §39.
//!
//! Owns request decoding, response models, input validation bounds, and
//! transport error mapping for tenant and branch operations. Dedicated DTOs
//! ensure raw request payloads never directly deserialize into persistence or
//! domain entities (§29), preventing property-level privilege injection.

use serde::{Deserialize, Serialize};
use sitolo_application::TenancyService;
use sitolo_domain::tenancy::{
    Branch, BranchId, BranchState, MembershipId, Organization, OrganizationId, OrganizationState,
    TenancyError, TenantUserId,
};
use sitolo_persistence::ProvisionedOrganization;

use crate::error::AppError;

/// Bounded input limits for tenancy requests (§39).
pub mod bounds {
    /// Maximum identifier length (`organization_id`, `branch_id`, etc.).
    pub const MAX_IDENTIFIER_LEN: usize = 128;
    /// Maximum display name length.
    pub const MAX_NAME_LEN: usize = 256;
}

/// Request DTO for provisioning a new organization (§29).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CreateOrganizationRequest {
    pub organization_id: String,
    pub organization_name: String,
    pub owner_membership_id: String,
    pub owner_user_id: String,
    pub default_branch_id: String,
    pub default_branch_name: String,
}

/// Request DTO for creating a new branch within an organization (§29).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CreateBranchRequest {
    pub branch_id: String,
    pub name: String,
}

/// Response DTO representing an organization entity.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct OrganizationResponse {
    pub id: String,
    pub name: String,
    pub state: String,
    pub state_version: u64,
}

impl From<&Organization> for OrganizationResponse {
    fn from(org: &Organization) -> Self {
        let state_str = match org.state {
            OrganizationState::Provisioning => "PROVISIONING",
            OrganizationState::Active => "ACTIVE",
            OrganizationState::Suspended => "SUSPENDED",
            OrganizationState::Closing => "CLOSING",
            OrganizationState::Closed => "CLOSED",
        };
        OrganizationResponse {
            id: org.id.as_str().to_string(),
            name: org.name.clone(),
            state: state_str.to_string(),
            state_version: org.state_version,
        }
    }
}

/// Response DTO representing a branch entity.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BranchResponse {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub state: String,
    pub state_version: u64,
}

impl From<&Branch> for BranchResponse {
    fn from(b: &Branch) -> Self {
        let state_str = match b.state {
            BranchState::Provisioning => "PROVISIONING",
            BranchState::Active => "ACTIVE",
            BranchState::Suspended => "SUSPENDED",
            BranchState::Closing => "CLOSING",
            BranchState::Closed => "CLOSED",
        };
        BranchResponse {
            id: b.id.as_str().to_string(),
            organization_id: b.organization_id.as_str().to_string(),
            name: b.name.clone(),
            state: state_str.to_string(),
            state_version: b.state_version,
        }
    }
}

/// Response DTO representing a newly provisioned organization bundle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProvisionedOrganizationResponse {
    pub organization: OrganizationResponse,
    pub owner_membership_id: String,
    pub default_branch: BranchResponse,
}

impl From<&ProvisionedOrganization> for ProvisionedOrganizationResponse {
    fn from(bundle: &ProvisionedOrganization) -> Self {
        ProvisionedOrganizationResponse {
            organization: OrganizationResponse::from(&bundle.organization),
            owner_membership_id: bundle.owner_membership.id.as_str().to_string(),
            default_branch: BranchResponse::from(&bundle.default_branch),
        }
    }
}

/// Validates raw inputs for organization creation before attempting domain execution (§39).
pub fn validate_create_organization_input(req: &CreateOrganizationRequest) -> Result<(), AppError> {
    validate_identifier(&req.organization_id)?;
    validate_name(&req.organization_name)?;
    validate_identifier(&req.owner_membership_id)?;
    validate_identifier(&req.owner_user_id)?;
    validate_identifier(&req.default_branch_id)?;
    validate_name(&req.default_branch_name)?;
    Ok(())
}

/// Validates raw inputs for branch creation before domain execution (§39).
pub fn validate_create_branch_input(
    organization_id: &str,
    req: &CreateBranchRequest,
) -> Result<(), AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(&req.branch_id)?;
    validate_name(&req.name)?;
    Ok(())
}

/// Helper function to validate identifier strings.
pub fn validate_identifier(id: &str) -> Result<(), AppError> {
    if id.is_empty()
        || id.len() > bounds::MAX_IDENTIFIER_LEN
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
    {
        return Err(AppError::Validation);
    }
    Ok(())
}

/// Helper function to validate display names.
pub fn validate_name(name: &str) -> Result<(), AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > bounds::MAX_NAME_LEN {
        return Err(AppError::Validation);
    }
    Ok(())
}

/// Maps semantic domain [`TenancyError`] values to stable public [`AppError`]
/// problem classifications (§30).
#[must_use]
pub fn map_tenancy_error(err: &TenancyError) -> AppError {
    match err {
        TenancyError::InvalidIdentifier
        | TenancyError::InvalidName
        | TenancyError::InvalidInvitation => AppError::Validation,
        TenancyError::NotFound => AppError::NotFound,
        TenancyError::Conflict | TenancyError::InvalidTransition | TenancyError::TerminalState => {
            AppError::Conflict
        }
        TenancyError::RateLimited => AppError::RateLimited,
    }
}

// --- HTTP Handlers for Organization & Branch Lifecycle (§28) ---

/// Handles organization provisioning (`POST /v1/organizations`).
pub async fn handle_provision_organization(
    service: &TenancyService,
    req: CreateOrganizationRequest,
) -> Result<ProvisionedOrganizationResponse, AppError> {
    validate_create_organization_input(&req)?;
    let org_id = OrganizationId::new(&req.organization_id).map_err(|e| map_tenancy_error(&e))?;
    let owner_mem_id =
        MembershipId::new(&req.owner_membership_id).map_err(|e| map_tenancy_error(&e))?;
    let owner_user_id = TenantUserId::new(&req.owner_user_id).map_err(|e| map_tenancy_error(&e))?;
    let branch_id = BranchId::new(&req.default_branch_id).map_err(|e| map_tenancy_error(&e))?;

    let bundle = service
        .provision_organization(
            org_id,
            &req.organization_name,
            owner_mem_id,
            owner_user_id,
            branch_id,
            &req.default_branch_name,
        )
        .await
        .map_err(|e| map_tenancy_error(&e))?;

    Ok(ProvisionedOrganizationResponse::from(&bundle))
}

/// Handles organization activation (`POST /v1/organizations/{id}/activate`).
pub async fn handle_activate_organization(
    service: &TenancyService,
    organization_id: &str,
) -> Result<OrganizationResponse, AppError> {
    validate_identifier(organization_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let org = service
        .activate_organization(&org_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(OrganizationResponse::from(&org))
}

/// Handles organization suspension (`POST /v1/organizations/{id}/suspend`).
pub async fn handle_suspend_organization(
    service: &TenancyService,
    organization_id: &str,
) -> Result<OrganizationResponse, AppError> {
    validate_identifier(organization_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let org = service
        .suspend_organization(&org_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(OrganizationResponse::from(&org))
}

/// Handles organization resume (`POST /v1/organizations/{id}/resume`).
pub async fn handle_resume_organization(
    service: &TenancyService,
    organization_id: &str,
) -> Result<OrganizationResponse, AppError> {
    validate_identifier(organization_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let org = service
        .resume_organization(&org_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(OrganizationResponse::from(&org))
}

/// Handles organization closure (`POST /v1/organizations/{id}/close`).
pub async fn handle_close_organization(
    service: &TenancyService,
    organization_id: &str,
) -> Result<OrganizationResponse, AppError> {
    validate_identifier(organization_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    service
        .begin_close_organization(&org_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    let org = service
        .close_organization(&org_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(OrganizationResponse::from(&org))
}

/// Handles branch creation (`POST /v1/organizations/{organization_id}/branches`).
pub async fn handle_create_branch(
    service: &TenancyService,
    organization_id: &str,
    req: CreateBranchRequest,
) -> Result<BranchResponse, AppError> {
    validate_create_branch_input(organization_id, &req)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let branch_id = BranchId::new(&req.branch_id).map_err(|e| map_tenancy_error(&e))?;

    let branch = service
        .create_branch(branch_id, org_id, &req.name)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

/// Handles branch activation (`POST /v1/organizations/{organization_id}/branches/{branch_id}/activate`).
pub async fn handle_activate_branch(
    service: &TenancyService,
    organization_id: &str,
    branch_id: &str,
) -> Result<BranchResponse, AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(branch_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let b_id = BranchId::new(branch_id).map_err(|e| map_tenancy_error(&e))?;

    let branch = service
        .activate_branch(&org_id, &b_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

/// Handles branch suspension (`POST /v1/organizations/{organization_id}/branches/{branch_id}/suspend`).
pub async fn handle_suspend_branch(
    service: &TenancyService,
    organization_id: &str,
    branch_id: &str,
) -> Result<BranchResponse, AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(branch_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let b_id = BranchId::new(branch_id).map_err(|e| map_tenancy_error(&e))?;

    let branch = service
        .suspend_branch(&org_id, &b_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

/// Handles branch resume (`POST /v1/organizations/{organization_id}/branches/{branch_id}/resume`).
pub async fn handle_resume_branch(
    service: &TenancyService,
    organization_id: &str,
    branch_id: &str,
) -> Result<BranchResponse, AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(branch_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let b_id = BranchId::new(branch_id).map_err(|e| map_tenancy_error(&e))?;

    let branch = service
        .resume_branch(&org_id, &b_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

/// Handles branch closure (`POST /v1/organizations/{organization_id}/branches/{branch_id}/close`).
pub async fn handle_close_branch(
    service: &TenancyService,
    organization_id: &str,
    branch_id: &str,
) -> Result<BranchResponse, AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(branch_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let b_id = BranchId::new(branch_id).map_err(|e| map_tenancy_error(&e))?;

    service
        .begin_close_branch(&org_id, &b_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    let branch = service
        .close_branch(&org_id, &b_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_persistence::TenancyDatabase;
    use std::sync::Arc;

    fn service() -> TenancyService {
        TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new())
    }

    #[tokio::test]
    async fn handler_organization_lifecycle_flows_through_dto() {
        let svc = service();

        let req = CreateOrganizationRequest {
            organization_id: "org-dto-1".into(),
            organization_name: "DTO Merchant".into(),
            owner_membership_id: "mem-owner-1".into(),
            owner_user_id: "usr-owner-1".into(),
            default_branch_id: "br-default-1".into(),
            default_branch_name: "Default Branch".into(),
        };

        let bundle = handle_provision_organization(&svc, req).await.unwrap();
        assert_eq!(bundle.organization.id, "org-dto-1");
        assert_eq!(bundle.organization.state, "ACTIVE");

        let suspended = handle_suspend_organization(&svc, "org-dto-1")
            .await
            .unwrap();
        assert_eq!(suspended.state, "SUSPENDED");

        let resumed = handle_resume_organization(&svc, "org-dto-1").await.unwrap();
        assert_eq!(resumed.state, "ACTIVE");

        let closed = handle_close_organization(&svc, "org-dto-1").await.unwrap();
        assert_eq!(closed.state, "CLOSED");
    }

    #[tokio::test]
    async fn handler_branch_lifecycle_flows_through_dto() {
        let svc = service();

        let req = CreateOrganizationRequest {
            organization_id: "org-dto-2".into(),
            organization_name: "DTO Merchant 2".into(),
            owner_membership_id: "mem-owner-2".into(),
            owner_user_id: "usr-owner-2".into(),
            default_branch_id: "br-default-2".into(),
            default_branch_name: "Default Branch 2".into(),
        };
        handle_provision_organization(&svc, req).await.unwrap();

        let branch_req = CreateBranchRequest {
            branch_id: "br-second".into(),
            name: "Second Branch".into(),
        };
        let branch = handle_create_branch(&svc, "org-dto-2", branch_req)
            .await
            .unwrap();
        assert_eq!(branch.id, "br-second");
        assert_eq!(branch.state, "PROVISIONING");

        let active = handle_activate_branch(&svc, "org-dto-2", "br-second")
            .await
            .unwrap();
        assert_eq!(active.state, "ACTIVE");

        let suspended = handle_suspend_branch(&svc, "org-dto-2", "br-second")
            .await
            .unwrap();
        assert_eq!(suspended.state, "SUSPENDED");

        let resumed = handle_resume_branch(&svc, "org-dto-2", "br-second")
            .await
            .unwrap();
        assert_eq!(resumed.state, "ACTIVE");

        let closed = handle_close_branch(&svc, "org-dto-2", "br-second")
            .await
            .unwrap();
        assert_eq!(closed.state, "CLOSED");
    }

    #[tokio::test]
    async fn handler_validates_inputs_before_service_call() {
        let svc = service();
        let invalid_req = CreateOrganizationRequest {
            organization_id: "invalid org id!".into(),
            organization_name: "DTO Merchant".into(),
            owner_membership_id: "mem-owner-1".into(),
            owner_user_id: "usr-owner-1".into(),
            default_branch_id: "br-default-1".into(),
            default_branch_name: "Default Branch".into(),
        };
        assert_eq!(
            handle_provision_organization(&svc, invalid_req).await,
            Err(AppError::Validation)
        );
    }
}
