//! In-memory identity database.
//!
//! Reference implementation of [`IdentityStores`] for tests and local
//! development. Each method simulates a transaction: a single lock is held
//! for the duration of the operation, so concurrent callers serialize and
//! the "one legal transition" invariant (§41) is provable with threads.
//! Production implementations replace this with PostgreSQL-backed
//! repositories that enforce the same semantics via row locks and unique
//! constraints (§12.3, §52).
//!
//! The deterministic randomness here is test-only and clearly labeled;
//! production wiring must use an OS CSPRNG through `sitolo_security::RandomSource`.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;

use sitolo_audit::{
    AuditError, AuditRecorder, AuditRequirement, AuthEventName, AuthenticationEvent, EventResult,
    InMemoryAuditSink,
};
use sitolo_auth::{
    Assurance, AuditEventId, AuthenticationMethod, ChallengeId, ChallengePurpose, ClientPlatform,
    Device, DeviceId, MfaAuthenticator, MfaAuthenticatorId, MfaKind, MfaState, RecoveryCodeId,
    RecoveryCodeRecord, RefreshCredential, RefreshCredentialId, RefreshFamily, RefreshFamilyId,
    RefreshLedger, RefreshState, ResetArtifactId, RevocationScope, RevocationTrigger,
    SecurityVersion, Session, SessionClass, SessionId, SessionPolicySet, UserId,
};
use sitolo_security::SealedRef;

use crate::ports::IdentityStores;

#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    pub session: Session,
    pub user_security_version: SecurityVersion,
}

#[derive(Debug, Clone)]
pub struct UserSnapshot {
    pub id: UserId,
    pub security_version: SecurityVersion,
    pub suspended: bool,
    pub has_password: bool,
    pub mfa_active: bool,
    pub password_verifier: Option<String>,
    pub password_policy_version: u32,
}

#[derive(Debug, Clone)]
pub struct EstablishedSession {
    pub session: Session,
    pub refresh_token: String,
    pub family_id: RefreshFamilyId,
}

#[derive(Debug, Clone)]
pub struct RefreshRotation {
    pub family_id: RefreshFamilyId,
    pub user_id: UserId,
    pub session_id: SessionId,
    pub successor_token: String,
}

#[derive(Debug, Clone)]
pub struct MfaEnrollmentResult {
    pub authenticator_id: MfaAuthenticatorId,
    pub challenge_id: ChallengeId,
}

#[derive(Debug, Clone)]
pub struct PasswordResetResult {
    pub user_id: Option<UserId>,
}

#[derive(Debug, Clone)]
pub struct DeviceRevocationEffect {
    pub device: Device,
    pub revoked_sessions: Vec<SessionId>,
}

#[derive(Debug, Clone)]
pub struct DeviceRegistrationInput {
    pub user_id: UserId,
    pub platform: ClientPlatform,
    pub security_version: SecurityVersion,
    pub now: SystemTime,
}

pub struct IdentityDatabase {
    state: Mutex<IdentityState>,
    session_policy: SessionPolicySet,
    refresh_ttl: Duration,
    audit: InMemoryAuditSink,
}

struct IdentityState {
    users: BTreeMap<UserId, UserRecord>,
    sessions: BTreeMap<SessionId, Session>,
    devices: BTreeMap<DeviceId, Device>,
    refresh: RefreshLedger,
    mfa: MfaState,
    resets: BTreeMap<[u8; 32], sitolo_auth::PasswordResetArtifact>,
    event_counter: u64,
}

#[derive(Debug, Clone)]
struct UserRecord {
    id: UserId,
    security_version: SecurityVersion,
    suspended: bool,
    password_verifier: Option<String>,
    password_policy_version: u32,
}

impl IdentityDatabase {
    pub fn new(session_policy: SessionPolicySet, refresh_ttl: Duration) -> Self {
        IdentityDatabase {
            state: Mutex::new(IdentityState {
                users: BTreeMap::new(),
                sessions: BTreeMap::new(),
                devices: BTreeMap::new(),
                refresh: RefreshLedger::new(),
                mfa: MfaState::new(),
                resets: BTreeMap::new(),
                event_counter: 0,
            }),
            session_policy,
            refresh_ttl,
            audit: InMemoryAuditSink::new(),
        }
    }

    fn random_hex(len: usize) -> String {
        let mut out = vec![0u8; len];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_add(42);
        }
        sitolo_auth::hex(&out)
    }
}

#[async_trait]
impl IdentityStores for IdentityDatabase {
    async fn create_user(
        &self,
        id: UserId,
        _password_version: Option<u32>,
        password_verifier: Option<String>,
    ) -> Result<UserSnapshot, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let record = UserRecord {
            id: id.clone(),
            security_version: SecurityVersion(1),
            suspended: false,
            password_verifier: password_verifier.clone(),
            password_policy_version: 1,
        };
        state.users.insert(id.clone(), record);
        Ok(UserSnapshot {
            id,
            security_version: SecurityVersion(1),
            suspended: false,
            has_password: password_verifier.is_some(),
            mfa_active: false,
            password_verifier,
            password_policy_version: 1,
        })
    }

    async fn user_snapshot(&self, id: &UserId) -> Option<UserSnapshot> {
        let state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let record = state.users.get(id)?;
        let mfa_active = state.mfa.active_authenticator(id).is_some();
        Some(UserSnapshot {
            id: record.id.clone(),
            security_version: record.security_version,
            suspended: record.suspended,
            has_password: record.password_verifier.is_some(),
            mfa_active,
            password_verifier: record.password_verifier.clone(),
            password_policy_version: record.password_policy_version,
        })
    }

    async fn set_password_verifier(
        &self,
        id: &UserId,
        verifier: String,
        policy_version: u32,
    ) -> Result<(), sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let record = state
            .users
            .get_mut(id)
            .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?;
        record.password_verifier = Some(verifier);
        record.password_policy_version = policy_version;
        Ok(())
    }

    async fn bump_user_security_version(
        &self,
        id: &UserId,
    ) -> Result<SecurityVersion, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let record = state
            .users
            .get_mut(id)
            .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?;
        record.security_version = record.security_version.next();
        Ok(record.security_version)
    }

    async fn suspend_user(&self, id: &UserId) -> Result<(), sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let record = state
            .users
            .get_mut(id)
            .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?;
        record.suspended = true;
        Ok(())
    }

    async fn establish_session(
        &self,
        user_id: UserId,
        device_id: Option<DeviceId>,
        class: SessionClass,
        method: AuthenticationMethod,
        platform: ClientPlatform,
        assurance: Assurance,
        now: SystemTime,
    ) -> Result<EstablishedSession, sitolo_auth::AuthError> {
        let (session, refresh_token, family_id, event) = {
            let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
            let user = state
                .users
                .get(&user_id)
                .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?;
            if user.suspended {
                return Err(sitolo_auth::AuthError::AuthenticationFailed);
            }
            let security_version = user.security_version;
            let lifetime = self.session_policy.for_class(class);
            let session_id = SessionId::new(format!("sess-{}", Self::random_hex(8)))
                .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?;
            let session = Session::create(
                session_id.clone(),
                user_id.clone(),
                device_id,
                class,
                method,
                assurance,
                security_version,
                lifetime,
                now,
            );
            let family_id = RefreshFamilyId::new(format!("fam-{}", Self::random_hex(8)))
                .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?;
            let refresh_token = Self::random_hex(32);
            let credential_id = RefreshCredentialId::new(format!("cred-{}", Self::random_hex(8)))
                .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?;
            state.refresh.create_family(RefreshFamily {
                id: family_id.clone(),
                user_id: user_id.clone(),
                session_id: session_id.clone(),
                compromised: false,
                security_version,
            });
            state.refresh.issue_credential(RefreshCredential {
                id: credential_id,
                family_id: family_id.clone(),
                user_id: user_id.clone(),
                session_id: session_id.clone(),
                token_hash: sitolo_auth::hash_raw(&refresh_token),
                issued_at: now,
                expires_at: now + self.refresh_ttl,
                state: RefreshState::Active,
                replaced_by: None,
                security_version,
            });
            state.sessions.insert(session_id.clone(), session.clone());
            state.event_counter += 1;
            let event = AuthenticationEvent {
                event_id: AuditEventId::new(format!("evt-{:08}", state.event_counter))
                    .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?,
                event_name: AuthEventName::SessionCreated,
                occurred_at: now,
                request_id: None,
                trace_id: None,
                subject_ref: Some(user_id.as_str().into()),
                session_ref: Some(session_id.as_str().into()),
                device_ref: None,
                client_platform: Some(platform),
                authentication_method: Some(method),
                assurance_level: Some(assurance),
                result: EventResult::Success,
                reason_class: None,
                security_version: Some(security_version),
            };
            (session, refresh_token, family_id, event)
        };
        let _ = self.audit.record(event, AuditRequirement::Mandatory).await;
        Ok(EstablishedSession {
            session,
            refresh_token,
            family_id,
        })
    }

    async fn session_snapshot(&self, id: &SessionId) -> Option<SessionSnapshot> {
        let state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let session = state.sessions.get(id)?;
        let user = state.users.get(&session.user_id)?;
        Some(SessionSnapshot {
            session: session.clone(),
            user_security_version: user.security_version,
        })
    }

    async fn accept_session(
        &self,
        id: &SessionId,
        now: SystemTime,
    ) -> Result<SessionSnapshot, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let user = state
            .users
            .get(
                &state
                    .sessions
                    .get(id)
                    .ok_or(sitolo_auth::AuthError::SessionInvalid)?
                    .user_id,
            )
            .ok_or(sitolo_auth::AuthError::SessionInvalid)?;
        let security_version = user.security_version;
        let session = state
            .sessions
            .get_mut(id)
            .ok_or(sitolo_auth::AuthError::SessionInvalid)?;
        let lifetime = self.session_policy.for_class(session.class);
        session.accept(now, security_version, None)?;
        session.slide_idle(lifetime, now);
        Ok(SessionSnapshot {
            session: session.clone(),
            user_security_version: security_version,
        })
    }

    async fn revoke_session(
        &self,
        id: &SessionId,
        trigger: RevocationTrigger,
    ) -> Result<(), sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let session = state
            .sessions
            .get_mut(id)
            .ok_or(sitolo_auth::AuthError::SessionInvalid)?;
        session.revoke(trigger);
        Ok(())
    }

    async fn revoke_sessions_by_scope(
        &self,
        scope: RevocationScope,
        trigger: RevocationTrigger,
        user_id: Option<&UserId>,
        device_id: Option<&DeviceId>,
    ) -> Result<u32, sitolo_auth::AuthError> {
        if matches!(scope, RevocationScope::AuthenticatorFamily) {
            return Err(sitolo_auth::AuthError::InvalidTransition);
        }
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let mut count = 0u32;
        let sessions: Vec<SessionId> = state.sessions.keys().cloned().collect();
        for sid in sessions {
            let Some(session) = state.sessions.get_mut(&sid) else {
                return Err(sitolo_auth::AuthError::InvalidTransition);
            };
            let matches = match scope {
                RevocationScope::CurrentSession => {
                    (user_id == Some(&session.user_id))
                        && device_id.is_none_or(|did| session.device_id.as_ref() == Some(did))
                }
                RevocationScope::AllSessionsOnCurrentDevice => {
                    device_id.is_some_and(|did| session.device_id.as_ref() == Some(did))
                }
                RevocationScope::AllUserSessions => user_id == Some(&session.user_id),
                RevocationScope::AuthenticatorFamily => {
                    unreachable!("handled before lock acquisition")
                }
            };
            if matches {
                session.revoke(trigger);
                count += 1;
            }
        }
        Ok(count)
    }

    async fn elevate_session_assurance(
        &self,
        id: &SessionId,
        to: Assurance,
    ) -> Result<(), sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let session = state
            .sessions
            .get_mut(id)
            .ok_or(sitolo_auth::AuthError::SessionInvalid)?;
        session.elevate_assurance(to);
        Ok(())
    }

    async fn rotate_refresh(
        &self,
        raw: &str,
        now: SystemTime,
    ) -> Result<RefreshRotation, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let key = sitolo_auth::hash_raw(raw);
        let existing = state
            .refresh
            .find_by_hash(key)
            .ok_or(sitolo_auth::AuthError::RefreshTokenInvalid)?
            .clone();
        let successor_token = Self::random_hex(32);
        let successor_id = RefreshCredentialId::new(format!("cred-{}", Self::random_hex(8)))
            .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?;
        let successor = RefreshCredential {
            id: successor_id.clone(),
            family_id: existing.family_id.clone(),
            user_id: existing.user_id.clone(),
            session_id: existing.session_id.clone(),
            token_hash: sitolo_auth::hash_raw(&successor_token),
            issued_at: now,
            expires_at: now + self.refresh_ttl,
            state: RefreshState::Active,
            replaced_by: None,
            security_version: existing.security_version,
        };
        let outcome = state.refresh.rotate(raw, successor, now)?;
        Ok(RefreshRotation {
            family_id: outcome.family_id,
            user_id: outcome.user_id,
            session_id: outcome.session_id,
            successor_token,
        })
    }

    async fn begin_device_registration(
        &self,
        input: DeviceRegistrationInput,
    ) -> Result<Device, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let device_id = DeviceId::new(format!("dev-{}", Self::random_hex(8)))
            .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?;
        let device = Device::begin_registration(
            device_id,
            input.user_id,
            input.platform,
            input.security_version,
            input.now,
        );
        state.devices.insert(device.id.clone(), device.clone());
        Ok(device)
    }

    async fn complete_device_registration(
        &self,
        device_id: &DeviceId,
        _now: SystemTime,
    ) -> Result<Device, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let device = state
            .devices
            .get_mut(device_id)
            .ok_or(sitolo_auth::AuthError::DeviceNotRegistered)?;
        device.require_verification()?;
        device.complete_verification()?;
        device.activate()?;
        Ok(device.clone())
    }

    async fn device_snapshot(&self, id: &DeviceId) -> Option<Device> {
        let state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        state.devices.get(id).cloned()
    }

    async fn revoke_device(
        &self,
        id: &DeviceId,
        now: SystemTime,
    ) -> Result<DeviceRevocationEffect, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        {
            let device = state
                .devices
                .get_mut(id)
                .ok_or(sitolo_auth::AuthError::DeviceNotRegistered)?;
            device.revoke(now)?;
        }
        let mut revoked_sessions = Vec::new();
        let sessions: Vec<SessionId> = state.sessions.keys().cloned().collect();
        for sid in sessions {
            let Some(session) = state.sessions.get_mut(&sid) else {
                return Err(sitolo_auth::AuthError::InvalidTransition);
            };
            if session.device_id.as_ref() == Some(id) {
                session.revoke(RevocationTrigger::DeviceRevocation);
                revoked_sessions.push(sid);
            }
        }
        let device = state
            .devices
            .get(id)
            .ok_or(sitolo_auth::AuthError::DeviceNotRegistered)?
            .clone();
        Ok(DeviceRevocationEffect {
            device,
            revoked_sessions,
        })
    }

    async fn replace_device(
        &self,
        old: &DeviceId,
        now: SystemTime,
    ) -> Result<Device, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let old_device = state
            .devices
            .get_mut(old)
            .ok_or(sitolo_auth::AuthError::DeviceNotRegistered)?;
        old_device.retire_for_replacement(now)?;
        let new_id = DeviceId::new(format!("dev-{}", Self::random_hex(8)))
            .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?;
        let new_device = Device::begin_registration(
            new_id,
            old_device.registered_by.clone(),
            old_device.platform,
            old_device.security_version,
            now,
        );
        state
            .devices
            .insert(new_device.id.clone(), new_device.clone());
        Ok(new_device)
    }

    async fn begin_mfa_enrollment(
        &self,
        user_id: UserId,
        kind: MfaKind,
        secret_reference: Option<SealedRef>,
        now: SystemTime,
    ) -> Result<MfaEnrollmentResult, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let auth_id = MfaAuthenticatorId::new(format!("mfa-{}", Self::random_hex(8)))
            .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?;
        let security_version = state
            .users
            .get(&user_id)
            .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?
            .security_version;
        state.mfa.begin_enrollment(
            auth_id.clone(),
            user_id.clone(),
            kind,
            secret_reference,
            now,
            security_version,
        )?;
        let challenge_id = ChallengeId::new(format!("chal-{}", Self::random_hex(8)))
            .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?;
        let dummy_session = SessionId::new("enrollment-session")
            .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?;
        state.mfa.issue_challenge(
            challenge_id.clone(),
            ChallengePurpose::MfaEnrollment,
            user_id,
            dummy_session,
            now,
            Duration::from_secs(300),
        );
        state.mfa.require_verification(&auth_id)?;
        Ok(MfaEnrollmentResult {
            authenticator_id: auth_id,
            challenge_id,
        })
    }

    async fn complete_mfa_enrollment(
        &self,
        authenticator_id: &MfaAuthenticatorId,
        now: SystemTime,
    ) -> Result<Vec<String>, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        state.mfa.complete_enrollment(authenticator_id, now)?;
        let user_id = state
            .mfa
            .authenticator(authenticator_id)
            .ok_or(sitolo_auth::AuthError::InvalidTransition)?
            .user_id
            .clone();
        let mut codes = Vec::new();
        for i in 0..8 {
            let code = format!("ABCD-{:04}", i);
            let record = RecoveryCodeRecord {
                id: RecoveryCodeId::new(format!("rec-{}", Self::random_hex(8)))
                    .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?,
                user_id: user_id.clone(),
                code_hash: RecoveryCodeRecord::hash_code(&code),
                created_at: now,
                consumed_at: None,
            };
            state.mfa.add_recovery_code(record);
            codes.push(code);
        }
        Ok(codes)
    }

    async fn record_totp_success(
        &self,
        authenticator_id: &MfaAuthenticatorId,
        step: u64,
    ) -> Result<(), sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        state.mfa.record_totp_success(authenticator_id, step)
    }

    async fn active_authenticator(&self, user_id: &UserId) -> Option<MfaAuthenticator> {
        let state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        state.mfa.active_authenticator(user_id).cloned()
    }

    async fn consume_recovery_code(
        &self,
        user_id: &UserId,
        raw: &str,
        now: SystemTime,
    ) -> Result<RecoveryCodeId, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        state.mfa.consume_recovery_code(user_id, raw, now)
    }

    async fn reset_mfa(
        &self,
        target: &UserId,
        _actor: &UserId,
        now: SystemTime,
    ) -> Result<Vec<MfaAuthenticatorId>, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let revoked = state.mfa.reset_all(target, now);
        Ok(revoked)
    }

    async fn request_password_reset(
        &self,
        user_id: &UserId,
        raw_token: &str,
        now: SystemTime,
        ttl: Duration,
    ) -> Result<PasswordResetResult, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let Some(user) = state.users.get(user_id) else {
            return Ok(PasswordResetResult { user_id: None });
        };
        let artifact = sitolo_auth::PasswordResetArtifact::issue(
            ResetArtifactId::new(format!("reset-{}", Self::random_hex(8)))
                .map_err(|_| sitolo_auth::AuthError::InvalidIdentifier)?,
            user_id.clone(),
            raw_token,
            now,
            ttl,
            user.security_version,
        );
        state.resets.insert(artifact.token_hash, artifact);
        Ok(PasswordResetResult {
            user_id: Some(user_id.clone()),
        })
    }

    async fn redeem_password_reset(
        &self,
        raw_token: &str,
        new_verifier: String,
        new_policy_version: u32,
        now: SystemTime,
    ) -> Result<UserId, sitolo_auth::AuthError> {
        let mut state = self.state.lock().unwrap_or_else(|_| std::process::abort());
        let token_hash = sitolo_auth::PasswordResetArtifact::hash_token(raw_token);
        let user_id = state
            .resets
            .get(&token_hash)
            .ok_or(sitolo_auth::AuthError::RecoveryArtifactInvalid)?
            .user_id
            .clone();
        let security_version = state
            .users
            .get(&user_id)
            .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?
            .security_version;
        state
            .resets
            .get_mut(&token_hash)
            .ok_or(sitolo_auth::AuthError::RecoveryArtifactInvalid)?
            .claim(raw_token, now, security_version)?;
        let user = state
            .users
            .get_mut(&user_id)
            .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?;
        user.password_verifier = Some(new_verifier);
        user.password_policy_version = new_policy_version;
        user.security_version = user.security_version.next();
        let sessions: Vec<SessionId> = state.sessions.keys().cloned().collect();
        for sid in sessions {
            let Some(session) = state.sessions.get_mut(&sid) else {
                return Err(sitolo_auth::AuthError::InvalidTransition);
            };
            if session.user_id == user_id {
                session.revoke(RevocationTrigger::PasswordReset);
            }
        }
        Ok(user_id)
    }

    async fn append_audit(&self, event: AuthenticationEvent) -> Result<(), AuditError> {
        self.audit.record(event, AuditRequirement::Diagnostic).await
    }

    async fn audit_trail(&self) -> Vec<AuthenticationEvent> {
        self.audit.events()
    }
}

#[async_trait]
impl AuditRecorder for IdentityDatabase {
    async fn record(
        &self,
        event: AuthenticationEvent,
        requirement: AuditRequirement,
    ) -> Result<(), AuditError> {
        self.audit.record(event, requirement).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_auth::SessionLifetime;

    #[test]
    fn database_constructs() {
        let policy = SessionPolicySet {
            merchant: SessionLifetime {
                idle: Duration::from_secs(30),
                absolute: Duration::from_secs(60),
            },
            administrator: SessionLifetime {
                idle: Duration::from_secs(60),
                absolute: Duration::from_secs(120),
            },
            break_glass: SessionLifetime {
                idle: Duration::from_secs(10),
                absolute: Duration::from_secs(20),
            },
        };
        let db = IdentityDatabase::new(policy, Duration::from_secs(3600));
        assert!(db.state.lock().is_ok());
    }
}
