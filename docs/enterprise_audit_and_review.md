# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main` at `e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc`
**Status:** Canonical Comprehensive Enterprise Audit & Review
**Authority:** Current repository source tree, governing architecture contracts, and `agent.md`

---

## Executive Summary

Sitolo is designed as a multi-tenant business operating system for African small and medium enterprises (SMEs). It features a modern Rust modular monolith architecture using Axum, Tokio, SQLx, and PostgreSQL Row Level Security (RLS).

A rigorous, end-to-end enterprise audit across **architecture, security, reliability, scalability, performance, maintainability, observability, testing, deployment readiness, and operational risk** reveals that the repository has established exceptional security and structural foundations, but is **NOT yet production-ready**.

### Key Strengths (Enterprise-Grade Primitives)
1. **Workspace Architecture & Policy Enforcement:** Clean crate separation (`sitolo-domain`, `sitolo-auth`, `sitolo-tenancy`, `sitolo-authz`, `sitolo-persistence`, `sitolo-config`, `sitolo-security`, `sitolo-observability`), explicit dependency directions, strict `#![forbid(unsafe_code)]` workspace-wide, and automated CI policy verifiers (`check-architecture`, `check-phase2-policy`, `check-icm-workspace`).
2. **PostgreSQL Security Authority:** Dual connection pool design (`PgAuthorityPools`) enforcing strict separation between administrative migration authority and a least-privileged runtime role (`app_runtime`). Comprehensive catalog checks verify that `app_runtime` lacks superuser, bypassrls, createrole, or schema creation rights, and enforces forced RLS on all tenant tables (`app.organization_id`, `app.branch_id`).
3. **Configuration & Secret Handling:** Typed configuration (`sitolo-config`) with multi-layer validation (Syntax, Semantic, Security, Environment, CrossField), hard ceilings on timeouts/sizes, loopback-only plaintext OTEL constraints, and indirect secret reference provenance (`SecretRef`).
4. **Identity & Authentication Primitives:** Domain models for credential management, Argon2 and PBKDF2 hashing policies, MFA and TOTP verification, PKCE state machines, token validation, session revocation lifecycles, and device security states (`sitolo-auth`).

### Key Blockers (Gaps to Production Readiness)
1. **API Transport Boundary Divergence:** The current API binary (`apps/api/src/serve.rs`) implements a hand-rolled, custom TCP socket loop with a fixed 32KB read buffer rather than using the documented framed web framework (Axum + Hyper). It lacks full HTTP/1.1 framing, streaming body handling, HTTP middleware, keep-alive support, and web security headers.
2. **Missing HTTP Authentication & Scope Enforcement:** The HTTP routes in `apps/api` do not extract or validate Authorization tokens or session cookies, exposing organization/branch mutation endpoints without authenticated `SecurityContext` enforcement.
3. **Unimplemented Background Worker Binary:** The background worker (`apps/worker/src/main.rs`) is an empty scaffold (`fn main() {}`). Outbox events, async job processing, fiscal submission tasks, and external provider retries are not executed.
4. **Missing Core Business Domain Engines:** The domain layer (`sitolo-domain`) only implements tenancy lifecycle. Core operational engines—Product Catalogue, Inventory Ledger, Point-of-Sale (POS) Sales, Payments Reconciliation, Procurement, and Offline Synchronization—remain contract/stub crates (`sitolo-events`, `sitolo-integrations`, `sitolo-sync`).
5. **In-Memory State in API Application:** The API binary currently wires `sitolo-application` to in-memory repositories (`MemoryTenancyStore`) wrapped in mutexes, causing total data loss on restart and preventing horizontal multi-instance deployment.

---

## System-Wide Comprehensive Review

### 1. Repository Structure & Dependency Boundaries
- **Evaluation:** Strong. The Rust workspace enforces strict module boundaries and acyclic crate dependencies. Higher-level crates (`sitolo-application`, `sitolo-api`) depend on domain/auth crates, while `sitolo-domain` remains infrastructure-free.
- **Issues:** High scaffold density in peripheral crates (`sitolo-events`, `sitolo-integrations`, `sitolo-sync`). `Cargo.toml` lockfiles are pinned, and `cargo deny` / `cargo audit` checks are integrated into CI.

### 2. Application Architecture & Module Separation
- **Evaluation:** Partial. Clean separation of concerns exists in theory. Domain logic is isolated from database and transport concerns. However, the application layer (`sitolo-application`) only handles tenancy/IAM, leaving all core enterprise business domain logic unbuilt.

### 3. API Design, Request Flow & Trust Boundaries
- **Evaluation:** Critical Risk. Bounded request DTOs with `serde(deny_unknown_fields)` exist in `sitolo-api`, but the custom TCP listener in `apps/api/src/serve.rs` lacks HTTP protocol compliance. It reads up to 32KB in a single socket read without parsing full chunked transfers or multi-packet TCP streams.

### 4. Authentication, Authorization & Session Handling
- **Evaluation:** High Risk. Primitive models in `sitolo-auth` and `sitolo-authz` (role catalogs, scope grants, MFA, PKCE, token verification) are mathematically sound and thoroughly unit-tested. However, they are not wired as HTTP middleware in `apps/api`. API requests execute without populating or asserting `AuthenticatedPrincipal` or `AuthorizedScope`.

### 5. Input Validation, Sanitization & Data Integrity
- **Evaluation:** Satisfactory Foundation. Config and API DTOs enforce strict length bounds, control-character rejection, and JSON deserialization rules. Domain entity constructors enforce state machine invariants.

### 6. Injection Risks, XSS, CSRF, SSRF, IDOR & Secret Leakage
- **Evaluation:** Mixed.
  - *SQL Injection:* Prevented by SQLx parameterized queries and PostgreSQL catalog checks.
  - *IDOR:* Protected at the database layer via PostgreSQL RLS policies matching `app.organization_id` and `app.branch_id`.
  - *SSRF:* `sitolo-config` enforces HTTPS for remote OTEL endpoints outside loopback.
  - *XSS/CSRF/Headers:* Absent because standard web security headers (CORS, CSP, HSTS, X-Frame-Options) are missing from the raw TCP HTTP response handler.
  - *Secret Leakage:* Strong; secrets use indirect `SecretRef` handles and sensitive memory zeroization primitives in `sitolo-security`.

### 7. Error Handling, Retry Behavior & Failure Isolation
- **Evaluation:** Partial. `AppError` and RFC 7807 `ProblemDetails` standardize API error responses without exposing internal stack traces or SQL details. However, background worker retry behavior, backoff strategies, circuit breakers, and Dead Letter Queues (DLQ) do not exist due to the stubbed worker binary.

### 8. CPU-Bound vs. I/O-Bound Bottlenecks
- **Evaluation:** High Risk. CPU-intensive Argon2/PBKDF2 password hashing routines in `sitolo-auth/src/password.rs` must be spawned onto Tokio blocking threads (`spawn_blocking`). If called directly on Tokio worker threads, password verification will block the async reactor under load.

### 9. Async Behavior, Blocking Operations & Concurrency Hazards
- **Evaluation:** High Risk. The custom TCP server accepts socket connections in a single loop and uses `Semaphore` for concurrency bounds, but in-process `RwLock` and `Mutex` locks in memory repositories create concurrency contention under high write loads. Transactions in PostgreSQL are not yet used for API business mutations.

### 10. Database Design, Query Efficiency, Transactions & Schema Evolution
- **Evaluation:** High Risk. PostgreSQL RLS infrastructure (`sitolo-persistence/src/postgres.rs`) enforces strict runtime permissions (`app_runtime`), catalog checks, and transaction-local GUC setting (`set_transaction_tenant_context`). However, real application schema migrations (`sqlx migrate`) and business domain tables do not exist outside test fixture SQL files (`rls_schema.sql`).

### 11. Caching Strategy, Invalidation Logic & Consistency
- **Evaluation:** Unimplemented. Caching is currently non-existent. No Redis/Valkey integration exists for session caching, permissions caching, or distributed rate limiting.

### 12. Queueing, Background Jobs, Event Handling & Idempotency
- **Evaluation:** Critical Risk. `apps/worker` is an empty scaffold. Transactional outbox pattern is designed in docs but unfulfilled in code. Event publication (`sitolo-events`) is an empty stub.

### 13. Observability: Logs, Metrics, Traces & Diagnosability
- **Evaluation:** Satisfactory Foundation. `sitolo-observability` provides structured JSON logging, tracing span management, bounded memory trace buffers, and correlation ID (`RequestId`) tracking. Production metrics exporters and alert triggers remain to be integrated.

### 14. Test Coverage, Test Quality & Edge Cases
- **Evaluation:** Medium Risk. Unit test suites for `sitolo-auth`, `sitolo-config`, `sitolo-tenancy`, and `sitolo-security` are thorough. RLS security integration tests in `sitolo-persistence` test PostgreSQL isolation thoroughly. However, business domain testing and end-to-end HTTP API integration testing cannot occur until those components are implemented.

### 15. Configuration Management, Environment Separation & Secrets
- **Evaluation:** Strong. Multi-layered validation prevents production environments from using local secret providers, verbose log levels, unencrypted remote endpoints, or development secret namespaces.

### 16. Deployment Safety, Rollback Readiness & Release Discipline
- **Evaluation:** Medium Risk. Automated CI checks enforce code formatting, clippy lints, locked toolchains, and policy scripts. Deployment specs (Helm, Kubernetes, Docker Compose, Terraform) and automated database rollback scripts are absent.

### 17. Code Quality, Naming, Technical Debt & Coupling
- **Evaluation:** Strong. Rust code across crates is well-structured, follows standard naming conventions, enforces `#![forbid(unsafe_code)]`, and avoids code duplication. Technical debt is concentrated in scaffold/stub crates.

### 18. Maintainability Under Team Growth & Code Ownership
- **Evaluation:** Strong. Domain boundaries and crate encapsulation ensure clear ownership and low coupling across modules.

### 19. Compliance & Enterprise Operational Expectations
- **Evaluation:** High Risk. Compliance contracts (MRA EIS fiscalization, privacy/data protection) are documented, but integration modules (`sitolo-integrations`) lack live fiscal API communication drivers.

---

## Severity-Ranked Findings List

```text
CRITICAL  (P0) : 4 Findings  — Immediate security, transport, or core functionality failure
HIGH      (P1) : 4 Findings  — Durability, scalability, database, or performance defect
MEDIUM    (P2) : 3 Findings  — Operational, integration, or reliability gap
LOW       (P3) : 2 Findings  — Technical debt, ergonomics, or minor observability issue
```

---

### Critical Findings (P0)

#### CRIT-01: Custom Hand-Rolled TCP HTTP Parser Replaces Axum/Hyper Framework
- **What is wrong:** `apps/api/src/serve.rs` reads incoming raw TCP streams directly into a fixed 32KB buffer (`PROBE_MAX_BYTES = 32 * 1024`) with `stream.read(&mut buf)` and parses HTTP request strings manually without using Axum or Hyper.
- **Why it is wrong:** Violates HTTP/1.1 specifications. It fails to handle HTTP request body streaming over 32KB, multi-packet TCP framing, chunked transfer encodings, keep-alive connections, or standard HTTP web security headers.
- **Where it appears:** `apps/api/src/serve.rs`, `apps/api/Cargo.toml`.
- **How it fails in production:** Legitimate user requests with JSON payloads exceeding 32KB or sent over fragmented TCP packets will be truncated or fail deserialization, resulting in 422 or 500 errors. Attackers can execute Slowloris attacks or craft HTTP header injection payloads.
- **What a proper fix looks like:** Replace hand-rolled TCP reading in `apps/api` with standard Axum (`axum::Router`) running on Tokio + Hyper, using standard middleware (`tower_http` for Cors, Trace, Timeout, RequestDecompression, LimitPayload).
- **Priority & Blast Radius:** P0 Critical. Blast radius: Entire API transport perimeter and incoming traffic handling.

---

#### CRIT-02: Background Worker Binary is an Empty Scaffold (`fn main() {}`)
- **What is wrong:** `apps/worker/src/main.rs` contains only `#![forbid(unsafe_code)] fn main() {}`.
- **Why it is wrong:** The transactional outbox pattern, background job execution, async event dispatching, fiscal MRA EIS submissions, and retry/DLQ classification are completely non-functional.
- **Where it appears:** `apps/worker/src/main.rs`, `crates/sitolo-events/src/lib.rs`.
- **How it fails in production:** Outbox records written to PostgreSQL will accumulate indefinitely without processing. Async fiscalization, invoice emails, SMS notifications, and payment settlement jobs will never run.
- **What a proper fix looks like:** Build an outbox worker loop in `apps/worker` using PostgreSQL `FOR UPDATE SKIP LOCKED` polling or `LISTEN/NOTIFY`, with exponential backoff retries, concurrency limits, and Dead Letter Queue (DLQ) fallback.
- **Priority & Blast Radius:** P0 Critical. Blast radius: All asynchronous operations, background processing, and regulatory fiscal compliance.

---

#### CRIT-03: Core Business Engine Aggregates Missing in Domain Crate
- **What is wrong:** `sitolo-domain` only exports `pub mod tenancy;`. Core SME business domain modules (Product Catalogue, Stock/Inventory Ledger, Sales POS, Payments Reconciliation, Procurement) are missing.
- **Why it is wrong:** Sitolo is intended as a SME Business Operating System, but currently lacks domain models, invariants, and state machines for core retail/POS business operations.
- **Where it appears:** `crates/sitolo-domain/src/lib.rs`.
- **How it fails in production:** The system cannot record sales, adjust inventory balances, process payments, generate invoices, or manage supplier purchases.
- **What a proper fix looks like:** Implement domain aggregates (`Product`, `StockLedger`, `SaleTransaction`, `Payment`, `FiscalInvoice`) in `sitolo-domain` with strict domain invariant methods.
- **Priority & Blast Radius:** P0 Critical. Blast radius: Core system business functionality.

---

#### CRIT-04: API Transport Auth Middleware Integration Absent
- **What is wrong:** HTTP handlers in `apps/api/src/serve.rs` do not parse, verify, or assert Authorization headers, JWT tokens, or session identifiers.
- **Why it is wrong:** Mutation endpoints (e.g. `POST /v1/organizations`, `POST /v1/organizations/{id}/branches/activate`) process business requests without validating request principal credentials or tenancy scopes.
- **Where it appears:** `apps/api/src/serve.rs`, `crates/sitolo-api/src/tenancy.rs`.
- **How it fails in production:** Unauthenticated external actors can send POST requests to tenant administration endpoints and execute state transitions across organizations without credentials.
- **What a proper fix looks like:** Implement an Axum authentication middleware/extractor that validates tokens via `sitolo-auth`, builds an `AuthenticatedPrincipal` and `AuthorizedScope`, and rejects unauthenticated/unauthorized requests before dispatching to handlers.
- **Priority & Blast Radius:** P0 Critical. Blast radius: Entire API security boundary and tenant authorization model.

---

### High Findings (P1)

#### HIGH-01: In-Memory Mutex Storage Used for API State instead of Durable PostgreSQL
- **What is wrong:** `apps/api/src/state.rs` wires `sitolo-application` to `sitolo-persistence`'s in-memory repositories (`MemoryTenancyStore`) wrapped in `RwLock`/`Mutex`.
- **Why it is wrong:** In-memory storage causes total state loss on process restart, lacks ACID properties, and prevents multi-instance horizontal scaling.
- **Where it appears:** `apps/api/src/state.rs`, `crates/sitolo-application/src/tenancy.rs`, `crates/sitolo-persistence/src/memory.rs`.
- **How it fails in production:** Deploying multiple API instances causes split-brain state where each pod holds isolated data. Restarting a pod wipes all provisioned organizations and branches.
- **What a proper fix looks like:** Wire `sitolo-application` services to `PgAuthorityPools` using transaction-scoped tenant context initialization (`set_transaction_tenant_context`).
- **Priority & Blast Radius:** P1 High. Blast radius: Data durability, consistency, and multi-node scalability.

---

#### HIGH-02: Missing Production Database Schema Migrations Framework
- **What is wrong:** Database schema definitions only exist in test fixture files (`crates/sitolo-persistence/tests/fixtures/rls_schema.sql`). There is no versioned migration framework (e.g. `sqlx migrate`) for production deployments.
- **Why it is wrong:** Database schemas cannot be deployed, updated, or audited deterministically across environments.
- **Where it appears:** `crates/sitolo-persistence/tests/fixtures/rls_schema.sql`, `crates/sitolo-persistence/src/postgres.rs`.
- **How it fails in production:** Automated database deployments cannot execute, or schema drift between staging and production causes runtime SQL errors.
- **What a proper fix looks like:** Establish a versioned `migrations/` directory managed via `sqlx migrate`, and integrate automated migration execution and dry-run validation into CI/CD pipelines.
- **Priority & Blast Radius:** P1 High. Blast radius: Database deployment, schema integrity, and release safety.

---

#### HIGH-03: Cryptographic Password Hashing executed Synchronously in Async Reactor
- **What is wrong:** Password hashing and verification functions (`Argon2` / `PBKDF2`) in `crates/sitolo-auth/src/password.rs` execute CPU-intensive computations synchronously.
- **Why it is wrong:** Executing heavy CPU operations directly inside Tokio async tasks blocks the thread reactor, starving other concurrent I/O tasks on that worker thread.
- **Where it appears:** `crates/sitolo-auth/src/password.rs`, `crates/sitolo-auth/src/login.rs`.
- **How it fails in production:** Spikes in login requests saturate Tokio worker threads, causing extreme latency degradation and health probe failures across all API endpoints.
- **What a proper fix looks like:** Wrap all password hashing calls in `tokio::task::spawn_blocking` to offload CPU computations to Tokio's dedicated blocking thread pool.
- **Priority & Blast Radius:** P1 High. Blast radius: API responsiveness, thread pool availability, and DoS resilience.

---

#### HIGH-04: Rate Limiting Lacks Distributed Backend for API Clusters
- **What is wrong:** `sitolo-auth` provides in-memory rate limiting (`RateLimiter`), but lacks a distributed storage backend (e.g. Redis token bucket).
- **Why it is wrong:** In-memory rate limiters track requests on a per-process basis. In a clustered deployment with $N$ API pods, an attacker gets $N$ times the allowed rate limit threshold.
- **Where it appears:** `crates/sitolo-auth/src/ratelimit.rs`.
- **How it fails in production:** Attackers bypass rate limits during brute-force credential stuffing attacks by spreading requests across multiple load-balanced API nodes.
- **What a proper fix looks like:** Implement a Redis-backed distributed rate limiter (e.g., sliding window or token bucket algorithm) while retaining the in-memory limiter as a local fallback.
- **Priority & Blast Radius:** P1 High. Blast radius: Authentication security, brute-force defense, and API rate limiting.

---

### Medium Findings (P2)

#### MED-01: Integration Adapters Stubbed Out for Fiscalization & Payments
- **What is wrong:** `crates/sitolo-integrations` is an empty crate containing no live HTTP client implementations for MRA EIS fiscal APIs or payment gateways.
- **Why it is wrong:** Regulatory fiscalization (Malawi Revenue Authority Electronic Invoicing System) and automated mobile money / card payment reconciliation cannot execute.
- **Where it appears:** `crates/sitolo-integrations/src/lib.rs`.
- **How it fails in production:** POS checkout transactions fail legal receipt fiscalization requirements or cannot accept/reconcile electronic payments.
- **What a proper fix looks like:** Implement provider adapters using HTTP clients with circuit breakers, request signing, retry handlers, and offline fallback queues.
- **Priority & Blast Radius:** P2 Medium. Blast radius: Regulatory compliance and electronic payment processing.

---

#### MED-02: Absence of Offline Data Synchronization Engine
- **What is wrong:** `crates/sitolo-sync` is an empty scaffold lacking delta batching, conflict resolution, or version vector tracking.
- **Why it is wrong:** SMEs operating in low-connectivity environments cannot synchronize offline POS transactions with the cloud backend reliably.
- **Where it appears:** `crates/sitolo-sync/src/lib.rs`.
- **How it fails in production:** Offline sales performed at retail stores fail to sync when connection is restored, or overwrite cloud inventory balances incorrectly due to missing conflict resolution.
- **What a proper fix looks like:** Build a vector-clock / CRDT-based offline sync engine in `sitolo-sync` supporting idempotent batch synchronization and conflict policy resolution.
- **Priority & Blast Radius:** P2 Medium. Blast radius: Offline store operations and multi-branch data consistency.

---

#### MED-03: Lack of Automated Schema Migration Rollback Testing in Release Pipelines
- **What is wrong:** CI workflows verify lockfiles, formatting, clippy lints, and unit tests, but do not execute database migration rollback tests or dry-run schema validations.
- **Why it is wrong:** Bad schema migrations that pass forward application tests may break rollback procedures during emergency production deployments.
- **Where it appears:** `.github/workflows/`, `scripts/ci/verify`.
- **How it fails in production:** An aborted deployment leaves the production database in a partially migrated or corrupted state, requiring manual DBA intervention.
- **What a proper fix looks like:** Add migration rollback test steps (`sqlx migrate revert`) and dry-run checks in `scripts/ci/verify`.
- **Priority & Blast Radius:** P2 Medium. Blast radius: Production release safety and database recovery time.

---

### Low Findings (P3)

#### LOW-01: Historical Documentation Drift in Secondary Specs
- **What is wrong:** Historical documents in `docs/` retain statements referencing earlier implementation states or pre-Phase-4 milestones.
- **Why it is wrong:** Outdated references create confusion for engineering teams and AI context systems regarding current repository capabilities.
- **Where it appears:** `docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md`, `docs/phase8_product_catalogue_implementation.md`.
- **How it fails in production:** Developer misinterpretation of system status, leading to redundant work or incorrect assumptions.
- **What a proper fix looks like:** Maintain strict document metadata headers (`Status: Historical`, `Status: Contract Target`, `Status: Implemented`) across the entire documentation hierarchy.
- **Priority & Blast Radius:** P3 Low. Blast radius: Developer ergonomics and documentation clarity.

---

#### LOW-02: Logging Span Context Enrichment in API Error Mapping
- **What is wrong:** API error formatting (`format_error_response` in `apps/api/src/serve.rs`) generates `ProblemDetails` with a server `RequestId`, but tracing spans do not consistently bind tenant context or user IDs to structured log events during error returns.
- **Why it is wrong:** Log searching during production debugging requires manual correlation between error response payloads and server log streams.
- **Where it appears:** `crates/sitolo-api/src/error.rs`, `apps/api/src/serve.rs`.
- **How it fails in production:** Increased Mean Time To Resolution (MTTR) when diagnosing sporadic API failures under load.
- **What a proper fix looks like:** Ensure all API tracing spans use `tracing::instrument(fields(request_id, tenant_id, user_id))` to bind contextual metadata to every log line.
- **Priority & Blast Radius:** P3 Low. Blast radius: Observability and operational log searching.

---

## Top 10 Highest-Risk Issues

| Rank | Issue | Location | Risk Impact |
|---|---|---|---|
| **1** | Custom hand-rolled TCP socket HTTP parser in API binary | `apps/api/src/serve.rs` | Socket truncation, missing HTTP/1.1 framing, payload limits (32KB), DoS vulnerability. |
| **2** | Unprotected API endpoints (missing HTTP Auth middleware) | `apps/api/src/serve.rs` | Unauthenticated tenant mutation requests, bypass of security context enforcement. |
| **3** | Background worker binary is an empty scaffold (`fn main() {}`) | `apps/worker/src/main.rs` | Zero outbox event processing, background jobs and fiscal submissions never execute. |
| **4** | Core business domain engines absent (POS, Inventory, Sales, Payments) | `crates/sitolo-domain/src/` | Platform cannot process retail sales, track stock, or perform payments. |
| **5** | API runtime backed by in-memory mutex store instead of PostgreSQL | `apps/api/src/state.rs` | Complete data loss on pod restart, zero horizontal multi-instance scalability. |
| **6** | Absence of versioned production database migration framework | `crates/sitolo-persistence/` | Inability to apply or rollback database schema changes safely in production. |
| **7** | CPU-bound cryptographic password hashing blocking Tokio async threads | `crates/sitolo-auth/src/password.rs` | Async thread reactor starvation under login load, API latency spikes. |
| **8** | In-memory rate limiter lacks distributed storage backend | `crates/sitolo-auth/src/ratelimit.rs` | Rate limit bypass across load-balanced multi-pod API deployments. |
| **9** | Unimplemented external integration adapters (MRA EIS fiscal & payments) | `crates/sitolo-integrations/` | Inability to comply with legal fiscal receipt mandates or accept mobile money/cards. |
| **10** | Unimplemented offline data synchronization engine | `crates/sitolo-sync/` | Data corruption or lost transactions when offline POS clients sync to cloud. |

---

## Top 10 Highest-Leverage Fixes

| Rank | Fix Strategy | Target Location | Leverage & Business Benefit |
|---|---|---|---|
| **1** | Adopt Axum Web Framework on Tokio + Hyper | `apps/api/` | Restores standard HTTP/1.1 & HTTP/2 framing, streaming, security headers, and middleware. |
| **2** | Implement Axum Auth & Scope Extraction Middleware | `sitolo-api`, `apps/api` | Enforces authentication, `SecurityContext`, and tenant bounds on all incoming API routes. |
| **3** | Build Outbox Worker Polling Loop in `apps/worker` | `apps/worker`, `sitolo-events` | Enables durable asynchronous background job processing using PostgreSQL `FOR UPDATE SKIP LOCKED`. |
| **4** | Build Core Domain Aggregates in `sitolo-domain` | `sitolo-domain` | Unlocks actual business logic: Inventory stock ledger, POS sales, payments, and invoices. |
| **5** | Wire API Application Layer to PostgreSQL `PgAuthorityPools` | `apps/api/src/state.rs` | Enables durable transactional storage with enforced PostgreSQL Row Level Security (RLS). |
| **6** | Establish `sqlx migrate` Versioned Database Migrations | `crates/sitolo-persistence/` | Guarantees deterministic database schema updates and automated CI rollback verification. |
| **7** | Wrap Cryptographic Hashing in `tokio::task::spawn_blocking` | `sitolo-auth` | Protects Tokio async event loop from CPU-bound starvation under heavy authentication load. |
| **8** | Implement Redis Token Bucket Distributed Rate Limiting | `sitolo-auth` | Ensures consistent rate limit enforcement across multi-instance API server clusters. |
| **9** | Implement MRA EIS Fiscal & Payment Client Adapters | `sitolo-integrations` | Delivers legal regulatory compliance and automated mobile money checkout integrations. |
| **10** | Build Vector Clock / Delta Sync Engine in `sitolo-sync` | `sitolo-sync` | Enables offline SME POS stores to sync sales reliably without data loss. |

---

## Phased Remediation Plan

```text
PHASE 1: Immediate      (Weeks 1–2)  ➔  Critical Transport & Security Hardening
PHASE 2: Short Term     (Weeks 3–4)  ➔  Persistence, Outbox & Migrations
PHASE 3: Medium Term    (Weeks 5–8)  ➔  Business Domain Engines & Integrations
PHASE 4: Long Term      (Weeks 9–12) ➔  Offline Sync & Production Certification
```

### Phase 1: Immediate (Weeks 1–2) — Critical Transport & Security Hardening
- [ ] Replace hand-rolled TCP socket listener in `apps/api/src/serve.rs` with `axum::Router` on Tokio + Hyper.
- [ ] Add standard web middleware (`tower_http` Cors, RequestDecompression, Timeout, LimitPayload, Trace).
- [ ] Implement Axum `FromRequestParts` extractor for `AuthenticatedPrincipal` and `AuthorizedScope` using `sitolo-auth`.
- [ ] Wrap CPU-bound password hashing in `tokio::task::spawn_blocking` within `sitolo-auth`.
- [ ] Verify all API unit and integration tests pass cleanly with Axum transport.

### Phase 2: Short Term (Weeks 3–4) — Persistence, Outbox & Migrations
- [ ] Replace `MemoryTenancyStore` in `apps/api/src/state.rs` with `PgAuthorityPools` and PostgreSQL repositories.
- [ ] Ensure all PostgreSQL queries set `app.organization_id` and `app.branch_id` transaction-local GUCs via `set_transaction_tenant_context`.
- [ ] Establish versioned SQL migrations directory using `sqlx migrate`.
- [ ] Implement outbox worker polling loop in `apps/worker` using PostgreSQL `FOR UPDATE SKIP LOCKED`.
- [ ] Integrate Redis distributed rate limiter into `sitolo-auth` and API middleware.

### Phase 3: Medium Term (Weeks 5–8) — Business Domain Engines & Integrations
- [ ] Implement Product Catalogue, Inventory Stock Ledger, and Sales POS aggregates in `sitolo-domain`.
- [ ] Build Application Services and API DTOs for Product, Stock, POS, and Checkout workflows.
- [ ] Implement MRA EIS fiscalization API client adapter with request signing and circuit breaker in `sitolo-integrations`.
- [ ] Implement Payment Gateway adapter (Airtel Money, TNM Mpamba, Visa/Mastercard) in `sitolo-integrations`.

### Phase 4: Long Term (Weeks 9–12) — Offline Sync & Production Certification
- [ ] Build vector clock / delta-based offline synchronization protocol engine in `sitolo-sync`.
- [ ] Implement local SQLite / IndexedDB sync client state handlers for desktop/mobile POS clients.
- [ ] Conduct end-to-end chaos engineering, load testing, and penetration testing.
- [ ] Execute canonical CI verification (`./scripts/ci/verify`) and obtain final Production Certification.

---

## Enterprise-Grade Acceptable vs. Unacceptable Matrix

| Area | What IS Acceptable (Enterprise-Grade Baseline) | What IS NOT Enterprise-Grade (Production Blocker) |
|---|---|---|
| **Architecture** | Layered modular Rust monolith with strict acyclic dependencies and `#![forbid(unsafe_code)]`. | Unbounded crate coupling, direct database calls from web handlers, or `unsafe` code blocks. |
| **Transport** | Axum web framework with framed HTTP/1.1 & HTTP/2, streaming, payload limits, and security headers. | Hand-rolled raw TCP socket reading into fixed buffers (`32KB`), missing HTTP headers. |
| **Security** | Server-authoritative token verification, strict RLS database isolation, typed scopes, Argon2 password hashing. | Unauthenticated API mutation endpoints, client-provided tenant headers without server validation. |
| **Database** | Dual-pool PostgreSQL architecture (`app_runtime` least privileged), forced RLS on all tenant tables, versioned migrations. | Running API with `BYPASSRLS` or superuser DB accounts, missing migration scripts, in-memory storage in production. |
| **Async & Jobs** | Outbox pattern worker loop with `FOR UPDATE SKIP LOCKED`, exponential backoff, DLQ, and transaction safety. | Empty scaffold background worker binary (`fn main() {}`), lost outbox events. |
| **Config & Secrets** | Pinned toolchains, multi-layered config validation, indirect `SecretRef` tracking, loopback-only plaintext rules. | Unchecked environment variables, raw passwords in config files, local secret provider enabled in production. |
| **Observability** | Structured JSON tracing logs with correlation `RequestId`, bounded trace memory buffers, span instruments. | Unstructured `println!` logging, missing correlation IDs across async task boundaries. |
| **Testing & CI** | Automated linting, toolchain locking, RLS catalog policy checks, policy enforcement scripts (`check-icm-workspace`). | Untested database RLS policies, failing CI verification, missing security tests. |

---

## Conclusion & Production Readiness Posture

Sitolo possesses an **exceptionally strong security and architectural blueprint**, featuring world-class Rust workspace organization, PostgreSQL Row Level Security enforcement, multi-layer configuration validation, and rigorous identity domain modeling.

However, because the current implementation lacks an Axum HTTP transport layer, API authentication middleware, background outbox worker loop, production database migration framework, and business domain engines, the system is **currently NOT ready for production deployment**.

Executing the **4-Phase Remediation Plan** will systematically close these gaps and elevate Sitolo into a secure, resilient, enterprise-grade Business Operating System for African SMEs.
