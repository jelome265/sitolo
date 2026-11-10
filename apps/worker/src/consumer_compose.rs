//! Consumer destination validation and processor composition.
//!
//! This module splits *composition* from *transport construction*. It wires an
//! already-constructed transport, the production inbox writer, a PostgreSQL
//! pool and an explicitly supplied business effect into the existing
//! [`ConsumerProcessor`]. It constructs no AWS client, resolves no credential
//! and performs no network activity, which keeps composition testable without
//! credentials.
//!
//! The business effect is caller-supplied and has no default. A production
//! worker whose configuration carries the consumer SQS destination must still
//! fail closed until an owner approves and injects the effect through this
//! composition path.
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;

use sitolo_config::AppConfig;
use sitolo_events::OutboxErrorClass;
use sitolo_persistence::consumer_inbox::{ConsumerInboxWriter, PostgresConsumerInbox};
use sqlx_postgres::PgPool;

use crate::consumer::{ConsumerBusinessEffect, ConsumerProcessor, ConsumerTransport};
use crate::sqs::{SqsConsumerInitError, SqsFifoConsumerConfig};

const CONSUMER_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// A validated SQS consumer destination read from the typed configuration.
///
/// Validation happens through [`SqsFifoConsumerConfig::new`] before the
/// destination exists; no construction path bypasses it.
#[derive(Debug, Clone)]
pub struct ConsumerDestination {
    config: SqsFifoConsumerConfig,
    consumer_name: String,
}

impl ConsumerDestination {
    /// Reads the validated consumer triple and region from `config`.
    ///
    /// Returns `Ok(None)` when no consumer destination is configured, matching
    /// the fail-closed relay-only startup path in the binary. A partially
    /// configured consumer is impossible after `sitolo-config` validation; the
    /// partial arm below is defence in depth for direct callers of this API.
    ///
    /// # Errors
    /// Returns [`ConsumerDestinationError::IncompleteDestination`] when only
    /// part of the consumer triple or the region is present, and
    /// [`ConsumerDestinationError::InvalidDestination`] when the queue URLs or
    /// region fail validation.
    pub fn from_config(config: &AppConfig) -> Result<Option<Self>, ConsumerDestinationError> {
        let configured = config.consumer_name.is_some()
            || config.consumer_sqs_queue_url.is_some()
            || config.consumer_sqs_quarantine_queue_url.is_some();
        if !configured {
            return Ok(None);
        }
        let (Some(region), Some(name), Some(source), Some(quarantine)) = (
            config.relay_aws_region.as_deref(),
            config.consumer_name.as_deref(),
            config.consumer_sqs_queue_url.as_deref(),
            config.consumer_sqs_quarantine_queue_url.as_deref(),
        ) else {
            return Err(ConsumerDestinationError::IncompleteDestination);
        };
        let config =
            SqsFifoConsumerConfig::new(region, source, quarantine, CONSUMER_REQUEST_TIMEOUT)
                .map_err(ConsumerDestinationError::InvalidDestination)?;
        Ok(Some(Self {
            config,
            consumer_name: name.to_owned(),
        }))
    }

    /// The validated consumer name.
    #[must_use]
    pub fn consumer_name(&self) -> &str {
        &self.consumer_name
    }

    /// The validated SQS FIFO consumer configuration.
    #[must_use]
    pub fn sqs_config(&self) -> &SqsFifoConsumerConfig {
        &self.config
    }
}

/// The consumer destination is incomplete or invalid.
#[derive(Debug, Error)]
pub enum ConsumerDestinationError {
    /// Only part of the consumer triple or the region is configured.
    #[error(
        "consumer destination is incomplete: region and both FIFO queue URLs must be configured together"
    )]
    IncompleteDestination,
    /// The configured URLs or the region failed FIFO validation.
    #[error("consumer SQS destination failed validation")]
    InvalidDestination(#[source] SqsConsumerInitError),
}

/// Composition failure.
#[derive(Debug, Error)]
pub enum ComposeError {
    /// The consumer name is not accepted by the processor.
    #[error("consumer name is invalid")]
    InvalidConsumerName,
}

/// Wires an already-constructed transport and an explicitly supplied effect
/// into the existing [`ConsumerProcessor`].
///
/// The inbox is the production [`PostgresConsumerInbox`] writer. The effect
/// must be injected by the caller and has no default.
///
/// # Errors
/// Returns [`ComposeError::InvalidConsumerName`] when the destination names an
/// invalid consumer name.
pub fn compose_consumer(
    destination: &ConsumerDestination,
    transport: Arc<dyn ConsumerTransport>,
    pool: PgPool,
    effect: Arc<dyn ConsumerBusinessEffect>,
    invalid_event_class: OutboxErrorClass,
) -> Result<ConsumerProcessor, ComposeError> {
    let inbox: Arc<dyn ConsumerInboxWriter> = Arc::new(PostgresConsumerInbox);
    ConsumerProcessor::new(
        pool,
        destination.consumer_name().to_owned(),
        inbox,
        effect,
        transport,
        invalid_event_class,
    )
    .map_err(|_invalid| ComposeError::InvalidConsumerName)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use sitolo_config::{EnvLoader, validate};
    use sqlx_postgres::PgPoolOptions;

    use super::*;
    use crate::consumer::{ConsumerEffectFailure, ConsumerEvent, ConsumerTransportError};

    const REGION: &str = "af-south-1";
    const SOURCE: &str = "https://sqs.af-south-1.amazonaws.com/123456789012/iam.fifo";
    const QUARANTINE: &str = "https://sqs.af-south-1.amazonaws.com/123456789012/iam-dlq.fifo";

    fn load_app_config(
        pairs: impl IntoIterator<Item = (String, String)>,
    ) -> Result<AppConfig, Box<dyn std::error::Error>> {
        let builder =
            EnvLoader::from_pairs(pairs).map_err(|unknown| std::io::Error::other(unknown.0))?;
        Ok(validate(builder)?)
    }

    fn config_with_consumer() -> Result<AppConfig, Box<dyn std::error::Error>> {
        load_app_config([
            (
                "SITOLO__EVENTS__RELAY__AWS_REGION".into(),
                REGION.to_owned(),
            ),
            (
                "SITOLO__EVENTS__RELAY__SNS_TOPIC_ARN".into(),
                "arn:aws:sns:af-south-1:123456789012:sitolo-iam-events-prod.fifo".into(),
            ),
            (
                "SITOLO__WORKER__DATABASE__PASSWORD_REF".into(),
                "development/sitolo/worker".into(),
            ),
            (
                "SITOLO__EVENTS__CONSUMER__NAME".into(),
                "iam-projection".into(),
            ),
            (
                "SITOLO__EVENTS__CONSUMER__SQS_QUEUE_URL".into(),
                SOURCE.into(),
            ),
            (
                "SITOLO__EVENTS__CONSUMER__SQS_QUARANTINE_QUEUE_URL".into(),
                QUARANTINE.into(),
            ),
        ])
    }

    fn config_relay_only() -> Result<AppConfig, Box<dyn std::error::Error>> {
        load_app_config([
            (
                "SITOLO__EVENTS__RELAY__AWS_REGION".into(),
                REGION.to_owned(),
            ),
            (
                "SITOLO__EVENTS__RELAY__SNS_TOPIC_ARN".into(),
                "arn:aws:sns:af-south-1:123456789012:sitolo-iam-events-prod.fifo".into(),
            ),
            (
                "SITOLO__WORKER__DATABASE__PASSWORD_REF".into(),
                "development/sitolo/worker".into(),
            ),
        ])
    }

    #[derive(Default)]
    struct TestStubEffect {
        applications: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl ConsumerBusinessEffect for TestStubEffect {
        async fn apply(
            &self,
            _tx: &mut sqlx_core::transaction::Transaction<'_, sqlx_postgres::Postgres>,
            _event: &ConsumerEvent,
        ) -> Result<(), ConsumerEffectFailure> {
            self.applications.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestRecordingTransport {
        received: AtomicUsize,
        acknowledged: AtomicUsize,
        quarantined: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl ConsumerTransport for TestRecordingTransport {
        async fn receive_batch(
            &self,
        ) -> Result<Vec<crate::consumer::ReceivedMessage>, ConsumerTransportError> {
            self.received.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }

        async fn acknowledge(&self, _receipt_handle: &str) -> Result<(), ConsumerTransportError> {
            self.acknowledged.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn extend_visibility(
            &self,
            _receipt_handle: &str,
            _visibility_timeout: Duration,
        ) -> Result<(), ConsumerTransportError> {
            Ok(())
        }

        async fn quarantine(
            &self,
            _message: &crate::consumer::ReceivedMessage,
            _error_class: &OutboxErrorClass,
        ) -> Result<(), ConsumerTransportError> {
            self.quarantined.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn lazy_pool() -> Result<PgPool, sqlx_core::error::Error> {
        PgPoolOptions::new().connect_lazy("postgres://app_worker:unused@127.0.0.1/sitolo")
    }

    fn valid_class() -> Result<OutboxErrorClass, Box<dyn std::error::Error>> {
        // `invalid_event` is the fail-closed class the consumer applies to
        // permanently malformed events; its grammar is checked on construction.
        Ok(OutboxErrorClass::new("invalid_event")?)
    }

    #[test]
    fn from_config_returns_none_when_only_the_relay_is_configured()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = config_relay_only()?;
        assert!(ConsumerDestination::from_config(&config)?.is_none());
        Ok(())
    }

    #[test]
    fn from_config_returns_a_validated_destination_when_queues_are_configured()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = config_with_consumer()?;
        let destination = ConsumerDestination::from_config(&config)?.ok_or("destination")?;
        assert_eq!(destination.consumer_name(), "iam-projection");
        Ok(())
    }

    #[tokio::test]
    async fn composition_does_not_activate_the_consumer_or_apply_the_effect()
    -> Result<(), Box<dyn std::error::Error>> {
        let destination =
            ConsumerDestination::from_config(&config_with_consumer()?)?.ok_or("destination")?;
        let transport = Arc::new(TestRecordingTransport::default());
        let applications = Arc::new(AtomicUsize::new(0));
        let effect = Arc::new(TestStubEffect {
            applications: Arc::clone(&applications),
        });
        let dyn_transport: Arc<dyn ConsumerTransport> =
            Arc::<TestRecordingTransport>::clone(&transport);
        let _processor = compose_consumer(
            &destination,
            dyn_transport,
            lazy_pool()?,
            effect,
            valid_class()?,
        )?;
        assert_eq!(transport.received.load(Ordering::SeqCst), 0);
        assert_eq!(transport.acknowledged.load(Ordering::SeqCst), 0);
        assert_eq!(applications.load(Ordering::SeqCst), 0);
        Ok(())
    }

    #[tokio::test]
    async fn composed_processor_does_not_expose_queue_credentials_in_debug_output()
    -> Result<(), Box<dyn std::error::Error>> {
        let destination =
            ConsumerDestination::from_config(&config_with_consumer()?)?.ok_or("destination")?;
        let processor = compose_consumer(
            &destination,
            Arc::new(TestRecordingTransport::default()),
            lazy_pool()?,
            Arc::new(TestStubEffect::default()),
            valid_class()?,
        )?;
        let rendered = format!("{processor:?}");
        assert!(!rendered.contains(SOURCE));
        assert!(!rendered.contains(QUARANTINE));
        assert!(rendered.contains("iam-projection"));
        Ok(())
    }

    fn destination_with_name(name: &str) -> Result<ConsumerDestination, ConsumerDestinationError> {
        // The configuration validator rejects these names before the processor
        // would see them; this exercises the defence-in-depth boundary inside
        // `compose_consumer` directly.
        let config =
            SqsFifoConsumerConfig::new(REGION, SOURCE, QUARANTINE, CONSUMER_REQUEST_TIMEOUT)
                .map_err(ConsumerDestinationError::InvalidDestination)?;
        Ok(ConsumerDestination {
            config,
            consumer_name: name.to_owned(),
        })
    }

    #[tokio::test]
    async fn composition_rejects_consumer_names_the_processor_rejects()
    -> Result<(), Box<dyn std::error::Error>> {
        for name in ["", "IAM", "iam projection", &"a".repeat(65)] {
            let composition = compose_consumer(
                &destination_with_name(name)?,
                Arc::new(TestRecordingTransport::default()),
                lazy_pool()?,
                Arc::new(TestStubEffect::default()),
                valid_class()?,
            );
            assert!(
                matches!(composition, Err(ComposeError::InvalidConsumerName)),
                "consumer name {name:?} must be rejected"
            );
        }
        Ok(())
    }
}
