# Phase 4 Part 8 PR-009 Continuation Guide

**PR:** [#49](https://github.com/jelome265/sitolo/pull/49)  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Lifecycle-payload fix:** `b095768f9903c86b399c13d788714ebd0294c2a2`  
**Verification evidence commit:** `555f2d5e5631fe6d879e2ff4c92a5ec7a1e25773`  
**Status:** NOT READY FOR MERGE; keep open and unmerged

The earlier 2026-09-24 execution contract remains historical. Use the current-status document and the dated ICM verification report for current evidence.

**Current status:** `docs/phase4_part8_pr009_current_status.md`  
**Exact-head verification:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-verification-2026-10-25-report.md`  
**Historical findings snapshot:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-read-only-re-audit-2026-10-25.md`

## 1. Verification status

The three provision/invitation evidence failures have been resolved and proven green by [Integration run 40160964822](https://github.com/jelome265/sitolo/actions/runs/40160964822). Root cause: `transition_membership_with_evidence` included `organization_id` in the strict `iam.membership.pending_acceptance` payload. The event schema permits only `membership_id`, `from_state`, `to_state`, and `state_version`. Removing the extra JSON key restores evidence persistence while keeping `organization_id` in the dedicated audit column. No tests were weakened and no new business effect was invented.

The six previously failing membership transaction tests also remain fixed: `membership_transaction_integration.rs` reports **44/44 passing** on implementation revision `3ee27a3`, with later PostgreSQL Integration CI green on `b095768`. The role-assignment outcome now distinguishes `Created` from `Replayed`, so identical retries do not emit a second audit/outbox evidence pair.

The Application → Persistence/SQLx production dependency violation is resolved: production code uses application-owned ports; the persistence and SQLx dependencies in `sitolo-application` are dev/test-only. Strict Clippy and the independent architecture/boundary check pass in the verification evidence.

## 2. Latest documented CI

The latest fully observed result set is for the report commit `555f2d5e5631fe6d879e2ff4c92a5ec7a1e25773`:

| Workflow | Result | Run |
|---|---|---|
| Policy | PASS | [40161606057](https://github.com/jelome265/sitolo/actions/runs/40161606057) |
| Security | PASS | [40161606063](https://github.com/jelome265/sitolo/actions/runs/40161606063) |
| Integration | PASS | [40161606055](https://github.com/jelome265/sitolo/actions/runs/40161606055) |
| Rust | FAIL | [40161606051](https://github.com/jelome265/sitolo/actions/runs/40161606051) |

These checks apply to that exact head. A later documentation commit triggers new workflow runs; verify the new source head rather than carrying statuses forward automatically.

## 3. Remaining Rust blocker: Part 8 RLS policy-count assertion

Rust CI still fails in `test_part8_relation_ownership_and_rls_metadata`: it expects exactly three Part 8 RLS policies but observes four. The additional row is `consumer_inbox_tenant_policy` from `crates/sitolo-persistence/schema/consumer_inbox.sql`, attached to the consumer inbox table and targeting the `app_worker` role. The Part 8 test's `pg_policies` query currently counts the extra consumer-inbox policy.

The verification artifact identifies this policy as pre-existing (added in `0cdd771`) and outside the one-line lifecycle payload change in `b095768`. That attribution alone does not resolve the failed CI check. The next action is to establish the intended scope from the binding contracts:
- Is the assertion intended to count policies only on the Part 8 audit/outbox evidence tables, in which case the query may need to filter to those tables?
- Or does the policy's existence/role target actually violate the binding security contract?

Do not change the policy or weaken the assertion before answering that contract question. Any corrective change needs a test that verifies the intended relation/role ownership rule and live PostgreSQL RLS behavior.

## 4. Remaining Part 8 closure gates

1. **RLS metadata finding:** resolve `test_part8_relation_ownership_and_rls_metadata` according to the binding table/role contract and obtain a passing required Rust workflow or a formally documented, approved exception.
2. **AUD-01:** obtain owner approval for the SQS FIFO queue/consumer contract and exact business effect; compose the consumer into the worker runtime; prove inbox deduplication, effect and commit are one transaction, acknowledgement follows commit, and retry/quarantine behaviour is correct.
3. **AUD-05:** complete the restricted worker role/pool and documented migration/release path for identity, tenancy, audit, outbox and inbox schemas.
4. **R3:** complete a source-grounded mutation matrix for actor provenance, tenant scope, authorization, state mutation, transactional audit/outbox and denied/no-write proof.
5. **R4 / AUD-04:** trace all approved lifecycle transitions from state mutation through persisted event schema and relay acceptance. Catalogue/schema validation alone is insufficient.
6. **Organization activation:** introduce a contract-backed persisted `PROVISIONING` transition or remove the route from the declared inventory; keep it closed until that decision is implemented.
7. **Independent review:** request and record Stage 06 re-audit against the final implementation source, then obtain the required human review. Stage 08 verification evidence is not Stage 06 audit approval.
8. **Handoff metadata:** refresh the stale PR description before reviewer handoff; it still contains old current-head and Application→Persistence/SQLx claims that no longer describe the code.

## 5. Closure rule

PR #49 remains open and unmerged. The six membership transaction failures and three provision/invitation evidence failures are fixed. Policy, Security and Integration pass on the currently documented head, but Rust remains red because of the RLS metadata assertion. Phase 4 Part 8 must not close and delivery must not begin until that finding is correctly dispositioned, AUD-01/AUD-05/R3/R4 are evidenced, and independent Stage 06 plus human review are complete.
