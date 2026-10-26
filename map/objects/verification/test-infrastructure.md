---
type: object
status: implemented
universe: source
cluster: verification
source_revision: feat/phase4-pr009-audit-outbox@d8032eff0422edb9447239ccc13f403f285c00db
source: crates/sitolo-testkit/src/lib.rs
source_citation: crates/sitolo-testkit/src/lib.rs:13,55-90; crates/sitolo-testkit/src/postgres.rs:31-109; apps/api/tests/tenancy_api.rs:466-526; crates/sitolo-application/tests/part8_provision_organization_evidence_tests.rs:47-100; crates/sitolo-application/tests/membership_transaction_integration.rs:79-125; crates/sitolo-persistence/tests/common/mod.rs:18-56
---
# Test Infrastructure


## Why this shape

Shared test infrastructure makes security, integration, concurrency and repository-boundary claims reproducible.

## Shape

Primary implementation: crates/sitolo-testkit. Canonical test strategy: docs/testing_strategy.md and docs/security_test_harness.md.

The PostgreSQL target type's debug representation redacts connection options; it never formats credential-bearing DSNs.

## Connected to

- `crates/sitolo-tenancy/src/scope.rs` unit tests consume validated lifecycle fixtures.
- `crates/sitolo-application/tests/tenancy_service.rs` consumes validated tenancy identifier helpers.
- `apps/api/tests/tenancy_api.rs`, application provisioning/membership tests, and persistence Part 8/RLS tests consume the shared disposable PostgreSQL target and schema-name validators before connecting or executing DDL.
- `scripts/ci/check-crate-boundaries.py` rejects production dependency edges to `sitolo-testkit`.

## If you change this

### Hits

Test determinism, fixtures, integration harnesses and evidence quality.

### Does not hit

Production runtime behavior unless the test helper is incorrectly linked into production code.

## Surfaces

Tenancy domain unit tests, application tenancy service tests, and guarded real
PostgreSQL integration suites. PostgreSQL, RLS, transaction and security proofs
continue to use their real database fixtures; this crate does not emulate
persistence.

## See

docs/testing_strategy.md

