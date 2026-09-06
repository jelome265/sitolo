//! Observability.
//!
//! Owns structured tracing, metrics, correlation identifiers, and telemetry
//! integration (Phase 1 specification, §5.10). Observability data is itself
//! sensitive.
#![forbid(unsafe_code)]

mod buffer;
mod context;
mod registry;
pub use buffer::{Priority, TelemetryBuffer, TelemetryState};
pub use context::{RequestId, TraceParent};
pub use registry::{EVENTS, EventDefinition, METRICS, MetricDefinition, MetricKind, event, metric};

/// Emits only registered, field-selected events; arbitrary request/object dumps
/// are deliberately absent from this boundary.
pub fn emit_registered(name: &str, request_id: &RequestId, operation: &str) -> bool {
    if event(name).is_none() {
        return false;
    }
    tracing::info!(
        event_name = name,
        event_schema_version = 1_u16,
        request_id = request_id.as_str(),
        operation,
        "sitolo operational event"
    );
    true
}
