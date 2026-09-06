//! Sitolo worker binary.
//!
//! Owns durable-job execution: validation, application operation, transaction,
//! external side effect when applicable, durable result, and retry/DLQ
//! classification. Business rules must not be duplicated with the API path.
//!
//! This is a Phase 1 scaffold and is not production functionality.
#![forbid(unsafe_code)]

fn main() {}
