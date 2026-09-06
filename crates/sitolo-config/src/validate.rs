use crate::{AppConfig, KNOWN_SCHEMA_VERSIONS, LogLevel, ceilings, defaults};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLayer {
    Syntax,
    Semantic,
    Security,
    Environment,
    CrossField,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProblem {
    pub layer: ValidationLayer,
    pub field: &'static str,
    pub reason: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationError {
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
pub fn validate(b: defaults::Builder) -> Result<AppConfig, ConfigValidationError> {
    let mut p = Vec::new();
    let mut add = |l, f, r| {
        p.push(ConfigProblem {
            layer: l,
            field: f,
            reason: r,
        })
    };
    if b.service_name.is_empty() || b.service_name.len() > ceilings::MAX_BOUNDED_STRING_BYTES {
        add(
            ValidationLayer::Semantic,
            "service_name",
            "must be bounded and non-empty",
        );
    }
    if b.service_version.is_empty() || b.service_version.len() > ceilings::MAX_BOUNDED_STRING_BYTES
    {
        add(
            ValidationLayer::Semantic,
            "service_version",
            "must be bounded and non-empty",
        );
    }
    if b.db_host.is_empty() || b.db_name.is_empty() || b.db_user.is_empty() {
        add(
            ValidationLayer::Semantic,
            "database",
            "identity fields must be non-empty",
        );
    }
    if b.max_request_body_bytes == 0
        || b.max_request_body_bytes > ceilings::MAX_REQUEST_BODY_CEILING_BYTES
    {
        add(
            ValidationLayer::Security,
            "max_request_body_bytes",
            "outside hard ceiling",
        );
    }
    for (name, v, max) in [
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
        if v == 0 || v > max {
            add(
                ValidationLayer::Security,
                name,
                "must be positive and below hard ceiling",
            );
        }
    }
    if b.db_pool_min > ceilings::DB_POOL_MIN_CEILING
        || b.db_pool_max > ceilings::DB_POOL_MAX_CEILING
        || b.db_pool_max < b.db_pool_min
    {
        add(
            ValidationLayer::CrossField,
            "database_pool",
            "invalid pool bounds",
        );
    }
    if !(0.0..=1.0).contains(&b.trace_sample_ratio) {
        add(
            ValidationLayer::Semantic,
            "trace_sample_ratio",
            "must be in 0..=1",
        );
    }
    if b.otel_max_queue == 0 || b.otel_max_queue > ceilings::OTEL_MAX_QUEUE_CEILING {
        add(
            ValidationLayer::Security,
            "otel_max_queue",
            "outside hard ceiling",
        );
    }
    if b.environment.is_production() && b.allow_local_secret_provider {
        add(
            ValidationLayer::Environment,
            "allow_local_secret_provider",
            "forbidden in production",
        );
    }
    if b.environment.is_production() && b.log_level < LogLevel::PRODUCTION_CEILING {
        add(
            ValidationLayer::Environment,
            "log_level",
            "too verbose in production",
        );
    }
    if let Some(endpoint) = &b.otel_endpoint
        && endpoint.contains("://")
        && !endpoint.starts_with("https://")
    {
        add(
            ValidationLayer::Security,
            "otel_endpoint",
            "remote endpoint requires TLS",
        );
    }
    if b.db_password_ref.is_none() {
        add(
            ValidationLayer::Security,
            "db_password_ref",
            "required secret reference absent",
        );
    }
    if !KNOWN_SCHEMA_VERSIONS.contains(&b.config_schema_version) {
        add(
            ValidationLayer::Environment,
            "config_schema_version",
            "unknown schema version",
        );
    }
    if !p.is_empty() {
        return Err(ConfigValidationError { problems: p });
    }
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
        db_user: b.db_user,
        db_password_ref: b.db_password_ref.expect("validated"),
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
    })
}
