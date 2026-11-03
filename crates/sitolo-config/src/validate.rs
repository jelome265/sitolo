//! Layered validation of the mutable configuration layer into [`AppConfig`].
//!
//! Validation is the only constructor path for [`AppConfig`]. Checks are
//! grouped by the layer that owns them, and every failure is collected so
//! startup diagnostics report the complete set of problems rather than only
//! the first.

use crate::{AppConfig, Environment, KNOWN_SCHEMA_VERSIONS, LogLevel, ceilings, defaults};

/// Validation layer that produced a [`ConfigProblem`].
///
/// Layers run from cheap structural checks to cross-field policy so an
/// operator can distinguish a typo from a security-policy violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLayer {
    /// A value does not match its declared grammar or representation.
    Syntax,
    /// A syntactically valid value violates a type-level or range constraint.
    Semantic,
    /// A value weakens a security invariant such as secret provenance,
    /// transport security, or a hard resource ceiling.
    Security,
    /// A value is valid in general but forbidden in the selected environment.
    Environment,
    /// Two or more individually valid values are inconsistent with each other.
    CrossField,
}

/// One concrete configuration failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProblem {
    /// Layer that produced the problem.
    pub layer: ValidationLayer,
    /// Canonical configuration field or logical field group.
    pub field: &'static str,
    /// Stable, non-secret description of the violated contract.
    pub reason: &'static str,
}

/// The complete set of problems found while validating one configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationError {
    /// Every problem found, in layer order.
    pub problems: Vec<ConfigProblem>,
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid configuration ({} problem(s))",
            self.problems.len()
        )
    }
}

impl std::error::Error for ConfigValidationError {}

/// Records one problem without aborting the remaining checks.
fn push(
    problems: &mut Vec<ConfigProblem>,
    layer: ValidationLayer,
    field: &'static str,
    reason: &'static str,
) {
    problems.push(ConfigProblem {
        layer,
        field,
        reason,
    });
}

/// Validates a layered builder and produces the effective [`AppConfig`].
///
/// This is the only way to construct [`AppConfig`]: missing, malformed,
/// out-of-range, environment-inappropriate, and mutually inconsistent values
/// are rejected before any runtime component observes the configuration.
///
/// # Errors
///
/// Returns [`ConfigValidationError`] when one or more of the following hold:
///
/// - service identity is empty, longer than
///   [`ceilings::MAX_BOUNDED_STRING_BYTES`], or contains control characters;
/// - database identity fields are empty, unbounded, contain control
///   characters, or the schema is not a bounded lowercase PostgreSQL
///   identifier;
/// - the database port is zero, or a non-development API pool does not
///   authenticate as `app_runtime`;
/// - request-body size, header/keep-alive/acquire/export timeouts, OTLP queue
///   depth, or database pool bounds are zero, above a hard ceiling, or
///   inverted;
/// - the trace sample ratio is outside `0.0..=1.0`;
/// - the local secret provider is enabled in production, or a production
///   database/worker reference points at the `development/` secret namespace;
/// - OIDC settings are partially configured, or absent outside development;
/// - the telemetry endpoint is remote (or any endpoint in production) without
///   `https://`;
/// - the required database secret reference is absent, or the worker
///   credential is configured without a complete relay destination (or the
///   reverse);
/// - the relay AWS region, relay SNS FIFO topic ARN, or managed-secret region
///   is malformed or inconsistent;
/// - the log level is more verbose than [`LogLevel::PRODUCTION_CEILING`] in
///   production;
/// - the configuration schema version is not in [`KNOWN_SCHEMA_VERSIONS`].
pub fn validate(b: defaults::Builder) -> Result<AppConfig, ConfigValidationError> {
    let mut problems = Vec::new();
    check_service_identity(&b, &mut problems);
    check_database_identity(&b, &mut problems);
    check_http_limits(&b, &mut problems);
    check_database_pool(&b, &mut problems);
    check_trace_ratio(&b, &mut problems);
    check_telemetry_queue(&b, &mut problems);
    check_secret_provider(&b, &mut problems);
    check_oidc(&b, &mut problems);
    check_managed_secret_region(&b, &mut problems);
    check_log_level(&b, &mut problems);
    check_otel_endpoint(&b, &mut problems);
    check_secret_references(&b, &mut problems);
    check_relay(&b, &mut problems);
    check_schema_version(&b, &mut problems);

    if !problems.is_empty() {
        return Err(ConfigValidationError { problems });
    }
    // `check_secret_references` reports an absent reference, so this branch is
    // unreachable for a builder that passed the checks above. It exists to move
    // the reference out of the builder without `unwrap`, and still fails closed
    // if the checks are ever reordered incorrectly.
    let Some(db_password_ref) = b.db_password_ref else {
        return Err(ConfigValidationError {
            problems: vec![ConfigProblem {
                layer: ValidationLayer::Security,
                field: "db_password_ref",
                reason: "required secret reference absent",
            }],
        });
    };
    Ok(AppConfig {
        environment: b.environment,
        service_name: b.service_name,
        service_version: b.service_version,
        bind_address: b.bind_address,
        max_request_body_bytes: b.max_request_body_bytes,
        request_header_timeout_ms: b.request_header_timeout_ms,
        keepalive_timeout_ms: b.keepalive_timeout_ms,
        db_host: b.db_host,
        db_port: b.db_port,
        db_name: b.db_name,
        db_schema: b.db_schema,
        db_user: b.db_user,
        oidc_issuer: b.oidc_issuer,
        oidc_client_id: b.oidc_client_id,
        oidc_token_endpoint: b.oidc_token_endpoint,
        oidc_redirect_uri: b.oidc_redirect_uri,
        db_password_ref,
        worker_db_password_ref: b.worker_db_password_ref,
        db_pool_min: b.db_pool_min,
        db_pool_max: b.db_pool_max,
        db_acquire_timeout_ms: b.db_acquire_timeout_ms,
        otel_endpoint: b.otel_endpoint,
        otel_export_timeout_ms: b.otel_export_timeout_ms,
        otel_max_queue: b.otel_max_queue,
        trace_sample_ratio: b.trace_sample_ratio,
        log_level: b.log_level,
        allow_local_secret_provider: b.allow_local_secret_provider,
        config_schema_version: b.config_schema_version,
        relay_aws_region: b.relay_aws_region,
        relay_sns_topic_arn: b.relay_sns_topic_arn,
        relay_batch_size: b.relay_batch_size,
        relay_max_attempts: b.relay_max_attempts,
        relay_backoff_max_ms: b.relay_backoff_max_ms,
        secrets_aws_region: b.secrets_aws_region,
    })
}

/// Service identity reaches logs and telemetry, so it is bounded and must not
/// carry control characters (F-005).
fn check_service_identity(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    for (field, value) in [
        ("service_name", b.service_name.as_str()),
        ("service_version", b.service_version.as_str()),
    ] {
        if value.is_empty() || value.len() > ceilings::MAX_BOUNDED_STRING_BYTES {
            push(
                problems,
                ValidationLayer::Semantic,
                field,
                "must be bounded and non-empty",
            );
        }
        if value.bytes().any(|byte| byte.is_ascii_control()) {
            push(
                problems,
                ValidationLayer::Security,
                field,
                "must contain no control characters",
            );
        }
    }
}

/// Database identity is bounded, non-empty, and uses a safe schema grammar.
fn check_database_identity(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if b.db_host.is_empty()
        || b.db_name.is_empty()
        || b.db_user.is_empty()
        || b.db_schema.is_empty()
    {
        push(
            problems,
            ValidationLayer::Semantic,
            "database",
            "identity fields must be non-empty",
        );
    }
    if b.db_schema.len() > 63
        || !b
            .db_schema
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte == b'_')
        || !b
            .db_schema
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        push(
            problems,
            ValidationLayer::Security,
            "db_schema",
            "must be a bounded lowercase PostgreSQL identifier",
        );
    }
    for (field, value) in [
        ("db_host", b.db_host.as_str()),
        ("db_name", b.db_name.as_str()),
        ("db_user", b.db_user.as_str()),
    ] {
        if value.len() > ceilings::MAX_BOUNDED_STRING_BYTES
            || value.bytes().any(|byte| byte.is_ascii_control())
        {
            push(
                problems,
                ValidationLayer::Security,
                field,
                "must be bounded and contain no control characters",
            );
        }
    }
    if b.db_port == 0 {
        push(
            problems,
            ValidationLayer::Semantic,
            "db_port",
            "must be a non-zero TCP port",
        );
    }
    if b.environment != Environment::Development && b.db_user != "app_runtime" {
        push(
            problems,
            ValidationLayer::Security,
            "db_user",
            "staging and production API pools must authenticate directly as app_runtime",
        );
    }
}

/// HTTP body size and timeout values must stay inside their hard ceilings.
fn check_http_limits(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if b.max_request_body_bytes == 0
        || b.max_request_body_bytes > ceilings::MAX_REQUEST_BODY_CEILING_BYTES
    {
        push(
            problems,
            ValidationLayer::Security,
            "max_request_body_bytes",
            "outside hard ceiling",
        );
    }
    for (name, value, max) in [
        (
            "request_header_timeout_ms",
            b.request_header_timeout_ms,
            ceilings::REQUEST_HEADER_TIMEOUT_CEILING_MS,
        ),
        (
            "keepalive_timeout_ms",
            b.keepalive_timeout_ms,
            ceilings::KEEPALIVE_TIMEOUT_CEILING_MS,
        ),
        (
            "db_acquire_timeout_ms",
            b.db_acquire_timeout_ms,
            ceilings::DB_ACQUIRE_TIMEOUT_CEILING_MS,
        ),
        (
            "otel_export_timeout_ms",
            b.otel_export_timeout_ms,
            ceilings::OTEL_EXPORT_TIMEOUT_CEILING_MS,
        ),
    ] {
        if value == 0 || value > max {
            push(
                problems,
                ValidationLayer::Security,
                name,
                "must be positive and below hard ceiling",
            );
        }
    }
}

/// Pool bounds must be ordered and below their ceilings.
fn check_database_pool(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if b.db_pool_min > ceilings::DB_POOL_MIN_CEILING
        || b.db_pool_max > ceilings::DB_POOL_MAX_CEILING
        || b.db_pool_max < b.db_pool_min
    {
        push(
            problems,
            ValidationLayer::CrossField,
            "database_pool",
            "invalid pool bounds",
        );
    }
}

/// The sampling ratio is a probability and must stay in `0.0..=1.0`.
fn check_trace_ratio(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if !(0.0..=1.0).contains(&b.trace_sample_ratio) {
        push(
            problems,
            ValidationLayer::Semantic,
            "trace_sample_ratio",
            "must be in 0..=1",
        );
    }
}

/// The OTLP export queue is bounded and must be non-zero when exporting.
fn check_telemetry_queue(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if b.otel_max_queue == 0 || b.otel_max_queue > ceilings::OTEL_MAX_QUEUE_CEILING {
        push(
            problems,
            ValidationLayer::Security,
            "otel_max_queue",
            "outside hard ceiling",
        );
    }
}

/// Production may not fall back to the development secret provider.
fn check_secret_provider(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if b.environment.is_production() && b.allow_local_secret_provider {
        push(
            problems,
            ValidationLayer::Environment,
            "allow_local_secret_provider",
            "forbidden in production",
        );
    }
}

/// OIDC is all-or-nothing, and non-development environments require it.
fn check_oidc(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    let oidc_values = [
        ("oidc_issuer", b.oidc_issuer.as_deref()),
        ("oidc_client_id", b.oidc_client_id.as_deref()),
        ("oidc_token_endpoint", b.oidc_token_endpoint.as_deref()),
        ("oidc_redirect_uri", b.oidc_redirect_uri.as_deref()),
    ];
    let oidc_count = oidc_values
        .iter()
        .filter(|(_, value)| value.is_some())
        .count();
    if (b.environment != Environment::Development && oidc_count != oidc_values.len())
        || (oidc_count != 0 && oidc_count != oidc_values.len())
    {
        push(
            problems,
            ValidationLayer::Environment,
            "oidc",
            "production requires all OIDC settings and partial configuration is invalid",
        );
    }
    for (field, value) in oidc_values {
        if value.is_some_and(|value| {
            value.is_empty()
                || value.len() > ceilings::MAX_BOUNDED_STRING_BYTES
                || value.bytes().any(|byte| byte.is_ascii_control())
        }) {
            push(
                problems,
                ValidationLayer::Security,
                field,
                "must be bounded, non-empty, and contain no control characters",
            );
        }
    }
}

/// The managed-secret provider region is a bounded AWS region identifier.
fn check_managed_secret_region(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if b.secrets_aws_region
        .as_ref()
        .is_some_and(|region| !is_aws_region(region))
    {
        push(
            problems,
            ValidationLayer::Semantic,
            "secrets_aws_region",
            "must be a bounded AWS region identifier",
        );
    }
}

/// Production logging may not be more verbose than its ceiling.
fn check_log_level(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if b.environment.is_production() && b.log_level < LogLevel::PRODUCTION_CEILING {
        push(
            problems,
            ValidationLayer::Environment,
            "log_level",
            "too verbose in production",
        );
    }
}

/// F-004: transport security is explicit. A missing scheme does not imply a
/// local endpoint: bare `host:port` values are remote unless loopback.
/// Plaintext HTTP to loopback is allowed outside production only.
fn check_otel_endpoint(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if let Some(endpoint) = &b.otel_endpoint
        && !endpoint_allows_plaintext(endpoint, b.environment.is_production())
        && !endpoint.starts_with("https://")
    {
        push(
            problems,
            ValidationLayer::Security,
            "otel_endpoint",
            "remote endpoint requires TLS",
        );
    }
}

/// Secret references must exist, use the right namespace, and pair with the
/// features that consume them.
fn check_secret_references(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    // F-003: secret-reference provenance. Production must never resolve a
    // development-namespace reference, even when the provider would serve it.
    if b.environment.is_production()
        && let Some(reference) = &b.db_password_ref
        && reference.path().starts_with("development/")
    {
        push(
            problems,
            ValidationLayer::Environment,
            "db_password_ref",
            "production must not use a development secret namespace",
        );
    }
    if b.db_password_ref.is_none() {
        push(
            problems,
            ValidationLayer::Security,
            "db_password_ref",
            "required secret reference absent",
        );
    }
    if b.worker_db_password_ref.is_some()
        != (b.relay_aws_region.is_some() && b.relay_sns_topic_arn.is_some())
    {
        push(
            problems,
            ValidationLayer::CrossField,
            "worker_db_password_ref",
            "worker credential and complete relay destination must be configured together",
        );
    }
    if b.environment.is_production()
        && b.worker_db_password_ref
            .as_ref()
            .is_some_and(|reference| reference.path().starts_with("development/"))
    {
        push(
            problems,
            ValidationLayer::Environment,
            "worker_db_password_ref",
            "production must not use a development secret namespace",
        );
    }
}

/// Relay settings are typed, bounded, and internally consistent.
fn check_relay(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if b.relay_aws_region
        .as_ref()
        .is_some_and(|region| !is_aws_region(region))
    {
        push(
            problems,
            ValidationLayer::Semantic,
            "relay_aws_region",
            "must be a bounded AWS region identifier",
        );
    }
    if b.relay_sns_topic_arn
        .as_ref()
        .is_some_and(|arn| !is_sns_fifo_topic_arn(arn, b.relay_aws_region.as_deref()))
    {
        push(
            problems,
            ValidationLayer::Semantic,
            "relay_sns_topic_arn",
            "must be an SNS FIFO topic ARN",
        );
    }
    if b.relay_aws_region.is_some() != b.relay_sns_topic_arn.is_some() {
        push(
            problems,
            ValidationLayer::CrossField,
            "relay_destination",
            "region and topic ARN must be configured together",
        );
    }
    if b.relay_batch_size == 0 || b.relay_batch_size > 100 {
        push(
            problems,
            ValidationLayer::Security,
            "relay_batch_size",
            "must be positive and at most 100",
        );
    }
    if b.relay_max_attempts == 0 || b.relay_max_attempts > 10 {
        push(
            problems,
            ValidationLayer::Security,
            "relay_max_attempts",
            "must be positive and at most 10",
        );
    }
    if b.relay_backoff_max_ms == 0 || b.relay_backoff_max_ms > 3_600_000 {
        push(
            problems,
            ValidationLayer::Security,
            "relay_backoff_max_ms",
            "must be positive and at most one hour",
        );
    }
}

/// Unknown schema versions fail startup instead of being interpreted.
fn check_schema_version(b: &defaults::Builder, problems: &mut Vec<ConfigProblem>) {
    if !KNOWN_SCHEMA_VERSIONS.contains(&b.config_schema_version) {
        push(
            problems,
            ValidationLayer::Environment,
            "config_schema_version",
            "unknown schema version",
        );
    }
}

/// A bounded AWS region identifier such as `af-south-1`.
fn is_aws_region(region: &str) -> bool {
    !region.is_empty()
        && region.len() <= 32
        && region
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Whether `arn` is a six-part SNS topic ARN in `region` whose resource name
/// ends in `.fifo`.
fn is_sns_fifo_topic_arn(arn: &str, region: Option<&str>) -> bool {
    let mut parts = arn.split(':');
    let (Some("arn"), Some("aws"), Some("sns")) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let Some(topic_region) = parts.next() else {
        return false;
    };
    let Some(account) = parts.next() else {
        return false;
    };
    let Some(resource) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    region == Some(topic_region)
        && account.len() == 12
        && account.bytes().all(|byte| byte.is_ascii_digit())
        && resource
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension == "fifo")
}

/// Reports whether `endpoint` is loopback-local. Accepts `scheme://authority`
/// and bare `host[:port]` shapes; the host is loopback only for `localhost`,
/// `127.*`, or `::1` (brackets optional).
fn is_loopback_endpoint(endpoint: &str) -> bool {
    let without_scheme = endpoint
        .split_once("://")
        .map_or(endpoint, |(_, rest)| rest);
    let authority = without_scheme.split(['/', '?', '#']).next().unwrap_or("");
    let hostport = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    // Bracketed IPv6 literal, optionally with a port: `[::1]` / `[::1]:4317`.
    if let Some(inner) = hostport.strip_prefix('[') {
        return inner.split(']').next().is_some_and(|host| host == "::1");
    }
    if hostport.eq_ignore_ascii_case("::1") {
        return true;
    }
    let host = hostport.rsplit_once(':').map_or(hostport, |(h, _)| h);
    if host.is_empty() {
        return false;
    }
    host.eq_ignore_ascii_case("localhost") || host.starts_with("127.")
}

/// Whether `endpoint` may use plaintext transport. Loopback collectors may be
/// plaintext outside production; everything else must be `https://`.
fn endpoint_allows_plaintext(endpoint: &str, production: bool) -> bool {
    is_loopback_endpoint(endpoint) && !production
}
