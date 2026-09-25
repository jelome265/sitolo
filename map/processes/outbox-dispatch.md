---
type: process
status: stub
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: docs/outbox-dispatch.md
---

# Outbox Dispatch

- type: process
- status: stub
- source: docs/system_architecture_design.md; docs/phase4_part8_audit_outbox_implementation_contract.md

## Input

Durable outbox record created with authoritative transaction state.

## Movement

1. Worker claims pending work.
2. Applies bounded retry policy.
3. Performs the external or derived side effect.
4. Records completion, failure or unknown outcome.
5. Preserves idempotency across duplicate delivery.

## Output

Durable side-effect state and operational evidence.

## If you change this

### Hits

Events and outbox, worker runtime, integrations, observability, retries and idempotency.

### Does not hit

Unrelated read-only endpoints.

## See

docs/phase4_part8_audit_outbox_implementation_contract.md

