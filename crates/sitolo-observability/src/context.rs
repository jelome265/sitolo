//! Bounded, untrusted diagnostic correlation context.
use std::sync::atomic::{AtomicU64, Ordering};
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const MAX_ID: usize = 128;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(String);
impl RequestId {
    pub fn new_server() -> Self {
        Self(format!(
            "req-{:016x}",
            REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
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
    pub fn parse(value: &str) -> Option<Self> {
        let p: Vec<_> = value.split('-').collect();
        (p.len() == 4
            && p[0].len() == 2
            && p[1].len() == 32
            && p[2].len() == 16
            && p[3].len() == 2
            && p.iter().all(|s| s.bytes().all(|b| b.is_ascii_hexdigit())))
        .then(|| Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_injected_id() {
        assert!(RequestId::parse_client("x\\nsecret").is_none());
    }
    #[test]
    fn accepts_only_w3c_shape() {
        assert!(
            TraceParent::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01").is_some()
        );
        assert!(TraceParent::parse("not-a-trace").is_none());
    }
}
