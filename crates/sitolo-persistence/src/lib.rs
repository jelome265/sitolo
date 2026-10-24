//! Persistence.
//!
//! Implements the application-owned repository ports with PostgreSQL
//! adapters and in-memory reference stores. The dependency edge points from
//! persistence to application, never the reverse (Phase 1 section 5.2;
//! Phase 4 sections 21 and 27).
#![forbid(unsafe_code)]

mod memory;
pub mod postgres;
mod postgres_time;
mod runtime;
pub mod runtime_pool;
mod tenancy;
pub mod tenancy_postgres;
mod tenancy_unit_of_work;

pub use identity_authority_postgres::PgRequestIdentityAuthority;
pub use memory::IdentityDatabase;
pub use runtime::{
    DatabaseCapability, DatabaseConnector, DatabaseLifecycle, DatabasePoolMetrics, DatabaseRuntime,
    PersistenceFailureKind, PersistenceInitError,
};
pub use runtime_pool::{PgRuntimePool, PgWorkerPool, RuntimePoolError};
pub use sitolo_application::{
    DeviceRegistrationInput, DeviceRevocationEffect, EstablishedSession, IdentityStores,
    MfaEnrollmentResult, PasswordResetResult, RefreshRotation, SessionSnapshot, TenancyStores,
    UserSnapshot,
};
pub use sitolo_auth::RequestIdentityAuthority;
pub use sitolo_domain::tenancy::ProvisionedOrganization;
pub use tenancy::TenancyDatabase;
pub use tenancy_unit_of_work::{PgTenancyTransaction, PgTenancyUnitOfWork};

pub mod audit_repository;
pub mod consumer_inbox;
pub mod identity_authority_postgres;
pub mod outbox_repository;
