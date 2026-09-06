# Sitolo — Contribution and Repository Policy

This repository is governed by [`agent.md`](agent.md), the repository-wide engineering governance contract. Every contributor — human or AI agent — is expected to operate within it.

## Repository policy summary

1. **Authority hierarchy.** Resolve ambiguity using the source hierarchy in `agent.md` §1. Technical convenience never overrides a security or correctness invariant.
2. **Clients request; server decides.** No client-provided total, role, tenant, payment status, or stock balance is ever treated as final authority.
3. **Tenant isolation is non-negotiable.** Cross-tenant vulnerability is Critical and blocks release.
4. **Financial and inventory integrity.** Finalized facts are corrected by explicit compensating operations, never silently overwritten.
5. **Dependency direction.** `API -> Application -> Domain`; `Infrastructure -> Application ports`. Domain must not depend on Axum, SQLx, Redis, HTTP clients, or provider SDKs.
6. **Reproducible builds.** The toolchain is pinned; `Cargo.lock` is tracked. Locked builds are the only verified builds.
7. **CI is a security boundary.** Required checks fail closed. Untrusted PRs never receive production secrets.
8. **PostgreSQL invariants require PostgreSQL tests.** Mock-only assurance is insufficient for RLS, transactions, constraints, locking, and isolation claims.
9. **Artifact identity is cryptographic.** A tested artifact is promoted; it is never rebuilt after approval.
10. **Evidence.** A control must produce evidence and must be able to fail. Prose is not an engineering control.

## Developer bootstrap

```text
install/activate pinned toolchain (rust-toolchain.toml)
verify required tools
start disposable dependencies
apply migrations where applicable
run verification
```

The canonical verification command is `./scripts/ci/verify` once it lands in the repository. Until then, the Phase 1 root command contract applies:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
```

## Commit discipline

Prefer cohesive commits scoped to a single concern. Do not combine foundational repository work with unrelated product features merely to reduce commit count.

```text
chore(repo): establish repository policy
build(rust): establish workspace and toolchain
ci: add required Rust verification
test(db): establish PostgreSQL harness
```

## Pull request contract

A substantive PR should disclose: summary; scope; architecture impact; security impact; tenant-isolation impact; data/migration impact; tests; operational impact; rollback; documentation. Dependency, workflow, and database PRs add their specific disclosures (see the Phase 1 specification, §34).

## Review expectations

- Sensitive paths (`migrations/`, `crates/sitolo-auth/`, `crates/sitolo-authz/`, `crates/sitolo-tenancy/`, `crates/sitolo-persistence/`, `crates/sitolo-sync/`, `crates/sitolo-integrations/`, `infra/`, `.github/workflows/`) require explicit ownership review.
- Lint exceptions require a specific lint, a specific reason, smallest practical scope, and review.
- `unsafe` requires a documented safety invariant, justification for why a safe abstraction is insufficient, minimal scope, tests, and review.
- Scanner failures and security-test failures fail closed; they are not advisory.

## Failures

A deterministic failure is fixed, never silently re-run. A flaky test requires identification, evidence, an owner, a tracking issue, and a review date. Security tests have a stricter quarantine threshold.

## License

Unless stated otherwise, contributions to this repository are Apache License 2.0 as described in [`LICENSE`](LICENSE).