//! Full process-environment bootstrap regression for the configuration /
//! secret namespace boundary.
//!
//! The existing startup tests inject configuration pairs and a stub provider.
//! These tests deliberately exercise [`StartupContext::build`] instead, so the
//! real [`EnvLoader`] and development [`EnvSecretProvider`] see one process
//! environment. Environment mutation is `unsafe` in edition 2024 and is kept
//! to this integration-test target.

use sitolo_api_bin::bootstrap::{Readiness, StartupContext, StartupError};

const SENTINEL: &str = "TEST_ONLY_STARTUP_ENV_SECRET_001";
const SECRET_KEY: &str = "SITOLO_SECRET__DATABASE__PASSWORD";
const OLD_CONFIG_KEY: &str = "SITOLO__DATABASE__PASSWORD";
const CONFIG_KEYS: [&str; 2] = ["SITOLO__RUNTIME__ENVIRONMENT", "SITOLO__DATABASE__HOST"];

/// Serializes process-environment mutation within this test binary.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Removes the variables this test sets, including on panic.
struct EnvGuard;

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: every test holds ENV_LOCK until after this guard is dropped.
        unsafe {
            for key in CONFIG_KEYS {
                std::env::remove_var(key);
            }
            std::env::remove_var(SECRET_KEY);
            std::env::remove_var(OLD_CONFIG_KEY);
        }
    }
}

#[tokio::test]
async fn process_environment_config_and_secret_coexist() {
    let _lock = ENV_LOCK.lock().await;
    let _guard = EnvGuard;
    // SAFETY: the test holds ENV_LOCK; EnvGuard removes these keys on drop.
    unsafe {
        std::env::set_var("SITOLO__RUNTIME__ENVIRONMENT", "development");
        std::env::set_var("SITOLO__DATABASE__HOST", "127.0.0.1");
        std::env::set_var(SECRET_KEY, SENTINEL);
        std::env::remove_var(OLD_CONFIG_KEY);
    }

    let context = StartupContext::build()
        .await
        .expect("configuration and a development secret value must coexist");
    assert_eq!(context.readiness(), Readiness::NotReady);
    assert_eq!(context.config().db_host, "127.0.0.1");
}

#[tokio::test]
async fn unknown_configuration_key_still_fails_closed() {
    let _lock = ENV_LOCK.lock().await;
    let _guard = EnvGuard;
    // SAFETY: the test holds ENV_LOCK; EnvGuard removes these keys on drop.
    unsafe {
        std::env::set_var("SITOLO__RUNTIME__ENVIRONMENT", "development");
        std::env::set_var(OLD_CONFIG_KEY, SENTINEL);
        std::env::remove_var(SECRET_KEY);
    }

    let error = StartupContext::build()
        .await
        .err()
        .expect("the former configuration-namespace password key must fail startup");
    assert!(
        matches!(error, StartupError::UnknownConfigKey { ref key } if key == "DATABASE__PASSWORD"),
        "unexpected startup failure: {error:?}"
    );
}
