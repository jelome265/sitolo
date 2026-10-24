---
artifact_type: verification-report
status: complete
run_slug: "pr009-part8"
---

# Verification Report — PR #49 Phase 4 Part 8 remediation

**Branch:** `feat/phase4-pr009-audit-outbox`
**Remediation input:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-remediation-2026-10-24-report.md`
**Audit input:** `docs/phase4_part8_pr009_remediation_report.md` (source `a1d29ae`)
**Security relevant:** yes
**Verification date:** 2026-10-24

## Commands

| Command | Result | Scope |
|---|---|---|
| `cargo fmt --all -- --check` | PASSED | Whole workspace, all targets |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASSED (exit 0, no diagnostics) | Whole workspace, all targets, all features, strict policy unchanged |
| `cargo check --workspace --all-targets --all-features --locked` | PASSED | Whole workspace, all targets |
| `cargo test --workspace --lib --all-features --locked` | PASSED — 259 tests, 0 failed | Library unit tests across 17 packages |
| `python scripts/ci/check-crate-boundaries.py` | PASSED — 17 packages classified | Exhaustive application/persistence dependency contract |
| `./scripts/ci/check-architecture` | PASSED | Pure-crate graph, single global logging installer, single config-fingerprint owner, boundary contract |
| `./scripts/ci/check-phase2-policy` | PASSED | Configuration/secret/logging policy |
| `cargo deny check` | PASSED — advisories, bans, licenses, sources | Dependency and license policy |
| `./scripts/ci/check-icm-workspace` | PASSED — 9 stages, exact handoffs, no synthetic approval | ICM routing, stage contracts, mechanical cold walk, System Map source hop |
| `cargo metadata --format-version 1 --locked` | PASSED | Lockfile is current |

Per-crate library results: `sitolo-api` 20, `sitolo-api-bin` 7, `sitolo-audit` 4, `sitolo-auth` 53,
`sitolo-authz` 22, `sitolo-config` 24, `sitolo-domain` 11, `sitolo-events` 12,
`sitolo-integrations` 12, `sitolo-observability` 10, `sitolo-persistence` 41, `sitolo-security` 18,
`sitolo-tenancy` 8, `sitolo-worker-bin` 17. `sitolo-application`, `sitolo-sync` and
`sitolo-testkit` have no library tests; their proof is the integration targets exercised in CI.

## Environment

- Windows 11, Git Bash, Rust 2024 edition.
- `rustc`/`cargo` 1.98.1 (matches CI toolchain pin); Python 3.14.7.
- No local PostgreSQL instance was installed, started, or used.
- `cargo-deny` 0.x is installed locally. `cargo-audit` is **not** installed locally.
- `scripts/ci/check-architecture`, `check-phase2-policy` and `check-icm-workspace` were executed
  through small local `rg`/`python3` shims because the host lacks ripgrep and the `python3`
  alias. The shims only translate the exact flags those scripts use (`-n`, `-g`) and forward to
  `grep`/the real Python interpreter; the checks themselves are unmodified. CI runs the same
  scripts with native `rg` and `python3`.

## Security control evidence

| Control ID | Gate/test | Result | Evidence | Limitation |
|---|---|---|---|---|
| SC-002 Tenant scope is server-derived | Port inversion; `check-crate-boundaries`; application tenancy integration targets | PASSED locally for compilation and unit scope | `crates/sitolo-application/src/tenancy_ports.rs`; `crates/sitolo-persistence/src/tenancy_unit_of_work.rs`; boundary check | The real PostgreSQL RLS suite and application transaction tests require the CI PostgreSQL service; not run here |
| SC-003 Authorization fails closed | `sitolo-authz` 22 tests; strict invitation-policy matcher | PASSED | `crates/sitolo-persistence/src/postgres.rs` `verify_part8_provisioning_rls_metadata`; `sitolo-authz` suite | Negative matrix against a live database remains CI-only |
| SC-005 Secrets stay narrow | `sitolo-security` 18 tests; manual `Debug` implementations that print presence flags only | PASSED | `apps/api/src/bootstrap.rs`, `state.rs`; `apps/worker/src/{consumer,relay,sns,sqs}.rs`; `sitolo-security` suite | Production AWS Secrets Manager resolution is deployment-specific |
| SC-009 Durable audit evidence | `sitolo-persistence` 41 tests; `FailingUnitOfWork`/`FailingTransaction` rollback targets | PASSED for compiled and unit scope | `crates/sitolo-application/tests/membership_transaction_integration.rs`; `part8_provision_organization_evidence_tests.rs` | Evidence-failure rollback against PostgreSQL runs only in CI |
| SC-010 Resource limits and fail-closed uncertainty | Bounded relay/consumer backoff and batch limits; route body/timeout bounds; `sitolo-worker-bin` 17 tests | PASSED | `apps/worker/src/relay.rs`, `consumer.rs`; `apps/api/src/serve.rs` | End-to-end load shedding is CI/runtime evidence |
| SC-011 Supply chain attributable and policy-checked | `cargo deny check`; `cargo metadata --locked`; lockfile diff limited to the persistence→application edge | PASSED locally | `Cargo.lock`; `deny.toml`; `cargo-deny` output | `cargo-audit` is not installed locally; the CI `advisories` job is authoritative |

## Evidence

- Strict workspace lint policy kept unchanged; no lint was suppressed to obtain a pass. The only
  `#[expect]` attributes added are for genuine structural exceptions (Rust 2024 process-environment
  mutation in the serialized test harness, and the worker's pre-logging stderr message), each with
  a written reason.
- `sitolo-application` no longer depends on `sitolo-persistence`; the ports live in
  `crates/sitolo-application/src/{identity_ports,tenancy_ports}.rs`, and persistence implements
  them (`crates/sitolo-persistence/src/tenancy_unit_of_work.rs`). Persistence now depends on
  application, never the reverse.
- `crates/sitolo-persistence/src/ports.rs` is deleted; persistence re-exports no DTOs of its own.
- `Cargo.lock` changes are limited to the persistence→application edge and the resulting
  alphabetical ordering.
- Error boundaries drop provider/SDK/database sources deliberately: startup and worker errors carry
  classes and counts only, never DSNs, secret references, receipt handles, or payloads.
- Branch activation uses `SECURITY DEFINER transition_branch_state` rather than widening `UPDATE`
  grants on `branches`.
- `sitolo-persistence/tests/rls_security_tests.rs` uses shared `FixtureIds`/`seed_fixtures` and
  `PgAuthorityError::Scope(#[from] ScopeError)`, so the Part 8 RLS verifier is split into focused
  helpers instead of one unverifiable block.
- `apps/api` and `apps/worker` opt into `[lints] workspace = true` and now pass the same strict
  policy as the libraries.

## Limitations

- Database-backed suites (`rls_security_tests`, `part8_postgres_integration_tests`, and the
  application PostgreSQL integration targets) were **not** run locally. They require the CI
  PostgreSQL service and the three DSNs.
- `cargo-audit` was not executed locally because the tool is absent from this host; `cargo deny
  check` passed.
- `scripts/ci/verify` as a single command was not executed end-to-end locally because it includes
  the PostgreSQL suite and `cargo audit`. Its non-database steps were executed individually and are
  recorded above.
- A green local run is evidence only for the code paths it exercised. Exact-head GitHub Actions
  remains the authority for database, advisory, and release-build gates.

## Audit status

The independent Stage 06 re-audit required by the remediation report has **not** been performed.
This verification therefore claims no audit approval, no security sign-off, and no release
readiness. R2 route coverage, R3 command-by-command mutation matrix, R4/AUD-04 transition mapping,
AUD-01 consumer completion, and AUD-05 worker runtime/migration path remain open.

## Completion condition

Local non-database gates are green and the crate-boundary, lint, and ICM contracts are enforced.
Verification is **not** complete for release: the exact-head CI database/advisory gates and the
independent re-audit are still required. Control returns to `06-audit` for that re-audit; `09-deliver`
is not entered.
