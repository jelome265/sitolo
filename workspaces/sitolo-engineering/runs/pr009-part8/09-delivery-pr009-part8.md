# Stage 09 Delivery — PR-009 Part 8

**Date:** 2026-10-21\
**Delivery head (code):** `891c27305ed8261b56514533cba1399a5cfcbdd8`\
**Verified predecessor:** `862775c5412b4928df3dfac93580cf8b1863aae1` (stage 08 evidence)\
**Head semantics:** the code head is the most recent commit that changes code. This handoff and any later commits are documentation-only; compare code evidence against `891c273`, not against the PR tip.\
**Disposition:** Review-ready, not merge-ready. Exact-head CI is green at `891c273` and no stage 06 finding is open, but the stage 08 release blockers, two unresolved HIGH static-audit findings, and human review remain.\
**Human check:** Not performed. A human owns the review and merge decision.

## Summary

This handoff carries the PR-009 Part 8 audit/outbox implementation plus a final CI-policy increment applied after stage 08 verification.

The increment installs `ripgrep` in the `verify` job. That install immediately exposed a gate that had never executed in this repository's CI: `check-architecture` and `check-phase2-policy` had been reporting PASS while their searches returned nothing. `check-phase2-policy` then failed on ten `DATABASE_URL` aliases in three PostgreSQL test harnesses.

The first attempt scoped the rule to exclude `tests/`, which turned the gate green but exempted an entire subtree. `891c273` replaces that with the stricter fix: the harness fallbacks are deleted, the exemption is removed, and both policy scripts are restored byte-identical to `main`.

Two smaller changes close the dependency surface: unused license allowances are removed, and no advisory is suppressed or ignored.

No production runtime composition, relay transport, or authenticated API authority is delivered. PR #49 remains open.

## Change inventory

### This delivery increment (`862775c..891c273`)

| Commit | Change | Why | Files |
|---|---|---|---|
| `fd50564` | Install `ripgrep` before the canonical verifier | The `verify` job never installed it. `check-architecture` and `check-phase2-policy` call `rg` directly and wrap every scan in `|| true`, so a runner without `rg` produced empty output, printed PASS, and exited 0. Stage 08 recorded `rg: command not found` for both. | `.github/workflows/rust.yml` (+4) |
| `e66c73a` | Scope the database-alias rule to runtime source | With `rg` present the check failed on ten `DATABASE_URL` occurrences across three test harnesses. Superseded by `891c273`, which removes the aliases instead of exempting them. | `scripts/ci/check-phase2-policy` (+4/−1), `scripts/ci/check-phase2-policy.ps1` (+7/−2) |
| `2ff9abe` | Remove license allowances no dependency uses | `Cargo.lock` resolves to 191 packages and none declares ISC, MPL-2.0, or Unicode-DFS-2016. `unicode-ident` 1.0.24 moved to `(MIT OR Apache-2.0) AND Unicode-3.0`, which stays allowed. | `deny.toml` (−3) |
| `891c273` | Remove the `tests/` exemption and the aliases it permitted | The exemption was broader than the rule's rationale and was written by the author who needed it. Neither workflow relies on the fallback, so the aliases could be deleted and both scripts restored unchanged. Net effect: the PR no longer modifies the guardrail at all. | 3 PostgreSQL test harnesses; `scripts/ci/check-phase2-policy` and `.ps1` restored (`+14/−26` across 5 files) |

`299d96b` (docs only) sits between the verified predecessor and this increment and changes no code.

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

### Exact-head CI at `891c273` — 10/10 checks pass

| Check | Workflow | Result | Link |
|---|---|---|---|
| `verify` | rust | PASS, 1m47s | job 119448218907 |
| `toolchain` | rust | PASS, 15s | job 119448219370 |
| `application and contract tests with PostgreSQL` | integration | PASS, 1m17s | job 119448218630 |
| `advisories` | security | PASS, 20s | job 119448220655 |
| `dependency-policy` | security | PASS, 17s | job 119448220570 |
| `secrets` | security | PASS, 6s | job 119448220000 |
| `codeql` | security | PASS, 2m25s | job 119448220295 |
| `offensive-regression` | security | PASS, 1m20s | job 119448220465 |
| `policy` | policy | PASS, 16s | job 119448219477 |
| `CodeQL` | code scanning | PASS, 3s | run 119449353811 |

`e66c73a` and `ef5c153` each also passed 10/10. The table above is the head this handoff delivers.

The `verify` job log is the load-bearing evidence for the gap closure. It contains `==> Architecture check` / `Architecture check PASSED` and `==> Phase 2 policy check` / `Phase 2 policy check PASSED`, and contains no `command not found` occurrence anywhere in the job. Previously the same job logged `rg: command not found` for both checks. Both gates therefore executed real searches at exact head and passed on a clean tree.

### Local verification at `891c273` (Windows host, ripgrep 15.2.0)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets --all-features --locked` | PASS, exit 0 |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS, exit 0 |
| `check-architecture` (bash, Linux-style path separators) | PASS |
| `check-phase2-policy` (bash) | PASS |
| `check-phase2-policy.ps1` | PASS |
| `check-icm-workspace` (bash) | PASS |
| Alias scan with **no exclusion** | 0 matches |

`cargo metadata --locked`, `cargo deny check`, `cargo test`, and `cargo audit` were last run locally at `e66c73a`. No manifest and no lockfile changed between `e66c73a` and `891c273`, and `cargo check --locked` passes at the new head; the `dependency-policy`, `advisories`, and `integration` jobs cover the rest at exact head.

### Not run locally, and why

- **`cargo test` at `891c273`:** not re-run locally. The workspace and application suites are covered by the exact-head `verify` and `integration` jobs above.
- **PostgreSQL execution:** NOT RUN locally. No server, `psql`, container runtime, or database URLs exist on this host. PostgreSQL evidence comes exclusively from the exact-head CI jobs.
- **`cargo audit` at `891c273`:** not run locally (not installed). The `advisories` job covers it at exact head.
- **`scripts/ci/verify` end to end:** not executed as one command locally, because it requires `cargo-audit` and a Linux shell. Its constituent checks were run individually as listed.

## Audit status

All stage 06 findings remain closed as recorded in `07-remediation-pr009-part8.md` and re-verified in `08-verification-pr009-part8.md`.

**Scope gap, stated plainly:** the four commits in this increment were produced after stage 08 verification and were **not** re-audited at stage 06. Stage 08's evidence is pinned to `862775c`; this handoff delivers `891c273`. The increment is covered by exact-head CI and the post-hoc static reviews below, not by a stage 06 re-audit. A human may require one before merge; that is a human-controlled decision, not an automatic dependency.

### Static audits received after stage 08

Two independent static reviews of `e66c73a` were received after this increment was drafted. Neither executed code, and neither approves or merges the PR.

| Finding | Status |
|---|---|
| F-04 / AUD-CFG-002 — the alias ban exempted all `tests/` paths | **Closed by `891c273`.** The exemption and the aliases it permitted are both gone; both scripts are byte-identical to `main`. |
| F-01 / AUD-SEC-001 — runtime privilege drop via `SET ROLE` is not a boundary | **Open, not addressed here.** Recorded under Known limitations. |
| F-02 / AUD-SEC-002 — the production Part 8 DDL contradicts the RLS verifier | **Open, not addressed here.** Recorded under Known limitations. |
| F-09 — the `deny.toml` license narrowing may break the gate | **Closed empirically.** `cargo deny check` passes at `e66c73a`; the `dependency-policy` job passes at `891c273`. |

The security-control positions recorded in stage 08 are unchanged by this increment, because it touches no runtime code:

| Control | Position |
|---|---|
| SC-002 — server-derived tenant scope | PARTIAL; API tenancy route still lacks authenticated principal context |
| SC-006 — transaction integrity and idempotency | PASS for the tested organization-provisioning path only |
| SC-009 — durable audit evidence | PASS for the exercised application transaction path |
| SC-010 — bounded/fail-closed behavior | PARTIAL; no real destination adapter or production worker runtime |
| SC-011 — dependency integrity | RESOLVED; no advisory suppressed, no exception added |

## Migrations and deployment impact

- **Schema:** no migration in this increment. `crates/sitolo-persistence/schema/phase4_part8_tenancy_provisioning.sql` is unchanged.
- **Runtime:** no runtime code changes; no configuration fields, environment variables, or API surfaces added or removed.
- **Tests:** the three PostgreSQL harnesses now require `ADMIN_DATABASE_URL` and no longer fall back to a bare `DATABASE_URL`. Both `rust.yml` and `integration.yml` already export it, so CI behaviour is unchanged.
- **CI:** the `rust` `verify` job now installs `ripgrep` via `apt-get`, adding roughly ten seconds and a package dependency to that runner image. If the Ubuntu image ever ships `ripgrep` preinstalled, the step becomes redundant but harmless.
- **Policy:** `deny.toml` now rejects ISC- and MPL-2.0-licensed dependencies. No current dependency requires them.

## Known limitations

1. **Search failure still passes silently.** Every scan wraps `rg` in `|| true`, which tolerates exit code 1 (no matches) but also swallows exit code 2 (search error). A malformed pattern, unreadable path, or broken `rg` invocation would still report PASS. This increment closes the *missing binary* case only.
2. **Bash policy scripts misfire on Windows.** `check-architecture` and `check-phase2-policy` filter with `grep -vE 'crates/sitolo-…'`, which cannot match `crates\sitolo-…`. Under Git Bash on Windows both report false violations. CI is Linux and is unaffected, but `scripts/ci/verify` is unreliable on a Windows host.
3. **Stage 08 release blockers stand unchanged:**
   - the relay response does not name a transport, endpoint/topic/queue, payload contract, acknowledgement behavior, or runtime configuration; `apps/worker/src/main.rs` refuses to start without an approved adapter;
   - the live HTTP command path still lacks authenticated principal-derived owner/actor authority;
   - no real destination adapter or production worker runtime exists.
4. **This increment was not re-audited** at stage 06, as stated above.
5. **F-01: the runtime database identity is not a privilege boundary.** The runtime pool connects as a login role and drops to `app_runtime` with `SET ROLE` in `after_connect`. `SET ROLE` is reversible for the rest of the session, and `verify_runtime_role` inspects the attributes of `app_runtime` only, never `session_user`. In CI, `RUNTIME_DATABASE_URL` and `WORKER_DATABASE_URL` both point at the `postgres` superuser, so the RLS suites prove that policies apply while the session stays in `app_runtime`; they do not prove the connecting identity is constrained. RLS still applies in that state, so this is the loss of the last barrier rather than an automatic bypass. Not fixed by this delivery: it requires a PostgreSQL server to verify and is a deployment-identity change.
6. **F-02: the production Part 8 DDL does not match the RLS verifier.** `phase4_part8_tenancy_provisioning.sql` creates `*_provision_insert` and `*_provision_read` policies, while `verify_rls_catalog_metadata` requires exactly one policy named `{table}_isolation_policy` on `organizations`, `branches`, and `tenant_resources`. The verifier's expectations match `tests/fixtures/rls_schema.sql`, not the production file. The DDL header also claims runtime receives INSERT only while line 74 grants `SELECT, INSERT`, and `tenant_resources` is not created by the DDL at all. Not fixed by this delivery. The owner's decision is that runtime may read its own organization and branch rows, so the remediation is to keep `SELECT` and correct the header, then reconcile the verifier with the file that is actually applied.
7. **No production readiness, certification, or complete validation is claimed.** The evidence proves the listed checks at the listed heads and nothing more.

## Rollback or disable considerations

No data migration or runtime rollback is required; every change is CI or policy configuration.

| Revert target | Effect | Caution |
|---|---|---|
| `fd50564` | Returns the `verify` job to not installing `ripgrep` | Not recommended: restores the silent-pass condition, so `check-architecture` and `check-phase2-policy` stop executing while still reporting PASS |
| `891c273` | Reinstates the `tests/` exemption and the harness `DATABASE_URL` fallbacks | Not recommended: re-widens the alias ban to every test path and restores ten aliases |
| `e66c73a` | Nothing against the current tree: `891c273` already reverted its script edits and removed the aliases | Retained for history only |
| `2ff9abe` | Restores ISC, MPL-2.0, Unicode-DFS-2016 allowances | Harmless today; re-widens the license gate to licenses nothing uses |

`891c273` supersedes `e66c73a`. Reverting only `891c273` would restore both the exemption and the aliases, which is the opposite of what both reviews recommended.

## Integration decision

PR #49 is `OPEN`, `MERGEABLE`, and `BLOCKED` on `REVIEW_REQUIRED` at `891c273`.

Stage 09 completion does not authorize merge. The blocking items are human review, the three unresolved release blockers above, the two open HIGH audit findings, and the absence of a stage 06 re-audit for this increment. Merge remains a human decision.
