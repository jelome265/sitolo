//! Sitolo worker binary.
//!
//! Owns durable-job execution: validation, application operation, transaction,
//! external side effect when applicable, durable result, and retry/DLQ
//! classification. Business rules must not be duplicated with the API path.

#![forbid(unsafe_code)]

#[tokio::main]
async fn main() {
    tracing::info!("Sitolo outbox worker initialized.");
}
