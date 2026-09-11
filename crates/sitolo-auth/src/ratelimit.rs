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
    Login,
    PasswordResetRequest,
    PasswordResetRedeem,
    MfaVerify,
    TotpVerify,
    PasskeyComplete,
    Refresh,
    SessionCreate,
    DeviceRegister,
    RecoveryRedeem,
    InvitationCreate,
    InvitationAccept,
}

impl AbuseClass {
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
    pub max_attempts: u32,
    pub window: Duration,
    pub lockout: Duration,
}

/// Rate limit decision (§23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allowed { remaining: u32 },
    Throttled { retry_after: Duration },
    Locked { until: SystemTime },
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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks the rate limit for a given class and key (§23).
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
        // Reset window if elapsed.
        if now > bucket.window_start + rule.window {
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
        bucket.count += 1;
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

    #[test]
    fn rate_limit_allows_within_window() {
        let mut limiter = RateLimiter::new();
        let rule = RateLimitRule {
            max_attempts: 5,
            window: Duration::from_secs(60),
            lockout: Duration::from_secs(300),
        };
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
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
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let _ = limiter.check(AbuseClass::Login, "user-1", &rule, now);
        let decision = limiter.check(AbuseClass::Login, "user-1", &rule, now);
        assert!(matches!(decision, RateLimitDecision::Locked { .. }));
        let later = now + Duration::from_secs(11);
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
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        for _ in 0..3 {
            let _ = limiter.check(AbuseClass::Login, "user-1", &rule, now);
        }
        let later = now + Duration::from_secs(61);
        let decision = limiter.check(AbuseClass::Login, "user-1", &rule, later);
        assert!(matches!(decision, RateLimitDecision::Allowed { .. }));
    }
}
