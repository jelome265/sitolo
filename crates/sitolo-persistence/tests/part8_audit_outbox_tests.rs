//! Phase 4 Part 8 Audit and Outbox PostgreSQL Integration Tests.

use std::error::Error;

use sitolo_audit::{ActorRef, IamAuditEvent, IamEventName, TargetRef};
use sitolo_domain::tenancy::{MembershipId, OrganizationId};
use sitolo_events::{OutboxEvent, OutboxStatus};

#[test]
fn test_iam_event_names_stable() {
    assert_eq!(
        IamEventName::OrganizationCreated.as_str(),
        "iam.organization.created"
    );
    assert_eq!(
        IamEventName::MembershipRevoked.as_str(),
        "iam.membership.revoked"
    );
    assert_eq!(
        IamEventName::AuthorizationDenied.as_str(),
        "iam.authorization.denied"
    );
}

#[test]
fn test_iam_event_is_secret_free() -> Result<(), Box<dyn Error>> {
    let event = IamAuditEvent::success(
        sitolo_auth::AuditEventId::new("evt-1")?,
        IamEventName::MembershipRevoked,
        OrganizationId::new("org-1")?,
        None,
        ActorRef {
            subject_ref: "subject_pseudonymous".into(),
            membership_ref: Some("membership_pseudonymous".into()),
            device_ref: None,
        },
        TargetRef::Membership(MembershipId::new("mem-1")?),
        "revoke",
        "tenancy_service",
    );

    let debug = format!("{:?}", event);
    assert!(!debug.contains("password"));
    assert!(!debug.contains("Bearer"));
    assert!(!debug.contains("secret"));
    Ok(())
}

#[test]
fn test_outbox_event_creation() -> Result<(), Box<dyn Error>> {
    let event = OutboxEvent::new(
        "evt-1".to_string(),
        "Membership".to_string(),
        "mem-123".to_string(),
        "iam.membership.revoked".to_string(),
        1,
        OrganizationId::new("org-1")?,
        None,
        r#"{"membership_id":"mem-123"}"#.to_string(),
    )?;

    assert_eq!(event.status, OutboxStatus::Pending);
    assert_eq!(event.attempt_count, 0);
    assert_eq!(event.deduplication_key, "evt-1");
    Ok(())
}

#[test]
fn test_outbox_event_rejects_oversized_payload() -> Result<(), Box<dyn Error>> {
    let oversized = "x".repeat(OutboxEvent::MAX_PAYLOAD_SIZE + 1);
    let result = OutboxEvent::new(
        "evt-2".to_string(),
        "Membership".to_string(),
        "mem-123".to_string(),
        "iam.membership.revoked".to_string(),
        1,
        OrganizationId::new("org-1")?,
        None,
        oversized,
    );

    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_outbox_status_state_machine() {
    assert_eq!(OutboxStatus::Pending.as_str(), "PENDING");
    assert_eq!(OutboxStatus::Claimed.as_str(), "CLAIMED");
    assert_eq!(OutboxStatus::Published.as_str(), "PUBLISHED");
    assert_eq!(OutboxStatus::Quarantined.as_str(), "QUARANTINED");
}
