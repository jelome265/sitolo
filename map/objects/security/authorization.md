---
type: object
status: verified
universe: live
cluster: security-domain
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: crates/sitolo-authz/src/lib.rs
source_citation: crates/sitolo-authz/src/lib.rs:14
---
# Authorization


## Why this shape

Authorization converts authenticated identity and scoped authority into an explicit allow or deny decision.

## Shape

Primary implementation: `crates/sitolo-authz`. Phase 6 owns the generalized policy-enforcement expansion.

## Connected to

Authentication, tenancy, memberships, roles, permissions, entitlements, domain operations and audit evidence.

## If you change this

### Hits

Permission resolution, scope enforcement, protected operations and security-test coverage.

### Does not hit

Authentication credential establishment itself.

## Surfaces

Application services, API handlers, persistence boundaries and security tests.

## See

crates/sitolo-authz/src/lib.rs

