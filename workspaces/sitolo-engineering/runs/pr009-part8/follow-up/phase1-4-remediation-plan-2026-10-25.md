---
artifact_type: cross-phase-remediation-plan
status: active-not-complete
run_slug: "pr009-part8"
security_relevant: yes
candidate_head: "a378beec2bc5097bb642825d2e8fcf4c967c105d"
date: "2026-10-25"
---

# Phase 1–4 Remediation Plan — PR #49

## 0. Plan status and guardrails

This is a plan, not an implementation report. It does not claim that the work below is performed. The audited source snapshot is a378beec2bc5097bb642825d2e8fcf4c967c105d; all existing workflow results refer to that snapshot.

**Do not merge or close Phase 4 Part 8 until all P0/P1 gates and the independent review gate are complete.** Do not remove assertions, exclude required targets, weaken Clippy/unsafe policy, invent an unapproved business effect, use mocks for database/RLS proof, or claim success from compilation where integration execution is required.

## 1. Prioritization and dependency order

| Priority | Workstream | Why first | Exit evidence |
|---|---|---|---|
| P0 | Guard destructive PostgreSQL test targets | Prevent misconfigured tests from touching an unintended database/cluster | Invalid targets rejected before network/DDL; disposable PostgreSQL test still passes |
| P0 | Complete real consumer/effect path | A producer publish is not consumer completion; side effects can duplicate or disappear | Approved effect, real queue wiring, inbox/effect atomic commit, ack-after-commit, retry/DLQ/order/idempotency evidence |
| P0 | Canonical migration/release plan | Source/schema tests cannot guarantee production schema parity | Versioned migration artifact/source, ownership, restricted roles, reproducible apply, readiness check and recovery runbook |
| P0 | Full mandatory IAM mutation-to-evidence matrix | Authorization and audit/outbox consistency are security properties | Every required mutation has positive, denial, rollback, replay and concurrency evidence |
| P1 | Resolve organization lifecycle/activation | Declared endpoint is intentionally unavailable | Persisted state machine and tests, or contract/API route explicitly deferred/removed |
| P1 | Phase 3 end-to-end identity evidence | Library/provider presence is not lifecycle correctness | Provider/authority and PostgreSQL lifecycle tests, race and invalidation evidence |
| P1 | Make testkit operational or retire it | Phase 1 promises shared test infrastructure but crate is empty | Actual test-only helper consumers, or approved contract/index adjustment and crate removal |
| P1 | Resolve CI scope ambiguity and verify merge controls | Prevent test coverage regression | Canonical required job policy plus branch-protection snapshot |
| P1 | Refresh status/PR/ICM evidence | Prevent old SHA reports from misleading future work | Exact source SHA, run URLs, updated PR body, historical reports retained |
| P2 | Complete fingerprint field coverage | Avoid silent configuration drift identity | Tests cover every serialized field and secret exclusion |
| P2 | Pin security tool versions and inventory lint exceptions | Improve reproducibility and detect lint bypass | Pinned tools, automated override/flag inventory and CI tests |
| P2 | Repair stale test infrastructure System Map entry | Remove ghost/stale architecture claims | Status/revision/edges reflect real testkit consumers |

## 2. Detailed workstreams and acceptance criteria

### R-01 — Fail-closed test database target validation

**Owner role:** persistence/test-infrastructure maintainer. Scope is test harness and tests only.

1. Create one test-only configuration parser for administrative/runtime/worker DSNs. It must execute before any connection or DDL.
2. Parse host, port, database and role. Do not rely only on substring checks.
3. Require DSNs to resolve to the expected disposable target family and intended database. Server/database must agree; role identities must be exactly the documented test roles.
4. Reject production/staging-like targets and remote hosts by default. If a remote disposable environment is later required, create a separate allowlisted contract with target identity, expiry, owner, credentials scope and cleanup policy.
5. Keep UUID-prefixed isolated schemas. Cleanup must target only the schema created by the same test invocation, with validated identifiers.
6. Add negative tests for missing DSNs, mismatched names/hosts, unexpected roles, production-like database names, unsafe hosts and missing opt-in. Prove failure occurs before connection/DDL.
7. Report only sanitized target identity, never password-bearing DSNs.
8. Keep CI test setup on the documented disposable PostgreSQL service. Do not silently skip tests or add a generic safety bypass.

**Pass condition:** a deliberate misconfiguration cannot create/drop schemas or provision roles before rejection; the real PostgreSQL test path remains runnable.

### R-02 — Complete consumer delivery and business effect

**Owner roles:** worker/eventing maintainer, accountable domain owner, infrastructure/security reviewer. The domain owner must approve the exact effect.

1. Read the owner-approved relay-delivery proposal and Part 8 binding contract. The selected design is SNS FIFO to per-consumer SQS FIFO/DLQ; infrastructure-specific details remain to be reviewed.
2. Approve the exact event names consumed, business effect, consumer identity, transaction owner, queue/DLQ identities and retry/visibility settings.
3. Register and validate required config in the canonical config schema, environment reference and fingerprint tests. Missing production config must fail closed.
4. Compose production worker components explicitly: restricted worker DB pool, SNS relay publisher, SQS consumer transport, durable inbox writer and approved effect adapter. Do not instantiate a no-op default.
5. Commit inbox marker and business effect in the same transaction. Delete/acknowledge the SQS message only after commit. Retryable failure must permit safe redelivery; permanent invalid/unsupported messages use approved quarantine/DLQ handling.
6. Prove first delivery, exact replay, concurrent duplicate, crash before transaction, crash after commit/before ack, lost ack response, transient error, poison payload, max receive count and DLQ redrive.
7. Prove aggregate ordering: a later sequence cannot overtake an earlier pending, claimed, retrying or quarantined event. Quarantine blocks ordered successors until an audited repair/redrive decision.
8. Apply least privilege: relay publishes only to approved topic; consumer receives/changes visibility/deletes only on its own queue and operates only its DLQ; neither gets broad topic/queue administration.
9. Review encryption/resource policy, actual account/region/resource ARNs, IAM and cost controls. Do not treat proposed identifiers as production truth.
10. Run contract-faithful integration evidence for producer acceptance, SQS delivery, inbox transaction/effect and ack. Unit fakes may test local mapping but are not end-to-end proof.
11. Add bounded metrics/alerts for pending age, attempts, publish failures, consumer failures, inbox duplicates, DLQ depth/age and sequence blockage. Prevent payload/secrets from entering logs.
12. Document retry, quarantine, redrive, manual replay and sequence-repair operations, preserving event identity and audit history.

**Pass condition:** one real approved effect is applied at most once per durable inbox key under at-least-once delivery, with ack-after-commit and tested retry/order semantics.

### R-03 — Production schema and release authority

**Owner roles:** database/platform owner and security approver.

1. Inventory every table, function, policy, role and grant needed by API identity, tenancy, audit, outbox and consumer inbox.
2. Select a canonical ordered versioned migration source. Either add the runner/bundle or name the external governed artifact, immutable version/hash, access mechanism and accountable owner.
3. Avoid independently maintained test SQL and production SQL. Apply the canonical bundle to isolated test schemas where feasible. If fixtures differ by design, add schema-parity verification.
4. Separate migration/admin, API runtime, relay worker and consumer identities. Verify grants, ownership, RLS policies, security-definer function owner and EXECUTE grants from PostgreSQL catalogs.
5. Ensure runtime identities do not own protected tables or bypass RLS. Respect PostgreSQL owner/BYPASSRLS semantics and keep worker inbox authority distinct from Part 8 evidence tables.
6. Define migration ordering, lock/timeout behavior, preflight compatibility, backup, rollback or forward-fix, and release result recording.
7. Fail production readiness on missing schema version/functions/policies/grants. Runtime processes must not execute DDL.
8. Rehearse a clean install, supported upgrade and interrupted-migration recovery on disposable PostgreSQL.

**Pass condition:** a clean environment can be built from the canonical migration source and deployed schema/roles can be compared to tested evidence.

### R-04 — Complete mandatory-mutation evidence matrix (R3/AUD-03)

**Owner role:** Phase 4 application/persistence maintainer; independent reviewer validates separately.

For every required command, record:
- API route/consumer entry point and DTO.
- Verified principal/session and actor provenance source.
- Organization/membership resolution and branch-scope rule.
- Required permission, denial response and no-write expectation.
- State transition, preconditions and database constraints.
- Repository/transaction port and adapter.
- Audit event name/version/target/actor/result/redaction.
- Outbox event name/version/aggregate ID/sequence/delivery policy.
- Whether events are mandatory or diagnostic.
- Positive test, denied/no-write test, rollback-on-audit-failure test, rollback-on-outbox-failure test, duplicate/replay test, concurrency test, schema rejection test.
- Test target and exact CI run URL/source SHA.

Derive commands from the full Phase 4 and Part 8 contract inventory: organization/branch creation and lifecycle, memberships, invitations/acceptance/pending/expiry/suspend/revoke, role assignment/revocation/regrant, ownership and scope grants, and any other explicitly required mutation. Do not guess a subset.

Regression requirements:
1. Preserve Created versus Replayed assignment semantics so no-op/replay emits no second event pair.
2. Preserve exact audit/outbox counts.
3. Trace denial mapping from route through scope resolver and application error; denied commands leave state unchanged and create no disallowed business event.
4. Ensure audit target matches contract and durable row.
5. Validate payload schema before writes and reject unknown keys.
6. Business state, mandatory audit and outbox share one authoritative PostgreSQL transaction.
7. Failure injection proves audit/outbox failures roll back business state.
8. Concurrent command retries yield one defined semantic transition/event result.

**Pass condition:** every mandatory mutation has complete source and real-PostgreSQL evidence. Source inspection alone is not enough.

### R-05 — Organization lifecycle and route scope

1. Compare the binding Phase 4 lifecycle to actual persisted states and API inventory.
2. Approve one path: implement durable PROVISIONING → ACTIVE, or defer/remove the activation route from the current API contract.
3. If implementing, enforce guards and authority, emit mandatory audit/outbox in the same transaction, and define repeated activation semantics.
4. Test authorized/unauthorized, cross-tenant, invalid-state, replay and concurrent activation.
5. If deferred, retain fail-closed behavior, update route inventory/docs, and explicitly list the unavailable feature. Never replace the 503 with unconditional success.

**Pass condition:** no route is falsely claimed as implemented; any enabled transition is durable, authorized, atomic and tested.

### R-06 — Phase 3 identity/session lifecycle proof

**Owner role:** identity/security maintainer.

1. Map every Phase 3 requirement to source, storage, entry point, test and current evidence: external identity linking, OIDC issuer/audience/signature, session creation/verification/invalidation, logout/revocation, refresh rotation/replay, MFA enrollment/challenge/recovery, password reset, device registration/revocation.
2. Exercise production-shaped composition with the actual OIDC/provider adapter and persistent PostgreSQL authority in a controlled environment with synthetic credentials.
3. Test invalid issuer/audience/signature, expiry, unknown/rotated key, provider timeout, missing authority, revoked/suspended user, cross-tenant confusion and fail-closed readiness.
4. Race refresh attempts and one-use artifacts against PostgreSQL; assert one legal transition and deterministic loser semantics.
5. Prove secrets do not enter logs, traces, metrics, audit payloads or error output.
6. Classify every requirement as implemented+evidence, partial, contract-deferred or not implemented.

**Pass condition:** all normative requirements have a real source path and appropriate executable proof; Phase 3 closure is a separate evidence-backed decision.

### R-07 — Activate or retire sitolo-testkit

1. Inventory genuinely shared fixture/helper code duplicated across tests.
2. Add a small module API with explicit fixture state, deterministic clock/IDs and no hidden global state or production secrets.
3. Move only genuinely reusable helpers into crates/sitolo-testkit. Use from tests via dev-dependencies only.
4. Keep RLS/DB/transaction tests on PostgreSQL; the kit may create isolated schemas and roles but may not simulate database semantics in memory.
5. Add a boundary rule prohibiting production dependency edges to testkit, with a regression check.
6. Update System Map metadata only after actual consumers exist.
7. If no shared value is found, record and approve removal/deferment and amend contract/register instead of leaving a ghost crate.

**Pass condition:** multiple tests actually consume reusable helpers, or the architecture is honestly amended to remove the dead abstraction.

### R-08 — CI test scope, toolchain and merge protection

1. Reconcile SCOPE-EXC-001 with current PostgreSQL suites and scripts/ci/verify. Explicitly state which jobs/targets execute.
2. Require Part 8 PostgreSQL/RLS/transaction tests on PR #49; do not let missing/skipped targets count as success.
3. Keep SHA-pinned Actions and least-privilege workflow permissions.
4. Pin cargo-deny and cargo-audit tool versions and validate toolchain consistency across build/scanner workflows.
5. Add a source/config policy check for allow, expect, cfg_attr, lint override and unsafe production code; reject --cap-lints allow.
6. Confirm all production crates inherit workspace lints and every exception is narrow/reasoned.
7. Verify actual branch protection requires Rust, Integration, Security, Policy and human review. The connector could not read branch protection (403); repository admin must verify it.
8. Ensure the reported workflow head SHA matches the source under review.

**Pass condition:** required controls cannot be silently omitted and every exception is current, narrow and auditable.

### R-09 — Phase 2 fingerprint coverage

1. Create the authoritative field list from each fingerprint writer.
2. Add table-driven tests for every field, including optional values, OIDC issuer/token endpoint/redirect, worker password reference, schema, pool settings, relay settings and secret-provider region.
3. Prove each effective non-secret change changes the digest.
4. Prove raw secret values cannot enter canonical input/output while changing a SecretRef does change the digest.
5. Prove None and Some canonicalize distinctly.
6. Keep length-prefix canonicalization, versioning and lowercase SHA-256; do not use Debug formatting or hash raw secret values.

**Pass condition:** every serialized field has a test and fingerprint compatibility changes are explicit.

### R-10 — Status and evidence refresh

1. Preserve dated reports and clearly label old SHA evidence historical.
2. Push a source candidate and obtain workflows naming that exact source SHA.
3. Update docs/phase4_part8_pr009_current_status.md, continuation guide, docs/current_implementation_status.md, ICM current status and PR description from one evidence set.
4. Keep this cross-phase report/plan/guide in follow-up index.
5. Run ICM stages in order with independent Stage 06 review against final source. Do not copy the supplemental audit into the independent audit shelf.
6. Record human review, branch-protection state and any authorized residual-risk exception.
7. Only then consider PR merge and Phase 4 exit.

## 3. Final closure gate

- [ ] No P0/P1 finding unresolved or waived without authorized written exception.
- [ ] Test DB guard blocks unintended target before DDL.
- [ ] Consumer/effect is wired, idempotent, acknowledgement-safe and observable.
- [ ] Canonical migration/release path is repeatable and roles/RLS match the release artifact.
- [ ] Mutation matrix has full source/test evidence.
- [ ] Organization activation is implemented correctly or explicitly deferred/removed.
- [ ] Phase 3 evidence is complete; no blanket completion inference.
- [ ] Testkit is used safely or approved for deprecation/removal.
- [ ] CI scope, tool versions and lint-bypass checks are reconciled.
- [ ] All required workflows pass on the same final candidate SHA.
- [ ] PR description/status pointers match that candidate.
- [ ] Independent Stage 06 audit and human review are recorded.
- [ ] Branch protection is verified by a repository administrator.

Until these items are complete, disposition remains **NOT READY for merge or Phase 4 closure**.
