//! Closed operational event and metric registries.
//!
//! Every event and metric the service may emit must appear here first. The
//! registries are the single source of truth for names, schema versions, and
//! label keys, so instrumentation cannot invent unbounded dimensions or
//! attacker-controlled field names at runtime.

/// Registry entry for one operational event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventDefinition {
    /// Stable dotted event name used for lookup and emission.
    pub name: &'static str,
    /// Schema version emitted with the event; bump when fields change.
    pub schema_version: u16,
    /// Team accountable for the event's schema and alerting.
    pub owner: &'static str,
}

/// Closed set of operational events permitted to reach the log pipeline.
///
/// Lookup goes through [`event`]; names absent from this slice are rejected
/// instead of being emitted as free-form text.
pub const EVENTS: &[EventDefinition] = &[
    EventDefinition {
        name: "service.starting",
        schema_version: 1,
        owner: "platform",
    },
    EventDefinition {
        name: "service.ready",
        schema_version: 1,
        owner: "platform",
    },
    EventDefinition {
        name: "configuration.loaded",
        schema_version: 1,
        owner: "platform",
    },
    EventDefinition {
        name: "configuration.validation.failed",
        schema_version: 1,
        owner: "platform",
    },
    EventDefinition {
        name: "secret.access.failed",
        schema_version: 1,
        owner: "security",
    },
    EventDefinition {
        name: "http.request.completed",
        schema_version: 1,
        owner: "api",
    },
    EventDefinition {
        name: "application.error",
        schema_version: 1,
        owner: "platform",
    },
    EventDefinition {
        name: "telemetry.export.failed",
        schema_version: 1,
        owner: "platform",
    },
    EventDefinition {
        name: "telemetry.record.dropped",
        schema_version: 1,
        owner: "platform",
    },
    EventDefinition {
        name: "external.request.unknown_outcome",
        schema_version: 1,
        owner: "integrations",
    },
];

/// Looks up a registered event by exact name.
///
/// Returns `None` for unregistered names, which is how callers refuse to
/// emit arbitrary or attacker-supplied event text.
#[must_use]
pub fn event(name: &str) -> Option<&'static EventDefinition> {
    EVENTS.iter().find(|e| e.name == name)
}

/// Metric instrument type, which determines aggregation semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    /// Monotonically increasing total.
    Counter,
    /// Distribution of observed values.
    Histogram,
    /// Point-in-time value that can rise and fall.
    Gauge,
}

/// Registry entry for one metric instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricDefinition {
    /// Stable metric name exposed to the collection pipeline.
    pub name: &'static str,
    /// Instrument type that determines aggregation semantics.
    pub kind: MetricKind,
    /// Unit of the recorded value.
    pub unit: &'static str,
    /// Closed set of label keys; values must never carry request content.
    pub labels: &'static [&'static str],
}

/// Closed set of metric instruments the service is allowed to emit.
///
/// Lookup goes through [`metric`]; unregistered names are not exported.
pub const METRICS: &[MetricDefinition] = &[
    MetricDefinition {
        name: "sitolo_http_requests_total",
        kind: MetricKind::Counter,
        unit: "requests",
        labels: &["method", "route_template", "status_class"],
    },
    MetricDefinition {
        name: "sitolo_http_request_duration_seconds",
        kind: MetricKind::Histogram,
        unit: "seconds",
        labels: &["method", "route_template"],
    },
    MetricDefinition {
        name: "sitolo_errors_total",
        kind: MetricKind::Counter,
        unit: "errors",
        labels: &["error_family", "operation"],
    },
    MetricDefinition {
        name: "sitolo_configuration_load_failures_total",
        kind: MetricKind::Counter,
        unit: "failures",
        labels: &["reason"],
    },
    MetricDefinition {
        name: "sitolo_secret_access_failures_total",
        kind: MetricKind::Counter,
        unit: "failures",
        labels: &["secret_class"],
    },
    MetricDefinition {
        name: "sitolo_telemetry_dropped_records_total",
        kind: MetricKind::Counter,
        unit: "records",
        labels: &["priority"],
    },
    MetricDefinition {
        name: "sitolo_db_pool_wait_seconds",
        kind: MetricKind::Histogram,
        unit: "seconds",
        labels: &["result"],
    },
    MetricDefinition {
        name: "sitolo_worker_jobs_total",
        kind: MetricKind::Counter,
        unit: "jobs",
        labels: &["job_type", "result"],
    },
];

/// Looks up a registered metric by exact name.
///
/// Returns `None` for unregistered names so instrumentation cannot create
/// unbounded metric or label dimensions at runtime.
#[must_use]
pub fn metric(name: &str) -> Option<&'static MetricDefinition> {
    METRICS.iter().find(|m| m.name == name)
}
