# Remediation Plan — PR #49 / Phase 4 Part 8

**Date:** 2026-10-22  
**Input audit:** [Follow-up audit](audit.md)  
**Related handoff:** [Stage 09 delivery record](../09-delivery-pr009-part8.md)  
**Current disposition:** Keep PR #49 open and unmerged until the security, atomicity, delivery, and human-review gates below are satisfied.

## 1. Goal

The repository owner approved the choices in [the decision index](decisions/README.md) on 2026-10-22. Decision selection is closed at repository-owner level; the work below implements and verifies those choices. If implementation reveals a concrete incompatibility, record an amendment before deviating.

Close the remaining Part 8 contract gaps without weakening RLS, least-privilege database identity, strict configuration parsing, audit requirements, or fail-closed startup. The current CI result is evidence for exercised behavior, not authority to claim production readiness.

Do not expand the scope by choosing a broker, transport, event name, consumer contract, or authorization rule without the appropriate owner decision.

## 2. Ordered work

### R1 — Implement the owner-approved relay and consumer contract

**Priority:** P1 / blocking  
**Dependency:** repository-owner approval is recorded; implementation depends on R2/R3 authority and transaction boundaries, plus the actual AWS account and resource setup.

Review [the relay and consumer contract proposal](decisions/relay-delivery-contract-proposal.md). If accepted, record it as the approved, versioned contract (or revise the proposal) covering:

- transport and logical destination(s), environment mapping, and ownership;
- event envelope, event-name registry, schema/version compatibility and size limits;
- authentication, authorization, tenant context, and secret/configuration requirements;
- what “accepted” means for an acknowledgement and when the worker may mark an event published;
- ordering guarantees, partitioning or aggregate sequencing needs, and expected duplicate behavior;
- retryable versus terminal failures, retry ceiling, quarantine/dead-letter handling, replay/repair, retention, and operator access;
- metrics, alerts, trace/log fields that are safe to emit, and operational ownership;
- consumer idempotency key and transaction semantics for recording processed event IDs and applying side effects.

**Acceptance:** the owner-approved contract exists in the authoritative project documentation, is reviewed, and defines enough behavior to write adapter and integration tests. No adapter is implemented against a guessed endpoint or broker.

### R2 — Integrate authenticated, server-derived API authority

**Priority:** P1 / security blocker  
**Dependency:** the approved identity/session context and authorization contracts.

1. Thread a verified authenticated principal/security context from the HTTP boundary into application commands.
2. Derive organization owner and audit actor identity on the server; treat request tenant/resource identifiers as selectors that require authorization, not proof of access.
3. Check current membership and operation permission on every protected request; default to deny when identity, membership, or tenant context is absent or stale.
4. Keep durable mutation routes disabled until the authoritative context exists. Remove the `unauthenticated_actor()` path from production command composition rather than merely changing the placeholder string.
5. Add real HTTP tests for missing/invalid credentials, authorized same-tenant operations, cross-tenant attempts, owner-ID spoofing, suspended/revoked membership, and audit actor correctness.

**Acceptance:** no client field can choose the durable owner or audit actor; authorization is performed at the actual route/application boundary; denied requests have no business, audit, or outbox mutation.

### R3 — Make business state, required audit, and outbox intent one transaction for every in-scope mutation

**Priority:** P1 / integrity and audit blocker  
**Dependency:** R2's trusted actor/tenant context and R4's event-scope decisions can proceed in parallel, but the final matrix must be complete before closure.

1. Create a transition coverage matrix for organization, branch, membership, invitation, role, and scope mutations. For each transition list the canonical command, event name, target, actor source, tenant scope, payload/schema version, transaction boundary, and negative test.
2. Refactor repository/application boundaries so the mutation, required audit row, and paired outbox row use the same authoritative PostgreSQL transaction. Avoid a sequence where a store mutation commits first and `record_evidence` opens a second transaction afterward.
3. For each in-scope operation, inject an audit-write failure and an outbox-write failure; assert the business mutation and the other evidence row roll back together.
4. Preserve idempotency semantics: a duplicate semantic command must not create a second event unless the contract explicitly defines it as a new operation. Keep divergent same-ID requests as conflicts.
5. Run the existing provisioning success, audit-failure rollback, outbox-failure rollback, retry/divergence, and concurrent same-payload tests after the refactor.

**Acceptance:** every operation classified as mandatory has evidence in the same transaction as its state change, backed by PostgreSQL tests that exercise the real repository path. If a transition is not mandatory, the contract must say so and the omission must be visible in the coverage matrix.

### R4 — Wire the owner-approved lifecycle event mappings

**Priority:** P1 for required events; do not add arbitrary vocabulary.  
**Dependency:** audit/event contract owner.

Implement the five approved mappings in [the decision record](decisions/iam-lifecycle-event-mapping-proposal.md): member pending acceptance, membership expiry, invitation issuance, invitation withdrawal, and per-invitation expiry during the sweep. Update the binding contract, source registry and schema validation before wiring the transitions. Do not reuse an existing event name if it changes the meaning of the audit event.

**Acceptance:** the matrix has no unexplained missing transition; contract and registry agree; the payload is bounded and secret-free; tests prove the selected behavior and atomicity.

### R5 — Implement the owner-approved production identity/provider baseline

**Priority:** P1 / production blocker  
**Dependencies:** R1 for worker destination settings; R2/R3 for safe API composition.

1. Implement the owner-approved [identity, runtime and Secrets Manager baseline](decisions/identity-production-runtime-proposal.md) through the existing provider boundary. Keep `SITOLO_SECRET__...` reserved for the development provider and never permit production fallback to it.
2. Construct actual PostgreSQL pools during production composition; the API must not treat database runtime intent or its current in-memory store as a production connection.
3. Authenticate the runtime pool as `app_runtime` and the worker pool as `app_worker`, each with separate restricted credentials. Reserve the administrative identity for provisioning and migrations.
4. Keep readiness false until the required secret, database, schema/role identity, and other mandatory runtime dependencies have been validated.
5. Test missing provider, missing/invalid secret, wrong DSN identity, unavailable database, missing schema/policy, and absent worker adapter. Each must fail closed without silently starting a reduced or privileged service.

**Acceptance:** production startup composes real dependencies and only reports ready when they are usable; tests prove least-privilege identity and fail-closed failure paths.

### R6 — Implement and prove real relay delivery

**Priority:** P1 / release blocker  
**Dependency:** approved R1 contract and R5 worker composition.

1. Implement the contract-specific publisher adapter and explicit runtime configuration.
2. Preserve stable event ID as the destination idempotency key; never report success before the contract's acknowledgement point.
3. Bind the adapter to the existing bounded claim/lease/retry/quarantine engine; preserve claim-token fencing and error classification.
4. Add integration tests for successful publish, duplicate/replayed publish, destination timeout, terminal rejection, worker crash after destination acceptance but before marking published, stale-lease recovery, poison/unknown event, and consumer deduplication.
5. Verify ordering only to the level the approved contract promises. Do not claim exactly-once delivery unless the destination and consumer protocol actually establish it.

**Acceptance:** an end-to-end test uses the concrete adapter and a contract-faithful destination/test double or sandbox, proves duplicate-safe consumer effects, and records the observed acknowledgement/retry semantics. Unit tests with a fake publisher remain useful but are not sufficient by themselves.

### R7 — Re-audit, verify, then request human review

**Priority:** merge gate  
**Dependencies:** R1–R6.

1. Update the audit and remediation evidence against the final code SHA; do not reuse old CI evidence after source changes.
2. Run formatting, compile, lint, config/bootstrap tests, all PostgreSQL suites, RLS tests, event coverage/atomicity tests, worker integration tests, architecture/policy checks, security scans, and the release-artifact workflow.
3. Confirm the documentation-reference checker has zero missing/ambiguous/escaping references and the System Map source pointers are current if mapped components changed.
4. Review the exact diff, exact check run URLs, and remaining-risk statement. Request human review only once the report is internally consistent.
5. Keep the PR unmerged until required review is recorded and all release-blocking findings have an accepted disposition.

## 3. Verification commands

Run the database-free checks first:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --all-features --locked
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo test -p sitolo-config -p sitolo-security --all-targets --all-features --locked
    cargo test -p sitolo-api-bin --test startup_env --all-features --locked
    ./scripts/ci/check-release-workflow
    ./scripts/ci/check-icm-workspace
    python scripts/ci/check-doc-references

**Local PostgreSQL is deferred to the next phase. Do not install, start, or run a local PostgreSQL instance for this phase.** Database-backed suites must run in the repository's PostgreSQL-enabled GitHub Actions workflow with its scoped test DSNs. A local `--no-run` compile is not database evidence. For this phase, obtain PostgreSQL proof from an Actions run whose `headSha` exactly matches the source under review, then record its run URL below:

    https://github.com/jelome265/sitolo/actions/workflows/integration.yml

Do not run `cargo test --workspace` locally when it would attempt database-backed integration tests. Run the database-free checks listed above and rely on exact-head CI for database execution.

The CI release-artifact workflow must also be exercised on the candidate source SHA when its exact-source test step or artifact packaging changes.

## 4. Definition of done

The PR is not merge-ready until all are true:

- [ ] Relay contract and consumer acknowledgement/idempotency semantics approved.
- [ ] Authenticated principal, actor, owner, tenant scope, and request authorization are derived/checked server-side.
- [ ] Every mandatory IAM state mutation and its required audit/outbox rows share one transaction.
- [ ] Event-catalogue gaps are resolved by an authoritative contract decision.
- [ ] API and worker production composition use real dependencies and restricted identities, with fail-closed startup.
- [ ] Concrete relay delivery and duplicate-safe consumer effects are integration-tested.
- [ ] Current-source CI and release-artifact verification pass.
- [ ] Follow-up audit reflects exact source evidence and human review is recorded.

## 5. Non-goals / constraints

This plan does not select a broker or cloud provider, invent a new event vocabulary, claim a completed production secret-store integration, promise exactly-once delivery, weaken strict configuration parsing, restore the administrator database identity to runtime, or authorize merging PR #49.
