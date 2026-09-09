//! Repository ports (§51).
//!
//! Repositories expose semantic operations rather than generic database
//! mutation. The IdentityStores trait bundles all Phase 3 identity
//! persistence; TenancyStores bundles Phase 4 tenancy and IAM persistence.

use async_trait::async_trait;

use sitolo_audit::{AuditRecorder, AuthenticationEvent};
use sitolo_auth::{
    Assurance, AuthenticationMethod, ClientPlatform, Device, DeviceId, MfaAuthenticatorId, MfaKind,
    RecoveryCodeId, SecurityVersion, SessionClass, SessionId, UserId,
};
use sitolo_authz::{
    BranchId, InvitationId, MembershipId, OrganizationId, OwnershipTransferId, RoleId, Scope,
};
use sitolo_security::SealedRef;
use sitolo_tenancy::{
    Branch, BranchStatus, Invitation, Membership, MembershipStatus, Organization,
    OrganizationStatus, OwnershipTransferRequest, TenancyError,
};

use crate::memory::{
    DeviceRegistrationInput, DeviceRevocationEffect, EstablishedSession, MfaEnrollmentResult,
    PasswordResetResult, RefreshRotation, SessionSnapshot, UserSnapshot,
};

/// Bundled identity persistence port (§51).
#[async_trait]
pub trait IdentityStores: Send + Sync + AuditRecorder {
    // --- users ---
    async fn create_user(
        &self,
        id: UserId,
        password_version: Option<u32>,
        password_verifier: Option<String>,
    ) -> Result<UserSnapshot, sitolo_auth::AuthError>;
    async fn user_snapshot(&self, id: &UserId) -> Option<UserSnapshot>;
    async fn set_password_verifier(
        &self,
        id: &UserId,
        verifier: String,
        policy_version: u32,
    ) -> Result<(), sitolo_auth::AuthError>;
    async fn bump_user_security_version(
        &self,
        id: &UserId,
    ) -> Result<SecurityVersion, sitolo_auth::AuthError>;
    async fn suspend_user(&self, id: &UserId) -> Result<(), sitolo_auth::AuthError>;

    // --- sessions ---
    #[allow(clippy::too_many_arguments)]
    async fn establish_session(
        &self,
        user_id: UserId,
        device_id: Option<DeviceId>,
        class: SessionClass,
        method: AuthenticationMethod,
        platform: ClientPlatform,
        assurance: Assurance,
        now: std::time::SystemTime,
    ) -> Result<EstablishedSession, sitolo_auth::AuthError>;
    async fn session_snapshot(&self, id: &SessionId) -> Option<SessionSnapshot>;
    async fn accept_session(
        &self,
        id: &SessionId,
        now: std::time::SystemTime,
    ) -> Result<SessionSnapshot, sitolo_auth::AuthError>;
    async fn revoke_session(
        &self,
        id: &SessionId,
        trigger: sitolo_auth::RevocationTrigger,
    ) -> Result<(), sitolo_auth::AuthError>;
    async fn revoke_sessions_by_scope(
        &self,
        scope: sitolo_auth::RevocationScope,
        trigger: sitolo_auth::RevocationTrigger,
        user_id: Option<&UserId>,
        device_id: Option<&DeviceId>,
    ) -> Result<u32, sitolo_auth::AuthError>;
    async fn elevate_session_assurance(
        &self,
        id: &SessionId,
        to: Assurance,
    ) -> Result<(), sitolo_auth::AuthError>;

    // --- refresh ---
    async fn rotate_refresh(
        &self,
        raw: &str,
        now: std::time::SystemTime,
    ) -> Result<RefreshRotation, sitolo_auth::AuthError>;

    // --- devices ---
    async fn begin_device_registration(
        &self,
        input: DeviceRegistrationInput,
    ) -> Result<Device, sitolo_auth::AuthError>;
    async fn complete_device_registration(
        &self,
        device_id: &DeviceId,
        now: std::time::SystemTime,
    ) -> Result<Device, sitolo_auth::AuthError>;
    async fn device_snapshot(&self, id: &DeviceId) -> Option<Device>;
    async fn revoke_device(
        &self,
        id: &DeviceId,
        now: std::time::SystemTime,
    ) -> Result<DeviceRevocationEffect, sitolo_auth::AuthError>;
    async fn replace_device(
        &self,
        old: &DeviceId,
        now: std::time::SystemTime,
    ) -> Result<Device, sitolo_auth::AuthError>;

    // --- mfa ---
    async fn begin_mfa_enrollment(
        &self,
        user_id: UserId,
        kind: MfaKind,
        secret_reference: Option<SealedRef>,
        now: std::time::SystemTime,
    ) -> Result<MfaEnrollmentResult, sitolo_auth::AuthError>;
    async fn complete_mfa_enrollment(
        &self,
        authenticator_id: &MfaAuthenticatorId,
        now: std::time::SystemTime,
    ) -> Result<Vec<String>, sitolo_auth::AuthError>;
    async fn record_totp_success(
        &self,
        authenticator_id: &MfaAuthenticatorId,
        step: u64,
    ) -> Result<(), sitolo_auth::AuthError>;
    async fn active_authenticator(&self, user_id: &UserId)
    -> Option<sitolo_auth::MfaAuthenticator>;
    async fn consume_recovery_code(
        &self,
        user_id: &UserId,
        raw: &str,
        now: std::time::SystemTime,
    ) -> Result<RecoveryCodeId, sitolo_auth::AuthError>;
    async fn reset_mfa(
        &self,
        target: &UserId,
        actor: &UserId,
        now: std::time::SystemTime,
    ) -> Result<Vec<MfaAuthenticatorId>, sitolo_auth::AuthError>;

    // --- password reset ---
    async fn request_password_reset(
        &self,
        user_id: &UserId,
        raw_token: &str,
        now: std::time::SystemTime,
        ttl: std::time::Duration,
    ) -> Result<PasswordResetResult, sitolo_auth::AuthError>;
    async fn redeem_password_reset(
        &self,
        raw_token: &str,
        new_verifier: String,
        new_policy_version: u32,
        now: std::time::SystemTime,
    ) -> Result<UserId, sitolo_auth::AuthError>;

    // --- audit ---
    async fn append_audit(
        &self,
        event: AuthenticationEvent,
    ) -> Result<(), sitolo_audit::AuditError>;
    async fn audit_trail(&self) -> Vec<AuthenticationEvent>;
}

/// Bundled tenancy & IAM persistence port (Phase 4).
#[async_trait]
pub trait TenancyStores: Send + Sync {
    // --- organization ---
    async fn create_organization(&self, org: Organization) -> Result<Organization, TenancyError>;
    async fn get_organization(
        &self,
        id: &OrganizationId,
    ) -> Result<Option<Organization>, TenancyError>;
    async fn update_organization_status(
        &self,
        id: &OrganizationId,
        status: OrganizationStatus,
        now_epoch_secs: u64,
    ) -> Result<Organization, TenancyError>;
    async fn update_organization_name(
        &self,
        id: &OrganizationId,
        name: String,
        now_epoch_secs: u64,
    ) -> Result<Organization, TenancyError>;

    // --- branch ---
    async fn create_branch(&self, branch: Branch) -> Result<Branch, TenancyError>;
    async fn get_branch(
        &self,
        org_id: &OrganizationId,
        branch_id: &BranchId,
    ) -> Result<Option<Branch>, TenancyError>;
    async fn list_branches(&self, org_id: &OrganizationId) -> Result<Vec<Branch>, TenancyError>;
    async fn update_branch_status(
        &self,
        org_id: &OrganizationId,
        branch_id: &BranchId,
        status: BranchStatus,
        now_epoch_secs: u64,
    ) -> Result<Branch, TenancyError>;

    // --- membership ---
    async fn create_membership(&self, membership: Membership) -> Result<Membership, TenancyError>;
    async fn get_membership(
        &self,
        org_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Option<Membership>, TenancyError>;
    async fn get_membership_by_user(
        &self,
        org_id: &OrganizationId,
        user_id: &UserId,
    ) -> Result<Option<Membership>, TenancyError>;
    async fn list_memberships(
        &self,
        org_id: &OrganizationId,
    ) -> Result<Vec<Membership>, TenancyError>;
    async fn list_user_memberships(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<Membership>, TenancyError>;
    async fn update_membership_status(
        &self,
        org_id: &OrganizationId,
        membership_id: &MembershipId,
        status: MembershipStatus,
        now_epoch_secs: u64,
    ) -> Result<Membership, TenancyError>;
    async fn assign_membership_role(
        &self,
        org_id: &OrganizationId,
        membership_id: &MembershipId,
        role: RoleId,
        now_epoch_secs: u64,
    ) -> Result<Membership, TenancyError>;
    async fn revoke_membership_role(
        &self,
        org_id: &OrganizationId,
        membership_id: &MembershipId,
        role: &RoleId,
        now_epoch_secs: u64,
    ) -> Result<Membership, TenancyError>;
    async fn update_membership_scopes(
        &self,
        org_id: &OrganizationId,
        membership_id: &MembershipId,
        scopes: Vec<Scope>,
        now_epoch_secs: u64,
    ) -> Result<Membership, TenancyError>;

    // --- invitation ---
    async fn create_invitation(&self, invitation: Invitation) -> Result<Invitation, TenancyError>;
    async fn get_invitation(&self, id: &InvitationId) -> Result<Option<Invitation>, TenancyError>;
    async fn list_invitations(
        &self,
        org_id: &OrganizationId,
    ) -> Result<Vec<Invitation>, TenancyError>;
    async fn accept_invitation(
        &self,
        id: &InvitationId,
        now_epoch_secs: u64,
    ) -> Result<Invitation, TenancyError>;
    async fn revoke_invitation(
        &self,
        org_id: &OrganizationId,
        id: &InvitationId,
    ) -> Result<Invitation, TenancyError>;

    // --- ownership transfer ---
    async fn create_ownership_transfer(
        &self,
        req: OwnershipTransferRequest,
    ) -> Result<OwnershipTransferRequest, TenancyError>;
    async fn get_ownership_transfer(
        &self,
        id: &OwnershipTransferId,
    ) -> Result<Option<OwnershipTransferRequest>, TenancyError>;
    async fn approve_ownership_transfer(
        &self,
        id: &OwnershipTransferId,
        now_epoch_secs: u64,
    ) -> Result<OwnershipTransferRequest, TenancyError>;
}
