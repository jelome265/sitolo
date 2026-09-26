---
type: object
status: stub
universe: ghost
cluster: continuity
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
source: crates/sitolo-sync/src/lib.rs
source_citation: crates/sitolo-sync/src/lib.rs:1
---
# Synchronization


## Why this shape

Synchronization defines how offline client commands return to server authority without allowing local state to become permanent business authority.

## Shape

Primary boundary: `crates/sitolo-sync`. The protocol is defined by `docs/sync_protocol.md`; the runtime sync engine remains phase-gated.

## Connected to

Mobile/desktop continuity, authentication, tenancy, authorization, domain transactions and audit evidence.

## If you change this

### Hits

Command identity, replay handling, reconciliation, offline security and client recovery.

### Does not hit

Server business truth itself unless reconciliation semantics change.

## Surfaces

Future clients, sync transport, integration tests and recovery tooling.

## See

crates/sitolo-sync/src/lib.rs

