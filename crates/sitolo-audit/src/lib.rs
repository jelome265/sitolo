//! Audit.
//!
//! Owns durable audit semantics. Logs are not a substitute for audit records
//! (Phase 1 specification, §5.9).
#![forbid(unsafe_code)]