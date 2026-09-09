//! Invitation entity and lifecycle state machine.

use serde::{Deserialize, Serialize};
use sitolo_auth::id::UserId;
use sitolo_authz::{InvitationId, OrganizationId, RoleId, Scope};

use crate::error::TenancyError;

/// Status of an Invitation (§9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Revoked,
    Expired,
}

impl InvitationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Revoked => "revoked",
            InvitationStatus::Expired => "expired",
        }
    }
}

/// Invitation aggregate entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invitation {
    pub id: InvitationId,
    pub organization_id: OrganizationId,
    pub issuer_user_id: UserId,
    pub target_contact: String,
    pub token_hash: String,
    pub proposed_role: RoleId,
    pub proposed_scope: Scope,
    pub status: InvitationStatus,
    pub expires_at_epoch_secs: u64,
    pub created_at_epoch_secs: u64,
}

impl Invitation {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: InvitationId,
        organization_id: OrganizationId,
        issuer_user_id: UserId,
        target_contact: impl Into<String>,
        token_hash: String,
        proposed_role: RoleId,
        proposed_scope: Scope,
        expires_at_epoch_secs: u64,
        now_epoch_secs: u64,
    ) -> Result<Self, TenancyError> {
        let contact = target_contact.into();
        if contact.trim().is_empty() {
            return Err(TenancyError::InvalidIdentifier);
        }

        Ok(Self {
            id,
            organization_id,
            issuer_user_id,
            target_contact: contact,
            token_hash,
            proposed_role,
            proposed_scope,
            status: InvitationStatus::Pending,
            expires_at_epoch_secs,
            created_at_epoch_secs: now_epoch_secs,
        })
    }

    pub fn accept(&mut self, now_epoch_secs: u64) -> Result<(), TenancyError> {
        if self.status != InvitationStatus::Pending {
            return Err(TenancyError::InvitationAlreadyAccepted);
        }
        if now_epoch_secs >= self.expires_at_epoch_secs {
            self.status = InvitationStatus::Expired;
            return Err(TenancyError::InvitationExpired);
        }

        self.status = InvitationStatus::Accepted;
        Ok(())
    }

    pub fn revoke(&mut self) -> Result<(), TenancyError> {
        if self.status == InvitationStatus::Accepted {
            return Err(TenancyError::InvitationAlreadyAccepted);
        }
        self.status = InvitationStatus::Revoked;
        Ok(())
    }

    pub fn is_valid(&self, now_epoch_secs: u64) -> bool {
        self.status == InvitationStatus::Pending && now_epoch_secs < self.expires_at_epoch_secs
    }
}
