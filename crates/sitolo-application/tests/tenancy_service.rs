//! Integration tests for [`TenancyService`] against the in-memory
//! reference tenancy store.
//!
//! These live outside `src/` because the reference store is supplied by the
//! persistence crate, which the application layer may only use from tests.
//! Production composition is covered by the PostgreSQL evidence tests.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use sitolo_application::{CreatedInvitation, TenancyService};
use sitolo_audit::ActorRef;
use sitolo_auth::{AbuseClass, RateLimitRule};
use sitolo_authz::{ContactKind, InvitationContact, InvitationError, Permission, Role, Scope};
use sitolo_domain::tenancy::{
    BranchId, InvitationId, MembershipId, OrganizationId, ProvisionedOrganization,
    RoleAssignmentId, ScopeGrantId, TenancyError, TenantUserId,
};
use sitolo_persistence::{PgTenancyUnitOfWork, TenancyDatabase};
use sitolo_tenancy::ScopeError;
use sqlx_postgres::PgPoolOptions;

fn org_id(value: &str) -> Result<OrganizationId, Box<dyn std::error::Error>> {
    Ok(OrganizationId::new(value)?)
}

fn user(value: &str) -> Result<TenantUserId, Box<dyn std::error::Error>> {
    Ok(TenantUserId::new(value)?)
}

/// Test-only actor identity. No evidence sink is wired in these unit
/// tests (`service()`/`strict_service()` never call `.with_evidence`),
/// so this value is never persisted or asserted on here — it exists
/// only to satisfy the `actor: ActorRef` parameter these methods now
/// require. Postgres-backed evidence assertions live in
/// `tests/part8_provision_organization_evidence_tests.rs`.
fn actor(subject: &str) -> ActorRef {
    ActorRef {
        subject_ref: subject.to_string(),
        membership_ref: None,
        device_ref: None,
    }
}

fn service() -> TenancyService {
    TenancyService::new_reference(Arc::new(TenancyDatabase::new()), Vec::new())
}

fn strict_service() -> TenancyService {
    // Single-attempt rules: the second identical call locks (section 38).
    let rule = RateLimitRule {
        max_attempts: 1,
        window: Duration::from_secs(3_600),
        lockout: Duration::from_secs(3_600),
    };
    TenancyService::new_reference(
        Arc::new(TenancyDatabase::new()),
        vec![
            (AbuseClass::InvitationCreate, rule),
            (AbuseClass::InvitationAccept, rule),
        ],
    )
}

/// Invites a member, activates the membership, assigns `role`, and narrows the
/// member to `branch` through the `grant_id` scope grant.
async fn seat_member(
    service: &TenancyService,
    org: &OrganizationId,
    membership: &str,
    subject: &str,
    role: Role,
    branch: &BranchId,
    grant_id: &str,
) -> Result<MembershipId, Box<dyn std::error::Error>> {
    let member = service
        .invite_member(
            MembershipId::new(membership)?,
            org.clone(),
            user(subject)?,
            actor("test"),
        )
        .await?;
    service
        .mark_member_pending(org, &member.id, actor("issuer"))
        .await?;
    service
        .accept_member(org, &member.id, actor("test"))
        .await?;
    service
        .assign_role(
            org.clone(),
            member.id.clone(),
            RoleAssignmentId::new(format!("ra-{membership}"))?,
            role,
            actor("test"),
        )
        .await?;
    service
        .grant_scope(
            org.clone(),
            member.id.clone(),
            ScopeGrantId::new(grant_id)?,
            Scope::Branch {
                organization_id: org.clone(),
                branch_id: branch.clone(),
            },
            actor("test"),
        )
        .await?;
    Ok(member.id)
}

#[tokio::test]
async fn unsupported_organization_mutation_fails_before_reference_store_write()
-> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(50))
        .connect_lazy("postgres://app_runtime@127.0.0.1:1/sitolo_test")?;
    let db: Arc<dyn sitolo_application::TenancyStores> = Arc::new(TenancyDatabase::new());
    let service = TenancyService::new_reference(Arc::clone(&db), Vec::new())
        .for_reference_with_evidence(Arc::new(PgTenancyUnitOfWork::new(pool)));
    let organization_id = org_id("org-fail-closed")?;
    let result = service
        .activate_organization(&organization_id, actor("operator"))
        .await;
    assert!(result.is_err());
    assert!(db.organization_snapshot(&organization_id).await.is_none());
    Ok(())
}

#[tokio::test]
async fn authenticated_user_authorization_never_falls_back_to_reference_store()
-> Result<(), Box<dyn std::error::Error>> {
    let service = service();
    let bundle = provisioned(&service).await?;

    let result = service
        .authorize_user_operation(
            &bundle.owner_membership.user_id,
            &bundle.organization.id,
            Some(&bundle.default_branch.id),
            Permission::SaleCreate,
        )
        .await;

    assert_eq!(result, Err(ScopeError::AuthorityUnavailable));
    Ok(())
}

async fn provisioned(
    service: &TenancyService,
) -> Result<ProvisionedOrganization, Box<dyn std::error::Error>> {
    Ok(service
        .provision_organization(
            org_id("o1")?,
            "Merchant",
            MembershipId::new("m-owner")?,
            user("owner")?,
            BranchId::new("b1")?,
            "Main Branch",
        )
        .await?)
}

#[tokio::test]
async fn provision_creates_operating_triple() -> Result<(), Box<dyn std::error::Error>> {
    let service = service();
    let bundle = provisioned(&service).await?;
    assert!(bundle.organization.can_operate());
    assert!(bundle.owner_membership.has_authority());
    assert!(bundle.default_branch.can_operate());
    Ok(())
}

#[tokio::test]
async fn member_lifecycle_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let service = service();
    let bundle = provisioned(&service).await?;
    let org = &bundle.organization.id;

    let invited = service
        .invite_member(
            MembershipId::new("m-cashier")?,
            org.clone(),
            user("cashier")?,
            actor("test"),
        )
        .await?;
    assert!(!invited.has_authority());
    // Cannot skip to Active.
    assert_eq!(
        service.accept_member(org, &invited.id, actor("test")).await,
        Err(TenancyError::InvalidTransition)
    );
    service
        .mark_member_pending(org, &invited.id, actor("issuer"))
        .await?;
    let active = service
        .accept_member(org, &invited.id, actor("test"))
        .await?;
    assert!(active.has_authority());

    let scope = service
        .effective_scope(org, &active.id, Some(&bundle.default_branch.id))
        .await?;
    assert_eq!(scope.organization_id, *org);

    service
        .suspend_member(org, &active.id, actor("test"))
        .await?;
    assert_eq!(
        service.effective_scope(org, &active.id, None).await,
        Err(ScopeError::MembershipNotActive)
    );
    service
        .resume_member(org, &active.id, actor("test"))
        .await?;
    assert!(service.effective_scope(org, &active.id, None).await.is_ok());
    service
        .revoke_member(org, &active.id, actor("test"))
        .await?;
    assert_eq!(
        service.effective_scope(org, &active.id, None).await,
        Err(ScopeError::MembershipNotActive)
    );
    Ok(())
}

#[tokio::test]
async fn persisted_cross_tenant_scope_is_denied() -> Result<(), Box<dyn std::error::Error>> {
    let service = service();
    let first = provisioned(&service).await?;
    service
        .provision_organization(
            org_id("o2")?,
            "Other Merchant",
            MembershipId::new("m-owner-2")?,
            user("owner-2")?,
            BranchId::new("b2")?,
            "Other Branch",
        )
        .await?;
    // Tenant A membership against Tenant B organization: denied. The
    // scoped membership lookup misses first, so the denial surfaces as
    // MembershipNotActive — still fully non-disclosing.
    assert_eq!(
        service
            .effective_scope(&org_id("o2")?, &first.owner_membership.id, None)
            .await,
        Err(ScopeError::MembershipNotActive)
    );
    // Tenant A membership with Tenant B branch: denied.
    assert_eq!(
        service
            .effective_scope(
                &first.organization.id,
                &first.owner_membership.id,
                Some(&BranchId::new("b2")?)
            )
            .await,
        Err(ScopeError::BranchDenied)
    );
    Ok(())
}

#[tokio::test]
async fn role_grants_resolve_to_least_privilege_permissions()
-> Result<(), Box<dyn std::error::Error>> {
    let service = service();
    let bundle = provisioned(&service).await?;
    let org = &bundle.organization.id;

    // Owner holds the full catalog.
    service
        .assign_role(
            org.clone(),
            bundle.owner_membership.id.clone(),
            RoleAssignmentId::new("ra-owner")?,
            Role::Owner,
            actor("test"),
        )
        .await?;
    let owner_permissions = service
        .member_permissions(org, &bundle.owner_membership.id)
        .await;
    assert_eq!(owner_permissions.len(), Permission::all().len());

    // A cashier resolves to exactly sale capabilities.
    let cashier = service
        .invite_member(
            MembershipId::new("m-cashier")?,
            org.clone(),
            user("cashier")?,
            actor("test"),
        )
        .await?;
    service
        .mark_member_pending(org, &cashier.id, actor("issuer"))
        .await?;
    service
        .accept_member(org, &cashier.id, actor("test"))
        .await?;
    service
        .assign_role(
            org.clone(),
            cashier.id.clone(),
            RoleAssignmentId::new("ra-cashier")?,
            Role::Cashier,
            actor("test"),
        )
        .await?;
    assert_eq!(
        service.member_permissions(org, &cashier.id).await,
        BTreeSet::from([Permission::SaleCreate, Permission::SaleView])
    );

    // Revocation drains authority without deleting history.
    service
        .revoke_role(org, &cashier.id, Role::Cashier, actor("test"))
        .await?;
    assert!(
        service
            .member_permissions(org, &cashier.id)
            .await
            .is_empty()
    );
    assert_eq!(service.member_roles(org, &cashier.id).await.len(), 1);

    // Suspended memberships resolve to nothing while suspended.
    service
        .assign_role(
            org.clone(),
            cashier.id.clone(),
            RoleAssignmentId::new("ra-cashier-2")?,
            Role::Cashier,
            actor("test"),
        )
        .await?;
    service
        .suspend_member(org, &cashier.id, actor("test"))
        .await?;
    assert!(
        service
            .member_permissions(org, &cashier.id)
            .await
            .is_empty()
    );

    // Unknown memberships resolve to nothing without disclosure.
    assert!(
        service
            .member_permissions(org, &MembershipId::new("m-ghost")?)
            .await
            .is_empty()
    );
    Ok(())
}

const INVITE_TOKEN: &str = "tok-invite-accept-0123456789abcdef";
const INVITE_TOKEN_2: &str = "tok-invite-second-0123456789abcdef";

fn mailto(value: &str) -> Result<InvitationContact, Box<dyn std::error::Error>> {
    Ok(InvitationContact::new(ContactKind::Email, value)?)
}

async fn invited(
    service: &TenancyService,
    org: &OrganizationId,
    token: &str,
) -> Result<CreatedInvitation, Box<dyn std::error::Error>> {
    Ok(service
        .create_invitation(
            org.clone(),
            InvitationId::new("inv-1")?,
            MembershipId::new("m-invited")?,
            user("invitee")?,
            Role::Cashier,
            None,
            mailto("invitee@example.com")?,
            token,
            Duration::from_secs(3_600),
            actor("test"),
        )
        .await?)
}

#[tokio::test]
async fn invitation_accept_activates_with_record_authority()
-> Result<(), Box<dyn std::error::Error>> {
    let service = service();
    let bundle = provisioned(&service).await?;
    let org = &bundle.organization.id;
    let created = invited(&service, org, INVITE_TOKEN).await?;
    assert!(!created.membership.has_authority());

    // Acceptance takes no role or scope parameters: authority comes
    // from the server-loaded record only (sections 9.2, T4). A tampered
    // client cannot smuggle OWNER through any argument.
    let accepted = service
        .accept_invitation(
            INVITE_TOKEN,
            RoleAssignmentId::new("ra-inv")?,
            None,
            actor("test"),
        )
        .await?;
    assert!(accepted.membership.has_authority());
    assert_eq!(accepted.assignment.role, Role::Cashier);
    assert_eq!(accepted.scope_grant, None);
    assert_eq!(
        accepted.invitation.state,
        sitolo_authz::InvitationState::Accepted
    );
    assert!(accepted.invitation.contact.is_none());
    assert!(accepted.invitation.token_hash.is_none());

    // The persistence boundary deletes the consumed token hash, so replay
    // is indistinguishable from an unknown token; no grant is duplicated.
    assert_eq!(
        service
            .accept_invitation(
                INVITE_TOKEN,
                RoleAssignmentId::new("ra-replay")?,
                None,
                actor("test")
            )
            .await,
        Err(InvitationError::Invalid)
    );
    assert_eq!(
        service
            .member_roles(org, &created.membership.id)
            .await
            .len(),
        1
    );

    // Unknown tokens are generically invalid.
    assert_eq!(
        service
            .accept_invitation(
                "tok-unknown-0123456789abcdefghijkl",
                RoleAssignmentId::new("ra-ghost")?,
                None,
                actor("test")
            )
            .await,
        Err(InvitationError::Invalid)
    );
    // Malformed tokens never reach the store.
    assert_eq!(
        service
            .accept_invitation("short", RoleAssignmentId::new("ra-x")?, None, actor("test"))
            .await,
        Err(InvitationError::Invalid)
    );
    Ok(())
}

#[tokio::test]
async fn invitation_applies_proposed_scope_grant() -> Result<(), Box<dyn std::error::Error>> {
    let service = service();
    let bundle = provisioned(&service).await?;
    let org = &bundle.organization.id;
    let branch = &bundle.default_branch.id;

    service
        .create_invitation(
            org.clone(),
            InvitationId::new("inv-scoped")?,
            MembershipId::new("m-scoped")?,
            user("scoped")?,
            Role::BranchManager,
            Some(Scope::Branch {
                organization_id: org.clone(),
                branch_id: branch.clone(),
            }),
            mailto("scoped@example.com")?,
            INVITE_TOKEN_2,
            Duration::from_secs(3_600),
            actor("test"),
        )
        .await?;
    // Accepting without the accompanying grant id is rejected: the
    // record demands a scope grant the caller did not mint.
    assert_eq!(
        service
            .accept_invitation(
                INVITE_TOKEN_2,
                RoleAssignmentId::new("ra-scoped")?,
                None,
                actor("test")
            )
            .await,
        Err(InvitationError::Invalid)
    );
    let accepted = service
        .accept_invitation(
            INVITE_TOKEN_2,
            RoleAssignmentId::new("ra-scoped")?,
            Some(ScopeGrantId::new("g-scoped")?),
            actor("test"),
        )
        .await?;
    let grant = accepted
        .scope_grant
        .ok_or_else(|| std::io::Error::other("proposed scope missing"))?;
    assert_eq!(
        grant.scope,
        Scope::Branch {
            organization_id: org.clone(),
            branch_id: branch.clone(),
        }
    );
    // The narrowed member authorizes inside the grant only.
    service
        .authorize_operation(
            org,
            &accepted.membership.id,
            Some(branch),
            Permission::SaleCreate,
        )
        .await?;
    assert_eq!(
        service
            .authorize_operation(org, &accepted.membership.id, None, Permission::SaleCreate)
            .await,
        Err(ScopeError::ScopeDenied)
    );
    Ok(())
}

#[tokio::test]
async fn revoked_invitation_releases_membership_for_reinvite()
-> Result<(), Box<dyn std::error::Error>> {
    let db: Arc<dyn sitolo_application::TenancyStores> = Arc::new(TenancyDatabase::new());
    let service = TenancyService::new_reference(Arc::clone(&db), Vec::new());
    let bundle = provisioned(&service).await?;
    let org = &bundle.organization.id;

    let created = invited(&service, org, INVITE_TOKEN).await?;
    let withdrawn = service
        .revoke_invitation(org, &created.invitation.id, actor("test"))
        .await?;
    assert!(withdrawn.contact.is_none());
    assert!(withdrawn.token_hash.is_none());
    // The withdrawn token is generically invalid.
    assert_eq!(
        service
            .accept_invitation(
                INVITE_TOKEN,
                RoleAssignmentId::new("ra-dead")?,
                None,
                actor("test")
            )
            .await,
        Err(InvitationError::Invalid)
    );
    // The linked membership expired, freeing re-enrollment.
    let membership = db
        .membership_snapshot(org, &created.membership.id)
        .await
        .ok_or_else(|| std::io::Error::other("reinvited membership missing"))?;
    assert!(!membership.has_authority());
    service
        .create_invitation(
            org.clone(),
            InvitationId::new("inv-2")?,
            MembershipId::new("m-reinvited")?,
            user("invitee")?,
            Role::Cashier,
            None,
            mailto("invitee@example.com")?,
            INVITE_TOKEN_2,
            Duration::from_secs(3_600),
            actor("test"),
        )
        .await?;
    Ok(())
}

#[tokio::test]
async fn invitation_paths_are_rate_limited() -> Result<(), Box<dyn std::error::Error>> {
    let service = strict_service();
    let bundle = provisioned(&service).await?;
    let org = &bundle.organization.id;

    service
        .create_invitation(
            org.clone(),
            InvitationId::new("inv-a")?,
            MembershipId::new("m-a")?,
            user("a")?,
            Role::Viewer,
            None,
            mailto("a@example.com")?,
            INVITE_TOKEN,
            Duration::from_secs(3_600),
            actor("test"),
        )
        .await?;
    // Second creation for the organization locks (section 38).
    assert_eq!(
        service
            .create_invitation(
                org.clone(),
                InvitationId::new("inv-b")?,
                MembershipId::new("m-b")?,
                user("b")?,
                Role::Viewer,
                None,
                mailto("b@example.com")?,
                INVITE_TOKEN_2,
                Duration::from_secs(3_600),
                actor("test"),
            )
            .await,
        Err(TenancyError::RateLimited)
    );

    // Acceptance throttle precedes redemption: the first unknown-token
    // attempt consumes the single attempt, and the retry reports
    // throttling instead of reaching the store.
    let throttle = strict_service();
    assert_eq!(
        throttle
            .accept_invitation(
                INVITE_TOKEN,
                RoleAssignmentId::new("ra-t")?,
                None,
                actor("test")
            )
            .await,
        Err(InvitationError::Invalid)
    );
    assert_eq!(
        throttle
            .accept_invitation(
                INVITE_TOKEN,
                RoleAssignmentId::new("ra-t")?,
                None,
                actor("test")
            )
            .await,
        Err(InvitationError::RateLimited)
    );
    Ok(())
}

#[tokio::test]
async fn authorize_operation_intersects_permission_and_scope()
-> Result<(), Box<dyn std::error::Error>> {
    use sitolo_domain::tenancy::ScopeGrantId;

    let service = service();
    let bundle = provisioned(&service).await?;
    let org = &bundle.organization.id;
    let owner = &bundle.owner_membership.id;
    let main_branch = &bundle.default_branch.id;

    service
        .assign_role(
            org.clone(),
            owner.clone(),
            RoleAssignmentId::new("ra-owner")?,
            Role::Owner,
            actor("test"),
        )
        .await?;
    let second_branch = service
        .create_branch(
            BranchId::new("b2")?,
            org.clone(),
            "Second Branch",
            actor("test"),
        )
        .await?;
    service
        .activate_branch(org, &second_branch.id, actor("test"))
        .await?;

    // No grants narrow: organization-wide and branch operations allowed.
    let access = service
        .authorize_operation(org, owner, None, Permission::SaleCreate)
        .await?;
    assert!(access.permissions.contains(&Permission::SaleCreate));
    service
        .authorize_operation(org, owner, Some(main_branch), Permission::SaleCreate)
        .await?;

    // Narrow the owner to the main branch.
    service
        .grant_scope(
            org.clone(),
            owner.clone(),
            ScopeGrantId::new("g-main")?,
            Scope::Branch {
                organization_id: org.clone(),
                branch_id: main_branch.clone(),
            },
            actor("test"),
        )
        .await?;
    assert!(
        service
            .authorize_operation(org, owner, None, Permission::SaleCreate)
            .await
            .is_err_and(|err| err == ScopeError::ScopeDenied)
    );
    service
        .authorize_operation(org, owner, Some(main_branch), Permission::SaleCreate)
        .await?;
    assert!(
        service
            .authorize_operation(org, owner, Some(&second_branch.id), Permission::SaleCreate)
            .await
            .is_err_and(|err| err == ScopeError::ScopeDenied)
    );

    // A cashier holds the scope but lacks the permission.
    let cashier_id = seat_member(
        &service,
        org,
        "m-cashier",
        "cashier",
        Role::Cashier,
        main_branch,
        "g-cashier",
    )
    .await?;
    assert!(
        service
            .authorize_operation(org, &cashier_id, Some(main_branch), Permission::SaleVoid)
            .await
            .is_err_and(|err| err == ScopeError::PermissionDenied)
    );
    service
        .authorize_operation(org, &cashier_id, Some(main_branch), Permission::SaleCreate)
        .await?;

    // Lifting the narrowing restores organization-wide authority.
    service
        .revoke_scope_grant(org, &ScopeGrantId::new("g-main")?, actor("test"))
        .await?;
    service
        .authorize_operation(org, owner, None, Permission::SaleCreate)
        .await?;
    Ok(())
}
