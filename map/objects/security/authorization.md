---
type: object
status: verified
universe: live
cluster: security-domain
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
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

