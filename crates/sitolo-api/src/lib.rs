//! API layer.
//!
//! Owns HTTP routes, request decoding, transport validation, authentication
//! extraction, response mapping, and HTTP error representation. It must not
//! become the business authority (Phase 1 specification, §5.3).
#![forbid(unsafe_code)]