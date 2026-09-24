//! Phase 4 IAM and Security audit events and record shape.
//!
//! Phase 4 Part 8 (§6). The IAM and security audit event set is enumerated
//! here; records are constructed from secret-free fields by design.

use std::time::SystemTime;

use sitolo_auth::{Assurance, AuditEventId};

use crate::event::EventResult;

/// Phase 4 IAM / Security audit event names (§6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IamAuditEventName {
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

impl IamAuditEventName {
    /// Stable dotted event name for the audit record.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            IamAuditEventName::OrganizationCreated => "iam.organization.created",
            IamAuditEventName::OrganizationActivated => "iam.organization.activated",
            IamAuditEventName::OrganizationSuspended => "iam.organization.suspended",
            IamAuditEventName::OrganizationResumed => "iam.organization.resumed",
            IamAuditEventName::OrganizationClosingStarted => "iam.organization.closing_started",
            IamAuditEventName::OrganizationClosed => "iam.organization.closed",

            IamAuditEventName::BranchCreated => "iam.branch.created",
            IamAuditEventName::BranchActivated => "iam.branch.activated",
            IamAuditEventName::BranchSuspended => "iam.branch.suspended",
            IamAuditEventName::BranchResumed => "iam.branch.resumed",
            IamAuditEventName::BranchClosingStarted => "iam.branch.closing_started",
            IamAuditEventName::BranchClosed => "iam.branch.closed",

            IamAuditEventName::MembershipInvited => "iam.membership.invited",
            IamAuditEventName::MembershipInvitationAccepted => "iam.membership.invitation_accepted",
            IamAuditEventName::MembershipActivated => "iam.membership.activated",
            IamAuditEventName::MembershipSuspended => "iam.membership.suspended",
            IamAuditEventName::MembershipRevoked => "iam.membership.revoked",

            IamAuditEventName::RoleAssigned => "iam.role.assigned",
            IamAuditEventName::RoleChanged => "iam.role.changed",
            IamAuditEventName::RoleRemoved => "iam.role.removed",

            IamAuditEventName::ScopeGranted => "iam.scope.granted",
            IamAuditEventName::ScopeChanged => "iam.scope.changed",
            IamAuditEventName::ScopeRevoked => "iam.scope.revoked",

            IamAuditEventName::AuthorizationDenied => "iam.authorization.denied",
            IamAuditEventName::SecurityScopeViolation => "iam.security.scope_violation",

            IamAuditEventName::DeviceBound => "iam.device.bound",
            IamAuditEventName::DeviceUnbound => "iam.device.unbound",
            IamAuditEventName::DeviceSuspended => "iam.device.suspended",
            IamAuditEventName::DeviceRevoked => "iam.device.revoked",
        }
    }
}

/// IAM / Security audit record (§6.0).
///
/// Constructed strictly from secret-free pseudonymous references, scope IDs,
/// target references, and outcome classifications.
#[derive(Debug, Clone)]
pub struct IamAuditEvent {
    pub event_id: AuditEventId,
    pub event_name: IamAuditEventName,
    pub event_version: u32,
    pub occurred_at: SystemTime,
    pub organization_id: Option<String>,
    pub branch_id: Option<String>,
    pub actor_subject_ref: Option<String>,
    pub actor_membership_ref: Option<String>,
    pub actor_device_ref: Option<String>,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub target_type: Option<String>,
    pub target_ref: Option<String>,
    pub action: String,
    pub result: EventResult,
    pub reason_class: Option<String>,
    pub assurance_level: Option<Assurance>,
    pub source: String,
    pub metadata: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iam_event_names_are_stable_and_dotted() {
        assert_eq!(
            IamAuditEventName::OrganizationCreated.as_str(),
            "iam.organization.created"
        );
        assert_eq!(
            IamAuditEventName::MembershipInvitationAccepted.as_str(),
            "iam.membership.invitation_accepted"
        );
        assert_eq!(
            IamAuditEventName::AuthorizationDenied.as_str(),
            "iam.authorization.denied"
        );
        assert_eq!(
            IamAuditEventName::SecurityScopeViolation.as_str(),
            "iam.security.scope_violation"
        );
    }

    #[test]
    fn record_shape_has_no_secret_fields() {
        let event = IamAuditEvent {
            event_id: AuditEventId::new("evt-iam-1").unwrap(),
            event_name: IamAuditEventName::RoleChanged,
            event_version: 1,
            occurred_at: SystemTime::UNIX_EPOCH,
            organization_id: Some("org-1".into()),
            branch_id: Some("br-1".into()),
            actor_subject_ref: Some("subj-1".into()),
            actor_membership_ref: Some("mem-1".into()),
            actor_device_ref: None,
            request_id: Some("req-1".into()),
            trace_id: None,
            target_type: Some("membership".into()),
            target_ref: Some("mem-2".into()),
            action: "iam.role.changed".into(),
            result: EventResult::Success,
            reason_class: None,
            assurance_level: Some(Assurance::A2),
            source: "api".into(),
            metadata: None,
        };
        let debug = format!("{event:?}");
        assert!(!debug.contains("password"));
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("token"));
    }
}
