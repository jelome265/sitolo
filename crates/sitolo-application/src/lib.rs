//! Application layer.
//!
//! Owns use cases, commands, queries, application orchestration, transaction
//! boundaries, and ports. Coordinates domain behavior; it is not an HTTP
//! framework (Phase 1 specification, §5.2).
#![forbid(unsafe_code)]