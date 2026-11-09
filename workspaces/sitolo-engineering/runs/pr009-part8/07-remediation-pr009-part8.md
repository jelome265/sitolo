# Stage 07 Remediation — PR-009 Part 8

## Current exact-head Stage 07 handoff — 2026-11-09

**Current authoritative handoff.** This addendum consumes the exact-head Stage 06 re-audit recorded at the top of 06-audit-pr009-part8.md. It supersedes older readiness and owner statements below where they conflict. Historical records remain unchanged for provenance.

### Input, scope and authority

- Audited implementation source: 5e963123dcb84d9d1d50993d7611ddbc1bae0466.
- Stage 06 artifact update commit: 4c98284be05c1af567a77d59b03e3222f30c49f9.
- PR: #49, feat/phase4-pr009-audit-outbox → main.
- Stage 07 purpose: disposition each current Stage 06 finding, identify the accountable owner role, record required decision/remediation and define evidence required for closure. This is not a claim that implementation or production deployment has occurred.
- No source-code remediation is authorized by this handoff without the owner decisions listed below. Do not invent business effects, AWS resource identities, migration ownership or role-risk rules.
- The repository owner remains the implementation/merge authority. The non-author human reviewer is not yet identified.

### Finding disposition and required handoff

| Finding / prior item | Current disposition | Accountable owner role | Required decision or remediation | Closure evidence |
|---|---|---|---|---|
| Historical F-01 — unauthenticated durable provisioning path | CLOSED for the unsafe route path | Engineering / API | Preserve production bootstrap composition using PgRequestIdentityAuthority and PgTenancyUnitOfWork. Keep owner and actor identity derived from verified SecurityContext; do not reintroduce caller-supplied authority. | Exact-head API and PostgreSQL tests continue to pass; route test verifies principal-derived owner and audit actor. Broader mutation coverage remains S06-01. |
| Historical F-02 — concurrent same-payload organization provisioning | CLOSED for organization provisioning only | Engineering / Persistence | Preserve unique/idempotent bundle semantics and the concurrent same-payload regression. Do not treat this as proof for every other command. | PostgreSQL Integration exact-head evidence for the application provisioning suite, including same-payload concurrent retry and divergent-ID conflict. |
| Historical F-03 — relay destination | PARTIALLY RESOLVED; consumer side remains blocked | Domain/Product + Platform/Security + Engineering | Preserve the explicit SNS FIFO producer contract. Obtain approval for the concrete consumer business effect and payload semantics; confirm exact source/quarantine queue identities, IAM permissions, deployment role and redrive policy before composing the SQS processor. | Real disposable integration evidence for publish/receive, effect+inbox atomicity, duplicate replay, acknowledgement only after commit, retry, quarantine and permission denial; production resource/role evidence. |
| Historical F-04 — RSA dependency advisory | CLOSED for the configured dependency gate | Engineering / Supply-chain | Keep the SQLx dependency remediation and advisory gate intact. Do not add ignores or suppressions. | Current exact-head Security workflow succeeds; dependency-policy/advisory evidence remains in that run. |
| S06-01 — mandatory mutation matrix not fully closed | PARTIAL — release gate remains open | Engineering, with Product/Security for operation scope | For every matrix row, record whether it is a live command, an internal application capability or explicitly deferred. For every live command, prove principal/actor provenance, tenant scope, permission, state transition, same-transaction audit/outbox, denial/no-write, rollback, replay and concurrency as applicable. Keep unapproved role/scope routes absent. Do not create routes merely to make the matrix look complete. | Updated source-to-test matrix on the implementation SHA; exact-head PostgreSQL success, denied/no-write, rollback, replay and concurrency cases for every command claimed in scope; explicit owner-approved dispositions for deferred commands. |
| S06-02 — consumer effect and production consumer composition | BLOCKED_BY_APPROVED_DECISION / NOT EVIDENCED | Product/Domain owner for the effect; Platform/Security for AWS resources and IAM; Engineering for implementation | The Product/Domain owner must name the real business effect and authoritative input/output semantics. Platform/Security must approve source and quarantine FIFO resources, task role, permissions, secret reference, redrive/DLQ and deployment boundary. Engineering may then inject the approved effect and compose ConsumerProcessor in worker startup. Never use a no-op effect or clear the existing fail-closed guard without that approval. | Integration run against approved disposable PostgreSQL and isolated SNS/SQS resources: inbox marker and business effect commit/rollback together; duplicate delivery applies the effect once; source acknowledgement follows commit; failed/retryable effect remains unacknowledged; permanent failure transfers to quarantine before source deletion. Capture resource and role evidence. |
| S06-03 — production migration/release ownership | PARTIAL; production release blocked | Database/Platform owner | Name the accountable migration owner and execution identity; identify the runner and durable schema-version ledger; approve rollout window, backup/PITR readiness, role provisioning and incident/recovery owner. Preserve immutable numbered migrations and manifest checksums. | Owner-approved release record; clean apply and upgrade evidence; ledger state; catalog/RLS/privilege checks; runtime DDL denial; documented recovery/forward-fix execution. |
| S06-04 — role-grant risk governance | BLOCKED_BY_APPROVED_DECISION | Product/Security owner | Approve role-risk classes, target-role constraints, scope rules, SoD, step-up requirements, second-person approval and ownership-transfer handling. Keep generic role assignment/revocation routes absent until the policy is approved and implemented. | Signed/recorded decision; application enforcement and PostgreSQL denial/no-write tests for owner/admin/cashier and cross-scope cases; independent re-audit. |
| S06-05 — Phase 3 identity lifecycle | PARTIAL; cross-phase release dependency | Identity / Security Engineering and Product owner | Complete or explicitly phase-gate durable login/session creation, refresh rotation/reuse detection, MFA/recovery, device lifecycle, provider evidence, secret redaction and readiness evidence. Do not claim Phase 3 complete from the logout implementation alone. | Updated Phase 3 evidence matrix with real PostgreSQL/provider lifecycle, invalidation, replay/race and failure evidence on an exact implementation SHA. |
| S06-06 — production AWS and operations evidence | PARTIAL; production release blocked | Platform/SRE/Security | Provide deployed SNS/SQS resource policies, ECS task-role trust/permissions, secret-manager wiring, metrics/alerts, worker health/shutdown, retry/quarantine/DLQ runbook and incident ownership. | Deployment configuration review; runtime identity and least-privilege tests; successful and failed delivery evidence; operator runbook validated in a controlled environment. |
| ECC Tools harness warning — the newly added Rust audit skill has no corresponding ECC manifest | OPEN, medium, non-blocking to the changed-config PASS unless repository ECC policy says otherwise | Harness maintainer / repository owner | Check the expected ECC manifest format and decide whether the skill needs a manifest. Do not mistake this warning for a Rust/security/config workflow failure. | ECC harness audit no longer reports the warning, or a documented owner disposition explains why the file is intentionally not manifested. |
| Non-author human approval | NOT OBTAINED | Repository owner to nominate; designated non-author reviewer to act | The repository owner has requested review from `zagweratadala33-cpu`, a non-author contributor with prior Phase 4/RLS implementation history. The reviewer must inspect the source diff, this Stage 06 report and this Stage 07 handoff against the final post-documentation head, then submit an actual APPROVE or REQUEST_CHANGES review. If the reviewer declines or cannot review, the owner must nominate another qualified non-author. The request itself is not approval. | GitHub review record with reviewer login distinct from jelome265, state APPROVED, and commit association to the final reviewed PR head. |

### Ownership and approval ledger

| Gate | Owner identity | Current state |
|---|---|---|
| Repository implementation and merge authority | jelome265 (PR author) | Owner; not eligible to satisfy the non-author review requirement |
| Product/domain business-effect approval | Not appointed in the repository evidence | NOT RECORDED |
| Product/security role-risk approval | Not appointed in the repository evidence | NOT RECORDED |
| Database/platform migration owner | Not appointed in the repository evidence | NOT RECORDED |
| Platform/AWS runtime owner | Not appointed in the repository evidence | NOT RECORDED |
| Independent human reviewer | `zagweratadala33-cpu` (non-author contributor; authored relevant Phase 4 PRs including [PR #31](https://github.com/jelome265/sitolo/pull/31) for RLS hardening) | REVIEW REQUESTED on PR #49 at head `a7fde04ab6d92ea92180be2b7e5e8e0b16448986`; approval NOT YET SUBMITTED; submitted review count was zero when checked |

A non-author reviewer request was submitted to `zagweratadala33-cpu`, a prior contributor to relevant Phase 4/RLS work (PRs #14–#31, including PR #31). GitHub accepted the request. This is only a request: the reviewer has not submitted an approval, and no human approval is claimed. Other unassigned business, security-policy, database/platform and AWS owner roles remain unassigned.

### Next instruction — Stage 08 exact-head verification and reviewer handoff

**Instruction for the next engineering/audit agent:**

1. Read this current Stage 07 section and the current exact-head Stage 06 section before taking action. Preserve the pinned audit source SHA 5e963123; do not rewrite its conclusion to match a later branch tip.
2. Fetch PR #49 and record its actual head SHA after the Stage 06 and Stage 07 documentation commits. The source-audit SHA and the new documentation/PR head are different by design.
3. Run or observe the required Rust, PostgreSQL Integration, Security and Policy workflows on that exact current PR head. Verify that every required run’s tested commit SHA equals the PR head SHA and that each conclusion is success. Do not reuse the 5e963123 run IDs as proof for a later documentation head.
4. Use the disposable PostgreSQL service in CI for database suites. Do not install or start a local database merely to reproduce CI. Confirm migration-manifest verification and ICM/document-reference checks are included in the exact-head required gates.
5. Append the exact new head, run IDs, per-workflow conclusions, and any test omissions to 08-verification-pr009-part8.md. Keep PARTIAL and BLOCKED findings open; green CI does not close owner decisions or production evidence gaps.
6. After the exact-head evidence is recorded, the repository owner must nominate a qualified non-author reviewer and request review. The reviewer must independently validate the Stage 06 trace and the unresolved Stage 07 dispositions, then submit an actual GitHub review. Do not create or imply approval through an AI account, bot comment, author self-review or unsubmitted draft.
7. Do not merge, close the PR, close Phase 4 Part 8, or claim production readiness. Stage 09 delivery remains blocked until the approved effect, mutation scope, migration ownership, role-risk policy, Phase 3 release dependency and required human approval have been resolved or formally accepted by their accountable owners.

**Stage 07 outcome:** HANDOFF RECORDED; REMEDIATION AND RELEASE GATES REMAIN OPEN. PR #49 stays OPEN and UNMERGED.

## Current remediation addendum — 2026-10-25

- R-07: replaced the empty testkit scaffold with reusable validated tenancy identifiers and active organization/branch/membership fixtures; tenancy-domain and application tests consume them; dependency edges are test-only and boundary-checked. Exact-head CI passed on `468186c`.
- R-05: removed the organization activation route from the active API inventory because organization provisioning atomically persists an `ACTIVE` tenant with its owner and default branch and no persisted provisioning state exists. The API contract and System Map now describe the route disposition; the Axum boundary test verifies 404 and all five remaining protected routes have missing-credential coverage. Exact-head CI passed on documentation head `0f70426`.
- No independent Stage 06 result or human review is claimed. Consumer business effect and production migration/release decisions remain unresolved; see the Phase 1–4 requirements matrix.

**Input:** `06-audit-pr009-part8.md`
**Decision:** Apply the safe correction available within this slice; retain unresolved findings and require re-audit.

## Finding disposition

| Finding | Action | Result / residual |
|---|---|---|
| F-01 Unauthenticated provisioning + durable runtime | Removed `StartupContext::build()` PostgreSQL connection and `AppState::configure_postgres_tenancy`; removed API binary's direct SQLx dependency | Prevents this change from wiring the unauthenticated HTTP DTO to durable database mutations. Atomic app/persistence capability and PostgreSQL integration tests remain for a future caller. Full closure requires authenticated principal integration and server-derived owner identity. Existing in-memory HTTP tenancy routes are a pre-existing Phase 3 limitation. |
| F-02 PostgreSQL behavior / provisioning race evidence | No local PostgreSQL or supplied DB identities are available; CI PostgreSQL service is available | Integration run 782 and canonical Rust run 632 on exact head `dd583b2` passed the concurrent same-payload provisioning case against PostgreSQL, along with 5 application evidence tests, 23 Part 8 persistence tests, and both 27-test RLS invocations. |
| F-03 Relay contract unresolved | Added the destination-independent worker engine, event/version envelope guard, stale-claim recovery, bounded retry/quarantine flow, and fail-closed executable. | PARTIAL remediation: orchestration exists and has local tests; no approved transport, adapter configuration, production DB composition, real consumer idempotency, metrics export, or destination-specific runbook. “Main and sub” remains insufficient. |
| F-04 dependency policy unresolved | Replaced the `sqlx` meta-crate and `sqlx-macros` with direct `sqlx-core` and `sqlx-postgres`; replaced the three `FromRow` derives with explicit implementations; regenerated `Cargo.lock`. No advisory suppression or exception added. | Remediated: `Cargo.lock` no longer contains `rsa`, `sqlx-mysql`, `sqlx-macros`, or the other non-PostgreSQL SQLx packages; `cargo audit` and `cargo-deny` pass locally, and exact-head `verify` and `advisories` passed at `862775c`. |

## Changed files

- `apps/api/src/bootstrap.rs`: removed production process database connection and role assumption.
- `apps/api/src/state.rs`: removed durable tenancy evidence-sink wiring.
- `apps/api/Cargo.toml`: removed now-unused direct SQLx dependency.
- `crates/sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql`: added a transaction-scoped advisory lock around cluster-global fixture role creation after the `dca49f8` PostgreSQL CI run exposed a unique-index race.
- `crates/sitolo-persistence/src/postgres.rs`: runtime pool now executes `SET ROLE app_runtime` on each connection, matching the explicit least-privileged runtime identity required by the RLS harness.
- `map/processes/postgres-authority-initialization.md` and `map/objects/data/persistence.md`: source revision and citations now track the exact code commit and describe the runtime role assumption; `map/objects/_index.md` was regenerated after policy run 483 found the stale source revision.
- `crates/sitolo-application/tests/part8_provision_organization_evidence_tests.rs`: added an `organizations.id`-specific count helper after the `561bbe9` PostgreSQL run exposed the generic helper's invalid `organization_id` query.
- Archived run reports: rewired relative stage-output references to sibling files under `runs/pr009-part8` after the policy checker surfaced six stale links.

The application transaction implementation remains and is still compile/test-target checked. This remediation does not claim production API atomicity: the endpoint cannot safely use it until authentication context exists. No audit or evidence requirement was weakened.

## Destination-independent worker remediation — 2026-10-20

Stage 04 allowed transport-neutral worker work while the destination remained unresolved. Added `apps/worker/src/lib.rs` and `apps/worker/src/relay.rs` in local commit `2048c8a6a2a216d466240e1e1a1f5f99dae111d7`; registered IAM event names in `crates/sitolo-audit/src/iam_event.rs`; added a library API for a future concrete `EventPublisher`; and changed `apps/worker/src/main.rs` to exit closed until an approved adapter/configuration exists. The worker recovers stale claims, validates bounded policy and event envelopes, passes stable event IDs to the adapter, persists token-fenced publish/retry/quarantine transitions, and emits payload-free aggregate trace counts. This commit is included in pushed exact head `862775c`.

The Stage 06 delta audit finds this engine locally correct for the tested orchestration but explicitly PARTIAL for event schemas and observability, and FAIL for full production worker execution until adapter and database composition exist. The destination and consumer contract remain external inputs. No endpoint, queue, topic, protocol, or configuration key was inferred.

## Verification and routing

The implementation change was re-audited in Stage 06. Targeted checks pass: `cargo fmt --all -- --check`, `cargo test -p sitolo-api-bin --test startup --test tenancy_api` (11 tests), `cargo check --workspace --all-targets --locked`, the ICM cold-walk, and `python scripts/ci/check-doc-references` (0 unresolved links). Review of `561bbe9` exposed an organization count query bug, corrected in `1383233`. On exact head `dd583b2`, integration run 782 and canonical Rust run 632 passed the concurrent same-payload provisioning test against PostgreSQL; Rust run 632 also passed 23 Part 8 persistence tests and both 27-test RLS suite invocations. Policy run 485 passed. Canonical verification and the security advisory job failed at cargo audit (`RUSTSEC-2023-0071`) on that head; the PostgreSQL-only SQLx remediation was verified at exact head `862775c`, where `verify`, `advisories`, `policy`, and integration all passed. Audit-stage human review was not performed; continuation is under the requester’s prior instruction to proceed autonomously, not a representation of approval. Production API authority and the worker relay contract remain unresolved, so release verification cannot clear.

## Verification hand-back — 2026-10-20

Stage 08 exact-head verification confirmed the database transaction and concurrency tests pass in CI but the canonical and security workflows failed on the locked RSA advisory at `dd583b2`. The RSA advisory is removed by the PostgreSQL-only SQLx dependency change; exact-head CI at `862775c` passed `verify`, `advisories`, `policy`, and integration. No safe in-scope automatic remediation is available for the remaining blockers: the relay lacks a defined transport/destination and event delivery contract, and the live API lacks authenticated principal-derived authority. Do not suppress any advisory or restore unauthenticated durable API wiring. The verification report records the exact CI evidence and local-host limitations; Stage 09 is not entered.

The worker engine changes are included in pushed exact head `862775c`. After the worker change, local format, worker/audit tests (7 + 3), targeted clippy, workspace all-target check, and diff checks pass; the exact-head `verify` job also ran the worker tests and all PostgreSQL suites. These checks do not replace a real destination adapter or consumer/database integration evidence for delivery. Since only destination-neutral work is possible without the missing contract, no further safe code change can complete F-03. Merge readiness remains blocked.

## Follow-on remediation — 2026-10-23 (working tree)

This section records the currently uncommitted continuation from repository `HEAD` `62eeae6ad17a78ccb062a1ab9445f5ac8e92c132`. The working tree is the candidate under review; that HEAD does not identify these uncommitted changes and is not represented as their final source SHA.

### Implemented in this working tree

- Added an explicit SNS FIFO publisher implementation in `apps/worker/src/sns.rs`. Its destination region and FIFO topic ARN are injected and validated; it retains event identity, type/version, organization/branch scope, aggregate sequence, and payload; derives FIFO grouping from tenant and aggregate identity; uses the event ID for deterministic deduplication; bounds the request timeout; and classifies retryable, terminal, and destination-unavailable errors. It does not invent a production topic ARN.
- Expanded API startup/authentication regression coverage and propagated persistence test setup/teardown failures. Readiness now additionally requires an explicitly composed tenancy service, along with a connected runtime database and identity/session dependencies.
- Tightened `PgRuntimePool` schema readiness to require the PostgreSQL organization/branch lifecycle and invitation-resolution functions used by the live tenancy command path.
- Added workspace Clippy denials for `unwrap_used`, `expect_used`, `panic`, `unimplemented`, `todo`, and `unreachable`, and enabled inherited workspace lints in member crates.
- Converted the RLS harness to propagate setup, test, and teardown failures instead of hiding them behind panics. PostgreSQL-backed execution was not performed locally.
- Added a validated consumer envelope parser and `ConsumerProcessor` ports for SQS-compatible delivery, an owner-supplied transactional business effect, and quarantine. The processor commits the effect and inbox marker together, acknowledges only after commit, leaves retryable failures unacknowledged, and routes invalid/permanent failures through the injected quarantine port. No queue URL, queue policy, or business effect is supplied.
- Added `PostgresConsumerInbox` and `consumer_inbox.sql`: event identity and SHA-256 payload metadata are recorded under a unique consumer/event key, duplicate content is safe, event-id reuse with different content fails closed, and RLS scopes rows by organization. A PostgreSQL integration case covers rollback, duplicate, conflicting payload, and cross-tenant visibility.
- Wired the relay binary to load validated configuration, retrieve a distinct worker database credential, verify a direct `app_worker` PostgreSQL login and required outbox functions, then run `RelayWorker` with the SNS FIFO publisher. Added `SITOLO__WORKER__DATABASE__PASSWORD_REF`; it must be configured together with the SNS relay destination. Production resolves it through ECS task-role Secrets Manager, while non-production uses the existing environment secret provider. Shutdown signals are forwarded to the relay loop and the worker waits for in-flight processing to finish.
- Added `PgWorkerPool` startup checks for restricted role attributes, absence of role memberships, schema presence, and the exact outbox functions required by the repository. The API runtime pool remains separately authenticated as `app_runtime`.

### Verified locally on the working tree

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `git diff --check` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS — includes worker executable composition and worker-role pool. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS — includes newly wired worker executable. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS |
| `cargo test -p sitolo-worker-bin --lib --all-features --locked` | PASS — 15 relay, SNS publisher, consumer envelope, and no-ack quarantine unit tests. |
| `cargo test -p sitolo-persistence --test part8_postgres_integration_tests --all-features --locked --no-run` | PASS — integration target including inbox rollback, duplicate, conflict, and tenant isolation compiles; no PostgreSQL execution is claimed locally. |
| `cargo test --workspace --lib --locked` | PASS — 259 unit tests at the time of that run; one later consumer quarantine test passes in the 15-test worker target. |
| `cargo test -p sitolo-api-bin --test startup --all-features --locked` | PASS — 4 startup tests. |
| `cargo test -p sitolo-api-bin --test tenancy_api --all-features --locked --no-run` | PASS — target compiles; its PostgreSQL runtime cases were not executed locally. |
| `cargo test --workspace --locked` | STARTED, then interrupted after prolonged silence; no passing result is claimed. PostgreSQL service is not available locally. |
| `cargo deny check` | PASS — advisories, bans, licenses, and sources. |
| `cargo audit --locked` | NOT RUN — `cargo-audit` is not installed on this host; the prior exact-head audit evidence remains only for `862775c`, not this working tree. |
| `python scripts/ci/check-doc-references` | FAIL — four ambiguous [root Codex entry point](../../../../AGENTS.md)/[root ICM router](../../../../CLAUDE.md) references occur only in the user-supplied untracked `.agents/` skill files; the source tree and remediation report were not the reported locations. |
| GitHub Actions at the working-tree candidate | NOT RUN; no candidate commit was pushed and no workflow result is available. |

### Remaining gaps and external dependencies

- The relay binary is now source-composed, but no live PostgreSQL/SNS execution or working-tree CI run is available. Runtime execution requires deployment to provision the restricted `app_worker` database identity, supply `SITOLO__WORKER__DATABASE__PASSWORD_REF`, SNS FIFO destination settings, and the managed secret-provider region/permissions. The SQS consumer receive/ack transport adapter, durable inbox schema deployment, and quarantine/DLQ adapter still require deployment composition; inbox behavior has only compile evidence until its PostgreSQL integration test runs in CI. The business effect and exact consumer queue configuration remain owner decisions.
- The live HTTP authentication path is present, but production tenancy reads and every mandatory organization, branch, membership, invitation, role, and scope mutation are not yet demonstrated to use authoritative PostgreSQL transactions with same-transaction audit and outbox, idempotency, and concurrency evidence. The provision-organization path has prior CI evidence only at `862775c`.
- The runtime database/session/tenancy composition and readiness gate are incomplete as production evidence: required schema compatibility and all dependency health checks have not been proven at this working-tree candidate. A migration/release interface and production migration ownership contract remain unresolved.
- Exact-head PostgreSQL evidence for this working tree, all requested GitHub checks, a current-source independent Stage 06 audit, and final human review are pending. No production deployment, human review, or merge is claimed.

The external owner decisions remain: (1) SQS FIFO consumer/business-effect approval and exact queue configuration, (2) production migration/release contract and owner, (3) independent Stage 06 auditor, and (4) human PR reviewer/merge authority. These decisions are not inferred by this implementation.

## Advisory remediation — 2026-10-21

The RSA advisory was remediated without exception or suppression:

- Root cause: the `sqlx` meta-crate and `sqlx-macros` declare optional MySQL/SQLite packages. Cargo records optional packages in `Cargo.lock`; `cargo audit` scans the lockfile even though no enabled feature or target compiles them.
- Change: the workspace now depends on `sqlx-core` and `sqlx-postgres` directly. `sqlx-core` uses `_rt-tokio`; `sqlx-postgres` uses `chrono` and `json`. Three `sqlx::FromRow` derives are explicit implementations. Query builders and shared traits come from `sqlx_core`; PostgreSQL types come from `sqlx_postgres`.
- Lockfile: `Cargo.lock` was regenerated by Cargo and no longer contains `rsa`, `sqlx`, `sqlx-mysql`, `sqlx-macros`, `sqlx-macros-core`, or `sqlx-sqlite`; 191 packages remain, down from 220.
- Policy: `deny.toml` remains `ignore = []`; `--locked` remains in every gate; no CI step was changed.
- Local evidence: `cargo audit` 0.22.2 PASS (no vulnerabilities); `cargo-deny` 0.20.2 PASS; format, locked all-target/all-feature check and clippy, 217 library tests, the non-PostgreSQL integration suites, all-target test compilation, and release build PASS. PostgreSQL execution remains NOT RUN locally; exact-head CI at `862775c` passed the PostgreSQL suites.
- Residual: exact-head `verify` and `advisories` passed at `862775c`. The pre-existing CI `rg` gap in `check-architecture`/`check-phase2-policy` is recorded in Stage 06 and not changed here.
