---
artifact_type: remediation-execution-report
status: partial-blocked
run_slug: "pr009-part8"
security_relevant: yes
source_head: "4858341fcd16459aad10494d8b75ddb684859861"
date: "2026-10-25"
---

# Phase 1–4 remediation execution — PR #49

## Disposition

Partial implementation completed on the requested feature branch. PR #49 remains open and **not ready for merge or Phase 4 closure**. This report records the bounded remediations implemented in this pass; it does not supersede the historical deep audit or claim that all Phase 1–4 requirements are complete.

The branch was at `a378beec2bc5097bb642825d2e8fcf4c967c105d` before the requested pull. `git pull --ff-only` advanced it to `6662076dcdb911e8ca0d783f8e3fc3ec089f1f28`. Implementation commits include `289ae0c`, `e48b12a`, `ef0e97b`, `468186c`, and lifecycle route disposition `4858341`.

## Completed remediations

### R-01 / F-05 — test database target guard

Both PostgreSQL integration harnesses now validate the explicit `SITOLO_TEST_DATABASE_DISPOSABLE=true` opt-in and parse DSNs before connecting or executing DDL. Admin/runtime targets must use the documented roles, loopback `127.0.0.1:5432`, approved disposable database names, and matching host/port/database. The Part 8 harness additionally validates the worker DSN and role before creating its schema. CI supplies the opt-in only beside its PostgreSQL service. The offensive-security runner and workflow policy checks require the same declaration. No remote-host override was added.

Pure parser tests reject missing opt-in, remote host, mismatched database, unexpected role, production database name, and a conflicting worker database. These are parser-only tests; they do not claim PostgreSQL security behavior. Real database integration remains covered by CI.

### R-09 / F-09 — optional configuration fingerprint values

The fingerprint previously encoded absent optional fields as the literal `<none>`, which could collide with an actual optional value of `<none>`. Optional values now use distinct `none` and length-prefixed `some` encodings. `FINGERPRINT_FORMAT_VERSION` is 2, making the canonical-format change explicit. Regression coverage includes all current effective `AppConfig` fields, OIDC fields, the worker secret reference, secret exclusion, and an absent-versus-present sentinel case.
### R-07 / F-06 — shared tenancy test fixtures

Implemented domain-validated identifier helpers and legal active organization, branch, and membership fixtures in `sitolo-testkit`. Tenancy domain and application service tests now consume these shared fixtures. The crate is a dev-only dependency, and the crate-boundary checker rejects normal dependency edges to it. PostgreSQL transaction, RLS, and audit/outbox behavior continue to use real PostgreSQL fixtures; this testkit does not simulate persistence.

### R-05 / F-03 — remove the unavailable organization activation endpoint

The API contract examples do not declare a separate organization activation route. Phase 4 §7 requires organization, owner membership, policy profile and default branch/configuration to be created atomically and says an organization without a valid owner must not be visible as a normal tenant unless an explicit provisioning subsystem is designed. Current provisioning commits an `ACTIVE` organization with owner membership and default branch; there is no persisted `PROVISIONING` organization for the old route to activate. The route was removed from the active inventory and API surface, with the API contract documenting the atomic onboarding path. The Axum boundary test asserts the removed path returns 404 and the five active protected routes remain covered by missing-credential denial tests. This does not claim that other domain lifecycle transitions are fully covered.

## Evidence

Local checks on source `e48b12a` (R-01/R-09):

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

These workflow results belong only to `ef0e97b`; later commits require their own exact-head status checks.

### R-08 — automated policy enforcement and scope reconciliation

Added `scripts/ci/check-rust-policy.py` to fail closed on workspace lint-policy drift, package lint overrides, Rust lint suppressions without reasons, unsafe production Rust, compiler lint-bypass flags, and removal/filtering of required workspace, Part 8, or RLS test commands. The canonical Unix and PowerShell verifiers run this check and require the database suites. The Policy workflow runs it directly. CI now installs exact pinned `cargo-deny` 0.20.2 and `cargo-audit` 0.22.2 versions through one version-checking installer.

Reconciled `SCOPE-EXC-001`: it records only that the Integration workflow excludes direct `sitolo-persistence` package tests. Its old rationale that database semantics did not exist was obsolete. The Rust canonical verifier and Security workflow retain required PostgreSQL suites; the scope record is not a testing waiver. Branch-protection evidence remains unavailable through the current repository permissions and still requires an administrator.

Local R-08 evidence: `python scripts/ci/check-rust-policy.py`, `cargo deny check`, and `cargo audit` — PASS. `scripts/ci/check-release-workflow` — PASS. The combined workflow-policy shell check could not find `python3` in this Windows Git Bash environment; its Rust policy subcheck passed when run directly with Python. The full workflow-policy check passed in Ubuntu CI.


R-07 local checks on `468186cad7a2ccb2abe8f3305e841ac3f81cd7bd`:

- `cargo test -p sitolo-testkit --locked` — PASS, 2 fixture tests.
- `cargo test -p sitolo-tenancy --locked` — PASS, 8 tests.
- `cargo test -p sitolo-application --test tenancy_service --locked` — PASS, 11 tests.
- `cargo fmt --all -- --check`, workspace Clippy/check, testkit docs, crate boundaries (17 packages), architecture, doc references (245 Markdown files / 1110 references), System Map process checks, and ICM workspace checks — PASS.
- No local PostgreSQL was started; persistence behavior remains covered only by real PostgreSQL CI suites.
- Exact-head GitHub Actions for `468186cad7a2ccb2abe8f3305e841ac3f81cd7bd`:

| Workflow | Result | Run |
|---|---|---|
| Rust | PASS | [40177758892](https://github.com/jelome265/sitolo/actions/runs/40177758892) |
| Integration | PASS, PostgreSQL application/contract tests | [40177758874](https://github.com/jelome265/sitolo/actions/runs/40177758874) |
| Security | PASS, including CodeQL, advisories, dependency policy, secrets and offensive PostgreSQL regression | [40177758868](https://github.com/jelome265/sitolo/actions/runs/40177758868) |
| Policy | PASS | [40177758949](https://github.com/jelome265/sitolo/actions/runs/40177758949) |

These exact-head results apply to `468186c`; the subsequent report commit requires a fresh set of exact-head status checks.

Lifecycle implementation source `4858341` is included in exact documentation head `0f70426ffc384b5f5ae69e3578b672a5777f0eda`. Rust [40178438974](https://github.com/jelome265/sitolo/actions/runs/40178438974), Integration [40178438961](https://github.com/jelome265/sitolo/actions/runs/40178438961), Security [40178438971](https://github.com/jelome265/sitolo/actions/runs/40178438971), and Policy [40178439059](https://github.com/jelome265/sitolo/actions/runs/40178439059) all passed on that exact head. Later report-only commits still require their own exact-head checks.
## Remaining findings and blockers

The following are still open from the cross-phase audit and remediation plan:

- **F-01 / R-02, P0:** no approved concrete business effect, production consumer composition, queue/resource identities, inbox/effect transaction proof, acknowledgement-after-commit, retry/order/DLQ evidence, or approved AWS test environment. Do not wire a no-op or invent the business effect or AWS identifiers. Required action: domain owner and infrastructure/security reviewer approve the exact effect and deployment contract.
- **F-02 / R-03, P0:** no accountable production migration owner or canonical immutable migration/release artifact. Required action: database/platform owner selects and approves the source, owner, role split, release and recovery contract.
- **F-04 / R-04, P0/P1:** the complete mandatory Phase 4 mutation matrix is not yet supported by source-traced, exact-head PostgreSQL success/denial/rollback/replay/concurrency evidence.
- **R-06:** Phase 3 production login/session/refresh/MFA/device lifecycle evidence remains incomplete against its full contract.
- **F-06 / R-07:** shared validated tenancy fixtures are implemented and consumed by domain and application tests. The crate is dev-only and does not replace real PostgreSQL evidence.
- **F-07 / F-08 / R-08:** automated lint/unsafe policy enforcement and the stale scope-exception reconciliation are implemented. Branch-protection evidence remains outstanding; settings require a repository administrator and prior API access returned 403.
- **F-10:** the independent Stage 06 audit and human review must evaluate the final implementation head. This supplemental report is not independent approval.

No P0/P1 finding is waived by this work. PR #49 is not ready to merge or close Phase 4. The bounded changes are ready for an independent review, with the remaining findings and unavailable branch-protection evidence clearly identified.

## Git and preservation

Changes were committed and pushed without force-pushing, merging or closing PR #49. No fake production effect, mock database proof, skipped database suite, or weakened assertion was introduced. A verification report was already modified in the working tree before this task; that working edit was preserved and excluded from both commits.
