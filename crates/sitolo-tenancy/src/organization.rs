//! Organization aggregate and lifecycle state machine.

use serde::{Deserialize, Serialize};
use sitolo_auth::id::SecurityVersion;
use sitolo_authz::OrganizationId;

use crate::error::TenancyError;

/// Lifecycle state for an Organization (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrganizationStatus {
    Provisioning,
    Active,
    Suspended,
    Closing,
    Closed,
}

impl OrganizationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OrganizationStatus::Provisioning => "provisioning",
            OrganizationStatus::Active => "active",
            OrganizationStatus::Suspended => "suspended",
            OrganizationStatus::Closing => "closing",
            OrganizationStatus::Closed => "closed",
        }
    }
}

/// Organization aggregate entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub status: OrganizationStatus,
    pub security_version: SecurityVersion,
    pub created_at_epoch_secs: u64,
    pub updated_at_epoch_secs: u64,
}

impl Organization {
    pub fn create(
        id: OrganizationId,
        name: impl Into<String>,
        now_epoch_secs: u64,
    ) -> Result<Self, TenancyError> {
        let name = name.into();
        if name.trim().is_empty() || name.len() > 256 {
            return Err(TenancyError::InvalidIdentifier);
        }

        Ok(Self {
            id,
            name,
            status: OrganizationStatus::Provisioning,
            security_version: SecurityVersion(1),
            created_at_epoch_secs: now_epoch_secs,
            updated_at_epoch_secs: now_epoch_secs,
        })
    }

    /// Activate the organization from Provisioning or Suspended (§6).
    pub fn activate(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        match self.status {
            OrganizationStatus::Provisioning | OrganizationStatus::Suspended => {
                self.status = OrganizationStatus::Active;
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

    /// Suspend the organization from Active (§6).
    pub fn suspend(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        if self.status != OrganizationStatus::Active {
            return Err(TenancyError::InvalidStatusTransition {
                from: self.status.as_str().to_string(),
                to: "suspended".to_string(),
            });
        }
        self.status = OrganizationStatus::Suspended;
        self.security_version = self.security_version.next();
        self.updated_at_epoch_secs = now_epoch_secs;
        Ok(())
    }

    /// Transition to Closing from Active or Suspended (§6).
    pub fn start_closing(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        match self.status {
            OrganizationStatus::Active | OrganizationStatus::Suspended => {
                self.status = OrganizationStatus::Closing;
                self.security_version = self.security_version.next();
                self.updated_at_epoch_secs = now_epoch_secs;
                Ok(())
            }
            _ => Err(TenancyError::InvalidStatusTransition {
                from: self.status.as_str().to_string(),
                to: "closing".to_string(),
            }),
        }
    }

    /// Close the organization permanently (§6).
    pub fn close(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        if self.status == OrganizationStatus::Closed {
            return Ok(());
        }
        self.status = OrganizationStatus::Closed;
        self.security_version = self.security_version.next();
        self.updated_at_epoch_secs = now_epoch_secs;
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.status == OrganizationStatus::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organization_lifecycle_transitions() {
        let org_id = OrganizationId::new("org-test-1").unwrap();
        let mut org = Organization::create(org_id, "Test Merchant", 1000).unwrap();
        assert_eq!(org.status, OrganizationStatus::Provisioning);
        assert_eq!(org.security_version, SecurityVersion(1));

        // Activate
        assert!(org.activate(1005).is_ok());
        assert_eq!(org.status, OrganizationStatus::Active);
        assert_eq!(org.security_version, SecurityVersion(2));

        // Suspend
        assert!(org.suspend(1010).is_ok());
        assert_eq!(org.status, OrganizationStatus::Suspended);
        assert_eq!(org.security_version, SecurityVersion(3));

        // Reactivate
        assert!(org.activate(1015).is_ok());
        assert_eq!(org.status, OrganizationStatus::Active);

        // Close
        assert!(org.close(1020).is_ok());
        assert_eq!(org.status, OrganizationStatus::Closed);

        // Cannot suspend or activate closed org
        assert!(org.suspend(1025).is_err());
        assert!(org.activate(1025).is_err());
    }
}
