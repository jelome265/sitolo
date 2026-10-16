# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main`
**Status:** Comprehensive Enterprise Audit & Review
**Authority:** Governing documentation hierarchy in `agent.md` and live repository source code

---

## 1. Executive Summary

Sitolo is designed as a multi-tenant, modular-monolith business operating system for African SMEs, providing point-of-sale (POS), inventory control, payment processing, regulatory fiscalization (MRA EIS tax compliance), and offline synchronization.

The current codebase contains high-quality security and domain foundational primitives:
- Rigid workspace dependency boundaries enforced via `#![deny(clippy::unwrap_used)]` in production crates (where applicable) and Cargo policy checks (`check-architecture`, `check-phase2-policy`).
- Domain-driven tenant isolation models (`sitolo-tenancy`, `sitolo-authz`, `sitolo-domain`) with trusted-vs-requested effective scope resolution (`AuthorizedScope`).
- Strong cryptographic token and session primitives (`sitolo-auth`) using Argon2id, HMAC, and strict TTL mechanics.
- PostgreSQL Row-Level Security (RLS) enforcement infrastructure and runtime least-privilege connection separation (`app_runtime` vs `app_admin`) in `sitolo-persistence`.

However, **Sitolo is NOT production-ready**.

The core operational and business capabilities required for a live enterprise deployment remain incomplete or unintegrated:
1. **HTTP Transport Divergence & Vulnerability:** The binary transport in `apps/api/src/serve.rs` bypasses the documented web framework (`Axum`) and implements a hand-rolled TCP socket reader. It reads incoming streams into a fixed 32KB buffer, parses HTTP headers with string splitting, and lacks streaming request body handling, TLS termination, header sanitization, or request pipelining/keep-alive handling.
2. **Unauthenticated HTTP Pipeline:** The current HTTP request dispatcher in `apps/api/src/serve.rs` invokes tenancy handlers (`/v1/organizations/*`) without extracting or verifying Bearer session tokens or enforcing RBAC/ABAC authorization.
3. **Unimplemented Background Worker & Asynchronous Subsystems:** `apps/worker` is an empty 10-line main function scaffold. The background message queue, outbox worker loop, event processing (`sitolo-events`), external provider integrations (`sitolo-integrations`), and offline client sync engine (`sitolo-sync`) are stubbed 5-line crates.
4. **Missing Business Domain Engines:** `sitolo-domain` contains tenancy and membership domain models, but point-of-sale, sales invoicing, product catalog, inventory ledger, supplier procurement, payments, and fiscal tax compliance (MRA EIS) engines are not implemented.
5. **In-Memory Volatile Reference Services in API Binary:** The API binary wires HTTP endpoints directly to `InMemTenancyService` using in-process `RwLock<HashMap>` structures. Transactions do not execute through durable PostgreSQL connections with transaction-local context (`app.organization_id`, `app.branch_id`).

### Health Assessment Matrix
```text
[ARCHITECTURAL SPECIFICATIONS & PRIMITIVES]  ---> EXCELLENT / PRODUCTION-GRADE
[TENANCY, IAM & SECURITY CRYPTO BOUNDARIES] ---> HIGH / ROBUST FOUNDATION
[DATABASE PERSISTENCE & RLS FRAMEWORK]     ---> GOOD / PARTIALLY INTEGRATED
[HTTP TRANSPORT & ROUTING PIPELINE]         ---> POOR / UNACCEPTABLE FOR PRODUCTION
[BACKGROUND WORKERS & EVENT OUTBOX]        ---> CRITICAL / STUBBED SCAFFOLD
[BUSINESS ENGINES & INTEGRATIONS]           ---> CRITICAL / UNIMPLEMENTED
```

---

## 2. System-Wide End-to-End Review

### 2.1 Repository Structure & Dependency Boundaries
- **Status:** Acceptable / Production-Grade Architecture.
- **Findings:**
  - Workspace structure consists of 15 modular crates in `crates/` and 2 binaries in `apps/`.
  - Dependency hierarchy flows inward towards domain core (`sitolo-domain` -> `sitolo-tenancy` / `sitolo-authz` -> `sitolo-application` -> `sitolo-api` / `sitolo-persistence`).
  - Strict workspace checks (`./scripts/ci/check-architecture`) enforce crate dependency rules to prevent circular references.

### 2.2 Application Architecture & Module Separation
- **Status:** Partial / Good Foundation.
- **Findings:**
  - `sitolo-application` cleanly separates orchestration services (`OrganizationService`, `BranchService`, `IAMService`) from transport and persistence layers.
  - Domain commands return explicit result types (`Result<T, AppError>`).
  - Lack of complete command/query segregation (CQRS) or event sourcing for complex inventory and fiscal ledgers.

### 2.3 API Design, Request Flow & Trust Boundaries
- **Status:** Not Enterprise-Grade.
- **Findings:**
  - API DTOs in `sitolo-api` use `#[serde(deny_unknown_fields)]` and enforce input size bounds (`MAX_TENANCY_BODY_BYTES = 32,768`).
  - However, `apps/api/src/serve.rs` handles HTTP requests over raw `TcpStream` using manually parsed request strings rather than standard Axum HTTP extractors.
  - Trust boundaries are breached because HTTP routing in `dispatch_request` does not execute session token verification or scope authorization before calling application services.

### 2.4 Authentication, Authorization & Session Handling
- **Status:** Partial (Primitives Strong, API Integration Missing).
- **Findings:**
  - `sitolo-auth` provides enterprise-grade session lifecycle management, Argon2id password hashing with custom salt generation, MFA device binding, and token revocation.
  - `sitolo-tenancy` and `sitolo-authz` implement effective scope resolution (`AuthorizedScope`), permission checks, and RBAC models.
  - **Critical Gap:** The HTTP transport in `apps/api` fails to wire `sitolo-auth` session extraction middleware into the request flow.

### 2.5 Input Validation, Sanitization & Data Integrity
- **Status:** Acceptable Foundation.
- **Findings:**
  - Strongly typed domain wrappers (e.g., `OrganizationId`, `UserId`, `BranchId`) validate format and prevent primitive obsession.
  - JSON payloads strictly deny unexpected fields.
  - Absence of html/rich-text sanitization (not currently needed as APIs consume strict JSON).

### 2.6 Injection Risks, Security & Insecure Defaults
- **Status:** High Foundation / Transport Weakness.
- **Findings:**
  - **SQL Injection:** Zero risk of SQL injection; queries use SQLx parameterized queries with compile-time verification or static prepared statements.
  - **XSS & CSRF:** API is JSON-only; cross-site request forgery is mitigated if Bearer tokens are used in headers rather than ambient cookies.
  - **SSRF:** Provider integrations crate (`sitolo-integrations`) is unbuilt; HTTP egress control and URL domain whitelisting are missing.
  - **Path Traversal:** No dynamic file path resolution exists in API endpoints.
  - **Insecure Defaults:** Development fallback secret keys exist in `sitolo-config` when environment variables are omitted; production config validation prevents boot if secrets are missing in production mode.

### 2.7 Error Handling, Retry Behavior & Failure Isolation
- **Status:** Good Domain Design / Missing Egress Retries.
- **Findings:**
  - `AppError` enum cleanly classifies error domains (`Validation`, `Authentication`, `Authorization`, `NotFound`, `Conflict`, `RateLimited`, `UpstreamTimeout`, etc.).
  - Handlers format RFC 7807 `ProblemDetails` responses with server request IDs.
  - Circuit breakers and exponential backoff policies are not implemented for external regulatory (MRA EIS) or payment provider calls.

### 2.8 CPU-bound vs I/O-bound Bottlenecks
- **Status:** Needs Refactoring.
- **Findings:**
  - Heavy CPU operations (Argon2id password hashing) in `sitolo-auth` run within async methods without explicit `tokio::task::spawn_blocking`.
  - Under concurrent login bursts, password hashing will block Tokio worker threads, starving I/O loop performance.

### 2.9 Async Behavior, Blocking Operations & Concurrency Safety
- **Status:** Moderate Risk.
- **Findings:**
  - Connection draining on shutdown is limited by `MAX_IN_FLIGHT_CONNECTIONS` (1024) and `SHUTDOWN_DRAIN_DEADLINE_SECS` (10).
  - Memory-backed reference repositories in `sitolo-application` rely on `tokio::sync::RwLock<HashMap>`. High concurrent write lock contention will degrade throughput under load.

### 2.10 Database Design, Query Efficiency, Transactions & RLS
- **Status:** Strong Infrastructure / Integration Missing in API.
- **Findings:**
  - `sitolo-persistence` implements PostgreSQL pooled connections with dual roles (`app_admin` for schema management, `app_runtime` for application queries).
  - RLS policies enforce tenant isolation via session variables (`app.organization_id`, `app.branch_id`).
  - **Gap:** Schema migration scripts for business tables (products, inventory, sales, fiscal invoices) are not fully realized or executed at startup.

### 2.11 Caching Strategy & Consistency
- **Status:** Unimplemented.
- **Findings:**
  - No caching layer (e.g., Redis or in-memory LRU) exists for session lookups or tenant authorization scope validation.
  - Every authorization check must hit database storage.

### 2.12 Queueing, Background Jobs, Events & Idempotency
- **Status:** Critical Gap.
- **Findings:**
  - `apps/worker` is non-functional.
  - Transactional outbox pattern is designed in docs but lacks an active polling worker or LISTEN/NOTIFY daemon.
  - No persistent idempotency storage key-value table exists for network-retried POS transactions.

### 2.13 Observability: Logs, Metrics, Traces & Diagnostics
- **Status:** Good Telemetry Substrate / Missing Distributed Tracing.
- **Findings:**
  - `sitolo-observability` sets up structured JSON tracing with custom fields (`request_id`, `tenant_id`, `trace_id`).
  - OpenTelemetry exporter and Prometheus metric scrape endpoints (`/metrics`) are not mounted on the HTTP server.

### 2.14 Test Coverage, Quality & Regression Safety
- **Status:** Good Unit/RLS Coverage / Missing End-to-End & Load Tests.
- **Findings:**
  - Domain, auth, and persistence crates contain comprehensive unit and RLS security tests (`rls_security_tests`).
  - Integration tests in `apps/api/tests/tenancy_api.rs` test tenancy endpoints against `InMemTenancyService`, but zero integration tests run against real HTTP sockets or real PostgreSQL database instances.

### 2.15 Configuration Management & Secrets Handling
- **Status:** Acceptable.
- **Findings:**
  - Pinned toolchain (Rust 1.98.1) and strict configuration validation in `sitolo-config`.
  - Secrets are typed (`SecretString`) to prevent accidental logging via `Debug` traits.

### 2.16 Deployment Safety & Release Discipline
- **Status:** Partial.
- **Findings:**
  - CI verification scripts (`scripts/ci/verify`) run format, clippy, unit tests, RLS tests, architecture rules, and dependency audits (`cargo deny`, `cargo audit`).
  - Docker containerization multi-stage Dockerfiles and Helm charts/Terraform manifests are absent.

### 2.17 Code Quality, Naming & Technical Debt
- **Status:** High Quality.
- **Findings:**
  - Clean idiom, strict Rust conventions, zero dead code warnings under clippy `-D warnings`.
  - Overuse of `.unwrap()` in test code (443 instances in workspace), though production crates restrict unwrap.

### 2.18 Maintainability & Code Ownership
- **Status:** Excellent.
- **Findings:**
  - Clear workspace layout and modular separation make the codebase easy to navigate and scale across engineering teams.

### 2.19 Compliance & Enterprise Expectations (MRA EIS Tax Compliance)
- **Status:** Non-Compliant / Unimplemented Engine.
- **Findings:**
  - Mauritius Revenue Authority (MRA) Electronic Invoice System (EIS) fiscalization requires real-time receipt Signing, QR code generation, and offline fiscal counters.
  - `sitolo-integrations` and MRA domain modules are stubs; compliance cannot be met in current state.

---

## 3. Severity-Ranked Findings List

### CRITICAL SEVERITY FINDINGS

#### Finding CRIT-01: Hand-Rolled TCP HTTP Server Lacks Production Security, TLS & Standard Parsing
- **What is wrong:** `apps/api/src/serve.rs` implements HTTP server transport directly on `TcpListener` and `TcpStream` using byte slice matching and `String::from_utf8_lossy`. It does not use the documented `Axum` framework or `hyper`.
- **Why it is wrong:** Manual HTTP parsing is extremely fragile and vulnerable to HTTP request smuggling, line-folding attacks, header splitting, slowloris DoS, malformed UTF-8 stream injection, and buffer truncation.
- **Where it appears:** `apps/api/src/serve.rs`, lines 45–115 (`handle_connection` and `dispatch_request`).
- **How it fails in production:**
  1. Requests exceeding `PROBE_MAX_BYTES` (32KB) are truncated, dropping HTTP headers or JSON body content silently.
  2. Malformed headers missing standard CRLF delimiters hang or cause unhandled line splitting logic.
  3. Lacks TLS termination; raw HTTP traffic is exposed over public networks unless an external reverse proxy is correctly configured.
- **Proper fix:** Refactor `apps/api` to use `Axum` routes with standard `tokio` listeners, tower middleware, `axum::extract::Json`, and streaming request limiters.
- **Priority:** P0
- **Blast Radius:** Entire API server transport.

#### Finding CRIT-02: Unauthenticated HTTP Request Dispatcher
- **What is wrong:** `dispatch_request` in `apps/api/src/serve.rs` routes incoming `POST /v1/organizations/*` requests directly to application service methods without extracting or validating Bearer tokens, API keys, or session identifiers.
- **Why it is wrong:** Any unauthenticated remote client on the network can create organizations, create branches, activate/suspend organizations, and perform admin actions.
- **Where it appears:** `apps/api/src/serve.rs`, lines 135–225.
- **How it fails in production:** An attacker can issue HTTP `POST` requests to `/v1/organizations` and provision or alter organizational entities without credentials or session cookies.
- **Proper fix:** Add Axum authentication extractor middleware (`AuthenticationMiddleware`) that validates Bearer JWT/Session tokens via `sitolo-auth`, constructs `SecurityContext`, and enforces tenant authorization before routing to application services.
- **Priority:** P0
- **Blast Radius:** Total security compromise of API endpoints.

#### Finding CRIT-03: Background Worker Binary & Event/Outbox Processing Are Empty Stubs
- **What is wrong:** `apps/worker/src/main.rs` is a 10-line stub that logs a message and exits. Crates `sitolo-events`, `sitolo-integrations`, and `sitolo-sync` contain single dummy files.
- **Why it is wrong:** Asynchronous processing, transactional outbox event delivery, payment callbacks, MRA EIS fiscal invoice submission, and offline device sync cannot run.
- **Where it appears:**
  - `apps/worker/src/main.rs`
  - `crates/sitolo-events/src/lib.rs`
  - `crates/sitolo-integrations/src/lib.rs`
  - `crates/sitolo-sync/src/lib.rs`
- **How it fails in production:** Operations that depend on asynchronous processing (e.g., sending email invitations, syncing offline sales transactions, posting tax receipts to government endpoints) fail silently or are never executed.
- **Proper fix:** Build a dedicated polling/notification worker in `apps/worker` that consumes from the PostgreSQL `outbox` table, uses `sitolo-events` for event routing, and handles background job retries.
- **Priority:** P0
- **Blast Radius:** Entire asynchronous execution engine and external integrations.

#### Finding CRIT-04: Core Business Domains (POS, Catalog, Inventory, Payments, MRA EIS) Unimplemented
- **What is wrong:** Core SME business operations—Product Catalog, Inventory Ledger, Point-of-Sale Checkout, Payment Reconciliation, Supplier Procurement, and MRA EIS Tax Fiscalization—do not exist in domain code or database schemas.
- **Why it is wrong:** Sitolo cannot function as a business operating system without its core transactional engines.
- **Where it appears:** `crates/sitolo-domain/src/lib.rs` (only contains tenancy/organization models).
- **How it fails in production:** Merchants cannot register products, record sales, calculate taxes, manage stock inventory, or process customer payments.
- **Proper fix:** Implement domain aggregates, state machines, and repository traits for catalog, inventory, sales, and payments in `sitolo-domain` and `sitolo-application`.
- **Priority:** P0
- **Blast Radius:** Core business functionality.

---

### HIGH SEVERITY FINDINGS

#### Finding HIGH-01: API Binary Uses In-Memory Mock Service Instead of PostgreSQL RLS Persistence
- **What is wrong:** `apps/api/src/bootstrap.rs` constructs `AppState` using `InMemTenancyService::new()` with in-memory `RwLock<HashMap>` stores rather than a PostgreSQL persistence repository connected to `sitolo-persistence`.
- **Why it is wrong:** All API data is lost when the API process restarts. Data isolation relying on PostgreSQL Row-Level Security (RLS) is not utilized by the live HTTP server.
- **Where it appears:** `apps/api/src/bootstrap.rs`, lines 25–45.
- **How it fails in production:** Server restarts wipe all created tenant and branch data. Multi-node deployments cannot share state.
- **Proper fix:** Replace `InMemTenancyService` in `AppState` with a PostgreSQL-backed tenancy service initialized with `PgPool` from `sitolo-persistence`.
- **Priority:** P1
- **Blast Radius:** State durability and multi-instance scalability.

#### Finding HIGH-02: CPU-Heavy Argon2id Hashing Blocks Tokio Async Worker Threads
- **What is wrong:** Password hashing in `sitolo-auth` executes `Argon2::hash_password` directly within an `async fn` without wrapping the computation in `tokio::task::spawn_blocking`.
- **Why it is wrong:** Argon2id is intentionally CPU and memory intensive. Running it on a Tokio worker thread blocks the async executor loop, preventing other concurrent HTTP requests on that thread from being polled.
- **Where it appears:** `crates/sitolo-auth/src/password.rs`, lines 40–65.
- **How it fails in production:** Under concurrent login attempts, Tokio worker threads become exhausted, leading to severe latency spikes, request timeouts, and health check failure on `/process/live`.
- **Proper fix:** Wrap `argon2` hashing and verification calls in `tokio::task::spawn_blocking(move || { ... }).await`.
- **Priority:** P1
- **Blast Radius:** API throughput and server responsiveness under load.

#### Finding HIGH-03: Lack of Application-Level & IP-Level Rate Limiting on HTTP Transport
- **What is wrong:** While `apps/api/src/serve.rs` bounds total concurrent connections via a semaphore (`MAX_IN_FLIGHT_CONNECTIONS = 1024`), it does not enforce per-IP, per-tenant, or per-endpoint rate limits.
- **Why it is wrong:** Malicious or misconfigured clients can saturate API endpoints, perform credential brute-forcing, or exhaust database connections.
- **Where it appears:** `apps/api/src/serve.rs`.
- **How it fails in production:** Credential stuffing attacks against authentication endpoints or rapid requests against mutation endpoints bypass concurrency limits and cause resource starvation.
- **Proper fix:** Implement rate-limiting middleware in Axum using `governor` or a Redis-backed sliding window counter.
- **Priority:** P1
- **Blast Radius:** Availability and security against brute-force attacks.

#### Finding HIGH-04: Missing Idempotency Key Storage Engine for Transactional Mutations
- **What is wrong:** `AppError::IdempotencyConflict` exists in error definitions, but there is no persistent storage or middleware to record `Idempotency-Key` headers and cache HTTP responses for mutation operations.
- **Why it is wrong:** SME point-of-sale environments in Africa frequently experience unstable mobile internet connectivity. POS clients retrying offline payment or sales requests will submit duplicate transactions.
- **Where it appears:** `crates/sitolo-api/src/error.rs` (error type exists, execution engine missing).
- **How it fails in production:** Retried POS payment or checkout requests result in double-charging customers or duplicating inventory deductions.
- **Proper fix:** Implement an `IdempotencyMiddleware` using PostgreSQL or Redis that checks `(tenant_id, idempotency_key)`, locks in-flight requests, and returns cached responses for identical retries.
- **Priority:** P1
- **Blast Radius:** Financial accuracy and inventory data integrity.

---

### MEDIUM SEVERITY FINDINGS

#### Finding MED-01: W3C Trace Context Propagation Missing in Request Headers
- **What is wrong:** HTTP request dispatching does not extract or inject W3C `traceparent` or `tracestate` headers into `tracing` spans.
- **Why it is wrong:** Distributed tracing across clients, API gateways, workers, and external service calls is broken.
- **Where it appears:** `apps/api/src/serve.rs` and `crates/sitolo-observability/src/lib.rs`.
- **How it fails in production:** Operational teams cannot trace end-to-end transaction latency or diagnose failing asynchronous worker requests across service boundaries.
- **Proper fix:** Integrate `tracing-opentelemetry` with Axum header extraction middleware to propagate `traceparent` context.
- **Priority:** P2
- **Blast Radius:** Observability and incident diagnosability.

#### Finding MED-02: Absence of Automated Database Migration Execution on Service Boot
- **What is wrong:** `apps/api/src/bootstrap.rs` does not verify database schema version or execute pending SQL migrations on startup.
- **Why it is wrong:** Deploying new API binaries without manually executing database migration tools will cause runtime schema mismatch errors.
- **Where it appears:** `apps/api/src/bootstrap.rs`.
- **How it fails in production:** New code expecting newly added columns or tables crashes with SQL errors in production.
- **Proper fix:** Call `sqlx::migrate!().run(&pool).await` or schema validation checks during API bootstrap before binding the TCP listener.
- **Priority:** P2
- **Blast Radius:** Deployment safety and release stability.

#### Finding MED-03: Widespread Overuse of `.unwrap()` in Test Utilities
- **What is wrong:** Over 440 occurrences of `.unwrap()` exist in test files across the workspace.
- **Why it is wrong:** While prohibited in production code by `#![deny(clippy::unwrap_used)]`, test failures produce opaque panics without contextual diagnostic information.
- **Where it appears:** `crates/*/tests/*.rs` and `apps/api/tests/tenancy_api.rs`.
- **How it fails in production:** Decreases test maintainability and masks underlying setup errors during automated CI/CD pipeline execution.
- **Proper fix:** Replace `.unwrap()` in tests with `expect("descriptive failure context")` or return `Result<(), Box<dyn Error>>` from test functions.
- **Priority:** P2
- **Blast Radius:** CI test diagnostics and developer experience.

---

### LOW SEVERITY FINDINGS

#### Finding LOW-01: Historical Documentation Blur Across Phase Specifications
- **What is wrong:** Specifications in `docs/phase8_product_catalogue_implementation.md` and subsequent phase files use imperative present-tense verbs ("API exposes endpoint...") for features that are not yet implemented.
- **Why it is wrong:** Causes confusion for engineers and automated tools regarding current implementation state versus future requirements.
- **Where it appears:** `docs/phase8_*`, `docs/phase9_*`, `docs/phase10_*`.
- **How it fails in production:** Non-code impact; leads to incorrect assumptions during system integration planning.
- **Proper fix:** Standardize header metadata in future phase specs with explicit status badges: `[STATUS: NORMATIVE SPECIFICATION - UNIMPLEMENTED]`.
- **Priority:** P3
- **Blast Radius:** Documentation clarity and developer onboarding.

#### Finding LOW-02: Stubbed Workspace Crates Emit Compilation Warnings if Unused
- **What is wrong:** Crates like `sitolo-events`, `sitolo-integrations`, `sitolo-sync`, and `sitolo-testkit` contain empty stubs without feature gating or explicit exports.
- **Why it is wrong:** Clutters workspace dependency graph and builds empty artifacts.
- **Where it appears:** `crates/sitolo-events`, `crates/sitolo-integrations`, `crates/sitolo-sync`.
- **How it fails in production:** No runtime impact.
- **Proper fix:** Add explicit module structures or mark placeholder crates clearly in Cargo.toml manifests.
- **Priority:** P3
- **Blast Radius:** Build hygiene.

---

## 4. Top 10 Highest-Risk Issues

| Rank | Risk ID | Description | Primary Vulnerability / Impact |
|---|---|---|---|
| 1 | **CRIT-01** | Hand-Rolled TCP HTTP Server | Transport DoS, HTTP request smuggling, missing TLS |
| 2 | **CRIT-02** | Unauthenticated HTTP Endpoint Dispatcher | Unauthorized tenant creation, modification & admin takeover |
| 3 | **CRIT-03** | Unimplemented Background Worker & Events | Outbox failure, broken async operations & fiscal reporting |
| 4 | **CRIT-04** | Missing Business Domain Engines (POS, Inventory, Fiscal) | System cannot perform core SME business operations |
| 5 | **HIGH-01** | API Binary Uses In-Memory Mock Service | Complete data loss on restart; RLS isolation unverified in live API |
| 6 | **HIGH-02** | Argon2 Hashing Blocks Async Worker Threads | Thread starvation, latency spikes & API denial of service under load |
| 7 | **HIGH-03** | Missing HTTP Rate Limiting Middleware | Vulnerable to brute-force credential stuffing & resource exhaustion |
| 8 | **HIGH-04** | Missing Persistent Idempotency Key Engine | Duplicate sales charges and stock deductions during POS retries |
| 9 | **MED-01** | Missing W3C Trace Context Propagation | Broken distributed tracing and incident diagnostics |
| 10 | **MED-02** | Unautomated Database Schema Migration on Boot | Runtime SQL crashes following deployment of schema changes |

---

## 5. Top 10 Highest-Leverage Fixes

| Rank | Fix Target | Implementation Action | Impact |
|---|---|---|---|
| 1 | **Axum Migration** | Replace `apps/api/src/serve.rs` with standard `Axum` router and extractors | Eliminates transport DoS, enables standard middleware, provides clean HTTP parsing |
| 2 | **Auth Middleware** | Wire `sitolo-auth` session token extractor into Axum middleware | Secures all API endpoints with mandatory authentication & scope validation |
| 3 | **PostgreSQL Binding** | Wire `sitolo-persistence` PgPool into `AppState` and application services | Establishes durable transactional persistence and real PostgreSQL RLS isolation |
| 4 | **Blocking Argon2** | Wrap password hashing in `tokio::task::spawn_blocking` | Prevents thread starvation and maintains API responsiveness under login load |
| 5 | **Outbox Worker** | Implement `apps/worker` outbox polling loop against PostgreSQL outbox table | Unblocks asynchronous event processing, email delivery, and provider sync |
| 6 | **Idempotency Engine** | Add PostgreSQL/Redis `IdempotencyMiddleware` for mutation endpoints | Guarantees exact-once execution for retried POS checkout operations |
| 7 | **Rate Limiting** | Add `governor` rate-limiting middleware to Axum endpoints | Protects against brute-force attacks and server resource exhaustion |
| 8 | **Catalog & POS Domain** | Implement product catalog, inventory ledger, and checkout aggregates | Delivers primary SME business value (sales, stock, checkout) |
| 9 | **MRA EIS Fiscalization** | Implement MRA EIS fiscal invoice signing and receipt counter engine | Achieves regulatory compliance for Mauritius tax authority mandates |
| 10 | **E2E Integration Suite** | Build real HTTP socket + PostgreSQL integration test suite | Ensures regression safety and verifies end-to-end transaction flows |

---

## 6. Phased Remediation Plan

```text
[PHASE 1: IMMEDIATE]  ---> Fix HTTP Transport, Authentication & Durability (Weeks 1–2)
[PHASE 2: SHORT TERM] ---> Build Background Worker, Idempotency & POS Domain (Weeks 3–6)
[PHASE 3: MEDIUM TERM]---> Implement Inventory, Payments & MRA EIS Tax Engine (Weeks 7–10)
[PHASE 4: LONG TERM]  ---> Offline Sync Engine, Advanced Observability & Hardening (Weeks 11–14)
```

### Phase 1: Immediate Remediation (Core Transport, Security & Durability)
1. **Refactor `apps/api` to Axum:**
   - Remove raw TCP parsing from `apps/api/src/serve.rs`.
   - Install `axum` dependency and construct standard routes for `/process/live`, `/process/ready`, and `/v1/organizations/*`.
2. **Implement Authentication & Authorization Middleware:**
   - Create `Axum` session extraction middleware using `sitolo-auth`.
   - Require valid Bearer tokens and enforce scope validation (`AuthorizedScope`) on all non-probe routes.
3. **Connect PostgreSQL Persistence to API:**
   - Update `apps/api/src/bootstrap.rs` to initialize `PgPool` using `sitolo-persistence`.
   - Replace `InMemTenancyService` with PostgreSQL-backed tenancy repositories.
4. **Offload Argon2 Hashing:**
   - Wrap `Argon2` hashing calls in `sitolo-auth` with `tokio::task::spawn_blocking`.

### Phase 2: Short-Term Remediation (Worker, Idempotency & Core POS)
1. **Build Outbox Worker Daemon (`apps/worker`):**
   - Implement event consumer loop polling `outbox` table in PostgreSQL.
   - Implement handler dispatcher in `sitolo-events`.
2. **Implement Idempotency Middleware:**
   - Add `Idempotency-Key` storage table in PostgreSQL.
   - Attach middleware to mutation routes to return cached responses on duplicate requests.
3. **Implement Product Catalog & POS Checkout Aggregate:**
   - Build `Product`, `Category`, `Price`, and `SaleTransaction` domain aggregates in `sitolo-domain`.
   - Expose POS sales endpoints in `apps/api`.

### Phase 3: Medium-Term Remediation (Inventory, Payments & Tax Compliance)
1. **Double-Entry Inventory Ledger:**
   - Implement append-only stock movement ledger in `sitolo-domain` and `sitolo-persistence`.
2. **MRA EIS Fiscalization Engine:**
   - Build MRA EIS integration client in `sitolo-integrations` with exponential backoff retries.
   - Implement electronic invoice signing, QR code generation, and fiscal counter sequencing.
3. **Payment Provider Integration:**
   - Implement payment gateway abstractions (mobile money, card terminals, cash reconciliation).

### Phase 4: Long-Term Remediation (Offline Sync & Production Certification)
1. **Offline Client Synchronization (`sitolo-sync`):**
   - Implement delta-based sync protocol for offline desktop/mobile POS clients.
   - Conflict resolution rules for offline stock deduction and sales numbering.
2. **Distributed Observability & Rate Limiting:**
   - Add OpenTelemetry trace context propagation.
   - Mount Prometheus `/metrics` scraping endpoint.
   - Configure per-tenant rate limits.
3. **End-to-End Production Certification:**
   - Run stress, chaos, and load testing against multi-node API and PostgreSQL setups.

---

## 7. Acceptable vs. Not Enterprise-Grade Matrix

| Architecture Dimension | Current State | Acceptable Standard | Not Enterprise-Grade (Current Deficit) |
|---|---|---|---|
| **HTTP Transport** | Hand-rolled TCP loop in `serve.rs` | Standard framework (`Axum`), HTTP/1.1 & HTTP/2 support, TLS termination, streaming limits | Hand-rolled TCP buffer reader parsing strings with `String::from_utf8_lossy` |
| **API Authentication** | Unauthenticated dispatcher | Mandatory Bearer token validation, session revocation check, RBAC context injection | Routing requests to application services without credential verification |
| **Data Persistence** | In-memory `RwLock<HashMap>` in API | PostgreSQL pooled connection, durable transactions, Row-Level Security (RLS) enforcement | Volatile in-memory mock service in API binary |
| **Async Tasks & Outbox** | Empty worker scaffold | Dedicated outbox polling daemon, at-least-once delivery, exponential backoff retries | 10-line dummy worker binary that immediately exits |
| **CPU Work Execution** | Inline async Argon2 | Offloaded to thread pool (`spawn_blocking`) to prevent worker starvation | CPU-heavy password hashing executing directly on Tokio worker threads |
| **Mutation Idempotency** | Error type exists; storage missing | Idempotency key storage table returning cached HTTP responses for retries | Retried POS mutations creating duplicate charges or stock deductions |
| **Domain Scope** | Tenancy and IAM models only | Complete business engine (Catalog, Inventory, POS, Payments, MRA Tax Fiscalization) | Stubs for core revenue-generating business engines |
| **Compliance (MRA EIS)** | Unimplemented stub | Automated invoice signing, QR generation, fiscal counter tracking, offline buffering | Absence of regulatory tax compliance integration |

---

## 8. Conclusion

Sitolo possesses a **well-crafted architectural blueprint and robust security/tenancy primitives**. Its domain boundaries, scope resolution models, and database Row-Level Security infrastructure reflect strong security engineering discipline.

However, because the live HTTP binary bypasses standard web framework safeguards, runs unauthenticated handlers against in-memory mock state, lacks a functional background worker, and omits the core business and tax fiscalization engines, **the system cannot be deployed to production in its current state**.

Executing the **4-Phase Remediation Plan** will transition Sitolo from a security-focused platform prototype into an enterprise-grade, production-ready business operating system.
