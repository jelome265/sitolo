use std::env;

use sitolo_testkit::postgres::{
    parse_disposable_targets, validate_generated_schema_name as validate_schema,
};
use sqlx_postgres::PgConnectOptions;

pub(super) struct TestDatabaseTargets {
    pub(super) admin: PgConnectOptions,
    pub(super) runtime: PgConnectOptions,
    pub(super) worker: PgConnectOptions,
}

#[expect(
    clippy::disallowed_methods,
    reason = "Integration harness reads the explicit disposable-target contract before connecting."
)]
pub(super) fn targets_from_env() -> Result<TestDatabaseTargets, String> {
    let disposable = env::var("SITOLO_TEST_DATABASE_DISPOSABLE").unwrap_or_default();
    let admin = env::var("ADMIN_DATABASE_URL")
        .map_err(|_error| "ADMIN_DATABASE_URL is required".to_owned())?;
    let runtime = env::var("RUNTIME_DATABASE_URL")
        .map_err(|_error| "RUNTIME_DATABASE_URL is required".to_owned())?;
    let worker = env::var("WORKER_DATABASE_URL")
        .map_err(|_error| "WORKER_DATABASE_URL is required".to_owned())?;
    let targets = parse_disposable_targets(&disposable, &admin, &runtime, &worker)?;
    Ok(TestDatabaseTargets {
        admin: targets.admin,
        runtime: targets.runtime,
        worker: targets.worker,
    })
}

pub(super) fn validate_generated_schema_name(name: &str) -> Result<(), String> {
    validate_schema(name)
}

#[cfg(test)]
mod tests {
    use sitolo_testkit::postgres::parse_disposable_targets;

    #[test]
    fn rejects_non_disposable_targets_before_network_access() {
        assert!(parse_disposable_targets("false", "", "", "").is_err());
        assert!(
            parse_disposable_targets(
                "true",
                "postgres://postgres@127.0.0.1:5432/sitolo_test",
                "postgres://app_runtime@127.0.0.1:5432/sitolo_test",
                "postgres://app_worker@evil:5432/sitolo_test",
            )
            .is_err()
        );
    }
}
