//! API layer.
//!
//! Owns HTTP routes, request decoding, transport validation, authentication
//! extraction, response mapping, and HTTP error representation. It must not
//! become the business authority (Phase 1 specification, §5.3).
#![forbid(unsafe_code)]

mod auth;
mod error;
pub use auth::{
    bounds, establish_context, extract_bearer, map_auth_error, validate_login_input,
    validate_otp_input,
};
pub use error::{AppError, ErrorFamily, ProblemDetails, PublicError, Retryability};
