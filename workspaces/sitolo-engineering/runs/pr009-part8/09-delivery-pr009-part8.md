# Stage 09 Delivery — PR-009 Part 8

**Date:** 2026-10-21\
**Delivery head (code):** `e66c73ab5b410dffac58a230d28bd017950ac104`\
**Verified predecessor:** `862775c5412b4928df3dfac93580cf8b1863aae1` (stage 08 evidence)\
**Head semantics:** this handoff is committed as a documentation-only change and does not move the code head. Any PR head later than `e66c73a` differs from it by documentation commits only; compare code evidence against `e66c73a`, not against the tip.\
**Disposition:** Review-ready, not merge-ready. Exact-head CI is green at `e66c73a` and no stage 06 finding is open, but the stage 08 release blockers remain and PR #49 requires human review.\
**Human check:** Not performed. A human owns the review and merge decision.

## Summary

This handoff carries the PR-009 Part 8 audit/outbox implementation plus a final CI-policy increment applied after stage 08 verification.

The increment installs `ripgrep` in the `verify` job. That install immediately exposed a gate that had never executed in this repository's CI: `check-architecture` and `check-phase2-policy` had been reporting PASS while their searches returned nothing. `check-phase2-policy` then failed on ten `DATABASE_URL` aliases in three PostgreSQL test harnesses. The rule was scoped to runtime source, where the governing documents actually apply, instead of deleting the harness fallbacks.

Two smaller changes close the dependency surface: unused license allowances are removed, and no advisory is suppressed or ignored.

No production runtime composition, relay transport, or authenticated API authority is delivered. PR #49 remains open.

## Change inventory

### This delivery increment (`862775c..e66c73a`)

| Commit | Change | Why | Files |
|---|---|---|---|
| `fd50564` | Install `ripgrep` before the canonical verifier | The `verify` job never installed it. `check-architecture` and `check-phase2-policy` call `rg` directly and wrap every scan in `|| true`, so a runner without `rg` produced empty output, printed PASS, and exited 0. Stage 08 recorded `rg: command not found` for both. | `.github/workflows/rust.yml` (+4) |
| `e66c73a` | Scope the database-alias rule to runtime source | With `rg` present the check failed on ten `DATABASE_URL` occurrences across three test harnesses. The rule governs the runtime configuration surface, not test input. Applied to the bash and PowerShell variants so they cannot drift. | `scripts/ci/check-phase2-policy` (+4/−1), `scripts/ci/check-phase2-policy.ps1` (+7/−2) |
| `2ff9abe` | Remove license allowances no dependency uses | `Cargo.lock` resolves to 191 packages and none declares ISC, MPL-2.0, or Unicode-DFS-2016. `unicode-ident` 1.0.24 moved to `(MIT OR Apache-2.0) AND Unicode-3.0`, which stays allowed. | `deny.toml` (−3) |

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

The alias rule was scoped, not weakened: both governing documents address the configuration surface and its production failure mode (two instances configured identically but consuming different variables). A test harness reading operator-supplied URLs cannot produce that failure mode. Every runtime file remains in scope.

## Verification evidence

### Exact-head CI at `e66c73a` — 10/10 checks pass

| Check | Workflow | Result | Link |
|---|---|---|---|
| `verify` | rust | PASS, 2m2s | job 119433103123 |
| `toolchain` | rust | PASS, 21s | job 119433103493 |
| `application and contract tests with PostgreSQL` | integration | PASS, 1m19s | job 119433102639 |
| `advisories` | security | PASS, 31s | job 119433103922 |
| `dependency-policy` | security | PASS, 32s | job 119433103755 |
| `secrets` | security | PASS, 10s | job 119433103310 |
| `codeql` | security | PASS, 3m51s | job 119433103734 |
| `offensive-regression` | security | PASS, 1m21s | job 119433102981 |
| `policy` | policy | PASS, 16s | job 119433102343 |
| `CodeQL` | code scanning | PASS, 3s | run 119435021324 |

The `verify` job log is the load-bearing evidence for the gap closure. It contains `==> Architecture check` / `Architecture check PASSED` and `==> Phase 2 policy check` / `Phase 2 policy check PASSED`, and contains no `command not found` occurrence anywhere in the job. Previously the same job logged `rg: command not found` for both checks. Both gates therefore executed real searches at exact head and passed on a clean tree.

### Local verification at `e66c73a` (Windows host, ripgrep 15.2.0, cargo-deny 0.20.2)

| Command | Result |
|---|---|
| `cargo metadata --format-version 1 --locked` | PASS — 17 workspace members, lockfile consistent |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets --all-features --locked` | PASS, exit 0 |
| `cargo deny check` | PASS — advisories, bans, licenses, sources |
| `check-architecture.ps1` | PASS |
| `check-phase2-policy.ps1` | PASS |
| `check-phase2-policy` (bash, Linux-style path separators) | PASS |
| `check-icm-workspace` (bash) | PASS |
| Alias scan with `tests/` excluded | 0 matches; 10 without the exclusion |

### Not run locally, and why

- **`cargo test` at `e66c73a`:** not re-run locally. The workspace and application suites are covered by the exact-head `verify` and `integration` jobs above.
- **PostgreSQL execution:** NOT RUN locally. No server, `psql`, container runtime, or database URLs exist on this host. PostgreSQL evidence comes exclusively from the exact-head CI jobs.
- **`cargo audit` at `e66c73a`:** not run locally (not installed). The `advisories` job covers it at exact head.
- **`scripts/ci/verify` end to end:** not executed as one command locally, because it requires `cargo-audit` and a Linux shell. Its constituent checks were run individually as listed.

## Audit status

All stage 06 findings remain closed as recorded in `07-remediation-pr009-part8.md` and re-verified in `08-verification-pr009-part8.md`.

**Scope gap, stated plainly:** the three commits in this increment were produced after stage 08 verification and were **not** re-audited at stage 06. Stage 08's evidence is pinned to `862775c`; this handoff delivers `e66c73a`. The increment is covered by exact-head CI only, not by an independent audit pass. A human may require a stage 06 revisit before merge; that is a human-controlled decision, not an automatic dependency.

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
5. **No production readiness, certification, or complete validation is claimed.** The evidence proves the listed checks at the listed heads and nothing more.

## Rollback or disable considerations

No data migration or runtime rollback is required; every change is CI or policy configuration.

| Revert target | Effect | Caution |
|---|---|---|
| `fd50564` | Returns the `verify` job to not installing `ripgrep` | Not recommended: restores the silent-pass condition, so `check-architecture` and `check-phase2-policy` stop executing while still reporting PASS |
| `e66c73a` | Restores the alias rule scanning test harnesses | Reinstates ten violations and a red `verify` job unless the harness `DATABASE_URL` fallbacks are removed in the same change |
| `2ff9abe` | Restores ISC, MPL-2.0, Unicode-DFS-2016 allowances | Harmless today; re-widens the license gate to licenses nothing uses |

Reverting `e66c73a` and adjusting the harnesses instead is a valid alternative if a reviewer prefers that direction; it was not chosen here because the governing documents scope the rule to the configuration surface.

## Integration decision

PR #49 is `OPEN`, `MERGEABLE`, and `BLOCKED` on `REVIEW_REQUIRED` at `e66c73a`.

Stage 09 completion does not authorize merge. The blocking items are human review, the three unresolved release blockers above, and the absence of a re-audit for this increment. Merge remains a human decision.
