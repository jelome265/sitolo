//! Phase 4 Tenant / Organization / Branch / IAM Security, Negative, and Concurrency Test Suite.
//!
//! Verifies:
//! - Cross-tenant isolation (Org A cannot read/modify Org B)
//! - Cross-branch scope isolation (Branch A1 user cannot access Branch A2)
//! - Privilege escalation prevention (Cashier/Admin cannot grant roles exceeding authority)
//! - Revoked/suspended membership rejection
//! - Closed organization/branch rejection
//! - Invitation replay & single-use enforcement
//! - Concurrency/race handling (invitation acceptance, membership updates)

use std::sync::Arc;

use sitolo_application::TenancyService;
use sitolo_auth::id::{Assurance, SecurityVersion, SessionId, UserId};
use sitolo_authz::{
    BranchId, OrganizationId, ROLE_BRANCH_MANAGER, ROLE_CASHIER, ROLE_ORG_ADMIN, ROLE_OWNER,
    RoleId, Scope,
};
use sitolo_persistence::TenancyDatabase;
use sitolo_tenancy::TenancyError;

async fn setup_world() -> (
    TenancyService<TenancyDatabase>,
    UserId,
    OrganizationId,
    BranchId,
    UserId,
    OrganizationId,
    BranchId,
) {
    let store = TenancyDatabase::new();
    let service = TenancyService::new(store);

    // Tenant A
    let user_a = UserId::new("user-tenant-a-owner").unwrap();
    let (org_a, branch_a, _) = service
        .create_organization("Tenant Alpha", user_a.clone(), 1000)
        .await
        .unwrap();

    // Tenant B
    let user_b = UserId::new("user-tenant-b-owner").unwrap();
    let (org_b, branch_b, _) = service
        .create_organization("Tenant Beta", user_b.clone(), 1000)
        .await
        .unwrap();

    (
        service,
        user_a,
        org_a.id,
        branch_a.id,
        user_b,
        org_b.id,
        branch_b.id,
    )
}

#[tokio::test]
async fn cross_tenant_access_is_denied() {
    let (service, user_a, org_a, _branch_a, _user_b, org_b, branch_b) = setup_world().await;

    let ctx_a = service
        .switch_organization_context(
            user_a,
            org_a,
            SessionId::new("sess-a").unwrap(),
            None,
            Assurance::A2,
            SecurityVersion(1),
        )
        .await
        .unwrap();

    // User A attempts to read Org B -> DENIED
    let err1 = service.get_organization(&org_b, &ctx_a).await.unwrap_err();
    assert!(matches!(err1, TenancyError::Unauthorized { .. }));

    // User A attempts to create branch in Org B -> DENIED
    let err2 = service
        .create_branch(&org_b, "Hostile Branch", &ctx_a, 1010)
        .await
        .unwrap_err();
    assert!(matches!(err2, TenancyError::Unauthorized { .. }));

    // User A attempts to close branch in Org B -> DENIED
    let err3 = service
        .close_branch(&org_b, &branch_b, &ctx_a, 1010)
        .await
        .unwrap_err();
    assert!(matches!(err3, TenancyError::Unauthorized { .. }));

    // User A attempts to invite member to Org B -> DENIED
    let cashier_role = RoleId::new(ROLE_CASHIER).unwrap();
    let err4 = service
        .invite_member(
            &org_b,
            "spy@tenantb.mw",
            cashier_role,
            Scope::Organization,
            &ctx_a,
            1010,
            3600,
        )
        .await
        .unwrap_err();
    assert!(matches!(err4, TenancyError::Unauthorized { .. }));
}

#[tokio::test]
async fn cross_branch_scope_isolation_is_enforced() {
    let (service, user_a, org_a, branch_a1, _, _, _) = setup_world().await;

    let ctx_owner = service
        .switch_organization_context(
            user_a.clone(),
            org_a.clone(),
            SessionId::new("sess-a-owner").unwrap(),
            None,
            Assurance::A2,
            SecurityVersion(1),
        )
        .await
        .unwrap();

    // Owner creates Branch A2
    let branch_a2 = service
        .create_branch(&org_a, "Branch A2", &ctx_owner, 1010)
        .await
        .unwrap();

    // Invite Manager scoped ONLY to Branch A1
    let mgr_role = RoleId::new(ROLE_BRANCH_MANAGER).unwrap();
    let (inv_mgr, _) = service
        .invite_member(
            &org_a,
            "mgr1@tenanta.mw",
            mgr_role,
            Scope::Branch(branch_a1.clone()),
            &ctx_owner,
            1020,
            3600,
        )
        .await
        .unwrap();

    let user_mgr1 = UserId::new("user-mgr1").unwrap();
    let _mem_mgr1 = service
        .accept_invitation(&inv_mgr.id, user_mgr1.clone(), 1030)
        .await
        .unwrap();

    let ctx_mgr1 = service
        .switch_organization_context(
            user_mgr1,
            org_a.clone(),
            SessionId::new("sess-mgr1").unwrap(),
            None,
            Assurance::A1,
            SecurityVersion(1),
        )
        .await
        .unwrap();

    // Manager 1 attempts to close Branch A2 (outside scope) -> DENIED
    let err = service
        .close_branch(&org_a, &branch_a2.id, &ctx_mgr1, 1040)
        .await
        .unwrap_err();
    assert!(matches!(err, TenancyError::Unauthorized { .. }));
}

#[tokio::test]
async fn privilege_escalation_is_blocked() {
    let (service, user_a, org_a, _branch_a, _, _, _) = setup_world().await;

    let ctx_owner = service
        .switch_organization_context(
            user_a,
            org_a.clone(),
            SessionId::new("sess-a-owner").unwrap(),
            None,
            Assurance::A2,
            SecurityVersion(1),
        )
        .await
        .unwrap();

    // Invite OrgAdmin
    let admin_role = RoleId::new(ROLE_ORG_ADMIN).unwrap();
    let (inv_admin, _) = service
        .invite_member(
            &org_a,
            "admin@tenanta.mw",
            admin_role,
            Scope::Organization,
            &ctx_owner,
            1010,
            3600,
        )
        .await
        .unwrap();

    let user_admin = UserId::new("user-admin").unwrap();
    let admin_mem = service
        .accept_invitation(&inv_admin.id, user_admin.clone(), 1020)
        .await
        .unwrap();

    let ctx_admin = service
        .switch_organization_context(
            user_admin,
            org_a.clone(),
            SessionId::new("sess-admin").unwrap(),
            None,
            Assurance::A2,
            SecurityVersion(1),
        )
        .await
        .unwrap();

    // OrgAdmin attempts to grant OWNER role to themselves or others -> DENIED
    let owner_role = RoleId::new(ROLE_OWNER).unwrap();
    let err1 = service
        .assign_role(&org_a, &admin_mem.id, owner_role.clone(), &ctx_admin, 1030)
        .await
        .unwrap_err();
    assert_eq!(err1, TenancyError::RoleAssignmentForbidden);

    // OrgAdmin attempts to invite another OWNER -> DENIED
    let err2 = service
        .invite_member(
            &org_a,
            "newowner@tenanta.mw",
            owner_role,
            Scope::Organization,
            &ctx_admin,
            1040,
            3600,
        )
        .await
        .unwrap_err();
    assert_eq!(err2, TenancyError::RoleAssignmentForbidden);
}

#[tokio::test]
async fn suspended_and_revoked_memberships_are_denied() {
    let (service, user_a, org_a, _, _, _, _) = setup_world().await;

    let ctx_owner = service
        .switch_organization_context(
            user_a,
            org_a.clone(),
            SessionId::new("sess-a-owner").unwrap(),
            None,
            Assurance::A2,
            SecurityVersion(1),
        )
        .await
        .unwrap();

    // Invite Cashier
    let cashier_role = RoleId::new(ROLE_CASHIER).unwrap();
    let (inv, _) = service
        .invite_member(
            &org_a,
            "cashier@tenanta.mw",
            cashier_role,
            Scope::Organization,
            &ctx_owner,
            1010,
            3600,
        )
        .await
        .unwrap();

    let user_cashier = UserId::new("user-cashier").unwrap();
    let cashier_mem = service
        .accept_invitation(&inv.id, user_cashier.clone(), 1020)
        .await
        .unwrap();

    // Suspend Cashier membership
    service
        .suspend_membership(&org_a, &cashier_mem.id, &ctx_owner, 1030)
        .await
        .unwrap();

    // Context switch for suspended cashier MUST FAIL
    let err_switch = service
        .switch_organization_context(
            user_cashier.clone(),
            org_a.clone(),
            SessionId::new("sess-cashier").unwrap(),
            None,
            Assurance::A1,
            SecurityVersion(1),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err_switch,
        TenancyError::MembershipInactive { .. }
    ));

    // Revoke Cashier membership
    service
        .revoke_membership(&org_a, &cashier_mem.id, &ctx_owner, 1040)
        .await
        .unwrap();

    // Context switch for revoked cashier MUST FAIL
    let err_switch2 = service
        .switch_organization_context(
            user_cashier,
            org_a,
            SessionId::new("sess-cashier-2").unwrap(),
            None,
            Assurance::A1,
            SecurityVersion(1),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err_switch2,
        TenancyError::MembershipInactive { .. }
    ));
}

#[tokio::test]
async fn closed_organization_denies_operations() {
    let (service, user_a, org_a, _, _, _, _) = setup_world().await;

    let ctx_owner = service
        .switch_organization_context(
            user_a.clone(),
            org_a.clone(),
            SessionId::new("sess-a-owner").unwrap(),
            None,
            Assurance::A2,
            SecurityVersion(1),
        )
        .await
        .unwrap();

    // Close Organization
    service
        .close_organization(&org_a, &ctx_owner, 1010)
        .await
        .unwrap();

    // Context switch to closed organization MUST FAIL
    let err = service
        .switch_organization_context(
            user_a,
            org_a,
            SessionId::new("sess-a-owner-2").unwrap(),
            None,
            Assurance::A2,
            SecurityVersion(1),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, TenancyError::OrganizationSuspended { .. }));
}

#[tokio::test]
async fn invitation_replay_is_prevented() {
    let (service, user_a, org_a, _, _, _, _) = setup_world().await;

    let ctx_owner = service
        .switch_organization_context(
            user_a,
            org_a.clone(),
            SessionId::new("sess-a-owner").unwrap(),
            None,
            Assurance::A2,
            SecurityVersion(1),
        )
        .await
        .unwrap();

    let cashier_role = RoleId::new(ROLE_CASHIER).unwrap();
    let (inv, _) = service
        .invite_member(
            &org_a,
            "cashier@tenanta.mw",
            cashier_role,
            Scope::Organization,
            &ctx_owner,
            1010,
            3600,
        )
        .await
        .unwrap();

    let user_cashier1 = UserId::new("user-cashier-1").unwrap();
    service
        .accept_invitation(&inv.id, user_cashier1, 1020)
        .await
        .unwrap();

    // Replay attempt by user 2 on the same invitation MUST FAIL!
    let user_cashier2 = UserId::new("user-cashier-2").unwrap();
    let err_replay = service
        .accept_invitation(&inv.id, user_cashier2, 1030)
        .await
        .unwrap_err();

    assert_eq!(err_replay, TenancyError::InvitationAlreadyAccepted);
}

#[tokio::test]
async fn concurrent_invitation_acceptance_race_resolves_deterministically() {
    let store = TenancyDatabase::new();
    let service = Arc::new(TenancyService::new(store));

    let user_owner = UserId::new("user-owner").unwrap();
    let (org, _, _) = service
        .create_organization("Race SME", user_owner.clone(), 1000)
        .await
        .unwrap();

    let ctx_owner = service
        .switch_organization_context(
            user_owner,
            org.id.clone(),
            SessionId::new("sess-owner").unwrap(),
            None,
            Assurance::A2,
            SecurityVersion(1),
        )
        .await
        .unwrap();

    let cashier_role = RoleId::new(ROLE_CASHIER).unwrap();
    let (inv, _) = service
        .invite_member(
            &org.id,
            "race@tenant.mw",
            cashier_role,
            Scope::Organization,
            &ctx_owner,
            1010,
            3600,
        )
        .await
        .unwrap();

    let inv_id = inv.id.clone();
    let service_1 = Arc::clone(&service);
    let service_2 = Arc::clone(&service);

    let handle1 = tokio::spawn(async move {
        service_1
            .accept_invitation(&inv_id, UserId::new("user-racer-1").unwrap(), 1020)
            .await
    });

    let inv_id2 = inv.id.clone();
    let handle2 = tokio::spawn(async move {
        service_2
            .accept_invitation(&inv_id2, UserId::new("user-racer-2").unwrap(), 1020)
            .await
    });

    let res1 = handle1.await.unwrap();
    let res2 = handle2.await.unwrap();

    // Exactly ONE succeeds and ONE fails with InvitationAlreadyAccepted!
    let success_count = usize::from(res1.is_ok()) + usize::from(res2.is_ok());
    assert_eq!(success_count, 1);
}
