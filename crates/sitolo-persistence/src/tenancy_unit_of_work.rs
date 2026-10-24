//! PostgreSQL implementation of the application-owned tenancy transaction
//! contract.
//!
//! The application decides what a use case means and owns the transaction
//! boundary; this adapter owns every SQL statement and every row-to-domain
//! mapping. Audit and outbox writes are part of the same transaction as the
//! business mutation, so a committed state change always carries its
//! mandatory evidence (Phase 4 sections 5, 21, 27 and 29).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx_core::error::Error as SqlxError;
use sqlx_core::query::query;
use sqlx_core::query_as::query_as;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::transaction::Transaction;
use sqlx_postgres::{PgPool, Postgres};

use sitolo_application::{
    ExpiredInvitation, InvitationRecord, MembershipRecord, MembershipSelector, ScopeMaterial,
    TenancyTransaction, TenancyUnitOfWork,
};
use sitolo_audit::IamAuditEvent;
use sitolo_authz::{
    ContactKind, Invitation, InvitationError, InvitationState, Role, RoleAssignment,
    RoleAssignmentState, Scope, ScopeGrant, ScopeGrantState,
};
use sitolo_domain::tenancy::{
    Branch, BranchId, BranchState, InvitationId, Membership, MembershipId, MembershipState,
    Organization, OrganizationId, OrganizationState, ProvisionedOrganization, RoleAssignmentId,
    ScopeGrantId, TenancyError, TenantUserId,
};
use sitolo_events::OutboxEvent;
use sitolo_tenancy::ScopeError;

use crate::audit_repository::{AuditWriter, PostgresAuditWriter};
use crate::outbox_repository::{OutboxWriter, PostgresOutboxRepository};
use crate::tenancy_postgres::{TenancyProvisioningError, insert_provisioned_organization};

type PersistedScopeRow = (
    String,
    String,
    i64,
    String,
    String,
    String,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
);

type PersistedRoleRow = (String, String, String, i64);
type PersistedGrantRow = (String, String, Option<String>, String, i64);

/// PostgreSQL unit of work for tenant-scoped use cases.
#[derive(Debug, Clone)]
pub struct PgTenancyUnitOfWork {
    pool: PgPool,
}

impl PgTenancyUnitOfWork {
    /// Creates the adapter over an existing connection pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// PostgreSQL transaction handed to the application.
pub struct PgTenancyTransaction {
    tx: Transaction<'static, Postgres>,
    pool: PgPool,
}

impl std::fmt::Debug for PgTenancyTransaction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PgTenancyTransaction")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl TenancyUnitOfWork for PgTenancyUnitOfWork {
    async fn begin_mutation(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Box<dyn TenancyTransaction>, TenancyError> {
        let mut tx = self.pool.begin().await.map_err(tenancy_db_error)?;
        set_tenant_context(&mut tx, organization_id)
            .await
            .map_err(tenancy_scope_error)?;
        Ok(Box::new(PgTenancyTransaction {
            tx,
            pool: self.pool.clone(),
        }))
    }

    async fn begin_authorized_mutation(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Box<dyn TenancyTransaction>, ScopeError> {
        let mut tx = self.pool.begin().await.map_err(scope_db_error)?;
        query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *tx)
            .await
            .map_err(scope_db_error)?;
        set_tenant_context(&mut tx, organization_id).await?;
        Ok(Box::new(PgTenancyTransaction {
            tx,
            pool: self.pool.clone(),
        }))
    }

    async fn begin_scope(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Box<dyn TenancyTransaction>, ScopeError> {
        let mut tx = self.pool.begin().await.map_err(scope_db_error)?;
        query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(scope_db_error)?;
        set_tenant_context(&mut tx, organization_id).await?;
        Ok(Box::new(PgTenancyTransaction {
            tx,
            pool: self.pool.clone(),
        }))
    }

    async fn begin_invitation(&self) -> Result<Box<dyn TenancyTransaction>, InvitationError> {
        let tx = self.pool.begin().await.map_err(invitation_db_error)?;
        Ok(Box::new(PgTenancyTransaction {
            tx,
            pool: self.pool.clone(),
        }))
    }

    async fn record_evidence(
        &self,
        audit: &IamAuditEvent,
        outbox: &OutboxEvent,
    ) -> Result<(), TenancyError> {
        let mut tx = self.pool.begin().await.map_err(tenancy_db_error)?;
        PostgresAuditWriter::new()
            .record_required(&mut tx, audit)
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
        PostgresOutboxRepository::new(self.pool.clone())
            .enqueue(&mut tx, outbox)
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
        tx.commit().await.map_err(tenancy_db_error)
    }

    async fn read_member_roles(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Vec<RoleAssignment>, ScopeError> {
        let mut tx = self.pool.begin().await.map_err(scope_db_error)?;
        query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(scope_db_error)?;
        set_tenant_context(&mut tx, organization_id).await?;
        let roles = persisted_role_assignments(&mut tx, organization_id, membership_id).await?;
        tx.commit().await.map_err(scope_db_error)?;
        Ok(roles)
    }

    async fn read_member_grants(
        &self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Vec<ScopeGrant>, ScopeError> {
        let mut tx = self.pool.begin().await.map_err(scope_db_error)?;
        query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(scope_db_error)?;
        set_tenant_context(&mut tx, organization_id).await?;
        let grants = persisted_scope_grants(&mut tx, organization_id, membership_id).await?;
        tx.commit().await.map_err(scope_db_error)?;
        Ok(grants)
    }
}

#[async_trait]
impl TenancyTransaction for PgTenancyTransaction {
    async fn set_tenant_context(
        &mut self,
        organization_id: &OrganizationId,
    ) -> Result<(), ScopeError> {
        set_tenant_context(&mut self.tx, organization_id).await
    }

    async fn lock_organization(
        &mut self,
        organization_id: &OrganizationId,
    ) -> Result<Option<String>, TenancyError> {
        query_scalar("SELECT lock_organization_mutations($1)")
            .bind(organization_id.as_str())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(tenancy_db_error)
    }

    async fn organization_state(
        &mut self,
        organization_id: &OrganizationId,
    ) -> Result<Option<String>, ScopeError> {
        query_scalar("SELECT state FROM organizations WHERE id = $1")
            .bind(organization_id.as_str())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(scope_db_error)
    }

    async fn load_scope_material(
        &mut self,
        organization_id: &OrganizationId,
        selector: MembershipSelector<'_>,
        branch_id: Option<&BranchId>,
    ) -> Result<ScopeMaterial, ScopeError> {
        let (membership_id, user_id) = match selector {
            MembershipSelector::Id(id) => (Some(id.as_str()), None),
            MembershipSelector::User(id) => (None, Some(id.as_str())),
        };
        let row: Option<PersistedScopeRow> = query_as(
            "SELECT o.name, o.state, o.state_version, m.id, m.user_id, m.state, m.state_version, \
             b.id, b.name, b.state, b.state_version \
             FROM organizations o \
             JOIN organization_memberships m ON m.organization_id = o.id \
             LEFT JOIN branches b ON b.organization_id = o.id AND b.id = $3 \
             WHERE o.id = $1 AND \
             (($2::TEXT IS NOT NULL AND m.id = $2) OR ($4::TEXT IS NOT NULL AND m.user_id = $4))",
        )
        .bind(organization_id.as_str())
        .bind(membership_id)
        .bind(branch_id.map(BranchId::as_str))
        .bind(user_id)
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(scope_db_error)?;
        let Some((
            organization_name,
            organization_state,
            organization_version,
            membership_id,
            membership_user_id,
            membership_state,
            membership_version,
            branch_id_raw,
            branch_name,
            branch_state,
            branch_version,
        )) = row
        else {
            return Err(ScopeError::OrganizationDenied);
        };

        let organization = Organization {
            id: organization_id.clone(),
            name: organization_name,
            state: organization_state_from_str(&organization_state)
                .ok_or(ScopeError::AuthorityUnavailable)?,
            state_version: u64::try_from(organization_version)
                .map_err(|_error| ScopeError::AuthorityUnavailable)?,
        };
        let membership = Membership {
            id: MembershipId::new(membership_id)
                .map_err(|_error| ScopeError::AuthorityUnavailable)?,
            organization_id: organization_id.clone(),
            user_id: TenantUserId::new(membership_user_id)
                .map_err(|_error| ScopeError::AuthorityUnavailable)?,
            state: membership_state_from_str(&membership_state)
                .ok_or(ScopeError::AuthorityUnavailable)?,
            state_version: u64::try_from(membership_version)
                .map_err(|_error| ScopeError::AuthorityUnavailable)?,
        };
        let branch = match (
            branch_id,
            branch_id_raw,
            branch_name,
            branch_state,
            branch_version,
        ) {
            (None, None, None, None, None) => None,
            (Some(_), Some(id), Some(name), Some(state), Some(version)) => Some(Branch {
                id: BranchId::new(id).map_err(|_error| ScopeError::AuthorityUnavailable)?,
                organization_id: organization_id.clone(),
                name,
                state: branch_state_from_str(&state).ok_or(ScopeError::AuthorityUnavailable)?,
                state_version: u64::try_from(version)
                    .map_err(|_error| ScopeError::AuthorityUnavailable)?,
            }),
            (Some(_), None, None, None, None) => return Err(ScopeError::BranchDenied),
            _ => return Err(ScopeError::AuthorityUnavailable),
        };

        let role_assignments =
            persisted_role_assignments(&mut self.tx, organization_id, &membership.id).await?;
        let scope_grants =
            persisted_scope_grants(&mut self.tx, organization_id, &membership.id).await?;

        Ok(ScopeMaterial {
            organization,
            membership,
            branch,
            role_assignments,
            scope_grants,
        })
    }

    async fn membership_record(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Option<MembershipRecord>, TenancyError> {
        let row: Option<(String, String, i64)> = query_as(
            "SELECT user_id, state, state_version FROM organization_memberships \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(tenancy_db_error)?;
        let Some((user_id, state, state_version)) = row else {
            return Ok(None);
        };
        Ok(Some(MembershipRecord {
            user_id: TenantUserId::new(user_id)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
            state: membership_state_from_str(&state)
                .ok_or(TenancyError::EvidencePersistenceFailed)?,
            state_version: u64::try_from(state_version)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
        }))
    }

    async fn transition_membership(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        from: &str,
        to: &str,
    ) -> Result<Membership, TenancyError> {
        let changed: Option<(String, String, i64)> = query_as(
            "UPDATE organization_memberships \
             SET state = $4, state_version = state_version + 1 \
             WHERE organization_id = $1 AND id = $2 AND state = $3 \
             RETURNING user_id, state, state_version",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(from)
        .bind(to)
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(tenancy_db_error)?;
        let Some((user_id, state, state_version)) = changed else {
            return Err(TenancyError::InvalidTransition);
        };
        Ok(Membership {
            id: membership_id.clone(),
            organization_id: organization_id.clone(),
            user_id: TenantUserId::new(user_id)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
            state: membership_state_from_str(&state)
                .ok_or(TenancyError::EvidencePersistenceFailed)?,
            state_version: u64::try_from(state_version)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
        })
    }

    async fn insert_membership(&mut self, membership: &Membership) -> Result<(), TenancyError> {
        let state_version = i64::try_from(membership.state_version)
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
        query(
            "INSERT INTO organization_memberships \
             (id, organization_id, user_id, state, state_version) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(membership.id.as_str())
        .bind(membership.organization_id.as_str())
        .bind(membership.user_id.as_str())
        .bind(membership_state(membership.state))
        .bind(state_version)
        .execute(&mut *self.tx)
        .await
        .map_err(tenancy_insert_error)?;
        Ok(())
    }

    async fn role_assignment(
        &mut self,
        assignment_id: &RoleAssignmentId,
    ) -> Result<Option<RoleAssignment>, TenancyError> {
        let row: Option<(String, String, String, String, i64)> = query_as(
            "SELECT organization_id, membership_id, role, state, state_version \
             FROM role_assignments WHERE id = $1",
        )
        .bind(assignment_id.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(tenancy_db_error)?;
        let Some((organization_id, membership_id, role, state, state_version)) = row else {
            return Ok(None);
        };
        Ok(Some(RoleAssignment {
            id: assignment_id.clone(),
            organization_id: OrganizationId::new(organization_id)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
            membership_id: MembershipId::new(membership_id)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
            role: role_from_str(&role).ok_or(TenancyError::EvidencePersistenceFailed)?,
            state: role_assignment_state_from_str(&state)
                .ok_or(TenancyError::EvidencePersistenceFailed)?,
            state_version: u64::try_from(state_version)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
        }))
    }

    async fn insert_role_assignment(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        assignment_id: &RoleAssignmentId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError> {
        if let Some(existing) = self.role_assignment(assignment_id).await? {
            if existing.organization_id != *organization_id
                || existing.membership_id != *membership_id
                || existing.role != role
            {
                return Err(TenancyError::Conflict);
            }
            return Ok(existing);
        }

        let state_version: i64 = query_scalar(
            "INSERT INTO role_assignments \
             (id, organization_id, membership_id, role, state, state_version) \
             VALUES ($1, $2, $3, $4, 'EFFECTIVE', 3) \
             RETURNING state_version",
        )
        .bind(assignment_id.as_str())
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(role.as_str())
        .fetch_one(&mut *self.tx)
        .await
        .map_err(tenancy_insert_error)?;
        Ok(RoleAssignment {
            id: assignment_id.clone(),
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            role,
            state: RoleAssignmentState::Effective,
            state_version: u64::try_from(state_version)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
        })
    }

    async fn revoke_role_assignment(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        role: Role,
    ) -> Result<RoleAssignment, TenancyError> {
        let changed: Option<(String, i64)> = query_as(
            "UPDATE role_assignments \
             SET state = 'REVOKED', state_version = state_version + 1 \
             WHERE organization_id = $1 AND membership_id = $2 \
               AND role = $3 AND state = 'EFFECTIVE' \
             RETURNING id, state_version",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(role.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(tenancy_db_error)?;
        let Some((assignment_id, version)) = changed else {
            return Err(TenancyError::NotFound);
        };
        Ok(RoleAssignment {
            id: RoleAssignmentId::new(assignment_id)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            role,
            state: RoleAssignmentState::Revoked,
            state_version: u64::try_from(version)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
        })
    }

    async fn scope_grant(
        &mut self,
        grant_id: &ScopeGrantId,
    ) -> Result<Option<ScopeGrant>, TenancyError> {
        let row: Option<(String, String, String, Option<String>, String, i64)> = query_as(
            "SELECT organization_id, membership_id, scope_kind, branch_id, state, state_version \
             FROM scope_grants WHERE id = $1",
        )
        .bind(grant_id.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(tenancy_db_error)?;
        let Some((organization_id, membership_id, scope_kind, branch_id, state, state_version)) =
            row
        else {
            return Ok(None);
        };
        let organization_id = OrganizationId::new(organization_id)
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
        let scope = scope_from_row(&organization_id, &scope_kind, branch_id.as_deref())
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        Ok(Some(ScopeGrant {
            id: grant_id.clone(),
            organization_id,
            membership_id: MembershipId::new(membership_id)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
            scope,
            state: scope_grant_state_from_str(&state)
                .ok_or(TenancyError::EvidencePersistenceFailed)?,
            state_version: u64::try_from(state_version)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
        }))
    }

    async fn insert_scope_grant(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        grant_id: &ScopeGrantId,
        scope: &Scope,
    ) -> Result<ScopeGrant, TenancyError> {
        let (scope_kind, branch_id) = scope_columns(scope);
        if let Some(existing) = self.scope_grant(grant_id).await? {
            if existing.organization_id != *organization_id
                || existing.membership_id != *membership_id
                || existing.scope != *scope
            {
                return Err(TenancyError::Conflict);
            }
            return Ok(existing);
        }
        let version: i64 = query_scalar(
            "INSERT INTO scope_grants \
             (id, organization_id, membership_id, scope_kind, branch_id, state, state_version) \
             VALUES ($1, $2, $3, $4, $5, 'ACTIVE', 1) RETURNING state_version",
        )
        .bind(grant_id.as_str())
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(scope_kind)
        .bind(branch_id)
        .fetch_one(&mut *self.tx)
        .await
        .map_err(tenancy_insert_error)?;
        Ok(ScopeGrant {
            id: grant_id.clone(),
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            scope: scope.clone(),
            state: ScopeGrantState::Active,
            state_version: u64::try_from(version)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
        })
    }

    async fn revoke_scope_grant(
        &mut self,
        organization_id: &OrganizationId,
        grant_id: &ScopeGrantId,
    ) -> Result<ScopeGrant, TenancyError> {
        let row: Option<(String, String, Option<String>, String, i64)> = query_as(
            "UPDATE scope_grants SET state = 'REVOKED', state_version = state_version + 1 \
             WHERE organization_id = $1 AND id = $2 AND state = 'ACTIVE' \
             RETURNING membership_id, scope_kind, branch_id, state, state_version",
        )
        .bind(organization_id.as_str())
        .bind(grant_id.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(tenancy_db_error)?;
        let Some((membership_raw, kind, branch, state, version)) = row else {
            return Err(TenancyError::NotFound);
        };
        let scope = scope_from_row(organization_id, &kind, branch.as_deref())
            .ok_or(TenancyError::EvidencePersistenceFailed)?;
        Ok(ScopeGrant {
            id: grant_id.clone(),
            organization_id: organization_id.clone(),
            membership_id: MembershipId::new(membership_raw)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
            scope,
            state: scope_grant_state_from_str(&state)
                .ok_or(TenancyError::EvidencePersistenceFailed)?,
            state_version: u64::try_from(version)
                .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
        })
    }

    async fn insert_branch(&mut self, branch: &Branch) -> Result<(), TenancyError> {
        let state_version = i64::try_from(branch.state_version)
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
        query(
            "INSERT INTO branches (id, organization_id, name, state, state_version) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(branch.id.as_str())
        .bind(branch.organization_id.as_str())
        .bind(&branch.name)
        .bind(branch_state(branch.state))
        .bind(state_version)
        .execute(&mut *self.tx)
        .await
        .map_err(tenancy_insert_error)?;
        Ok(())
    }

    async fn provision_organization(
        &mut self,
        provisioned: &ProvisionedOrganization,
    ) -> Result<bool, TenancyError> {
        insert_provisioned_organization(
            &mut self.tx,
            &provisioned.organization,
            &provisioned.owner_membership,
            &provisioned.default_branch,
        )
        .await
        .map_err(|error| match error {
            TenancyProvisioningError::Conflict => TenancyError::Conflict,
            TenancyProvisioningError::Database(_) => TenancyError::EvidencePersistenceFailed,
        })
    }

    async fn transition_organization(
        &mut self,
        from: &str,
        to: &str,
    ) -> Result<Option<(String, u64)>, TenancyError> {
        let changed: Option<(String, i64)> =
            query_as("SELECT name, state_version FROM transition_organization_state($1, $2)")
                .bind(from)
                .bind(to)
                .fetch_optional(&mut *self.tx)
                .await
                .map_err(tenancy_db_error)?;
        changed
            .map(|(name, version)| {
                Ok((
                    name,
                    u64::try_from(version)
                        .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
                ))
            })
            .transpose()
    }

    async fn transition_branch(
        &mut self,
        branch_id: &BranchId,
        from: &str,
        to: &str,
    ) -> Result<Option<(String, u64)>, TenancyError> {
        let changed: Option<(String, i64)> =
            query_as("SELECT name, state_version FROM transition_branch_state($1, $2, $3)")
                .bind(branch_id.as_str())
                .bind(from)
                .bind(to)
                .fetch_optional(&mut *self.tx)
                .await
                .map_err(tenancy_db_error)?;
        changed
            .map(|(name, version)| {
                Ok((
                    name,
                    u64::try_from(version)
                        .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
                ))
            })
            .transpose()
    }

    async fn resolve_invitation_organization(
        &mut self,
        token_hash: &[u8; 32],
    ) -> Result<Option<OrganizationId>, InvitationError> {
        let organization_raw: Option<String> =
            query_scalar("SELECT resolve_invitation_organization($1)")
                .bind(token_hash.to_vec())
                .fetch_one(&mut *self.tx)
                .await
                .map_err(invitation_db_error)?;
        organization_raw
            .map(|value| {
                OrganizationId::new(value)
                    .map_err(|_error| InvitationError::EvidencePersistenceFailed)
            })
            .transpose()
    }

    async fn lock_issued_invitation(
        &mut self,
        organization_id: &OrganizationId,
        token_hash: &[u8; 32],
    ) -> Result<Option<InvitationRecord>, InvitationError> {
        let row: Option<(
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            DateTime<Utc>,
            DateTime<Utc>,
            String,
            i64,
        )> = query_as(
            "SELECT id, membership_id, proposed_role, scope_kind, branch_id, issued_at, \
                 expires_at, state, state_version FROM invitations \
                 WHERE organization_id = $1 AND token_hash = $2 AND state = 'ISSUED' FOR UPDATE",
        )
        .bind(organization_id.as_str())
        .bind(token_hash.to_vec())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(invitation_db_error)?;
        row.map(|value| invitation_record(organization_id, value))
            .transpose()
    }

    async fn lock_invitation(
        &mut self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<Option<InvitationRecord>, InvitationError> {
        let row: Option<(
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            DateTime<Utc>,
            DateTime<Utc>,
            String,
            i64,
        )> = query_as(
            "SELECT id, membership_id, proposed_role, scope_kind, branch_id, issued_at, \
                 expires_at, state, state_version FROM invitations \
                 WHERE organization_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(organization_id.as_str())
        .bind(invitation_id.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(invitation_db_error)?;
        row.map(|value| invitation_record(organization_id, value))
            .transpose()
    }

    async fn insert_invitation(&mut self, invitation: &Invitation) -> Result<(), TenancyError> {
        let (scope_kind, branch_id) = invitation
            .proposed_scope
            .as_ref()
            .map_or((None, None), |scope| scope_columns(scope));
        let (contact_kind, contact_value) = match &invitation.contact {
            Some(contact) => (
                Some(match contact.kind {
                    ContactKind::Email => "EMAIL",
                    ContactKind::Phone => "PHONE",
                }),
                Some(contact.value.as_str()),
            ),
            None => (None, None),
        };
        let token_hash = invitation
            .token_hash
            .ok_or(TenancyError::InvalidInvitation)?;
        let issued_at =
            system_time_to_utc(invitation.issued_at).ok_or(TenancyError::InvalidInvitation)?;
        let expires_at =
            system_time_to_utc(invitation.expires_at).ok_or(TenancyError::InvalidInvitation)?;
        let state_version = i64::try_from(invitation.state_version)
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
        query(
            "INSERT INTO invitations \
             (id, organization_id, membership_id, proposed_role, scope_kind, branch_id, \
              contact_kind, contact_value, token_hash, state, issued_at, expires_at, state_version) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(invitation.id.as_str())
        .bind(invitation.organization_id.as_str())
        .bind(invitation.membership_id.as_str())
        .bind(invitation.proposed_role.as_str())
        .bind(scope_kind)
        .bind(branch_id)
        .bind(contact_kind)
        .bind(contact_value)
        .bind(token_hash.to_vec())
        .bind(invitation_state(invitation.state))
        .bind(issued_at)
        .bind(expires_at)
        .bind(state_version)
        .execute(&mut *self.tx)
        .await
        .map_err(tenancy_insert_error)?;
        Ok(())
    }

    async fn transition_invitation_membership(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        from: &str,
        to: &str,
    ) -> Result<u64, InvitationError> {
        let version: Option<i64> = query_scalar(
            "UPDATE organization_memberships SET state = $4, state_version = state_version + 1 \
             WHERE organization_id = $1 AND id = $2 AND state = $3 RETURNING state_version",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(from)
        .bind(to)
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(invitation_db_error)?;
        version.ok_or(InvitationError::Invalid).and_then(|value| {
            u64::try_from(value).map_err(|_error| InvitationError::EvidencePersistenceFailed)
        })
    }

    async fn insert_invitation_role_assignment(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        assignment_id: &RoleAssignmentId,
        role: Role,
    ) -> Result<RoleAssignment, InvitationError> {
        let assignment_version: i64 = query_scalar(
            "INSERT INTO role_assignments \
             (id, organization_id, membership_id, role, state, state_version) \
             VALUES ($1, $2, $3, $4, 'EFFECTIVE', 3) RETURNING state_version",
        )
        .bind(assignment_id.as_str())
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(role.as_str())
        .fetch_one(&mut *self.tx)
        .await
        .map_err(invitation_insert_error)?;
        Ok(RoleAssignment {
            id: assignment_id.clone(),
            organization_id: organization_id.clone(),
            membership_id: membership_id.clone(),
            role,
            state: RoleAssignmentState::Effective,
            state_version: u64::try_from(assignment_version)
                .map_err(|_error| InvitationError::EvidencePersistenceFailed)?,
        })
    }

    async fn insert_invitation_scope_grant(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
        grant_id: &ScopeGrantId,
        scope: &Scope,
    ) -> Result<ScopeGrant, InvitationError> {
        let (kind, branch_id) = scope_columns(scope);
        query(
            "INSERT INTO scope_grants \
             (id, organization_id, membership_id, scope_kind, branch_id, state, state_version) \
             VALUES ($1, $2, $3, $4, $5, 'ACTIVE', 1)",
        )
        .bind(grant_id.as_str())
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .bind(kind)
        .bind(branch_id)
        .execute(&mut *self.tx)
        .await
        .map_err(invitation_insert_error)?;
        Ok(ScopeGrant::grant(
            grant_id.clone(),
            organization_id.clone(),
            membership_id.clone(),
            scope.clone(),
        ))
    }

    async fn accept_invitation(
        &mut self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<u64, InvitationError> {
        let version: Option<i64> = query_scalar(
            "UPDATE invitations SET state = 'ACCEPTED', state_version = state_version + 1, \
             contact_kind = NULL, contact_value = NULL, token_hash = NULL \
             WHERE organization_id = $1 AND id = $2 AND state = 'ISSUED' \
             RETURNING state_version",
        )
        .bind(organization_id.as_str())
        .bind(invitation_id.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(invitation_db_error)?;
        version.ok_or(InvitationError::Invalid).and_then(|value| {
            u64::try_from(value).map_err(|_error| InvitationError::EvidencePersistenceFailed)
        })
    }

    async fn withdraw_invitation(
        &mut self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<u64, InvitationError> {
        let version: Option<i64> = query_scalar(
            "UPDATE invitations SET state = 'WITHDRAWN', state_version = state_version + 1, \
             contact_kind = NULL, contact_value = NULL, token_hash = NULL \
             WHERE organization_id = $1 AND id = $2 AND state = 'ISSUED' \
             RETURNING state_version",
        )
        .bind(organization_id.as_str())
        .bind(invitation_id.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(invitation_db_error)?;
        version.ok_or(InvitationError::Invalid).and_then(|value| {
            u64::try_from(value).map_err(|_error| InvitationError::EvidencePersistenceFailed)
        })
    }

    async fn expire_linked_membership(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<(), InvitationError> {
        query(
            "UPDATE organization_memberships SET state = 'EXPIRED', \
             state_version = state_version + 2 \
             WHERE organization_id = $1 AND id = $2 AND state = 'INVITED'",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(invitation_db_error)?;
        Ok(())
    }

    async fn expired_invitation_batch(
        &mut self,
        organization_id: &OrganizationId,
        limit: i64,
    ) -> Result<Vec<ExpiredInvitation>, TenancyError> {
        let candidates: Vec<(String, String, DateTime<Utc>)> = query_as(
            "SELECT id, membership_id, expires_at FROM invitations \
             WHERE organization_id = $1 AND state = 'ISSUED' AND expires_at <= now() \
             ORDER BY expires_at, id LIMIT $2 FOR UPDATE SKIP LOCKED",
        )
        .bind(organization_id.as_str())
        .bind(limit)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(tenancy_db_error)?;
        candidates
            .into_iter()
            .map(|(invitation_id, membership_id, expires_at)| {
                Ok(ExpiredInvitation {
                    invitation_id: InvitationId::new(invitation_id)
                        .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
                    membership_id: MembershipId::new(membership_id)
                        .map_err(|_error| TenancyError::EvidencePersistenceFailed)?,
                    expires_at: utc_to_system_time(expires_at)
                        .ok_or(TenancyError::EvidencePersistenceFailed)?,
                })
            })
            .collect()
    }

    async fn expire_invitation(
        &mut self,
        organization_id: &OrganizationId,
        invitation_id: &InvitationId,
    ) -> Result<Option<u64>, TenancyError> {
        let version: Option<i64> = query_scalar(
            "UPDATE invitations SET state = 'EXPIRED', state_version = state_version + 1, \
             contact_kind = NULL, contact_value = NULL, token_hash = NULL \
             WHERE organization_id = $1 AND id = $2 AND state = 'ISSUED' \
               AND expires_at <= now() RETURNING state_version",
        )
        .bind(organization_id.as_str())
        .bind(invitation_id.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(tenancy_db_error)?;
        version
            .map(|value| {
                u64::try_from(value).map_err(|_error| TenancyError::EvidencePersistenceFailed)
            })
            .transpose()
    }

    async fn expire_invited_membership(
        &mut self,
        organization_id: &OrganizationId,
        membership_id: &MembershipId,
    ) -> Result<Option<u64>, TenancyError> {
        let version: Option<i64> = query_scalar(
            "UPDATE organization_memberships SET state = 'EXPIRED', \
             state_version = state_version + 1 \
             WHERE organization_id = $1 AND id = $2 AND state = 'INVITED' \
             RETURNING state_version",
        )
        .bind(organization_id.as_str())
        .bind(membership_id.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(tenancy_db_error)?;
        version
            .map(|value| {
                u64::try_from(value).map_err(|_error| TenancyError::EvidencePersistenceFailed)
            })
            .transpose()
    }

    async fn record_evidence(
        &mut self,
        audit: &IamAuditEvent,
        outbox: &OutboxEvent,
    ) -> Result<(), TenancyError> {
        PostgresAuditWriter::new()
            .record_required(&mut self.tx, audit)
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
        PostgresOutboxRepository::new(self.pool.clone())
            .enqueue(&mut self.tx, outbox)
            .await
            .map_err(|_error| TenancyError::EvidencePersistenceFailed)?;
        Ok(())
    }

    async fn commit(self: Box<Self>) -> Result<(), TenancyError> {
        let PgTenancyTransaction { tx, .. } = *self;
        tx.commit().await.map_err(tenancy_db_error)
    }
}

async fn set_tenant_context(
    tx: &mut Transaction<'static, Postgres>,
    organization_id: &OrganizationId,
) -> Result<(), ScopeError> {
    query("SELECT set_config('app.organization_id', $1, true)")
        .bind(organization_id.as_str())
        .execute(&mut **tx)
        .await
        .map_err(scope_db_error)?;
    Ok(())
}

async fn persisted_role_assignments(
    tx: &mut Transaction<'static, Postgres>,
    organization_id: &OrganizationId,
    membership_id: &MembershipId,
) -> Result<Vec<RoleAssignment>, ScopeError> {
    let rows: Vec<PersistedRoleRow> = query_as(
        "SELECT id, role, state, state_version FROM role_assignments \
         WHERE organization_id = $1 AND membership_id = $2 ORDER BY id",
    )
    .bind(organization_id.as_str())
    .bind(membership_id.as_str())
    .fetch_all(&mut **tx)
    .await
    .map_err(scope_db_error)?;
    rows.into_iter()
        .map(|(id, role, state, state_version)| {
            Ok(RoleAssignment {
                id: RoleAssignmentId::new(id).map_err(|_error| ScopeError::AuthorityUnavailable)?,
                organization_id: organization_id.clone(),
                membership_id: membership_id.clone(),
                role: role_from_str(&role).ok_or(ScopeError::AuthorityUnavailable)?,
                state: role_assignment_state_from_str(&state)
                    .ok_or(ScopeError::AuthorityUnavailable)?,
                state_version: u64::try_from(state_version)
                    .map_err(|_error| ScopeError::AuthorityUnavailable)?,
            })
        })
        .collect()
}

async fn persisted_scope_grants(
    tx: &mut Transaction<'static, Postgres>,
    organization_id: &OrganizationId,
    membership_id: &MembershipId,
) -> Result<Vec<ScopeGrant>, ScopeError> {
    let rows: Vec<PersistedGrantRow> = query_as(
        "SELECT id, scope_kind, branch_id, state, state_version FROM scope_grants \
         WHERE organization_id = $1 AND membership_id = $2 ORDER BY id",
    )
    .bind(organization_id.as_str())
    .bind(membership_id.as_str())
    .fetch_all(&mut **tx)
    .await
    .map_err(scope_db_error)?;
    rows.into_iter()
        .map(|(id, scope_kind, branch, state, state_version)| {
            Ok(ScopeGrant {
                id: ScopeGrantId::new(id).map_err(|_error| ScopeError::AuthorityUnavailable)?,
                organization_id: organization_id.clone(),
                membership_id: membership_id.clone(),
                scope: scope_from_row(organization_id, &scope_kind, branch.as_deref())
                    .ok_or(ScopeError::AuthorityUnavailable)?,
                state: scope_grant_state_from_str(&state)
                    .ok_or(ScopeError::AuthorityUnavailable)?,
                state_version: u64::try_from(state_version)
                    .map_err(|_error| ScopeError::AuthorityUnavailable)?,
            })
        })
        .collect()
}

/// Row tuple selected by the invitation query, in column order.
type InvitationRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
    DateTime<Utc>,
    String,
    i64,
);

fn invitation_record(
    organization_id: &OrganizationId,
    value: InvitationRow,
) -> Result<InvitationRecord, InvitationError> {
    let (id, membership_id, role, scope_kind, branch_id, issued_at, expires_at, state, version) =
        value;
    let proposed_scope = match (scope_kind.as_deref(), branch_id.as_deref()) {
        (None, None) => None,
        (Some("ORGANIZATION"), None) => Some(Scope::Organization {
            organization_id: organization_id.clone(),
        }),
        (Some("BRANCH"), Some(branch)) => Some(Scope::Branch {
            organization_id: organization_id.clone(),
            branch_id: BranchId::new(branch.to_owned())
                .map_err(|_error| InvitationError::EvidencePersistenceFailed)?,
        }),
        _ => return Err(InvitationError::EvidencePersistenceFailed),
    };
    Ok(InvitationRecord {
        id: InvitationId::new(id).map_err(|_error| InvitationError::EvidencePersistenceFailed)?,
        membership_id: MembershipId::new(membership_id)
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?,
        proposed_role: role_from_str(&role).ok_or(InvitationError::EvidencePersistenceFailed)?,
        proposed_scope,
        state: invitation_state_from_str(&state)
            .ok_or(InvitationError::EvidencePersistenceFailed)?,
        issued_at: utc_to_system_time(issued_at)
            .ok_or(InvitationError::EvidencePersistenceFailed)?,
        expires_at: utc_to_system_time(expires_at)
            .ok_or(InvitationError::EvidencePersistenceFailed)?,
        state_version: u64::try_from(version)
            .map_err(|_error| InvitationError::EvidencePersistenceFailed)?,
    })
}

fn scope_columns(scope: &Scope) -> (Option<&'static str>, Option<&str>) {
    match scope {
        Scope::Organization { .. } => (Some("ORGANIZATION"), None),
        Scope::Branch { branch_id, .. } => (Some("BRANCH"), Some(branch_id.as_str())),
    }
}

fn scope_from_row(
    organization_id: &OrganizationId,
    scope_kind: &str,
    branch_id: Option<&str>,
) -> Option<Scope> {
    match (scope_kind, branch_id) {
        ("ORGANIZATION", None) => Some(Scope::Organization {
            organization_id: organization_id.clone(),
        }),
        ("BRANCH", Some(branch)) => Some(Scope::Branch {
            organization_id: organization_id.clone(),
            branch_id: BranchId::new(branch.to_owned()).ok()?,
        }),
        _ => None,
    }
}

fn role_from_str(value: &str) -> Option<Role> {
    Role::all()
        .iter()
        .copied()
        .find(|candidate| candidate.as_str() == value)
}

fn role_assignment_state_from_str(value: &str) -> Option<RoleAssignmentState> {
    match value {
        "EFFECTIVE" => Some(RoleAssignmentState::Effective),
        "REVOKED" => Some(RoleAssignmentState::Revoked),
        _ => None,
    }
}

fn scope_grant_state_from_str(value: &str) -> Option<ScopeGrantState> {
    match value {
        "ACTIVE" => Some(ScopeGrantState::Active),
        "REVOKED" => Some(ScopeGrantState::Revoked),
        _ => None,
    }
}

fn invitation_state_from_str(value: &str) -> Option<InvitationState> {
    match value {
        "ISSUED" => Some(InvitationState::Issued),
        "ACCEPTED" => Some(InvitationState::Accepted),
        "WITHDRAWN" => Some(InvitationState::Revoked),
        "EXPIRED" => Some(InvitationState::Expired),
        _ => None,
    }
}

fn invitation_state(state: InvitationState) -> &'static str {
    match state {
        InvitationState::Issued => "ISSUED",
        InvitationState::Accepted => "ACCEPTED",
        InvitationState::Revoked => "WITHDRAWN",
        InvitationState::Expired => "EXPIRED",
    }
}

fn organization_state_from_str(value: &str) -> Option<OrganizationState> {
    match value {
        "PROVISIONING" => Some(OrganizationState::Provisioning),
        "ACTIVE" => Some(OrganizationState::Active),
        "SUSPENDED" => Some(OrganizationState::Suspended),
        "CLOSING" => Some(OrganizationState::Closing),
        "CLOSED" => Some(OrganizationState::Closed),
        _ => None,
    }
}

fn membership_state_from_str(value: &str) -> Option<MembershipState> {
    match value {
        "INVITED" => Some(MembershipState::Invited),
        "PENDING_ACCEPTANCE" => Some(MembershipState::PendingAcceptance),
        "EXPIRED" => Some(MembershipState::Expired),
        "ACTIVE" => Some(MembershipState::Active),
        "SUSPENDED" => Some(MembershipState::Suspended),
        "REVOKED" => Some(MembershipState::Revoked),
        _ => None,
    }
}

fn branch_state_from_str(value: &str) -> Option<BranchState> {
    match value {
        "PROVISIONING" => Some(BranchState::Provisioning),
        "ACTIVE" => Some(BranchState::Active),
        "SUSPENDED" => Some(BranchState::Suspended),
        "CLOSING" => Some(BranchState::Closing),
        "CLOSED" => Some(BranchState::Closed),
        _ => None,
    }
}

fn membership_state(state: MembershipState) -> &'static str {
    match state {
        MembershipState::Invited => "INVITED",
        MembershipState::PendingAcceptance => "PENDING_ACCEPTANCE",
        MembershipState::Expired => "EXPIRED",
        MembershipState::Active => "ACTIVE",
        MembershipState::Suspended => "SUSPENDED",
        MembershipState::Revoked => "REVOKED",
    }
}

fn branch_state(state: BranchState) -> &'static str {
    match state {
        BranchState::Provisioning => "PROVISIONING",
        BranchState::Active => "ACTIVE",
        BranchState::Suspended => "SUSPENDED",
        BranchState::Closing => "CLOSING",
        BranchState::Closed => "CLOSED",
    }
}

fn system_time_to_utc(value: SystemTime) -> Option<DateTime<Utc>> {
    let duration = value.duration_since(UNIX_EPOCH).ok()?;
    let seconds = i64::try_from(duration.as_secs()).ok()?;
    DateTime::from_timestamp(seconds, duration.subsec_nanos())
}

fn utc_to_system_time(value: DateTime<Utc>) -> Option<SystemTime> {
    let seconds = value.timestamp();
    let nanos = value.timestamp_subsec_nanos();
    if seconds >= 0 {
        UNIX_EPOCH.checked_add(Duration::new(u64::try_from(seconds).ok()?, nanos))
    } else {
        UNIX_EPOCH
            .checked_sub(Duration::new(seconds.unsigned_abs(), 0))
            .and_then(|time| time.checked_add(Duration::new(0, nanos)))
    }
}

fn tenancy_db_error(error: SqlxError) -> TenancyError {
    let sqlstate = if let SqlxError::Database(database_error) = error {
        database_error.code().map(std::borrow::Cow::into_owned)
    } else {
        None
    };
    TenancyError::EvidencePersistenceFailedAt {
        stage: "tenancy_transaction",
        sqlstate,
    }
}

fn tenancy_insert_error(error: SqlxError) -> TenancyError {
    let unique_violation = if let SqlxError::Database(database_error) = &error {
        database_error.code().as_deref() == Some("23505")
    } else {
        false
    };
    if unique_violation {
        TenancyError::Conflict
    } else {
        tenancy_db_error(error)
    }
}

fn scope_db_error(_error: SqlxError) -> ScopeError {
    ScopeError::AuthorityUnavailable
}

fn invitation_db_error(_error: SqlxError) -> InvitationError {
    InvitationError::EvidencePersistenceFailed
}

fn invitation_insert_error(error: SqlxError) -> InvitationError {
    if let SqlxError::Database(database_error) = error
        && matches!(database_error.code().as_deref(), Some("23505" | "23503"))
    {
        return InvitationError::Invalid;
    }
    InvitationError::EvidencePersistenceFailed
}

fn tenancy_scope_error(_error: ScopeError) -> TenancyError {
    TenancyError::EvidencePersistenceFailed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_assignment_state_rejects_unknown_values() {
        assert_eq!(
            role_assignment_state_from_str("EFFECTIVE"),
            Some(RoleAssignmentState::Effective)
        );
        assert_eq!(
            role_assignment_state_from_str("REVOKED"),
            Some(RoleAssignmentState::Revoked)
        );
        assert_eq!(role_assignment_state_from_str("effective"), None);
        assert_eq!(role_assignment_state_from_str("UNKNOWN"), None);
    }

    #[test]
    fn scope_grant_state_rejects_unknown_values() {
        assert_eq!(
            scope_grant_state_from_str("ACTIVE"),
            Some(ScopeGrantState::Active)
        );
        assert_eq!(
            scope_grant_state_from_str("REVOKED"),
            Some(ScopeGrantState::Revoked)
        );
        assert_eq!(scope_grant_state_from_str("active"), None);
        assert_eq!(scope_grant_state_from_str("UNKNOWN"), None);
    }
}
