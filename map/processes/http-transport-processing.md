---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@884c375239a1faa437e014f7a859a9bf04201bd8
source: apps/api/src/serve.rs
source_citation: apps/api/src/serve.rs:59-109,133-200,245-299
---

# HTTP Transport Processing

## Input

Accepted TCP connection and bounded application state.

## Movement

1. Axum accepts connections from the already-bound Tokio listener and serves the reviewed route inventory. (apps/api/src/serve.rs:133-200)
2. Apply per-route and global request-body limits, request deadlines, a concurrency cap, and load shedding. (apps/api/src/serve.rs:139-176)
3. Expose liveness; readiness remains 503 until the runtime is explicitly marked ready after dependency composition. (apps/api/src/serve.rs:245-263)
4. Protected routes read the Bearer Authorization and `Session-ID` headers and call the composed identity/session authority. Missing or invalid identity is denied; a valid identity still receives dependency-unavailable while tenant authorization and command composition are absent. (apps/api/src/serve.rs:265-274; apps/api/src/state.rs)
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

Verified against the pinned source revision. Axum owns HTTP parsing; the prior hand-written single-read parser is removed. This does not claim an explicit HTTP header-read timeout, production provider composition, tenant authorization, or successful business mutation wiring. The protected path remains fail-closed.

## See

apps/api/src/serve.rs
