---
artifact_type: remediation-execution-report
status: partial-blocked
run_slug: "pr009-part8"
security_relevant: yes
source_head: "e48b12a5995b4d79aca86a7443b014144d893164"
date: "2026-10-25"
---

# Phase 1–4 remediation execution — PR #49

## Disposition

Partial implementation completed on the requested feature branch. PR #49 remains open and **not ready for merge or Phase 4 closure**. This report records the two bounded remediations implemented in this pass; it does not supersede the historical deep audit or claim that all Phase 1–4 requirements are complete.

The branch was at `a378beec2bc5097bb642825d2e8fcf4c967c105d` before the requested pull. `git pull --ff-only` advanced it to `6662076dcdb911e8ca0d783f8e3fc3ec089f1f28`. The implementation commits are `289ae0c` and `e48b12a`; candidate source SHA is `e48b12a5995b4d79aca86a7443b014144d893164`.

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

Exact-head GitHub Actions observed for `e48b12a5995b4d79aca86a7443b014144d893164`:

| Workflow | Result | Run |
|---|---|---|
| Rust | PASS | [40174232907](https://github.com/jelome265/sitolo/actions/runs/40174232907) |
| Integration | PASS | [40174232901](https://github.com/jelome265/sitolo/actions/runs/40174232901) |
| Policy | PASS | [40174232875](https://github.com/jelome265/sitolo/actions/runs/40174232875) |
| Security | PASS, including CodeQL, advisories, dependency policy, secrets and offensive PostgreSQL regression | [40174232895](https://github.com/jelome265/sitolo/actions/runs/40174232895) |

These workflow results belong only to `e48b12a`; later documentation commits require their own exact-head status checks.

## Remaining findings and blockers

The following are still open from the cross-phase audit and remediation plan:

- **F-01 / R-02, P0:** no approved concrete business effect, production consumer composition, queue/resource identities, inbox/effect transaction proof, acknowledgement-after-commit, retry/order/DLQ evidence, or approved AWS test environment. Do not wire a no-op or invent the business effect or AWS identifiers. Required action: domain owner and infrastructure/security reviewer approve the exact effect and deployment contract.
- **F-02 / R-03, P0:** no accountable production migration owner or canonical immutable migration/release artifact. Required action: database/platform owner selects and approves the source, owner, role split, release and recovery contract.
- **F-04 / R-04, P0/P1:** the complete mandatory Phase 4 mutation matrix is not yet supported by source-traced, exact-head PostgreSQL success/denial/rollback/replay/concurrency evidence.
- **F-03 / R-05, P1:** organization activation is fail-closed; a product/domain owner must approve persisted lifecycle semantics or explicitly defer/remove the route from the declared API contract.
- **R-06:** Phase 3 production login/session/refresh/MFA/device lifecycle evidence remains incomplete against its full contract.
- **F-06 / R-07:** `sitolo-testkit` remains an unused scaffold; no approved removal/deferment or shared fixture API was completed.
- **F-07 / F-08 / R-08:** complete lint-override/unsafe policy inventory, test-scope exception reconciliation and branch-protection evidence remain outstanding. Branch-protection settings require a repository administrator; prior API access returned 403.
- **F-10:** the independent Stage 06 audit and human review must evaluate the final implementation head. This supplemental report is not independent approval.

No P0/P1 finding is waived by this work. PR #49 is not technically ready for independent closure review until the source-level blockers above have the required implementation/evidence; the final changes are ready for an independent audit of this bounded patch once its exact-head workflows finish.

## Git and preservation

Changes were committed and pushed without force-pushing, merging or closing PR #49. No fake production effect, mock database proof, skipped database suite, or weakened assertion was introduced. A verification report was already modified in the working tree before this task; that working edit was preserved and excluded from both commits.
