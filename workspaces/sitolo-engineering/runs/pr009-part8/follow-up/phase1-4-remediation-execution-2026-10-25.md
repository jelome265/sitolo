---
artifact_type: remediation-execution-report
status: partial-blocked
run_slug: "pr009-part8"
security_relevant: yes
source_head: "15147ef45126917ad4184ca1326282db6631078a"
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

### R-01 extension — shared disposable PostgreSQL guard (commit `18dce7e`)

Follow-up inspection found that the two application PostgreSQL suites and the RLS suite still had separate connection setup paths; the earlier guard covered only Part 8 persistence. Added `sitolo-testkit::postgres` as shared test-only infrastructure and routed the application provisioning/membership suites, persistence Part 8 suite, and persistence RLS suite through it. All three DSNs are parsed and validated before connection or DDL: explicit disposable opt-in, loopback address and port, allowlisted database, expected admin/runtime/worker role, and a common host/port/database target. Generated schema names for the consuming suites are validated against allowlisted prefixes and lowercase UUID suffixes before SQL interpolation. Parser-only negative tests exercise unsafe targets without opening a connection. The security-harness contract now requires the same checks for every PostgreSQL harness.

Local evidence on `18dce7e59c945cedcccf793b2e69b354b9b86e4d`: testkit unit tests **6 passed**; application and persistence integration targets compile with `--no-run`; strict Clippy for the three affected crates, format, crate-boundary, Rust policy, architecture, doc-reference, Phase 2 policy, and diff checks pass. The approved disposable GitHub PostgreSQL integration workflow ran and passed; no local database was started.

Exact-head GitHub Actions for `18dce7e59c945cedcccf793b2e69b354b9b86e4d`:

| Workflow | Result | Run |
|---|---|---|
| Rust, canonical verification and PostgreSQL suites | PASS | [40182743696](https://github.com/jelome265/sitolo/actions/runs/40182743696) |
| Integration, PostgreSQL application/contract tests | PASS | [40182743665](https://github.com/jelome265/sitolo/actions/runs/40182743665) |
| Security, including CodeQL and offensive regression | PASS | [40182743667](https://github.com/jelome265/sitolo/actions/runs/40182743667) |
| Policy | PASS | [40182743657](https://github.com/jelome265/sitolo/actions/runs/40182743657) |

This closes the shared test-target guard gap only. It does not resolve the concrete production consumer/effect, canonical migration/release owner and artifact, full mutation evidence, Phase 3 requirements, independent Stage 06 review or human/branch-protection requirements. PR #49 remains open and not ready for merge or Phase 4 closure.

### R-01 completion extension — API tenancy harness and redacted diagnostics (commit `d8032ef`)

A repository-wide connection scan after the shared-guard change found one remaining real PostgreSQL integration harness at `apps/api/tests/tenancy_api.rs`. It previously opened an admin connection and created a schema before validating its DSNs. The API target now uses the shared testkit, validates all three admin/runtime/worker DSNs and its generated schema before its first connection, and is a dev-only testkit consumer. `DisposableTargets` has a custom `Debug` implementation that does not format connection options or passwords. Parser regressions cover the API schema prefix and verify password redaction.

Local evidence on `d8032eff0422edb9447239ccc13f403f285c00db`: testkit tests **8 passed**; the API tenancy integration target compiled but was not executed locally; focused strict Clippy, formatting, Rust policy and crate-boundary checks passed. No local PostgreSQL server was started.

Exact-head GitHub Actions on `d8032eff0422edb9447239ccc13f403f285c00db`:

| Workflow | Result | Run |
|---|---|---|
| Rust, canonical verification and PostgreSQL suites | PASS | [40190324443](https://github.com/jelome265/sitolo/actions/runs/40190324443) |
| Integration, PostgreSQL application/contract tests | PASS | [40190324475](https://github.com/jelome265/sitolo/actions/runs/40190324475) |
| Security, including CodeQL and offensive regression | PASS | [40190324472](https://github.com/jelome265/sitolo/actions/runs/40190324472) |
| Policy | PASS | [40190324477](https://github.com/jelome265/sitolo/actions/runs/40190324477) |

This establishes the target-safety acceptance gate across the current repository PostgreSQL integration harnesses. It does not establish production consumer delivery, migration/release ownership, complete Phase 4 mutation evidence, Phase 3 completion, independent Stage 06 review or human approval.

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

The Phase 3 and current-status report additions are included in exact documentation head `d53e05297963b5457d44006e236522c4a179d257`. Rust [40179488202](https://github.com/jelome265/sitolo/actions/runs/40179488202), Integration [40179488191](https://github.com/jelome265/sitolo/actions/runs/40179488191), Security [40179488204](https://github.com/jelome265/sitolo/actions/runs/40179488204), and Policy [40179488198](https://github.com/jelome265/sitolo/actions/runs/40179488198) all passed on that exact head. Later report-only commits require fresh exact-head checks.

## Focused test-harness follow-up — `db4fcba`

Added an explicit validator for the generated `test_part8_<32 lowercase hex>` schema identifier and call it before the first PostgreSQL connection or DDL. The regression rejects SQL metacharacters and unrelated identifiers using parser-only input. Local focused test (1), strict Clippy, formatting and diff checks passed. The full Rust verifier, PostgreSQL Integration, Security and Policy workflows all passed on exact code head `db4fcbaa69f12b96971d95d8bc6c2786c978858f`; evidence is in the [verification follow-up](pr49-phase4-part8-verification-follow-up-2026-10-25.md). No local database was started. This closes only the generated-identifier guard subrequirement; the consumer, canonical migration/release, complete mutation evidence, Phase 3 and independent/human review blockers remain open.
### R-06 extension — persisted suspension and security-version denial (commit `23d8e14`)

Extended `postgres_identity_mapping_and_session_authority_fail_closed` with real PostgreSQL coverage for a suspended user's external-identity lookup and active-looking session denial, plus invalidation of an existing session after the persisted user's `security_version` advances. This exercises the production `resolve_external_identity` and `accept_sitolo_session` database functions through `PgRequestIdentityAuthority`; it does not claim a distinct disabled state or complete session lifecycle.

Local evidence: `cargo fmt --all -- --check`, strict Clippy for the affected persistence integration target, and `cargo test -p sitolo-persistence --test part8_postgres_integration_tests --no-run --locked` passed. The integration target was not executed locally. The approved disposable PostgreSQL Integration workflow executed and passed on exact source head `23d8e14aa357dc213fb4e42a80c94c0c6e010445`: Rust [40192070689](https://github.com/jelome265/sitolo/actions/runs/40192070689), Integration [40192070759](https://github.com/jelome265/sitolo/actions/runs/40192070759), Security [40192070695](https://github.com/jelome265/sitolo/actions/runs/40192070695), and Policy [40192070679](https://github.com/jelome265/sitolo/actions/runs/40192070679) all passed. No local PostgreSQL server was started.

### R-09 extension — exhaustive fingerprint field inventory (commit `1743f3a`)

The fingerprint tests now exhaustively destructure every `AppConfig` field. Adding a field to the configuration model therefore fails compilation until the inventory guard is updated, prompting review of both the canonical writer and the per-field fingerprint-change regression. This complements the existing tests for all current non-secret fields, optional `None`/`Some` distinction, reference changes and absence of raw secret values. Secret values remain outside `AppConfig` by contract; configuration stores their references only.

Local evidence on `1743f3a198605e1e50a30776a3366d2e9e9a36f5`: `cargo fmt --all -- --check`, `cargo test -p sitolo-config --locked` (28 tests), `cargo clippy -p sitolo-config --all-targets --all-features --locked -- -D warnings`, and `git diff --check` passed.

Exact-head GitHub Actions for `1743f3a198605e1e50a30776a3366d2e9e9a36f5`: Rust [40193380363](https://github.com/jelome265/sitolo/actions/runs/40193380363), Integration [40193380330](https://github.com/jelome265/sitolo/actions/runs/40193380330), Security [40193380381](https://github.com/jelome265/sitolo/actions/runs/40193380381), and Policy [40193380574](https://github.com/jelome265/sitolo/actions/runs/40193380574) all passed; CodeQL [121169950121](https://github.com/jelome265/sitolo/runs/121169950121) passed. The branch remains not ready to merge; the unresolved blockers listed below are unchanged.

### R-04 extension — real PostgreSQL evidence-write failure injection (commit `15147ef`)

Removed `FailingUnitOfWork` and `FailingTransaction` from the PostgreSQL application integration suite. All existing evidence-failure cases now compose the production `PgTenancyUnitOfWork` with the real `app_runtime` role. Per-test isolated schemas define audit and outbox `BEFORE INSERT` triggers. Dedicated runtime pools identify audit-failure and outbox-failure cases through their PostgreSQL `application_name`; trigger exceptions occur inside the actual transaction. The existing cases assert that business state and any earlier evidence writes are absent after failure. This strengthens rollback proof for the covered mutations but does not complete the entire mutation matrix or independently verify route, authorization, replay and concurrency requirements.

Local evidence on `15147ef45126917ad4184ca1326282db6631078a`: `cargo fmt --all -- --check`, `cargo test -p sitolo-application --test membership_transaction_integration --no-run --locked`, `cargo clippy -p sitolo-application --test membership_transaction_integration --all-features --locked -- -D warnings`, and `git diff --check` passed. The PostgreSQL integration target was not run locally.

Exact-head GitHub Actions for `15147ef45126917ad4184ca1326282db6631078a`: Rust [40195430303](https://github.com/jelome265/sitolo/actions/runs/40195430303), Integration [40195430299](https://github.com/jelome265/sitolo/actions/runs/40195430299), Security [40195430302](https://github.com/jelome265/sitolo/actions/runs/40195430302), and Policy [40195430306](https://github.com/jelome265/sitolo/actions/runs/40195430306) all passed; CodeQL [121176342832](https://github.com/jelome265/sitolo/runs/121176342832) passed. Integration executed these tests in the approved disposable PostgreSQL service.

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

### R-02 / F-01 extension — enforce consumer and producer aggregate ordering (`7f17666`)

The approved relay delivery proposal requires ordered delivery within organization-scoped aggregate groups. A test-first PostgreSQL regression failed on `7aa7abb` because the inbox accepted a later event without its predecessor ([Rust run 40196875334](https://github.com/jelome265/sitolo/actions/runs/40196875334)). Commit `7f176661b205650871d293b185e27715a0caf00c` adds transaction-scoped per-consumer/organization/aggregate serialization and contiguous inbox sequence checks, while preserving exact duplicate detection. Producer sequence allocation, uniqueness, and predecessor blocking now include organization scope; the test fixture enforces tenant RLS for sequence metadata. New coverage checks gaps, concurrent deliveries against PostgreSQL lock state, tenant-scoped sequence allocation, and independent ordering between organizations sharing an aggregate ID. See the [remediation extension](phase1-4-remediation-extension-7f17666.md).

Local format, integration-target compile-only, strict Clippy and diff checks pass. Compile-only is not database proof. Rust [40197666873](https://github.com/jelome265/sitolo/actions/runs/40197666873), Integration [40197666692](https://github.com/jelome265/sitolo/actions/runs/40197666692), Security [40197666922](https://github.com/jelome265/sitolo/actions/runs/40197666922), and Policy [40197666960](https://github.com/jelome265/sitolo/actions/runs/40197666960) passed on exact head `7f176661b205650871d293b185e27715a0caf00c`; Rust canonical verification ran the required persistence PostgreSQL suite. Integration excludes direct persistence tests. The security run included passing CodeQL, advisories, secret scanning and offensive PostgreSQL regression. The documentation reference inventory and ICM security/workspace and mechanical cold-walk checks passed locally. This does not resolve the approved concrete consumer effect, production migration/release ownership, authorized replay/skip, complete mutation evidence, Phase 3 gaps, Stage 06 re-audit or human review. The previously modified verification report was preserved and excluded. See the [remediation extension](phase1-4-remediation-extension-7f17666.md).
