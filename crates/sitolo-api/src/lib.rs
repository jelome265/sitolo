//! API layer.
//!
//! Owns HTTP routes, request decoding, transport validation, authentication
//! extraction, response mapping, and HTTP error representation. It must not
//! become the business authority (Phase 1 specification, §5.3; Phase 4 specification).
#![forbid(unsafe_code)]

pub mod dto;
pub mod error;

pub use dto::*;
pub use error::{AppError, ErrorFamily, ProblemDetails, PublicError, Retryability};
