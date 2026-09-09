//! Ownership transfer state machine and entity.

use serde::{Deserialize, Serialize};
use sitolo_auth::id::UserId;
use sitolo_authz::{OrganizationId, OwnershipTransferId};

use crate::error::TenancyError;

/// Status of Ownership Transfer (§12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnershipTransferStatus {
    Requested,
    VerificationRequired,
    PendingEffective,
    Transferred,
    Rejected,
    Expired,
}

impl OwnershipTransferStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OwnershipTransferStatus::Requested => "requested",
            OwnershipTransferStatus::VerificationRequired => "verification_required",
            OwnershipTransferStatus::PendingEffective => "pending_effective",
            OwnershipTransferStatus::Transferred => "transferred",
            OwnershipTransferStatus::Rejected => "rejected",
            OwnershipTransferStatus::Expired => "expired",
        }
    }
}

/// Ownership transfer request aggregate entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnershipTransferRequest {
    pub id: OwnershipTransferId,
    pub organization_id: OrganizationId,
    pub current_owner_user_id: UserId,
    pub proposed_owner_user_id: UserId,
    pub status: OwnershipTransferStatus,
    pub expires_at_epoch_secs: u64,
    pub created_at_epoch_secs: u64,
}

impl OwnershipTransferRequest {
    pub fn create(
        id: OwnershipTransferId,
        organization_id: OrganizationId,
        current_owner_user_id: UserId,
        proposed_owner_user_id: UserId,
        expires_at_epoch_secs: u64,
        now_epoch_secs: u64,
    ) -> Self {
        Self {
            id,
            organization_id,
            current_owner_user_id,
            proposed_owner_user_id,
            status: OwnershipTransferStatus::Requested,
            expires_at_epoch_secs,
            created_at_epoch_secs: now_epoch_secs,
        }
    }

    pub fn approve(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        if now_epoch_secs >= self.expires_at_epoch_secs {
            self.status = OwnershipTransferStatus::Expired;
            return Err(TenancyError::OwnershipTransferInvalidState);
        }
        match self.status {
            OwnershipTransferStatus::Requested
            | OwnershipTransferStatus::VerificationRequired
            | OwnershipTransferStatus::PendingEffective => {
                self.status = OwnershipTransferStatus::Transferred;
                Ok(())
            }
            _ => Err(TenancyError::OwnershipTransferInvalidState),
        }
    }

    pub fn reject(&mut self) -> Result<(), TenancyError> {
        if self.status == OwnershipTransferStatus::Transferred {
            return Err(TenancyError::OwnershipTransferInvalidState);
        }
        self.status = OwnershipTransferStatus::Rejected;
        Ok(())
    }
}
