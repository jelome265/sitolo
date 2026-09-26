---
type: process
status: verified
universe: live
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
source: crates/sitolo-persistence/src/postgres.rs
source_citation: crates/sitolo-persistence/src/postgres.rs:38-61
---

# PostgreSQL Authority Initialization

## Input

Administrative and least-privileged runtime PostgreSQL connection options.

## Movement

1. Build the administrative pool with bounded connection capacity. (crates/sitolo-persistence/src/postgres.rs:38-52)
2. Build the least-privileged runtime pool with the same bounded pool policy. (crates/sitolo-persistence/src/postgres.rs:53-58)
3. Return both pools as a single authority pair for downstream persistence operations. (crates/sitolo-persistence/src/postgres.rs:59-61)

## Output

Initialized PostgreSQL authority pools with separate setup and runtime identities.

## Consumes

[Persistence](../objects/data/persistence.md) · [Security Controls](../objects/security/security-controls.md)

## Produces

[Persistence](../objects/data/persistence.md)

## If you change this

### Hits

Database connection behavior, privilege separation, pool sizing and startup failure handling.

### Does not hit

Business transaction semantics inside later repository/application operations.

## Verification

Verified against the current PostgreSQL authority implementation. This movement establishes persistence authority; it does not claim that the complete business mutation/outbox flow is executable.

## See

crates/sitolo-persistence/src/postgres.rs
