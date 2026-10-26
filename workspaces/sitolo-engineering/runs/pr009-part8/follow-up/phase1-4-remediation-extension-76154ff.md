---
artifact_type: remediation-extension
status: implementation-complete-pending-independent-audit
run_slug: "pr009-part8"
security_relevant: yes
source_sha: "76154ffd639ff6467729f3e4ab508f554a72f919"
date: "2026-10-26"
---

# Remediation extension — reject source lint-level downgrades

## Finding and change

The lint exception inventory added in `9d6ba1c` covered explicit `#[expect]` exceptions, but the policy checker still only rejected source-level `#[allow]` and `cfg_attr(...allow...)`. It did not detect direct or conditional `#[warn]` attributes that can lower strict lint levels.

Commit `76154ffd639ff6467729f3e4ab508f554a72f919` updates the checker to reject `#[allow]`, `#[warn]`, and `cfg_attr` forms that use either downgrade. Six regression tests now cover direct allow/warn, conditional allow/warn, strict forbid, expected exceptions, and inventory drift. No source lint was lowered, and no existing `#[expect]` exception was changed.

## Verification

Local: six Rust policy tests pass; `python scripts/ci/check-rust-policy.py`, `./scripts/ci/check-workflow-policy.ps1`, and `git diff --check` pass.

All four required workflows pass on exact source SHA `76154ffd639ff6467729f3e4ab508f554a72f919`:

| Workflow | Result | Run |
|---|---|---|
| Rust canonical verification, including required PostgreSQL suites | PASS | [40290717776](https://github.com/jelome265/sitolo/actions/runs/40290717776) |
| Integration, PostgreSQL application/contract tests | PASS | [40290717827](https://github.com/jelome265/sitolo/actions/runs/40290717827) |
| Security, including CodeQL and offensive regression | PASS | [40290718200](https://github.com/jelome265/sitolo/actions/runs/40290718200) |
| Policy, including six regression tests and ICM checks | PASS | [40290717731](https://github.com/jelome265/sitolo/actions/runs/40290717731) |

## Disposition

This closes the identified source lint downgrade gap in Workstream H. Branch-protection and human-review requirements remain unverified; the branch-wide mutation, identity, consumer and migration/release blockers remain. This is not an independent Stage 06 audit. PR #49 remains open and review-required, and the pre-existing modified verification report remains untouched.
