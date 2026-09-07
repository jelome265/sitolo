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
pub fn hash_raw(raw: &str) -> [u8; 32] {
    Sha256::digest(raw.as_bytes()).into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshState {
    Active,
    Consumed,
    Expired,
    Revoked,
}

#[derive(Debug, Clone)]
pub struct RefreshCredential {
    pub id: RefreshCredentialId,
    pub family_id: RefreshFamilyId,
    pub user_id: UserId,
    pub session_id: SessionId,
    pub token_hash: [u8; 32],
    pub issued_at: SystemTime,
    pub expires_at: SystemTime,
    pub state: RefreshState,
    pub replaced_by: Option<RefreshCredentialId>,
    pub security_version: SecurityVersion,
}

#[derive(Debug, Clone)]
pub struct RefreshFamily {
    pub id: RefreshFamilyId,
    pub user_id: UserId,
    pub session_id: SessionId,
    pub compromised: bool,
    pub security_version: SecurityVersion,
}

/// The outcome of a successful rotation (§12.1).
#[derive(Debug, Clone)]
pub struct RotationOutcome {
    pub family_id: RefreshFamilyId,
    pub user_id: UserId,
    pub session_id: SessionId,
    pub successor_id: RefreshCredentialId,
    pub previous_id: RefreshCredentialId,
}

/// The outcome of a detected replay (§12.2).
#[derive(Debug, Clone)]
pub struct ReplayOutcome {
    pub family_id: RefreshFamilyId,
    pub user_id: UserId,
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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

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
        let previous = self.credentials.get_mut(&id).expect("existing");
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

    pub fn family(&self, id: &RefreshFamilyId) -> Option<&RefreshFamily> {
        self.families.get(id)
    }

    pub fn credential(&self, id: &RefreshCredentialId) -> Option<&RefreshCredential> {
        self.credentials.get(id)
    }

    /// Looks up a credential by its token hash (§12.3). Production
    /// implementations query the database directly; this is for the in-memory
    /// reference.
    pub fn find_by_hash(&self, hash: [u8; 32]) -> Option<&RefreshCredential> {
        let id = self.by_hash.get(&hash)?;
        self.credentials.get(id)
    }

    /// All session identifiers associated with a family (for revocation
    /// cascades, §12.2).
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

    fn setup() -> (RefreshLedger, SystemTime, RefreshCredentialId) {
        let mut ledger = RefreshLedger::new();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let family = RefreshFamilyId::new("fam-1").unwrap();
        let user = UserId::new("user-1").unwrap();
        let session = SessionId::new("sess-1").unwrap();
        ledger.create_family(RefreshFamily {
            id: family.clone(),
            user_id: user.clone(),
            session_id: session.clone(),
            compromised: false,
            security_version: SecurityVersion(1),
        });
        let first = RefreshCredentialId::new("cred-1").unwrap();
        ledger.issue_credential(RefreshCredential {
            id: first.clone(),
            family_id: family.clone(),
            user_id: user.clone(),
            session_id: session.clone(),
            token_hash: hash_raw("r1"),
            issued_at: now,
            expires_at: now + Duration::from_secs(3_600),
            state: RefreshState::Active,
            replaced_by: None,
            security_version: SecurityVersion(1),
        });
        (ledger, now, first)
    }

    fn successor(
        family: &RefreshFamilyId,
        user: &UserId,
        session: &SessionId,
    ) -> RefreshCredential {
        RefreshCredential {
            id: RefreshCredentialId::new("cred-2").unwrap(),
            family_id: family.clone(),
            user_id: user.clone(),
            session_id: session.clone(),
            token_hash: hash_raw("r2"),
            issued_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
            expires_at: SystemTime::UNIX_EPOCH + Duration::from_secs(4_600),
            state: RefreshState::Active,
            replaced_by: None,
            security_version: SecurityVersion(1),
        }
    }

    #[test]
    fn rotation_consumes_and_issues_successor() {
        let (mut ledger, now, first) = setup();
        let family = ledger.credential(&first).unwrap().family_id.clone();
        let user = ledger.credential(&first).unwrap().user_id.clone();
        let session = ledger.credential(&first).unwrap().session_id.clone();
        let outcome = ledger
            .rotate("r1", successor(&family, &user, &session), now)
            .unwrap();
        assert_eq!(outcome.previous_id, first);
        assert_eq!(
            ledger.credential(&first).unwrap().state,
            RefreshState::Consumed
        );
        assert_eq!(
            ledger.credential(&outcome.successor_id).unwrap().state,
            RefreshState::Active
        );
    }

    #[test]
    fn replay_compromises_family() {
        let (mut ledger, now, first) = setup();
        let family = ledger.credential(&first).unwrap().family_id.clone();
        let user = ledger.credential(&first).unwrap().user_id.clone();
        let session = ledger.credential(&first).unwrap().session_id.clone();
        ledger
            .rotate("r1", successor(&family, &user, &session), now)
            .unwrap();
        // Replay of the consumed credential must trigger containment.
        let err = ledger
            .rotate("r1", successor(&family, &user, &session), now)
            .unwrap_err();
        assert_eq!(err, AuthError::RefreshTokenReused);
        assert!(ledger.family(&family).unwrap().compromised);
        // Even the valid successor is revoked after replay detection.
        let successor_id = RefreshCredentialId::new("cred-2").unwrap();
        assert_eq!(
            ledger.credential(&successor_id).unwrap().state,
            RefreshState::Revoked
        );
    }

    #[test]
    fn unknown_token_is_rejected_without_containment() {
        let (mut ledger, now, first) = setup();
        let err = ledger
            .rotate(
                "unknown",
                successor(
                    &RefreshFamilyId::new("fam-1").unwrap(),
                    &UserId::new("user-1").unwrap(),
                    &SessionId::new("sess-1").unwrap(),
                ),
                now,
            )
            .unwrap_err();
        assert_eq!(err, AuthError::RefreshTokenInvalid);
        assert!(
            !ledger
                .family(&ledger.credential(&first).unwrap().family_id)
                .unwrap()
                .compromised
        );
    }
}
