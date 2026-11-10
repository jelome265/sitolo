---
name: rust-enterprise-frameworks
description: Enforced framework-specific practice for Rust services. Defines the approved stack (tokio, axum with tower and hyper, tonic, sqlx, reqwest, serde, tracing with OpenTelemetry, clap, rustls), pinned majors, version floors, MSRV floors, and banned alternatives (actix-web, rocket, warp, diesel, sea-orm, legacy hyper). Enforcement uses three mechanisms, strongest first, which are clippy disallowed-methods and disallowed-types hard bans, route and client inventory tests, and gate checks on Cargo.lock and sqlx offline artefacts. Use when adding or changing a web, RPC, database, HTTP client, TLS, serialisation, CLI, logging or telemetry dependency, writing an axum router, tonic service, sqlx query, reqwest client or OpenTelemetry setup, upgrading framework versions, or reviewing framework usage. Triggers include use axum, add a route, set up tonic, sqlx query, reqwest client, tokio spawn, OpenTelemetry setup, upgrade axum, framework policy, approved stack, clippy disallowed.
---

# Enterprise Rust Framework Enforcement (FW)

Frameworks are where most production Rust defects enter: a default with no timeout, a router with no body limit, a deprecated constructor, a second async runtime, a client that trusts every certificate. This skill makes framework practice enforced, not advisory. Each rule names the mechanism that enforces it and the evidence that verified the claim.

Read `../rust-enterprise-best-practices/SKILL.md` first. This skill refines BP rules for named frameworks and does not relax any of them. Hardening controls (SEC, RES, OBS) are in `rust-enterprise-harden`. Audit procedures are in `rust-enterprise-audit`.

## FW-0. The approved stack

Source of truth: `references/frameworks-policy.toml`. The gate reads it.

| Area | Crate | Pinned major or series | Version floor | MSRV | Rules |
|---|---|---|---|---|---|
| Async runtime | tokio | 1 | 1.53.2 | 1.71 | FW-TOK |
| HTTP server | axum (+ axum-core) | 0.8 (axum-core 0.5) | 0.8.9 (0.5.6) | 1.80 | FW-AXM |
| Middleware | tower | 0.5 | 0.5.3 | 1.64 | FW-TWR |
| Middleware (HTTP) | tower-http | 0.7 | 0.7.1 | 1.65 | FW-TWR, FW-AXM |
| HTTP protocol | hyper (+ hyper-util for servers) | 1 | 1.12.0 | 1.63 | FW-HYP |
| gRPC | tonic (+ prost) | 0.14 | 0.14.6 (prost 0.14.4) | 1.88 | FW-TNC |
| Database | sqlx | 0.9 | 0.9.0 | 1.94.0 | FW-SQX |
| Outbound HTTP | reqwest | 0.13 | 0.13.5 | 1.85.0 | FW-RQW |
| TLS | rustls | 0.23 | 0.23.45 | 1.71 | FW-RLS |
| Serialisation | serde, serde_json | 1 | 1.0.229, 1.0.154 | 1.56 | FW-SRD |
| CLI and config | clap | 4 | 4.6.7 | 1.85 | FW-CLP |
| Logging and spans | tracing | 0.1 | 0.1.44 | 1.65 | FW-TRC |
| Telemetry bridge | tracing-opentelemetry, opentelemetry, opentelemetry_sdk | 0.34 with 0.33 (paired) | 0.34.0, 0.33.1 | 1.75 | FW-OTL |

Repository MSRV floor: **1.94.0**, set by sqlx 0.9.0. The pinned toolchain (1.98.1) satisfies every floor.

**Banned** (the gate fails if any appears in `Cargo.lock`, transitively included): `actix-web`, `actix-rt`, `async-std`, `rocket`, `warp`, `diesel`, `sea-orm`, `native-tls`, `openssl`, `openssl-sys`. Hyper 0.14 is also rejected, because the hyper entry requires major 1 and checks every version in the lock.

Adding or replacing an approved framework requires an ADR naming the owner, the reason the stack cannot serve the need, and the migration cost (BP-DC-04). Editing the policy file is itself a reviewed change.

## FW-1. Enforcement mechanisms (strongest first)

**M1. Hard bans via clippy.** `clippy.toml` `disallowed-methods` and `disallowed-types`, with `disallowed_methods = "deny"` and `disallowed_types = "deny"` in `[workspace.lints.clippy]`. Clippy resolves each configured path and flags every call in the linted code. A violation is a compile-time error. Services use `references/clippy.service.toml`. Libraries use `references/clippy.library.toml`, which is stricter. Clippy honours the nearest `clippy.toml`, so a library crate can carry its own stricter bans.

**M2. Inventory tests.** Some rules cannot be expressed as a call ban: "every route has a permission", "every client has timeouts", "the route set has not changed without review". For these, all construction goes through one registry or builder that validates its input. Tests then assert the validation and compare the route inventory to a reviewed snapshot. `references/inventory_pattern.rs` implements the validators and the snapshot test. The axum and reqwest glue calls those validators before it builds anything.

**M3. Artefact gates.** `scripts/check_frameworks.py` runs in gate step 7b. It checks the lockfile against the policy (banned crates, version floors, majors, duplicates, pre-releases), the repository MSRV against the framework floor, and the sqlx offline cache when compile-time macros are used. Gate step 4 builds with `DATABASE_URL` unset, so sqlx checks must succeed from the committed cache.

Preference order: use M1 where a call or type can be named. Use M2 when the property is about configuration or inventory. Use M3 for artefacts and versions. A rule enforced only by review must say so.

## FW-GATE. Gate rules for the artefact check

- **FW-GATE-01 (BLOCKER).** No banned crate appears in `Cargo.lock`, at any depth.
- **FW-GATE-02 (BLOCKER).** Every version of every approved crate in the lock satisfies its floor and its major (or 0.x minor). Pre-release and build-metadata versions are rejected for policy purposes.
- **FW-GATE-03 (BLOCKER).** Each approved crate appears at most one version in the lock. Duplicates are unified before merge.
- **FW-GATE-04 (BLOCKER).** `rust-version` in the root manifest is at least `repo.min_rust_version`.
- **FW-GATE-05 (BLOCKER).** If sqlx compile-time macros are used, an offline cache directory with at least one JSON file is committed (see FW-SQX-02).
- **FW-LN-01 (BLOCKER).** A root `clippy.toml` with `disallowed-methods` is present (gate step 7b), and `disallowed_methods` and `disallowed_types` are deny in the workspace lints.

Verified behaviour: each rule above fired on a seeded violation (banned actix-web; duplicate and below-floor tokio; transitive hyper 0.14.32; repository MSRV 1.90 against a 1.94.0 floor; pre-release `0.7.1-rc.1`; sqlx macros without a cache), and the compliant lock passed. Gate step 7b was run from the installed skills layout.

## FW-TOK. Tokio

- **FW-TOK-01 (BLOCKER, M1/E6).** One runtime per process, created by `#[tokio::main]` or one explicit runtime in the binary. Libraries never create a runtime.
- **FW-TOK-02 (BLOCKER, M1).** `std::thread::sleep` is banned in async code. Use `tokio::time::sleep`. Verified: flagged in a real build.
- **FW-TOK-03 (BLOCKER, M1).** `tokio::spawn` is banned. Spawn through an owned `tokio::task::JoinSet`, or keep the `JoinHandle` and await or abort it at shutdown (BP-AS-04). Verified: `tokio::spawn` flagged, `JoinSet::spawn` not flagged, on tokio 1.53.2 built with the sandbox toolchain.
- **FW-TOK-04 (BLOCKER, M1).** `Runtime::block_on` is banned. The binary entry point is the only exception. `#[tokio::main]` expands to `block_on`, so the entry point carries a scoped `#[expect(clippy::disallowed_methods, reason = "...")]`. `expect` with a reason requires Rust 1.81 or later, which the pinned toolchain satisfies. Verified: `#[tokio::main]` triggers the ban, and an allow on the entry point suppresses only that expansion. An allow on a helper does not suppress `main`'s expansion.
- **FW-TOK-05 (BLOCKER, E4).** Only one async runtime may appear in the lock. `async-std` and `actix-rt` are banned (see FW-0).
- **FW-TOK-06 (MAJOR, E6).** Blocking work goes to `spawn_blocking`, or to a bounded pool (BP-CC-09). CPU parallelism uses a bounded pool, not per-request threads.
- **FW-TOK-07 (MAJOR, E6).** `tokio::select!` branches are cancel-safe (BP-AS-03). Check each branch against tokio's documentation for the primitive it awaits.
- **FW-TOK-08 (MAJOR, E6).** Shutdown uses `tokio::signal::ctrl_c` (verified in tokio 1.53.2) and SIGTERM on Unix, with a cancellation token and a bounded drain (RES-SHUT-01).

## FW-AXM. axum (with tower and hyper)

- **FW-AXM-01 (BLOCKER, M1/M2).** `axum::Router::route` is banned outside the route registry. Every route is declared with a `RouteSpec` (method, path, `Access`, body limit, timeout). `Access` has no implicit public variant: a public route declares `Access::Public { reason }`. The registry rejects duplicates. The reviewed snapshot is updated in the same PR as any new route. Verified: the validators and the snapshot test pass (12 tests, in the rustacean-conformance crate). The route ban relies on the same root re-export pattern verified on rustls, but that is a structural result: it was not run against axum. The real axum glue was not compiled, because axum 0.8 needs Rust 1.80 or later and the sandbox has 1.75.
- **FW-AXM-02 (BLOCKER, E6).** Every route sets a body limit. axum's default body limit is 2 MB (verified in axum-core 0.5.6 source). Set the limit per route with `DefaultBodyLimit::max(n)` (verified constructor). `DefaultBodyLimit::max` is `const fn` and takes a `usize`. Global limits are set at the outermost layer, and per-route limits are tighter, not looser. The limit comes from the route spec, so it cannot be omitted.
- **FW-AXM-03 (BLOCKER, E6).** Every request has a deadline. Use `tower_http::timeout::TimeoutLayer::with_status_code` (see FW-TWR-01) with a configured duration.
- **FW-AXM-04 (BLOCKER, E6).** Authentication is an extractor or layer that every protected route passes through. Deny-by-default is checked by the registry: a route with `Access::Permission` must not be reachable without the authenticated principal. An integration test enumerates the registry and sends each route a request with no credentials, expecting 401 or 403.
- **FW-AXM-05 (MAJOR, E6).** Serve with `axum::serve(listener, router).with_graceful_shutdown(signal)` (verified method). Drain is bounded (RES-SHUT-01).
- **FW-AXM-06 (MAJOR, E6).** Error responses are generic, carry a correlation ID, and never include internal error chains (SEC-LOG-03, SEC-LOG-04). Handler errors implement `IntoResponse` in one place.
- **FW-AXM-07 (MAJOR, E6).** Extractors that parse untrusted bodies (`Json`, `Form`, `Bytes`, `String`) run after the body limit applies. Prefer `Json` extractors over manual body reads.
- **FW-AXM-08 (MAJOR, E6).** Permissive CORS is forbidden. `CorsLayer` uses an explicit origin allowlist (CFG-03). `CorsLayer` was not API-verified in this session; check its builder against the pinned tower-http before use.

## FW-TWR. tower and tower-http

- **FW-TWR-01 (BLOCKER, M1).** `tower_http::timeout::TimeoutLayer::new` is banned. It is deprecated since tower-http 0.6.7 (verified in the 0.7.1 source), and the deprecation warning fails the build under `-D warnings`. Use `TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, duration)` (verified constructor).
- **FW-TWR-02 (BLOCKER, E6).** Request bodies are limited by middleware at the service boundary as well as per route. `RequestBodyLimitLayer::new(limit)` (verified) covers services that are not axum routers.
- **FW-TWR-03 (MAJOR, E6).** Layer order is part of the contract. In tower's `ServiceBuilder`, the first layer added is the outermost (tower's documented semantics; re-verify against the pinned tower version). The usual order is tracing, then request ID, then timeout, then body limit, then concurrency limit, then authentication, then the handler. Document the order in one place and test it.
- **FW-TWR-04 (MAJOR, E6).** Concurrency limits (`tower::limit::ConcurrencyLimitLayer`) are set per dependency and at ingress (RES-SHED-01, RES-BULK-01).

## FW-HYP. hyper

- **FW-HYP-01 (BLOCKER, E4).** hyper 1.x only (FW-0 and FW-GATE-02). hyper 0.14 is banned transitively.
- **FW-HYP-02 (BLOCKER, E6).** Direct server connections built with hyper-util's auto builder set `header_read_timeout` (verified method in hyper-util 0.1.21) and `max_buf_size` (verified). Without a header read timeout, slow-header attacks are not bounded (SEC-DOS-06).
- **FW-HYP-03 (MAJOR, E6).** Application code uses axum or tonic, not raw hyper, unless an ADR records why (for example a proxy). Raw hyper code is reviewed as high risk.

## FW-TNC. tonic (gRPC)

- **FW-TNC-01 (BLOCKER, E6).** Every `tonic::transport::Server` sets `timeout(...)` (verified), `concurrency_limit_per_connection(...)` (verified), and `max_frame_size(...)` (verified).
- **FW-TNC-02 (BLOCKER, E6).** Every generated service sets `max_decoding_message_size(...)` (verified on server and client gRPC types). The default is 4 MiB (verified: `DEFAULT_MAX_RECV_MESSAGE_SIZE = 4 * 1024 * 1024`). Set the limit explicitly from configuration, and never raise it without review.
- **FW-TNC-03 (BLOCKER, E6).** TLS uses `Server::tls_config(...)` (verified), backed by the approved rustls stack (FW-RLS). Plaintext gRPC is allowed only inside a documented trust boundary (SEC-TLS-05).
- **FW-TNC-04 (BLOCKER, E6).** Authentication is a tonic interceptor or a tower layer that runs before any handler. The same deny-by-default inventory as FW-AXM-04 applies to every gRPC method.
- **FW-TNC-05 (MAJOR, E6).** Protobuf schemas are versioned contracts (BP-SE-01). Field numbers are never reused. Breaking changes ship as new packages.

## FW-SQX. sqlx

- **FW-SQX-01 (BLOCKER, E5/E6).** Queries use bound parameters only: `query!`, `query_as!`, `query_scalar!`, or `query` with `.bind()`. A query string built with `format!` or concatenation from input is a finding (SEC-INJ-01). The gate pattern scan flags `query(&format!`.
- **FW-SQX-02 (BLOCKER, M3).** Compile-time macros require the offline cache. Offline support is part of sqlx 0.9's `macros` feature (verified: `macros` enables `sqlx-core/offline` and the driver offline features). Commit the `.sqlx/` directory. CI runs `cargo sqlx prepare --check` against a database that matches the schema, and the normal gate builds with `DATABASE_URL` unset. The offline check reads `.sqlx/*.json`. Verified: the gate fails when macros exist without a cache, and passes when the cache is present.
- **FW-SQX-03 (BLOCKER, E6).** Dynamic identifiers (table or column names) come from a closed enum, never from input (SEC-INJ-01).
- **FW-SQX-04 (BLOCKER, E6).** Pool configuration is explicit: `max_connections`, `acquire_timeout`, `max_lifetime`, and `idle_timeout` (all verified in sqlx-core 0.9.0). Defaults are not relied on. An acquire timeout bounds waiting for a connection (RES-TO-04).
- **FW-SQX-05 (BLOCKER, E6).** A server-side statement timeout is set per connection, for example in `PoolOptions::after_connect` (verified method). The statement timeout must be set in the database role or the connection setup; the exact SQL is database-specific and is recorded in the runbook.
- **FW-SQX-06 (BLOCKER, E6).** The runtime database role has no DDL rights. Migrations run as a separate job with a separate role (DB-02, OPS-MIG-01). Migration code is not run at application start by default.
- **FW-SQX-07 (MAJOR, E6).** Transactions do not contain network calls (DB-05). Money and state transitions are covered by the idempotency rules (RES-IDEM-03).

## FW-RQW. reqwest

- **FW-RQW-01 (BLOCKER, M1/M2).** `reqwest::Client::new` and `reqwest::Client::builder` are banned outside the policy builder. The builder sets `timeout`, `connect_timeout`, `read_timeout`, `pool_max_idle_per_host` (all verified in reqwest 0.13.5), `https_only(true)` (verified), and `tls_version_min` (verified). The policy values are validated by `validate_client_policy` in `inventory_pattern.rs` (tested: zero or inverted timeouts, unbounded idle pool, and plaintext are rejected).
- **FW-RQW-02 (BLOCKER, M1).** `reqwest::blocking::Client` is banned in service code. The ban fired on a stub with the same type path (structural; see FW-VERIFY).
- **FW-RQW-03 (BLOCKER, E6).** Redirects are explicit. `redirect(Policy::limited(n))` or `Policy::none()` (verified builder method exists). Credentials are never forwarded across origins (SEC-TLS-04).
- **FW-RQW-04 (MAJOR, E6).** Outbound URLs built from input pass the SSRF allowlist (SEC-INJ-03) before the client is called.
- **FW-RQW-05 (MAJOR, E6).** Default TLS is rustls in reqwest 0.13 (verified: `default-tls = ["rustls"]`). Native TLS is not enabled (FW-0 bans `native-tls` and `openssl`).

## FW-RLS. rustls

- **FW-RLS-01 (BLOCKER, E4).** rustls is the only TLS stack. `native-tls`, `openssl`, and `openssl-sys` are banned in the lock (FW-0). rustls 0.23 has a crypto provider choice (`ring` or `aws-lc-rs`). Select the provider explicitly in Cargo features and record the choice in an ADR, rather than relying on a default.
- **FW-RLS-02 (BLOCKER, M1).** `ClientConfig::dangerous` and `DangerousClientConfig::set_certificate_verifier` are banned (SEC-TLS-03). Verified: the root-level `rustls::ClientConfig::dangerous` path is flagged through a re-export-mirroring stub. Real rustls 0.23.45 could not be compiled on the sandbox toolchain, because a transitive dependency uses edition 2024.
- **FW-RLS-03 (BLOCKER, E6).** Trust roots are explicit: the system store or a documented private CA bundle. No empty root store in production configuration (`RootCertStore::empty()` is flagged in review).

## FW-SRD. serde and serde_json

- **FW-SRD-01 (BLOCKER, E6).** Input DTOs derive `Deserialize` with `#[serde(deny_unknown_fields)]` (BP-SE-02).
- **FW-SRD-02 (BLOCKER, E6).** Input size is checked before parsing (SEC-IN-03). The body limit in FW-AXM-02 is the first check.
- **FW-SRD-03 (BLOCKER, M1/M2).** The recursion limit is never disabled. `serde_json::de::Deserializer::disable_recursion_limit` is banned (verified in a real build with the `unbounded_depth` feature enabled). The feature itself must be denied: a Cargo feature that exposes the method is an audit finding, and CI must not enable `unbounded_depth` (checked by the lock review and by the audit skill). Verified: the method exists only behind that feature, so the ban alone is not sufficient.
- **FW-SRD-04 (MAJOR, E3).** Round-trip and golden-file tests exist for every wire type (BP-SE-04).

## FW-CLP. clap

- **FW-CLP-01 (BLOCKER, M1).** `clap::Parser::parse` and `clap::Command::get_matches` are banned (both are in clippy.service.toml). They exit the process on error. Use `try_parse`, `try_parse_from`, or `try_get_matches` (verified: `try_get_matches` exists in clap_builder 4.6.7; `Parser::parse` ban verified on a stub with the same trait path).
- **FW-CLP-02 (BLOCKER, M1).** `std::env::var` and `std::env::var_os` are banned in service code. Environment variables are read by clap's `env` attribute at the binary boundary (IMP-6.1, verified: `hide_env_values(true)` exists in clap_builder 4.6.7 for secrets).
- **FW-CLP-03 (MAJOR, E6).** Secrets are typed (`SecretString`) and hidden from help output with `hide_env_values` (BP-TY-09, SEC-SEC-02).

## FW-TRC. tracing

- **FW-TRC-01 (BLOCKER, E6).** The subscriber is installed once, in the telemetry module called from `main`. Libraries never install a subscriber.
- **FW-TRC-02 (BLOCKER, E6).** Fields are typed. Secret-typed values are never logged (BP-OB-02). Sensitive values never appear in span attributes (OBS-TRACE-03).
- **FW-TRC-03 (MAJOR, E6).** Spans wrap each inbound request and each outbound call. Span names are low-cardinality (BP-OB-03).
- **FW-TRC-04 (MAJOR, E6).** Errors are recorded once at the handling boundary (BP-OB-04).

## FW-OTL. OpenTelemetry

- **FW-OTL-01 (BLOCKER, E4).** `tracing-opentelemetry` 0.34 is paired with `opentelemetry` and `opentelemetry_sdk` 0.33 (verified: tracing-opentelemetry 0.34.0 depends on opentelemetry 0.33.0). The policy pairs them. A mismatched pair fails the gate's duplicate and floor checks, or fails to compile.
- **FW-OTL-02 (BLOCKER, E6).** The SDK provider is built once with `SdkTracerProvider::builder()` (verified) and `with_batch_exporter(...)` (verified). It is kept alive for the process and shut down on exit with `shutdown_with_timeout(...)` (verified) after `force_flush()` (verified). Shutdown is bounded.
- **FW-OTL-03 (BLOCKER, E6).** The exporter endpoint uses TLS or workload identity (SEC-TLS-05). Exporter credentials are secrets (SEC-SEC-04).
- **FW-OTL-04 (MAJOR, E6).** A sampling policy is written down (OBS-TRACE-03). Head sampling and tail sampling are not mixed without an ADR.

## FW-INV. Inventory and snapshot rules

- **FW-INV-01 (BLOCKER, M2).** Every route appears in the registry. Routes outside the registry are impossible to register because the ban applies (FW-AXM-01).
- **FW-INV-02 (BLOCKER, M2).** The route snapshot file is committed. CI fails if the registry output differs from the snapshot. Changes are reviewed as diffs.
- **FW-INV-03 (BLOCKER, M2).** Every outbound client is built through the policy builder. The client inventory is the set of builder calls, reviewed when it changes.

## FW-VER. Version policy and upgrades

- **FW-VER-01 (BLOCKER, E4).** Framework versions are pinned in `Cargo.lock`, and the lock is committed (BP-CG-01).
- **FW-VER-02 (BLOCKER, E6).** An upgrade changes the policy file in the same PR, with the release notes reviewed for deprecations, feature renames, and changed defaults. Deprecations are fixed in the same PR (the `deprecated` lint fails the build under `-D warnings`).
- **FW-VER-03 (MAJOR, E6).** Upgrades run the full gate, the framework gate, and the route and client inventory snapshot tests. Major upgrades run in a branch with a soak test in staging (OPS-REL-01).
- **FW-VER-04 (MAJOR, E6).** The repository MSRV is raised only when a framework floor requires it, and the change is recorded in an ADR (BP-TC-03).

## FW-VERIFY. Verification status of this skill

Every claim in this skill has one of four statuses. Only "verified" claims should be relied on without re-checking.

| Status | Meaning | Items |
|---|---|---|
| Verified (source) | Checked against the downloaded crate source at the pinned version | tokio JoinSet and spawn path; axum-core `DefaultBodyLimit::max` and the 2 MB default; axum `serve` graceful shutdown; tower-http `TimeoutLayer` deprecation and `with_status_code`; `RequestBodyLimitLayer::new`; hyper-util `header_read_timeout`, `max_buf_size`; tonic server `timeout`, `concurrency_limit_per_connection`, `max_frame_size`, `tls_config`, `max_decoding_message_size`, 4 MiB default; sqlx-core `max_connections`, `acquire_timeout`, `max_lifetime`, `idle_timeout`, `after_connect`; sqlx offline behaviour under `macros`; reqwest builder timeouts, `https_only`, `tls_version_min`, `pool_max_idle_per_host`, `redirect`, rustls default; rustls `ClientConfig::dangerous` and verifier setter; clap `hide_env_values`, `try_get_matches`; tokio `signal::ctrl_c`; OpenTelemetry SDK provider methods; tracing-opentelemetry 0.34 to opentelemetry 0.33 pairing; MSRV of every approved crate |
| Verified (test) | Enforced in a real build or test on the sandbox | tokio 1.53.2 built and `tokio::spawn` and `block_on` bans fired, `JoinSet` not flagged; `#[tokio::main]` false positive and allow scoping; per-crate `clippy.toml` honoured; serde_json 1.0.154 `disable_recursion_limit` ban fired; `disallowed-types` fired; inventory validators, snapshot and client policy (12 tests in the rustacean-conformance crate, clippy-clean under the strict workspace policy on an edition 2021 proxy); framework gate (13 cases including banned, duplicate, floor, MSRV, pre-release, sqlx cache); gate step 7b from the installed layout |
| Structural (stub) | Path resolution tested with a stub that mirrors the crate's module layout, not the real crate | rustls root re-export path (`rustls::ClientConfig::dangerous`); clap `Parser::parse` trait path; reqwest `Client::new` path; reqwest blocking type; axum `Router::route` re-export layout (the same pattern as rustls, not run against axum) |
| Unverified | Not compiled or run here. Re-verify before relying on it | axum route glue compiled against axum 0.8.9 (MSRV 1.80, above the sandbox's 1.75); reqwest and rustls real builds (MSRV 1.85 and a transitive edition 2024 dependency); `cargo-deny` configuration; `CorsLayer` builder; the `set_certificate_verifier` path under a real build; `std::env::var` in clap's derive expansion; tonic and hyper real builds; `cargo sqlx prepare --check` in CI |

## FW-LIMIT. Limits of the three mechanisms

- **Clippy sees resolved calls only.** A ban on `tokio::spawn` does not stop a dependency from spawning, and it does not stop code reached through a re-export under a different path that the configured path does not match. The verified re-export cases work; other aliasing is not covered. Dependencies are constrained by the policy and the lock, not by clippy.
- **Clippy sees macro expansions from the linted crate.** A macro defined in another crate that calls a banned function is flagged in the expansion site only where the path resolves. Test each macro-heavy framework path before relying on it.
- **Module-scoped rules need allows.** `std::env::var` is banned workspace-wide. The config module carries a scoped allow with a reason, and nowhere else.
- **Inventory tests depend on discipline.** They prove that registered routes are valid. They do not prove that every route is registered. The route ban and the snapshot together cover that gap. Remove either and the gap reopens.
- **The gate proves artefacts, not behaviour.** A correct timeout value is a configuration decision. The gate checks that a timeout exists, not that it is right.

## FW-DONE. Done criteria for framework-touching changes

- [ ] Every framework used is in the approved table, at a version satisfying its floor and major.
- [ ] `clippy.toml` at the workspace root is the service profile; each library crate has its own stricter profile.
- [ ] No banned call or type appears; every scoped allow has a reason and sits on the smallest item.
- [ ] Every route is in the registry, has a permission or an explicit public reason, a body limit, and a timeout; the snapshot is updated in this PR.
- [ ] Every outbound client is built by the policy builder with timeouts, `https_only`, and a bounded pool.
- [ ] Every gRPC server sets timeout, concurrency, frame size, and decoding limit.
- [ ] sqlx queries are bound; `.sqlx/` is committed and current; pool and statement timeouts are set.
- [ ] tracing and OpenTelemetry versions are paired; the provider shuts down with a bounded timeout.
- [ ] `check_frameworks.py` passes with no violation.
- [ ] The MSRV is at least the framework floor.
- [ ] Deprecations introduced by an upgrade are fixed in the same PR.

## References

- `references/frameworks-policy.toml`: approved versions, floors, MSRVs, banned crates. Read by the gate.
- `references/clippy.service.toml`: hard bans for services and binaries. Copy to the workspace root as `clippy.toml`.
- `references/clippy.library.toml`: stricter bans for library crates. Copy to a library crate directory as `clippy.toml`.
- `references/inventory_pattern.rs`: route and client validators with tests, and the reviewed snapshot pattern.
- `scripts/check_frameworks.py`: the artefact gate (Python 3.11 or later).
