//! End-to-end process-environment regression for the configuration/secret
//! namespace boundary.
//!
//! [`EnvLoader::load`] reads the real process environment, and the development
//! secret provider resolves its value from the same process environment. These
//! tests exercise that complete ingestion path, not just `from_pairs`, so a
//! future change that makes the two namespaces collide fails here.
//!
//! Environment mutation is `unsafe` in edition 2024 and the library source
//! forbids unsafe code, so the tests live in an integration test target.

#![expect(
    unsafe_code,
    reason = "Rust 2024 environment mutation is unsafe and serialized in this integration-test harness"
)]

use std::ffi::{OsStr, OsString};
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use sitolo_config::{EnvLoader, Environment, UnknownKey, validate};
use sitolo_security::{EnvSecretProvider, SecretProvider};

const SENTINEL: &str = "TEST_ONLY_ENV_NAMESPACE_SECRET_001";
const SECRET_VAR: &str = "SITOLO_SECRET__DATABASE__PASSWORD";
const CONFIG_VAR: &str = "SITOLO__DATABASE__PASSWORD";

/// Serializes process-environment mutation: integration tests in one target
/// share a process, so env-mutating tests must never run concurrently.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn managed(key: &OsStr) -> bool {
    let key = key.to_string_lossy();
    key.starts_with("SITOLO__") || key.starts_with("SITOLO_SECRET__")
}

/// Removes every Sitolo configuration/secret variable for the duration of a
/// test and restores the original process environment on drop.
struct EnvSnapshot {
    saved: Vec<(OsString, OsString)>,
}

impl EnvSnapshot {
    /// # Safety
    ///
    /// Every caller holds `ENV_LOCK`, and no other test target in this package
    /// reads the process environment concurrently.
    unsafe fn isolate() -> Self {
        let managed_vars: Vec<(OsString, OsString)> = std::env::vars_os()
            .filter(|(key, _)| managed(key))
            .collect();
        let mut saved = Vec::with_capacity(managed_vars.len());
        for (key, value) in managed_vars {
            saved.push((key.clone(), value));
            // SAFETY: the caller holds ENV_LOCK.
            unsafe { std::env::remove_var(&key) };
        }
        Self { saved }
    }
}

impl Drop for EnvSnapshot {
    fn drop(&mut self) {
        let current: Vec<OsString> = std::env::vars_os()
            .filter(|(key, _)| managed(key))
            .map(|(key, _)| key)
            .collect();
        // SAFETY: the test holds ENV_LOCK until after this snapshot is dropped.
        unsafe {
            for key in current {
                std::env::remove_var(&key);
            }
            for (key, value) in &self.saved {
                std::env::set_var(key, value);
            }
        }
    }
}

/// Minimal single-poll executor. Provider futures complete without yielding,
/// so a noop waker suffices and no async runtime dependency is introduced.
fn block_on<F: Future>(mut future: F) -> Result<F::Output, std::io::Error> {
    unsafe fn clone(_: *const ()) -> RawWaker {
        raw_waker()
    }
    unsafe fn noop(_: *const ()) {}
    fn raw_waker() -> RawWaker {
        RawWaker::new(
            std::ptr::null(),
            &RawWakerVTable::new(clone, noop, noop, noop),
        )
    }
    // SAFETY: the waker carries no state; clone/drop/wake are no-ops.
    let waker = unsafe { Waker::from_raw(raw_waker()) };
    let mut context = Context::from_waker(&waker);
    // SAFETY: the future is never moved after pinning; it is polled to
    // completion on this stack before this function returns.
    let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
    match pinned.as_mut().poll(&mut context) {
        Poll::Ready(output) => Ok(output),
        Poll::Pending => Err(std::io::Error::other(
            "test provider future yielded unexpectedly",
        )),
    }
}

#[test]
fn configuration_and_secret_value_namespaces_coexist() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = lock_env();
    let _snapshot = unsafe { EnvSnapshot::isolate() };
    // SAFETY: the test holds ENV_LOCK; the snapshot restores the process
    // environment on drop.
    unsafe {
        std::env::set_var("SITOLO__RUNTIME__ENVIRONMENT", "development");
        std::env::set_var("SITOLO__DATABASE__HOST", "127.0.0.1");
        std::env::set_var(SECRET_VAR, SENTINEL);
    }

    let builder = EnvLoader::load().map_err(|error| std::io::Error::other(error.0))?;
    assert_eq!(builder.environment, Environment::Development);
    assert_eq!(builder.db_host, "127.0.0.1");

    let config = validate(builder)?;
    let secret = block_on(EnvSecretProvider::new(false).get(config.db_secret_ref()))??;
    assert_eq!(secret.into_string(), SENTINEL);
    Ok(())
}

#[test]
fn unknown_configuration_key_still_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = lock_env();
    let _snapshot = unsafe { EnvSnapshot::isolate() };
    // SAFETY: the test holds ENV_LOCK; the snapshot restores the process
    // environment on drop.
    unsafe {
        std::env::set_var("SITOLO__RUNTIME__ENVIRONMENT", "development");
        std::env::set_var(CONFIG_VAR, SENTINEL);
    }

    let error: UnknownKey = match EnvLoader::load() {
        Err(error) => error,
        Ok(_) => return Err(std::io::Error::other("unknown key was accepted").into()),
    };
    assert_eq!(error.0, "DATABASE__PASSWORD");
    Ok(())
}
