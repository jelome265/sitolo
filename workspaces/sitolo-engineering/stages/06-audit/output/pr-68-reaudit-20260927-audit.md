# PR #68 Phase 2 Re-Audit — 2026-09-27

**Repository:** jelome265/sitolo
**PR:** #68 — feat: reconcile threat model and restore Axum HTTP architecture
**Audit stage:** ICM Stage 06 — Audit
**Audit baseline:** c27cbe53dbcb05301791b618c85b03a7258dcfa7
**Scope:** recent remediation pushes, current transport implementation, Phase 2 contract compliance, security/observability requirements, CI evidence, and ICM evidence integrity
**Disposition:** **REMEDIATION REQUIRED — NOT PHASE 2 VERIFIED**

## 1. Executive conclusion

The latest remediation commit is a real improvement. PR #68 now contains a genuine Tokio listener -> Hyper/Hyper-util -> Tower/Axum -> handler path, and the transport tests were rewritten to exercise real TCP sockets rather than claim-only assertions.

The remediation still does not satisfy the Phase 2 implementation contract.

Current-head CI evidence is absent: GitHub reported no status records and no pull-request workflow runs for c27cbe53dbcb05301791b618c85b03a7258dcfa7 at the time of this audit. The remediation commit also explicitly states that cargo check, cargo test, and scripts/ci/verify were not executed in its authoring environment.

Independent inspection additionally found compile-breaking defects in the Phase 2 configuration crate.

## 2. Recent-push audit

### Current PR head

c27cbe53dbcb05301791b618c85b03a7258dcfa7

Commit:
fix(transport): real remediation of PR #68 audit findings, not fakes

This is newer than c4130cecf9684ea57172ab2a9a039cf04c064228, which the PR body still describes as the final verification baseline. That older SHA is therefore stale for current-head verification.

### Evidence integrity

The latest remediation commit correctly records that earlier non-canonical Stage 07/08 artifacts claimed COMPLETED and VERIFIED without evidence.

Those files remain present:

- workspaces/sitolo-engineering/stages/07-remediation/output/pr-68-remediation-report.md
- workspaces/sitolo-engineering/stages/08-verification/output/pr-68-verification-report.md

They still contain unconditional COMPLETED / VERIFIED claims. The canonical current remediation artifact is under 07-remediate and says PARTIAL with re-audit required.

This is an ICM evidence-integrity defect: stale or false historical artifacts must not be mistaken for current verification evidence.

## 3. Phase 2 requirement matrix

| Requirement | Current implementation/evidence | Disposition |
|---|---|---|
| Typed runtime configuration | AppConfig exists, but current Builder/model, field catalogue and validation sources contain compile-breaking drift | **FAIL** |
| Deterministic canonical environment parsing | SITOLO__ namespace, BTreeMap normalization and environment-first profile selection | **PASS** subject to compilation |
| Production/staging separation | Production disables local secret provider and removes development DB identity defaults | **PASS** subject to compilation |
| Startup validation before readiness | Validation precedes state construction and readiness publication | **PARTIAL** because the config crate is currently non-compilable |
| Secret boundary | AppConfig stores SecretRef; startup probe resolves then drops the secret value | **PASS** for inspected path |
| Hard configuration ceilings | HTTP, DB and telemetry ceilings are declared and checked | **PASS/PARTIAL**; execution unavailable |
| Configuration fingerprint | SHA-256 canonical fingerprint exists | **FAIL** because newly added HTTP timeout fields are omitted |
| Structured logging | Central JSON tracing subscriber initialization exists | **PASS** |
| Request correlation | RequestId parser exists, but TraceLayer uses raw x-request-id and error responses create a fresh server ID | **FAIL** |
| W3C trace context | TraceParent parser exists, but transport does not extract/propagate traceparent | **PARTIAL** |
| Typed errors / safe Problem Details | Bounded error taxonomy and public mapping exist | **PASS/PARTIAL** |
| HTTP telemetry registry | http.request.completed and HTTP metrics are registered | **PASS** as registry evidence |
| Runtime HTTP telemetry | TraceLayer logs only; no observed TelemetryBuffer ingestion, metric recording path or OpenTelemetry export path | **FAIL** |
| Telemetry backpressure | Priority-aware bounded buffer model exists | **PARTIAL** because the HTTP runtime/export path is not wired to it |
| HTTP/1 persistence | Real-socket test sends two requests on one TCP connection | **PASS** as test design |
| HTTP/1 true idle semantics | Idle sleep starts at connection acceptance and is never reset after traffic | **FAIL** |
| TCP keepalive | socket2 TCP keepalive is configured from dedicated setting | **PASS** |
| HTTP/2 keepalive | H2 interval and timeout are separate transport settings | **PASS/PARTIAL**; current-head execution unproven |
| Request/header/body timeouts | Typed transport fields are wired to Hyper/Tower layers | **PASS/PARTIAL**; config crate compilation blocks end-to-end proof |
| Response-body timeout | ResponseBodyTimeoutLayer is wired | **PARTIAL**; no dedicated slow-consumer test |
| Oversized request bodies | RequestBodyLimitLayer caps at configured value and tenancy hard maximum | **PASS** by source inspection |
| Chunked oversized body | Real socket test uses Transfer-Encoding: chunked without Content-Length | **PASS/PARTIAL**; execution unproven |
| Connection/request concurrency | Connection semaphore and GlobalConcurrencyLimitLayer are real | **PASS/PARTIAL**; no stress evidence |
| Graceful shutdown | Shutdown signal triggers Hyper graceful_shutdown, bounded drain, then hard cutoff | **PASS/PARTIAL**; execution unproven |
| Readiness | /process/ready checks runtime readiness and draining state | **PASS** by source inspection |
| Phase 2 CI enforcement | Canonical verify script and phase2 policy gate exist | **PASS** as policy implementation, **UNVERIFIED** on current head |
| Phase 2 exit gate | Requires executable config/telemetry/error/security evidence and release evidence | **FAIL** |

## 4. Confirmed compile-breaking findings

### F-2A — Duplicate fields in defaults::Builder

**File:** crates/sitolo-config/src/model.rs

The current defaults::Builder declaration contains duplicate timeout fields:

- request_timeout_ms
- request_body_idle_timeout_ms
- http1_idle_timeout_ms
- http2_ping_interval_ms
- response_body_timeout_ms
- http2_keep_alive_timeout_ms

Rust struct declarations cannot contain duplicate field names.

**Impact:** the Phase 2 configuration model cannot compile.

**Severity:** P0

### F-2B — Invalid field catalogue macro invocation

**File:** crates/sitolo-config/src/field.rs

The field! macro accepts one key literal followed by class/required/owner/syntax.

The current catalogue contains one invocation passing multiple string literals before Tunable. That invocation cannot match the macro definition.

**Impact:** the configuration field catalogue cannot compile.

**Severity:** P0

### F-2C — AppConfig construction omits newly introduced fields

**File:** crates/sitolo-config/src/validate.rs

AppConfig declares the new timeout fields, but the final AppConfig initializer omits them.

Missing:

- request_timeout_ms
- request_body_idle_timeout_ms
- http1_idle_timeout_ms
- http2_ping_interval_ms
- response_body_timeout_ms
- http2_keep_alive_timeout_ms

**Impact:** even after F-2A/F-2B are fixed, validation still fails compilation with missing-field errors.

**Severity:** P0

These three findings independently prevent Phase 2 configuration from being accepted as executable implementation evidence.

## 5. Configuration fingerprint defect

**File:** crates/sitolo-config/src/fingerprint.rs

canonical_non_secret_config includes older fields but omits:

- request_timeout_ms
- request_body_idle_timeout_ms
- http1_idle_timeout_ms
- http2_ping_interval_ms
- response_body_timeout_ms
- http2_keep_alive_timeout_ms

Therefore:

effective timeout policy changes -> identical configuration fingerprint

This directly undermines the Phase 2 configuration-drift requirement. A deployment could change request timeout policy while retaining the same reported fingerprint.

**Severity:** P1

Required correction: include every effective non-secret AppConfig field and add regression coverage for every newly introduced timeout field.

## 6. Request-correlation defect

**Files:** apps/api/src/serve.rs, crates/sitolo-observability/src/context.rs, crates/sitolo-api/src/error.rs

RequestId::parse_client provides a bounded validation boundary, but the HTTP TraceLayer reads the raw x-request-id header and places it directly into the span.

Separately, format_error_response generates a new RequestId when serializing the public error.

This can produce:

transport span request ID != request ID returned in the error response

The Phase 2 correlation contract requires the same request identity to connect the HTTP request, logs and safe API errors.

**Severity:** P1

## 7. HTTP/1 idle semantics defect

**File:** apps/api/src/serve.rs

The connection task creates one sleep for transport.http1_idle_timeout at connection acceptance. The timer is not reset after request/response activity.

That means the code implements a connection-age timer, not a true idle timer.

Example:

1. TCP connection opens.
2. Client sends valid requests periodically.
3. Original sleep still expires at the original deadline.
4. Server starts graceful shutdown even though the connection is active.

TCP keepalive is correctly configured separately; it does not repair this HTTP-level semantic defect.

**Severity:** P1

## 8. HTTP telemetry remains incomplete

TraceLayer is a real improvement, but the Phase 2 contract requires more than trace log emission.

Current evidence shows:

- telemetry event/metric registries exist;
- TelemetryBuffer exists and is bounded;
- TraceLayer emits tracing events;
- no inspected HTTP path pushes completed request events into TelemetryBuffer;
- no observed runtime metric recording path for the registered HTTP metrics;
- no OpenTelemetry exporter/bridge runtime path was found.

Therefore the chain:

telemetry contract -> registry -> runtime emission -> bounded export/degradation

is not closed.

**Severity:** P1

## 9. Transport remediation: what is genuinely fixed

The following changes are materially real and should be retained:

### Real cold-socket integration
Tests bind a real localhost TcpListener, spawn the real serve function and drive it with a real TcpStream.

**Disposition:** PASS as implementation/test design; execution pending.

### HTTP/1 persistence
Two requests are sent over the same socket and both must receive 200 responses.

**Disposition:** PASS as test design.

### Slow-header enforcement
A partial request stalls beyond the configured header timeout and the test expects the server to close/reject it.

**Disposition:** PASS as test design.

### Slow-body enforcement
A declared body is only partially sent and then stalled past the configured body-idle timeout.

**Disposition:** PASS as test design.

### Chunked oversized-body enforcement
A real chunked body without Content-Length is sent above the effective limit and 413 is required.

**Disposition:** PASS as test design.

### HTTP/2 prior-knowledge path
The test sends the actual h2c preface and SETTINGS frame and expects a server SETTINGS frame.

**Disposition:** PASS as test design.

### Active shutdown
The test uses the actual shutdown signal and actual shutdown deadline while a real socket is open.

**Disposition:** PASS/PARTIAL as test design.

### Readiness
The route now consults real readiness/draining state instead of unconditional success.

**Disposition:** PASS by source inspection.

### 413 classification
Both inspected JSON endpoints explicitly map PayloadTooLarge to HTTP 413.

**Disposition:** PASS by source inspection.

### Protocol-aware shutdown
Active Hyper connections are sent graceful_shutdown on shutdown, with bounded drain and abort fallback.

**Disposition:** PASS/PARTIAL by source inspection; execution pending.

## 10. Security-control traceability

Applicable controls:

| Control | Evidence | Disposition |
|---|---|---|
| SC-004 API/input/boundary security | body bounds, timeout layers, safe error mapping; request-ID validation exists but is bypassed by TraceLayer | **PARTIAL** |
| SC-005 secrets/crypto lifecycle | typed secret references and production fail-closed provider selection | **PASS/PARTIAL** |
| SC-010 resource limits | connection/request/body bounds exist; no current-head execution or stress evidence | **PARTIAL** |
| SC-011 source/dependency/release integrity | pinned toolchain/actions, Cargo.lock, security workflows; no current-head execution record | **PARTIAL / UNVERIFIED CURRENT HEAD** |

The T-001..T-030 matrix must not be reported as fully runtime-verified by this transport PR.

## 11. Phase 2 gate

The Phase 2 exit gate requires evidence for:

- validated configuration;
- secret resolution boundary;
- safe structured logging;
- request correlation;
- deterministic public errors;
- bounded metrics and telemetry;
- telemetry failure degradation;
- leakage tests;
- config/telemetry/error tests;
- release-blocking CI;
- runbooks;
- release evidence.

The current PR does not satisfy that gate.

The main blocker is concrete: the current Phase 2 config crate is not compile-clean. Additional semantic gaps remain in fingerprint coverage, request correlation, HTTP idle semantics and actual telemetry wiring.

**Final disposition: FAIL — remediation required before Phase 2 verification/closure can be claimed.**

## 12. Required remediation order

1. Remove duplicate timeout fields from defaults::Builder.
2. Correct the invalid field! catalogue entry and synchronize catalogue/accepted-key/parser coverage.
3. Initialize all new timeout fields in AppConfig validation.
4. Extend configuration fingerprint canonicalization and tests to cover all new timeout fields.
5. Apply RequestId::parse_client at the HTTP boundary and propagate one request ID into logs and safe errors.
6. Implement true HTTP/1 idle semantics with a deadline refreshed by traffic, or use a documented protocol-native idle mechanism.
7. Wire actual HTTP request events/metrics into the existing telemetry architecture and establish an OpenTelemetry-compatible bounded export path.
8. Add a dedicated slow-response-consumer transport test.
9. Add resource-pressure tests for connection/request concurrency ceilings.
10. Execute the full canonical verification command set on the resulting head.
11. Retire or explicitly mark the stale non-canonical COMPLETED/VERIFIED artifacts as historical and non-authoritative.
12. Re-enter Stage 06 for re-audit, then Stage 08 for executed verification only.

## 13. CI trigger

The repository workflows trigger on pull_request events targeting main:

- .github/workflows/rust.yml
- .github/workflows/security.yml
- .github/workflows/policy.yml
- .github/workflows/integration.yml

An audit-only documentation commit on this PR branch is therefore an appropriate way to cause a fresh pull-request CI run without altering the transport implementation.

No CI result is considered passed until GitHub reports the actual run outcome.

## 14. Evidence inspected

Governance/routing:
- CLAUDE.md
- workspaces/CLAUDE.md
- workspaces/sitolo-engineering/CLAUDE.md
- workspaces/sitolo-engineering/CONTEXT.md
- workspaces/sitolo-engineering/shared/phase-context/CONTEXT.md
- workspaces/sitolo-engineering/stages/06-audit/CONTEXT.md
- workspaces/sitolo-engineering/stages/06-audit/references/audit-contract.md
- workspaces/sitolo-engineering/stages/06-audit/references/correctness-checks.md
- workspaces/sitolo-engineering/stages/06-audit/references/security-checks.md

Canonical contracts:
- docs/phase2_config_secrets_logging_errors_telemetry_implementation.md
- docs/observability_spec.md
- docs/security_control_register.md

Implementation:
- apps/api/src/serve.rs
- apps/api/src/bootstrap.rs
- apps/api/src/shutdown.rs
- apps/api/src/state.rs
- crates/sitolo-config/src/model.rs
- crates/sitolo-config/src/parse.rs
- crates/sitolo-config/src/validate.rs
- crates/sitolo-config/src/field.rs
- crates/sitolo-config/src/fingerprint.rs
- crates/sitolo-observability/src/context.rs
- crates/sitolo-observability/src/buffer.rs
- crates/sitolo-observability/src/registry.rs
- crates/sitolo-api/src/error.rs
- scripts/ci/verify
- scripts/ci/check-phase2-policy
- .github/workflows/rust.yml
- .github/workflows/security.yml
- .github/workflows/policy.yml
- .github/workflows/integration.yml

**End state:** PR #68 remediation is materially improved, but the current head is not Phase 2 compliant and is not verified by CI.
