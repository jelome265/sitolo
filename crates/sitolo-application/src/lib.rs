//! Application layer.
//!
//! Owns use cases, commands, queries, application orchestration, transaction
//! boundaries, and ports. Coordinates domain behavior; it is not an HTTP
//! framework (Phase 1 specification, §5.2).
//!
//! Phase 3: identity use cases that wire authentication, sessions, MFA,
//! device lifecycle, password management, and security-context construction
//! against the persistence ports.
#![forbid(unsafe_code)]

mod identity;

pub use identity::IdentityService;
