//! Refresh-token family and replay containment.
//!
//! Phase 3 specification, §12. Each successful refresh rotates to a new
//! credential; reuse of an already-consumed credential compromises the whole
//! family and invalidates affected sessions (§12.2, §55). Concurrency is
//! resolved by the persistence layer (§12.3); this module owns the pure
//! transition semantics.

use std::collections::BTreeMap;
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::error::AuthError;
use crate::id::{RefreshCredentialId, RefreshFamilyId, SecurityVersion, SessionId, UserId};

/// Hash a raw refresh credential into a fixed-size key. The raw value is
/// never retained in memory beyond the caller's scope (§49).
#[must_use]
pub fn hash_raw(raw: &str) -> [u8; 32] {
    Sha256::digest(raw.as_bytes()).into()
}

/// Lifecycle state of one refresh credential (§12.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshState {
    /// The credential may still be rotated.
    Active,
    /// The credential was rotated and replaced by a successor.
    Consumed,
    /// The credential passed its expiry before use.
    Expired,
    /// The credential was revoked by family containment (§12.2).
    Revoked,
}

/// One refresh credential inside a family (§12.1).
#[derive(Debug, Clone)]
pub struct RefreshCredential {
    /// Identifier of this credential.
    pub id: RefreshCredentialId,
    /// Family this credential belongs to.
    pub family_id: RefreshFamilyId,
    /// User the family belongs to.
    pub user_id: UserId,
    /// Session the credential may refresh.
    pub session_id: SessionId,
    /// Hash of the raw credential; the raw value is never stored (§49).
    pub token_hash: [u8; 32],
    /// When the credential was issued.
    pub issued_at: SystemTime,
    /// Hard expiry of the credential.
    pub expires_at: SystemTime,
    /// Current lifecycle state.
    pub state: RefreshState,
    /// Successor issued when this credential was rotated.
    pub replaced_by: Option<RefreshCredentialId>,
    /// Security version the credential is bound to (§31).
    pub security_version: SecurityVersion,
}

/// A refresh-token family, the containment unit for replay (§12.2).
#[derive(Debug, Clone)]
pub struct RefreshFamily {
    /// Identifier of the family.
    pub id: RefreshFamilyId,
    /// User the family belongs to.
    pub user_id: UserId,
    /// Session the family may refresh.
    pub session_id: SessionId,
    /// Whether replay detection has compromised the family.
    pub compromised: bool,
    /// Security version the family is bound to (§31).
    pub security_version: SecurityVersion,
}

/// The outcome of a successful rotation (§12.1).
#[derive(Debug, Clone)]
pub struct RotationOutcome {
    /// Family the rotation happened in.
    pub family_id: RefreshFamilyId,
    /// User the family belongs to.
    pub user_id: UserId,
    /// Session the successor may refresh.
    pub session_id: SessionId,
    /// Identifier of the newly issued credential.
    pub successor_id: RefreshCredentialId,
    /// Identifier of the credential that was consumed.
    pub previous_id: RefreshCredentialId,
}

/// The outcome of a detected replay (§12.2).
#[derive(Debug, Clone)]
pub struct ReplayOutcome {
    /// Family that was compromised.
    pub family_id: RefreshFamilyId,
    /// User the family belongs to.
    pub user_id: UserId,
    /// Session that must be revoked as part of containment.
    pub session_id: SessionId,
}

/// In-memory reference ledger. Production persistence replaces this with a
/// transactional repository that enforces single-consumption via a UNIQUE
/// constraint on the consumed token key (§12.3).
#[derive(Debug, Default)]
pub struct RefreshLedger {
    families: BTreeMap<RefreshFamilyId, RefreshFamily>,
    credentials: BTreeMap<RefreshCredentialId, RefreshCredential>,
    by_hash: BTreeMap<[u8; 32], RefreshCredentialId>,
}

impl RefreshLedger {
    /// Creates an empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new family.
    pub fn create_family(&mut self, family: RefreshFamily) {
        self.families.insert(family.id.clone(), family);
    }

    /// Issues a new credential inside an existing family.
    pub fn issue_credential(&mut self, credential: RefreshCredential) {
        self.by_hash
            .insert(credential.token_hash, credential.id.clone());
        self.credentials.insert(credential.id.clone(), credential);
    }

    /// Rotates a credential: consumes the old, issues the successor. Reuse
    /// of a consumed/revoked/expired credential compromises the family
    /// (§12.2).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::RefreshTokenInvalid`] when `raw` does not match a
    /// known credential, when the matching record is missing, or when the
    /// credential has expired at `now`. Returns
    /// [`AuthError::RefreshTokenReused`] when the credential is no longer
    /// active (a replay, which compromises the family) or when the family was
    /// already compromised.
    pub fn rotate(
        &mut self,
        raw: &str,
        successor: RefreshCredential,
        now: SystemTime,
    ) -> Result<RotationOutcome, AuthError> {
        let key = hash_raw(raw);
        let Some(id) = self.by_hash.get(&key).cloned() else {
            return Err(AuthError::RefreshTokenInvalid);
        };
        let Some(existing) = self.credentials.get(&id) else {
            return Err(AuthError::RefreshTokenInvalid);
        };
        if existing.state != RefreshState::Active {
            // Replay containment (§12.2).
            let family_id = existing.family_id.clone();
            self.compromise_family(&family_id, now);
            return Err(AuthError::RefreshTokenReused);
        }
        if now >= existing.expires_at {
            return Err(AuthError::RefreshTokenInvalid);
        }
        // The family may have been compromised by an earlier replay; refuse
        // to issue a successor in that case (§12.2).
        if let Some(family) = self.families.get(&existing.family_id)
            && family.compromised
        {
            return Err(AuthError::RefreshTokenReused);
        }
        let previous = self
            .credentials
            .get_mut(&id)
            .ok_or(AuthError::RefreshTokenInvalid)?;
        previous.state = RefreshState::Consumed;
        let successor_id = successor.id.clone();
        previous.replaced_by = Some(successor_id.clone());
        let outcome = RotationOutcome {
            family_id: previous.family_id.clone(),
            user_id: previous.user_id.clone(),
            session_id: previous.session_id.clone(),
            successor_id: successor_id.clone(),
            previous_id: id,
        };
        self.by_hash
            .insert(successor.token_hash, successor_id.clone());
        self.credentials.insert(successor_id, successor);
        Ok(outcome)
    }

    /// Compromises the entire family and revokes all its credentials (§12.2).
    pub fn compromise_family(&mut self, family_id: &RefreshFamilyId, _now: SystemTime) {
        if let Some(family) = self.families.get_mut(family_id) {
            family.compromised = true;
        }
        for credential in self.credentials.values_mut() {
            if &credential.family_id == family_id && credential.state == RefreshState::Active {
                credential.state = RefreshState::Revoked;
            }
        }
    }

    /// Returns the family with the given identifier, if it exists.
    #[must_use]
    pub fn family(&self, id: &RefreshFamilyId) -> Option<&RefreshFamily> {
        self.families.get(id)
    }

    /// Returns the credential with the given identifier, if it exists.
    #[must_use]
    pub fn credential(&self, id: &RefreshCredentialId) -> Option<&RefreshCredential> {
        self.credentials.get(id)
    }

    /// Looks up a credential by its token hash (§12.3). Production
    /// implementations query the database directly; this is for the in-memory
    /// reference.
    #[must_use]
    pub fn find_by_hash(&self, hash: [u8; 32]) -> Option<&RefreshCredential> {
        let id = self.by_hash.get(&hash)?;
        self.credentials.get(id)
    }

    /// All session identifiers associated with a family (for revocation
    /// cascades, §12.2).
    #[must_use]
    pub fn sessions_for_family(&self, family_id: &RefreshFamilyId) -> Vec<SessionId> {
        self.credentials
            .values()
            .filter(|c| &c.family_id == family_id)
            .map(|c| c.session_id.clone())
            .collect()
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

    fn setup() -> Result<(RefreshLedger, SystemTime, RefreshCredentialId), AuthError> {
        let mut ledger = RefreshLedger::new();
        let now = at(1_000);
        let family = RefreshFamilyId::new("fam-1")?;
        let user = UserId::new("user-1")?;
        let session = SessionId::new("sess-1")?;
        ledger.create_family(RefreshFamily {
            id: family.clone(),
            user_id: user.clone(),
            session_id: session.clone(),
            compromised: false,
            security_version: SecurityVersion(1),
        });
        let first = RefreshCredentialId::new("cred-1")?;
        ledger.issue_credential(RefreshCredential {
            id: first.clone(),
            family_id: family,
            user_id: user,
            session_id: session,
            token_hash: hash_raw("r1"),
            issued_at: now,
            expires_at: at(4_600),
            state: RefreshState::Active,
            replaced_by: None,
            security_version: SecurityVersion(1),
        });
        Ok((ledger, now, first))
    }

    fn successor(
        family: &RefreshFamilyId,
        user: &UserId,
        session: &SessionId,
    ) -> Result<RefreshCredential, AuthError> {
        Ok(RefreshCredential {
            id: RefreshCredentialId::new("cred-2")?,
            family_id: family.clone(),
            user_id: user.clone(),
            session_id: session.clone(),
            token_hash: hash_raw("r2"),
            issued_at: at(1_000),
            expires_at: at(4_600),
            state: RefreshState::Active,
            replaced_by: None,
            security_version: SecurityVersion(1),
        })
    }

    #[test]
    fn rotation_consumes_and_issues_successor() -> Result<(), AuthError> {
        let (mut ledger, now, first) = setup()?;
        let initial = ledger
            .credential(&first)
            .ok_or(AuthError::InvalidIdentifier)?;
        let family = initial.family_id.clone();
        let user = initial.user_id.clone();
        let session = initial.session_id.clone();
        let outcome = ledger.rotate("r1", successor(&family, &user, &session)?, now)?;
        assert_eq!(outcome.previous_id, first);
        assert_eq!(
            ledger
                .credential(&first)
                .ok_or(AuthError::InvalidIdentifier)?
                .state,
            RefreshState::Consumed
        );
        assert_eq!(
            ledger
                .credential(&outcome.successor_id)
                .ok_or(AuthError::InvalidIdentifier)?
                .state,
            RefreshState::Active
        );
        Ok(())
    }

    #[test]
    fn replay_compromises_family() -> Result<(), AuthError> {
        let (mut ledger, now, first) = setup()?;
        let initial = ledger
            .credential(&first)
            .ok_or(AuthError::InvalidIdentifier)?;
        let family = initial.family_id.clone();
        let user = initial.user_id.clone();
        let session = initial.session_id.clone();
        ledger.rotate("r1", successor(&family, &user, &session)?, now)?;
        // Replay of the consumed credential must trigger containment.
        assert!(matches!(
            ledger.rotate("r1", successor(&family, &user, &session)?, now),
            Err(AuthError::RefreshTokenReused)
        ));
        assert!(
            ledger
                .family(&family)
                .ok_or(AuthError::InvalidIdentifier)?
                .compromised
        );
        // Even the valid successor is revoked after replay detection.
        let successor_id = RefreshCredentialId::new("cred-2")?;
        assert_eq!(
            ledger
                .credential(&successor_id)
                .ok_or(AuthError::InvalidIdentifier)?
                .state,
            RefreshState::Revoked
        );
        Ok(())
    }

    #[test]
    fn unknown_token_is_rejected_without_containment() -> Result<(), AuthError> {
        let (mut ledger, now, first) = setup()?;
        assert!(matches!(
            ledger.rotate(
                "unknown",
                successor(
                    &RefreshFamilyId::new("fam-1")?,
                    &UserId::new("user-1")?,
                    &SessionId::new("sess-1")?,
                )?,
                now,
            ),
            Err(AuthError::RefreshTokenInvalid)
        ));
        assert!(
            !ledger
                .family(
                    &ledger
                        .credential(&first)
                        .ok_or(AuthError::InvalidIdentifier)?
                        .family_id,
                )
                .ok_or(AuthError::InvalidIdentifier)?
                .compromised
        );
        Ok(())
    }
}
