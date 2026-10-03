# Sitolo Codebase Comprehensive Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main`
**Status:** Current-State Comprehensive Enterprise Audit & Operational Assessment
**Authority:** Current repository source tree plus governing specification hierarchy (`agent.md`, `../CLAUDE.md`, `docs/README.md`)

---

## Executive Summary

Sitolo is **not production-ready**. While the repository contains a well-designed domain conceptual model and partial security/tenancy primitives (`sitolo-auth`, `sitolo-authz`, `sitolo-tenancy`, `sitolo-security`), it exhibits severe architectural divergences, missing transport security controls, unauthenticated API endpoints, unexecuted persistence authority code, and completely absent core operational engines (POS/Sales, Inventory, Payments, Reconciliation, Async Workers, Offline Sync).

### Overall Health Assessment

The system currently sits at a posture best described as:

```text
CONCEPTUAL ARCHITECTURE & SECURITY SPECS
               ↓
PARTIAL SECURITY & TENANCY PRIMITIVES
               ↓
INCOMPLETE HTTP / PERSISTENCE / WORKER BOUNDARIES
               ↓
NOT PRODUCTION READY
```

### Key Architectural & Operational Gaps

1. **HTTP Transport Divergence & Manual Buffer Parsing**: The documented HTTP server architecture (`agent.md`, `system_architecture_design.md`, `api_contract.md`) specifies Rust + Axum + Tokio. The binary in `apps/api/src/serve.rs` relies on a custom raw TCP stream parser (`stream.read(&mut buf)`) that does not parse HTTP headers properly, truncates TCP frames larger than 32 KB, lacks TLS, lacks CORS/CSRF middleware, and uses unsafe manual string slicing for HTTP dispatch.
2. **Unauthenticated & Unprotected Public API Endpoints**: In `apps/api/src/serve.rs`, request dispatch (`dispatch_request`) invokes tenancy mutation handlers (`handle_provision_organization`, `handle_suspend_organization`, `handle_create_branch`, etc.) directly without extracting bearer tokens, validating sessions via `sitolo-auth`, or constructing `AuthorizedScope`. Any unauthenticated user with network access to the TCP port can mutate tenant organizations and branches.
3. **Dead PostgreSQL Authority Code & In-Memory Fallback**: Production persistence logic in `crates/sitolo-persistence/src/postgres.rs` (`PgAuthorityPools`, `verify_runtime_role`, `verify_effective_privileges`, `verify_rls_catalog_metadata`) is marked with dead code compiler warnings and is never invoked by the API binary. `apps/api` runs against `sitolo-persistence/src/memory.rs` (in-memory hash maps with `Mutex`/`RwLock`), which wipes state on process restart, cannot scale beyond a single instance, and provides no real RLS or SQL transaction boundaries.
4. **Unimplemented Async Workers and Event Runtime**: `apps/worker/src/main.rs` is an empty scaffold (`fn main() {}`). Event publishing (`sitolo-events`), external integrations (`sitolo-integrations`), and offline synchronization (`sitolo-sync`) are empty crate stubs containing only module docstrings.
5. **Missing Business Domain Engines**: Core commercial capabilities—Product Catalogue, Inventory Ledger, POS / Sales, Payments & Reconciliation, Procurement, Cash Close, Tax / MRA EIS, and Reporting—have no runtime code in `crates/sitolo-domain/src/` (which only contains `tenancy.rs`).

---

# 1. System Domain Deep-Dive Review

### 1.1 Repository Structure & Dependency Boundaries
- **Current State**: The repository uses a Rust Cargo workspace split across `apps/` (`api`, `worker`) and 15 crates under `crates/`.
- **Strengths**: Dependency directions are explicitly bounded in `Cargo.toml`. `sitolo-domain` has no external web or SQL framework dependencies (`#![forbid(unsafe_code)]`).
- **Defects**: `apps/api` bypasses the specified Axum web framework and implements a custom TCP parser. `apps/worker` is a dummy binary with no worker loop. `sitolo-events`, `sitolo-integrations`, and `sitolo-sync` are empty stubs.

### 1.2 Application Architecture & Module Separation
- **Current State**: `sitolo-application` orchestrates tenancy and IAM services using in-memory repositories.
- **Defects**: Module isolation is incomplete because business domain engines (catalogue, inventory, sales, payments) do not exist in domain crates. Application logic directly depends on in-memory persistence stubs without full domain aggregate state transitions.

### 1.3 API Design, Request Flow & Trust Boundaries
- **Current State**: Transport DTOs in `sitolo-api` enforce `deny_unknown_fields` and request payload byte bounds.
- **Defects**: Request dispatch in `apps/api/src/serve.rs` lacks HTTP header parsing, JWT token extraction, and authorization context construction. Unauthenticated requests reaching `POST /v1/organizations` directly invoke application mutation logic.

### 1.4 Authentication, Authorization & Session Handling
- **Current State**: `sitolo-auth`, `sitolo-authz`, and `sitolo-tenancy` contain well-designed algorithms for session sliding, password hashing, MFA, role catalogs, and scope resolution.
- **Defects**: `apps/api` does NOT wire `sitolo-auth` middleware into the request flow. HTTP handlers do not extract Authorization headers, leaving the entire API unauthenticated at the transport layer.

### 1.5 Input Validation, Sanitization & Data Integrity
- **Current State**: `sitolo-api::tenancy` enforces body size limits (`MAX_TENANCY_BODY_BYTES`) and string length bounds on identifiers and names.
- **Defects**: Manual HTTP parsing in `serve.rs` splits requests on whitespace (`parts[0]`, `parts[1]`). Malformed or HTTP/1.0 headers without space delimiters panic or get misrouted. No URL encoding or path normalization is performed before route comparison.

### 1.6 Vulnerability Analysis (Injection, XSS, CSRF, SSRF, IDOR, Path Traversal, Secrets)
- **Current State**: Compile-time SQL parameterization is enforced via `sqlx`. Secret masking is implemented in `sitolo-security`.
- **Defects**:
  - **IDOR**: API routes accept `org_id` and `branch_id` path segments directly without validating whether the caller's session token grants access to those specific UUIDs.
  - **CSRF / CORS**: No CORS headers (`Access-Control-Allow-Origin`) or CSRF checks exist on HTTP handlers.
  - **Transport Security**: Server binds plain TCP (`TcpListener::bind`) with no TLS support. Credentials and session tokens travel in plaintext.

### 1.7 Error Handling, Retry Behavior, Timeout Strategy & Failure Isolation
- **Current State**: `AppError` and `ProblemDetails` conform to RFC 7807 error specifications.
- **Defects**: TCP connection handler in `serve.rs` catches timeout errors during `stream.read()` and abruptly drops connections without returning a standard 408 Timeout or 500 error response.

### 1.8 CPU-Bound vs I/O-Bound Bottlenecks
- **Current State**: Async primitives use Tokio.
- **Defects**: In-memory repositories in `sitolo-persistence/src/memory.rs` use synchronous `Mutex` and `RwLock` inside async handler futures. Under high concurrent request volumes, lock contention will block Tokio worker threads.

### 1.9 Async Behavior, Blocking Operations, Race Conditions & Concurrency Hazards
- **Current State**: `serve.rs` manages connection limits using `tokio::sync::Semaphore` (max 1024 concurrent connections).
- **Defects**: Multi-packet TCP request streams are not buffered or framed correctly. A client sending headers in one packet and body in another will have its body truncated, causing non-deterministic JSON parse errors.

### 1.10 Database Design, Query Efficiency, Transactions, Migrations, Indexing & RLS
- **Current State**: `crates/sitolo-persistence/src/postgres.rs` contains robust RLS verification helper functions (`verify_runtime_role`, `verify_effective_privileges`, `verify_rls_catalog_metadata`).
- **Defects**: All `postgres.rs` helpers are unreferenced/dead code in the production API binary. `apps/api` connects exclusively to in-memory mocks. No actual PostgreSQL migrations or repositories are wired to domain commands.

### 1.11 Caching Strategy, Invalidation Logic & Consistency Tradeoffs
- **Current State**: Redis is defined in architecture specs as an optional read acceleration layer.
- **Defects**: No caching infrastructure or invalidation patterns exist in source code.

### 1.12 Queueing, Background Jobs, Event Handling & Idempotency
- **Current State**: Outbox and transactional event patterns are specified in Phase 4 Part 8 specs.
- **Defects**: `apps/worker` is an empty scaffold. `sitolo-events` contains no event publication runtime. Outbox messages are never claimed or processed.

### 1.13 Observability: Logs, Metrics, Traces, Alertability & Diagnosability
- **Current State**: `sitolo-observability` provides structured JSON tracing and bounded priority event buffers.
- **Defects**: No Prometheus/OpenTelemetry metrics exporters exist. Tracing spans are not propagated across HTTP request headers or background jobs.

### 1.14 Test Coverage, Quality, Edge Cases & Regression Risk
- **Current State**: Crate-level unit tests for auth, tenancy, config, and observability pass reliably.
- **Defects**: PostgreSQL integration tests (`rls_security_tests.rs`) fail in environments lacking a live PostgreSQL instance (`ADMIN_DATABASE_URL`). End-to-end API integration tests run against mock in-memory state only.

### 1.15 Configuration Management, Environment Separation & Secrets Handling
- **Current State**: `sitolo-config` enforces environment overlays (dev, staging, production) and rejects development secrets in production profile.
- **Defects**: Production secret distribution relies on environment variables without integration with external secret managers (e.g. HashiCorp Vault, AWS Secrets Manager).

### 1.16 Deployment Safety, Rollback Readiness, Versioning & Release Discipline
- **Current State**: Cargo workspace uses unified workspace versioning.
- **Defects**: No health readiness probes check PostgreSQL connection viability or worker queue health. `GET /process/ready` unconditionally returns `200 OK`.

### 1.17 Code Quality, Naming, Technical Debt & Dead Code
- **Current State**: Workspace rules enforce `#![forbid(unsafe_code)]` across all crates.
- **Defects**: Dead code compiler warnings exist in `sitolo-persistence/src/postgres.rs`.

### 1.18 Maintainability, Code Ownership & Team Scale
- **Current State**: Crate ownership is cleanly segmented by domain concerns.
- **Defects**: Discrepancy between documentation specifications (Axum/Tokio) and implementation (`serve.rs` custom TCP) creates developer confusion.

### 1.19 Enterprise Compliance & Operational Expectations (MRA EIS, Tax, Privacy)
- **Current State**: Compliance specifications exist for MRA EIS (Malawi Revenue Authority Electronic Invoicing System).
- **Defects**: Zero integration code exists for MRA EIS fiscal signature generation or tax submission queues in `sitolo-integrations`.

---

# 2. Executive Codebase Health Matrix

| Dimension | Posture | Root Cause / Evidence |
|---|---|---|
| **1. Architecture & Boundaries** | **Unacceptable** | Documented Axum architecture diverges from raw TCP parser in `apps/api/src/serve.rs`. |
| **2. Application Architecture** | **Partial** | Tenancy application services exist, but core operational business engines are absent. |
| **3. API & Trust Boundaries** | **Critical Defect** | Public HTTP handlers in `apps/api/src/serve.rs` execute unauthenticated. |
| **4. AuthN & AuthZ** | **Partial / Unwired** | Primitives exist in `sitolo-auth`/`sitolo-authz` but are not attached to HTTP transport. |
| **5. Input Validation** | **Partial** | DTO byte limits exist; HTTP request parsing uses naive string slicing. |
| **6. Vulnerability Risks** | **Critical Defect** | Missing TLS, missing CORS/CSRF, unauthenticated IDOR paths on tenant mutations. |
| **7. Errors & Isolation** | **Partial** | Standard RFC 7807 problem details exist; TCP read timeouts drop sockets abruptly. |
| **8. Performance / Bottlenecks** | **Unacceptable** | In-memory synchronous locks (`Mutex`/`RwLock`) block Tokio runtime threads. |
| **9. Concurrency Hazards** | **Unacceptable** | TCP frame fragmentation truncates JSON payloads across packet boundaries. |
| **10. Database & RLS** | **Unacceptable** | PostgreSQL authority code in `postgres.rs` is dead code; API uses in-memory mock. |
| **11. Caching Strategy** | **Not Implemented** | No cache layer or invalidation logic present. |
| **12. Workers & Queues** | **Unacceptable** | `apps/worker` is an empty scaffold (`fn main() {}`). |
| **13. Observability** | **Partial** | Structured JSON logs exist; missing OTLP metrics and trace propagation. |
| **14. Testing & Quality** | **Partial** | High unit test coverage; missing live integration and E2E API tests. |
| **15. Config & Secrets** | **Acceptable Substrate**| Environment-aware config validation exists; missing KMS secret provider. |
| **16. Deployment Safety** | **Unacceptable** | Readiness probe returns fake 200 OK without checking database connectivity. |
| **17. Code Quality & Debt** | **Partial** | Clean `#![forbid(unsafe_code)]` compliance; dead code in persistence crate. |
| **18. Maintainability** | **Partial** | Modular structure; high risk of confusion due to spec/code divergence. |
| **19. Enterprise Compliance** | **Not Implemented** | MRA EIS tax integration is contract-only with no runtime code. |

---

# 3. Severity-Ranked Findings List

## Critical Findings (P0 — Release Blocking)

### Finding C-01: Unauthenticated Public API Exposure on Tenancy Mutation Endpoints
- **What is wrong**: `dispatch_request` in `apps/api/src/serve.rs` dispatches `POST /v1/organizations` and organization lifecycle actions (`activate`, `suspend`, `resume`, `begin_close`, `close`) directly to application services without checking authentication headers or verifying caller identity.
- **Why it is wrong**: Violates Core Directives 3, 4, 9, and 10 in `agent.md`. Anyone on the network can create, suspend, or delete tenant organizations without credentials.
- **Where it appears**: `apps/api/src/serve.rs`, lines 112–215.
- **How it fails in production**: An unauthenticated attacker sends an HTTP POST request to `/v1/organizations/{org_id}/suspend` and immediately shuts down a tenant's business operations.
- **What a proper fix looks like**: Refactor `apps/api` to use Axum with authentication middleware (`sitolo-auth`) that extracts bearer tokens, validates session state, and resolves `AuthorizedScope` before reaching handlers.
- **Priority & Blast Radius**: **P0 / System-Wide Multi-Tenant Compromise**.

### Finding C-02: Non-Standard TCP HTTP Parser and Frame Truncation Bug
- **What is wrong**: `apps/api/src/serve.rs` uses `stream.read(&mut buf)` with a single 32 KB buffer read over raw TCP instead of using a standard HTTP stack (Axum/Hyper).
- **Why it is wrong**: HTTP requests split across multiple TCP packets or exceeding 32 KB have their body truncated before JSON deserialization (`serde_json::from_str`).
- **Where it appears**: `apps/api/src/serve.rs`, lines 58–75.
- **How it fails in production**: Requests sent over slow mobile connections or carrying larger payloads fail with unprocessable entity errors or corrupt state due to truncated JSON inputs.
- **What a proper fix looks like**: Replace custom TCP loop in `serve.rs` with `axum::serve` and Hyper, using `axum::extract::Json` and standard Tokio HTTP connection drivers.
- **Priority & Blast Radius**: **P0 / API Availability & Data Integrity Breakdown**.

### Finding C-03: Production Persistence Uses In-Memory Mock Instead of PostgreSQL RLS
- **What is wrong**: The API binary instantiates `MemoryTenancyRepository` (`sitolo-persistence/src/memory.rs`) instead of connecting to PostgreSQL via `PgAuthorityPools` (`sitolo-persistence/src/postgres.rs`).
- **Why it is wrong**: The entire PostgreSQL authority model, row-level security (RLS), and transaction isolation guarantees described in `database_design.md` are completely bypassed.
- **Where it appears**: `apps/api/src/bootstrap.rs` and `apps/api/src/serve.rs`.
- **How it fails in production**: All data is stored in process RAM. A server restart or crash permanently loses all tenant data. Deploying multiple API instances causes data divergence across pods.
- **What a proper fix looks like**: Wire `PgAuthorityPools` into `apps/api/src/bootstrap.rs`, execute migrations on startup, and pass PostgreSQL transaction contexts to application services.
- **Priority & Blast Radius**: **P0 / Data Loss & State Persistence Failure**.

---

## High Findings (P1 — Serious Operational / Security Risks)

### Finding H-01: Completely Scaffolded Background Worker Binary
- **What is wrong**: `apps/worker/src/main.rs` consists solely of `fn main() {}`.
- **Why it is wrong**: Asynchronous tasks, transactional outbox delivery, email/SMS notifications, and payment callbacks cannot be processed asynchronously.
- **Where it appears**: `apps/worker/src/main.rs`.
- **How it fails in production**: Outbox records accumulate indefinitely in the database, background jobs never execute, and external side effects fail silently.
- **What a proper fix looks like**: Implement a robust worker polling loop in `apps/worker` using Tokio and SQLx `FOR UPDATE SKIP LOCKED` outbox claiming semantics.
- **Priority & Blast Radius**: **P1 / Asynchronous Processing & Integration Outages**.

### Finding H-02: Lack of Transport Layer Security (TLS) and Plaintext Traffic
- **What is wrong**: `TcpListener::bind` serves unencrypted HTTP over port 8080.
- **Why it is wrong**: Authentication tokens, session cookies, and sensitive merchant data are transmitted in plaintext over public networks.
- **Where it appears**: `apps/api/src/serve.rs`, line 36.
- **How it fails in production**: Network eavesdroppers intercept credentials and session tokens via man-in-the-middle (MITM) attacks.
- **What a proper fix looks like**: Enforce TLS termination using Rustls/Axum or require an authenticated ingress proxy (e.g. NGINX/Envoy) with strict HTTPS redirection.
- **Priority & Blast Radius**: **P1 / Network Eavesdropping & Credential Theft**.

### Finding H-03: Absence of Core Business Domain Engines
- **What is wrong**: `crates/sitolo-domain/src/` contains only `tenancy.rs`. Operational domains (sales, catalogue, inventory, payments, cash, tax) are missing.
- **Why it is wrong**: Sitolo cannot perform its primary commercial functions (processing POS sales, managing inventory balances, issuing fiscal receipts).
- **Where it appears**: `crates/sitolo-domain/src/`.
- **How it fails in production**: API cannot serve checkout, stock management, or reporting workflows.
- **What a proper fix looks like**: Implement domain aggregates, state transitions, and invariants for sales, inventory, catalogue, and payments according to Phase 8–15 specifications.
- **Priority & Blast Radius**: **P1 / Core Commercial Capability Gap**.

---

## Medium Findings (P2 — Reliability & Maintainability Debt)

### Finding M-01: Dishonest Readiness Probe Behavior
- **What is wrong**: `GET /process/ready` unconditionally returns `200 OK` with `{"status":"ready"}` without checking PostgreSQL database connectivity or internal subsystem health.
- **Why it is wrong**: Orchestrators (Kubernetes) will route live user traffic to API pods that have lost database connectivity.
- **Where it appears**: `apps/api/src/serve.rs`, lines 105–108.
- **How it fails in production**: Traffic is routed to unhealthy instances, causing widespread 500 error spikes for end users.
- **What a proper fix looks like**: Implement database ping checks and pool health verification inside the `/process/ready` handler.
- **Priority & Blast Radius**: **P2 / Load Balancing & Fault Tolerance Degradation**.

### Finding M-02: Synchronous Mutex Contention in Async Runtime
- **What is wrong**: `MemoryTenancyRepository` uses `std::sync::Mutex` and `RwLock` inside async futures.
- **Why it is wrong**: Holding synchronous std locks across async await points or under high thread contention starves the Tokio thread pool.
- **Where it appears**: `crates/sitolo-persistence/src/memory.rs`.
- **How it fails in production**: Under heavy load, API request latency spikes exponentially as Tokio worker threads stall waiting on locks.
- **What a proper fix looks like**: Replace in-memory locks with PostgreSQL transactional queries or `tokio::sync::RwLock` for temporary in-memory structures.
- **Priority & Blast Radius**: **P2 / Latency Spikes & Tokio Thread Starvation**.

---

## Low Findings (P3 — Code Hygiene & Telemetry Gaps)

### Finding L-01: Dead Code Warnings in Persistence Crate
- **What is wrong**: Compiler emits `dead_code` warnings for `PgAuthorityPools` and its verification methods in `sitolo-persistence`.
- **Why it is wrong**: Clutters build logs and indicates unexecuted infrastructure code.
- **Where it appears**: `crates/sitolo-persistence/src/postgres.rs`.
- **How it fails in production**: Does not cause runtime failure directly, but reflects unintegrated security code.
- **What a proper fix looks like**: Wire `PgAuthorityPools` into application startup and integration test suites.
- **Priority & Blast Radius**: **P3 / Code Hygiene & Maintainability**.

---

# 4. Top 10 Highest-Risk Issues

1. **Unauthenticated API Mutation Endpoints**: Public access to tenant creation, suspension, and deletion (`apps/api/src/serve.rs`).
2. **Raw TCP Buffer Slicing & HTTP Body Truncation**: Custom TCP server truncates HTTP payloads split across packets (`apps/api/src/serve.rs`).
3. **In-Memory Volatile Persistence in Production API**: All state wiped on restart; zero multi-pod scaling support (`sitolo-persistence/src/memory.rs`).
4. **Bypassed PostgreSQL Row-Level Security (RLS)**: Real database authority code (`postgres.rs`) is completely unexecuted.
5. **Empty Async Worker Binary**: Background jobs, outbox processing, and external side effects never run (`apps/worker/src/main.rs`).
6. **Unencrypted HTTP Traffic**: Plaintext HTTP exposure over raw TCP socket without TLS (`apps/api/src/serve.rs`).
7. **Missing Business Domain Engines**: POS Sales, Inventory Ledger, Catalogue, and Payments domain logic completely absent (`sitolo-domain`).
8. **Dishonest Readiness Probes**: `/process/ready` returns 200 OK even if database or dependencies are offline (`apps/api/src/serve.rs`).
9. **Unenforced IDOR & Scope Boundaries at HTTP Layer**: API path parameters (`org_id`, `branch_id`) are not verified against caller tokens.
10. **Lack of Integration Event Runtime**: `sitolo-events`, `sitolo-integrations`, and `sitolo-sync` are empty stubs.

---

# 5. Top 10 Highest-Leverage Fixes

1. **Migrate `apps/api` to Axum & Tokio**: Replace raw TCP parser in `serve.rs` with Axum web framework to gain robust HTTP parsing, routing, and body streaming.
2. **Attach `sitolo-auth` Middleware to API Routes**: Require valid bearer token authentication and `AuthorizedScope` context on all non-probe endpoints.
3. **Connect `apps/api` to PostgreSQL `PgAuthorityPools`**: Replace in-memory mock repositories with SQLx PostgreSQL persistent repositories enforcing RLS.
4. **Implement Transactional Outbox Worker in `apps/worker`**: Build an outbox worker loop using `FOR UPDATE SKIP LOCKED` for reliable async execution.
5. **Implement POS Sales & Inventory Domain Engines**: Build core domain aggregates for sales finalization, price calculation, and stock ledger movements in `sitolo-domain`.
6. **Add Database Health Check to `/process/ready`**: Ensure readiness probe pings PostgreSQL before returning HTTP 200 OK.
7. **Enforce TLS / HTTPS Ingress**: Require TLS termination at ingress or bind HTTPS using Rustls in Axum.
8. **Wire OpenTelemetry & Prometheus Exporters**: Add metrics endpoints (`/metrics`) and trace context propagation across HTTP headers and worker jobs.
9. **Implement MRA EIS Regulatory Tax Integration**: Build fiscal signature and submission queues in `sitolo-integrations`.
10. **Automate End-to-End Integration Tests in CI**: Run live PostgreSQL integration tests (`rls_security_tests.rs`) in GitHub Actions workflows.

---

# 6. Phased Remediation Plan

### Phase 1: Immediate Remediation (Weeks 1–2) — Security & Transport Foundations
- Replace custom TCP loop in `apps/api/src/serve.rs` with Axum framework.
- Enforce authentication middleware across all `/v1/*` routes using `sitolo-auth`.
- Connect `apps/api` to `PgAuthorityPools` in PostgreSQL; deprecate in-memory persistence in production.
- Implement real database connectivity checks in `/process/ready`.

### Phase 2: Short-Term Remediation (Weeks 3–6) — Core Business Engines
- Implement Product Catalogue (`Phase 8`) and Inventory Ledger (`Phase 9`) in `sitolo-domain`.
- Implement POS / Sales engine (`Phase 10`) with immutable sales history and total validation.
- Implement outbox processing loop in `apps/worker/src/main.rs` with DLQ handling.

### Phase 3: Medium-Term Remediation (Weeks 7–12) — Integrations & Offline Sync
- Implement Payment & Mobile Money reconciliation adapters in `sitolo-integrations` (`Phase 11`).
- Implement MRA EIS fiscal tax submission integration (`Phase 15`).
- Implement offline synchronization engine in `sitolo-sync` (`Phase 12`).

### Phase 4: Long-Term Remediation (Weeks 13–20) — Hardening & Certification
- Conduct full load testing, chaos testing, and disaster recovery drills (`Phase 19`).
- Complete production security certification and compliance audit (`Phase 20`).

---

# 7. Enterprise Acceptability Criteria

| Domain | Acceptable Enterprise Practice | Current Unacceptable Practice in Codebase |
|---|---|---|
| **HTTP Transport** | Industry-standard HTTP stack (Axum/Hyper) with TLS, CORS, and streaming bodies | Custom raw TCP socket reading fixed 32 KB buffers without HTTP header parsing |
| **Authentication** | Server-enforced JWT/Session validation on every protected route | Unauthenticated API handlers executing tenant mutation commands directly |
| **Data Persistence** | PostgreSQL relational store with RLS, transactions, and FK constraints | Volatile in-memory HashMaps (`Mutex`/`RwLock`) resetting state on restart |
| **Async Processing** | Transactional outbox pattern polled by dedicated worker processes | Empty worker binary `fn main() {}` and unexecuted outbox records |
| **Domain Logic** | Isolated domain crate containing business aggregates and invariant checks | Missing domain engines for Sales, Inventory, Payments, and Catalogue |
| **Observability** | Prometheus metrics, OpenTelemetry distributed tracing, and health checks | Probe returning fake 200 OK without pinging DB; missing metrics exporter |
