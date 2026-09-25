---
type: process
status: stub
universe: ghost
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: crates/sitolo-sync/src/lib.rs; docs/sync_protocol.md
source_citation: crates/sitolo-sync/src/lib.rs:1
---

# Offline Command Synchronization

Input: durable client-side command generated offline.

## Movement

1. Persist the command locally as a durable intent before acknowledging offline success. (docs/sync_protocol.md)
2. Resume transmission after reconnect. (docs/sync_protocol.md)
3. Re-authenticate and authorize against current server authority. (docs/sync_protocol.md)
4. Apply an idempotent server mutation where accepted. (docs/sync_protocol.md)
5. Record the authoritative result and convergence state. (docs/sync_protocol.md)
6. Retry only inside bounded policy. (docs/sync_protocol.md)

## Output

Server-authoritative result with explicit local convergence state.

## Consumes

[Synchronization](../objects/continuity/synchronization.md) · [Authentication](../objects/security/authentication.md) · [Tenancy](../objects/security/tenancy.md) · [Authorization](../objects/security/authorization.md)

## Produces

[Persistence](../objects/data/persistence.md) · [Events and Audit](../objects/reliability/events-and-audit.md)

## If you change this

### Hits

Sync, authorization, tenancy, persistence, idempotency and client state.

### Does not hit

Static reporting views that do not consume synchronized state.

## Verification

Ghost/unwired: `crates/sitolo-sync` is currently a contract boundary without substantive runtime synchronization implementation.

## See

docs/sync_protocol.md