---
artifact_type: repository-settings-evidence
status: verified-incomplete
run_slug: "pr009-part8"
security_relevant: yes
observed_pr_head: "1eba7f976fab4635662415b63881174ee3b73e29"
ruleset_id: 22459107
date: "2026-10-26"
---

# PR #49 branch ruleset evidence

## Observed configuration

Read-only GitHub API inspection on 2026-10-26 found:

This inspection supersedes older notes that the ruleset API returned 403; the current authenticated read returned the active ruleset data below.

- Classic branch protection for `main` returns HTTP 404 (`Branch not protected`).
- Repository ruleset `22459107` named `main` is active and targets `~DEFAULT_BRANCH`.
- Its rules are `non_fast_forward` and `pull_request` with one required approving review.
- It has no required status-check rule. Thus Rust, Integration, Security and Policy workflow success is not enforced by this ruleset before updating `main`.
- `dismiss_stale_reviews_on_push`, `require_code_owner_review`, `require_last_push_approval`, and `required_review_thread_resolution` are false. A repository-role bypass actor has `always` bypass mode; the authenticated user can bypass.

For PR head `1eba7f976fab4635662415b63881174ee3b73e29`, `gh pr checks 49` reports the Rust `verify`, Integration application/contract tests, Security jobs, Policy, and CodeQL checks all successful. These are observed results, not evidence that the ruleset requires them.

## Disposition and required action

The repository settings do not satisfy the work order's required-check gate. A repository administrator must configure the active default-branch ruleset to require the canonical Rust, Integration, Security and Policy checks and review the always-bypass role against the human-review requirement. Re-read the ruleset after configuration and record the resulting exact contexts and review/bypass controls. Do not claim branch protection is complete until that evidence exists.

This record documents read-only inspection; it does not change repository settings. PR #49 remains open and review-required.
