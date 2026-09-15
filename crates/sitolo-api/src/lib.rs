//! API layer.
//!
//! Owns HTTP routes, request decoding, transport validation, authentication
//! extraction, response mapping, and HTTP error representation. It must not
//! become the business authority (Phase 1 specification, §5.3).
#![forbid(unsafe_code)]

mod auth;
mod error;
pub mod tenancy;
pub use auth::{
    bounds, establish_context, extract_bearer, map_auth_error, validate_login_input,
    validate_otp_input,
};
pub use error::{AppError, ErrorFamily, ProblemDetails, PublicError, Retryability};
pub use tenancy::{
    BranchResponse, CreateBranchRequest, CreateOrganizationRequest, OrganizationResponse,
    ProvisionedOrganizationResponse, bounds as tenancy_bounds, ensure_body_bounded,
    handle_activate_branch, handle_activate_organization, handle_begin_close_branch,
    handle_begin_close_organization, handle_close_branch, handle_close_organization,
    handle_create_branch, handle_provision_organization, handle_resume_branch,
    handle_resume_organization, handle_suspend_branch, handle_suspend_organization,
    map_tenancy_error, validate_create_branch_input, validate_create_organization_input,
    validate_identifier, validate_name,
};
