# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main` at `e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc`
**Evaluation context:** Full production enterprise readiness audit across architecture, security, reliability, scalability, performance, maintainability, observability, testing, deployment readiness, and operational risk.
**Status:** Canonical Enterprise Audit & Architecture Review Report
**Authority:** Source tree inspection plus governing architecture and security specifications (`agent.md`).

---

## Executive Summary

Sitolo is currently **NOT PRODUCTION READY**.

While the codebase exhibits strong architectural intentions, well-structured security primitives (in `sitolo-auth`, `sitolo-security`, and `sitolo-tenancy`), and pinned toolchains, it remains a **partially wired platform foundation** rather than a functional business operating system.

The overall health of the codebase is classified as **TRANSITIONAL FOUNDATION**:

- **Security & Identity Foundation:** **PARTIAL / STRONG PRIMITIVES.** Password hashing (Argon2id), session sliding, PKCE, MFA state machines, and bounded DTO validation exist. However, the HTTP transport layer (`apps/api`) does not integrate authentication middleware or enforce multi-tenant authorization policies on incoming request handlers.
- **Core Business Domain Engines:** **UNIMPLEMENTED.** `crates/sitolo-domain` contains only tenancy entities. Domain aggregates and state logic for product catalogue, pricing, inventory ledgers, POS sales transactions, cash register closing, payments, reconciliation, and tax submissions do not exist.
- **HTTP Transport Layer:** **CRITICAL IMPLEMENTATION GAP.** The implemented API binary (`apps/api/src/serve.rs`) uses a hand-written TCP request parser without TLS, framing handling, or standard HTTP server abstractions, directly contradicting the governing specification requiring Rust + Axum + Tokio.
- **Asynchronous Execution & Workers:** **UNIMPLEMENTED.** `apps/worker` is an empty scaffold (`fn main() {}`). Outbox event publication, worker polling loops, background reconciliation, and external provider retries are non-functional.
- **Database & Persistence Authority:** **INCOMPLETE.** PostgreSQL authority and RLS primitives exist only as test fixtures (`crates/sitolo-persistence/tests/fixtures/rls_schema.sql`). No versioned SQLx migrations or persistent PostgreSQL repositories are wired into the application layer, leaving the API running entirely on volatile in-memory databases.
- **External Integrations & Client Interfaces:** **STUBS.** `sitolo-integrations` (payments, MRA EIS) and `sitolo-sync` (offline client synchronization) are empty crate stubs. Flutter mobile and Tauri desktop client applications are completely absent from the repository.

```text
CURRENT CODEBASE POSTURE:

  +-------------------------------------------------------------+
  |              DOCUMENTED ARCHITECTURAL CONTRACT              |
  |     (Rust + Axum + Tokio + PostgreSQL + RLS + Workers)       |
  +-------------------------------------------------------------+
                                |
                                v
  +-------------------------------------------------------------+
  |              IMPLEMENTED SECURITY PRIMITIVES                |
  |   (sitolo-auth, sitolo-tenancy, sitolo-config, sitolo-authz) |
  +-------------------------------------------------------------+
                                |
                                v
  +-------------------------------------------------------------+
  |               API TRANSPORT & PERSISTENCE GAP               |
  | (Custom TCP Parser, In-Memory DB, Unauthenticated Handlers) |
  +-------------------------------------------------------------+
                                |
                                v
  +-------------------------------------------------------------+
  |             MISSING BUSINESS DOMAINS & WORKERS              |
  | (Catalogue, Inventory, POS, Payments, MRA, Sync, Worker)   |
  +-------------------------------------------------------------+
```

---

## 1. System-Wide End-to-End Architectural & Domain Review

### 1.1 Repository Structure and Dependency Boundaries
- **Current State:** The repository is organized as a Cargo workspace with 2 applications (`apps/api`, `apps/worker`) and 15 domain/infrastructure crates (`crates/sitolo-*`). Dependency direction generally enforces `API -> Application -> Domain`.
- **Concrete Weakness:** `crates/sitolo-persistence/src/lib.rs` conditionally compiles `pub(crate) mod postgres;` in non-test builds. Because application services rely exclusively on in-memory stores (`TenancyDatabase`), key PostgreSQL authority structures (`PgAuthorityPools`) generate compiler dead-code warnings and remain disconnected from runtime HTTP traffic.
- **Production Impact:** The system cannot persist data across server restarts, rendering all API operations ephemeral.

### 1.2 Application Architecture and Module Separation
- **Current State:** Bounded contexts are defined in documentation (`identity`, `tenant`, `authz`, `catalogue`, `inventory`, `sales`, `cash`, `payments`, `reconciliation`, `tax`).
- **Concrete Weakness:** In code, only `tenancy` and `identity` have application service implementations (`sitolo-application/src/tenancy.rs` and `identity.rs`). All other modules exist only as names in documentation.
- **Production Impact:** Business actions cannot be executed; application orchestration for commercial SME operations is missing.

### 1.3 API Design, Request Flow, and Trust Boundaries
- **Current State:** `crates/sitolo-api` defines bounded request/response DTOs with `#[serde(deny_unknown_fields)]` and input bounds checks (`MAX_TENANCY_BODY_BYTES`, `MAX_IDENTIFIER_LEN`).
- **Concrete Weakness:** `apps/api/src/serve.rs` parses raw HTTP over plain TCP. It extracts body and headers from the first `stream.read` call without buffering or chunk assembly, failing on any multi-packet HTTP payload or network fragmentation.
- **Production Impact:** Legitimate requests spanning multiple TCP segments are truncated and rejected as invalid JSON.

### 1.4 Authentication, Authorization, and Session Handling
- **Current State:** `sitolo-auth` provides robust password hashing (Argon2id), session management, PKCE verification, MFA state handling, and device registration primitives.
- **Concrete Weakness:** HTTP request handlers in `apps/api/src/serve.rs` do NOT extract `Authorization: Bearer <token>` headers, validate sessions, or construct `SecurityContext` before calling tenancy operations.
- **Production Impact:** Any unauthenticated HTTP client can issue requests to provision, suspend, resume, or close organizations and branches.

### 1.5 Input Validation, Sanitization, and Data Integrity
- **Current State:** DTO validation functions (`validate_identifier`, `validate_name`, `validate_create_organization_input`) reject control characters, invalid length bounds, and unknown JSON properties.
- **Concrete Weakness:** Validation is applied only to tenancy DTOs. Because business endpoints do not exist, there is no validation logic for monetary values, transaction lines, barcodes, tax numbers, or inventory movement quantities.
- **Production Impact:** Future endpoint additions without strict validation will risk floating-point corruption, SQL injection, or negative inventory states.

### 1.6 Injection Risks, XSS, CSRF, SSRF, IDOR, Path Traversal, Secret Leakage, and Insecure Defaults
- **Current State:** `#![forbid(unsafe_code)]` is enforced workspace-wide. SQLx parameterized queries are specified for database access. Secrets are wrapped in `RedactedValue` to prevent accidental logging.
- **Concrete Weakness:** `apps/api/src/serve.rs` serves plaintext HTTP without TLS encryption. IDOR protection relies on client-provided IDs in `CreateOrganizationRequest` rather than server-derived authenticated principal claims.
- **Production Impact:** Network eavesdroppers can capture plain-text credentials and tokens in transit; attackers can perform IDOR attacks by specifying arbitrary user or membership IDs.

### 1.7 Error Handling, Retry Behavior, Timeout Strategy, and Failure Isolation
- **Current State:** `sitolo-api::error::AppError` maps domain errors to RFC 7807 Problem Details responses. Probe read timeouts (`PROBE_READ_TIMEOUT_SECS = 5`) exist in `serve.rs`.
- **Concrete Weakness:** No retry policies, exponential backoff, circuit breakers, or bulkhead patterns exist for external dependencies (e.g. payment gateways or MRA EIS endpoints).
- **Production Impact:** Transient upstream network blips cause immediate transaction failure or uncaught exceptions, cascading across API handlers.

### 1.8 CPU-Bound versus I/O-Bound Bottlenecks
- **Current State:** Argon2id password hashing is used for credential verification.
- **Concrete Weakness:** Dedicated `tokio::task::spawn_blocking` pools are not configured for Argon2id hashing during HTTP authentication handling, risking async worker thread starvation under login spikes.
- **Production Impact:** High concurrent login volume blocks the main Tokio reactor threads, causing general API latency spikes and probe timeouts.

### 1.9 Async Behavior, Blocking Operations, Race Conditions, and Concurrency Hazards
- **Current State:** In-memory stores in `sitolo-persistence` use `std::sync::RwLock` for concurrency control.
- **Concrete Weakness:** `std::sync::RwLock` guarded mutations in `sitolo-persistence/src/memory.rs` are blocking sync calls inside async Tokio tasks. Under high thread contention, Tokio workers stall.
- **Production Impact:** Concurrency contention in memory creates Tokio runtime lockup; multiple API replicas cannot run concurrently due to lack of distributed locking.

### 1.10 Database Design, Query Efficiency, Transactions, Migrations, Indexing, and Schema Evolution
- **Current State:** `crates/sitolo-persistence/tests/fixtures/rls_schema.sql` defines a relational PostgreSQL schema with RLS policies, foreign keys, and indexes for tenant isolation.
- **Concrete Weakness:** Schema definitions exist only as string literals in test files. No SQLx migration directory (`migrations/`) exists. Application code uses in-memory HashMaps rather than PostgreSQL tables.
- **Production Impact:** No database schema can be deployed to production PostgreSQL instances; migrations cannot be versioned or applied automatically.

### 1.11 Caching Strategy, Invalidation Logic, and Consistency Tradeoffs
- **Current State:** No distributed or local caching layer (e.g. Redis or moka) is implemented.
- **Concrete Weakness:** Read path queries (such as session or permission lookups) execute directly against storage without caching or negative caching strategies.
- **Production Impact:** High read traffic causes unnecessary storage layer strain; high latency on authorization checks.

### 1.12 Queueing, Background Jobs, Event Handling, and Idempotency
- **Current State:** `sitolo-events` defines event taxonomy requirements.
- **Concrete Weakness:** `apps/worker/src/main.rs` is an empty `fn main() {}`. The outbox pattern is not implemented; background job queues, webhook delivery, and idempotency tracking are completely absent.
- **Production Impact:** Transactional side-effects (e.g., sending payment confirmation webhooks or tax filings) fail to process asynchronously.

### 1.13 Observability: Logs, Metrics, Traces, Alertability, and Incident Diagnosability
- **Current State:** `sitolo-observability` contains structured context types (`RequestId`, `TraceParent`) and a priority buffer (`PriorityBuffer`).
- **Concrete Weakness:** `apps/api/src/serve.rs` does not instantiate tracing spans or attach correlation IDs to HTTP request execution contexts. Metrics exporters (Prometheus/OpenTelemetry) are absent.
- **Production Impact:** Operators cannot trace request execution across logs, measure HTTP request rates/latencies, or diagnose production failures.

### 1.14 Test Coverage, Test Quality, Edge Cases, and Regression Risk
- **Current State:** Excellent unit test suites exist for `sitolo-auth`, `sitolo-config`, `sitolo-tenancy`, and `sitolo-security`.
- **Concrete Weakness:** PostgreSQL RLS tests in `sitolo-persistence/tests/rls_security_tests.rs` fail completely when run without a live PostgreSQL instance (`ADMIN_DATABASE_URL`). End-to-end API integration tests do not test authenticated multi-tenant workflows.
- **Production Impact:** CI builds fail when database containers are unavailable; core multi-tenant security boundaries are not tested in continuous integration.

### 1.15 Configuration Management, Environment Separation, and Secrets Handling
- **Current State:** `sitolo-config` enforces strict environment hierarchy, fingerprinting, and non-secret configuration parsing.
- **Concrete Weakness:** Production database credentials and secret key rotation mechanisms are not integrated with external vault managers (e.g., AWS Secrets Manager or HashiCorp Vault).
- **Production Impact:** Secret rotation requires full application restarts with environment variable updates; risk of plain-text secret exposure in environment variables.

### 1.16 Deployment Safety, Rollback Readiness, Versioning, and Release Discipline
- **Current State:** Dockerfiles exist in `apps/api` and `apps/worker`. Toolchains are pinned to Rust 1.98.1.
- **Concrete Weakness:** No zero-downtime deployment strategy, health check readiness probe validation, or database rollback scripts exist.
- **Production Impact:** Deploying updates requires service downtime and risks unrecoverable database state corruption during failed migrations.

### 1.17 Code Quality, Naming, Duplication, Coupling, Dead Code, and Technical Debt
- **Current State:** `#![forbid(unsafe_code)]` and strict Clippy lints are enabled.
- **Concrete Weakness:** Conditionally compiled modules in `sitolo-persistence` produce `dead_code` compiler warnings when building non-test binaries. Hand-written HTTP parsing in `apps/api` duplicates standard library/framework functionality brittlely.
- **Production Impact:** Dead code creates maintainability confusion; custom HTTP parsers accumulate technical debt and edge-case bugs.

### 1.18 Maintainability Under Team Growth, Code Ownership, and Refactor Cost
- **Current State:** Clean crate boundaries with explicit `Cargo.toml` manifests.
- **Concrete Weakness:** Absence of core domain implementations forces future developers to write all business logic from scratch, increasing refactor costs and divergence risk across teams.
- **Production Impact:** Onboarding delay and high risk of inconsistent domain logic implementations across team members.

### 1.19 Compliance and Enterprise Operational Expectations
- **Current State:** `docs/mra_eis_integration_spec.md` documents regulatory tax integration requirements for Malawi Revenue Authority (MRA EIS).
- **Concrete Weakness:** No MRA EIS signing, counter, or transmission code exists in source tree. Financial audit logs are not stored immutably.
- **Production Impact:** Merchants using Sitolo cannot comply with regional tax authority regulations, risking legal penalties and operational closure.

---

## 2. Top 10 Highest-Risk Issues

| Rank | Issue ID | Description | Location | Primary Failure Mode | Business & Engineering Impact |
|---|---|---|---|---|---|
| **1** | **CRIT-01** | Hand-written plain TCP request parser without TLS or packet framing | `apps/api/src/serve.rs` | TCP packet fragmentation drops requests; plain-text transport leaks secrets | **CRITICAL:** Total transport vulnerability, MITM attacks, data corruption |
| **2** | **CRIT-02** | Unauthenticated HTTP API endpoints for tenancy & organization lifecycle | `apps/api/src/serve.rs` | Anonymous callers mutate organization, branch, and membership state | **CRITICAL:** Multi-tenant compromise, unauthorized state modification |
| **3** | **CRIT-03** | Missing core business domain modules (Catalogue, Inventory, POS, Cash, Tax) | `crates/sitolo-domain/src` | System cannot process sales, track inventory, or calculate totals | **CRITICAL:** Product is non-functional as a Business Operating System |
| **4** | **CRIT-04** | Empty worker process and non-functional asynchronous outbox queue | `apps/worker/src/main.rs` | Outbox records accumulate indefinitely; webhooks/jobs never run | **CRITICAL:** Eventual consistency breakdown, background processing failure |
| **5** | **CRIT-05** | Lack of versioned PostgreSQL database schema migrations | `crates/sitolo-persistence` | Production database cannot be provisioned or updated | **CRITICAL:** Unusable in real database environments |
| **6** | **HIGH-01** | Unimplemented integration adapters for payment gateways and MRA EIS | `crates/sitolo-integrations` | Payment capture and fiscal tax invoice generation fail | **HIGH:** Inability to collect revenue or comply with tax law |
| **7** | **HIGH-02** | Non-existent offline device synchronization engine | `crates/sitolo-sync` | Offline POS terminals cannot sync sales or inventory with server | **HIGH:** Core offline-first value proposition broken |
| **8** | **HIGH-03** | Persistent storage disconnected from API (API runs on volatile in-memory DB) | `crates/sitolo-persistence` | Process restart wipes all organizations, branches, and user records | **HIGH:** Complete data loss on process restart or deploy |
| **9** | **HIGH-04** | Observability context and tracing spans unattached to HTTP request flow | `apps/api/src/serve.rs` | Inability to trace requests, log correlation IDs, or monitor latency | **HIGH:** Zero operational visibility during production outages |
| **10** | **HIGH-05** | Absence of client frontend applications (Flutter Mobile & Tauri Desktop) | Repository root | No user interface exists for merchants or sales cashiers | **HIGH:** Unusable product for end users |

---

## 3. Top 10 Highest-Leverage Fixes

| Rank | Leverage Fix | Target Component | Engineering Effort | Primary Benefit |
|---|---|---|---|---|
| **1** | Replace custom TCP parser in `apps/api` with **Axum + hyper + rustls** | `apps/api` | **Low (2-3 days)** | Instantly resolves transport safety, TLS encryption, HTTP/1.1 framing, and request streaming. |
| **2** | Implement **Axum Authentication & Tenancy Middleware** | `apps/api`, `sitolo-api` | **Low (2 days)** | Enforces token extraction, session validation, and tenant scope check on all routes. |
| **3** | Establish **SQLx Versioned Migrations** (`migrations/`) | `sitolo-persistence` | **Low (2 days)** | Moves database schema from test fixtures to version-controlled production migrations. |
| **4** | Wire **PostgreSQL Repositories** into Application Services | `sitolo-persistence`, `apps/api` | **Medium (4-5 days)** | Achieves true durable data persistence across process restarts. |
| **5** | Implement **Async Worker Loop & Outbox Consumer** | `apps/worker`, `sitolo-events` | **Medium (1 week)** | Enables background job processing, retries, and outbox event dispatching. |
| **6** | Develop Core Domain Engine for **Product Catalogue & Inventory Ledger** | `sitolo-domain` | **Medium (2 weeks)** | Delivers authoritative stock movements, SKU tracking, and catalogue management. |
| **7** | Implement Domain Engine for **POS Sales & Cash Register Closing** | `sitolo-domain` | **Medium (2 weeks)** | Enables atomic sale finalization, payment intent linking, and cash balancing. |
| **8** | Implement **Payment Provider & MRA EIS Adapters** | `sitolo-integrations` | **Medium (2 weeks)** | Unlocks payment processing (Airtel/TNM) and fiscal tax compliance. |
| **9** | Build **Offline Client Synchronization Protocol** | `sitolo-sync` | **High (3 weeks)** | Enables robust offline continuity and reconciliation for mobile/desktop apps. |
| **10** | Attach **Tracing Spans & Prometheus Metrics** to API Handlers | `apps/api`, `sitolo-observability` | **Low (3 days)** | Provides complete request tracing, error rate monitoring, and latency visibility. |

---

## 4. Phased Remediation Plan

```text
REMEDIATION TIMELINE:

  Month 1: P0 Critical Blockers (Axum, Auth Middleware, SQLx Migrations, Core Domains)
  Month 2: P1 High Priority (Workers, Persistence Repos, Integrations, Sync Engine)
  Month 3: P2 Architectural Depth (Observability, Caching, DoS Limits, Client Scaffolds)
  Months 4-6: P3 Operational Scale (Load Testing, Certification, Client POLISH, DR Drills)
```

### Phase 1: Immediate Remediation (Weeks 1–2 / P0 Critical Blockers)
- **Goal:** Resolve architectural divergences, secure transport, and establish database migrations.
  1. **HTTP Transport Overhaul:** Refactor `apps/api` to use `Axum` framework with `Tokio` runtime.
  2. **Authentication Middleware:** Implement Axum middleware that extracts bearer tokens, validates sessions with `sitolo-auth`, and populates `SecurityContext`.
  3. **Database Migrations:** Create `crates/sitolo-persistence/migrations/` containing version-controlled SQL scripts derived from `rls_schema.sql`.
  4. **Unconditional PostgreSQL Export:** Update `crates/sitolo-persistence/src/lib.rs` to expose `pub mod postgres;` unconditionally.

### Phase 2: Short-Term Remediation (Weeks 3–6 / P1 High Priority)
- **Goal:** Implement core business domains, durable persistence, and background worker processing.
  1. **Domain Engine Implementation:** Implement domain aggregates in `crates/sitolo-domain`: `catalogue`, `inventory`, `sales`, `cash`, `payments`.
  2. **PostgreSQL Repositories:** Implement SQLx-backed persistence repositories implementing domain ports in `sitolo-persistence`.
  3. **Worker Processing Loop:** Implement worker polling loop in `apps/worker` with `SKIP LOCKED` outbox consumption.
  4. **Integration Adapters:** Implement payment and MRA EIS integration adapters in `sitolo-integrations`.

### Phase 3: Medium-Term Remediation (Months 2–3 / P2 Architectural Depth)
- **Goal:** Complete offline sync, client applications, and full observability.
  1. **Sync Protocol Engine:** Implement offline synchronization protocol in `crates/sitolo-sync`.
  2. **Observability Integration:** Attach `tracing` spans, correlation IDs, and Prometheus metrics to all HTTP routes and worker jobs.
  3. **Client Application Scaffolds:** Initialize Flutter mobile (`apps/mobile`) and Tauri desktop (`apps/desktop`) shells with local SQLite persistence.
  4. **Rate Limiting & DoS Protection:** Wire token-bucket rate limiting middleware into Axum routes.

### Phase 4: Long-Term Remediation (Months 4–6 / P3 Operational Scale & Production Readiness)
- **Goal:** Production hardening, certification, and enterprise scaling.
  1. **MRA EIS Regulatory Certification:** Complete sandbox and compliance testing with MRA tax servers.
  2. **Disaster Recovery & Load Testing:** Perform failover drills, point-in-time database restores, and high-concurrency load testing.
  3. **Security Penetration Audit:** Conduct independent third-party penetration testing and multi-tenant isolation verification.

---

## 5. Enterprise-Grade Evaluation Matrix

| Category | Current Implementation State | Production Enterprise Requirement | Status |
|---|---|---|---|
| **API Framework** | Hand-written TCP parser (`apps/api/src/serve.rs`) | Rust + Axum + hyper + Tokio with TLS termination | **NOT ENTERPRISE GRADE** |
| **API Authentication** | Handlers execute without session extraction | Mandatory Bearer token validation & `SecurityContext` | **NOT ENTERPRISE GRADE** |
| **Multi-Tenant Isolation** | In-memory filtering + test-only RLS fixtures | PostgreSQL Row-Level Security (RLS) + DB session context | **PARTIAL / NOT ENTERPRISE GRADE** |
| **Business Logic** | Only Tenancy entities exist in `sitolo-domain` | Complete Catalogue, Inventory Ledger, POS, Cash, Tax engines | **NOT ENTERPRISE GRADE** |
| **Persistence Layer** | Volatile in-memory HashMaps | Versioned SQLx migrations + PostgreSQL transaction pools | **NOT ENTERPRISE GRADE** |
| **Background Processing** | Empty scaffold binary (`apps/worker`) | Outbox table polling (`SKIP LOCKED`) + DLQ + Idempotency | **NOT ENTERPRISE GRADE** |
| **Offline Synchronization** | Empty crate stub (`sitolo-sync`) | Domain-aware batch sync, state versioning, conflict resolution | **NOT ENTERPRISE GRADE** |
| **External Integrations** | Empty crate stub (`sitolo-integrations`) | Retrying, circuit-broken adapters for Payments & MRA EIS | **NOT ENTERPRISE GRADE** |
| **Observability** | Context types exist; unattached to HTTP pipeline | Distributed tracing spans, request correlation IDs, Prometheus metrics | **PARTIAL / NOT ENTERPRISE GRADE** |
| **Password Hashing** | Argon2id in `sitolo-auth` | Argon2id offloaded to blocking thread pools | **ACCEPTABLE PRIMITIVES** |
| **Configuration** | Fingerprinted, hierarchical in `sitolo-config` | Environment hierarchy, zero secret leakage, pinned toolchains | **ENTERPRISE GRADE** |
| **Code Hygiene** | `#![forbid(unsafe_code)]` enabled workspace-wide | Zero unsafe code, strict linting, clean dependency graph | **ENTERPRISE GRADE** |

---

## 6. Severity-Ranked Findings List

### Critical Severity Findings (P0 / Immediate Release Blockers)

#### FINDING CRIT-01: Direct TCP Manual HTTP Parsing Without Transport Security (TLS) or Framing
- **What is wrong:** `apps/api/src/serve.rs` implements HTTP server functionality by opening a raw `TcpListener` and reading bytes into a fixed 32KB buffer (`stream.read(&mut buf)`). It parses headers using string slicing (`raw.windows(4).position(...)`) and handles body extraction manually. It does not support TLS encryption or HTTP framing.
- **Why it is wrong:** The system architecture specification explicitly requires Rust + Axum + Tokio. Hand-rolled HTTP request parsers suffer from buffer overflow vulnerabilities, request smuggling risks, incomplete packet read bugs under network jitter, and lack TLS stream encryption.
- **Where it appears:** `apps/api/src/serve.rs` (`serve`, `handle_connection`, `dispatch_request`), `apps/api/Cargo.toml`.
- **How it fails in production:**
  1. Requests whose headers or JSON bodies are fragmented across two TCP packets are read partially, causing `serde_json::from_str` to fail and drop valid requests.
  2. All network traffic (passwords, bearer tokens, customer data) is transmitted in cleartext over the wire, exposing data to network sniffing.
  3. Attacker sending malformed HTTP headers can cause panics or thread exhaustion in `handle_connection`.
- **What a proper fix looks like:** Re-architect `apps/api` to use `axum::Router`, `hyper`, and `tokio-rustls`. Replace manual socket loops with standard Axum request extractors and router handlers.
- **Priority & Blast Radius:** Priority P0 (Critical). Blast Radius: Entire API transport boundary and system security.

#### FINDING CRIT-02: Unauthenticated HTTP API Routes for Multi-Tenant Lifecycle Operations
- **What is wrong:** `dispatch_request` in `apps/api/src/serve.rs` routes HTTP requests directly to tenancy handlers (`handle_provision_organization`, `handle_create_branch`, `handle_suspend_organization`, etc.) without requiring or validating an `Authorization` header.
- **Why it is wrong:** `CreateOrganizationRequest` carries client-provided fields (`owner_user_id`, `owner_membership_id`). In production, user identity MUST be derived exclusively from a verified `SecurityContext` generated by authentication middleware.
- **Where it appears:** `apps/api/src/serve.rs` (`dispatch_request`), `crates/sitolo-api/src/tenancy.rs` (`CreateOrganizationRequest`), `crates/sitolo-api/src/auth.rs`.
- **How it fails in production:** An unauthenticated internet actor can send an HTTP POST to `/v1/organizations` or `/v1/organizations/{id}/suspend` and manipulate organization or branch states for any tenant across the system.
- **What a proper fix looks like:** Implement Axum authentication middleware that extracts Bearer tokens, validates sessions via `sitolo-auth`, establishes a `SecurityContext`, and enforces tenant membership policies before invoking handler functions.
- **Priority & Blast Radius:** Priority P0 (Critical). Blast Radius: Whole multi-tenant authorization boundary and access control system.

#### FINDING CRIT-03: Total Absence of Core Business Domain Engines
- **What is wrong:** `crates/sitolo-domain/src/lib.rs` exports only `pub mod tenancy;`. The domain crate contains no modules or aggregates for product catalogue, pricing rules, procurement, inventory ledgers, sales transactions, cash register closing, payment processing, reconciliation, or tax calculation.
- **Why it is wrong:** Sitolo is intended to function as an enterprise Business Operating System for SMEs. Without core business domain models, the software cannot execute commercial workflows.
- **Where it appears:** `crates/sitolo-domain/src/lib.rs`, `crates/sitolo-domain/src/`.
- **How it fails in production:** The application cannot record stock receipts, finalize point-of-sale transactions, reconcile cash drawers, or file tax invoices. The system is unusable as a business application.
- **What a proper fix looks like:** Implement domain aggregates, value objects, state transition rules, and domain events across `catalogue`, `inventory`, `sales`, `cash`, `payments`, `reconciliation`, and `tax` modules per Phase 8–18 specifications.
- **Priority & Blast Radius:** Priority P0 (Critical). Blast Radius: Complete application business functionality.

#### FINDING CRIT-04: Non-Functional Worker Binary & Background Job Infrastructure
- **What is wrong:** `apps/worker/src/main.rs` contains an empty `fn main() {}` function. `crates/sitolo-events/src/lib.rs` contains only doc comments.
- **Why it is wrong:** The architecture relies on an asynchronous transactional outbox pattern to process background tasks (MRA EIS tax submissions, payment webhook dispatch, asynchronous notification delivery, reconciliation jobs).
- **Where it appears:** `apps/worker/src/main.rs`, `crates/sitolo-events/src/lib.rs`.
- **How it fails in production:** Outbox records written to database tables during API transactions accumulate indefinitely. Asynchronous side-effects are never executed, breaking eventual consistency guarantees.
- **What a proper fix looks like:** Build a polling worker loop in `apps/worker` using Tokio and SQLx, claiming outbox records via `SELECT ... FOR UPDATE SKIP LOCKED` with explicit tenant scoping, exponential backoff retries, and dead-letter queue (DLQ) isolation.
- **Priority & Blast Radius:** Priority P0 (Critical). Blast Radius: Asynchronous processing, outbox reliability, and external integrations.

#### FINDING CRIT-05: Missing Versioned PostgreSQL Schema Migrations
- **What is wrong:** PostgreSQL database schema definitions exist exclusively as an inline string literal inside an integration test file (`crates/sitolo-persistence/tests/fixtures/rls_schema.sql`). No `migrations/` directory or SQLx migration pipeline exists.
- **Why it is wrong:** Database changes cannot be deployed, version-tracked, or rolled back in real environments. The application cannot initialize a production PostgreSQL database.
- **Where it appears:** `crates/sitolo-persistence/tests/fixtures/rls_schema.sql`, `crates/sitolo-persistence/src/postgres.rs`.
- **How it fails in production:** Provisioning a new production environment fails because there are no automated database migration scripts. Schema evolution across application updates is impossible.
- **What a proper fix looks like:** Establish a versioned SQLx migration directory (`crates/sitolo-persistence/migrations/0001_init_iam.sql`, etc.) and embed them into the binary using `sqlx::migrate!`.
- **Priority & Blast Radius:** Priority P0 (Critical). Blast Radius: Persistence integrity, deployment readiness, and database evolution.

---

### High Severity Findings (P1 / High Priority)

#### FINDING HIGH-01: Empty External Integration Adapters Crate
- **What is wrong:** `crates/sitolo-integrations/src/lib.rs` contains only module documentation and no implementation code.
- **Why it is wrong:** Third-party integrations (Airtel Money, TNM Mpamba, Malawi Revenue Authority EIS, SMS gateways) must be isolated behind strongly-typed adapter ports.
- **Where it appears:** `crates/sitolo-integrations/src/lib.rs`, `crates/sitolo-integrations/Cargo.toml`.
- **How it fails in production:** Payment confirmation callbacks cannot be processed; fiscal receipts cannot be generated or transmitted to regulatory authorities.
- **What a proper fix looks like:** Implement adapter modules in `sitolo-integrations` implementing domain ports, including request serialization, response validation, HTTP timeouts, circuit breakers, and payload sanitization.
- **Priority & Blast Radius:** Priority P1 (High). Blast Radius: Revenue processing, regulatory compliance, and partner integrations.

#### FINDING HIGH-02: Non-Existent Offline Client Synchronization Engine
- **What is wrong:** `crates/sitolo-sync/src/lib.rs` is an empty crate stub.
- **Why it is wrong:** Sitolo's core architecture requires offline operational continuity for mobile and desktop POS terminals operating over unstable regional networks.
- **Where it appears:** `crates/sitolo-sync/src/lib.rs`, `crates/sitolo-sync/Cargo.toml`.
- **How it fails in production:** Offline client terminals operate in complete isolation. Sales transactions or stock movements executed offline can never be uploaded or reconciled with authoritative server state.
- **What a proper fix looks like:** Implement a synchronization engine handling batch transaction upload, state version validation, idempotency checks, conflict resolution, and server acceptance/rejection response framing.
- **Priority & Blast Radius:** Priority P1 (High). Blast Radius: Offline-first terminal reliability and state synchronization.

#### FINDING HIGH-03: Application Services Unlinked From PostgreSQL Persistence
- **What is wrong:** `crates/sitolo-persistence/src/lib.rs` conditionally compiles `pub(crate) mod postgres;` only during test execution. `TenancyService` in `sitolo-application` relies on `TenancyDatabase` (in-memory `RwLock` HashMaps).
- **Why it is wrong:** Production state changes must persist to authoritative PostgreSQL storage. In-memory storage is appropriate only for unit tests and local prototypes.
- **Where it appears:** `crates/sitolo-persistence/src/lib.rs`, `crates/sitolo-application/src/tenancy.rs`.
- **How it fails in production:** Every API process restart wipes all organizations, branches, and user memberships. All operational state is completely volatile.
- **What a proper fix looks like:** Export `pub mod postgres;` unconditionally, implement SQLx database repositories for `TenancyStores` and `IdentityStores`, and inject PostgreSQL pools into application services during bootstrap.
- **Priority & Blast Radius:** Priority P1 (High). Blast Radius: Data durability, persistence authority, and operational reliability.

#### FINDING HIGH-04: Unattached Observability Context and Request Correlation
- **What is wrong:** `apps/api/src/serve.rs` does not instantiate tracing spans or attach `RequestId` / `TraceParent` context to incoming HTTP requests.
- **Why it is wrong:** Distributed tracing and structured request correlation are required for diagnosing production failures, latency bottlenecks, and security anomalies.
- **Where it appears:** `apps/api/src/serve.rs`, `crates/sitolo-observability/src/lib.rs`.
- **How it fails in production:** Log output lacks request correlation IDs. Operators cannot trace request execution flow across API handlers, domain services, database calls, and background jobs.
- **What a proper fix looks like:** Attach `tracing::info_span!` middleware to the Axum router, extracting or generating `RequestId` and propagating context across all asynchronous tasks.
- **Priority & Blast Radius:** Priority P1 (High). Blast Radius: Observability, incident response, and diagnosability.

#### FINDING HIGH-05: Total Absence of Frontend Client Applications
- **What is wrong:** No client source code exists in the repository for Flutter Mobile (`apps/mobile`) or Tauri Desktop (`apps/desktop`).
- **Why it is wrong:** The system design relies on client applications as operational surfaces for retail merchants, cashiers, and warehouse operators.
- **Where it appears:** Repository workspace root, `apps/`.
- **How it fails in production:** End users have no frontend client interface to operate point-of-sale terminals, manage catalogue items, or perform cash closes.
- **What a proper fix looks like:** Initialize Flutter and Tauri client project structures with local SQLite persistence and sync integration layers.
- **Priority & Blast Radius:** Priority P1 (High). Blast Radius: User accessibility and end-to-end operational viability.

---

### Medium Severity Findings (P2 / Medium Priority)

#### FINDING MED-01: In-Memory Synchronization Locking Limits Horizontal Scaling
- **What is wrong:** `crates/sitolo-persistence/src/memory.rs` uses `std::sync::RwLock` around in-memory `HashMap` structures for tenancy and identity records.
- **Why it is wrong:** In-memory locks provide thread safety only within a single OS process. They cannot synchronize state across multiple API server instances.
- **Where it appears:** `crates/sitolo-persistence/src/memory.rs` (`TenancyDatabase`, `IdentityDatabase`).
- **How it fails in production:** Deploying multiple API server replicas behind a load balancer leads to state divergence and race conditions because each replica maintains isolated in-memory state.
- **What a proper fix looks like:** Transition all persistence ports to PostgreSQL repositories using ACID transactions and `SELECT ... FOR UPDATE` row locks.
- **Priority & Blast Radius:** Priority P2 (Medium). Blast Radius: Horizontal scalability and multi-instance deployment stability.

#### FINDING MED-02: Missing Local Container Fallback for PostgreSQL Integration Tests
- **What is wrong:** `crates/sitolo-persistence/tests/rls_security_tests.rs` panics immediately with `FAIL-CLOSED: Required ADMIN_DATABASE_URL or DATABASE_URL env variable not provided` when executed without an external PostgreSQL instance.
- **Why it is wrong:** Integration test suites should gracefully skip or spin up local testcontainers when database environment variables are not set, preventing local developer build failures.
- **Where it appears:** `crates/sitolo-persistence/tests/rls_security_tests.rs`.
- **How it fails in production:** Running `cargo test --workspace` on a fresh developer machine fails entirely unless a PostgreSQL instance is manually configured.
- **What a proper fix looks like:** Integrate `testcontainers-rs` or implement conditional test skipping with clear setup instructions when database environment variables are absent.
- **Priority & Blast Radius:** Priority P2 (Medium). Blast Radius: Developer ergonomics and CI build reliability.

#### FINDING MED-03: Lack of HTTP Transport Rate Limiting Middleware
- **What is wrong:** Although `sitolo-auth` provides token-bucket rate limiter algorithms (`ratelimit.rs`), no rate limiting middleware is attached to the HTTP request pipeline in `apps/api/src/serve.rs`.
- **Why it is wrong:** Unthrottled API endpoints are vulnerable to Denial of Service (DoS) attacks and brute-force credential guessing.
- **Where it appears:** `apps/api/src/serve.rs`, `crates/sitolo-auth/src/ratelimit.rs`.
- **How it fails in production:** Attackers can flood authentication or tenancy routes with high request volumes, consuming server CPU and memory.
- **What a proper fix looks like:** Attach rate limiting middleware (e.g. `tower-governor` or custom token-bucket layer) to Axum routes, returning RFC 7807 `429 Too Many Requests` responses when limits are exceeded.
- **Priority & Blast Radius:** Priority P2 (Medium). Blast Radius: Application availability and DoS resistance.

#### FINDING MED-04: Absence of Automated OpenAPI / API Spec Generation
- **What is wrong:** API contracts are documented manually in Markdown (`docs/api_contract.md`). No machine-readable OpenAPI (Swagger) schema files are generated by the API codebase.
- **Why it is wrong:** Manual documentation leads to schema drift between backend endpoints and frontend client models.
- **Where it appears:** `crates/sitolo-api`, `docs/api_contract.md`.
- **How it fails in production:** Frontend client developers write manual DTO bindings, introducing serialization errors and type mismatches.
- **What a proper fix looks like:** Use `utoipa` or `aide` crates in `sitolo-api` to generate OpenAPI 3.1 JSON/YAML specifications automatically during build and CI verification.
- **Priority & Blast Radius:** Priority P2 (Medium). Blast Radius: Frontend-backend contract alignment.

---

### Low Severity Findings (P3 / Low Priority)

#### FINDING LOW-01: Historical Documentation Drift in Remediation Plans
- **What is wrong:** Files such as `docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md` describe older repository baselines and contain historical status statements that no longer match the current codebase.
- **Why it is wrong:** Stale documentation creates confusion for human developers and automated coding agents attempting to navigate project status.
- **Where it appears:** `docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md`.
- **How it fails in production:** Developers or agents rely on outdated design assumptions, leading to unnecessary rework or misaligned implementations.
- **What a proper fix looks like:** Mark historical phase documents with an explicit `[HISTORICAL / DEPRECATED]` header banner and archive them from active routing trees.
- **Priority & Blast Radius:** Priority P3 (Low). Blast Radius: Documentation readability and team clarity.

#### FINDING LOW-02: Missing Automatic Git Pre-Commit Hook Configuration Script
- **What is wrong:** Verification scripts exist in `scripts/ci/verify`, but no automated script exists to install pre-commit Git hooks in developer environments.
- **Why it is wrong:** Developers may forget to run `./scripts/ci/verify` before pushing branches, causing avoidable CI pipeline failures.
- **Where it appears:** `scripts/`, `.git/hooks/`.
- **How it fails in production:** Pull requests are submitted with minor formatting or linting errors, delaying CI build checks.
- **What a proper fix looks like:** Provide a `scripts/install-hooks.sh` script that links `./scripts/ci/verify` to `.git/hooks/pre-commit`.
- **Priority & Blast Radius:** Priority P3 (Low). Blast Radius: Developer workflow convenience.

---

## 7. Audit Final Conclusion

The Sitolo codebase possesses a **well-architected security and configuration foundation** with clean Cargo crate separation, strict linting (`#![forbid(unsafe_code)]`), and robust authentication primitives.

However, in its current state, **it is NOT production-ready**.

The system must be bridged from a security prototype into a complete enterprise application by executing four crucial remediation steps:
1. Migrating `apps/api` to **Axum + hyper + rustls** with mandatory authentication middleware.
2. Implementing the **core business domain engines** (`catalogue`, `inventory`, `sales`, `cash`, `payments`, `tax`).
3. Deploying **versioned SQLx PostgreSQL migrations** and linking persistent database repositories to application services.
4. Implementing the **asynchronous background worker process** (`apps/worker`) and **offline sync engine** (`sitolo-sync`).

Executing these phased recommendations will bring Sitolo into full compliance with enterprise operational expectations and complete its transformation into a reliable Business Operating System for African SMEs.
