---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@382de95eeb8f9f0622d022f304feb7dd7a5db802
source: crates/sitolo-persistence/src/runtime_pool.rs
source_citation: crates/sitolo-persistence/src/runtime_pool.rs:139-293,297-304,374-384
---

# PostgreSQL Runtime Authority Initialization

## Input

Validated database runtime configuration, a database credential resolved through the configured secret provider, and an explicit TLS requirement.

## Movement

1. Reject any role other than `app_runtime` and any schema outside the validated lowercase PostgreSQL identifier form. (crates/sitolo-persistence/src/runtime_pool.rs:156-161,374-384)
2. Construct PostgreSQL options from configured host, port, username, password, database, TLS mode, application name, and schema search path, then drop the resolved password after options are built. (crates/sitolo-persistence/src/runtime_pool.rs:163-177)
3. Open a bounded pool with configured minimum/maximum connections and acquire timeout, plus explicit connection lifetime and idle timeout. (crates/sitolo-persistence/src/runtime_pool.rs:179-187)
4. Verify the connected login/current role, restricted privilege flags, absence of memberships, schema presence, and required identity, tenancy, audit, and outbox objects, including the owner-bound audited `revoke_sitolo_session` function. The identity audit table must be owned by `app_identity_owner`, and `app_runtime` must have no direct table privileges; close and fail closed on mismatch. (crates/sitolo-persistence/src/runtime_pool.rs:189-232)

## Output

A PostgreSQL pool authenticated directly as restricted `app_runtime`, pinned to the configured schema, and returned only after the required schema and role checks pass.

## Consumes

[Persistence](../objects/data/persistence.md) · [Security Controls](../objects/security/security-controls.md)

## Produces

[Persistence](../objects/data/persistence.md)

## If you change this

### Hits

Runtime database identity, schema selection, TLS verification, pool behavior, and API readiness.

### Does not hit

Administrative database provisioning, migrations, worker database identity, or business transaction semantics.

## Verification

Source-verified against implementation commit `382de95eeb8f9f0622d022f304feb7dd7a5db802`. API startup composes this pool only after resolving the secret, probing the configured Cognito issuer, and validating production/staging configuration. The runtime pool and readiness probe now require the owner-bound `revoke_sitolo_session` function. Exact-head PostgreSQL catalog and failure-injection checks are pending for the implementation commit; no local PostgreSQL was started.

## See

crates/sitolo-persistence/src/runtime_pool.rs
