//! Tenancy API transport boundary and DTO definitions.
//!
//! Phase 4 specification, §28-§30, §39 and §56. Owns request decoding,
//! response models, input validation bounds, and transport error mapping for
//! organization and branch operations (§28 representative, PR-006 scope).
//! Dedicated DTOs ensure raw request payloads never directly deserialize into
//! persistence or domain entities (§29), preventing property-level privilege
//! injection. The raw HTTP body is bounded before deserialization (§10.2) and
//! unknown fields are rejected (§10.1).

use serde::{Deserialize, Serialize};
use sitolo_application::TenancyService;
use sitolo_domain::tenancy::{
    Branch, BranchId, BranchState, MembershipId, Organization, OrganizationId, OrganizationState,
    TenancyError, TenantUserId,
};
use sitolo_persistence::ProvisionedOrganization;

use crate::error::AppError;

/// Bounded input limits for tenancy requests (§39, §10.2).
pub mod bounds {
    /// Maximum raw JSON body for tenancy POSTs (§10.2: larger than probes).
    pub const MAX_TENANCY_BODY_BYTES: usize = 32 * 1024;
    /// Maximum identifier length (identifier types already enforce 128, this
    /// mirrors it at the transport layer for early rejection, §11).
    pub const MAX_IDENTIFIER_LEN: usize = 128;
    /// Maximum display name length (domain enforces 256, transport mirrors).
    pub const MAX_NAME_LEN: usize = 256;
}

/// Request DTO for provisioning a new organization (§29 PR-006).
///
/// `owner_user_id` is intentionally NOT a free-form client authority field:
/// in production it must be derived from the authenticated principal
/// (Phase 4 §5, API contract §2.4). This DTO carries it only for the
/// in-memory reference implementation and tests; production wiring replaces
/// it with the `SecurityContext` subject once authentication is fully
/// integrated (deferred to PR-007 with full auth middleware). The
/// `organization_id` / `branch_id` / `membership_id` values provide
/// record-level idempotency (§56): same ids + same names → original result,
/// divergent names → conflict.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct CreateBranchRequest {
    pub branch_id: String,
    pub name: String,
}

/// Response DTO representing an organization entity (read projection, §2.1).
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

/// Response DTO representing a branch entity (read projection).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BranchResponse {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub state: String,
    pub state_version: u64,
}

impl From<&Branch> for BranchResponse {
    fn from(branch: &Branch) -> Self {
        let state_str = match branch.state {
            BranchState::Provisioning => "PROVISIONING",
            BranchState::Active => "ACTIVE",
            BranchState::Suspended => "SUSPENDED",
            BranchState::Closing => "CLOSING",
            BranchState::Closed => "CLOSED",
        };
        BranchResponse {
            id: branch.id.as_str().to_string(),
            organization_id: branch.organization_id.as_str().to_string(),
            name: branch.name.clone(),
            state: state_str.to_string(),
            state_version: branch.state_version,
        }
    }
}

/// Response DTO representing a newly provisioned organization bundle (§7.1).
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

/// Rejects oversized bodies before deserialization (§10.2).
pub fn ensure_body_bounded(body: &str) -> Result<(), AppError> {
    if body.len() > bounds::MAX_TENANCY_BODY_BYTES {
        return Err(AppError::Validation);
    }
    Ok(())
}

/// Validates identifier strings at the transport layer (early, cheap, §39).
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

/// Validates display names at the transport layer (§39, domain enforces
/// 1..=256 trimmed).
pub fn validate_name(name: &str) -> Result<(), AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > bounds::MAX_NAME_LEN {
        return Err(AppError::Validation);
    }
    Ok(())
}

/// Validates `CreateOrganizationRequest` before domain execution (§39).
pub fn validate_create_organization_input(req: &CreateOrganizationRequest) -> Result<(), AppError> {
    validate_identifier(&req.organization_id)?;
    validate_name(&req.organization_name)?;
    validate_identifier(&req.owner_membership_id)?;
    validate_identifier(&req.owner_user_id)?;
    validate_identifier(&req.default_branch_id)?;
    validate_name(&req.default_branch_name)?;
    Ok(())
}

/// Validates `CreateBranchRequest` together with its path organization id.
pub fn validate_create_branch_input(
    organization_id: &str,
    req: &CreateBranchRequest,
) -> Result<(), AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(&req.branch_id)?;
    validate_name(&req.name)?;
    Ok(())
}

/// Maps domain [`TenancyError`] to stable public [`AppError`] (§30).
///
/// `NotFound` covers `ORGANIZATION_NOT_FOUND` / `BRANCH_NOT_FOUND` etc.
/// (404, no existence disclosure beyond the generic status — the path
/// already names the requested resource). `Conflict` covers
/// `MEMBERSHIP_ALREADY_EXISTS` / `IAM_CONCURRENCY_CONFLICT` / terminal
/// states (409, retry with a fresh id). `InvalidTransition` maps to
/// `Conflict` (409) matching the domain's "illegal transition" semantics
/// (e.g. `ORGANIZATION_CLOSED` cannot be suspended). Unknown future codes
/// remain `INTERNAL` until explicitly mapped.
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

// --- Handlers for Organization & Branch Lifecycle (§28 PR-006) ---

/// `POST /v1/organizations` — provisions organization + owner + default branch (§7.1).
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

/// `POST /v1/organizations/{organization_id}/branches` (§28 PR-006).
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

/// Each lifecycle handler validates the path identifiers (reject malformed
/// before any DB work, §11) and maps domain errors (§30).
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

pub async fn handle_begin_close_organization(
    service: &TenancyService,
    organization_id: &str,
) -> Result<OrganizationResponse, AppError> {
    validate_identifier(organization_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let org = service
        .begin_close_organization(&org_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(OrganizationResponse::from(&org))
}

pub async fn handle_close_organization(
    service: &TenancyService,
    organization_id: &str,
) -> Result<OrganizationResponse, AppError> {
    validate_identifier(organization_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let org = service
        .close_organization(&org_id)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(OrganizationResponse::from(&org))
}

pub async fn handle_activate_branch(
    service: &TenancyService,
    organization_id: &str,
    branch_id: &str,
) -> Result<BranchResponse, AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(branch_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let bid = BranchId::new(branch_id).map_err(|e| map_tenancy_error(&e))?;
    let branch = service
        .activate_branch(&org_id, &bid)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

pub async fn handle_suspend_branch(
    service: &TenancyService,
    organization_id: &str,
    branch_id: &str,
) -> Result<BranchResponse, AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(branch_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let bid = BranchId::new(branch_id).map_err(|e| map_tenancy_error(&e))?;
    let branch = service
        .suspend_branch(&org_id, &bid)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

pub async fn handle_resume_branch(
    service: &TenancyService,
    organization_id: &str,
    branch_id: &str,
) -> Result<BranchResponse, AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(branch_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let bid = BranchId::new(branch_id).map_err(|e| map_tenancy_error(&e))?;
    let branch = service
        .resume_branch(&org_id, &bid)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

pub async fn handle_begin_close_branch(
    service: &TenancyService,
    organization_id: &str,
    branch_id: &str,
) -> Result<BranchResponse, AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(branch_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let bid = BranchId::new(branch_id).map_err(|e| map_tenancy_error(&e))?;
    let branch = service
        .begin_close_branch(&org_id, &bid)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

pub async fn handle_close_branch(
    service: &TenancyService,
    organization_id: &str,
    branch_id: &str,
) -> Result<BranchResponse, AppError> {
    validate_identifier(organization_id)?;
    validate_identifier(branch_id)?;
    let org_id = OrganizationId::new(organization_id).map_err(|e| map_tenancy_error(&e))?;
    let bid = BranchId::new(branch_id).map_err(|e| map_tenancy_error(&e))?;
    let branch = service
        .close_branch(&org_id, &bid)
        .await
        .map_err(|e| map_tenancy_error(&e))?;
    Ok(BranchResponse::from(&branch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_domain::tenancy::TenancyError;

    #[test]
    fn identifier_validation_rejects_hostile_shapes() {
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("has space").is_err());
        assert!(validate_identifier("inject\r\n").is_err());
        assert!(validate_identifier(&"x".repeat(129)).is_err());
        assert!(validate_identifier("org_01:branch-2").is_ok());
    }

    #[test]
    fn name_validation_rejects_empty_and_oversized() {
        assert!(validate_name("").is_err());
        assert!(validate_name("   ").is_err());
        assert!(validate_name(&"n".repeat(257)).is_err());
        assert!(validate_name("Main Branch").is_ok());
    }

    #[test]
    fn body_bound_enforced() {
        assert!(ensure_body_bounded(&"x".repeat(bounds::MAX_TENANCY_BODY_BYTES + 1)).is_err());
        assert!(ensure_body_bounded("{\"a\":1}").is_ok());
    }

    #[test]
    fn error_mapping_is_generic_and_safe() {
        assert_eq!(
            map_tenancy_error(&TenancyError::NotFound),
            AppError::NotFound
        );
        assert_eq!(
            map_tenancy_error(&TenancyError::InvalidIdentifier),
            AppError::Validation
        );
        assert_eq!(
            map_tenancy_error(&TenancyError::Conflict),
            AppError::Conflict
        );
        assert_eq!(
            map_tenancy_error(&TenancyError::InvalidTransition),
            AppError::Conflict
        );
        assert_eq!(
            map_tenancy_error(&TenancyError::TerminalState),
            AppError::Conflict
        );
        assert_eq!(
            map_tenancy_error(&TenancyError::RateLimited),
            AppError::RateLimited
        );
    }

    #[test]
    fn dto_rejects_unknown_fields() {
        let raw = r#"{"organization_id":"o1","organization_name":"M","owner_membership_id":"m1","owner_user_id":"u1","default_branch_id":"b1","default_branch_name":"B","extra":"field"}"#;
        let parsed: Result<CreateOrganizationRequest, _> = serde_json::from_str(raw);
        assert!(parsed.is_err(), "unknown field must be rejected (§10.1)");
    }

    #[tokio::test]
    async fn provision_handler_validates_before_service() {
        use sitolo_application::TenancyService;
        use sitolo_persistence::TenancyDatabase;
        use std::sync::Arc;

        let service = TenancyService::new(Arc::new(TenancyDatabase::new()), Vec::new());
        let bad = CreateOrganizationRequest {
            organization_id: "".into(),
            organization_name: "M".into(),
            owner_membership_id: "m1".into(),
            owner_user_id: "u1".into(),
            default_branch_id: "b1".into(),
            default_branch_name: "B".into(),
        };
        assert_eq!(
            handle_provision_organization(&service, bad).await,
            Err(AppError::Validation)
        );
    }
}
