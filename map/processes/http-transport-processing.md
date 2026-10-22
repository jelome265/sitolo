---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@ee6a51c64e98ec14460ef61ec116bd61f153c106
source: apps/api/src/serve.rs
source_citation: apps/api/src/serve.rs:24-45,53-93,105-143
---

# HTTP Transport Processing

## Input

Accepted TCP connection and bounded application state.

## Movement

1. Accept connections under the configured in-flight semaphore. (apps/api/src/serve.rs:28-45)
2. Read request bytes under a five-second timeout into a 32 KiB bounded buffer. (apps/api/src/serve.rs:53-70)
3. Extract the Authorization header and parse the first request line. The token is not validated by this transport. (apps/api/src/serve.rs:71-88)
4. Dispatch liveness/readiness probes; protected organization POSTs fail closed with 401 because verified OIDC, session, membership, and permission composition is absent. Other paths return 404. (apps/api/src/serve.rs:105-143)
5. Build the bounded HTTP response, write it, and close the connection. (apps/api/src/serve.rs:89-93)

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

Verified against the pinned source revision. This describes transport behavior only. It does not imply authenticated business mutations are available or that every product mutation is wired end-to-end.

## See

apps/api/src/serve.rs
