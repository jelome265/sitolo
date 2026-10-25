# Sitolo — Phase 4 Part 8 PR-009 Current Status

**Status:** NOT READY FOR MERGE — three PostgreSQL-backed provision/invitation evidence tests remain failing; independent Stage 06 re-audit is outstanding.  
**PR:** [#49 — Phase 4 Part 8 audit and outbox](https://github.com/jelome265/sitolo/pull/49)  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Current PR head when checked:** `22d276a64baa3254a3842296c315b61cccfa6de5`  
**Implementation commit whose fixes were tested:** `3ee27a3a660557975e602f48535916bd44e82b90`  
**Date:** 2026-10-25  
**Binding contract:** `docs/phase4_part8_audit_outbox_implementation_contract.md`  
**Continuation guide:** `docs/phase4_part8_pr009_remediation_execution_contract.md`  
**Exact-head verification record:** [ICM verification report](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-verification-2026-10-25-report.md)  
**Previous findings snapshot:** [2026-10-25 read-only follow-up findings](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-read-only-re-audit-2026-10-25.md)

## Exact-head CI result

The checks below were observed on PR head `22d276a64baa3254a3842296c315b61cccfa6de5`. These workflows rerun when documentation changes create a new head; do not attribute these results to any later commit without checking that commit's runs.

| Workflow | Result | Run |
|---|---|---|
| Policy | **PASS** | [40159185647](https://github.com/jelome265/sitolo/actions/runs/40159185647) |
| Rust | **FAIL** | [40159185592](https://github.com/jelome265/sitolo/actions/runs/40159185592) |
| Integration | **FAIL** | [40159185631](https://github.com/jelome265/sitolo/actions/runs/40159185631) |
| Security | **PASS** | [40159185632](https://github.com/jelome265/sitolo/actions/runs/40159185632) |

## Resolved findings from the previous audit snapshot

The six membership transaction failures reported against the earlier candidate are fixed. At implementation commit `3ee27a3`, `membership_transaction_integration.rs` reports **44/44 passing** in PostgreSQL-backed CI (run [40158776564](https://github.com/jelome265/sitolo/actions/runs/40158776564)). The remediation adds typed `RoleAssignmentOutcome::Created/Replayed` semantics, emits role-assignment evidence only for a new transition, corrects the audit target identity, makes denied suspend/revoke requests produce the contract-defined permission-denied result, and preserves exact event-count assertions.

The API crate's production dependency on `sitolo-persistence`/SQLx is also already removed; the three dependencies are development/test-only. Format, strict Clippy, workspace check, 259 library tests, release build, architecture/boundary checks, R2 route-boundary tests, R4 event-catalogue tests, policy checks and ICM validation are reported green in the linked verification record.

## Remaining exact-head failures

The PostgreSQL target `part8_provision_organization_evidence_tests.rs` reports **15 passed, 3 failed** on implementation commit `3ee27a3` and keeps Rust/Integration CI red:

1. `expire_invitation_commits_membership_audit_and_outbox_together` — `EvidencePersistenceFailed`.
2. `mark_member_pending_commits_state_audit_and_outbox_together` — `RowNotFound`.
3. `concurrent_mark_member_pending_emits_one_semantic_event` — expected evidence count mismatch.

The verification artifact identifies these as outside the six-test role-assignment remediation slice because that code change does not touch the invitation / membership-lifecycle path. That scope attribution does **not** make the failures green or waive them as release gates. They remain documented open work related to invitation/lifecycle evidence, AUD-01 and R4, and require the approved owner contract before a business effect is invented. Do not weaken assertions or claim overall CI passes.

## Remaining Phase 4 Part 8 closure gates

- **AUD-01:** approve the SQS FIFO queue/consumer contract and exact business effect; compose the consumer/inbox flow into the worker runtime and prove inbox deduplication, effect and commit are atomic, with acknowledgement after commit.
- **AUD-05:** finish the restricted worker role/pool and documented migration/release path for identity, tenancy, audit, outbox and inbox schemas.
- **R3:** complete a source-grounded mutation matrix covering principal/actor, tenant scope, authorization, state mutation, audit/outbox transaction, and denied/no-write evidence.
- **R4 / AUD-04:** trace each approved lifecycle transition through persisted event schema and relay acceptance; catalogue/schema validation alone is not closure.
- **Organization activation:** implement a real persisted `PROVISIONING` transition under contract, or remove the activation route from the declared API inventory; keep it closed until then.
- **Stage 06 and human review:** independent re-audit at the final implementation SHA, followed by required human review. The linked verification artifact is Stage 08 evidence, **not** independent Stage 06 approval.
- **PR metadata:** the current PR description still contains stale head `90f8f610...` and an already-remediated Application→Persistence/SQLx dependency allegation. Refresh it before handoff; do not use those stale claims as current evidence.

## Disposition

Keep PR #49 **open and unmerged**. The six assigned membership failures are proven fixed, but the overall Rust and Integration workflows still fail on three provision/invitation evidence tests and the independent Stage 06 re-audit has not occurred. Do not mark the PR merge-ready, close Phase 4 Part 8, or proceed to delivery until unresolved contract gates and repository merge requirements are satisfied.
