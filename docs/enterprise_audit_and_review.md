# Sitolo Codebase Enterprise Audit & Architecture Review

**Target System:** Sitolo — Business Operating System for African SMEs
**Audit Date:** 2026-09-25
**Source Baseline:** `main`
**Status:** Canonical Enterprise-Grade Audit & Architecture Review
**Target Audience:** CTO, Principal Architects, Security Lead, Engineering Teams

---

## Executive Summary

Sitolo is an ambitious modular monolith written in Rust, designed as a multi-tenant business operating system for African Small and Medium Enterprises (SMEs).

An end-to-end audit of the codebase reveals that **Sitolo possesses a strong architectural design and security foundation, but is currently NOT enterprise production-ready**.

### Overall Codebase Health

| Health Tier | Status | Assessment |
|---|---|---|
| **Architectural Foundation** | **High Quality** | Rust workspace, crate boundaries, dependency governance, and type-level tenancy models are exceptionally well-structured. |
| **Security & Auth Infrastructure** | **Strong Substrate** | Cryptographic token handling, session lifecycles, password hashing contracts, and PostgreSQL Row Level Security (RLS) policies are mathematically sound. |
| **HTTP Transport & API Boundary** | **Critical Gap** | Current HTTP server (`apps/api`) uses an ad-hoc TCP listener with fragile buffer reading, lacks authentication middleware, and diverges from Axum specifications. |
| **Business Domain Engines** | **Unimplemented** | Core domain crates for product catalogue, inventory ledger, POS sales, payments, reconciliation, and tax compliance (MRA EIS) remain scaffolds or future specifications. |
| **Asynchronous & Integration Execution** | **Unimplemented** | Worker binary (`apps/worker`) is an empty scaffold (`fn main() {}`). Outbox event consumers, background jobs, payment webhooks, and offline sync are absent. |

**Final Verdict:** The codebase is suitable as a secure platform foundation (Phase 1–4), but cannot survive real production traffic, real business operations, untrusted network traffic, or adversarial attackers in its current state.

---

## System Assessment Across 19 Review Dimensions

### 1. Repository Structure & Dependency Boundaries
- **Current State:** Well-organized Cargo workspace using 15 crates under `crates/` and 2 applications under `apps/`. Unsafe code is forbidden via `#![forbid(unsafe_code)]` across all crates.
- **Defect:** `apps/api` implements an ad-hoc custom TCP server loop rather than using Axum, creating a direct conflict with architecture specifications (ADR-001, `system_architecture_design.md`).

### 2. Application Architecture & Module Separation
- **Current State:** Clean layered separation (`sitolo-domain` -> `sitolo-application` -> `sitolo-api`). Dependency graph strictly flows inwards.
- **Defect:** Layering is only populated for Tenancy and IAM. Business logic for retail operations, stock movements, and financial transactions is completely absent from `sitolo-domain`.

### 3. API Design, Request Flow & Trust Boundaries
- **Current State:** DTOs in `sitolo-api` enforce strict JSON deserialization (`deny_unknown_fields`) and body size bounds.
- **Defect:** HTTP connection handler in `apps/api/src/serve.rs` reads raw TCP sockets in a single `stream.read()` call with a hardcoded 32KB buffer without HTTP framing parser, standard header processing, or Keep-Alive connection handling.

### 4. Authentication, Authorization & Session Handling
- **Current State:** Excellent domain models in `sitolo-auth` (Argon2id hashing contracts, session sliding expiration, refresh token family rotation, device binding, MFA TOTP/recovery codes) and `sitolo-authz` (role-based and scope-based permissions).
- **Defect:** HTTP routes in `apps/api/src/serve.rs` do NOT invoke authentication or authorization checks. API routes accept requests without Bearer tokens or session validation.

### 5. Input Validation, Sanitization & Data Integrity
- **Current State:** Value objects in `sitolo-domain` and `sitolo-tenancy` enforce non-empty string constraints, UUID formatting, and bounded string sizes.
- **Defect:** Validation occurs at the application boundary, but because HTTP request bodies are parsed from raw TCP chunks, partial JSON payloads cause `422 Unprocessable Entity` rather than proper malformed request errors.

### 6. Injection Risks, Security Defaults & Boundary Security
- **Current State:** PostgreSQL queries in `sitolo-persistence` use `sqlx` parameterized queries. RLS policies are CATALOG-verified (`verify_rls_catalog_metadata`).
- **Defect:** In-memory store fallback (`sitolo-persistence/src/memory.rs`) uses deterministic token generation (`*byte = (i as u8).wrapping_add(42)`). If misconfigured in staging/production, token secrets are completely predictable.

### 7. Error Handling, Retry Behavior & Timeout Strategy
- **Current State:** Bounded error types (`AppError`, `AuthError`, `PgAuthorityError`) with RFC 7807 `ProblemDetails` formatting.
- **Defect:** Outbound HTTP client retries, circuit breakers, and upstream timeout handlers do not exist because integration adapters (`sitolo-integrations`) are unimplemented.

### 8. CPU-Bound vs I/O-Bound Bottlenecks
- **Current State:** Blocking password hashing operations are isolated behind async traits.
- **Defect:** Single-threaded raw TCP loop in `apps/api/src/serve.rs` spawns Tokio tasks per connection without CPU isolation or connection pool throttling.

### 9. Async Behavior, Blocking Operations & Concurrency Hazards
- **Current State:** Reference in-memory database uses `std::sync::Mutex` across async methods, holding lock across memory operations.
- **Defect:** While lock hold times are brief, under high async concurrency, thread contention on `IdentityDatabase.state` will block the Tokio worker pool.

### 10. Database Design, Query Efficiency, Transactions & Migrations
- **Current State:** Dual connection pool architecture (`PgAuthorityPools`) separating administrative migrations from least-privileged runtime (`app_runtime`). Strict transaction-local GUCs (`set_transaction_tenant_context`).
- **Defect:** Business entity tables (products, inventory ledger, sales, payments) do not exist in database migrations or repository implementations.

### 11. Caching Strategy, Invalidation & Consistency
- **Current State:** Architectural documents designate Redis for session/cache acceleration.
- **Defect:** Zero caching code or invalidation logic is implemented in the repository.

### 12. Queueing, Background Jobs, Event Handling & Idempotency
- **Current State:** Audit event structures exist in `sitolo-audit`.
- **Defect:** `apps/worker` is `fn main() {}`. Outbox table polling, job queueing, event publishing, and consumer idempotency are absent.

### 13. Observability: Logs, Metrics, Traces & Diagnosability
- **Current State:** `sitolo-observability` contains structured event context, request ID tracking, and in-memory trace buffers.
- **Defect:** OTLP gRPC/HTTP collectors and Prometheus metrics exporters are not connected to application entrypoints. Log output falls back to stdout without log rotation or telemetry fan-out.

### 14. Test Coverage, Test Quality & Edge Cases
- **Current State:** Comprehensive unit tests for auth/tenancy domain state and real PostgreSQL RLS integration tests (`rls_security_tests.rs`).
- **Defect:** End-to-end HTTP integration tests, fault injection tests, network partitioning tests, and load tests are completely missing.

### 15. Configuration Management & Secrets Handling
- **Current State:** Highly robust configuration validator (`sitolo-config`) with schema versioning, strict environment separation (`Development`, `Staging`, `Production`), and secret references (`SecretRef`).
- **Defect:** Deployment manifests (K8s / Systemd) and production KMS secret provider implementations are not checked into the repository.

### 16. Deployment Safety, Rollback Readiness & Release Discipline
- **Current State:** Comprehensive CI script (`./scripts/ci/verify`) enforcing format, clippy lints, locked compilation, unit tests, and workspace checks.
- **Defect:** Automated database migration rollback strategies and zero-downtime deployment pipelines are not implemented.

### 17. Code Quality, Technical Debt & Coupling
- **Current State:** Idiomatic Rust code, descriptive domain types, zero `unsafe` code, low coupling.
- **Defect:** Stale/historical documentation and scaffold binaries create confusion regarding completed vs. contractual features.

### 18. Maintainability Under Team Growth & Code Ownership
- **Current State:** Clear workspace domain structure (`workspaces/sitolo-engineering`, `workspaces/sitolo-product`, etc.) and system map process cards.
- **Defect:** High refactoring cost will occur when transitioning from current TCP transport to Axum and when implementing missing domain engines.

### 19. Compliance & Enterprise Operational Expectations
- **Current State:** Designed to satisfy MRA EIS fiscal compliance and strict data privacy regulations.
- **Defect:** MRA EIS integration is unwritten; compliance verification cannot be performed without real fiscal signing adapters.

---

## Severity-Ranked Findings List

### CRITICAL SEVERITY (P0)

#### Finding FIND-CRIT-01: Ad-Hoc TCP Transport Loop & Divergence from Axum Architecture
- **What is wrong:** `apps/api/src/serve.rs` implements a custom TCP connection reader instead of an enterprise Axum HTTP framework router.
- **Why it is wrong:** The custom TCP reader uses `stream.read(&mut buf)` with a single 32KB buffer. It assumes entire HTTP requests arrive in a single TCP packet and header/body boundaries occur cleanly within 32KB.
- **Where it appears:** `apps/api/src/serve.rs` (`handle_connection`, `dispatch_request`).
- **How it fails in production:** Any request whose body is fragmented across multiple TCP packets (e.g. under standard mobile network latency or HTTP chunked transfer) results in `stream.read` returning headers with an incomplete or empty body. `serde_json::from_str("")` fails, returning `422 Unprocessable Entity` to legitimate clients. Large requests (>32KB) are cut off and rejected.
- **What a proper fix looks like:** Replace `apps/api/src/serve.rs` with standard `axum::Router` served via `axum::serve` on Tokio TCP listener, using standard middleware (`tower-http`) for timeout, tracing, request body framing, and CORS.
- **Priority:** P0
- **Blast Radius:** Total API transport unreliability and widespread false client errors across all endpoints.

#### Finding FIND-CRIT-02: Missing HTTP Authentication & Authorization Enforcement
- **What is wrong:** HTTP route dispatching in `apps/api/src/serve.rs` does not extract HTTP `Authorization` headers, validate session tokens, or enforce RBAC/ABAC authorization checks.
- **Why it is wrong:** Although `sitolo-auth` and `sitolo-authz` provide authentication and permission primitives, they are not wired into the HTTP request processing pipeline in `apps/api`.
- **Where it appears:** `apps/api/src/serve.rs` (`dispatch_request`).
- **How it fails in production:** Any unauthenticated HTTP client can issue `POST` requests to `/v1/organizations` or `/v1/organizations/{org_id}/suspend` and manipulate tenant states without providing any credentials or Bearer tokens.
- **What a proper fix looks like:** Implement Axum authentication middleware (`AuthExtractor` / `MiddlewareLayer`) that extracts Bearer tokens, resolves `SecurityContext` and `AuthorizedScope`, and rejects unauthenticated or unauthorized requests before dispatching to application handlers.
- **Priority:** P0
- **Blast Radius:** Complete breach of tenant security boundary and total bypass of IAM policies.

#### Finding FIND-CRIT-03: Absence of Core SME Business Domain Engines
- **What is wrong:** Core business domain engines (Product Catalogue, Inventory Ledger, POS Sales, Payments, Reconciliation, Offline Sync, MRA Fiscal EIS Invoicing) are completely missing from the codebase.
- **Why it is wrong:** `sitolo-domain` currently only contains `tenancy.rs`. All other business capabilities exist solely as specification documents in `docs/`.
- **Where it appears:** `crates/sitolo-domain`, `crates/sitolo-application`, `apps/api`.
- **How it fails in production:** The system cannot record a point-of-sale transaction, adjust stock levels, calculate taxes, process mobile money payments, or issue fiscal receipts.
- **What a proper fix looks like:** Implement domain aggregates, state machines, application services, PostgreSQL schemas, and API handlers for product catalogue, inventory ledger, sales, payments, and fiscal compliance.
- **Priority:** P0
- **Blast Radius:** Complete functional gap; system cannot perform any SME business operations.

#### Finding FIND-CRIT-04: Unimplemented Asynchronous Worker & Outbox Event Processing
- **What is wrong:** The background worker binary `apps/worker` contains only `fn main() {}`, and `sitolo-events`, `sitolo-integrations`, and `sitolo-sync` are empty shell crates.
- **Why it is wrong:** Transactional outbox events, fiscal receipt submissions to tax authorities, payment status polling, audit event persistence, and cross-device sync require asynchronous execution.
- **Where it appears:** `apps/worker/src/main.rs`, `crates/sitolo-events`, `crates/sitolo-integrations`, `crates/sitolo-sync`.
- **How it fails in production:** Outbox events written during database transactions remain unprocessed forever. Fiscal tax compliance reporting fails, payment webhook reconciliation stalls, and offline client syncing never occurs.
- **What a proper fix looks like:** Build a resilient worker polling engine in `apps/worker` that reads from `transactional_outbox`, processes events with idempotency keys, manages retries with exponential backoff, and routes dead-letter payloads to a DLQ table.
- **Priority:** P0
- **Blast Radius:** Asynchronous side effects, integrations, and compliance workflows fail entirely.

---

### HIGH SEVERITY (P1)

#### Finding FIND-HIGH-01: In-Memory Store Fallback & Deterministic Test Randomness
- **What is wrong:** `IdentityDatabase` in `sitolo-persistence/src/memory.rs` uses deterministic byte generation for token hashes (`*byte = (i as u8).wrapping_add(42)`).
- **Why it is wrong:** While labeled for test use, `IdentityDatabase` is compiled into the main persistence crate and served as a fallback store when PostgreSQL is not configured.
- **Where it appears:** `crates/sitolo-persistence/src/memory.rs` (`random_hex`).
- **How it fails in production:** If deployed with in-memory persistence in local, staging, or fallback environments, generated session tokens, refresh tokens, and MFA challenges are deterministic and easily guessable by attackers. Furthermore, all state is lost upon process restart.
- **What a proper fix looks like:** Require PostgreSQL as a mandatory requirement for non-test deployment environments (`Environment::Production` and `Environment::Staging`), and enforce OS CSPRNG (`sitolo_security::RandomSource`) for token generation across all memory implementations.
- **Priority:** P1
- **Blast Radius:** Potential session hijacking and total loss of state on process restart.

#### Finding FIND-HIGH-02: Absence of HTTP Transport Rate Limiting and DDoS Protection
- **What is wrong:** The HTTP server in `apps/api` lacks request rate limiting, connection throttling, and HTTP slowloris protection.
- **Why it is wrong:** Although `sitolo-auth/src/ratelimit.rs` defines token bucket logic, it is not wired into HTTP request handling.
- **Where it appears:** `apps/api/src/serve.rs`, `apps/api/src/main.rs`.
- **How it fails in production:** An attacker can flood endpoints with concurrent POST requests or keep TCP connections open indefinitely, consuming file descriptors and exhausting memory/CPU on the server.
- **What a proper fix looks like:** Attach `tower-governor` or custom Axum rate-limiting middleware to enforce IP-level and tenant-level request limits, along with TCP connection idle timeouts.
- **Priority:** P1
- **Blast Radius:** Denial of Service (DoS) and API service degradation under attack.

#### Finding FIND-HIGH-03: Incomplete RLS Context Propagation on Non-Transactional Data Paths
- **What is wrong:** PostgreSQL Row Level Security (RLS) enforcement relies on setting transaction-local GUC variables (`app.organization_id`, `app.branch_id`) via `set_transaction_tenant_context`.
- **Why it is wrong:** Database connection checkouts outside explicit tenant transaction blocks do not automatically clear or bind tenant GUCs.
- **Where it appears:** `crates/sitolo-persistence/src/postgres.rs`.
- **How it fails in production:** If a database connection is returned to the pool with dirty GUC state and subsequently reused by a query lacking explicit tenant context, queries might return cross-tenant data or execute under the wrong tenant context.
- **What a proper fix looks like:** Wrap all database operations in a strict transactional Unit of Work wrapper that executes `set_transaction_tenant_context` on checkout and issues `RESET ALL` or rolls back upon completion.
- **Priority:** P1
- **Blast Radius:** Cross-tenant data leakage or unauthorized data modification.

#### Finding FIND-HIGH-04: Lack of Distributed Caching and Offline Synchronization Engine
- **What is wrong:** Offline sync protocol (`sitolo-sync`) and distributed caching are completely unwritten.
- **Why it is wrong:** African SME POS environments frequently experience intermittent internet connectivity. Offline sales must be reconciled deterministically upon reconnection.
- **Where it appears:** `crates/sitolo-sync/src/lib.rs`.
- **How it fails in production:** POS terminals operating offline cannot sync sales or stock movements with the backend, leading to inventory discrepancies, duplicate transaction IDs, and negative stock balances.
- **What a proper fix looks like:** Implement a vector-clock / state-based offline sync engine in `sitolo-sync` with idempotency guarantees, Last-Write-Wins (LWW) / CRDT conflict resolution, and offline mutation queues.
- **Priority:** P1
- **Blast Radius:** Data inconsistency, financial loss, and POS operational failure during network outages.

---

### MEDIUM SEVERITY (P2)

#### Finding FIND-MED-01: Disconnected Observability & OTLP Telemetry Exporters
- **What is wrong:** `sitolo-observability` contains structured event context and ring buffers, but OTLP tracing/metrics exporters are not wired to `apps/api`.
- **Why it is wrong:** Traces and metrics are stored in temporary in-memory buffers without exporting to OpenTelemetry collectors or Prometheus endpoints.
- **Where it appears:** `crates/sitolo-observability/src/registry.rs`, `apps/api/src/main.rs`.
- **How it fails in production:** Production operational teams cannot inspect distributed request traces, track API latency percentiles (p95/p99), or receive real-time alerts on error spikes.
- **What a proper fix looks like:** Wire `tracing-opentelemetry` and `prometheus` metrics exporters during application bootstrap in `apps/api/src/bootstrap.rs`.
- **Priority:** P2
- **Blast Radius:** Reduced operational visibility and delayed incident response.

#### Finding FIND-MED-02: Absence of Business Integration & End-to-End HTTP Tests
- **What is wrong:** The test suite extensively covers unit-level identity/auth logic and catalog RLS metadata, but lacks end-to-end HTTP endpoint tests and negative edge-case integration tests.
- **Why it is wrong:** HTTP parsing, error serialization, header extraction, and database connection pool behavior are not tested under real network conditions.
- **Where it appears:** `apps/api/tests/`, `crates/sitolo-persistence/tests/`.
- **How it fails in production:** Regressions in request serialization, response problem details, CORS handling, or database transactions pass CI unnoticed.
- **What a proper fix looks like:** Add Playwright/HTTP integration tests in `apps/api/tests/` covering full request-response lifecycles, malformed payloads, and database disconnects.
- **Priority:** P2
- **Blast Radius:** Regression vulnerabilities during refactoring or release.

#### Finding FIND-MED-03: Architectural Specification & Source Code Divergence
- **What is wrong:** Architectural documents and specifications describe completed capabilities (e.g. Axum framework, completed phase implementations) that do not match current source code.
- **Why it is wrong:** Developers and automated coding agents reading documentation will make wrong assumptions regarding platform readiness and available APIs.
- **Where it appears:** `docs/` specifications vs `crates/` source code.
- **How it fails in production:** Development teams waste effort attempting to invoke non-existent endpoints or relying on unimplemented abstractions.
- **What a proper fix looks like:** Update documentation status registers (`docs/README.md`, `phase_contract_coverage_register.md`) to explicitly label features as `Implemented`, `Contract/Target`, or `Historical`.
- **Priority:** P2
- **Blast Radius:** Engineering drag and architectural confusion.

---

### LOW SEVERITY (P3)

#### Finding FIND-LOW-01: Dead Code Compiler Warnings in Non-Test Builds
- **What is wrong:** `sitolo-persistence` emits dead code warnings for `PgAuthorityError` and `PgAuthorityPools` when compiled without `test-support` feature flag.
- **Why it is wrong:** `postgres.rs` is conditionally compiled as `pub(crate)` during normal builds and `pub` during test builds.
- **Where it appears:** `crates/sitolo-persistence/src/postgres.rs`.
- **How it fails in production:** Clutters compiler output and CI logs, making it harder to notice real warnings.
- **What a proper fix looks like:** Clean up module export visibility and conditional compilation attributes in `sitolo-persistence/src/lib.rs`.
- **Priority:** P3
- **Blast Radius:** Developer experience and CI log cleanliness.

#### Finding FIND-LOW-02: Coarse Error Sub-Codes in ProblemDetails Output
- **What is wrong:** `format_error_response` maps `AppError` variants directly to generic HTTP status titles without fine-grained error sub-codes or field error lists.
- **Why it is wrong:** RFC 7807 problem details output does not specify which specific validation rule failed (e.g., string length vs regex pattern).
- **Where it appears:** `apps/api/src/serve.rs`.
- **How it fails in production:** Frontend mobile and web clients cannot display localized or field-specific validation error messages to users.
- **What a proper fix looks like:** Extend `ProblemDetails` to include an `invalid_params` array detailing field name and specific validation violation.
- **Priority:** P3
- **Blast Radius:** Frontend user experience friction.

---

## Top 10 Highest-Risk Issues

1. **Unauthenticated API Routes (FIND-CRIT-02):** Lack of HTTP auth middleware allows unauthorized tenant creation and manipulation.
2. **Fragile Custom TCP Transport (FIND-CRIT-01):** Ad-hoc 32KB TCP buffer reader fails under packet fragmentation and latency.
3. **Missing Business Domain Logic (FIND-CRIT-03):** Entire absence of catalogue, inventory, sales, and payment engines.
4. **Scaffold Background Worker (FIND-CRIT-04):** Empty worker binary leaves outbox events and background tasks permanently unexecuted.
5. **Deterministic Token Fallback (FIND-HIGH-01):** In-memory fallback uses deterministic byte generation for token hashing.
6. **No DoS / Rate-Limiting Protection (FIND-HIGH-02):** Unprotected TCP endpoints are vulnerable to request flooding and resource exhaustion.
7. **Potential RLS Context Leakage (FIND-HIGH-03):** Database connections reused without explicit tenant GUC cleanup risk cross-tenant leaks.
8. **Unwritten Offline Sync Protocol (FIND-HIGH-04):** POS terminals lose data integrity during network disconnects.
9. **Disconnected OTLP Observability (FIND-MED-01):** Absence of live metric and trace exports blocks incident diagnosis.
10. **Documentation/Implementation Drift (FIND-MED-03):** Contradictions between design specs and implementation create integration defects.

---

## Top 10 Highest-Leverage Fixes

1. **Migrate `apps/api` to Axum + Tokio Framework:** Eliminates TCP transport bugs, enables standard HTTP parsing, and provides middleware support.
2. **Implement Axum Bearer Auth & Tenant Scope Extractor:** Enforces authentication and authorized tenant isolation globally across all routes.
3. **Build Core Domain Engines (Catalogue, Inventory, POS):** Transforms the repository from an IAM scaffold into a working SME operating system.
4. **Implement Resilient Outbox Polling Worker Loop in `apps/worker`:** Processes transactional outbox records, background jobs, and provider webhooks reliably.
5. **Enforce Mandatory PostgreSQL Store for Non-Test Environments:** Prevents use of in-memory fallback stores and deterministic tokens in staging and production.
6. **Add Rate Limiting Middleware (`tower-governor`):** Protects API endpoints against brute-force attacks and DoS connection flooding.
7. **Implement Transactional Unit-of-Work Guard for RLS Context:** Guarantees `set_transaction_tenant_context` is executed on every database checkout.
8. **Develop `sitolo-sync` Offline Reconciliation Protocol:** Enables offline POS terminal operations with deterministic conflict resolution.
9. **Connect OpenTelemetry & Prometheus Exporters:** Provides real-time metrics, distributed tracing, and automated alerting.
10. **Reconcile Documentation with Source Baseline:** Aligns phase status registers and architecture contracts with actual code state.

---

## Phased Remediation Plan

```text
PHASE 0: Immediate Emergency Hardening (Week 1–2)
├── Replace custom TCP loop in apps/api with Axum + Tokio HTTP server
├── Add Axum Bearer token & AuthorizedScope authentication middleware
├── Enforce PostgreSQL requirement for non-test environments
└── Remove stale workspace artifacts and clean up compiler warnings

PHASE 1: Core Transport & Persistence Completeness (Week 3–6)
├── Implement Transactional Unit-of-Work wrapper for PostgreSQL RLS GUCs
├── Build background worker loop in apps/worker reading transactional_outbox
├── Wire tower-governor rate limiting and connection timeout middleware
└── Connect OpenTelemetry OTLP exporter and Prometheus metrics endpoint

PHASE 2: Business Domain Engine Implementation (Week 7–14)
├── Implement Product Catalogue aggregate, migrations, and API routes (Phase 8)
├── Implement Inventory Ledger, stock adjustments, and migrations (Phase 9)
├── Implement POS Sales engine, cart state machine, and receipts (Phase 10)
└── Implement Payments & Mobile Money reconciliation adapters (Phase 11)

PHASE 3: Offline Sync, Compliance & Production Certification (Week 15–20)
├── Develop sitolo-sync offline sync protocol and conflict resolution (Phase 12)
├── Build MRA EIS fiscal compliance signing adapter and outbox handler (Phase 15)
├── Develop full E2E HTTP integration test suite and load test harness
└── Conduct final production security certification and disaster recovery drill
```

---

## Acceptable vs. Non-Enterprise-Grade Capabilities

| Feature Area | Acceptable (Production-Grade) | Not Enterprise-Grade (Must Remediate) |
|---|---|---|
| **Repository & Governance** | Cargo workspace modularity, `#![forbid(unsafe_code)]` enforcement, toolchain pinning (Rust 1.98.1), CI verification pipeline. | Stale workspace output artifacts, doc/code status drift. |
| **Authentication & IAM** | Session lifecycles, password hashing contracts, refresh token rotation, device identity, role/scope permission models. | HTTP routes in `apps/api` accepting unauthenticated requests; lack of auth middleware. |
| **Database & Persistence** | PostgreSQL `PgAuthorityPools` dual-pool model, catalog-verified RLS isolation policies, transactional tenant GUC setting. | Missing business entity tables (products, inventory, sales); in-memory fallback using deterministic tokens. |
| **HTTP Transport** | Typed DTO validation (`deny_unknown_fields`), bounded body size limits. | Custom TCP listener with single `stream.read()` chunk reading; lack of Axum router framing. |
| **Asynchronous Processing** | Audit event catalogue definitions in `sitolo-audit`. | Worker binary (`apps/worker`) as `fn main() {}`; unwritten outbox worker loop. |
| **Integrations & Sync** | High-level spec contracts (`mra_eis_integration_spec.md`). | Shell crates (`sitolo-integrations`, `sitolo-sync`) with zero implementation code. |
