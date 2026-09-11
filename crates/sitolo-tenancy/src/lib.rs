//! Tenancy.
//!
//! Owns tenant, organization, branch, and scope-context propagation (Phase 1
//! specification, §5.6). Tenant isolation is a security boundary.
//!
//! Phase 4 owns the organizational authority topology: domain entities and
//! state machines live in [`sitolo_domain::tenancy`]; this crate resolves the
//! server-authoritative [`EffectiveScope`] from trusted records. Requested
//! (client-supplied) and trusted (server-resolved) identifiers are distinct
//! types so authority confusion cannot compile by accident.
#![forbid(unsafe_code)]

mod scope;

pub use scope::{
    EffectiveScope, RequestedBranchId, RequestedOrganizationId, ScopeError, TrustedOrganizationId,
    bind_organization, resolve_effective_scope,
};
