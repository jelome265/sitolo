---
type: process
status: verified
universe: live
source_revision: feat/phase4-pr009-audit-outbox@b7de7eb0bee7209b8526aa0017b2416efeea53fd
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
4. Verify the connected login/current role, restricted privilege flags, absence of memberships, schema presence, and required identity, tenancy, audit, and outbox objects. The identity audit table must be owned by `app_identity_owner`, and `app_runtime` must have no direct table privileges; close and fail closed on mismatch. (crates/sitolo-persistence/src/runtime_pool.rs:189-232)

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

Source-verified against branch commit `b7de7eb0bee7209b8526aa0017b2416efeea53fd`. API startup composes this pool only after resolving the secret, probing the configured Cognito issuer, and validating production/staging configuration. The new identity-audit owner and privilege checks require exact-source PostgreSQL-enabled GitHub Actions; those checks are pending for the current documentation update.

## See

crates/sitolo-persistence/src/runtime_pool.rs
