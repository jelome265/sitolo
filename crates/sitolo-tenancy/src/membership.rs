//! Membership entity and lifecycle state machine.

use serde::{Deserialize, Serialize};
use sitolo_auth::id::{SecurityVersion, UserId};
use sitolo_authz::{MembershipId, OrganizationId, RoleId, Scope};

use crate::error::TenancyError;

/// Membership status (§8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MembershipStatus {
    Invited,
    PendingAcceptance,
    Expired,
    Active,
    Suspended,
    Revoked,
}

impl MembershipStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            MembershipStatus::Invited => "invited",
            MembershipStatus::PendingAcceptance => "pending_acceptance",
            MembershipStatus::Expired => "expired",
            MembershipStatus::Active => "active",
            MembershipStatus::Suspended => "suspended",
            MembershipStatus::Revoked => "revoked",
        }
    }
}

/// Organization membership entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Membership {
    pub id: MembershipId,
    pub organization_id: OrganizationId,
    pub user_id: UserId,
    pub status: MembershipStatus,
    pub roles: Vec<RoleId>,
    pub scope_grants: Vec<Scope>,
    pub security_version: SecurityVersion,
    pub created_at_epoch_secs: u64,
    pub updated_at_epoch_secs: u64,
}

impl Membership {
    pub fn create_active(
        id: MembershipId,
        organization_id: OrganizationId,
        user_id: UserId,
        roles: Vec<RoleId>,
        scope_grants: Vec<Scope>,
        now_epoch_secs: u64,
    ) -> Self {
        Self {
            id,
            organization_id,
            user_id,
            status: MembershipStatus::Active,
            roles,
            scope_grants,
            security_version: SecurityVersion(1),
            created_at_epoch_secs: now_epoch_secs,
            updated_at_epoch_secs: now_epoch_secs,
        }
    }

    pub fn accept(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        match self.status {
            MembershipStatus::Invited | MembershipStatus::PendingAcceptance => {
                self.status = MembershipStatus::Active;
                self.security_version = self.security_version.next();
                self.updated_at_epoch_secs = now_epoch_secs;
                Ok(())
            }
            _ => Err(TenancyError::InvalidStatusTransition {
                from: self.status.as_str().to_string(),
                to: "active".to_string(),
            }),
        }
    }

    pub fn suspend(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        if self.status != MembershipStatus::Active {
            return Err(TenancyError::InvalidStatusTransition {
                from: self.status.as_str().to_string(),
                to: "suspended".to_string(),
            });
        }
        self.status = MembershipStatus::Suspended;
        self.security_version = self.security_version.next();
        self.updated_at_epoch_secs = now_epoch_secs;
        Ok(())
    }

    pub fn reactivate(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        if self.status != MembershipStatus::Suspended {
            return Err(TenancyError::InvalidStatusTransition {
                from: self.status.as_str().to_string(),
                to: "active".to_string(),
            });
        }
        self.status = MembershipStatus::Active;
        self.security_version = self.security_version.next();
        self.updated_at_epoch_secs = now_epoch_secs;
        Ok(())
    }

    pub fn revoke(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        if self.status == MembershipStatus::Revoked {
            return Ok(());
        }
        self.status = MembershipStatus::Revoked;
        self.security_version = self.security_version.next();
        self.updated_at_epoch_secs = now_epoch_secs;
        Ok(())
    }

    pub fn assign_role(&mut self, role: RoleId, now_epoch_secs: u64) {
        if !self.roles.contains(&role) {
            self.roles.push(role);
            self.security_version = self.security_version.next();
            self.updated_at_epoch_secs = now_epoch_secs;
        }
    }

    pub fn revoke_role(&mut self, role: &RoleId, now_epoch_secs: u64) -> bool {
        if let Some(idx) = self.roles.iter().position(|r| r == role) {
            self.roles.remove(idx);
            self.security_version = self.security_version.next();
            self.updated_at_epoch_secs = now_epoch_secs;
            true
        } else {
            false
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == MembershipStatus::Active
    }
}
