---
artifact_type: verification-report
status: exact-head-checks-passed-open-blockers
run_slug: "pr009-part8"
security_relevant: yes
source_head: "e4935905b14ed6cf5293d84c1b32e4e45f07c693"
branch: "feat/phase4-pr009-audit-outbox"
date: "2026-11-01"
---

# PR #49 exact-head verification — 2026-11-01

## Scope

This report records the latest source and workflow evidence for PR #49 after the role-grant policy blocker was documented. The code implementation head is `5b4563bdd5107008f44a30e9f3516f967f81a492` (authenticated owner-bound logout and its PostgreSQL trigger correction); commits `25b40df08cdac63de520b36d890ec74fb5474ee3`, `da818106a7a181e7faa258be2d30a87e89a0cb23`, and `e4935905b14ed6cf5293d84c1b32e4e45f07c693` add verification, policy-blocker and current-status documentation. The tested exact branch head is `e4935905b14ed6cf5293d84c1b32e4e45f07c693`.

## Exact-head workflows

| Gate | Result | Exact-head run |
|---|---|---|
| Rust canonical verification, including `scripts/ci/verify` | PASS | [41270291604](https://github.com/jelome265/sitolo/actions/runs/41270291604) |
| PostgreSQL application and contract integration | PASS | [41270291610](https://github.com/jelome265/sitolo/actions/runs/41270291610) |
| Security: advisories, dependency policy, secrets, offensive regression and CodeQL | PASS | [41270291603](https://github.com/jelome265/sitolo/actions/runs/41270291603) |
| Policy, including lint, workflow and ICM checks | PASS | [41270291620](https://github.com/jelome265/sitolo/actions/runs/41270291620) |

All four workflow runs identify the same `head_sha`, `e4935905b14ed6cf5293d84c1b32e4e45f07c693`. PostgreSQL execution occurred in the approved disposable CI service. No local PostgreSQL was started. Locally, `python scripts/ci/check-doc-references` passed with 259 Markdown files and 1,151 resolved references, `scripts/ci/check-icm-workspace` passed through Git Bash including the security ICM policy check, and `git diff --check` passed for the status refresh before commit.

## Implementation and evidence disposition

- The source path for logout derives the user and session from the verified request context. Migration 0006 writes revocation and `auth.session.revoked` evidence atomically; its owner-binding, replay/concurrency and audit-failure rollback cases are part of the PostgreSQL integration suite.
- The role-grant policy remains blocked on explicit Product/Security approval of role risk classes, target-role/scope restrictions, separation-of-duties, step-up and second-person approval. No role grant/revoke HTTP route is exposed.
- The latest CI pass does not complete the broader Phase 3 identity lifecycle, mandatory Phase 4 mutation matrix, consumer effect and acknowledgement proof, or production migration/release process.
- PR #49 remains open, unmerged and `REVIEW_REQUIRED`. No independent Stage 06 audit or human approval is claimed. Default-branch required-check and bypass configuration also remains unresolved per the 2026-10-26 ruleset evidence.

## Worktree handling

The pre-existing user edit to `pr49-phase4-part8-verification-2026-10-25-report.md` and untracked `.vs/` directory were not included in this report's changes and must remain untouched.
