---
type: object
status: verified
universe: live
cluster: reliability
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
source: crates/sitolo-audit/src/lib.rs
source_citation: crates/sitolo-audit/src/lib.rs:14
---
# Events and Audit


## Why this shape

Audit evidence and event intent preserve accountability and support durable side effects without making telemetry the source of truth.

## Shape

Current audit boundary: `crates/sitolo-audit`. Event/outbox boundary is defined under `crates/sitolo-events`; durable Part 8 relay remains under the active Phase 4 Part 8 contract.

## Connected to

Authorization, persistence, workers, integrations and observability.

## If you change this

### Hits

Audit semantics, outbox contracts, transaction coupling and asynchronous delivery evidence.

### Does not hit

Core domain meaning unless the event contract changes the represented business fact.

## Surfaces

Application transaction boundaries, workers and security/operational evidence.

## See

crates/sitolo-audit/src/lib.rs

