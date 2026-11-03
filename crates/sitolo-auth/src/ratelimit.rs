//! Rate limiting and abuse controls.
//!
//! Phase 3 specification, §23. Authentication endpoints are denial-of-service
//! targets; every externally reachable path must have an abuse-control
//! classification. Controls are layered: per-subject, per-source, progressive
//! backoff, temporary lockouts (no permanent lockouts that enable
//! attacker-driven denial of service).

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

/// Abuse control classification (§23; Phase 4 §38 IAM classes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AbuseClass {
    /// Password or provider login attempt.
    Login,
    /// Request for a password-reset artifact.
    PasswordResetRequest,
    /// Redemption of a password-reset artifact.
    PasswordResetRedeem,
    /// MFA challenge verification.
    MfaVerify,
    /// TOTP code verification.
    TotpVerify,
    /// Passkey authentication completion.
    PasskeyComplete,
    /// Refresh-token rotation.
    Refresh,
    /// Session creation.
    SessionCreate,
    /// Device registration.
    DeviceRegister,
    /// Recovery-code redemption.
    RecoveryRedeem,
    /// Invitation creation.
    InvitationCreate,
    /// Invitation acceptance.
    InvitationAccept,
}

impl AbuseClass {
    /// Stable bounded label for telemetry (§38).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AbuseClass::Login => "login",
            AbuseClass::PasswordResetRequest => "password_reset_request",
            AbuseClass::PasswordResetRedeem => "password_reset_redeem",
            AbuseClass::MfaVerify => "mfa_verify",
            AbuseClass::TotpVerify => "totp_verify",
            AbuseClass::PasskeyComplete => "passkey_complete",
            AbuseClass::Refresh => "refresh",
            AbuseClass::SessionCreate => "session_create",
            AbuseClass::DeviceRegister => "device_register",
            AbuseClass::RecoveryRedeem => "recovery_redeem",
            AbuseClass::InvitationCreate => "invitation_create",
            AbuseClass::InvitationAccept => "invitation_accept",
        }
    }
}

/// Rate limit rule (§23).
#[derive(Debug, Clone, Copy)]
pub struct RateLimitRule {
    /// Attempts permitted inside one window.
    pub max_attempts: u32,
    /// Length of the counting window.
    pub window: Duration,
    /// How long the bucket stays locked after the limit is exceeded.
    pub lockout: Duration,
}

/// Rate limit decision (§23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    /// The attempt is permitted; `remaining` attempts stay in the window.
    Allowed {
        /// Attempts still available before the bucket locks.
        remaining: u32,
    },
    /// The attempt is throttled without a lockout.
    Throttled {
        /// How long the caller should wait before retrying.
        retry_after: Duration,
    },
    /// The bucket is locked until `until`.
    Locked {
        /// End of the current lockout.
        until: SystemTime,
    },
}

/// Rate limiter state (§23).
#[derive(Debug, Default)]
pub struct RateLimiter {
    buckets: BTreeMap<(AbuseClass, String), Bucket>,
}

#[derive(Debug, Clone)]
struct Bucket {
    count: u32,
    window_start: SystemTime,
    locked_until: Option<SystemTime>,
}

impl RateLimiter {
    /// Creates an empty rate limiter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks the rate limit for a given class and key (§23).
    ///
    /// The lockout deadline is server-controlled configuration and the
    /// decision type has no error variant. A checked fallback would either
    /// unlock the bucket immediately or fabricate a deadline, both of which
    /// fail open, so an unrepresentable deadline fails loudly instead.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "The lockout deadline cannot be reported as an error from a decision-returning \
                  API, and every fallback would fail open for a security control."
    )]
    #[must_use]
    pub fn check(
        &mut self,
        class: AbuseClass,
        key: &str,
        rule: &RateLimitRule,
        now: SystemTime,
    ) -> RateLimitDecision {
        let bucket = self
            .buckets
            .entry((class, key.to_string()))
            .or_insert_with(|| Bucket {
                count: 0,
                window_start: now,
                locked_until: None,
            });
        // Check lockout.
        if let Some(until) = bucket.locked_until {
            if now < until {
                return RateLimitDecision::Locked { until };
            }
            bucket.locked_until = None;
            bucket.count = 0;
            bucket.window_start = now;
        }
        // Reset window if elapsed. An unrepresentable window end means the
        // window cannot have elapsed, so the count is retained.
        if bucket
            .window_start
            .checked_add(rule.window)
            .is_some_and(|end| now > end)
        {
            bucket.window_start = now;
            bucket.count = 0;
        }
        // Check limit.
        if bucket.count >= rule.max_attempts {
            bucket.locked_until = Some(now + rule.lockout);
            return RateLimitDecision::Locked {
                until: now + rule.lockout,
            };
        }
        bucket.count = bucket.count.saturating_add(1);
        let remaining = rule.max_attempts.saturating_sub(bucket.count);
        RateLimitDecision::Allowed { remaining }
    }

    /// Records a successful operation (optional; resets count if desired).
    pub fn record_success(&mut self, class: AbuseClass, key: &str) {
        if let Some(bucket) = self.buckets.get_mut(&(class, key.to_string())) {
            bucket.count = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }

    #[test]
    fn rate_limit_allows_within_window() {
        let mut limiter = RateLimiter::new();
        let rule = RateLimitRule {
            max_attempts: 5,
            window: Duration::from_secs(60),
            lockout: Duration::from_secs(300),
        };
        let now = at(1_000);
        for _ in 0..5 {
            let decision = limiter.check(AbuseClass::Login, "user-1", &rule, now);
            assert!(matches!(decision, RateLimitDecision::Allowed { .. }));
        }
        let decision = limiter.check(AbuseClass::Login, "user-1", &rule, now);
        assert!(matches!(decision, RateLimitDecision::Locked { .. }));
    }

    #[test]
    fn lockout_expires() {
        let mut limiter = RateLimiter::new();
        let rule = RateLimitRule {
            max_attempts: 1,
            window: Duration::from_secs(60),
            lockout: Duration::from_secs(10),
        };
        let now = at(1_000);
        let _first = limiter.check(AbuseClass::Login, "user-1", &rule, now);
        let decision = limiter.check(AbuseClass::Login, "user-1", &rule, now);
        assert!(matches!(decision, RateLimitDecision::Locked { .. }));
        let later = at(1_011);
        let decision = limiter.check(AbuseClass::Login, "user-1", &rule, later);
        assert!(matches!(decision, RateLimitDecision::Allowed { .. }));
    }

    #[test]
    fn window_reset_clears_count() {
        let mut limiter = RateLimiter::new();
        let rule = RateLimitRule {
            max_attempts: 3,
            window: Duration::from_secs(60),
            lockout: Duration::from_secs(300),
        };
        let now = at(1_000);
        for _ in 0..3 {
            let _decision = limiter.check(AbuseClass::Login, "user-1", &rule, now);
        }
        let later = at(1_061);
        let decision = limiter.check(AbuseClass::Login, "user-1", &rule, later);
        assert!(matches!(decision, RateLimitDecision::Allowed { .. }));
    }
}
