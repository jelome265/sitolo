//! Application layer.
//!
//! Owns use cases, commands, queries, application orchestration, transaction
//! boundaries, and ports. Coordinates domain behavior; it is not an HTTP
//! framework (Phase 1 specification, §5.2).
//!
//! Phase 3: identity use cases that wire authentication, sessions, MFA,
//! device lifecycle, password management, and security-context construction
//! against the persistence ports.
//!
//! Phase 4: tenancy use cases that wire organization provisioning, branch
//! lifecycle, and membership lifecycle against the tenancy persistence port.
#![forbid(unsafe_code)]

mod identity;
mod tenancy;

pub use identity::IdentityService;
pub use tenancy::TenancyService;
