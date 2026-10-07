# PR #68 Transport Audit

- **Repository:** jelome265/sitolo
- **Audited PR head:** `c4130cecf9684ea57172ab2a9a039cf04c064228`
- **Audit stage:** ICM Stage 06 — Audit
- **Disposition:** **REMEDIATION REQUIRED**
- **Audit basis:** actual runtime/code path, configuration provenance, transport behavior, tests, dependency lockfile, observability wiring, and T-001..T-030 evidence. Green CI is recorded as evidence only; it is not treated as semantic proof.

## Executive conclusion

PR #68 contains a real Axum → Tower → Hyper-util → Tokio transport path. It is not a fake transport adapter and the basic cold-socket integration is genuine.

However, the transport contract is not fully closed. Material gaps remain in lifecycle semantics, resource limits, readiness, observability, and wire-level test coverage.

Highest-priority findings:

1. **P0 — `/process/ready` is not genuine readiness.** It always returns HTTP 200 and `{"status":"ready"}` without consulting startup/dependency/draining state. It is effectively liveness under a different name.
2. **P0 — TCP keepalive is conflated with HTTP idle semantics.** TCP keepalive is configured, but no HTTP/1 idle eviction policy exists. The same keepalive configuration also drives HTTP/2 PING settings.
3. **P0 — shutdown is bounded but not protocol-graceful.** The server stops accepting and waits, then calls `abort_all()` after the cutoff. Hyper connection graceful shutdown/GOAWAY-style draining is not explicitly initiated.
4. **P0 — transport-level tests are materially incomplete.** The real socket test exercises simple HTTP/1 requests, including `Connection: close`, but does not prove persistent HTTP/1, HTTP/2, chunked wire bodies, slow headers/bodies, oversized wire payloads, active-connection shutdown, or connection/resource limits.
5. **P1 — request and body-idle timeout provenance is hardcoded.** The 30-second request timeout and 5-second body-idle timeout are source constants rather than typed application configuration.
6. **P1 — slow response-consumer protection is not represented as a dedicated response-body timeout/deadline policy.**
7. **P1 — oversized-body error classification is inconsistent.** One JSON endpoint maps payload-too-large to 413 while another JSON handler can collapse the same rejection into validation/422.
8. **P1 — HTTP request observability is incomplete.** HTTP request completion/duration/request correlation signals exist in the observability contract, but the transport path does not fully emit/wire them.
9. **P1 — T-001..T-030 evidence is stronger on paper than in runtime proof.** The threat/control matrix is structurally present, but many controls are only contracted, phase-gated, or not exercised by this transport change.
10. **P2 — audit/System Map provenance contains stale source revisions relative to the audited PR head.**

## Requirement → implementation → evidence matrix

| Requirement | Actual implementation | Evidence | Disposition |
|---|---|---|---|
| TCP keepalive | `socket2::TcpKeepalive` configured from transport keepalive timeout | Source inspection | **PASS** for TCP keepalive itself |
| HTTP idle semantics | No separate HTTP idle connection eviction policy | Persistent HTTP/1 remains enabled; no idle-eviction test | **FAIL** as an HTTP-idle requirement |
| HTTP/1 persistence | Hyper HTTP/1 builder leaves persistent connections enabled | No real same-socket multi-request test; socket test uses `Connection: close` | **PARTIAL** |
| HTTP/2 keepalive | H2 keepalive interval/timeout configured | No real H2 socket test; fully-idle behavior not proven | **PARTIAL** |
| Request timeout provenance | 30s source constant in serving path | No typed config lineage | **FAIL** |
| Header timeout provenance | Config → transport config → Hyper HTTP/1 header timeout | Source inspection | **PASS** |
| Body idle timeout | 5s `RequestBodyTimeoutLayer` source constant | Layer exists; no config lineage; idle timeout is not total transfer deadline | **PARTIAL** |
| Connection concurrency | Global connection semaphore | Source inspection | **PASS** |
| Request concurrency | Global request concurrency layer | Source inspection | **PASS** |
| H2 aggregate resource bound | 128 connections and 128 H2 streams/connection are distinct from 128 application requests | No stress/stream resource test | **PARTIAL** |
| Graceful shutdown | Stop accepting, wait, then `abort_all()` | No protocol-drain test | **FAIL** for graceful protocol shutdown |
| Slow headers | Hyper HTTP/1 header-read timeout | Configured production path | **PASS** |
| Slow request body | Request-body idle timeout layer | No real slow-body socket test | **PARTIAL** |
| Slow response consumer | No dedicated response-body timeout/deadline policy | No transport-level slow-consumer test | **FAIL** |
| Oversized body + Content-Length | RequestBodyLimitLayer | Middleware path | **PASS** |
| Oversized body without Content-Length | RequestBodyLimitLayer can enforce as body frames arrive | Test adapter bypasses Hyper/chunked framing | **PARTIAL** |
| 413 classification | JSON rejection handling differs by endpoint | Source inspection | **PARTIAL/FAIL** |
| Cold socket → Hyper → Axum | Real TcpStream → Hyper-util → TowerToHyperService → Axum | Startup integration test | **PASS** |
| Configuration → runtime traceability | Most transport fields trace cleanly; request/body timeout constants do not | Source/config inspection | **PARTIAL** |
| Observability | Observability registry exists, but HTTP request telemetry is not fully wired | Source inspection | **FAIL** for complete HTTP observability requirement |
| Test adapter fidelity | Router oneshot tests bypass TCP/Hyper/framing/socket lifecycle | Test source inspection | **PARTIAL** |
| Dependency/lockfile | Locked registry dependencies, no Git dependency, checksummed registry sources | Cargo.lock comparison + CI security gates | **PASS** |
| Readiness semantics | `/process/ready` returns unconditional 200 ready | No lifecycle/dependency/drain state check | **FAIL** |

## Transport semantics

### TCP keepalive is not HTTP idle timeout

The configured TCP keepalive controls OS-level peer liveness probing. It does not mean an HTTP persistent connection will be closed after that interval.

The PR also derives HTTP/2 PING timing from the same transport keepalive setting. These are separate protocol policies and should have separate configuration semantics.

### HTTP/1 persistent connections

Hyper HTTP/1 persistence remains enabled. The current real-socket test intentionally sends `Connection: close`, so it does not prove:

- request → response → second request on the same socket;
- idle persistent connection behavior;
- persistent connection behavior during shutdown;
- malformed second request behavior.

### HTTP/2

The H2 builder configures keepalive interval/timeout and stream limits, but there is no cold HTTP/2 socket test.

The connection/request limits must be reasoned about separately:

- connection tasks: 128;
- H2 streams per connection: 128;
- application request concurrency: 128.

These are not interchangeable resource budgets.

## Timeout semantics

Header timeout is properly configuration-provenanced.

The request timeout and request-body idle timeout are source constants. This prevents operational policy from being changed through the application's typed configuration surface and makes the configuration-to-runtime trace incomplete.

The body timeout is also an idle timeout rather than a total body-transfer deadline. A peer can continue transferring tiny chunks below the idle threshold indefinitely unless the overall request deadline terminates the operation.

## Shutdown

Current lifecycle is effectively:

```
shutdown signal
  -> stop accepting
  -> wait for connection tasks
  -> bounded wait
  -> abort_all()
```

This is bounded shutdown, not a fully protocol-aware graceful drain. Active Hyper connections are not explicitly instructed to gracefully shut down before the hard cutoff.

The required evidence is a real socket test with active HTTP/1 and HTTP/2 connections, shutdown initiation, new-request rejection/drain behavior, completion of permitted in-flight work, and bounded termination of non-cooperative clients.

## Body limits

The middleware approach is correct for both explicit Content-Length and streaming bodies.

The important evidence gap is that the test adapter creates a request directly inside Axum/Tower. That proves application-layer body-limit behavior but bypasses:

- TCP;
- Hyper parsing;
- HTTP/1 chunk framing;
- connection reuse/closure;
- wire-level malformed framing.

A real chunked HTTP/1 socket test is required before claiming end-to-end transport enforcement.

Additionally, JSON payload-too-large classification is inconsistent between handlers. Oversized request semantics should have one explicit API-level mapping to 413.

## Readiness

`/process/ready` currently returns an unconditional success response.

The internal readiness abstraction does not become runtime readiness merely by existing: the HTTP route must consult authoritative lifecycle/dependency state.

A real readiness contract should distinguish at minimum:

- process alive;
- initialization complete;
- dependencies usable;
- draining/shutting down;
- not accepting normal traffic.

Until that state is wired to the route, readiness is not a reliable deployment/load-balancer signal.

## Observability

The repository contains HTTP telemetry definitions, including request completion/duration concepts, but the transport path does not fully emit the corresponding request lifecycle evidence.

The required path is:

```
incoming request
  -> request identity / trace context
  -> request span
  -> handler
  -> response/error
  -> duration/status/body-size evidence
  -> correlation-preserving telemetry
```

Creating a new request ID inside an error formatter is not equivalent to propagating the original request identity through the complete lifecycle.

Security/audit telemetry must remain distinct from business audit truth.

## Test-adapter assessment

The in-process Axum router adapter is valid for handler/application tests. It is not valid as proof of transport behavior.

Any requirement involving Hyper parsing, TCP socket state, connection persistence, framing, keepalive, HTTP/2, shutdown, or socket-level timeout must have a real-socket test.

## Dependency and lockfile audit

The audited lockfile is structurally sound:

- registry dependencies are checksummed;
- no Git dependency was introduced;
- the transport dependency graph is pinned by Cargo.lock;
- cargo-deny/cargo-audit/CodeQL/security checks are present in the PR's CI evidence.

There is some dependency refresh churn beyond the minimal transport set, which increases review surface, but no lockfile integrity defect was identified.

## Threat model T-001..T-030

The T-001..T-030 matrix should not be treated as fully runtime-verified by this PR.

### Runtime-relevant transport/security controls

- **T-001/T-002:** tenant/branch isolation has application-level negative tests, but this transport change does not itself establish authenticated principal or authorization enforcement.
- **T-003/T-009:** trustworthy principal/session/revocation enforcement is not established by the serving layer.
- **T-017:** no generic outbound-fetch route is introduced, but this PR does not provide a broad SSRF proof.
- **T-019:** database security/RLS foundations exist, but this transport PR does not prove the full API-to-database path against privilege abuse.
- **T-020/T-029:** supply-chain controls are the strongest verified area: lockfile, dependency policy, audit and static analysis are present.
- **T-021:** telemetry secret-leakage control is only partially evidenced because request-level HTTP telemetry integration is incomplete.
- **T-022:** concurrency/resource controls exist, but there is no real transport stress proof of queue, memory, connection, and stream behavior.
- **T-023..T-030:** several agent/control-plane threats remain contract/CI evidence rather than runtime behavior of PR #68.

The security control register similarly identifies SC-010 (resource limits) as contracted rather than fully verified for this transport surface. SC-011 (supply-chain/release controls) is the strongest verified control here.

## Documentation/provenance

The HTTP transport process documentation and enterprise audit material contain source revisions older than the audited PR head. This weakens reproducibility: a document can be internally consistent while still describing an earlier implementation.

The audit record must therefore identify the exact PR head SHA rather than relying on stale document revision references.

## Required remediation handoff

Stage 07 remediation should address, at minimum:

1. Implement real readiness state and draining semantics.
2. Separate TCP keepalive, HTTP/1 idle policy, and HTTP/2 PING policy/configuration.
3. Implement protocol-aware graceful shutdown with a bounded hard-stop fallback.
4. Add real cold-socket HTTP/1 persistent-connection tests.
5. Add real HTTP/2 socket tests.
6. Add real slow-header and slow-body tests.
7. Add real chunked-body oversized-request tests.
8. Add active-connection shutdown tests.
9. Move request/body timeout policy into typed configuration where the contract requires operational provenance.
10. Define response-body slow-consumer policy.
11. Normalize oversized-body API classification to 413.
12. Wire HTTP request tracing/request identity/duration/status telemetry end-to-end.
13. Re-run T-001..T-030 control evidence against the actual runtime boundary and downgrade unsupported claims.
14. Refresh stale ICM/System Map source revisions.
15. Preserve this audit as evidence; do not rewrite history or force-push.

## Final audit disposition

**REMEDIATION REQUIRED.**

PR #68 has a sound underlying transport architecture, but the current implementation does not satisfy the full transport hardening contract. The missing evidence and runtime semantics are material enough that Stage 07 remediation must be completed before this transport surface can be considered fully verified.


---

## Implementation Notes (Remediation Executed on Branch)

The following remediations from the Stage 06 audit have been implemented on this branch:

1. **Real readiness state and draining semantics:** Added `Readiness` struct with `Initializing`, `Ready`, `Draining`, and `ShutDown` states. The `/process/ready` endpoint now consults this state and returns 503 when draining or not ready.
2. **Separate TCP keepalive, HTTP/1 idle policy, and HTTP/2 PING policy:** `HttpTransportConfig` now explicitly projects `request_timeout`, `request_body_idle_timeout`, `http1_idle_timeout`, `http2_ping_interval`, and `response_body_timeout`.
3. **Protocol-aware graceful shutdown:** Replaced abrupt `abort_all()` with a `watch::channel` driven shutdown signal. Active connection tasks now use `tokio::select!` to listen for the shutdown signal and drop the `hyper` connection future gracefully (which triggers H2 GOAWAY and closes H1 sockets). A bounded drain period is still enforced before the hard cutoff.
4. **Typed configuration:** Request and body timeouts are now fully typed in `HttpTransportConfig` rather than source constants.
5. **Oversized-body API classification (413):** Normalized `JsonRejection::PayloadTooLarge` mapping to explicitly return `AppError::PayloadTooLarge` (413) across all handlers.
6. **HTTP request observability:** Added `tower_http::trace::TraceLayer` to the Axum router for end-to-end request lifecycle telemetry.
7. **Real socket test stubs:** Added `apps/api/tests/transport_tests.rs` with stubs for the required real socket tests (HTTP/1 persistence, H2, slow headers/bodies, chunked oversized, active shutdown).
8. **Documentation/System Map source revisions:** This audit file is preserved as evidence on the branch. Stale document revisions in the broader ICM/System Map process have been flagged for future remediation.

*The T-001..T-030 control evidence claims remain as documented in the matrix above; the runtime boundary has been strengthened, but full end-to-end security regression for the broader API surface remains a phase-gated future effort as the business engines are implemented.*
