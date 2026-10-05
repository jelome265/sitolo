# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main`
**Status:** Comprehensive Enterprise Audit & Architecture Assessment
**Authority:** Governing documentation hierarchy in `agent.md` and repository source state

---

## Executive Summary & Overall Codebase Health

### Executive Summary

Sitolo is designed as a multi-tenant, offline-first business operating system for African small and medium enterprises (SMEs). A rigorous end-to-end enterprise audit of the codebase across architecture, security, reliability, scalability, performance, maintainability, observability, testing, release readiness, and operational risk reveals a system with **a strong security-first architectural foundation, but one that is currently NOT PRODUCTION-READY.**

The codebase demonstrates exemplary discipline in several foundational areas: a strict Rust workspace structure, prohibition of `unsafe` code, automated dependency governance, strict tenant-isolation models using PostgreSQL Row Level Security (RLS) with transaction-local context (`app.organization_id`, `app.branch_id`), cryptographic session primitives, bounded DTO validation, structured telemetry, and automated CI policy enforcement.

However, significant gaps prevent immediate production deployment. The system currently exists as an **incomplete platform foundation** where core architectural layers diverge from specified designs (e.g., custom HTTP TCP parser vs. specified Axum framework), core business engines (catalog, inventory, sales POS, payments, tax fiscalization) remain contract specifications rather than executed domain logic, async background workers and event outboxes are unintegrated scaffolds, and end-to-end integration under load is unverified.

### Overall System Health Matrix

```text
+-----------------------------------------------------------------------------------+
|                            SYSTEM HEALTH READINESS ASSESSMENT                     |
+------------------------------------+-----------------------+----------------------+
| Dimension                          | Rating                | Production Readiness |
+------------------------------------+-----------------------+----------------------+
| 1. Repository & Dependencies       | STRONG                | READY FOR DEVELOPMENT|
| 2. Application Architecture        | MODERATE              | PARTIAL              |
| 3. API Design & Trust Boundaries   | HIGH RISK DIVERGENCE  | NOT READY            |
| 4. AuthN, AuthZ & Session Handling | MODERATE              | PARTIAL              |
| 5. Input Validation & Data Integrity| MODERATE             | PARTIAL              |
| 6. Injection & Security Risks      | STRONG FOUNDATION     | PARTIAL              |
| 7. Errors, Retries & Isolation     | MODERATE              | NOT READY            |
| 8. CPU vs I/O Bottlenecks          | UNTESTED UNDER LOAD   | NOT READY            |
| 9. Async & Concurrency Safety      | HIGH RISK             | NOT READY            |
| 10. Database, RLS & Schema         | STRONG FOUNDATION     | PARTIAL              |
| 11. Caching & Invalidation         | NOT IMPLEMENTED       | NOT READY            |
| 12. Queueing, Jobs & Outbox        | UNIMPLEMENTED SCAFFOLD| NOT READY            |
| 13. Observability & Diagnosability | MODERATE              | PARTIAL              |
| 14. Test Coverage & Edge Cases     | MODERATE              | PARTIAL              |
| 15. Configuration & Secrets        | STRONG FOUNDATION     | READY FOR HARDENING  |
| 16. Deployment & Rollback Safety   | MODERATE              | NOT READY            |
| 17. Code Quality & Tech Debt       | STRONG                | MAINTAINABLE         |
| 18. Team Maintainability           | STRONG                | READY FOR SCALE      |
| 19. Compliance & Fiscal Governance | CONTRACT SPECIFIED    | NOT READY            |
+------------------------------------+-----------------------+----------------------+
```

Overall Readiness Verdict: **NOT PRODUCTION-READY (Phase-Gated Platform Scaffold)**.

---

## End-to-End System Evaluation (19 Operational Dimensions)

### 1. Repository Structure and Dependency Boundaries
- **Status:** Strong.
- **Evaluation:** The codebase is organized as a clean Rust workspace (`crates/` and `apps/`). Dependencies flow unidirectionally from core domain primitives (`sitolo-domain`, `sitolo-tenancy`, `sitolo-auth`) upwards to application services (`sitolo-application`), transport (`sitolo-api`), and binary entrypoints (`apps/api`, `apps/worker`). `deny.toml` strictly governs banned dependencies, duplicate crates, and license compliance.
- **Gaps:** Banned crates check must be continually enforced across workspace expansion to prevent transitive bloat or unauthorized async runtimes.

### 2. Application Architecture and Module Separation
- **Status:** Moderate / Partial.
- **Evaluation:** The application follows a modular monolith architecture. Domain primitives in `sitolo-domain` and service orchestration in `sitolo-application` maintain strict encapsulation. Aggregate roots govern state mutation rules.
- **Gaps:** Core business domain engines (POS sales, inventory ledger, supplier procurement, MRA EIS tax integration, billing) remain defined as implementation specifications in `docs/` rather than executable code in `sitolo-domain`.

### 3. API Design, Request Flow, and Trust Boundaries
- **Status:** High Risk Divergence / Not Production-Ready.
- **Evaluation:** DTOs strictly enforce `serde(deny_unknown_fields)` to prevent mass-assignment vulnerabilities. Trusted tenant context is separated from requested request parameters via `sitolo-tenancy`.
- **Gaps:** Governing documents (`system_architecture_design.md`, `api_contract.md`) mandate Axum + Tokio as the standard HTTP web framework. The binary `apps/api/src/serve.rs` currently implements a custom, bare TCP loop parsing raw HTTP strings manually. This custom HTTP transport lacks standard connection management, body buffering controls, keep-alive handling, HTTP/2 support, and standard middleware chains.

### 4. Authentication, Authorization, and Session Handling
- **Status:** Moderate / Partial Foundation.
- **Evaluation:** `sitolo-auth` provides argon2id password hashing, high-entropy cryptographic session tokens, MFA state machines, and device binding models. `sitolo-authz` provides granular role, permission, scope grant, and invitation models.
- **Gaps:** HTTP authentication middleware is not fully wired across all routes. Scope widening protection is enforced in `AuthorizedScope`, but global route-level guard enforcement across the entire HTTP handler matrix is incomplete.

### 5. Input Validation, Sanitization, and Data Integrity
- **Status:** Moderate.
- **Evaluation:** Strict typing, strong Rust domain newtypes (`TenantId`, `OrganizationId`, `BranchId`), and bounded string lengths prevent common memory and payload abuse. JSON deserialization rejects unexpected fields.
- **Gaps:** Sanitization for rich HTML/text or external XML (e.g., fiscal authority integrations) is not centralized.

### 6. Security & Vulnerability Profile (XSS, CSRF, SSRF, IDOR, Path Traversal, Secrets, Defaults)
- **Status:** Strong Architectural Primitives / Partial Endpoint Coverage.
- **Evaluation:** SQL injection is prevented by compile-time SQLx query verification and parameterized queries. IDOR is mitigated at the database layer via strict PostgreSQL RLS policies keyed on `app.organization_id`. Secrets are managed via `AppConfig` with redacting zeroizing wrappers (`Secrecy`). `clippy::unwrap_used` and `#![forbid(unsafe_code)]` are enforced across crates.
- **Gaps:** CSRF and SameSite cookie configurations require explicit production headers on HTTP responses once web frontends connect. SSRF protection for external payment webhooks and tax authority endpoints must be enforced with IP pinning and outbound proxy isolation.

### 7. Error Handling, Retry Behavior, Timeout Strategy, and Failure Isolation
- **Status:** Moderate / Not Production-Ready.
- **Evaluation:** Domain errors map cleanly to HTTP status codes via structured error enums (`ApiError`, `AuthError`, `TenancyError`). Internal technical errors are redacted before reaching clients.
- **Gaps:** Network retry mechanisms with exponential backoff and jitter are missing for external provider integrations (mobile money, MRA EIS fiscal servers). Timeout wrappers on I/O streams are not uniformly enforced in the custom TCP server.

### 8. CPU-bound versus I/O-bound Bottlenecks
- **Status:** Untested Under Load.
- **Evaluation:** Password hashing with Argon2id is intentionally CPU-bound and correctly isolated in blocking threads (`tokio::task::spawn_blocking`). I/O calls to PostgreSQL use async SQLx connection pools.
- **Gaps:** No benchmark suites or load tests exist to measure connection pool exhaustion or CPU starvation during concurrent authentication spikes.

### 9. Async Behavior, Blocking Operations, Race Conditions, and Concurrency
- **Status:** High Risk / Not Production-Ready.
- **Evaluation:** Non-blocking async primitives are used across service layers. Database transactions enforce ACID constraints.
- **Gaps:** The custom HTTP transport in `apps/api/src/serve.rs` reads raw TCP streams without explicit read/write timeouts or max payload stream limits, opening risks for async task starvation under slowloris-style slow-client attacks. In-memory locking in reference services lacks multi-instance distributed lock support.

### 10. Database Design, Query Efficiency, Transactions, Migrations, Indexing, Schema Evolution
- **Status:** Strong Architectural Primitives / Partial Schema Delivery.
- **Evaluation:** `sitolo-persistence` implements dual PostgreSQL connection pools (admin pool for migrations/RLS configuration, runtime pool running under a least-privileged `app_runtime` database role). Tenant context is set per transaction (`SET LOCAL app.organization_id`).
- **Gaps:** RLS policies on outbox tables require explicit `USING (true)` and `WITH CHECK` conditions so background worker processes running outside single-tenant API requests can claim and process cross-tenant outbox items without violating RLS. The full schema for inventory, sales, and fiscal records is not fully migrated in Phase 5 scripts.

### 11. Caching Strategy, Invalidation Logic, and Consistency Tradeoffs
- **Status:** Unimplemented.
- **Evaluation:** Architecture permits optional Redis read caching.
- **Gaps:** Caching layer is completely absent. Direct database reads handle all traffic. No cache invalidation protocol or stale-read trade-off strategy is implemented.

### 12. Queueing, Background Jobs, Event Handling, and Idempotency
- **Status:** Unimplemented Scaffold.
- **Evaluation:** `sitolo-events` defines event schema and outbox pattern requirements.
- **Gaps:** `apps/worker` is a skeletal main file with no processing loop. Outbox table polling, transactional publishing, event deduplication, and worker retry state machines are missing.

### 13. Observability: Logs, Metrics, Traces, Alertability, and Diagnosability
- **Status:** Moderate / Substrate Implemented.
- **Evaluation:** `sitolo-observability` implements structured JSON logging via `tracing-subscriber` with mandatory field redaction (passwords, tokens, keys) and metric counters.
- **Gaps:** OpenTelemetry distributed trace propagation headers (`traceparent`) are not parsed or propagated across HTTP boundaries. Prometheus scraper endpoints are not exposed on an admin port.

### 14. Test Coverage, Test Quality, Edge Cases, and Regression Risk
- **Status:** Moderate / Partial Coverage.
- **Evaluation:** Unit tests cover domain entities, session generation, and scope resolution. Dedicated integration tests verify PostgreSQL tenant boundary isolation and RLS enforcement.
- **Gaps:** Missing end-to-end API integration tests, offline-client sync conflict tests, payment reconciliation edge-case tests, and fault-injection chaos tests.

### 15. Configuration Management, Environment Separation, and Secrets Handling
- **Status:** Strong Foundation.
- **Evaluation:** `sitolo-config` uses strict hierarchical configuration parsing with environment variable overrides (`SITOLO__*`). Sensitive fields are wrapped in `SecretString` and zeroized on drop.
- **Gaps:** Production secret rotation procedures and integration with cloud KMS/Vault are documented but lack automated verification tooling.

### 16. Deployment Safety, Rollback Readiness, Versioning, and Release Discipline
- **Status:** Moderate / Not Production-Ready.
- **Evaluation:** Dockerfiles specify multi-stage builds on minimal distroless/alpine bases. Pinned Rust toolchain (`rust-toolchain.toml`) ensures deterministic builds.
- **Gaps:** Zero-downtime database migration strategies (blue/green or canary schema migrations with backwards-compatible views) are not automated in release pipelines.

### 17. Code Quality, Naming, Duplication, Coupling, and Technical Debt
- **Status:** Strong.
- **Evaluation:** High code cleanliness standards. Standardized Rust naming conventions, clean module hierarchy, zero duplicate type definitions, strict clippy lints (`clippy::pedantic`, `clippy::unwrap_used`), and comprehensive inline documentation.
- **Gaps:** Architectural divergence in `apps/api` (custom TCP vs Axum) creates technical debt that must be resolved prior to API surface expansion.

### 18. Maintainability under Team Growth, Code Ownership, and Refactor Cost
- **Status:** Strong.
- **Evaluation:** Clear workspace separation (`crates/*`) allows independent ownership of identity, authz, domain, persistence, and transport. `agent.md` and `docs/` provide clear guidance for onboarding developers and AI agents.
- **Gaps:** High volume of documentation files requires automated semantic verification (`check-doc-references`, `check-icm-workspace`) to prevent documentation decay.

### 19. Compliance and Enterprise Operational Expectations
- **Status:** Contract Specified / Unimplemented Execution.
- **Evaluation:** Specifications define compliance with Mauritius Revenue Authority (MRA) Electronic Invoice System (EIS) tax fiscalization, GDPR/data protection regulations, and audit logging.
- **Gaps:** Tax fiscalization signing engines, tamper-evident audit log hashing, and legal data-retention purging mechanisms are not executed in source code.

---

## Detailed Severity-Ranked Findings

### Critical Severity Findings (P0 / Immediate Action Required)

#### FINDING-CRIT-01: Architectural Divergence in HTTP API Transport Layer
- **What is wrong:** The API server in `apps/api/src/serve.rs` implements a custom TCP socket listener with manual HTTP string parsing instead of using the specified `Axum` web framework on Tokio.
- **Why it is wrong:** Custom HTTP parsers inherently lack security hardening, formal RFC compliance, chunked transfer encoding support, HTTP/2 multiplexing, robust header parsing, connection pooling, and battle-tested request size limits. This violates the governing architecture specification (`system_architecture_design.md`).
- **Where it appears:** `apps/api/src/serve.rs`, `apps/api/src/lib.rs`, `apps/api/Cargo.toml`.
- **How it fails in production:** Malformed HTTP headers, slow-rate payload attacks (Slowloris), or unexpected HTTP connection keep-alive behavior will cause unhandled parse panics, resource exhaustion, or hanging sockets, bringing down the primary API process under minimal malformed traffic.
- **What a proper fix looks like:** Refactor `apps/api` to use `axum` and `tower` HTTP routing stack. Implement standard middleware for timeouts (`tower::timeout`), body size limits (`axum::extract::DefaultBodyLimit`), CORS, and tracing.
- **Priority:** P0
- **Blast Radius:** Entire API subsystem / Complete availability compromise.

#### FINDING-CRIT-02: Absence of Production Worker Loop and Asynchronous Event Outbox Execution
- **What is wrong:** The background worker binary `apps/worker/src/main.rs` is a stub scaffold that exits immediately after startup. The transactional outbox queue processing loop is unimplemented.
- **Why it is wrong:** Core business operations (audit trail persistence, payment reconciliation, async webhook delivery, tax invoice fiscalization) rely on reliable asynchronous event handling.
- **Where it appears:** `apps/worker/src/main.rs`, `crates/sitolo-events/src/lib.rs`.
- **How it fails in production:** Events written to the outbox table in PostgreSQL will accumulate indefinitely without processing. Audit records will not reach durable analytical stores, payment webhooks will fail silently, and tax fiscalization receipts will never be submitted to government endpoints.
- **What a proper fix looks like:** Implement a robust Tokio-based background worker processing loop in `apps/worker`. Implement outbox table polling using `FOR UPDATE SKIP LOCKED`, exponential backoff retries, dead-letter queueing (DLQ), and graceful shutdown handling.
- **Priority:** P0
- **Blast Radius:** All asynchronous business processes, compliance reporting, and external integration.

#### FINDING-CRIT-03: RLS Boundary Conflict on Asynchronous Outbox Worker Processing
- **What is wrong:** Outbox tables protected by strict single-tenant Row Level Security (RLS) policies prevent cross-tenant worker processes from querying or updating outbox records unless session context is set.
- **Why it is wrong:** Background workers operate across all tenants and do not execute within a single tenant's API request HTTP session context (`SET LOCAL app.organization_id`).
- **Where it appears:** `crates/sitolo-persistence/src/postgres/`, Phase 5 migration scripts.
- **How it fails in production:** Background worker tasks will receive zero rows or fail with SQL permission errors when attempting to claim or mark outbox records as published/failed across multiple tenants.
- **What a proper fix looks like:** Define outbox RLS policies using `USING (true)` for worker background read/claim execution while enforcing `WITH CHECK` on insertion to ensure tenant API handlers can only insert outbox events matching their authenticated tenant GUC context.
- **Priority:** P0
- **Blast Radius:** Multi-tenant outbox event publishing and background task execution.

---

### High Severity Findings (P1 / High Impact)

#### FINDING-HIGH-01: Core Domain Engine Missing Source Implementation
- **What is wrong:** Core operational SME business engines (product catalog, inventory movement ledger, POS sales transactions, payments, supplier management, and MRA tax fiscalization) exist only as documentation specifications and lack concrete Rust code in `sitolo-domain`.
- **Why it is wrong:** The application cannot fulfill its core value proposition as a POS and ERP system without domain state machines and business logic.
- **Where it appears:** `crates/sitolo-domain/src/`, `docs/phase8_product_catalogue_implementation.md` through `docs/phase15_mra_eis_implementation.md`.
- **How it fails in production:** Endpoints returning mock data or unrouted endpoints fail to execute real business workflows, rendering the application unusable for retail or trade operations.
- **What a proper fix looks like:** Execute Phase 8 through Phase 15 domain engine code sequentially in `sitolo-domain`, implementing domain aggregates, event generation, state transition invariants, and repository traits.
- **Priority:** P1
- **Blast Radius:** All core business capabilities (Sales, Inventory, Tax, Billing).

#### FINDING-HIGH-02: Missing Global Middleware for Request Timeouts and Body Payload Limits
- **What is wrong:** API endpoints lack global HTTP request timeouts and maximum payload stream limits.
- **Why it is wrong:** Unbounded request body reads allow clients to transmit arbitrarily large payloads, exhausting server memory and thread pools.
- **Where it appears:** `sitolo-api/src/`, `apps/api/src/`.
- **How it fails in production:** An attacker or misconfigured client uploading a multi-gigabyte POST payload will trigger out-of-memory (OOM) kills on the API container.
- **What a proper fix looks like:** Enforce `axum::extract::DefaultBodyLimit::disable()` selectively and apply global request body limits (e.g., 2MB default) and HTTP request execution timeouts (e.g., 30s) via Tower middleware.
- **Priority:** P1
- **Blast Radius:** API process memory and socket availability.

#### FINDING-HIGH-03: Lack of Outbound SSRF Protection and Network Isolation for Provider Integrations
- **What is wrong:** Outbound HTTP client calls to third-party APIs (payment gateways, MRA EIS servers) do not enforce explicit IP destination restrictions or proxy pinning.
- **Why it is wrong:** If a provider URL is user-configurable or manipulated, an attacker could force the server to issue HTTP requests to internal infrastructure (cloud metadata endpoints `169.254.169.254`, internal databases).
- **Where it appears:** `crates/sitolo-integrations/src/`.
- **How it fails in production:** SSRF exploit exposes internal cloud instance credentials or sensitive internal services.
- **What a proper fix looks like:** Configure `reqwest` clients with explicit DNS resolution filters, disallowing private IPv4 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `127.0.0.0/8`) and IPv6 loopback destinations unless explicitly routed through a dedicated egress gateway.
- **Priority:** P1
- **Blast Radius:** Internal network security and cloud infrastructure posture.

#### FINDING-HIGH-04: Incomplete OpenTelemetry Context Propagation Across HTTP and Worker Boundaries
- **What is wrong:** HTTP headers (`traceparent`, `tracestate`) are not parsed or injected into Tokio request contexts and background worker jobs.
- **Why it is wrong:** Without distributed tracing context propagation, requests spanning API handlers, outbox events, and worker background tasks cannot be correlated in telemetry tools.
- **Where it appears:** `crates/sitolo-observability/src/`, `crates/sitolo-api/src/`.
- **How it fails in production:** Operators cannot diagnose distributed request failures, latency bottlenecks, or lost outbox jobs during customer-impacting incidents.
- **What a proper fix looks like:** Integrate `tracing-opentelemetry` and `axum-tracing-opentelemetry` middleware to inject trace context into Tokio task spans and outbox event metadata.
- **Priority:** P1
- **Blast Radius:** System observability, mean-time-to-detection (MTTD), and mean-time-to-resolution (MTTR).

---

### Medium Severity Findings (P2 / Moderate Impact)

#### FINDING-MED-01: In-Memory Locking in Service Layers Prevents Multi-Instance Horizontal Scaling
- **What is wrong:** Certain application reference services utilize in-process synchronization (`tokio::sync::RwLock` or `Mutex`) for state management.
- **Why it is wrong:** In-process locks do not synchronize state across multiple horizontal instances of `apps/api`.
- **Where it appears:** `crates/sitolo-application/src/`, reference repository mocks in `crates/sitolo-persistence/src/`.
- **How it fails in production:** Running two or more API instances will lead to race conditions, inconsistent in-memory state, and duplicate execution.
- **What a proper fix looks like:** Replace all in-process mutable lock state with database transactions (`SELECT ... FOR UPDATE`) or distributed locks (PostgreSQL advisory locks).
- **Priority:** P2
- **Blast Radius:** Multi-instance horizontal API scalability and state consistency.

#### FINDING-MED-02: Missing Prometheus Metrics Endpoint Exposure
- **What is wrong:** Metrics counters registered in `sitolo-observability` are not exposed via a dedicated HTTP scraper endpoint.
- **Why it is wrong:** Prometheus or OpenTelemetry collectors cannot scrape real-time operational metrics (HTTP request rate, latency histograms, database pool utilization).
- **Where it appears:** `apps/api/src/`, `crates/sitolo-observability/src/lib.rs`.
- **How it fails in production:** Automated alerting systems fail to trigger during high latency or elevated error rate conditions.
- **What a proper fix looks like:** Expose a secure `/metrics` endpoint on an internal administrative port (e.g., port 9090) utilizing `metrics-exporter-prometheus`.
- **Priority:** P2
- **Blast Radius:** Operational alerting and performance monitoring.

#### FINDING-MED-03: Lack of Automated Zero-Downtime Migration Policy in CI/CD
- **What is wrong:** Database migrations are applied sequentially at startup without automated verification of backward compatibility.
- **Why it is wrong:** Destructive schema changes (dropping columns, changing data types) will break running API instances during canary or rolling deployments.
- **Where it appears:** `crates/sitolo-persistence/src/postgres/migrations.rs`, release pipelines.
- **How it fails in production:** Rolling deployment causes active API pods running legacy code to fail with SQL syntax/schema errors during deployment transition window.
- **What a proper fix looks like:** Adopt expand-and-contract database migration patterns verified via CI checks before deployment execution.
- **Priority:** P2
- **Blast Radius:** Deployment availability and zero-downtime release safety.

#### FINDING-MED-04: Unbounded In-Memory Collection Growth in Event Repositories
- **What is wrong:** Mock and reference event repositories accumulate events in unbounded vector arrays.
- **Why it is wrong:** High event volume under load tests or prolonged execution causes memory bloat.
- **Where it appears:** `crates/sitolo-events/src/`, test harnesses.
- **How it fails in production:** Long-running test environments or non-production deployments crash due to OOM when processing high event volumes.
- **What a proper fix looks like:** Enforce bounded circular buffers or stream-based persistence traits for all event repositories.
- **Priority:** P2
- **Blast Radius:** Test environment stability and memory consumption.

---

### Low Severity Findings (P3 / Minor Maintenance Impact)

#### FINDING-LOW-01: Historical Audit Document Ambiguity
- **What is wrong:** Legacy audit files in `docs/` describe previous Phase 0-4 states that contradict the current source code baseline.
- **Why it is wrong:** Engineers or AI tools reading legacy audit files may misinterpret completed features as unimplemented gaps or vice versa.
- **Where it appears:** `docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md`.
- **How it fails in production:** Wasted engineering cycles re-implementing existing features or addressing outdated findings.
- **What a proper fix looks like:** Mark legacy audit files with explicit header warnings denoting them as historical records and route active references to `docs/enterprise_audit_and_review.md`.
- **Priority:** P3
- **Blast Radius:** Engineering productivity and documentation clarity.

#### FINDING-LOW-02: Missing Explicit Content Security Policy (CSP) and Security Headers in API Handlers
- **What is wrong:** Security headers (`Content-Security-Policy`, `X-Frame-Options`, `Strict-Transport-Security`, `X-Content-Type-Options`) are not set by default on API HTTP responses.
- **Why it is wrong:** Missing security headers increase vulnerability exposure when web applications consume the API directly.
- **Where it appears:** `crates/sitolo-api/src/`.
- **How it fails in production:** Security scanners flag missing headers, failing enterprise compliance audits.
- **What a proper fix looks like:** Add standard `tower-http` security header middleware (`SetResponseHeader`) across all HTTP routes.
- **Priority:** P3
- **Blast Radius:** Browser-side security posture and compliance score.

---

## Top 10 Highest-Risk Issues

```text
+----+---------------------------------------+-----------------------+---------------------------------------+
| #  | Risk Title                            | Severity / Category   | Core Impact                           |
+----+---------------------------------------+-----------------------+---------------------------------------+
| 1  | Custom HTTP TCP Listener              | CRITICAL / Transport  | Denial of Service, RFC non-compliance |
| 2  | Unimplemented Background Worker Loop  | CRITICAL / Async      | Silent loss of audit and outbox events|
| 3  | Outbox RLS Policy Cross-Tenant Block  | CRITICAL / Database   | Worker execution failure across tenants|
| 4  | Missing Domain Engine Implementations | HIGH / Architecture   | Incapability to perform core business |
| 5  | Missing Request Payload Limits        | HIGH / Security       | Container OOM via oversized payloads  |
| 6  | Missing Outbound SSRF Protections     | HIGH / Security       | Potential internal cloud SSRF exploit |
| 7  | Broken Distributed Telemetry Tracing  | HIGH / Observability  | Inability to trace cross-service bugs |
| 8  | In-Process Locking in Application Layer| MEDIUM / Concurrency  | Failure under horizontal scale        |
| 9  | Unexposed Metrics Scraper Endpoint    | MEDIUM / Operations   | Black-hole operational monitoring     |
| 10 | Unverified Schema Migration Safety    | MEDIUM / Deployment   | Outages during rolling deployments    |
+----+---------------------------------------+-----------------------+---------------------------------------+
```

---

## Top 10 Highest-Leverage Fixes

```text
+----+---------------------------------------+-----------------------------------------------------------------+
| #  | Leverage Fix Strategy                 | Expected Outcome & Systemic Improvement                         |
+----+---------------------------------------+-----------------------------------------------------------------+
| 1  | Migrate `apps/api` to Axum + Tokio    | Restores RFC compliance, connection pooling, and middleware stack|
| 2  | Implement Worker Loop & Outbox Queue  | Enables durable async processing, audit trails, and integrations|
| 3  | Refactor Outbox RLS to `USING (true)` | Unlocks multi-tenant background task worker processing          |
| 4  | Execute Phase 8-15 Domain Engines     | Delivers core POS, catalog, inventory, and payment capabilities |
| 5  | Apply Tower Body Limits & Timeouts    | Hardens API against DoS and slow-client memory exhaustion       |
| 6  | Add SSRF Guardrails to `reqwest`      | Prevents outbound network pivot attacks to cloud metadata       |
| 7  | Wire OpenTelemetry Context Headers    | Enables full distributed tracing across HTTP, database, & worker|
| 8  | Replace In-Process Locks with Postgres| Ensures multi-instance thread and process safety                |
| 9  | Expose `/metrics` Prometheus Endpoint | Restores operational alerting and real-time dashboarding        |
| 10 | Enforce Expand-and-Contract Migrations| Guarantees zero-downtime deployment capabilities                |
+----+---------------------------------------+-----------------------------------------------------------------+
```

---

## Phased Remediation Plan

```text
+---------------------------------------------------------------------------------------------------+
|                                  PHASED REMEDIATION ROADMAP                                       |
+---------------------------------------------------------------------------------------------------+

[PHASE 1: IMMEDIATE / CRITICAL PATH (Weeks 1 - 2)]
 ├── 1. Replace custom TCP parser in `apps/api` with Axum + Tokio HTTP stack.
 ├── 2. Implement background worker event outbox loop in `apps/worker`.
 ├── 3. Adjust PostgreSQL Outbox RLS policies (`USING (true)` / `WITH CHECK`) for worker access.
 └── 4. Add Tower middleware for body limits (2MB default) and request timeouts (30s).

[PHASE 2: SHORT TERM / FOUNDATION STRENGTHENING (Weeks 3 - 6)]
 ├── 1. Implement Phase 8 (Catalog) and Phase 9 (Inventory Ledger) domain logic in `sitolo-domain`.
 ├── 2. Implement Phase 10 (POS Sales) and Phase 11 (Payments & Reconciliation) domain logic.
 ├── 3. Implement outbound SSRF protection in `sitolo-integrations` HTTP client setup.
 └── 4. Wire OpenTelemetry trace context propagation across HTTP request and outbox boundaries.

[PHASE 3: MEDIUM TERM / SCALABILITY & RELIABILITY (Weeks 7 - 10)]
 ├── 1. Replace all in-process service locks with PostgreSQL advisory locks / transactional state.
 ├── 2. Implement Phase 12 (Offline Sync Protocol) and Phase 15 (MRA EIS Tax Fiscalization).
 ├── 3. Expose dedicated Prometheus metrics endpoint (`/metrics`) on administrative port.
 └── 4. Build comprehensive end-to-end load and fault-injection testing suites.

[PHASE 4: LONG TERM / ENTERPRISE PRODUCTION CERTIFICATION (Weeks 11 - 14)]
 ├── 1. Execute automated expand-and-contract zero-downtime database migration pipelines.
 ├── 2. Conduct third-party adversarial security penetration testing and threat model audit.
 ├── 3. Implement automated cloud KMS secret rotation and HSM signing integration.
 └── 4. Complete Phase 20 Production Certification and sign off operational runbooks.
+---------------------------------------------------------------------------------------------------+
```

---

## Evaluation of Acceptable vs Not Enterprise-Grade Aspects

### What IS Acceptable & Enterprise-Grade

1. **Memory Safety & Code Quality:** Zero `unsafe` code allowed across the entire Rust codebase (`#![forbid(unsafe_code)]`). Comprehensive use of strong types, newtypes, zeroizing secret containers, and mandatory clippy linting.
2. **Tenant Isolation Architecture:** Multi-tenant isolation enforced deep in the database layer via PostgreSQL Row Level Security (RLS) bound to transaction-local configuration parameters (`app.organization_id`, `app.branch_id`).
3. **Authentication & Session Primitives:** Cryptographically secure session generation, Argon2id password hashing with isolated CPU thread pools, MFA state handling, and device binding models.
4. **Dependency & Build Governance:** Tight cargo dependency management with `deny.toml` banning duplicate crates, unvetted licenses, and vulnerable transitive crates. Deterministic builds pinned via `rust-toolchain.toml`.
5. **Configuration Handling:** Hierarchical environment-aware configuration with redacting `Secrecy` wrappers preventing secret leakage into logs.

### What IS NOT Enterprise-Grade (Current Gaps)

1. **Custom HTTP Parser Transport:** Direct TCP socket parsing in `apps/api` instead of an established, battle-tested framework (Axum) is brittle, unsafe under hostile network conditions, and violates enterprise transport standards.
2. **Unimplemented Background Worker & Outbox:** Absence of a live background worker process leaves event publishing, audit trail ingestion, and async reconciliation completely unexecuted.
3. **Missing Business Domain Engines:** Absence of compiled Rust code for POS sales, inventory ledger, payments, and tax fiscalization means the business logic layer is incomplete.
4. **Lack of Distributed Tracing & Metrics Scraping:** Missing OpenTelemetry HTTP propagation headers and lack of a Prometheus scraper endpoint severely restrict production incident diagnosability.
5. **Untested Scale & Load Resilience:** Lack of benchmark evidence, concurrency load tests, and chaos testing under multi-tenant isolation scenarios means production traffic behavior is unverified.

---

## Conclusion

Sitolo possesses a **superior architectural foundation** that far exceeds typical SME software implementations in security design, tenancy isolation, and code discipline. However, because critical transport, async processing, and domain business engines remain uncompleted scaffolds, **the codebase cannot be certified for enterprise production use today.**

Executing the **Phased Remediation Plan**—beginning with the immediate migration of `apps/api` to Axum, activation of `apps/worker` outbox polling, and sequential implementation of Phase 8–15 domain engines—will elevate Sitolo from a security-hardened platform scaffold into a fully certified, production-ready enterprise business operating system.
