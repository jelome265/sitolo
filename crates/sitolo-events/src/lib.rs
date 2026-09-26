//! Events.
//!
//! Owns internal event contracts. Domain events, integration events, outbox
//! records, and provider events remain distinct concepts (Phase 1
//! specification, §5.8).
//!
//! Phase 4 Part 8 adds the transactional outbox boundary. Outbox records
//! store durable event intent in the same database transaction as the
//! business mutation. A separate relay/worker eventually publishes these
//! events to external systems.
#![forbid(unsafe_code)]

mod outbox;

pub use outbox::{OutboxError, OutboxEvent, OutboxReader, OutboxStatus, OutboxWriter, RetryClass};
