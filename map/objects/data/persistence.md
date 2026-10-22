---
type: object
status: verified
universe: live
cluster: data
source_revision: feat/phase4-pr009-audit-outbox@754b0685a2b0869b6c695ec7659d1d788b728c29
source: crates/sitolo-persistence/src/postgres.rs
source_citation: crates/sitolo-persistence/src/postgres.rs:45-77
---
# Persistence


## Why this shape

Persistence is the durable authority boundary where transactional business truth and database security controls meet.

## Shape

Primary implementation: `crates/sitolo-persistence`. PostgreSQL authority/runtime primitives and the Part 8 tenant-scoped organization, membership, invitation, and role-assignment schema exist. The complete Phase 5 business schema remains phase-gated. Application role-assignment writes couple PostgreSQL state with mandatory audit and outbox evidence in one transaction.

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

