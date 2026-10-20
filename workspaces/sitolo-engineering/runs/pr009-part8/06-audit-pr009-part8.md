# Stage 06 Audit — PR-009 Part 8

**Disposition:** Re-audit after Stage 07 remediation. The newly introduced durable API wiring is removed. Other material requirements still lack implementation or proof, so Stage 08 is not cleared.
**Review basis:** implementation diff and code paths, Stage 01 brief, Stage 02 research, Stage 04 plan, Stage 05 report, security controls SC-002, SC-006, SC-009, SC-010, and audit/security/correctness contracts.
**Human check:** The requester authorized autonomous continuation after saying the file was not visible. This is not recorded as manual review or acceptance of findings.

## Requirement trace

| Requirement / control | Actual path | Evidence | Disposition |
|---|---|---|---|
| Provision organization, owner membership, default branch, audit, and outbox atomically (SC-006, SC-009) | `TenancyService::provision_organization` uses one caller-owned SQLx transaction when evidence sink is configured; inserts bundle, audit, outbox, then commits | PostgreSQL application integration suite passed on `1383233691ec2aa0b294ea0311d8143836ca0d19` (run 779): success and injected audit/outbox rollback tests all passed | PARTIAL — application transaction behavior is proven; canonical run 629 failed at the later legacy RLS identity precondition, and the runtime-role fix awaits rerun |
| Idempotent provisioning and divergent ID conflict | Persistence insert treats a matching full bundle as retry and conflicts on mismatch | PostgreSQL application integration suite passed on exact commit; same-payload retry/divergence case passed | PARTIAL — concurrent same-ID race remains unverified; Part 8 persistence tests passed, but the legacy RLS suite's identity fix awaits rerun |
| Tenant bootstrap scope is server-derived, never caller authority (SC-002) | The API process now retains its in-memory reference path. The PostgreSQL transaction path exists only when a trusted application composition injects the evidence sink; the HTTP router still has no principal and passes request `owner_user_id` | Direct source inspection of `apps/api/src/serve.rs`, `crates/sitolo-api/src/tenancy.rs`, `apps/api/src/state.rs`, and `crates/sitolo-application/src/tenancy.rs`; targeted API tests pass but do not exercise authenticated authorization | PARTIAL — this change no longer amplifies the existing unauthenticated route into durable state, but planned production API integration is absent until authenticated authority exists |
| Database schema/role/RLS denies unauthorized paths (SC-002, SC-010) | New deployable SQL applies forced RLS and grants runtime SELECT/INSERT | Part 8 PostgreSQL suite passed 23 tests on `1383233`; the canonical legacy RLS suite attempted the real DB and failed its runtime identity precondition (`current_user=postgres`, expected `app_runtime`) | PARTIAL — Stage 07 now applies `SET ROLE app_runtime` on runtime-pool connections; full RLS re-run is required |
| Relay uses approved transport, destination, payload, and acknowledgement semantics | No destination adapter/runtime config in worker; requester answered “main and sub to be specific” | Repository evidence did not identify a contract; no endpoint/queue/topic or runtime field set supplied | FAIL — destination-dependent delivery, idempotent consumer proof, and operator evidence cannot be implemented without inventing semantics |
| CI verifies PostgreSQL and exact-head gates | Integration and Rust workflows provide a PostgreSQL service and admin/runtime/worker URL variables | Pushed `1223ed3` integration job failed because URLs were absent; current workflow fix has not run on this revision | PARTIAL |
| Supply-chain gates pass (SC-011) | Lockfile updates yoke-derive; RSA 0.9.10 is in lockfile | Exact-head old audit reports RUSTSEC-2023-0071; all-features/target Cargo tree has no active RSA path; current cargo-deny toolchain workaround is unverified | FAIL — audit gate and dependency-policy evidence unresolved |

## Findings

### F-01 — Durable API provisioning awaits authenticated authority

**Severity:** Release-blocking contract/security gap (SC-002). The HTTP server dispatches organization creation without authentication context. Its DTO contains a caller-supplied owner ID; comments explicitly defer principal-derived identity to PR-007. Stage 05 had briefly wired this route to durable PostgreSQL state, which would have persisted caller-controlled authority. Stage 07 removed that wiring. The application transaction capability remains available to a future authenticated caller. The current HTTP tenancy API remains an unauthenticated/in-memory reference; the planned production API integration is therefore still incomplete.

**Re-audit result:** The unsafe new durable wiring is removed. No newly added HTTP database path remains. Residual closure requires authenticated principal integration, server-derived owner/actor/tenant authority, and exercising the real request boundary before durable API wiring is restored.

### F-02 — PostgreSQL behavior is unverified

The requester cannot provide a local PostgreSQL instance/URLs. The pushed `dca49f8` integration job reached PostgreSQL but raced while creating cluster-global fixture roles; the follow-up lock fixed that race. The `561bbe9` run then exposed an app test helper querying `organization_id` on the `organizations` table, which uses `id`. Commit `1383233` corrected the helper, and integration run 779 passed all four application provisioning tests against PostgreSQL, including success, audit rollback, outbox rollback, idempotent retry, and divergent conflict. The canonical verifier on `1383233` also passed 23 Part 8 persistence tests, but the separate legacy RLS suite failed before its test bodies because its runtime pool had `current_user=postgres` instead of `app_runtime`. Stage 07 added `SET ROLE app_runtime` on runtime-pool connections; that correction requires a fresh exact-head run. Do not generalize app/Part 8 evidence to the unpassed legacy RLS suite.

### F-03 — Worker relay contract is unresolved

“main and sub” does not establish transport, endpoint/topic/queue names, payload/version contract, acknowledgement semantics, configuration keys, or operator ownership. No relay destination was inferred. Destination-specific implementation and delivery/idempotent-consumer evidence remain blocked.

### F-04 — Required dependency policy status is unresolved

The pushed head's `cargo audit` reports `RUSTSEC-2023-0071` for `rsa 0.9.10` (Marvin timing attack); RustSec lists no patched release. The current all-features/target Cargo tree has no active path to RSA, but the lockfile scanner still reports it and there is no approved exception. Do not suppress it. The pinned cargo-deny action passes on the exact `1383233` security run with the Rust 1.85.0 toolchain override and non-yanked yoke-derive lock resolution. The separate advisory job still fails; resolve through dependency remediation or an authorized governance disposition, not by weakening the gate.

## CI evidence reviewed

For pushed SHA `1223ed3aef3818f56c5b444ee22fff287546349f`, GitHub reported:

- The original pushed head's policy run rejected reports inside reusable output shelves. The later `dca49f8` policy run passed ICM, PowerShell parity, and System Map checks, then found six archived-document links still pointing into those shelves. The links are fixed locally; `python scripts/ci/check-doc-references` passes with 0 unresolved references.
- The original integration job omitted database URLs. `dca49f8` correctly started PostgreSQL and exposed the shared-role setup race; `561bbe9` confirmed the fixture lock removed it and surfaced an incorrect organization count query. Integration run 779 on `1383233` passed all four app provisioning tests against PostgreSQL after the assertion fix.
- `dca49f8` dependency-policy passed with the pinned cargo-deny action and toolchain override. The advisory job still fails for RUSTSEC-2023-0071. CodeQL/offensive-regression were in progress in the last snapshot; secret scanning passed.
- On `1383233`, the canonical verifier reached all-target tests and the PostgreSQL suites: the application provisioning tests and 23 Part 8 persistence tests passed; the legacy RLS suite failed its runtime identity precondition because its pool connected as `postgres`. Stage 07 added `SET ROLE app_runtime` to the runtime-pool connection path; exact-head rerun is pending. Cargo audit fails on `RUSTSEC-2023-0071`; exact-head cargo-deny passes.
- Policy run 483 on `0b85158` failed the System Map process check because the PostgreSQL authority card still cited the pre-change source revision. The process card and persistence object card now cite `0b85158`, describe the role assumption, and the generated object index has been refreshed. Local map verification passes; the policy rerun is pending.

## Audit conclusion

The application transaction and Part 8 persistence behavior now have exact-head PostgreSQL evidence, but the legacy RLS suite failed its runtime identity check and has a local correction awaiting rerun. The planned production API integration is incomplete, relay semantics are absent, and cargo-audit remains blocked by the upstream-unpatched RSA advisory. PR-009 Part 8 is not merge-ready. Stage 08 remains uncleared pending these open findings and exact-head evidence.
