//! Integration test proving the identity service composes against the
//! in-memory reference identity store.
//!
//! The reference store lives in the persistence crate, which the application
//! layer may only use from tests, so this test lives outside `src/`.

use std::sync::Arc;
use std::time::Duration;

use sitolo_application::{IdentityPolicy, IdentityService};
use sitolo_auth::{AbuseClass, Assurance, RateLimitRule, SessionPolicySet, TestPasswordHasher};
use sitolo_persistence::IdentityDatabase;
use sitolo_security::{DeterministicRandom, RandomSource};

#[test]
fn service_constructs() {
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
    let _service = IdentityService::new(
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
    );
}
