# Phase 4 Part 8 PR-009 Continuation Guide

**PR:** [#49](https://github.com/jelome265/sitolo/pull/49)  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Implementation code commit:** `5a128c4f95db61f7f6cf9fd885d22911b78553c1`  
**CI-evaluated source head:** `68f5ae5be7cdb8659e0cbd4f90fae16a7ee24a85`  
**Status:** NOT READY; do not merge

The previous 2026-09-24 execution contract is preserved at `docs/phase4_part8_pr009_remediation_execution_contract_2026-09-24_historical.md`. The 2026-10-24 remediation and verification reports are retained as records of their named source/local checks; neither is evidence that the PostgreSQL-backed suite passed on the source head assessed below.

**Current exact-head findings:** [ICM follow-up record](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-read-only-re-audit-2026-10-25.md)  
**Current status pointer:** `docs/phase4_part8_pr009_current_status.md`

## 1. Exact-head CI status

At source head `68f5ae5be7cdb8659e0cbd4f90fae16a7ee24a85`:

| Workflow | Result |
|---|---|
| Policy | PASS — [run 40156547558](https://github.com/jelome265/sitolo/actions/runs/40156547558) |
| Rust | FAIL — [run 40156547515](https://github.com/jelome265/sitolo/actions/runs/40156547515) |
| Integration | FAIL — [run 40156547478](https://github.com/jelome265/sitolo/actions/runs/40156547478) |
| Security | PASS — [run 40156547500](https://github.com/jelome265/sitolo/actions/runs/40156547500) |

The Rust and Integration workflows each report 38 passing and 6 failing PostgreSQL-backed application transaction tests. The failures concern role-assignment audit lookup, suspend/revoke permission error mapping, and duplicate audit/outbox evidence across assignment retry/idempotency and revoke/regrant. These results override any earlier “CI pending” or “all checks passed” wording for this source candidate.

The architecture/boundary job and strict Clippy pass on the evaluated candidate. The production issue is not the lint gate; it is transaction/evidence/authorization correctness.

## 2. Current implementation state

The API process composes managed production secret resolution, Cognito OIDC issuer validation, a PostgreSQL `app_runtime` pool and schema checks, persisted identity/session authority, and the durable tenancy service. Application-owned ports live in `sitolo-application`; PostgreSQL implementations live behind `sitolo-persistence`. The intended transaction includes authorization, the business mutation, audit evidence and outbox intent.

The declared protected-route inventory has six routes. Five are wired to authenticated application commands:

- `POST /v1/organizations` — owner is derived from the verified principal.
- `POST /v1/organizations/{organization_id}/branches`.
- `POST /v1/organizations/{organization_id}/branches/{branch_id}/activate`.
- `POST /v1/organizations/{organization_id}/members/{membership_id}/suspend`.
- `POST /v1/organizations/{organization_id}/members/{membership_id}/revoke`.

The sixth, `POST /v1/organizations/{organization_id}/activate`, is explicitly closed in `CLOSED_PROTECTED_ROUTES`. `provision_organization` activates the organization before insertion, so the persisted `PROVISIONING` state needed by this route does not currently exist. The handler authenticates and returns `503`; undeclared protected routes fail startup via `InvalidRouteInventory`.

The R2 route-boundary test passes and proves all six declared routes reject missing credentials. It does not complete the database-backed positive/negative mutation matrix.

### Role mutation/evidence defects

The role-assignment path records `RoleAssigned` evidence after the persistence call without a typed result that distinguishes a newly committed transition from an identical replay or no-op. The revocation path similarly records `RoleRemoved` after the persistence call. Exact-head tests observe extra audit/outbox rows under retry/idempotency and revoke/regrant.

The assignment evidence test also queries `target_ref = 'assignment-target'` while the application call supplies `TargetRef::Membership(membership_id)`. Resolve intended audit target semantics from the binding contract before aligning writer and assertion. Suspend/revoke negative tests receive an error variant other than the expected permission-denied variant; the current CI output does not name the actual variant. See the linked follow-up findings for the exact tests and correction constraints.

### Worker/consumer state

The worker executable composes and runs the bounded outbox relay with the SNS FIFO publisher. Consumer infrastructure also exists in `apps/worker/src/consumer.rs` and `apps/worker/src/sqs.rs`, with durable inbox support in `crates/sitolo-persistence/src/consumer_inbox.rs` and `schema/consumer_inbox.sql`. However, `apps/worker/src/main.rs` does not compose/run the SQS consumer processor or an owner-approved concrete business effect. Infrastructure existence must not be described as end-to-end consumer completion.

## 3. Required work, in order

1. **Fix semantic idempotency and evidence cardinality.** Make role mutation outcomes distinguish created transition, identical replay and conflict. Emit transition evidence only when state changes. Correct revoke/regrant semantics while preserving history and deterministic handling of the effective-role uniqueness constraint.
2. **Resolve audit-target and authorization-error contracts.** Specify the assignment evidence target. Trace suspend/revoke denial through scope/policy evaluation and error mapping. Prove denied commands change neither authoritative state nor audit/outbox rows; prove mandatory-evidence failure rolls back the business mutation.
3. **Re-run the complete PostgreSQL-backed workflows on one source head.** Do not replace these with library-only tests or local non-database gates. Include concurrency, replay, revoke/regrant and denied-request no-write assertions.
4. **Close AUD-01.** Obtain owner approval for the SQS FIFO queue/consumer contract and exact business effect. Compose the consumer into the worker runtime; ensure inbox deduplication, effect and commit share one database transaction, and acknowledge only after commit. Verify retry and quarantine/dead-letter behavior.
5. **Close AUD-05.** Finish the restricted worker role/pool and documented migration/release path across identity, tenancy, audit, outbox and inbox schemas.
6. **Resolve organization activation.** Either introduce a real persisted `PROVISIONING` lifecycle with contract and tests, or remove this activation route from the declared inventory/contract. Do not change route status alone.
7. **Complete R3 and R4/AUD-04.** Build the source-grounded mutation matrix for principal/actor, scope, authorization, mutation, transactional evidence, and denial proofs. Trace each approved event from actual domain transition to persisted schema and relay acceptance.
8. **Run independent governance review.** Request the Stage 06 re-audit against the final implementation source and complete required human review. Do not place this supplemental follow-up in `stages/06-audit/output/` or present it as independent approval.
9. **Refresh release documentation.** After fixes, update the remediation/verification/current-status documents and PR description to the final source SHA. Check that branch protection requires the architecture/boundary job.

## 4. Existing local evidence and its limits

Local verification recorded at implementation commit `5a128c4` passed formatting, strict Clippy, workspace check, 259 library tests, release build, cargo-deny, architecture/boundary, Phase 2 policy, workflow/release/security ICM checks, documentation references, System Map and ICM workspace validation. R2 route-boundary tests and R4 catalogue/schema tests also pass at that candidate.

Those results establish useful static/unit coverage; they do **not** negate the later exact-head PostgreSQL integration failures above. The current Security workflow passes, including the CI advisories/cargo-audit job. No local PostgreSQL service was used for the local-only check set; the failed database-backed suite ran in GitHub Actions.

## 5. Closure rule

PR #49 stays unmerged and Phase 4 Part 8 remains open until all material contract requirements have implementation and final-head evidence, the required Rust and Integration workflows pass on the same source head, the consumer/release contracts are closed, R3/R4 evidence is complete, independent Stage 06 and human review are recorded, and branch-protection policy is satisfied. Do not infer closure from a green Clippy or policy workflow alone.
