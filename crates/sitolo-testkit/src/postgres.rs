//! Safety checks shared by real PostgreSQL integration-test harnesses.

use sqlx_postgres::PgConnectOptions;

/// Parsed, allowlisted targets for disposable PostgreSQL integration tests.
#[derive(Debug)]
pub struct DisposableTargets {
    /// Privileged test-only schema bootstrap target.
    pub admin: PgConnectOptions,
    /// Application runtime target.
    pub runtime: PgConnectOptions,
    /// Background worker target.
    pub worker: PgConnectOptions,
}

/// Validate all three DSNs before opening a connection or executing DDL.
///
/// # Errors
/// Returns an error when disposable opt-in, role, host, port, database name,
/// or cross-target identity does not match the test-only allowlist.
pub fn parse_disposable_targets(
    disposable: &str,
    admin_url: &str,
    runtime_url: &str,
    worker_url: &str,
) -> Result<DisposableTargets, String> {
    if disposable != "true" {
        return Err("SITOLO_TEST_DATABASE_DISPOSABLE=true is required".to_owned());
    }
    let admin = parse_target("ADMIN_DATABASE_URL", admin_url, "postgres")?;
    let runtime = parse_target("RUNTIME_DATABASE_URL", runtime_url, "app_runtime")?;
    let worker = parse_target("WORKER_DATABASE_URL", worker_url, "app_worker")?;
    if !same_database(&admin, &runtime) || !same_database(&admin, &worker) {
        return Err("test database URLs must target the same host, port, and database".to_owned());
    }
    Ok(DisposableTargets {
        admin,
        runtime,
        worker,
    })
}

/// Accept only schema names generated for the repository's integration suites.
///
/// # Errors
/// Returns an error when the name has an unknown suite prefix or does not end
/// in exactly 32 lowercase hexadecimal UUID characters.
pub fn validate_generated_schema_name(name: &str) -> Result<(), String> {
    let suffix = [
        "test_schema_fail_inject_",
        "test_part8_app_",
        "test_membership_tx_",
        "test_schema_",
        "test_part8_",
    ]
    .iter()
    .find_map(|prefix| name.strip_prefix(prefix))
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

fn same_database(left: &PgConnectOptions, right: &PgConnectOptions) -> bool {
    left.get_host() == right.get_host()
        && left.get_port() == right.get_port()
        && left.get_database() == right.get_database()
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

    const ADMIN: &str = "postgres://postgres@127.0.0.1:5432/sitolo_test";
    const RUNTIME: &str = "postgres://app_runtime@127.0.0.1:5432/sitolo_test";
    const WORKER: &str = "postgres://app_worker@127.0.0.1:5432/sitolo_test";

    #[test]
    fn validates_all_disposable_targets_before_network_access() {
        assert!(parse_disposable_targets("true", ADMIN, RUNTIME, WORKER).is_ok());
        assert!(parse_disposable_targets("false", ADMIN, RUNTIME, WORKER).is_err());
        assert!(
            parse_disposable_targets(
                "true",
                ADMIN,
                RUNTIME,
                "postgres://app_worker@evil:5432/sitolo_test"
            )
            .is_err()
        );
        assert!(
            parse_disposable_targets(
                "true",
                ADMIN,
                RUNTIME,
                "postgres://app_worker@127.0.0.1:5432/production"
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_remote_admin_and_runtime_targets_before_network_access() {
        assert!(
            parse_disposable_targets(
                "true",
                "postgres://postgres@db.example:5432/sitolo_test",
                RUNTIME,
                WORKER,
            )
            .is_err()
        );
        assert!(
            parse_disposable_targets(
                "true",
                ADMIN,
                "postgres://app_runtime@db.example:5432/sitolo_test",
                WORKER,
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_role_and_database_mismatches_before_network_access() {
        assert!(
            parse_disposable_targets(
                "true",
                "postgres://admin@127.0.0.1:5432/sitolo_test",
                RUNTIME,
                WORKER,
            )
            .is_err()
        );
        assert!(
            parse_disposable_targets(
                "true",
                ADMIN,
                "postgres://app_runtime@127.0.0.1:5432/production",
                WORKER,
            )
            .is_err()
        );
        assert!(
            parse_disposable_targets(
                "true",
                ADMIN,
                RUNTIME,
                "postgres://app_worker@127.0.0.1:5432/sitolo_offensive",
            )
            .is_err()
        );
    }

    #[test]
    fn generated_schema_names_are_limited_to_known_prefixes_and_uuid_hex() {
        assert!(
            validate_generated_schema_name("test_part8_app_0123456789abcdef0123456789abcdef")
                .is_ok()
        );
        assert!(
            validate_generated_schema_name("test_membership_tx_0123456789abcdef0123456789abcdef")
                .is_ok()
        );
        assert!(
            validate_generated_schema_name(
                "test_schema_fail_inject_0123456789abcdef0123456789abcdef"
            )
            .is_ok()
        );
        assert!(
            validate_generated_schema_name("test_schema_0123456789abcdef0123456789abcdef").is_ok()
        );
        assert!(validate_generated_schema_name("public").is_err());
        assert!(
            validate_generated_schema_name("test_schema_0123456789abcdef0123456789abcdeF").is_err()
        );
        assert!(
            validate_generated_schema_name(
                "test_part8_0123456789abcdef0123456789abcdef; DROP SCHEMA public"
            )
            .is_err()
        );
    }
}
