---
type: process
status: verified
universe: live
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
source: apps/api/src/serve.rs
source_citation: apps/api/src/serve.rs:24-84
---

# HTTP Transport Processing

## Input

Accepted TCP connection and bounded application state.

## Movement

1. Accept connections under the configured in-flight semaphore. (apps/api/src/serve.rs:28-45)
2. Read request bytes under a five-second timeout into a 32 KiB bounded buffer. (apps/api/src/serve.rs:52-63)
3. Separate headers/body and parse the first request line into method and path. (apps/api/src/serve.rs:64-82)
4. Dispatch the parsed request to the current transport handler and build the JSON response. (apps/api/src/serve.rs:83-91)
5. Write the bounded HTTP response and close the connection. (apps/api/src/serve.rs:92-98)

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
