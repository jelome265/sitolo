# PR #68 Transport Remediation Report

**Status:** PARTIAL — real fixes applied for the P0 blockers and most P1s; two
items remain genuinely open (see "Not remediated" below). Re-audit required
per stage contract step 8 before this proceeds to 08-verify.

## Provenance note on prior stage-07/08 artifacts

`workspaces/sitolo-engineering/stages/07-remediation/output/pr-68-remediation-report.md`
and `.../08-verification/output/pr-68-verification-report.md` (non-canonical
paths — the contract's canonical names are `07-remediate` and `08-verify`)
each asserted "COMPLETED"/"VERIFIED" with no evidence. Checked against the
actual code, that was false: the branch did not compile (two separate
struct-field errors, detailed below) and 4 of the 6 "real socket" tests in
`transport_tests.rs` were `assert!(true, "<claim>")` — they asserted nothing.
This report does not build on those artifacts' claims; every item below was
independently re-verified by reading the current source.

## Findings addressed

### F1 (P0, audit #3). Compile-breaking: duplicate struct fields in `HttpTransportConfig::from_config`
**Root cause:** `apps/api/src/serve.rs` `from_config` initialized
`request_timeout`, `request_body_idle_timeout`, `http1_idle_timeout`,
`http2_ping_interval`, and `response_body_timeout` from real `AppConfig`
values, then immediately re-initialized the same five fields again from
hardcoded source constants (`REQUEST_TIMEOUT_SECS` etc.) — a hard `rustc`
E0062 error, and simultaneously the reason audit item #5 (timeout provenance
hardcoded) was not actually fixed despite the config plumbing existing.
**Fix:** Deleted the five dead constants and the duplicate block; the typed,
env-sourced `AppConfig` values (already fully wired in `sitolo-config` —
env parsing in `parse.rs`, ceiling validation in `validate.rs`, real
defaults in `model.rs`) now flow through uncontested.
**Proof:** Manual review of the resulting single struct-literal; each field
initialized exactly once, from `config.<field>_ms`. Not compiled in this
sandbox (no toolchain available) — see "Verification limitations."

### F2 (P0/P1, related to audit #5). Compile-breaking: missing `http2_keep_alive_timeout` field
**Root cause:** `serve.rs`'s connection setup reads
`transport.http2_keep_alive_timeout`, but `HttpTransportConfig` never
declared that field and `from_config` never populated it, even though
`AppConfig.http2_keep_alive_timeout_ms` already existed with real env
parsing and ceiling validation. Second independent compile error.
**Fix:** Added the field to `HttpTransportConfig` and populated it from
`config.http2_keep_alive_timeout_ms` in `from_config`.
**Proof:** Manual review; matches the pattern of the other five timeout
fields exactly.

### F3 (P0, audit #4). Transport tests materially incomplete / fabricated
**Root cause:** `apps/api/tests/transport_tests.rs` had two tests that bound
a `TcpListener` but never spawned `serve()` on it (would hang against
nothing accepting the connection), and four tests that were
`assert!(true, "<claim>")` — no request sent, no byte read, no protocol
exercised.
**Fix:** Rewrote the file. Every test now spawns the real `serve()` loop
(the same function `main.rs` runs) on a real `TcpListener` and drives it
with a raw `TcpStream`:
- `real_socket_http1_persistent_connection` — two sequential requests on
  one connection, asserting real 200 responses (proves keep-alive).
- `real_socket_slow_header_test` — sends a partial request line, stalls
  600ms against a 200ms `request_header_timeout`, asserts the server
  actually closes/rejects rather than hanging.
- `real_socket_slow_body_test` — declares `Content-Length: 20`, sends 1
  byte, stalls 700ms against a 200ms `request_body_idle_timeout`, asserts
  the stalled body is never treated as a complete request.
- `real_socket_chunked_oversized_request_test` — sends a genuine
  chunked-transfer-encoded body (no `Content-Length`) totaling 40 KiB
  against the 32 KiB cap, asserts a real `413` comes back.
- `real_socket_h2_prior_knowledge_test` — sends the literal RFC 9113 §3.4
  h2c connection preface and an empty SETTINGS frame at the byte level,
  asserts the server's first response frame is type `0x04` (SETTINGS) —
  proves the listener genuinely speaks HTTP/2, not a string match.
- `real_socket_active_connection_shutdown_test` — warms up a real
  connection, sends the actual shutdown signal through the same
  `oneshot::Sender` `main.rs` wires to SIGTERM, asserts the connection is
  closed within the real `SHUTDOWN_DRAIN_DEADLINE_SECS` (imported, not
  duplicated) and that `serve()` returns a non-empty subsystem list.
**Not covered by this pass:** a resource-limit test proving
`MAX_IN_FLIGHT_CONNECTIONS`/`MAX_IN_FLIGHT_REQUESTS` are actually enforced
under load (audit #4 also named "connection/resource limits" — this test
was not written; flagging rather than silently dropping it).
**Proof:** Manual review only — not executed. See "Verification
limitations."

### F4 (P0, audit #1). `/process/ready` readiness — verified as genuinely real, no fix needed
Independently re-checked (not merely trusted from the prior fake report):
`ready()` in `serve.rs` calls `state.readiness().is_draining()` and
`.get_state()`, returning 503 for `Draining`/non-`Ready` and 200 only for
`Ready`. `bootstrap.rs` calls `state.readiness().mark_ready()` only after
every startup step succeeds; `serve.rs` calls `mark_draining()` on shutdown
signal. This is real, already correct on this branch prior to my changes.

### F5 (P0, audit #2/#3). HTTP/1 idle eviction and protocol-graceful shutdown — verified as genuinely real, no fix needed
Independently re-checked: each accepted connection runs its own
`tokio::select!` racing the connection future against an idle-timeout sleep
and a shutdown watch-channel; both the idle and shutdown paths call
`conn.as_mut().graceful_shutdown()` (hyper-util's real GOAWAY-equivalent
graceful shutdown), not just `abort_all()`. `abort_all()` is only reached
after the bounded drain deadline as a last-resort cutoff, which is
appropriate. Real, already correct prior to my changes — F3's new
`real_socket_active_connection_shutdown_test` is the first thing to
actually prove this at the socket level.

### F6 (P1, audit #7). Oversized-body 413 classification — verified as genuinely real, no fix needed
Both `provision_organization` and `create_branch` handlers in `serve.rs`
inspect the `JsonRejection`'s status and explicitly return
`AppError::PayloadTooLarge` for `413`, falling through to `Validation`
otherwise. Consistent across both endpoints. Real, already correct prior to
my changes.

## Not remediated (carried forward, not silently dropped)

- **Audit #6 (P1) — dedicated response-body timeout for slow consumers.**
  `ResponseBodyTimeoutLayer` is wired from real config
  (`transport.response_body_timeout`), so the mechanism exists, but no test
  in this pass exercises a slow-consumer scenario specifically (F3's tests
  cover slow *request* header/body, not slow response consumption). Needs a
  dedicated test before this can be called proven.
- **Audit #8 (P1) — HTTP request observability wiring.** `TraceLayer` emits
  real `tracing::info_span!`/`tracing::info!` with method, URI, request ID,
  status, and latency. I checked whether this connects to `AppState`'s
  `TelemetryBuffer` (`sitolo-observability`) and it does not — `TraceLayer`
  writes only to the `tracing` subscriber, `TelemetryBuffer` is a separate
  queue-depth/backpressure tracker never touched by the transport path. I
  have **not** checked this against the project's observability contract
  document to determine whether that gap is what audit #8 meant, or whether
  `tracing` output alone satisfies it — flagging as unresolved rather than
  guessing.
- **Audit #9 (P1) — T-001..T-030 threat/control matrix evidence.** Not
  attempted in this pass; this is a cross-cutting evidence-gathering
  exercise across the full control register, out of scope for a
  transport-focused remediation pass.
- **Audit #10 (P2) — stale source-revision provenance in audit/System Map.**
  Documentation drift, lowest priority per remediation order; not touched.
- **`bootstrap::Readiness` (separate from `shutdown::Readiness`, pre-existing, out of audit scope):**
  noticed in passing that `StartupContext` carries its own, unrelated
  `Readiness { Ready, NotReady }` enum hardcoded to `Ready` on every success
  path (only consumed by tautological assertions in `startup.rs`). Not an
  audit finding, not touched, noted for awareness only.

## Verification limitations

**No Rust toolchain is available in this sandbox, and installing one is not
permitted here.** Every fix above was verified by careful manual reading of
the surrounding code (exact field names, trait signatures, and library APIs
cross-checked against `docs.rs` for `axum` 0.8.9 / `tower-http` where
relevant) — not by `cargo check`, `cargo build`, or `cargo test`. This is a
materially weaker form of proof than compilation and test execution, and
`08-verify`'s `./scripts/ci/verify` must be run for real (in CI or on a
machine with a toolchain) before this can be marked verified. Per the stage
contract, re-audit is required before proceeding to verification.

## Files changed

- `apps/api/src/serve.rs` — F1, F2
- `apps/api/tests/transport_tests.rs` — F3 (full rewrite)
