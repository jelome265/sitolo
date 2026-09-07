//! Version-controlled metadata for every accepted canonical configuration key.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigClass {
    Static,
    SecretReference,
    Tunable,
}

/// This intentionally records safe schema information only. Secret values are
/// never configuration metadata and are never accepted as ordinary config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigField {
    pub key: &'static str,
    pub class: ConfigClass,
    pub source: &'static str,
    pub default_behavior: &'static str,
    pub required: bool,
    pub reloadable: bool,
    pub owner: &'static str,
    pub syntax: &'static str,
    pub startup_failure: &'static str,
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
];
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
    "DATABASE__USER",
    "DATABASE__PASSWORD_REF",
    "DATABASE__POOL_MIN",
    "DATABASE__POOL_MAX",
    "DATABASE__ACQUIRE_TIMEOUT_MS",
    "TELEMETRY__OTLP_ENDPOINT",
    "TELEMETRY__EXPORT_TIMEOUT_MS",
    "TELEMETRY__MAX_QUEUE",
    "TELEMETRY__TRACE_SAMPLE_RATIO",
    "LOG__LEVEL",
    "SECRETS__ALLOW_LOCAL_PROVIDER",
];
