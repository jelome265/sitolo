//! Domain layer.
//!
//! Owns entities, value objects, aggregates, domain events, state machines,
//! invariants, domain errors, and business policies.
//!
//! This crate is infrastructure-independent and MUST NOT depend on Axum,
//! SQLx, Redis, HTTP clients, filesystem/network implementations, or provider
//! SDKs (Phase 1 specification, §5.1, §6).
#![forbid(unsafe_code)]
