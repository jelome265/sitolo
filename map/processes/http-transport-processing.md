---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@98d49304c8318f708e79019d79449fe33f0069d3
source: apps/api/src/serve.rs; apps/api/src/state.rs; apps/api/src/bootstrap.rs
source_citation: apps/api/src/serve.rs:157-179,183-245,252-315,317-366,378-540; apps/api/src/state.rs:23-53,55-130; apps/api/src/bootstrap.rs:169-198,345-428
---

# HTTP Transport Processing

## Input

Accepted TCP connection and the immutable runtime state assembled by process bootstrap.

## Movement

1. Build the live Axum router only after the process bootstrap has constructed the runtime state; route registration is checked against the reviewed route inventory. (apps/api/src/main.rs; apps/api/src/serve.rs:157-179,272-315)
2. Apply per-route and global body limits, request deadlines, a concurrency cap, and load shedding. (apps/api/src/serve.rs:168-178,218-245)
3. Expose public liveness and readiness probes; production readiness is published only after managed credentials, the verified database runtime, OIDC provider, persistent identity authority, and durable tenancy service have been composed. (apps/api/src/serve.rs:317-335; apps/api/src/bootstrap.rs:169-198,354-428)
4. Authenticate protected requests using bearer credentials and the `session-id` selector before JSON extraction. The organization-create body rejects caller-supplied authority fields and derives its owner from the verified principal; the PostgreSQL command writes organization, owner membership, branch, audit, and outbox together. (apps/api/src/serve.rs:337-354,378-431; crates/sitolo-application/src/tenancy.rs:1053-1162)
5. For branch creation, branch activation, member suspension, and member revocation, resolve persisted membership, role permission, and scope under the organization mutation lock before committing the state change and its audit/outbox evidence. (apps/api/src/serve.rs:433-540; crates/sitolo-application/src/tenancy.rs:1240-1402,1568-1714)
6. Drain Axum serving under the configured process deadline; process entry awaits and classifies the server result. (apps/api/src/serve.rs:247-270; apps/api/src/main.rs)

## Output

HTTP response bytes at the API transport boundary.

## Consumes

[API Boundary](../objects/runtime/api-boundary.md) · [Configuration](../objects/platform/configuration.md)

## Produces

[API Boundary](../objects/runtime/api-boundary.md)

## If you change this

### Hits

HTTP parsing, request bounds, authentication/authorization dispatch, readiness, transport security, and shutdown behavior.

### Does not hit

Administrative database provisioning, migration execution, worker relay or consumer behavior.

## Verification

Source-verified against branch commit `98d49304c8318f708e79019d79449fe33f0069d3`. Database-backed HTTP authorization and provisioning tests are executed only in the PostgreSQL-enabled GitHub Actions workflow; local compilation is not database evidence. The declared route inventory is fully wired to authenticated handlers, but the database-backed proof for those commands remains CI-only.

## See

apps/api/src/serve.rs
