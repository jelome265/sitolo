//! Audit.
//!
//! Owns durable audit semantics. Logs are not a substitute for audit records
//! (Phase 1 specification, §5.9). Phase 3 specification, §37: authentication
//! is security-sensitive and must emit structured evidence.
//!
//! This crate owns the audit event catalogue, the record shape, and the
//! recorder port. In-memory and PostgreSQL implementations live in
//! `sitolo-persistence`.
//!
//! Phase 4 Part 8 extends the boundary with IAM/security events covering
//! organization, branch, membership, role, scope, and security violations.
//! The Phase 3 authentication audit remains intact and compatible.
#![forbid(unsafe_code)]

mod event;
mod iam_event;

pub use event::{
    AuditError, AuditRecorder, AuditRequirement, AuthEventName, AuthenticationEvent, EventResult,
    InMemoryAuditSink,
};
pub use iam_event::{
    ActorRef, EventVersion, IamAuditEvent, IamEventName, IamEventResult, ReasonClass, TargetRef,
};
