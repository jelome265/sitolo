---
artifact_type: remediation-extension
status: implemented-partial
run_slug: "pr009-part8"
security_relevant: yes
implementation_source: "31d89a9b63e76c1b47c88b1b1276cc7eb1d2d296"
verification_head: "31d89a9b63e76c1b47c88b1b1276cc7eb1d2d296"
date: "2026-10-26"
---

# Phase 1–4 migration/release remediation extension

## Implemented

- Established `crates/sitolo-persistence/schema/migrations/` as one ordered, versioned SQL source for identity authority, audit/outbox, tenancy and consumer inbox (`0001` through `0004`).
- Moved audit/outbox DDL out of the test fixture into migration `0002`; removed the duplicate DDL fixture. Test-only cluster role creation is in `tests/fixtures/role_bootstrap.sql` and does not supply production credentials.
- Separated identity owner role provisioning from the Phase 3 schema migration. The migration requires the platform-provisioned role and validates its restricted attributes before transferring schema ownership.
- Updated the API, application and persistence PostgreSQL test harnesses to apply canonical migration sources. The persistence suite applies the complete ordered bundle in its disposable schema.
- Added `manifest.toml` with migration order and SHA-256 values, plus `check-schema-migrations.py` in the canonical Rust verifier to reject modified, missing, extra, renumbered or unlisted migration files.
- Kept the retired `schema/consumer_inbox.sql` path as a fail-closed tombstone for historical report links; it contains no schema definition.

## Verification

Local database-free checks on the source candidate passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --all-features --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-targets --all-features --locked --no-run`, `python scripts/ci/check-schema-migrations.py`, `python scripts/ci/check-doc-references`, and `git diff --check`. No local PostgreSQL server was started. The ICM PowerShell wrapper could not run on this Windows host because `bash` is unavailable.

All exact-head workflows passed at `31d89a9b63e76c1b47c88b1b1276cc7eb1d2d296`:

- Rust canonical verification, including disposable PostgreSQL suites: [run 40295892357](https://github.com/jelome265/sitolo/actions/runs/40295892357).
- PostgreSQL application/contract integration: [run 40295892353](https://github.com/jelome265/sitolo/actions/runs/40295892353).
- Security, including advisories, dependency policy, offensive regression and secret scanning: [run 40295892391](https://github.com/jelome265/sitolo/actions/runs/40295892391).
- Policy: [run 40295892367](https://github.com/jelome265/sitolo/actions/runs/40295892367).
- CodeQL: [check 121483882612](https://github.com/jelome265/sitolo/runs/121483882612).

## Remaining release dependency

This creates and exercises a canonical source bundle; it does not complete production migration/release governance. A database/platform owner must identify the accountable release owner, approve the DDL execution identity and production runner, define and verify a durable migration-version ledger, readiness/preflight checks, rollout order, and recovery/forward-fix procedure. Production catalog/privilege verification and upgrade/failure recovery under that approved process remain required before this requirement can be marked complete. The requirements matrix records this as `PARTIAL`.

No production deployment, role/password provisioning, owner approval, independent Stage 06 audit, or human review is claimed. PR #49 remains open and unmerged.
