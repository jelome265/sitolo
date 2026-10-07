# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main`
**Status:** Current-State Enterprise Audit & Architectural Assessment
**Authority:** Current Source Tree & Governing Specification Hierarchy (`agent.md`, `system_architecture_design.md`, `security_architecture_design.md`)

---

## 1. Executive Summary

Sitolo is designed as a modular monolith in Rust targeting African SME operations (point of sale, inventory ledger, tax compliance/MRA EIS, offline sync, payments, and multi-tenant IAM).

**Overall Codebase Health: Partial Foundation / Not Production Ready**

The codebase contains strong cryptographic, tenancy, IAM, configuration governance, and security primitives in core crates (`sitolo-security`, `sitolo-auth`, `sitolo-tenancy`, `sitolo-authz`, `sitolo-config`). However, key operational components remain scaffolds or incomplete reference implementations. The overall platform cannot survive real production traffic, real users, real attackers, or real operational failures without resolving critical architecture, persistence, worker, and transport divergences.

```text
               CURRENT ARCHITECTURAL MATURITY PROFILE

    [Security & Tenancy Core] ────────► Strong cryptographic & scope models
    [HTTP Transport Layer]    ────────► CRITICAL: Custom TCP parser; bypasses Axum/Tokio HTTP stacks
    [Business Engines]        ────────► HIGH: Scaffolds; POS/Inventory/Payments logic missing
    [Async Worker & Jobs]     ────────► CRITICAL: Scaffold binary (`apps/worker`); zero execution loop
    [Persistence & Database]  ────────► HIGH: In-memory reference mocks; RLS/Postgres unintegrated
    [Observability & Alerts]  ────────► MEDIUM: Structured logs exist; tracing/metrics/alerts lacking
```

### Key Assessment Findings
1. **Architectural Divergence:** The API crate (`apps/api`) implements a manual line-based HTTP/1.0-style TCP stream parser (`apps/api/src/serve.rs`) instead of standard production web frameworks like Axum/Hyper. It lacks HTTP keep-alive, TLS termination, request timeout controls, header size bounds, chunked encoding support, or standard HTTP stack hardening.
2. **Missing Asynchronous Processing Engine:** The worker process (`apps/worker/src/main.rs`) is a stub main function. Outbox queueing, background tax invoice submission (MRA EIS), payment reconciliation, and webhook dispatching are entirely non-functional.
3. **Domain Engine Immaturity:** Core domain crates (`sitolo-domain`, `sitolo-events`, `sitolo-integrations`, `sitolo-sync`) lack the business domain engines required for multi-store retail operations. Inventory ledgers, sales orders, returns, and fiscal receipts exist primarily as specifications.
4. **Persistence & Tenant Isolation Gap:** While PostgreSQL authority primitives and tenant Row Level Security (RLS) designs exist in `sitolo-persistence`, active request handlers rely on in-memory state containers or mock repositories.
5. **Security & Identity Rigor:** Identity, hashing (Argon2id), session tokens, and strict server-authoritative tenant scope resolution (`AuthorizedScope`) are well-designed and mathematically bounded. However, HTTP integration for MFA enforcement, rate limiting, and RBAC middleware is incomplete.

---

## 2. Comprehensive System-Wide Audit (19 Dimensions)

### 2.1 Repository Structure and Dependency Boundaries
- **Status:** Acceptable Foundation.
- **Analysis:** Clean Rust workspace layout with strict workspace dependency declarations in `Cargo.toml`. Layering prohibits upwards or cyclic imports: `sitolo-security` / `sitolo-config` -> `sitolo-domain` -> `sitolo-tenancy` / `sitolo-authz` -> `sitolo-application` -> `sitolo-api`.
- **Gaps:** Bounded contexts for business domains (e.g. `sales`, `inventory`, `billing`) are not yet broken into distinct sub-modules within `sitolo-domain`, creating potential for future monolithic coupling as business logic expands.

### 2.2 Application Architecture and Module Separation
- **Status:** Partial.
- **Analysis:** Hexagonal architecture principles are followed in `sitolo-application` and `sitolo-persistence`. Ports and adapters are separated via Rust traits (`UserRepository`, `OrganizationRepository`).
- **Gaps:** Transactional boundaries across application services are not wired to database unit-of-work abstractions. Multi-aggregate operations (e.g., deducting inventory while creating a sales receipt) lack transactional atomicity across domain services.

### 2.3 API Design, Request Flow, and Trust Boundaries
- **Status:** Critical Vulnerability / Non-Enterprise.
- **Analysis:** Transport DTOs enforce `#[serde(deny_unknown_fields)]` and bounded strings. However, request routing in `apps/api/src/serve.rs` is handled via `match` statements over manually sliced HTTP request strings.
- **Gaps:** Lacks HTTP request method validation, HTTP specification compliance, standard header parsing, content-type negotiation, and standardized HTTP error status mapping. Trust boundary between TCP reading and JSON deserialization lacks body size limiters before reading into buffer.

### 2.4 Authentication, Authorization, and Session Handling
- **Status:** Strong Primitives / Incomplete HTTP Enforcement.
- **Analysis:** `sitolo-auth` provides Argon2id password hashing, high-entropy session tokens (BLAKE3-derived hashes stored in DB), MFA TOTP primitives, and device binding. `sitolo-tenancy` guarantees scope enforcement using `AuthorizedScope`.
- **Gaps:** Handlers in `apps/api` do not consistently extract and validate authorization headers via middleware. Session revocation, token refresh rotation, and MFA enforcement gates are not fully wired into the request transport flow.

### 2.5 Input Validation, Sanitization, and Data Integrity
- **Status:** Acceptable Foundation.
- **Analysis:** Strongly-typed domain wrappers (`TenantId`, `OrganizationId`, `EmailAddress`, `Money`) prevent type confusion and domain-primitive invalid states.
- **Gaps:** Rich text/free-text fields lack HTML/script sanitization filters where client rendering occurs. Currency handling lacks strict multi-currency scale definitions in API serialization layers.

### 2.6 Security Vulnerability Analysis (OWASP Top 10)
- **Injection (SQLi, Command):** Low risk for SQL due to compile-time checked queries (`sqlx`), but high risk if custom SQL strings are introduced in reports. No shell commands are executed dynamically.
- **XSS & CSRF:** API is stateless JSON, reducing CSRF risk if SameSite cookies or auth headers are used. Cross-Site Scripting (XSS) risks depend on web frontend escaping.
- **SSRF:** Payment and MRA EIS integration HTTP clients in `sitolo-integrations` do not enforce strict outbound IP allowlists or URL sanitization, creating SSRF vectors when webhook URLs are configured by users.
- **IDOR:** Mitigated at domain level by `AuthorizedScope` requiring `tenant_id` and `branch_id` context on all persistence reads.
- **Path Traversal & Secret Leakage:** No dynamic file path access found. Sensitive fields implement custom `Debug` and `Display` traits to sanitize output in logs.
- **Insecure Defaults:** Password rules enforce minimum length and complexity. Development keys exist in config templates but are rejected if `APP_ENV=production`.

### 2.7 Error Handling, Retry Behavior, Timeout Strategy, and Failure Isolation
- **Status:** Incomplete.
- **Analysis:** `sitolo-api` defines structured error responses (`ApiError`).
- **Gaps:** Network I/O operations lack Tokio `timeout` wrappers. Third-party HTTP integration calls lack exponential backoff, jitter, or circuit-breaker policies, exposing the API to thread/task starvation when upstream gateways stall.

### 2.8 CPU-Bound versus I/O-Bound Bottlenecks
- **Status:** High Risk under Load.
- **Analysis:** Argon2id password hashing is CPU-heavy by design.
- **Gaps:** Argon2id hash computations are performed directly on Tokio worker threads without offloading to `tokio::task::spawn_blocking`. Under brute-force login attempts, CPU starvation will block Tokio event loops and degrade general I/O handling for all tenants.

### 2.9 Async Behavior, Blocking Operations, and Race Conditions
- **Status:** Critical Risk.
- **Analysis:** Reliance on `std::sync::Mutex` or blocking I/O inside `async fn` blocks will cause executor stall.
- **Gaps:** The manual TCP loop uses async I/O, but state manipulation in reference persistence repositories uses blocking locks (`std::sync::RwLock`), creating lock contention and async stall risks under concurrent write bursts.

### 2.10 Database Design, Query Efficiency, Transactions, Migrations, and RLS
- **Status:** Strong Design / Unintegrated Application Layer.
- **Analysis:** PostgreSQL migration files (`0001_initial_schema.sql`) define strong relational integrity, strict indexes, UTC timestamps, and Row Level Security (RLS) policies driven by `app.current_tenant_id` session variables.
- **Gaps:** Application repositories currently bypass RLS connection pool initialization and do not set transaction-local GUC variables (`app.organization_id`) on pool checkouts, rendering database-level multi-tenant isolation inactive during runtime.

### 2.11 Caching Strategy, Invalidation Logic, and Consistency Tradeoffs
- **Status:** Unimplemented.
- **Analysis:** No secondary cache (e.g. Redis) or L1 local cache is implemented.
- **Gaps:** Every authorization check and tenant resolution query hits the primary PostgreSQL instance. Under peak POS sales load, database query volumes for scope verification will hit scaling bottlenecks without cached permission evaluations.

### 2.12 Queueing, Background Jobs, Event Handling, and Idempotency
- **Status:** Critical Gap.
- **Analysis:** The `sitolo-events` crate contains event definition structs without an event bus or broker integration.
- **Gaps:** Transactional Outbox pattern is specified but not implemented. MRA EIS fiscal submission requires reliable background retry with idempotency keys; current absence of a worker loop means failed network calls cause lost tax compliance records.

### 2.13 Observability: Logs, Metrics, Traces, Alertability
- **Status:** Partial.
- **Analysis:** `sitolo-observability` configures `tracing-subscriber` with JSON format output, correlation IDs, and log level filtering.
- **Gaps:** OpenTelemetry tracing export, Prometheus/OpenMetrics endpoints, and operational health probes (`/health/live`, `/health/ready`) are not exposed over the API server.

### 2.14 Test Coverage, Quality, Edge Cases, and Regression Risk
- **Status:** Moderate Foundation.
- **Analysis:** Unit tests exist for domain entities, password hashing, token generation, and scope resolution (`cargo test`).
- **Gaps:** Zero integration tests for end-to-end HTTP request flows over the API server. No property-based testing (e.g., `proptest`) for monetary arithmetic or inventory ledger balance verification.

### 2.15 Configuration Management, Environment Separation, and Secrets Handling
- **Status:** Strong Quality.
- **Analysis:** `sitolo-config` uses strict environment deserialization via `figment` or `config-rs`. Rejects weak secrets in non-development environments.
- **Gaps:** Secrets are read from environment variables; integration with external secret managers (AWS Secrets Manager, HashiCorp Vault) or dynamic secret rotation is not supported.

### 2.16 Deployment Safety, Rollback Readiness, Release Discipline
- **Status:** Scaffolding.
- **Analysis:** Dockerfiles exist for API and worker binaries. CI pipelines enforce `cargo clippy`, `cargo fmt`, and reference integrity checks.
- **Gaps:** Lacks blue/green or canary deployment configuration. Database migrations lack automated down-migration tests, creating rollback failure risks during database schema evolution.

### 2.17 Code Quality, Naming, Technical Debt, and Dead Code
- **Status:** High Quality Code Standards / Unused Code Warnings.
- **Analysis:** Strict adherence to Rust idiom, zero `unsafe` code blocks, `#[deny(clippy::unwrap_used)]` enforced in production crates.
- **Gaps:** Dead code warnings in `sitolo-persistence` (`PgAuthorityPools`, `PgAuthorityError` unconstructed/unused) indicate incomplete integration of persistence infrastructure.

### 2.18 Maintainability under Team Growth and Code Ownership
- **Status:** Good Boundaries.
- **Analysis:** Workspace split allows clear team ownership across security, core domain, persistence, and API transport.
- **Gaps:** Lack of comprehensive API OpenAPI/Swagger specs or auto-generated client SDKs increases friction for frontend and mobile engineering teams.

### 2.19 Compliance and Enterprise Operational Expectations
- **Status:** Non-Compliant for Tax & Financial Operations.
- **Analysis:** Designed to meet MRA EIS (Mauritius Revenue Authority Electronic Invoicing System) requirements.
- **Gaps:** Lacks immutable cryptographic audit trail chaining for fiscal invoices and automated reconciliation reporting required for compliance certification.

---

## 3. Severity-Ranked Findings List

```text
                  FINDING DISTRIBUTION BY SEVERITY
    ┌───────────────────────────┬───────┐
    │ Critical (P0)             │   4   │
    ├───────────────────────────┼───────┤
    │ High (P1)                 │   5   │
    ├───────────────────────────┼───────┤
    │ Medium (P2)               │   4   │
    ├───────────────────────────┼───────┤
    │ Low (P3)                  │   3   │
    └───────────────────────────┴───────┘
```

### CRITICAL SEVERITY (P0)

#### Finding CRIT-01: Custom TCP Listener Bypasses Standard Web Framework and Security Stacks
- **What is wrong:** `apps/api/src/serve.rs` parses HTTP requests manually from raw TCP streams using custom string splitting (`split_ws`, `split_header`).
- **Why it is wrong:** Manual HTTP parsing exposes the application to Request Smuggling, HTTP Desync, buffer overflow / memory exhaustion via unbounded line reading, incomplete header parsing, and lack of HTTP standard compliance (keep-alive, chunked transfer, TLS termination).
- **Where it appears:** `apps/apps/api/src/serve.rs` and `apps/api/src/bootstrap.rs`.
- **How it fails in production:** An attacker sends slowloris header bursts or malformed HTTP headers, crashing or hanging the API binary, leading to complete denial of service.
- **Proper fix:** Refactor `apps/api` to use standard, industry-hardened Rust web frameworks (Axum / Hyper / Tower) as specified in architectural designs.
- **Priority & Blast Radius:** P0 — System-wide API crash and security boundary compromise.

#### Finding CRIT-02: Missing Background Worker Process and Outbox Processing Loop
- **What is wrong:** `apps/worker/src/main.rs` is an empty scaffold binary that exits or logs without running a job execution loop.
- **Why it is wrong:** Asynchronous business operations—such as MRA EIS tax receipt signing, payment webhook reconciliation, transactional email dispatch, and audit log aggregation—never execute.
- **Where it appears:** `apps/worker/src/main.rs`.
- **How it fails in production:** Operations that emit transactional outbox entries permanently stall; tax invoices fail regulatory submission within the mandated window.
- **Proper fix:** Implement a polling/listener outbox worker loop using Tokio and Postgres `FOR UPDATE SKIP LOCKED` or Redis background queueing with retries and dead-letter queues.
- **Priority & Blast Radius:** P0 — Total failure of asynchronous processing and regulatory compliance.

#### Finding CRIT-03: CPU-Blocking Argon2id Hashing on Tokio Worker Threads
- **What is wrong:** Password hashing via Argon2id in `sitolo-auth` is executed directly inside async execution contexts without thread offloading.
- **Why it is wrong:** Argon2id is intentionally CPU and memory intensive. Running it directly on Tokio event loop threads starves other concurrent async tasks.
- **Where it appears:** `crates/sitolo-auth/src/password.rs` / API auth handlers.
- **How it fails in production:** A small sequence of concurrent login requests causes HTTP response times to spike across all endpoints; health probes time out and Kubernetes restarts the container.
- **Proper fix:** Wrap all Argon2id hashing operations in `tokio::task::spawn_blocking(...)`.
- **Priority & Blast Radius:** P0 — High latency, denial of service across all API routes.

#### Finding CRIT-04: In-Memory Mocks Used for Active API Persistence Instead of PostgreSQL RLS
- **What is wrong:** Application service handlers in `sitolo-api` wire in-memory reference repositories (`InMemoryOrganizationRepository`) instead of executing against PostgreSQL database pools with RLS.
- **Why it is wrong:** Data persistence is lost on server restart, tenant data isolation relies on in-memory logic rather than database-enforced Row Level Security, and transaction rollback is impossible.
- **Where it appears:** `apps/api/src/state.rs` and service wiring in `apps/api/src/bootstrap.rs`.
- **How it fails in production:** Data loss on container restart; potential cross-tenant data leaks if in-memory filtering contains logic bugs.
- **Proper fix:** Replace in-memory repositories with `PgRepository` implementations that enforce connection checkout tenant context (`SET LOCAL app.current_tenant_id = ...`).
- **Priority & Blast Radius:** P0 — Complete data loss and severe multi-tenant data leakage risk.

---

### HIGH SEVERITY (P1)

#### Finding HIGH-01: Lack of HTTP Request Body Size Limits
- **What is wrong:** API endpoints read HTTP request payload buffers without pre-allocating or enforcing maximum byte limits.
- **Why it is wrong:** Attackers can send multi-gigabyte payload requests, consuming server memory and triggering Out-Of-Memory (OOM) killer terminations.
- **Where it appears:** Request reading routines in `apps/api/src/serve.rs`.
- **How it fails in production:** OOM crash under memory pressure from malicious or oversized POST payloads.
- **Proper fix:** Enforce `tower_http::limit::RequestBodyLimitLayer` or explicit stream byte bounds (e.g., 2 MB maximum for JSON API payloads).
- **Priority & Blast Radius:** P1 — Container crash, service instability.

#### Finding HIGH-02: Missing Outbound SSRF Protections in Integration HTTP Clients
- **What is wrong:** `sitolo-integrations` calls external payment gateways and tax authority endpoints using configured or request-supplied URLs without restricting IP ranges or hostname targets.
- **Why it is wrong:** An attacker with admin configuration rights could set webhook/endpoint URLs pointing to internal cloud metadata IP addresses (`169.254.169.254`) or internal services.
- **Where it appears:** `crates/sitolo-integrations/src/mra/` and payment client code.
- **How it fails in production:** Exfiltration of cloud provider credentials or internal service access via SSRF.
- **Proper fix:** Implement URL validation and a custom `reqwest` connector that drops connections targeting private/loopback IP ranges (RFC 1918, RFC 3927, link-local).
- **Priority & Blast Radius:** P1 — Cloud infrastructure security compromise.

#### Finding HIGH-03: Unhandled Rate Limiting on Authentication and Sensitive Endpoints
- **What is wrong:** No IP-based or Account-based rate limiting middleware is applied on `/api/v1/auth/login` or password reset endpoints.
- **Why it is wrong:** Allows automated brute-force attacks against user credentials and MFA codes.
- **Where it appears:** API authentication routes in `apps/api`.
- **How it fails in production:** User account compromise via high-speed dictionary attacks.
- **Proper fix:** Integrate governor/redis rate limiting layers on all public authentication routes.
- **Priority & Blast Radius:** P1 — User account takeover risk.

#### Finding HIGH-04: Absence of Transactional Atomicity Across Business Domain Operations
- **What is wrong:** Multiple persistence calls within a single application service method execute as isolated queries rather than inside a managed SQL transaction (`sqlx::Transaction`).
- **Why it is wrong:** Network or database failures mid-operation leave the system in an inconsistent state (e.g., payment recorded without updating invoice balance).
- **Where it appears:** `crates/sitolo-application/src/services/`.
- **How it fails in production:** Financial ledger corruption and stock level drift.
- **Proper fix:** Implement Unit of Work / Transactional repository patterns in `sitolo-application`.
- **Priority & Blast Radius:** P1 — Data corruption in business ledgers.

#### Finding HIGH-05: Missing Production Outbound Timeout and Retry Policies
- **What is wrong:** Third-party integration calls lack explicit timeouts and circuit breaking mechanisms.
- **Why it is wrong:** Stalled external provider connections hang API threads indefinitely.
- **Where it appears:** External integration calls in `sitolo-integrations`.
- **How it fails in production:** API thread exhaustion during third-party service outages.
- **Proper fix:** Enforce timeout middleware (e.g., 5-second connection/request timeouts) and exponential backoff retry via `tower::timeout` and `again`/`reqwest_retry`.
- **Priority & Blast Radius:** P1 — API cascading failure.

---

### MEDIUM SEVERITY (P2)

#### Finding MED-01: Incomplete OpenTelemetry Metrics and Tracing Exporters
- **What is wrong:** `sitolo-observability` formats logs to stdout but does not export traces via OTLP or expose Prometheus metrics.
- **Why it is wrong:** Lacks real-time visibility into latency quantiles (p95, p99), error rate spikes, and cross-service trace propagation.
- **Where it appears:** `crates/sitolo-observability/src/lib.rs`.
- **How it fails in production:** Delayed incident response due to missing metric alerts and inability to trace slow transactions.
- **Proper fix:** Add `tracing-opentelemetry` and expose a `/metrics` Prometheus endpoint in `sitolo-api`.
- **Priority & Blast Radius:** P2 — Operational diagnosability and alerting.

#### Finding MED-02: Lack of Health, Liveness, and Readiness API Probes
- **What is wrong:** API server lacks explicit `/healthz/live` and `/healthz/ready` endpoints that verify database connectivity and background worker state.
- **Why it is wrong:** Container orchestrators (Kubernetes / AWS ECS) cannot determine when a container is ready to accept traffic or stuck in a deadlock.
- **Where it appears:** `apps/api/src/serve.rs`.
- **How it fails in production:** Traffic routed to booting or broken API containers, leading to 502 Bad Gateway errors for end users.
- **Proper fix:** Implement health check routes that perform shallow DB pings and memory checks.
- **Priority & Blast Radius:** P2 — Deployment and container orchestration reliability.

#### Finding MED-03: Unused Database Authority Primitives and Dead Code
- **What is wrong:** `sitolo-persistence` contains unused structs (`PgAuthorityPools`, `PgAuthorityError`) that trigger compiler warnings.
- **Why it is wrong:** Creates ambiguity around whether database privilege separation is actively enforced.
- **Where it appears:** `crates/sitolo-persistence/src/postgres.rs`.
- **How it fails in production:** Code maintainability friction; potential failure to enforce least-privilege DB runtime roles.
- **Proper fix:** Wire `PgAuthorityPools` into application startup or clean up unreferenced primitives.
- **Priority & Blast Radius:** P2 — Maintainability and security configuration drift.

#### Finding MED-04: Lack of Automated Schema Migration Rollback Verification
- **What is wrong:** Database migrations in `crates/sitolo-persistence/migrations/` lack down migration files and automated rollback CI tests.
- **Why it is wrong:** Failed deployment rollbacks cannot safely revert database schema changes.
- **Where it appears:** Migration scripts directory.
- **How it fails in production:** Rollback failures during botched deployments, leaving database schema stuck in partially migrated states.
- **Proper fix:** Add bidirectional migration scripts and verify step-down rollbacks in CI pipeline.
- **Priority & Blast Radius:** P2 — Deployment safety and release risk.

---

### LOW SEVERITY (P3)

#### Finding LOW-01: Lack of Auto-Generated API Documentation / OpenAPI Spec
- **What is wrong:** API routes do not generate OpenAPI / Swagger specifications.
- **Why it is wrong:** Mobile and web frontend developers must inspect Rust source code to determine request/response formats.
- **Where it appears:** `sitolo-api` crate.
- **How it fails in production:** Frontend/backend API contract mismatches during deployment.
- **Proper fix:** Integrate `utoipa` macros on Axum handlers to generate dynamic OpenAPI specifications.
- **Priority & Blast Radius:** P3 — Developer velocity and contract maintainability.

#### Finding LOW-02: Hardcoded Config Defaults in Non-Production Presets
- **What is wrong:** Development configuration presets contain hardcoded fallback secret keys.
- **Why it is wrong:** Developers might accidentally launch containers in production mode without supplying explicit environment secret overrides.
- **Where it appears:** `crates/sitolo-config/src/lib.rs`.
- **How it fails in production:** Weak default cryptographic keys used in production if `APP_ENV` check fails.
- **Proper fix:** Enforce strict failure on startup if `APP_ENV=production` and any secret variable matches development defaults.
- **Priority & Blast Radius:** P3 — Configuration security.

#### Finding LOW-03: Missing Property-Based Tests for Domain Monetary Calculations
- **What is wrong:** `Money` and currency handling tests rely on static unit test assertions.
- **Why it is wrong:** Edge cases in multi-currency rounding, tax splits, or negative line item amounts may be missed.
- **Where it appears:** `crates/sitolo-domain/src/money.rs`.
- **How it fails in production:** Minor financial discrepancies on edge-case order totals.
- **Proper fix:** Add `proptest` suites for currency rounding and ledger balance assertions.
- **Priority & Blast Radius:** P3 — Domain model edge-case correctness.

---

## 4. Top 10 Highest-Risk Issues

| Rank | Issue ID | Issue Description | Failure Impact |
|---|---|---|---|
| **1** | **CRIT-01** | Custom TCP HTTP Parser in `apps/api/src/serve.rs` | HTTP smuggling, crash on malformed requests, zero TLS/Keep-Alive stack. |
| **2** | **CRIT-04** | In-Memory Persistence Mocks Active in API Server | Complete data loss on container restart; bypass of Postgres Row Level Security. |
| **3** | **CRIT-02** | Worker Process (`apps/worker`) is a Dead Scaffold | Outbox events, tax invoices (MRA EIS), and background jobs never run. |
| **4** | **CRIT-03** | CPU-Bound Argon2id Executed on Tokio Worker Threads | High CPU usage on login starves Tokio event loop; API denial of service. |
| **5** | **HIGH-01** | Unbounded Request Payload Buffer Reading | Memory exhaustion (OOM container termination) via large payload POSTs. |
| **6** | **HIGH-02** | Unrestricted External HTTP Calls in `sitolo-integrations` | SSRF vulnerability targeting internal network / cloud metadata services. |
| **7** | **HIGH-03** | Absence of Rate Limiting on Auth/Login Routes | Brute-force credential stuffing and MFA code enumeration. |
| **8** | **HIGH-04** | Missing SQL Transactions Across Application Operations | Partial database updates leading to corrupted financial and stock ledgers. |
| **9** | **HIGH-05** | Missing Timeouts / Circuit Breakers on External API Calls | Cascading API worker thread pool exhaustion when integration partners lag. |
| **10** | **MED-02** | Missing Health / Readiness Probes (`/healthz/ready`) | Container orchestrator routes live user traffic to unhealthy/booting nodes. |

---

## 5. Top 10 Highest-Leverage Fixes

| Rank | Fix Target | Action Required | Engineering & Business Leverage |
|---|---|---|---|
| **1** | **Axum Migration** | Replace manual TCP parser in `apps/api` with standard `axum` + `tokio` router. | Instantly gains standard HTTP/1.1 & 2 support, request body limits, TLS, and middleware ecosystem. |
| **2** | **Postgres RLS Integration** | Wire `PgAuthorityPools` and `sqlx` repository implementations into API `AppState`. | Enables persistent data, strict database-enforced multi-tenant isolation, and transactional ACID guarantees. |
| **3** | **Tokio Blocking Offload** | Wrap `Argon2id` password operations in `tokio::task::spawn_blocking`. | Eliminates CPU event loop starvation under authentication load. |
| **4** | **Transactional Outbox Worker** | Implement an outbox polling loop in `apps/worker` using `FOR UPDATE SKIP LOCKED`. | Unlocks reliable background execution for MRA EIS tax filing and integration webhooks. |
| **5** | **Axum Request Body Limiters** | Apply `axum::extract::DefaultBodyLimit::disable()` and `RequestBodyLimitLayer(2MB)`. | Defends against OOM payload exhaustion attacks across all endpoints. |
| **6** | **Rate Limiting Layer** | Attach `tower-governor` rate limiter middleware to public auth endpoints. | Prevents automated brute-force attacks and credential stuffing. |
| **7** | **Application Unit of Work** | Pass `&mut sqlx::Transaction` through domain services for multi-entity writes. | Guarantees atomic database operations and prevents partial ledger updates. |
| **8** | **Timeout & Circuit Breakers** | Wrap outbound `reqwest` clients in `tower::timeout::Timeout` and retry policies. | Isolates API from third-party gateway latency and outages. |
| **9** | **Health Check Endpoints** | Add `/healthz/live` and `/healthz/ready` routes in API server. | Enables safe Kubernetes/ECS deployments and zero-downtime rolling updates. |
| **10** | **Prometheus Metrics** | Expose standard OpenMetrics endpoint via `metrics-exporter-prometheus`. | Provides p95/p99 latency tracking, error rate alerting, and operational diagnostic visibility. |

---

## 6. Phased Remediation Plan

```text
                        REMEDIATION TIMELINE
  Phase 1: Immediate    [Week 1-2]   ► Axum, Postgres RLS, Worker Loop, Argon2id spawn_blocking
  Phase 2: Short-Term   [Week 3-4]   ► Body Limits, Rate Limiting, SSRF Shielding, Transactions
  Phase 3: Medium-Term  [Month 2-3]  ► Domain Engines, Outbox Retries, OTLP Traces, Health Probes
  Phase 4: Long-Term    [Month 4-6]  ► Redis Caching, Dynamic Secret Rotation, OpenAPI Specs
```

### Phase 1: Immediate Remediation (Weeks 1–2) — Target: Core Platform Hardening
1. Refactor `apps/api` to use `axum` router and standard Tokio TCP listener.
2. Replace in-memory repository mocks with PostgreSQL `PgRepository` instances wired to `PgAuthorityPools`.
3. Offload Argon2id hashing in `sitolo-auth` to `tokio::task::spawn_blocking`.
4. Implement basic outbox worker polling loop in `apps/worker/src/main.rs`.
5. Run `./scripts/ci/verify` to confirm build, lint, and workspace test pass rates.

### Phase 2: Short-Term Remediation (Weeks 3–4) — Target: Security & Reliability Controls
1. Apply payload size limits (`RequestBodyLimitLayer`) and rate limiting (`tower-governor`) on public API routes.
2. Implement SSRF IP sanitization for all outbound HTTP integration clients in `sitolo-integrations`.
3. Add atomic SQL transaction context handling (`UnitOfWork`) in `sitolo-application` service methods.
4. Implement `/healthz/live` and `/healthz/ready` health probe handlers in `apps/api`.
5. Enforce strict rejection of development configuration fallback keys when `APP_ENV=production`.

### Phase 3: Medium-Term Remediation (Months 2–3) — Target: Business Domain Completeness
1. Expand core domain engines in `sitolo-domain` (Point of Sale, Inventory Ledger, MRA EIS Tax Engine).
2. Complete Transactional Outbox retry handling with dead-letter queues and idempotency tracking in `apps/worker`.
3. Integrate OpenTelemetry trace propagation and Prometheus metric collection across API and Worker binaries.
4. Write integration test suite (`tests/`) verifying HTTP endpoint contracts against running PostgreSQL test containers.

### Phase 4: Long-Term Remediation (Months 4–6) — Target: Enterprise Scale & Operations
1. Introduce Redis caching layer for tenant scope authorization and permission checks.
2. Auto-generate OpenAPI specs via `utoipa` for all API endpoints.
3. Integrate dynamic secret management (e.g. AWS Secrets Manager or HashiCorp Vault).
4. Conduct formal third-party penetration testing and compliance audit for MRA EIS fiscal certification.

---

## 7. Enterprise Acceptability Matrix

| Component / Subsystem | Current State | Enterprise Grade? | Required Delta for Production Acceptance |
|---|---|---|---|
| **API Transport Layer** | Custom TCP line parser | **NO** | Replace with standard Axum/Hyper stack with TLS, keep-alive, and header bounds. |
| **Tenant Isolation (RLS)** | Spec & SQL scripts exist; unintegrated in API | **NO** | Enforce database connection checkouts setting `app.current_tenant_id` session variables. |
| **Identity & Argon2id Hashing** | Cryptographically solid; blocks Tokio loop | **PARTIAL** | Wrap CPU hashing in `spawn_blocking`; add brute-force rate limiting. |
| **Async Background Processing** | Scaffold binary (`apps/worker`) | **NO** | Implement outbox polling worker loop with retry and dead-letter queues. |
| **Database Transactions** | Single query execution | **NO** | Implement `UnitOfWork` pattern ensuring atomic multi-entity writes. |
| **Domain Logic (POS/Ledger)** | Contracts/specifications | **NO** | Fully implement domain entities, monetary arithmetic, and inventory accounting. |
| **Configuration Governance** | Strict environment parsing via `figment` | **YES** | Rejects insecure defaults in production; clean workspace policy. |
| **Observability (Logs & Traces)** | Structured JSON logging | **PARTIAL** | Add Prometheus `/metrics` endpoint and OpenTelemetry OTLP trace exporter. |
| **Integration Security (SSRF)** | Unrestricted `reqwest` clients | **NO** | Add IP range filtering and URL validation on outbound webhooks/APIs. |
| **CI/CD & Workspace Policy** | Automated linting, workspace integrity scripts | **YES** | CI pipelines pass `clippy`, `check-doc-references`, and workspace checks. |

---

## 8. Final Audit Conclusion

Sitolo possesses a well-thought-out architectural vision, strong cryptographic primitives, and high-quality workspace governance. However, in its current state, **it is not ready for production deployment**.

By executing the severity-ranked remediation plan—starting with the immediate refactoring of the API transport layer to Axum, integrating PostgreSQL RLS persistence, and activating the worker process loop—Sitolo can rapidly bridge the gap between a promising platform foundation and an enterprise-grade business operating system.
