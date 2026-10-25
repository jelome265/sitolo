use std::env;

use sqlx_postgres::PgConnectOptions;

/// Validated connection options for the repository's disposable PostgreSQL suites.
pub(super) struct TestDatabaseTargets {
    pub(super) admin: PgConnectOptions,
    pub(super) runtime: PgConnectOptions,
}

/// Parse and validate admin/runtime targets before any network connection or DDL.
#[expect(
    clippy::disallowed_methods,
    reason = "The integration-test boundary reads its explicit disposable-target contract and DSNs; production configuration remains parsed at the binary boundary."
)]
pub(super) fn targets_from_env() -> Result<TestDatabaseTargets, String> {
    let disposable = env::var("SITOLO_TEST_DATABASE_DISPOSABLE").unwrap_or_default();
    let admin = env::var("ADMIN_DATABASE_URL")
        .map_err(|_error| "ADMIN_DATABASE_URL is required".to_owned())?;
    let runtime = env::var("RUNTIME_DATABASE_URL")
        .map_err(|_error| "RUNTIME_DATABASE_URL is required".to_owned())?;
    validate_targets(&disposable, &admin, &runtime)
}

fn validate_targets(
    disposable: &str,
    admin_url: &str,
    runtime_url: &str,
) -> Result<TestDatabaseTargets, String> {
    if disposable != "true" {
        return Err("SITOLO_TEST_DATABASE_DISPOSABLE=true is required".to_owned());
    }

    let admin = parse_target("ADMIN_DATABASE_URL", admin_url, "postgres")?;
    let runtime = parse_target("RUNTIME_DATABASE_URL", runtime_url, "app_runtime")?;
    if !same_database(&admin, &runtime) {
        return Err("test database URLs must target the same host, port, and database".to_owned());
    }

    Ok(TestDatabaseTargets { admin, runtime })
}

fn same_database(left: &PgConnectOptions, right: &PgConnectOptions) -> bool {
    left.get_host() == right.get_host()
        && left.get_port() == right.get_port()
        && left.get_database() == right.get_database()
}

pub(super) fn validate_worker_target(
    targets: &TestDatabaseTargets,
    raw: &str,
) -> Result<PgConnectOptions, String> {
    let worker = parse_target("WORKER_DATABASE_URL", raw, "app_worker")?;
    if !same_database(&targets.admin, &worker) {
        return Err("worker test database URL must match the admin target".to_owned());
    }
    Ok(worker)
}

/// Accepts only the schema naming format generated for one integration-test invocation.
pub(super) fn validate_generated_schema_name(name: &str) -> Result<(), String> {
    let suffix = name
        .strip_prefix("test_part8_")
        .ok_or_else(|| "generated schema name has an invalid prefix".to_owned())?;
    if suffix.len() != 32
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("generated schema name must contain a 32-character lowercase UUID".to_owned());
    }
    Ok(())
}

fn parse_target(name: &str, raw: &str, expected_role: &str) -> Result<PgConnectOptions, String> {
    let options: PgConnectOptions = raw
        .parse()
        .map_err(|_error| format!("{name} is not a valid PostgreSQL connection URL"))?;

    if options.get_host() != "127.0.0.1" || options.get_port() != 5432 {
        return Err(format!(
            "{name} must target loopback PostgreSQL on port 5432"
        ));
    }
    if !matches!(
        options.get_database(),
        Some("sitolo_test" | "sitolo_offensive")
    ) {
        return Err(format!(
            "{name} must target an approved disposable test database"
        ));
    }
    if options.get_username() != expected_role {
        return Err(format!("{name} must authenticate as role {expected_role}"));
    }

    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_policy_rejects_missing_opt_in_before_database_access() {
        // This test uses only parser input and cannot connect to PostgreSQL.
        assert!(validate_targets("false", "", "").is_err());
    }

    #[test]
    fn target_policy_rejects_remote_host_before_database_access() {
        let result = validate_targets(
            "true",
            "postgres://postgres@db.example:5432/sitolo_test",
            "postgres://app_runtime@db.example:5432/sitolo_test",
        );
        assert!(result.is_err());
    }

    #[test]
    fn target_policy_rejects_mismatched_database_before_database_access() {
        let result = validate_targets(
            "true",
            "postgres://postgres@127.0.0.1:5432/sitolo_test",
            "postgres://app_runtime@127.0.0.1:5432/production",
        );
        assert!(result.is_err());
    }

    #[test]
    fn target_policy_rejects_unexpected_role_before_database_access() {
        let result = validate_targets(
            "true",
            "postgres://postgres@127.0.0.1:5432/sitolo_test",
            "postgres://admin@127.0.0.1:5432/sitolo_test",
        );
        assert!(result.is_err());
    }

    #[test]
    fn target_policy_rejects_production_database_name_before_database_access() {
        let result = validate_targets(
            "true",
            "postgres://postgres@127.0.0.1:5432/production",
            "postgres://app_runtime@127.0.0.1:5432/production",
        );
        assert!(result.is_err());
    }

    #[test]
    fn worker_policy_rejects_conflicting_database_before_database_access()
    -> Result<(), Box<dyn std::error::Error>> {
        let targets = validate_targets(
            "true",
            "postgres://postgres@127.0.0.1:5432/sitolo_test",
            "postgres://app_runtime@127.0.0.1:5432/sitolo_test",
        )
        .map_err(std::io::Error::other)?;
        let result = validate_worker_target(
            &targets,
            "postgres://app_worker@127.0.0.1:5432/sitolo_offensive",
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn generated_schema_name_rejects_sql_identifiers_outside_the_uuid_contract() {
        assert!(
            validate_generated_schema_name("test_part8_0123456789abcdef0123456789abcdef").is_ok()
        );
        assert!(
            validate_generated_schema_name(
                "test_part8_0123456789abcdef0123456789abcde\";DROP SCHEMA public;--"
            )
            .is_err()
        );
        assert!(validate_generated_schema_name("public").is_err());
    }
}
