---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@9aeed46280a1ce3921cd3284281e3c92276686aa
source: apps/api/src/serve.rs; apps/api/src/state.rs; apps/api/src/bootstrap.rs
source_citation: apps/api/src/serve.rs:68-200,279-427; apps/api/src/state.rs:53-133; apps/api/src/bootstrap.rs:128-275
---

# HTTP Transport Processing

## Input

Accepted TCP connection and the immutable runtime state assembled by process bootstrap.

## Movement

1. Build the live Axum router only after the process bootstrap has constructed the runtime state; route registration is checked against the reviewed route inventory. (apps/api/src/main.rs; apps/api/src/serve.rs:143-200)
2. Apply per-route and global body limits, request deadlines, a concurrency cap, and load shedding. (apps/api/src/serve.rs:143-216)
3. Expose public liveness and readiness probes; production readiness is published only after managed credentials, the verified database runtime, OIDC provider, persistent identity authority, and durable tenancy service have been composed. (apps/api/src/serve.rs:279-297; apps/api/src/bootstrap.rs:128-275)
4. Authenticate protected requests using bearer credentials and the `session-id` selector before JSON extraction. The organization-create body rejects caller-supplied authority fields and derives its owner from the verified principal; the PostgreSQL command writes organization, owner membership, branch, audit, and outbox together. (apps/api/src/serve.rs:299-393; crates/sitolo-application/src/tenancy.rs:1451-1558)
5. For branch creation, resolve persisted membership, role permission, and scope under the organization mutation lock before committing the branch and its audit/outbox evidence. Other protected routes remain closed until their commands are composed. (apps/api/src/serve.rs:395-427; crates/sitolo-application/src/tenancy.rs:1708-1798)
6. Drain Axum serving under the configured process deadline; process entry awaits and classifies the server result. (apps/api/src/serve.rs:218-243; apps/api/src/main.rs)

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

Source-verified against candidate `a1d29ae692f6ff5c59e48ef7135f514ae979b222`. Database-backed HTTP authorization and provisioning tests are executed only in the PostgreSQL-enabled GitHub Actions workflow; local compilation is not database evidence. Other lifecycle mutation routes remain closed, so this card does not claim the complete R2 route matrix.

## See

apps/api/src/serve.rs
