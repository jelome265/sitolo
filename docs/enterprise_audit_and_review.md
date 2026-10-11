# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main` at `e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc`
**Status:** Canonical Enterprise-Grade Audit & Technical Architecture Review
**Authority:** Current repository source tree, Cargo workspace manifests, and governing documentation in `../AGENTS.md` and `docs/`

---

## Executive Summary

Sitolo is currently **NOT PRODUCTION-READY**.

While the repository demonstrates high technical sophistication in foundational domain design (tenancy isolation contracts, request typing, zero-unsafe Rust workspace rules, Argon2id password hashing, and structured telemetry models), it currently operates as a **partially connected scaffold**.

### Overall Codebase Health Matrix

| Dimension | Grade | Status Summary |
| :--- | :---: | :--- |
| **Architecture & Boundaries** | **C+** | Strong workspace structure & boundary enforcement, but direct custom TCP transport in `apps/api` contradicts Axum contracts. |
| **Security & Authorization** | **D** | Security primitives exist, but `apps/api` endpoints bypass authentication & authorization entirely. |
| **Reliability & Async** | **D-** | Worker process is an unexecuted scaffold; CPU-bound Argon2 hashing runs on async worker threads. |
| **Database & Persistence** | **C-** | PostgreSQL RLS and dual-pool authority code exist in `sitolo-persistence`, but `apps/api` runs on ephemeral in-memory state. |
| **Domain Completeness** | **F** | Core business engines (sales, inventory, POS, MRA EIS tax integration, payments, reconciliation) exist only as documentation contracts. |
| **Observability & Testing** | **B-** | Good unit/contract testing and structured tracing buffers; missing OTLP exporters, distributed tracing, and live metrics endpoints. |
| **Deployment Readiness** | **F** | No production secret provider adapter; no worker runtime; no container/K8s release assets. |

---

## 1. End-to-End System Architecture Review

### 1.1 Repository Structure and Dependency Boundaries
- **Current State:** The workspace is cleanly split into `apps/` (`api`, `worker`) and `crates/` (`sitolo-api`, `sitolo-application`, `sitolo-audit`, `sitolo-auth`, `sitolo-authz`, `sitolo-config`, `sitolo-domain`, `sitolo-events`, `sitolo-integrations`, `sitolo-observability`, `sitolo-persistence`, `sitolo-security`, `sitolo-sync`, `sitolo-tenancy`, `sitolo-testkit`). Workspace `Cargo.toml` enforces `forbidden` `unsafe` code across all crates.
- **Weakness:** Circular domain dependency risk is mitigated by layering, but `crates/sitolo-events`, `crates/sitolo-integrations`, and `crates/sitolo-sync` are empty crate stubs containing only module declarations without implementation logic.

### 1.2 Application Architecture and Module Separation
- **Current State:** Domain entities (`sitolo-domain`), application orchestration (`sitolo-application`), and transport adapters (`sitolo-api`) are formally separated.
- **Weakness:** `apps/api` bypasses the standard layered application pipeline for HTTP execution, coupling direct TCP reading to crude string matching and manual JSON parsing.

### 1.3 API Design, Request Flow, and Trust Boundaries
- **Current State:** Transport DTOs in `sitolo-api` enforce strict bounded JSON decoding (`MAX_TENANCY_BODY_BYTES`) and `deny_unknown_fields`.
- **Weakness:** HTTP request handling in `apps/api/src/serve.rs` parses HTTP/1.1 headers manually over raw `TcpStream`. It lacks HTTP/2, TLS termination, request header extraction (e.g. `Authorization`, `X-Correlation-ID`), chunked encoding support, and proper HTTP pipeline handling.

### 1.4 Authentication, Authorization, and Session Handling
- **Current State:** `sitolo-auth` provides robust session tokens, PKCE, MFA, and Argon2id password hashing. `sitolo-authz` provides role-based and scope-based permissions.
- **Weakness:** The API transport in `apps/api/src/serve.rs` does **not** invoke authentication or authorization handlers. All HTTP endpoints (including organization lifecycle mutations like `/v1/organizations/{id}/suspend`) are completely unauthenticated and unvalidated.

### 1.5 Input Validation, Sanitization, and Data Integrity
- **Current State:** Strongly typed domain wrappers (e.g. `OrganizationId`, `BranchId`) restrict untyped strings.
- **Weakness:** The raw TCP parser in `apps/api/src/serve.rs` truncates HTTP request bodies to the first 32KB read buffer (`PROBE_MAX_BYTES`). Any multi-packet TCP payload is cut off midway, producing invalid JSON or truncated input errors.

### 1.6 Injection Risks, XSS, CSRF, SSRF, IDOR, Path Traversal, and Secrets Leakage
- **Current State:** Parametrized SQL queries are enforced via `sqlx`. Secrets are held in `SecretString` wrappers and dropped from memory after initialization.
- **Weakness:** Lack of authentication on `apps/api` endpoints creates a catastrophic IDOR vulnerability: any unauthenticated attacker can manipulate any organization or branch ID passed in URL path parameters.

### 1.7 Error Handling, Retry Behavior, Timeout Strategy, and Failure Isolation
- **Current State:** `AppError` maps domain errors to RFC 7807 `ProblemDetails` structures.
- **Weakness:** Background processing is absent (`apps/worker/src/main.rs` is empty). Failure retries and transactional outbox pattern delivery for external systems (e.g. MRA EIS, payment gateways) do not exist.

### 1.8 CPU-Bound versus I/O-Bound Bottlenecks
- **Current State:** Async reactor pattern using Tokio is configured in `apps/api`.
- **Weakness:** Argon2id password hashing inside `sitolo-auth/src/password.rs` is a synchronous CPU-bound operation executed directly inside Tokio async tasks without `tokio::task::spawn_blocking`. Under load, password hashing will saturate Tokio worker threads and cause server-wide event loop starvation.

### 1.9 Async Behavior, Blocking Operations, Race Conditions, and Concurrency Hazards
- **Current State:** Shared state uses atomic operations and thread-safe data structures.
- **Weakness:** Reference repositories in `sitolo-persistence/src/memory.rs` use in-memory `std::sync::RwLock` and `Mutex`. Under high concurrent write traffic, lock contention will severely degrade throughput and create blocking hazards across Tokio worker threads.

### 1.10 Database Design, Query Efficiency, Transactions, Migrations, Indexing, and Schema Evolution
- **Current State:** `sitolo-persistence/src/postgres.rs` defines strict dual-pool PostgreSQL authority setup (`admin_pool` vs. least-privileged `runtime_pool`) with catalog-level RLS policy verification.
- **Weakness:** `apps/api` does not connect to PostgreSQL at runtime! It initializes non-connecting `DatabaseRuntimeConfig` and defaults to ephemeral in-memory state. Real database transactions, migrations, and query execution are completely bypassed.

### 1.11 Caching Strategy, Invalidation Logic, and Consistency Tradeoffs
- **Current State:** Redis/distributed cache is non-existent.
- **Weakness:** All read requests query in-memory state or database directly without a caching or cache-invalidation layer, leading to potential database exhaustion under scale.

### 1.12 Queueing, Background Jobs, Event Handling, and Idempotency
- **Current State:** `sitolo-events` contains no event bus runtime.
- **Weakness:** `apps/worker` contains no job processing loop, queue subscription, or retry mechanism. Async tasks, background emails, audit log flushing, and tax reporting cannot be processed asynchronously.

### 1.13 Observability: Logs, Metrics, Traces, Alertability, and Incident Diagnosability
- **Current State:** Structured JSON logging via `tracing-subscriber` and in-memory `TelemetryBuffer` in `sitolo-observability`.
- **Weakness:** Metrics are not exported to Prometheus or OpenTelemetry collector endpoints. Traces are buffered in memory without OTLP gRPC/HTTP exporter integration, making cross-service tracing impossible in production.

### 1.14 Test Coverage, Test Quality, Edge Cases, and Regression Risk
- **Current State:** Unit and contract test coverage across domain, auth, and persistence modules is high.
- **Weakness:** Lack of end-to-end integration tests for real HTTP transport, complete user authentication flows, and PostgreSQL RLS transaction behavior under multi-tenant concurrent traffic.

### 1.15 Configuration Management, Environment Separation, and Secrets Handling
- **Current State:** Environment configuration is validated with schema versioning and config fingerprinting. Development secrets use `EnvSecretProvider`.
- **Weakness:** `Production` secret adapter is missing in `apps/api/src/bootstrap.rs` (causes startup failure by design, but prevents actual production deployment until AWS KMS / HashiCorp Vault adapter is written).

### 1.16 Deployment Safety, Rollback Readiness, Versioning, and Release Discipline
- **Current State:** Cargo toolchain and dependencies are pinned in `Cargo.lock` and `rust-toolchain.toml`.
- **Weakness:** Missing container definitions (Dockerfile), Kubernetes manifests, health check probes for HTTP framework, and zero-downtime deployment scripts.

### 1.17 Code Quality, Naming, Duplication, Coupling, Dead Code, and Technical Debt
- **Current State:** Strict Clippy and `deny.toml` policies are configured.
- **Weakness:** Unused code warnings exist in `sitolo-persistence/src/postgres.rs` (`PgAuthorityError`, `PgAuthorityPools` methods, `set_transaction_tenant_context`). `apps/worker` is a dead stub.

### 1.18 Maintainability Under Team Growth, Code Ownership, and Refactor Cost
- **Current State:** Modular Rust crate topology enforces boundary separation.
- **Weakness:** Divergence between architecture specifications (which mandate Axum + Tokio router) and current binary implementation (`apps/api/src/serve.rs`) creates developer confusion and high refactor cost.

### 1.19 Compliance and Enterprise Operational Expectations
- **Current State:** Cryptographic specifications and audit event structures align with enterprise standards.
- **Weakness:** Missing operational runbooks, data retention enforcement, tax authority (MRA EIS) integration runtime, and automated DR (Disaster Recovery) failover procedures.

---

## 2. Comprehensive Findings List

### Critical Severity Findings

#### [FINDING-CRIT-01] Complete Absence of Authentication & Authorization Middleware on HTTP API
- **What is wrong:** The API server (`apps/api/src/serve.rs`) dispatches incoming HTTP endpoints directly to `sitolo-api::tenancy` handlers without verifying HTTP `Authorization` headers, JWT tokens, session cookies, or MFA credentials.
- **Why it is wrong:** Any user or external network caller can execute organization and branch mutations (`POST /v1/organizations`, `POST /v1/organizations/{id}/suspend`, `POST /v1/organizations/{id}/branches`, etc.) without proving identity or permissions.
- **Where it appears:** `apps/api/src/serve.rs`, function `dispatch_request()`.
- **How it fails in production:** An unauthenticated attacker sends a HTTP POST request to `/v1/organizations/org_123/suspend` and successfully shuts down a tenant's business operation.
- **Proper fix:** Migrate `apps/api` to Axum framework. Add Tower middleware for authentication (`sitolo-auth`), session validation, and scope authorization (`sitolo-authz`/`sitolo-tenancy`).
- **Priority:** P0 | **Blast Radius:** System-wide complete compromise.

#### [FINDING-CRIT-02] Broken Custom HTTP Parser Over Raw TCP Stream
- **What is wrong:** `apps/api/src/serve.rs` handles HTTP requests by directly invoking `stream.read(&mut buf)` into a fixed 32KB buffer (`PROBE_MAX_BYTES`).
- **Why it is wrong:** TCP is a stream protocol; request bodies larger than a single TCP packet or split across packet boundaries are truncated. The code assumes the entire HTTP request headers and body arrive in a single `read()` call. Furthermore, HTTP header parsing is implemented with simple string splits (`headers_part.lines()`), missing HTTP standard compliance (chunked transfer encoding, keep-alive, header normalization, TLS decryption).
- **Where it appears:** `apps/api/src/serve.rs`, function `handle_connection()`.
- **How it fails in production:** Valid POST requests with JSON bodies exceeding a few kilobytes or arriving in multiple TCP packets are parsed as truncated JSON, causing 422 errors or malformed data processing.
- **Proper fix:** Replace custom raw TCP loop in `apps/api/src/serve.rs` with `axum::Router` running on `tokio::net::TcpListener` via `axum::serve`.
- **Priority:** P0 | **Blast Radius:** All incoming API traffic.

#### [FINDING-CRIT-03] Core Business Logic Engines Are Completely Unimplemented
- **What is wrong:** The entire suite of business engines—product catalogue, inventory ledger, POS sales, payment processing, bank reconciliation, MRA EIS fiscal compliance, returns/refunds, and billing—exists only as Markdown documentation contracts and is entirely absent from Rust source code.
- **Why it is wrong:** The system cannot process sales, track inventory, compute tax, or handle payments.
- **Where it appears:** `crates/sitolo-domain/src/lib.rs` (only exposes `pub mod tenancy;`), `crates/sitolo-application/src/`.
- **How it fails in production:** Any operational API request beyond tenancy management returns 404 Not Found.
- **Proper fix:** Implement core domain aggregates, state machines, and application services across `sitolo-domain` and `sitolo-application`.
- **Priority:** P0 | **Blast Radius:** Core business capability.

---

### High Severity Findings

#### [FINDING-HIGH-01] API Binary Runs on Ephemeral In-Memory State; PostgreSQL Disconnected
- **What is wrong:** `apps/api/src/bootstrap.rs` builds a `DatabaseRuntimeConfig` intent but performs no database connection setup or I/O. The API state (`AppState`) initializes in-memory tenancy repositories instead of PostgreSQL pools.
- **Why it is wrong:** Data written through the API is stored in ephemeral RAM and lost immediately upon process restart. PostgreSQL RLS isolation policies and durable transactions are ignored.
- **Where it appears:** `apps/api/src/bootstrap.rs` and `apps/api/src/state.rs`.
- **How it fails in production:** Restarting the API container wipes all provisioned organizations, branches, and tenant data.
- **Proper fix:** Connect `PgAuthorityPools` during bootstrap and inject PostgreSQL-backed repositories into `AppState`.
- **Priority:** P1 | **Blast Radius:** Data persistence and tenant durability.

#### [FINDING-HIGH-02] Worker Binary Is an Unexecuted Scaffold
- **What is wrong:** `apps/worker/src/main.rs` contains only `println!("sitolo worker scaffold");` and terminates immediately.
- **Why it is wrong:** Background jobs, transactional outbox processing, async event handling, and scheduled tasks cannot run.
- **Where it appears:** `apps/worker/src/main.rs`.
- **How it fails in production:** Asynchronous integration tasks (e.g. submitting fiscal invoices to MRA EIS or sending receipt emails) are never processed.
- **Proper fix:** Implement a Tokio worker runtime with PostgreSQL outbox table polling, job consumer loops, and exponential backoff retries.
- **Priority:** P1 | **Blast Radius:** Asynchronous processing & integrations.

#### [FINDING-HIGH-03] Blocking CPU-Bound Password Hashing on Async Reactor Threads
- **What is wrong:** Argon2id password hashing in `crates/sitolo-auth/src/password.rs` is executed synchronously inside async functions without delegating to blocking thread pools.
- **Why it is wrong:** Argon2id is intentionally CPU and memory intensive. Running it on a Tokio worker thread blocks that thread from processing other async I/O events.
- **Where it appears:** `crates/sitolo-auth/src/password.rs`.
- **How it fails in production:** Concurrent login requests saturate all Tokio worker threads, causing massive HTTP request latency spikes and connection timeouts across the entire API.
- **Proper fix:** Wrap Argon2id hashing calls with `tokio::task::spawn_blocking`.
- **Priority:** P1 | **Blast Radius:** Server latency and concurrency performance.

---

### Medium Severity Findings

#### [FINDING-MED-01] Missing Managed Production Secret Provider
- **What is wrong:** `apps/api/src/bootstrap.rs` returns `StartupError::SecretProviderUnavailable` when `environment == Production` because no production secret adapter (e.g. AWS KMS, HashiCorp Vault) is implemented.
- **Why it is wrong:** The application cannot start in production mode.
- **Where it appears:** `apps/api/src/bootstrap.rs`.
- **How it fails in production:** Deployment fails at process startup with `SecretProviderUnavailable`.
- **Proper fix:** Implement a KMS/Vault backed `SecretProvider` in `sitolo-security`.
- **Priority:** P2 | **Blast Radius:** Deployment readiness.

#### [FINDING-MED-02] In-Memory Telemetry Buffer Missing OTLP/Prometheus Exporters
- **What is wrong:** `sitolo-observability` buffers traces and logs in an in-memory `TelemetryBuffer` ring buffer, but does not export them to external telemetry collectors.
- **Why it is wrong:** Operational metrics and traces are lost when process restarts, and cannot be monitored in Grafana/Datadog.
- **Where it appears:** `crates/sitolo-observability/src/lib.rs`.
- **How it fails in production:** Operators have zero visibility into real-time production metrics, errors, or performance bottlenecks.
- **Proper fix:** Add OpenTelemetry OTLP gRPC/HTTP exporter and Prometheus metrics endpoint (`/metrics`).
- **Priority:** P2 | **Blast Radius:** Observability and incident response.

---

### Low Severity Findings

#### [FINDING-LOW-01] Dead Code Warnings in `sitolo-persistence`
- **What is wrong:** Compiler warnings for unused functions and types (`PgAuthorityError`, `PgAuthorityPools` methods, `set_transaction_tenant_context`).
- **Why it is wrong:** Code clutter and potential developer confusion.
- **Where it appears:** `crates/sitolo-persistence/src/postgres.rs`.
- **How it fails in production:** No operational failure, but degrades code maintainability.
- **Proper fix:** Wire `postgres.rs` functions into application repositories or mark as public API for integration tests.
- **Priority:** P3 | **Blast Radius:** Code maintainability.

---

## 3. Top 10 Highest-Risk Issues

1. **Unauthenticated HTTP API:** Anyone can mutate tenant organizations and branches without credentials.
2. **Raw TCP HTTP Parser:** Incomplete stream handling leads to truncated payloads and request failures under multi-packet TCP traffic.
3. **Missing Business Logic Engines:** No POS, inventory, tax, or sales processing capabilities in code.
4. **Disconnected Database Persistence:** API relies on ephemeral RAM; restarts lose all state.
5. **Worker Scaffold Deadlock:** Async/background tasks, tax submissions, and outbox event publishing are non-functional.
6. **Thread Starvation via Argon2 Hashing:** CPU-bound password hashing executed on Tokio reactor loops blocks async processing.
7. **Absence of Rate Limiting Middleware on API:** Susceptible to Denial of Service (DoS) and brute-force credential stuffing.
8. **Missing Production Secret Adapter:** Process refuses to start in production environment mode.
9. **Lack of Distributed Tracing & External Metrics:** In-memory telemetry buffer fails to report to monitoring infrastructure.
10. **In-Memory Lock Contention:** Shared `RwLock` in memory repositories causes blocking under concurrent write load.

---

## 4. Top 10 Highest-Leverage Fixes

1. **Axum HTTP Framework Adoption:** Replace raw TCP loop in `apps/api/src/serve.rs` with standard Axum router and Tower middleware stack.
2. **Auth & Scope Extraction Middleware:** Enforce session token validation and `AuthorizedScope` checks on all API endpoints.
3. **PostgreSQL Persistence Wiring:** Connect `PgAuthorityPools` during bootstrap and replace memory repositories with SQLx/PostgreSQL implementations.
4. **Offload Argon2 to `spawn_blocking`:** Wrap synchronous CPU-heavy password hashing to prevent Tokio thread pool starvation.
5. **Transactional Outbox & Worker Runtime:** Implement outbox event publisher and worker loop in `apps/worker`.
6. **Core Domain Engine Implementation:** Build product catalogue, inventory ledger, POS, and sales modules in `sitolo-domain`.
7. **OTLP & Prometheus Exporting:** Wire `sitolo-observability` to OpenTelemetry gRPC exporters and expose `/metrics`.
8. **AWS KMS / Vault Secret Provider:** Implement production secret provider adapter in `sitolo-security`.
9. **Containerization & Deployment Assets:** Provide multi-stage Dockerfile, Helm charts, and health check endpoints.
10. **End-to-End Integration Test Suite:** Build integration test suite verifying tenant RLS, HTTP auth, and database persistence.

---

## 5. Phased Remediation Plan

```text
PHASE 1 (Immediate: 0–14 Days)
├── Replace raw TCP parser in apps/api with Axum + Tokio router
├── Wire sitolo-auth & sitolo-authz middleware into API pipeline
├── Connect PgAuthorityPools to API state and enable PostgreSQL persistence
└── Offload Argon2 password hashing to tokio::task::spawn_blocking

PHASE 2 (Short Term: 15–45 Days)
├── Implement core domain engines (catalogue, inventory ledger, sales, POS)
├── Implement outbox queue processor and worker loop in apps/worker
├── Implement production secret provider adapter (AWS KMS / HashiCorp Vault)
└── Add OTLP gRPC telemetry exporter & Prometheus metrics endpoint

PHASE 3 (Medium Term: 46–90 Days)
├── Implement MRA EIS fiscal tax integration & payment gateway adapters
├── Build mobile/desktop offline synchronization protocol in sitolo-sync
├── Implement Redis distributed caching layer
└── Implement multi-region automated DR failover & database backup verification

PHASE 4 (Long Term: 90+ Days)
├── Conduct external third-party penetration testing and compliance audit
├── Perform enterprise load/stress testing (10,000+ RPS)
└── Complete multi-tenant production certification & pilot SME rollout
```

---

## 6. Enterprise-Grade Classification Benchmark

| Subsystem / Capability | Current Implementation Status | Enterprise-Grade Status |
| :--- | :--- | :--- |
| **Workspace Architecture & Rules** | Zero `unsafe` code policy, strict Clippy & dependency governance | **ACCEPTABLE** |
| **Tenancy Isolation Types** | Server-authoritative `AuthorizedScope` & requested-vs-trusted scoping | **ACCEPTABLE** |
| **Password Hashing Standard** | Argon2id with memory-hard parameter configuration | **ACCEPTABLE** |
| **HTTP Transport Layer** | Custom TCP string-splitting loop without headers/TLS | **NOT ENTERPRISE-GRADE** |
| **API Security Enforcement** | Unauthenticated endpoints without auth/authz middleware | **NOT ENTERPRISE-GRADE** |
| **Persistence Infrastructure** | Non-connecting DB intent; ephemeral in-memory storage | **NOT ENTERPRISE-GRADE** |
| **Background Processing** | Non-functional scaffold binary (`apps/worker`) | **NOT ENTERPRISE-GRADE** |
| **Business Domain Engine** | Documentation contracts only; zero domain code for POS/inventory | **NOT ENTERPRISE-GRADE** |
| **Observability Exporters** | In-memory buffer without OTLP / Prometheus connection | **NOT ENTERPRISE-GRADE** |
