//! PostgreSQL writes for tenancy operations that must share the application
//! transaction with audit and outbox evidence.

use sqlx::{Postgres, Transaction};
use thiserror::Error;

use sitolo_domain::tenancy::{Branch, Membership, Organization};

#[derive(Debug, Error)]
pub enum TenancyProvisioningError {
    #[error("organization provisioning conflicts with existing tenancy state")]
    Conflict,
    #[error("database operation failed")]
    Database(#[source] sqlx::Error),
}

impl From<sqlx::Error> for TenancyProvisioningError {
    fn from(error: sqlx::Error) -> Self {
        if error
            .as_database_error()
            .and_then(|database_error| database_error.code())
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
pub async fn insert_provisioned_organization(
    tx: &mut Transaction<'_, Postgres>,
    organization: &Organization,
    owner_membership: &Membership,
    default_branch: &Branch,
) -> Result<bool, TenancyProvisioningError> {
    if organization.state != sitolo_domain::tenancy::OrganizationState::Active
        || owner_membership.state != sitolo_domain::tenancy::MembershipState::Active
        || default_branch.state != sitolo_domain::tenancy::BranchState::Active
        || owner_membership.organization_id != organization.id
        || default_branch.organization_id != organization.id
        || organization.state_version != 2
        || owner_membership.state_version != 3
        || default_branch.state_version != 2
        || organization.name.trim() != organization.name
        || organization.name.is_empty()
        || organization.name.len() > 256
        || default_branch.name.trim() != default_branch.name
        || default_branch.name.is_empty()
        || default_branch.name.len() > 256
    {
        return Err(TenancyProvisioningError::Conflict);
    }

    let organization_version = i64::try_from(organization.state_version).map_err(|_| {
        sqlx::Error::Protocol("organization state version exceeds PostgreSQL BIGINT".into())
    })?;
    let membership_version = i64::try_from(owner_membership.state_version).map_err(|_| {
        sqlx::Error::Protocol("membership state version exceeds PostgreSQL BIGINT".into())
    })?;
    let branch_version = i64::try_from(default_branch.state_version).map_err(|_| {
        sqlx::Error::Protocol("branch state version exceeds PostgreSQL BIGINT".into())
    })?;

    let inserted_id: Option<String> = sqlx::query_scalar(
        "INSERT INTO organizations (id, name, state, state_version) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING RETURNING id",
    )
    .bind(organization.id.as_str())
    .bind(&organization.name)
    .bind(organization_state(organization.state))
    .bind(organization_version)
    .fetch_optional(&mut **tx)
    .await?;

    if inserted_id.is_none() {
        let existing_organization: Option<(String, String, String, i64)> = sqlx::query_as(
            "SELECT id, name, state, state_version FROM organizations WHERE id = $1",
        )
        .bind(organization.id.as_str())
        .fetch_optional(&mut **tx)
        .await?;
        let existing_membership: Option<(String, String, String, String, i64)> = sqlx::query_as(
            "SELECT id, organization_id, user_id, state, state_version FROM organization_memberships WHERE id = $1 AND organization_id = $2",
        )
        .bind(owner_membership.id.as_str())
        .bind(organization.id.as_str())
        .fetch_optional(&mut **tx)
        .await?;
        let existing_branch: Option<(String, String, String, String, i64)> = sqlx::query_as(
            "SELECT id, organization_id, name, state, state_version FROM branches WHERE id = $1 AND organization_id = $2",
        )
        .bind(default_branch.id.as_str())
        .bind(organization.id.as_str())
        .fetch_optional(&mut **tx)
        .await?;

        let bundle_matches =
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
            });
        return if bundle_matches {
            Ok(false)
        } else {
            Err(TenancyProvisioningError::Conflict)
        };
    }

    sqlx::query(
        "INSERT INTO organization_memberships (id, organization_id, user_id, state, state_version) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(owner_membership.id.as_str())
    .bind(owner_membership.organization_id.as_str())
    .bind(owner_membership.user_id.as_str())
    .bind(membership_state(owner_membership.state))
    .bind(membership_version)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
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
