//! Version-controlled schema metadata for every accepted configuration key.

/// Operational configuration class from Phase 2 §5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigClass {
    Static,
    SecretReference,
    Tunable,
}

/// Safe, machine-readable metadata. It deliberately contains no secret values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigField {
    pub key: &'static str,
    pub class: ConfigClass,
    pub required: bool,
    pub reloadable: bool,
    pub owner: &'static str,
}

const CATALOGUE: &[ConfigField] = &[
    ConfigField {
        key: "RUNTIME__ENVIRONMENT",
        class: ConfigClass::Static,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "RUNTIME__SERVICE_NAME",
        class: ConfigClass::Static,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "RUNTIME__SERVICE_VERSION",
        class: ConfigClass::Static,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "HTTP__BIND_ADDRESS",
        class: ConfigClass::Static,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "HTTP__MAX_BODY_BYTES",
        class: ConfigClass::Static,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "DATABASE__HOST",
        class: ConfigClass::Static,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "DATABASE__PORT",
        class: ConfigClass::Static,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "DATABASE__NAME",
        class: ConfigClass::Static,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "DATABASE__USER",
        class: ConfigClass::Static,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "DATABASE__PASSWORD_REF",
        class: ConfigClass::SecretReference,
        required: true,
        reloadable: false,
        owner: "security",
    },
    ConfigField {
        key: "DATABASE__POOL_MIN",
        class: ConfigClass::Tunable,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "DATABASE__POOL_MAX",
        class: ConfigClass::Tunable,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "TELEMETRY__OTLP_ENDPOINT",
        class: ConfigClass::Static,
        required: false,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "TELEMETRY__TRACE_SAMPLE_RATIO",
        class: ConfigClass::Tunable,
        required: true,
        reloadable: false,
        owner: "platform",
    },
    ConfigField {
        key: "TELEMETRY__MAX_QUEUE",
        class: ConfigClass::Tunable,
        required: true,
        reloadable: false,
        owner: "platform",
    },
];

pub fn catalogue() -> &'static [ConfigField] {
    CATALOGUE
}
