---
artifact_type: verification-report
status: complete
run_slug: "pr009-part8"
---

# Verification Report — PR #49 Phase 4 Part 8 remediation

**Branch:** `feat/phase4-pr009-audit-outbox`
**Remediation input:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-remediation-2026-10-24-report.md`
**Audit input:** `docs/phase4_part8_pr009_remediation_report.md` (source `a1d29ae`)
**Current source basis:** `5a128c4f95db61f7f6cf9fd885d22911b78553c1` (2026-10-25)
**Security relevant:** yes
**Verification window:** 2026-10-24 to 2026-10-25

## Commands

| Command | Result | Scope |
|---|---|---|
| `cargo fmt --all -- --check` | PASSED | Whole workspace, all targets |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASSED (exit 0, no diagnostics) | Whole workspace, all targets, all features, strict policy unchanged |
| `cargo check --workspace --all-targets --all-features --locked` | PASSED | Whole workspace, all targets |
| `cargo test --workspace --lib --all-features --locked` | PASSED — 259 tests, 0 failed | Library unit tests across 17 packages |
| `cargo build --workspace --release --locked` | PASSED — finished in 5m 21s | Whole workspace, release profile |
| `cargo test -p sitolo-api-bin --test axum_boundary --all-features --locked` | PASSED — 3 tests | R2 deny-by-default over every declared protected route; exact closed-route partition |
| `cargo test -p sitolo-application --test lifecycle_event_contract --all-features --locked` | PASSED — 2 tests | R4 five-event catalogue/validator consistency, wrong-version and unknown-field rejection |
| `python scripts/ci/check-crate-boundaries.py` | PASSED — 17 packages classified | Exhaustive application/persistence dependency contract |
| `./scripts/ci/check-architecture` | PASSED | Pure-crate graph, single global logging installer, single config-fingerprint owner, boundary contract |
| `./scripts/ci/check-phase2-policy` | PASSED | Configuration/secret/logging policy |
| `./scripts/ci/check-workflow-policy` | PASSED | GitHub Actions workflow policy |
| `./scripts/ci/check-release-workflow` | PASSED | Release workflow provenance and promotion policy |
| `./scripts/ci/check-security-icm-policy` | PASSED | Security ICM policy |
| `python scripts/ci/check-doc-references` | PASSED — 238 files, 1054 references, 0 errors | Documentation link integrity |
| `./scripts/ci/check-system-map-processes` | PASSED — 2 source-grounded cards | System Map process/source hop |
| `cargo deny check` | PASSED — advisories, bans, licenses, sources | Dependency and license policy, including the `serde_json/unbounded_depth` feature ban |
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
- `scripts/ci/check-architecture`, `check-phase2-policy`, `check-workflow-policy`,
  `check-release-workflow` and `check-icm-workspace` were executed through small local
  `rg`/`python3` shims because the host lacks ripgrep and the `python3` alias. The shims only
  translate the flags those scripts use (`-n`, `-q`, `-g`) and forward to `grep`/the real Python
  interpreter; the checks themselves are unmodified. CI runs the same scripts with native `rg` and
  `python3`.

## Security control evidence

| Control ID | Gate/test | Result | Evidence | Limitation |
|---|---|---|---|---|
| SC-002 Tenant scope is server-derived | Port inversion; `check-crate-boundaries`; application tenancy integration targets | PASSED locally for compilation and unit scope | `crates/sitolo-application/src/tenancy_ports.rs`; `crates/sitolo-persistence/src/tenancy_unit_of_work.rs`; boundary check | The real PostgreSQL RLS suite and application transaction tests require the CI PostgreSQL service; not run here |
| SC-003 Authorization fails closed | `sitolo-authz` 22 tests; strict invitation-policy matcher | PASSED | `crates/sitolo-persistence/src/postgres.rs` `verify_part8_provisioning_rls_metadata`; `sitolo-authz` suite | Negative matrix against a live database remains CI-only |
| SC-005 Secrets stay narrow | `sitolo-security` 18 tests; manual `Debug` implementations that print presence flags only | PASSED | `apps/api/src/bootstrap.rs`, `state.rs`; `apps/worker/src/{consumer,relay,sns,sqs}.rs`; `sitolo-security` suite | Production AWS Secrets Manager resolution is deployment-specific |
| SC-009 Durable audit evidence | `sitolo-persistence` 41 tests; `FailingUnitOfWork`/`FailingTransaction` rollback targets | PASSED for compiled and unit scope | `crates/sitolo-application/tests/membership_transaction_integration.rs`; `part8_provision_organization_evidence_tests.rs` | Evidence-failure rollback against PostgreSQL runs only in CI |
| SC-010 Resource limits and fail-closed uncertainty | Bounded relay/consumer backoff and batch limits; route body/timeout bounds; exhaustive protected-route inventory; R2 missing-credentials denial | PASSED | `apps/worker/src/relay.rs`, `consumer.rs`; `apps/api/src/serve.rs`; `apps/api/tests/axum_boundary.rs` | End-to-end load shedding is CI/runtime evidence |
| SC-011 Supply chain attributable and policy-checked | `cargo deny check`; `cargo metadata --locked`; lockfile diff limited to the persistence→application edge; `serde_json/unbounded_depth` feature denied | PASSED locally | `Cargo.lock`; `deny.toml`; `cargo-deny` output | `cargo-audit` is not installed locally; the CI `advisories` job is authoritative |

## Evidence

- Strict workspace lint policy kept unchanged; no lint was suppressed to obtain a pass. The
  framework-ban policy was reconciled with the enterprise reference: every reachable ban is active,
  and the two deliberate omissions are documented with proof (`Runtime::block_on` fires on all 183
  `#[tokio::test]` expansions; `disable_recursion_limit` is unreachable without `unbounded_depth`,
  which is denied in `deny.toml`). Scoped `#[expect(..., reason = ...)]` attributes cover only
  genuine structural exceptions: the two runtime entry points, the reviewed route registry, the
  local-only secret adapter, and bounded test spawn/DSN helpers. `#[allow]` remains banned.
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
- Every code gate above was executed against commit `5a128c4f95db61f7f6cf9fd885d22911b78553c1`
  with a clean working tree before this documentation refresh. The documentation-only changes do
  not alter any Rust source, manifest, workflow, or gate script.
- The reviewed route inventory now exposes `declared_routes()` and `CLOSED_PROTECTED_ROUTES`;
  `apps/api/tests/axum_boundary.rs` proves all six declared protected routes reject missing
  credentials and that exactly five are wired while organization activation is explicitly closed.
- `crates/sitolo-application/tests/lifecycle_event_contract.rs` pins the five approved lifecycle
  names to the strict payload validator: v1-only, no unknown fields, unique names.

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
  remains the authority for the database and advisory gates.

## Audit status

The independent Stage 06 re-audit required by the remediation report has **not** been performed.
This verification therefore claims no audit approval, no security sign-off, and no release
readiness. R2 route coverage is proven locally over the declared inventory; R3 is refreshed as a
source-inspection matrix; R4 registry consistency is proven, but transition emission remains a
source-trace item. AUD-01 consumer completion and AUD-05 worker runtime/migration path remain open.
Exact-head GitHub Actions has not run for `5a128c4`, and the branch has not been pushed.

## Completion condition

Local non-database gates are green and the crate-boundary, lint, route-inventory, and ICM contracts
are enforced. Verification is **not** complete for release: the exact-head CI database/advisory
gates and the independent re-audit are still required. Control returns to `06-audit` for that
re-audit; `09-deliver` is not entered.
