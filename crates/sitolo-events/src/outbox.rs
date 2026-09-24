//! Phase 4 Part 8 Outbox Record model and lifecycle states.
//!
//! Phase 4 Part 8 (§14, §15, §25). Outbox records represent durable event intent
//! to be published asynchronously by worker/relay.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutboxEventId(String);

impl OutboxEventId {
    pub fn new(id: impl Into<String>) -> Result<Self, OutboxError> {
        let s = id.into();
        if s.trim().is_empty() {
            Err(OutboxError::InvalidId)
        } else {
            Ok(Self(s))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OutboxEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("outbox event ID cannot be empty")]
    InvalidId,
    #[error("outbox persistence or delivery failed")]
    PersistenceFailed,
}

/// Outbox processing status (§15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutboxStatus {
    Pending,
    Claimed,
    Published,
    Quarantined,
}

impl OutboxStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutboxStatus::Pending => "PENDING",
            OutboxStatus::Claimed => "CLAIMED",
            OutboxStatus::Published => "PUBLISHED",
            OutboxStatus::Quarantined => "QUARANTINED",
        }
    }
}

/// Worker retry classification (§25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetryClassification {
    Retryable,
    Terminal,
    Unknown,
}

/// Outbox Event Record (§14).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEvent {
    pub event_id: OutboxEventId,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_name: String,
    pub event_version: u32,
    pub organization_id: Option<String>,
    pub branch_id: Option<String>,
    #[serde(with = "time_serde")]
    pub occurred_at: SystemTime,
    pub payload: String,
    pub status: OutboxStatus,
    #[serde(with = "time_serde")]
    pub available_at: SystemTime,
    pub attempt_count: u32,
    #[serde(with = "opt_time_serde", default)]
    pub locked_at: Option<SystemTime>,
    #[serde(with = "opt_time_serde", default)]
    pub published_at: Option<SystemTime>,
    pub last_error_class: Option<String>,
    pub deduplication_key: String,
    pub schema_version: u32,
}

mod time_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let nanos = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos() as u64;
        serializer.serialize_u64(nanos)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let nanos = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_nanos(nanos))
    }
}

mod opt_time_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match time {
            Some(t) => {
                let nanos = t
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::ZERO)
                    .as_nanos() as u64;
                serializer.serialize_some(&nanos)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<u64>::deserialize(deserializer)?;
        Ok(opt.map(|nanos| UNIX_EPOCH + Duration::from_nanos(nanos)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outbox_id_validation() {
        assert!(OutboxEventId::new("").is_err());
        assert!(OutboxEventId::new("   ").is_err());
        assert!(OutboxEventId::new("evt-outbox-123").is_ok());
    }

    #[test]
    fn status_string_conversion() {
        assert_eq!(OutboxStatus::Pending.as_str(), "PENDING");
        assert_eq!(OutboxStatus::Claimed.as_str(), "CLAIMED");
        assert_eq!(OutboxStatus::Published.as_str(), "PUBLISHED");
        assert_eq!(OutboxStatus::Quarantined.as_str(), "QUARANTINED");
    }

    #[test]
    fn outbox_event_serialization_roundtrip() {
        let event = OutboxEvent {
            event_id: OutboxEventId::new("evt-1").unwrap(),
            aggregate_type: "membership".into(),
            aggregate_id: "mem-1".into(),
            event_name: "iam.membership.role_changed".into(),
            event_version: 1,
            organization_id: Some("org-1".into()),
            branch_id: Some("br-1".into()),
            occurred_at: SystemTime::UNIX_EPOCH,
            payload: "{\"role\":\"manager\"}".into(),
            status: OutboxStatus::Pending,
            available_at: SystemTime::UNIX_EPOCH,
            attempt_count: 0,
            locked_at: None,
            published_at: None,
            last_error_class: None,
            deduplication_key: "dedup-1".into(),
            schema_version: 1,
        };
        let json = serde_json::to_string(&event).unwrap();
        let de: OutboxEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(de.event_id, event.event_id);
        assert_eq!(de.event_name, event.event_name);
    }
}
