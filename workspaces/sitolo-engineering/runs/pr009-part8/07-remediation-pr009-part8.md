# Stage 07 Remediation — PR-009 Part 8

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

## Advisory remediation — 2026-10-21

The RSA advisory was remediated without exception or suppression:

- Root cause: the `sqlx` meta-crate and `sqlx-macros` declare optional MySQL/SQLite packages. Cargo records optional packages in `Cargo.lock`; `cargo audit` scans the lockfile even though no enabled feature or target compiles them.
- Change: the workspace now depends on `sqlx-core` and `sqlx-postgres` directly. `sqlx-core` uses `_rt-tokio`; `sqlx-postgres` uses `chrono` and `json`. Three `sqlx::FromRow` derives are explicit implementations. Query builders and shared traits come from `sqlx_core`; PostgreSQL types come from `sqlx_postgres`.
- Lockfile: `Cargo.lock` was regenerated by Cargo and no longer contains `rsa`, `sqlx`, `sqlx-mysql`, `sqlx-macros`, `sqlx-macros-core`, or `sqlx-sqlite`; 191 packages remain, down from 220.
- Policy: `deny.toml` remains `ignore = []`; `--locked` remains in every gate; no CI step was changed.
- Local evidence: `cargo audit` 0.22.2 PASS (no vulnerabilities); `cargo-deny` 0.20.2 PASS; format, locked all-target/all-feature check and clippy, 217 library tests, the non-PostgreSQL integration suites, all-target test compilation, and release build PASS. PostgreSQL execution remains NOT RUN locally; exact-head CI at `862775c` passed the PostgreSQL suites.
- Residual: exact-head `verify` and `advisories` passed at `862775c`. The pre-existing CI `rg` gap in `check-architecture`/`check-phase2-policy` is recorded in Stage 06 and not changed here.
