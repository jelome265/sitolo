---
type: process
status: verified
universe: live
source_revision: branch@3020c5c055465a31f7cbf69f71a00c8bf2e575f4
source: apps/api/src/serve.rs; apps/api/src/telemetry.rs
source_citation: apps/api/src/serve.rs:129-415
---

# HTTP Transport Processing

## Input

Tokio runtime/listener and bounded application state enter the Axum router; Hyper-util provides configured protocol transport over the Tokio socket.

## Movement

1. Bind the asynchronous `tokio::net::TcpListener` in the API process, only after startup validation has produced the application state. (apps/api/src/main.rs:29-40)
2. Build the production `axum::Router` with explicit health, readiness and tenancy routes. (apps/api/src/serve.rs:133-160)
3. Wrap the routes in Tower layers, innermost first: response-body idle timeout, total request deadline, request-body idle timeout, body-size limit, and a shared in-flight request ceiling; then an Axum middleware that records request telemetry; then the outermost layer that assigns one request identity (a client `x-request-id` when well formed, otherwise a server one), accepts only a single valid `traceparent`, and opens the request span. (apps/api/src/serve.rs:162-258)
4. Enforce a separate TCP connection ceiling with a Tokio semaphore (connections over it are dropped), configure TCP keepalive, Hyper HTTP/1 header timeout and HTTP/2 PING and stream limits, wrap the socket in an idle-eviction reader that resets on every read, and adapt the Axum Tower service with `TowerToHyperService`. (apps/api/src/serve.rs:299-366)
5. Extract paths and JSON bodies with Axum and dispatch typed commands to the tenancy application handlers; oversized bodies are classified 413 and errors carry the request identity assigned in step 3. (apps/api/src/serve.rs:590-683)
6. After the response is produced, offer one registered `http.request.completed` record to a bounded, priority-aware export queue; probes are lowest priority, and a record that cannot be admitted is shed and counted without blocking or altering the request. (apps/api/src/serve.rs:516-548)
7. Drain the export queue on a background loop into the structured-log sink, reporting shedding, queue-state changes and export failures once per interval. (apps/api/src/telemetry.rs:154-180)
8. On shutdown: flip readiness to draining, close the listener so late connections are refused, ask every Hyper connection to drain gracefully within the drain deadline and abort the rest, then flush the telemetry queue within a bound. (apps/api/src/serve.rs:305-415)

## Output

HTTP response bytes at the API transport boundary, and one structured completion record per request.

## Consumes

[API Boundary](../objects/runtime/api-boundary.md) · [Configuration](../objects/platform/configuration.md)

## Produces

[API Boundary](../objects/runtime/api-boundary.md)

## If you change this

### Hits

HTTP parsing, protocol bounds, request limits, handler dispatch, request identity and trace context, request telemetry, transport security and shutdown behavior.

### Does not hit

PostgreSQL transaction semantics when the request handler does not cross that boundary.

## Verification

Verified against the current transport implementation. Real-socket integration tests exercise HTTP/1 persistence and idle eviction, slow headers and bodies, oversized bodies, the connection ceiling, HTTP/2 multiplexing, and graceful shutdown; request-level tests drive the production router. This is the executable transport movement; it does not imply that every product business mutation is wired end-to-end.

## See

apps/api/src/serve.rs
