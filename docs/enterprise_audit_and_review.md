# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-26
**Source baseline:** `main` at repository root
**Scope:** Full End-to-End Enterprise Architecture, Security, Reliability, Performance, Testing & Operational Risk
**Status:** Canonical Current-State Enterprise Audit
**Authority:** Governing documentation hierarchy in `agent.md` and current source tree

---

## Executive Summary

Sitolo is designed as an enterprise business operating system for African SMEs, featuring a Rust modular monolith architecture using Axum, Tokio, and PostgreSQL with Row-Level Security (RLS).

An end-to-end audit across architecture, security, reliability, scalability, performance, maintainability, observability, testing, deployment readiness, and operational risk reveals that while **the platform possesses strong foundational primitives in security, identity, tenancy, and authorization models**, it is **NOT READY FOR PRODUCTION DEPLOYMENT**.

### Overall Health Grade: Conditional Platform Foundation (Unfit for Production Traffic)

| Assessment Vector | Current Posture | Strategic Reality |
|---|---|---|
| **Architectural Model** | Strong Foundation | Rust workspace structure, domain separation, and zero-unsafe policies are strictly enforced. However, the API HTTP transport deviates from contract specs (custom TCP loop instead of Axum). |
| **Security & Identity** | Partial High-Assurance | Multi-tenant isolation models, effective-scope resolution, token structures, and RLS test harnesses are robust. Password hashing defaults to a test double, and API routes lack HTTP auth middleware. |
| **Business Operations** | Contract Phase / Scaffold | Only tenancy/IAM domain entities exist in code. Core business engines (Catalogue, Inventory, POS, Payments, Procurement, MRA EIS, Sync) remain unwritten or scaffolded. |
| **Data & Persistence** | Dual-Tier / Unfinished Schema | Reference in-memory store is non-durable across process restarts. PostgreSQL RLS authority primitives exist, but complete business migrations are phase-gated. |
| **Async & Background** | Scaffold | The background worker binary is empty (`fn main() {}`), and events/integrations/sync crates lack active runtimes. |
| **Observability & Ops** | Partial Substrate | Bounded telemetry and structured logging exist, but live distributed tracing exporters and Prometheus metrics endpoints are missing. |

---

## 1. System End-to-End Dimensional Review

### 1.1 Repository Structure and Dependency Boundaries
- **Status:** Acceptable Baseline
- **Analysis:** The repository enforces a clean Rust workspace layout (`apps/` vs `crates/`). Dependency direction flows strictly inward toward `sitolo-domain`. Circular dependencies are prohibited, and `clippy.toml` / `deny.toml` strictly forbid `unsafe` code and unvetted dependencies.
- **Risk:** High dependency coupling on `sitolo-persistence` in integration tests due to feature-gated `postgres` module exposing internal authority types.

### 1.2 Application Architecture and Module Separation
- **Status:** Partially Implemented
- **Analysis:** Domain orchestration follows clean architecture principles (`sitolo-application` coordinates `sitolo-domain` and `sitolo-tenancy`). However, because core business domain modules are absent in `sitolo-domain`, application services currently only orchestrate tenancy and IAM lifecycle operations.

### 1.3 API Design, Request Flow, and Trust Boundaries
- **Status:** CRITICAL GAP
- **Analysis:** Documented specifications mandate Rust + Axum + Tokio. The actual implementation in `apps/api/src/serve.rs` uses a manual TCP connection loop with raw string parsing for HTTP/1.1 requests. Requests are accepted up to 32KB without HTTP request smuggling defenses or HTTP/2-3 support. Furthermore, tenancy mutation endpoints (`/v1/organizations/*`) bypass HTTP authentication middleware entirely.

### 1.4 Authentication, Authorization, and Session Handling
- **Status:** High Risk
- **Analysis:** Cryptographic primitives for MFA, PKCE, JWT tokens, and session sliding windows in `sitolo-auth` are well-designed. However, `PasswordHasher` defaults to `TestPasswordHasher` (single-pass SHA-256 with no salt), and no production Argon2id adapter is wired in `sitolo-auth`. Authorization policy enforcement (`sitolo-authz`) is not connected to API endpoint handlers.

### 1.5 Input Validation, Sanitization, and Data Integrity
- **Status:** Moderate
- **Analysis:** Bounded JSON deserialization (`deny_unknown_fields`, length limits) exists on tenancy DTOs. However, path parameter validation in custom API handlers relies on basic string splitting (`split('/')`), which is fragile under malformed URI input.

### 1.6 Injection Risks, XSS, CSRF, SSRF, IDOR, Path Traversal & Secret Leakage
- **Status:** Moderate / High
- **Analysis:** SQL injection risk is low due to SQLx parameter binding and PostgreSQL RLS. IDOR risk is mitigated at the domain level by `AuthorizedScope`. However, lack of HTTP middleware authentication on API routes creates an extreme risk of unauthenticated IDOR and unauthorized tenant mutations. Secrets handling via `sitolo-security` uses `SecretValue` with `Zeroize`, but fallback environment secret providers expose local secret risks.

### 1.7 Error Handling, Retry Behavior, Timeout Strategy & Failure Isolation
- **Status:** Moderate
- **Analysis:** `AppError` maps to RFC 7807 `ProblemDetails` responses. Probe read timeouts (5s) are enforced in `serve.rs`. However, upstream integration retries, backoff algorithms, and circuit breakers do not exist because `sitolo-integrations` is empty.

### 1.8 CPU-Bound versus I/O-Bound Bottlenecks
- **Status:** High Risk
- **Analysis:** Argon2id password hashing and SHA-256 token signing are CPU-bound operations. If executed synchronously inside Tokio async tasks without `tokio::task::spawn_blocking`, heavy request volume will stall the Tokio worker thread pool, blocking I/O throughput.

### 1.9 Async Behavior, Blocking Operations, Race Conditions & Concurrency Hazards
- **Status:** Moderate
- **Analysis:** Tokio async usage is standard, but the reference persistence layer (`sitolo-persistence/src/memory.rs`) uses in-process `std::sync::RwLock` blocking mutexes across async boundaries. Under high concurrency, worker threads will suffer lock contention.

### 1.10 Database Design, Query Efficiency, Transactions, Migrations & Schema Evolution
- **Status:** High Risk / Unfinished
- **Analysis:** PostgreSQL RLS security models in `crates/sitolo-persistence/tests/rls_security_tests.rs` are sophisticated, enforcing tenant isolation via `app.organization_id` session GUCs. However, application database migrations, schema evolution tools, and production business table schemas are missing from the repository.

### 1.11 Caching Strategy, Invalidation Logic & Consistency Tradeoffs
- **Status:** Unimplemented
- **Analysis:** No distributed caching layer (e.g., Redis) is implemented. In-memory JWKS and JWK negative caches exist in `sitolo-auth`, but lack distributed invalidation mechanisms across API nodes.

### 1.12 Queueing, Background Jobs, Event Handling & Idempotency
- **Status:** CRITICAL GAP
- **Analysis:** `apps/worker` is `fn main() {}`. `sitolo-events` contains no outbox table scanner or event bus. Asynchronous background processing, retry loops, and Dead-Letter Queues (DLQ) are completely non-functional.

### 1.13 Observability: Logs, Metrics, Traces, Alertability & Incident Diagnosability
- **Status:** Moderate
- **Analysis:** `sitolo-observability` provides structured tracing and bounded event buffers. However, standard metric exporters (e.g., Prometheus `/metrics`) and OpenTelemetry OTLP trace exporters are not wired into the API runtime.

### 1.14 Test Coverage, Test Quality, Edge Cases & Regression Risk
- **Status:** Moderate
- **Analysis:** Unit test suites for auth, tenancy, config, and security are comprehensive. An offensive RLS test suite exists in `sitolo-persistence`. However, end-to-end HTTP integration tests against a live server and load/stress tests are absent.

### 1.15 Configuration Management, Environment Separation & Secrets Handling
- **Status:** Acceptable Baseline
- **Analysis:** Environment selectors and SHA-256 configuration fingerprints in `sitolo-config` prevent silent configuration drift. Secret references resolve via provider abstractions.

### 1.16 Deployment Safety, Rollback Readiness, Versioning & Release Discipline
- **Status:** Moderate
- **Analysis:** Release governance and toolchain pinning (Rust 1.98.1) are enforced via CI scripts. Deployment descriptors (Dockerfiles, Helm charts, Terraform) are absent from the repository.

### 1.17 Code Quality, Naming, Duplication, Coupling & Technical Debt
- **Status:** Acceptable
- **Analysis:** Code quality is high, formatted cleanly via `rustfmt`, and linted strictly with `clippy`. Technical debt is concentrated in stubbed crates and intermediate TCP transport code.

### 1.18 Maintainability under Team Growth, Code Ownership & Refactor Cost
- **Status:** Good
- **Analysis:** Strict crate separation and System Map documentation (`map/`) enable parallel engineering streams without context collision.

### 1.19 Compliance and Enterprise Operational Expectations
- **Status:** CRITICAL GAP
- **Analysis:** MRA EIS regulatory fiscalization requirements (Malawi Revenue Authority Tax Invoicing) and payment provider reconciliation contracts are specified in documentation but entirely unwritten in source code.

---

## 2. Severity-Ranked Findings List

### 2.1 Critical Severity Findings (P0)

#### FINDING-CRIT-01: Manual TCP HTTP/1.1 Parser with Missing Authentication & Route Authorization
- **What is wrong:** The API service in `apps/api/src/serve.rs` implements a custom TCP connection handler using `tokio::net::TcpListener` and string splitting (`headers_part.lines()`, `split('/')`) to parse HTTP requests instead of using an enterprise web framework like Axum. Crucially, incoming request handlers for `/v1/organizations/*` do not validate authentication headers, JWT tokens, or tenant authorization scopes.
- **Why it is wrong:** Manual HTTP parsers are notoriously vulnerable to HTTP request smuggling, header injection, buffer overflows, and malformed chunked transfer attacks. Omitting auth middleware leaves multi-tenant organization creation, suspension, and deletion completely unprotected.
- **Where it appears:** `apps/api/src/serve.rs` (lines 48–230), `apps/api/src/dispatch_request`
- **How it fails in production:** An unauthenticated attacker sends a HTTP POST request to `/v1/organizations/{org_id}/suspend` over raw TCP, causing immediate denial-of-service by suspending arbitrary tenant organizations without credentials. Malformed HTTP requests crash the connection task or lead to request smuggling.
- **What a proper fix looks like:** Replace custom TCP parsing with Axum (`axum::Router`). Implement standard Axum tower middleware layers for TLS termination, trace context extraction, rate limiting, JWT session authentication (`AuthLayer`), and tenancy scope authorization (`AuthorizeScopeLayer`).
- **Priority and Blast Radius:** P0 (Critical) | Blast Radius: Whole API Service & Multi-Tenant Security Boundary.

---

#### FINDING-CRIT-02: Missing Production Password Hasher & Test-Only SHA256 Fallback
- **What is wrong:** `crates/sitolo-auth/src/password.rs` only contains `TestPasswordHasher`, which uses single-pass SHA-256 hashing without salt (`testv{version}${hex}`). Production Argon2id password hashing is delegated to an unwritten external adapter.
- **Why it is wrong:** Single-pass SHA-256 without salt is trivial to break using rainbow tables or GPU cracking arrays. Storing user passwords using this test double exposes all tenant user credentials to instant compromise in the event of a database breach.
- **Where it appears:** `crates/sitolo-auth/src/password.rs` (lines 80–135)
- **How it fails in production:** If deployed as-is, user passwords created during registration are stored as weak SHA-256 digests. Attackers who obtain database read access can instantly reverse millions of user passwords per second.
- **What a proper fix looks like:** Implement `Argon2idPasswordHasher` inside `sitolo-auth` using the `argon2` crate with parameter bounds conforming to OWASP specifications (m=19456 KiB, t=2, p=1). Ensure hashing operations run inside `tokio::task::spawn_blocking`.
- **Priority and Blast Radius:** P0 (Critical) | Blast Radius: All User Authentication & Password Credential Security.

---

#### FINDING-CRIT-03: Completely Unimplemented Worker Binary & Background Execution
- **What is wrong:** The background worker entry point in `apps/worker/src/main.rs` consists of an empty `fn main() {}`.
- **Why it is wrong:** Asynchronous background processing—including transactional outbox dispatch, audit event forwarding, payment webhook processing, MRA EIS fiscal invoice submission, and email invitation processing—cannot execute.
- **Where it appears:** `apps/worker/src/main.rs` (lines 1–11)
- **How it fails in production:** Background jobs scheduled by the API or domain events remain stuck indefinitely in queues or database tables. Operations expecting asynchronous processing silently stall or fail.
- **What a proper fix looks like:** Implement a resilient worker process loop in `apps/worker` using Tokio. Include signal handling, graceful shutdown, database connection pooling, transactional outbox polling with exponential backoff, and Dead-Letter Queue (DLQ) error isolation.
- **Priority and Blast Radius:** P0 (Critical) | Blast Radius: Entire Asynchronous System, Outbox Pipeline & Integrations.

---

#### FINDING-CRIT-04: Non-Durable In-Memory Persistence as Default Engine
- **What is wrong:** The primary active repositories in `sitolo-persistence` (e.g. `TenancyDatabase`, `IdentityDatabase`) default to in-memory hash maps (`std::collections::HashMap` wrapped in `RwLock`).
- **Why it is wrong:** In-memory storage is volatile. Any container restart, deployment, or process crash completely erases all tenant state, registered organizations, active sessions, and MFA enrollments.
- **Where it appears:** `crates/sitolo-persistence/src/memory.rs` (lines 1–350), `crates/sitolo-persistence/src/lib.rs`
- **How it fails in production:** Upon process restart or pod reschedule, the application loses all data, forcing users to re-register and leaving the system in an unrecoverable state.
- **What a proper fix looks like:** Wire `sitolo-persistence` to PostgreSQL 18 as the default production storage engine. Ensure all entity repositories (`OrganizationRepository`, `MembershipRepository`, `SessionRepository`) execute SQL queries within PostgreSQL transactions utilizing Row-Level Security (RLS).
- **Priority and Blast Radius:** P0 (Critical) | Blast Radius: Data Durability & System State Retention.

---

#### FINDING-CRIT-05: Missing Core Business Engine Implementations in Domain Monolith
- **What is wrong:** `crates/sitolo-domain/src/lib.rs` only exports `pub mod tenancy;`. Core operational SME business modules—Product Catalogue, Inventory Ledger, POS Sales, Procurement, Returns/Refunds, MRA EIS Integration, Reporting, and Billing—are entirely absent from code.
- **Why it is wrong:** The system cannot perform any retail, wholesale, inventory, tax fiscalization, or financial transaction processing. It is currently a tenancy/IAM administrative shell rather than a business operating system.
- **Where it appears:** `crates/sitolo-domain/src/lib.rs` (lines 1–12)
- **How it fails in production:** API calls to perform core SME operations return 404 Not Found or cannot be compiled because domain aggregates do not exist.
- **What a proper fix looks like:** Implement domain aggregates, state machines, and business rules sequentially across `sitolo-domain` for Product Catalogue (Phase 8), Inventory Ledger (Phase 9), POS Sales (Phase 10), Payments & Reconciliation (Phase 11), Procurement (Phase 13), and MRA EIS (Phase 15).
- **Priority and Blast Radius:** P0 (Critical) | Blast Radius: Core Business Functionality.

---

### 2.2 High Severity Findings (P1)

#### FINDING-HIGH-01: Unimplemented Event Bus & Asynchronous Outbox Pipeline
- **What is wrong:** `crates/sitolo-events` is an empty crate (`//! Events.`). No event publisher, subscriber, or transactional outbox table worker exists.
- **Why it is wrong:** Domain events emitted by business aggregates cannot be published or processed asynchronously, breaking side-effect isolation and domain event consistency.
- **Where it appears:** `crates/sitolo-events/src/lib.rs`
- **How it fails in production:** Domain events are silently dropped or never produced, leaving downstream subsystems (audit logging, analytics, notification services) permanently out of sync.
- **What a proper fix looks like:** Build typed domain event structures in `sitolo-events`. Implement a transactional outbox table pattern in PostgreSQL (`outbox_events`), and a transactional publisher/consumer worker loop in `apps/worker`.
- **Priority and Blast Radius:** P1 (High) | Blast Radius: Event-Driven Architecture & Downstream Workflows.

---

#### FINDING-HIGH-02: Empty Integrations Boundary for MRA EIS & Payment Gateways
- **What is wrong:** `crates/sitolo-integrations` is an empty crate scaffold without active HTTP/REST client integration runtimes for external providers (MRA EIS tax portal, Airtel Money, TNM Mpamba, Bank APIs).
- **Why it is wrong:** African SME compliance and payment processing strictly require live tax authority communication (MRA EIS fiscal signature verification) and mobile money gateway integrations.
- **Where it appears:** `crates/sitolo-integrations/src/lib.rs`
- **How it fails in production:** Sales transactions cannot be fiscalized, violating legal tax compliance in Malawi. Mobile money payment checkouts fail immediately due to missing integration adapters.
- **What a proper fix looks like:** Implement provider-specific integration modules in `sitolo-integrations` using `reqwest` with mTLS, request signing, idempotency header propagation, and strict response deserialization.
- **Priority and Blast Radius:** P1 (High) | Blast Radius: External Payment Checkouts & Legal Tax Compliance.

---

#### FINDING-HIGH-03: Missing Offline Synchronization Engine & Conflict Resolution
- **What is wrong:** `crates/sitolo-sync` is an empty crate scaffold without offline sync protocols or conflict-resolution algorithms (CRDTs or vector clocks).
- **Why it is wrong:** African SME environments frequently suffer network latency and connectivity outages. POS terminals must operate offline and sync state reliably when reconnected.
- **Where it appears:** `crates/sitolo-sync/src/lib.rs`
- **How it fails in production:** Offline sales recorded on POS devices cannot sync back to the cloud server, or overwrite existing records uncontrollably during reconnection due to lack of conflict resolution.
- **What a proper fix looks like:** Implement a domain-aware synchronization protocol in `sitolo-sync` handling versioned transaction delta pushes, server reconciliation, and deterministic conflict resolution.
- **Priority and Blast Radius:** P1 (High) | Blast Radius: POS Reliability in Low-Connectivity Regions.

---

#### FINDING-HIGH-04: Lack of Distributed Tracing & Metric Exporters
- **What is wrong:** `crates/sitolo-observability` contains in-memory log buffer logic, but lacks OpenTelemetry OTLP trace exporters and a Prometheus `/metrics` scraper endpoint in the API process.
- **Why it is wrong:** Operators cannot visualize request flow spans across HTTP handlers, application services, and database queries. Real-time metric alerts (latency spikes, 5xx error rates, pool exhaustion) cannot be collected by Prometheus/Grafana.
- **Where it appears:** `crates/sitolo-observability/src/lib.rs`, `apps/api/src/serve.rs`
- **How it fails in production:** Incidents in production cannot be effectively diagnosed or alerted on, leading to increased Mean Time To Resolution (MTTR) and unseen service degradation.
- **What a proper fix looks like:** Integrate `tracing-opentelemetry` and `metrics-exporter-prometheus` in `sitolo-observability`. Expose a protected `/metrics` HTTP endpoint on the API binary.
- **Priority and Blast Radius:** P1 (High) | Blast Radius: System Diagnostics, Alerting & Incident Response.

---

#### FINDING-HIGH-05: CPU-Intensive Cryptographic Hashing Blocking Async Tokio Runtime Threads
- **What is wrong:** Cryptographic operations (Argon2id hashing, RSA/ECDSA key generation) executed during authentication requests are synchronous CPU-bound tasks. Currently, no `tokio::task::spawn_blocking` boundary wraps password verification.
- **Why it is wrong:** Tokio worker threads are cooperative. Running CPU-intensive KDF loops directly on a Tokio worker thread starves other async tasks running on that thread, causing severe latency spikes for concurrent I/O operations.
- **Where it appears:** `crates/sitolo-auth/src/password.rs`, `crates/sitolo-auth/src/session.rs`
- **How it fails in production:** Under high login throughput (e.g. shift start at retail branches), CPU-heavy hashing blocks the Tokio worker threads, causing HTTP request timeouts and 504 gateway errors for all incoming API traffic.
- **What a proper fix looks like:** Offload all CPU-bound password hashing, verification, and key creation tasks to Tokio's dedicated blocking thread pool via `tokio::task::spawn_blocking`.
- **Priority and Blast Radius:** P1 (High) | Blast Radius: API Latency & Tokio Runtime Thread Pool.

---

### 2.3 Medium Severity Findings (P2)

#### FINDING-MED-01: In-Process RWLock Locks as Concurrency Primitive in Reference Persistence
- **What is wrong:** `crates/sitolo-persistence/src/memory.rs` uses `std::sync::RwLock` to synchronize access to in-memory collections across async functions.
- **Why it is wrong:** Standard library `RwLock` guards held across async `.await` points cause deadlocks or block Tokio worker threads. Even when held briefly, global write locks create serialization bottlenecks under concurrent write operations.
- **Where it appears:** `crates/sitolo-persistence/src/memory.rs` (lines 45–180)
- **How it fails in production:** High-frequency write requests block the entire memory repository, degrading throughput and causing thread starvation.
- **What a proper fix looks like:** Use database-level row locks (`SELECT ... FOR UPDATE`) in PostgreSQL transactions rather than in-process locks.
- **Priority and Blast Radius:** P2 (Medium) | Blast Radius: Persistence Concurrency & Write Throughput.

---

#### FINDING-MED-02: Absence of Distributed Rate Limiting & Per-Principal Throttling
- **What is wrong:** `crates/sitolo-auth/src/ratelimit.rs` provides in-memory rate limiting, but no distributed rate limiter (e.g. Redis sliding window) is wired into the HTTP request pipeline in `apps/api/src/serve.rs`.
- **Why it is wrong:** Rate limiting enforced in process memory fails when API nodes are scaled horizontally behind a load balancer, allowing attackers to bypass rate limits by distributing requests across multiple nodes.
- **Where it appears:** `apps/api/src/serve.rs`, `crates/sitolo-auth/src/ratelimit.rs`
- **How it fails in production:** Brute-force credential stuffing or API scraping attacks succeed by targeting multiple IP addresses and API instances.
- **What a proper fix looks like:** Implement an Axum middleware layer backed by a Redis sliding window algorithm to enforce per-IP and per-principal rate limits globally.
- **Priority and Blast Radius:** P2 (Medium) | Blast Radius: API Protection against Denial-of-Service & Brute Force.

---

#### FINDING-MED-03: Missing Database Migration Versioning & Schema Evolution Harness
- **What is wrong:** The database setup code in `crates/sitolo-persistence/tests/rls_security_tests.rs` loads raw SQL scripts using `include_str!("fixtures/rls_schema.sql")` rather than using a version-controlled migration tool (e.g., `sqlx-cli` or `refinery`).
- **Why it is wrong:** Production schema evolution requires deterministic, reversible, versioned migration steps with execution history tracked in a `_sqlx_migrations` schema table.
- **Where it appears:** `crates/sitolo-persistence/tests/fixtures/rls_schema.sql`, `crates/sitolo-persistence/src/postgres.rs`
- **How it fails in production:** Schema changes during deployments risk database corruption, drift between environments, or failed zero-downtime updates.
- **What a proper fix looks like:** Structure database schemas into versioned SQL migration files under `migrations/` and run `sqlx::migrate!()` automatically during application startup.
- **Priority and Blast Radius:** P2 (Medium) | Blast Radius: Database Schema Evolution & Deployment Safety.

---

#### FINDING-MED-04: Lack of Centralized Application Health Checks for Downstream Dependencies
- **What is wrong:** The readiness probe in `apps/api/src/serve.rs` (`GET /process/ready`) returns `{"status":"ready"}` statically without verifying active connectivity to PostgreSQL or external services.
- **Why it is wrong:** Kubernetes or load balancers will route traffic to API pods that have lost database connectivity, causing 500 errors for end users.
- **Where it appears:** `apps/api/src/serve.rs` (lines 110–120)
- **How it fails in production:** Traffic is routed to unhealthy instances, causing cascading failure during database restarts or network disruptions.
- **What a proper fix looks like:** Implement deep readiness checks that perform `SELECT 1` queries on PostgreSQL connection pools before returning HTTP 200 OK.
- **Priority and Blast Radius:** P2 (Medium) | Blast Radius: Traffic Routing & High Availability.

---

#### FINDING-MED-05: Configuration Fingerprint & Environment Secret Overrides
- **What is wrong:** `crates/sitolo-security/src/provider.rs` allows falling back to local environment variable secret resolution (`SITOLO__DATABASE__PASSWORD`) in development mode without strict production compile-time gating.
- **Why it is wrong:** Developers might accidentally deploy binaries with local secret resolution enabled, exposing credentials in process environment variables (`/proc/self/environ`).
- **Where it appears:** `crates/sitolo-security/src/provider.rs`, `crates/sitolo-config/src/lib.rs`
- **How it fails in production:** Secret leakage occurs via server inspection, core dumps, or process logs.
- **What a proper fix looks like:** Enforce strict compile-time `#![cfg(not(feature = "production"))]` flags on local secret providers and require cloud secret manager integration (AWS Secrets Manager, HashiCorp Vault) in production builds.
- **Priority and Blast Radius:** P2 (Medium) | Blast Radius: Secrets Security & Environment Separation.

---

### 2.4 Low Severity Findings (P3)

#### FINDING-LOW-01: Unused Warnings and Dead Code in Non-Test Persistence Builds
- **What is wrong:** Non-test compilations of `sitolo-persistence` trigger compiler warnings for unused functions and types (`PgAuthorityError`, `set_transaction_tenant_context`).
- **Why it is wrong:** Warnings clutter build outputs and hide real compilation issues.
- **Where it appears:** `crates/sitolo-persistence/src/postgres.rs`
- **How it fails in production:** Build output clutter reduces developer efficiency.
- **What a proper fix looks like:** Clean up `pub(crate)` visibility and conditional compilation attributes.
- **Priority and Blast Radius:** P3 (Low) | Blast Radius: Developer Experience & Code Cleanliness.

---

#### FINDING-LOW-02: Hardcoded Timeouts in TCP Probe Handling
- **What is wrong:** Connection read timeouts in `apps/api/src/serve.rs` are hardcoded as `PROBE_READ_TIMEOUT_SECS = 5`.
- **Why it is wrong:** Hardcoded timeouts cannot be tuned by operators for high-latency mobile networks in rural Africa.
- **Where it appears:** `apps/api/src/serve.rs` (line 18)
- **How it fails in production:** Slow 2G/3G client connections are aborted prematurely.
- **What a proper fix looks like:** Move socket timeout parameters into external `AppConfig` structures.
- **Priority and Blast Radius:** P3 (Low) | Blast Radius: Tail Latency Tolerances.

---

#### FINDING-LOW-03: Contract-to-Code Documentation Synchronization Drift
- **What is wrong:** Several architecture and phase contract documents describe features (such as Axum router integration and domain aggregates) as actively existing when they are currently contract specifications.
- **Why it is wrong:** AI agents and human engineers can be misled regarding current codebase capabilities.
- **Where it appears:** `docs/phase8_product_catalogue_implementation.md`, `docs/system_architecture_design.md`
- **How it fails in production:** Engineering planning errors due to misinformed assumptions about feature readiness.
- **What a proper fix looks like:** Add explicit `Status: Specification / Contract` banners to phase documentation that has not yet been implemented in code.
- **Priority and Blast Radius:** P3 (Low) | Blast Radius: System Documentation Integrity.

---

## 3. Top 10 Highest-Risk Issues

| Rank | Finding ID | Title | Business & Engineering Impact | Blast Radius |
|---|---|---|---|---|
| **1** | FINDING-CRIT-01 | Manual TCP HTTP Parser & Unauthenticated API Routes | Unauthenticated multi-tenant org manipulation, DoS, request smuggling | Multi-Tenant Security & API Layer |
| **2** | FINDING-CRIT-02 | Test-Only SHA-256 Password Hasher | Single-round un-salted password hashes vulnerable to instant GPU cracking | All User Accounts & Passwords |
| **3** | FINDING-CRIT-04 | In-Memory Persistence Engine | Total loss of state and business data on process restart | Complete Data Retention & Durability |
| **4** | FINDING-CRIT-03 | Unimplemented Background Worker Binary | Total failure of asynchronous background execution and transactional outbox | Outbox, Jobs & Event Processing |
| **5** | FINDING-CRIT-05 | Missing Domain Engines (Catalogue, Inventory, POS) | Core SME operational features do not exist in code | Core Business OS Capability |
| **6** | FINDING-HIGH-02 | Empty MRA EIS & Payment Integrations | Inability to issue tax fiscal receipts or accept mobile money checkouts | Legal Tax Compliance & Revenue |
| **7** | FINDING-HIGH-01 | Unwritten Event Bus & Outbox Worker | Side-effect isolation failure, broken integration pipeline | Event-Driven System Architecture |
| **8** | FINDING-HIGH-05 | Synchronous KDF Hashing Blocking Tokio Runtime | High CPU load during auth causes global Tokio thread starvation and timeouts | API System Throughput |
| **9** | FINDING-HIGH-03 | Missing Offline Sync Engine | POS terminals fail or corrupt data when operating in low-connectivity areas | Retail POS Operations |
| **10** | FINDING-MED-02 | Lack of Distributed Rate Limiting | Vulnerability to distributed brute-force attacks and API abuse | Perimeter Defense |

---

## 4. Top 10 Highest-Leverage Fixes

| Rank | Target Area | Recommended Action | Architectural Impact | Effort |
|---|---|---|---|---|
| **1** | API Transport | Migrate `apps/api` to Axum framework with standard authentication and tenant scope middleware layers | Restores robust HTTP parsing, routing, and multi-tenant security enforcement | Low |
| **2** | Authentication | Implement `Argon2idPasswordHasher` inside `sitolo-auth` wrapped in `tokio::task::spawn_blocking` | Upgrades credential security to OWASP standards and prevents Tokio thread starvation | Low |
| **3** | Persistence | Wire `sitolo-persistence` repositories directly to PostgreSQL 18 with RLS transaction boundaries | Ensures full data durability and production-grade tenant isolation | Medium |
| **4** | Background Worker | Implement background worker loop in `apps/worker` with outbox table scanner and backoff logic | Enables resilient asynchronous processing and event side effects | Medium |
| **5** | Domain Engines | Implement domain aggregates in `sitolo-domain` for Product Catalogue (Phase 8), Inventory (Phase 9), POS (Phase 10) | Unlocks actual business operating system capabilities | High |
| **6** | External Integrations | Build MRA EIS and Mobile Money HTTP client adapters in `sitolo-integrations` | Delivers mandatory tax compliance and payment checkouts | Medium |
| **7** | Database Evolution | Add versioned SQL migrations (`migrations/`) and `sqlx::migrate!()` startup execution | Guarantees deterministic, safe schema deployment and rollback readiness | Low |
| **8** | Observability | Integrate Prometheus metrics and OpenTelemetry trace exporters into `sitolo-observability` | Provides real-time diagnosability, latency metrics, and automated alerting | Low |
| **9** | Offline Sync | Implement domain-aware delta sync protocol and CRDT conflict resolution in `sitolo-sync` | Ensures resilient offline POS terminal operation in low-connectivity regions | High |
| **10** | Health Checks | Implement deep PostgreSQL connection pool readiness checks in API probes | Prevents routing traffic to broken API pods during outages | Low |

---

## 5. Phased Remediation Plan

```text
IMMEDIATE (Days 0–14)   ──►   SHORT TERM (Days 15–45)   ──►   MEDIUM TERM (Days 46–90)   ──►   LONG TERM (Days 91–180)
• Axum API Migration           • Background Worker Loop         • Event Outbox Pipeline          • Offline Sync Engine
• Auth Middleware Wiring       • PostgreSQL Business Schema     • MRA EIS Integration            • Multi-Region HA / DR
• Production Argon2id          • Catalogue & Inventory Modules  • Mobile Money Payments          • External SOC2/PCI Audit
```

### Phase 1: Immediate Remediation (Days 0–14) — Perimeter & Security Hardening
- **Objective:** Eliminate critical security vulnerabilities and complete basic API transport.
- **Deliverables:**
  1. Replace custom TCP parser in `apps/api` with Axum framework.
  2. Implement Axum tower middleware for session authentication (`AuthLayer`) and tenant scope verification (`AuthorizeScopeLayer`).
  3. Implement production `Argon2idPasswordHasher` using the `argon2` crate and offload hashing to `tokio::task::spawn_blocking`.
  4. Fix dead code warnings and clean up `sitolo-persistence` module visibility.

### Phase 2: Short-Term Hardening (Days 15–45) — Data Durability & Core Domain
- **Objective:** Establish persistent storage and build core SME operational capabilities.
- **Deliverables:**
  1. Replace in-memory repositories with PostgreSQL 18 RLS implementation in `sitolo-persistence`.
  2. Create versioned SQL schema migrations (`migrations/`) for organizations, branches, users, products, and inventory.
  3. Implement domain aggregates in `sitolo-domain` for Product Catalogue (Phase 8) and Inventory Ledger (Phase 9).
  4. Build active worker process loop in `apps/worker` to process background jobs.

### Phase 3: Medium-Term Expansion (Days 46–90) — Integrations & Operations
- **Objective:** Enable asynchronous events, tax compliance, and payment checkouts.
- **Deliverables:**
  1. Implement transactional outbox event publisher in `sitolo-events` and worker consumer.
  2. Implement MRA EIS fiscal invoice client adapter in `sitolo-integrations` (Phase 15).
  3. Implement Airtel Money and TNM Mpamba mobile payment adapters in `sitolo-integrations` (Phase 11).
  4. Wire Prometheus `/metrics` endpoint and OpenTelemetry tracing into `apps/api`.

### Phase 4: Long-Term Enterprise Certification (Days 91–180) — High Availability & Offline POS
- **Objective:** Achieve production certification, offline reliability, and external compliance.
- **Deliverables:**
  1. Implement offline synchronization protocol and conflict resolution engine in `sitolo-sync` (Phase 12).
  2. Deploy multi-region PostgreSQL high-availability replication and automated disaster recovery failover.
  3. Conduct external SOC2 Type II, PCI-DSS, and penetration testing certification.

---

## 6. Enterprise Acceptance Standard Matrix

| System Aspect | Current State Baseline | Unacceptable for Enterprise Production | Acceptable Enterprise Standard |
|---|---|---|---|
| **API Transport** | Custom TCP string parser in `serve.rs` | Manual TCP string parsing, unauthenticated mutation routes | Axum framework with TLS termination, trace context, auth middleware |
| **Password Security** | Test double `TestPasswordHasher` (SHA-256) | Single-round un-salted hashes, synchronous CPU blocking in Tokio | Argon2id KDF with OWASP parameters, run inside `spawn_blocking` |
| **Data Storage** | In-memory hash maps in `memory.rs` | Volatile storage wiped on process restart | PostgreSQL 18 with Row-Level Security (RLS) & transactional ACID bounds |
| **Background Processing**| `fn main() {}` scaffold in `apps/worker` | Empty main function, stuck async side effects | Resilient worker loop, transactional outbox polling, backoff & DLQ |
| **Domain Monolith** | Tenancy module only | Missing core retail/SME domain engines | Sequential implementation of Catalogue, Inventory, POS, and Billing |
| **External Integrations** | Empty `sitolo-integrations` crate | Unhandled payment checkouts or missing MRA tax fiscalization | Resilient HTTP integration adapters with mTLS, retries, and circuit breakers |
| **Observability** | In-memory log buffer in `sitolo-observability` | No live metric scraping or distributed trace context | Prometheus `/metrics` exporter & OpenTelemetry OTLP tracing |
| **Database Migrations** | Static SQL fixture strings in test harness | Manual schema edits, unversioned database state | Versioned SQL migrations (`migrations/`) managed via `sqlx::migrate!()` |

---

## 7. Conclusion

Sitolo possesses a **well-architected modular blueprint and high-quality Rust code foundation**. The foundational design around tenancy isolation, effective scope derivation, and PostgreSQL Row-Level Security demonstrates strong engineering discipline.

However, **it is currently incomplete and cannot be deployed to production**. Resolving the findings in this report—specifically migrating the API transport to Axum, replacing test password hashing with Argon2id, wiring PostgreSQL persistence, implementing the background worker, and completing the core domain modules—is mandatory before processing real business transactions or real customer data.
