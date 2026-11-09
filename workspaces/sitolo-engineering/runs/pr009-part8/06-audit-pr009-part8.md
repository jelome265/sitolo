# Stage 06 Audit — PR-009 Part 8

## Current exact-head Stage 06 re-audit — 2026-11-09

**Current authoritative audit section.** This re-audit supersedes the old disposition below for the implementation source identified here. The earlier October audit is retained as historical provenance and must not be read as the current-head conclusion.

### Engagement and audit identity

- Repository / PR: jelome265/sitolo, PR #49, base main.
- Audited implementation HEAD: 5e963123dcb84d9d1d50993d7611ddbc1bae0466.
- Audited branch: feat/phase4-pr009-audit-outbox. The branch resolved to the same SHA when checked.
- Audit date: 2026-11-09.
- Auditor: GPT-6, OpenAI AI assistant, performing a read-only source and evidence review separate from the implementation commits. This is an AI audit record, not a human auditor’s attestation, certification, GitHub review, or approval.
- Scope: PR-009 Part 8 source and evidence relevant to transactional tenancy, audit/outbox persistence, API authority, worker relay/consumer, current SQS configuration delta, migration/release controls, and carried Phase 3 release dependencies.
- Binding basis: docs/phase4_part8_audit_outbox_implementation_contract.md; Stage 01 brief; Stage 04 plan; Stage 05 implementation record; mandatory mutation transaction matrix; SC-002, SC-006, SC-009, SC-010 and SC-011; migration release runbook.
- Method: read-only inspection of the pinned source files, their test targets, the requirement and mutation matrices, and GitHub workflow results. No source files or runtime systems were modified by this audit.
- Execution limitation: no local build, test, PostgreSQL server, AWS account, SNS topic, SQS queue, or production deployment was executed by the auditor. Passing test claims below are limited to the exact-head GitHub workflow evidence and the named test implementations.
- Review boundary: the PR contains 248 changed paths. This audit traces the security-critical Part 8/API/worker/config paths and the directly relevant test suites; it is not a claim that every line in all 248 changed paths received a full line-by-line audit.

### Exact-head evidence

PR #49 was open and unmerged at the audited HEAD. All four required workflows completed successfully on that exact SHA:

| Required workflow | Exact-head run | Result |
|---|---:|---|
| Rust | 42666982592 | PASS |
| PostgreSQL Integration | 42666982770 | PASS |
| Security, including CodeQL and offensive regression | 42666983110 | PASS |
| Policy | 42666983348 | PASS |

The ECC Tools changed-configuration audit also reported no changed-config issues on 5e963123. Its separate harness audit reported one medium warning: the ECC manifest is missing for .agents/skills/rust-enterprise-audit/SKILL.md. That warning is not a changed-config failure and is dispositioned separately in Stage 07.

### Requirement → implementation → test/evidence → disposition

| Requirement / control | Implementation path examined | Test / evidence examined | Disposition |
|---|---|---|---|
| Canonical Rust, PostgreSQL, security and policy gates | .github/workflows/rust.yml, integration.yml, security.yml and policy.yml; canonical scripts/ci/verify and migration checks | Four required workflow runs above, all completed successfully on 5e963123 | PASS for the configured CI surface on this SHA. This is not production deployment evidence. |
| Atomic organization provisioning with mandatory audit and outbox (SC-006, SC-009) | apps/api/src/bootstrap.rs; apps/api/src/serve.rs; crates/sitolo-application/src/tenancy.rs; crates/sitolo-persistence/src/tenancy_unit_of_work.rs and tenancy_postgres.rs | part8_provision_organization_evidence_tests; membership_transaction_integration; PostgreSQL Integration workflow. Source shows business rows, audit and outbox use one caller-owned transaction and commit once. | PASS for the exercised provisioning transaction, including success, rollback, retry and concurrent same-payload cases. Do not generalize this result to every IAM command. |
| Tenant identity and protected HTTP authority (SC-002) | apps/api/src/bootstrap.rs composes PgRuntimePool, PgRequestIdentityAuthority and PgTenancyUnitOfWork; apps/api/src/serve.rs authenticates before handlers; organization owner identity is taken from SecurityContext; branch and member commands use authenticated service wrappers. | apps/api/tests/tenancy_api.rs: live_organization_route_derives_owner_and_audit_actor_from_authenticated_subject; live_branch_route_authorizes_and_mutates_only_the_verified_tenant; live_branch_activation_authenticates_and_commits_authorized_transition; live_membership_revoke_authorizes_tenant_and_records_verified_actor (also exercises suspend); axum_boundary.rs enumerates protected routes and missing-credential denial. | PASS for the current exposed route inventory and exercised PostgreSQL request paths. The route inventory is intentionally narrower than the full mutation matrix. |
| PostgreSQL RLS, role separation and SECURITY DEFINER boundaries | crates/sitolo-persistence/schema/migrations/0001–0006; postgres.rs; runtime_pool.rs; tenancy_unit_of_work.rs; worker pool and evidence repositories | part8_postgres_integration_tests: test_security_definer_functions_are_hardened; test_part8_relation_ownership_and_rls_metadata; test_runtime_cannot_read_or_truncate_evidence; test_worker_cannot_select_outbox_table; test_runtime_cannot_execute_worker_transitions; test_outbox_cross_tenant_insert_is_denied; plus PostgreSQL Integration and Rust workflow results. | PASS for the catalog, privilege, RLS and role behaviors exercised by those PostgreSQL tests. Production-cluster role provisioning remains a deployment gate. |
| Complete mandatory Phase 4 mutation matrix (SC-002, SC-006, SC-009) | crates/sitolo-application/src/tenancy.rs; persistence unit of work; workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-mandatory-mutation-transaction-matrix-2026-10-23.md | membership_transaction_integration includes lifecycle success/rollback, branch creation rollback, invitation issue/accept/withdraw/expiry races, member transitions, role and scope grant/revocation, concurrency and evidence-failure cases. Live HTTP coverage is a smaller subset; many commands remain actor-taking application operations without a live authenticated route. | PARTIAL. PostgreSQL transaction evidence is substantial, but the matrix is not closed for every required command’s public authority, request boundary, denied/no-write and concurrency contract. The owner must explicitly close or exclude each non-live command; do not expose generic routes by inference. |
| Outbox claim fencing, aggregate order, retry and quarantine | apps/worker/src/relay.rs; crates/sitolo-persistence/src/outbox_repository.rs; apps/worker/src/sns.rs | part8_postgres_integration_tests: concurrent worker claim, lease recovery, concurrent aggregate sequence assignment, same-aggregate order, retry reschedule/fencing, terminal quarantine and attempt-budget exhaustion; RelayWorker tests cover event/version rejection and publisher failure classes. | PASS for the tested repository/relay policy and state transitions. Live AWS delivery and production IAM are not established by unit or PostgreSQL CI. |
| Production SNS FIFO relay composition | apps/worker/src/main.rs; apps/worker/src/sns.rs; apps/worker/src/relay.rs; PgWorkerPool and PostgresOutboxRepository | Source composition uses explicit SNS FIFO topic ARN, region, ECS task-role credentials, bounded calls, worker DB secret reference and the relay worker. SNS tests validate region/topic, stable event identity and envelope contents. No live AWS publish/deployment test was evidenced. | PARTIAL. The producer runtime and adapter are implemented; actual AWS resource policy, deployed task role, queue/topic configuration, delivery and operational proof remain unverified. |
| Transactional consumer inbox marker and tenant/order constraints | crates/sitolo-persistence/src/consumer_inbox.rs; crates/sitolo-persistence/schema/migrations/0004_worker_consumer_inbox.sql; apps/worker/src/consumer.rs | part8_postgres_integration_tests: test_consumer_inbox_rolls_back_and_deduplicates_identical_events and test_consumer_inbox_serializes_concurrent_aggregate_sequences. | PASS for the inbox marker, duplicate identity, rollback, tenant scope and sequence checks that those tests exercise. This does not prove a real business effect. |
| Concrete consumer effect, production SQS loop and acknowledgement-after-effect | apps/worker/src/consumer.rs and sqs.rs define the injected effect/transport contracts; apps/worker/src/main.rs is the production entry point. | ConsumerProcessor source commits inbox and injected effect in one transaction, then acknowledges after commit. However, worker main deliberately exits when consumer SQS configuration is present because no approved business effect is supplied. Inbox database tests do not execute a production effect. | BLOCKED_BY_APPROVED_DECISION / NOT EVIDENCED. No concrete business effect or production consumer composition exists. No acknowledgement-after-real-effect, duplicate-effect or end-to-end SQS proof may be claimed. |
| Consumer SQS configuration delta | crates/sitolo-config/src/validate.rs; crates/sitolo-config/src/model.rs and fingerprint.rs; apps/worker/src/sqs.rs; environment configuration docs | consumer_settings_are_typed_and_invalid_values_fail_closed; SQS FIFO URL/region tests; owner-local fmt, strict Clippy and sitolo-config tests (28 + 2) recorded in the 2026-11-09 implementation note; exact-head CI above. | PASS for the configuration delta and tested validation behavior. This does not unblock consumer execution without an approved effect. |
| Schema migration source integrity and repository procedure | crates/sitolo-persistence/schema/migrations/0001–0006 and manifest.toml; scripts/ci/check-schema-migrations.py; docs/runbooks/postgresql-migration-release.md | Exact-head Rust and Integration workflows; manifest/checker are included in repository verification and disposable PostgreSQL test application. | PARTIAL. Ordered, checksum-pinned repository migrations and a repository-side runbook exist. A production migration owner, execution identity/runner, durable schema-version ledger, approved rollout, backup/PITR and incident/recovery ownership are not evidenced. |
| Risk-aware role assignment and revocation policy | crates/sitolo-application/src/tenancy.rs; role/scope operations and Phase 4 decision records | The authenticated application wrapper checks ROLE_ASSIGN and transactional tests exist, but the binding API/Phase 4 policy requires target-role risk classes, scope, SoD, step-up and any second-person approval. No generic assignment/revocation HTTP route is exposed. | BLOCKED_BY_APPROVED_DECISION. Do not expose generic role grant/revoke routes or infer risk policy. |
| Phase 3 identity/session completeness carried into release readiness | apps/api/src/bootstrap.rs; identity authority and session persistence; migrations 0005 and 0006; Phase 3 evidence matrix | Current session/logout implementation and related PostgreSQL tests exist. The carried requirements matrix still identifies missing complete login/session creation, refresh rotation/reuse, MFA/recovery, device lifecycle and real-provider evidence. | PARTIAL. This is a cross-phase release dependency; Part 8 CI success does not close Phase 3. |
| Operational observability and runbooks | worker relay trace/report paths; bounded worker metrics view; docs/runbooks/postgresql-migration-release.md | PostgreSQL metrics-view bounds and relay policy tests; repository migration runbook | PARTIAL. Technical bounds and repository migration procedure exist, but production metrics export/alerting, quarantine/DLQ operator procedure, deployment ownership and incident response evidence are not established. |

### Current audit findings

**S06-01 — Mandatory mutation matrix is not fully closed.** Severity: release-blocking for the claimed full Phase 4 mutation scope. Disposition: PARTIAL. The code has a durable transaction path and broad PostgreSQL evidence, but not every command has a demonstrated authenticated request path and complete actor/scope/denied-no-write/replay/concurrency evidence. The Stage 07 handoff must disposition each non-live command with an accountable owner; it must not silently equate an application method accepting ActorRef with a production-authorized command.

**S06-02 — No approved consumer business effect or production consumer composition.** Severity: release-blocking for consumer delivery. Disposition: BLOCKED_BY_APPROVED_DECISION / NOT EVIDENCED. The fail-closed startup guard is correct; removing it or supplying a no-op would be unsafe. A domain owner must approve the effect and its payload semantics before Engineering wires ConsumerProcessor into the worker runtime.

**S06-03 — Production migration/release governance is incomplete.** Severity: production-release blocker. Disposition: PARTIAL. Source migrations and a repository procedure are present, but the named accountable production owner, execution identity, runner, durable ledger location, rollout approval, backup/PITR and recovery/incident ownership are not recorded.

**S06-04 — Role-grant risk governance is not approved.** Severity: privileged-access blocker. Disposition: BLOCKED_BY_APPROVED_DECISION. The code’s ROLE_ASSIGN check is not a substitute for approved target-role risk classes, scope, separation-of-duties, step-up and second-person approval rules. Routes remain correctly absent.

**S06-05 — Phase 3 identity lifecycle evidence remains partial.** Severity: cross-phase release dependency. Disposition: PARTIAL. Existing OIDC/session/logout code does not prove the full login, refresh, MFA, recovery, device and provider lifecycle requirement.

**S06-06 — Production AWS and operations evidence remains partial.** Severity: production-release blocker. Disposition: PARTIAL. Source uses ECS task roles, SNS/SQS adapters and bounded processing, but live resource policy, runtime role deployment, delivery, metrics/alerts and quarantine/DLQ operations are not demonstrated.

### Historical Stage 06 finding reconciliation

| Historical finding | Current disposition | Basis / boundary |
|---|---|---|
| F-01 — unauthenticated provisioning could be wired to durable state | CLOSED for the previously unsafe route path. | The production bootstrap now composes persistent identity authority and PgTenancyUnitOfWork; the HTTP route derives owner identity from the verified SecurityContext. This does not close the entire mutation matrix. |
| F-02 — same-ID organization provisioning concurrency was unverified | CLOSED for organization provisioning. | The exact-head PostgreSQL Integration workflow passes the provisioning evidence suite; the same-payload concurrent case is included in the recorded evidence. Do not generalize it to all mutation commands. |
| F-03 — relay destination/runtime contract unresolved | SUPERSEDED / PARTIAL. | An explicit SNS FIFO producer adapter and production relay composition now exist. Consumer business effect and production SQS consumer composition remain blocked under S06-02. |
| F-04 — locked RSA dependency advisory | CLOSED for the configured dependency gate. | The dependency remediation was recorded in the implementation history and the current exact-head Security workflow succeeds. No advisory suppression is claimed. |

### Overall Stage 06 disposition

**AUDIT RESULT: NOT READY FOR MERGE OR PHASE 4 PART 8 CLOSURE.** Exact-head CI and the changed configuration audit pass. The organization provisioning transaction, tested RLS/privilege boundary, selected API authority paths and outbox state machine have meaningful source and PostgreSQL evidence. The outstanding findings above are not waived by a green build.

**Independence limitation:** this is a read-only AI source audit, not the required non-author human approval. A designated independent human reviewer must inspect this artifact and the pinned source and submit an actual GitHub APPROVE review. No approval is claimed or synthesized here.

**Disposition:** PR #49 remains OPEN and UNMERGED. Do not mark merge-ready, production-ready or Phase 4 Part 8 complete.

**Disposition:** Re-audit after Stage 07 remediation and exact-head CI on `dd583b2`. The newly introduced durable API wiring is removed. The same-payload concurrent provisioning requirement now has real PostgreSQL CI evidence. Other material requirements remain incomplete, so Stage 08 cannot clear the release gates.
**Review basis:** implementation diff and code paths, Stage 01 brief, Stage 02 research, Stage 04 plan, Stage 05 report, security controls SC-002, SC-006, SC-009, SC-010, and audit/security/correctness contracts.
**Human check:** The requester authorized autonomous continuation after saying the file was not visible. This is not recorded as manual review or acceptance of findings.

## Requirement trace

| Requirement / control | Actual path | Evidence | Disposition |
|---|---|---|---|
| Provision organization, owner membership, default branch, audit, and outbox atomically (SC-006, SC-009) | `TenancyService::provision_organization` uses one caller-owned SQLx transaction when evidence sink is configured; inserts bundle, audit, outbox, then commits | Application integration run 781 and canonical Rust run 631 passed success, audit rollback, and outbox rollback tests against PostgreSQL on `1aad95c` | PASS for the tested application transaction path; production API composition remains incomplete under F-01 |
| Idempotent provisioning and divergent ID conflict | Persistence insert treats a matching full bundle as retry and conflicts on mismatch | PostgreSQL integration run 782 on exact head `dd583b2` passed the application evidence suite (5 tests), including `provision_organization_concurrent_same_payload_is_idempotent`; canonical Rust run 632 also passed this case. Sequential divergent-ID behavior passed in earlier exact-head CI. | PASS for the tested organization-provisioning retry cases; does not prove other tenancy commands or relay consumer deduplication |
| Tenant bootstrap scope is server-derived, never caller authority (SC-002) | The API process now retains its in-memory reference path. The PostgreSQL transaction path exists only when a trusted application composition injects the evidence sink; the HTTP router still has no principal and passes request `owner_user_id` | Direct source inspection of `apps/api/src/serve.rs`, `crates/sitolo-api/src/tenancy.rs`, `apps/api/src/state.rs`, and `crates/sitolo-application/src/tenancy.rs`; targeted API tests pass but do not exercise authenticated authorization | PARTIAL — this change no longer amplifies the existing unauthenticated route into durable state, but planned production API integration is absent until authenticated authority exists |
| Database schema/role/RLS denies unauthorized paths (SC-002, SC-010) | New deployable SQL applies forced RLS and grants runtime SELECT/INSERT | On `1aad95c`, canonical Rust run 631 passed 23 Part 8 persistence tests and both invocations of the 27-test legacy RLS suite with the `app_runtime` identity asserted | PASS for the exercised schema, role, catalog, and RLS paths; this does not establish authenticated HTTP integration |
| Destination-independent worker control loop | `apps/worker/src/lib.rs` exposes a `RelayWorker` that recovers stale claims, claims a bounded batch, validates IAM event/version/envelope, publishes with `event_id` idempotency key, and fences each transition with its claim token | Local worker tests exercise successful publication, retry/backoff, terminal/unknown and unsupported-event quarantine, policy bounds, and fail-closed persistence errors | PARTIAL — orchestration logic is unit-tested, but no concrete database/runtime composition or approved destination adapter exists; no real consumer idempotency proof |
| Relay uses approved transport, destination, payload, and acknowledgement semantics | No destination adapter/runtime config in worker; requester answered “main and sub to be specific” | Repository evidence did not identify a contract; no endpoint/queue/topic or runtime field set supplied | FAIL — destination-dependent delivery, idempotent consumer proof, and operator evidence cannot be implemented without inventing semantics |
| CI verifies PostgreSQL and exact-head gates | Integration and Rust workflows provide a PostgreSQL service and admin/runtime/worker URL variables | On `1aad95c`, policy run 484 and integration run 781 passed; canonical Rust run 631 passed all tests, PostgreSQL suites, build and non-advisory checks, then failed at cargo-audit | PARTIAL — PostgreSQL evidence is green; required advisory gate keeps canonical verification red |
| Supply-chain gates pass (SC-011) | Lockfile updates yoke-derive; RSA 0.9.10 is in lockfile | Exact-head old audit reports RUSTSEC-2023-0071; all-features/target Cargo tree has no active RSA path; current cargo-deny toolchain workaround is unverified | FAIL — audit gate and dependency-policy evidence unresolved |

## Findings

### F-01 — Durable API provisioning awaits authenticated authority

**Severity:** Release-blocking contract/security gap (SC-002). The HTTP server dispatches organization creation without authentication context. Its DTO contains a caller-supplied owner ID; comments explicitly defer principal-derived identity to PR-007. Stage 05 had briefly wired this route to durable PostgreSQL state, which would have persisted caller-controlled authority. Stage 07 removed that wiring. The application transaction capability remains available to a future authenticated caller. The current HTTP tenancy API remains an unauthenticated/in-memory reference; the planned production API integration is therefore still incomplete.

**Re-audit result:** The unsafe new durable wiring is removed. No newly added HTTP database path remains. Residual closure requires authenticated principal integration, server-derived owner/actor/tenant authority, and exercising the real request boundary before durable API wiring is restored.

### F-02 — Concurrent same-ID provisioning remains unverified

The requester cannot provide a local PostgreSQL instance/URLs, but CI now supplies real PostgreSQL. After the fixture role lock and organization-count query fixes, application integration run 781 and canonical Rust run 631 passed the PostgreSQL application, Part 8 persistence, and legacy RLS suites. Run 631 passed 23 Part 8 tests and both 27-test RLS invocations; the `SET ROLE app_runtime` correction is therefore exercised. The sequential retry test proves idempotency and divergent conflict. The plan also requires concurrent same-ID requests; a two-request integration test is added locally but has only compiled, not executed against PostgreSQL. No claim is made for that concurrency behavior until exact-head CI runs it.

### F-03 — Worker relay contract is unresolved

The transport-neutral orchestration engine now exists in `apps/worker/src/relay.rs`, but the executable deliberately refuses to start. “main and sub” does not establish transport, endpoint/topic/queue names, payload/version contract, acknowledgement semantics, configuration keys, or operator ownership. No relay destination was inferred. Destination-specific adapter, production composition, delivery/idempotent-consumer evidence, metrics export, and runbook remain blocked.

### F-04 — Required dependency policy status is unresolved

The `1aad95c` `cargo audit` job reports `RUSTSEC-2023-0071` for locked `rsa 0.9.10` (Marvin timing attack); RustSec lists no patched release. The all-features/target Cargo tree has no active path to RSA, but the lockfile scanner still reports it and there is no approved exception. Do not suppress it. The pinned cargo-deny action passes on `1aad95c` with the Rust 1.85.0 toolchain override and non-yanked yoke-derive lock resolution. The separate advisory job still fails; resolve through dependency remediation or an authorized governance disposition, not by weakening the gate.

## CI evidence reviewed

Historical findings for earlier pushed SHAs are retained below. The latest exact-head evidence is:

- On `dd583b24a4f548437b368d59e91fe4e438dc624a`, integration run 782 passed; policy run 485 passed; security run 639 passed dependency-policy, CodeQL, offensive-regression, and secret scanning, while advisories failed on `RUSTSEC-2023-0071`.
- Canonical Rust run 632 passed application evidence tests (5), Part 8 PostgreSQL persistence tests (23), the RLS suite (27 in both invocations), format, lint, compile, release build, architecture, Phase 2 policy, and dependency/license policy. It failed at `cargo audit` on locked `rsa 0.9.10` (`RUSTSEC-2023-0071`). The concurrent same-payload provisioning test ran against PostgreSQL and passed.

For pushed SHA `1223ed3aef3818f56c5b444ee22fff287546349f`, GitHub reported:

- The original pushed head's policy run rejected reports inside reusable output shelves. The later `dca49f8` policy run passed ICM, PowerShell parity, and System Map checks, then found six archived-document links still pointing into those shelves. The links are fixed locally; `python scripts/ci/check-doc-references` passes with 0 unresolved references.
- The original integration job omitted database URLs. `dca49f8` correctly started PostgreSQL and exposed the shared-role setup race; `561bbe9` confirmed the fixture lock removed it and surfaced an incorrect organization count query. Integration run 779 on `1383233` passed all four app provisioning tests against PostgreSQL after the assertion fix.
- `dca49f8` dependency-policy passed with the pinned cargo-deny action and toolchain override. The advisory job still fails for RUSTSEC-2023-0071. CodeQL/offensive-regression were in progress in the last snapshot; secret scanning passed.
- On `1383233`, the canonical verifier reached all-target tests and Part 8 persistence tests, then failed the legacy RLS suite's runtime identity precondition. Commit `0b85158` added `SET ROLE app_runtime`; `1aad95c` policy run 484 passed and integration run 781 passed. Canonical Rust run 631 passed the application and all PostgreSQL persistence/RLS tests, then failed at `cargo audit` on `RUSTSEC-2023-0071`; exact-head cargo-deny passed. A concurrent same-ID provisioning test was added locally and later passed at exact head `862775c`.
- Policy run 483 on `0b85158` failed the System Map process check because the PostgreSQL authority card still cited the pre-change source revision. The process card and persistence object card now cite `0b85158`, describe the role assumption, and the generated object index has been refreshed. Exact-head policy run 484 on `1aad95c` passed.

## Audit conclusion

The application transaction, Part 8 persistence, legacy RLS behavior, and concurrent same-payload organization provisioning have exact-head PostgreSQL CI evidence on `dd583b2` and again on `862775c`. This evidence is scoped to the tested application transaction and fixture. The new worker engine's unit tests and all-target suite pass at `862775c`, but it has no real destination adapter and has not delivered an event. Production API integration awaits authenticated authority, the relay contract remains unresolved, and the RSA advisory was removed at `862775c`; exact-head `verify` and `advisories` passed. PR-009 Part 8 is not merge-ready. The production command coverage and real relay delivery remain open.

## Worker engine audit delta — 2026-10-20

Reviewed `apps/worker/src/lib.rs`, `apps/worker/src/relay.rs`, `apps/worker/src/main.rs`, and the `IamEventName::ALL` registry addition at local commit `2048c8a6a2a216d466240e1e1a1f5f99dae111d7` against the Stage 04 destination-independent plan and Part 8 contract. This commit is included in the pushed `862775c` head; its worker tests and all-target suite passed in the exact-head `verify` job.

| Check | Finding | Status |
|---|---|---|
| Bounded claim and lease recovery | `RelayWorker::process_batch` recovers stale leases before claiming with the configured bounded `OutboxClaimPolicy`; it requires `CLAIMED` state and a claim token before any publish or transition. Recovery/persistence errors propagate. | PASS for orchestration; fake-reader tests only, no exact-head PostgreSQL integration of the worker engine |
| Event trust and schema boundary | The engine accepts only registered IAM names, event version 1, equal schema version, and a JSON object envelope; unsupported events are quarantined before publish. | PARTIAL — there is no per-event JSON field/schema validation or registry-owned payload type |
| Publish result and retry | The stable `event_id` is passed as the idempotency key; success is persisted only after adapter acknowledgement; retryable failures receive capped exponential backoff; terminal and unknown classifications quarantine. | PASS for the adapter port/orchestration tests; no concrete adapter acknowledgement semantics or real idempotent consumer exists |
| Failure handling | Database and claim-transition errors stop the batch/run path; no persistence error is swallowed. Unknown adapter results fail closed to quarantine. | PASS in code inspection; recovery failure has a regression test |
| Resource bounds | Batch size, lease duration, attempts, poll interval, and retry interval have finite bounds. | PASS for policy constructor and tests |
| Observability / operations | The batch emits aggregate structured trace counts without payloads or tokens. | PARTIAL — no exported worker counters, production telemetry composition, deployment configuration, or operator quarantine runbook |
| Production executable | `main` exits 78 until the approved adapter and its runtime configuration exist; it does not connect to PostgreSQL or publish events. | FAIL for the full P0 worker runtime/delivery requirement; this deliberate guard is not a running relay |

Local audit evidence: worker/audit tests 7 + 3 pass, targeted clippy passes, workspace all-target check passes, and `git diff --check` passes. The new files ran in the exact-head `verify` job at `862775c`. This delta does not close F-03 or authorize Stage 09. It confirms a useful destination-neutral engine while retaining the full external relay contract and production-wiring blocker.

## F-04 advisory remediation delta — 2026-10-21

F-04 disposition changed from "unresolved, no safe automatic remediation" to "remediated by dependency construction and verified at exact head `862775c`."

- Scope reviewed: the workspace uses only PostgreSQL and only the `FromRow` derive. The `sqlx` meta-crate and `sqlx-macros` exist only to forward features and macros, but their optional MySQL/SQLite packages are still recorded in `Cargo.lock`. `cargo audit` scans the lockfile, not the enabled feature graph, which is why `rsa 0.9.10` was reported while `cargo tree -i rsa` found no active path.
- Code change audited: direct `sqlx-core` and `sqlx-postgres` dependencies; `_rt-tokio` on `sqlx-core`; `chrono` and `json` on `sqlx-postgres`; explicit `FromRow` implementations for `OutboxEventRow`, `FunctionSecurityRow`, and `RelationSecurityRow`. Query builders and core traits come from `sqlx_core`; PostgreSQL types come from `sqlx_postgres`. No compile-time query macros, migrations, or non-PostgreSQL backends are used.
- Integrity: `Cargo.lock` was regenerated by Cargo. It no longer contains `rsa`, `sqlx`, `sqlx-mysql`, `sqlx-macros`, `sqlx-macros-core`, or `sqlx-sqlite`; package count is 191, down from 220. `deny.toml` remains `ignore = []`.
- Local gate evidence: `cargo-audit` 0.22.2 PASS (191 dependencies, no vulnerabilities); `cargo-deny` 0.20.2 PASS (advisories, bans, licenses, sources); format, locked all-target/all-feature check, all-feature clippy, 217 library tests, 11 API tests, 5 outbox tests, 4 security tests, all-target test compilation, and release build all pass with one build job. Exact-head `verify` and `advisories` passed at `862775c`; PostgreSQL suites also passed there.
- Residual: Stage 08 remains the authority for final disposition; this delta does not authorize merge. The pre-existing `rg` gap means the CI `verify` job's architecture and Phase 2 checks did not actually execute.

A separate pre-existing gate defect was observed during local verification: the CI `verify` job does not install `ripgrep`, so `check-architecture` and `check-phase2-policy` currently report PASS without executing their `rg` searches. With `rg` present, `check-phase2-policy` flags bare `DATABASE_URL` fallbacks in three PostgreSQL test files. Those reads are test harness inputs, not runtime source, and they predate this change. This delta records the finding; it does not modify the policy script or the tests, so a repository human can decide whether to scope the check to runtime source or remove the aliases in a separate change.
