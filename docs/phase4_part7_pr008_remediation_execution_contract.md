# Phase 4 Part 7 / PR-008 Remediation Execution Contract: Real PostgreSQL RLS Tenant Boundary

This execution contract records the non-negotiable verification gates and test evidence for Phase 4 PR-008 PostgreSQL Row-Level Security (RLS) integration.

---

## Commit & Evidence Metadata

- **Commit SHA**: `4ea642b3163ed8f0cce87482b0bcb1f7b7017cd8`
- **Workflow Run ID**: `35909767457`
- **Workflow Job ID**: `107346354132`
- **Workflow Job Name**: `verify`
- **Workflow Artifact Status**: No release artifacts were configured or uploaded for the verification build; test execution and catalog proofs run dynamically inside isolated PostgreSQL schemas.

---

## Security Verification Gate Records (G01–G23)

| Gate ID | Verification Category | Command / Target | Expected Result | Observed Result | Exit Code | Security Finding / Result | Notes / Limitations |
|---|---|---|---|---|---|---|---|
| **P4-008-G01** | Real Database Provisioning | `cargo test -p sitolo-persistence --test rls_security_tests --all-features` | Real PostgreSQL connected via env vars | Connected to real PostgreSQL 18 container | 0 | PASSED | Uses `ADMIN_DATABASE_URL` and `RUNTIME_DATABASE_URL` |
| **P4-008-G02** | Least-Privileged Role Attributes | `test_catalog_runtime_role_privileges` | `SUPERUSER=f`, `BYPASSRLS=f`, `REPLICATION=f`, `INHERIT=f` | Catalog query confirms zero admin privileges | 0 | PASSED | Direct catalog query on `pg_roles` |
| **P4-008-G03** | Schema Ownership Separation | `test_catalog_runtime_role_privileges` | `app_runtime` owns 0 tables, no `CREATE` privilege | Admin pool owns schema; `app_runtime` has no schema `CREATE` | 0 | PASSED | Table owner is admin user |
| **P4-008-G04** | Table RLS Enabled | `test_catalog_rls_policy_metadata` | `relrowsecurity=t`, `relforcerowsecurity=t` | Catalog confirms RLS forced on protected relations | 0 | PASSED | Verified on `organizations`, `branches`, `tenant_resources` |
| **P4-008-G05** | Catalog Policy Expression | `test_catalog_rls_policy_metadata` | `pg_get_expr` matches canonical SQL policy strings with grouping | Exact canonical expression match preserving boolean structure | 0 | PASSED | Validates boolean structure, whitespace, and casts |
| **P4-008-G06** | Catalog Policy Role Bounds | `test_catalog_rls_policy_metadata` | `polroles = [runtime_oid]` | Catalog confirms policies bound strictly to `app_runtime` OID | 0 | PASSED | No `PUBLIC` (0) or extraneous roles |
| **P4-008-G07** | Scope Encapsulation | `cargo check -p sitolo-tenancy` | `AuthorizedScope` fields private | Construction restricted to server-authoritative constructors | 0 | PASSED | Prevents scope literal forgery |
| **P4-008-G08** | Transaction-Local Context | `set_transaction_tenant_context` | `set_config(..., true)` called within transaction | Sets `app.organization_id` & `app.branch_id` transaction-locally | 0 | PASSED | Automatically scoped to `Transaction` |
| **P4-008-G09** | Connection Pool Cleanliness | `test_connection_pool_context_leakage_and_rollback_safety` | 10-iteration reuse proves no context leak | Sequential transactions execute without tenant leakage | 0 | PASSED | Tenant A GUC does not leak into Tenant B |
| **P4-008-G10** | Missing Context Fail-Closed | `test_missing_tenant_context_fails_closed` | Read query returns 0 rows; INSERT fails at RLS `WITH CHECK` | Query returns 0 rows; INSERT returns SQLSTATE `42501`/`44000` | 0 | PASSED | Read and write fail-closed without context |
| **P4-008-G11** | Invalid Context Fail-Closed | `test_invalid_tenant_context_fails_closed` | Nonexistent org ID returns 0 rows | Query returns 0 rows | 0 | PASSED | Nonexistent GUC fails closed |
| **P4-008-G12** | Cross-Tenant Read Denial | `test_negative_tenant_a_cannot_read_b` | Tenant A reading Tenant B returns `NotFoundOrDenied` | `NotFoundOrDenied` returned | 0 | PASSED | Direct RLS read denial |
| **P4-008-G13** | Cross-Tenant Update Denial | `test_negative_tenant_a_cannot_update_b` | Update returns `NotFoundOrDenied`, DB unchanged | `NotFoundOrDenied` returned; DB state unchanged | 0 | PASSED | Direct RLS update denial |
| **P4-008-G14** | Cross-Tenant Delete Denial | `test_negative_tenant_a_cannot_delete_b` | Delete returns `NotFoundOrDenied`, DB unchanged | `NotFoundOrDenied` returned; DB state unchanged | 0 | PASSED | Direct RLS delete denial |
| **P4-008-G15** | Cross-Tenant Insert Denial | `test_negative_tenant_a_cannot_insert_b_owned_row_relationally_valid` | INSERT fails at RLS `WITH CHECK` | SQLSTATE `42501`/`44000` returned | 0 | PASSED | RLS `WITH CHECK` enforcement |
| **P4-008-G16** | Ownership-Changing Update | `test_negative_ownership_changing_update_relationally_valid` | UPDATE changing `organization_id` fails | SQLSTATE `42501`/`44000` returned | 0 | PASSED | RLS `WITH CHECK` enforcement |
| **P4-008-G17** | Direct DB Query Denial | `test_direct_db_query_without_application_predicate` | Query without `WHERE organization_id` hides row | Query returns `None` | 0 | PASSED | RLS hides cross-tenant row independently |
| **P4-008-G18** | Application Composition Seam | `test_end_to_end_application_and_db_composition` | App authorization rejection stops execution | Invocation counter proves 0 DB calls on app auth failure | 0 | PASSED | App + DB defense-in-depth composition |
| **P4-008-G19** | Concurrent Isolation | `test_concurrent_tenant_isolation_reads_and_writes` | 8 concurrent workers execute without cross-talk | All 8 workers complete with strict isolation | 0 | PASSED | Interleaved Tenant A & B execution |
| **P4-008-G20** | Unknown Resource Mutations | `test_negative_unknown_resource_update_fails_closed` | Nonexistent resource update/delete returns `NotFoundOrDenied` | `NotFoundOrDenied` returned; DB state unchanged | 0 | PASSED | Fail-closed on missing target |
| **P4-008-G21** | Foreign Key Classification | `test_cross_tenant_branch_binding_denial` | Invalid branch reference returns SQLSTATE 23503 | SQLSTATE `23503` (foreign key violation) returned | 0 | PASSED | Correct SQLSTATE classification |
| **P4-008-G22** | CI Fail-Closed Enforcement | `./scripts/ci/verify` | Missing `cargo-deny`/`cargo-audit` causes non-zero exit | Fail-closed logic exits with 1 if tool missing | 0 | PASSED | Script checks tool presence |
| **P4-008-G23** | Canonical Pipeline Verification | `./scripts/ci/verify` | Full pipeline passes (fmt, clippy, unit, rls suite, audit, deny) | All verification checks pass cleanly | 0 | PASSED | Canonical CI gate |

---

## Real Executable Evidence Summary

```text
running 27 tests
test test_branch_scoped_read_isolation ... ok
test test_catalog_rls_policy_metadata ... ok
test test_catalog_runtime_role_privileges ... ok
test test_cross_tenant_branch_binding_denial ... ok
test test_connection_pool_context_leakage_and_rollback_safety ... ok
test test_concurrent_tenant_isolation_reads_and_writes ... ok
test test_direct_db_query_without_application_predicate ... ok
test test_missing_tenant_context_fails_closed ... ok
test test_invalid_tenant_context_fails_closed ... ok
test test_end_to_end_application_and_db_composition ... ok
test test_negative_ownership_changing_update_relationally_valid ... ok
test test_negative_tenant_a_cannot_delete_b ... ok
test test_negative_tenant_a_cannot_insert_b_owned_row_relationally_valid ... ok
test test_negative_tenant_a_cannot_read_b ... ok
test test_negative_tenant_a_cannot_update_b ... ok
test test_negative_unknown_resource_delete_fails_closed ... ok
test test_negative_unknown_resource_does_not_bypass_scope ... ok
test test_negative_unknown_resource_update_fails_closed ... ok
test test_positive_tenant_a_creates_a ... ok
test test_positive_tenant_a_deletes_a ... ok
test test_positive_tenant_a_reads_a ... ok
test test_positive_tenant_a_updates_a ... ok
test test_positive_tenant_b_creates_b ... ok
test test_positive_tenant_b_reads_b ... ok
test test_positive_tenant_b_deletes_b ... ok
test test_setup_failure_injection_cleans_up_schema ... ok
test test_positive_tenant_b_updates_b ... ok

test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.06s
```
