//! Events.
//!
//! Owns internal event contracts. Domain events, integration events, outbox
//! records, and provider events remain distinct concepts (Phase 1
//! specification, §5.8). Phase 4 Part 8 owns outbox record model and worker state.
#![forbid(unsafe_code)]

mod outbox;
mod relay;

pub use outbox::{OutboxEvent, OutboxEventId, OutboxStatus, RetryClassification};
pub use relay::{
    DispatchError, EventDispatcher, InMemoryDispatcher, OutboxRelayMetrics, RelayConfig,
};
