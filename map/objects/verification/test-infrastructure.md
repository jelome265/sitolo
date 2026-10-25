---
type: object
status: implemented
universe: source
cluster: verification
source_revision: feat/phase4-pr009-audit-outbox@468186cad7a2ccb2abe8f3305e841ac3f81cd7bd
source: crates/sitolo-testkit/src/lib.rs
source_citation: crates/sitolo-testkit/src/lib.rs:13
---
# Test Infrastructure


## Why this shape

Shared test infrastructure makes security, integration, concurrency and repository-boundary claims reproducible.

## Shape

Primary implementation: crates/sitolo-testkit. Canonical test strategy: docs/testing_strategy.md and docs/security_test_harness.md.

## Connected to

- `crates/sitolo-tenancy/src/scope.rs` unit tests consume validated lifecycle fixtures.
- `crates/sitolo-application/tests/tenancy_service.rs` consumes validated tenancy identifier helpers.
- `scripts/ci/check-crate-boundaries.py` rejects production dependency edges to `sitolo-testkit`.

## If you change this

### Hits

Test determinism, fixtures, integration harnesses and evidence quality.

### Does not hit

Production runtime behavior unless the test helper is incorrectly linked into production code.

## Surfaces

Tenancy domain unit tests and application tenancy service tests. PostgreSQL, RLS,
transaction and security proofs continue to use their real PostgreSQL fixtures;
this crate does not emulate persistence.

## See

docs/testing_strategy.md

