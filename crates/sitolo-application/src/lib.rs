//! Application layer.
//!
//! Owns use cases, commands, queries, application orchestration, transaction
//! boundaries, and ports. Coordinates domain behavior; it is not an HTTP
//! framework (Phase 1 specification, §5.2).
//!
//! Phase 3: identity use cases that wire authentication, sessions, MFA,
//! device lifecycle, password management, and security-context construction
//! against application-owned identity ports.
//!
//! Phase 4: tenancy use cases that wire organization provisioning, branch
//! lifecycle, and membership lifecycle against application-owned tenancy
//! ports. The durable adapter implements [`TenancyUnitOfWork`]; the
//! application never imports a database driver or the persistence crate.
#![forbid(unsafe_code)]

mod identity;
mod identity_ports;
mod tenancy;
mod tenancy_ports;

pub use identity::{IdentityPolicy, IdentityService, LoginResult, RefreshResult};
pub use identity_ports::{
    DeviceRegistrationInput, DeviceRevocationEffect, EstablishedSession, IdentityStores,
    MfaEnrollmentResult, PasswordResetResult, RefreshRotation, SessionSnapshot, UserSnapshot,
};
pub use tenancy::{AuthorizedTenancyError, TenancyService};
pub use tenancy_ports::{
    AcceptedInvitation, CreateInvitationInput, CreatedInvitation, ExpiredInvitation,
    InvitationRecord, MembershipRecord, MembershipSelector, ScopeMaterial, TenancyStores,
    TenancyTransaction, TenancyUnitOfWork,
};
