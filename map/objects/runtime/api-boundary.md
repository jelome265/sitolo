---
type: object
status: verified
universe: live
cluster: runtime
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: crates/sitolo-api/src/lib.rs
source_citation: crates/sitolo-api/src/lib.rs:10
---
# API Boundary


## Why this shape

The API boundary converts untrusted transport input into bounded application operations without owning domain truth.

## Shape

Primary implementation: `crates/sitolo-api`. The executable transport is assembled under `apps/api`.

## Connected to

Authentication, tenancy, authorization, domain/application services, persistence and observability.

## If you change this

### Hits

Request validation, route behavior, trust boundaries, public errors and application dispatch.

### Does not hit

Database schema semantics unless the API contract itself changes.

## Surfaces

Public HTTP transport, integration tests and API-focused security tests.

## See

crates/sitolo-api/src/lib.rs

