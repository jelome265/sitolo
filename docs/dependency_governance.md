# Sitolo — Dependency Governance

This document defines how the repository treats dependencies, lockfiles, and licensing. It implements the Phase 1 dependency/lock governance contract (PR-003).

## Principles

- Every dependency must have a reason. Evaluate security history, maintenance, license, transitive graph, compile cost, runtime footprint, API stability, and MSRV/toolchain compatibility before adding one.
- Direct dependencies must be declared directly, never relied on via transitive availability.
- A dependency update is itself a code change.
- `Cargo.lock` is a tracked reproducibility boundary. CI and release builds use `--locked`; a job that mutates the lockfile fails.
- Do not add a dependency that saves a few lines if it introduces disproportionate supply-chain or maintenance risk.

## Tools

- **cargo-deny** (`deny.toml` at the repository root) enforces:
  - advisory policy (known vulnerabilities and yanked crates fail);
  - license policy (crates must carry an allowed license);
  - source policy (crates-io only; no git sources by default).
- **cargo-audit** checks the advisory database.
- **cargo tree --locked** makes the dependency graph inspectable.
- Secret scanning and workflow policy are handled by the security pipeline (PR-007).

Installation:

```text
cargo install cargo-deny --locked
cargo install cargo-audit --locked
```

Both are exercised locally by `scripts/ci/verify` when present and unconditionally by the CI security workflow.

## Adding or updating a dependency

1. Modify the manifest intentionally.
2. Update the lockfile intentionally (`cargo update` or `cargo add`).
3. Inspect direct and transitive changes: `cargo tree --locked`.
4. Inspect advisories: `cargo audit`.
5. Inspect licenses: `cargo deny check licenses`.
6. Confirm the dependency's license is compatible and recorded.
7. Confirm the domain layer remains free of infrastructure/provider dependencies (architecture check).
8. Run the full verification gate.
9. Review the resulting graph and commit the change as its own commit.

## Lockfile drift

If CI reports lockfile drift, the resolution is an intentional dependency update per the process above. Never regenerate a lockfile as a silent side effect of another change.

## Forbidden dependencies in the domain layer

`crates/sitolo-domain` (and domain-adjacent layers) must not depend on Axum, SQLx, Redis clients, HTTP clients, filesystem/network implementations, or provider SDKs. The architecture check (`scripts/ci/check-architecture.sh`) enforces this and fails closed.