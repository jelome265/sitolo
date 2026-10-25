---
artifact_type: cross-phase-audit-report
status: not-ready
run_slug: "pr009-part8"
security_relevant: yes
candidate_head: "a378beec2bc5097bb642825d2e8fcf4c967c105d"
date: "2026-10-25"
---

# Cross-Phase Deep Audit — Phases 1–4 / PR #49

## 0. Decision

**Disposition: NOT READY for Phase 4 Part 8 closure or production-readiness claims. Do not merge PR #49 or begin the next product phase based on green CI alone.**

This is a supplemental, report-only audit pinned to source candidate a378beec2bc5097bb642825d2e8fcf4c967c105d. It does not modify application code, approve the pull request, or replace the independent ICM Stage 06 audit and human review. These report files document evidence and planned remediation; they do not claim proposed fixes have been implemented.

The direction is sound: explicit crate boundaries, typed configuration, deny-by-default authorization, real PostgreSQL/RLS tests, restricted runtime identities, strict Rust/Clippy policy, exact-head CI, and transactional audit/outbox work are appropriate foundations. The remaining problem is that the full Phase 1–4 contract set has not been proven end-to-end, and production release paths are explicitly unfinished.

## 1. Pinned evidence and review limits

- Repository: jelome265/sitolo
- PR: [#49](https://github.com/jelome265/sitolo/pull/49)
- Branch: feat/phase4-pr009-audit-outbox
- Source candidate reviewed: a378beec2bc5097bb642825d2e8fcf4c967c105d
- Base: main at d2dbcce6606db49b1b87669f364721905c6509f8
- Review date: 2026-10-25
- PR state observed during review: open, unmerged.
- Scope: Phase 1–4 contracts and named remediations; current PR/ICM evidence; workspace manifests, lint policy, architecture boundary contract/checker, CI workflows, API bootstrap/readiness, worker composition and consumer ports, PostgreSQL/RLS integration tests, configuration fingerprint, testkit, schema/migration documentation, and test-safety scripts.

### Exact-head workflows observed

All four workflows completed successfully against the same source candidate, a378beec2bc5097bb642825d2e8fcf4c967c105d.

| Workflow | Result | Evidence |
|---|---|---|
| Rust | PASS | [Run 40163505238](https://github.com/jelome265/sitolo/actions/runs/40163505238) |
| Integration | PASS | [Run 40163505245](https://github.com/jelome265/sitolo/actions/runs/40163505245) |
| Security | PASS | [Run 40163505240](https://github.com/jelome265/sitolo/actions/runs/40163505240) |
| Policy | PASS | [Run 40163505254](https://github.com/jelome265/sitolo/actions/runs/40163505254) |

This supersedes older statements that Rust was still failing on a four-versus-three RLS policy count. The current source fixed that assertion by scoping the inventory query to the Part 8 evidence tables, audit_events and outbox_events. The worker consumer_inbox_tenant_policy remains a separate worker/inbox policy. This is a legitimate scope correction when checked against the contract; it is not evidence that every consumer requirement is complete.

**Important:** these results apply only to the source snapshot named above. Committing these reports creates a newer branch head. Do not copy these statuses forward to that newer head without checking its own workflow runs.

### Boundaries of confidence

This is a source-and-evidence audit, not a fresh local execution of the toolchain. The connected GitHub evidence allowed source and workflow inspection but did not allow reading branch protection settings (API returned 403). This report therefore cannot confirm that repository merge rules require every check, prevent bypass, or mandate independent review. A full unrestricted local worktree scan and exhaustive review of every line of every general-purpose architecture/security document were not performed. The principal Phase 1–4 contracts and current implementation paths were inspected; findings are marked Verified, Risk, or Not demonstrated to avoid converting missing evidence into proof of a defect.

## 2. Governing document inventory

Phase contracts are requirements, not implementation proof. Supporting specifications and historical PR reports provide context but cannot silently override current binding contracts.

### Phase 1 — repository, workspace, CI, testing and supply chain

- [Phase 1 deep implementation contract](../../../../../docs/phase1_repository_rust_workspace_ci_deep_implementation.md)
- [Phase contract coverage register](../../../../../docs/phase_contract_coverage_register.md)
- [Testing strategy](../../../../../docs/testing_strategy.md)
- [Security test harness](../../../../../docs/security_test_harness.md)
- [CI enforcement](../../../../../docs/ci_enforcement.md)
- [Workspace architecture](../../../../../docs/workspace_domain_architecture.md)
- [Crate boundary policy](../../../../../architecture/crate-boundaries.toml)
- [Crate boundary checker](../../../../../scripts/ci/check-crate-boundaries.py)
- [Canonical verification script](../../../../../scripts/ci/verify)
- [ICM workspace checker](../../../../../scripts/ci/check-icm-workspace)
- Test infrastructure System Map object: map/objects/verification/test-infrastructure.md

### Phase 2 — configuration, secrets, errors, telemetry and persistence boundaries

- [Phase 2 configuration/secrets/logging/errors/telemetry contract](../../../../../docs/phase2_config_secrets_logging_errors_telemetry_implementation.md)
- [AppConfig fingerprint remediation](../../../../../docs/phase2_appconfig_fingerprint_coverage_and_configuration_contract_remediation.md)
- [Fingerprint lower-hex CI remediation](../../../../../docs/phase2_ci_sha256_fingerprint_lowerhex_failure_remediation.md)
- [PostgreSQL runtime/configuration/persistence boundary contract](../../../../../docs/phase2_postgresql_runtime_configuration_persistence_boundary_implementation.md)
- [Environment configuration reference](../../../../../docs/environment-configuration-reference.md)
- Sources reviewed include crates/sitolo-config/src/fingerprint.rs and the API bootstrap/readiness composition.

### Phase 3 — identity, sessions, MFA, recovery and devices

- [Phase 3 identity implementation contract](../../../../../docs/phase3_identity_sessions_mfa_device_identity_implementation.md)
- Identity authority: crates/sitolo-persistence/src/identity_authority_postgres.rs
- In-memory reference implementation: crates/sitolo-persistence/src/memory.rs
- API composition: apps/api/src/bootstrap.rs, apps/api/src/state.rs, apps/api/src/serve.rs
- OIDC/secrets adapters: crates/sitolo-integrations/src/oidc.rs and crates/sitolo-integrations/src/secrets.rs

### Phase 4 — tenancy, organization/branch IAM, RLS and Part 8 audit/outbox

- [Phase 4 tenancy/organization/branch/IAM contract](../../../../../docs/phase4_tenant_organization_branch_iam_implementation.md)
- [Historical Part 6 contract](../../../../../docs/phase4_part6_jules_implementation_contract.md)
- [Historical Part 7 contract](../../../../../docs/phase4_part7_jules_implementation_contract.md)
- [Historical Part 7 / PR-008 evidence](../../../../../docs/phase4_part7_pr008_remediation_execution_contract.md)
- [Binding Part 8 audit/outbox contract](../../../../../docs/phase4_part8_audit_outbox_implementation_contract.md)
- [PR-009 current status pointer — stale relative to this audit head](../../../../../docs/phase4_part8_pr009_current_status.md)
- [PR-009 remediation execution contract — stale relative to this audit head](../../../../../docs/phase4_part8_pr009_remediation_execution_contract.md)
- [Superseded implementation instructions](../../../../../docs/phase4_part8_pr009_remediation_implementation_instructions.md)
- [Earlier remediation report](../../../../../docs/phase4_part8_pr009_remediation_report.md)
- ICM run records: stages 01-select through 09-deliver, dated follow-ups, mandatory mutation matrix, and owner-approved relay-delivery proposal.
- Sources reviewed include application-owned ports, PostgreSQL unit of work, schema/RLS policy, API route closure, worker relay and consumer source, and Part 8 PostgreSQL tests.

Historical Phase 4 Part 6/7/PR-008 records are useful for provenance; they are not current-head closure evidence. The Phase 4 canonical contract and current source govern requirements and current truth.

## 3. Executive result by phase

| Phase | Contract status | Current implementation/evidence assessment | Phase closure |
|---|---|---|---|
| 1. Workspace, CI and supply chain | Detailed contract exists | Strong implementation of workspace, locked builds, enforced boundaries, strict lint and CI. Testkit is an empty scaffold; a test-scope exception and database-test safety risk remain. | Not demonstrated |
| 2. Config, secrets, logging and telemetry | Detailed contract and remediations exist | Typed config, secret references, production secret-provider path, runtime composition and versioned secret-free fingerprint exist. Field-coverage tests do not visibly cover every serialized field. | Not demonstrated |
| 3. Identity and sessions | Detailed contract exists | OIDC/provider and persistent authority composition exist; in-memory identity is explicitly described as test/local. Full production login/session/refresh/revoke/MFA/device lifecycle proof is not demonstrated against the entire contract. | Not demonstrated |
| 4. Tenancy/IAM/RLS/audit/outbox | Detailed contract and Part 8 binding contract exist | Real PostgreSQL/RLS and atomicity tests exist; selected IAM evidence defects were remediated and current workflows pass. Activation is intentionally closed; consumer/effect wiring, migration/release path and comprehensive mutation evidence remain incomplete. | Active; not ready to close |

**Answer to “Was everything documented from Phase 1 to Phase 4 implemented?” — No.** Meaningful parts are implemented, but contract coverage, source presence and passing CI are different claims. There is no honest basis to mark every Phase 1–4 requirement complete.

## 4. Findings

### F-01 — Production consumer is not fully composed end-to-end

**Severity: P0 / release-blocking. Status: Verified gap.**

Evidence:
- apps/worker/src/main.rs composes PostgresOutboxRepository, the SNS FIFO publisher and RelayWorker.
- apps/worker/src/consumer.rs provides a strict processor and ConsumerBusinessEffect port. The file states the concrete effect is owner-supplied and queue configuration needs approval.
- apps/worker/src/sqs.rs contains an SQS adapter, but inspected worker main does not compose/run the consumer processor or an approved concrete business effect.
- The owner-approved relay proposal selects SNS FIFO to per-consumer SQS FIFO queues, dedicated DLQs, at-least-once delivery, durable inbox idempotency and acknowledgement only after transaction commit. It also states that actual AWS resources, IAM, costs, deployment and end-to-end tests still need delivery/release work.

Impact: a real publisher and reusable consumer abstraction do not prove that an event reaches a queue and that an approved downstream business effect executes safely under retry, crash or redelivery. A fake publisher cannot establish this.

Required evidence to close:
1. Registered and validated production configuration for the approved topic, queue, consumer identity and DLQ.
2. Provisioned infrastructure/resource policies and least-privilege identities reviewed against the owner-approved contract.
3. A concrete, owner-approved business effect; do not invent or silently substitute a no-op.
4. Inbox marker and business effect commit in one database transaction.
5. SQS message acknowledgement/delete only after successful commit.
6. Duplicate delivery, retry, lease/visibility timeout, quarantine/DLQ and redrive evidence.
7. PostgreSQL-backed concurrency/order tests plus authorized end-to-end tests against disposable AWS resources or an explicitly approved contract-faithful environment.
8. Runbook and observability for queue age/depth, retries and quarantine.

### F-02 — Production migration/release contract is incomplete

**Severity: P0 / release-blocking. Status: Verified documentation gap.**

Evidence: crates/sitolo-persistence/schema/README.md says there is no automatic production migration runner and no versioned canonical Phase 5 migration bundle in this repository. It says the production migration owner and mechanism are not defined here, and the runtime must not target a partially provisioned schema.

Impact: correct SQL files and tests do not make a reproducible deployment. Production can drift if schema order, ownership, RLS policies or function definitions differ from the tested fixtures.

Required evidence to close:
- A versioned, checksummed, ordered schema/migration bundle or a governed external migration artifact with immutable version/hash.
- Named accountable owner and approval path for schema release.
- Separate migration/admin, API runtime, outbox relay and consumer identities with verified effective privileges.
- Rehearsed upgrade and rollback/forward-recovery procedure.
- Deployment gate that checks applied migration version and required functions/policies and fails readiness on missing schema.
- CI execution from the same canonical migration source used for release, not a hand-maintained duplicate fixture.

### F-03 — Organization activation lifecycle does not match the route surface

**Severity: P1 / release-blocking for this route. Status: Verified intentional closure.**

Evidence: apps/api/src/serve.rs states provision_organization activates the organization before insert, so there is no persisted PROVISIONING organization to transition. The activation endpoint is intentionally closed and returns 503 through the closed-route path.

Impact: failing closed is safer than pretending a transition occurred. However, the route inventory or documentation must not describe activation as implemented.

Required decision and evidence:
- Either implement persisted PROVISIONING → ACTIVE using an approved lifecycle contract, status invariants and transactional audit/outbox event, with positive/negative/concurrent PostgreSQL tests; or remove/defer the activation endpoint from the declared surface and document it as unavailable.
- Do not merely replace the 503 handler with a success response.
- Verify suspension/closing/closed semantics, permissions, actor provenance, replay behavior and evidence rollback.

### F-04 — Mandatory mutation-to-audit/outbox proof remains incomplete across the full matrix

**Severity: P0/P1 / release-blocking. Status: Not demonstrated comprehensively.**

Evidence:
- The binding Part 8 contract requires business transition, mandatory audit record and outbox record to commit atomically, with rollback if a mandatory component fails.
- Earlier exact-head remediation reports documented and fixed real defects: role assignment/replay could produce duplicate evidence; revoke/regrant could write evidence for no-op transitions; audit-target expectation disagreed with the writer; a denial returned the wrong error variant; and an invitation payload carried a schema-rejected extra key.
- Current four-workflow success is good evidence that the current required suite is green, but it does not prove every mandatory Phase 4 mutation is source-traced in a complete independently reviewed matrix. The existing ICM matrix describes itself as a source-inspection index, not independent proof.

Required evidence to close each command:
- Exact handler/entry point and application use case.
- Server-verified principal, actor, organization, membership and branch scope; no caller-controlled authority.
- Authorization rule and denial result.
- State transition/preconditions and DB invariant.
- Repository/transaction port and adapter.
- Audit event name/version/target/actor/result and redaction requirements.
- Outbox event name/version/aggregate ID/sequence/delivery policy.
- Positive integration test, denied/no-write test, rollback-on-audit-failure test, rollback-on-outbox-failure test, replay test, concurrency test and payload-schema rejection test.
- Test file and exact CI run URL/source SHA.

Derive the full command list from the Phase 4 and Part 8 contracts, not a guessed subset. Preserve exact event cardinality and replay assertions. Do not weaken a test to obtain green CI.

### F-05 — Database-test cleanup is isolated in CI but insufficiently guarded against a misconfigured manual target

**Severity: P1 / test safety. Status: Risk observed.**

Evidence:
- crates/sitolo-persistence/tests/part8_postgres_integration_tests.rs reads ADMIN_DATABASE_URL, RUNTIME_DATABASE_URL and WORKER_DATABASE_URL, creates a UUID-named test_part8 schema, and teardown executes DROP SCHEMA IF EXISTS "<schema>" CASCADE.
- CI uses a disposable PostgreSQL 18 service, which is the correct test design.
- The inspected test setup requires the DSNs but does not visibly reject a non-test database name or remote production host before administrative DDL. A wrong local environment variable could point a destructive test helper at an unintended cluster. A generated schema reduces blast radius but does not remove this risk.

Required remediation:
1. Fail before network/DDL unless an explicit test-only target contract is satisfied.
2. Verify all DSNs resolve to the same known disposable server/database and expected test identities.
3. Require explicit opt-in for destructive integration suites outside CI, and still reject production/staging target patterns.
4. Add negative tests showing production-like DB names, non-loopback/manual remote hosts and conflicting DSNs are refused before connection/schema operation.
5. Keep random schema names, restricted runtime/worker connections and scoped cleanup.
6. Never rely only on a URL substring when production hosts can match it; validate parsed host/database and an explicit disposable-target contract.

The reviewed helper drops a generated schema, not the whole database. The issue is missing fail-closed target validation, not that the current CI run necessarily targets production.

### F-06 — Phase 1 testkit crate is an unused empty scaffold

**Severity: P1 / architecture and maintainability debt. Status: Verified.**

Evidence:
- crates/sitolo-testkit/Cargo.toml has an empty dependencies section.
- crates/sitolo-testkit/src/lib.rs contains only documentation and forbid(unsafe_code).
- Repository search found no consumer using sitolo-testkit as a dependency. map/objects/verification/test-infrastructure.md still marks it as stub/ghost and pins an old source revision.
- Phase 1 §5.14 says the crate owns shared reusable fixtures/test infrastructure and production crates must not depend on it.

Impact: the crate currently provides no shared value, and its existence can mislead reviews into believing that a test infrastructure boundary is implemented.

Recommended resolution:
- Use it now for genuinely shared test-only fixture builders, deterministic clock/ID sources, safe configuration-pair builders, and principal/tenant/membership contexts.
- Use only as dev-dependency from tests; never add a production normal dependency.
- Real PostgreSQL/RLS/transaction evidence must still use PostgreSQL. Testkit may provision isolated schemas and roles; it must not emulate RLS in a map/mutex.
- If no genuine reusable helper emerges after a measured inventory, approve updating the Phase 1 contract/System Map and remove the placeholder. Until then, do not describe testkit as implemented.

### F-07 — Clippy is strong and enforced, but lint policy is not a complete security/correctness proof

**Severity: P1 / control verification. Status: Strong control observed; residual assurance needed.**

Evidence:
- Workspace lints deny clippy::all, unwrap/expect/panic/unreachable/unimplemented/todo, console/debug macros, indexing/slicing, casts, arithmetic side effects, selected async/concurrency and memory hazards, and lint-suppression attributes without reasons.
- CI invokes cargo clippy --workspace --all-targets --all-features --locked -- -D warnings; the Rust workflow passed on the audited candidate.
- clippy.toml disallows selected risky framework APIs with reasons. Some exceptions are intentional and explained, such as the Runtime::block_on lint collision with Tokio test/runtime macros.
- Workspace policy denies unsafe Rust by default; current production entrypoints visibly forbid unsafe code. PR notes narrowly scoped, reasoned test-only unsafe expectations for Rust 2024 environment mutation.
- The connector-backed review did not perform an exhaustive local scan of every file, generated code path and compiler flag; this report does not claim proof that no suppression/bypass exists anywhere.

Disposition: **Clippy does not appear weakened in the inspected workspace policy or primary verification command.** It is materially stricter than a basic Clippy pass. A green lint check still cannot prove authorization, correct state transitions or transaction ownership.

Required hardening:
- Add automated inventory of every allow, expect, cfg_attr, crate-level lint override and compiler flag; require a reviewed path-specific exception registry.
- Reject command-line lint downgrades such as --cap-lints allow in verification/release scripts.
- Verify every production crate inherits workspace lints and no per-target config weakens them.
- Keep narrow, reasoned exceptions reviewed; do not blanket-allow a lint to make CI pass.
- Attach the exact Clippy run URL/source SHA to ICM evidence.

### F-08 — Integration test scope exception is ambiguous against current Phase 4 proof

**Severity: P1 / evidence-policy ambiguity. Status: Verified configuration fact; decision required.**

Evidence: .github/workflows/integration.yml excludes sitolo-persistence from its workspace test command and records SCOPE-EXC-001 as a Phase 1 PostgreSQL integration harness deferral to Phase 5. The separate canonical Rust workflow invokes scripts/ci/verify, which includes PostgreSQL service use and the sitolo-persistence RLS test. Current Rust and Integration workflows both passed.

Impact: a green workflow named Integration may be misread as proof for every persistence test, while the exception may be interpreted as waiving DB testing broadly. This does not mean all DB tests are skipped; it is a conflicting scope contract likely to regress.

Required remediation:
- Make each workflow/job explicit about the suites it executes.
- Decide which PostgreSQL/RLS/transaction suites are mandatory on every PR. For this PR, Part 8 PostgreSQL tests must remain mandatory because they cover audit/outbox, privileges, RLS and transactions.
- Retire or narrowly rewrite SCOPE-EXC-001 if a formerly deferred suite is now required; never silently extend the waiver.
- Add policy validation that fails if mandatory DB jobs are missing, skipped or filtered out.
- Verify repository branch protection requires those jobs. This audit connection could not read branch protection.

### F-09 — Phase 2 fingerprint regression coverage is incomplete

**Severity: P2 / configuration correctness. Status: Verified test-coverage gap.**

Evidence:
- crates/sitolo-config/src/fingerprint.rs uses length-prefixed canonical key/value records, format versioning and SHA-256 lowercase hex. It includes database, HTTP, OIDC, secret references (not values), pools, telemetry, worker reference and relay configuration.
- The every_effective_non_secret_field_changes_fingerprint test checks many fields, but did not visibly mutate every field serialized by the writers. Specifically oidc_issuer, oidc_token_endpoint, oidc_redirect_uri and worker_db_password_ref were serialized but not covered by that test body.

Required remediation:
- Create one authoritative field inventory or table-driven test covering each fingerprint writer.
- Prove a changed effective non-secret value changes the fingerprint.
- Prove secret values never enter canonical input or output, while changing a secret reference changes the fingerprint.
- Prove optional None and concrete values canonicalize distinctly.
- Treat fingerprint schema changes as a compatibility decision.

### F-10 — Status and ICM evidence drift can mislead the next implementation pass

**Severity: P1 / governance and verification integrity. Status: Verified.**

Evidence:
- docs/phase4_part8_pr009_current_status.md and its guide refer to older candidate a5dffa4 and a Rust failure that was resolved on a378beec.
- The 2026-10-25 supplemental re-audit is pinned to 68f5ae5; the Stage 06 ICM record names a previous candidate. Neither is current-head proof.
- The PR description names a stale head and says Rust/Integration are still in progress/pending, while all four workflows have completed successfully on a378beec.
- docs/current_implementation_status.md and the System Map test-infrastructure object also point to old revisions.
- Independent Stage 06 and human review remain separate exit gates. A supplemental report must not be relabelled as the independent audit.

Required remediation:
1. After implementation changes, update one canonical current-status pointer with actual source SHA and exact run URLs.
2. Retain prior records and label them historical/superseded instead of rewriting history.
3. Update the PR body after new-head workflows complete.
4. Run ICM stages in order; independent Stage 06 must review final source.
5. Verify branch-protection and required-review settings in repository administration.

### F-11 — Reference/in-memory adapters must not be mistaken for production proof

**Severity: P1 / claim accuracy. Status: Differentiated.**

Evidence:
- crates/sitolo-persistence/src/memory.rs explicitly labels itself an in-memory identity reference for tests/local development and says production implementations replace it with PostgreSQL.
- The API bootstrap inspected on this snapshot composes PostgreSQL runtime pool, persistent identity authority, PostgreSQL tenancy unit of work and external OIDC provider for non-development runtime. Development deliberately does not compose the production DB path.
- Real PostgreSQL tests use distinct admin/runtime/worker identities.

Disposition: inspected production composition is not the in-memory store. Its mere presence does not prove that production is fake. Nevertheless, in-memory tests cannot prove database locking, RLS, transaction rollback or PostgreSQL concurrency semantics. Consider test-only packaging/visibility if the in-memory implementation has no intentional local-development use. Add a boundary check preventing production composition roots from referencing it.

## 5. Mock/fake policy

The repository testing strategy explicitly permits mock/fake external providers for narrow unit/adapter tests. Other contracts reject mock-only proof for PostgreSQL, RLS, locking, tenant isolation and transactions. These rules can coexist if their scope is explicit.

Recommended policy:
- Production runtime: no mocks, fakes, no-op effects or in-memory authority substitutes.
- DB-bound security/correctness claims: no fake DB/RLS/policy evaluator; use real PostgreSQL and actual role privileges/policies.
- End-to-end SNS/SQS delivery claim: fake publisher/transport does not count as end-to-end evidence.
- Narrow unit tests: external-provider fakes may test local mapping/error handling but must be labelled as unit/adapter evidence and cannot satisfy production integration gates.
- Contract-faithful integration: use real services in disposable environments, or an approved emulation only for behaviors it faithfully implements and with deviations recorded.

If “no mocks or fakes” means no test double anywhere, that contradicts the current testing-strategy wording and requires an explicit policy change. The recommended rule is: never use a fake as production functionality or as proof of authoritative DB/security/external-delivery behavior.

## 6. Command and unsafe-operation review

### Positive controls observed

- Canonical verifier uses set -euo pipefail, runs locked format/lint/check/test/build stages, and fails if cargo-deny or cargo-audit is missing.
- Inspected GitHub Actions use full SHA pins and narrow explicit permissions.
- Offensive regression runner requires SITOLO_OFFENSIVE_ALLOWED=true, SITOLO_OFFENSIVE_ENV=disposable and loopback admin/runtime DSNs before proceeding; its workflow provisions ephemeral PostgreSQL.
- ICM cold-walk creates a dedicated mktemp directory, copies into it, and removes that quoted temporary path on exit.
- Workspace policy denies unsafe Rust by default; current workflow passed.

### Risks and fixes

- PostgreSQL integration helper executes DROP SCHEMA ... CASCADE on a generated test schema using an admin DSN. The helper did not visibly enforce a test database name/host before DDL. Add the F-05 guard.
- CI installs ripgrep with sudo apt-get update/install on an ephemeral runner. This is standard provisioning, not a harmful command in this context.
- CI uses cargo install cargo-deny --locked and cargo install cargo-audit --locked. Dependency resolution is locked, but the package versions are not explicitly pinned; pin tool versions or use a reviewed toolchain/binary source.
- POSTGRES_HOST_AUTH_METHOD=trust appears only in isolated GitHub-hosted PostgreSQL services. Do not use it in production or a shared DB.
- No curl | sh, git reset --hard, force-push command or --cap-lints allow was observed in the inspected key scripts/workflows. This does not claim an unrestricted binary scan of every repository file or dependency was performed.

## 7. External engineering references checked

These references support general engineering principles; Sitolo's binding contracts and owner-approved decisions govern product-specific behavior.

- [AWS — Transactional outbox pattern](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html): commit business state and durable event intent together, design for duplicate delivery.
- [PostgreSQL — Row security policies](https://www.postgresql.org/docs/current/ddl-rowsecurity.html): owner/BYPASSRLS semantics matter; RLS and grants are distinct controls.
- [OWASP — Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html): deny by default and validate authorization on every request.
- [Clippy lint configuration](https://doc.rust-lang.org/clippy/lint_configuration.html): lint policy and exceptions must be consciously configured/reviewed.
- [Cargo workspace lints](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-lints-table): centralized policy still requires member crates to opt into inheritance.
- [GitHub Actions security hardening](https://docs.github.com/en/actions/how-tos/secure-your-work): pin actions, minimize permissions and preserve trusted workflow boundaries.

## 8. Final answers

**Is it enterprise-ready?** Not as an end-to-end production system yet. Engineering controls are promising and current required CI passed, but production readiness is blocked by consumer/effect wiring, migration/release authority, lifecycle mismatch, incomplete full mutation tracing and independent review.

**Is the implementation real?** Substantial parts are real: PostgreSQL/RLS tests run against a PostgreSQL service; the API source composes PostgreSQL and an external OIDC provider outside development; the relay uses the AWS SDK-backed SNS publisher. But not all required side effects are connected end-to-end. The consumer processor and SQS adapter existing in source does not prove production delivery/effect.

**Is Clippy weakened or bypassed?** The inspected workspace policy is strong and CI uses -D warnings; no weakening was found in the reviewed primary path. This is narrower than an exhaustive scan. Add suppression/compiler-flag inventory and verify branch protection.

**Are harmful commands being used?** No obviously reckless command was found in the inspected key scripts. There is an actionable destructive-test safety risk: validate DB test targets before DDL. Temporary-directory cleanup and ephemeral runner installation are scoped and expected.

**Why is testkit unused?** It is currently a stub. Implement reusable test-only fixtures or formally amend the contract and remove the dead abstraction.

**Are we on the right path?** Yes on architecture and security direction; no on claiming closure. Keep PR #49 open, complete blockers, rerun exact-head evidence and independent review, then proceed.
