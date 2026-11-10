//! AWS SNS FIFO event publisher for the owner-approved Part 8 relay contract.

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::time::{Duration, SystemTime};

    use sitolo_domain::tenancy::OrganizationId;
    use sitolo_events::{OutboxEvent, OutboxStatus};

    use super::*;

    #[test]
    fn fifo_configuration_requires_matching_region_and_fifo_topic() -> Result<(), Box<dyn Error>> {
        let config = SnsRelayConfig::new(
            "af-south-1",
            "arn:aws:sns:af-south-1:123456789012:sitolo-iam-events-prod.fifo",
            Duration::from_secs(5),
        )?;
        assert_eq!(config.region(), "af-south-1");
        assert_eq!(
            config.topic_arn(),
            "arn:aws:sns:af-south-1:123456789012:sitolo-iam-events-prod.fifo"
        );
        Ok(())
    }

    #[test]
    fn fifo_configuration_rejects_region_mismatch_standard_topic_and_bad_account() {
        for (region, topic) in [
            (
                "af-south-1",
                "arn:aws:sns:eu-west-1:123456789012:sitolo-iam-events-prod.fifo",
            ),
            (
                "af-south-1",
                "arn:aws:sns:af-south-1:123456789012:sitolo-iam-events-prod",
            ),
            (
                "af-south-1",
                "arn:aws:sns:af-south-1:123:sitolo-iam-events-prod.fifo",
            ),
        ] {
            assert!(SnsRelayConfig::new(region, topic, Duration::from_secs(5)).is_err());
        }
        assert!(
            SnsRelayConfig::new(
                "af-south-1",
                "arn:aws:sns:af-south-1:123456789012:sitolo-iam-events-prod.fifo",
                Duration::ZERO,
            )
            .is_err()
        );
    }

    #[test]
    fn published_envelope_keeps_stable_identity_and_excludes_delivery_metadata()
    -> Result<(), Box<dyn Error>> {
        let mut event = sample_event()?;
        event.attempt_count = 7;
        event.last_error_class = Some("credential_failure".to_owned());
        event.available_at = SystemTime::UNIX_EPOCH;

        let first = encode_envelope(&event, "evt-1")?;
        let second = encode_envelope(&event, "evt-1")?;
        assert_eq!(first.message_deduplication_id, "evt-1");
        assert_eq!(first.message_group_id, second.message_group_id);
        assert_eq!(first.body, second.body);
        assert!(first.body.contains("iam.invitation.issued"));
        assert!(first.body.contains("aggregate_sequence"));
        assert!(!first.body.contains("attempt_count"));
        assert!(!first.body.contains("available_at"));
        assert!(!first.body.contains("claim_token"));
        assert!(!first.body.contains("credential_failure"));
        Ok(())
    }

    #[test]
    fn published_envelope_rejects_sequence_or_idempotency_mismatch() -> Result<(), Box<dyn Error>> {
        let mut event = sample_event()?;
        event.aggregate_sequence = None;
        assert!(encode_envelope(&event, "evt-1").is_err());

        let event = sample_event()?;
        assert!(encode_envelope(&event, "different-event").is_err());
        Ok(())
    }

    fn sample_event() -> Result<OutboxEvent, Box<dyn Error>> {
        Ok(OutboxEvent {
            event_id: "evt-1".to_owned(),
            aggregate_type: "Membership".to_owned(),
            aggregate_id: "mem-1".to_owned(),
            event_name: "iam.invitation.issued".to_owned(),
            event_version: 1,
            organization_id: OrganizationId::new("org-1")?,
            branch_id: None,
            occurred_at: SystemTime::UNIX_EPOCH,
            payload: r#"{"invitation_id":"inv-1","membership_id":"mem-1","expires_at":"2026-10-23T00:00:00Z","state_version":1}"#.to_owned(),
            status: OutboxStatus::Claimed,
            available_at: SystemTime::UNIX_EPOCH,
            attempt_count: 1,
            locked_at: Some(SystemTime::UNIX_EPOCH),
            published_at: None,
            last_error_class: None,
            deduplication_key: "evt-1".to_owned(),
            schema_version: 1,
            aggregate_sequence: Some(1),
            claim_token: None,
            claim_generation: 1,
        })
    }
}
use std::time::Duration;

use async_trait::async_trait;
use aws_config::{BehaviorVersion, ecs::EcsCredentialsProvider};
use aws_sdk_sns::{Client, config::Region, error::ProvideErrorMetadata};
use chrono::{SecondsFormat, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use sitolo_audit::IamEventName;
use sitolo_domain::tenancy::BranchId;
use sitolo_events::{
    OutboxError, OutboxErrorClass, OutboxEvent, OutboxStatus, RetryClass, validate_iam_payload,
};
use thiserror::Error;

use crate::relay::{DeliveryFailure, EventPublisher};

const MAX_SNS_MESSAGE_BYTES: usize = 256 * 1024;
const MAX_PUBLISH_TIMEOUT: Duration = Duration::from_secs(60);

/// Validated SNS FIFO destination details. The topic identity is supplied by
/// deployment configuration; this code does not create or guess AWS resources.
#[derive(Clone, Debug)]
pub struct SnsRelayConfig {
    region: String,
    topic_arn: String,
    request_timeout: Duration,
}

impl SnsRelayConfig {
    /// Validates the explicit AWS region, SNS FIFO topic ARN and request timeout.
    ///
    /// # Errors
    ///
    /// Returns [`SnsPublisherInitError::InvalidConfiguration`] when the topic
    /// ARN does not identify an SNS FIFO topic in the configured region or the
    /// timeout is outside the supported range.
    pub fn new(
        region: impl Into<String>,
        topic_arn: impl Into<String>,
        request_timeout: Duration,
    ) -> Result<Self, SnsPublisherInitError> {
        let region = region.into();
        let topic_arn = topic_arn.into();
        if region.is_empty()
            || region.len() > 32
            || !region
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || !is_sns_fifo_topic_arn(&region, &topic_arn)
            || request_timeout.is_zero()
            || request_timeout > MAX_PUBLISH_TIMEOUT
        {
            return Err(SnsPublisherInitError::InvalidConfiguration);
        }
        Ok(Self {
            region,
            topic_arn,
            request_timeout,
        })
    }

    /// Returns the explicit AWS region used to sign and route requests.
    #[must_use]
    pub fn region(&self) -> &str {
        &self.region
    }

    /// Returns the explicitly configured FIFO topic ARN.
    #[must_use]
    pub fn topic_arn(&self) -> &str {
        &self.topic_arn
    }
}

/// Publisher construction or destination configuration error.
#[derive(Debug, Error)]
pub enum SnsPublisherInitError {
    /// The region, topic ARN, or request timeout failed validation.
    #[error("invalid SNS FIFO publisher configuration")]
    InvalidConfiguration,
    /// A fixed error classification was rejected by its grammar.
    #[error("invalid bounded relay error classification")]
    InvalidErrorClass(#[source] OutboxError),
}

/// Concrete SNS FIFO publisher using only ECS task-role credentials.
pub struct SnsFifoPublisher {
    client: Client,
    config: SnsRelayConfig,
    failure_classes: SnsFailureClasses,
}

impl std::fmt::Debug for SnsFifoPublisher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The AWS client is omitted deliberately: it carries credential and
        // endpoint state that must never reach a log line.
        formatter
            .debug_struct("SnsFifoPublisher")
            .field("config", &self.config)
            .field("failure_classes", &self.failure_classes)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct SnsFailureClasses {
    retryable: OutboxErrorClass,
    terminal: OutboxErrorClass,
    unavailable: OutboxErrorClass,
}

impl SnsFifoPublisher {
    /// Loads an AWS client with the ECS task credential provider and an explicit
    /// region. The general AWS credential chain is deliberately not used, so
    /// static access-key environment variables and developer profiles cannot
    /// become production workload identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the destination configuration or fixed error
    /// classes are invalid. ECS credential resolution and network failures are
    /// reported on the first publish attempt through [`DeliveryFailure`].
    pub async fn from_ecs_task_role(config: SnsRelayConfig) -> Result<Self, SnsPublisherInitError> {
        let credentials = EcsCredentialsProvider::builder()
            .connect_timeout(Duration::from_secs(2))
            .read_timeout(Duration::from_secs(5))
            .build();
        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .credentials_provider(credentials)
            .load()
            .await;
        let client = Client::new(&shared_config);
        Self::new(client, config)
    }

    fn new(client: Client, config: SnsRelayConfig) -> Result<Self, SnsPublisherInitError> {
        let failure_classes = SnsFailureClasses {
            retryable: OutboxErrorClass::new("sns_retryable")
                .map_err(SnsPublisherInitError::InvalidErrorClass)?,
            terminal: OutboxErrorClass::new("sns_event_rejected")
                .map_err(SnsPublisherInitError::InvalidErrorClass)?,
            unavailable: OutboxErrorClass::new("sns_destination_unavailable")
                .map_err(SnsPublisherInitError::InvalidErrorClass)?,
        };
        Ok(Self {
            client,
            config,
            failure_classes,
        })
    }
}

#[async_trait]
impl EventPublisher for SnsFifoPublisher {
    async fn publish(
        &self,
        event: &OutboxEvent,
        idempotency_key: &str,
    ) -> Result<(), DeliveryFailure> {
        let encoded = encode_envelope(event, idempotency_key).map_err(|_rejected| {
            // Encoding failures are permanent for this event; the specific
            // schema detail is not exposed across the delivery boundary.
            DeliveryFailure::new(self.failure_classes.terminal.clone(), RetryClass::Terminal)
        })?;
        let request = self
            .client
            .publish()
            .topic_arn(&self.config.topic_arn)
            .message(encoded.body)
            .message_group_id(encoded.message_group_id)
            .message_deduplication_id(encoded.message_deduplication_id)
            .send();

        let response = tokio::time::timeout(self.config.request_timeout, request).await;
        match response {
            Ok(Ok(output)) if output.message_id().is_some_and(|id| !id.is_empty()) => Ok(()),
            Ok(Err(error)) => Err(self.classify_publish_error(&error)),
            // A timeout or a response without an acceptance identifier is
            // ambiguous; the worker retries with the same FIFO deduplication key.
            Err(_) | Ok(Ok(_)) => Err(DeliveryFailure::new(
                self.failure_classes.retryable.clone(),
                RetryClass::Retryable,
            )),
        }
    }
}

impl SnsFifoPublisher {
    fn classify_publish_error(
        &self,
        error: &aws_sdk_sns::error::SdkError<aws_sdk_sns::operation::publish::PublishError>,
    ) -> DeliveryFailure {
        let code = error
            .as_service_error()
            .and_then(ProvideErrorMetadata::code);
        match code {
            Some("AccessDenied" | "AuthorizationError" | "NotFound" | "InvalidSecurity") => {
                DeliveryFailure::unavailable(self.failure_classes.unavailable.clone())
            }
            Some("InvalidParameter" | "InvalidParameterValue" | "InvalidMessageContents") => {
                DeliveryFailure::new(self.failure_classes.terminal.clone(), RetryClass::Terminal)
            }
            _ => DeliveryFailure::new(
                self.failure_classes.retryable.clone(),
                RetryClass::Retryable,
            ),
        }
    }
}

struct EncodedSnsMessage {
    body: String,
    message_group_id: String,
    message_deduplication_id: String,
}

fn encode_envelope(
    event: &OutboxEvent,
    idempotency_key: &str,
) -> Result<EncodedSnsMessage, OutboxError> {
    let aggregate_sequence = event.aggregate_sequence.ok_or(OutboxError::InvalidSchema)?;
    if event.status != OutboxStatus::Claimed
        || event.event_id != idempotency_key
        || !valid_sns_deduplication_id(idempotency_key)
        || event.event_version != 1
        || event.schema_version != event.event_version
        || !IamEventName::ALL
            .iter()
            .any(|registered| registered.as_str() == event.event_name)
    {
        return Err(OutboxError::InvalidSchema);
    }
    validate_iam_payload(&event.event_name, event.event_version, &event.payload)?;
    let payload: serde_json::Value = serde_json::from_str(&event.payload).map_err(|_rejected| {
        // The persisted payload was already validated before claim; any
        // re-parse failure is a permanent schema mismatch.
        OutboxError::InvalidSchema
    })?;
    let occurred_at = chrono::DateTime::<Utc>::from(event.occurred_at)
        .to_rfc3339_opts(SecondsFormat::Nanos, true);
    let branch_id = event.branch_id.as_ref().map(BranchId::as_str);
    let body = serde_json::to_string(&json!({
        "event_id": event.event_id,
        "event_name": event.event_name,
        "event_version": event.event_version,
        "occurred_at": occurred_at,
        "organization_id": event.organization_id.as_str(),
        "branch_id": branch_id,
        "aggregate_type": event.aggregate_type,
        "aggregate_id": event.aggregate_id,
        "aggregate_sequence": aggregate_sequence,
        "payload": payload,
    }))
    .map_err(|_rejected| {
        // Serializing an owned JSON value cannot fail in practice; if it does,
        // the event is permanently unencodable.
        OutboxError::InvalidSchema
    })?;
    if body.len() > MAX_SNS_MESSAGE_BYTES {
        return Err(OutboxError::PayloadTooLarge);
    }
    let message_group_id = aggregate_group_id(
        event.organization_id.as_str(),
        &event.aggregate_type,
        &event.aggregate_id,
    );
    Ok(EncodedSnsMessage {
        body,
        message_group_id,
        message_deduplication_id: idempotency_key.to_owned(),
    })
}

fn valid_sns_deduplication_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~".contains(&byte)
        })
}

pub(crate) fn aggregate_group_id(
    organization_id: &str,
    aggregate_type: &str,
    aggregate_id: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(organization_id.as_bytes());
    hasher.update([0]);
    hasher.update(aggregate_type.as_bytes());
    hasher.update([0]);
    hasher.update(aggregate_id.as_bytes());
    let digest = hasher.finalize();
    let mut output = String::with_capacity(65);
    output.push('g');
    for byte in digest {
        output.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        output.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    output
}

fn is_sns_fifo_topic_arn(region: &str, topic_arn: &str) -> bool {
    let mut segments = topic_arn.split(':');
    let valid_prefix = segments.next() == Some("arn")
        && segments.next() == Some("aws")
        && segments.next() == Some("sns");
    let arn_region = segments.next();
    let account_id = segments.next();
    let topic_name = segments.next();
    valid_prefix
        && arn_region == Some(region)
        && account_id.is_some_and(|account| {
            account.len() == 12 && account.bytes().all(|byte| byte.is_ascii_digit())
        })
        && topic_name.is_some_and(|name| {
            name.strip_suffix(".fifo")
                .is_some_and(|base| !base.is_empty() && base.len() <= 251)
        })
        && segments.next().is_none()
}
