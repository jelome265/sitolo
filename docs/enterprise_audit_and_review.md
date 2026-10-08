# Sitolo Codebase Enterprise Audit & Architecture Review

**Target System:** Sitolo — Business Operating System for African SMEs
**Audit Date:** 2026-10-08
**Scope:** Complete Codebase (Architecture, Security, Reliability, Scalability, Performance, Observability, Testing, Deployment Readiness)
**Status:** Comprehensive Enterprise Audit & Operational Risk Assessment
**Source Baseline:** Repository Root (`apps/`, `crates/`, `docs/`, `scripts/`)

---

## Executive Summary

Sitolo is designed as a modular monolith in Rust intended to serve as a high-reliability business operating system for African Small and Medium Enterprises (SMEs). The system aims to handle point-of-sale (POS) operations, multi-branch inventory management, offline transaction synchronization, payment provider integrations (M-Pesa, Airtel Money), and statutory tax authority compliance (e.g., Mauritius Revenue Authority Electronic Invoicing System - MRA EIS).

### Overall Health Status: NOT PRODUCTION-READY

While the codebase exhibits **exceptionally high-quality foundational engineering** in its security primitives, configuration validation, tenancy typing, and PostgreSQL Row Level Security (RLS) catalog verification, **it is currently an incomplete platform scaffold and cannot support live production workloads**.

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                           CURRENT SYSTEM STATE                          │
├───────────────────────────────────┬─────────────────────────────────────┤
│ Acceptable Enterprise Foundations │ Critical Operational & Domain Gaps  │
├───────────────────────────────────┼─────────────────────────────────────┤
│ ✓ Strict Cargo workspace limits   │ ✗ Unauthenticated HTTP API endpoints│
│ ✓ #![forbid(unsafe_code)] policy  │ ✗ Custom naive TCP HTTP parser      │
│ ✓ Pinned toolchains & dependencies│ ✗ Empty worker binary (fn main)     │
│ ✓ Typed AuthorizedScope tenancy   │ ✗ Unimplemented retail domain models│
│ ✓ Least-privileged DB role rules  │ ✗ Empty integrations & sync crates │
└───────────────────────────────────┴─────────────────────────────────────┘
```

The core blocker is not an architectural defect, but the gap between **implemented security/tenancy scaffolding** and **unimplemented core business processing pipelines**. The server binary currently accepts unauthenticated HTTP requests over raw TCP, the background worker is an empty scaffold, and domain engines for inventory, sales, payment processing, and statutory tax reporting remain completely unwritten.

---

## End-to-End System Evaluation Across Dimensions

### 1. Repository Structure & Dependency Boundaries
- **Evaluation:** Strong.
- **Analysis:** The repository enforces strict Rust workspace boundaries across 15 crates (`crates/`) and 2 applications (`apps/`). Dependency direction flows strictly inward toward `sitolo-domain`. Infrastructure crates do not pollute domain logic. `deny.toml` strictly enforces single-version dependencies, banned licenses, and advisory checks. All crates mandate `#![forbid(unsafe_code)]`.

### 2. Application Architecture & Module Separation
- **Evaluation:** Acceptable Scaffolding / Incomplete Domain.
- **Analysis:** The layered architecture (`domain` -> `application` -> `persistence` / `api`) is cleanly structured. `sitolo-application` orchestrates domain logic through application services. However, `sitolo-domain` currently only contains `tenancy`. Core retail subdomains (catalogue, inventory, POS sales, procurement, returns) are absent from the domain crate.

### 3. API Design, Request Flow & Trust Boundaries
- **Evaluation:** Critical Risk.
- **Analysis:** The documented HTTP contract specifies Rust + Axum + Tokio. However, `apps/api/src/serve.rs` currently implements a custom TCP request loop parsing raw HTTP/1.1 bytes manually. Request bodies are limited to a single 32 KB `read` buffer. Request paths are matched via manual string splitting rather than a standard web framework router.

### 4. Authentication, Authorization & Session Handling
- **Evaluation:** Critical Risk in HTTP Transport / Strong Primitives in Core Crates.
- **Analysis:** `sitolo-auth` provides state-of-the-art Argon2id password hashing, PKCE verification, MFA TOTP flow, and JWT token handling. `sitolo-authz` provides role-based and scope-grant permission models. **However, `apps/api/src/serve.rs` does not wire authentication or authorization checks into its HTTP endpoints.** Any caller can send a raw `POST` request to organization/branch endpoints without presenting a bearer token or session cookie.

### 5. Input Validation, Sanitization & Data Integrity
- **Evaluation:** Partial / Good Baseline.
- **Analysis:** DTOs in `sitolo-api` enforce `#[serde(deny_unknown_fields)]` and field-length bounds. `sitolo-config` rigorously validates environment variable structure, types, and secret formatting on startup using `sitolo-security`. However, domain-level invariants for financial transactions (e.g., non-negative inventory balances, fiscal serial numbers) cannot be validated until domain models exist.

### 6. Injection Risks, XSS, CSRF, SSRF, IDOR & Secret Leakage
- **Evaluation:** Strong Persistence Isolation / Transport Vulnerabilities.
- **Analysis:** PostgreSQL persistence in `sitolo-persistence` uses parameterized SQL queries via SQLx and enforces database-level RLS policies bound to `app.organization_id` and `app.branch_id`. Secret values are wrapped in `ProtectedSecret<T>` to prevent accidental logging. However, the custom HTTP listener lacks TLS enforcement, CORS origin policies, anti-CSRF tokens, or rate-limiting middleware.

### 7. Error Handling, Retry Behavior, Timeout Strategy & Failure Isolation
- **Evaluation:** Partial.
- **Analysis:** API errors map to RFC 7807 `ProblemDetails` structures via `AppError`. Probe reads time out after 5 seconds. However, upstream payment/tax integrations lack circuit breakers, exponential backoff jitter, or bulkheads, creating cascading failure risks under external outage conditions.

### 8. CPU-Bound vs I/O-Bound Bottlenecks
- **Evaluation:** High Risk under Heavy Load.
- **Analysis:** Password hashing in `sitolo-auth/src/password.rs` uses Argon2id with memory-hard parameters. If executed directly within Tokio async task handlers without `tokio::task::spawn_blocking`, cryptographic operations will starve Tokio worker threads, blocking all concurrent I/O on the event loop.

### 9. Async Behavior, Blocking Operations & Concurrency Hazards
- **Evaluation:** Medium Risk.
- **Analysis:** The API server uses a `JoinSet` and `Semaphore` to limit concurrent connections to 1,024. However, in-memory reference repositories in `sitolo-persistence` rely on synchronous `std::sync::Mutex` wrapping in-memory maps, which can introduce lock contention under high concurrent request spikes.

### 10. Database Design, Query Efficiency, Transactions & Migrations
- **Evaluation:** Strong Infrastructure / Unfinished Application Schema.
- **Analysis:** `sitolo-persistence/src/postgres.rs` establishes separate administrative and least-privileged `app_runtime` connection pools. It verifies PostgreSQL catalog metadata, ensuring `app_runtime` lacks superuser, bypassrls, or table ownership privileges. However, business application tables for catalogue, inventory ledger, and fiscal receipts are not yet wired into active API handlers.

### 11. Caching Strategy, Invalidation & Consistency
- **Evaluation:** Unimplemented.
- **Analysis:** No distributed caching layer (e.g., Redis) is integrated. State reads execute directly against database repositories.

### 12. Queueing, Background Jobs, Event Handling & Idempotency
- **Evaluation:** Critical Risk.
- **Analysis:** `apps/worker/src/main.rs` is an empty scaffold (`fn main() {}`). `sitolo-events` contains no outbox polling loop or event dispatcher. Asynchronous workflows—such as sending receipt emails, triggering payment webhooks, or syncing fiscal invoices to tax authorities—do not run.

### 13. Observability: Logs, Metrics, Traces & Incident Diagnosability
- **Evaluation:** Partial Scaffolding.
- **Analysis:** `sitolo-observability` defines structured `RequestId` generation, bounded log metrics, and buffer management. However, HTTP handlers in `apps/api/src/serve.rs` do not propagate tracing spans across async task boundaries.

### 14. Test Coverage, Test Quality & Edge Cases
- **Evaluation:** Partial / High Unit Quality, Missing Integration Coverage.
- **Analysis:** Unit tests in `sitolo-auth`, `sitolo-config`, and `sitolo-security` are thorough. `sitolo-persistence` includes database catalog security tests. However, end-to-end multi-tenant business flows, network failure injection tests, and offline sync edge-case tests are missing.

### 15. Configuration Management, Environment Separation & Secrets Handling
- **Evaluation:** Strong.
- **Analysis:** Configuration uses strict environment parsing (`sitolo-config`) and lowerhex SHA-256 fingerprint verification. Production secrets are never printed in plain text.

### 16. Deployment Safety, Rollback Readiness & Release Discipline
- **Evaluation:** Acceptable Automation Baseline.
- **Analysis:** GitHub Actions CI pipelines enforce format checks, clippy lints, unit/integration test suites, document reference integrity, and ICM workspace cold-walk constraints. Deployment manifests (Helm charts, Dockerfiles) are not present in the primary source tree.

### 17. Code Quality, Naming, Technical Debt & Coupling
- **Evaluation:** Excellent Code Quality / High Functional Debt.
- **Analysis:** Code style is idiomatic Rust with clear naming conventions and zero dead code warnings. Technical debt is concentrated in empty crate scaffolds and intermediate HTTP listener implementation.

### 18. Maintainability Under Team Growth & Code Ownership
- **Evaluation:** Excellent.
- **Analysis:** Workspace structure cleanly isolates domain concerns, making code ownership clear across sub-teams (e.g., Security, Billing, POS, Integrations).

### 19. Compliance & Enterprise Operational Expectations
- **Evaluation:** Unimplemented Compliance Runtime.
- **Analysis:** Mauritius Revenue Authority (MRA EIS) specs are well-documented, but the fiscal signing client and invoice transmission engine remain unwritten.

---

## Severity-Ranked Findings List

### Critical Severity (P0)

#### [C-01] Unauthenticated HTTP API Endpoints and Unenforced Transport Authorization
- **What is wrong:** HTTP mutation routes for provisioning, activating, suspending, and closing organizations/branches do not verify authentication tokens or check caller authorization.
- **Why it is wrong:** Any anonymous network client can execute administrative tenancy operations directly against the API server.
- **Where it appears:** `apps/api/src/serve.rs` (lines 115–220), `crates/sitolo-api/src/tenancy.rs`.
- **How it fails in production:** An attacker sends a crafted HTTP `POST /v1/organizations/{org_id}/suspend` request, shutting down a business tenant's access without valid credentials.
- **What a proper fix looks like:** Replace raw request handling with Axum web framework middleware. Extract session tokens via `sitolo-auth`, validate effective scope via `sitolo-tenancy`, and enforce RBAC policies via `sitolo-authz` before dispatching to handlers.
- **Priority:** P0 | **Blast Radius:** Entire API Surface / All Tenant Data.

#### [C-02] Custom Naive TCP HTTP Parser Missing HTTP/1.1 Framing and Body Streaming
- **What is wrong:** `apps/api/src/serve.rs` parses incoming HTTP requests manually using `TcpStream::read()` into a fixed 32 KB buffer. It does not handle chunked encoding, HTTP keep-alive pipelining, multi-packet TCP body reads, or streaming content-lengths.
- **Why it is wrong:** Custom HTTP parsing violates ADR-001 (Axum + Tokio requirement) and introduces severe transport vulnerabilities.
- **How it fails in production:**
  1. Requests with bodies split across multiple TCP packets are truncated, resulting in JSON parse failures or corrupted payloads.
  2. Susceptible to HTTP Request Smuggling, header injection, and Slowloris Denial of Service (DoS) attacks.
- **What a proper fix looks like:** Delete custom TCP byte parsing in `apps/api/src/serve.rs` and migrate `apps/api` to use `axum::Router` powered by `hyper` and `tokio`.
- **Priority:** P0 | **Blast Radius:** Network Transport Layer / All HTTP Requests.

#### [C-03] Unimplemented Background Worker Binary (`apps/worker`)
- **What is wrong:** `apps/worker/src/main.rs` consists of a single empty function (`fn main() {}`).
- **Why it is wrong:** Background jobs, asynchronous processing, reconciliation, and audit outbox delivery cannot execute.
- **How it fails in production:** Outbox events accumulate indefinitely in the database without being published. Fiscal invoice sign-offs and payment status webhooks fail to process asynchronously.
- **What a proper fix looks like:** Implement a Tokio async worker loop in `apps/worker` that claims outbox records using PostgreSQL `FOR UPDATE SKIP LOCKED`, processes them idempotently, and updates status indicators.
- **Priority:** P0 | **Blast Radius:** Asynchronous Workflows / Outbox / External Integrations.

#### [C-04] Absent Retail Business Subdomains in Core Domain Crate (`sitolo-domain`)
- **What is wrong:** `crates/sitolo-domain/src/lib.rs` exports only `pub mod tenancy;`. Product catalogue, inventory ledger, POS sales, procurement, and returns aggregates do not exist.
- **Why it is wrong:** The system cannot execute retail business logic, track inventory stock levels, or generate sales transactions.
- **How it fails in production:** Point-of-sale terminals cannot create orders, deduct stock, or calculate cart totals because the underlying domain aggregates are missing.
- **What a proper fix looks like:** Implement rich domain entities, value objects, state machines, and business invariant checks in `sitolo-domain` for Catalogue, Inventory, and Sales.
- **Priority:** P0 | **Blast Radius:** Core Retail Operations.

#### [C-05] Missing Asynchronous Event Bus and Transactional Outbox Engine (`sitolo-events`)
- **What is wrong:** `crates/sitolo-events/src/lib.rs` is an empty scaffold with no concrete event models or event bus implementation.
- **Why it is wrong:** Mutations in state cannot broadcast domain events to secondary systems or asynchronous handlers.
- **How it fails in production:** State changes (e.g., stock adjustment) occur in isolation, causing search indexes, reporting stores, and external tax databases to fall out of sync.
- **What a proper fix looks like:** Build standard domain event structures in `sitolo-events` and implement a transactional outbox publisher with guaranteed at-least-once delivery semantics.
- **Priority:** P0 | **Blast Radius:** System-Wide Event Consistency.

---

### High Severity (P1)

#### [H-01] Unimplemented External Integration Adapters (`sitolo-integrations`)
- **What is wrong:** `crates/sitolo-integrations/src/lib.rs` is an empty crate scaffold without concrete client implementations for payment gateways or MRA EIS tax APIs.
- **Why it is wrong:** The system cannot accept mobile money payments or perform mandatory fiscal invoice registration.
- **How it fails in production:** Payment requests stall, and invoices are generated without statutory tax authority QR codes/signatures, leading to regulatory non-compliance.
- **What a proper fix looks like:** Implement resilient HTTP client adapters in `sitolo-integrations` using `reqwest` / `hyper` with mutual TLS, request signing, rate limiting, and circuit breaker protection.
- **Priority:** P1 | **Blast Radius:** Digital Payments & Statutory Tax Compliance.

#### [H-02] Unimplemented Offline POS Synchronization Protocol (`sitolo-sync`)
- **What is wrong:** `crates/sitolo-sync/src/lib.rs` contains no synchronization logic or CRDT state machine code.
- **Why it is wrong:** POS terminals operating in offline SME environments cannot queue sales or synchronize with the central server upon reconnecting.
- **How it fails in production:** POS devices suffer data loss or generate conflicting transaction IDs when re-establishing connectivity after network outages.
- **What a proper fix looks like:** Implement a deterministic vector-clock / state-based synchronization engine with conflict resolution rules for offline sales batches.
- **Priority:** P1 | **Blast Radius:** Offline POS Terminal Operations.

#### [H-03] API Handlers Defaulting to In-Memory Repositories (`sitolo-persistence`)
- **What is wrong:** Application state in `apps/api/src/state.rs` initializes using `MemoryStore` reference repositories rather than real PostgreSQL database pools.
- **Why it is wrong:** All tenant and organization state created via API endpoints is stored in process RAM and lost upon process restart.
- **How it fails in production:** Any server restart or container re-deployment wipes out all registered organizations, branches, and application state.
- **What a proper fix looks like:** Wire `PgPool` persistent repositories as the mandatory default in `AppState`, ensuring all mutations execute inside PostgreSQL transactions with tenant RLS context.
- **Priority:** P1 | **Blast Radius:** Data Durability & Persistence.

#### [H-04] Unencrypted Plaintext HTTP Listener without Security Middleware
- **What is wrong:** `apps/api/src/serve.rs` binds raw TCP sockets without TLS encryption or standard HTTP security headers (HSTS, CSP, X-Frame-Options, CORS).
- **Why it is wrong:** API traffic transmitted over public networks is vulnerable to eavesdropping and active tampering.
- **How it fails in production:** Attackers perform man-in-the-middle (MitM) attacks, stealing session tokens and modifying transaction amounts in transit.
- **What a proper fix looks like:** Enforce reverse-proxy TLS termination (e.g., NGINX / Caddy) or integrate `rustls` directly into the web server, adding security header middleware on all responses.
- **Priority:** P1 | **Blast Radius:** Network Security & Data Privacy.

#### [H-05] Potential Tokio Async Runtime Starvation from CPU-Bound Cryptographic Operations
- **What is wrong:** `sitolo-auth/src/password.rs` executes memory-hard Argon2id password hashing synchronously.
- **Why it is wrong:** Argon2id hashing consumes significant CPU time. Running it on standard Tokio worker threads blocks the event loop thread from handling concurrent network I/O.
- **How it fails in production:** During login spikes, Tokio worker threads become blocked by CPU-bound hashing, causing unrelated API requests to time out across the entire service.
- **What a proper fix looks like:** Wrap all password hash creation and verification calls in `tokio::task::spawn_blocking` to offload work to a dedicated blocking thread pool.
- **Priority:** P1 | **Blast Radius:** Service Availability & Latency.

---

### Medium Severity (P2)

#### [M-01] Disconnected Observability Context and Tracing Span Propagation
- **What is wrong:** HTTP handlers in `apps/api/src/serve.rs` generate local `RequestId` instances but do not create OpenTelemetry tracing spans or attach request context to log events.
- **Why it is wrong:** Log lines cannot be correlated across asynchronous task boundaries or downstream service calls.
- **How it fails in production:** Engineers responding to incidents cannot trace a failed API request back to its underlying database query or background job execution.
- **What a proper fix looks like:** Instrument all API handlers and application services with `tracing::instrument` and inject trace headers into downstream calls.
- **Priority:** P2 | **Blast Radius:** Incident Response & Diagnosability.

#### [M-02] Absence of Upstream Circuit Breakers and Resilience Policies
- **What is wrong:** HTTP client calls lack configurable circuit breakers, bulkheads, or exponential retry backoff with jitter.
- **Why it is wrong:** External provider outages can trigger cascading thread depletion and resource exhaustion inside Sitolo.
- **How it fails in production:** When a mobile money gateway suffers an outage, API request handlers hang waiting for responses until connections exhaust pool limits.
- **What a proper fix looks like:** Integrate `tower::circuit_breaker` or custom failure-counting rate limiters around all external HTTP client adapters.
- **Priority:** P2 | **Blast Radius:** System Reliability under Third-Party Failures.

#### [M-03] Unimplemented Dynamic Secret Rotation Boundaries
- **What is wrong:** Secrets loaded via `sitolo-config` are immutable once parsed during process bootstrap.
- **Why it is wrong:** Rotating database credentials or JWT signing keys requires a full application process restart.
- **How it fails in production:** Secret rotation events cause service disruption or require coordinated zero-downtime rolling re-deployments.
- **What a proper fix looks like:** Wrap sensitive configuration handles in atomic `ArcSwap` containers with signal-driven or interval-driven background key refresh capabilities.
- **Priority:** P2 | **Blast Radius:** Configuration Lifecycle & Key Management.

#### [M-04] Hardcoded Connection Pool Limits without Dynamic Backpressure
- **What is wrong:** `PgAuthorityPools::connect_options` hardcodes `max_connections(20)` for both administrative and runtime pools without dynamic sizing or acquire timeout configurations.
- **Why it is wrong:** Unbounded demand during traffic surges can exhaust database connections or hang workers indefinitely waiting for a pool connection.
- **How it fails in production:** Under high traffic spikes, connection checkouts block indefinitely, causing thread pool starvation and 500 error cascades.
- **What a proper fix looks like:** Expose connection pool sizing and checkout timeouts (`acquire_timeout`) via environment configuration (`sitolo-config`).
- **Priority:** P2 | **Blast Radius:** Database Performance & API Latency.

---

### Low Severity (P3)

#### [L-01] Architectural Terminology Drift Between Documentation and Intermediate Binary Implementation
- **What is wrong:** Phase contracts reference Axum handlers while `apps/api/src/serve.rs` implements raw TCP dispatching.
- **Why it is wrong:** Divergence creates developer onboarding confusion and risks incorrect operational assumptions.
- **How it fails in production:** Developers writing new endpoints follow outdated patterns or write conflicting transport logic.
- **What a proper fix looks like:** Align `apps/api` implementation with Axum architectural contracts.
- **Priority:** P3 | **Blast Radius:** Code Maintainability.

#### [L-02] Placeholder Code Stubs and Unused Scaffold Comments Across Crate Roots
- **What is wrong:** Multiple `lib.rs` files contain placeholder doc comments without exported types or functions.
- **Why it is wrong:** Clutters documentation generation and increases audit noise.
- **How it fails in production:** Does not fail directly in production, but increases cognitive load during code reviews.
- **What a proper fix looks like:** Prune empty scaffolds or feature-gate experimental modules.
- **Priority:** P3 | **Blast Radius:** Code Hygiene.

---

## Top 10 Highest-Risk Issues

```text
┌────┬────────────────────────────────────────────────────────┬──────────┬────────────────────────────┐
│ #  │ Issue Title                                            │ Severity │ Impact Area                │
├────┼────────────────────────────────────────────────────────┼──────────┼────────────────────────────┤
│ 1  │ Unauthenticated Tenancy API Endpoints                  │ Critical │ Authorization & Security   │
│ 2  │ Naive Custom TCP HTTP Parser & Framing Vulnerability    │ Critical │ Network & Transport        │
│ 3  │ Empty Background Worker Binary (`apps/worker`)         │ Critical │ Asynchronous Processing    │
│ 4  │ Absent Retail Business Domain Aggregate Models         │ Critical │ Business Logic             │
│ 5  │ Missing Transactional Outbox & Event Bus               │ Critical │ Event Consistency          │
│ 6  │ Unimplemented Payment & MRA EIS Integration Adapters   │ High     │ Digital Payments & Tax     │
│ 7  │ Unimplemented POS Offline Synchronization Protocol     │ High     │ Offline POS Operations     │
│ 8  │ API Handlers Defaulting to In-Memory Persistence       │ High     │ Data Durability            │
│ 9  │ Tokio Event Loop Starvation from CPU-Bound Argon2 Hashing│ High   │ Latency & Availability     │
│ 10 │ Unencrypted HTTP Listener Without Security Headers     │ High     │ Network Security           │
└────┴────────────────────────────────────────────────────────┴──────────┴────────────────────────────┘
```

---

## Top 10 Highest-Leverage Fixes

1. **Migrate `apps/api` to Axum Web Framework:** Replaces custom TCP parsing with `axum::Router`, restoring standard HTTP/1.1 framing, streaming, path routing, and middleware capabilities.
2. **Implement Axum Authentication & Authorization Middleware:** Wire `sitolo-auth` token verification and `sitolo-authz` policy checks into Axum router layers, securing all endpoints.
3. **Build Async Outbox Worker Loop in `apps/worker`:** Implement outbox table polling using PostgreSQL `FOR UPDATE SKIP LOCKED` for reliable, idempotent background job execution.
4. **Implement Retail Domain Aggregates in `sitolo-domain`:** Build domain aggregates and state machines for Product Catalogue, Inventory Ledger, and POS Sales.
5. **Implement Transactional Outbox Event Publisher in `sitolo-events`:** Guarantee at-least-once domain event publication bound to database transactions.
6. **Build Payment & Tax Client Adapters in `sitolo-integrations`:** Build resilient client adapters with mTLS and circuit breakers for M-Pesa, Airtel Money, and MRA EIS fiscal signing.
7. **Build CRDT-Based Offline POS Sync Protocol in `sitolo-sync`:** Implement vector-clock state synchronization for offline sales replay and stock reconciliation.
8. **Connect Persistent PostgreSQL Repositories in `AppState`:** Ensure API handlers use PostgreSQL persistence with mandatory `AuthorizedScope` RLS session context.
9. **Offload Argon2id Hashing to Blocking Threads:** Wrap all password hashing operations in `tokio::task::spawn_blocking` to protect async runtime availability.
10. **Propagate OpenTelemetry Context Across API Handlers:** Attach tracing spans and request correlation IDs across async task boundaries and database queries.

---

## Phased Remediation Plan

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                          PHASED REMEDIATION TIMELINE                      │
├───────────────────┬───────────────────┬───────────────────┬───────────────┤
│ Immediate (W1-W2) │ Short Term (W3-W6)│ Medium Term (M2-M3│ Long Term (M4+)│
├───────────────────┼───────────────────┼───────────────────┼───────────────┤
│ • Migrate to Axum │ • Worker Loop     │ • Payment Adapters│ • Penetration │
│ • Auth Middleware │ • Retail Domain   │ • MRA EIS Tax API │   Testing     │
│ • Fix Argon2 Task │ • PostgreSQL State│ • Offline POS Sync│ • Load Tests  │
└───────────────────┴───────────────────┴───────────────────┴───────────────┘
```

### Phase 1: Immediate Remediation (Weeks 1–2) — Transport & Auth Security
- [ ] Migrate `apps/api` from custom TCP byte parser to `axum` + `tokio`.
- [ ] Implement Axum middleware layers for token extraction (`sitolo-auth`) and scope authorization (`sitolo-authz`).
- [ ] Offload Argon2id password hashing in `sitolo-auth` to `tokio::task::spawn_blocking`.
- [ ] Enforce security headers (HSTS, CORS, CSP) on all HTTP responses.

### Phase 2: Short-Term Remediation (Weeks 3–6) — Core Domain & Persistence
- [ ] Implement Product Catalogue, Inventory Ledger, and POS Sales domain aggregates in `sitolo-domain`.
- [ ] Wire canonical PostgreSQL persistent repositories into `AppState` with strict RLS tenant context.
- [ ] Build background event worker loop in `apps/worker` with outbox table polling.
- [ ] Add OpenTelemetry span context propagation across all HTTP handlers.

### Phase 3: Medium-Term Remediation (Months 2–3) — Integrations & Offline Sync
- [ ] Implement mobile money payment adapters (M-Pesa, Airtel Money) in `sitolo-integrations`.
- [ ] Build MRA EIS fiscal invoice registration and QR code generation client in `sitolo-integrations`.
- [ ] Build CRDT-based offline POS transaction queue and synchronization protocol in `sitolo-sync`.
- [ ] Implement circuit breakers and retry policies (`tower`) across external client boundaries.

### Phase 4: Long-Term Remediation (Months 4–6) — Hardening & Production Certification
- [ ] Execute comprehensive multi-tenant penetration testing and adversarial security simulation.
- [ ] Conduct end-to-end load testing under 10,000 req/sec simulated SME point-of-sale traffic.
- [ ] Perform MRA EIS statutory tax compliance certification and audit verification.
- [ ] Implement automated multi-region database failover and disaster recovery drills.

---

## Enterprise-Grade Criteria: Acceptable vs. Unacceptable

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                      ENTERPRISE-GRADE ASSESSMENT MATRIX                  │
├─────────────────────────────────────┬─────────────────────────────────────┤
│ Acceptable & Production-Grade       │ NOT Enterprise-Grade / Must Fix     │
├─────────────────────────────────────┼─────────────────────────────────────┤
│ ✓ Cargo Workspace dependency policy │ ✗ Custom naive TCP HTTP parser      │
│ ✓ #![forbid(unsafe_code)] directive │ ✗ Unauthenticated mutation routes   │
│ ✓ Pinned toolchain & lockfiles      │ ✗ Empty worker binary (fn main)     │
│ ✓ Checked PgAuthorityPools RLS rules│ ✗ Missing retail domain aggregates  │
│ ✓ Environment config validation     │ ✗ In-memory repository default state│
│ ✓ Secret masking in logs            │ ✗ CPU-bound crypto on async threads │
└─────────────────────────────────────┴─────────────────────────────────────┘
```

### Acceptable & Enterprise-Grade
- **Dependency Governance:** Pinned toolchains, Cargo lockfile governance (`deny.toml`), single-version crate dependencies, and mandatory `#![forbid(unsafe_code)]`.
- **Database Authority Isolation:** Separate administrative and runtime PostgreSQL connection pools with catalog-verified least-privileged role enforcement (`app_runtime`).
- **Configuration Security:** Environment variable parsing with strict type checks and secret redacting handles (`sitolo-config`, `sitolo-security`).
- **Tenancy Boundary Rules:** Server-authoritative `AuthorizedScope` type resolution preventing free-form scope widening.

### Unacceptable & NOT Enterprise-Grade
- **Custom Raw TCP HTTP Parsing:** Parsing raw bytes on a socket without standard HTTP framing, streaming, or security middleware.
- **Unauthenticated Mutation Endpoints:** Exposing tenant management routes without mandatory token extraction and RBAC authorization.
- **Empty Worker Binary:** Running an empty `fn main() {}` in background worker services.
- **Absence of Core Business Aggregates:** Relying on scaffold crates without concrete domain logic for inventory, sales, and fiscal compliance.
- **RAM-Only Default State:** Defaulting API application state to in-memory maps instead of persistent PostgreSQL databases.
