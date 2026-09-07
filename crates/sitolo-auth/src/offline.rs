//! Offline capability boundary.
//!
//! Phase 3 specification, §30. Offline support is a product requirement but
//! must not create a permanent credential bypass. The client may cache enough
//! data for operational continuity, but it cannot mint new authority locally.

use std::collections::BTreeSet;
use std::time::SystemTime;

use crate::device::DeviceState;
use crate::error::AuthError;
use crate::id::{DeviceId, SecurityVersion, SessionId};

/// Offline operation classification (§30).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OfflineOperation {
    SaleCreate,
    CatalogueRead,
    InventoryRead,
    PriceRead,
}

/// Bounded offline capability (§30).
#[derive(Debug, Clone)]
pub struct OfflineGrant {
    pub session_id: SessionId,
    pub device_id: DeviceId,
    pub granted_at: SystemTime,
    pub expires_at: SystemTime,
    pub allowed_operations: BTreeSet<OfflineOperation>,
    pub security_version: SecurityVersion,
}

impl OfflineGrant {
    /// Validates an offline operation (§30). Expired grants, revoked devices,
    /// and disallowed operations are denied.
    pub fn validate(
        &self,
        operation: OfflineOperation,
        now: SystemTime,
        device_state: DeviceState,
        current_version: SecurityVersion,
    ) -> Result<(), AuthError> {
        if now >= self.expires_at {
            return Err(AuthError::SessionExpired);
        }
        if device_state != DeviceState::Active {
            return Err(AuthError::DeviceRevoked);
        }
        if current_version != self.security_version {
            return Err(AuthError::SecurityVersionMismatch);
        }
        if !self.allowed_operations.contains(&operation) {
            return Err(AuthError::AuthorizationDenied);
        }
        Ok(())
    }
}

/// Authorization denied error (added for offline validation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorizationDenied;

impl From<AuthorizationDenied> for AuthError {
    fn from(_: AuthorizationDenied) -> Self {
        AuthError::AuthorizationDenied
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn grant() -> OfflineGrant {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        OfflineGrant {
            session_id: SessionId::new("sess-1").unwrap(),
            device_id: DeviceId::new("dev-1").unwrap(),
            granted_at: now,
            expires_at: now + Duration::from_secs(3_600),
            allowed_operations: [
                OfflineOperation::SaleCreate,
                OfflineOperation::CatalogueRead,
            ]
            .into_iter()
            .collect(),
            security_version: SecurityVersion(1),
        }
    }

    #[test]
    fn valid_offline_operation_accepted() {
        let g = grant();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_100);
        assert!(
            g.validate(
                OfflineOperation::SaleCreate,
                now,
                DeviceState::Active,
                SecurityVersion(1)
            )
            .is_ok()
        );
    }

    #[test]
    fn expired_grant_rejected() {
        let g = grant();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(5_000);
        assert!(
            g.validate(
                OfflineOperation::SaleCreate,
                now,
                DeviceState::Active,
                SecurityVersion(1)
            )
            .is_err()
        );
    }

    #[test]
    fn revoked_device_rejected() {
        let g = grant();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_100);
        assert!(
            g.validate(
                OfflineOperation::SaleCreate,
                now,
                DeviceState::Revoked,
                SecurityVersion(1)
            )
            .is_err()
        );
    }

    #[test]
    fn disallowed_operation_rejected() {
        let g = grant();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_100);
        assert!(
            g.validate(
                OfflineOperation::InventoryRead,
                now,
                DeviceState::Active,
                SecurityVersion(1)
            )
            .is_err()
        );
    }

    #[test]
    fn security_version_mismatch_rejected() {
        let g = grant();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_100);
        assert!(
            g.validate(
                OfflineOperation::SaleCreate,
                now,
                DeviceState::Active,
                SecurityVersion(2)
            )
            .is_err()
        );
    }
}
