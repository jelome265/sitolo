---
type: object
status: stub
universe: ghost
cluster: runtime
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
source: apps/worker/src/main.rs
source_citation: apps/worker/src/main.rs:10
---
# Worker Runtime


## Why this shape

The worker boundary exists to isolate durable asynchronous side effects from request/transaction handling.

## Shape

Primary executable: `apps/worker/src/main.rs`. Current implementation is a Phase 1 scaffold; durable worker execution is a later requirement.

## Connected to

Outbox/events, integrations, retries, observability and deployment.

## If you change this

### Hits

Asynchronous execution, retry policy, worker identity and operational recovery.

### Does not hit

Synchronous API authorization semantics unless the worker duplicates those rules.

## Surfaces

Worker process, CI integration tests and future outbox relay.

## See

apps/worker/src/main.rs

