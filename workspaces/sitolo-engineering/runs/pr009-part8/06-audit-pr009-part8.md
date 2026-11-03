# Stage 06 Audit — PR-009 Part 8

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
