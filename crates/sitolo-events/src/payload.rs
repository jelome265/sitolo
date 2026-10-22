//! Strict payload validation for approved IAM integration-event schemas.

use chrono::DateTime;
use serde_json::Value;
use sitolo_domain::tenancy::{InvitationId, MembershipId};

use crate::OutboxError;

/// Validates payloads whose v1 schemas are defined by the approved PR-009
/// lifecycle event mapping. Other registered event payloads retain their
/// existing schema-validation path.
///
/// # Errors
///
/// Returns [`OutboxError::InvalidSchema`] when a mapped event has an
/// unsupported version, malformed JSON, missing/unknown fields, or invalid
/// field values.
pub fn validate_iam_payload(
    event_name: &str,
    event_version: u32,
    payload: &str,
) -> Result<(), OutboxError> {
    let Some(schema) = LifecycleSchema::from_event_name(event_name) else {
        return Ok(());
    };
    if event_version != 1 {
        return Err(OutboxError::InvalidSchema);
    }

    let value: Value = serde_json::from_str(payload).map_err(|_| OutboxError::InvalidSchema)?;
    let object = value.as_object().ok_or(OutboxError::InvalidSchema)?;
    match schema {
        LifecycleSchema::MembershipPendingAcceptance => {
            validate_keys(
                object,
                &["membership_id", "from_state", "to_state", "state_version"],
                &[],
            )?;
            validate_membership_id(object)?;
            validate_state_transition(object, "INVITED", "PENDING_ACCEPTANCE")?;
            validate_state_version(object)
        }
        LifecycleSchema::MembershipExpired => {
            validate_keys(
                object,
                &[
                    "membership_id",
                    "from_state",
                    "to_state",
                    "state_version",
                    "expiry_reason",
                ],
                &[],
            )?;
            validate_membership_id(object)?;
            validate_state_transition_from(object, &["PENDING_ACCEPTANCE", "INVITED"], "EXPIRED")?;
            validate_state_version(object)?;
            validate_bounded_reason(object, "expiry_reason")
        }
        LifecycleSchema::InvitationIssued => {
            validate_keys(
                object,
                &[
                    "invitation_id",
                    "membership_id",
                    "expires_at",
                    "state_version",
                ],
                &[],
            )?;
            validate_invitation_id(object)?;
            validate_membership_id(object)?;
            validate_utc_timestamp(object, "expires_at")?;
            validate_state_version(object)
        }
        LifecycleSchema::InvitationWithdrawn => {
            validate_keys(
                object,
                &[
                    "invitation_id",
                    "membership_id",
                    "from_state",
                    "to_state",
                    "state_version",
                ],
                &["reason"],
            )?;
            validate_invitation_id(object)?;
            validate_membership_id(object)?;
            validate_state_transition(object, "ISSUED", "WITHDRAWN")?;
            validate_state_version(object)?;
            if object.contains_key("reason") {
                validate_bounded_reason(object, "reason")?;
            }
            Ok(())
        }
        LifecycleSchema::InvitationExpired => {
            validate_keys(
                object,
                &[
                    "invitation_id",
                    "membership_id",
                    "from_state",
                    "to_state",
                    "expires_at",
                    "state_version",
                ],
                &[],
            )?;
            validate_invitation_id(object)?;
            validate_membership_id(object)?;
            validate_state_transition(object, "ISSUED", "EXPIRED")?;
            validate_utc_timestamp(object, "expires_at")?;
            validate_state_version(object)
        }
    }
}

#[derive(Clone, Copy)]
enum LifecycleSchema {
    MembershipPendingAcceptance,
    MembershipExpired,
    InvitationIssued,
    InvitationWithdrawn,
    InvitationExpired,
}

impl LifecycleSchema {
    fn from_event_name(event_name: &str) -> Option<Self> {
        match event_name {
            "iam.membership.pending_acceptance" => Some(Self::MembershipPendingAcceptance),
            "iam.membership.expired" => Some(Self::MembershipExpired),
            "iam.invitation.issued" => Some(Self::InvitationIssued),
            "iam.invitation.withdrawn" => Some(Self::InvitationWithdrawn),
            "iam.invitation.expired" => Some(Self::InvitationExpired),
            _ => None,
        }
    }
}

fn validate_keys(
    object: &serde_json::Map<String, Value>,
    required: &[&str],
    optional: &[&str],
) -> Result<(), OutboxError> {
    if required.iter().any(|key| !object.contains_key(*key))
        || object
            .keys()
            .any(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        return Err(OutboxError::InvalidSchema);
    }
    Ok(())
}

fn validate_membership_id(object: &serde_json::Map<String, Value>) -> Result<(), OutboxError> {
    let id = object
        .get("membership_id")
        .and_then(Value::as_str)
        .ok_or(OutboxError::InvalidSchema)?;
    MembershipId::new(id.to_owned()).map_err(|_| OutboxError::InvalidSchema)?;
    Ok(())
}

fn validate_invitation_id(object: &serde_json::Map<String, Value>) -> Result<(), OutboxError> {
    let id = object
        .get("invitation_id")
        .and_then(Value::as_str)
        .ok_or(OutboxError::InvalidSchema)?;
    InvitationId::new(id.to_owned()).map_err(|_| OutboxError::InvalidSchema)?;
    Ok(())
}

fn validate_state_transition(
    object: &serde_json::Map<String, Value>,
    expected_from: &str,
    expected_to: &str,
) -> Result<(), OutboxError> {
    validate_state_transition_from(object, &[expected_from], expected_to)
}

fn validate_state_transition_from(
    object: &serde_json::Map<String, Value>,
    expected_from: &[&str],
    expected_to: &str,
) -> Result<(), OutboxError> {
    if !expected_from.contains(
        &object
            .get("from_state")
            .and_then(Value::as_str)
            .unwrap_or(""),
    ) || object.get("to_state").and_then(Value::as_str) != Some(expected_to)
    {
        return Err(OutboxError::InvalidSchema);
    }
    Ok(())
}

fn validate_state_version(object: &serde_json::Map<String, Value>) -> Result<(), OutboxError> {
    if object
        .get("state_version")
        .and_then(Value::as_u64)
        .is_none_or(|version| version == 0)
    {
        return Err(OutboxError::InvalidSchema);
    }
    Ok(())
}

fn validate_bounded_reason(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<(), OutboxError> {
    let reason = object
        .get(key)
        .and_then(Value::as_str)
        .ok_or(OutboxError::InvalidSchema)?;
    if reason.is_empty()
        || reason.len() > 64
        || !reason
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(OutboxError::InvalidSchema);
    }
    Ok(())
}

fn validate_utc_timestamp(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<(), OutboxError> {
    let timestamp = object
        .get(key)
        .and_then(Value::as_str)
        .ok_or(OutboxError::InvalidSchema)?;
    let parsed = DateTime::parse_from_rfc3339(timestamp).map_err(|_| OutboxError::InvalidSchema)?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(OutboxError::InvalidSchema);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_iam_payload;

    #[test]
    fn approved_lifecycle_schemas_accept_only_their_v1_shape() {
        let cases = [
            (
                "iam.membership.pending_acceptance",
                r#"{"membership_id":"member-1","from_state":"INVITED","to_state":"PENDING_ACCEPTANCE","state_version":2}"#,
            ),
            (
                "iam.membership.expired",
                r#"{"membership_id":"member-1","from_state":"PENDING_ACCEPTANCE","to_state":"EXPIRED","state_version":3,"expiry_reason":"window_elapsed"}"#,
            ),
            (
                "iam.membership.expired",
                r#"{"membership_id":"member-1","from_state":"INVITED","to_state":"EXPIRED","state_version":2,"expiry_reason":"invitation_expired"}"#,
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

        for (name, payload) in cases {
            assert!(validate_iam_payload(name, 1, payload).is_ok(), "{name}");
        }
    }

    #[test]
    fn approved_lifecycle_schemas_reject_wrong_version_transition_and_non_utc_time() {
        let valid = r#"{"membership_id":"member-1","from_state":"INVITED","to_state":"PENDING_ACCEPTANCE","state_version":2}"#;
        assert!(validate_iam_payload("iam.membership.pending_acceptance", 2, valid).is_err());
        assert!(validate_iam_payload(
            "iam.membership.pending_acceptance",
            1,
            r#"{"membership_id":"member-1","from_state":"ACTIVE","to_state":"PENDING_ACCEPTANCE","state_version":2}"#
        )
        .is_err());
        assert!(validate_iam_payload(
            "iam.membership.expired",
            1,
            r#"{"membership_id":"member-1","from_state":"ACTIVE","to_state":"EXPIRED","state_version":2,"expiry_reason":"invitation_expired"}"#
        )
        .is_err());
        assert!(validate_iam_payload(
            "iam.invitation.issued",
            1,
            r#"{"invitation_id":"invite-1","membership_id":"member-1","expires_at":"2026-10-23T00:00:00+02:00","state_version":1}"#
        )
        .is_err());
    }
}
