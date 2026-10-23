---
type: object
status: stub
universe: ghost
cluster: runtime
source_revision: feat/phase4-pr009-audit-outbox@2048c8a6a2a216d466240e1e1a1f5f99dae111d7
source: apps/worker/src/lib.rs
source_citation: apps/worker/src/lib.rs:4
---
# Worker Runtime


## Why this shape

The worker boundary exists to isolate durable asynchronous side effects from request/transaction handling.

## Shape

The transport-neutral outbox engine lives in `apps/worker/src/lib.rs` and `apps/worker/src/relay.rs`; it recovers stale claims, validates registered IAM event envelopes, delegates publishing to an adapter, and persists token-fenced outcomes with bounded retry/quarantine behavior. `apps/worker/src/main.rs` fails closed because no approved transport adapter or configuration exists. The engine is not composed with PostgreSQL credentials or a destination, so this object remains a stub/ghost rather than a live worker process.

## Connected to

Outbox/events, integrations, retries, observability and deployment.

## If you change this

### Hits

Asynchronous execution, retry policy, worker identity and operational recovery.

### Does not hit

Synchronous API authorization semantics unless the worker duplicates those rules.

## Surfaces

Worker relay engine tests, future destination adapter, runtime configuration, and deployment composition.

## See

apps/worker/src/lib.rs
apps/worker/src/relay.rs
apps/worker/src/main.rs

