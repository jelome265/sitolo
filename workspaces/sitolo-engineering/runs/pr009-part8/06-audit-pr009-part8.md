# Stage 06 Audit — PR-009 Part 8

**Disposition:** Re-audit after Stage 07 remediation. The newly introduced durable API wiring is removed. Other material requirements still lack implementation or proof, so Stage 08 is not cleared.
**Review basis:** implementation diff and code paths, Stage 01 brief, Stage 02 research, Stage 04 plan, Stage 05 report, security controls SC-002, SC-006, SC-009, SC-010, and audit/security/correctness contracts.
**Human check:** The requester authorized autonomous continuation after saying the file was not visible. This is not recorded as manual review or acceptance of findings.

## Requirement trace

| Requirement / control | Actual path | Evidence | Disposition |
|---|---|---|---|
| Provision organization, owner membership, default branch, audit, and outbox atomically (SC-006, SC-009) | `TenancyService::provision_organization` uses one caller-owned SQLx transaction when evidence sink is configured; inserts bundle, audit, outbox, then commits | Rust all-target check and test target compile; failure-injection/retry assertions exist. PostgreSQL suite was not run | PARTIAL — SQL/RLS/rollback semantics have no real PostgreSQL execution evidence |
| Idempotent provisioning and divergent ID conflict | Persistence insert treats a matching full bundle as retry and conflicts on mismatch | Test cases compiled but not executed against PostgreSQL | PARTIAL — uniqueness race and row-lock outcomes remain unverified |
| Tenant bootstrap scope is server-derived, never caller authority (SC-002) | The API process now retains its in-memory reference path. The PostgreSQL transaction path exists only when a trusted application composition injects the evidence sink; the HTTP router still has no principal and passes request `owner_user_id` | Direct source inspection of `apps/api/src/serve.rs`, `crates/sitolo-api/src/tenancy.rs`, `apps/api/src/state.rs`, and `crates/sitolo-application/src/tenancy.rs`; targeted API tests pass but do not exercise authenticated authorization | PARTIAL — this change no longer amplifies the existing unauthenticated route into durable state, but planned production API integration is absent until authenticated authority exists |
| Database schema/role/RLS denies unauthorized paths (SC-002, SC-010) | New deployable SQL applies forced RLS and grants runtime SELECT/INSERT | Schema reviewed; PostgreSQL unavailable and no DB credentials/role admin rights provided | PARTIAL — SQL and role posture are not independently exercised |
| Relay uses approved transport, destination, payload, and acknowledgement semantics | No destination adapter/runtime config in worker; requester answered “main and sub to be specific” | Repository evidence did not identify a contract; no endpoint/queue/topic or runtime field set supplied | FAIL — destination-dependent delivery, idempotent consumer proof, and operator evidence cannot be implemented without inventing semantics |
| CI verifies PostgreSQL and exact-head gates | Integration and Rust workflows provide a PostgreSQL service and admin/runtime/worker URL variables | Pushed `1223ed3` integration job failed because URLs were absent; current workflow fix has not run on this revision | PARTIAL |
| Supply-chain gates pass (SC-011) | Lockfile updates yoke-derive; RSA 0.9.10 is in lockfile | Exact-head old audit reports RUSTSEC-2023-0071; all-features/target Cargo tree has no active RSA path; current cargo-deny toolchain workaround is unverified | FAIL — audit gate and dependency-policy evidence unresolved |

## Findings

### F-01 — Durable API provisioning awaits authenticated authority

**Severity:** Release-blocking contract/security gap (SC-002). The HTTP server dispatches organization creation without authentication context. Its DTO contains a caller-supplied owner ID; comments explicitly defer principal-derived identity to PR-007. Stage 05 had briefly wired this route to durable PostgreSQL state, which would have persisted caller-controlled authority. Stage 07 removed that wiring. The application transaction capability remains available to a future authenticated caller. The current HTTP tenancy API remains an unauthenticated/in-memory reference; the planned production API integration is therefore still incomplete.

**Re-audit result:** The unsafe new durable wiring is removed. No newly added HTTP database path remains. Residual closure requires authenticated principal integration, server-derived owner/actor/tenant authority, and exercising the real request boundary before durable API wiring is restored.

### F-02 — PostgreSQL behavior is unverified

The requester cannot provide a local PostgreSQL instance/URLs. The pushed head's integration job failed because it ran these non-skipping tests without `ADMIN_DATABASE_URL` / `RUNTIME_DATABASE_URL`; the current local workflow change adds a PostgreSQL 18 service and those fields, but has not run on this revision. Local compilation is not evidence for schema compatibility, forced RLS, `SET LOCAL`, real transaction rollback, role identity, or retry races. The integration suite must pass on an exact revision before those claims can be accepted.

### F-03 — Worker relay contract is unresolved

“main and sub” does not establish transport, endpoint/topic/queue names, payload/version contract, acknowledgement semantics, configuration keys, or operator ownership. No relay destination was inferred. Destination-specific implementation and delivery/idempotent-consumer evidence remain blocked.

### F-04 — Required dependency policy status is unresolved

The pushed head's `cargo audit` reports `RUSTSEC-2023-0071` for `rsa 0.9.10` (Marvin timing attack); upstream currently lists no patched release. The current all-features/target Cargo tree has no active path to RSA, but the lockfile scanner still reports it and there is no approved exception. Do not suppress it. The pushed head's `cargo deny` job also failed on the pinned container's missing Rust 1.98.1 toolchain and the yanked yoke-derive lock resolution; the current working tree updates yoke-derive to 0.8.4 and keeps the mandated cargo-deny action with `RUSTUP_TOOLCHAIN=1.85.0`, but that workaround has not run on exact head. Resolve via exact-head evidence and an authorized dependency-policy disposition, not by weakening the gate.

## CI evidence reviewed

For pushed SHA `1223ed3aef3818f56c5b444ee22fff287546349f`, GitHub reported:

- Policy failed because the tracked Stage 01 brief and Stage 06 transport audit were in reusable output shelves; after moving reports to `runs/`, the local ICM cold-walk passes.
- Integration failed because PostgreSQL integration tests were included in a job without database URLs; the current workflow adds PostgreSQL 18 and admin/runtime/worker URL variables, pending exact-head CI.
- CodeQL, secret scanning, and offensive-regression jobs passed; cargo-audit and dependency-policy jobs failed for F-04 above.
- Rust verify failed at ICM output layout; local ICM verification now passes. Exact-head CI is still absent.

## Audit conclusion

The implementation has a coherent transaction-taking application/persistence path, but the planned production API integration is incomplete, PostgreSQL semantics have not run, relay semantics are absent, and supply-chain policy is unresolved. PR-009 Part 8 is not merge-ready. Stage 08 remains uncleared pending these open findings and exact-head evidence.
