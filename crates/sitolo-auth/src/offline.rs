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
    /// Record a sale while disconnected.
    SaleCreate,
    /// Read the cached catalogue.
    CatalogueRead,
    /// Read cached inventory levels.
    InventoryRead,
    /// Read cached prices.
    PriceRead,
}

/// Bounded offline capability (§30).
#[derive(Debug, Clone)]
pub struct OfflineGrant {
    /// Session the grant was issued for.
    pub session_id: SessionId,
    /// Device the grant is bound to.
    pub device_id: DeviceId,
    /// When the grant was issued.
    pub granted_at: SystemTime,
    /// Hard expiry of the grant; the capability cannot outlive it.
    pub expires_at: SystemTime,
    /// The operations this grant permits while offline.
    pub allowed_operations: BTreeSet<OfflineOperation>,
    /// Security version the grant is bound to (§31).
    pub security_version: SecurityVersion,
}

impl OfflineGrant {
    /// Validates an offline operation (§30). Expired grants, revoked devices,
    /// and disallowed operations are denied.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::SessionExpired`] when `now` is at or past
    /// `expires_at`, [`AuthError::DeviceRevoked`] when the device is not
    /// [`DeviceState::Active`], [`AuthError::SecurityVersionMismatch`] when
    /// the caller's security version differs from the grant's, and
    /// [`AuthError::AuthorizationDenied`] when the operation is not in
    /// `allowed_operations`.
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

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }

    fn grant() -> Result<OfflineGrant, AuthError> {
        let now = at(1_000);
        Ok(OfflineGrant {
            session_id: SessionId::new("sess-1")?,
            device_id: DeviceId::new("dev-1")?,
            granted_at: now,
            expires_at: at(4_600),
            allowed_operations: [
                OfflineOperation::SaleCreate,
                OfflineOperation::CatalogueRead,
            ]
            .into_iter()
            .collect(),
            security_version: SecurityVersion(1),
        })
    }

    #[test]
    fn valid_offline_operation_accepted() -> Result<(), AuthError> {
        let g = grant()?;
        let now = at(1_100);
        assert!(
            g.validate(
                OfflineOperation::SaleCreate,
                now,
                DeviceState::Active,
                SecurityVersion(1)
            )
            .is_ok()
        );
        Ok(())
    }

    #[test]
    fn expired_grant_rejected() -> Result<(), AuthError> {
        let g = grant()?;
        let now = at(5_000);
        assert!(
            g.validate(
                OfflineOperation::SaleCreate,
                now,
                DeviceState::Active,
                SecurityVersion(1)
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn revoked_device_rejected() -> Result<(), AuthError> {
        let g = grant()?;
        let now = at(1_100);
        assert!(
            g.validate(
                OfflineOperation::SaleCreate,
                now,
                DeviceState::Revoked,
                SecurityVersion(1)
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn disallowed_operation_rejected() -> Result<(), AuthError> {
        let g = grant()?;
        let now = at(1_100);
        assert!(
            g.validate(
                OfflineOperation::InventoryRead,
                now,
                DeviceState::Active,
                SecurityVersion(1)
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn security_version_mismatch_rejected() -> Result<(), AuthError> {
        let g = grant()?;
        let now = at(1_100);
        assert!(
            g.validate(
                OfflineOperation::SaleCreate,
                now,
                DeviceState::Active,
                SecurityVersion(2)
            )
            .is_err()
        );
        Ok(())
    }
}
