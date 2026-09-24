## Summary

Phase 4 Part 7 / PR-008 PostgreSQL Row-Level Security (RLS) integration and negative tests.
Introduces a real, isolated PostgreSQL security proof test suite executing against actual RLS policies to enforce tenant isolation, least-privileged runtime roles, transaction-local context binding, and defense-in-depth.

## Scope

In scope:
- Real PostgreSQL RLS integration tests executing against dynamic schemas
- Implementation of `PgAuthorityPools` separating admin authority from least-privileged runtime authority (`app_runtime`)
- 27-test real security suite proving cross-tenant denial, fail-closed missing/invalid context, and application composition seams
- Runtime-role strict catalog verification (NO superuser, NO bypassrls, NO createrole, NO createdb, NO inherit, NO replication)
- Transaction-local tenant context (`app.organization_id`, `app.branch_id`) derived from `AuthorizedScope`

Out of scope:
- Phase 5 PostgreSQL schema migrations and persistent runtime schema state (remains deferred)
- Production database provisioning and operational runbooks
- Application API layer modifications

## Architecture impact

- Persistence layer introduces `PgAuthorityPools` encapsulating the split authority model: `admin_pool` for schema/RLS provisioning and `runtime_pool` for least-privileged tenant-scoped operations.
- Dependency direction remains strictly from persistence toward domain/tenancy; no domain types are exposed to the database connection logic.
- Test harness introduces `SchemaGuard` providing strict isolation via PostgreSQL schemas, guaranteeing cleanup even under error conditions (verified via failure-injection regression test).

## Security impact

- **Authentication/Authorization**: Transaction context is strictly bound to application-resolved `AuthorizedScope`. Missing or invalid contexts fail closed.
- **Tenant Isolation**: Enforced natively by PostgreSQL RLS `USING` and `WITH CHECK` clauses bound strictly to the `app_runtime` role OID.
- **Least Privilege**: `app_runtime` cannot CREATE schemas, cannot escalate privileges, and has no administrative rights.
- **Injection**: All tenant context is bound via transaction-local `set_config(..., true)`, preventing GUC leakage across transactions or pool connections.

## Tenant-isolation impact

Cross-tenant negative cases are explicitly tested and proven:
- Tenant A cannot read Tenant B resources.
- Tenant A cannot update/delete Tenant B resources (returns `NotFoundOrDenied` hiding existence).
- Tenant A cannot INSERT Tenant B owned rows (fails at RLS `WITH CHECK` with SQLSTATE 42501/44000).
- Tenant A cannot UPDATE `organization_id` to claim Tenant B resources.
- Branch binding across tenants fails relationally (SQLSTATE 23503 foreign key violation).

## Data / migration impact

- Schema changes are applied dynamically per-test inside isolated PostgreSQL schemas using the real test fixture: `crates/sitolo-persistence/tests/fixtures/rls_schema.sql`.
- No persistent migrations are introduced in this PR.
- Forward-fix/rollback: No production state is mutated; schema isolation ensures tests leave zero catalog traces.

## Tests

Added a 27-test security suite (`crates/sitolo-persistence/tests/rls_security_tests.rs`) executing against a real PostgreSQL 18 container via `ADMIN_DATABASE_URL` and `RUNTIME_DATABASE_URL`.

Suite breakdown:
- 2 Catalog & Role Privilege assertions
- 8 Positive Isolation & Symmetric Operations
- 8 Negative Cross-Tenant & `WITH CHECK` proofs
- 1 End-to-End Application + DB composition proof
- 1 Direct DB Boundary proof (no app predicate)
- 2 Branch Isolation tests
- 2 Missing & Invalid Context fail-closed tests
- 1 Connection Pool Leakage & Rollback safety test
- 1 Concurrent Tenant Isolation test
- 1 Setup failure-injection cleanup regression test

All tests verified passing in CI.

## Operational impact

- Test infrastructure requires a real PostgreSQL container with split connection URLs (`ADMIN_DATABASE_URL` and `RUNTIME_DATABASE_URL`).
- Failure-injection mechanism is scoped strictly to the test runtime via `tokio::task_local!` and cannot leak into production logic.

## Rollback

- Because this PR introduces no production schema migrations or state mutations, rollback simply reverts the codebase to the prior state with zero database impact.

## Documentation

- Added `docs/phase4_part7_pr008_remediation_execution_contract.md` recording the exact CI evidence, commit SHAs, and 23 verification gates.
- Updated evidence document to accurately reflect the current PR implementation head (`fca2417...`) versus the workflow merge ref (`2cad5...`).

---

### Dependency changes?

- [x] Lockfile intentionally changed (None)
- [x] Advisory review performed (`cargo audit`)
- [x] License review performed (`cargo deny check licenses`)
- [x] Transitive changes inspected (`cargo tree --locked`)

### Workflow changes?

- [x] Event trust reviewed
- [x] Permissions minimal and justified
- [x] No new secrets exposed to untrusted contexts
- [x] Third-party actions pinned to immutable SHAs
- [x] Cache trust separated

### Database changes?

- [x] Migration compatibility reviewed (Deferred to Phase 5)
- [x] Locking/concurrency effects reviewed
- [x] Forward-fix / rollback strategy documented

---

## Current Limitations & Deferred Work

- **Phase 5 PostgreSQL migration/persistence remains deferred.** This PR proves the runtime security boundary and RLS model but does not introduce permanent production migrations.
- **No verification artifacts** are uploaded for the current run; test execution and catalog proofs run dynamically inside isolated PostgreSQL schemas during the workflow execution.

## Current CI Evidence

- **PR Implementation Head**: `fca2417cc5594db201eafb02d255f4070d46fae7`
- **Workflow Merge Ref**: `2cad50062111e6fb3c7c042f82e4433233467ed9`
- **Workflow Run ID**: `35954086334`
- **Workflow Job ID**: `107488579244`
- **Result**: 27 passed, 0 failed, 0 ignored
- **cargo-deny**: executed successfully
- **cargo-audit**: executed successfully
