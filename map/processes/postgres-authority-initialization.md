---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@505da2fdc8e36b8871bcf19c85d7db5c846e47a5
source: crates/sitolo-persistence/src/postgres.rs
source_citation: crates/sitolo-persistence/src/postgres.rs:45-76
---

# PostgreSQL Authority Initialization

## Input

Administrative and runtime PostgreSQL connection options; the runtime options must authenticate as the least-privileged `app_runtime` login.

## Movement

1. Build the administrative pool with bounded connection capacity. (crates/sitolo-persistence/src/postgres.rs:62-65)
2. Build the bounded runtime pool from options that authenticate as the restricted `app_runtime` login; the pool does not issue `SET ROLE`. (crates/sitolo-persistence/src/postgres.rs:67-70)
3. Return both pools as a single authority pair for downstream persistence operations. (crates/sitolo-persistence/src/postgres.rs:72-76)

## Output

Initialized PostgreSQL authority pools with administrative setup authority and a runtime session authenticated as the least-privileged `app_runtime` role.

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

Verified against the PostgreSQL authority and tenancy implementation at the pinned source revision. The runtime pool authenticates as `app_runtime` directly; `verify_runtime_role` asserts `current_user` and `session_user`, and the RLS integration suite proves that `RESET ROLE` does not change the identity and `SET ROLE postgres` is denied. The Part 8 metadata verifier checks forced tenant RLS and that role assignments permit only state and state-version updates, not table-wide or identity-column updates. Application role assignment and revocation couple PostgreSQL state with audit/outbox evidence in a transaction, with rollback and concurrency integration cases. Other IAM mutations remain incomplete; this process does not claim the complete business mutation/outbox flow is production-wired.

## See

crates/sitolo-persistence/src/postgres.rs
