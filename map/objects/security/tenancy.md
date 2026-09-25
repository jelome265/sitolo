# Tenancy

- type: object
- status: verified
- source revision: Sitolo main at e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
- cluster: security-domain

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
