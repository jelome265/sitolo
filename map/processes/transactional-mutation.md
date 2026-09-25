---
type: process
status: stub
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: docs/transactional-mutation.md
---

# Transactional Business Mutation

- type: process
- status: stub
- source: docs/system_architecture_design.md; docs/database_design.md; docs/domain_model.md

## Input

Validated business command.

## Movement

1. Validate domain invariants.
2. Open the required transaction.
3. Lock or conditionally update authoritative rows as required.
4. Apply the state transition.
5. Register required audit and outbox evidence inside the transaction.
6. Commit atomically.
7. Perform external network work outside the long-lived transaction.

## Output

Committed PostgreSQL authority and durable follow-up work.

## If you change this

### Hits

Domain, persistence, constraints, concurrency, idempotency, audit and workers.

### Does not hit

Pure presentation logic.

## See

docs/database_design.md

