---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@43d6e04161964fc771481a325c20b7d8192f9231
source: crates/sitolo-persistence/src/runtime_pool.rs
source_citation: crates/sitolo-persistence/src/runtime_pool.rs:43-154
---

# PostgreSQL Runtime Authority Initialization

## Input

Validated database runtime configuration, a database credential resolved through the configured secret provider, and an explicit TLS requirement.

## Movement

1. Reject any role other than `app_runtime` and any schema outside the validated lowercase PostgreSQL identifier form. (crates/sitolo-persistence/src/runtime_pool.rs:51-53, 157-165)
2. Construct PostgreSQL options from configured host, port, username, password, database, TLS mode, application name, and schema search path. (crates/sitolo-persistence/src/runtime_pool.rs:61-77)
3. Open a bounded pool with configured minimum/maximum connections and acquire timeout, plus explicit connection lifetime and idle timeout. (crates/sitolo-persistence/src/runtime_pool.rs:79-88)
4. Verify the connected login/current role, restricted privilege flags, absence of memberships, schema presence, and required identity, tenancy, audit, and outbox objects; close and fail closed on mismatch. (crates/sitolo-persistence/src/runtime_pool.rs:90-153)

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

Source-verified against candidate `43d6e04161964fc771481a325c20b7d8192f9231`. API startup composes this pool only after resolving the secret, probing the configured Cognito issuer, and validating production/staging configuration. Local workspace compilation and unit checks passed; database-backed startup and role verification require exact-source PostgreSQL-enabled GitHub Actions and are not claimed here.

## See

crates/sitolo-persistence/src/runtime_pool.rs
