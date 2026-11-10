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
    /// Email address delivery.
    Email,
    /// Phone or SMS delivery.
    Phone,
}

/// Validated invitation target contact (section 39). Structural bounds only:
/// deliverability is proven by delivery, not by parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvitationContact {
    /// Delivery channel the value belongs to.
    pub kind: ContactKind,
    /// Trimmed contact value, structurally validated for its channel.
    pub value: String,
}

impl InvitationContact {
    /// Validates `value` structurally for `kind` and builds the contact.
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidInvitation`] when the trimmed value is
    /// empty or longer than [`MAX_CONTACT_LEN`], when an email contains ASCII
    /// whitespace or does not have exactly one `@` with a non-empty local part
    /// and a dotted domain, and when a phone value exceeds 32 bytes, contains
    /// a byte outside digits and `+ - ( )` plus space, or holds fewer than 7
    /// or more than 15 digits.
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
    let digits = value.bytes().filter(u8::is_ascii_digit).count();
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
    /// Unknown, revoked, or otherwise unusable token; token holders learn
    /// nothing beyond rejection.
    #[error("invitation is invalid")]
    Invalid,
    /// The invitation lapsed before redemption.
    #[error("invitation has expired")]
    Expired,
    /// The invitation was already redeemed exactly once.
    #[error("invitation was already accepted")]
    AlreadyAccepted,
    /// Issuance was rate limited.
    #[error("invitation rate limited")]
    RateLimited,
    /// Mandatory Phase 4 Part 8 audit/outbox evidence could not be durably
    /// recorded after the invitation was validly redeemed. Distinct from
    /// the token-validity codes above by design: this is an internal
    /// persistence failure on an otherwise-successful redemption, not a
    /// statement about the token, so it must not be folded into `Invalid`
    /// (that would misreport a real evidence outage as a bad token) nor
    /// exposed to the caller under a token-validity code (a future HTTP
    /// mapping should treat this as a 5xx, not one of the 4xx invitation
    /// codes in `code()`).
    #[error("mandatory audit/outbox evidence persistence failed")]
    EvidencePersistenceFailed,
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
            InvitationError::EvidencePersistenceFailed => "INVITATION_EVIDENCE_PERSISTENCE_FAILED",
        }
    }
}

/// Invitation lifecycle state (section 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvitationState {
    /// Minted and redeemable until expiry.
    Issued,
    /// Terminal; the invitation was redeemed exactly once.
    Accepted,
    /// Terminal; the invitation was withdrawn before redemption.
    Revoked,
    /// Terminal; the invitation lapsed before redemption.
    Expired,
}

impl InvitationState {
    /// Returns `true` when the invitation is in any terminal state.
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
    /// Stable identifier of this invitation record.
    pub id: InvitationId,
    /// Organization the invitation enrolls into.
    pub organization_id: OrganizationId,
    /// Membership the invitation is bound to.
    pub membership_id: MembershipId,
    /// Server-authoritative role applied on acceptance; never taken from
    /// client input.
    pub proposed_role: Role,
    /// `None` means organization-wide; `Some` narrows to the named scope and
    /// is materialized as a scope grant on acceptance.
    pub proposed_scope: Option<Scope>,
    /// Removed when the invitation reaches any terminal state.
    pub contact: Option<InvitationContact>,
    /// Removed when the invitation reaches any terminal state.
    pub token_hash: Option<[u8; 32]>,
    /// Instant at which the invitation was issued.
    pub issued_at: SystemTime,
    /// Instant at or after which acceptance reports expiry.
    pub expires_at: SystemTime,
    /// Lifecycle state of the invitation.
    pub state: InvitationState,
    /// Monotonic version, incremented on every accepted transition.
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
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidInvitation`] when `raw` is shorter than
    /// [`MIN_TOKEN_LEN`] or longer than [`MAX_TOKEN_LEN`] bytes, or contains a
    /// byte outside the printable ASCII range `0x21..=0x7e`.
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
    ///
    /// # Errors
    ///
    /// Returns [`TenancyError::InvalidInvitation`] when `raw_token` fails
    /// [`Invitation::validate_raw_token`], when `ttl` is zero, or when the
    /// expiry instant cannot be represented.
    #[expect(
        clippy::too_many_arguments,
        reason = "Invitation issuance keeps tenant, actor, scope, expiry, and token inputs explicit for security review."
    )]
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
        let expires_at = now
            .checked_add(ttl)
            .ok_or(TenancyError::InvalidInvitation)?;
        Ok(Invitation {
            id,
            organization_id,
            membership_id,
            proposed_role,
            proposed_scope,
            contact: Some(contact),
            token_hash: Some(Self::hash_token(raw_token)),
            issued_at: now,
            expires_at,
            state: InvitationState::Issued,
            state_version: 1,
        })
    }

    /// Claims the invitation for redemption: the single-use gate (sections
    /// 9.1, 9.3). A lapsed invitation transitions to `Expired` as it is
    /// observed; consumed or revoked tokens stay terminal and indistinguishable
    /// beyond their precise holder-visible states.
    ///
    /// # Errors
    ///
    /// Returns [`InvitationError::AlreadyAccepted`] when the invitation was
    /// already redeemed, [`InvitationError::Invalid`] when it was revoked, and
    /// [`InvitationError::Expired`] when it had already expired or lapses at
    /// `now`.
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
            self.erase_terminal_data();
            return Err(InvitationError::Expired);
        }
        self.state = InvitationState::Accepted;
        self.state_version = self.state_version.saturating_add(1);
        self.erase_terminal_data();
        Ok(())
    }

    /// Withdraws an issued invitation. Only `Issued` can be revoked;
    /// consumed records keep their terminal meaning.
    ///
    /// # Errors
    ///
    /// Returns [`InvitationError::AlreadyAccepted`] when the invitation was
    /// already redeemed, [`InvitationError::Invalid`] when it is already
    /// revoked, and [`InvitationError::Expired`] when it had already expired.
    pub fn revoke(&mut self) -> Result<(), InvitationError> {
        match self.state {
            InvitationState::Issued => {
                self.state = InvitationState::Revoked;
                self.state_version = self.state_version.saturating_add(1);
                self.erase_terminal_data();
                Ok(())
            }
            InvitationState::Accepted => Err(InvitationError::AlreadyAccepted),
            InvitationState::Revoked => Err(InvitationError::Invalid),
            InvitationState::Expired => Err(InvitationError::Expired),
        }
    }

    fn erase_terminal_data(&mut self) {
        self.contact = None;
        self.token_hash = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    type AuthzTestError = Box<dyn Error>;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn contact() -> Result<InvitationContact, TenancyError> {
        InvitationContact::new(ContactKind::Email, "cashier@example.com")
    }

    fn invitation() -> Result<Invitation, TenancyError> {
        let now = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(1_000))
            .ok_or(TenancyError::InvalidInvitation)?;
        Invitation::issue(
            InvitationId::new("inv-1")?,
            OrganizationId::new("o1")?,
            MembershipId::new("m1")?,
            Role::Cashier,
            None,
            contact()?,
            TOKEN,
            now,
            Duration::from_secs(3_600),
        )
    }

    #[test]
    fn issue_stores_hash_never_raw() -> Result<(), TenancyError> {
        let invitation = invitation()?;
        assert_eq!(invitation.token_hash, Some(Invitation::hash_token(TOKEN)));
        assert_eq!(invitation.state, InvitationState::Issued);
        // The record carries no raw-token field: debug output cannot leak it.
        let rendered = format!("{invitation:?}");
        assert!(!rendered.contains(TOKEN));
        Ok(())
    }

    #[test]
    fn raw_token_bounds_reject_extremes() {
        assert!(Invitation::validate_raw_token("short").is_err());
        assert!(Invitation::validate_raw_token(&"t".repeat(MAX_TOKEN_LEN + 1)).is_err());
        assert!(Invitation::validate_raw_token("has space padding around token!!!!").is_err());
        assert!(Invitation::validate_raw_token(TOKEN).is_ok());
    }

    #[test]
    fn zero_ttl_cannot_issue() -> Result<(), TenancyError> {
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(
            Invitation::issue(
                InvitationId::new("inv-1")?,
                OrganizationId::new("o1")?,
                MembershipId::new("m1")?,
                Role::Cashier,
                None,
                contact()?,
                TOKEN,
                now,
                Duration::ZERO,
            ),
            Err(TenancyError::InvalidInvitation)
        );
        Ok(())
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
    fn accept_consumes_exactly_once() -> Result<(), AuthzTestError> {
        let mut invitation = invitation()?;
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_001);
        invitation.accept(now)?;
        assert_eq!(invitation.state, InvitationState::Accepted);
        assert!(invitation.contact.is_none());
        assert!(invitation.token_hash.is_none());
        assert_eq!(
            invitation.accept(now),
            Err(InvitationError::AlreadyAccepted)
        );
        Ok(())
    }

    #[test]
    fn lapsed_invitation_expires_on_observation() -> Result<(), TenancyError> {
        let mut invitation = invitation()?;
        let late = SystemTime::UNIX_EPOCH + Duration::from_secs(5_000);
        assert_eq!(invitation.accept(late), Err(InvitationError::Expired));
        assert_eq!(invitation.state, InvitationState::Expired);
        assert!(invitation.contact.is_none());
        assert!(invitation.token_hash.is_none());
        assert_eq!(invitation.accept(late), Err(InvitationError::Expired));
        Ok(())
    }

    #[test]
    fn withdrawal_erases_contact_and_token_hash() -> Result<(), AuthzTestError> {
        let mut invitation = invitation()?;
        invitation.revoke()?;
        assert_eq!(invitation.state, InvitationState::Revoked);
        assert!(invitation.contact.is_none());
        assert!(invitation.token_hash.is_none());
        Ok(())
    }

    #[test]
    fn revoked_tokens_are_generically_invalid() -> Result<(), AuthzTestError> {
        let mut invitation = invitation()?;
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_001);
        invitation.revoke()?;
        assert_eq!(invitation.accept(now), Err(InvitationError::Invalid));
        assert_eq!(invitation.revoke(), Err(InvitationError::Invalid));
        Ok(())
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
