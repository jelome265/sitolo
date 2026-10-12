# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main` at repository root
**Status:** Comprehensive End-to-End Enterprise Codebase Audit
**Target Posture:** Enterprise Production Readiness

---

## Executive Summary & Overall Health Assessment

Sitolo is designed as a multi-tenant business operating system for African SMEs, featuring a Rust modular monolith architecture (`Axum`, `Tokio`, `SQLx`, `PostgreSQL`).

An end-to-end evaluation across code structure, security posture, runtime behavior, data integrity, background processing, testing, and operational readiness indicates that while the system possesses **strong security-focused foundational primitives** (such as bounded tenancy identifiers, role/permission catalogs, session lifetime policy machines, strict configuration fingerprinting, and redacted security types), **the codebase in its current state is NOT production-ready**.

```text
CURRENT SYSTEM POSTURE:
┌───────────────────────────────────────┐
│     SECURITY & IAM PRIMITIVES         │  ← (Strong Foundation: Auth types, tenancy scope,
├───────────────────────────────────────┤     policy models, config fingerprinting)
│     TRANSPORT & API LAYER             │  ← (CRITICAL GAP: Custom TCP loop, missing HTTP auth,
├───────────────────────────────────────┤     divergence from Axum spec)
│     BUSINESS DOMAIN ENGINES           │  ← (CRITICAL GAP: Sales, Catalogue, Ledger, Payments,
├───────────────────────────────────────┤     Procurement, and Tax missing)
│     PERSISTENCE & BACKGROUND EXEC     │  ← (HIGH GAP: Unfinished worker loop, in-memory Mutex
└───────────────────────────────────────┘     bottlenecks, unexecuted PG migrations)
```

### Overall System Health Score: **42 / 100 (Unfinished Platform Foundation)**

- **Architecture & Boundaries (65/100):** Crate boundaries follow dependency direction rules, but HTTP transport implemented in `apps/api` diverges significantly from the documented Axum architecture.
- **Security & IAM (60/100):** Domain security models, session state machines, and configuration fingerprinting are well-designed. However, missing HTTP authentication middleware and missing Argon2id KDF adapters leave the API exposed.
- **API Transport & Routing (25/100):** Uses a raw TCP loop (`apps/api/src/serve.rs`) without proper HTTP parsing, chunked handling, header checks, or rate limiting middleware.
- **Domain Engine Completeness (15/100):** Core business logic (POS sales, stock management, invoicing, tax integration, ledger) is missing from `crates/sitolo-domain`.
- **Database & Persistence (45/100):** PostgreSQL RLS policies and authority schemas exist in test scripts, but SQLx production migrations runner and repository backends are incomplete.
- **Async & Background Processing (10/100):** Background worker (`apps/worker`) is an empty `main()` stub; event bus and transactional outbox engines are unimplemented.
- **Observability & Operational Readiness (40/100):** Structured log context and bounded telemetry buffers exist, but trace propagation and production metrics exporters are missing.

---

## Comprehensive Assessment Matrix

| Dimension | Current State | Assessment & Impact |
|---|---|---|
| **1. Repository & Boundaries** | **Partial** | Crate dependency topology is clean, but `apps/api` diverges from Axum specification. |
| **2. Application Architecture** | **Partial** | Tenancy and IAM service orchestration exists; core business domain engines are missing. |
| **3. API Design & Trust Boundaries** | **Critical Defect** | Custom TCP server bypasses auth checks; unauthenticated callers can trigger tenancy mutations. |
| **4. Auth & Session Handling** | **High Defect** | Strong session state machines, but missing Argon2id adapter and missing HTTP auth middleware. |
| **5. Input Validation & Integrity** | **Partial** | DTOs feature bounded length types; raw URL path segments are extracted without format checks. |
| **6. Injection, IDOR, Boundary Security** | **Critical Defect** | Unauthenticated endpoints allow IDOR on organization and branch operations by raw ID string. |
| **7. Error Handling & Timeout Strategy** | **Partial** | ProblemDetails format used; TCP stream errors swallowed without telemetry emission. |
| **8. CPU vs. I/O Bottlenecks** | **High Defect** | In-memory database locks single `Mutex` across all operations; async tasks risk blocking. |
| **9. Concurrency & Race Hazards** | **High Defect** | Immediate connection dropping under concurrency limit; PG RLS context leakage risks. |
| **10. Database, Transactions & RLS** | **Partial** | RLS schema designed in test fixtures; missing production migration pipeline. |
| **11. Caching Strategy** | **Not Implemented** | No Redis or distributed caching layer implemented. |
| **12. Queueing & Idempotency** | **Critical Defect** | Worker binary is empty; event bus and transactional outbox processing are missing. |
| **13. Observability** | **Partial** | Telemetry buffer exists; missing HTTP request tracing, latency metrics, and alerts. |
| **14. Test Coverage & Quality** | **Partial** | Unit tests pass for auth/config; PG security tests require live local database; E2E tests missing. |
| **15. Configuration & Secrets** | **Strong** | Pinned toolchain, secret redact types, and fingerprint validation are production-grade. |
| **16. Deployment Safety** | **Not Implemented** | Dockerfiles, Kubernetes/Helm manifests, and deployment pipelines are missing. |
| **17. Technical Debt & Quality** | **Partial** | Strict lints and `#![forbid(unsafe_code)]` enforced; empty scaffold crates remain. |
| **18. Maintainability & Ownership** | **Moderate** | Clear documentation index and architectural contracts facilitate expansion. |
| **19. Operational Compliance** | **Not Implemented** | MRA EIS fiscalization and audit export engines exist as contracts only. |

---

## Top 10 Highest-Risk Issues

1. **Unauthenticated Multi-Tenant API Mutations (Critical):** `apps/api/src/serve.rs` dispatches POST requests to tenancy endpoints without inspecting Authorization headers or verifying session tokens.
2. **Brittle Custom TCP HTTP Implementation (Critical):** `apps/api/src/serve.rs` replaces Axum/Hyper with a hand-written TCP read loop using a fixed 32KB buffer, vulnerable to request truncation, HTTP smuggling, and connection exhaustion.
3. **Missing Production Password Hashing KDF (Critical):** `crates/sitolo-auth/src/password.rs` only implements `TestPasswordHasher` (iterated SHA-256); no Argon2id adapter exists for production deployment.
4. **Unimplemented Background Worker Binary (Critical):** `apps/worker/src/main.rs` is an empty `fn main() {}` stub, preventing background job execution, email delivery, outbox sweeping, and asynchronous reconciliation.
5. **Missing Core Business Engines (High):** `crates/sitolo-domain` lacks domain logic for sales, product catalogue, inventory ledger, payments, and fiscal tax compliance.
6. **Single Mutex Throughput Bottleneck in Reference Store (High):** `crates/sitolo-persistence/src/memory.rs` serializes all database queries and session updates across threads under a single `Mutex<IdentityState>`.
7. **Deterministic Pseudo-Random Generation in Reference Database (High):** `IdentityDatabase::random_hex` generates deterministic token values using fixed byte arithmetic, creating predictable tokens if used in non-test runtimes.
8. **Silent Connection Dropping Under Load (High):** When active connections reach `MAX_IN_FLIGHT_CONNECTIONS` in `apps/api/src/serve.rs`, extra connections are immediately dropped without returning HTTP 503 or sending proper TCP resets.
9. **Missing Production Database Migration Pipeline (High):** RLS schema exists only in test fixtures (`crates/sitolo-persistence/tests/fixtures/rls_schema.sql`); no automated migration runner handles production deployments.
10. **Absence of HTTP Request Observability and Distributed Tracing (Medium):** Request latency histograms, HTTP status counters, and OpenTelemetry W3C trace propagation are absent from `apps/api`.

---

## Top 10 Highest-Leverage Fixes

1. **Adopt Axum Framework for API Transport:** Replace `apps/api/src/serve.rs` with standard Axum handlers, Hyper HTTP engine, and Tower middleware stack.
2. **Implement Production Argon2id Password Adapter:** Add an Argon2id adapter in `sitolo-auth` matching OWASP KDF guidelines (m=19456, t=2, p=1).
3. **Enforce Global Authentication and Tenancy Middleware:** Add Tower/Axum extractors that parse Bearer tokens, validate session state, and inject trusted `AuthorizedScope` into request extensions.
4. **Implement Transactional Outbox Sweeper in Worker:** Build an async outbox sweeper in `apps/worker` using PostgreSQL `FOR UPDATE SKIP LOCKED` row locking.
5. **Build Core Business Domain Engines:** Implement aggregates and state machines for Catalogue, Inventory Ledger, Sales/POS, Payments, and Fiscalization in `sitolo-domain`.
6. **Deploy SQLx Migration Engine:** Integrate SQLx migration runner into `apps/api` and `apps/worker` boot sequences to guarantee schema and RLS policy application.
7. **Refactor Reference State Locks:** Replace single `Mutex` in `IdentityDatabase` with concurrent lock-free maps (`dashmap`) or granular rwlocks per user/session.
8. **Strict OS CSPRNG Enforcement:** Mandate `sitolo_security::RandomSource` usage for all identifier and token generation outside explicitly marked test doubles.
9. **Integrate OpenTelemetry Tracing and Metrics:** Wire standard `tracing-opentelemetry` and `metrics-exporter-prometheus` middleware into Axum routes.
10. **Publish Production Deployment Artifacts:** Provide multi-stage Dockerfiles and Helm charts enforcing non-root execution and resource limits.

---

## Severity-Ranked Findings

### 1. Critical Severity Findings

#### FINDING-CRIT-01: Unauthenticated Multi-Tenant API Mutation Endpoints
* **What is wrong:** The API binary dispatches requests to organization and branch mutation endpoints without extracting or validating authentication credentials or session state.
* **Why it is wrong:** Any network client can issue POST requests to create, activate, suspend, resume, or close organizations and branches without presenting a valid token or proving authorization.
* **Where it appears:** `apps/api/src/serve.rs` in `dispatch_request()`.
* **How it fails in production:** An unauthenticated attacker sends a HTTP POST to `/v1/organizations/org-123/suspend` or `/v1/organizations/org-123/close`, halting an enterprise client's operations instantly.
* **What a proper fix looks like:** Re-architect the HTTP transport using Axum and Tower, inserting an authentication layer that validates session tokens and enforces `AuthorizedScope` before hitting handlers.
* **Priority & Blast Radius:** Priority: P0. Blast Radius: Critical (Complete compromise of multi-tenant administrative control and data isolation).

#### FINDING-CRIT-02: Non-Standard, Brittle TCP Request Loop for HTTP Transport
* **What is wrong:** `apps/api/src/serve.rs` uses a hand-rolled TCP socket listener that reads up to 32KB into a buffer and splits on `\r\n\r\n`.
* **Why it is wrong:** It violates the documented architectural decision (Rust + Axum + Tokio). It lacks support for chunked transfer encoding, streaming, HTTP/2, TLS termination, header parsing, and HTTP pipelining.
* **Where it appears:** `apps/api/src/serve.rs` in `handle_connection()`.
* **How it fails in production:** Requests larger than 32KB are truncated, resulting in JSON deserialization failures (`422 Unprocessable Entity`). Slowloris attacks or fragmented TCP packets stall connection handlers.
* **What a proper fix looks like:** Delete custom socket loop in `serve.rs` and depend on `axum` and `tokio::net::TcpListener`.
* **Priority & Blast Radius:** Priority: P0. Blast Radius: Critical (System-wide API instability, denial of service, and malformed request handling).

#### FINDING-CRIT-03: Missing Production Password Hashing KDF Adapter
* **What is wrong:** The codebase provides only `TestPasswordHasher` (iterated SHA-256 over static prefix). No Argon2id implementation exists.
* **Why it is wrong:** Storing password verifiers hashed with SHA-256 allows rapid offline dictionary and rainbow table attacks if verifiers are leaked.
* **Where it appears:** `crates/sitolo-auth/src/password.rs`.
* **How it fails in production:** If a production environment boots with `TestPasswordHasher`, user passwords are saved in weak SHA-256 format.
* **What a proper fix looks like:** Implement `Argon2idPasswordHasher` behind the `PasswordHasher` trait using the standard `argon2` crate with OWASP recommended parameters.
* **Priority & Blast Radius:** Priority: P0. Blast Radius: Critical (User credential compromise and security specification non-compliance).

#### FINDING-CRIT-04: Unimplemented Background Worker Loop
* **What is wrong:** The worker application entrypoint (`apps/worker/src/main.rs`) is an empty `fn main() {}` function.
* **Why it is wrong:** Asynchronous tasks such as outbox message processing, audit log archival, reconciliation, and integration webhooks will never execute.
* **Where it appears:** `apps/worker/src/main.rs`.
* **How it fails in production:** Outbox events pile up indefinitely in the database. Asynchronous payment confirmations and external tax registrations fail silently.
* **What a proper fix looks like:** Implement a production Tokio worker loop in `apps/worker/src/main.rs` that polls outbox tables using `FOR UPDATE SKIP LOCKED`.
* **Priority & Blast Radius:** Priority: P0. Blast Radius: Critical (Complete breakdown of asynchronous processing, integration delivery, and audit guarantees).

---

### 2. High Severity Findings

#### FINDING-HIGH-01: Incomplete Core Business Domain Logic
* **What is wrong:** `crates/sitolo-domain` exports only `tenancy.rs`. Domain logic for POS sales, product catalogue, inventory ledger, payments, supplier procurement, and MRA EIS tax integration is missing.
* **Why it is wrong:** The system cannot perform its primary function as a SME Business Operating System.
* **Where it appears:** `crates/sitolo-domain/src/lib.rs`.
* **How it fails in production:** Attempts to route API requests for product management, sales checkout, or inventory tracking result in `404 Not Found`.
* **What a proper fix looks like:** Implement domain aggregates, state transitions, and business invariants for core modules in `crates/sitolo-domain`.
* **Priority & Blast Radius:** Priority: P1. Blast Radius: High (Functional incapacity for core retail and accounting features).

#### FINDING-HIGH-02: Single-Mutex State Bottleneck in Memory Database
* **What is wrong:** `IdentityDatabase` in `crates/sitolo-persistence/src/memory.rs` wraps all user, session, device, refresh, and MFA state inside a single `std::sync::Mutex<IdentityState>`.
* **Why it is wrong:** Every single request (authentication, session check, token refresh) must acquire this exclusive lock, causing severe lock contention across multi-threaded Tokio runtimes.
* **Where it appears:** `crates/sitolo-persistence/src/memory.rs`.
* **How it fails in production:** High concurrent load results in thread starvation, excessive latency spikes, and Tokio worker thread blocking.
* **What a proper fix looks like:** Replace single `Mutex` with concurrent lock-free collections (`dashmap`) or granular read-write locks (`tokio::sync::RwLock`) per aggregate.
* **Priority & Blast Radius:** Priority: P1. Blast Radius: High (Severe performance degradation and throughput collapse under concurrent load).

#### FINDING-HIGH-03: Predictable Hex Generation in Reference Store
* **What is wrong:** `IdentityDatabase::random_hex` produces deterministic byte sequences derived from `(i as u8).wrapping_add(42)`.
* **Why it is wrong:** Session IDs, refresh tokens, challenge IDs, and device IDs generated by this function are fully predictable.
* **Where it appears:** `crates/sitolo-persistence/src/memory.rs` in `random_hex()`.
* **How it fails in production:** If reference persistence is used in staging or local instances, session tokens can be guessed by attackers to hijack sessions.
* **What a proper fix looks like:** Replace fixed byte loops with `sitolo_security::RandomSource` using OS CSPRNG (`rand::rngs::OsRng`).
* **Priority & Blast Radius:** Priority: P1. Blast Radius: High (Session hijacking and predictable credential creation).

#### FINDING-HIGH-04: Hard Dropping of Excess Connections Without HTTP Error Response
* **What is wrong:** In `apps/api/src/serve.rs`, when acquiring a permit from `Semaphore::new(MAX_IN_FLIGHT_CONNECTIONS)` fails, the connection loop executes `let Ok(permit) = ... else { continue };`.
* **Why it is wrong:** Incoming TCP connections above the threshold are ignored/dropped without reading the request or returning an HTTP `503 Service Unavailable` status.
* **Where it appears:** `apps/api/src/serve.rs` in `serve()`.
* **How it fails in production:** Clients experience abrupt TCP connection resets (`ECONNRESET`) or hanging connections without diagnostic feedback.
* **What a proper fix looks like:** Use Tower concurrency limit middleware that accepts the connection and returns HTTP 503 with Retry-After header when saturated.
* **Priority & Blast Radius:** Priority: P1. Blast Radius: High (Poor client experience and un-diagnosable capacity limit failures).

#### FINDING-HIGH-05: Unautomated PostgreSQL Schema and RLS Policy Migrations
* **What is wrong:** Production schema and Row-Level Security (RLS) policies exist only as a test SQL fixture script (`crates/sitolo-persistence/tests/fixtures/rls_schema.sql`).
* **Why it is wrong:** There is no automated SQLx migration management system integrated into the application deployment pipeline.
* **Where it appears:** `crates/sitolo-persistence/`.
* **How it fails in production:** Deploying the API against a clean PostgreSQL instance causes runtime errors due to missing tables, functions, or RLS policies.
* **What a proper fix looks like:** Set up standard `migrations/` folder with versioned SQL scripts and run `sqlx::migrate!()` on application startup.
* **Priority & Blast Radius:** Priority: P1. Blast Radius: High (Deployment failure and database schema drift).

---

### 3. Medium Severity Findings

#### FINDING-MED-01: Incomplete HTTP Request Telemetry and Context Propagation
* **What is wrong:** The custom HTTP transport does not construct OpenTelemetry tracing spans or record request latency metrics per route.
* **Why it is wrong:** Operators cannot monitor P95/P99 latency, request rates, or error rates per endpoint in Grafana/Prometheus.
* **Where it appears:** `apps/api/src/serve.rs`.
* **How it fails in production:** Incidents involving slow endpoints or 5xx spikes cannot be diagnosed efficiently due to missing metric metrics and distributed trace IDs.
* **What a proper fix looks like:** Integrate `tower-http` tracing and metrics middleware to emit standard W3C `traceparent` contexts and route metrics.
* **Priority & Blast Radius:** Priority: P2. Blast Radius: Medium (Degraded operational diagnosability and incident response delays).

#### FINDING-MED-02: Manual Path Parameter Extraction and Routing Logic
* **What is wrong:** `dispatch_request` parses request URLs by splitting on `/` and indexing raw string slices (`segments[3]`, `segments[5]`).
* **Why it is wrong:** Manual path parsing is fragile, prone to index-out-of-bounds panics, and fails to handle URL decoding, trailing slashes, or query parameters cleanly.
* **Where it appears:** `apps/api/src/serve.rs`.
* **How it fails in production:** Requests containing trailing slashes or URL-encoded path parameters trigger unexpected `404 Not Found` or validation errors.
* **What a proper fix looks like:** Use Axum's strongly-typed `Path<T>` and `Query<T>` extractors.
* **Priority & Blast Radius:** Priority: P2. Blast Radius: Medium (API route routing instability and unexpected 404/422 responses).

#### FINDING-MED-03: Hardcoded Timeout Constants in API Server
* **What is wrong:** Probe read timeout (`PROBE_READ_TIMEOUT_SECS = 5`) and drain deadline (`SHUTDOWN_DRAIN_DEADLINE_SECS = 10`) are hardcoded constants in `serve.rs`.
* **Why it is wrong:** Hardcoded values cannot be adjusted for slow network conditions, high-latency satellite connections in remote areas, or specific deployment environments.
* **Where it appears:** `apps/api/src/serve.rs`.
* **How it fails in production:** Clients on slow 2G/3G mobile networks in target SME regions experience premature timeout disconnections during request transmission.
* **What a proper fix looks like:** Expose connection timeouts as configurable parameters in `sitolo-config`.
* **Priority & Blast Radius:** Priority: P2. Blast Radius: Medium (Client connection dropouts on high-latency networks).

#### FINDING-MED-04: Lack of Security Headers on HTTP Responses
* **What is wrong:** Responses returned by `handle_connection` include only `content-type` and `content-length` headers.
* **Why it is wrong:** Standard HTTP security hardening headers (`X-Content-Type-Options`, `X-Frame-Options`, `Content-Security-Policy`, `Strict-Transport-Security`) are omitted.
* **Where it appears:** `apps/api/src/serve.rs`.
* **How it fails in production:** Web clients interacting with API endpoints are vulnerable to MIME-sniffing and clickjacking attacks.
* **What a proper fix looks like:** Attach standard security response headers via Tower middleware on all HTTP responses.
* **Priority & Blast Radius:** Priority: P2. Blast Radius: Medium (Client-side browser security vulnerabilities).

---

### 4. Low Severity Findings

#### FINDING-LOW-01: Dead Code Warnings in `sitolo-persistence` PostgreSQL Module
* **What is wrong:** Compiling `sitolo-persistence` produces unused warnings for `PgAuthorityError`, `PgAuthorityPools`, and `set_transaction_tenant_context`.
* **Why it is wrong:** Unused types and functions clutter compilation output and indicate orphaned or incomplete code paths.
* **Where it appears:** `crates/sitolo-persistence/src/postgres.rs`.
* **How it fails in production:** No direct operational failure, but increases developer cognitive overhead and technical debt.
* **What a proper fix looks like:** Wire unused PostgreSQL helpers into active repository implementations or mark with `#[allow(dead_code)]` if reserved for upcoming phases.
* **Priority & Blast Radius:** Priority: P3. Blast Radius: Low (Code hygiene and developer friction).

#### FINDING-LOW-02: Missing Production Dockerfiles and Deployment Manifests
* **What is wrong:** The repository contains no Dockerfile, Containerfile, or Helm deployment charts for `apps/api` and `apps/worker`.
* **Why it is wrong:** Deployment procedures are undefined, leading to inconsistent runtime environments in production.
* **Where it appears:** Repository root.
* **How it fails in production:** Operators must craft ad-hoc build scripts, increasing deployment risk and environment mismatch errors.
* **What a proper fix looks like:** Add optimized multi-stage Dockerfiles (`distroless` or `alpine` base) and Kubernetes deployment manifests.
* **Priority & Blast Radius:** Priority: P3. Blast Radius: Low (Deployment friction and configuration inconsistency).

---

## Phased Remediation Plan

```text
PHASED REMEDIATION TIMELINE:
[Immediate: 0-14 Days]   → Fix P0 Transport & Auth Vulnerabilities
[Short Term: 15-45 Days] → Build Argon2id Adapter, Axum API, & PG Migrations
[Medium Term: 46-90 Days]→ Build Domain Engines (Sales, Catalogue, Inventory, Outbox)
[Long Term: 91-180 Days] → Integrations, Offline Client Sync, & DR Certification
```

### Phase 1: Immediate Remediation (Days 0 – 14)
* **Goal:** Eliminate critical security defects, enforce HTTP authentication, and restore standard Axum architecture.
1. **Re-architect API Gateway with Axum:** Replace `apps/api/src/serve.rs` custom TCP loop with `axum` router and Tokio `TcpListener`.
2. **Wire Authentication & Authorization Middleware:** Implement Axum extractors to parse Bearer tokens, validate session state against `IdentityStores`, and reject unauthenticated requests to `/v1/organizations/*`.
3. **Implement Argon2id KDF Adapter:** Add OWASP-compliant `Argon2idPasswordHasher` to `sitolo-auth`.
4. **Fix Connection Rejection Behavior:** Implement proper Tower concurrency limiting middleware that returns HTTP 503 Service Unavailable under capacity saturation.

### Phase 2: Short-Term Hardening (Days 15 – 45)
* **Goal:** Operationalize background processing and durable persistence.
1. **Implement Background Worker Loop:** Build outbox polling loop in `apps/worker` using PostgreSQL `FOR UPDATE SKIP LOCKED`.
2. **Automate SQLx Schema Migrations:** Set up automated schema migration runner in `sitolo-persistence` executed on API/worker boot.
3. **Refactor Reference State Locks:** Replace single `Mutex` in `IdentityDatabase` with `DashMap` or granular locks to eliminate thread contention.
4. **Enforce CSPRNG for Tokens:** Ensure all token and identifier creation paths strictly use `sitolo_security::RandomSource`.

### Phase 3: Medium-Term Expansion (Days 46 – 90)
* **Goal:** Implement core business logic and observability.
1. **Build Core Domain Engines:** Implement aggregates and state machines for Product Catalogue, Inventory Ledger, POS Sales, Payments, and Procurement in `crates/sitolo-domain`.
2. **Wire OpenTelemetry & Prometheus Metrics:** Add tracing spans and Prometheus metric exporters across all API routes and background workers.
3. **Implement Payment & Tax Integrations:** Build provider integrations for M-Pesa, Airtel Money, and MRA EIS fiscalization in `crates/sitolo-integrations`.

### Phase 4: Long-Term Enterprise Readiness (Days 91 – 180)
* **Goal:** Offline sync, client SDKs, and enterprise disaster recovery certification.
1. **Implement Offline Synchronization Protocol:** Build offline queueing, conflict resolution, and client sync engine in `crates/sitolo-sync`.
2. **Containerization & Helm Manifests:** Create multi-stage Dockerfiles and Helm charts supporting zero-downtime rolling upgrades.
3. **End-to-End Load & Chaos Testing:** Execute automated end-to-end chaos and load tests verifying system resilience under peak African SME network conditions.

---

## Enterprise Operating Standards & Acceptance Criteria

To ensure clarity for engineering teams and stakeholders, the table below defines the boundaries between acceptable (enterprise-grade) behavior and unacceptable (prototype/demo) behavior across all core system domains:

| System Domain | Enterprise-Grade Standard (ACCEPTABLE) | Prototype / Demo Behavior (UNACCEPTABLE) |
|---|---|---|
| **API Transport & Routing** | Standard Axum/Hyper stack with streaming body bounds, connection draining, and HTTP/2 support. | Custom TCP socket read loops with fixed buffer sizes (`32KB`) and manual whitespace/newline splitting. |
| **Authentication & IAM** | Global Bearer token authentication middleware enforcing server-authoritative tenant/branch scope on every endpoint. | Endpoint handlers accepting raw entity IDs from paths without authentication token validation. |
| **Password Storage** | OWASP-compliant Argon2id password hashing adapter (`m=19456, t=2, p=1`). | Test double SHA-256 hashers deployed in non-test environments. |
| **Background Processing** | Asynchronous worker loop consuming transactional outbox tables via SQL row locking (`FOR UPDATE SKIP LOCKED`). | Empty `main()` binary stubs or synchronous in-process side-effect execution. |
| **Database & Schema** | Version-controlled SQLx migrations executed automatically during deployment; enforced RLS policies on all tenant tables. | Database schemas existing only in test fixtures or manual SQL execution scripts. |
| **Domain Completeness** | Rich domain aggregates enforcing state machine invariants for Sales, Catalogue, Ledger, and Tax. | Stubbed domain crates exporting only tenancy structs without business engines. |
| **Concurrency & Locks** | Lock-free or fine-grained per-entity locks preventing Tokio reactor thread starvation. | Single global `std::sync::Mutex` wrapping the entire database state across all threads. |
| **Randomness & Cryptography** | OS CSPRNG (`OsRng`) utilized exclusively for all security-sensitive tokens and identifiers. | Fixed deterministic byte arithmetic loops producing predictable hex outputs. |
| **Observability** | OpenTelemetry W3C trace propagation, Prometheus metrics middleware, and structured JSON logging. | Unformatted stdout prints without trace context or route latency histograms. |
| **Deployment & Ops** | Automated multi-stage Dockerfiles, Kubernetes Helm charts, and automated CI/CD release verification. | Ad-hoc manual compilation binaries without containerization or deployment manifests. |
