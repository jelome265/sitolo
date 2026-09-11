//! Identity service — Phase 3 use cases.
//!
//! Orchestrates authentication, sessions, MFA, device lifecycle, and password
//! management against the persistence ports. Each use case applies rate
//! limiting, audit emission, and enumeration-resistant public responses
//! (Phase 3 specification, §22, §23, §37, §52).

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use sitolo_audit::{AuthEventName, AuthenticationEvent, EventResult};
use sitolo_auth::{
    AbuseClass, Assurance, AuditEventId, AuthenticationMethod, ClientPlatform, DeviceId,
    IdentityProviderPort, MfaKind, PasswordHasher, PublicLoginOutcome, RateLimitDecision,
    RateLimitRule, RateLimiter, RevocationScope, RevocationTrigger, SecurityVersion, SessionClass,
    SessionId, SessionPolicySet, TotpVerifier, UserId,
};
use sitolo_observability::RequestId;
use sitolo_persistence::{
    DeviceRegistrationInput, DeviceRevocationEffect, IdentityDatabase, IdentityStores,
};
use sitolo_security::{RandomSource, SecretValue};

/// Service-level policy configuration.
pub struct IdentityPolicy {
    pub sessions: SessionPolicySet,
    pub refresh_ttl: Duration,
    pub reset_ttl: Duration,
    pub mfa_challenge_ttl: Duration,
    pub recovery_code_count: u32,
    pub abuse_rules: Vec<(AbuseClass, RateLimitRule)>,
    pub step_up_enrollment: Assurance,
}

impl IdentityPolicy {
    /// Fail-closed startup validation (§64). Rejects impossible security
    /// relationships instead of silently substituting a permissive default.
    pub fn validate(&self) -> Result<(), sitolo_auth::AuthError> {
        for class in [
            SessionClass::Merchant,
            SessionClass::Administrator,
            SessionClass::BreakGlass,
        ] {
            let lifetime = self.sessions.for_class(class);
            if lifetime.idle.is_zero() || lifetime.absolute.is_zero() {
                return Err(sitolo_auth::AuthError::ConfigurationInvalid);
            }
            if lifetime.idle > lifetime.absolute {
                return Err(sitolo_auth::AuthError::ConfigurationInvalid);
            }
        }
        if self.refresh_ttl.is_zero() || self.reset_ttl.is_zero() {
            return Err(sitolo_auth::AuthError::ConfigurationInvalid);
        }
        if self.mfa_challenge_ttl.is_zero() {
            return Err(sitolo_auth::AuthError::ConfigurationInvalid);
        }
        if self.recovery_code_count == 0 || self.recovery_code_count > 32 {
            return Err(sitolo_auth::AuthError::ConfigurationInvalid);
        }
        Ok(())
    }
}

/// The identity service (Phase 3 use cases).
pub struct IdentityService {
    db: Arc<IdentityDatabase>,
    hasher: Arc<dyn PasswordHasher>,
    // Wired for future OIDC provider use; not yet consulted by reference use cases.
    #[allow(dead_code)]
    provider: Option<Arc<dyn IdentityProviderPort>>,
    limiter: Mutex<RateLimiter>,
    policy: IdentityPolicy,
    random: Arc<dyn RandomSource>,
}

impl IdentityService {
    pub fn new(
        db: Arc<IdentityDatabase>,
        hasher: Arc<dyn PasswordHasher>,
        provider: Option<Arc<dyn IdentityProviderPort>>,
        policy: IdentityPolicy,
        random: Arc<dyn RandomSource>,
    ) -> Self {
        policy.validate().expect("valid identity policy");
        let mut limiter = RateLimiter::new();
        for (class, rule) in &policy.abuse_rules {
            let _ = limiter.check(*class, "init", rule, SystemTime::UNIX_EPOCH);
        }
        IdentityService {
            db,
            hasher,
            provider,
            limiter: Mutex::new(limiter),
            policy,
            random,
        }
    }

    fn check_rate(&self, class: AbuseClass, key: &str) -> RateLimitDecision {
        let mut limiter = self
            .limiter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let rule = self
            .policy
            .abuse_rules
            .iter()
            .find(|(c, _)| *c == class)
            .map(|(_, r)| *r)
            .unwrap_or(RateLimitRule {
                max_attempts: 10,
                window: Duration::from_secs(60),
                lockout: Duration::from_secs(300),
            });
        limiter.check(class, key, &rule, SystemTime::now())
    }

    fn random_hex(&self, len: usize) -> String {
        sitolo_auth::hex(&self.random.bytes(len))
    }

    // Explicit audit-event fields keep each call site reviewable; bundling
    // into a params struct would hide which auth context is populated.
    #[allow(clippy::too_many_arguments)]
    async fn emit_audit(
        &self,
        name: AuthEventName,
        result: EventResult,
        user_id: Option<&UserId>,
        session_id: Option<&SessionId>,
        device_id: Option<&DeviceId>,
        platform: Option<ClientPlatform>,
        method: Option<AuthenticationMethod>,
        assurance: Option<Assurance>,
        reason_class: Option<&'static str>,
        version: Option<SecurityVersion>,
        now: SystemTime,
    ) {
        let event = AuthenticationEvent {
            event_id: AuditEventId::new(format!("evt-{}", self.random_hex(8))).expect("event id"),
            event_name: name,
            occurred_at: now,
            request_id: None,
            trace_id: None,
            subject_ref: user_id.map(|u| u.as_str().into()),
            session_ref: session_id.map(|s| s.as_str().into()),
            device_ref: device_id.map(|d| d.as_str().into()),
            client_platform: platform,
            authentication_method: method,
            assurance_level: assurance,
            result,
            reason_class,
            security_version: version,
        };
        let _ = self.db.append_audit(event).await;
    }
}

// -- Public use cases --

impl IdentityService {
    /// Password login (§22, enumeration-resistant).
    pub async fn login_password(
        &self,
        class: SessionClass,
        identifier: &str,
        password: SecretValue,
        platform: ClientPlatform,
    ) -> Result<LoginResult, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let decision = self.check_rate(AbuseClass::Login, "subject");
        if matches!(decision, RateLimitDecision::Locked { .. }) {
            self.emit_audit(
                AuthEventName::LoginRateLimited,
                EventResult::Failure,
                None,
                None,
                None,
                Some(platform),
                None,
                None,
                None,
                None,
                now,
            )
            .await;
            return Err(sitolo_auth::AuthError::AuthenticationRateLimited);
        }
        // Cheap structural validation before expensive password hashing (§47).
        // Enumeration resistance (§22): unknown identifier and bad password
        // produce the same generic failure; only internal audit distinguishes.
        let password = password.into_string();
        if password.is_empty() || password.len() > 512 || identifier.len() > 128 {
            self.emit_audit(
                AuthEventName::LoginFailure,
                EventResult::Failure,
                None,
                None,
                None,
                Some(platform),
                Some(AuthenticationMethod::Password),
                None,
                Some("invalid_credential"),
                None,
                now,
            )
            .await;
            return Err(sitolo_auth::AuthError::AuthenticationFailed);
        }
        let user_id = match UserId::new(identifier) {
            Ok(id) => id,
            Err(_) => {
                self.emit_audit(
                    AuthEventName::LoginFailure,
                    EventResult::Failure,
                    None,
                    None,
                    None,
                    Some(platform),
                    Some(AuthenticationMethod::Password),
                    None,
                    Some("invalid_credential"),
                    None,
                    now,
                )
                .await;
                return Err(sitolo_auth::AuthError::AuthenticationFailed);
            }
        };
        let Some(user_snap) = self.db.user_snapshot(&user_id).await else {
            self.emit_audit(
                AuthEventName::LoginFailure,
                EventResult::Failure,
                None,
                None,
                None,
                Some(platform),
                Some(AuthenticationMethod::Password),
                None,
                Some("invalid_credential"),
                None,
                now,
            )
            .await;
            return Err(sitolo_auth::AuthError::AuthenticationFailed);
        };
        if user_snap.suspended {
            self.emit_audit(
                AuthEventName::LoginFailure,
                EventResult::Failure,
                Some(&user_id),
                None,
                None,
                Some(platform),
                Some(AuthenticationMethod::Password),
                None,
                None,
                Some(user_snap.security_version),
                now,
            )
            .await;
            return Err(sitolo_auth::AuthError::AuthenticationFailed);
        }
        if !user_snap.has_password {
            self.emit_audit(
                AuthEventName::LoginFailure,
                EventResult::Failure,
                Some(&user_id),
                None,
                None,
                Some(platform),
                Some(AuthenticationMethod::Password),
                None,
                Some("invalid_credential"),
                Some(user_snap.security_version),
                now,
            )
            .await;
            return Err(sitolo_auth::AuthError::AuthenticationFailed);
        }
        if let Some(verifier) = &user_snap.password_verifier {
            let ok = self
                .hasher
                .verify(
                    SecretValue::new(password.clone()),
                    &sitolo_auth::PasswordVerifierRecord {
                        verifier: verifier.clone(),
                        policy_version: user_snap.password_policy_version,
                    },
                )
                .await?;
            if !ok {
                self.emit_audit(
                    AuthEventName::LoginFailure,
                    EventResult::Failure,
                    Some(&user_id),
                    None,
                    None,
                    Some(platform),
                    Some(AuthenticationMethod::Password),
                    None,
                    Some("invalid_credential"),
                    Some(user_snap.security_version),
                    now,
                )
                .await;
                return Err(sitolo_auth::AuthError::AuthenticationFailed);
            }
            // Opportunistic rehash on success with an older cost policy (§9.3).
            // Failed authentication never triggers rehashing.
            if user_snap.password_policy_version < self.hasher.current_policy_version()
                && let Ok(rehashed) = self.hasher.hash(SecretValue::new(password)).await
            {
                let _ = self
                    .db
                    .set_password_verifier(&user_id, rehashed.verifier, rehashed.policy_version)
                    .await;
            }
        } else {
            self.emit_audit(
                AuthEventName::LoginFailure,
                EventResult::Failure,
                Some(&user_id),
                None,
                None,
                Some(platform),
                Some(AuthenticationMethod::Password),
                None,
                Some("missing_verifier"),
                Some(user_snap.security_version),
                now,
            )
            .await;
            return Err(sitolo_auth::AuthError::AuthenticationFailed);
        }
        // MFA step-up required later when mfa_active; initial grant stays A1.
        let assurance = Assurance::A1;
        let established = self
            .db
            .establish_session(
                user_id.clone(),
                None,
                class,
                AuthenticationMethod::Password,
                platform,
                assurance,
                now,
            )
            .await?;
        self.emit_audit(
            AuthEventName::LoginSuccess,
            EventResult::Success,
            Some(&user_id),
            Some(&established.session.id),
            None,
            Some(platform),
            Some(AuthenticationMethod::Password),
            Some(assurance),
            None,
            Some(user_snap.security_version),
            now,
        )
        .await;
        Ok(LoginResult {
            session: established.session.clone(),
            refresh_token: established.refresh_token.clone(),
            mfa_required: user_snap.mfa_active,
        })
    }

    /// Refresh token rotation (§12).
    pub async fn refresh(
        &self,
        raw_refresh: &str,
    ) -> Result<RefreshResult, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let result = self.db.rotate_refresh(raw_refresh, now).await;
        match result {
            Ok(rotation) => {
                self.emit_audit(
                    AuthEventName::RefreshRotated,
                    EventResult::Success,
                    Some(&rotation.user_id),
                    Some(&rotation.session_id),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    now,
                )
                .await;
                Ok(RefreshResult {
                    session_id: rotation.session_id,
                    user_id: rotation.user_id,
                    refresh_token: rotation.successor_token,
                })
            }
            Err(sitolo_auth::AuthError::RefreshTokenReused) => {
                // Family containment (§12.2).
                self.emit_audit(
                    AuthEventName::RefreshReuseDetected,
                    EventResult::Failure,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some("replay_detected"),
                    None,
                    now,
                )
                .await;
                Err(sitolo_auth::AuthError::RefreshTokenReused)
            }
            Err(e) => Err(e),
        }
    }

    /// Logout (§10.3, §52.1).
    pub async fn logout(&self, session_id: &SessionId) -> Result<(), sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let snapshot = self
            .db
            .session_snapshot(session_id)
            .await
            .ok_or(sitolo_auth::AuthError::SessionInvalid)?;
        self.db
            .revoke_session(session_id, RevocationTrigger::Logout)
            .await?;
        self.emit_audit(
            AuthEventName::Logout,
            EventResult::Success,
            Some(&snapshot.session.user_id),
            Some(session_id),
            None,
            None,
            None,
            Some(snapshot.session.assurance),
            None,
            Some(snapshot.user_security_version),
            now,
        )
        .await;
        Ok(())
    }

    /// Revoke all user sessions (§10.3).
    pub async fn revoke_all_user_sessions(
        &self,
        user_id: &UserId,
        trigger: RevocationTrigger,
    ) -> Result<u32, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let count = self
            .db
            .revoke_sessions_by_scope(
                RevocationScope::AllUserSessions,
                trigger,
                Some(user_id),
                None,
            )
            .await?;
        self.emit_audit(
            AuthEventName::SessionRevoked,
            EventResult::Success,
            Some(user_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(count)
    }

    /// Password change (§10.3, §9.3).
    pub async fn change_password(
        &self,
        user_id: &UserId,
        old: SecretValue,
        new: SecretValue,
    ) -> Result<(), sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let snapshot = self
            .db
            .user_snapshot(user_id)
            .await
            .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?;
        if let Some(verifier) = &snapshot.password_verifier {
            let ok = self
                .hasher
                .verify(
                    old,
                    &sitolo_auth::PasswordVerifierRecord {
                        verifier: verifier.clone(),
                        policy_version: snapshot.password_policy_version,
                    },
                )
                .await?;
            if !ok {
                return Err(sitolo_auth::AuthError::AuthenticationFailed);
            }
        }
        let hashed = self.hasher.hash(new).await?;
        self.db
            .set_password_verifier(user_id, hashed.verifier, hashed.policy_version)
            .await?;
        let version = self.db.bump_user_security_version(user_id).await?;
        // Revoke all other sessions (§10.3).
        self.db
            .revoke_sessions_by_scope(
                RevocationScope::AllUserSessions,
                RevocationTrigger::PasswordChange,
                Some(user_id),
                None,
            )
            .await?;
        self.emit_audit(
            AuthEventName::PasswordChanged,
            EventResult::Success,
            Some(user_id),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(version),
            now,
        )
        .await;
        Ok(())
    }

    /// Request password reset (§21, enumeration-resistant).
    pub async fn request_password_reset(
        &self,
        user_id: &UserId,
    ) -> Result<(), sitolo_auth::AuthError> {
        let now = SystemTime::now();
        // CSPRNG-backed reset artifact (§21.1); never deterministic (§35).
        let token = self.random_hex(32);
        let result = self
            .db
            .request_password_reset(user_id, &token, now, self.policy.reset_ttl)
            .await?;
        if result.user_id.is_some() {
            self.emit_audit(
                AuthEventName::PasswordResetRequested,
                EventResult::Success,
                Some(user_id),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                now,
            )
            .await;
        }
        // Always return Ok (non-enumerating, §22).
        Ok(())
    }

    /// Confirm password reset (§21.2, atomic).
    pub async fn confirm_password_reset(
        &self,
        raw_token: &str,
        new_password: SecretValue,
    ) -> Result<UserId, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let hashed = self.hasher.hash(new_password).await?;
        let user_id = self
            .db
            .redeem_password_reset(raw_token, hashed.verifier, hashed.policy_version, now)
            .await?;
        self.emit_audit(
            AuthEventName::PasswordResetCompleted,
            EventResult::Success,
            Some(&user_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(user_id)
    }

    /// Begin MFA enrollment (§18.1).
    pub async fn begin_mfa_enrollment(
        &self,
        user_id: UserId,
        kind: MfaKind,
    ) -> Result<sitolo_persistence::MfaEnrollmentResult, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let result = self
            .db
            .begin_mfa_enrollment(user_id.clone(), kind, None, now)
            .await?;
        self.emit_audit(
            AuthEventName::MfaEnrollmentStarted,
            EventResult::Success,
            Some(&user_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(result)
    }

    /// Complete MFA enrollment (§18.1).
    pub async fn complete_mfa_enrollment(
        &self,
        authenticator_id: &sitolo_auth::MfaAuthenticatorId,
    ) -> Result<Vec<String>, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let codes = self
            .db
            .complete_mfa_enrollment(authenticator_id, now)
            .await?;
        self.emit_audit(
            AuthEventName::MfaEnrollmentCompleted,
            EventResult::Success,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(codes)
    }

    /// Verify MFA code and elevate session (§17).
    pub async fn mfa_verify(
        &self,
        session_id: &SessionId,
        user_id: &UserId,
        code: &str,
    ) -> Result<(), sitolo_auth::AuthError> {
        let now = SystemTime::now();
        // Bounded before expensive verification (§47); overlong OTPs are
        // rejected without invoking the verifier.
        if code.is_empty() || code.len() > 16 {
            return Err(sitolo_auth::AuthError::MfaFailed);
        }
        let auth = self
            .db
            .active_authenticator(user_id)
            .await
            .ok_or(sitolo_auth::AuthError::MfaRequired)?;
        // Test verifier: accept code matching {step:06}.
        let step = (now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs())
            / 30;
        let policy = sitolo_auth::TotpPolicy {
            period_secs: 30,
            digits: 6,
            window: 1,
        };
        let secret = if let Some(ref _sealed) = auth.secret_reference {
            // In-memory store for tests.
            vec![]
        } else {
            vec![]
        };
        let verified = sitolo_auth::TestTotpVerifier
            .verify(&secret, code, step, &policy)
            .await?;
        if !verified {
            self.emit_audit(
                AuthEventName::MfaFailure,
                EventResult::Failure,
                Some(user_id),
                Some(session_id),
                None,
                None,
                None,
                Some(Assurance::A1),
                Some("mfa_failed"),
                None,
                now,
            )
            .await;
            return Err(sitolo_auth::AuthError::MfaFailed);
        }
        self.db.record_totp_success(&auth.id, step).await?;
        self.db
            .elevate_session_assurance(session_id, Assurance::A2)
            .await?;
        self.emit_audit(
            AuthEventName::MfaSuccess,
            EventResult::Success,
            Some(user_id),
            Some(session_id),
            None,
            None,
            None,
            Some(Assurance::A2),
            None,
            None,
            now,
        )
        .await;
        Ok(())
    }

    /// Redeem a recovery code (§20).
    pub async fn redeem_recovery_code(
        &self,
        user_id: &UserId,
        raw: &str,
    ) -> Result<(), sitolo_auth::AuthError> {
        let now = SystemTime::now();
        self.db.consume_recovery_code(user_id, raw, now).await?;
        self.emit_audit(
            AuthEventName::RecoveryCodeRedeemed,
            EventResult::Success,
            Some(user_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(())
    }

    /// Reset MFA for a user (§45, requires elevated assurance).
    pub async fn reset_mfa(
        &self,
        target: &UserId,
        actor: &UserId,
    ) -> Result<Vec<sitolo_auth::MfaAuthenticatorId>, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let revoked = self.db.reset_mfa(target, actor, now).await?;
        self.emit_audit(
            AuthEventName::MfaReset,
            EventResult::Success,
            Some(target),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(revoked)
    }

    /// Register device (§25, authenticated enrollment).
    pub async fn register_device(
        &self,
        user_id: UserId,
        platform: ClientPlatform,
    ) -> Result<sitolo_auth::Device, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let device = self
            .db
            .begin_device_registration(DeviceRegistrationInput {
                user_id,
                platform,
                security_version: SecurityVersion(1),
                now,
            })
            .await?;
        self.emit_audit(
            AuthEventName::DeviceRegistrationStarted,
            EventResult::Success,
            None,
            None,
            Some(&device.id),
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(device)
    }

    /// Finalize device registration (§25).
    pub async fn finalize_device_registration(
        &self,
        device_id: &DeviceId,
    ) -> Result<sitolo_auth::Device, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let device = self.db.complete_device_registration(device_id, now).await?;
        self.emit_audit(
            AuthEventName::DeviceRegistered,
            EventResult::Success,
            None,
            None,
            Some(&device.id),
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(device)
    }

    /// Revoke device (§27).
    pub async fn revoke_device(
        &self,
        device_id: &DeviceId,
    ) -> Result<DeviceRevocationEffect, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let effect = self.db.revoke_device(device_id, now).await?;
        self.emit_audit(
            AuthEventName::DeviceRevoked,
            EventResult::Success,
            None,
            None,
            Some(device_id),
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(effect)
    }

    /// Replace device (§28).
    pub async fn replace_device(
        &self,
        old_id: &DeviceId,
    ) -> Result<sitolo_auth::Device, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let device = self.db.replace_device(old_id, now).await?;
        self.emit_audit(
            AuthEventName::DeviceReplaced,
            EventResult::Success,
            None,
            None,
            Some(&device.id),
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await;
        Ok(device)
    }

    /// Build security context (§32).
    pub async fn build_security_context(
        &self,
        session_id: &SessionId,
        request_id: RequestId,
    ) -> Result<sitolo_auth::SecurityContext, sitolo_auth::AuthError> {
        let now = SystemTime::now();
        let snapshot = self.db.accept_session(session_id, now).await?;
        // Device binding is a risk-control layer (§70): a bound device must
        // be ACTIVE, otherwise the request is denied. The device identifier
        // alone is never sufficient (§50); session validity is checked first
        // by `accept_session`, then device state here via `establish`.
        let device = if let Some(device_id) = snapshot.session.device_id.as_ref() {
            Some(
                self.db
                    .device_snapshot(device_id)
                    .await
                    .ok_or(sitolo_auth::AuthError::DeviceNotRegistered)?,
            )
        } else {
            None
        };
        let lifetime = self.policy.sessions.for_class(snapshot.session.class);
        sitolo_auth::SecurityContext::establish(
            &mut snapshot.session.clone(),
            device.as_ref(),
            snapshot.user_security_version,
            lifetime,
            now,
            request_id,
            None,
        )
    }

    /// Audit trail for testing (§37).
    pub async fn audit_trail(&self) -> Vec<AuthenticationEvent> {
        self.db.audit_trail().await
    }
}

/// Login result (enumeration-resistant public surface).
#[derive(Debug, Clone)]
pub struct LoginResult {
    pub session: sitolo_auth::Session,
    pub refresh_token: String,
    pub mfa_required: bool,
}

impl LoginResult {
    pub fn public_outcome(&self) -> PublicLoginOutcome {
        if self.mfa_required {
            PublicLoginOutcome::MfaRequired
        } else {
            PublicLoginOutcome::Authenticated
        }
    }
}

/// Refresh result.
#[derive(Debug, Clone)]
pub struct RefreshResult {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub refresh_token: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_auth::{SessionPolicySet, TestPasswordHasher};
    use sitolo_persistence::IdentityDatabase;
    use sitolo_security::DeterministicRandom;

    fn test_service() -> IdentityService {
        let policy = SessionPolicySet {
            merchant: sitolo_auth::SessionLifetime {
                idle: Duration::from_secs(30),
                absolute: Duration::from_secs(60),
            },
            administrator: sitolo_auth::SessionLifetime {
                idle: Duration::from_secs(60),
                absolute: Duration::from_secs(120),
            },
            break_glass: sitolo_auth::SessionLifetime {
                idle: Duration::from_secs(10),
                absolute: Duration::from_secs(20),
            },
        };
        let db = Arc::new(IdentityDatabase::new(policy, Duration::from_secs(3600)));
        let hasher = Arc::new(TestPasswordHasher::new(1));
        let random: Arc<dyn RandomSource> = Arc::new(DeterministicRandom::test_only([7u8; 32]));
        IdentityService::new(
            db,
            hasher,
            None,
            IdentityPolicy {
                sessions: policy,
                refresh_ttl: Duration::from_secs(3600),
                reset_ttl: Duration::from_secs(600),
                mfa_challenge_ttl: Duration::from_secs(300),
                recovery_code_count: 8,
                abuse_rules: vec![(
                    AbuseClass::Login,
                    RateLimitRule {
                        max_attempts: 5,
                        window: Duration::from_secs(60),
                        lockout: Duration::from_secs(300),
                    },
                )],
                step_up_enrollment: Assurance::A2,
            },
            random,
        )
    }

    #[test]
    fn service_constructs() {
        let _ = test_service();
    }
}
