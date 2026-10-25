---
artifact_type: remediation-execution-report
status: partial-blocked
run_slug: "pr009-part8"
security_relevant: yes
source_head: "ef0e97b3cd9e239db42c16c876828eea5454f62d"
date: "2026-10-25"
---

# Phase 1–4 remediation execution — PR #49

## Disposition

Partial implementation completed on the requested feature branch. PR #49 remains open and **not ready for merge or Phase 4 closure**. This report records the two bounded remediations implemented in this pass; it does not supersede the historical deep audit or claim that all Phase 1–4 requirements are complete.

The branch was at `a378beec2bc5097bb642825d2e8fcf4c967c105d` before the requested pull. `git pull --ff-only` advanced it to `6662076dcdb911e8ca0d783f8e3fc3ec089f1f28`. The implementation commits are `289ae0c`, `e48b12a`, and `ef0e97b`; this report's source SHA is `ef0e97b3cd9e239db42c16c876828eea5454f62d`.

## Completed remediations

### R-01 / F-05 — test database target guard

Both PostgreSQL integration harnesses now validate the explicit `SITOLO_TEST_DATABASE_DISPOSABLE=true` opt-in and parse DSNs before connecting or executing DDL. Admin/runtime targets must use the documented roles, loopback `127.0.0.1:5432`, approved disposable database names, and matching host/port/database. The Part 8 harness additionally validates the worker DSN and role before creating its schema. CI supplies the opt-in only beside its PostgreSQL service. The offensive-security runner and workflow policy checks require the same declaration. No remote-host override was added.

Pure parser tests reject missing opt-in, remote host, mismatched database, unexpected role, production database name, and a conflicting worker database. These are parser-only tests; they do not claim PostgreSQL security behavior. Real database integration remains covered by CI.

### R-09 / F-09 — optional configuration fingerprint values

The fingerprint previously encoded absent optional fields as the literal `<none>`, which could collide with an actual optional value of `<none>`. Optional values now use distinct `none` and length-prefixed `some` encodings. `FINGERPRINT_FORMAT_VERSION` is 2, making the canonical-format change explicit. Regression coverage includes all current effective `AppConfig` fields, OIDC fields, the worker secret reference, secret exclusion, and an absent-versus-present sentinel case.

## Evidence

Local checks on source `e48b12a`:

- `cargo fmt --all -- --check` — PASS.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — PASS.
- `cargo check --workspace --all-targets --all-features --locked` — PASS.
- `cargo test -p sitolo-config fingerprint --all-features --locked` — PASS, 7 tests.
- `cargo test -p sitolo-persistence --test part8_postgres_integration_tests common::tests --all-features --locked` — PASS, 6 parser tests; no database connection.
- `cargo test -p sitolo-persistence --test rls_security_tests common::tests --all-features --locked` — PASS, 6 parser tests; no database connection.
- Crate boundaries, ICM workspace, workflow policy, release-workflow, security-ICM policy, System Map verification, and doc references — PASS.
- Native `scripts/ci/check-phase2-policy.ps1` — PASS. An initial Git Bash invocation produced a Windows-path false positive; the repository's PowerShell checker passed.
- Full local PostgreSQL tests were not run. The repository contract requires the approved disposable GitHub Actions PostgreSQL service, and no local server was started.

Exact-head GitHub Actions observed for `ef0e97b3cd9e239db42c16c876828eea5454f62d`:

| Workflow | Result | Run |
|---|---|---|
| Rust | PASS, including canonical verification and required PostgreSQL suites | [40176319651](https://github.com/jelome265/sitolo/actions/runs/40176319651) |
| Integration | PASS, PostgreSQL application/contract tests | [40176319635](https://github.com/jelome265/sitolo/actions/runs/40176319635) |
| Security | PASS, including CodeQL, advisories, dependency policy, secrets and offensive PostgreSQL regression | [40176319631](https://github.com/jelome265/sitolo/actions/runs/40176319631) |
| Policy | PASS | [40176319642](https://github.com/jelome265/sitolo/actions/runs/40176319642) |

These workflow results belong only to `ef0e97b`; a later documentation-only report commit requires its own exact-head status checks.

### R-08 — automated policy enforcement and scope reconciliation

Added `scripts/ci/check-rust-policy.py` to fail closed on workspace lint-policy drift, package lint overrides, Rust lint suppressions without reasons, unsafe production Rust, compiler lint-bypass flags, and removal/filtering of required workspace, Part 8, or RLS test commands. The canonical Unix and PowerShell verifiers run this check and require the database suites. The Policy workflow runs it directly. CI now installs exact pinned `cargo-deny` 0.20.2 and `cargo-audit` 0.22.2 versions through one version-checking installer.

Reconciled `SCOPE-EXC-001`: it records only that the Integration workflow excludes direct `sitolo-persistence` package tests. Its old rationale that database semantics did not exist was obsolete. The Rust canonical verifier and Security workflow retain required PostgreSQL suites; the scope record is not a testing waiver. Branch-protection evidence remains unavailable through the current repository permissions and still requires an administrator.

Local R-08 evidence: `python scripts/ci/check-rust-policy.py`, `cargo deny check`, and `cargo audit` — PASS. `scripts/ci/check-release-workflow` — PASS. The combined workflow-policy shell check could not find `python3` in this Windows Git Bash environment; its Rust policy subcheck passed when run directly with Python. The full workflow-policy check passed in Ubuntu CI.

## Remaining findings and blockers

The following are still open from the cross-phase audit and remediation plan:

- **F-01 / R-02, P0:** no approved concrete business effect, production consumer composition, queue/resource identities, inbox/effect transaction proof, acknowledgement-after-commit, retry/order/DLQ evidence, or approved AWS test environment. Do not wire a no-op or invent the business effect or AWS identifiers. Required action: domain owner and infrastructure/security reviewer approve the exact effect and deployment contract.
- **F-02 / R-03, P0:** no accountable production migration owner or canonical immutable migration/release artifact. Required action: database/platform owner selects and approves the source, owner, role split, release and recovery contract.
- **F-04 / R-04, P0/P1:** the complete mandatory Phase 4 mutation matrix is not yet supported by source-traced, exact-head PostgreSQL success/denial/rollback/replay/concurrency evidence.
- **F-03 / R-05, P1:** organization activation is fail-closed; a product/domain owner must approve persisted lifecycle semantics or explicitly defer/remove the route from the declared API contract.
- **R-06:** Phase 3 production login/session/refresh/MFA/device lifecycle evidence remains incomplete against its full contract.
- **F-06 / R-07:** `sitolo-testkit` remains an unused scaffold; no approved removal/deferment or shared fixture API was completed.
- **F-07 / F-08 / R-08:** automated lint/unsafe policy enforcement and the stale scope-exception reconciliation are implemented. Branch-protection evidence remains outstanding; settings require a repository administrator and prior API access returned 403.
- **F-10:** the independent Stage 06 audit and human review must evaluate the final implementation head. This supplemental report is not independent approval.

No P0/P1 finding is waived by this work. PR #49 is not technically ready for independent closure review until the source-level blockers above have the required implementation/evidence; the final changes are ready for an independent audit of this bounded patch once its exact-head workflows finish.

## Git and preservation

Changes were committed and pushed without force-pushing, merging or closing PR #49. No fake production effect, mock database proof, skipped database suite, or weakened assertion was introduced. A verification report was already modified in the working tree before this task; that working edit was preserved and excluded from both commits.
