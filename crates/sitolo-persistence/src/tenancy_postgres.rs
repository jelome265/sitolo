//! PostgreSQL writes for tenancy operations that must share the application
//! transaction with audit and outbox evidence.

use sqlx_core::error::Error;
use sqlx_core::query::query;
use sqlx_core::query_as::query_as;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::transaction::Transaction;
use sqlx_postgres::Postgres;
use thiserror::Error;

use sitolo_domain::tenancy::{Branch, Membership, Organization};

/// Errors from provisioning the authoritative tenancy rows.
#[derive(Debug, Error)]
pub enum TenancyProvisioningError {
    /// The supplied bundle conflicts with existing or invalid tenancy state.
    #[error("organization provisioning conflicts with existing tenancy state")]
    Conflict,
    /// The database rejected the write.
    #[error("database operation failed")]
    Database(#[source] Error),
}

impl From<Error> for TenancyProvisioningError {
    fn from(error: Error) -> Self {
        if error
            .as_database_error()
            .and_then(sqlx_core::error::DatabaseError::code)
            .as_deref()
            == Some("23505")
        {
            Self::Conflict
        } else {
            Self::Database(error)
        }
    }
}

/// Inserts the three authoritative rows for organization provisioning.
///
/// The caller owns the transaction and must insert mandatory audit/outbox
/// evidence before committing it. Any error leaves rollback to that caller.
///
/// # Errors
///
/// Returns [`TenancyProvisioningError::Conflict`] when the bundle is invalid or
/// conflicts with existing rows, and [`TenancyProvisioningError::Database`] when
/// a write fails.
pub async fn insert_provisioned_organization(
    tx: &mut Transaction<'_, Postgres>,
    organization: &Organization,
    owner_membership: &Membership,
    default_branch: &Branch,
) -> Result<bool, TenancyProvisioningError> {
    validate_bundle(organization, owner_membership, default_branch)?;

    let organization_version = version(organization.state_version, "organization")?;
    let membership_version = version(owner_membership.state_version, "membership")?;
    let branch_version = version(default_branch.state_version, "branch")?;

    let inserted_id: Option<String> = query_scalar(
        "INSERT INTO organizations (id, name, state, state_version) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING RETURNING id",
    )
    .bind(organization.id.as_str())
    .bind(&organization.name)
    .bind(organization_state(organization.state))
    .bind(organization_version)
    .fetch_optional(&mut **tx)
    .await?;

    if inserted_id.is_none() {
        if !existing_bundle_matches(
            tx,
            organization,
            owner_membership,
            default_branch,
            (organization_version, membership_version, branch_version),
        )
        .await?
        {
            return Err(TenancyProvisioningError::Conflict);
        }
        return Ok(false);
    }

    query(
        "INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(owner_membership.id.as_str())
    .bind(owner_membership.organization_id.as_str())
    .bind(owner_membership.user_id.as_str())
    .bind(membership_state(owner_membership.state))
    .bind(membership_version)
    .execute(&mut **tx)
    .await?;

    query(
        "INSERT INTO branches (id, organization_id, name, state, state_version) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(default_branch.id.as_str())
    .bind(default_branch.organization_id.as_str())
    .bind(&default_branch.name)
    .bind(branch_state(default_branch.state))
    .bind(branch_version)
    .execute(&mut **tx)
    .await?;

    Ok(true)
}

fn validate_bundle(
    organization: &Organization,
    owner_membership: &Membership,
    default_branch: &Branch,
) -> Result<(), TenancyProvisioningError> {
    use sitolo_domain::tenancy::{BranchState, MembershipState, OrganizationState};

    let valid = organization.state == OrganizationState::Active
        && owner_membership.state == MembershipState::Active
        && default_branch.state == BranchState::Active
        && owner_membership.organization_id == organization.id
        && default_branch.organization_id == organization.id
        && organization.state_version == 2
        && owner_membership.state_version == 3
        && default_branch.state_version == 2
        && organization.name.trim() == organization.name
        && !organization.name.is_empty()
        && organization.name.len() <= 256
        && default_branch.name.trim() == default_branch.name
        && !default_branch.name.is_empty()
        && default_branch.name.len() <= 256;
    if valid {
        Ok(())
    } else {
        Err(TenancyProvisioningError::Conflict)
    }
}

fn version(state_version: u64, entity: &'static str) -> Result<i64, TenancyProvisioningError> {
    i64::try_from(state_version).map_err(|_error| {
        TenancyProvisioningError::Database(Error::Protocol(format!(
            "{entity} state version exceeds PostgreSQL BIGINT"
        )))
    })
}

async fn existing_bundle_matches(
    tx: &mut Transaction<'_, Postgres>,
    organization: &Organization,
    owner_membership: &Membership,
    default_branch: &Branch,
    versions: (i64, i64, i64),
) -> Result<bool, TenancyProvisioningError> {
    let (organization_version, membership_version, branch_version) = versions;
    let existing_organization: Option<(String, String, String, i64)> =
        query_as("SELECT id, name, state, state_version FROM organizations WHERE id = $1")
            .bind(organization.id.as_str())
            .fetch_optional(&mut **tx)
            .await?;
    let existing_membership: Option<(String, String, String, String, i64)> = query_as(
        "SELECT id, organization_id, user_id, state, state_version FROM organization_memberships WHERE id = $1 AND organization_id = $2",
    )
    .bind(owner_membership.id.as_str())
    .bind(organization.id.as_str())
    .fetch_optional(&mut **tx)
    .await?;
    let existing_branch: Option<(String, String, String, String, i64)> = query_as(
        "SELECT id, organization_id, name, state, state_version FROM branches WHERE id = $1 AND organization_id = $2",
    )
    .bind(default_branch.id.as_str())
    .bind(organization.id.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    Ok(
        existing_organization.is_some_and(|(id, name, state, version)| {
            id == organization.id.as_str()
                && name == organization.name
                && state == organization_state(organization.state)
                && version == organization_version
        }) && existing_membership.is_some_and(|(id, org_id, user_id, state, version)| {
            id == owner_membership.id.as_str()
                && org_id == organization.id.as_str()
                && user_id == owner_membership.user_id.as_str()
                && state == membership_state(owner_membership.state)
                && version == membership_version
        }) && existing_branch.is_some_and(|(id, org_id, name, state, version)| {
            id == default_branch.id.as_str()
                && org_id == organization.id.as_str()
                && name == default_branch.name
                && state == branch_state(default_branch.state)
                && version == branch_version
        }),
    )
}

fn organization_state(state: sitolo_domain::tenancy::OrganizationState) -> &'static str {
    use sitolo_domain::tenancy::OrganizationState as S;
    match state {
        S::Provisioning => "PROVISIONING",
        S::Active => "ACTIVE",
        S::Suspended => "SUSPENDED",
        S::Closing => "CLOSING",
        S::Closed => "CLOSED",
    }
}

fn branch_state(state: sitolo_domain::tenancy::BranchState) -> &'static str {
    use sitolo_domain::tenancy::BranchState as S;
    match state {
        S::Provisioning => "PROVISIONING",
        S::Active => "ACTIVE",
        S::Suspended => "SUSPENDED",
        S::Closing => "CLOSING",
        S::Closed => "CLOSED",
    }
}

fn membership_state(state: sitolo_domain::tenancy::MembershipState) -> &'static str {
    use sitolo_domain::tenancy::MembershipState as S;
    match state {
        S::Invited => "INVITED",
        S::PendingAcceptance => "PENDING_ACCEPTANCE",
        S::Expired => "EXPIRED",
        S::Active => "ACTIVE",
        S::Suspended => "SUSPENDED",
        S::Revoked => "REVOKED",
    }
}
