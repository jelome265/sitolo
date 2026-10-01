# PR-68 Transport Remediation

Run-slug: `pr-68-transport`
Source branch: `feat/threat-model-axum-reconciliation-20260926`
Working branch: `remediation/pr-68-transport-hardening`
Audit input: `workspaces/sitolo-engineering/stages/06-audit/output/pr-68-transport-audit.md`
(audited PR head `c4130cecf9684ea57172ab2a9a039cf04c064228`)

Ordered per `references/remediation-order.md`.

## Important: this branch had a broken intermediate commit

Between this remediation's first pass and this one, commit `daabd24`
("feat: close all P0/P1/P2 transport audit findings") was pushed directly
to the source branch. It claimed to close every finding but did not
compile: duplicate struct fields in `crates/sitolo-config/src/model.rs`
(`request_timeout_ms` and five siblings each declared twice in `Builder`),
a malformed `field!` macro invocation in `field.rs`, six fields missing
from the final `AppConfig` construction in `validate.rs`, and a reference
to an undefined `transport.http2_keep_alive_timeout` field in `serve.rs`.
It also deleted `dispatch_request`/`dispatch_request_without_content_length`
and the router's CI-gate-pinned two-argument signature, both still
required by `apps/api/tests/tenancy_api.rs` and
`scripts/ci/check-api-architecture`. Its own `transport_tests.rs` bound a
`TcpListener` that nothing ever called `.accept()` on, and four of its six
tests were `assert!(true, "...")` placeholders. Two files claiming
`Status: COMPLETED` / `Status: VERIFIED` were added alongside this,
under a duplicate `stages/07-remediation/` and `stages/08-verification/`
pair that does not match the `07-remediate` directory this workspace's own
`CONTEXT.md` contract points to. Those two files have been overwritten
with corrections; they are not deleted, so the false claim stays visible
in history. This remediation is built on top of `daabd24`'s real ideas
(the `Readiness`/`ReadinessState` abstraction, `TraceLayer`, and
`ResponseBodyTimeoutLayer` were all sound design choices) with the actual
defects fixed, not discarded wholesale.

## 1. Release-blocking integrity defects

### Compile-blocking defects introduced by `daabd24` — fixed

- Deduplicated the six doubled fields in `defaults::Builder`
  (`crates/sitolo-config/src/model.rs`).
- Fixed the malformed `field!(...)` macro call in `field.rs` (bare string
  literals had been stacked into a single call's key argument).
- Added the six missing fields to the final `Ok(AppConfig { .. })`
  construction in `validate.rs` (silently absent — `E0063`).
- Added the missing `http2_keep_alive_timeout: Duration` field to
  `HttpTransportConfig` in `serve.rs` (was read but never declared).
- Restored `router()`'s CI-pinned two-argument signature
  (`scripts/ci/check-api-architecture` and `apps/api/tests/tenancy_api.rs`
  both require it) via a `router_with_transport` internal split, same
  pattern as this remediation's first pass.
- Restored `dispatch_request` / `dispatch_request_without_content_length`
  and the `#[cfg(test)] mod tests` block, all deleted by `daabd24` and all
  still depended on by `tenancy_api.rs`.
- Restored the `shutdown.rs` test module (`ShutdownCoordinator` tests),
  also deleted by `daabd24`, and added equivalent coverage for the new
  `Readiness` type.

### P0 #1 — `/process/ready` was liveness under a different name — Fixed

`Readiness`/`ReadinessState` (`Initializing`/`Ready`/`Draining`/`ShutDown`)
lives on `AppState`. `bootstrap.rs` calls `mark_ready()` only after every
startup check passes; `serve.rs` calls `mark_draining()` the instant the
shutdown signal arrives, before the accept loop even breaks. `/process/ready`
returns 503 for anything other than `Ready`.

### P0 #2 — TCP keepalive / HTTP/1 idle / HTTP/2 PING conflation — Fixed, and corrected

`crates/sitolo-config` carries six distinct typed, validated, ceiling-bound
fields instead of one reused value: `request_timeout_ms`,
`request_body_idle_timeout_ms`, `http1_idle_timeout_ms`,
`http2_ping_interval_ms`, `http2_keep_alive_timeout_ms`,
`response_body_timeout_ms`.

`daabd24`'s HTTP/1 idle-eviction mechanism was a bare `tokio::time::sleep`
raced against the connection future once at connection open — it never
reset on activity, so it would have force-drained any connection that
merely stayed open longer than `http1_idle_timeout`, active or not. That
is a real functional defect, not just a style issue: it contradicts the
audit's own definition of "idle." Replaced with `IdleTimeoutIo<T>`, an
`AsyncRead`/`AsyncWrite` wrapper (implemented against the raw traits only —
no `AsyncReadExt`/`AsyncWriteExt`/`stream.read(`, preserving the
`check-api-architecture` gate) that resets its deadline on every successful
read. `transport_tests.rs::real_socket_active_connection_survives_past_idle_timeout`
proves a connection making requests throughout the window is never evicted;
`real_socket_idle_connection_is_evicted_after_configured_timeout` proves a
genuinely idle one is.

### P0 #3 — shutdown was bounded but not protocol-graceful — Fixed

A `tokio::sync::watch` broadcast, fired the instant the accept loop breaks,
races each connection task's `hyper_util` `Connection::graceful_shutdown()`
against the connection future via `tokio::select!`. The bounded
`abort_all()` hard-stop fallback is unchanged.

### P0 #4 — transport-level tests were materially incomplete — Fixed

`transport_tests.rs` was rewritten in full. Every one of the six tests
that were `assert!(true, "...")` placeholders, or bound a listener nothing
ever accepted on, now spawns the real `serve()` future and drives it over
a real socket:

- `real_socket_http1_persistent_connection` — two requests, one socket.
- `real_socket_slow_header_is_bounded` — a connection that never finishes
  its headers is closed within the configured timeout, not held forever.
- `real_socket_idle_connection_is_evicted_after_configured_timeout` /
  `real_socket_active_connection_survives_past_idle_timeout` — D-1 closure
  (see below); proves both halves of the idle-eviction claim.
- `real_socket_oversized_body_is_413`.
- `real_socket_stalled_body_is_bounded` — a body that stalls mid-stream is
  bounded rather than hanging (the test deliberately does not pin the exact
  terminal response, since that is genuinely implementation-defined; it
  does assert the connection cannot hang past the bound).
- `real_socket_http2_preface_is_recognized` — sends the RFC 9113 §3.4
  connection preface plus an empty SETTINGS frame on a plaintext socket and
  asserts the server responds with its own SETTINGS frame (frame type
  `0x04`), proving `hyper_util`'s `auto::Builder` actually negotiates h2c
  prior-knowledge on this listener. This is a preface/SETTINGS-level proof,
  not a full request/response cycle — doing that honestly would need a
  real `h2` client dependency, which was not available to add in this
  sandbox (no `cargo`, so no way to update `Cargo.lock`). Flagged as D-1
  residual below.
- `real_socket_open_connection_drains_cleanly_on_shutdown`.

`startup.rs` additionally gained
`ready_reports_unavailable_while_draining_and_shutdown_still_completes`,
proving the P0 #1/#3 ordering directly.

## 2. Contract violations

### P1 #7 — inconsistent 413 vs 422 classification — Fixed

`create_branch` now classifies a `PAYLOAD_TOO_LARGE` `JsonRejection`
identically to `provision_organization`.

## 3. Data/transaction/concurrency/retry defects

Not in scope; unchanged from the original audit's own scoping decision.

## 4. Missing security or negative tests

### D-1 (residual) — full HTTP/2 request/response real-socket proof

The preface/SETTINGS-level proof above is genuine but partial. A complete
proof (an actual request over an h2 stream, receiving a real response)
needs an `h2`-capable client dependency that this sandbox could not add
(no local Cargo toolchain to regenerate `Cargo.lock` against crates.io, even
though the registry itself is reachable). Owner: whoever next has a working
`cargo` in this repo — add `h2` (or equivalent) as a dev-dependency and
extend this test.

## 5. Operational and observability gaps

### P1 #12 — HTTP request observability — Substantially fixed

`daabd24` had already added `TraceLayer`; kept. Went further to close the
audit's *specific* complaint — "creating a new request ID inside an error
formatter is not equivalent to propagating the original request identity
through the complete lifecycle" — which `daabd24`'s version still did
exactly (its `make_span_with` derived an ID from headers *or generated one
inline*, and `format_error_response` separately called
`RequestId::new_server()` again, so the span and the error body could
disagram on the request's own identity). Fixed by adding a
`ServiceBuilder::map_request` step, applied before `TraceLayer`, that
assigns the request identity exactly once (reusing a well-formed client
`x-request-id` if present) into request extensions. `TraceLayer`'s span,
every handler, and `format_error_response` now all read the *same*
`Extension<RequestId>` rather than each deriving their own.

### D-2 (residual) — not fully wired

Two things remain open, honestly:

- Requests intercepted by `TimeoutLayer` / `RequestBodyLimitLayer` (both
  applied in a second, separate `.layer(...)` call, a pre-existing split
  from before this remediation) never reach the tracing layer, so a timed-
  out or oversized request currently produces no trace span. This is a
  minor observability gap, not a correctness defect — those two layers
  already respond with the correct status codes independently of tracing.
- Request completion telemetry goes to `tracing`, not into
  `sitolo_observability::TelemetryBuffer`. Whether business-audit-relevant
  HTTP telemetry belongs in that buffer at all is a design question this
  remediation pass is not positioned to answer unilaterally; flagging for
  Stage 04 planning rather than guessing.

## 6. Documentation drift

### D-3 (residual) — mechanical, human/CI step

`docs/threat_model.md` is integrity-pinned by
`docs/threat_model.integrity.json`'s `git_blob_sha`, checked by
`scripts/ci/check-threat-model-integrity`. Recomputing that hash requires
`git hash-object` against the final committed blob; this sandbox was
instructed not to run anything destructive/exploratory beyond what this
task needed, and has no local `cargo` besides. Owner: human maintainer at
merge time, alongside the System Map source-revision refresh
(`scripts/ci/generate-system-map-index`, `-routing`).

## 7. Optional hardening

Not pursued; nothing in this category was flagged FAIL by Stage 06.

## Files changed (this pass, on top of `daabd24`)

- `crates/sitolo-config/src/{model.rs,field.rs,validate.rs}` — dedup,
  macro fix, missing-field fix.
- `apps/api/src/state.rs`, `apps/api/src/shutdown.rs` — restored
  formatting/doc comments and test modules; added `Readiness` test coverage.
- `apps/api/src/serve.rs` — restored CI-pinned `router()` signature and
  test dispatch helpers; replaced the non-resetting idle timer with
  `IdleTimeoutIo`; added request-identity propagation via
  `ServiceBuilder::map_request` + request extensions.
- `apps/api/tests/transport_tests.rs` — full rewrite; six real tests
  replacing six broken/fake ones.
- `apps/api/tests/startup.rs` — added the readiness-during-drain proof.
- `workspaces/sitolo-engineering/stages/07-remediation/output/pr-68-remediation-report.md`,
  `workspaces/sitolo-engineering/stages/08-verification/output/pr-68-verification-report.md`
  — corrected from fabricated `COMPLETED`/`VERIFIED` claims.

## CI verification (actual results)

The authoring sandbox has no usable Rust toolchain (apt ships 1.75; this
workspace is edition 2024 pinned to 1.98.1; rustup hosts are blocked), so
every claim below was verified by running the repository's own gates on
GitHub Actions against the pinned toolchain, via the `integration.yml`
`workflow_dispatch` trigger on branch `remediation/pr-68-transport-hardening`.
`rust.yml` (the full `scripts/ci/verify`) is scoped to `main` only and cannot
be dispatched from a feature branch; a per-gate mirror of it was used instead
(`cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features
--locked -D warnings`, `cargo test --workspace --all-targets --all-features
--locked --exclude sitolo-persistence`, lockfile resolution, and the
`check-api-architecture` / `check-architecture` / `check-phase2-policy`
scripts).

Earlier state: run `36670737345` on `c55ba96` — policy, test, clippy, fmt and
lockfile all pass, and the official `non-database contract tests` job passed
on the preceding run `36670423550`.

### Defects that only real compilation/execution could expose

None of these were visible to hand review, and several were in code I had
already declared correct. Recorded so the remediation history is honest:

1. **Compile error, layer ordering.** `tower_http::timeout::Timeout<S>`
   requires the inner response body to be `Default`; `TimeoutBody` (from
   `ResponseBodyTimeoutLayer`) is not, only `limit::ResponseBody<B: Default>`
   is. `ResponseBodyTimeoutLayer` must wrap `TimeoutLayer`, not sit inside it.
2. **`--locked` failure.** Enabling tower-http's `trace` feature adds a
   `tower-http -> tracing` edge that `Cargo.lock` did not record.
3. **Merge drift.** A parallel fix attempt left `tenancy_api.rs` calling a
   3-argument `router()`; the signature is pinned to 2 arguments by
   `scripts/ci/check-api-architecture`. Reconciled.
4. **Production defect, silent hang on shutdown.** After the shutdown signal
   the accept loop stopped calling `accept()` but the `TcpListener` stayed
   open, so the kernel kept completing handshakes and clients connecting
   during drain hung forever. `serve()` now drops the listener immediately.
5. **Test-validity defects.** The idle-eviction tests ran with a 200ms
   header-read timeout under a 500ms idle policy; hyper's header timer also
   covers the wait for the next keep-alive request, so it closed connections
   first. One test failed, and its sibling could have passed without ever
   exercising `IdleTimeoutIo`. Both now use `idle_isolated_transport()`.
6. **rustfmt.** Several diffs in new code; `cargo fmt --check` is a gate.

Operational note surfaced by (5): for HTTP/1 the effective idle bound is the
smaller of `request_header_timeout` and `http1_idle_timeout`. With the
defaults (5s header, 60s idle) the header timer is the binding limit for a
keep-alive connection that goes quiet; `http1_idle_timeout` binds when it is
shorter, and bounds a peer that trickles bytes without completing a request.
Documented on `HttpTransportConfig`.

## Additional findings closed from the independent re-audit

- **Config fingerprint omitted the six new timeout fields** (re-audit #5), so
  a timeout-policy change left the fingerprint unchanged. Added to
  `canonical_non_secret_config` with per-field regression coverage.
  `FINGERPRINT_FORMAT_VERSION` stays 1: these fields exist only on this
  unmerged branch, so no deployed fingerprint changes, and the fingerprint
  contract (s41) says an incomplete fingerprint is corrected, not preserved.
- **Connection-ceiling resource-pressure test** added (admits exactly
  `MAX_IN_FLIGHT_CONNECTIONS`, refuses the next, recovers on release).

## Also closed after the re-audit (verified by CI)

- **Full HTTP/2 proof** with a real `h2` client: two concurrent streams on one
  connection both answered by the production router, and an HTTP/2 request
  body reaching the real Axum JSON extractor with the client's `x-request-id`
  echoed in the `problem+json` body. `h2` is a dev-dependency only; it was
  already resolved via hyper (0.4.19, MIT, passes `deny.toml`), so it added one
  `Cargo.lock` edge and no new crate. The byte-level preface test remains as a
  lower-level check.
- **Request telemetry conforms to the registry** (`docs/telemetry/`): the
  registered `http.request.completed` event with `status`, `status_class` and
  `latency_ms`; the span carries `method` and `route_template` taken from
  Axum's `MatchedPath` (requires axum's dependency-free `matched-path`
  feature). The raw URI is no longer logged, since it embeds organization and
  branch ids that `redaction.yaml` forbids as labels.
- **Requests rejected by the body-limit/timeout layers are now observed.** The
  request-id + tracing layer is outermost, so a 413 produced by
  `RequestBodyLimitLayer` (handler never runs) still yields a completion event.
- **One request identity per request**, asserted against real captured
  `tracing-subscriber` output and the `problem+json` body, for both
  client-supplied and server-minted ids, and over both HTTP/1 and HTTP/2.

- **W3C `traceparent` extraction at the HTTP boundary.** A single,
  semantically valid header (`TraceParent::parse`: version `00`, non-zero ids,
  exact hex lengths) is recorded on the request span. Multiple `traceparent`
  headers are discarded, per the W3C rule against guessing which to trust;
  invalid values are ignored and never echoed into telemetry; a bad diagnostic
  header never changes the response. Correlation only, never authorization
  evidence. Covered by tests for valid, four invalid shapes, and duplicates,
  asserted against real captured output.

### Decision: request telemetry is NOT written into `TelemetryBuffer`

The re-audit asked for HTTP telemetry to reach the bounded buffer. I did not do
this, deliberately. `TelemetryBuffer` is documented as an admission/shedding
*model*, "not a record store", and nothing in the repository ever calls
`export_one()`; there is no exporter (no OpenTelemetry crate is resolved).
Pushing a record per request would fill it after `capacity` requests and pin
`state()` at `Blackout` permanently with no consumer, producing a
meaningless signal that looks like integration. That wiring belongs with the
exporter (Stage 04 planning), where draining and payload ownership exist.

## Still open (deliberately not claimed)

- **Slow response consumer test: not written, on purpose.** Every route's
  response is a small, fully buffered `Body::from(String)` / `Json`. A few
  hundred bytes fit in kernel socket buffers, so a stalled reader can never
  create backpressure and a test could only pass vacuously. The behavior is
  bounded by `ResponseBodyTimeoutLayer` (verified in the layer composition),
  which exists as defense for future streaming routes. When one exists, the
  test should be written against that route.
- **Exporter and `TelemetryBuffer` wiring** (see decision above).
- **Docs integrity (D-3).** `docs/threat_model.integrity.json` pins a git blob
  hash; recomputing it needs the final committed blob.
- **`rust.yml` (full `scripts/ci/verify`) is scoped to `main`** and cannot be
  dispatched from a feature branch, so `cargo deny` / `cargo audit` were not
  run here. Per-gate fmt, clippy, tests, lockfile and the architecture/policy
  scripts were.
- **Housekeeping before merge.** Duplicate stage directories
  `07-remediation` and `08-verification` (siblings of the contract-defined
  `07-remediate` and `08-verify`) contain retracted claims and should be
  reconciled by the workspace owner.

## Re-audit requirement

Implementation changed after the Stage 06 re-audit, so Stage 06 must run again
before Stage 08. Because a prior "verified" claim on this branch proved false,
the re-audit should read the diff and the CI runs above, not a status line.
