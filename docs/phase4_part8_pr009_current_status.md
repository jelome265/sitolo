# Sitolo — Phase 4 Part 8 PR-009 Current Status

**Status:** NOT READY FOR MERGE — Integration passes, but Rust CI remains blocked by one PostgreSQL-backed RLS metadata test; independent Stage 06 re-audit is outstanding.  
**PR:** [#49 — Phase 4 Part 8 audit and outbox](https://github.com/jelome265/sitolo/pull/49)  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Implementation fix for lifecycle payload:** `b095768f9903c86b399c13d788714ebd0294c2a2`  
**ICM verification-report commit:** `555f2d5e5631fe6d879e2ff4c92a5ec7a1e25773`  
**Date:** 2026-10-25  
**Binding contract:** `docs/phase4_part8_audit_outbox_implementation_contract.md`  
**Continuation guide:** `docs/phase4_part8_pr009_remediation_execution_contract.md`  
**Exact-head verification record:** [ICM verification report](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-verification-2026-10-25-report.md)  
**Historical audit-finding snapshot:** [2026-10-25 read-only follow-up findings](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-read-only-re-audit-2026-10-25.md)

## Exact-head CI evidence

The following workflow runs correspond to PR head `555f2d5e5631fe6d879e2ff4c92a5ec7a1e25773`, which archives the verification evidence for implementation fix `b095768`.

| Workflow | Result | Run |
|---|---|---|
| Policy | **PASS** | [40161606057](https://github.com/jelome265/sitolo/actions/runs/40161606057) |
| Integration | **PASS** | [40161606055](https://github.com/jelome265/sitolo/actions/runs/40161606055) |
| Security | **PASS** | [40161606063](https://github.com/jelome265/sitolo/actions/runs/40161606063) |
| Rust | **FAIL** | [40161606051](https://github.com/jelome265/sitolo/actions/runs/40161606051) |

These results are specific to the stated head. Any subsequent documentation commit creates a newer head and may trigger another set of workflows; do not inherit a result without checking the new head.

## Findings resolved

### Membership transaction idempotency and authorization

The six previously identified failures in `membership_transaction_integration.rs` are fixed. The suite reports **44/44 passing** on implementation revision `3ee27a3`, and the later Integration workflow passes on `b095768`. The fix introduces `RoleAssignmentOutcome::Created/Replayed`, records role evidence only for a genuine state transition, reconciles audit target identity, and verifies contract-defined permission denials without weakening event-count or no-write assertions.

The Application → Persistence/SQLx production dependency violation is also resolved. Application-owned ports are defined in `sitolo-application`; direct `sitolo-persistence` / SQLx dependencies in the application crate are development/test-only. The exhaustive architecture/boundary job and strict Clippy pass in the recorded verification.

### Membership lifecycle event payload (commit `b095768`)

The three prior provision/invitation evidence failures are fixed and proven green by [Integration run 40160964822](https://github.com/jelome265/sitolo/actions/runs/40160964822).

Root cause: `transition_membership_with_evidence` emitted `organization_id` in the `iam.membership.pending_acceptance` payload. The strict event validator permits only `membership_id`, `from_state`, `to_state` and `state_version`. That extra field caused schema rejection and evidence-write failures. The fix removes only that payload key; `organization_id` remains available as the dedicated audit table column. No business effect was invented and no test assertion was weakened.

## Remaining Rust CI failure

The Rust workflow fails in `test_part8_relation_ownership_and_rls_metadata`: the assertion expects three Part 8 RLS policies but counts four because the unscoped `pg_policies` query includes `consumer_inbox_tenant_policy`, defined in `crates/sitolo-persistence/schema/consumer_inbox.sql` for the `app_worker` role.

The verification record attributes this to an earlier consumer-inbox addition (`0cdd771`), not to the one-line payload correction in `b095768`. The failure is in the consumer/inbox security area and is tracked with AUD-01. That historical attribution does **not** make Rust CI green or automatically waive the finding. Before merge, the team must establish whether the assertion’s scope is wrong, whether the policy violates the binding Part 8 contract, or whether a documented owner decision is required. Do not change the policy or weaken the test without tracing the intended table/role contract.

## Remaining Phase 4 Part 8 closure gates

- **Rust CI / RLS metadata:** disposition `test_part8_relation_ownership_and_rls_metadata` and obtain a passing required Rust workflow or an explicitly approved governance exception. Do not label the PR all-green while the check fails.
- **AUD-01:** approve the SQS FIFO queue/consumer contract and exact business effect; compose the consumer/inbox flow into the worker runtime and prove inbox deduplication, effect and commit are atomic, acknowledgement occurs after commit, and retry/quarantine paths are correct.
- **AUD-05:** complete the restricted worker role/pool and documented migration/release path across identity, tenancy, audit, outbox and inbox schemas.
- **R3:** complete the source-grounded mutation matrix for actor provenance, tenant scope, authorization, state mutation, audit/outbox transaction, and denied/no-write evidence.
- **R4 / AUD-04:** trace each approved lifecycle transition through persisted event schema and relay acceptance; validator/catalogue tests alone do not close this gate.
- **Organization activation:** implement a real persisted `PROVISIONING` transition under contract, or remove the activation route from the declared API inventory. Keep it closed until resolved.
- **Stage 06 and human review:** complete the independent re-audit against the final implementation SHA, then record the required human review. The Stage 08 verification artifact is evidence, **not** independent Stage 06 approval.
- **PR metadata:** the PR description still contains stale head and architecture claims. Refresh it before reviewer handoff.

## Disposition

PR #49 remains **OPEN and UNMERGED**. The six membership tests and three provision/invitation tests are now green in PostgreSQL-backed CI, and Policy/Security/Integration pass on the documented head. Rust CI still fails on the consumer-inbox RLS metadata assertion. Do not call the PR merge-ready, close Phase 4 Part 8, or proceed to delivery until that failure is correctly dispositioned, other AUD-01/AUD-05/R3/R4 gates are evidenced, and independent Stage 06 plus human review are recorded.
