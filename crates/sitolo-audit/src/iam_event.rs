//! Phase 4 IAM/security audit events.
//!
//! Extends the authentication audit boundary with organizational, membership,
//! role, scope, and security violation events. Records are secret-free by
//! construction and carry tenant scope from trusted server-side authorization.

use std::time::SystemTime;

use sitolo_auth::AuditEventId;
use sitolo_domain::tenancy::{BranchId, InvitationId, MembershipId, OrganizationId};

/// Phase 4 IAM/security event names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IamEventName {
    /// Organization created in the provisioning state.
    OrganizationCreated,
    /// Organization activated.
    OrganizationActivated,
    /// Organization suspended.
    OrganizationSuspended,
    /// Organization resumed.
    OrganizationResumed,
    /// Organization began closing.
    OrganizationClosingStarted,
    /// Organization closed.
    OrganizationClosed,
    /// Branch created.
    BranchCreated,
    /// Branch activated.
    BranchActivated,
    /// Branch suspended.
    BranchSuspended,
    /// Branch resumed.
    BranchResumed,
    /// Branch began closing.
    BranchClosingStarted,
    /// Branch closed.
    BranchClosed,
    /// Membership invited.
    MembershipInvited,
    /// Invited membership accepted.
    MembershipInvitationAccepted,
    /// Membership activated.
    MembershipActivated,
    /// Membership suspended.
    MembershipSuspended,
    /// Membership revoked.
    MembershipRevoked,
    /// Membership advanced to pending acceptance.
    MembershipPendingAcceptance,
    /// Membership expired.
    MembershipExpired,
    /// Invitation issued.
    InvitationIssued,
    /// Invitation withdrawn.
    InvitationWithdrawn,
    /// Invitation expired.
    InvitationExpired,
    /// Role assigned.
    RoleAssigned,
    /// Role changed.
    RoleChanged,
    /// Role removed.
    RoleRemoved,
    /// Scope grant created.
    ScopeGranted,
    /// Scope grant changed.
    ScopeChanged,
    /// Scope grant revoked.
    ScopeRevoked,
    /// Authorization denied.
    AuthorizationDenied,
    /// Security scope violation detected.
    SecurityScopeViolation,
    /// Device bound to a subject.
    DeviceBound,
    /// Device unbound from a subject.
    DeviceUnbound,
    /// Device suspended.
    DeviceSuspended,
    /// Device revoked.
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
        Self::MembershipPendingAcceptance,
        Self::MembershipExpired,
        Self::InvitationIssued,
        Self::InvitationWithdrawn,
        Self::InvitationExpired,
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

    /// Stable dotted event name for the audit record and outbox envelope.
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
            IamEventName::MembershipPendingAcceptance => "iam.membership.pending_acceptance",
            IamEventName::MembershipExpired => "iam.membership.expired",
            IamEventName::InvitationIssued => "iam.invitation.issued",
            IamEventName::InvitationWithdrawn => "iam.invitation.withdrawn",
            IamEventName::InvitationExpired => "iam.invitation.expired",
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

    #[test]
    fn owner_approved_lifecycle_event_names_are_registered() {
        let names: Vec<_> = IamEventName::ALL
            .iter()
            .map(|event| event.as_str())
            .collect();

        for approved_name in [
            "iam.membership.pending_acceptance",
            "iam.membership.expired",
            "iam.invitation.issued",
            "iam.invitation.withdrawn",
            "iam.invitation.expired",
        ] {
            assert!(names.contains(&approved_name), "missing {approved_name}");
        }
    }
}

/// Version of the IAM audit record envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventVersion(
    /// Monotonic envelope version.
    pub u32,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Outcome classification of an IAM audit event.
pub enum IamEventResult {
    /// The audited operation succeeded.
    Success,
    /// The audited operation failed.
    Failure,
}

/// Bounded failure reason carried by a failed IAM audit event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasonClass {
    /// The caller lacks the required scope.
    InsufficientScope,
    /// The requested state transition is not legal.
    InvalidTransition,
    /// The addressed record does not exist.
    NotFound,
    /// The write conflicts with existing semantics.
    Conflict,
    /// The record is already in a terminal state.
    TerminalState,
    /// The operation was rate limited.
    RateLimited,
    /// The supplied parameters are invalid.
    InvalidParameters,
}

impl ReasonClass {
    /// Stable snake-case label used in audit payloads.
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
/// Pseudonymous actor provenance for an IAM audit event.
pub struct ActorRef {
    /// Pseudonymous subject reference.
    pub subject_ref: String,
    /// Membership reference active at the time of the action.
    pub membership_ref: Option<String>,
    /// Device reference bound to the action.
    pub device_ref: Option<String>,
}

/// Record an IAM audit event applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetRef {
    /// Organization target.
    Organization(OrganizationId),
    /// Branch target.
    Branch(BranchId),
    /// Invitation target.
    Invitation(InvitationId),
    /// Membership target.
    Membership(MembershipId),
    /// Role target identified by name.
    Role(String),
    /// Scope target identified by serialized scope.
    Scope(String),
    /// Device target identified by pseudonymous reference.
    Device(String),
}

/// Secret-free IAM/security audit record.
#[derive(Debug, Clone)]
pub struct IamAuditEvent {
    /// Globally unique audit event identifier.
    pub event_id: AuditEventId,
    /// Stable IAM event name from the catalogue.
    pub event_name: IamEventName,
    /// Audit envelope version.
    pub event_version: EventVersion,
    /// Server time at which the event occurred.
    pub occurred_at: SystemTime,
    /// Trusted tenant scope of the event.
    pub organization_id: OrganizationId,
    /// Optional branch scope of the event.
    pub branch_id: Option<BranchId>,
    /// Pseudonymous actor provenance.
    pub actor: ActorRef,
    /// Correlation identifier for the originating request.
    pub request_id: Option<String>,
    /// Distributed trace identifier, when one was propagated.
    pub trace_id: Option<String>,
    /// Record the event applies to.
    pub target: TargetRef,
    /// Stable action label for the use case.
    pub action: &'static str,
    /// Outcome classification.
    pub result: IamEventResult,
    /// Bounded failure reason, when the result is a failure.
    pub reason_class: Option<ReasonClass>,
    /// Assurance level in force at the time of the action.
    pub assurance_level: Option<String>,
    /// Component that emitted the event.
    pub source: &'static str,
}

impl IamAuditEvent {
    /// Builds a successful IAM audit event.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "the audit envelope keeps tenant, actor, target, and provenance explicit"
    )]
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

    /// Builds a failed IAM audit event with a bounded reason class.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "the audit envelope keeps tenant, actor, target, and provenance explicit"
    )]
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
