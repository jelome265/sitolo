# Sitolo Enterprise Architecture & Codebase Security Audit Report

**Target System:** Sitolo — Business Operating System for African SMEs
**Audit Date:** 2026-09-26
**Target Environment:** Multi-Tenant Enterprise Microservices / Modular Monolith
**Source Baseline:** `main` (commit `e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc`)
**Status:** Canonical Enterprise Audit & Review
**Authority:** Root repository source tree plus governing specification hierarchy (`../AGENTS.md`, `../CLAUDE.md`, `../agent.md`)

---

## Executive Summary & Overall Codebase Health

Sitolo is currently **NOT READY FOR PRODUCTION**. While the codebase demonstrates exceptional engineering discipline in foundational security primitives, domain typing, and isolation contracts, it remains an incomplete platform foundation rather than an operational enterprise business system.

### Overall Codebase Health Rating: **DEVELOPMENT / PLATFORM FOUNDATION ONLY (5.5 / 10)**

```text
       [ ARCHITECTURAL SCAFFOLD & SECURITY PRIMITIVES ]  <-- STRENGTH
                             ↓
       [ CUSTOM TCP TRANSPORT & DIVERGENT HTTP SPEC ]    <-- CRITICAL GAP
                             ↓
       [ UNFINISHED BUSINESS ENGINES & WORKER SCAFFOLDS ] <-- CRITICAL GAP
                             ↓
       [ UNCOMMITTED/MISSING DISTRIBUTED INFRASTRUCTURE ] <-- HIGH GAP
```

### Key Strengths (Acceptable Enterprise-Grade Elements)
1. **Security & Identity Primitives (`sitolo-auth`, `sitolo-security`, `sitolo-authz`):** Outstanding implementation of security-versioned tokens, session sliding, MFA challenges, CSPRNG token generation, password verification (using Argon2 and Bcrypt) with cost-version upgrading, and least-privilege role/permission matrices.
2. **Tenant Isolation & Authority Model (`sitolo-tenancy`, `sitolo-persistence`):** Multi-tenant scope resolution (`AuthorizedScope`) strictly isolates requested vs. trusted scopes. PostgreSQL Row-Level Security (RLS) enforcement on `organizations`, `branches`, and `tenant_resources` uses separate administrative (`admin_pool`) and runtime (`app_runtime`) database roles.
3. **Configuration & Governance (`sitolo-config`, `deny.toml`, toolchain pinning):** Enforces strict `#![forbid(unsafe_code)]` across all crates, pinned toolchains (Rust 1.98.1), deterministic configuration hashing/fingerprinting, and strict environment separation without hardcoded production defaults.
4. **Structured Telemetry Substrate (`sitolo-observability`):** Priority-aware, thread-safe ring buffering with priority eviction (security logs displace debug noise) and bounded trace/request ID contexts.

### Critical Production Blockers (Non-Enterprise-Grade Elements)
1. **HTTP Transport Divergence & Custom TCP Protocol Parser:** The governing architecture (`agent.md`, API specification) mandates Rust + Axum + Tokio + Hyper. However, `apps/api/src/serve.rs` implements a direct, custom TCP parsing loop (`TcpListener::accept()`) that reads up to 32 KB without verifying HTTP `Content-Length`, chunked encoding, header continuation, or TLS termination.
2. **Missing Core Business Domain Engines:** Core SME engines—Product Catalogue (Phase 8), Inventory Ledger (Phase 9), POS Sales (Phase 10), Payments & Reconciliation (Phase 11), Offline Sync (Phase 12), and MRA EIS Fiscalization (Phase 15)—are contract specifications or empty crate scaffolds (`sitolo-events`, `sitolo-integrations`, `sitolo-sync`).
3. **Worker & Async Processing Scaffold:** `apps/worker/src/main.rs` is an empty `fn main() {}` scaffold. Background job processing, transactional outbox consumption, event publication, and payment webhook reconciliation do not exist in executable code.
4. **Tokio Async Worker Thread Starvation:** Expensive cryptographic operations (Argon2 and Bcrypt password hashing) in `sitolo-application/src/identity.rs` are executed directly within Tokio async tasks without offloading to `tokio::task::spawn_blocking`. Under concurrent login load, CPU-heavy hashing starves async worker threads and halts I/O processing.
5. **In-Memory Synchronization Bottlenecks:** Reference implementations in `sitolo-application` and `sitolo-persistence/src/memory.rs` use `std::sync::Mutex` across state collections, creating single-node concurrency bottlenecks that prevent horizontal scaling across multiple API instances.

---

## 2. Current System Baseline & Authority Hierarchy

The Sitolo governance model enforces the following normative hierarchy:
1. Regulatory & Statutory Law (e.g., MRA EIS Fiscalization standards)
2. External Provider & Financial Integration Contracts
3. Product & Business Architecture Contracts
4. Security Architecture & Security Implementation Contracts (`agent.md`)
5. Domain Entity & Lifecycle Models
6. Database Schema & API Specifications
7. Observability, Deployment, & Reliability Specs
8. Architecture Decision Records (ADRs)
9. Source Code Implementation

### Repository Topography
- `apps/api`: API binary and listener lifecycle (`sitolo-api-bin`).
- `apps/worker`: Background worker scaffold (`sitolo-worker-bin`).
- `crates/sitolo-auth`: Identity, session, MFA, device, and security-versioning engine.
- `crates/sitolo-authz`: Roles, permissions, assignments, scope grants, and invitations.
- `crates/sitolo-domain`: Business domain primitives (currently tenancy-only).
- `crates/sitolo-tenancy`: Server-authoritative scope typing (`AuthorizedScope`, `EffectiveScope`).
- `crates/sitolo-application`: Service orchestration for identity and tenancy.
- `crates/sitolo-api`: HTTP DTOs, route handlers, error mappings, and validation bounds.
- `crates/sitolo-persistence`: Memory stores and PostgreSQL authority/RLS controls.
- `crates/sitolo-security`: CSPRNG, reference sealing, string redaction, and secret values.
- `crates/sitolo-observability`: Bounded priority buffers, trace contexts, and metrics schemas.
- `crates/sitolo-events`, `sitolo-integrations`, `sitolo-sync`: Empty crate scaffolds.

---

## 3. Enterprise Assessment Matrix (19 System Dimensions)

| Dimension | Enterprise Readiness | Status & Summary of Evidence |
|---|---|---|
| **1. Repository Structure & Boundaries** | **PARTIAL** | Clear workspace boundaries (`crates/`, `apps/`); strict `deny.toml` and `#![forbid(unsafe_code)]`. Scaffolds exist for worker and integration crates. |
| **2. Application Architecture** | **NON-ENTERPRISE** | Core domain engines (inventory, sales, payments, fiscalization) are missing. Application services cover tenancy/IAM only. |
| **3. API Design & Trust Boundaries** | **CRITICAL RISK** | Custom TCP request loop in `apps/api/src/serve.rs` replaces specified Axum framework. Unbounded HTTP request parsing, lack of middleware stack (CORS, CSRF, TLS, compression). |
| **4. Auth, Authz & Session Handling** | **ACCEPTABLE (CORE)** | High-quality token security, session sliding, security versions, and permission resolution. Lacks HTTP-level session extraction middleware. |
| **5. Input Validation & Integrity** | **PARTIAL** | DTOs enforce `deny_unknown_fields` and body byte limits (`MAX_TENANCY_BODY_BYTES`), but custom HTTP header parser is vulnerable to raw input malformation. |
| **6. Boundary Security (Injection/XSS/SSRF/IDOR)** | **PARTIAL** | SQL injection prevented by `sqlx` parameterized queries and RLS. Missing CSRF checks, security headers, and SSRF outbound validation in integrations. |
| **7. Error Handling, Retries & Isolation** | **PARTIAL** | Clean `ProblemDetails` (RFC 7807) error responses. Missing backoff, circuit breakers, and failure isolation for external integrations. |
| **8. CPU vs I/O Bottlenecks** | **CRITICAL RISK** | Argon2/Bcrypt CPU-bound hashing runs directly on Tokio async reactor threads without `spawn_blocking`, blocking I/O tasks under load. |
| **9. Concurrency & Async Safety** | **HIGH RISK** | Single-threaded TCP accept loop; in-process `std::sync::Mutex` locks block Tokio threads; non-distributed state prevents horizontal scaling. |
| **10. Database, Queries & RLS** | **PARTIAL** | PostgreSQL dual-pool authority (`admin_pool` / `app_runtime`) and RLS policies are well-engineered. Business schemas and migration rollbacks are phase-gated. |
| **11. Caching & Consistency** | **NON-ENTERPRISE** | No distributed cache (Redis) implemented. Invalidation relies on in-memory version bumping. |
| **12. Queueing, Events & Idempotency** | **CRITICAL RISK** | `apps/worker` is an empty scaffold (`fn main() {}`). No transactional outbox worker, queue consumer, or event publication runtime. |
| **13. Observability & Diagnosability** | **PARTIAL** | In-memory priority ring buffer implemented; lacks Prometheus/OTLP exporter integration in API binary. |
| **14. Test Coverage & Quality** | **PARTIAL** | Excellent unit test coverage for auth and tenancy domain rules (53+ tests). Persistence RLS tests panic when Postgres is offline (`FAIL-CLOSED`). |
| **15. Configuration & Secrets** | **ACCEPTABLE** | Deterministic configuration fingerprinting; strict environment separation; secrets wrapped in `SecretValue` and redacted in logs. |
| **16. Deployment & Rollback Safety** | **NON-ENTERPRISE** | Liveliness/readiness endpoints exist. Lacks deployment manifests, blue-green deployment strategies, and automated DB migration rollback checks. |
| **17. Code Quality & Tech Debt** | **PARTIAL** | Clean Rust code structure, but contains dead code/warnings in `sitolo-persistence` (`PgAuthorityPools` unused when DB tests excluded) and empty scaffolds. |
| **18. Maintainability & Growth Scale** | **ACCEPTABLE** | Modular monolith design allows clean crate splitting; System Map and ICM governance maintain semantic integrity. |
| **19. Compliance & Operations** | **NON-ENTERPRISE** | MRA EIS fiscalization and payment gateway integrations are contract specifications only with no executable code. |

---

## 4. Severity-Ranked Findings List

### CRITICAL SEVERITY FINDINGS

#### Finding C-01: Custom TCP HTTP Request Parser Vulnerable to Request Smuggling, Body Truncation, and Denial of Service
- **What is wrong:** `apps/api/src/serve.rs` implements a custom TCP request handler (`handle_connection`) using `TcpListener::accept()` and raw `stream.read()` into a fixed 32 KB buffer (`PROBE_MAX_BYTES`).
- **Why it is wrong:** The architecture contract specifies Axum + Tokio + Hyper. The custom parser does not handle `Content-Length` validation, HTTP chunked transfer encoding, multi-line headers, HTTP pipelining, or TLS termination.
- **Where it appears:** `apps/api/src/serve.rs:60-101`
- **How it fails in production:** If a client sends an HTTP request where the headers or body are split across multiple TCP packets, `stream.read()` returns the first chunk only. The JSON payload is truncated, causing request parsing to fail or process partial data. An attacker can send slow HTTP headers (Slowloris attack) or crafted HTTP pipelined requests to bypass request limits or lock worker tasks.
- **What a proper fix looks like:** Rebind `apps/api` to use the standard `axum` web framework built on `hyper` and `tower`. Replace `serve.rs` custom parsing with Axum route handlers, standard middleware (`tower_http::trace`, `tower_http::limit::RequestBodyLimitLayer`), and proper Tokio connection management.
- **Priority:** P0 (Immediate)
- **Blast Radius:** ENTIRE API TRANSPORT LAYER (All HTTP Endpoints)

#### Finding C-02: Empty Background Worker Scaffold (`apps/worker`) Prevents Asynchronous Execution and Outbox Consumption
- **What is wrong:** `apps/worker/src/main.rs` contains only `fn main() {}`. There is no job consumer, queue listener, or transactional outbox worker loop.
- **Why it is wrong:** Enterprise operations (sending email invitations, processing payment webhooks, submitting MRA EIS fiscal invoices, background reconciliation) must run asynchronously out of the HTTP request path.
- **Where it appears:** `apps/worker/src/main.rs:1-11`, `crates/sitolo-events/src/lib.rs:1-6`
- **How it fails in production:** Asynchronous tasks cannot execute. Any domain operation relying on outbox records (e.g., invitation emails) stalls indefinitely. Synchronous inline execution of external side-effects will cause HTTP requests to timeout or fail when external providers degrade.
- **What a proper fix looks like:** Implement an outbox worker processor in `apps/worker` using PostgreSQL `FOR UPDATE SKIP LOCKED` polling or notification queues. Consume `audit_outbox` and `event_outbox` tables and process side-effects with idempotent retry classification.
- **Priority:** P0 (Immediate)
- **Blast Radius:** ENTIRE SYSTEM ASYNCHRONOUS ENGINE

#### Finding C-03: CPU-Bound Password Hashing on Tokio Reactor Threads Causes Async Runtime Thread Starvation
- **What is wrong:** In `crates/sitolo-application/src/identity.rs`, methods `login_password` and `change_password` execute `self.hasher.verify()` and `self.hasher.hash()` directly inside async functions without delegating to `tokio::task::spawn_blocking`.
- **Why it is wrong:** Argon2 and Bcrypt password hashing algorithms are intentionally CPU-bound and memory-hard. Running them directly on a Tokio async reactor thread blocks that thread from polling other async futures for tens to hundreds of milliseconds.
- **Where it appears:** `crates/sitolo-application/src/identity.rs:199-231`, `crates/sitolo-application/src/identity.rs:374-388`
- **How it fails in production:** Under moderate concurrent login traffic (e.g., 50 requests/sec), all Tokio worker threads become blocked executing Argon2 computations. Non-CPU tasks (health probes, database queries, light API requests) stall completely, resulting in high latency spikes, HTTP gateway timeouts (504 Gateway Timeout), and service health probe failures.
- **What a proper fix looks like:** Wrap all CPU-bound hashing calls in `tokio::task::spawn_blocking(move || { ... })` within the password hasher adapter or application service.
- **Priority:** P0 (Immediate)
- **Blast Radius:** APPLICATION RUNTIME & LATENCY SLA

---

### HIGH SEVERITY FINDINGS

#### Finding H-01: In-Process `std::sync::Mutex` Locks Create Concurrency Bottlenecks and Prevent Horizontal Scaling
- **What is wrong:** `IdentityDatabase` (`crates/sitolo-persistence/src/memory.rs`), `TenancyService` (`crates/sitolo-application/src/tenancy.rs`), and `IdentityService` (`crates/sitolo-application/src/identity.rs`) wrap core state collections and rate limiters in standard `std::sync::Mutex`.
- **Why it is wrong:**
  1. Standard `std::sync::Mutex` locks across async contexts block OS threads rather than yielding tasks to Tokio.
  2. In-memory state is local to a single process instance; state is not shared across multiple API replicas.
- **Where it appears:** `crates/sitolo-persistence/src/memory.rs:88`, `crates/sitolo-application/src/identity.rs:62`, `crates/sitolo-application/src/tenancy.rs:35`
- **How it fails in production:** In a multi-instance container deployment (e.g., Kubernetes with 3 replicas), rate limiting and session states become inconsistent. A client rate-limited on Instance A can bypass rate limits by hitting Instance B. Lock contention under high concurrent requests causes thread blocking and API latency degrade.
- **What a proper fix looks like:** Replace in-memory `std::sync::Mutex` stores with PostgreSQL-backed persistent repositories utilizing transaction-level locks (`SELECT FOR UPDATE`), row-level security, and Redis for distributed rate-limiting.
- **Priority:** P1 (Short Term)
- **Blast Radius:** SCALABILITY & CONCURRENCY CONTROLS

#### Finding H-02: Absence of Implemented Business Domain Engines (Phase 8–17 Crate Scaffolds)
- **What is wrong:** Crates `sitolo-events`, `sitolo-integrations`, and `sitolo-sync` contain empty `lib.rs` files without executable code. Business modules for Catalogue, Inventory, Sales, Payments, and Fiscalization exist only as markdown specifications under `docs/`.
- **Why it is wrong:** The system cannot perform core business operating functions (creating products, deducting inventory, recording cash/M-Pesa sales, issuing fiscal invoices).
- **Where it appears:** `crates/sitolo-events/src/lib.rs`, `crates/sitolo-integrations/src/lib.rs`, `crates/sitolo-sync/src/lib.rs`, `crates/sitolo-domain/src/lib.rs`
- **How it fails in production:** System cannot process merchant operations. Attempting to call non-existent domain APIs results in 404 Not Found errors.
- **What a proper fix looks like:** Sequentially execute Phase 8 through Phase 17 implementation contracts, building domain entities, database migrations, repository persistence, and API handlers for each engine.
- **Priority:** P1 (Short Term)
- **Blast Radius:** CORE PRODUCT FUNCTIONALITY

#### Finding H-03: Missing HTTP Security Header and CORS/CSRF Protection Stack
- **What is wrong:** Custom TCP HTTP dispatcher in `apps/api/src/serve.rs` does not append HTTP security headers (`Strict-Transport-Security`, `Content-Security-Policy`, `X-Content-Type-Options`, `X-Frame-Options`) or validate Origin / CSRF headers.
- **Why it is wrong:** Web applications consuming the API from browsers are vulnerable to cross-site request forgery (CSRF), cross-site scripting (XSS) clickjacking, and MIME-sniffing exploits.
- **Where it appears:** `apps/api/src/serve.rs:103-120`
- **How it fails in production:** Malicious websites visited by logged-in merchant administrators can issue forged state-changing API requests (e.g., suspending branches or creating unauthorized users) via browser session credentials.
- **What a proper fix looks like:** Integrate `tower-http` middleware layers in Axum for CORS (`CorsLayer`), Security Headers (`SetResponseHeaderLayer`), and strict request body limits.
- **Priority:** P1 (Short Term)
- **Blast Radius:** WEB FRONTEND & API SECURITY BOUNDARY

---

### MEDIUM SEVERITY FINDINGS

#### Finding M-01: Persistence Test Suite Panics (`FAIL-CLOSED`) When PostgreSQL Environment Is Unreachable
- **What is wrong:** Running `./scripts/ci/verify` or `cargo test --workspace` fails when PostgreSQL is not running locally because `crates/sitolo-persistence/tests/rls_security_tests.rs` panics with `FAIL-CLOSED: Required ADMIN_DATABASE_URL or DATABASE_URL env variable not provided`.
- **Why it is wrong:** Test suites should gracefully skip database integration tests or spin up a test container if database environment variables are missing, allowing standard unit test suites to pass deterministically in standard CI environments.
- **Where it appears:** `crates/sitolo-persistence/tests/rls_security_tests.rs:304`
- **How it fails in production:** CI pipelines fail automatically on code verification unless a live PostgreSQL instance is configured and migrated prior to test execution.
- **What a proper fix looks like:** Guard PostgreSQL integration tests with conditional execution (e.g., `#[ignore = "requires PostgreSQL"]` or feature flags), or automate ephemeral test database provisioning via Testcontainers in CI scripts.
- **Priority:** P2 (Medium Term)
- **Blast Radius:** CI/CD PIPELINE & DEVELOPER EXPERIENCE

#### Finding M-02: Missing Distributed Telemetry & Prometheus Metric Exporters in API Binary
- **What is wrong:** `sitolo-observability` contains robust priority-aware in-memory ring buffers and trace context types, but `apps/api` does not expose a `/metrics` endpoint or integrate an OTLP/Prometheus exporter.
- **Why it is wrong:** Operational monitoring tools (Prometheus, Grafana, Datadog) cannot scrape runtime metrics (HTTP request rates, latency histograms, error rates, connection pool exhaustion).
- **Where it appears:** `apps/api/src/serve.rs`, `crates/sitolo-observability/src/lib.rs`
- **How it fails in production:** Production incidents (memory leaks, slow queries, database connection pool exhaustion) occur without alertability or visibility into system telemetry.
- **What a proper fix looks like:** Add `metrics-exporter-prometheus` or OpenTelemetry OTLP exporters to `apps/api` and expose a restricted `/metrics` endpoint for scraper ingestion.
- **Priority:** P2 (Medium Term)
- **Blast Radius:** OPERATIONAL OBSERVABILITY & INCIDENT RESPONSE

#### Finding M-03: Unused Code & Compilation Warnings in `sitolo-persistence`
- **What is wrong:** Compiling `sitolo-persistence` without full test feature flags triggers compiler warnings regarding unused types: `PgAuthorityError` and `PgAuthorityPools` methods (`connect_options`, `runtime_pool`, `verify_runtime_role`, `verify_effective_privileges`, `verify_rls_catalog_metadata`, `set_transaction_tenant_context`).
- **Why it is wrong:** Dead code and unused warnings indicate unlinked runtime paths or missing integration wiring.
- **Where it appears:** `crates/sitolo-persistence/src/postgres.rs:14`, `crates/sitolo-persistence/src/postgres.rs:43`
- **How it fails in production:** Unused authority verification methods mean PostgreSQL runtime safety checks are not actively run during production service startup.
- **What a proper fix looks like:** Wire `PgAuthorityPools::connect_options` and privilege verification checks into `apps/api/src/bootstrap.rs` during API startup.
- **Priority:** P2 (Medium Term)
- **Blast Radius:** MAINTAINABILITY & CODE QUALITY

---

### LOW SEVERITY FINDINGS

#### Finding L-01: Redundant Log Formatting and Escaping in Probes
- **What is wrong:** `apps/api/src/serve.rs` uses manual JSON escaping (`json_escape`) for string interpolation in `/process/live` probe responses instead of `serde_json::to_string`.
- **Why it is wrong:** Manual string construction is prone to formatting bugs if service identity strings contain unhandled control characters.
- **Where it appears:** `apps/api/src/serve.rs:125-131`, `apps/api/src/serve.rs:215-224`
- **How it fails in production:** Minor risk of invalid JSON formatting if service metadata contains special unicode characters.
- **What a proper fix looks like:** Derive `Serialize` on health response structs and use standard `serde_json`.
- **Priority:** P3 (Long Term)
- **Blast Radius:** API HEALTH RESPONSE FORMATTING

---

## 5. Top 10 Highest-Risk Issues

| Rank | Risk Title | Severity | Impact Summary |
|---|---|---|---|
| **1** | Custom TCP HTTP Parser in `apps/api` | **CRITICAL** | Request smuggling, body truncation, DoS vulnerabilities, lack of TLS. |
| **2** | Unimplemented Worker Binary (`apps/worker`) | **CRITICAL** | Total absence of asynchronous outbox and background job execution. |
| **3** | Async Reactor Thread Starvation via Argon2 | **CRITICAL** | CPU-bound hashing blocks Tokio async threads under concurrent logins. |
| **4** | In-Process Mutex Concurrency Bottlenecks | **HIGH** | Prevents horizontal scaling and causes thread contention under load. |
| **5** | Missing Business Domain Engines (Phases 8–17) | **HIGH** | Platform cannot perform sales, inventory, payment, or fiscal operations. |
| **6** | Absence of CORS, CSRF, & Security Headers | **HIGH** | Vulnerable to cross-site request forgery and browser-side exploits. |
| **7** | Inactive PostgreSQL Startup Authority Verification | **MEDIUM** | Runtime database role safety checks are never executed at startup. |
| **8** | Unexported Telemetry & Prometheus Metrics | **MEDIUM** | Zero operational visibility into request latencies or system metrics. |
| **9** | Persistence Test Suite Failures without Postgres | **MEDIUM** | CI pipeline brittleness due to unhandled database test dependency. |
| **10**| Lack of Automated DB Migration Rollbacks | **MEDIUM** | Risk of failed database deployments without automated rollback safety. |

---

## 6. Top 10 Highest-Leverage Fixes

| Rank | Leverage Fix Description | Target Component | Architectural Benefit |
|---|---|---|---|
| **1** | Migrate `apps/api` to Axum + Hyper + Tower | `apps/api/src/serve.rs` | Eliminates transport security risks, provides standard HTTP parsing, middleware, and request body limits. |
| **2** | Offload Password Hashing to `tokio::task::spawn_blocking` | `sitolo-application` | Restores async reactor throughput and prevents thread starvation under high login concurrency. |
| **3** | Implement Transactional Outbox Consumer in `apps/worker` | `apps/worker` & `sitolo-events` | Enables reliable, asynchronous processing of emails, webhooks, and background jobs. |
| **4** | Replace In-Memory Mutexes with PostgreSQL Persistence | `sitolo-persistence` | Enables multi-instance horizontal scaling, row locking, and persistent data durability. |
| **5** | Wire `PgAuthorityPools` Verification into API Bootstrap | `apps/api/src/bootstrap.rs` | Enforces least-privileged PostgreSQL runtime privileges and RLS metadata checks on startup. |
| **6** | Add `tower-http` Security Headers & CORS Middleware | `apps/api` | Secures browser API clients against CSRF, clickjacking, and cross-site scripting. |
| **7** | Implement Phase 8 (Catalogue) & Phase 9 (Inventory) | `sitolo-domain` | Unlocks core business engine functionality for merchants. |
| **8** | Integrate Prometheus Exporter in `sitolo-observability` | `apps/api` & `sitolo-observability` | Provides live operational metrics, dashboards, and incident alerting. |
| **9** | Add Ephemeral Postgres Provisioning for CI Tests | `scripts/ci/verify` | Ensures deterministic, reliable execution of persistence tests in CI. |
| **10**| Implement Redis Distributed Rate Limiting | `sitolo-application` | Replaces single-node rate limiters with cluster-wide abuse prevention. |

---

## 7. Phased Remediation Roadmap

```text
  [ PHASE 1: IMMEDIATE ]   -->  [ PHASE 2: SHORT-TERM ]  -->  [ PHASE 3: MEDIUM-TERM ]  -->  [ PHASE 4: LONG-TERM ]
  • Axum Transport             • Phase 8–10 Domain Engines    • Phase 11–15 Integrations   • Multi-Region DR
  • Spawn Blocking Hashing     • PostgreSQL Repositories      • Redis Distributed Cache    • Advanced Analytics
  • Worker Outbox Loop         • Security Header Middleware   • Prometheus/OTLP Exporters  • Automated Chaos Testing
```

### Phase 1: Immediate Remediation (0 – 2 Weeks)
- **Transport Security:** Refactor `apps/api/src/serve.rs` to replace custom TCP listener with standard Axum router and Tower middleware stack.
- **Async Safety:** Wrap Argon2/Bcrypt password hashing in `tokio::task::spawn_blocking`.
- **Worker Execution:** Implement basic outbox worker loop in `apps/worker` consuming `audit_outbox` records.
- **Startup Verification:** Call `PgAuthorityPools::verify_runtime_role` and `verify_rls_catalog_metadata` during `apps/api` startup in `bootstrap.rs`.

### Phase 2: Short-Term Remediation (2 – 6 Weeks)
- **Domain Engines:** Execute Phase 8 (Product Catalogue) and Phase 9 (Inventory Ledger) domain and persistence implementations.
- **Database Persistence:** Replace in-memory identity and tenancy stores with production PostgreSQL repositories behind `IdentityStores` and `TenancyStores` traits.
- **Browser Security:** Add `tower-http` CORS, CSRF, and security header middleware layers to `apps/api`.
- **CI Reliability:** Update `./scripts/ci/verify` to manage PostgreSQL test container lifecycle for `rls_security_tests.rs`.

### Phase 3: Medium-Term Remediation (6 – 12 Weeks)
- **Integrations:** Implement Phase 11 (Payments & Reconciliation) and Phase 15 (MRA EIS Fiscalization) in `sitolo-integrations`.
- **Distributed Caching:** Introduce Redis for distributed session caching and multi-node rate limiting.
- **Observability:** Integrate Prometheus `/metrics` scraping endpoint and OTLP distributed tracing in `sitolo-observability`.
- **Offline Synchronization:** Implement Phase 12 offline device sync protocol in `sitolo-sync`.

### Phase 4: Long-Term Enterprise Hardening (3 – 6 Months)
- **Production Certification:** Execute Phase 20 production certification, zero-downtime rolling deployment testing, and disaster recovery drills.
- **Automated Chaos Testing:** Implement fault-injection testing for database connection drops, network degradation, and worker crashes.

---

## 8. Enterprise Readiness Thresholds: Acceptable vs. Non-Enterprise

To maintain objective evaluation standards, the following table explicitly distinguishes features that meet enterprise production standards from those that do not:

| Capability / Surface | Current Status | Acceptable Enterprise Standard | Non-Enterprise / Unacceptable Gap |
|---|---|---|---|
| **Identity & Tokens** | **ACCEPTABLE** | Security-versioned JWTs, CSPRNG reset tokens, Argon2 cost-upgrades, MFA TOTP. | Hardcoded secrets, unhashed tokens, weak RNGs, non-expiring sessions. |
| **Tenant Scope Resolution**| **ACCEPTABLE** | Server-authoritative `AuthorizedScope` derivation; explicit requested vs. trusted separation. | Client-supplied tenant IDs without server validation, scope widening. |
| **Database Security** | **ACCEPTABLE** | Dual connection pools (`admin` vs `app_runtime`), mandatory RLS on tenant tables. | Single superuser DB pool, missing RLS, inline unparameterized SQL. |
| **HTTP Transport Layer** | **UNACCEPTABLE** | Axum / Hyper framework, strict body limits, CORS/CSRF headers, TLS termination. | Custom TCP read loops, raw buffer slicing, missing Content-Length checks. |
| **Async Processing** | **UNACCEPTABLE** | Durable outbox workers (`SKIP LOCKED`), background job retries, event publishing. | Empty worker scaffolds (`fn main() {}`), inline blocking external API calls. |
| **Thread Reactor Safety** | **UNACCEPTABLE** | CPU-bound crypto (Argon2) isolated on `spawn_blocking` thread pools. | CPU-heavy hashing running directly on Tokio reactor threads. |
| **Domain Completeness** | **UNACCEPTABLE** | Full business engines for Catalogue, Inventory, Sales, Payments, Fiscalization. | Empty crate scaffolds, missing core merchant business logic. |
| **Distributed Scaling** | **UNACCEPTABLE** | PostgreSQL persistent stores, distributed Redis rate limiting and caching. | In-process `std::sync::Mutex` state, single-instance memory databases. |

---
*Report compiled and certified for Sitolo Enterprise Platform Engineering.*
