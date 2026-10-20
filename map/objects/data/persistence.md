---
type: object
status: verified
universe: live
cluster: data
source_revision: feat/phase4-pr009-audit-outbox@bd2f60a5a2c24041905c1fe1555278094f059264
source: crates/sitolo-persistence/src/postgres.rs
source_citation: crates/sitolo-persistence/src/postgres.rs:45-77
---
# Persistence


## Why this shape

Persistence is the durable authority boundary where transactional business truth and database security controls meet.

## Shape

Primary implementation: `crates/sitolo-persistence`. PostgreSQL authority/runtime primitives exist; the complete Phase 5 business schema remains phase-gated.

## Connected to

Domain/application services, tenancy, RLS, audit/outbox, migrations and tests.

## If you change this

### Hits

Transactions, constraints, tenant isolation, repository semantics and migration compatibility.

### Does not hit

Pure transport formatting with no persistence contract change.

## Surfaces

PostgreSQL, repositories, integration tests and release verification.

## See

crates/sitolo-persistence/src/lib.rs

