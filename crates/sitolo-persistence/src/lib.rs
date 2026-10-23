//! Persistence.
//!
//! Owns the repository ports and in-memory reference implementation.
//! PostgreSQL implementations arrive in Phase 5; this crate provides the
//! semantic boundary that application code depends on (§51).
#![forbid(unsafe_code)]

mod memory;
mod ports;
pub mod postgres;
mod postgres_time;
mod runtime;
pub mod runtime_pool;
mod tenancy;
pub mod tenancy_postgres;

pub use identity_authority_postgres::PgRequestIdentityAuthority;
pub use memory::{
    DeviceRegistrationInput, DeviceRevocationEffect, EstablishedSession, IdentityDatabase,
    MfaEnrollmentResult, PasswordResetResult, RefreshRotation, SessionSnapshot, UserSnapshot,
};
pub use ports::IdentityStores;
pub use runtime::{
    DatabaseCapability, DatabaseConnector, DatabaseLifecycle, DatabasePoolMetrics, DatabaseRuntime,
    PersistenceFailureKind, PersistenceInitError,
};
pub use runtime_pool::{PgRuntimePool, PgWorkerPool, RuntimePoolError};
pub use sitolo_auth::RequestIdentityAuthority;
pub use sitolo_domain::tenancy::ProvisionedOrganization;
pub use tenancy::{
    AcceptedInvitation, CreateInvitationInput, CreatedInvitation, TenancyDatabase, TenancyStores,
};

pub mod audit_repository;
pub mod consumer_inbox;
pub mod identity_authority_postgres;
pub mod outbox_repository;
