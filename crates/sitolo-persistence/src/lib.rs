//! Persistence.
//!
//! Owns the repository ports and in-memory reference implementation.
//! PostgreSQL implementations arrive in Phase 5; this crate provides the
//! semantic boundary that application code depends on (§51).
#![forbid(unsafe_code)]

mod memory;
mod ports;
mod runtime;
mod tenancy;

pub use memory::{
    DeviceRegistrationInput, DeviceRevocationEffect, EstablishedSession, IdentityDatabase,
    MfaEnrollmentResult, PasswordResetResult, RefreshRotation, SessionSnapshot, UserSnapshot,
};
pub use ports::IdentityStores;
pub use runtime::{
    DatabaseCapability, DatabaseConnector, DatabaseLifecycle, DatabasePoolMetrics, DatabaseRuntime,
    PersistenceFailureKind, PersistenceInitError,
};
pub use tenancy::{ProvisionedOrganization, TenancyDatabase, TenancyStores};
