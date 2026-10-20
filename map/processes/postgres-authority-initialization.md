---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@0b851583030687cc7d663289b995c63991224dc2
source: crates/sitolo-persistence/src/postgres.rs
source_citation: crates/sitolo-persistence/src/postgres.rs:41-73
---

# PostgreSQL Authority Initialization

## Input

Administrative and runtime PostgreSQL connection options; the runtime connection must be able to assume `app_runtime`.

## Movement

1. Build the administrative pool with bounded connection capacity. (crates/sitolo-persistence/src/postgres.rs:54-57)
2. Build the bounded runtime pool and execute `SET ROLE app_runtime` on every new connection. (crates/sitolo-persistence/src/postgres.rs:59-68)
3. Return both pools as a single authority pair for downstream persistence operations. (crates/sitolo-persistence/src/postgres.rs:70-73)

## Output

Initialized PostgreSQL authority pools with administrative setup authority and a runtime session assuming the least-privileged `app_runtime` role.

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

Verified against the current PostgreSQL authority implementation. The runtime pool explicitly assumes `app_runtime`; the RLS integration suite must verify the actual session identity. This movement does not claim that the complete business mutation/outbox flow is production-wired.

## See

crates/sitolo-persistence/src/postgres.rs
