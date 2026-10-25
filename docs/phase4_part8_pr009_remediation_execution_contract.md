# Phase 4 Part 8 PR-009 Continuation Guide

**PR:** [#49](https://github.com/jelome265/sitolo/pull/49)  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Lifecycle-payload fix:** `b095768f9903c86b399c13d788714ebd0294c2a2`  
**Latest completed CI candidate:** `a5dffa4fe734cb82717a783f552d4f7489bfd4ff`  
**Status:** NOT READY FOR MERGE; keep open and unmerged

Use this guide with `docs/phase4_part8_pr009_current_status.md` and the dated ICM verification artifact:

`workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-verification-2026-10-25-report.md`

The earlier 2026-09-24 execution contract and the previous read-only findings snapshot are historical evidence, not a replacement for current verification or independent Stage 06 review.

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

## 3. Remaining Rust blocker

Rust run [40161977327](https://github.com/jelome265/sitolo/actions/runs/40161977327) fails in `test_part8_relation_ownership_and_rls_metadata`: the test observes four policies where it expects three. The extra policy is `consumer_inbox_tenant_policy` from `crates/sitolo-persistence/schema/consumer_inbox.sql`, attached to `consumer_inbox` and targeting `app_worker`. It was introduced earlier in `0cdd771`; the latest payload fix and documentation-only changes do not modify the SQL or the test.

This is outside the changed payload's scope, but it remains a failing Rust check and is **not waived**. Establish whether the test intended to enumerate only Part 8 evidence-table policies or whether the worker policy violates the binding RLS/consumer contract. Then change only the contract-supported component and add/retain PostgreSQL proof. Do not weaken the test or alter the policy based on the count alone.

## 4. Remaining closure gates

1. **RLS metadata finding:** make a contract-backed disposition of the policy-count failure; obtain a passing required Rust workflow or a formally approved exception.
2. **AUD-01:** approve the SQS FIFO queue/consumer contract and exact business effect; compose the consumer in the worker runtime; prove inbox deduplication, effect and commit are atomic, acknowledgement follows commit, and retry/quarantine behave correctly.
3. **AUD-05:** complete the restricted worker role/pool and documented migration/release path for identity, tenancy, audit, outbox and inbox.
4. **R3:** complete a source-grounded mutation matrix for actor provenance, tenant scope, authorization, mutation, transactional audit/outbox and denied/no-write evidence.
5. **R4 / AUD-04:** trace all approved lifecycle transitions from state mutation to persisted event schema and relay acceptance.
6. **Organization activation:** implement a contract-backed persisted `PROVISIONING` transition or remove the route from the declared inventory; keep it closed until resolved.
7. **Independent review:** obtain and record Stage 06 re-audit against the final implementation source, then human review. Stage 08 verification is not independent Stage 06 approval.
8. **PR handoff metadata:** the PR description still contains a stale source head and an obsolete Application→Persistence/SQLx dependency allegation. Refresh it before reviewer handoff.

## 5. Closure rule

PR #49 stays open and unmerged. The six membership tests and three provision/invitation evidence tests are fixed. Policy, Security and Integration pass on candidate `a5dffa4`, but Rust fails on the consumer-inbox RLS metadata assertion. Do not close Phase 4 Part 8 or enter delivery until the RLS finding has a contract-backed disposition, AUD-01/AUD-05/R3/R4 are evidenced, and independent Stage 06 plus human review are recorded.
