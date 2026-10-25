# Phase 4 Part 8 PR-009 Continuation Guide

**PR:** [#49](https://github.com/jelome265/sitolo/pull/49)  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Last tested implementation revision:** `3ee27a3a660557975e602f48535916bd44e82b90`  
**ICM verification-report commit:** `22d276a64baa3254a3842296c315b61cccfa6de5`  
**Status:** NOT READY FOR MERGE; keep open and unmerged

The previous 2026-09-24 execution contract is preserved at `docs/phase4_part8_pr009_remediation_execution_contract_2026-09-24_historical.md`. The 2026-10-24 remediation/verification reports describe their named source/local checks and are historical. Use this guide together with the latest current-status pointer and the exact-head ICM verification artifact.

**Current status:** `docs/phase4_part8_pr009_current_status.md`  
**Exact-head verification:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-verification-2026-10-25-report.md`  
**Prior audit findings snapshot:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-read-only-re-audit-2026-10-25.md`

## 1. Verification status

On the tested implementation revision `3ee27a3`, the PostgreSQL-backed `membership_transaction_integration.rs` target reports **44/44 passed** in CI run [40158776564](https://github.com/jelome265/sitolo/actions/runs/40158776564). This closes the six earlier membership failures from the previous audit snapshot without weakening expected event cardinality.

The fix adds typed `RoleAssignmentOutcome::Created/Replayed` semantics and only writes role-assignment evidence for a real state transition. It also resolves the audit target mismatch and corrects the denied suspend/revoke test actor so the expected `ScopeError::PermissionDenied` is exercised. The API's production dependency boundary is fixed: `sitolo-persistence` and SQLx are development/test-only dependencies of `sitolo-application`. Formatting, strict Clippy, workspace check, 259 library tests, release build and architecture/boundary checks passed in the verification record.

**The PR is not fully green.** The same source revision's `part8_provision_organization_evidence_tests.rs` target reports 15 passed and 3 failed. The runs on PR documentation/report commit `22d276a` were Policy PASS ([40159185647](https://github.com/jelome265/sitolo/actions/runs/40159185647)), Security PASS ([40159185632](https://github.com/jelome265/sitolo/actions/runs/40159185632)), Rust FAIL ([40159185592](https://github.com/jelome265/sitolo/actions/runs/40159185592)) and Integration FAIL ([40159185631](https://github.com/jelome265/sitolo/actions/runs/40159185631)).

Those remaining test failures are:
- `expire_invitation_commits_membership_audit_and_outbox_together` — `EvidencePersistenceFailed`.
- `mark_member_pending_commits_state_audit_and_outbox_together` — `RowNotFound`.
- `concurrent_mark_member_pending_emits_one_semantic_event` — evidence-count mismatch.

The verification report attributes them to the invitation / membership-lifecycle evidence surface, untouched by the scoped role-assignment fix. Record them as **remaining open CI blockers**, not as passes or waived findings. No approved business effect or contract should be invented to force these tests green.

## 2. Current implementation boundary

The API composes managed production secret resolution, Cognito OIDC validation, an `app_runtime` PostgreSQL pool, persisted identity/session authority and the durable tenancy service. Application-owned ports live in `sitolo-application`; PostgreSQL implementations live behind `sitolo-persistence`. The intended mutation, authorization and audit/outbox evidence share one transaction.

Five of the six declared protected routes have authenticated application paths. The organization activation endpoint remains deliberately closed because provisioning currently activates the organization before inserting it, so no persisted `PROVISIONING` state is available for that transition. Keep it closed until the lifecycle contract is implemented, or remove the route from the inventory after an approved contract decision.

The worker relay/SNS FIFO publishing infrastructure exists. Consumer infrastructure and durable inbox support also exist, but the production worker entry point does not yet compose and run the SQS consumer with an owner-approved concrete business effect. Do not describe abstractions or inbox storage alone as complete end-to-end consumer delivery.

## 3. Required work before closure

1. **Independent Stage 06 audit.** Re-audit the final implementation SHA, distinguish resolved findings from the three remaining provision/invitation failures, and record the audit only through the prescribed ICM stage. The Stage 08 verification report is evidence, not Stage 06 approval.
2. **Resolve remaining PostgreSQL failures.** Keep the three `part8_provision_organization_evidence_tests.rs` failures open. Obtain the owner-approved invitation/lifecycle contract needed to determine expected semantics; then implement and regression-test without weakening evidence/rollback assertions.
3. **AUD-01:** approve the SQS FIFO queue/consumer contract and exact business effect; wire the consumer into worker runtime; prove inbox idempotency, effect and database commit are atomic, acknowledgement follows commit, and retry/quarantine paths behave correctly.
4. **AUD-05:** establish the restricted worker role/pool and independently documented migration/release path for identity, tenancy, audit, outbox and inbox schemas.
5. **R3:** complete the source-grounded mandatory-mutation matrix for actor provenance, tenant scope, authorization, state mutation, transactional audit/outbox and denied/no-write proof.
6. **R4 / AUD-04:** trace all approved lifecycle transitions to persisted event schemas and relay acceptance. Catalogue/validator tests alone do not establish that each state transition emits its required evidence.
7. **Organization activation:** implement persisted `PROVISIONING` semantics with contract and tests, or remove the route from the declared API inventory. Do not reopen the handler alone.
8. **PR metadata and branch rules:** the PR description has stale head `90f8f610...` claims, including the already-fixed Application→Persistence/SQLx dependency allegation. Refresh the description before handoff. Ensure the architecture/boundary job is required by branch protection and obtain human review.

## 4. Evidence and merge rule

The verification artifact documents local non-database checks as green and the six assigned membership failures as fixed. GitHub Actions still reports Rust and Integration failing because of the three provision/invitation tests. The last observed check runs are attached to PR head `22d276a`; any later documentation commit can trigger new runs and must be rechecked rather than inheriting a pass claim automatically.

Keep PR #49 open and unmerged. Do not enter delivery or close Phase 4 Part 8 until the remaining test failures are resolved under an approved contract, required workflows pass on the same final source head, AUD-01/AUD-05/R3/R4 gates are evidenced, and independent Stage 06 plus human review are recorded.
