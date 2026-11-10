# PostgreSQL migration release procedure (repository-side)

**Status:** Repository procedure only. This runbook does not authorize a
production migration and does not name a production owner, account, or
credential. A named database/platform owner must approve and execute any
production run.

## Authority

The canonical ordered source is
`crates/sitolo-persistence/schema/migrations/`, pinned by `manifest.toml`
(format version 1, SHA-256 per artifact). The integration harness applies
these exact files to disposable schemas. Do not create a second migration
framework or duplicate schema definitions.

## Preflight (owner-executed)

1. Confirm the source revision is the approved candidate and record its SHA.
2. Verify manifest integrity:
   `python scripts/ci/check-schema-migrations.py`.
3. Provision a dedicated migration identity (`migration_owner` or the
   owner-approved equivalent); runtime roles (`app_runtime`, `app_worker`)
   must never execute DDL.
4. Verify target roles exist and are registered in the cluster before DDL:
   `app_runtime`, `app_worker`, `app_outbox_owner`, `app_identity_owner`.
5. Validate the `__SCHEMA__` substitution target against the approved
   schema-name contract (bounded lowercase PostgreSQL identifier, max 63
   bytes); substitute a quoted identifier, never a value.
6. For files without `__SCHEMA__`, set a connection-local `search_path`
   to the same validated target.
7. Run ordered apply 0001 → 0006 in a transaction where DDL permits;
   record each artifact's SHA-256 and execution order in the release log.

## Verification

1. Confirm schema-version ledger state matches the applied manifest order.
2. Catalog checks: table owners, `FORCE ROW LEVEL SECURITY`, and RLS policies
   per `crates/sitolo-persistence/src/postgres.rs` verifier functions.
3. Privilege checks: runtime roles have only the grants in the migration
   files (e.g., `app_runtime` SELECT/INSERT/UPDATE on tenancy tables,
   `app_worker` SELECT/INSERT on `consumer_inbox`); no DDL grant.
4. Runtime DDL denial: attempt a DDL statement as each runtime role and
   confirm denial.
5. Exercise the Integration workflow against a disposable PostgreSQL
   service before promoting the same source revision.

## Forward-fix and recovery

Migrations are immutable once released. Never edit a shipped artifact.
Follow-up changes ship as a new numbered migration with a new SHA-256 entry.
On failure: stop, preserve the ledger state, diagnose with the release log,
apply a forward fix, and re-run verification. Do not re-run partially
applied DDL as a substitute for understanding the ledger state.

## Outstanding owner gates

- Accountable production migration owner and execution identity.
- Durable schema-version ledger location and retention.
- Rollout window, backup/PITR readiness, and incident owner.
- Production role provisioning and secret-manager wiring.

Until the owner records these, production migration remains `BLOCKED_BY_APPROVED_DECISION`.
