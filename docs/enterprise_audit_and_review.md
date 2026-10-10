# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-10-10
**Source baseline:** `main`
**Status:** Canonical current-state enterprise audit & review
**Authority:** Current source tree plus governing documentation hierarchy in `agent.md`

---

## 1. Executive Summary

Sitolo is a Rust-based modular monolith designed as a business operating system for African Small and Medium Enterprises (SMEs). This end-to-end enterprise audit evaluates the entire system across architecture, security, reliability, scalability, performance, maintainability, observability, testing, deployment readiness, and operational risk.

### Overall System Health
**Status:** **NOT PRODUCTION-READY (STRONG FOUNDATION / PARTIAL IMPLEMENTATION)**

The repository exhibits a high-quality foundation in domain modeling, strongly-typed security contracts, zero `unsafe` Rust code (`#![forbid(unsafe_code)]`), security-focused configuration management, and database Row-Level Security (RLS) catalog verification primitives. However, the system cannot be deployed to production in its current state due to critical architectural and runtime implementation gaps.

```text
========================================================================================
                               SYSTEM ARCHITECTURE POSTURE
========================================================================================
[ STRONGLY TYPED FOUNDATIONS ]   -->  Domain entities, IAM scope typing, RLS schemas
[ IMPLEMENTATION GAPS ]          -->  Custom raw TCP server vs Axum spec, empty background worker
[ SECURITY BOUNDARY RISKS ]      -->  Unauthenticated HTTP route handlers, missing authz checks
[ PRODUCTION READINESS GAP ]     -->  In-memory persistence default, stubbed domain engines
========================================================================================
```

### Key Architectural Strengths
1. **Strict Dependency Boundaries:** Strict unidirectional crate architecture enforced by workspace configuration, preventing circular dependencies and domain leaking.
2. **Type-Safe Tenancy Scope Resolution:** `sitolo-tenancy` strictly separates requested parameters from server-authoritative `AuthorizedScope`, preventing scope widening attacks.
3. **Defense-in-Depth Database RLS:** `sitolo-persistence/src/postgres.rs` validates catalog-level RLS policies, forced row security, and least-privileged `app_runtime` database roles prior to accepting queries.
4. **Zero-Unsafe Policy:** Complete workspace enforcement of `#![forbid(unsafe_code)]`.

### Key Production Blockers
1. **Transport Framework Divergence:** The architecture documentation specifies Axum + Tokio, whereas `apps/api/src/serve.rs` implements a manual raw TCP listener and HTTP string parser susceptible to transport-level attacks.
2. **Missing Authentication & Authorization Wiring:** The API route dispatcher dispatches tenancy operations without verifying bearer tokens, MFA tokens, or evaluating permission policies from `sitolo-authz`.
3. **In-Memory Store Reference Default:** Domain orchestration services default to in-memory `HashMap` persistence (`sitolo-persistence/src/memory.rs`), resulting in state loss on restart and inability to scale horizontally.
4. **Empty Asynchronous Worker Engine:** `apps/worker/src/main.rs` is an empty stub (`fn main() {}`), leaving outbox processing, audit logging, fiscal submission, and background job handling unimplemented.
5. **Stubbed Core Domain Engines:** Inventory, POS sales, payments, procurement, offline synchronization, and MRA EIS fiscalization remain contract specs or empty crates (`sitolo-integrations`, `sitolo-sync`, `sitolo-events`).

---

## 2. End-to-End System Evaluation Matrix

| Category | Evaluation | Current Baseline Status |
|---|---|---|
| **Repository Structure & Dependencies** | **Strong** | Pinned toolchain, deny.toml license/security checks, strict workspace crate layout. |
| **Application Architecture** | **Partial** | Modular monolith design with clean ports & adapters; application orchestrators depend on in-memory implementations. |
| **API Design & Request Flow** | **Brittle / Critical Gap** | DTO validation exists, but transport relies on raw TCP socket parsing without Axum pipeline. |
| **AuthN, AuthZ & Session Handling** | **Partial Foundation** | Rich primitives in `sitolo-auth` & `sitolo-authz`, but HTTP API dispatch does not enforce auth middleware. |
| **Input Validation & Data Integrity** | **Acceptable** | Bounded JSON parsing (`deny_unknown_fields`), body size limits; missing business-level invariant checks. |
| **Injection, XSS, CSRF, IDOR, SSRF** | **High Risk** | No raw SQL injection found due to SQLx parameter binding; high IDOR risk due to missing API authorization policy checks. |
| **Error Handling & Failure Isolation** | **Acceptable** | Strongly-typed `AppError` and RFC 7807 `ProblemDetails`; missing circuit breakers and retry jitter for external APIs. |
| **CPU vs I/O Bottlenecks** | **Acceptable** | Argon2id password hashing properly bounded; database queries rely on async Tokio tasks. |
| **Async, Race Conditions & Concurrency** | **High Risk** | In-memory locks (`tokio::sync::Mutex`) used for data isolation instead of cross-process DB transactions. |
| **Database Design & Schema Evolution** | **Strong Spec / Partial Runtime** | Dual-pool PostgreSQL architecture (`admin` vs `app_runtime`) with tenant RLS; full schema migrations are phase-gated. |
| **Caching Strategy & Consistency** | **Not Implemented** | No production cache layer (Redis); all state currently resides in memory or DB. |
| **Queueing, Workers & Outbox** | **Unimplemented** | Worker binary is empty; outbox event table lacks worker consumer loop and idempotency handling. |
| **Observability & Diagnosability** | **Partial** | Telemetry registry and `RequestId` propagation implemented; missing distributed tracing spans and exporter endpoints. |
| **Test Coverage & Quality** | **Partial** | Security and RLS integration tests exist, but business domain engines lack test suites. |
| **Configuration & Secrets** | **Strong** | Pinned environment configuration parsing, secret redaction (`sitolo-security`), and config fingerprinting. |
| **Deployment & Rollback Safety** | **Contract Spec** | Release contracts defined; deployment scripts, container health probes, and rollback runbooks are incomplete. |
| **Code Quality & Technical Debt** | **Strong Baseline** | Zero `unsafe` code, high code readability, strict linting (`clippy.toml`); tech debt concentrated in transport & workers. |
| **Compliance & Operational Risk** | **High Risk** | Fiscalization (MRA EIS) and payment gateway contracts defined but runtime engines are missing. |

---

## 3. Top 10 Highest-Risk Issues

1. **Unauthenticated HTTP Mutation Endpoints (`CRITICAL-01`)**: API route dispatcher processes tenancy lifecycle mutations without verifying session tokens or principal identity.
2. **Raw TCP Socket HTTP Server (`CRITICAL-02`)**: API uses a manual TCP connection loop with custom string split parsing instead of Axum, vulnerable to HTTP smuggling and DoS.
3. **In-Memory Volatile Persistence in Application Services (`CRITICAL-03`)**: Tenant and IAM state defaults to in-memory `HashMap` storage, losing data on restart and breaking multi-node deployments.
4. **Empty Worker Binary & Unprocessed Outbox (`CRITICAL-04`)**: `apps/worker` is an empty scaffold (`fn main() {}`), causing outbox events, background jobs, and fiscal logs to be dropped indefinitely.
5. **Missing Authorization Policy Enforcement on API Dispatch (`CRITICAL-05`)**: API endpoints do not evaluate `sitolo-authz` permission grants, creating severe IDOR and privilege escalation vulnerabilities.
6. **In-Memory Mutex Locking for Multi-Tenant Concurrency (`HIGH-01`)**: Concurrency control relies on in-memory `tokio::sync::Mutex` locks, causing race conditions across horizontally scaled API nodes.
7. **Lack of Transport-Level Rate Limiting & Slowloris Protection (`HIGH-02`)**: No connection rate limiting or per-route request timeouts at the transport boundary, exposing the API to thread starvation DoS.
8. **Unimplemented External Integration Adapters (`HIGH-03`)**: Payment gateway and MRA EIS tax integration crates (`sitolo-integrations`) are empty stubs, preventing real-world commercial transactions.
9. **Missing Distributed Tracing Spans and Telemetry Exporters (`MEDIUM-01`)**: Observability crate lacks open telemetry exporters and request-level tracing spans, hindering incident diagnosis in production.
10. **Incomplete Disaster Recovery and Chaos Verification (`MEDIUM-02`)**: System lacks automated failover testing and database connection drop recovery verification under high load.

---

## 4. Top 10 Highest-Leverage Fixes

1. **Migrate `apps/api` to Axum Web Framework**: Replace custom TCP parsing in `serve.rs` with Axum routers, extractors, and Tower middleware.
2. **Enforce `sitolo-auth` Middleware Across API Routes**: Add session authentication extractors to validate bearer tokens and inject `AuthenticatedPrincipal` into request extensions.
3. **Wire `sitolo-application` to PostgreSQL Persistence**: Bind application orchestrators to PostgreSQL pools using RLS context setting (`set_transaction_tenant_context`).
4. **Implement Outbox Worker Consumer Loop in `apps/worker`**: Build the poll-and-claim outbox processor in `apps/worker` using `FOR UPDATE SKIP LOCKED` database transactions.
5. **Integrate `sitolo-authz` Authorization Guard Extractors**: Require explicit permission checks (`authorizer.authorize(...)`) before executing domain command handlers.
6. **Implement Transport Middleware for Limits and Timeouts**: Add Tower middleware for body limits (`DefaultBodyLimit`), request timeouts, and rate-limiting buckets.
7. **Connect Domain Engines to Database Repositories**: Complete Phase 8-15 business domain logic (Catalogue, Ledger, POS, Payments) with PostgreSQL backing.
8. **Implement MRA EIS and Mobile Money Adapter Drivers**: Complete real adapter clients in `sitolo-integrations` with signature verification and retry/circuit-breaker logic.
9. **Configure OpenTelemetry Tracing and Prometheus Exporters**: Attach `tracing::instrument` macros to API handlers and export metrics at `/metrics`.
10. **Automate End-to-End Integration Test Suite in CI**: Configure full integration testing with live PostgreSQL and migration executions in CI pipeline.

---

## 5. Comprehensive Severity-Ranked Findings

---

### 5.1 Critical Severity Findings

#### Finding CRITICAL-01: Unauthenticated HTTP Mutation Endpoints
* **What is wrong:** The HTTP API dispatcher dispatches tenant provisioning and branch lifecycle mutations without authenticating the caller.
* **Why it is wrong:** Allows unauthenticated callers to execute privileged administrative actions.
* **Where it appears:** `apps/api/src/serve.rs` inside `dispatch_request()`.
* **How it fails in production:** An attacker sends an HTTP `POST /v1/organizations/org_123/suspend` request and successfully suspends an active tenant without presenting credentials.
* **What a proper fix looks like:** Implement Axum authentication middleware using `sitolo-auth` that inspects `Authorization: Bearer <token>` or session cookies, validates session freshness, and rejects unauthenticated requests with HTTP 401.
* **Priority & Blast Radius:** P0 / Critical. Entire API attack surface.

#### Finding CRITICAL-02: Documented HTTP Framework Divergence (Raw TCP vs Axum)
* **What is wrong:** Architecture specifications mandate Rust + Axum + Tokio, but `apps/api/src/serve.rs` implements a custom TCP socket server using `TcpListener`, `buf.windows()`, and manual string splitting.
* **Why it is wrong:** Manual HTTP parsers lack compliance with HTTP specifications (handling transfer-encoding, malformed headers, HTTP/2, request body streaming, header validation) and are prone to HTTP smuggling and denial-of-service vulnerabilities.
* **Where it appears:** `apps/api/Cargo.toml`, `apps/api/src/serve.rs`.
* **How it fails in production:** Malformed HTTP requests, slow headers, or HTTP desynchronization payloads crash the listener loop or allow header injection.
* **What a proper fix looks like:** Refactor `apps/api` to use `axum::Router`, standard extractors (`axum::Json`, `axum::Extension`), and Tower service layers.
* **Priority & Blast Radius:** P0 / Critical. Entire API transport layer.

#### Finding CRITICAL-03: Volatile In-Memory Persistence Default in Application Layer
* **What is wrong:** `sitolo-application` services bind to `TenancyDatabase` in-memory reference implementations by default (`sitolo-persistence/src/memory.rs`).
* **Why it is wrong:** All tenant, organization, branch, and IAM state resides in volatile process memory (`HashMap` behind `RwLock`).
* **How it fails in production:** Server restarts wipe all application state. Running multiple API replicas causes split-brain data corruption as each replica maintains an isolated memory store.
* **What a proper fix looks like:** Rebind application orchestrators to PostgreSQL repository implementations utilizing `PgAuthorityPools` and transaction-local RLS configuration (`set_transaction_tenant_context`).
* **Priority & Blast Radius:** P0 / Critical. Core database persistence and data durability.

#### Finding CRITICAL-04: Empty Background Worker Binary & Unprocessed Outbox
* **What is wrong:** `apps/worker/src/main.rs` contains an empty `main` function (`fn main() {}`) and no event processing loop.
* **Why it is wrong:** Asynchronous tasks, domain outbox event dispatch, fiscal tax logging (MRA EIS), payment reconciliation, and audit event processing are never executed.
* **How it fails in production:** Outbox records accumulate indefinitely in the database, background reconciliations never run, and fiscal compliance filings fail silent.
* **What a proper fix looks like:** Implement a resilient worker processing loop in `apps/worker/src/main.rs` with graceful shutdown, outbox claiming (`SELECT ... FOR UPDATE SKIP LOCKED`), exponential backoff retries, and dead-letter queueing.
* **Priority & Blast Radius:** P0 / Critical. Asynchronous event infrastructure & background jobs.

#### Finding CRITICAL-05: Missing Authorization Policy Enforcement on API Routes
* **What is wrong:** API handlers do not evaluate authorization policies from `sitolo-authz` prior to executing application operations.
* **Why it is wrong:** Even if a user authenticates, any authenticated user can invoke operations across organizations or branches regardless of their assigned role or granted scope.
* **How it fails in production:** A regular cashier user at Branch A sends a request to close Branch B or modify Organization root settings, and the request succeeds due to missing permission guards.
* **What a proper fix looks like:** Create an Axum `AuthorizationGuard` extractor that accepts required permissions (e.g. `Permission::BranchClose`) and verifies that the principal's `AuthorizedScope` grants the required privilege before proceeding.
* **Priority & Blast Radius:** P0 / Critical. Security, IDOR, and multi-tenant authorization boundary.

---

### 5.2 High Severity Findings

#### Finding HIGH-01: In-Memory Mutex Locking for Concurrency Isolation
* **What is wrong:** Concurrency and tenant isolation in `sitolo-persistence/src/memory.rs` rely on `tokio::sync::Mutex` and `std::sync::RwLock`.
* **Why it is wrong:** In-memory locks do not provide ACID transaction isolation across multiple process instances or multi-threaded database transactions.
* **Where it appears:** `crates/sitolo-persistence/src/memory.rs`.
* **How it fails in production:** Under concurrent requests, race conditions cause duplicate entity creation or dirty writes; across multiple API instances, locking is completely ineffective.
* **What a proper fix looks like:** Replace in-memory locks with PostgreSQL ACID transactions using `BEGIN ... COMMIT` and explicit row locking (`SELECT ... FOR UPDATE`) backed by RLS context.
* **Priority & Blast Radius:** P1 / High. Data integrity and multi-node concurrency.

#### Finding HIGH-02: Transport Layer Unprotected Against DoS and Slowloris
* **What is wrong:** The TCP listener in `apps/api/src/serve.rs` reads requests with a flat 5-second timeout and fixed buffer without connection rate limits or per-IP throttling.
* **Why it is wrong:** An attacker can open 1,000 slow connections (exhausting `MAX_IN_FLIGHT_CONNECTIONS = 1000`) and hold them open, preventing legitimate clients from connecting.
* **Where it appears:** `apps/api/src/serve.rs`.
* **How it fails in production:** API becomes completely unresponsive under low-bandwidth connection floods or HTTP Slowloris attacks.
* **What a proper fix looks like:** Utilize Axum/Tower middleware: `tower_http::limit::RequestBodyLimitLayer`, `tower::timeout::TimeoutLayer`, and rate-limiting middleware (`governor` or token bucket).
* **Priority & Blast Radius:** P1 / High. System availability and DDoS protection.

#### Finding HIGH-03: Unimplemented External Payment & Fiscalization Integrations
* **What is wrong:** Crates `sitolo-integrations` and `sitolo-sync` contain no concrete implementation drivers for payment providers (Airtel Money, MTN MoMo) or tax authorities (MRA EIS).
* **Why it is wrong:** The core commercial value proposition (point-of-sale payments, fiscal compliance, offline device sync) cannot function.
* **Where it appears:** `crates/sitolo-integrations/src/lib.rs`, `crates/sitolo-sync/src/lib.rs`.
* **How it fails in production:** Payment processing attempts fail with unimplemented errors; merchants cannot issue fiscalized invoices required by law.
* **What a proper fix looks like:** Implement integration drivers with robust HTTP clients, TLS pinning, HMAC signature validation, idempotent retries, and offline queueing.
* **Priority & Blast Radius:** P1 / High. Core commercial functionality and legal compliance.

#### Finding HIGH-04: Lack of Transactional Outbox Pattern in Application Handlers
* **What is wrong:** Application command handlers mutate domain entities in memory without transactionally writing audit logs or outbox events to persistent storage in the same atomic unit of work.
* **Why it is wrong:** Dual-write problem: if state updates succeed but event publishing fails, domain events and audit logs are lost, causing state inconsistency between API and background workers.
* **Where it appears:** `crates/sitolo-application/src/tenancy.rs`.
* **How it fails in production:** Organization provisioning succeeds in DB, but the outbox write fails due to network interrupt; downstream systems (billing, notification, search index) never receive the creation event.
* **What a proper fix looks like:** Wrap entity mutations and outbox table inserts into a single PostgreSQL transaction (`sqlx::Transaction`).
* **Priority & Blast Radius:** P1 / High. Data consistency and event-driven architecture integrity.

---

### 5.3 Medium Severity Findings

#### Finding MEDIUM-01: Incomplete Observability and Distributed Tracing
* **What is wrong:** `sitolo-observability` provides structured log formatting and counter buffers, but API request handlers do not consistently attach distributed tracing spans or expose Prometheus metrics scrapers.
* **Why it is wrong:** Operational monitoring cannot track end-to-end request latency, database query bottlenecks, or cross-service trace propagation.
* **Where it appears:** `apps/api/src/serve.rs`, `crates/sitolo-observability/src/lib.rs`.
* **How it fails in production:** During latency spikes or partial outages, SRE teams cannot identify which database query or external dependency is failing.
* **What a proper fix looks like:** Annotate API handlers with `#[tracing::instrument]`, propagate W3C TraceContext headers, and expose standard Prometheus metrics at `/metrics`.
* **Priority & Blast Radius:** P2 / Medium. Observability and incident diagnosability.

#### Finding MEDIUM-02: Secret Defaults and Fallback Risks in Configuration
* **What is wrong:** `sitolo-config` validates configuration fields but includes permissive development fallback logic for database connection strings and session signing keys.
* **Why it is wrong:** Misconfigured production environments might fall back to weak default development keys without throwing a fatal startup error.
* **Where it appears:** `crates/sitolo-config/src/validate.rs`.
* **How it fails in production:** An operator forgets to set `JWT_SECRET` in production, and the application starts using a default hardcoded secret, allowing token forgery.
* **What a proper fix looks like:** Strictly require non-empty, non-default production secrets when `APP_ENV=production` and crash immediately during startup validation if defaults are detected.
* **Priority & Blast Radius:** P2 / Medium. Security configuration governance.

#### Finding MEDIUM-03: Absence of End-to-End Integration and DR Verification
* **What is wrong:** Test suite heavily relies on unit tests and in-memory mocks; live database integration tests are gated on manual environment variable injection (`ADMIN_DATABASE_URL`).
* **Why it is wrong:** Automated CI pipelines do not continuously run full integration tests against real PostgreSQL instances with RLS policies enforced.
* **Where it appears:** `crates/sitolo-persistence/tests/rls_security_tests.rs`.
* **How it fails in production:** Schema migrations or RLS policy changes that break existing application queries pass CI unnoticed.
* **What a proper fix looks like:** Configure CI pipeline to spin up a PostgreSQL service container, execute database migrations, and run all integration tests automatically.
* **Priority & Blast Radius:** P2 / Medium. Test coverage and regression prevention.

---

### 5.4 Low Severity Findings

#### Finding LOW-01: Public Export Gaps in Persistence Module Crate
* **What is wrong:** Module visibility in `crates/sitolo-persistence/src/lib.rs` required explicit public re-export (`pub mod postgres;`) to allow external integration test targets to compile cleanly.
* **Why it is wrong:** Inconsistent `pub(crate)` vs `pub` visibility across library boundaries causes compilation warnings or test target build failures.
* **Where it appears:** `crates/sitolo-persistence/src/lib.rs`.
* **How it fails in production:** Development friction and build errors when creating new integration test suites.
* **What a proper fix looks like:** Standardize crate public API re-exports across `sitolo-persistence` and document module visibility contracts.
* **Priority & Blast Radius:** P3 / Low. Developer experience and code maintainability.

#### Finding LOW-02: String Splitting and Unescaped Log Placeholders
* **What is wrong:** Certain error formatting and helper utilities in API transport use manual string concatenation and basic JSON string escaping routines (`json_escape()`).
* **Why it is wrong:** Manual string construction is prone to subtle JSON formatting errors or incomplete character escaping when handling special control characters.
* **Where it appears:** `apps/api/src/serve.rs`.
* **How it fails in production:** Unexpected control characters in organization names cause malformed JSON error responses.
* **What a proper fix looks like:** Use standard `serde_json::to_string()` for all response serialization.
* **Priority & Blast Radius:** P3 / Low. Code quality and formatting robustness.

---

## 6. Phased Remediation Plan

```text
========================================================================================
                              PHASED REMEDIATION ROADMAP
========================================================================================
[ PHASE 1: IMMEDIATE ]   --> Migrate API to Axum, enforce AuthN/AuthZ middleware
[ PHASE 2: SHORT TERM ]  --> Wire PostgreSQL persistence, build outbox worker loop
[ PHASE 3: MEDIUM TERM ] --> Implement domain engines (POS, Inventory, MRA EIS, Payments)
[ PHASE 4: LONG TERM ]   --> Enable offline device sync, E2E chaos testing, production DR
========================================================================================
```

### Phase 1: Immediate Remediation (Sprint 1 - SRE & Security Readiness)
* **P1.1 Transport Refactoring:** Refactor `apps/api` to use Axum + Tokio, replacing custom TCP listener code (`CRITICAL-02`).
* **P1.2 Auth Middleware Integration:** Add session token extraction and validation middleware to all non-public API endpoints (`CRITICAL-01`).
* **P1.3 Authorization Guards:** Implement `sitolo-authz` policy checks on API route handlers (`CRITICAL-05`).
* **P1.4 Transport Rate Limiting:** Add Tower middleware for body limits, connection timeouts, and IP rate limiting (`HIGH-02`).

### Phase 2: Short-Term Remediation (Sprint 2-3 - Data & Process Integrity)
* **P2.1 PostgreSQL Persistence Wiring:** Rebind `sitolo-application` services to PostgreSQL repositories using tenant RLS session context (`CRITICAL-03`).
* **P2.2 Outbox Background Worker:** Implement outbox worker consumer loop in `apps/worker` with `SKIP LOCKED` processing (`CRITICAL-04`).
* **P2.3 Transactional Outbox Pattern:** Ensure all command handlers execute domain mutations and outbox writes inside atomic SQL transactions (`HIGH-04`).
* **P2.4 Production Config Validation:** Enforce fatal startup failures if default secrets are used in production mode (`MEDIUM-02`).

### Phase 3: Medium-Term Remediation (Sprint 4-6 - Business Domain Engine Delivery)
* **P3.1 Core Business Engines:** Implement domain logic and database schema for Product Catalogue (Phase 8), Inventory Ledger (Phase 9), and POS Sales (Phase 10).
* **P3.2 External Integration Drivers:** Build payment gateway adapters (MTN MoMo, Airtel Money) and MRA EIS fiscal invoice submission clients (`HIGH-03`).
* **P3.3 OpenTelemetry & Prometheus:** Attach tracing spans across all API handlers and database query execution, exposing `/metrics` endpoints (`MEDIUM-01`).
* **P3.4 Automated Integration CI:** Configure CI pipeline to execute full integration test suites against live PostgreSQL containers (`MEDIUM-03`).

### Phase 4: Long-Term Readiness (Sprint 7+ - Production Scale & Certification)
* **P4.1 Offline Synchronization:** Implement domain-aware offline device sync protocol (`sitolo-sync`) with conflict resolution.
* **P4.2 Chaos & DR Testing:** Implement automated database failover, network partition, and load test suites.
* **P4.3 Production Certification:** Complete Phase 20 Production Certification and security penetration testing audit.

---

## 7. Operational Standards: Acceptable vs. Non-Enterprise-Grade

To maintain high software engineering discipline, the Sitolo repository enforces clear boundaries between acceptable enterprise practices and forbidden non-enterprise patterns.

```text
+----------------------------------------------------+----------------------------------------------------+
| ACCEPTABLE ENTERPRISE-GRADE PRACTICE               | NON-ENTERPRISE-GRADE / FORBIDDEN PATTERN           |
+----------------------------------------------------+----------------------------------------------------+
| 1. Axum + Tokio structured HTTP routing            | 1. Custom raw TCP socket string parsing            |
| 2. PostgreSQL authority with Row-Level Security    | 2. In-memory HashMap default state in production   |
| 3. Mandatory AuthN + AuthZ extractors on API       | 3. Unauthenticated HTTP handler dispatch           |
| 4. Atomic SQL transactions with transactional outbox| 4. Dual-writes with uncommitted background jobs   |
| 5. Outbox worker loop with SKIP LOCKED             | 5. Empty worker main scaffold                      |
| 6. Complete #![forbid(unsafe_code)] workspace policy| 6. Unsafe memory blocks or direct pointer tricks   |
| 7. Explicit non-default production secrets        | 7. Hardcoded default secret fallbacks in prod      |
| 8. Distributed tracing & Prometheus telemetry     | 8. Unstructured print statements or ignored errors |
+----------------------------------------------------+----------------------------------------------------+
```

---

## 8. Conclusion & Sign-Off

The Sitolo repository possesses an exceptionally strong architectural blueprint and high-quality core domain primitives. By executing the phased remediation plan—starting immediately with transport refactoring to Axum, authentication/authorization middleware integration, and PostgreSQL persistence wiring—Sitolo will transition into a robust, enterprise-grade business operating system ready for African SME deployment.
