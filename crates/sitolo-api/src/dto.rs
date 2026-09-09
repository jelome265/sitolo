//! Named request and response DTOs for Tenancy and IAM APIs (§29).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub security_version: u64,
    pub created_at_epoch_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBranchRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchResponse {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub status: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteMemberRequest {
    pub target_contact: String,
    pub proposed_role: String,
    pub scope_type: Option<String>,
    pub scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationResponse {
    pub id: String,
    pub organization_id: String,
    pub target_contact: String,
    pub proposed_role: String,
    pub status: String,
    pub expires_at_epoch_secs: u64,
    pub raw_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptInvitationRequest {
    pub invitation_id: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipResponse {
    pub id: String,
    pub organization_id: String,
    pub user_id: String,
    pub status: String,
    pub roles: Vec<String>,
    pub created_at_epoch_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignRoleRequest {
    pub role_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestOwnershipTransferRequest {
    pub proposed_owner_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnershipTransferResponse {
    pub id: String,
    pub organization_id: String,
    pub current_owner_user_id: String,
    pub proposed_owner_user_id: String,
    pub status: String,
}
