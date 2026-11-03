//! R4 contract proof for the five owner-approved v1 lifecycle events.
//!
//! The catalogue (`IamEventName`) and the strict payload validator
//! (`validate_iam_payload`) are separate registries that can drift. This test
//! pins them together without a database: every approved name must exist in the
//! catalogue, accept exactly its v1 shape, reject a wrong version, and reject an
//! unknown field such as an email address.

use serde_json::{Value, json};
use sitolo_audit::IamEventName;
use sitolo_events::validate_iam_payload;

/// Approved transition-to-event matrix with a canonical v1 payload.
const APPROVED: [(&str, &str); 5] = [
    (
        "iam.membership.pending_acceptance",
        r#"{"membership_id":"member-1","from_state":"INVITED","to_state":"PENDING_ACCEPTANCE","state_version":2}"#,
    ),
    (
        "iam.membership.expired",
        r#"{"membership_id":"member-1","from_state":"PENDING_ACCEPTANCE","to_state":"EXPIRED","state_version":3,"expiry_reason":"window_elapsed"}"#,
    ),
    (
        "iam.invitation.issued",
        r#"{"invitation_id":"invite-1","membership_id":"member-1","expires_at":"2026-10-23T00:00:00Z","state_version":1}"#,
    ),
    (
        "iam.invitation.withdrawn",
        r#"{"invitation_id":"invite-1","membership_id":"member-1","from_state":"ISSUED","to_state":"WITHDRAWN","state_version":2,"reason":"owner_request"}"#,
    ),
    (
        "iam.invitation.expired",
        r#"{"invitation_id":"invite-1","membership_id":"member-1","from_state":"ISSUED","to_state":"EXPIRED","expires_at":"2026-10-23T00:00:00Z","state_version":2}"#,
    ),
];

#[test]
fn approved_lifecycle_catalogue_matches_the_payload_contract()
-> Result<(), Box<dyn std::error::Error>> {
    for (name, payload) in APPROVED {
        let variant = IamEventName::ALL
            .iter()
            .copied()
            .find(|event| event.as_str() == name)
            .ok_or_else(|| format!("missing catalogue variant for {name}"))?;
        assert_eq!(variant.as_str(), name, "catalogue name drift");

        validate_iam_payload(name, 1, payload)?;
        assert!(
            validate_iam_payload(name, 2, payload).is_err(),
            "{name} accepted a non-v1 version"
        );
        assert!(
            validate_iam_payload(name, 1, &with_unknown_field(payload)?).is_err(),
            "{name} accepted an unknown field"
        );
    }
    Ok(())
}

#[test]
fn catalogue_event_names_are_unique() {
    let mut names: Vec<&str> = IamEventName::ALL
        .iter()
        .map(|event| event.as_str())
        .collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        names.len(),
        total,
        "duplicate IAM event name in the catalogue"
    );
}

/// Adds a field that is never part of an approved schema. A validator that
/// accepts it would let contact data or token material leak into the outbox.
fn with_unknown_field(payload: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut value: Value = serde_json::from_str(payload)?;
    let Some(object) = value.as_object_mut() else {
        return Err("payload is not a JSON object".into());
    };
    object.insert("email".to_owned(), json!("person@example.test"));
    Ok(serde_json::to_string(&value)?)
}
