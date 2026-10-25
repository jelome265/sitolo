# Phase 4 Part 8 PR-009 Continuation Guide

**PR:** [#49](https://github.com/jelome265/sitolo/pull/49)  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Current implementation source:** `4858341fcd16459aad10494d8b75ddb684859861`
**Latest exact-head check set:** `0f70426ffc384b5f5ae69e3578b672a5777f0eda` — Rust [40178438974](https://github.com/jelome265/sitolo/actions/runs/40178438974), Integration [40178438961](https://github.com/jelome265/sitolo/actions/runs/40178438961), Security [40178438971](https://github.com/jelome265/sitolo/actions/runs/40178438971), Policy [40178439059](https://github.com/jelome265/sitolo/actions/runs/40178439059), all PASS.
**Lifecycle-payload fix:** `b095768f9903c86b399c13d788714ebd0294c2a2`  
**Status:** NOT READY FOR MERGE; keep open and unmerged. Consumer effect, canonical migration authority, full mutation and Phase 3 identity evidence, branch-protection evidence, independent Stage 06 and human review remain outstanding.

Use this guide with `docs/phase4_part8_pr009_current_status.md` and the dated ICM verification artifact:

`workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-verification-2026-10-25-report.md`

The earlier 2026-09-24 execution contract and the historical checkpoints below are historical evidence, not a replacement for current verification or independent Stage 06 review. The consumer-inbox RLS metadata failure described below was fixed on later source; do not treat it as a current failure.

## Current remediation checkpoint — 2026-10-25

- Database target guards and versioned optional-field fingerprint coverage pass exact-head workflows (see the remediation execution report).
- `sitolo-testkit` now provides validated tenancy fixtures consumed by tenancy-domain and application tests; the dependency is test-only and the boundary checker enforces that rule.
- The unavailable organization activation route was removed from the active API inventory and contract surface. Provisioning commits a complete active organization with its owner and default branch; the boundary regression verifies that the removed path returns 404.
- Strict Rust lint/CI policy enforcement and stale `SCOPE-EXC-001` reconciliation are implemented. Repository branch-protection settings remain unverified because the API returned 403.
- The [requirements matrix](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/phase1-4-requirements-matrix-2026-10-25.md) and [remediation execution report](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/phase1-4-remediation-execution-2026-10-25.md) record the exact current implementation evidence and blockers.

## 1. Exact-head CI

The following results correspond to PR head `a5dffa4fe734cb82717a783f552d4f7489bfd4ff`:

| Workflow | Result | Run |
|---|---|---|
| Policy | PASS | [40161977349](https://github.com/jelome265/sitolo/actions/runs/40161977349) |
| Security | PASS | [40161977235](https://github.com/jelome265/sitolo/actions/runs/40161977235) |
| Integration | PASS | [40161977330](https://github.com/jelome265/sitolo/actions/runs/40161977330) |
| Rust | FAIL | [40161977327](https://github.com/jelome265/sitolo/actions/runs/40161977327) |

The current ICM report includes this head-specific result set. Documentation-only commits made after `a5dffa4` trigger new runs; check those new results rather than copying the earlier statuses forward automatically.

## 2. Remediation proven green

### Six membership transaction failures

`membership_transaction_integration.rs` passes **44/44**. The assignment evidence path distinguishes `RoleAssignmentOutcome::Created` from `Replayed`; replay does not emit another audit/outbox pair. Audit target identity and denied suspend/revoke semantics were corrected without weakening expected event counts or no-write assertions.

### Three provision/invitation failures

`part8_provision_organization_evidence_tests.rs` passes **18/18** in [Integration run 40160964822](https://github.com/jelome265/sitolo/actions/runs/40160964822), and the Integration workflow also passes on candidate head `a5dffa4`.

Root cause: `transition_membership_with_evidence` included `organization_id` in the strict `iam.membership.pending_acceptance` payload. The schema allows only `membership_id`, `from_state`, `to_state`, and `state_version`; the extra key caused payload validation and evidence persistence to fail. Commit `b095768` removes that payload key; `organization_id` remains in the dedicated audit column. No tests were weakened and no unapproved consumer business effect was invented.

### Architecture and local gates

The Application → Persistence/SQLx production dependency violation is resolved: application-owned ports define the boundary, while direct persistence/SQLx dependencies in `sitolo-application` are dev/test-only. The recorded local non-database gate set passes formatting, strict Clippy, workspace check, 259 library tests, release build, R2 route proof, R4 catalogue/schema tests, architecture/crate boundaries, policy checks, doc references, System Map, cargo-deny and ICM cold walk.

## 3. Historical Rust blocker — superseded

Rust run [40161977327](https://github.com/jelome265/sitolo/actions/runs/40161977327) failed in `test_part8_relation_ownership_and_rls_metadata`: the test observed four policies where it expected three. This failure is historical and is superseded by later exact-head passing Rust workflows.

The historical run remains retained for provenance. Current source and current workflow status govern.

## 4. Remaining closure gates

1. **AUD-01:** approve the SQS FIFO queue/consumer contract and exact business effect; compose the consumer in the worker runtime; prove inbox deduplication, effect and commit are atomic, acknowledgement follows commit, and retry/quarantine behave correctly.
2. **AUD-05:** complete the restricted worker role/pool and documented migration/release path for identity, tenancy, audit, outbox and inbox, with a named database/platform owner and immutable artifact.
3. **R3:** complete independent source-to-test and PostgreSQL evidence for every required mutation, including actor provenance, scope, authorization, audit/outbox failures, denial/no-write, replay and concurrency.
4. **Phase 3:** complete the identity/session/refresh/MFA/recovery/device requirement and real-provider/PostgreSQL evidence matrix.
5. **Repository settings:** an administrator must provide branch-protection and mandatory-review evidence; API access returned 403.
6. **Independent review:** obtain and record Stage 06 re-audit against the final implementation source, then human review. Stage 08 verification is not independent Stage 06 approval.
7. **PR handoff metadata:** refresh the PR description and comment to the latest verified branch head before reviewer handoff.

## 5. Closure rule

PR #49 stays open and unmerged. The historical Rust RLS failure is resolved; all four current exact-head workflows pass on `0f70426`. Do not close Phase 4 Part 8 or enter delivery until AUD-01, AUD-05, R3, Phase 3 identity evidence, branch-protection evidence, independent Stage 06 and human review are resolved.
