# Phase 4 Part 7 / PR-008 Remediation Execution Contract: Real PostgreSQL RLS Tenant Boundary

This execution contract records the non-negotiable verification gates, actual sha256 artifact digests, and test evidence for Phase 4 PR-008 PostgreSQL Row-Level Security (RLS) integration.

---

## Artifact SHA-256 Checksums

| Artifact | File Path | SHA-256 Digest |
|---|---|---|
| **RLS Schema Fixture** | `crates/sitolo-persistence/tests/fixtures/rls_schema.sql` | `f04b2162810c3e2c5fc83c6aabc56ae60e5ac605a8349616c39cfd66557f8592` |
| **Persistence Infrastructure** | `crates/sitolo-persistence/src/postgres.rs` | `34b3bffa8a7803acb4f5233196ccbc17e8ce4397ef353814896767807d61e7ba` |
| **RLS Security Test Suite** | `crates/sitolo-persistence/tests/rls_security_tests.rs` | `1f6dd5fc97ec206e0425373c021be8767872becb039732517d525d8b1727889d` |
| **GitHub Workflow (Rust)** | `.github/workflows/rust.yml` | `44ae2b36fbb871881a011d7fcd3c3b9d8c0aa13e83ffa8b4012a773bcfde9129` |
| **GitHub Workflow (Artifact)** | `.github/workflows/artifact.yml` | `352522065476a63c02573a6f0af8841fc38ff13070d80b8e9168bffbe0f10c8e` |

---

## Security Verification Gate Records

| Gate ID | Verification Description | Status | Evidence Record |
|---|---|---|---|
| **P4-008-G01** | Real PostgreSQL Provisioning | PASSED | Real PostgreSQL 18 container connected via `DATABASE_URL` / `ADMIN_DATABASE_URL` / `RUNTIME_DATABASE_URL`. |
| **P4-008-G02** | Least-Privileged Role Attributes | PASSED | `app_runtime` verified with `SUPERUSER=false`, `BYPASSRLS=false`, `REPLICATION=false`, `INHERIT=false`. |
| **P4-008-G03** | Schema Ownership Separation | PASSED | Migration/schema setup runs as admin owner; `app_runtime` owns zero tables and holds no schema `CREATE` privilege. |
| **P4-008-G04** | Table RLS Enabled | PASSED | PostgreSQL catalog (`pg_class.relrowsecurity`) confirms RLS enabled on `organizations`, `branches`, `tenant_resources`. |
| **P4-008-G05** | Catalog Policy Expression | PASSED | Catalog (`pg_policy.polqual`, `polwithcheck`) matches normalized SQL policy strings (`current_setting(...)`). |
| **P4-008-G06** | Catalog Policy Role Bounds | PASSED | Catalog (`pg_policy.polroles`) confirms policies bound strictly to `app_runtime` OID (`polroles = [runtime_oid]`). |
| **P4-008-G07** | Scope Encapsulation | PASSED | `AuthorizedScope` fields are private with read-only accessors, preventing arbitrary struct literal forgery. |
| **P4-008-G08** | Transaction-Local Context | PASSED | Context set via `set_config('app.organization_id', ..., true)` within `PgAuthorityPools::set_transaction_tenant_context`. |
| **P4-008-G09** | Connection Pool Cleanliness | PASSED | 10-iteration sequential pool reuse test proves Tenant A GUC context does not leak into subsequent Tenant B transactions. |
| **P4-008-G10** | Missing Context Fail-Closed | PASSED | Transaction with unset tenant GUC returns 0 rows and rejects writes. |
| **P4-008-G11** | Invalid Context Fail-Closed | PASSED | Transaction with nonexistent tenant GUC returns 0 rows. |
| **P4-008-G12** | Cross-Tenant Read Denial | PASSED | Tenant A reading Tenant B resource returns `NotFoundOrDenied`. |
| **P4-008-G13** | Cross-Tenant Update Denial | PASSED | Tenant A updating Tenant B resource returns `NotFoundOrDenied` with DB state unchanged. |
| **P4-008-G14** | Cross-Tenant Delete Denial | PASSED | Tenant A deleting Tenant B resource returns `NotFoundOrDenied` with DB state unchanged. |
| **P4-008-G15** | Cross-Tenant Insert Denial | PASSED | Tenant A inserting Tenant B owned row fails at PostgreSQL RLS `WITH CHECK` (SQLSTATE `42501`/`44000`). |
| **P4-008-G16** | Ownership-Changing Update | PASSED | Tenant A updating row `organization_id` to Tenant B fails at PostgreSQL RLS `WITH CHECK` (SQLSTATE `42501`/`44000`). |
| **P4-008-G17** | Direct DB Query Denial | PASSED | Raw SQL `SELECT * FROM tenant_resources WHERE id = $1` without application `WHERE organization_id` predicate hides Tenant B row. |
| **P4-008-G18** | Application Composition Seam | PASSED | Invocation counter proves application authorization rejection stops execution with 0 persistence database calls. |
| **P4-008-G19** | Concurrent Isolation | PASSED | 8 concurrent task workers issuing interleaved Tenant A and B reads and writes execute without cross-talk or lock contention. |
| **P4-008-G20** | Unknown Resource Mutations | PASSED | Updating/deleting nonexistent resources returns `NotFoundOrDenied` without state modification or leakage. |
| **P4-008-G21** | Foreign Key Classification | PASSED | Mismatched branch reference fails with foreign key violation (SQLSTATE `23503`), distinct from RLS policy violation. |
| **P4-008-G22** | CI Fail-Closed Enforcement | PASSED | CI workflows (`rust.yml` and `artifact.yml`) enforce credential-free URL environment injection and halt on PostgreSQL failure. |
| **P4-008-G23** | Canonical Pipeline Verification | PASSED | `./scripts/ci/verify` passes cleanly (fmt, clippy, unit, integration, and workspace tests). |

---

## Real Executable Evidence Summary

```text
running 27 tests
test test_branch_scoped_read_isolation ... ok
test test_catalog_rls_policy_metadata ... ok
test test_catalog_runtime_role_privileges ... ok
test test_concurrent_tenant_isolation_reads_and_writes ... ok
test test_cross_tenant_branch_binding_denial ... ok
test test_direct_db_query_without_application_predicate ... ok
test test_connection_pool_context_leakage_and_rollback_safety ... ok
test test_end_to_end_application_and_db_composition ... ok
test test_invalid_tenant_context_fails_closed ... ok
test test_missing_tenant_context_fails_closed ... ok
test test_negative_ownership_changing_update_relationally_valid ... ok
test test_negative_tenant_a_cannot_delete_b ... ok
test test_negative_tenant_a_cannot_insert_b_owned_row_relationally_valid ... ok
test test_negative_tenant_a_cannot_read_b ... ok
test test_negative_tenant_a_cannot_update_b ... ok
test test_negative_unknown_resource_delete_fails_closed ... ok
test test_negative_unknown_resource_does_not_bypass_scope ... ok
test test_negative_unknown_resource_update_fails_closed ... ok
test test_positive_tenant_a_creates_a ... ok
test test_positive_tenant_a_reads_a ... ok
test test_positive_tenant_a_updates_a ... ok
test test_positive_tenant_a_deletes_a ... ok
test test_positive_tenant_b_creates_b ... ok
test test_positive_tenant_b_deletes_b ... ok
test test_positive_tenant_b_reads_b ... ok
test test_positive_tenant_b_updates_b ... ok
test test_schema_isolation_and_teardown_regression ... ok

test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.28s
```
