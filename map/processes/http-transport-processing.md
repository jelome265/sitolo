---
type: process
status: verified
universe: live
source_revision: branch@99fe18fe06816b88e8567187f62104637f0fb723
source: apps/api/src/serve.rs
source_citation: apps/api/src/serve.rs:45-115
---

# HTTP Transport Processing

## Input

Tokio runtime/listener and bounded application state enter the Axum router; Hyper-util provides configured protocol transport over the Tokio socket.

## Movement

1. Bind the asynchronous `tokio::net::TcpListener` in the API process. (apps/api/src/main.rs:24-40)
2. Build the production `axum::Router` with explicit health and tenancy routes. (apps/api/src/serve.rs:55-70)
3. Enforce request-body, body-idle, request-duration, and globally shared in-flight request limits through Tower/Tower-HTTP layers. (apps/api/src/serve.rs:61-76)
4. Enforce a separate TCP connection-task ceiling with a Tokio semaphore, then configure Hyper HTTP/1/HTTP/2 protocol limits and adapt the Axum Tower service with `TowerToHyperService`. (apps/api/src/serve.rs:118-225)
5. Extract paths and JSON bodies with Axum and dispatch typed commands to the tenancy application handlers. (apps/api/src/serve.rs:85-125)
6. Run the Hyper connection task through Tokio and perform bounded graceful shutdown; integration tests exercise the same production Axum router through an in-process request, not a parallel transport parser. (apps/api/src/serve.rs:264-309)

## Output

HTTP response bytes at the API transport boundary.

## Consumes

[API Boundary](../objects/runtime/api-boundary.md) · [Configuration](../objects/platform/configuration.md)

## Produces

[API Boundary](../objects/runtime/api-boundary.md)

## If you change this

### Hits

HTTP parsing, protocol bounds, request limits, handler dispatch, transport security and shutdown behavior.

### Does not hit

PostgreSQL transaction semantics when the request handler does not cross that boundary.

## Verification

Verified against the current transport implementation. This is the executable transport movement; it does not imply that every product business mutation is wired end-to-end.

## See

apps/api/src/serve.rs
