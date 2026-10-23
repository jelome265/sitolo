# PR #49 Phase 4 Part 8 Remediation Plan

**Date:** 2026-10-23  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Base source head:** `f73d6a3e987df309c9e2480ea68f791934e449ed`  
**PR:** https://github.com/jelome265/sitolo/pull/49  
**Security relevant:** yes (SC-001, SC-002, SC-003, SC-004, SC-005, SC-009, SC-010, SC-011)

## Objective and authority

Implement the remaining requirements in the user's PR #49 remediation brief, using the fetched current branch head as the base. The authoritative project contract is `docs/phase4_part8_audit_outbox_implementation_contract.md`, with the approved follow-up decision records under `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/decisions/`. The new request specifies no local PostgreSQL and requires database evidence from PostgreSQL-enabled GitHub Actions against the exact candidate SHA. PR #49 must remain open and unmerged.

The request names the SNS FIFO to SQS FIFO relay contract and the five lifecycle event names/payload constraints. Do not invent cloud resource identifiers or enable delivery without explicit runtime configuration and provisioned workload permissions. Existing `.agents/` user-provided skill files are untracked and must not be included in source commits.

## Current evidence and baseline

- `git fetch origin feat/phase4-pr009-audit-outbox` completed. `HEAD` and fetched branch both resolve to `f73d6a3e987df309c9e2480ea68f791934e449ed`; no newer remote commit was found.
- PR #49 is OPEN and `REVIEW_REQUIRED`. Every reported check on that exact head completed successfully, including PostgreSQL integration, policy, Rust verify, dependency policy, advisories, secrets, offensive regression, and CodeQL (GitHub query on 2026-10-23).
- `cargo metadata --format-version 1 --locked` exited 0; `cargo fmt --all -- --check` exited 0.
- The worktree initially contained only untracked `.agents/` skill files.
- No local PostgreSQL is to be installed, started, or used. New database integration evidence must come from exact-head GitHub Actions.

## Invariants and planned closure

| Requirement | Invariant | Implementation/evidence required |
|---|---|---|
| R2 live HTTP authority | Protected command receives identity, session, active membership, tenant, permission and resource scope resolved by trusted server adapters; denied requests have no writes | Axum route tests against the actual router and production adapter seams for absent/invalid credentials, mapped identity, session ownership, inactive membership, same-tenant allow, cross-tenant denial and spoofed identifiers |
| R3 authoritative tenancy | Production reads and writes use the same durable PostgreSQL authority; each required mutation and mandatory audit/outbox writes commit or roll back together | Transition matrix for organization, branch, membership, invitation, role and scope; failure-injection and concurrency/idempotency PostgreSQL tests for each wired transition |
| R4 lifecycle mappings | The five approved v1 events describe real state transitions, validate at construction/persistence/relay, and exclude secrets/contact data | Domain-to-event tests and PostgreSQL state/audit/outbox assertions for `iam.membership.pending_acceptance`, `iam.membership.expired`, `iam.invitation.issued`, `iam.invitation.withdrawn`, `iam.invitation.expired` |
| AUD-01 relay | SNS acceptance is the producer acknowledgement; FIFO uses stable event ID and aggregate group; consumer records `(consumer_id,event_id)` with effects atomically before SQS delete | Concrete publisher, explicit configuration/workload role, durable inbox, failure/retry/restart/order tests, and approved AWS sandbox or contract-faithful integration evidence |
| AUD-05 runtime | Production uses managed secrets, restricted API/worker database identities, authoritative migrations and readiness probes of mandatory dependencies; no test identity provider in served runtime | Production composition and bootstrap tests for missing provider/secret/database/role/schema/identity/relay, plus exact-source integration checks |
| Verification/review | All claims tie to the final source SHA; independent Stage 06 and human review remain outstanding until actually performed | Exact-head Actions links, updated current-status/remediation evidence, new Stage 06 audit, and recorded human approval; no merge |

## Bounded ownership and failure behavior

- `sitolo-api` owns HTTP parsing and request context extraction. `sitolo-application` owns authorization and tenancy command orchestration. `sitolo-persistence` owns PostgreSQL records and transaction implementation. `sitolo-events` owns event names, envelopes and schemas. `apps/worker` owns bounded relay orchestration and concrete transport composition. The AWS adapter must not become domain authority.
- Authentication, current session, membership, organization/branch state, permission, and resource scope fail closed when absent or stale.
- Business mutation, mandatory audit record, and outbox intent share a PostgreSQL transaction. External SNS/SQS I/O occurs only after claim/commit, never inside a business transaction.
- Delivery remains at least once. SNS FIFO deduplication does not replace consumer inbox idempotency. Consumer effects and inbox insert are atomic; SQS acknowledgement follows commit.
- The approved workload roles remain separate: API has no SNS publish or queue consumption; worker may publish only to its configured topic; consumer is restricted to its assigned queue; migrations/admin are never runtime fallbacks.

## Security mapping

| Control | Threat/requirement | Required proof | Residual dependency |
|---|---|---|---|
| SC-001 | Forged identity or revoked session | Bad/missing token and session-owner/revocation HTTP tests | Configured OIDC provider and production identity credentials |
| SC-002 | Cross-tenant access or caller-selected tenant | Same-tenant allow and cross-tenant deny with no mutation | PostgreSQL CI for RLS and authority queries |
| SC-003 | Permission bypass | Permission-denied command tests with no side effect | Independent Stage 06 audit |
| SC-004 | Malformed or oversized HTTP/event input | Axum parser/body-limit and event schema tests | Fuzz and security CI |
| SC-005 | Secret leakage or production provider fallback | Redaction/config/bootstrap tests; role-scoped secret access | AWS account, IAM, KMS, Secrets Manager |
| SC-009 | Missing or non-atomic audit/outbox evidence | PostgreSQL failure injection for every mandatory mutation | Exact-source PostgreSQL CI |
| SC-010 | Unbounded request/worker resource use | Bounded route, claim batch, retry/quarantine and timeout tests | Operational limits and metrics |
| SC-011 | Dependency and artifact risk | locked policy, advisories, secret scan, CodeQL, release checks | Exact candidate CI and artifact attestation |

## Sequencing

1. Reconcile source against the current head and existing exact-head CI evidence; finish the local database-free baseline.
2. Implement the smallest safe vertical slice: production authentication and request-scoped authorization composition while preserving fail-closed mutation behavior until full authority is connected.
3. Unify tenancy authority and transition transaction semantics, adding the complete mandatory transition matrix and failure tests.
4. Verify the five lifecycle event mappings across domain transition, durable event validation, and relay acceptance.
5. Implement the approved SNS FIFO publisher, consumer inbox, explicit configuration and runtime readiness; leave cloud delivery unclaimed until a sandbox/account is available.
6. Run database-free checks locally; push the exact candidate and require PostgreSQL-enabled Actions, policy/security/release checks.
7. Refresh remediation and current-status evidence. Record Stage 06 as requiring independent audit and human approval; never self-certify it.

## Rollout and rollback

Keep protected writes disabled until verified authority, durable transaction, evidence and production dependencies are fully composed. Keep the worker non-ready until explicit topic configuration, role identity, DB capability and relay adapter validate. Roll back by reverting the candidate source commit(s); retain additive schema/data history and use a forward migration if any committed data makes direct rollback unsafe.

## Open evidence dependencies

- AWS account, FIFO resources/subscriptions, IAM policies, sandbox permissions and consumer service are not established by source configuration alone. The adapter may be implemented without guessing resource IDs, but live delivery and consumer end-to-end proof remain BLOCKED until a contract-faithful environment is available.
- No local PostgreSQL is allowed. Database-backed checks must run in GitHub Actions and match the exact candidate SHA.
- Independent Stage 06 audit and required human review cannot be authored as if performed by this implementation run.
