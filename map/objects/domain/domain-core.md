# Domain Core

- type: object
- status: verified
- source revision: Sitolo main at e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
- cluster: domain

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
