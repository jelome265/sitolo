---
artifact_type: audit-report
status: complete
run_slug: "pr-68-reaudit-20261003"
---

# Audit Report: PR #68 transport remediation (re-audit of the remediated branch)

## Scope

**Audited:** branch `remediation/pr-68-transport-hardening` (PR #72), starting from tip `3020c5c`, plus the working-tree changes made during this audit (poison recovery in `apps/api/src/shutdown.rs`, relocation of run artifacts, threat-model and System Map updates).

**Inputs:** the original audit (`docs/evidence/pr-68/pr-68-transport-audit.md`), the earlier independent re-audit (`docs/evidence/pr-68/pr-68-reaudit-20260927-audit.md`), and the consolidated remediation report (`docs/evidence/pr-68/pr-68-remediation.md`).

**Limits of this audit, stated first because they bound every disposition:**

1. **Not independent.** It was written by the same agent that made most of the changes. The stage contract calls for an independent review; this one is a structured self-review and an independent pass is still recommended.
2. **Nothing was compiled by the auditor.** The authoring environment has no usable Rust toolchain. Compilation and test execution evidence comes only from GitHub Actions runs 36819614391 (`c665c6d`) and 37048001808 (`3a32151`). Commits after those changed Rust only by rustfmt hunks reported by the latter run, plus the `shutdown.rs` change made during this audit. **Those have not been compiled or run.**
3. **`rust.yml` first ran at `226f8c3`**, after the PR conflict was cleared, and failed; see the findings below. Its result for the final head is shown by the PR checks and is not claimed here.
4. Per the contract, green CI and test names are not treated as proof. Each disposition below cites the code path inspected directly, using a mechanical interrogation of the source (field consistency across the five config files, layer order with comments stripped, handler and identity propagation, hygiene of the diff).

## Requirement to implementation to evidence matrix

| Requirement | Implementation (inspected) | Evidence | Status |
|---|---|---|---|
| Readiness reflects lifecycle | `Readiness` starts `Initializing`; bootstrap publishes `Ready` only after all startup checks; `/process/ready` returns 503 unless `Ready`; `mark_draining()` runs at the shutdown signal before the accept loop exits | `startup::ready_reports_unavailable_while_draining_and_shutdown_still_completes`; CI run 37048001808 | **PASS** for lifecycle. Dependency checks are **N/A today**: the serving path has no live dependency (`TenancyDatabase::new()` is in-memory; the database is described as non-connecting intent). `docs/observability_spec.md` section 27.2 allows database connectivity as a readiness input, so this **must** be extended when persistence is wired |
| Bounded, graceful shutdown | Listener dropped immediately; each connection receives `graceful_shutdown()`; drain bounded by `SHUTDOWN_DRAIN_DEADLINE_SECS`; `abort_all()` fallback; telemetry flushed afterwards within `FINAL_FLUSH_DEADLINE` | `real_socket_active_connection_shutdown_test`; listener-drop defect found and fixed through a failing test | **PASS** |
| HTTP/1 idle eviction independent of TCP keepalive | `IdleTimeoutIo` wraps the socket, deadline resets on every successful read | `real_socket_idle_connection_is_evicted_after_configured_timeout`, `real_socket_active_connection_survives_past_idle_timeout` | **PASS**. Interaction documented: hyper's header timer also covers the wait for the next request, so the effective idle bound is the smaller of the two |
| Typed, validated timeout configuration | Six fields: declared once each in `AppConfig` and `Builder`, validated against a ceiling, constructed once, parsed, catalogued, and present in `ACCEPTED_KEYS` (all eight HTTP duration fields checked by script; catalogue and accepted-key sets equal) | Source interrogation; CI compile | **PASS** |
| Config fingerprint covers every effective field | All HTTP duration fields appear in `canonical_non_secret_config` (script found none missing) | `every_effective_non_secret_field_changes_fingerprint` with six new cases | **PASS** |
| Total request deadline | `TimeoutLayer` with typed config, 408 | Layer present; **no dedicated test** | **PARTIAL** |
| Request concurrency ceiling | `GlobalConcurrencyLimitLayer` at `MAX_IN_FLIGHT_REQUESTS` | **no dedicated test** | **PARTIAL** |
| Connection concurrency ceiling | Semaphore permit per connection; over-ceiling connections dropped at accept | `real_socket_connection_ceiling_rejects_then_recovers` (admits 128, refuses the next, recovers on release) | **PASS** |
| Request-body size cap and consistent 413 | `RequestBodyLimitLayer` (min of configured and tenancy maximum); `create_branch` and `provision_organization` classify identically | `real_socket_chunked_oversized_request_test`; `requests_rejected_by_the_body_limit_layer_are_still_observed` | **PASS** |
| Slow header and stalled body bounded | Hyper header timeout; `RequestBodyTimeoutLayer` | `real_socket_slow_header_test`, `real_socket_slow_body_test` | **PASS** |
| HTTP/2 serves real requests | Auto-negotiated h2c; stream cap and PING settings configured | `real_socket_http2_multiplexes_requests_on_one_connection`, `real_socket_http2_request_body_reaches_the_handler_with_request_id` (real `h2` client) | **PASS** for serving. **PARTIAL** for PING enforcement and the stream cap: configured, not exercised |
| Response-body slow-consumer protection | `ResponseBodyTimeoutLayer` wraps `TimeoutLayer` (order verified with comments stripped: it is added before it) | None possible today: every response is small and fully buffered, so a stalled reader cannot create backpressure | **PARTIAL**: policy implemented, unverifiable until a streaming route exists |
| One request identity per request | Assigned once in the outermost layer into request extensions; the span, completion event and `problem+json` all read it; `format_error_response` takes it as a parameter and never mints one; `RequestId::new_server` occurs once, in the assignment closure; four handlers extract it | `client_request_id_is_shared_by_the_log_and_the_error_body`, `server_minted_request_id_is_shared_by_the_log_and_the_error_body`; the HTTP/2 variant | **PASS** |
| `traceparent` handled strictly | Accepted only when single and valid per `TraceParent::parse`; multiples and malformed values discarded and never echoed | `valid_traceparent_is_recorded_on_the_request_span`, `invalid_traceparent_values_are_ignored_not_echoed_into_telemetry`, `multiple_traceparent_headers_are_discarded` | **PASS** |
| Telemetry conforms to the registry and redaction rules | Registered `http.request.completed` event; labels `method`, `route_template` (matched pattern or `unmatched`), `status_class`; the raw path is never emitted | `labels_use_the_route_template_never_user_controlled_identifiers`; the real-server test confirms an unknown path is reported as `unmatched` | **PASS** |
| Telemetry export is correct and bounded | `TelemetryExporter` keeps payload queues in lockstep with the buffer; lock order is fixed (`queues` then `buffer`); the sink runs with no lock held | 7 unit tests including a 20,000-step lockstep and capacity run; `probe_traffic_is_shed_before_normal_traffic_and_responses_are_unaffected` | **PASS** |
| Telemetry loss never blocks business operations | Producer takes short critical sections only; shed records are counted, not signalled; poisoned locks recovered | `a_queue_saturated_by_higher_priority_records_sheds_without_blocking_requests`; exporter poison test | **PASS** for the exporter. See finding 1 for `Readiness` |
| Telemetry reaches a collector or metric | The only sink is structured `tracing` output; no collector exporter; the dropped-records metric has no endpoint | Source inspection | **PARTIAL** (TARGET, documented in the threat model) |
| Production path never uses development defaults | `router()` is the pinned two-argument shim with development defaults; its only callers are test-support adapters and tests; `main` calls `serve()` with real config | Source search | **PASS** |
| System Map reflects the code | `http-transport-processing` rewritten against current code with verified line citations | `scripts/ci/check-system-map-processes` passes | **PASS** |
| Stage shelves follow the ICM rule | Run artifacts moved out of every `stages/*/output/`; the two stray stage directories removed | `scripts/ci/check-icm-workspace` (see gate table) | **PASS** |
| Tests are real | No placeholder assertions in any test file (search; the only match is a comment saying there are none); every transport test drives the real `serve()` over a real socket | Source search | **PASS** |
| Repository hygiene | 0 TODO/FIXME in changed files; 0 credential patterns in the tree or history; no `unsafe`; no `println!`/`dbg!` in new production code | Source search | **PASS** |
| T-001..T-030 control evidence matches runtime | Not re-verified | n/a | **NOT PERFORMED** (handoff item 13) |

## Security control trace

| Control ID | Requirement | Implementation | Security test/evidence | Status | Residual risk/exception |
|---|---|---|---|---|---|
| SC-004 | Inputs and API boundaries are bounded | Body cap, header and body deadlines, 413 classification, request-id validation, strict `traceparent` | Real-socket and request-level tests listed above | **PARTIAL** | Total request deadline has no test |
| SC-010 | Resource limits bound requests, concurrency, queues and expensive work; uncertainty fails closed | Connection and request ceilings, idle and header timers, bounded telemetry queue with shedding | Connection-ceiling test; exporter unit tests | **PARTIAL** | Ceilings are global, not per source; request ceiling and HTTP/2 PING/stream cap untested. The register status was left at "Contracted" on purpose: SC-010 also covers retries and expensive work, which this change does not touch |

## Security and integrity findings

1. **Readiness mutex could panic on the request path** (fixed in this audit, **unverified by CI**). `Readiness` used `Mutex::lock().expect(...)` in three places, so a poisoned lock would have taken down `/process/ready`. This is the opposite of the poison-recovery rule applied to the exporter. It now recovers from poisoning, with a regression test. Neither the change nor the test has been compiled or run.
2. **Connection-ceiling rejections log one warning per rejected connection, including the peer address.** This line predates this work. Under a connection flood it produces unbounded warning volume, an amplification path for the very attack the ceiling defends against, and it records client addresses. Recommended: count rejections in the exporter and report an aggregate per interval. Not fixed here.
3. **Ceilings are global, not per source.** One client can occupy all connection slots. Per-source abuse limiting exists only inside application services (`sitolo-auth`), not at the transport boundary. Documented in the threat model as T-031 residual risk.
4. **At the maximum permitted `otel_max_queue` the queue can hold about a million records.** The default is 2,048. The ceiling is documented, but an operator choosing it trades memory for retention.
5. **`check-phase2-policy` is fail-open on the CI runner.** It runs `rg ... || true`; with no ripgrep installed the searches return nothing and the check passes without checking anything. This was found because the same script fails in an environment that does have ripgrep. It predates this change. Recommended: install ripgrep in `rust.yml` or fail closed when `rg` is absent, then resolve the line it will flag.
6. **A yanked dependency fails `cargo deny` on `main` and on this branch.** `yoke-derive 0.8.3` was yanked from crates.io after `main`'s last green `cargo deny` run. `main`'s own `Cargo.lock` pins the same version, so `main` fails the same check today and this is not caused by this branch. It enters through `sqlx` (url, idna, icu, yoke), not the HTTP stack. A scan of all 179 registry packages in the lockfile against the crates.io index found it to be the only yanked one. The branch bumps it to `0.8.4` (identical dependency list, satisfies `yoke`'s `^0.8.2`), a lockfile-only change; `main` was not touched.
7. **A rustfmt hunk in the re-audit's own `shutdown.rs` fix** stopped `rust / verify` at its fmt step. It is a defect in a change made during this audit, and it hid finding 6 because `verify` runs fmt before deny.

## Concurrency and failure findings

- **Lock ordering.** Producer, drain and snapshot all take `queues` then `buffer`. No path takes them in the opposite order, so no deadlock is possible. Confirmed by inspection and the lockstep run.
- **Cancellation.** If `serve()` is dropped, the telemetry task observes the closed stop channel (`changed()` resolves on sender drop), flushes and exits, so the task cannot leak.
- **Shutdown ordering.** Telemetry stops after `abort_all()`, so records for connections aborted at the deadline may be lost. Bounded loss at a bounded deadline; acceptable.
- **Shedding order.** The least important class is evicted first; the oldest payload of the evicted class is dropped. Verified by identity in a unit test, not just by counts.

## Documentation findings

- `docs/threat_model.md`: new T-031, section 5.1.1 and section 20.4; the integrity pin was recomputed and both gates now require T-001..T-031. The first draft claimed rate limiting had "no implementation behind it". That was wrong: `sitolo-auth` has a `RateLimiter`. It was found by re-checking the claim, and corrected.
- `docs/current_implementation_status.md` and `docs/observability_spec.md` contain no claim that this change contradicts. The spec's readiness section (27.2) permits dependency checks; see the readiness row above.
- An earlier report said the integrity pin could only be recomputed from a committed blob. That was wrong (it is a plain blob hash of the file). The consolidated report records the correction.
- `docs/evidence/pr-68/pr-68-reaudit-20260927-audit.md` had two dangling paths to deleted files. They were neutralized, and a post-audit note records that.

## Local gate results (this audit, executed)

Every `scripts/ci/check-*` script was run against the audited tree and, as a baseline, against a clean checkout of `main`. These are the repository's own gates; none require a Rust toolchain.

| Script | main | audited tree |
|---|---|---|
| check-api-architecture | pass | pass |
| check-architecture | pass | pass |
| check-doc-references | pass | pass |
| check-icm-workspace | **fail** | pass |
| check-icm-workspace-contracts | pass | pass |
| check-security-icm-policy | pass | pass |
| check-system-map-processes | pass | pass |
| check-threat-model-integrity | pass | pass |
| check-workflow-policy | pass | pass |
| check-phase2-policy | passes on CI (vacuously); fails with ripgrep installed | same |

`check-icm-workspace` was red on `main` (the audit artifact sat in a stage output shelf) and passes on the audited tree. `check-phase2-policy` flags `DATABASE_URL` in `crates/sitolo-persistence/tests/rls_security_tests.rs`, but **only when ripgrep is installed**, as it was in this audit's sandbox. The flagged line pre-dates `main`'s last green `rust.yml` run and `rust.yml` installs no ripgrep, so on the CI runner the check passes vacuously (`rg ... || true`): the gate is fail-open there. This is not caused by this change and was not touched. An earlier version of this report called it a failure "identical on main"; that overstated it, because the failure depends on the environment. The PowerShell scripts were not run.

## Required remediation

1. Confirm the PR's `rust`, `security`, `policy` and `integration` checks pass on the final head. At `226f8c3` `policy` and `integration` passed (compiling and running the `shutdown.rs` change); `rust` and `security` failed for the two reasons in findings 6 and 7, both since fixed. **Blocking before merge.**
2. Re-verify T-001..T-030 against the runtime boundary and downgrade unsupported claims (handoff item 13).
3. Add tests for the total request deadline, the request-concurrency ceiling, and HTTP/2 PING enforcement and the stream cap.
4. Replace the per-rejection warning (finding 2) with an aggregated count.
5. Extend readiness with dependency checks when persistence is wired into the serving path.
6. Provide per-source limiting at the transport boundary, at the edge or in-process.
7. Add a collector-backed sink and a metrics endpoint when an exporter exists.
8. Write the slow-response-consumer test against the first streaming route.

## Unresolved questions

- Does hyper send HTTP/2 PING frames while a connection has no open streams? If it does, PING acknowledgements count as read activity and an idle HTTP/2 client would never be evicted by the idle timer; if it does not (the believed default), idle HTTP/2 connections are evicted like HTTP/1. Not tested, so no claim is made either way.
- Should `check-phase2-policy` fail closed? It silently does nothing on a runner without ripgrep, and where ripgrep exists it flags a pre-existing line in the persistence tests. Which behavior is intended is a decision for the owner.
- PR #72 conflicts with `main` in one file (the audit document, which the branch carries with 18 appended lines). It must be resolved before pull-request workflows will run.
