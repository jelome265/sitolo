---
artifact_type: remediation-extension
status: implementation-complete-pending-independent-audit
run_slug: "pr009-part8"
security_relevant: yes
source_sha: "9d6ba1c58601691fce4195b76ec700ad207d2cdb"
date: "2026-10-26"
---

# Remediation extension — path-specific Rust lint exception inventory

The Phase 1 CI policy required narrow, reasoned lint exceptions and an inventory of source-level lint overrides. Before this change, CI rejected `#[allow]`, `cfg_attr(...allow...)`, and reasonless `#[expect]`, but did not require a path-specific record tying an accepted `#[expect]` to its exact lint and reason.

Commit `9d6ba1c58601691fce4195b76ec700ad207d2cdb` adds `scripts/ci/rust-lint-exceptions.json`, with one record for each of the 45 existing distinct path/attribute pairs. `check-rust-policy.py` compares the inventory against every source `#[expect]`, rejecting missing, changed and stale entries. The Policy workflow runs regression tests for matching, missing, and stale/changed exceptions, and its path filter includes both the inventory and the tests. `docs/ci_enforcement.md` documents the rule.

No lint was downgraded, no exception was added to source, and no assertion was weakened.

## Verification

Local checks: `python -m unittest discover -s scripts/ci -p "test_*.py"` passed (3 tests); `python scripts/ci/check-rust-policy.py`, `scripts/ci/check-workflow-policy.ps1`, `python scripts/ci/check-doc-references`, and `git diff --check` passed. The Bash-only ICM workspace wrapper could not run on this Windows host; the exact-head Policy workflow ran both ICM checks.

Exact-head GitHub checks all passed on `9d6ba1c58601691fce4195b76ec700ad207d2cdb`:

| Workflow | Result | Run |
|---|---|---|
| Rust canonical verification, including required PostgreSQL suites | PASS | [40289598902](https://github.com/jelome265/sitolo/actions/runs/40289598902) |
| Integration, PostgreSQL application/contract tests | PASS | [40289598918](https://github.com/jelome265/sitolo/actions/runs/40289598918) |
| Security, including CodeQL, advisories, secret scan and offensive regression | PASS | [40289599101](https://github.com/jelome265/sitolo/actions/runs/40289599101) |
| Policy, including inventory regression and ICM checks | PASS | [40289598905](https://github.com/jelome265/sitolo/actions/runs/40289598905) |

## Disposition

This closes the reasoned path-specific `#[expect]` inventory gap in Workstream H only. The Phase 4 mutation matrix, Phase 3 identity lifecycle, production consumer/effect, and migration/release decisions remain incomplete. This is not an independent Stage 06 audit. PR #49 remains open and review-required; the pre-existing modified verification report was not changed or staged.
