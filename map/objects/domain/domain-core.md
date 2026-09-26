---
type: object
status: verified
universe: live
cluster: domain
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
source: crates/sitolo-domain/src/lib.rs
source_citation: crates/sitolo-domain/src/lib.rs:11
---
# Domain Core


## Why this shape

The domain core owns typed business concepts and invariants instead of transport or persistence mechanics.

## Shape

Primary implementation: `crates/sitolo-domain`. Current main contains tenancy domain state; wider business engines remain phase-gated.

## Connected to

Application services, tenancy, authorization, persistence and business-domain contracts.

## If you change this

### Hits

Aggregate invariants, lifecycle transitions and downstream persistence semantics.

### Does not hit

HTTP formatting or deployment configuration that does not alter domain rules.

## Surfaces

Application services, tests and future catalogue/inventory/sales engines.

## See

crates/sitolo-domain/src/lib.rs

