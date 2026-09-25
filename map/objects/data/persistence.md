---
type: object
status: verified
cluster: data
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: crates/sitolo-persistence/src/postgres.rs
source_citation: crates/sitolo-persistence/src/postgres.rs:43
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

