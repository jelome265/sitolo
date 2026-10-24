//! Version-controlled metadata for every accepted canonical configuration key.

/// Lifecycle class of a configuration key.
///
/// The class decides how a key participates in reload and secret handling; it
/// is metadata only and never carries a configuration or secret value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigClass {
    /// Process-lifetime value that only changes through a redeploy.
    Static,
    /// Reference to a secret in an external secret authority; never the secret
    /// value itself.
    SecretReference,
    /// Bounded operational tuning value that validation keeps below a hard
    /// ceiling.
    Tunable,
}

/// This intentionally records safe schema information only. Secret values are
/// never configuration metadata and are never accepted as ordinary config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigField {
    /// Canonical key name without the `SITOLO__` prefix.
    pub key: &'static str,
    /// Lifecycle class that governs reload and secret handling.
    pub class: ConfigClass,
    /// Layer that may supply the value (environment over compiled default).
    pub source: &'static str,
    /// Documented behaviour when no layer supplies the key.
    pub default_behavior: &'static str,
    /// Whether startup fails when the key is absent.
    pub required: bool,
    /// Whether the value may change without restarting the process.
    pub reloadable: bool,
    /// Domain that owns the value and its semantics.
    pub owner: &'static str,
    /// Accepted syntax, including bounds where applicable.
    pub syntax: &'static str,
    /// Contract when the value is missing or invalid at startup.
    pub startup_failure: &'static str,
    /// Contract for a value that becomes invalid after startup.
    pub runtime_failure: &'static str,
}

macro_rules! field {
    ($key:literal,$class:ident,$required:expr,$owner:literal,$syntax:literal) => {
        ConfigField {
            key: $key,
            class: ConfigClass::$class,
            source: "SITOLO__ environment over compiled safe default",
            default_behavior: "compiled safe default applies when absent",
            required: $required,
            reloadable: false,
            owner: $owner,
            syntax: $syntax,
            startup_failure: "fail startup",
            runtime_failure: "immutable after startup; redeploy with valid configuration",
        }
    };
}
const CATALOGUE: &[ConfigField] = &[
    field!(
        "RUNTIME__ENVIRONMENT",
        Static,
        true,
        "platform",
        "development|staging|production"
    ),
    field!(
        "RUNTIME__SERVICE_NAME",
        Static,
        true,
        "platform",
        "bounded non-empty string"
    ),
    field!(
        "RUNTIME__SERVICE_VERSION",
        Static,
        true,
        "platform",
        "bounded non-empty string"
    ),
    field!(
        "RUNTIME__CONFIG_SCHEMA_VERSION",
        Static,
        true,
        "platform",
        "known u32 schema version"
    ),
    field!(
        "HTTP__BIND_ADDRESS",
        Static,
        true,
        "platform",
        "socket address"
    ),
    field!(
        "HTTP__MAX_BODY_BYTES",
        Static,
        true,
        "platform",
        "u64 <= hard ceiling"
    ),
    field!(
        "HTTP__REQUEST_HEADER_TIMEOUT_MS",
        Tunable,
        true,
        "platform",
        "positive milliseconds <= hard ceiling"
    ),
    field!(
        "HTTP__KEEPALIVE_TIMEOUT_MS",
        Tunable,
        true,
        "platform",
        "positive milliseconds <= hard ceiling"
    ),
    field!("DATABASE__HOST", Static, true, "platform", "bounded host"),
    field!("DATABASE__PORT", Static, true, "platform", "non-zero u16"),
    field!(
        "DATABASE__NAME",
        Static,
        true,
        "platform",
        "bounded database name"
    ),
    field!(
        "DATABASE__SCHEMA",
        Static,
        true,
        "platform",
        "PostgreSQL identifier: lowercase ASCII letters, digits, underscore; maximum 63 bytes"
    ),
    field!(
        "DATABASE__USER",
        Static,
        true,
        "platform",
        "bounded role name"
    ),
    field!(
        "DATABASE__PASSWORD_REF",
        SecretReference,
        true,
        "security",
        "typed secret reference"
    ),
    field!(
        "WORKER__DATABASE__PASSWORD_REF",
        SecretReference,
        false,
        "security",
        "typed database secret reference for app_worker"
    ),
    field!(
        "IDENTITY__OIDC__ISSUER",
        Static,
        false,
        "security",
        "HTTPS Cognito issuer URL"
    ),
    field!(
        "IDENTITY__OIDC__CLIENT_ID",
        Static,
        false,
        "security",
        "bounded Cognito app client identifier"
    ),
    field!(
        "IDENTITY__OIDC__TOKEN_ENDPOINT",
        Static,
        false,
        "security",
        "HTTPS OAuth token endpoint URL"
    ),
    field!(
        "IDENTITY__OIDC__REDIRECT_URI",
        Static,
        false,
        "security",
        "registered HTTPS or sitolo callback URI"
    ),
    field!(
        "DATABASE__POOL_MIN",
        Tunable,
        true,
        "platform",
        "u32 <= hard ceiling"
    ),
    field!(
        "DATABASE__POOL_MAX",
        Tunable,
        true,
        "platform",
        "u32 >= pool_min and <= hard ceiling"
    ),
    field!(
        "DATABASE__ACQUIRE_TIMEOUT_MS",
        Tunable,
        true,
        "platform",
        "positive milliseconds <= hard ceiling"
    ),
    field!(
        "TELEMETRY__OTLP_ENDPOINT",
        Static,
        false,
        "platform",
        "https URL or absent"
    ),
    field!(
        "TELEMETRY__EXPORT_TIMEOUT_MS",
        Tunable,
        true,
        "platform",
        "positive milliseconds <= hard ceiling"
    ),
    field!(
        "TELEMETRY__MAX_QUEUE",
        Tunable,
        true,
        "platform",
        "positive u32 <= hard ceiling"
    ),
    field!(
        "TELEMETRY__TRACE_SAMPLE_RATIO",
        Tunable,
        true,
        "platform",
        "0.0..=1.0"
    ),
    field!(
        "LOG__LEVEL",
        Tunable,
        true,
        "platform",
        "trace|debug|info|warn|error"
    ),
    field!(
        "SECRETS__ALLOW_LOCAL_PROVIDER",
        Static,
        true,
        "security",
        "boolean; false in production"
    ),
    field!(
        "SECRETS__AWS_REGION",
        Static,
        false,
        "security",
        "AWS region identifier for ECS workload secret provider"
    ),
    field!(
        "EVENTS__RELAY__AWS_REGION",
        Static,
        false,
        "platform",
        "AWS region identifier"
    ),
    field!(
        "EVENTS__RELAY__SNS_TOPIC_ARN",
        Static,
        false,
        "platform",
        "SNS FIFO topic ARN"
    ),
    field!(
        "EVENTS__RELAY__BATCH_SIZE",
        Tunable,
        true,
        "platform",
        "positive u32 <= 100"
    ),
    field!(
        "EVENTS__RELAY__MAX_ATTEMPTS",
        Tunable,
        true,
        "platform",
        "positive u32 <= 10"
    ),
    field!(
        "EVENTS__RELAY__BACKOFF_MAX_MS",
        Tunable,
        true,
        "platform",
        "positive milliseconds <= 1 hour"
    ),
];
/// Returns the version-controlled catalogue of every accepted configuration
/// key.
///
/// The catalogue is the schema contract: parser acceptance, validation, and
/// documentation must all agree with it.
#[must_use]
pub fn catalogue() -> &'static [ConfigField] {
    CATALOGUE
}

/// Canonical keys accepted by the parser. Keeping this list next to the
/// catalogue makes parser/catalogue drift an executable contract.
pub const ACCEPTED_KEYS: &[&str] = &[
    "RUNTIME__ENVIRONMENT",
    "RUNTIME__SERVICE_NAME",
    "RUNTIME__SERVICE_VERSION",
    "RUNTIME__CONFIG_SCHEMA_VERSION",
    "HTTP__BIND_ADDRESS",
    "HTTP__MAX_BODY_BYTES",
    "HTTP__REQUEST_HEADER_TIMEOUT_MS",
    "HTTP__KEEPALIVE_TIMEOUT_MS",
    "DATABASE__HOST",
    "DATABASE__PORT",
    "DATABASE__NAME",
    "DATABASE__SCHEMA",
    "DATABASE__USER",
    "DATABASE__PASSWORD_REF",
    "WORKER__DATABASE__PASSWORD_REF",
    "IDENTITY__OIDC__ISSUER",
    "IDENTITY__OIDC__CLIENT_ID",
    "IDENTITY__OIDC__TOKEN_ENDPOINT",
    "IDENTITY__OIDC__REDIRECT_URI",
    "DATABASE__POOL_MIN",
    "DATABASE__POOL_MAX",
    "DATABASE__ACQUIRE_TIMEOUT_MS",
    "TELEMETRY__OTLP_ENDPOINT",
    "TELEMETRY__EXPORT_TIMEOUT_MS",
    "TELEMETRY__MAX_QUEUE",
    "TELEMETRY__TRACE_SAMPLE_RATIO",
    "LOG__LEVEL",
    "SECRETS__ALLOW_LOCAL_PROVIDER",
    "SECRETS__AWS_REGION",
    "EVENTS__RELAY__AWS_REGION",
    "EVENTS__RELAY__SNS_TOPIC_ARN",
    "EVENTS__RELAY__BATCH_SIZE",
    "EVENTS__RELAY__MAX_ATTEMPTS",
    "EVENTS__RELAY__BACKOFF_MAX_MS",
];
