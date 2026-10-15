# Sitolo Codebase Enterprise Audit & Architecture Review

**Target System:** Sitolo — Business Operating System for African SMEs
**Audit Date:** 2026-10-15
**Source Baseline:** `main`
**Status:** Canonical Enterprise Audit & Architecture Review Report
**Authority:** Governing documentation hierarchy in `agent.md` & repository source tree

---

## Executive Summary

Sitolo is designed as a security-first modular monolith built in Rust (Axum, Tokio, SQLx) for retail and SME business management across African markets. The repository demonstrates exceptional cryptographic discipline, formal tenant/scope abstractions, strict dependency rules (`deny.toml`), and fail-closed security design.

However, **Sitolo is NOT production-ready**.

While core security primitives, identity structures, and configuration frameworks are robust, critical gaps exist between architectural specifications and current runtime implementations. Core business domain engines (catalogue, inventory ledger, POS sales, payments reconciliation, offline sync) remain contractual specs or early scaffolds. Furthermore, key architectural transport mismatches, missing transactional background worker loops, and missing runtime database migrations mean the system cannot currently process real-world production transactions at scale.

```text
CURRENT ARCHITECTURAL MATURITY POSTURE:

  [ Cryptographic & Security Primitives ]  -> STABLE & PRODUCTION-GRADE
  [ Tenancy & Effective Scope Resolver ]  -> STABLE & PRODUCTION-GRADE
  [ HTTP Transport Layer (apps/api)    ]  -> DIVERGENT (Custom TCP loop vs Axum Spec)
  [ PostgreSQL Runtime Authority       ]  -> PARTIAL (Primitives exist; RLS / Migrations incomplete)
  [ Core Business Engines (Sales, Ledger)] -> MISSING / SCENARIO CONTRACTS
  [ Worker & Async Queue System        ]  -> SCAFFOLD ONLY (No execution loop)
  [ External Integrations (MRA/EIS, Pay)] -> CONTRACT ONLY
```

### Health Scorecard

| Dimension | Rating | Status Summary |
|---|---|---|
| **Architecture & Module Boundaries** | ⚠️ Needs Work | Clean Rust workspace separation; critical divergence between HTTP transport and Axum specification. |
| **Security & Cryptography** | ✅ Enterprise Grade | Argon2id, AES-256-GCM, zeroize, CSPRNG, fail-closed scope resolution, zero raw secret logging. |
| **Authentication & Authorization** | ⚠️ Needs Work | Strong session/MFA primitives; authorization enforcement not hooked into all HTTP handlers. |
| **Database & Persistence** | ⚠️ Needs Work | RLS policies and dual-pool authority implemented; application migration pipeline unfinished. |
| **Async, Concurrency & Workers** | 🔴 Unacceptable | `apps/worker` is a scaffold; zero durable background execution, retry handling, or outbox polling. |
| **Observability & Diagnostics** | ⚠️ Needs Work | Structured tracing and bounded buffers implemented; lacks production OTLP exporters & alert gates. |
| **Domain Engine Completeness** | 🔴 Unacceptable | Catalogue, POS, inventory, payments, MRA/EIS integrations exist only as specifications/scaffolds. |
| **Testing & Verification** | ⚠️ Needs Work | High-quality security unit/integration unit tests; zero integration tests for database without active Postgres container. |

---

## Detailed End-to-End Operational & Architectural Reviews

### 1. Repository Structure & Dependency Boundaries
The workspace uses Rust workspaces (`apps/` and `crates/`). Workspace dependencies are strictly constrained via `Cargo.toml` and verified by `./scripts/ci/check-architecture`. `crates/sitolo-domain` enforces zero external persistence or transport dependencies. `crates/sitolo-security` enforces `#![forbid(unsafe_code)]`.
- **Strengths:** Unsafe code is forbidden across security and domain crates. Clean acyclic dependency tree.
- **Weaknesses:** Direct circular or improper crate linkages are prevented by CI, but `apps/api` duplicates transport serialization logic that should be unified within API DTO crates.

### 2. Application Architecture & Module Separation
The system follows a DDD-inspired modular monolith pattern (`sitolo-domain`, `sitolo-application`, `sitolo-persistence`, `sitolo-api`).
- **Strengths:** Clean separation between domain entity definitions, application orchestration services, and persistence primitives.
- **Weaknesses:** The application layer (`sitolo-application`) currently orchestrates identity/IAM, but lacks command/query handler patterns for domain events or high-throughput retail operations.

### 3. API Design, Request Flow & Trust Boundaries
API contracts specify JSON over HTTPS with strict header-based or bearer token authentication.
- **Strengths:** Explicit, bounded DTOs using `serde` with `#[serde(deny_unknown_fields)]` preventing parameter pollution.
- **Weaknesses:** `apps/api/src/serve.rs` currently implements a manual, custom HTTP parsing loop over raw Tokio TCP streams rather than standard, enterprise-hardened Axum routers.

### 4. Authentication, Authorization & Session Handling
`sitolo-auth` provides session tokens, refresh tokens with replay detection, MFA TOTP verification, and device binding. `sitolo-tenancy` handles `AuthorizedScope` with requested-vs-trusted scope separation.
- **Strengths:** Security versions prevent stale session usage; scope widening is structurally impossible in `AuthorizedScope`.
- **Weaknesses:** Role-Based Access Control (RBAC) and scope enforcement are verified in isolation unit tests, but are not universally applied across all API endpoints in `apps/api`.

### 5. Input Validation, Sanitization & Data Integrity
Data inputs leverage strong domain primitive types (e.g., `TenantId`, `BranchId`, string wrappers with max length limits).
- **Strengths:** Type-safe domain wrappers eliminate primitive obsession.
- **Weaknesses:** Streamed upload validation or bulk payload boundary limits (e.g., catalog imports) are not yet enforced at the transport proxy level.

### 6. Injection Risks, Memory Safety & Boundary Defaults
- **Injection:** SQL injection risk is eliminated by standard SQLx parameterized queries; raw query formatting is forbidden.
- **XSS & CSRF:** API endpoints return standard JSON without raw HTML rendering; CORS headers are configurable.
- **SSRF & Path Traversal:** File paths are not accepted directly from web input; external integration URLs are pinned in configuration.
- **Secret Leakage:** `sitolo-security` defines `ProtectedSecret` and `RedactedText` which zeroize memory on drop and mask sensitive strings in logs.

### 7. Error Handling, Retry Behavior & Timeout Strategy
- **Strengths:** Domain and API errors map to standardized, error-coded responses (e.g., `SECTION_30` error taxonomy).
- **Weaknesses:** Outbound HTTP clients and internal database operations lack circuit breakers, exponential backoff retries, and jitter policies.

### 8. CPU-Bound vs I/O-Bound Bottlenecks
Password hashing (`argon2`) is CPU-heavy and correctly uses `tokio::task::spawn_blocking` to prevent starving the Tokio async reactor. I/O operations (database, network) use Tokio primitives.

### 9. Async Behavior, Blocking Operations & Concurrency Hazards
- **Strengths:** Async operations strictly use Tokio primitives.
- **Weaknesses:** In-memory repositories in `sitolo-persistence` rely on `tokio::sync::RwLock` for state management, which is unsuitable for multi-replica horizontal scaling.

### 10. Database Design, Query Efficiency, Transactions & Migrations
PostgreSQL schema specifications define Row-Level Security (RLS) using session variables (`app.organization_id`, `app.branch_id`).
- **Strengths:** `PgAuthorityPools` separates admin/migration connections from least-privileged runtime connections (`app_runtime`).
- **Weaknesses:** Migration scripts and RLS enforcement are incomplete in production startup paths; database-backed tests panic fail-closed if `ADMIN_DATABASE_URL` is missing.

### 11. Caching Strategy & Consistency Tradeoffs
No distributed cache (e.g., Redis) is currently integrated into domain request flows. All reads query state directly, which guarantees consistency but risks DB degradation under high sales traffic.

### 12. Queueing, Background Jobs, Event Handling & Idempotency
- **Strengths:** Domain event schemas and audit log entry formats are strictly specified.
- **Weaknesses:** `apps/worker` is an empty scaffold. There is no transactional outbox poller, no queue consumer (NATS/RabbitMQ/SQS), and no idempotency token enforcement for background retries.

### 13. Observability: Logs, Metrics, Traces & Incident Diagnosability
`sitolo-observability` provides structured tracing with bounded memory buffers to prevent OOMs under log floods.
- **Strengths:** Prevents trace ID injection and redacts secrets from telemetry output.
- **Weaknesses:** Operational endpoints (`/healthz`, `/readyz`, `/metrics`) return static mock JSON in `apps/api/src/serve.rs` rather than real subsystem health checks.

### 14. Test Coverage, Test Quality & Edge Cases
Unit tests covering security, config, tenancy, and auth are thorough.
- **Weaknesses:** Database tests in `sitolo-persistence` fail when PostgreSQL environment variables are absent, preventing automated CI test execution without specialized test infrastructure. End-to-end integration test coverage is missing for business workflows.

### 15. Configuration Management & Secrets Handling
`sitolo-config` uses `dotenvy` and environment parsing with strict SHA-256 lower-hex fingerprinting.
- **Strengths:** Hardcoded production defaults are rejected; secret values never appear in config fingerprints or `Debug` prints.

### 16. Deployment Safety, Rollback Readiness & Release Discipline
CI pipelines enforce `cargo fmt`, `cargo clippy`, `cargo deny`, `cargo audit`, and architecture constraints.
- **Weaknesses:** Lack of zero-downtime database migration strategy and zero automated blue-green/canary release manifests.

### 17. Code Quality, Naming, Duplication & Technical Debt
Code is well-structured and idiomatic Rust. Technical debt is concentrated in stubbed or scaffolded modules (`apps/worker`, `sitolo-events`, `sitolo-integrations`, `sitolo-sync`).

### 18. Maintainability Under Team Growth & Code Ownership
Modular crate architecture enables clean team ownership boundaries across auth, domain, API, and persistence crates.

### 19. Compliance & Regulatory Expectations
`docs/mra_eis_integration_spec.md` details tax clearance compliance (MRA EIS for Mauritius). However, actual tax signing, certificate handling, and payload generation are unexecuted specifications.

---

## Top 10 Highest-Risk Issues

| Rank | Issue ID | Title | Risk Category | Failure Impact |
|---|---|---|---|---|
| 1 | **CRIT-01** | Custom TCP HTTP Loop in `apps/api` | Architecture / Security | Request parsing bugs, Denial of Service (DoS), lack of standard HTTP middleware protection. |
| 2 | **CRIT-02** | Scaffolded Worker Process (`apps/worker`) | Reliability / Async | Unprocessed outbox events, failed async job execution, zero background transaction processing. |
| 3 | **CRIT-03** | In-Memory Repository Fallbacks | Data Integrity | Total state loss upon container restart; non-functional horizontally scaled deployments. |
| 4 | **HIGH-01** | Unfinished Production Database Migrations | Persistence | Database queries fail in production due to missing tables, columns, or RLS policies. |
| 5 | **HIGH-02** | Incomplete Authorization Middleware Wiring | Access Control | API endpoints accessible without verifying active permissions or tenant context. |
| 6 | **HIGH-03** | CI Pipeline Fails DB Integration Tests Without Postgres | Test Governance | Integration tests panic when running standard `cargo test` without active DB environment. |
| 7 | **HIGH-04** | Static Health & Readiness Probe Responses | Observability | Load balancer routes traffic to unhealthy pods with broken database or network connections. |
| 8 | **MED-01** | Unimplemented External Integrations (MRA/EIS, Pay) | Compliance / Domain | Inability to process payments or issue fiscalized receipts in regulated markets. |
| 9 | **MED-02** | Lack of Circuit Breakers & Retry Policies | Fault Tolerance | Cascading failures and connection starvation during external service outages. |
| 10 | **MED-03** | Missing Distributed Caching Layer | Performance | Excessive database CPU load during high-volume point-of-sale activity. |

---

## Top 10 Highest-Leverage Fixes

| Rank | Fix Strategy | Target Area | Engineering & Business ROI |
|---|---|---|---|
| 1 | **Reconcile HTTP Layer with Axum Framework** | `apps/api` | Restores spec compliance, enables production-grade routing, timeout, CORS, and body size limits. |
| 2 | **Implement Transactional Outbox & Worker Loop** | `apps/worker`, `sitolo-events` | Guarantees event delivery, background job processing, and async reliability across services. |
| 3 | **Enforce Production SQLx Database Persistence** | `sitolo-persistence` | Replaces in-memory state with durable PostgreSQL schema, migrations, and RLS enforcement. |
| 4 | **Wire Standard Tenancy & Auth Middleware** | `sitolo-api` | Guarantees zero unauthenticated or unauthorized requests bypass scope checks on any route. |
| 5 | **Inject Subsystem Health Checks into Probes** | `apps/api/src/serve.rs` | Provides true cluster diagnosability and prevents traffic routing to failing application nodes. |
| 6 | **Implement DB Container Test Fixture / Docker Compose in CI** | `.github/workflows`, `sitolo-persistence` | Enables full database and RLS verification in standard CI pipeline without failing closed. |
| 7 | **Implement POS Sales & Inventory Domain Engines** | `sitolo-domain`, `sitolo-application` | Unlocks core business functionality for retail SMEs (sales transactions, stock tracking). |
| 8 | **Add Circuit Breaker & Retry Infrastructure** | `sitolo-integrations` | Prevents upstream provider degradation from crashing system APIs. |
| 9 | **Implement Cryptographic MRA/EIS Fiscal Invoicing** | `sitolo-integrations` | Achieves legal compliance for retail tax reporting in targeted African jurisdictions. |
| 10 | **Integrate OpenTelemetry Exporters** | `sitolo-observability` | Streamlines APM tracing, log aggregation, and real-time operational alerting. |

---

## Detailed Severity-Ranked Findings

### Critical Severity Findings

#### Finding CRIT-01: Custom TCP Listener Loop Replaces Axum Framework in `apps/api`
- **What is wrong:** `apps/api/src/serve.rs` parses HTTP requests manually over `tokio::net::TcpListener` instead of leveraging Axum routing framework.
- **Why it is wrong:** Manual HTTP parsing exposes the server to HTTP request smuggling, header injection, buffer overflows, and slowloris attacks. It contradicts `docs/system_architecture_design.md`.
- **Where it appears:** `apps/api/src/serve.rs`, `apps/api/Cargo.toml`.
- **How it fails in production:** Malformed client HTTP headers panic the connection loop or crash request handling, leading to widespread API service outage.
- **What a proper fix looks like:** Replace manual TCP stream parsing with Axum `Router`, attaching middleware for timeouts, body limits, compression, and request tracing.
- **Priority & Blast Radius:** Priority P0. Blast Radius: Entire API Gateway & all incoming client traffic.

#### Finding CRIT-02: Worker Process (`apps/worker`) is an Empty Scaffold
- **What is wrong:** `apps/worker/src/main.rs` contains only a placeholder print statement and exits immediately.
- **Why it is wrong:** Asynchronous tasks (outbox event delivery, payment reconciliation, audit archiving, receipt generation) cannot execute.
- **Where it appears:** `apps/worker/src/main.rs`, `crates/sitolo-events/src/lib.rs`.
- **How it fails in production:** Domain events remain unprocessed in outbox tables forever, causing third-party integrations and async workflows to stall indefinitely.
- **What a proper fix looks like:** Implement a polling/listening background worker loop with Tokio, consuming transactional outbox entries with exponential backoff and dead-letter queues.
- **Priority & Blast Radius:** Priority P0. Blast Radius: All asynchronous processing and background job orchestration.

#### Finding CRIT-03: In-Memory Domain Persistence Default
- **What is wrong:** Service state in `sitolo-persistence` defaults to in-memory `HashMap` primitives protected by `RwLock`.
- **Why it is wrong:** Multi-instance deployments cannot share state, and restarting a container wipes all system data.
- **Where it appears:** `crates/sitolo-persistence/src/memory.rs`.
- **How it fails in production:** Container restarts lead to catastrophic data loss; load balancers route requests to nodes with divergent state views.
- **What a proper fix looks like:** Wire SQLx PostgreSQL repositories as the default persistent implementation for all production environments.
- **Priority & Blast Radius:** Priority P0. Blast Radius: Global data persistence and multi-node transaction consistency.

---

### High Severity Findings

#### Finding HIGH-01: PostgreSQL Migrations & RLS Policies Unapplied in Application Startup
- **What is wrong:** SQL schema migrations and Row-Level Security definitions are not executed during application boot or CI database setup.
- **Why it is wrong:** Standard runtime queries against PostgreSQL fail because required tables, functions, or RLS policies (`app.organization_id`) are missing.
- **Where it appears:** `crates/sitolo-persistence/src/postgres.rs`, `apps/api/src/bootstrap.rs`.
- **How it fails in production:** Database queries fail with missing relation errors or RLS authorization rejections upon initial query execution.
- **What a proper fix looks like:** Embed SQLx migrations (`sqlx::migrate!()`) in application startup or dedicated pre-deployment migration jobs.
- **Priority & Blast Radius:** Priority P1. Blast Radius: Database transaction layer.

#### Finding HIGH-02: Missing Authorization Middleware on HTTP Endpoints
- **What is wrong:** Scope checking (`AuthorizedScope`) is implemented in unit tests but not wired into all HTTP handler paths in `sitolo-api`.
- **Why it is wrong:** Endpoints can process requests without verifying if the caller possesses the necessary permissions or branch scope.
- **Where it appears:** `crates/sitolo-api/src/lib.rs`, `apps/api/src/serve.rs`.
- **How it fails in production:** Insecure Direct Object Reference (IDOR) vulnerabilities allow cross-tenant data access if handler logic forgets explicit scope validation.
- **What a proper fix looks like:** Implement Axum extractor/middleware enforcing `AuthorizedScope` validation on every incoming HTTP request.
- **Priority & Blast Radius:** Priority P1. Blast Radius: API authorization boundary.

#### Finding HIGH-03: RLS Security Tests Panic When PostgreSQL Environment is Absent
- **What is wrong:** `cargo test -p sitolo-persistence --test rls_security_tests` panics fail-closed if `ADMIN_DATABASE_URL` is missing.
- **Why it is wrong:** Developer local test runs and default CI jobs fail unless a live PostgreSQL instance is pre-configured and supplied.
- **Where it appears:** `crates/sitolo-persistence/tests/rls_security_tests.rs`.
- **How it fails in production:** CI build pipelines break or skip database security verification during rapid developer commits.
- **What a proper fix looks like:** Add test guards that gracefully skip DB-dependent tests or auto-provision ephemeral Postgres instances via `testcontainers-rs` when environment variables are absent.
- **Priority & Blast Radius:** Priority P1. Blast Radius: Automated security test harness.

#### Finding HIGH-04: Static Mock Responses for Operational Health Probes
- **What is wrong:** Probes (`/healthz`, `/readyz`) return static HTTP 200 responses without checking database connection pools or internal queue status.
- **Why it is wrong:** Kubernetes or load balancers consider the pod healthy even when database connections are completely dead.
- **Where it appears:** `apps/api/src/serve.rs`.
- **How it fails in production:** Orchestrators route user traffic to broken app instances that fail on every database transaction.
- **What a proper fix looks like:** Implement dynamic health probes that execute `ping()` against database pools and internal worker channels before returning HTTP 200.
- **Priority & Blast Radius:** Priority P1. Blast Radius: Infrastructure orchestration and availability.

---

### Medium Severity Findings

#### Finding MED-01: External Integration Specs (MRA/EIS, Mobile Money) Unimplemented
- **What is wrong:** Fiscalization tax reporting and payment gateway code exists only as specs and structural traits.
- **Why it is wrong:** Merchant transactions cannot be compliance-signed or processed via mobile money in target regions.
- **Where it appears:** `crates/sitolo-integrations/src/lib.rs`.
- **How it fails in production:** Payments fail at point-of-sale checkout, and sales receipts violate local tax legislation.
- **What a proper fix looks like:** Implement production clients for tax authority endpoints and regional payment gateways with retry mechanisms.
- **Priority & Blast Radius:** Priority P2. Blast Radius: Merchant billing and regulatory compliance.

#### Finding MED-02: Missing Circuit Breakers & Outbound HTTP Timeouts
- **What is wrong:** HTTP integration calls lack circuit breaker logic and explicit timeout enforcement.
- **Why it is wrong:** Third-party API latencies consume Tokio threadpool tasks, leading to resource exhaustion.
- **Where it appears:** `crates/sitolo-integrations`.
- **How it fails in production:** Slow third-party responses cause API gateway worker thread starvation, locking up the entire web application.
- **What a proper fix looks like:** Wrap external clients with `tower::buffer`, `tower::limit`, and circuit breakers (`failsafe` or `tower-circuitbreaker`).
- **Priority & Blast Radius:** Priority P2. Blast Radius: External communications and worker pool availability.

#### Finding MED-03: Lack of Distributed Cache Infrastructure
- **What is wrong:** High-frequency read queries (product lookup, price checks, permissions) directly hit PostgreSQL.
- **Why it is wrong:** High POS check-out traffic creates heavy read pressure on primary database nodes.
- **Where it appears:** `crates/sitolo-persistence`.
- **How it fails in production:** Read query spikes cause database connection pool exhaustion during peak shopping hours.
- **What a proper fix looks like:** Integrate an optional Redis/Valkey cache layer for immutable reference data with cache invalidation on domain events.
- **Priority & Blast Radius:** Priority P2. Blast Radius: Database performance under heavy load.

---

### Low Severity Findings

#### Finding LOW-01: Incomplete Logging Context on Unhandled Errors
- **What is wrong:** Certain API error branches return generic internal error codes without logging underlying cause context to telemetry buffers.
- **Why it is wrong:** Incident triage requires manual code trace analysis due to missing internal error details in logs.
- **Where it appears:** `crates/sitolo-api/src/errors.rs`.
- **How it fails in production:** Debugging production exceptions takes longer due to sparse contextual diagnostic logs.
- **What a proper fix looks like:** Ensure all error conversions capture `tracing::error!` event logs with source error chains prior to returning generic client responses.
- **Priority & Blast Radius:** Priority P3. Blast Radius: Operational diagnosability and debugging.

#### Finding LOW-02: Workspace Documentation History Ambiguity
- **What is wrong:** Historical audit documents in `docs/` describe previous repo states without clear deprecation headers.
- **Why it is wrong:** New developers or AI agents can mistake outdated design notes for current architecture contracts.
- **Where it appears:** `docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md`.
- **How it fails in production:** Engineering time is wasted addressing deprecated or historical issues.
- **What a proper fix looks like:** Annotate historical documents with clear status banners (`HISTORICAL - ARCHIVED`).
- **Priority & Blast Radius:** Priority P3. Blast Radius: Developer onboarding and documentation maintainability.

---

## Phased Remediation Roadmap

```text
+-----------------------------------------------------------------------------------+
| PHASE 1: IMMEDIATE HOTFIXES & FOUNDATION (Weeks 1-2)                              |
| - Reconcile apps/api HTTP transport with standard Axum framework.                 |
| - Wire PostgreSQL SQLx migrations into application startup.                       |
| - Fix database test runner in CI with testcontainers or graceful fallback.        |
+-----------------------------------------------------------------------------------+
                                         |
                                         v
+-----------------------------------------------------------------------------------+
| PHASE 2: CORE PERSISTENCE & WORKER ENGINE (Weeks 3-4)                              |
| - Replace in-memory default repositories with PostgreSQL persistence.             |
| - Implement durable background worker loop and transactional outbox poller.       |
| - Wire dynamic health checks (/healthz, /readyz) into DB pool status.             |
+-----------------------------------------------------------------------------------+
                                         |
                                         v
+-----------------------------------------------------------------------------------+
| PHASE 3: BUSINESS DOMAIN & INTEGRATIONS EXECUTION (Weeks 5-8)                     |
| - Implement POS sales, catalogue, and inventory domain transaction engines.      |
| - Implement MRA/EIS fiscal receipt signing and payment gateway clients.          |
| - Attach circuit breakers and timeout policies to outbound HTTP services.         |
+-----------------------------------------------------------------------------------+
                                         |
                                         v
+-----------------------------------------------------------------------------------+
| PHASE 4: ENTERPRISE HARDENING & SCALE (Weeks 9-12)                                |
| - Deploy Redis distributed caching for catalog and token reference data.          |
| - Export OpenTelemetry metrics and trace signals to production APM.               |
| - Conduct full disaster recovery and end-to-end chaos load testing.               |
+-----------------------------------------------------------------------------------+
```

---

## Enterprise Acceptability Matrix

| Architectural Subsystem | Acceptable Enterprise Baseline Pattern | Current Codebase Anti-Pattern / Unacceptable State |
|---|---|---|
| **HTTP Web Transport** | Standard production framework (Axum) with timeouts, CORS, and body size limits. | Custom TCP parsing loop in `apps/api/src/serve.rs`. |
| **Data Persistence** | Relational DB with strict migrations, RLS policies, and transactional integrity. | In-memory `HashMap` fallbacks in `sitolo-persistence`. |
| **Async Background Work** | Transactional outbox with idempotent worker polling and dead-letter queues. | Empty scaffold in `apps/worker/src/main.rs`. |
| **Authentication & Scope** | Server-authoritative `AuthorizedScope` enforced on 100% of HTTP endpoints. | Authorization logic present in unit tests but missing from HTTP handlers. |
| **Operational Health** | Dynamic probes verifying database, queue, and network dependencies. | Static HTTP 200 JSON responses in `/healthz` and `/readyz`. |
| **Third-Party Integrations** | Bounded requests with circuit breakers, retries, and jitter policies. | Direct unbuffered API calls lacking circuit breaker protection. |
| **Secret Management** | Zeroized memory references (`ProtectedSecret`), masked logging (`RedactedText`). | **Acceptable** — Full cryptographic discipline implemented in `sitolo-security`. |
| **Configuration Governance** | Deterministic SHA-256 fingerprinting, rejection of unpinned defaults. | **Acceptable** — Full configuration contracts enforced in `sitolo-config`. |

---

## Conclusion & Production Readiness Statement

Sitolo contains an exceptional security, identity, and cryptographic foundation. However, **it cannot be certified for production deployment today**.

Executing the Phased Remediation Roadmap—specifically transitioning to Axum HTTP transport, connecting production PostgreSQL state with SQLx, implementing the background worker outbox execution loop, and completing core business domain engines—will elevate Sitolo to a fully production-ready, enterprise-grade business operating system.
