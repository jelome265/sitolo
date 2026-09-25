---
type: process
status: stub
universe: ghost
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: apps/worker/src/main.rs; crates/sitolo-events/src/lib.rs; docs/phase4_part8_audit_outbox_implementation_contract.md
source_citation: apps/worker/src/main.rs:10
---

# Outbox Dispatch

## Input

durable outbox record created with authoritative transaction state.

## Movement

1. Worker claims pending work using the lease/claim contract. (docs/phase4_part8_audit_outbox_implementation_contract.md)
2. Apply bounded retry and quarantine policy. (docs/phase4_part8_audit_outbox_implementation_contract.md)
3. Perform the external or derived side effect outside the originating transaction. (docs/system_architecture_design.md)
4. Record completion, failure, or unknown outcome. (docs/phase4_part8_audit_outbox_implementation_contract.md)
5. Preserve idempotency across duplicate delivery. (docs/phase4_part8_audit_outbox_implementation_contract.md)

## Output

Durable side-effect state and operational evidence.

## Consumes

[Events and Audit](../objects/reliability/events-and-audit.md) · [Worker Runtime](../objects/runtime/worker-runtime.md)

## Produces

[Integrations](../objects/integration/integrations.md) · [Observability](../objects/platform/observability.md)

## If you change this

### Hits

Events and outbox, worker runtime, integrations, observability, retries and idempotency.

### Does not hit

Unrelated read-only endpoints.

## Verification

Ghost/unwired: the worker binary remains an empty Phase 1 scaffold and the events crate is contract-only, so the end-to-end relay does not currently run.

## See

docs/phase4_part8_audit_outbox_implementation_contract.md; apps/worker/src/main.rs:10