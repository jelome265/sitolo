//! Invitation workflow: token-bound enrollment into membership.
//!
//! Phase 4 specification, section 9 (with race rule 9.3/22.3, tamper threat
//! T4, error codes from section 30). An invitation is a controlled transition
//! from no membership to pending membership carrying the organization, the
//! server-authoritative proposed role and scope, an expiry, a target contact,
//! and a single-use token reference:
//!
//! ```text
//! ISSUED -> ACCEPTED | REVOKED | EXPIRED   (all terminal)
//! ```
//!
//! Security properties enforced here:
//!
//! - the raw token is never stored — only its SHA-256 hash (section 9.1,
//!   mirroring the Phase 3 reset-artifact precedent);
//! - the token identifies the invitation; role and scope always come from
//!   the server-loaded record, never from client input (sections 9.2, T4);
//! - acceptance is an explicit single-claim transition: concurrent attempts
//!   resolve to exactly one success (sections 9.3, 22.3).
//!
//! Delivery (email/SMS), audit emission, and metrics arrive with later Phase
//! 4 PRs; this module owns the lifecycle, validation, and redemption
//! semantics.

use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};
use sitolo_domain::tenancy::{InvitationId, MembershipId, OrganizationId, TenancyError};
use thiserror::Error;

use crate::roles::Role;
use crate::scopes::Scope;

/// Minimum raw-token length in bytes. Tokens are machine-minted
/// high-entropy values (section 9.1); short presenter strings are rejected
/// before any hashing work.
pub const MIN_TOKEN_LEN: usize = 32;
/// Maximum raw-token length in bytes (section 39: reject oversized strings).
pub const MAX_TOKEN_LEN: usize = 512;
/// Maximum contact value length in bytes (section 39).
pub const MAX_CONTACT_LEN: usize = 254;

/// Target contact channel for invitation delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContactKind {
    Email,
    Phone,
}

/// Validated invitation target contact (section 39). Structural bounds only:
/// deliverability is proven by delivery, not by parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvitationContact {
    pub kind: ContactKind,
    pub value: String,
}

impl InvitationContact {
    pub fn new(kind: ContactKind, value: &str) -> Result<Self, TenancyError> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_CONTACT_LEN {
            return Err(TenancyError::InvalidInvitation);
        }
        match kind {
            ContactKind::Email => validate_email(trimmed)?,
            ContactKind::Phone => validate_phone(trimmed)?,
        }
        Ok(InvitationContact {
            kind,
            value: trimmed.to_string(),
        })
    }
}

fn validate_email(value: &str) -> Result<(), TenancyError> {
    if value.bytes().any(|b| b.is_ascii_whitespace()) {
        return Err(TenancyError::InvalidInvitation);
    }
    let (local, domain) = value
        .split_once('@')
        .ok_or(TenancyError::InvalidInvitation)?;
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err(TenancyError::InvalidInvitation);
    }
    if value.matches('@').count() != 1 {
        return Err(TenancyError::InvalidInvitation);
    }
    Ok(())
}

fn validate_phone(value: &str) -> Result<(), TenancyError> {
    if value.len() > 32 {
        return Err(TenancyError::InvalidInvitation);
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b' ' | b'-' | b'(' | b')'))
    {
        return Err(TenancyError::InvalidInvitation);
    }
    let digits = value.bytes().filter(|b| b.is_ascii_digit()).count();
    if !(7..=15).contains(&digits) {
        return Err(TenancyError::InvalidInvitation);
    }
    Ok(())
}

/// Redemption failures with stable public codes (Phase 4 section 30).
/// Unknown or revoked tokens share `Invalid`: token holders learn nothing
/// beyond rejection.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvitationError {
    #[error("invitation is invalid")]
    Invalid,
    #[error("invitation has expired")]
    Expired,
    #[error("invitation was already accepted")]
    AlreadyAccepted,
    #[error("invitation rate limited")]
    RateLimited,
}

impl InvitationError {
    /// Stable machine-readable code (section 30).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            InvitationError::Invalid => "INVITATION_INVALID",
            InvitationError::Expired => "INVITATION_EXPIRED",
            InvitationError::AlreadyAccepted => "INVITATION_ALREADY_ACCEPTED",
            InvitationError::RateLimited => "INVITATION_RATE_LIMITED",
        }
    }
}

/// Invitation lifecycle state (section 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvitationState {
    Issued,
    Accepted,
    Revoked,
    Expired,
}

impl InvitationState {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        !matches!(self, InvitationState::Issued)
    }
}

/// An invitation record (section 9). The proposed role and scope are
/// server-authoritative data applied at acceptance; the token is an opaque
/// single-use identifier carrying no authority by itself (sections 9.2, T4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invitation {
    pub id: InvitationId,
    pub organization_id: OrganizationId,
    pub membership_id: MembershipId,
    pub proposed_role: Role,
    /// `None` means organization-wide; `Some` narrows to the named scope and
    /// is materialized as a scope grant on acceptance.
    pub proposed_scope: Option<Scope>,
    pub contact: InvitationContact,
    pub token_hash: [u8; 32],
    pub issued_at: SystemTime,
    pub expires_at: SystemTime,
    pub state: InvitationState,
    pub state_version: u64,
}

impl Invitation {
    /// Hashes a raw token into the stored representation (section 9.1).
    #[must_use]
    pub fn hash_token(raw: &str) -> [u8; 32] {
        Sha256::digest(raw.as_bytes()).into()
    }

    /// Validates raw-token shape before hashing (section 39). Tokens are
    /// machine-minted printable values; control characters and extremes of
    /// length are rejected without touching the store.
    pub fn validate_raw_token(raw: &str) -> Result<(), TenancyError> {
        if !(MIN_TOKEN_LEN..=MAX_TOKEN_LEN).contains(&raw.len()) {
            return Err(TenancyError::InvalidInvitation);
        }
        if !raw.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
            return Err(TenancyError::InvalidInvitation);
        }
        Ok(())
    }

    /// Issues an invitation. A zero TTL would mint a stillborn record, so
    /// expiry-bounded issuance requires a positive TTL (section 9.1).
    #[allow(clippy::too_many_arguments)]
    pub fn issue(
        id: InvitationId,
        organization_id: OrganizationId,
        membership_id: MembershipId,
        proposed_role: Role,
        proposed_scope: Option<Scope>,
        contact: InvitationContact,
        raw_token: &str,
        now: SystemTime,
        ttl: Duration,
    ) -> Result<Self, TenancyError> {
        Self::validate_raw_token(raw_token)?;
        if ttl.is_zero() {
            return Err(TenancyError::InvalidInvitation);
        }
        Ok(Invitation {
            id,
            organization_id,
            membership_id,
            proposed_role,
            proposed_scope,
            contact,
            token_hash: Self::hash_token(raw_token),
            issued_at: now,
            expires_at: now + ttl,
            state: InvitationState::Issued,
            state_version: 1,
        })
    }

    /// Claims the invitation for redemption: the single-use gate (sections
    /// 9.1, 9.3). A lapsed invitation transitions to `Expired` as it is
    /// observed; consumed or revoked tokens stay terminal and indistinguishable
    /// beyond their precise holder-visible states.
    pub fn accept(&mut self, now: SystemTime) -> Result<(), InvitationError> {
        match self.state {
            InvitationState::Accepted => return Err(InvitationError::AlreadyAccepted),
            InvitationState::Revoked => return Err(InvitationError::Invalid),
            InvitationState::Expired => return Err(InvitationError::Expired),
            InvitationState::Issued => {}
        }
        if now >= self.expires_at {
            self.state = InvitationState::Expired;
            self.state_version = self.state_version.saturating_add(1);
            return Err(InvitationError::Expired);
        }
        self.state = InvitationState::Accepted;
        self.state_version = self.state_version.saturating_add(1);
        Ok(())
    }

    /// Withdraws an issued invitation. Only `Issued` can be revoked;
    /// consumed records keep their terminal meaning.
    pub fn revoke(&mut self) -> Result<(), InvitationError> {
        match self.state {
            InvitationState::Issued => {
                self.state = InvitationState::Revoked;
                self.state_version = self.state_version.saturating_add(1);
                Ok(())
            }
            InvitationState::Accepted => Err(InvitationError::AlreadyAccepted),
            InvitationState::Revoked => Err(InvitationError::Invalid),
            InvitationState::Expired => Err(InvitationError::Expired),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn contact() -> InvitationContact {
        InvitationContact::new(ContactKind::Email, "cashier@example.com").unwrap()
    }

    fn invitation() -> Invitation {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        Invitation::issue(
            InvitationId::new("inv-1").unwrap(),
            OrganizationId::new("o1").unwrap(),
            MembershipId::new("m1").unwrap(),
            Role::Cashier,
            None,
            contact(),
            TOKEN,
            now,
            Duration::from_secs(3_600),
        )
        .unwrap()
    }

    #[test]
    fn issue_stores_hash_never_raw() {
        let invitation = invitation();
        assert_eq!(invitation.token_hash, Invitation::hash_token(TOKEN));
        assert_eq!(invitation.state, InvitationState::Issued);
        // The record carries no raw-token field: debug output cannot leak it.
        let rendered = format!("{invitation:?}");
        assert!(!rendered.contains(TOKEN));
    }

    #[test]
    fn raw_token_bounds_reject_extremes() {
        assert!(Invitation::validate_raw_token("short").is_err());
        assert!(Invitation::validate_raw_token(&"t".repeat(MAX_TOKEN_LEN + 1)).is_err());
        assert!(Invitation::validate_raw_token("has space padding around token!!!!").is_err());
        assert!(Invitation::validate_raw_token(TOKEN).is_ok());
    }

    #[test]
    fn zero_ttl_cannot_issue() {
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(
            Invitation::issue(
                InvitationId::new("inv-1").unwrap(),
                OrganizationId::new("o1").unwrap(),
                MembershipId::new("m1").unwrap(),
                Role::Cashier,
                None,
                contact(),
                TOKEN,
                now,
                Duration::ZERO,
            ),
            Err(TenancyError::InvalidInvitation)
        );
    }

    #[test]
    fn contacts_validate_structurally() {
        assert!(InvitationContact::new(ContactKind::Email, "a@b.co").is_ok());
        assert!(InvitationContact::new(ContactKind::Email, "no-at-sign").is_err());
        assert!(InvitationContact::new(ContactKind::Email, "a@b@c.co").is_err());
        assert!(InvitationContact::new(ContactKind::Email, "a@nodot").is_err());
        assert!(InvitationContact::new(ContactKind::Email, "").is_err());
        assert!(InvitationContact::new(ContactKind::Phone, "+265 999 000 001").is_ok());
        assert!(InvitationContact::new(ContactKind::Phone, "123").is_err());
        assert!(InvitationContact::new(ContactKind::Phone, "call me now").is_err());
    }

    #[test]
    fn accept_consumes_exactly_once() {
        let mut invitation = invitation();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_001);
        invitation.accept(now).unwrap();
        assert_eq!(invitation.state, InvitationState::Accepted);
        assert_eq!(
            invitation.accept(now),
            Err(InvitationError::AlreadyAccepted)
        );
    }

    #[test]
    fn lapsed_invitation_expires_on_observation() {
        let mut invitation = invitation();
        let late = SystemTime::UNIX_EPOCH + Duration::from_secs(5_000);
        assert_eq!(invitation.accept(late), Err(InvitationError::Expired));
        assert_eq!(invitation.state, InvitationState::Expired);
        assert_eq!(invitation.accept(late), Err(InvitationError::Expired));
    }

    #[test]
    fn revoked_tokens_are_generically_invalid() {
        let mut invitation = invitation();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_001);
        invitation.revoke().unwrap();
        assert_eq!(invitation.accept(now), Err(InvitationError::Invalid));
        assert_eq!(invitation.revoke(), Err(InvitationError::Invalid));
    }

    #[test]
    fn error_codes_match_section_30() {
        assert_eq!(InvitationError::Invalid.code(), "INVITATION_INVALID");
        assert_eq!(InvitationError::Expired.code(), "INVITATION_EXPIRED");
        assert_eq!(
            InvitationError::AlreadyAccepted.code(),
            "INVITATION_ALREADY_ACCEPTED"
        );
        assert_eq!(
            InvitationError::RateLimited.code(),
            "INVITATION_RATE_LIMITED"
        );
    }
}
