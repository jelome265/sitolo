//! Phase 4 IAM/security audit events.
//!
//! Extends the authentication audit boundary with organizational, membership,
//! role, scope, and security violation events. Records are secret-free by
//! construction and carry tenant scope from trusted server-side authorization.

use std::time::SystemTime;

use sitolo_auth::AuditEventId;
use sitolo_domain::tenancy::{BranchId, MembershipId, OrganizationId};

/// Phase 4 IAM/security event names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IamEventName {
    OrganizationCreated,
    OrganizationActivated,
    OrganizationSuspended,
    OrganizationResumed,
    OrganizationClosingStarted,
    OrganizationClosed,
    BranchCreated,
    BranchActivated,
    BranchSuspended,
    BranchResumed,
    BranchClosingStarted,
    BranchClosed,
    MembershipInvited,
    MembershipInvitationAccepted,
    MembershipActivated,
    MembershipSuspended,
    MembershipRevoked,
    RoleAssigned,
    RoleChanged,
    RoleRemoved,
    ScopeGranted,
    ScopeChanged,
    ScopeRevoked,
    AuthorizationDenied,
    SecurityScopeViolation,
    DeviceBound,
    DeviceUnbound,
    DeviceSuspended,
    DeviceRevoked,
}

impl IamEventName {
    /// Complete set of registered IAM event names. Outbox consumers use this
    /// catalogue to fail closed on an event name that this release does not
    /// understand.
    pub const ALL: &'static [Self] = &[
        Self::OrganizationCreated,
        Self::OrganizationActivated,
        Self::OrganizationSuspended,
        Self::OrganizationResumed,
        Self::OrganizationClosingStarted,
        Self::OrganizationClosed,
        Self::BranchCreated,
        Self::BranchActivated,
        Self::BranchSuspended,
        Self::BranchResumed,
        Self::BranchClosingStarted,
        Self::BranchClosed,
        Self::MembershipInvited,
        Self::MembershipInvitationAccepted,
        Self::MembershipActivated,
        Self::MembershipSuspended,
        Self::MembershipRevoked,
        Self::RoleAssigned,
        Self::RoleChanged,
        Self::RoleRemoved,
        Self::ScopeGranted,
        Self::ScopeChanged,
        Self::ScopeRevoked,
        Self::AuthorizationDenied,
        Self::SecurityScopeViolation,
        Self::DeviceBound,
        Self::DeviceUnbound,
        Self::DeviceSuspended,
        Self::DeviceRevoked,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            IamEventName::OrganizationCreated => "iam.organization.created",
            IamEventName::OrganizationActivated => "iam.organization.activated",
            IamEventName::OrganizationSuspended => "iam.organization.suspended",
            IamEventName::OrganizationResumed => "iam.organization.resumed",
            IamEventName::OrganizationClosingStarted => "iam.organization.closing_started",
            IamEventName::OrganizationClosed => "iam.organization.closed",
            IamEventName::BranchCreated => "iam.branch.created",
            IamEventName::BranchActivated => "iam.branch.activated",
            IamEventName::BranchSuspended => "iam.branch.suspended",
            IamEventName::BranchResumed => "iam.branch.resumed",
            IamEventName::BranchClosingStarted => "iam.branch.closing_started",
            IamEventName::BranchClosed => "iam.branch.closed",
            IamEventName::MembershipInvited => "iam.membership.invited",
            IamEventName::MembershipInvitationAccepted => "iam.membership.invitation_accepted",
            IamEventName::MembershipActivated => "iam.membership.activated",
            IamEventName::MembershipSuspended => "iam.membership.suspended",
            IamEventName::MembershipRevoked => "iam.membership.revoked",
            IamEventName::RoleAssigned => "iam.role.assigned",
            IamEventName::RoleChanged => "iam.role.changed",
            IamEventName::RoleRemoved => "iam.role.removed",
            IamEventName::ScopeGranted => "iam.scope.granted",
            IamEventName::ScopeChanged => "iam.scope.changed",
            IamEventName::ScopeRevoked => "iam.scope.revoked",
            IamEventName::AuthorizationDenied => "iam.authorization.denied",
            IamEventName::SecurityScopeViolation => "iam.security.scope_violation",
            IamEventName::DeviceBound => "iam.device.bound",
            IamEventName::DeviceUnbound => "iam.device.unbound",
            IamEventName::DeviceSuspended => "iam.device.suspended",
            IamEventName::DeviceRevoked => "iam.device.revoked",
        }
    }
}

#[cfg(test)]
mod registry_tests {
    use super::IamEventName;

    #[test]
    fn registered_iam_event_names_are_unique_and_dotted() {
        let names: Vec<_> = IamEventName::ALL
            .iter()
            .map(|event| event.as_str())
            .collect();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();

        assert_eq!(names.len(), unique.len());
        assert!(names.iter().all(|name| name.starts_with("iam.")));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventVersion(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IamEventResult {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasonClass {
    InsufficientScope,
    InvalidTransition,
    NotFound,
    Conflict,
    TerminalState,
    RateLimited,
    InvalidParameters,
}

impl ReasonClass {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ReasonClass::InsufficientScope => "insufficient_scope",
            ReasonClass::InvalidTransition => "invalid_transition",
            ReasonClass::NotFound => "not_found",
            ReasonClass::Conflict => "conflict",
            ReasonClass::TerminalState => "terminal_state",
            ReasonClass::RateLimited => "rate_limited",
            ReasonClass::InvalidParameters => "invalid_parameters",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorRef {
    pub subject_ref: String,
    pub membership_ref: Option<String>,
    pub device_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetRef {
    Organization(OrganizationId),
    Branch(BranchId),
    Membership(MembershipId),
    Role(String),
    Scope(String),
    Device(String),
}

#[derive(Debug, Clone)]
pub struct IamAuditEvent {
    pub event_id: AuditEventId,
    pub event_name: IamEventName,
    pub event_version: EventVersion,
    pub occurred_at: SystemTime,
    pub organization_id: OrganizationId,
    pub branch_id: Option<BranchId>,
    pub actor: ActorRef,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub target: TargetRef,
    pub action: &'static str,
    pub result: IamEventResult,
    pub reason_class: Option<ReasonClass>,
    pub assurance_level: Option<String>,
    pub source: &'static str,
}

impl IamAuditEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn success(
        event_id: AuditEventId,
        event_name: IamEventName,
        organization_id: OrganizationId,
        branch_id: Option<BranchId>,
        actor: ActorRef,
        target: TargetRef,
        action: &'static str,
        source: &'static str,
    ) -> Self {
        IamAuditEvent {
            event_id,
            event_name,
            event_version: EventVersion(1),
            occurred_at: SystemTime::now(),
            organization_id,
            branch_id,
            actor,
            request_id: None,
            trace_id: None,
            target,
            action,
            result: IamEventResult::Success,
            reason_class: None,
            assurance_level: None,
            source,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn failure(
        event_id: AuditEventId,
        event_name: IamEventName,
        organization_id: OrganizationId,
        branch_id: Option<BranchId>,
        actor: ActorRef,
        target: TargetRef,
        action: &'static str,
        reason: ReasonClass,
        source: &'static str,
    ) -> Self {
        IamAuditEvent {
            event_id,
            event_name,
            event_version: EventVersion(1),
            occurred_at: SystemTime::now(),
            organization_id,
            branch_id,
            actor,
            request_id: None,
            trace_id: None,
            target,
            action,
            result: IamEventResult::Failure,
            reason_class: Some(reason),
            assurance_level: None,
            source,
        }
    }
}
