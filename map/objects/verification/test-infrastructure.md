# Test Infrastructure

- type: object
- status: verified
- source revision: Sitolo main at e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
- cluster: verification

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
