---
type: process
status: stub
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: docs/offline-sync.md
---

# Offline Command Synchronization

- type: process
- status: stub
- source: docs/sync_protocol.md; docs/system_architecture_design.md

## Input

Durable client-side command generated offline.

## Movement

1. Persist the command locally.
2. Resume after reconnect.
3. Authenticate and authorize against current server authority.
4. Apply an idempotent server mutation where accepted.
5. Record authoritative result and convergence state.
6. Retry only within bounded policy.

## Output

Server-authoritative result with explicit local convergence state.

## If you change this

### Hits

Sync, authorization, tenancy, persistence, idempotency and client state.

### Does not hit

Static reporting views that do not consume synchronized state.

## See

docs/sync_protocol.md

