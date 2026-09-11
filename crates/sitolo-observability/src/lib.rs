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
pub use context::{MAX_ID, RequestId, TraceParent};
pub use registry::{EVENTS, EventDefinition, METRICS, MetricDefinition, MetricKind, event, metric};

/// Maximum operation-name length (F-008: bounded telemetry dimensions).
pub const MAX_OPERATION: usize = 64;

/// A bounded, grammar-checked operation name for telemetry fields (F-008).
///
/// Raw request/body content must never become an operation label: only
/// lowercase alphanumeric segments joined by `.`, `_`, or `-` are accepted,
/// so attacker-controlled strings are rejected at this boundary instead of
/// flowing into logs and metric labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationName(String);

impl OperationName {
    pub fn parse(value: &str) -> Option<Self> {
        if value.is_empty()
            || value.len() > MAX_OPERATION
            || !value.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            })
        {
            return None;
        }
        Some(OperationName(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Emits only registered, field-selected events; arbitrary request/object dumps
/// are deliberately absent from this boundary. The emitted schema version
/// always comes from the registry definition (F-009), never a literal.
pub fn emit_registered(name: &str, request_id: &RequestId, operation: &OperationName) -> bool {
    let Some(definition) = event(name) else {
        return false;
    };
    tracing::info!(
        event_name = definition.name,
        event_schema_version = definition.schema_version,
        request_id = request_id.as_str(),
        operation = operation.as_str(),
        "sitolo operational event"
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_name_rejects_hostile_input() {
        assert!(OperationName::parse("auth.login").is_some());
        assert!(OperationName::parse("db.acquire").is_some());
        // Attacker-sized values are rejected.
        assert!(OperationName::parse(&"a".repeat(MAX_OPERATION + 1)).is_none());
        assert!(OperationName::parse("").is_none());
        // Control characters, secrets, and JSON are rejected.
        assert!(OperationName::parse("auth\nlogin").is_none());
        assert!(OperationName::parse("password=hunter2").is_none());
        assert!(OperationName::parse("{\"op\":\"x\"}").is_none());
        assert!(OperationName::parse("Auth.Login").is_none());
    }

    #[test]
    fn emission_uses_registry_schema_version() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct Sink(Arc<Mutex<Vec<u8>>>);
        impl Write for Sink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for Sink {
            type Writer = Sink;
            fn make_writer(&self) -> Sink {
                self.clone()
            }
        }

        let sink = Sink::default();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_writer(sink.clone())
            .finish();
        let request_id = RequestId::parse_client("client-1").expect("client id");
        let operation = OperationName::parse("auth.login").expect("operation");
        tracing::subscriber::with_default(subscriber, || {
            assert!(emit_registered("service.starting", &request_id, &operation));
            assert!(!emit_registered("no.such.event", &request_id, &operation));
        });
        let output = String::from_utf8(
            sink.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
        )
        .expect("captured JSON is UTF-8");
        // Schema version comes from the registry definition, and the
        // bounded operation is the only operation text emitted.
        let expected_version = event("service.starting")
            .expect("registered")
            .schema_version;
        assert!(output.contains(&format!("\"event_schema_version\":{expected_version}")));
        assert!(output.contains("\"operation\":\"auth.login\""));
        assert!(!output.contains("no.such.event"));
    }
}
