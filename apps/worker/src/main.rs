//! Sitolo worker binary.
//!
//! Initializes configuration, connects to PostgreSQL with app_worker authority,
//! runs the outbox relay, and shuts down gracefully.

#![forbid(unsafe_code)]

use sitolo_events::{InMemoryDispatcher, OutboxRelayMetrics, RelayConfig};
use sitolo_persistence::audit_outbox::PgAuditOutboxStore;
use sitolo_worker::OutboxRelayRunner;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("Starting Sitolo outbox worker...");

    let worker_url = std::env::var("WORKER_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("WORKER_DATABASE_URL or DATABASE_URL must be set");

    let opts: sqlx::postgres::PgConnectOptions = worker_url.parse()?;
    let worker_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await?;

    let store = Arc::new(PgAuditOutboxStore::new(worker_pool.clone(), worker_pool.clone()));
    let dispatcher = Arc::new(InMemoryDispatcher::new());
    let metrics = Arc::new(OutboxRelayMetrics::new());
    let config = RelayConfig {
        batch_size: 10,
        lease_duration: Duration::from_secs(60),
        max_attempts: 5,
        backoff_duration: Duration::from_secs(10),
    };

    let runner = OutboxRelayRunner::new(store, dispatcher, metrics.clone(), config);

    info!("Worker started. Polling for outbox events...");

    let mut interval = tokio::time::interval(Duration::from_secs(5));
    let mut shutdown = tokio::signal::ctrl_c();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let now = std::time::SystemTime::now();
                match runner.process_batch(now).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Processed {} outbox events", count);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error processing outbox batch: {}", e);
                    }
                }
            }
            _ = &mut shutdown => {
                info!("Shutdown signal received, stopping worker...");
                break;
            }
        }
    }

    worker_pool.close().await;
    info!("Worker shut down gracefully.");

    Ok(())
}
