//! Persistence.
//!
//! Owns the SQLx/PostgreSQL implementation, repositories, transactions, and
//! mapping (Phase 1 specification, §5.7). PostgreSQL is the authoritative
//! server-side business state.
#![forbid(unsafe_code)]
