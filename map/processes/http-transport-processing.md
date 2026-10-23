---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@0644eb4ef6ab9d005c5def7b93f910afa3f4b924
source: apps/api/src/serve.rs
source_citation: apps/api/src/serve.rs:65-115,140-197,266-349; apps/api/src/state.rs:53-90
---

# HTTP Transport Processing

## Input

Accepted TCP connection and bounded application state.

## Movement

1. Axum accepts connections from the already-bound Tokio listener and serves the reviewed route inventory. (apps/api/src/serve.rs:133-200)
2. Apply per-route and global request-body limits, request deadlines, a concurrency cap, and load shedding. (apps/api/src/serve.rs:139-176)
3. Expose liveness; readiness remains 503 until the runtime is explicitly marked ready after dependency composition. (apps/api/src/serve.rs:245-263)
4. The branch-create route authenticates the bearer and `Session-ID` before typed JSON extraction. When an explicitly composed durable tenancy service exists, it checks current organization membership, role permission, and scope under the organization mutation lock, then commits the branch, audit record, and outbox event together. Missing durable service or denied authority fails closed. Other protected routes authenticate and return dependency-unavailable until their command paths are composed. (apps/api/src/serve.rs:163-180,286-349; apps/api/src/state.rs:67-90)
5. Drain Axum serving under the configured process deadline; process entry awaits and classifies the server result. (apps/api/src/serve.rs:184-200; apps/api/src/main.rs)

## Output

HTTP response bytes at the API transport boundary.

## Consumes

[API Boundary](../objects/runtime/api-boundary.md) · [Configuration](../objects/platform/configuration.md)

## Produces

[API Boundary](../objects/runtime/api-boundary.md)

## If you change this

### Hits

HTTP parsing, request bounds, handler dispatch, transport security and shutdown behavior.

### Does not hit

PostgreSQL transaction semantics when the request handler does not cross that boundary.

## Verification

The pinned source contains a fully authorized branch-create command path for runtimes that explicitly inject the durable tenancy service. The default production bootstrap still does not compose identity or tenancy dependencies, so it remains not ready and protected requests fail closed. Other protected mutations remain uncomposed. This card does not claim production identity-provider configuration or global runtime readiness.

## See

apps/api/src/serve.rs
