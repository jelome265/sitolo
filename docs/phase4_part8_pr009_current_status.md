# Sitolo — Phase 4 Part 8 PR-009 Current Status

**Status:** NOT READY FOR MERGE — the known RLS metadata failure is fixed; this branch still has unresolved Phase 1–4 release blockers and independent Stage 06 review is outstanding.
**PR:** [#49 — Phase 4 Part 8 audit and outbox](https://github.com/jelome265/sitolo/pull/49)
**Branch:** `feat/phase4-pr009-audit-outbox`
**Latest implementation candidate:** `e48b12a5995b4d79aca86a7443b014144d893164`
**Latest exact-head workflows:** [Rust](https://github.com/jelome265/sitolo/actions/runs/40174232907), [Integration](https://github.com/jelome265/sitolo/actions/runs/40174232901), [Policy](https://github.com/jelome265/sitolo/actions/runs/40174232875), and [Security](https://github.com/jelome265/sitolo/actions/runs/40174232895) all pass on `e48b12a5995b4d79aca86a7443b014144d893164`.
**Latest remediation:** commits `289ae0c` (database target guards) and `e48b12a` (optional fingerprint encoding).
**Cross-phase evidence:** [Phase 1–4 deep audit](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/phase1-4-deep-audit-2026-10-25.md), [remediation plan](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/phase1-4-remediation-plan-2026-10-25.md), [follow-up index](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/README.md).
**Verification evidence:** [ICM verification report](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-verification-2026-10-25-report.md)
**Continuation guide:** `docs/phase4_part8_pr009_remediation_execution_contract.md`  
**Date:** 2026-10-25

## Remaining release blockers

- Production consumer/effect composition, approved resource identities, inbox transaction and acknowledgement-after-commit evidence remain unresolved.
- The canonical production migration/release owner and immutable migration artifact remain unresolved.
- The complete mandatory mutation matrix and Phase 3 lifecycle evidence are not demonstrated.
- Organization activation remains intentionally closed pending an approved lifecycle decision.
- `sitolo-testkit` remains an unused scaffold; lint/CI policy gaps and actual branch-protection requirements still need closure.
- An independent Stage 06 audit and human review are still required. PR #49 remains open and unmerged.

The detailed earlier records below are historical checkpoints; their run results and failure descriptions do not describe the latest implementation candidate.

## Historical checkpoint: PR head `a5dffa4`

| Workflow | Result | Run |
|---|---|---|
| Policy | **PASS** | [40161977349](https://github.com/jelome265/sitolo/actions/runs/40161977349) |
| Security | **PASS** | [40161977235](https://github.com/jelome265/sitolo/actions/runs/40161977235) |
| Integration | **PASS** | [40161977330](https://github.com/jelome265/sitolo/actions/runs/40161977330) |
| Rust | **FAIL** | [40161977327](https://github.com/jelome265/sitolo/actions/runs/40161977327) |

These workflows evaluated PR head `a5dffa4fe734cb82717a783f552d4f7489bfd4ff`. The report-only ICM updates after that head do not change implementation code; any newer head must nevertheless be checked for its own workflow results instead of inheriting the run status by assumption.

## Resolved remediation findings

- **Six membership transaction failures:** fixed; `membership_transaction_integration.rs` passes **44/44**.
- **Three provision/invitation evidence failures:** fixed; `part8_provision_organization_evidence_tests.rs` passes **18/18**. Root cause was `organization_id` leaking into the strict `iam.membership.pending_acceptance` payload. Commit `b095768` removes only that extra payload key; `organization_id` remains a dedicated audit column. No test assertions were weakened.
- **Application dependency boundary:** Application owns its ports; the persistence/SQLx dependencies are development/test-only, not production dependencies.
- **Local non-database gates:** reported green at the tested implementation revision, including formatting, strict Clippy, workspace check, 259 library tests, release build, R2 route proof, R4 schema/catalogue tests, architecture/crate boundaries, Phase 2/workflow/security-ICM checks, doc references, System Map, cargo-deny and ICM cold walk.

## Remaining Rust CI failure

Rust run [40161977327](https://github.com/jelome265/sitolo/actions/runs/40161977327) fails in `test_part8_relation_ownership_and_rls_metadata`: four policies are observed where the test expects three. The extra policy is `consumer_inbox_tenant_policy`, defined in `crates/sitolo-persistence/schema/consumer_inbox.sql` on `consumer_inbox`, targeting `app_worker`. The policy was added earlier in `0cdd771`; the payload fix and documentation-only changes do not modify it.

The failure is outside the scope of the latest payload-remediation change, but it remains a red required workflow until correctly dispositioned. The next step is not to assume the test is wrong or that the policy is wrong: compare the assertion's intended table scope with the binding RLS/consumer-inbox contract, then make only the contract-supported correction. Do not relax the test or alter the worker policy without that decision.

## Remaining Phase 4 Part 8 gates

- **RLS metadata:** resolve the policy-count finding against the binding table/role contract and obtain a passing Rust workflow or formally approved exception.
- **AUD-01:** owner-approved SQS FIFO queue/consumer contract and business effect; consumer runtime wiring; transactional inbox/effect/commit proof; acknowledgement only after commit; retry and quarantine proof.
- **AUD-05:** restricted worker role/pool and documented migration/release path across identity, tenancy, audit, outbox and inbox.
- **R3:** source-grounded mutation matrix for actor, scope, authorization, state mutation, transactional audit/outbox and denied/no-write proof.
- **R4 / AUD-04:** trace approved lifecycle transitions through persisted event schema and relay acceptance.
- **Organization activation:** implement a contract-backed persisted `PROVISIONING` transition or remove the route from the inventory; keep it closed until resolved.
- **Independent Stage 06 and human review:** the Stage 08 verification report is evidence, not independent audit approval.
- **PR description:** its body still contains stale head and Application→Persistence/SQLx claims; refresh it before reviewer handoff.

## Disposition

PR #49 remains **OPEN and UNMERGED**. The six membership failures and three provision/invitation failures are proven fixed. Policy, Security and Integration pass on the documented candidate, while Rust remains red on the consumer-inbox RLS metadata assertion. Do not mark the PR merge-ready, close Phase 4 Part 8 or proceed to delivery until the RLS finding has a contract-backed disposition, AUD-01/AUD-05/R3/R4 have evidence, and independent Stage 06 plus human review are recorded.
