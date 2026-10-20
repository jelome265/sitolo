# Stage 06 Audit — PR-009 Part 8

**Disposition:** Re-audit after Stage 07 remediation. The newly introduced durable API wiring is removed. Other material requirements still lack implementation or proof, so Stage 08 is not cleared.
**Review basis:** implementation diff and code paths, Stage 01 brief, Stage 02 research, Stage 04 plan, Stage 05 report, security controls SC-002, SC-006, SC-009, SC-010, and audit/security/correctness contracts.
**Human check:** The requester authorized autonomous continuation after saying the file was not visible. This is not recorded as manual review or acceptance of findings.

## Requirement trace

| Requirement / control | Actual path | Evidence | Disposition |
|---|---|---|---|
| Provision organization, owner membership, default branch, audit, and outbox atomically (SC-006, SC-009) | `TenancyService::provision_organization` uses one caller-owned SQLx transaction when evidence sink is configured; inserts bundle, audit, outbox, then commits | Application integration run 781 and canonical Rust run 631 passed success, audit rollback, and outbox rollback tests against PostgreSQL on `1aad95c` | PASS for the tested application transaction path; production API composition remains incomplete under F-01 |
| Idempotent provisioning and divergent ID conflict | Persistence insert treats a matching full bundle as retry and conflicts on mismatch | Sequential retry/divergent-ID assertions passed in PostgreSQL integration on `1383233`; all persistence/RLS suites passed on `1aad95c` | PARTIAL — concurrent same-ID provisioning was not exercised on `1aad95c`; a two-request integration test is added locally and awaits exact-head PostgreSQL execution |
| Tenant bootstrap scope is server-derived, never caller authority (SC-002) | The API process now retains its in-memory reference path. The PostgreSQL transaction path exists only when a trusted application composition injects the evidence sink; the HTTP router still has no principal and passes request `owner_user_id` | Direct source inspection of `apps/api/src/serve.rs`, `crates/sitolo-api/src/tenancy.rs`, `apps/api/src/state.rs`, and `crates/sitolo-application/src/tenancy.rs`; targeted API tests pass but do not exercise authenticated authorization | PARTIAL — this change no longer amplifies the existing unauthenticated route into durable state, but planned production API integration is absent until authenticated authority exists |
| Database schema/role/RLS denies unauthorized paths (SC-002, SC-010) | New deployable SQL applies forced RLS and grants runtime SELECT/INSERT | On `1aad95c`, canonical Rust run 631 passed 23 Part 8 persistence tests and both invocations of the 27-test legacy RLS suite with the `app_runtime` identity asserted | PASS for the exercised schema, role, catalog, and RLS paths; this does not establish authenticated HTTP integration |
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

“main and sub” does not establish transport, endpoint/topic/queue names, payload/version contract, acknowledgement semantics, configuration keys, or operator ownership. No relay destination was inferred. Destination-specific implementation and delivery/idempotent-consumer evidence remain blocked.

### F-04 — Required dependency policy status is unresolved

The `1aad95c` `cargo audit` job reports `RUSTSEC-2023-0071` for locked `rsa 0.9.10` (Marvin timing attack); RustSec lists no patched release. The all-features/target Cargo tree has no active path to RSA, but the lockfile scanner still reports it and there is no approved exception. Do not suppress it. The pinned cargo-deny action passes on `1aad95c` with the Rust 1.85.0 toolchain override and non-yanked yoke-derive lock resolution. The separate advisory job still fails; resolve through dependency remediation or an authorized governance disposition, not by weakening the gate.

## CI evidence reviewed

For pushed SHA `1223ed3aef3818f56c5b444ee22fff287546349f`, GitHub reported:

- The original pushed head's policy run rejected reports inside reusable output shelves. The later `dca49f8` policy run passed ICM, PowerShell parity, and System Map checks, then found six archived-document links still pointing into those shelves. The links are fixed locally; `python scripts/ci/check-doc-references` passes with 0 unresolved references.
- The original integration job omitted database URLs. `dca49f8` correctly started PostgreSQL and exposed the shared-role setup race; `561bbe9` confirmed the fixture lock removed it and surfaced an incorrect organization count query. Integration run 779 on `1383233` passed all four app provisioning tests against PostgreSQL after the assertion fix.
- `dca49f8` dependency-policy passed with the pinned cargo-deny action and toolchain override. The advisory job still fails for RUSTSEC-2023-0071. CodeQL/offensive-regression were in progress in the last snapshot; secret scanning passed.
- On `1383233`, the canonical verifier reached all-target tests and Part 8 persistence tests, then failed the legacy RLS suite's runtime identity precondition. Commit `0b85158` added `SET ROLE app_runtime`; `1aad95c` policy run 484 passed and integration run 781 passed. Canonical Rust run 631 passed the application and all PostgreSQL persistence/RLS tests, then failed at `cargo audit` on `RUSTSEC-2023-0071`; exact-head cargo-deny passed. A concurrent same-ID provisioning test is now added locally and awaits CI.
- Policy run 483 on `0b85158` failed the System Map process check because the PostgreSQL authority card still cited the pre-change source revision. The process card and persistence object card now cite `0b85158`, describe the role assumption, and the generated object index has been refreshed. Exact-head policy run 484 on `1aad95c` passed.

## Audit conclusion

The application transaction, Part 8 persistence, and legacy RLS behavior have exact-head PostgreSQL evidence on `1aad95c`. Concurrent same-ID tenancy provisioning has a newly added local test that still needs exact-head execution. Production API integration awaits authenticated authority, the relay contract is unresolved, and cargo-audit fails on the upstream-unpatched RSA advisory. PR-009 Part 8 is not merge-ready. Stage 08 remains uncleared pending these open findings and exact-head evidence.
