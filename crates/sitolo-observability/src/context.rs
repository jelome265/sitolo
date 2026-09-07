//! Bounded, untrusted diagnostic correlation context.
//!
//! Server-generated identifiers are globally unique 128-bit random values
//! (UUID v4), so two application instances can never collide and no request
//! ordering or volume information leaks. Client-supplied identifiers remain
//! untrusted diagnostic input validated against a bounded grammar.

/// Maximum client-supplied identifier length.
pub const MAX_ID: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(String);

impl RequestId {
    /// Generates a globally unique server correlation ID (`req_` + 32
    /// lowercase hex digits). Never sequential, never instance-local.
    pub fn new_server() -> Self {
        Self(format!("req_{}", uuid::Uuid::new_v4().simple()))
    }

    pub fn parse_client(value: &str) -> Option<Self> {
        valid(value).then(|| Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceParent(String);

impl TraceParent {
    /// Parses W3C trace-context `traceparent` with semantic validation, not
    /// just shape validation (F-006). Only version `00` is accepted: future
    /// versions require deliberate protocol support, and `ff` is reserved.
    /// All-zero trace and parent identifiers are rejected. Hex case is
    /// accepted leniently; correlation data is diagnostic only and never
    /// authorization evidence.
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<_> = value.split('-').collect();
        if parts.len() != 4 {
            return None;
        }
        let [version, trace_id, parent_id, flags] = <[&str; 4]>::try_from(parts).ok()?;
        if version.len() != 2 || trace_id.len() != 32 || parent_id.len() != 16 || flags.len() != 2 {
            return None;
        }
        if ![version, trace_id, parent_id, flags]
            .iter()
            .all(|s| s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return None;
        }
        if version != "00" {
            return None;
        }
        if trace_id.bytes().all(|b| b == b'0') || parent_id.bytes().all(|b| b == b'0') {
            return None;
        }
        Some(Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn rejects_injected_id() {
        assert!(RequestId::parse_client("x\\nsecret").is_none());
        assert!(RequestId::parse_client("").is_none());
        assert!(RequestId::parse_client(&"a".repeat(MAX_ID + 1)).is_none());
    }

    #[test]
    fn server_ids_are_unique_and_bounded() {
        let ids: BTreeSet<_> = (0..256).map(|_| RequestId::new_server()).collect();
        assert_eq!(ids.len(), 256, "server IDs must not collide");
        for id in &ids {
            let text = id.as_str();
            assert!(text.starts_with("req_"));
            assert_eq!(text.len(), 36);
            assert!(
                text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
                "server ID must stay log/header-safe"
            );
        }
    }

    #[test]
    fn traceparent_matrix() {
        let valid = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        assert!(TraceParent::parse(valid).is_some());
        // Wrong field count / lengths / non-hex.
        assert!(TraceParent::parse("not-a-trace").is_none());
        assert!(
            TraceParent::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b-01").is_none()
        );
        assert!(
            TraceParent::parse("00-4bf92f3577b34da6a3ce929d0e0e47zz-00f067aa0ba902b7-01").is_none()
        );
        // All-zero identifiers are protocol-invalid.
        assert!(
            TraceParent::parse("00-00000000000000000000000000000000-00f067aa0ba902b7-01").is_none()
        );
        assert!(
            TraceParent::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01").is_none()
        );
        // Reserved and unknown versions are rejected (strict boundary).
        assert!(
            TraceParent::parse("ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01").is_none()
        );
        assert!(
            TraceParent::parse("01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01").is_none()
        );
        // Uppercase hex is accepted leniently for correlation purposes.
        assert!(
            TraceParent::parse("00-4BF92F3577B34DA6A3CE929D0E0E4736-00F067AA0BA902B7-01").is_some()
        );
        // Flags variants with valid shape pass through.
        assert!(
            TraceParent::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00").is_some()
        );
    }
}
