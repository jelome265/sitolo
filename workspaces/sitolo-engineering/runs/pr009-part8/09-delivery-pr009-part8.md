# Stage 09 Delivery — PR-009 Part 8

## Current handoff checkpoint — 2026-10-25

Current code source is `4858341fcd16459aad10494d8b75ddb684859861`; exact documentation head `7f42a5b674565940220321df452750b5b391da7c` passed all four required workflows (Rust, Integration, Security and Policy; see the current status pointer). The shared testkit fixtures and organization activation route disposition are included. PR #49 remains open and not merge-ready: the approved business effect/consumer runtime proof, canonical production migration owner/artifact, complete mandatory mutation and Phase 3 evidence, branch-protection settings, independent Stage 06 audit and human review remain outstanding. No human check or independent approval is claimed. The next documentation commit requires its own exact-head workflow cycle.

**Date:** 2026-10-22\
**Delivery head (code):** `e49de425b74c0095fee0737612ae40dbc50c770a`\
**Environment/docs head:** `144a28432e67622de53eb7a45f5e2bf8375927b5` (this delivery record is a later documentation-only commit)\
**Verified predecessor:** `862775c5412b4928df3dfac93580cf8b1863aae1` (stage 08 evidence)\
**Head semantics:** the code head is the most recent commit that changes code. Later commits in this handoff are documentation-only; compare code evidence against `e49de42`, not against the PR tip.\
**Disposition:** Review-ready, not merge-ready. Exact-head CI is green at `144a284` (code `e49de42`), the release workflow branch dispatch passed both jobs, F-01 and F-02 remain closed, and the additional follow-up P1 findings are closed. The stage 08 release blockers and human review remain.\
**Human check:** Not performed. A human owns the review and merge decision.

## Follow-up review packet — 2026-10-22

A current post-Stage-08 source review and action plan are recorded alongside this handoff:

- [Follow-up audit](follow-up/audit.md) — current source findings and evidence boundaries.
- [Remediation plan](follow-up/remediation-plan.md) — dependency-ordered closure tasks and acceptance criteria.
- [Continuation guide](follow-up/guide.md) — practical implementation, verification, and reviewer handoff.
- [Decision proposals index](follow-up/decisions/README.md) — proposed choices and owner-approval gates.
- [Relay and consumer contract proposal](follow-up/decisions/relay-delivery-contract-proposal.md)
- [IAM lifecycle event-mapping proposal](follow-up/decisions/iam-lifecycle-event-mapping-proposal.md)
- [Identity, production runtime and secret-provider proposal](follow-up/decisions/identity-production-runtime-proposal.md)

The repository owner approved the AWS relay/identity/secrets baseline and lifecycle mappings on 2026-10-22. The goal is no longer blocked waiting for those three decisions; implementation, integration evidence, deployment safeguards, re-audit and human review remain outstanding. PR #49 remains open and unmerged.

## Summary

This handoff carries the PR-009 Part 8 audit/outbox implementation plus a final CI-policy increment applied after stage 08 verification.

The increment installs `ripgrep` in the `verify` job. That install immediately exposed a gate that had never executed in this repository's CI: `check-architecture` and `check-phase2-policy` had been reporting PASS while their searches returned nothing. `check-phase2-policy` then failed on ten `DATABASE_URL` aliases in three PostgreSQL test harnesses.

The first attempt scoped the rule to exclude `tests/`, which turned the gate green but exempted an entire subtree. `891c273` replaces that with the stricter fix: the harness fallbacks are deleted, the exemption is removed, and both policy scripts are restored byte-identical to `main`.

A static review then found that the runtime pool did not authenticate as its restricted role: it logged in with the administrator and ran `SET ROLE app_runtime`, which any session can reverse. `c090809` removes that mechanism. The runtime and worker pools now authenticate directly as the `app_runtime` and `app_worker` LOGIN roles, `verify_runtime_role` asserts `current_user` and `session_user`, and a new negative test proves `RESET ROLE` is a no-op and `SET ROLE postgres` is denied. `67315a4` updates the one test that still asserted the old `NOLOGIN` role shape.

A second review found that the RLS verifier was bound to the Part 7 fixture while its name claimed general coverage: it required `tenant_resources` and one `{table}_isolation_policy` per table, neither of which the Part 8 production DDL creates. `3056da6` separates the two proofs. The Part 7 verifier is explicitly bound to `tests/fixtures/rls_schema.sql`; a new Part 8 verifier checks the three production tables, the two operation-specific policies per table, and the SELECT + INSERT grant decision. `7d892eb` adds violation diagnostics after the first CI run failed on an unstated policy name, and `52c7392` binds the verifier to the DDL's actual `memberships_provision_*` names. The production DDL itself changes only in its header comment; no table, policy, or grant is added to satisfy a verifier.

A third follow-up review found four additional P1 gaps, all closed in this handoff:

- **Secret/config namespace collision.** The development secret value used `SITOLO__DATABASE__PASSWORD`, the same prefix as the canonical configuration contract. The configuration loader correctly rejects unknown `SITOLO__` keys, so the secret value could not coexist with that contract. `e49de42` moves every development secret value to the disjoint `SITOLO_SECRET__` namespace (`SITOLO_SECRET__DATABASE__PASSWORD` and the other class names). Unknown `SITOLO__` keys still fail closed, production still rejects the development provider, and process-environment tests cover both the config layer and the full API bootstrap.
- **Part 8 verifier accepted tenant-blind predicates.** The F-02 fix bound the verifier to the DDL but retained substring helpers: `id IS NOT NULL` satisfied the SELECT helper, and `id <> ''` satisfied the INSERT helper. `e49de42` replaces those helpers with exact per-table, per-operation canonical predicates after renderer-only normalization. Two new PostgreSQL negative tests prove the verifier rejects those previously accepted policies for all three tables.
- **Release job had no database environment.** `build_release_artifact` ran `cargo test --workspace` without a PostgreSQL service or the three scoped DSNs. GitHub Actions jobs do not share service containers or step environments, so that step would abort before packaging. `e49de42` gives the release job its own PostgreSQL service and job-level `ADMIN_DATABASE_URL`, `RUNTIME_DATABASE_URL`, and `WORKER_DATABASE_URL`, and makes the release `verify` job install the tools the canonical verifier requires.
- **No environment reference.** `144a284` adds `.env.example`, `.env.test.example`, and `docs/environment-configuration-reference.md`. They are derived from the typed configuration contract and document process-environment injection. No dotenv dependency or automatic loader is added.

Two smaller changes close the dependency surface: unused license allowances are removed, and no advisory is suppressed or ignored.

No production runtime composition, relay transport, or authenticated API authority is delivered. PR #49 remains open.

## Change inventory

### This delivery increment (`862775c..52c7392`)

| Commit | Change | Why | Files |
|---|---|---|---|
| `fd50564` | Install `ripgrep` before the canonical verifier | The `verify` job never installed it. `check-architecture` and `check-phase2-policy` call `rg` directly and wrap every scan in `|| true`, so a runner without `rg` produced empty output, printed PASS, and exited 0. Stage 08 recorded `rg: command not found` for both. | `.github/workflows/rust.yml` (+4) |
| `e66c73a` | Scope the database-alias rule to runtime source | With `rg` present the check failed on ten `DATABASE_URL` occurrences across three test harnesses. Superseded by `891c273`, which removes the aliases instead of exempting them. | `scripts/ci/check-phase2-policy` (+4/−1), `scripts/ci/check-phase2-policy.ps1` (+7/−2) |
| `2ff9abe` | Remove license allowances no dependency uses | `Cargo.lock` resolves to 191 packages and none declares ISC, MPL-2.0, or Unicode-DFS-2016. `unicode-ident` 1.0.24 moved to `(MIT OR Apache-2.0) AND Unicode-3.0`, which stays allowed. | `deny.toml` (−3) |
| `891c273` | Remove the `tests/` exemption and the aliases it permitted | The exemption was broader than the rule's rationale and was written by the author who needed it. Neither workflow relies on the fallback, so the aliases could be deleted and both scripts restored unchanged. Net effect: the PR no longer modifies the guardrail at all. | 3 PostgreSQL test harnesses; `scripts/ci/check-phase2-policy` and `.ps1` restored (`+14/−26` across 5 files) |
| `c090809` | Authenticate the runtime and worker pools as their restricted LOGIN roles | F-01: `SET ROLE` is reversible for the remainder of the session, so logging in as the administrator and dropping privileges is not a boundary. The fixture now creates `app_runtime` and `app_worker` as LOGIN; `verify_runtime_role` asserts `current_user` and `session_user`; a negative test proves `RESET ROLE` is a no-op and `SET ROLE postgres` is denied. `app_outbox_owner` stays NOLOGIN. | `postgres.rs`, the Part 8 fixture, three PostgreSQL harnesses, `rust.yml`, `integration.yml`, `artifact.yml` (8 files, +125/−59) |
| `b61506d` | Re-pin and rewrite the PostgreSQL authority process card | The System Map card recorded `postgres.rs` at `bd2f60a`, so `check-icm-workspace` failed as soon as the pool construction changed. | `map/processes/postgres-authority-initialization.md` (+8/−8) |
| `67315a4` | Update the role-shape assertion for direct login | The hardening test still asserted `app_runtime`/`app_worker` NOLOGIN, which encoded the old model. It now asserts LOGIN for the client roles and NOLOGIN for `app_outbox_owner`. | `part8_postgres_integration_tests.rs` (+13/−1) |
| `3056da6` | Bind each RLS verifier to its authoritative schema | F-02: the verifier required `tenant_resources` and one `{table}_isolation_policy` per table, neither of which the Part 8 production DDL creates. The Part 7 verifier is now explicitly bound to `tests/fixtures/rls_schema.sql`; a new Part 8 verifier checks the three production tables, two policies each, INSERT → `WITH CHECK` only, SELECT → `USING` only, `app_runtime` only, and SELECT + INSERT grants only. The DDL header is corrected; no table, policy, or grant is added to satisfy a verifier. | `postgres.rs`, `phase4_part8_tenancy_provisioning.sql`, `rls_security_tests.rs`, `part8_provision_organization_evidence_tests.rs` (+155/−6) |
| `7d892eb` | Report the failing Part 8 RLS contract check | The first F-02 CI run returned a bare `SecurityViolation`, so the log could not identify the table or policy. The checks are unchanged; each failure now prints the specific violation. | `postgres.rs` (+78/−24) |
| `52c7392` | Bind the Part 8 policy names to the DDL | The DDL names the membership policies `memberships_provision_*`, not `organization_memberships_provision_*`. The verifier now carries an explicit table-to-prefix map instead of imposing a naming convention on the authoritative schema. | `postgres.rs` (+11/−3) |

`299d96b` and `ea3ece2` (docs only) sit between the verified predecessor and this increment and change no code. `e0a146d`, `c11ca45`, and `5653591` are System Map re-pins for `3056da6`, `7d892eb`, and `52c7392`; they change no code either.

### Follow-up increment (`7fc6d86..144a284`)

| Commit | Change | Why | Files |
|---|---|---|---|
| `e49de42` | Separate secret values from configuration; harden the Part 8 verifier; repair the release workflow | The development provider read a value under `SITOLO__`, which the canonical loader rejects as unknown configuration. The Part 8 verifier still used substring helpers that accepted tenant-blind expressions. The release job had no PostgreSQL service or DSNs, and the release verify job did not install ripgrep, cargo-deny, or cargo-audit. | 13 files, +647/−41 |
| `144a284` | Add environment examples and reference; update the runtime and Phase 2 docs; re-pin the PostgreSQL authority map | The environment contract had no maintainable reference, and the secret namespace change needed to be documented together with the code. The map source moved to `e49de42`. | 8 files, +281/−11 |

The code head is `e49de42`; `144a284` changes documentation, examples, and the System Map pin only. This delivery record also changes documentation only.

### Inherited from the verified predecessor

The audit/outbox implementation, PostgreSQL tenancy provisioning transaction, RLS suites, worker outbox engine, and the SQLx dependency remediation are delivered as verified at `862775c` and recorded in `08-verification-pr009-part8.md`. This handoff does not re-summarize them.

## Governing contract

| Source | Scope |
|---|---|
| `docs/phase4_part8_pr009_remediation_execution_contract.md` | PR-009 Part 8 scope and required evidence |
| `docs/phase4_part8_pr009_current_status.md` | Current Part 8 completion position |
| `docs/phase2_config_secrets_logging_errors_telemetry_implementation.md` §"Avoid a proliferation of aliases" | Prohibits generic database-URL aliases in application configuration |
| `docs/phase2_postgresql_runtime_configuration_persistence_boundary_implementation.md` §"Keep canonical `SITOLO__DATABASE__*` parsing" | Canonical database identity; no `DATABASE_URL` configuration alias |
| `agent.md` | Engineering authority for delivery and review rules |

The alias rule is not relaxed by this delivery. An earlier attempt scoped the rule to runtime source; it was withdrawn in `891c273` in favour of deleting the harness aliases. Both policy scripts are now byte-identical to `main`, and the ban applies to every path.

## Verification evidence

### Exact-head CI at `144a284` (code at `e49de42`) — 10/10 checks pass

| Check | Workflow | Result | Link |
|---|---|---|---|
| `verify` | rust | PASS, 2m12s | [job 119578784192](https://github.com/jelome265/sitolo/actions/runs/39703748028/job/119578784192) |
| `toolchain` | rust | PASS, 26s | [job 119578783698](https://github.com/jelome265/sitolo/actions/runs/39703748028/job/119578783698) |
| `application and contract tests with PostgreSQL` | integration | PASS, 1m22s | [job 119578784150](https://github.com/jelome265/sitolo/actions/runs/39703748021/job/119578784150) |
| `advisories` | security | PASS, 32s | [job 119578784924](https://github.com/jelome265/sitolo/actions/runs/39703748184/job/119578784924) |
| `dependency-policy` | security | PASS, 31s | [job 119578784913](https://github.com/jelome265/sitolo/actions/runs/39703748184/job/119578784913) |
| `secrets` | security | PASS, 9s | [job 119578784737](https://github.com/jelome265/sitolo/actions/runs/39703748184/job/119578784737) |
| `codeql` | security | PASS, 2m51s | [job 119578785013](https://github.com/jelome265/sitolo/actions/runs/39703748184/job/119578785013) |
| `offensive-regression` | security | PASS, 1m25s | [job 119578784862](https://github.com/jelome265/sitolo/actions/runs/39703748184/job/119578784862) |
| `policy` | policy | PASS, 19s | [job 119578783627](https://github.com/jelome265/sitolo/actions/runs/39703748023/job/119578783627) |
| `CodeQL` | code scanning | PASS, 2s | [run 119579670814](https://github.com/jelome265/sitolo/runs/119579670814) |

The evidence tip is `144a284` (environment/docs and the System Map re-pin); the last code change is `e49de42`, and every check above ran on the same tree. This delivery record is a later documentation-only commit; the final PR tip is covered by the same check set in PR #49.

The `verify` job log is the load-bearing evidence for this increment:

- The full API bootstrap suite passes **2 tests**: `process_environment_config_and_secret_coexist ... ok` and `unknown_configuration_key_still_fails_closed ... ok`.
- The configuration-layer process-environment suite passes **2 tests**: `configuration_and_secret_value_namespaces_coexist ... ok` and `unknown_configuration_key_still_fails_closed ... ok`.
- The Part 8 application evidence suite passes **7 tests**, including `part8_verifier_rejects_select_policies_without_tenant_context ... ok` and `part8_verifier_rejects_insert_policies_without_tenant_context ... ok`.
- The Part 8 PostgreSQL suite passes **23 tests**, and the dedicated RLS suite passes **28 tests**.
- The log contains `Architecture check PASSED` and `Phase 2 policy check PASSED`, so both policy gates executed real searches.

The `integration` job independently passes the same Part 8 suite at **7 tests**, including both negative verifier tests. The `policy` job contains `Release workflow check PASSED: verify installs its tools; build_release_artifact has PostgreSQL and scoped DSNs`, `Workflow policy check PASSED`, the System Map source check, and the documentation-reference check (213 Markdown files, 985 resolved references, 0 unresolved/ambiguous/escaping errors).

### Release workflow dispatch

`gh workflow run artifact.yml --ref feat/phase4-pr009-audit-outbox` dispatched [run 39703940077](https://github.com/jelome265/sitolo/actions/runs/39703940077) at `144a284`. It passed both jobs:

| Job | Result | Link |
|---|---|---|
| `verify` | PASS, 5m35s | [job 119579422265](https://github.com/jelome265/sitolo/actions/runs/39703940077/job/119579422265) |
| `build_release_artifact` | PASS, 2m13s | [job 119581205259](https://github.com/jelome265/sitolo/actions/runs/39703940077/job/119581205259) |

The branch run skipped `Create release record before SBOM publication` as designed. It ran the exact-source build, applied the production DDL in the release job's own PostgreSQL service, ran the workspace tests (Part 8 evidence **7 passed**, Part 8 PostgreSQL **23 passed**, RLS **28 passed**), packaged the artifact, generated an SPDX SBOM, attested provenance, and retained the artifact. This is the execution check for the release path that the policy job validates structurally; it is a branch dispatch, not a tagged production release. The release-record and publication steps remain part of a tagged release.

### Local verification at `e49de42` (Windows host, ripgrep 15.2.0)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets --all-features --locked` | PASS, exit 0 |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS, exit 0 |
| `cargo test -p sitolo-config -p sitolo-security --all-targets --all-features --locked` | PASS (20 config unit tests, 2 process-environment config tests, security provider tests) |
| `cargo test -p sitolo-persistence --lib --all-features --locked` | PASS (35 unit tests, including the policy-normalization tests) |
| `cargo test -p sitolo-api-bin --test startup_env --all-features --locked` | PASS (2 full-bootstrap tests) |
| `check-architecture` (bash, forward-slash shim) | PASS |
| `check-phase2-policy` (bash, forward-slash shim) | PASS |
| `check-phase2-policy.ps1` | PASS |
| `check-workflow-policy` (includes the release-workflow check) | PASS |
| `check-release-workflow` | PASS |
| `check-icm-workspace` (bash) | PASS |
| `check-doc-references` | PASS (213 Markdown files, 986 resolved references, 0 errors) |
| Alias scan with **no exclusion** | 0 matches |

The temporary `rg` binary on this host had been purged between runs. Because the bash scripts wrap every scan in `|| true`, they printed PASS while the binary was missing. Restoring ripgrep 15.2.0 and placing the forward-slash path shim first in `PATH` produced genuine searches and the PASS results above. This is a host artefact, not a repository state; the CI runner installs ripgrep explicitly.

`cargo metadata --locked`, `cargo deny check`, and `cargo audit` were last run locally at `e66c73a`. No manifest and no lockfile changed between `e66c73a` and `e49de42`; `cargo check --locked` passes at the new head, and the `dependency-policy`, `advisories`, and `integration` jobs cover those tools at exact head.

### Not run locally, and why

- **`cargo test` at `e49de42`:** not run as one workspace command locally. The non-PostgreSQL config, security, persistence, and API bootstrap suites were run individually; the full workspace suite is covered by the exact-head `verify` job above.
- **PostgreSQL execution:** NOT RUN locally. No server, `psql`, container runtime, or database URLs exist on this host. PostgreSQL evidence comes exclusively from the exact-head `integration` and `verify` jobs. That includes the direct-login identity proof, the Part 8 verifier against the production DDL, and both negative verifier tests.
- **`cargo audit` at `e49de42`:** not run locally (not installed). The `advisories` job covers it at exact head.
- **Release workflow execution:** the workflow was dispatched on GitHub at `144a284`; both jobs passed, but it cannot be executed locally.
- **`scripts/ci/verify` end to end:** not executed as one command locally, because it requires `cargo-audit` and a Linux shell. Its constituent checks were run individually as listed.

## Audit status

All stage 06 findings remain closed as recorded in `07-remediation-pr009-part8.md` and re-verified in `08-verification-pr009-part8.md`.

**Audit history and current state:** this increment was produced after the original Stage 08 verification, whose evidence is pinned to `862775c`; the original Stage 06 pass did not cover these later changes. This packet now includes a follow-up audit against code head `e49de42` in [follow-up/audit.md](follow-up/audit.md), plus the [remediation plan](follow-up/remediation-plan.md) and [continuation guide](follow-up/guide.md). The follow-up report is a static engineering review, not human approval or production certification; human review remains pending.

### Static audits received after stage 08

Two independent static reviews of `e66c73a` were received after this increment was drafted. Neither executed code, and neither approves or merges the PR.

| Finding | Status |
|---|---|
| F-04 / AUD-CFG-002 — the alias ban exempted all `tests/` paths | **Closed by `891c273`.** The exemption and the aliases it permitted are both gone; both scripts are byte-identical to `main`. |
| F-01 / AUD-SEC-001 — runtime privilege drop via `SET ROLE` is not a boundary | **Closed by `c090809`.** The runtime and worker pools authenticate as their own LOGIN roles; `verify_runtime_role` asserts `current_user` and `session_user`; `test_runtime_pool_identity_cannot_escalate` proves `RESET ROLE` is a no-op and `SET ROLE postgres` is denied. `67315a4` updates the obsolete NOLOGIN assertion. PostgreSQL CI passes at `52c7392`. |
| F-02 / AUD-SEC-002 — the production Part 8 DDL contradicts the RLS verifier | **Closed at `3056da6`, corrected in `52c7392`.** The Part 7 verifier is explicitly bound to `tests/fixtures/rls_schema.sql`; a new Part 8 verifier checks the production DDL's three tables, two operation-specific policies per table, INSERT → `WITH CHECK` only, SELECT → `USING` only, `app_runtime` only, and SELECT + INSERT grants only. The application evidence test applies the production DDL and runs the verifier; PostgreSQL CI passes at `52c7392` with no `Part 8 RLS contract violation`. The DDL header is corrected; no table, policy, or grant was added to satisfy a verifier. |
| F-09 — the `deny.toml` license narrowing may break the gate | **Closed empirically.** `cargo deny check` passes at `e66c73a`; the `dependency-policy` job passes at `e49de42`. |

The follow-up review added four P1 findings. None was a P0. One closes a residual weakness in the F-02 verifier without reopening the DDL/verifier divergence finding; the others are independent:

| Follow-up finding | Status |
|---|---|
| Secret values lived under the configuration prefix, so the config loader rejected them as unknown keys | **Closed by `e49de42`.** Development secret values use `SITOLO_SECRET__...`; ordinary configuration remains `SITOLO__...`; unknown keys still fail; the production provider guard is unchanged. Process-environment and full-bootstrap tests cover the boundary. |
| Part 8 verifier substring helpers accepted tenant-blind expressions | **Closed by `e49de42`.** The verifier compares exact per-table, per-operation canonical predicates after renderer-only normalization. Negative PostgreSQL tests now prove that `id IS NOT NULL` SELECT and `id <> ''` INSERT policies fail for all three tables. This closes the residual weakness in the F-02 fix; the DDL and policy set are unchanged. |
| `build_release_artifact` had no PostgreSQL service or DSNs, and the release `verify` job did not install the verifier's tools | **Closed by `e49de42`.** Both jobs now have a PostgreSQL service; the release job has all three scoped DSNs; the release verify job installs ripgrep, cargo-deny, and cargo-audit. `check-release-workflow` enforces the structure in the policy job, and branch dispatch 39703940077 passed both jobs. |
| No environment examples or reference existed for the typed configuration contract | **Closed by `144a284`.** `.env.example`, `.env.test.example`, and `docs/environment-configuration-reference.md` document process-environment injection; no loader or dotenv dependency is added. |

The security-control positions recorded in stage 08 are unchanged for the tenancy, audit, and worker behaviour. This increment changes the database connection identity and the RLS verification boundary, which the evidence above now covers:

| Control | Position |
|---|---|
| SC-002 — server-derived tenant scope | PARTIAL; API tenancy route still lacks authenticated principal context |
| SC-006 — transaction integrity and idempotency | PASS for the tested organization-provisioning path only |
| SC-009 — durable audit evidence | PASS for the exercised application transaction path |
| SC-010 — bounded/fail-closed behavior | PARTIAL; no real destination adapter or production worker runtime |
| SC-011 — dependency integrity | RESOLVED; no advisory suppressed, no exception added |

## Migrations and deployment impact

- **Schema:** no migration. `crates/sitolo-persistence/schema/phase4_part8_tenancy_provisioning.sql` keeps the same tables, policies, and grants; only its header comment is corrected to state the SELECT + INSERT decision. The new Part 8 verifier binds to that file.
- **Runtime:** the runtime pool no longer issues `SET ROLE`. `RUNTIME_DATABASE_URL` must authenticate as `app_runtime` and `WORKER_DATABASE_URL` as `app_worker`, each with its own credentials. Deployment must provision both as LOGIN with `NOINHERIT`, `NOSUPERUSER`, `NOCREATEDB`, `NOCREATEROLE`, `NOREPLICATION`, and `NOBYPASSRLS`, and reserve the administrative identity for provisioning and migration. This is a breaking configuration change for any environment that still supplies the administrator DSN for the runtime pool; `verify_runtime_role` fails closed rather than falling back.
- **Tests:** the three PostgreSQL harnesses now require `ADMIN_DATABASE_URL`, `RUNTIME_DATABASE_URL`, and `WORKER_DATABASE_URL`, and the fixture creates the client roles as LOGIN before the pools connect. The application evidence test applies the production DDL and calls `verify_part8_provisioning_rls_metadata` immediately afterwards, so every PostgreSQL run checks the Part 8 contract against the file it applies. `rust.yml`, `integration.yml`, and `artifact.yml` export the split DSNs.
- **Release workflow:** `artifact.yml` now gives both `verify` and `build_release_artifact` their own PostgreSQL service and scoped DSNs. Jobs do not share service containers or step environments, so the release job cannot inherit the verification job's database. The release `verify` job installs `ripgrep`, `cargo-deny`, and `cargo-audit`; `scripts/ci/check-release-workflow` enforces the wiring in the policy job. Branch dispatch 39703940077 passed both jobs.
- **Local secret input:** rename any local `SITOLO__DATABASE__PASSWORD` to `SITOLO_SECRET__DATABASE__PASSWORD`. The old key is now rejected as unknown configuration; the new key is read only by the development provider. `.env.example` documents a placeholder, and no automatic `.env` loader is added.
- **Environment reference:** `.env.example` covers the canonical application keys and the development secret placeholder; `.env.test.example` covers the three PostgreSQL harness DSNs. `docs/environment-configuration-reference.md` states that values are injected through the process environment, not loaded from a file.
- **CI:** the `rust` and release `verify` jobs install `ripgrep` via `apt-get`, adding roughly ten seconds and a package dependency to those runner images. If the Ubuntu image ever ships `ripgrep` preinstalled, the step becomes redundant but harmless.
- **Policy:** `deny.toml` now rejects ISC- and MPL-2.0-licensed dependencies. No current dependency requires them.

## Known limitations

1. **Search failure still passes silently.** Every scan wraps `rg` in `|| true`, which tolerates exit code 1 (no matches) but also swallows exit code 2 (search error). A malformed pattern, unreadable path, or broken `rg` invocation would still report PASS. This increment closes the *missing binary* case only.
2. **Bash policy scripts misfire on Windows.** `check-architecture` and `check-phase2-policy` filter with `grep -vE 'crates/sitolo-…'`, which cannot match `crates\sitolo-…`. Under Git Bash on Windows both report false violations. CI is Linux and is unaffected, but `scripts/ci/verify` is unreliable on a Windows host.
3. **Stage 08 release blockers stand unchanged:**
   - the relay response does not name a transport, endpoint/topic/queue, payload contract, acknowledgement behavior, or runtime configuration; `apps/worker/src/main.rs` refuses to start without an approved adapter;
   - the live HTTP command path still lacks authenticated principal-derived owner/actor authority;
   - no real destination adapter or production worker runtime exists.
4. **Human review remains pending.** The post-Stage-08 follow-up audit is linked above; it does not substitute for an authorized human review or the merge decision.
5. **The release path has a successful branch-dispatch proof, not a tagged release.** `artifact.yml` was dispatched at `144a284`; both jobs passed (5m35s and 2m13s), including the exact-source test step in the release job's own PostgreSQL service. A tagged release would still execute the release-record and publication steps. The static `check-release-workflow` check is structural; the dispatch is the execution evidence.
6. **Development secret values are process-environment values, not a production secret store.** `SITOLO_SECRET__...` is read only by the development provider. Production still rejects that provider and has no managed adapter, so production startup fails closed rather than reading these values.
7. **No production readiness, certification, or complete validation is claimed.** The evidence proves the listed checks at the listed heads and nothing more.

## Rollback or disable considerations

No data migration is required. Every change is CI, policy, map, test, connection-identity, or verifier configuration. Reverting `c090809` restores the previous login-as-administrator composition, and reverting `3056da6` restores the conflated verifier; neither is recommended.

| Revert target | Effect | Caution |
|---|---|---|
| `fd50564` | Returns the `verify` job to not installing `ripgrep` | Not recommended: restores the silent-pass condition, so `check-architecture` and `check-phase2-policy` stop executing while still reporting PASS |
| `891c273` | Reinstates the `tests/` exemption and the harness `DATABASE_URL` fallbacks | Not recommended: re-widens the alias ban to every test path and restores ten aliases |
| `e66c73a` | Nothing against the current tree: `891c273` already reverted its script edits and removed the aliases | Retained for history only |
| `2ff9abe` | Restores ISC, MPL-2.0, Unicode-DFS-2016 allowances | Harmless today; re-widens the license gate to licenses nothing uses |
| `c090809` | Restores login as the administrator plus `SET ROLE app_runtime`/`app_worker` | Not recommended: reintroduces F-01 and removes the `current_user`/`session_user` proof |
| `b61506d` | Re-pins the System Map card to `bd2f60a` | Documentation only, but breaks `check-icm-workspace` once `c090809` is present |
| `67315a4` | Restores the obsolete NOLOGIN assertion | The test would fail against the current LOGIN fixture; do not revert alone |
| `3056da6` | Restores the single verifier bound to the Part 7 fixture | Not recommended: reintroduces F-02, and the Part 8 DDL would again have no verifier |
| `7d892eb` | Restores the bare `SecurityViolation` error | Diagnostics only; behaviour is unchanged |
| `52c7392` | Restores the `{table}_provision_*` name assumption | The verifier would reject the DDL's `memberships_provision_*` policies; do not revert alone |
| `e49de42` | Restores secret values under `SITOLO__`, the substring verifier, and a release job without PostgreSQL | Not recommended: reopens all four follow-up P1 findings |
| `144a284` | Removes the environment examples/reference and reverts the System Map pin | Not recommended alone: loses the environment contract and breaks `check-icm-workspace` |

`891c273` supersedes `e66c73a`. Reverting only `891c273` would restore both the exemption and the aliases, which is the opposite of what both reviews recommended. `c090809` is the F-01 fix; do not revert it without replacing the authority mechanism and the identity assertions together. `3056da6` is the F-02 fix and `52c7392` corrects its policy-name binding; revert them together only if replacing the Part 8 verifier. `e49de42` is the follow-up fix for the four P1 findings; it must not be reverted together with `144a284`, which only documents the new contract and re-pins the map.

## Integration decision

PR #49 is `OPEN` and `BLOCKED` on `REVIEW_REQUIRED`; its code head is `e49de42` and its environment/docs head is `144a284`. This delivery record is a later documentation-only commit.

Stage 09 completion does not authorize merge. The blocking items are human review and the remaining implementation blockers (real relay adapter/delivery proof, authenticated API authority, full transaction/event-registry coverage, and production composition), plus re-audit of the eventual implementation changes. The release-artifact database gap is closed, and branch dispatch 39703940077 passed both jobs. No P0 static-audit finding remains open: F-01 and F-02 remain closed, and the four follow-up P1 findings are closed. Merge remains a human decision.

## Current PR check and disposition addendum — 2026-10-22

After this delivery snapshot, PR #49 advanced to documentation commit `bf6b7b04edc5de9fb5fe459d72d0d42d7755814b`. A direct check-run query for that exact commit reports **10/10 checks completed successfully**: PostgreSQL integration [run 39706516578](https://github.com/jelome265/sitolo/actions/runs/39706516578), policy [run 39706516527](https://github.com/jelome265/sitolo/actions/runs/39706516527), Rust toolchain and verification [run 39706516488](https://github.com/jelome265/sitolo/actions/runs/39706516488), and security checks [run 39706516531](https://github.com/jelome265/sitolo/actions/runs/39706516531), plus the separate [CodeQL check](https://github.com/jelome265/sitolo/runs/119588864768). Every check's `head_sha` equals `bf6b7b04edc5de9fb5fe459d72d0d42d7755814b`.

The PR remains `OPEN`, `REVIEW_REQUIRED`, and `BLOCKED` for merge. The current successful checks do not close the independent follow-up findings AUD-01 through AUD-05 in [the follow-up audit](follow-up/audit.md). In particular, the reviewed code head remains `e49de42`; the newer commits change documentation only. The owner-approved relay/consumer contract still needs its adapter and delivery evidence; server-derived API authority, full transaction coupling, event registry/schema updates, and managed production secret-provider composition remain outstanding.

The earlier sentence in this record that says “the four follow-up P1 findings are closed” refers to the separate static-review increment summarized above; it does not refer to AUD-01 through AUD-05, which were raised by the later follow-up audit and remain open.


## Owner-approval status update — 2026-10-22

The repository owner explicitly instructed that the three decision records be labelled **OWNER-APPROVED** on this date. The approval scope is the proposals as written:

- SNS FIFO topic → separate SQS FIFO queues for the main and sub-consumers, at-least-once delivery, durable consumer idempotency, per-aggregate ordering, explicit acknowledgements and bounded retry/quarantine.
- Five lifecycle mappings: `iam.membership.pending_acceptance`, `iam.membership.expired`, `iam.invitation.issued`, `iam.invitation.withdrawn`, and per-invitation `iam.invitation.expired` for the scheduled sweep.
- Cognito behind Sitolo's OIDC provider boundary; ECS Fargate, RDS PostgreSQL, and a restricted Secrets Manager-backed provider with separate workload roles.

See [decision index](follow-up/decisions/README.md), [relay contract](follow-up/decisions/relay-delivery-contract-proposal.md), [event mappings](follow-up/decisions/iam-lifecycle-event-mapping-proposal.md), and [identity/runtime contract](follow-up/decisions/identity-production-runtime-proposal.md).

**Current goal status:** the decision-selection blocker is **closed by repository-owner approval**. Engineering goal remains **BLOCKED on implementation and evidence**: real relay adapter and consumer integration tests; authenticated principal-derived API authority; atomic business/audit/outbox coverage for all mandatory IAM paths; the approved event mappings wired into the binding contract and source catalogue; and production runtime/secret-provider composition. The docs do not authorize a production rollout or PR merge.

Approval is attributed to the repository owner; no separate signatures from distinct platform, security or domain role holders are claimed unless those roles are delegated to the repository owner. PR #49 remains open and unmerged pending implementation, exact-head verification, re-audit, and human review.

## Implementation and verification update — 2026-10-22

This addendum supersedes the older code-head and verification statements above where they conflict. The branch head is `7752636695576ad49d57175eb0d9a114fd911539`. All 10 checks passed for this exact head: [PostgreSQL integration](https://github.com/jelome265/sitolo/actions/runs/39721887375), [Rust verification](https://github.com/jelome265/sitolo/actions/runs/39721886981), [policy](https://github.com/jelome265/sitolo/actions/runs/39721886982), [security](https://github.com/jelome265/sitolo/actions/runs/39721887010), and [CodeQL](https://github.com/jelome265/sitolo/runs/119637951175).

The implementation now couples the `mark_member_pending` and membership-expiry transitions with their audit/outbox writes in one tenant-scoped PostgreSQL transaction. The PostgreSQL event-name constraints accept the approved underscore-containing lifecycle name. Invitation objects erase contact and token-hash fields on acceptance, withdrawal and expiry, and the in-memory token lookup entry is removed at terminalization. Local PostgreSQL execution was not performed; the database-backed CI suite is the evidence for these code paths.

Disposition remains **AUD-03 OPEN** and **AUD-04 PARTIAL**. Other mandatory IAM state changes remain outside authoritative PostgreSQL transactions; invitation issuance, withdrawal and sweep do not yet have authoritative PostgreSQL storage and event wiring. The approved retention behavior is covered for the in-memory lifecycle, but end-to-end PostgreSQL invitation storage/retention remains work. AUD-01, AUD-02 and AUD-05 also remain open. PR #49 is open and unmerged. Exact-head re-audit and human review are still required before merge.

## Exact-head implementation correction — 2026-10-22

This section supersedes the preceding implementation addendum where it says invitation issuance, withdrawal, or expiry sweeping lacks PostgreSQL persistence/event wiring. At implementation head `8a4a58bb55512343eeb31056a83610a34bc57b00`, those paths have tenant-scoped PostgreSQL transactions coupling invitation/membership changes with audit and outbox writes. The bounded expiry sweep deletes terminal contact/token-hash data, handles linked `INVITED → EXPIRED` membership transitions under the owner-approved v1 schema, and has integration coverage for concurrency, repeat safety, and rollback. Direct membership-expiry audit/outbox rollback tests are also present.

All 10 exact-head PR checks passed: [PostgreSQL integration](https://github.com/jelome265/sitolo/actions/runs/39731541826), [Rust/workspace verification](https://github.com/jelome265/sitolo/actions/runs/39731541832), [policy](https://github.com/jelome265/sitolo/actions/runs/39731541896), [security](https://github.com/jelome265/sitolo/actions/runs/39731541825), and [CodeQL](https://github.com/jelome265/sitolo/runs/119668322293). No local PostgreSQL was run; it remains deferred to the next phase.

AUD-04's five approved mappings are now wired with PostgreSQL integration evidence, pending independent Stage 06 re-audit before closure. AUD-03 remains OPEN because not every mandatory IAM operation has authoritative transaction handling and complete failure/race coverage; authenticated API authority remains a separate blocker. AUD-01, AUD-02 and AUD-05 remain OPEN. PR #49 remains open and unmerged.

## Implementation evidence update — 2026-10-22

Current source head `8eaaf18530027a3fdbb854361a1aa4a0ba580e30` passed all nine required checks, including PostgreSQL integration [39737396713](https://github.com/jelome265/sitolo/actions/runs/39737396713), Rust verification [39737395716](https://github.com/jelome265/sitolo/actions/runs/39737395716), policy [39737395738](https://github.com/jelome265/sitolo/actions/runs/39737395738), and security/CodeQL [39737395727](https://github.com/jelome265/sitolo/actions/runs/39737395727). The PostgreSQL suite includes the new invitation accept/withdraw/expiry race regression. Local PostgreSQL was not used.

AUD-04 is wired and integration-tested for the five approved mappings, including `INVITED → EXPIRED`, but remains pending independent re-audit. AUD-03 remains open pending re-audit of all transaction boundaries and actor/tenant provenance. AUD-02 remains open; protected HTTP mutations remain fail-closed. AUD-01 and AUD-05 remain open. PR #49 is open and unmerged.

## PostgreSQL rollback coverage update — 2026-10-22

Source `96ca56b08804211895106612fb5b393e2d66e686` adds PostgreSQL failure-injection coverage for direct branch creation, direct member invitation, and scope-grant revocation. All nine exact-head checks passed, including the PostgreSQL workflow [39738690198](https://github.com/jelome265/sitolo/actions/runs/39738690198), Rust verification [39738690312](https://github.com/jelome265/sitolo/actions/runs/39738690312), policy [39738690391](https://github.com/jelome265/sitolo/actions/runs/39738690391), and security/CodeQL [39738690317](https://github.com/jelome265/sitolo/actions/runs/39738690317). No local PostgreSQL was run.

AUD-03 remains open; AUD-04 remains partial pending Stage 06 re-audit. AUD-02 is open, with protected HTTP writes still fail-closed. See the follow-up remediation report for exact disposition.
