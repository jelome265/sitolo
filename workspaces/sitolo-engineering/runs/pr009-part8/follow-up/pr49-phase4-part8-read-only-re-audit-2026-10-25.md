---
artifact_type: follow-up-findings
status: not-ready
run_slug: "pr009-part8"
security_relevant: yes
candidate_head: "68f5ae5be7cdb8659e0cbd4f90fae16a7ee24a85"
implementation_commit: "5a128c4f95db61f7f6cf9fd885d22911b78553c1"
date: "2026-10-25"
---

# PR #49 — Exact-head follow-up findings

**Decision:** NOT READY. Do not merge PR #49 or advance Phase 4 Part 8 closure.

**PR:** [#49](https://github.com/jelome265/sitolo/pull/49)  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Exact candidate head:** `68f5ae5be7cdb8659e0cbd4f90fae16a7ee24a85`  
**Implementation commit under test:** `5a128c4f95db61f7f6cf9fd885d22911b78553c1`

> This is a supplemental, source-linked follow-up to the current run record. It records observed exact-head CI failures and source-level gaps; it is **not** an independent Stage 06 audit, approval, or substitute for the required human review. The independent Stage 06 output shelf remains untouched.

## 1. Exact-head CI evidence

The following GitHub Actions runs are associated with candidate head `68f5ae5be7cdb8659e0cbd4f90fae16a7ee24a85`:

| Workflow | Result | Evidence and scope |
|---|---|---|
| Policy | **PASS** | [Run 40156547558](https://github.com/jelome265/sitolo/actions/runs/40156547558) |
| Rust | **FAIL** | [Run 40156547515](https://github.com/jelome265/sitolo/actions/runs/40156547515); architecture, crate-boundary, format, strict Clippy and compile jobs pass; workspace PostgreSQL-backed tests fail |
| Integration | **FAIL** | [Run 40156547478](https://github.com/jelome265/sitolo/actions/runs/40156547478); same six application transaction integration failures |
| Security | **PASS** | [Run 40156547500](https://github.com/jelome265/sitolo/actions/runs/40156547500); CodeQL, offensive-regression, advisories/cargo-audit, dependency policy and secret checks pass |

The PostgreSQL-backed application integration suite reports **38 passed and 6 failed**. The six failures are:

1. `authenticated_role_assignment_authorizes_and_records_server_actor` — the audit-event lookup returns `RowNotFound`.
2. `authenticated_membership_suspend_authorizes_and_records_server_actor` — the observed error does not match the expected `AuthorizedTenancyError::Authorization(ScopeError::PermissionDenied)` variant.
3. `authenticated_membership_revoke_authorizes_and_records_server_actor` — the observed error does not match the same expected permission-denied variant.
4. `concurrent_role_assignment_retries_emit_one_semantic_event` — expected assignment/audit/outbox counts `(1, 1, 1)`; actual counts are `(1, 2, 2)`.
5. `role_assignment_commits_with_evidence_and_is_semantically_idempotent` — expected audit/outbox counts `(1, 1)`; actual counts are `(2, 2)`.
6. `role_revocation_preserves_history_and_regrant_is_duplicate_safe` — expected count tuple `(2, 1, 2, 2, 1)`; actual tuple is `(2, 1, 3, 3, 1)`.

These are release blockers. The failing tests must be repaired at the semantic boundary; do not loosen their expected evidence cardinality merely to obtain a green build.

## 2. Source findings and required correction

### F-01 — Role assignment replay can emit duplicate evidence

**Observed source path:**
- `crates/sitolo-application/src/tenancy.rs`: `assign_role_with_evidence` obtains the result of `tx.insert_role_assignment(...)` and then records `IamEventName::RoleAssigned` evidence.
- `crates/sitolo-persistence/src/tenancy_unit_of_work.rs`: the assignment persistence path can return an existing assignment when its identifier and parameters match.

The application records `RoleAssigned` after the operation returns without an explicit typed result distinguishing a new state transition from an identical replay/no-op. This is consistent with the duplicate audit/outbox counts in failures 4 and 5. The source path is a strong causal lead, but the fix must be confirmed by regression tests.

**Required action:** make the transaction/port report a semantic outcome such as `Created`, `IdenticalReplay`, or a defined conflict. Write mandatory transition evidence only when the authoritative state transition occurs. Handle an already-effective membership/role pair and the database uniqueness constraint deterministically, without swallowing unrelated database errors. Preserve one-event semantics under concurrency and retry.

### F-02 — Role revocation/regrant path may record evidence for a no-op

**Observed source path:** `revoke_role_with_evidence` records `RoleRemoved` after the persistence operation returns. The failing history/regrant test observes one extra audit/outbox event.

**Required action:** distinguish an actual revocation transition from an already-revoked replay. Assert both durable state history and exact audit/outbox cardinality for first execution, retry, concurrent retry, revoke, and regrant.

### F-03 — Audit target identity does not currently line up with the failing test lookup

The assignment evidence test queries `audit_events` using `target_ref = 'assignment-target'`, while the application evidence call supplies a `TargetRef::Membership(membership_id)` target. This is an observed disagreement between the query expectation and the writer's target type; the correct target must be established from the approved audit contract.

**Required action:** decide and document whether role-assignment evidence targets the role-assignment entity or its membership. Align the domain contract, writer, persisted row and test. Do not simply weaken or rewrite the failing assertion without deciding the intended audit subject.

### F-04 — Membership suspend/revoke error mapping is not proven correct

Both authenticated negative tests fail because the returned error variant differs from the expected `ScopeError::PermissionDenied`. The CI output does not identify the actual variant, so this report does not infer it.

**Required action:** trace the authenticated request through actor/scope resolution, authorization policy, application error and HTTP/error mapping. Return the contract-defined denial consistently. Extend tests to prove that denied requests leave business state, audit rows and outbox rows unchanged.

### F-05 — PostgreSQL uniqueness conflict requires deterministic domain handling

The database trace includes a duplicate-key violation on `uq_role_assignment_effective`. That trace alone does not establish which of the six failures it caused. Nevertheless, the effective-role uniqueness invariant must be reconciled with the application’s retry and conflict handling.

**Required action:** add a transactionally correct lookup/locking or conflict-mapping path consistent with the approved concurrency contract. Test racing first assignments and retries using the CI PostgreSQL service.

## 3. Findings that are improved but not closed

### Architecture and lint enforcement

- The Application-to-Persistence production dependency inversion is addressed: application-owned ports live in `crates/sitolo-application/src/identity_ports.rs` and `tenancy_ports.rs`; PostgreSQL implementation lives behind `sitolo-persistence`.
- `scripts/ci/check-crate-boundaries.py` classifies all 17 workspace packages from Cargo metadata, checks coverage for each package and rejects forbidden normal dependency edges. The independent architecture job passes on the exact candidate head.
- Strict workspace Clippy passes on the exact candidate head, including `--all-targets --all-features --locked -- -D warnings`. Clippy is **not** the current blocker.
- Confirm branch protection requires the architecture/boundary check; a passing job is not by itself proof that repository merge rules require it.

### R2 — Protected route denial

The route-boundary tests pass and prove that each of the six declared protected routes rejects missing credentials. Five routes have authenticated application paths. `POST /v1/organizations/{organization_id}/activate` remains explicitly closed: `provision_organization` activates the organization before insert, so no persisted `PROVISIONING` organization exists for this transition.

This proves denial behaviour for the declared route set; it does not complete the full database-backed positive/negative mutation matrix.

**Decision still required:** implement a real `PROVISIONING` state and persistence transition under an approved contract, or remove the activation route from the declared API inventory/contract. Do not reopen the handler without the corresponding persisted lifecycle.

### R3 — Mutation matrix

R3 remains **PARTIAL**. The refreshed matrix is a source-inspection index, not independent proof that every required mutation has correct principal/actor, tenant scope, authorization policy, transactional mutation, event schema and negative-test evidence. The six exact-head failures mean the role/suspend/revoke transaction path is not currently verified.

### R4 / AUD-04 — Lifecycle event mapping

The lifecycle event contract tests pass for the five declared event names and version-1 schema shape validation, including rejection of wrong versions and unknown fields. This does not prove every actual domain transition emits the correct persisted event through the outbox and relay. AUD-04 remains open until the transition-to-evidence mapping is source-traced and independently audited.

## 4. Consumer and release-path status

The consumer-related source exists:
- `apps/worker/src/consumer.rs` defines the consumer processor, transport abstraction, injected business-effect interface and transaction/acknowledgement sequence.
- `apps/worker/src/sqs.rs` implements SQS FIFO transport operations.
- `crates/sitolo-persistence/src/consumer_inbox.rs` and `schema/consumer_inbox.sql` define durable inbox persistence and its database boundary.

However, `apps/worker/src/main.rs` composes the outbox relay and SNS FIFO publisher; it does **not** compose/run the SQS consumer processor or an approved concrete business effect. Thus consumer infrastructure exists, but production consumer wiring and the end-to-end business effect are not proven complete.

- **AUD-01 remains OPEN:** approve the SQS queue/consumer contract and the exact business effect; wire the consumer into worker composition; prove inbox deduplication, business effect and commit are one transaction; acknowledge only after commit; exercise retry and quarantine/dead-letter behaviour.
- **AUD-05 remains OPEN/PARTIAL:** complete and document the restricted worker role/pool and migration/release path across the required identity, tenancy, audit, outbox and inbox schema. Existing `app_worker` pool/source is not proof of a complete independent production migration/release contract.

## 5. ICM status and review boundary

- The exact-head evidence above supersedes any earlier “CI pending” or “all checks green” statement for this candidate.
- The 2026-10-24 remediation/verification reports remain historical evidence for the source and local checks they name; do not read them as a pass for exact-head database tests.
- No Stage 06 independent audit approval is claimed here. Do not backfill `stages/06-audit/output/` with this follow-up: the independent audit artifact must be produced through the ICM audit stage and checked against the final implementation head.
- Required independent Stage 06 re-audit and human review remain outstanding.
- The current PR description also contains stale head/CI and architecture-finding statements; refresh it only after the implementation is corrected and exact-head evidence is current.

## 6. Ordered closure plan

1. Fix assignment/revocation semantic outcomes and evidence idempotency; resolve the audit target contract; correct permission-denial mapping.
2. Add/retain PostgreSQL integration proofs for atomic rollback on evidence failure, no evidence/state mutation on denied commands, assignment concurrency, retry idempotency and revocation/regrant history.
3. Run the full exact-head Rust and Integration workflows until all PostgreSQL-backed tests pass. Do not substitute library-only green tests for these checks.
4. Finish the approved SQS queue/effect contract and production consumer composition, with inbox/effect transactionality and retry/quarantine proof.
5. Resolve organization activation by contract and implementation, not by changing the route's status alone.
6. Complete the source-grounded R3 matrix and R4 transition-to-persisted-event/relay trace; close AUD-04 only with evidence.
7. Complete AUD-05's restricted worker role and migration/release contract.
8. Refresh current status, continuation guidance and the PR description to the final candidate SHA; ensure required branch protection; request the independent Stage 06 audit and human review.

**Exit condition:** all material contract requirements have implementation and exact-head evidence, all required workflows pass on the same head, no unresolved critical/high transaction or authorization finding remains, Stage 06 and human review are recorded, and the approved merge policy is satisfied. Until then, PR #49 stays unmerged and Phase 4 Part 8 does not close.
