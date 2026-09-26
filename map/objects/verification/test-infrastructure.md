---
type: object
status: stub
universe: ghost
cluster: verification
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
source: crates/sitolo-testkit/src/lib.rs
source_citation: crates/sitolo-testkit/src/lib.rs:1
---
# Test Infrastructure


## Why this shape

Shared test infrastructure makes security, integration, concurrency and repository-boundary claims reproducible.

## Shape

Primary implementation: crates/sitolo-testkit. Canonical test strategy: docs/testing_strategy.md and docs/security_test_harness.md.

## Connected to

Every domain and infrastructure boundary under test.

## If you change this

### Hits

Test determinism, fixtures, integration harnesses and evidence quality.

### Does not hit

Production runtime behavior unless the test helper is incorrectly linked into production code.

## Surfaces

Unit, integration and security test suites.

## See

docs/testing_strategy.md

