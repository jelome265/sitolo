//! Concrete, workload-identity-backed SQS FIFO consumer transport.
//!
//! Queue URLs are explicit deployment inputs. This adapter never creates
//! queues or grants permissions; the consumer role needs receive/delete on
//! its source queue and send permission on its own FIFO quarantine queue.

use std::time::Duration;

use async_trait::async_trait;
use aws_config::{BehaviorVersion, ecs::EcsCredentialsProvider};
use aws_sdk_sqs::{
    Client,
    config::Region,
    types::{MessageAttributeValue, MessageSystemAttributeName},
};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::consumer::{ConsumerTransport, ConsumerTransportError, ReceivedMessage};
use sitolo_events::OutboxErrorClass;

const MAX_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_SQS_BODY_BYTES: usize = 1_048_576;

/// Explicit SQS FIFO endpoints and bounded SDK call timeout.
#[derive(Clone)]
pub struct SqsFifoConsumerConfig {
    region: String,
    queue_url: String,
    quarantine_queue_url: String,
    request_timeout: Duration,
}

impl SqsFifoConsumerConfig {
    /// Validates AWS region, same-region FIFO queue URLs and timeout.
    pub fn new(
        region: impl Into<String>,
        queue_url: impl Into<String>,
        quarantine_queue_url: impl Into<String>,
        request_timeout: Duration,
    ) -> Result<Self, SqsConsumerInitError> {
        let region = region.into();
        let queue_url = queue_url.into();
        let quarantine_queue_url = quarantine_queue_url.into();
        if !valid_region(&region)
            || !valid_queue_url(&region, &queue_url)
            || !valid_queue_url(&region, &quarantine_queue_url)
            || queue_url == quarantine_queue_url
            || request_timeout.is_zero()
            || request_timeout > MAX_REQUEST_TIMEOUT
        {
            return Err(SqsConsumerInitError::InvalidConfiguration);
        }
        Ok(Self {
            region,
            queue_url,
            quarantine_queue_url,
            request_timeout,
        })
    }
}

/// SQS client construction error.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SqsConsumerInitError {
    #[error("invalid SQS FIFO consumer configuration")]
    InvalidConfiguration,
}

/// Receives, deletes and durably transfers terminal messages using one
/// consumer's explicitly configured source and quarantine FIFO queues.
pub struct SqsFifoConsumerTransport {
    client: Client,
    config: SqsFifoConsumerConfig,
}

impl SqsFifoConsumerTransport {
    /// Uses ECS task-role credentials and the configured region only.
    pub async fn from_ecs_task_role(
        config: SqsFifoConsumerConfig,
    ) -> Result<Self, SqsConsumerInitError> {
        let credentials = EcsCredentialsProvider::builder()
            .connect_timeout(Duration::from_secs(2))
            .read_timeout(Duration::from_secs(5))
            .build();
        let shared = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .credentials_provider(credentials)
            .load()
            .await;
        Ok(Self {
            client: Client::new(&shared),
            config,
        })
    }

    /// Long-polls one bounded FIFO batch. SQS itself serializes delivery within
    /// each message group; this method returns the batch in service order.
    pub async fn receive_batch(&self) -> Result<Vec<ReceivedMessage>, ConsumerTransportError> {
        let response = tokio::time::timeout(
            self.config.request_timeout,
            self.client
                .receive_message()
                .queue_url(&self.config.queue_url)
                .max_number_of_messages(10)
                .wait_time_seconds(20)
                .visibility_timeout(60)
                .message_system_attribute_names(MessageSystemAttributeName::MessageGroupId)
                .send(),
        )
        .await
        .map_err(|_| ConsumerTransportError)?
        .map_err(|_| ConsumerTransportError)?;

        response
            .messages()
            .iter()
            .map(|message| {
                let message_id = message
                    .message_id()
                    .filter(|value| !value.is_empty() && value.len() <= 128)
                    .ok_or(ConsumerTransportError)?;
                let body = message.body().ok_or(ConsumerTransportError)?;
                if body.len() > MAX_SQS_BODY_BYTES {
                    return Err(ConsumerTransportError);
                }
                let receipt_handle = message
                    .receipt_handle()
                    .filter(|receipt| !receipt.is_empty() && receipt.len() <= 1024)
                    .ok_or(ConsumerTransportError)?;
                let message_group_id = message
                    .attributes()
                    .and_then(|attributes| {
                        attributes.get(&MessageSystemAttributeName::MessageGroupId)
                    })
                    .cloned();
                Ok(ReceivedMessage {
                    message_id: message_id.to_owned(),
                    body: body.to_owned(),
                    receipt_handle: receipt_handle.to_owned(),
                    message_group_id,
                })
            })
            .collect()
    }
}

#[async_trait]
impl ConsumerTransport for SqsFifoConsumerTransport {
    async fn receive_batch(&self) -> Result<Vec<ReceivedMessage>, ConsumerTransportError> {
        SqsFifoConsumerTransport::receive_batch(self).await
    }

    async fn acknowledge(&self, receipt_handle: &str) -> Result<(), ConsumerTransportError> {
        if receipt_handle.is_empty() || receipt_handle.len() > 1024 {
            return Err(ConsumerTransportError);
        }
        tokio::time::timeout(
            self.config.request_timeout,
            self.client
                .delete_message()
                .queue_url(&self.config.queue_url)
                .receipt_handle(receipt_handle)
                .send(),
        )
        .await
        .map_err(|_| ConsumerTransportError)?
        .map_err(|_| ConsumerTransportError)?;
        Ok(())
    }

    async fn quarantine(
        &self,
        message: &ReceivedMessage,
        error_class: &OutboxErrorClass,
    ) -> Result<(), ConsumerTransportError> {
        if message.message_id.is_empty()
            || message.message_id.len() > 128
            || message.body.len() > MAX_SQS_BODY_BYTES
            || message.receipt_handle.is_empty()
            || message.receipt_handle.len() > 1024
        {
            return Err(ConsumerTransportError);
        }
        let group_id = message
            .message_group_id
            .as_deref()
            .filter(|value| !value.is_empty() && value.len() <= 128)
            .ok_or(ConsumerTransportError)?;
        let deduplication_id =
            quarantine_deduplication_id(&message.message_id, error_class.as_str());
        let attributes = MessageAttributeValue::builder()
            .data_type("String")
            .string_value(error_class.as_str())
            .build()
            .map_err(|_| ConsumerTransportError)?;

        // Transfer first; only delete the source after SQS confirms the DLQ
        // accepted the message. A lost response replays the same dedup ID.
        let transferred = tokio::time::timeout(
            self.config.request_timeout,
            self.client
                .send_message()
                .queue_url(&self.config.quarantine_queue_url)
                .message_body(&message.body)
                .message_group_id(group_id)
                .message_deduplication_id(deduplication_id)
                .message_attributes("sitolo_error_class", attributes)
                .send(),
        )
        .await
        .map_err(|_| ConsumerTransportError)?
        .map_err(|_| ConsumerTransportError)?;
        if transferred.message_id().is_none_or(str::is_empty) {
            return Err(ConsumerTransportError);
        }
        self.acknowledge(&message.receipt_handle).await
    }
}

fn valid_region(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_queue_url(region: &str, value: &str) -> bool {
    let Some(authority_and_path) = value.strip_prefix("https://") else {
        return false;
    };
    let Some((host, path)) = authority_and_path.split_once('/') else {
        return false;
    };
    if host != format!("sqs.{region}.amazonaws.com") || path.contains(['?', '#', '@', '\\']) {
        return false;
    }
    let mut parts = path.split('/');
    let account = parts.next();
    let queue = parts.next();
    account
        .is_some_and(|value| value.len() == 12 && value.bytes().all(|byte| byte.is_ascii_digit()))
        && queue.is_some_and(|value| {
            value.strip_suffix(".fifo").is_some_and(|base| {
                !base.is_empty()
                    && base.len() <= 75
                    && base
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            })
        })
        && parts.next().is_none()
}

fn quarantine_deduplication_id(source_message_id: &str, error_class: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(error_class.as_bytes());
    hasher.update([0]);
    hasher.update(source_message_id.as_bytes());
    let digest = hasher.finalize();
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        output.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "https://sqs.af-south-1.amazonaws.com/123456789012/main.fifo";
    const DLQ: &str = "https://sqs.af-south-1.amazonaws.com/123456789012/main-dlq.fifo";

    #[test]
    fn consumer_configuration_accepts_explicit_fifo_urls_only() {
        assert!(
            SqsFifoConsumerConfig::new("af-south-1", SOURCE, DLQ, Duration::from_secs(10)).is_ok()
        );
        assert!(
            SqsFifoConsumerConfig::new("af-south-1", SOURCE, SOURCE, Duration::from_secs(10))
                .is_err()
        );
        assert!(
            SqsFifoConsumerConfig::new(
                "af-south-1",
                "http://sqs.af-south-1.amazonaws.com/123456789012/main.fifo",
                DLQ,
                Duration::from_secs(10)
            )
            .is_err()
        );
        assert!(
            SqsFifoConsumerConfig::new(
                "af-south-1",
                "https://sqs.us-east-1.amazonaws.com/123456789012/main.fifo",
                DLQ,
                Duration::from_secs(10)
            )
            .is_err()
        );
        assert!(
            SqsFifoConsumerConfig::new(
                "af-south-1",
                "https://example.com/123456789012/main.fifo",
                DLQ,
                Duration::from_secs(10)
            )
            .is_err()
        );
    }

    #[test]
    fn quarantine_deduplication_identity_is_stable_and_source_bound() {
        let first = quarantine_deduplication_id("source-id", "invalid_event");
        assert_eq!(
            first,
            quarantine_deduplication_id("source-id", "invalid_event")
        );
        assert_ne!(
            first,
            quarantine_deduplication_id("other-id", "invalid_event")
        );
        assert_ne!(
            first,
            quarantine_deduplication_id("source-id", "other_error")
        );
    }
}
