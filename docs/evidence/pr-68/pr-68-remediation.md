# PR #68 transport remediation: consolidated report

**Run slug:** pr-68
**Branch:** remediation/pr-68-transport-hardening (PR #72, base `main`)
**Audit inputs:** `docs/evidence/pr-68/pr-68-transport-audit.md` (original) and `docs/evidence/pr-68/pr-68-reaudit-20260927-audit.md` (independent re-audit)
**Re-audit of this state:** `docs/evidence/pr-68/pr-68-reaudit-20261003-audit.md`

**Status.** All fifteen handoff items are closed except #13 (re-running the T-001..T-030 control evidence), which was **not performed**. Stage 08 verification has **not** been performed. See the last section for what remains open.

## 1. Provenance

Two earlier attempts preceded this one, and neither could be trusted as written:

- Commit `daabd24` claimed to close every finding. It did not compile (duplicate struct fields, a malformed macro, six fields missing from a struct literal, a read of an undeclared field), deleted helpers that `tenancy_api.rs` depends on, and four of its six "real socket" tests were `assert!(true, ...)`. Two files in non-canonical directories asserted `COMPLETED` and `VERIFIED` with no evidence. Those claims are retracted and the files are removed.
- Commit `c27cbe5` was an independent parallel attempt. Its report is preserved verbatim in Appendix A. Its model code still contained the duplicate-field defect.

This pass rebuilt on top of both, kept the sound ideas (`Readiness`, request tracing, `ResponseBodyTimeoutLayer`), and fixed the rest.

**Artifacts moved out of the stage shelves.** The repository's own gate (`scripts/ci/check-icm-workspace`, run by `scripts/ci/verify`) requires every `stages/*/output/` directory to hold only `.gitkeep`. Commit `d2dbcce` committed the audit into that shelf, and `main`'s `rust.yml` went from green (`3289f10`) to red at exactly that commit. The run artifacts now live in `docs/evidence/pr-68/` (moved with `git mv`, content unchanged, history preserved). The two duplicate stage directories were deleted; only the contract-defined stage directories remain.

## 2. Disposition of the audit's fifteen handoff items

| # | Item | Status | Evidence |
|---|---|---|---|
| 1 | Real readiness and draining | Fixed | `Readiness` state published after startup; `/process/ready` returns 503 unless ready. `startup::ready_reports_unavailable_while_draining_and_shutdown_still_completes` |
| 2 | Separate TCP keepalive, HTTP/1 idle, HTTP/2 PING | Fixed | Distinct typed config fields. `IdleTimeoutIo` resets on every read, so a busy connection is never evicted. `real_socket_idle_connection_is_evicted_after_configured_timeout`, `real_socket_active_connection_survives_past_idle_timeout` |
| 3 | Protocol-aware graceful shutdown | Fixed | Hyper `graceful_shutdown` per connection, bounded drain, then abort. The listener is dropped at once so late connections are refused instead of hanging. `real_socket_active_connection_shutdown_test` |
| 4 | Real HTTP/1 persistence test | Fixed | `real_socket_http1_persistent_connection` |
| 5 | Real HTTP/2 socket tests | Fixed | A real `h2` client: two concurrent streams on one connection, and a request body reaching the handler. `real_socket_http2_multiplexes_requests_on_one_connection`, `real_socket_http2_request_body_reaches_the_handler_with_request_id` |
| 6 | Slow header and slow body tests | Fixed | `real_socket_slow_header_test`, `real_socket_slow_body_test` |
| 7 | Chunked oversized body test | Fixed | `real_socket_chunked_oversized_request_test` |
| 8 | Active-connection shutdown test | Fixed | as item 3 |
| 9 | Typed timeout configuration | Fixed | Six fields with ceilings, env parsing, validation, and a cross-field check. The config fingerprint now covers all of them (a re-audit finding), with per-field regression coverage |
| 10 | Response-body slow-consumer policy | Policy defined, **no test** | `ResponseBodyTimeoutLayer` with typed config, and a layer-order constraint (it must wrap `TimeoutLayer`). A test cannot be meaningful today: every response is a small, fully buffered body that fits socket buffers, so a stalled reader never creates backpressure. The test belongs with the first streaming route |
| 11 | Normalize oversized-body classification to 413 | Fixed | `create_branch` now matches `provision_organization` |
| 12 | End-to-end request telemetry | Fixed | See section 4 |
| 13 | Re-run T-001..T-030 evidence, downgrade unsupported claims | **Not performed** | The threat model was reviewed for the transport boundary only (section 5). T-001..T-030 are unchanged: neither re-verified nor downgraded |
| 14 | Refresh stale System Map revisions | Fixed | The `http-transport-processing` card now matches the current source and passes `check-system-map-processes` |
| 15 | Preserve the audit as evidence | Done | Moved with `git mv`, content and history intact |

## 3. Defects that only execution could expose

None of these were visible to hand review, and several were in code already declared correct.

1. **Compile error, layer order.** `tower_http::timeout::Timeout` requires the inner response body to be `Default`. `TimeoutBody` is not; `limit::ResponseBody<B: Default>` is. `ResponseBodyTimeoutLayer` must wrap `TimeoutLayer`.
2. **`--locked` failure.** Enabling tower-http's `trace` feature adds a `tower-http -> tracing` edge that `Cargo.lock` did not record.
3. **Merge drift.** `tenancy_api.rs` called a 3-argument `router()`; the signature is pinned to 2 arguments by `scripts/ci/check-api-architecture`.
4. **Production defect, silent hang on shutdown.** After the shutdown signal the accept loop stopped calling `accept()` but the listener stayed open. The kernel kept completing handshakes, so clients connecting during drain succeeded at the TCP layer and then hung forever. Found by a failing test, not by review.
5. **Invalid tests.** The idle-eviction tests ran under a 200 ms header timeout beneath a 500 ms idle policy. hyper's header timer also covers the wait for the next keep-alive request, so it closed connections first: one test failed and its sibling could have passed without ever exercising `IdleTimeoutIo`.
6. **Failures invisible to CI.** Running the repository's own `check-*` scripts locally, which the branch's CI never ran, found that this branch broke `check-doc-references` and `check-system-map-processes`, both green on `main`.

## 4. Request telemetry and the TelemetryBuffer

The earlier decision here was to *not* write telemetry into `TelemetryBuffer`, on the grounds that nothing drains it. That reasoning was right about the problem and wrong to stop there. `TelemetryBuffer` is an admission and shedding model with no record store, and its own documentation assigns payloads, ordering and drain to "the export integration". So the correct fix was to build that integration:

- `TelemetryExporter<T>` (`crates/sitolo-observability`) keeps one FIFO payload queue per priority class in lockstep with the buffer's decisions, so each class's queue length always equals the buffer's count. Bounded by `otel_max_queue`; the producer path takes only short critical sections; a failing sink releases capacity instead of wedging the queue; poisoned locks are recovered. A 20,000-step pseudo-random run asserts the lockstep and capacity invariants after every operation.
- An Axum `from_fn_with_state` middleware offers one registered `http.request.completed` record per request. It sits outside the timeout, body-limit and concurrency layers, so requests those layers reject (a 413, for example) are still recorded.
- A drain loop emits to the sink for the server's lifetime, reports shedding per priority, state transitions and export failures once per tick (never fed back into the queue), and performs a bounded final flush after traffic has drained.
- Liveness and readiness probes are lowest priority and shed first. A queue saturated with higher-priority records never blocks or alters a request.
- One request identity is assigned per request and shared by the span, the event and the `problem+json` body. A `traceparent` is accepted only when single and semantically valid.
- The raw URI is never logged: it embeds organization and branch identifiers that `docs/telemetry/redaction.yaml` forbids as labels. The matched route template is recorded instead.

The only sink is structured `tracing` output; there is no external collector exporter, and the `sitolo_telemetry_dropped_records_total` metric is not exposed because no metrics endpoint exists (the counters are available from the exporter snapshot).

## 5. Threat model

`docs/threat_model.md` section 8.1 requires re-review on material HTTP transport changes, and the model had no content on transport-level availability. Changes:

- New **T-031 HTTP Connection and Resource Exhaustion**, mapped to SC-004 and SC-010, with traceability, release-gate and incident-detection rows.
- New **section 5.1.1**: each transport control with its mechanism, configured bound and the executed test that exercises it, or an explicit statement that none exists. Gaps are listed as gaps (total request deadline, request concurrency ceiling, HTTP/2 PING enforcement and stream cap, TCP keepalive, response-body timeout).
- New **section 20.4**: what request telemetry implements today versus what is TARGET.
- Stated limits: ceilings are global, not per source; per-source limiting exists only inside application services, not at the transport boundary; the process does not terminate TLS.
- `scripts/ci/check-threat-model-integrity` and `scripts/ci/check-security-icm-policy` now require T-001..T-031. The integrity pin was recomputed.

**Correction.** An earlier version of this report said recomputing the integrity pin "needs the final committed blob". That was wrong: the pin is a git blob hash of the file's contents, computable at any time with `git hash-object`.

The status of SC-010 in `docs/security_control_register.md` was left at "Contracted". It also covers retries, queues and expensive work that this change does not touch, so upgrading it would overstate.

## 6. Evidence

**CI (GitHub Actions, pinned toolchain 1.98.1, `integration.yml` via `workflow_dispatch`).** Run 36819614391 on `c665c6d`: success. Run 37048001808 on `3a32151`: success, with per-gate output showing tests, clippy, lockfile and policy passing and rustfmt reporting four hunks, which were applied in `3020c5c`. Commits after `3a32151` changed Rust only by those rustfmt hunks. They were **not** re-run, because CI budget was reported exhausted.

**Repository `check-*` scripts, run locally against this branch and a clean `main`:**

| Script | main | this branch |
|---|---|---|
| check-api-architecture | pass | pass |
| check-architecture | pass | pass |
| check-doc-references | pass | pass |
| check-icm-workspace | **fail** (the artifact in the shelf) | pass |
| check-icm-workspace-contracts | pass | pass |
| check-security-icm-policy | pass | pass |
| check-system-map-processes | pass | pass |
| check-threat-model-integrity | pass | pass |
| check-workflow-policy | pass | pass |
| check-phase2-policy | passes on CI (vacuously, no ripgrep); fails where ripgrep is installed | same |

`check-phase2-policy` flags `DATABASE_URL` in `crates/sitolo-persistence/tests/rls_security_tests.rs`, but **only when ripgrep is installed**, as it was in the authoring sandbox. The flagged line pre-dates `main`'s last green `rust.yml` run and `rust.yml` installs no ripgrep, so on the CI runner the check passes vacuously (`rg ... || true`): the gate is fail-open there. This is not caused by this change and was not touched. An earlier version of this report called it a failure "identical on main"; that overstated it, because the failure depends on the environment.

## 7. Still open

- **T-001..T-030 re-verification** (handoff item 13). Not performed.
- **Not covered by a test:** the total request deadline, the request-concurrency ceiling, HTTP/2 PING enforcement and the stream cap, TCP keepalive, and the response-body timeout.
- **Per-source limiting at the transport boundary** does not exist; a single client can occupy every connection slot. TLS is not terminated in-process.
- **No external telemetry exporter** and no metrics endpoint.
- **`rust.yml` has not run on this branch.** It is scoped to `main`, and the PR had a merge conflict that suppressed pull-request workflows. Once the conflict is resolved every push to the PR triggers the full suite. `cargo deny` and `cargo audit` have therefore never run against these changes. Note that `main`'s own `verify` was already red before this PR.
- **Rotate the personal access token** pasted into the working session earlier. A search of the tree and full history found no credential committed.

## 8. Re-audit requirement

Implementation changed after the earlier re-audit, so Stage 06 must run again before Stage 08. The re-audit of this state is `docs/evidence/pr-68/pr-68-reaudit-20261003-audit.md`. It was written by the same agent that made the changes, so an independent pass is still recommended.

---

# Appendix A: earlier pass (commit c27cbe5), retained verbatim

Headings are demoted one level, and two paths to since-removed directories are shown as plain text. Nothing else is altered. This report was accurate about its own limits at the time, but the model defect it did not catch was still present at that commit, and the status line below is superseded by the sections above.

## PR #68 Transport Remediation Report

**Status:** PARTIAL — real fixes applied for the P0 blockers and most P1s; two
items remain genuinely open (see "Not remediated" below). Re-audit required
per stage contract step 8 before this proceeds to 08-verify.

### Provenance note on prior stage-07/08 artifacts

(removed file) (removed)
and (removed file) (removed) (non-canonical
paths — the contract's canonical names are `07-remediate` and `08-verify`)
each asserted "COMPLETED"/"VERIFIED" with no evidence. Checked against the
actual code, that was false: the branch did not compile (two separate
struct-field errors, detailed below) and 4 of the 6 "real socket" tests in
`transport_tests.rs` were `assert!(true, "<claim>")` — they asserted nothing.
This report does not build on those artifacts' claims; every item below was
independently re-verified by reading the current source.

### Findings addressed

#### F1 (P0, audit #3). Compile-breaking: duplicate struct fields in `HttpTransportConfig::from_config`
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

#### F2 (P0/P1, related to audit #5). Compile-breaking: missing `http2_keep_alive_timeout` field
**Root cause:** `serve.rs`'s connection setup reads
`transport.http2_keep_alive_timeout`, but `HttpTransportConfig` never
declared that field and `from_config` never populated it, even though
`AppConfig.http2_keep_alive_timeout_ms` already existed with real env
parsing and ceiling validation. Second independent compile error.
**Fix:** Added the field to `HttpTransportConfig` and populated it from
`config.http2_keep_alive_timeout_ms` in `from_config`.
**Proof:** Manual review; matches the pattern of the other five timeout
fields exactly.

#### F3 (P0, audit #4). Transport tests materially incomplete / fabricated
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

#### F4 (P0, audit #1). `/process/ready` readiness — verified as genuinely real, no fix needed
Independently re-checked (not merely trusted from the prior fake report):
`ready()` in `serve.rs` calls `state.readiness().is_draining()` and
`.get_state()`, returning 503 for `Draining`/non-`Ready` and 200 only for
`Ready`. `bootstrap.rs` calls `state.readiness().mark_ready()` only after
every startup step succeeds; `serve.rs` calls `mark_draining()` on shutdown
signal. This is real, already correct on this branch prior to my changes.

#### F5 (P0, audit #2/#3). HTTP/1 idle eviction and protocol-graceful shutdown — verified as genuinely real, no fix needed
Independently re-checked: each accepted connection runs its own
`tokio::select!` racing the connection future against an idle-timeout sleep
and a shutdown watch-channel; both the idle and shutdown paths call
`conn.as_mut().graceful_shutdown()` (hyper-util's real GOAWAY-equivalent
graceful shutdown), not just `abort_all()`. `abort_all()` is only reached
after the bounded drain deadline as a last-resort cutoff, which is
appropriate. Real, already correct prior to my changes — F3's new
`real_socket_active_connection_shutdown_test` is the first thing to
actually prove this at the socket level.

#### F6 (P1, audit #7). Oversized-body 413 classification — verified as genuinely real, no fix needed
Both `provision_organization` and `create_branch` handlers in `serve.rs`
inspect the `JsonRejection`'s status and explicitly return
`AppError::PayloadTooLarge` for `413`, falling through to `Validation`
otherwise. Consistent across both endpoints. Real, already correct prior to
my changes.

### Not remediated (carried forward, not silently dropped)

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

### Verification limitations

**No Rust toolchain is available in this sandbox, and installing one is not
permitted here.** Every fix above was verified by careful manual reading of
the surrounding code (exact field names, trait signatures, and library APIs
cross-checked against `docs.rs` for `axum` 0.8.9 / `tower-http` where
relevant) — not by `cargo check`, `cargo build`, or `cargo test`. This is a
materially weaker form of proof than compilation and test execution, and
`08-verify`'s `./scripts/ci/verify` must be run for real (in CI or on a
machine with a toolchain) before this can be marked verified. Per the stage
contract, re-audit is required before proceeding to verification.

### Files changed

- `apps/api/src/serve.rs` — F1, F2
- `apps/api/tests/transport_tests.rs` — F3 (full rewrite)
