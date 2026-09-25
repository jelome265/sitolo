# Events and Audit

- type: object
- status: verified
- source revision: Sitolo main at e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
- cluster: reliability

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
