//! Executable secret-provider boundary tests (audit F-012/F-013).
//!
//! These live in `tests/` rather than inline because environment mutation is
//! `unsafe` in edition 2024 and library sources forbid unsafe code outright.
//! Each test scopes its mutation and no other test target in this package
//! reads process environment, so a scoped set/remove cannot race a reader.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use sitolo_security::{EnvSecretProvider, SecretClass, SecretError, SecretProvider, SecretRef};

const VAR: &str = "SITOLO_SECRET__DATABASE__PASSWORD";
const CONFIG_VAR: &str = "SITOLO__DATABASE__PASSWORD";
const SENTINEL: &str = "TEST_ONLY_PROVIDER_SECRET_001";

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
            "test provider future unexpectedly yielded",
        )),
    }
}

fn database_ref(path: &str) -> Result<SecretRef, Box<dyn std::error::Error>> {
    Ok(SecretRef::new(SecretClass::Database, path)?)
}

/// Serializes process-environment mutation: integration tests in one target
/// share a process, so env-mutating tests must never run concurrently.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

// SAFETY for the helpers below: every caller holds `ENV_LOCK`, and no other
// test in this package reads process environment.
unsafe fn set_var() {
    unsafe {
        std::env::set_var(VAR, SENTINEL);
    }
}

unsafe fn remove_var() {
    unsafe {
        std::env::remove_var(VAR);
    }
}

#[test]
fn production_get_is_forbidden_without_reading_environment()
-> Result<(), Box<dyn std::error::Error>> {
    // The sentinel proves the guard fires before any environment read:
    // even with a configured value present, production is refused.
    let _guard = lock_env();
    unsafe { set_var() };
    let denied = block_on(EnvSecretProvider::new(true).get(&database_ref("prod/sitolo/db")?))?;
    unsafe { remove_var() };
    assert!(
        matches!(denied, Err(SecretError::LocalProviderForbidden)),
        "production provider must fail closed, got: {denied:?}"
    );
    Ok(())
}

#[test]
fn development_resolves_configured_value() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = lock_env();
    unsafe { set_var() };
    let value =
        block_on(EnvSecretProvider::new(false).get(&database_ref("development/sitolo/db")?))??;
    unsafe { remove_var() };
    assert_eq!(value.into_string(), SENTINEL);
    Ok(())
}

#[test]
fn configuration_namespace_is_not_a_secret_value_source() -> Result<(), Box<dyn std::error::Error>>
{
    // The provider must read only the dedicated `SITOLO_SECRET__` namespace.
    // A value under the former `SITOLO__` configuration name is ignored rather
    // than treated as a secret, so a leftover variable cannot silently become
    // the credential.
    let _guard = lock_env();
    unsafe {
        std::env::remove_var(VAR);
        std::env::set_var(CONFIG_VAR, SENTINEL);
    }
    let value =
        block_on(EnvSecretProvider::new(false).get(&database_ref("development/sitolo/db")?))?;
    unsafe { std::env::remove_var(CONFIG_VAR) };
    assert!(
        matches!(&value, Err(SecretError::Missing { .. })),
        "configuration namespace must not satisfy a secret read, got: {value:?}"
    );
    Ok(())
}

#[test]
fn missing_and_empty_secrets_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = lock_env();
    unsafe { remove_var() };
    let missing =
        block_on(EnvSecretProvider::new(false).get(&database_ref("development/sitolo/db")?))?;
    assert!(
        matches!(missing, Err(SecretError::Missing { .. })),
        "absent secret must fail closed, got: {missing:?}"
    );
    unsafe {
        std::env::set_var(VAR, "");
    }
    let empty =
        block_on(EnvSecretProvider::new(false).get(&database_ref("development/sitolo/db")?))?;
    unsafe { remove_var() };
    assert!(
        matches!(empty, Err(SecretError::Missing { .. })),
        "empty secret must fail closed, got: {empty:?}"
    );
    Ok(())
}

#[test]
fn class_only_mapping_is_pinned() -> Result<(), Box<dyn std::error::Error>> {
    // F-013, Model 1: two references of the same class resolve through the
    // same variable by design; the path selects nothing.
    let _guard = lock_env();
    unsafe { set_var() };
    let first =
        block_on(EnvSecretProvider::new(false).get(&database_ref("development/sitolo/db-a")?))??;
    let second =
        block_on(EnvSecretProvider::new(false).get(&database_ref("development/sitolo/db-b")?))??;
    unsafe { remove_var() };
    assert_eq!(first.into_string(), SENTINEL);
    assert_eq!(second.into_string(), SENTINEL);
    Ok(())
}
