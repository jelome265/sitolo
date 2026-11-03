# Stage 07 Remediation — PR-009 Part 8

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
