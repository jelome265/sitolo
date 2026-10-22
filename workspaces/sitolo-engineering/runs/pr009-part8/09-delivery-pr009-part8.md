# Stage 09 Delivery — PR-009 Part 8

**Date:** 2026-10-22\
**Delivery head (code):** `52c7392bca56b4fd6b1c90b52e57cec52b651cd3`\
**Verified predecessor:** `862775c5412b4928df3dfac93580cf8b1863aae1` (stage 08 evidence)\
**Head semantics:** the code head is the most recent commit that changes code. This handoff and any later commits are documentation-only; compare code evidence against `52c7392`, not against the PR tip.\
**Disposition:** Review-ready, not merge-ready. Exact-head CI is green at `52c7392` (tip `5653591`), F-01 and F-02 are closed, but the stage 08 release blockers and human review remain.\
**Human check:** Not performed. A human owns the review and merge decision.

## Summary

This handoff carries the PR-009 Part 8 audit/outbox implementation plus a final CI-policy increment applied after stage 08 verification.

The increment installs `ripgrep` in the `verify` job. That install immediately exposed a gate that had never executed in this repository's CI: `check-architecture` and `check-phase2-policy` had been reporting PASS while their searches returned nothing. `check-phase2-policy` then failed on ten `DATABASE_URL` aliases in three PostgreSQL test harnesses.

The first attempt scoped the rule to exclude `tests/`, which turned the gate green but exempted an entire subtree. `891c273` replaces that with the stricter fix: the harness fallbacks are deleted, the exemption is removed, and both policy scripts are restored byte-identical to `main`.

A static review then found that the runtime pool did not authenticate as its restricted role: it logged in with the administrator and ran `SET ROLE app_runtime`, which any session can reverse. `c090809` removes that mechanism. The runtime and worker pools now authenticate directly as the `app_runtime` and `app_worker` LOGIN roles, `verify_runtime_role` asserts `current_user` and `session_user`, and a new negative test proves `RESET ROLE` is a no-op and `SET ROLE postgres` is denied. `67315a4` updates the one test that still asserted the old `NOLOGIN` role shape.

A second review found that the RLS verifier was bound to the Part 7 fixture while its name claimed general coverage: it required `tenant_resources` and one `{table}_isolation_policy` per table, neither of which the Part 8 production DDL creates. `3056da6` separates the two proofs. The Part 7 verifier is explicitly bound to `tests/fixtures/rls_schema.sql`; a new Part 8 verifier checks the three production tables, the two operation-specific policies per table, and the SELECT + INSERT grant decision. `7d892eb` adds violation diagnostics after the first CI run failed on an unstated policy name, and `52c7392` binds the verifier to the DDL's actual `memberships_provision_*` names. The production DDL itself changes only in its header comment; no table, policy, or grant is added to satisfy a verifier.

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

### Exact-head CI at `5653591` (code at `52c7392`) — 10/10 checks pass

| Check | Workflow | Result | Link |
|---|---|---|---|
| `verify` | rust | PASS, 2m0s | job 119564433986 |
| `toolchain` | rust | PASS, 17s | job 119564433733 |
| `application and contract tests with PostgreSQL` | integration | PASS, 1m5s | job 119564433761 |
| `advisories` | security | PASS, 20s | job 119564433917 |
| `dependency-policy` | security | PASS, 21s | job 119564434039 |
| `secrets` | security | PASS, 6s | job 119564434170 |
| `codeql` | security | PASS, 3m56s | job 119564434012 |
| `offensive-regression` | security | PASS, 1m27s | job 119564433717 |
| `policy` | policy | PASS, 12s | job 119564433680 |
| `CodeQL` | code scanning | PASS, 2s | run 119565686727 |

The tip is `5653591` (a System Map re-pin); the last code change is `52c7392`, and every check above ran on the same tree.

The `verify` job log is the load-bearing evidence. For the policy gap it contains `==> Architecture check` / `Architecture check PASSED` and `==> Phase 2 policy check` / `Phase 2 policy check PASSED`, and contains no `command not found` occurrence anywhere in the job, so both gates executed real searches. For F-01 it runs the Part 8 PostgreSQL suite at **23 passed** and the dedicated RLS suite **twice at 28 passed each**, including `test_runtime_pool_identity_cannot_escalate ... ok`; the second RLS run is the standalone `==> PostgreSQL RLS Security Suite` step after the workspace tests. For F-02 the application evidence suite passes **5 tests**, each of which applies `phase4_part8_tenancy_provisioning.sql` and then calls `verify_part8_provisioning_rls_metadata`; the job contains no `Part 8 RLS contract violation` occurrence. The `integration` job passed the same five application tests separately.

### Local verification at `52c7392` (Windows host, ripgrep 15.2.0)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets --all-features --locked` | PASS, exit 0 |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS, exit 0 |
| `check-architecture` (bash, forward-slash shim) | PASS |
| `check-phase2-policy` (bash, forward-slash shim) | PASS |
| `check-phase2-policy.ps1` | PASS |
| `check-icm-workspace` (bash) | PASS |
| Alias scan with **no exclusion** | 0 matches |

The temporary `rg` binary on this host had been purged between runs. Because the bash scripts wrap every scan in `|| true`, they printed PASS while the binary was missing. Restoring ripgrep 15.2.0 and placing the forward-slash path shim first in `PATH` produced genuine searches and the PASS results above. This is a host artefact, not a repository state; the CI runner installs ripgrep explicitly.

`cargo metadata --locked`, `cargo deny check`, `cargo test`, and `cargo audit` were last run locally at `e66c73a`. No manifest and no lockfile changed between `e66c73a` and `52c7392`, and `cargo check --locked` passes at the new head; the `dependency-policy`, `advisories`, and `integration` jobs cover the rest at exact head.

### Not run locally, and why

- **`cargo test` at `52c7392`:** not re-run locally. The workspace and application suites are covered by the exact-head `verify` and `integration` jobs above.
- **PostgreSQL execution:** NOT RUN locally. No server, `psql`, container runtime, or database URLs exist on this host. PostgreSQL evidence comes exclusively from the exact-head CI jobs. That includes the direct-login identity proof and the Part 8 verifier against the production DDL, both of which only a live server can execute.
- **`cargo audit` at `52c7392`:** not run locally (not installed). The `advisories` job covers it at exact head.
- **`scripts/ci/verify` end to end:** not executed as one command locally, because it requires `cargo-audit` and a Linux shell. Its constituent checks were run individually as listed.

## Audit status

All stage 06 findings remain closed as recorded in `07-remediation-pr009-part8.md` and re-verified in `08-verification-pr009-part8.md`.

**Scope gap, stated plainly:** the commits in this increment were produced after stage 08 verification and were **not** re-audited at stage 06. Stage 08's evidence is pinned to `862775c`; this handoff delivers `52c7392`. The increment is covered by exact-head CI and the post-hoc static reviews below, not by a stage 06 re-audit. A human may require one before merge; that is a human-controlled decision, not an automatic dependency.

### Static audits received after stage 08

Two independent static reviews of `e66c73a` were received after this increment was drafted. Neither executed code, and neither approves or merges the PR.

| Finding | Status |
|---|---|
| F-04 / AUD-CFG-002 — the alias ban exempted all `tests/` paths | **Closed by `891c273`.** The exemption and the aliases it permitted are both gone; both scripts are byte-identical to `main`. |
| F-01 / AUD-SEC-001 — runtime privilege drop via `SET ROLE` is not a boundary | **Closed by `c090809`.** The runtime and worker pools authenticate as their own LOGIN roles; `verify_runtime_role` asserts `current_user` and `session_user`; `test_runtime_pool_identity_cannot_escalate` proves `RESET ROLE` is a no-op and `SET ROLE postgres` is denied. `67315a4` updates the obsolete NOLOGIN assertion. PostgreSQL CI passes at `52c7392`. |
| F-02 / AUD-SEC-002 — the production Part 8 DDL contradicts the RLS verifier | **Closed at `3056da6`, corrected in `52c7392`.** The Part 7 verifier is explicitly bound to `tests/fixtures/rls_schema.sql`; a new Part 8 verifier checks the production DDL's three tables, two operation-specific policies per table, INSERT → `WITH CHECK` only, SELECT → `USING` only, `app_runtime` only, and SELECT + INSERT grants only. The application evidence test applies the production DDL and runs the verifier; PostgreSQL CI passes at `52c7392` with no `Part 8 RLS contract violation`. The DDL header is corrected; no table, policy, or grant was added to satisfy a verifier. |
| F-09 — the `deny.toml` license narrowing may break the gate | **Closed empirically.** `cargo deny check` passes at `e66c73a`; the `dependency-policy` job passes at `52c7392`. |

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
- **Tests:** the three PostgreSQL harnesses now require `ADMIN_DATABASE_URL`, `RUNTIME_DATABASE_URL`, and `WORKER_DATABASE_URL`, and the fixture creates the client roles as LOGIN before the pools connect. The application evidence test applies the production DDL and calls `verify_part8_provisioning_rls_metadata` immediately afterwards, so every PostgreSQL run checks the Part 8 contract against the file it applies. `rust.yml`, `integration.yml`, and `artifact.yml` export the split DSNs; `artifact.yml` also gains the worker DSN it was missing.
- **CI:** the `rust` `verify` job installs `ripgrep` via `apt-get`, adding roughly ten seconds and a package dependency to that runner image. If the Ubuntu image ever ships `ripgrep` preinstalled, the step becomes redundant but harmless.
- **Policy:** `deny.toml` now rejects ISC- and MPL-2.0-licensed dependencies. No current dependency requires them.

## Known limitations

1. **Search failure still passes silently.** Every scan wraps `rg` in `|| true`, which tolerates exit code 1 (no matches) but also swallows exit code 2 (search error). A malformed pattern, unreadable path, or broken `rg` invocation would still report PASS. This increment closes the *missing binary* case only.
2. **Bash policy scripts misfire on Windows.** `check-architecture` and `check-phase2-policy` filter with `grep -vE 'crates/sitolo-…'`, which cannot match `crates\sitolo-…`. Under Git Bash on Windows both report false violations. CI is Linux and is unaffected, but `scripts/ci/verify` is unreliable on a Windows host.
3. **Stage 08 release blockers stand unchanged:**
   - the relay response does not name a transport, endpoint/topic/queue, payload contract, acknowledgement behavior, or runtime configuration; `apps/worker/src/main.rs` refuses to start without an approved adapter;
   - the live HTTP command path still lacks authenticated principal-derived owner/actor authority;
   - no real destination adapter or production worker runtime exists.
4. **This increment was not re-audited** at stage 06, as stated above.
5. **The release-artifact test step has no database environment.** `build_release_artifact` in `artifact.yml` runs `cargo test --workspace --all-targets --all-features --locked` without a PostgreSQL service or the three DSNs. The Part 8 harnesses are fail-closed by design, so that step would abort before the release is packaged. This is pre-existing and dormant — the workflow runs on tags and manual dispatch — and was not changed here because fixing it requires a release-pipeline composition decision rather than a test edit.
6. **No production readiness, certification, or complete validation is claimed.** The evidence proves the listed checks at the listed heads and nothing more.

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

`891c273` supersedes `e66c73a`. Reverting only `891c273` would restore both the exemption and the aliases, which is the opposite of what both reviews recommended. `c090809` is the F-01 fix; do not revert it without replacing the authority mechanism and the identity assertions together. `3056da6` is the F-02 fix and `52c7392` corrects its policy-name binding; revert them together only if replacing the Part 8 verifier.

## Integration decision

PR #49 is `OPEN`, `MERGEABLE`, and `BLOCKED` on `REVIEW_REQUIRED` at `5653591` (code head `52c7392`).

Stage 09 completion does not authorize merge. The blocking items are human review, the three unresolved release blockers above, the P1 items (authenticated API authority and the relay contract), and the absence of a stage 06 re-audit for this increment. No P0 static-audit finding remains open: F-01 and F-02 are closed. Merge remains a human decision.
