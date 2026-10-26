# Implementation Report — PR-009 Part 8

**Stage:** 05 Implement\
**Date:** 2026-10-20\
**Source before changes:** `1223ed3aef3818f56c5b444ee22fff287546349f`\
**Base commit:** `dd583b24a4f548437b368d59e91fe4e438dc624a`; worker engine commit: `2048c8a6a2a216d466240e1e1a1f5f99dae111d7` (included in pushed exact head `862775c`)\
**Goal:** PostgreSQL transaction for organization + owner membership + default branch + audit + outbox; repair database-test wiring; keep PR open.

## Implemented

- Added `crates/sitolo-persistence/src/tenancy_postgres.rs`, which inserts a validated provisioning bundle inside a caller-owned PostgreSQL transaction. It treats a complete same-payload existing bundle as a retry and rejects divergent/pre-existing partial rows.
- Updated `TenancyService::provision_organization`: when the durable evidence sink is configured, it constructs the domain bundle, applies transaction-local organization scope, inserts the three business rows, writes mandatory audit and outbox evidence through the same transaction, and commits once. A matching retry returns without adding another semantic event. The in-memory reference path remains for non-durable tests.
- Added `crates/sitolo-persistence/schema/phase4_part8_tenancy_provisioning.sql` and schema README. The schema creates bounded tenancy tables, forced RLS, scoped runtime read/insert policies, and only the required runtime grants. Administrative deployment is required before runtime startup; the API never creates schema objects.
- API process wiring was initially added, then removed during audit remediation because the live HTTP provisioning route has no authenticated principal and accepts caller-provided `owner_user_id`. The durable application transaction capability remains, but the API must wait for authenticated, server-derived identity before it can safely be wired to PostgreSQL.
- Reworked the application PostgreSQL integration fixture to use the full Part 8 role/RLS schema plus the tenancy schema artifact. Tests now assert a complete bundle and evidence on success; outbox and audit failures each leave no business rows; same-payload retry emits one evidence pair and divergent reuse conflicts.
- Updated Rust and integration GitHub workflows to provide PostgreSQL 18 and required database URLs to suites that include PostgreSQL-backed tests. The runtime and worker pools connect with the PostgreSQL service login and assume their `NOLOGIN` test roles.
- Updated the locked `yoke-derive` resolution from yanked `0.8.3` to non-yanked `0.8.4`.

## Verification

| Command / evidence | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo metadata --format-version 1 --locked` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS; integration targets compile |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --lib --locked` | PASS (workspace library tests) |
| `cargo test -p sitolo-application --test part8_provision_organization_evidence_tests --locked --no-run` | PASS (test target compiles) |
| `cargo test -p sitolo-api-bin --test startup --test tenancy_api` (after audit remediation) | PASS (11 tests) |
| `cargo fmt --all -- --check` (after audit remediation) | PASS |
| `cargo check --workspace --all-targets --locked` (after audit remediation) | PASS |
| Real PostgreSQL application integration suite | NOT RUN locally: no PostgreSQL server or database URLs are available. Exact-head CI at `862775c` passed the integration job and the canonical verifier's PostgreSQL suites. |
| Full canonical verification / exact-head CI | PASS at `862775c`: `rust` run 39508142399 (`verify` job 8m2s), `security` run 39508142385, `policy` run 39508142377, and `integration` run 39508142396. |
| `cargo audit`, `cargo deny check` | Initially NOT RUN locally because `cargo-audit` installation failed on the full C: volume. After the 2026-10-21 PostgreSQL-only SQLx remediation: `cargo-audit` 0.22.2 PASS (191 dependencies, no vulnerabilities) and `cargo-deny` 0.20.2 PASS (advisories, bans, licenses, sources). The prior locked `rsa 0.9.10` advisory is no longer in `Cargo.lock`; exact-head `verify` and `advisories` passed at `862775c`. |

The unit/build checks do not by themselves prove RLS, role, SQL transaction, or rollback behavior. Those claims now have exact-head PostgreSQL CI evidence at `862775c`; the local host still could not execute them.

## Remaining gates

- The worker relay destination remains undefined. “Main and sub” still does not identify a transport, endpoints/topics/queues, event delivery contract, or runtime fields. No relay target has been invented.
- Destination-dependent relay, idempotent consumer delivery, worker runtime completion, and associated operator evidence remain incomplete.
- Application PostgreSQL CI passed on `1383233`; the persistence/RLS suite and its catalog, privilege, and concurrency assertions passed in the canonical verifier at `862775c` (23 Part 8 tests, 27 RLS tests in-suite and standalone).
- RSA advisory disposition: the PostgreSQL-only SQLx remediation removes `rsa` from `Cargo.lock`; exact-head `verify` and `advisories` passed at `862775c`. See the 2026-10-21 section.
- Other tenancy transitions besides organization provisioning still use the in-memory reference store; this implementation does not claim their business mutations are PostgreSQL-backed.

## Human check

The requester directed continuation (“do what you have to to make it all work”) after saying the prior memo was not visible. This authorized implementation to proceed without manual file review; it is not a claim that the requester reviewed this diff. Do not merge the PR.

## Current remediation increment — 2026-10-25

Implementation source commits for this increment: `468186c` adds domain-validated shared tenancy fixtures consumed by tenancy and application tests; `4858341` removes the organization activation route that had no persisted `PROVISIONING` transition, updates the active API contract, and adds a 404 boundary regression. `map/objects/verification/test-infrastructure.md` and `map/processes/http-transport-processing.md` were updated only after source consumers and route behavior existed. No persistence fake or synthetic success path was added. The current exact-head workflow evidence and remaining blockers are recorded in the dated follow-up execution report and requirements matrix.

## Exact-head PostgreSQL follow-up — 2026-10-20

Commit `1383233691ec2aa0b294ea0311d8143836ca0d19` fixes the organization assertion helper to query `organizations.id`. GitHub integration run 779 passed on this exact commit. All four `part8_provision_organization_evidence_tests` passed against PostgreSQL, including successful provisioning, audit-failure rollback, outbox-failure rollback, same-payload retry, and divergent-ID conflict. The separate persistence/RLS suite under the canonical verifier had not yet completed at that point; it later passed at `862775c`. These application tests do not prove its role/catalog/concurrency checks by themselves. The requester has no local PostgreSQL instance, which remains true.

Canonical verifier run 629 then passed all 23 Part 8 PostgreSQL persistence tests but failed the legacy `rls_security_tests`: `PgAuthorityPools` opened the runtime pool as `postgres` and did not assume `app_runtime`, so every test failed its explicit identity precondition before exercising its body. The local remediation adds `SET ROLE app_runtime` in the runtime pool's `after_connect` hook. Local format, all-target check, clippy, and diff checks pass after that change; the exact-head rerun passed at `862775c`.

## Exact-head verification follow-up — 2026-10-20

On commit `1aad95cf03e98d63f610c7fe8bc268aae45b2e4c`, policy run 484 passed, application PostgreSQL integration run 781 passed, and canonical Rust run 631 passed the all-target test set, 23 Part 8 PostgreSQL tests, and the legacy RLS suite (27 tests, also passed in its explicit standalone invocation). The verifier then failed at `cargo audit` on `RUSTSEC-2023-0071`; all preceding canonical gates passed, including cargo-deny. The security workflow's dependency-policy, CodeQL, offensive-regression, and secrets jobs passed, while its advisory job failed on the same RSA finding.

A concurrent identical-provisioning test was added after that run to exercise two PostgreSQL transactions racing on the same organization ID.

## Exact-head follow-up — 2026-10-20

On `dd583b24a4f548437b368d59e91fe4e438dc624a`, integration run 782 passed the 5-test application evidence suite against PostgreSQL, including the concurrent identical-provisioning case. Canonical Rust run 632 passed that test, all 23 Part 8 persistence tests, and both invocations of the 27-test RLS suite. Policy run 485 passed. Canonical verification reached `cargo audit` and failed on `RUSTSEC-2023-0071` for locked `rsa 0.9.10`; security run 639 passed dependency-policy, CodeQL, offensive-regression, and secrets while advisories failed on the same finding. No local PostgreSQL was available; these PostgreSQL assertions are based on CI. The API durable wiring remains removed until a trusted authenticated principal is available. Worker relay and other tenancy mutations remain incomplete.

## Destination-independent worker engine — 2026-10-20 (`2048c8a`, local)

- Added `apps/worker/src/lib.rs` and `relay.rs` with a bounded claim loop, stale-claim recovery before each batch, a registry of the known IAM event names, version/schema-version agreement, and a JSON-object envelope check.
- A concrete publisher receives the stable `event_id` as the idempotency key. Retryable failures use capped exponential backoff; terminal/unknown failures and unsupported event envelopes are quarantined. Persistence errors propagate and stop processing. Batch totals are emitted as structured tracing fields.
- `apps/worker/src/main.rs` exits with a clear fail-closed error until the approved transport adapter and its configuration contract exist. No database connection, destination, topic/queue, or acknowledgement behavior has been invented.
- Local verification after this change: `cargo fmt --all -- --check`; `cargo test -p sitolo-worker-bin -p sitolo-audit --locked` (7 worker + 3 audit tests passed); `cargo clippy -p sitolo-worker-bin -p sitolo-audit --all-targets --locked -- -D warnings`; `cargo check --workspace --all-targets --locked`; and `git diff --check` all pass.
- This is a transport-neutral processing engine, not an operational relay. It does not prove consumer idempotency, event-specific JSON schemas, destination configuration, database pool composition, exported metrics, runbooks, or delivery under a real adapter. Exact-head CI for these files passed at `862775c`.

## Advisory remediation implementation — 2026-10-21

The RSA advisory is removed from the lockfile by construction:

- `Cargo.toml`: the `sqlx` meta-crate workspace dependency was replaced with `sqlx-core` (feature `_rt-tokio`) and `sqlx-postgres` (features `chrono`, `json`). The comment records why.
- `crates/sitolo-application/Cargo.toml` and `crates/sitolo-persistence/Cargo.toml`: replaced `sqlx.workspace = true` with the two backend crates.
- `crates/sitolo-persistence/src/outbox_repository.rs`: replaced `#[derive(sqlx::FromRow)]` with an explicit `FromRow<PgRow>` implementation for `OutboxEventRow`.
- `crates/sitolo-persistence/tests/part8_postgres_integration_tests.rs`: replaced the two `sqlx::FromRow` derives with explicit implementations for `FunctionSecurityRow` and `RelationSecurityRow`.
- Remaining SQLx imports now come from `sqlx_core` (query builders, `Error`, `FromRow`, `Row`, `Transaction`) and `sqlx_postgres` (`PgPool`, `PgRow`, `Postgres`, `PgConnectOptions`, `PgPoolOptions`). No `sqlx::query!`, migration, or other macro is used.
- `Cargo.lock` was regenerated by Cargo, not edited by hand. It dropped from 220 to 191 packages and no longer contains `rsa`, `sqlx`, `sqlx-mysql`, `sqlx-macros`, `sqlx-macros-core`, or `sqlx-sqlite`.

Local verification after this change:

| Command / evidence | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo metadata --format-version 1 --locked` | PASS |
| `cargo check --workspace --all-targets --all-features --locked` | PASS |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked --lib -j 1` | PASS (210 workspace library tests plus 7 worker tests) |
| `cargo test -p sitolo-api-bin --test startup --test tenancy_api --all-features --locked -j 1` | PASS (11 tests) |
| `cargo test -p sitolo-persistence --test part8_audit_outbox_tests --all-features --locked -j 1` | PASS (5 tests) |
| `cargo test -p sitolo-security --test provider_env --all-features --locked -j 1` | PASS (4 tests) |
| `cargo test --workspace --all-targets --all-features --locked --no-run -j 1` | PASS; every test target compiles, including the PostgreSQL integration and RLS targets |
| `cargo build --workspace --release --locked -j 1` | PASS |
| `cargo audit` 0.22.2 | PASS; 1348 advisories loaded, 191 dependencies scanned, no vulnerabilities |
| `cargo deny check` 0.20.2 | PASS; advisories ok, bans ok, licenses ok, sources ok |
| PostgreSQL integration and RLS execution | NOT RUN locally: no PostgreSQL server or database URLs are available. Compilation and prior CI evidence only. |

The explicit `FromRow` mappings use the same field names as the derives, so column lookup behavior is unchanged. `deny.toml` remains `ignore = []`; no advisory suppression, `--locked` removal, or gate weakening was applied.

## Follow-up implementation update — 2026-10-22 — source `8eaaf18530027a3fdbb854361a1aa4a0ba580e30`

Fixed two PostgreSQL invitation-acceptance integration fixtures to meet the domain token-length invariant. Added `invitation_accept_withdraw_and_expiry_race_commits_one_terminal_transition`, which runs acceptance, withdrawal, and the bounded expiry sweep concurrently on an expired invitation and asserts one terminal winner, rejection of acceptance, terminal PII deletion, and one matching invitation event. The owner-approved `INVITED → EXPIRED` membership event remains schema-validated and emitted only when the sweep changes the membership row.

Local checks passed: `cargo fmt --all`, `cargo check --workspace --all-targets --all-features --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, integration target `--no-run`, and `git diff --check`. No local PostgreSQL was started. Exact-head PostgreSQL, Rust, policy, security, and CodeQL checks passed for `8eaaf18`; see the follow-up remediation report for run URLs. The race test passed in the PostgreSQL workflow.

AUD-04 is implemented but remains partial pending Stage 06 re-audit. AUD-03 remains open pending confirmation that every mandatory IAM mutation has same-transaction state/audit/outbox coverage and server-derived actor/tenant authority. Protected live HTTP writes remain fail-closed. PR #49 remains open and unmerged.

## Current implementation increment — authenticated session logout, 2026-10-26

Implementation commit `382de95eeb8f9f0622d022f304feb7dd7a5db802` adds the authenticated `POST /v1/auth/logout` route and migration 0006. The session identifier and user are derived from the verified security context. The PostgreSQL function revokes only the matching active session and inserts `auth.session.revoked` in the same transaction; runtime startup and readiness require that function. Focused API coverage verifies logout revokes the current session and subsequent protected requests fail. PostgreSQL tests cover wrong-user no-write, concurrent/replayed idempotency and audit-failure rollback.

Exact-head CI on `382de95` exposed the stale protected-route count and stale System Map source pins. The route count is corrected in `2b5e05fde370ecaca971290a5f014aee165c8402`. Verification on documentation head `4ec9cd19e1a863942921c1822daa45ea034a32d3` then exposed an invalid backslash in the PostgreSQL audit-failure trigger SQL (`verify` run [40309032901](https://github.com/jelome265/sitolo/actions/runs/40309032901)); the failure was reproduced by disposable PostgreSQL CI. Commit `5b4563bdd5107008f44a30e9f3516f967f81a492` fixes the SQL string continuation. All required exact-head workflows pass on `5b4563b`: Rust [40309475692](https://github.com/jelome265/sitolo/actions/runs/40309475692), Integration [40309475680](https://github.com/jelome265/sitolo/actions/runs/40309475680), Security [40309475702](https://github.com/jelome265/sitolo/actions/runs/40309475702), Policy [40309475681](https://github.com/jelome265/sitolo/actions/runs/40309475681), and CodeQL [121529026690](https://github.com/jelome265/sitolo/runs/121529026690). The logout PostgreSQL owner-binding, replay/concurrency and audit-rollback regressions passed in CI; no local PostgreSQL was started. This increment does not complete Phase 3, the nine-workstream remediation, Stage 06 audit or human review.

## PostgreSQL rollback coverage addendum — `96ca56b08804211895106612fb5b393e2d66e686`

Added failure-injection tests for direct branch creation, direct member invitation, and scope-grant revocation. Each test runs against the PostgreSQL workflow, injects both audit and outbox failures, and checks database state/evidence rollback. The exact-head CI result is recorded in `follow-up/remediation-report.md`; all nine checks passed, with no local PostgreSQL execution.

AUD-03 remains open pending full transaction/actor-authority re-audit and live API authority composition. AUD-04 remains partial pending Stage 06 re-audit. PR #49 remains open and unmerged.

## PostgreSQL lifecycle fault-matrix addendum — `39cb7ddb9b07400873466a3b8d2ec8aa4fa9500a`

Added `every_organization_and_branch_transition_rolls_back_on_audit_and_outbox_failures` to the PostgreSQL transaction integration suite. It exercises all ten organization and branch lifecycle commands with audit-writer and outbox-writer failures, and checks the persisted state/version plus absence of partial audit/outbox rows. This extends the prior rollback test from one organization and one branch transition to every lifecycle command handled by the shared transaction paths.

Local verification: `cargo fmt --all -- --check`, `cargo test -p sitolo-application --test membership_transaction_integration --no-run --locked`, `cargo clippy -p sitolo-application --test membership_transaction_integration --locked -- -D warnings`, and `git diff --check` passed. The test executed and passed in the exact-head PostgreSQL workflow; the membership transaction target reported 39 passed. The PostgreSQL workflow [39743712879](https://github.com/jelome265/sitolo/actions/runs/39743712879), Rust verification [39743712958](https://github.com/jelome265/sitolo/actions/runs/39743712958), security [39743712881](https://github.com/jelome265/sitolo/actions/runs/39743712881), and policy [39743712882](https://github.com/jelome265/sitolo/actions/runs/39743712882) all passed on this exact SHA, including CodeQL. The remaining PR check also passed. No local PostgreSQL was started or used.

This adds fault-path coverage but does not independently close AUD-03. The exact source must pass PostgreSQL CI and receive Stage 06 re-audit; AUD-02's production OIDC/session/authorization composition also remains open, and protected live HTTP writes remain fail-closed. PR #49 remains open and unmerged.

## Current implementation increment — durable session expiry, 2026-10-26

From source `49affa5a118339ad6056cb7f4d1520820d14ebc8`, this increment adds
an immutable follow-on migration that persists the Phase 3 `idle_expired` and
`absolute_expired` terminal states when a matching session is presented after
its deadline. The real PostgreSQL integration test now asserts both denial and
persisted terminal state. The persistence harness applies migration 0005 after
0004; the migration manifest pins its checksum. Local formatting, persistence
all-target check, integration-target compilation, strict persistence Clippy,
migration inventory and diff checks pass. The database test was not executed
locally; it passed in the approved disposable PostgreSQL CI service on exact
implementation source `0385aadd2697b9900e77eecd47ab0f71f886c66a`. Rust
[40298701887](https://github.com/jelome265/sitolo/actions/runs/40298701887),
Integration [40298701841](https://github.com/jelome265/sitolo/actions/runs/40298701841),
Security [40298701912](https://github.com/jelome265/sitolo/actions/runs/40298701912),
Policy [40298701810](https://github.com/jelome265/sitolo/actions/runs/40298701810),
and CodeQL [121493288342](https://github.com/jelome265/sitolo/runs/121493288342)
all passed on that exact source. See
`follow-up/phase3-session-expiry-remediation-2026-10-26.md`. This does not close
the broader Phase 3 lifecycle or independent Stage 06 review.

### Audit-driven correction — session expiry evidence

Source `0385aadd` passed its listed workflows, but subsequent contract review
found that its session expiry state transition lacked the required durable
`auth.session.expired` audit record (Phase 3 §§1, 37, 52). That source's CI
does not verify the corrected candidate. The current correction adds a
restricted identity audit table, writes the audit event atomically with expiry,
checks ownership and runtime privileges at startup/readiness, and adds
PostgreSQL evidence for audit failure rollback, replay cardinality and
wrong-user no-write behavior. Corrected implementation
`b7de7eb0bee7209b8526aa0017b2416efeea53fd` passed PostgreSQL-enabled
exact-head CI at documentation head `c387047306dfa580395eccc8cdb6de23a1a8b3a6`.
Rust [40301735355](https://github.com/jelome265/sitolo/actions/runs/40301735355)
executed both expiry regressions, including audit rollback; Integration
[40301735469](https://github.com/jelome265/sitolo/actions/runs/40301735469),
Security [40301735345](https://github.com/jelome265/sitolo/actions/runs/40301735345),
Policy [40301735372](https://github.com/jelome265/sitolo/actions/runs/40301735372),
and CodeQL [121503273264](https://github.com/jelome265/sitolo/runs/121503273264)
also passed on that head. See
`follow-up/phase3-session-expiry-audit-remediation-2026-10-26.md`.
