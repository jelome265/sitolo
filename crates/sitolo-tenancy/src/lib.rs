//! Tenancy.
//!
//! Owns tenant, organization, branch, and scope-context propagation (Phase 1
//! specification, §5.6; Phase 4 specification). Tenant isolation is a security boundary.
#![forbid(unsafe_code)]

pub mod branch;
pub mod entity;
pub mod error;
pub mod invitation;
pub mod membership;
pub mod organization;
pub mod ownership;

pub use branch::{Branch, BranchStatus};
pub use entity::{BusinessEntity, Register, Warehouse};
pub use error::TenancyError;
pub use invitation::{Invitation, InvitationStatus};
pub use membership::{Membership, MembershipStatus};
pub use organization::{Organization, OrganizationStatus};
pub use ownership::{OwnershipTransferRequest, OwnershipTransferStatus};
