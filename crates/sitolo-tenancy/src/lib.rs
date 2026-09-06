//! Tenancy.
//!
//! Owns tenant, organization, branch, and scope-context propagation (Phase 1
//! specification, §5.6). Tenant isolation is a security boundary.
#![forbid(unsafe_code)]