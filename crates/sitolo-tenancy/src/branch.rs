//! Branch entity and lifecycle state machine.

use serde::{Deserialize, Serialize};
use sitolo_auth::id::SecurityVersion;
use sitolo_authz::{BranchId, OrganizationId};

use crate::error::TenancyError;

/// Lifecycle state for a Branch (§14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BranchStatus {
    Provisioning,
    Active,
    Suspended,
    Closing,
    Closed,
}

impl BranchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BranchStatus::Provisioning => "provisioning",
            BranchStatus::Active => "active",
            BranchStatus::Suspended => "suspended",
            BranchStatus::Closing => "closing",
            BranchStatus::Closed => "closed",
        }
    }
}

/// Branch entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    pub id: BranchId,
    pub organization_id: OrganizationId,
    pub name: String,
    pub status: BranchStatus,
    pub is_default: bool,
    pub security_version: SecurityVersion,
    pub created_at_epoch_secs: u64,
    pub updated_at_epoch_secs: u64,
}

impl Branch {
    pub fn create(
        id: BranchId,
        organization_id: OrganizationId,
        name: impl Into<String>,
        is_default: bool,
        now_epoch_secs: u64,
    ) -> Result<Self, TenancyError> {
        let name = name.into();
        if name.trim().is_empty() || name.len() > 256 {
            return Err(TenancyError::InvalidIdentifier);
        }

        Ok(Self {
            id,
            organization_id,
            name,
            status: BranchStatus::Active,
            is_default,
            security_version: SecurityVersion(1),
            created_at_epoch_secs: now_epoch_secs,
            updated_at_epoch_secs: now_epoch_secs,
        })
    }

    pub fn suspend(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        if self.status != BranchStatus::Active {
            return Err(TenancyError::InvalidStatusTransition {
                from: self.status.as_str().to_string(),
                to: "suspended".to_string(),
            });
        }
        self.status = BranchStatus::Suspended;
        self.security_version = self.security_version.next();
        self.updated_at_epoch_secs = now_epoch_secs;
        Ok(())
    }

    pub fn close(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        if self.status == BranchStatus::Closed {
            return Ok(());
        }
        self.status = BranchStatus::Closed;
        self.security_version = self.security_version.next();
        self.updated_at_epoch_secs = now_epoch_secs;
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.status == BranchStatus::Active
    }
}
