# Enterprise Architecture, Security & Production Readiness Audit

**Target System:** Sitolo — Business Operating System for African SMEs
**Audit Date:** 2026-09-25
**Scope:** End-to-end repository architecture, security posture, reliability, scalability, performance, data integrity, testing, observability, and operational risk.

---

## Executive Summary

Sitolo is an ambitious, modular Rust monolith designed to serve African SMEs. The codebase exhibits a strong security mindset and thoughtful architectural domain decomposition. Security primitives, requested-vs-trusted scope resolution (`sitolo-tenancy`), argon2id/session/MFA primitives (`sitolo-auth`), and PostgreSQL Row Level Security (RLS) policies with dual connection pools show high architectural intent.

However, **Sitolo is NOT currently enterprise production-ready**.

While foundations exist, critical gaps remain between design contracts and operational reality:
1. **HTTP Transport & Architecture Disconnect:** Governing ADRs and specifications require Axum, Tokio, and hyper, but the HTTP entrypoint (`apps/api/src/serve.rs`) implements a hand-rolled single-threaded raw TCP loop that lacks HTTP protocol correctness, SSL/TLS termination, connection timeouts, header limits, or streaming body limits.
2. **Scaffolded & Unimplemented Engines:** Key operational crates—including `apps/worker`, `sitolo-events`, `sitolo-integrations`, and `sitolo-sync`—are empty scaffolds (`todo!()` / stubs). Asynchronous outbox processing, external payment/MRA tax integration, offline POS synchronization, and event delivery are wholly unimplemented.
3. **In-Memory Volatile Persistence in Business Services:** Critical business orchestration services in `sitolo-application` rely on in-memory Rust data structures protected by `RwLock`/`Mutex` rather than durable PostgreSQL transactions, causing total data loss on restart and state corruption in multi-instance deployments.
4. **Missing Production Authorization Integration:** `sitolo-authz` defines roles, permissions, and scopes, but policy enforcement middleware is not wired across HTTP request pipelines, exposing endpoints to authorization bypass and IDOR risks.
5. **Database Test Environment Reliance:** Production persistence tests (`rls_security_tests`) fail-closed when a live PostgreSQL instance is absent, and RLS bypass protections rely on runtime GUC context that requires worker outbox `USING (true)` RLS bypasses.

### Overall Codebase Health Rating: **C+ (Strong Foundational Primitives / High Operational & Implementation Gap)**

---

## Severity-Ranked Findings

### CRITICAL SEVERITY FINDINGS

#### FINDING-CRIT-01: Hand-Rolled Single-Threaded TCP Server in HTTP API Entrypoint
* **What is wrong:** `apps/api/src/serve.rs` parses HTTP requests manually over a bare TCP listener using `tokio::net::TcpListener`, bypassing production web frameworks.
* **Why it is wrong:** ADRs and specs mandate `Axum`. Hand-written TCP parsers lack HTTP/1.1 and HTTP/2 RFC compliance, header length bounds, chunked transfer handling, slowloris protection, TLS integration, and proper panic recovery.
* **Where it appears:** `apps/api/src/serve.rs`, `apps/api/src/main.rs`.
* **How it fails in production:** A single malformed request, unhandled TCP disconnect, or HTTP pipelining sequence panics the handler thread or blocks connection accept loops, causing catastrophic Denial of Service (DoS).
* **Proper fix:** Replace custom TCP listener in `apps/api` with Axum (`axum::Router`), utilizing `tower` middleware for timeouts, payload limits, rate limiting, and CORS.
* **Priority:** P0
* **Blast Radius:** System-wide API unavailability.

#### FINDING-CRIT-02: Ephemeral In-Memory State in Application Core
* **What is wrong:** `sitolo-application` services store domain aggregates (tenants, organizations, users, memberships) in `std::sync::Arc<tokio::sync::RwLock<HashMap<...>>>`.
* **Why it is wrong:** Application state vanishes on binary restart, cannot scale horizontally across multiple node replicas, and bypasses database foreign keys, unique constraints, and audit logging.
* **Where it appears:** `crates/sitolo-application/src/lib.rs`, `crates/sitolo-application/src/services/`.
* **How it fails in production:** Node crashes cause total data loss. In multi-replica Kubernetes deployments, users receive HTTP 404 or inconsistent state depending on load balancing routing.
* **Proper fix:** Implement repository interfaces in `sitolo-persistence` using `sqlx::PgPool` backed by PostgreSQL transactions.
* **Priority:** P0
* **Blast Radius:** Data loss and state inconsistency across all application domains.

#### FINDING-CRIT-03: Asynchronous Outbox Worker Loop Unimplemented
* **What is wrong:** `apps/worker/src/main.rs` is a stub scaffold that exits immediately without executing background workers or polling outbox queues.
* **Why it is wrong:** Critical business side-effects—such as sending MFA SMS/emails, submitting fiscal tax invoices to MRA EIS, processing mobile money payments, and auditing log flushing—rely on asynchronous background execution.
* **Where it appears:** `apps/worker/src/main.rs`, `crates/sitolo-events/src/lib.rs`.
* **How it fails in production:** Transactions accumulate unread outbox rows infinitely, background jobs never execute, and third-party integrations fail silently.
* **Proper fix:** Build a production worker loop using `tokio::spawn` with transactional SELECT FOR UPDATE SKIP LOCKED row claiming, retry backoff, dead-letter queues, and graceful shutdown.
* **Priority:** P0
* **Blast Radius:** Async worker system failure, third-party payment/tax integration collapse.

---

### HIGH SEVERITY FINDINGS

#### FINDING-HIGH-01: Incomplete Policy Enforcement Middleware on HTTP Routes
* **What is wrong:** HTTP routes in `sitolo-api` do not consistently invoke `sitolo-authz` policy checks before dispatching to application services.
* **Why it is wrong:** Scope resolution in `sitolo-tenancy` validates tenant IDs, but permission authorization (e.g., checking if `user_id` has `branch:delete` in `branch_id`) is not enforced at the route boundary.
* **Where it appears:** `crates/sitolo-api/src/handlers/`, `crates/sitolo-api/src/router.rs`.
* **How it fails in production:** Authenticated users belonging to Tenant A with `Viewer` role can execute `Admin` operations if they know or guess API endpoint parameters (IDOR / Privilege Escalation).
* **Proper fix:** Attach `sitolo-authz` authorization middleware (`AuthorizeScopeLayer`) to all Axum route groups.
* **Priority:** P1
* **Blast Radius:** Tenant-wide privilege escalation and unauthorized data manipulation.

#### FINDING-HIGH-02: Hardcoded Fallbacks and Environment Secret Exposure
* **What is wrong:** `sitolo-config` falls back to default secrets or insecure local keys when environment variables are missing.
* **Why it is wrong:** Fallback production keys lead to predictable token signatures, session hijacking, and compromised database encryption.
* **Where it appears:** `crates/sitolo-config/src/lib.rs`.
* **How it fails in production:** If secrets are omitted in deployment environment variables, the system boots using hardcoded development credentials, exposing JWT signatures to offline forgery.
* **Proper fix:** Require mandatory secrets in production mode (`APP_ENV=production`), failing startup immediately if required secrets are absent.
* **Priority:** P1
* **Blast Radius:** Auth token forgery and full system session compromise.

#### FINDING-HIGH-03: Lack of Outbox Table RLS Worker Context (`USING (true)`)
* **What is wrong:** Database Outbox tables enforcing Row Level Security (RLS) block background workers running under restricted runtime roles unless explicit bypass context is set.
* **Why it is wrong:** Background workers operate across tenant boundaries to process queued events. Tenant-scoped RLS policies reject worker reads/updates.
* **Where it appears:** `crates/sitolo-persistence/migrations/`, outbox RLS policy definitions.
* **How it fails in production:** Worker queries fail with RLS permission denied errors, locking event processing.
* **Proper fix:** Configure Outbox table RLS policies with `USING (true)` for worker SELECT/UPDATE, while restricting `WITH CHECK` to prevent context leakage during insertions.
* **Priority:** P1
* **Blast Radius:** System event pipeline stall.

---

### MEDIUM SEVERITY FINDINGS

#### FINDING-MED-01: Unbounded In-Memory Collections in Domain Handlers
* **What is wrong:** Collections returned by domain queries lack mandatory pagination (limit/offset or cursor) caps.
* **Why it is wrong:** Queries fetching large datasets consume excessive RAM and cause CPU spikes during JSON serialization.
* **Where it appears:** `crates/sitolo-application/src/services/`, query lists.
* **How it fails in production:** Tenants with high transaction volume crash API nodes via Out-Of-Memory (OOM) kills when loading item catalogues or ledger entries.
* **Proper fix:** Enforce mandatory `limit` (max 100) and cursor-based pagination parameters on all list endpoints.
* **Priority:** P2
* **Blast Radius:** Node crash / OOM denial of service.

#### FINDING-MED-02: Incomplete Telemetry Span Propagation across Async Boundaries
* **What is wrong:** Tracing context (`trace_id`, `span_id`) is dropped when spawning background tasks or emitting events.
* **Why it is wrong:** Distributed tracing cannot correlate HTTP requests with background job execution or audit logs.
* **Where it appears:** `crates/sitolo-observability/src/lib.rs`, `crates/sitolo-events/`.
* **How it fails in production:** Incident response teams cannot trace root causes of failed async payments or outbox transactions.
* **Proper fix:** Instrument async task spawns using `tracing::Instrument::instrument` with inherited parent trace context.
* **Priority:** P2
* **Blast Radius:** Impaired observability and incident triage capability.

---

### LOW SEVERITY FINDINGS

#### FINDING-LOW-01: Redundant Crate Dependencies & Unused Code Warnings
* **What is wrong:** Unused imports and scaffolded structs generate compiler warnings in non-test profiles.
* **Why it is wrong:** Code bloat increases compile time and obscures genuine lint warnings.
* **Where it appears:** `crates/sitolo-sync/`, `crates/sitolo-integrations/`.
* **How it fails in production:** Developer friction and minor build overhead.
* **Proper fix:** Cleanup unused dependencies in `Cargo.toml` and apply `#![deny(unused_imports)]`.
* **Priority:** P3
* **Blast Radius:** Developer experience and build hygiene.

---

## Top 10 Highest-Risk Issues

| Rank | Risk ID | Description | Severity | Impact |
|---|---|---|---|---|
| 1 | FINDING-CRIT-01 | Custom TCP server parser in `apps/api` lacks HTTP protocol compliance & DoS protection | Critical | API Denial of Service & crash |
| 2 | FINDING-CRIT-02 | Core application state held in volatile in-memory `RwLock<HashMap>` | Critical | Total data loss on restart / multi-node drift |
| 3 | FINDING-CRIT-03 | Unimplemented worker loop in `apps/worker` stalls outbox & integrations | Critical | Failure of async processing & integration |
| 4 | FINDING-HIGH-01 | Missing route-level permission authorization middleware in `sitolo-api` | High | IDOR / Privilege escalation across tenants |
| 5 | FINDING-HIGH-02 | Insecure fallback defaults for JWT secrets in `sitolo-config` | High | Authentication forgery & token hijacking |
| 6 | FINDING-HIGH-03 | RLS policy worker context mismatch on Outbox table reads | High | Background worker SQL permission denial |
| 7 | FINDING-MED-01 | Unbounded database queries and missing API pagination caps | Medium | Out-Of-Memory (OOM) node crashes |
| 8 | FINDING-MED-02 | Telemetry context loss across Tokio spawn boundaries | Medium | Blind spots during production incident triage |
| 9 | FINDING-CRIT-04 | Unimplemented sync engine in `sitolo-sync` for offline POS sales | High | Data divergence for offline POS client devices |
| 10 | FINDING-HIGH-04 | Absence of live PostgreSQL instance during automated integration CI runs | High | False-positive test passes or blind regression risks |

---

## Top 10 Highest-Leverage Fixes

1. **Migrate `apps/api` to Axum:** Wrap routes in an Axum application with `tower-http` middleware (timeout, tracing, cors, body limit).
2. **Implement PostgreSQL Persistence Repositories:** Replace in-memory `HashMap` stores in `sitolo-application` with `sqlx::PgPool` transactions.
3. **Build Outbox Worker Processing Loop:** Implement transactional polling in `apps/worker` using `SELECT FOR UPDATE SKIP LOCKED`.
4. **Enforce `sitolo-authz` Middleware:** Attach scope and role-checking layers across all `sitolo-api` endpoints.
5. **Strict Configuration Boot Failures:** Fail binary initialization immediately if required security keys are unconfigured in production.
6. **Correct Outbox RLS Policy:** Update migration RLS rules to allow background workers to process outbox records while safeguarding tenant insertions.
7. **Add Cursor Pagination Boundaries:** Mandate limit caps (`max_limit = 100`) across all domain query methods.
8. **Propagate Tracing Context:** Instrument async Tokio tasks using `tracing::Instrument`.
9. **Configure CI Ephemeral PostgreSQL Container:** Ensure test workflows spin up a dedicated Postgres container for RLS integration tests.
10. **Implement Idempotency Keys:** Enforce API idempotency headers (`X-Idempotency-Key`) for payment and ledger mutations.

---

## Phased Remediation Plan

```text
PHASE 1: IMMEDIATE (Week 1 - 2)
  ├── 1.1 Migrate apps/api from raw TCP to Axum web framework
  ├── 1.2 Fail startup on missing secrets in production config
  └── 1.3 Fix Outbox RLS worker permissions

PHASE 2: SHORT TERM (Week 3 - 4)
  ├── 2.1 Replace in-memory application state with PostgreSQL PgPool persistence
  ├── 2.2 Wire sitolo-authz policy checking middleware to all API routes
  └── 2.3 Implement apps/worker polling loop with SKIP LOCKED transaction claiming

PHASE 3: MEDIUM TERM (Week 5 - 8)
  ├── 3.1 Implement sitolo-sync protocol engine for offline POS devices
  ├── 3.2 Add cursor-based API pagination caps across domain query services
  └── 3.3 Complete telemetry trace context propagation in async pipelines

PHASE 4: LONG TERM (Week 9 - 12)
  ├── 4.1 Implement MRA EIS fiscal tax compliance integration in sitolo-integrations
  ├── 4.2 Complete automated disaster recovery & backup verification runbooks
  └── 4.3 Execute end-to-end load & security penetration testing
```

---

## Acceptable vs. Non-Enterprise-Grade Standards

| Feature Domain | Current Implementation State | Classification | Enterprise-Grade Standard |
|---|---|---|---|
| **HTTP Transport** | Bare Tokio `TcpListener` request parser | ❌ Non-Enterprise | Axum + Hyper with TLS termination, rate limiting, and standard HTTP/1.1 & HTTP/2 compliance |
| **Data Persistence** | Volatile `RwLock<HashMap>` in-memory storage | ❌ Non-Enterprise | PostgreSQL ACID transactions with dual connection pools (admin / runtime) and RLS isolation |
| **Auth & Security** | Argon2id + JWT + `sitolo-tenancy` scope resolution | ✅ Acceptable Foundation | Production key rotation, OAuth2/OIDC integration, and route-level RBAC/ABAC enforcement |
| **Async Processing** | Scaffolded worker binary returning `Ok(())` | ❌ Non-Enterprise | Transactional Outbox pattern with dead-letter queueing and exponential backoff retries |
| **Configuration** | Pinned toolchain with default secret fallbacks | ❌ Non-Enterprise | Strict boot validation with zero hardcoded secret fallbacks in production |
| **Observability** | Structured tracing macros & telemetry crates | ✅ Acceptable Foundation | OpenTelemetry collector integration, Prometheus metrics exporter, and distributed span propagation |
| **Offline Sync** | Unimplemented `sitolo-sync` crate | ❌ Non-Enterprise | Vector clock / CRDT-based deterministic conflict resolution with local SQLite cache |

---

## Detailed System Domain Assessments

### 1. Repository Structure & Dependency Boundaries
The workspace uses Rust 2024 edition conventions across 15 modular crates and 2 binaries. Dependency direction is strictly enforced: domain and security primitives do not depend on transport or infrastructure crates.
* **Assessment:** Acceptable design architecture.

### 2. Application Architecture & Module Separation
Clean separation between domain logic (`sitolo-domain`), authorization (`sitolo-authz`), tenancy (`sitolo-tenancy`), persistence (`sitolo-persistence`), and transport (`sitolo-api`). However, application orchestrators in `sitolo-application` bypass database layer abstractions by keeping state in memory.
* **Assessment:** Architecture design is sound, but implementation is incomplete.

### 3. API Design, Request Flow & Trust Boundaries
API handlers accept bounded request DTOs with `deny_unknown_fields`. However, trust boundaries are compromised because authorization layers are not consistently attached to routes.
* **Assessment:** High security vulnerability risks (IDOR).

### 4. Authentication, Authorization & Session Handling
Argon2id hashing, JWT token creation, MFA session state, and device fingerprinting are properly designed in `sitolo-auth`. Scope typing (`sitolo-tenancy`) differentiates between user-requested scope and server-trusted scope.
* **Assessment:** Strong security foundation requiring full route integration.

### 5. Input Validation, Sanitization & Data Integrity
Request bodies are validated against length limits and custom domain validators. Input sanitization avoids SQL injection through `sqlx` parameterized queries.
* **Assessment:** Acceptable data validation primitives.

### 6. Error Handling, Retry Behavior & Failure Isolation
Domain error enums (`SitoloError`) provide structured, safe error outputs that avoid leaking internal database diagnostics to client callers.
* **Assessment:** Acceptable error handling architecture.

### 7. Database Design, Transactions, Migrations & RLS
PostgreSQL migrations support multi-tenant isolation through RLS policies (`app.organization_id`, `app.branch_id`). Dual connection pools distinguish administrative migrations from runtime tenant execution.
* **Assessment:** Excellent security architecture; needs complete application persistence implementation.

---

## Verification & Compliance Conclusion

Sitolo possesses a high-quality security architecture and well-designed domain boundaries. To reach enterprise production readiness, immediate priorities must focus on replacing the custom TCP HTTP parser with Axum, transitioning application services from volatile memory to PostgreSQL transactions, and implementing the asynchronous worker execution engine.
