---
type: object
status: stub
universe: ghost
cluster: integration
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: crates/sitolo-integrations/src/lib.rs
source_citation: crates/sitolo-integrations/src/lib.rs:1
---
# Integrations


## Why this shape

External providers are failure-prone trust boundaries and must remain adapters rather than becoming sources of Sitolo business truth.

## Shape

Primary boundary: `crates/sitolo-integrations`. Provider contracts live in `docs/payment_integration_spec.md` and `docs/mra_eis_integration_spec.md`.

## Connected to

Workers, outbox, payments, MRA EIS, observability and reconciliation.

## If you change this

### Hits

Provider authentication, callback validation, retry/idempotency and operational failure handling.

### Does not hit

Internal business truth unless an explicit integration contract changes.

## Surfaces

Worker adapters, callback endpoints, provider sandboxes and production credentials.

## See

crates/sitolo-integrations/src/lib.rs

