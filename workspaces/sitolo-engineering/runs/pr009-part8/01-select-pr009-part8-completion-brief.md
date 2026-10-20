# Run Brief — PR-009 Part 8 Completion

**Stage:** 01 Select
**Date:** 2026-10-20
**Classification:** feature / security / persistence-boundary remediation
**Branch:** `feat/phase4-pr009-audit-outbox` (PR #49)
**Base:** `aac53bf` — "feat(tenancy): wire audit+outbox evidence into remaining IAM use cases (step 2/N)"
**Main baseline:** `d2dbcce`
**Worktree:** `D:\jelome\adifferentwrld\project1\sitolo-pr009`
**Binding sources:**
- `docs/current_implementation_status.md` — active stream is Phase 4 Part 8 / PR-009
- `docs/phase4_part8_audit_outbox_implementation_contract.md` — binding Part 8 contract
- `docs/phase4_part8_pr009_remediation_execution_contract.md` — re-audit execution contract (§22 order, §23 DoD)
- `docs/phase4_part8_pr009_current_status.md` — current review snapshot (covers head `49c9399`, stale vs `aac53bf`)
- `agent.md` §2, §4, §5, §6, §24, §27, §28, §31, §32, §35, §36

## Request

Complete the active Phase 4 Part 8 / PR-009 audit + transactional outbox stream against the binding contract and its closure gates. Leave the PR open for human review and merge.

The request is not "write more code". The target is executable proof of the Part 8 invariants on the exact implementation head.

## Business capability

Durable, transactionally coupled audit evidence and outbox intent for Phase 4 IAM commands, plus a bounded worker relay that delivers committed events without becoming a second source of business truth.

## Affected bounded contexts

| Context | Location |
|---|---|
| Tenancy / IAM application | `crates/sitolo-application/src/tenancy.rs` |
| Audit | `crates/sitolo-audit/` |
| Events / outbox semantics | `crates/sitolo-events/` |
| Persistence | `crates/sitolo-persistence/` (repositories + PostgreSQL fixtures/tests) |
| API edge | `crates/sitolo-api/src/tenancy.rs` |
| Worker runtime | `apps/worker/` |
| Database contract | `crates/sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql` |

## Scope

In scope:

1. Repair the red baseline and keep locked builds green (contract §22 steps 1–2).
2. SECURITY DEFINER ownership, `search_path`, and `EXECUTE` grant hardening (step 3).
3. Explicit worker DB identity and privilege tests (step 4).
4. Claim token/generation and stale-worker rejection (step 5).
5. Concurrency-safe aggregate ordering, replacing `MAX()+1`, with a uniqueness constraint (steps 6–7).
6. Bounded worker controls and a real retry/backoff/quarantine policy (step 8).
7. Complete the authorized business + audit + outbox transaction for the selected Phase 4 command (step 9; partially present).
8. Narrow worker relay (step 10).
9. Atomicity, failure-injection, duplicate-delivery, and idempotent-consumer tests (steps 11–12).
10. Event registry, schema/version enforcement, bounded audit metadata (step 13).
11. Worker/audit observability and operational runbooks (step 14).
12. Canonical CI/evidence on the exact head (step 15).

Explicit non-goals (binding contract §3):

- generalized Phase 6 authorization engine;
- cache/versioning implementation;
- device binding or ownership transfer;
- support/admin separation;
- complete business-domain event catalogue;
- payment provider, MRA/EIS, or general worker/job platform;
- Kafka/RabbitMQ, event sourcing, CQRS, microservice extraction;
- any later-phase feature under the audit/outbox label;
- unrelated refactors, dependency churn, or policy relaxation;
- merging the PR.

## Acceptance criteria

The run is complete only when every item below is true on the exact head:

**P0**
- [ ] `cargo check --locked` passes.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` passes.
- [ ] Workspace tests pass.
- [ ] `./scripts/ci/verify` passes.
- [ ] A real business mutation is coupled to audit and outbox in one PostgreSQL transaction.
- [ ] Mandatory audit failure rolls back business state (failure-injection test).
- [ ] Outbox failure rolls back business state (failure-injection test).
- [ ] Worker runtime exists and is not the Phase 1 empty scaffold.
- [ ] SECURITY DEFINER functions are safely owned; `PUBLIC` execute is revoked.
- [ ] A stale worker cannot finalize a newer claim.
- [ ] Aggregate ordering is concurrency-safe.
- [ ] Persistence errors are never swallowed.
- [ ] Worker credentials are explicit and least-privileged.
- [ ] `SET LOCAL` tenant tests use real transactions.
- [ ] Integration schemas/functions are isolated per test.

**P1**
- [ ] Retry/backoff/quarantine is executable policy, not comments.
- [ ] Duplicate delivery reaches an idempotent consumer.
- [ ] Event registry/version/schema boundary exists.
- [ ] Audit metadata and field bounds exist.
- [ ] Worker/audit observability exists.
- [ ] Operational runbooks exist.
- [ ] Exact test evidence is recorded.

## Baseline (recorded before feature work)

At `aac53bf`:

- `cargo metadata --locked` — **FAILED**: `crates/sitolo-persistence/Cargo.toml` inherits `chrono` from `[workspace.dependencies]`, which does not declare it.
- Baseline repair applied in this run: `chrono = "0.4"` (`std`, no default features) added to `[workspace.dependencies]`; `sqlx` `chrono` and `json` features enabled; `sitolo-domain` declared as a workspace dependency and added to `sitolo-audit`; `Cargo.lock` updated.
- `cargo check --workspace --all-targets --locked` — **PASS** (after repair).
- `cargo fmt --all -- --check` — **PASS** (after repair).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — **PASS** (after repair).
- Workspace tests and `scripts/ci/verify` — **NOT RUN**.
- PostgreSQL availability and `DATABASE_URL` — **UNVERIFIED**.

The PR head was red before this run: `aac53bf` and the reviewed head `49c9399` both referenced `chrono` without a workspace declaration. No prior claim of a green PR build is reliable.

A pre-existing red baseline is not a regression from this run. No feature claim may be made while any P0 gate is red.

## External-evidence requirement

- Real PostgreSQL is required for persistence, RLS, SECURITY DEFINER, transaction, and duplicate-delivery evidence. Mock-only evidence is invalid for those claims.
- Canonical CI (`scripts/ci/verify` and the GitHub required checks) must pass on the exact head before merge readiness is claimed.
- Every evidence row records command, environment, exit status, and test case; missing evidence is `NOT RUN` or `BLOCKED`, never assumed.

## Known constraints

- Toolchain is pinned to Rust `1.98.1`; no floating channel.
- The working branch already contains partial implementation and a stale review snapshot.
- `agent.md` is the engineering authority; ICM workspace stages govern artifacts.
- No `#[allow]`/gate weakening, no removal of tests, no advisory suppression without a recorded exception.
- Never merge; a human owns merge (BP-GT-01, IMP-00-B).
- Secrets must not enter logs, fixtures, reports, or evidence.

## Unresolved decisions

1. **Worker relay destination.** The requester replied "main and sub to be specific" on 2026-10-20. That phrase does not identify a transport, destination endpoint/topic, protocol, or runtime configuration contract in the repository. Clarification is still required before implementing the relay; do not infer what "main" and "sub" mean.
2. **Scope of the first implementation slice.** The requester explicitly selected a PostgreSQL-backed tenancy mutation to satisfy full business + audit + outbox atomicity on 2026-10-20. Stage 04 must split the remaining §22 sequence into reviewable slices while preserving that acceptance requirement. No atomicity scope-out is approved.
3. **PostgreSQL verification environment.** The requester cannot provide an instance or the required database URLs. A real PostgreSQL environment remains necessary for verification; no in-memory substitute can prove RLS, privilege, transaction, or concurrency behavior.

## Security applicability

`security_relevant: yes`

Rationale: the change crosses tenant scope, database privilege boundaries, audit evidence integrity, outbox delivery/idempotency, and transaction rollback semantics.

Affected security boundaries:

- transaction-local tenant context and RLS;
- SECURITY DEFINER ownership/`search_path`/grants;
- application vs worker database identities;
- audit append-only integrity and PII minimization;
- outbox duplicate delivery, retry, quarantine, and ordering;
- worker external I/O and bounded resource use.

Candidate control IDs:

- **SC-002** — server-derived tenant/org/branch scope;
- **SC-006** — financial/inventory mutation authorization, transaction integrity, idempotency, durable evidence;
- **SC-009** — durable audit evidence distinct from telemetry;
- **SC-010** — resource limits and fail-closed behavior.

Required security skill: `workspaces/sitolo-engineering/skills/security-engineering/SKILL.md` (defensive). Offensive testing is out of scope for this run unless the audit stage explicitly authorizes a target.

## Human check

Confirmed by the requester on 2026-10-20 when directing continuation from Stage 01 and selecting the PostgreSQL-backed tenancy mutation:

1. Scope is limited to PR-009 Part 8 closure (not later phases).
2. The existing chrono + SQLx-feature baseline repair remains part of this run.
3. The build/security-definer/claim-order P0 cluster remains required; the requester additionally requires the real business + audit + outbox atomicity implementation.

No later stage may silently expand scope or change the security applicability decision.

---

## Run results — 2026-10-20 (implementation head, unmerged)

This section records what was actually implemented and verified in the isolated
worktree. It does not convert blocked evidence into a claim.

### Delivered

**Baseline repair**

- Declared `chrono` in root `[workspace.dependencies]` with `std`, no default
  features.
- Enabled `sqlx` `chrono` and `json` features.
- Declared `sitolo-domain` as a workspace dependency and added it to
  `sitolo-audit`.
- Regenerated `Cargo.lock`; see lockfile note below.
- Removed the now-empty `sitolo-persistence` `test-support` feature after making
  the PostgreSQL persistence module public unconditionally.

**Event/outbox model (`crates/sitolo-events`)**

- Added `OutboxClaimPolicy` with bounded batch size, lease, and attempt budget.
- Added `ClaimToken` with redacting `Debug`, `OutboxErrorClass`, `RetryClass`,
  and explicit bound constants.
- Added database-assigned `aggregate_sequence`, `claim_token`, and
  `claim_generation` to `OutboxEvent`.
- Added `OutboxEvent::is_fresh()` so enqueue can reject forged or partially
  transitioned state.
- Removed duplicate unused `OutboxWriter`/`OutboxReader` traits and the
  `async-trait` dependency from this crate.

**PostgreSQL persistence (`crates/sitolo-persistence`)**

- `PostgresOutboxRepository` assigns aggregate sequences through
  `outbox_aggregate_sequences` with `ON CONFLICT DO UPDATE` (no `MAX()+1` race,
  no swallowed errors).
- Every final transition is fenced by the claim token; stale workers receive
  `ClaimLost`.
- Repository enforces policy bounds before SQL and uses checked integer
  conversions. Explicit column lists and persisted-row invariant checks are in
  place.
- `PostgresAuditWriter::new()` is stateless; the unused pool field is gone.
- `postgres_time.rs` converts `SystemTime` to/from `chrono::DateTime<Utc>`
  without panicking; out-of-range persisted timestamps return
  `InvalidTimestamp` instead of aborting a worker.
- Audit event-version binding uses checked conversion, not `as i32`.

**Real-PostgreSQL fixture and harness**

- Dedicated non-login `app_outbox_owner` owns the tables, functions, and metrics
  view.
- All five SECURITY DEFINER functions pin `search_path` to `pg_catalog` plus the
  per-test schema, revoke `PUBLIC` execute, and grant execute only to
  `app_worker`.
- `app_runtime` and `app_worker` are `NOLOGIN`, `NOINHERIT`, `NOREPLICATION`,
  `NOSUPERUSER`, `NOBYPASSRLS`; role creation is race-tolerant.
- Evidence tables force RLS. Runtime inserts are constrained to the
  transaction-local organization context. Runtime has no audit/outbox SELECT,
  UPDATE, DELETE, or TRUNCATE privilege. Worker has no table SELECT; it receives
  only the five transition functions.
- Claim token/generation, unique aggregate sequence, checked status/error
  classes, and an owner-rights bounded `outbox_metrics` view are present.
- Same-aggregate claim order is head-of-line: an event is eligible only when
  every earlier sequence for that aggregate is `PUBLISHED`. A quarantined
  predecessor blocks its aggregate until operator resolution.
- The harness requires distinct `ADMIN_DATABASE_URL` (or `DATABASE_URL`),
  `RUNTIME_DATABASE_URL`, and `WORKER_DATABASE_URL`; it sets role and
  `search_path` per pool, executes `SET ROLE`, and asserts `SELECT current_user`
  for both runtime and worker.

**Tests added or strengthened**

- Atomicity/failure injection for audit and outbox writes, and the success
  commit path.
- Transaction-scoped tenant context tests (FAIL CLOSED on missing/mismatched
  organization).
- Runtime update/delete/read/truncate denial; cross-tenant outbox insert denial.
- Concurrent worker claim; lease recovery; stale-token fencing after recovery.
- Retry release/backoff/fencing; terminal quarantine; attempt-budget exhaustion.
- Concurrent aggregate-sequence assignment; same-aggregate claim ordering.
- Worker SELECT denial; runtime worker-function denial; bounded metrics view.
- Catalog proof for function owner, `SECURITY DEFINER`, pinned `search_path`,
  explicit ACL, `PUBLIC` execute denial, role attributes, and role memberships.
- Catalog proof for relation ownership, RLS enabled/forced, and exact policy
  roles.
- The old `test_duplicate_delivery_idempotency` was renamed to
  `test_published_event_is_not_reclaimable`; it proves terminality only. The
  duplicate-delivery/idempotent-consumer requirement remains blocked.

### Verification evidence

All commands below were run in
`D:\jelome\adifferentwrld\project1\sitolo-pr009` on the implementation head.
Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`,
`cargo 1.98.1 (797e8a9bc 2026-08-05)`.

| Command | Result |
|---|---|
| `cargo metadata --format-version 1 --locked` | PASS (lockfile current) |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --lib --locked` | PASS, 209 tests |
| `cargo test -p sitolo-persistence --test part8_audit_outbox_tests --locked` | PASS, 5 tests |
| `cargo test -p sitolo-api-bin --test tenancy_api --test startup --locked` | PASS, 7 + 4 tests |
| `cargo test -p sitolo-security --test provider_env --locked` | PASS, 4 tests |
| `cargo test --workspace --all-targets --locked --no-run` | PASS (all test targets compile, including PostgreSQL-only tests and the worker binary) |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` | PASS |
| `cargo build --workspace --release --locked` | PASS |
| `cargo test -p sitolo-persistence --test part8_postgres_integration_tests --locked` | **NOT RUN — BLOCKED** (no PostgreSQL executable/service and no DB URLs) |
| `cargo test -p sitolo-application --test part8_provision_organization_evidence_tests --locked` | **NOT RUN — BLOCKED** (same) |
| `./scripts/ci/verify` / `scripts/ci/verify.ps1` | **NOT RUN — BLOCKED** (PowerShell execution policy; `bash` and `rg` absent; PostgreSQL env absent) |
| `cargo-deny`, `cargo-audit`, `cargo-vet`, `cargo-nextest` | **NOT RUN — BLOCKED** (tools not installed) |

**Do not treat the local green commands as proof of the PostgreSQL behavior
above.** The SQL fixture, RLS policies, SECURITY DEFINER catalog state, claim
ordering, and concurrency behavior are unverified until the integration suite
runs on real PostgreSQL.

### Lockfile note

`Cargo.lock` adds 510 lines and removes none. Most of the additions are the
`chrono` dependency and optional SQLx backend packages (`sqlx-mysql`,
`sqlx-sqlite`, `rsa`, `flume`, `libsqlite3-sys`, and their transitive
dependencies). They appear because SQLx's feature declarations contain weak
optional-backend references. The feature graph does not activate them:
`cargo tree -e features -i sqlx-mysql` prints nothing. They are locked but not
compiled.

### Blockers that require a decision or external environment

1. **Worker relay destination is unspecified.** `apps/worker/src/main.rs` is
   still the Phase 1 empty scaffold. Implementing a relay requires a real
   destination port/adapter and configuration contract. Do not invent one.
2. **No real business + audit + outbox transaction yet.** The tenancy store is
   in-memory (`TenancyDatabase` over a `Mutex`). `provision_organization`
   commits the business mutation first and then records audit/outbox evidence in
   a separate PostgreSQL transaction. The application evidence tests document
   this gap; they do not claim Test A atomicity. A PostgreSQL-backed tenancy
   mutation or an explicit scope decision is required.
3. **PostgreSQL is unavailable here.** No PostgreSQL server, Docker/Podman,
   `psql`, `pg_ctl`, or `ADMIN_DATABASE_URL`/`RUNTIME_DATABASE_URL`/
   `WORKER_DATABASE_URL`. The admin role must be able to create/alter roles,
   transfer ownership to `app_outbox_owner`, and grant privileges; the runtime
   and worker login roles must be allowed to `SET ROLE`.
4. **Worker-dependent P1 proof remains.** Duplicate delivery to an idempotent
   consumer, worker metrics, and worker runbooks cannot be completed without the
   relay destination. The event registry/version boundary is also not
   implemented; only `is_fresh` and the database name/version constraints exist.
5. **Canonical CI cannot run in this shell.** The required executable evidence
   must still be produced by the repository CI or a host with PowerShell script
   execution, `bash`, `rg`, PostgreSQL, and the supply-chain tools.

### Acceptance status on this head

**Met and locally verified (but not PostgreSQL-verified):** build, format,
clippy, library tests, audit/outbox type bounds, checked conversions,
concurrency-safe sequence allocation code, claim-token fencing code,
security-definer fixture hardening, explicit role identities, per-test schema
isolation, and the catalog/privilege tests (compiled, not run).

**Not met:** real business + audit + outbox transaction; audit/outbox failure
rolling back authoritative business state; worker runtime; duplicate delivery to
an idempotent consumer; event registry; complete observability/runbooks;
canonical CI on the exact head.

### Commit plan (no merge)

1. `feat(outbox): harden Part 8 audit and outbox persistence` — baseline
   timestamp/build repair, rustfmt required by the format gate, event model,
   repositories, PostgreSQL fixture, integration tests, application evidence
   fixture/tests, and manifests. This is one commit because the build repair
   requires the same timestamp-boundary conversion as the feature; a
   manifest-only commit does not compile.
2. `docs: record PR-009 Part 8 slice evidence` — this brief.

The branch remains unmerged. A human owns merge after the blockers and canonical
evidence are resolved.

## Continuation status — 2026-10-20

The requester restarted this run at Stage 01, selected a PostgreSQL-backed
tenancy mutation for full business/audit/outbox atomicity, cannot supply a
PostgreSQL server or database URLs, and directed continuation without needing
to open the research memo.

Stage 05 has now added a PostgreSQL organization provisioning path, tenancy
schema SQL, API runtime wiring to `app_runtime`, and real PostgreSQL tests for
success, audit rollback, outbox rollback, and same-payload retry. Local format,
locked workspace check, clippy, and library tests pass. The PostgreSQL
integration suite only compiles here; it has not run on a real server. CI is
configured to provide PostgreSQL 18 but has not run on these changes.

Still incomplete: worker relay destination (“main and sub” does not identify a
transport or endpoint/topic/queue/configuration), destination-dependent worker
delivery and idempotent-consumer proof, real PostgreSQL CI execution, complete
canonical gates, supply-chain advisory disposition, and independent Stage 06
audit. Other tenancy transitions still use the in-memory reference store; the
new durable path covers organization provisioning only. PR #49 remains open;
nothing has been merged.

### Exact-head follow-up — 2026-10-20

The assertion helper fix was committed and pushed as
`1383233691ec2aa0b294ea0311d8143836ca0d19`. On that exact commit, GitHub
integration run 779 passed all four organization-provisioning tests against
PostgreSQL, including audit/outbox rollback and retry/conflict behavior. The policy workflow passed. The canonical Rust verifier passed the application
PostgreSQL cases and all 23 Part 8 persistence tests, then failed the legacy
RLS suite because its runtime pool's `current_user` was `postgres`, not
`app_runtime`. Stage 07 added `SET ROLE app_runtime` on runtime-pool
connections; the exact-head rerun is pending. The security workflow's
dependency-policy, CodeQL, offensive-regression, and secret-scanning jobs
passed, while its advisory job failed on `RUSTSEC-2023-0071` for inactive
locked `rsa 0.9.10` (RustSec lists no patched release). The run remains
incomplete and PR #49 remains open.

