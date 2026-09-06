//! Closed operational event and metric registries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventDefinition {
    pub name: &'static str,
    pub schema_version: u16,
    pub owner: &'static str,
}
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
pub fn event(name: &str) -> Option<&'static EventDefinition> {
    EVENTS.iter().find(|e| e.name == name)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    Counter,
    Histogram,
    Gauge,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricDefinition {
    pub name: &'static str,
    pub kind: MetricKind,
    pub unit: &'static str,
    pub labels: &'static [&'static str],
}
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
pub fn metric(name: &str) -> Option<&'static MetricDefinition> {
    METRICS.iter().find(|m| m.name == name)
}
