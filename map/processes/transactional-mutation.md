---
type: process
status: stub
universe: ghost
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: crates/sitolo-persistence/src/postgres.rs; docs/database_design.md; docs/phase4_part8_audit_outbox_implementation_contract.md
source_citation: crates/sitolo-persistence/src/postgres.rs:43
---

# Transactional Business Mutation

## Input

validated business command.

## Movement

1. Resolve the authoritative transaction boundary for the command. (docs/database_design.md)
2. Lock or conditionally update authoritative rows where the business invariant requires it. (docs/database_design.md)
3. Apply the domain state transition through application/domain ownership. (docs/domain_model.md)
4. Register required audit and outbox evidence in the same transaction when the contract requires atomicity. (docs/phase4_part8_audit_outbox_implementation_contract.md)
5. Commit authoritative PostgreSQL state. (crates/sitolo-persistence/src/postgres.rs:43-44)
6. Perform external network work after the durable transaction boundary. (docs/system_architecture_design.md)

## Output

Committed PostgreSQL authority and durable follow-up work.

## Consumes

[Domain Core](../objects/domain/domain-core.md) · [Tenancy](../objects/security/tenancy.md) · [Authorization](../objects/security/authorization.md)

## Produces

[Persistence](../objects/data/persistence.md) · [Events and Audit](../objects/reliability/events-and-audit.md)

## If you change this

### Hits

Domain, persistence, constraints, concurrency, idempotency, audit and workers.

### Does not hit

Pure presentation logic.

## Verification

Ghost/unwired as a complete product process: persistence transaction primitives exist, but the repository does not yet provide a real business mutation wired atomically to audit and outbox.

## See

docs/database_design.md; docs/phase4_part8_audit_outbox_implementation_contract.md