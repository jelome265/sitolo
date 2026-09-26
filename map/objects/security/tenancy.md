---
type: object
status: verified
universe: live
cluster: security-domain
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
source: crates/sitolo-tenancy/src/scope.rs
source_citation: crates/sitolo-tenancy/src/scope.rs:113
---
# Tenancy


## Why this shape

Tenant, organization and branch scope are security boundaries used to constrain every tenant-owned operation.

## Shape

Primary implementation: `crates/sitolo-tenancy` with domain tenancy types under `crates/sitolo-domain`.

## Connected to

Authentication, memberships, authorization, API, PostgreSQL/RLS and business resources.

## If you change this

### Hits

Scope resolution, cross-tenant isolation, branch authorization and persistence predicates.

### Does not hit

Unrelated product calculations that consume an unchanged authorized scope.

## Surfaces

Application authorization context, persistence and security tests.

## See

crates/sitolo-tenancy/src/lib.rs

