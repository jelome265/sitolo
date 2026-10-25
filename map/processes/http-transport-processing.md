---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@4858341fcd16459aad10494d8b75ddb684859861
source: apps/api/src/serve.rs; apps/api/src/state.rs; apps/api/src/bootstrap.rs
source_citation: apps/api/src/serve.rs:68-162,198-296,317-380,382-430,432-594; apps/api/src/state.rs:23-53,55-130; apps/api/src/bootstrap.rs:169-198,345-428
---

# HTTP Transport Processing

## Input

Accepted TCP connection and the immutable runtime state assembled by process bootstrap.

## Movement

1. Build the live Axum router only after the process bootstrap has constructed the runtime state; route registration is checked against the reviewed route inventory, and an undeclared protected route is a startup error. (apps/api/src/main.rs; apps/api/src/serve.rs:198-222,317-342)
2. Apply per-route and global body limits, request deadlines, a concurrency cap, and load shedding. (apps/api/src/serve.rs:264-315)
3. Expose public liveness and readiness probes; production readiness is published only after managed credentials, the verified database runtime, OIDC provider, persistent identity authority, and durable tenancy service have been composed. (apps/api/src/serve.rs:362-380; apps/api/src/bootstrap.rs:169-198,354-428)
4. Authenticate protected requests using bearer credentials and the `session-id` selector before JSON extraction. The organization-create body rejects caller-supplied authority fields and derives its owner from the verified principal; the PostgreSQL command writes organization, owner membership, branch, audit, and outbox together. (apps/api/src/serve.rs:382-485; crates/sitolo-application/src/tenancy.rs:1053-1162)
5. For branch creation, branch activation, member suspension, and member revocation, resolve persisted membership, role permission, and scope under the organization mutation lock before committing the state change and its audit/outbox evidence. (apps/api/src/serve.rs:487-594; crates/sitolo-application/src/tenancy.rs:1240-1402,1568-1714)
6. Drain Axum serving under the configured process deadline; process entry awaits and classifies the server result. (apps/api/src/serve.rs:297-315; apps/api/src/main.rs)

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

Source-verified against branch commit `4858341fcd16459aad10494d8b75ddb684859861`. Database-backed HTTP authorization and provisioning tests are executed only in the PostgreSQL-enabled GitHub Actions workflow; local compilation is not database evidence. All five declared protected routes are wired to authenticated handlers. Organization activation is not an HTTP endpoint: provisioning atomically creates an active organization with its owner membership and default branch, and incomplete provisioning is not exposed as a normal tenant. `apps/api/tests/axum_boundary.rs` proves every declared protected route rejects missing credentials and the removed activation path returns 404. The database-backed proof for the wired commands remains CI-only.

## See

apps/api/src/serve.rs
