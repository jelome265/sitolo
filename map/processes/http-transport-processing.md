---
type: process
status: verified
universe: live
source_revision: branch@58de49757bffe99af69cd79968390df6b7654577
source: apps/api/src/serve.rs
source_citation: apps/api/src/serve.rs:45-115
---

# HTTP Transport Processing

## Input

Tokio runtime listener and bounded application state enter the Axum router boundary.

## Movement

1. Bind the asynchronous `tokio::net::TcpListener` in the API process. (apps/api/src/main.rs:24-40)
2. Build the production `axum::Router` with explicit health and tenancy routes. (apps/api/src/serve.rs:45-60)
3. Enforce request-body, body-idle, request-duration, and in-flight concurrency limits through Tower/Tower-HTTP layers. (apps/api/src/serve.rs:61-76)
4. Extract paths and JSON bodies with Axum and dispatch typed commands to the tenancy application handlers. (apps/api/src/serve.rs:118-225)
5. Run the Axum server on the Tokio listener and perform bounded graceful shutdown. (apps/api/src/serve.rs:78-99)
6. Exercise the same production router in integration tests through an in-process Axum request, not a parallel transport parser. (apps/api/src/serve.rs:264-309)
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

Verified against the current transport implementation. This is the executable transport movement; it does not imply that every product business mutation is wired end-to-end.

## See

apps/api/src/serve.rs
