# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main` at commit `e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc`
**Status:** Current-state comprehensive enterprise audit
**Scope:** Full end-to-end repository, architecture, security, performance, reliability, and operations review

---

## 1. Executive Summary & Production Readiness Posture

Sitolo is currently **NOT production-ready**.

The repository exhibits an impressive, highly disciplined foundation of security specifications, domain modeling, configuration management, and tenancy isolation primitives. Crate boundaries follow strict clean-architecture rules (`sitolo-security` -> `sitolo-auth` -> `sitolo-tenancy` -> `sitolo-authz` -> `sitolo-application` -> `sitolo-api`), and CI scripts enforce forbidden `unsafe` code, lockfile hygiene, and dependency direction.

However, the repository currently operates as a **partially connected platform foundation** with major implementation gaps at the HTTP transport, worker, and database runtime layers. Crucially, the system lacks production HTTP authentication enforcement, lacks a production password hashing algorithm, lacks an asynchronous worker engine, and lacks operational business domain engines (product catalogue, inventory ledger, sales/POS, payments, MRA EIS compliance, reconciliation, and reporting).

```text
+-----------------------------------------------------------------------+
|                         CURRENT SYSTEM POSTURE                        |
+-----------------------------------------------------------------------+
|  [ SPECIFICATIONS & CONTRACTS ]  --> Enterprise-Grade / Complete     |
|  [ SECURITY & IAM PRIMITIVES ]   --> Strong Foundation / Partial      |
|  [ HTTP & NETWORK TRANSPORT ]    --> Critical Incomplete Gaps         |
|  [ DATABASE & PERSISTENCE ]      --> Memory-Backed Default / Incomplete|
|  [ WORKER & ASYNC ENGINES ]      --> Scaffold Only / Unimplemented    |
|  [ BUSINESS DOMAIN ENGINES ]     --> Missing Runtime Implementations  |
+-----------------------------------------------------------------------+
```

### Overall Health Summary
- **Architecture & Design:** Grade B+. Modular monolith design is well-conceived, but transport layer diverges from specifications.
- **Security & IAM:** Grade C+. Primitives (MFA, session sliding, PKCE, bounded types) are well-designed, but HTTP endpoints lack auth wiring and password hashing uses a test double.
- **Data Persistence & Isolation:** Grade C. PostgreSQL authority and RLS verification exist, but runtime application handlers fallback to in-memory non-transactional state.
- **Reliability & Background Execution:** Grade F. Background worker is an empty scaffold; no asynchronous outbox or job runner exists.
- **Business Capability Completeness:** Grade D. Only organizational tenancy management is partially implemented. Core commercial workflows do not exist.

---

## 2. End-to-End System Evaluation Across 19 Enterprise Domains

### 2.1 Repository Structure and Dependency Boundaries
- **Status:** Satisfactory Foundation with Divergence.
- **Evaluation:** Workspace hierarchy in `Cargo.toml` separates concerns logically into `apps/` (`api`, `worker`) and `crates/` (`sitolo-api`, `sitolo-application`, `sitolo-auth`, `sitolo-authz`, `sitolo-tenancy`, `sitolo-persistence`, `sitolo-security`, `sitolo-observability`, `sitolo-config`, `sitolo-domain`, `sitolo-events`, `sitolo-integrations`, `sitolo-sync`).
- **Deficiencies:** The `apps/api` transport layer directly invokes `sitolo_api::tenancy::handle_*` which routes directly to an in-memory repository, bypassing `sitolo-application` workflow orchestration. `apps/worker` lacks required workspace dependencies on `sitolo-events` and `sitolo-persistence`.

### 2.2 Application Architecture and Module Separation
- **Status:** Partial.
- **Evaluation:** Application service traits are properly defined in `sitolo-application`. Tenancy and IAM lifecycle orchestration are structured cleanly.
- **Deficiencies:** Domain logic in `sitolo-domain` is restricted almost entirely to tenancy structures (`tenancy.rs`). Core domain models for Catalogue, Inventory, Sales, Payments, and Fiscalization are entirely missing or consist of empty placeholder crates (`sitolo-integrations`, `sitolo-sync`, `sitolo-events`).

### 2.3 API Design, Request Flow, and Trust Boundaries
- **Status:** Unacceptable for Production.
- **Evaluation:** `crates/sitolo-api` contains well-bounded DTOs (`bounds::MAX_TENANCY_BODY_BYTES`), `deny_unknown_fields` attributes, and RFC 7807 `ProblemDetails` error formatting.
- **Deficiencies:** `apps/api/src/serve.rs` bypasses standard web frameworks (Axum/Tower) and parses raw TCP socket streams using custom string operations (`split_whitespace()`, `split('/')`). Request flow lacks HTTP request buffer assembly, header normalization, content-type checks, and bearer token extraction on tenancy routes.

### 2.4 Authentication, Authorization, and Session Handling
- **Status:** Unacceptable for Production.
- **Evaluation:** `sitolo-auth` provides state-of-the-art session sliding, MFA state machines, device registration, PKCE verification, and scope derivation.
- **Deficiencies:** None of the authentication or authorization middleware is wired into `apps/api/src/serve.rs`. Every HTTP route in `dispatch_request` is publicly accessible without token verification or session context establishment. Furthermore, `PasswordHasher` is only implemented by `TestPasswordHasher` (iterated SHA-256), with zero production Argon2id implementation present.

### 2.5 Input Validation, Sanitization, and Data Integrity
- **Status:** Partial.
- **Evaluation:** DTOs strictly limit string lengths and enforce non-empty invariants using custom wrapper types. Request body sizes are capped before JSON parsing.
- **Deficiencies:** Path parameters in HTTP URLs (e.g., `org_id`, `branch_id`) are parsed as raw strings in `serve.rs` without URL decoding, character set sanitization, or early UUID formatting checks.

### 2.6 Injection Risks, XSS, CSRF, SSRF, IDOR, Path Traversal, and Secrets Handling
- **Status:** Critical Security Risk.
- **Evaluation:** Prepared statements are mandated in `sitolo-persistence`, and `SecretValue` prevents sensitive data logging.
- **Deficiencies:** Complete lack of authentication on `POST /v1/organizations/{org_id}/suspend` and `/close` endpoints creates an catastrophic **IDOR / Authorization Bypass** vulnerability. Path splitting on raw TCP strings (`split('/')`) is vulnerable to path traversal and routing ambiguity if percent-encoded characters or query strings are passed.

### 2.7 Error Handling, Retry Behavior, Timeout Strategy, and Failure Isolation
- **Status:** Partial.
- **Evaluation:** `AppError` provides clear taxonomy, mapping domain errors to HTTP status codes without leaking internal trace details.
- **Deficiencies:** TCP connection processing in `serve.rs` enforces a 5-second timeout on initial socket reads, but lacks read/write timeouts during response writing. Downstream database operations in `sitolo-persistence` lack retry policies, circuit breakers, or exponential backoff handling.

### 2.8 CPU-Bound versus I/O-Bound Bottlenecks
- **Status:** Hazardous Execution Model.
- **Evaluation:** Non-blocking async I/O is used throughout Tokio tasks.
- **Deficiencies:** Password hashing (`TestPasswordHasher`) runs directly within Tokio async thread contexts. When Argon2id is implemented, running CPU-intensive Argon2id hashing on async worker threads without `tokio::task::spawn_blocking` will stall the event loop under concurrent login loads.

### 2.9 Async Behavior, Blocking Operations, Race Conditions, and Concurrency
- **Status:** Partial / High Risk.
- **Evaluation:** Tokio multi-threaded runtime is configured with structured shutdown signals.
- **Deficiencies:** `apps/api/src/serve.rs` uses `semaphore.clone().try_acquire_owned()`. When connection limits (1024) are exceeded, incoming TCP connections are silently dropped without returning `503 Service Unavailable`. In-memory repositories use `std::sync::Mutex` which can cause thread blocking across Tokio tasks.

### 2.10 Database Design, Query Efficiency, Transactions, Migrations, and RLS
- **Status:** Incomplete Execution Boundary.
- **Evaluation:** `sitolo-persistence` contains PostgreSQL catalog verification (`verify_runtime_role`, `verify_rls_catalog_metadata`) to guarantee `app_runtime` cannot bypass RLS.
- **Deficiencies:** Domain repositories currently use `InMemoryTenancyRepository` in `apps/api`. PostgreSQL table creation scripts and RLS enforcement logic exist in tests, but production application handlers do not execute within PostgreSQL `Transaction` blocks or call `set_transaction_tenant_context()`.

### 2.11 Caching Strategy, Invalidation Logic, and Consistency Tradeoffs
- **Status:** Unimplemented.
- **Evaluation:** In-memory JWKS public key caching exists with stampede protection.
- **Deficiencies:** No distributed caching layer (Redis) is integrated for session state, authorization scope evaluation, or product catalogue lookups.

### 2.12 Queueing, Background Jobs, Event Handling, and Idempotency
- **Status:** Non-existent (Scaffold Only).
- **Evaluation:** Event structures are defined in `sitolo-audit`.
- **Deficiencies:** `apps/worker/src/main.rs` is an empty `fn main() {}`. Outbox table processing, asynchronous event publishing, retries, dead-letter queues (DLQ), and external provider synchronization do not exist at runtime.

### 2.13 Observability: Logs, Metrics, Traces, Alertability, and Diagnosability
- **Status:** Substrate Implemented, Unwired.
- **Evaluation:** `sitolo-observability` features bounded ring buffers, `RequestId`, `TraceParent`, and structured redaction.
- **Deficiencies:** Metrics collection (Prometheus endpoints) and OpenTelemetry exporter wiring are missing in `apps/api`. Request contexts are not attached to Tokio tracing spans during HTTP request dispatch.

### 2.14 Test Coverage, Test Quality, Edge Cases, and Regression Risk
- **Status:** Mixed / Fragile.
- **Evaluation:** Unit test suites across `sitolo-auth`, `sitolo-tenancy`, `sitolo-config`, and `sitolo-authz` are excellent and pass reliably.
- **Deficiencies:** PostgreSQL security tests in `crates/sitolo-persistence/tests/rls_security_tests.rs` fail-closed when run without an active PostgreSQL instance, breaking standard `./scripts/ci/verify` runs in offline environments. HTTP integration tests only test in-memory mocks without security headers or auth pipelines.

### 2.15 Configuration Management, Environment Separation, and Secrets Handling
- **Status:** Enterprise-Grade Model.
- **Evaluation:** `sitolo-config` provides pinned toolchain validation, environment profile cascading (Development, Staging, Production), and SHA-256 configuration fingerprinting. Secret values are wrapped in `SecretValue` to prevent accidental log exposure.
- **Deficiencies:** Production deployment configuration relies on environment variable injection without dynamic integration with cloud key management services (AWS KMS / HashiCorp Vault).

### 2.16 Deployment Safety, Rollback Readiness, Versioning, and Release Discipline
- **Status:** Partial.
- **Evaluation:** Semantic versioning, workflow policy checks (`check-workflow-policy`), and architecture enforcement scripts exist.
- **Deficiencies:** Production container images (Dockerfiles), database migration execution scripts, health probe validation rules, and automated zero-downtime deployment pipelines are omitted from the repo repository.

### 2.17 Code Quality, Naming, Duplication, Coupling, Dead Code, and Technical Debt
- **Status:** High Quality Code, Unused Primitives.
- **Evaluation:** `#[forbid(unsafe_code)]` is enforced workspace-wide. Code formatting, module naming, and type safety are excellent.
- **Deficiencies:** Multiple warnings exist in `sitolo-persistence` regarding dead/unused code (`PgAuthorityPools`, `PgAuthorityError`, `set_transaction_tenant_context`) when compiled outside database integration tests.

### 2.18 Maintainability Under Team Growth, Code Ownership, and Refactor Cost
- **Status:** Good Architectural Layout.
- **Evaluation:** Crate boundaries are cleanly separated, preventing tight coupling across domain tiers.
- **Deficiencies:** Future teams face a massive implementation burden because 80% of core business features (inventory, sales, reconciliation, tax) exist only as documentation contracts.

### 2.19 Compliance and Enterprise Operational Expectations
- **Status:** Contract-Only / Unimplemented Runtime.
- **Evaluation:** Extensive documentation specifies Mauritius Revenue Authority (MRA) EIS compliance, VAT handling, and audit trails.
- **Deficiencies:** No fiscalization API integration, digital signature signing engine, QR code generation, or fiscal transmission logic is implemented in `sitolo-integrations`.

---

## 3. Severity-Ranked Findings List

### 3.1 Critical Severity Findings (P0)

#### [FINDING-CRIT-01] Missing HTTP Authentication & Authorization Boundary in API Server
- **What is wrong:** Every HTTP endpoint in `apps/api/src/serve.rs` operates completely unauthenticated.
- **Why it is wrong:** `dispatch_request` processes administrative tenancy mutations (`POST /v1/organizations`, `POST /v1/organizations/{org_id}/suspend`, `POST /v1/organizations/{org_id}/close`, etc.) without extracting Bearer tokens, checking session validity, or resolving `AuthorizedScope`.
- **Where it appears:** `apps/api/src/serve.rs`, lines 88–212 (`dispatch_request`).
- **How it fails in production:** Any anonymous network attacker can issue HTTP POST requests to suspend or close all tenant organizations, wipe out access, or provision fake organizations, causing total loss of multi-tenant isolation and complete system disruption.
- **What a proper fix looks like:** Re-architect `apps/api` using Axum or Tower middleware. Implement an authentication layer (`extract_bearer`, `sitolo_api::auth::establish_context`) and an authorization layer that enforces `AuthorizedScope` on every business endpoint before dispatching commands.
- **Priority & Blast Radius:** Priority P0. Blast Radius: Entire System / Total Compromise.

#### [FINDING-CRIT-02] Production Password Hashing Algorithm (Argon2id) Is Missing
- **What is wrong:** The workspace does not contain a real password hashing implementation.
- **Why it is wrong:** `sitolo-auth` defines `PasswordHasher`, but the only implementation in the codebase is `TestPasswordHasher` in `crates/sitolo-auth/src/password.rs`, which uses iterated SHA-256 over a fixed string prefix. `Cargo.toml` does not include `argon2`.
- **Where it appears:** `crates/sitolo-auth/src/password.rs`, lines 94–150 (`TestPasswordHasher`).
- **How it fails in production:** If user passwords are hashed using `TestPasswordHasher` in production, database leaks will expose passwords to high-speed GPU cracking attacks, violating fundamental security compliance (§9, §6.1).
- **What a proper fix looks like:** Add the `argon2` crate to `sitolo-auth/Cargo.toml`. Implement `Argon2idHasher` implementing `PasswordHasher` with parameters matching OWASP guidelines (m=19456 KB, t=2, p=1). Ensure hashing operations execute inside `tokio::task::spawn_blocking`.
- **Priority & Blast Radius:** Priority P0. Blast Radius: User Credential Compromise.

#### [FINDING-CRIT-03] Flawed Custom TCP Transport & HTTP Request Parser
- **What is wrong:** `apps/api/src/serve.rs` parses HTTP requests manually from a single raw TCP socket `read()` call using basic string splitting.
- **Why it is wrong:** TCP streams do not guarantee complete HTTP request delivery in a single `read()` call. Parsing headers with `split_whitespace()` and `split('/')` fails to handle HTTP request chunking, Keep-Alive connection reuse, header normalization, percent-encoding, query parameters, or malformed HTTP requests.
- **Where it appears:** `apps/api/src/serve.rs`, lines 56–86 (`handle_connection`).
- **How it fails in production:** HTTP requests split across multiple TCP packets will cause partial JSON payload parsing errors or truncated request failures. Malformed HTTP requests or header injection will crash or bypass routing.
- **What a proper fix looks like:** Replace the custom TCP socket loop in `apps/api/src/serve.rs` with standard hyper/axum HTTP server infrastructure.
- **Priority & Blast Radius:** Priority P0. Blast Radius: API Transport Instability / Denial of Service.

#### [FINDING-CRIT-04] Background Worker Engine Is an Empty Scaffold
- **What is wrong:** `apps/worker/src/main.rs` contains only `fn main() {}`.
- **Why it is wrong:** Sitolo’s architecture requires asynchronous background execution for outbox event delivery, tax compliance reporting (MRA EIS), notification delivery, and scheduled reconciliation.
- **Where it appears:** `apps/worker/src/main.rs`, lines 1–10.
- **How it fails in production:** Outbox audit events and fiscal tax transactions remain stuck in outbox tables indefinitely. Real-time regulatory submission fails, causing enterprise tax compliance non-compliance.
- **What a proper fix looks like:** Implement a production job worker engine in `apps/worker` with database polling/listener channels, job locking, exponential backoff retries, dead-letter queues (DLQ), and execution tracing.
- **Priority & Blast Radius:** Priority P0. Blast Radius: Async Workflows / Compliance Failure.

---

### 3.2 High Severity Findings (P1)

#### [FINDING-HIGH-01] Application Handlers Default to Non-Transactional In-Memory State
- **What is wrong:** The API binary uses `InMemoryTenancyRepository` for production request state.
- **Why it is wrong:** `InMemoryTenancyRepository` stores state in volatile heap memory protected by standard mutexes. State is lost on process restart, cannot scale across multiple API instances, and lacks database Row Level Security (RLS) enforcement.
- **Where it appears:** `apps/api/src/bootstrap.rs`, lines 70–95 (`bootstrap_state`).
- **How it fails in production:** Data written to one API instance is invisible to another. Any server restart wipes all tenant state. Multi-tenant database RLS policies are completely bypassed.
- **What a proper fix looks like:** Implement a production `PgTenancyRepository` in `sitolo-persistence` that executes SQL queries within PostgreSQL transactions while explicitly executing `set_transaction_tenant_context()`.
- **Priority & Blast Radius:** Priority P1. Blast Radius: Data Durability & Multi-Tenant Security.

#### [FINDING-HIGH-02] Path Parsing Insecure against Path Traversal and Percent-Encoding
- **What is wrong:** Endpoint route dispatching uses `path.split('/')` directly on raw HTTP request paths.
- **Why it is wrong:** `path.split('/')` does not perform URI normalization or percent-decoding (`%2F`, `%2E%2E`).
- **Where it appears:** `apps/api/src/serve.rs`, lines 112–210.
- **How it fails in production:** Route dispatchers can be confused by query strings (e.g. `/v1/organizations?foo=1`) or path traversal sequences, causing unintended routing fallthrough or 440 errors.
- **What a proper fix looks like:** Use a compliant HTTP router (e.g. `axum::Router` or `matchit`) that normalizes URL paths and parses path variables into typed parameters.
- **Priority & Blast Radius:** Priority P1. Blast Radius: API Routing & Authorization.

#### [FINDING-HIGH-03] Connection Dropping Under High Load Without Backpressure or 503 Response
- **What is wrong:** `apps/api/src/serve.rs` uses `semaphore.clone().try_acquire_owned()` and silently ignores connection permits when full.
- **Why it is wrong:** When `MAX_IN_FLIGHT_CONNECTIONS` (1024) is reached, `try_acquire_owned()` fails and execution falls through with `let Ok(permit) = ... else { continue };`, silently dropping the TCP connection without returning an HTTP 503 status code.
- **Where it appears:** `apps/api/src/serve.rs`, lines 39–46.
- **How it fails in production:** Clients experiencing peak load receive abrupt TCP connection resets/drops instead of clean 503 overloaded responses with `Retry-After` headers.
- **What a proper fix looks like:** Queue incoming connections or accept the TCP socket and immediately write a formatted `503 Service Unavailable` JSON response before closing.
- **Priority & Blast Radius:** Priority P1. Blast Radius: High Load Availability & Client Reliability.

#### [FINDING-HIGH-04] Core Business Domain Engines Are Completely Unimplemented
- **What is wrong:** Product Catalogue, Inventory Ledger, Sales/POS, Payments, MRA Fiscalization, and Reporting crates/modules are non-existent.
- **Why it is wrong:** Sitolo claims to be a Business Operating System for African SMEs, but the source code contains zero commercial POS or inventory logic.
- **Where it appears:** `crates/sitolo-domain`, `crates/sitolo-integrations`, `crates/sitolo-sync`.
- **How it fails in production:** SME users cannot manage products, record sales, issue receipts, calculate VAT, or synchronize offline devices.
- **What a proper fix looks like:** Implement domain engines and persistence layers for Phase 8 (Catalogue), Phase 9 (Inventory), Phase 10 (POS Sales), Phase 11 (Payments), and Phase 15 (MRA EIS).
- **Priority & Blast Radius:** Priority P1. Blast Radius: Core Business Functionality.

---

### 3.3 Medium Severity Findings (P2)

#### [FINDING-MED-01] Documentation and Code Divergence Regarding API Framework
- **What is wrong:** Documentation across `docs/` specifies Rust + Axum + Tokio, but `apps/api` implements custom TCP loops without Axum.
- **Why it is wrong:** Architectural drift confuses developers, automated tooling, and security auditors.
- **Where it appears:** `docs/system_architecture_design.md`, `docs/api_contract.md`, vs `apps/api/Cargo.toml`.
- **How it fails in production:** Engineering teams attempt to configure Axum middleware or extensions that do not exist in code.
- **What a proper fix looks like:** Upgrade `apps/api` to use Axum as documented, bringing code into alignment with specification.
- **Priority & Blast Radius:** Priority P2. Blast Radius: Architecture & Maintainability.

#### [FINDING-MED-02] PostgreSQL RLS Security Tests Require Live Database in CI Suite
- **What is wrong:** `crates/sitolo-persistence/tests/rls_security_tests.rs` panics when `ADMIN_DATABASE_URL` is absent.
- **Why it is wrong:** Running `./scripts/ci/verify` in environments without PostgreSQL causes test failures, blocking offline development.
- **Where it appears:** `crates/sitolo-persistence/tests/rls_security_tests.rs`, lines 300–308.
- **How it fails in production:** CI pipelines without containerized DB instances fail builds prematurely.
- **What a proper fix looks like:** Annotate live DB tests with `#[ignore]` or skip gracefully with informative warnings when `ADMIN_DATABASE_URL` is not set, while keeping DB verification mandatory in dedicated integration CI jobs.
- **Priority & Blast Radius:** Priority P2. Blast Radius: CI/CD Pipeline Usability.

#### [FINDING-MED-03] Unused Code Warnings in Persistence Crate During Non-DB Builds
- **What is wrong:** `cargo check` outputs warnings for unused code in `sitolo-persistence` (`PgAuthorityError`, `PgAuthorityPools`, `set_transaction_tenant_context`).
- **Why it is wrong:** Unused warnings clutter build output and indicate orphaned or dead code paths.
- **Where it appears:** `crates/sitolo-persistence/src/postgres.rs`.
- **How it fails in production:** Dead code paths increase maintenance overhead and code complexity.
- **What a proper fix looks like:** Integrate `PgAuthorityPools` into the application bootstrap flow and persistence handlers so all functions are actively utilized.
- **Priority & Blast Radius:** Priority P2. Blast Radius: Code Maintainability.

---

### 3.4 Low Severity Findings (P3)

#### [FINDING-LOW-01] Lack of Security Response Headers in Custom HTTP Transport
- **What is wrong:** `apps/api/src/serve.rs` returns responses with only `content-type` and `content-length`.
- **Why it is wrong:** Missing `Strict-Transport-Security`, `X-Content-Type-Options: nosniff`, `Content-Security-Policy`, and `Cache-Control: no-store` headers.
- **Where it appears:** `apps/api/src/serve.rs`, line 78.
- **How it fails in production:** Browsers and clients may cache sensitive API responses or suffer MIME-sniffing vulnerabilities.
- **What a proper fix looks like:** Add standard security headers in the HTTP response generator or Tower middleware layer.
- **Priority & Blast Radius:** Priority P3. Blast Radius: Hardening / Defense-in-Depth.

#### [FINDING-LOW-02] Minimal JSON Escaping in Diagnostic Health Endpoints
- **What is wrong:** `json_escape()` in `apps/api/src/serve.rs` only escapes quotes `"` and backslashes `\`.
- **Why it is wrong:** Control characters (`\n`, `\r`, `\t`) in service name or version strings could produce invalid JSON output.
- **Where it appears:** `apps/api/src/serve.rs`, lines 225–235.
- **How it fails in production:** Invalid JSON output on `/process/live` probes if configuration contains linebreaks.
- **What a proper fix looks like:** Use `serde_json::to_string` for serializing probe response structures.
- **Priority & Blast Radius:** Priority P3. Blast Radius: Health Check Diagnostics.

---

## 4. Top 10 Highest-Risk Issues

| Rank | Risk Issue | Primary Impact | Blast Radius |
|---|---|---|---|
| **1** | **Unauthenticated API Routes** (`dispatch_request` in `serve.rs`) | Total loss of multi-tenant security and unauthorized access | Entire System |
| **2** | **Missing Production Password Hasher** (SHA-256 test double in production path) | Weak password storage vulnerable to cracking | All User Accounts |
| **3** | **Flawed Custom TCP HTTP Parser** (Single socket read, no header stream buffering) | Request corruption, header injection, transport instability | Network Interface |
| **4** | **Empty Background Worker Scaffold** (`apps/worker/src/main.rs` is `fn main() {}`) | Asynchronous jobs, tax filing, and outbox events never run | Async Processing |
| **5** | **In-Memory Volatile Persistence Default** (`InMemoryTenancyRepository` used in production) | Data loss on restart, no multi-instance scaling, RLS bypassed | Persistence Layer |
| **6** | **Unprotected Tenant Mutations (IDOR)** (Org suspend/close without authz check) | Malicious suspension or deletion of competitor organizations | Multi-Tenant Boundary |
| **7** | **Insecure URL Path Splitting** (`split('/')` without URI decoding) | Route confusion, bypass of path validation checks | API Router |
| **8** | **Missing Core Domain Engines** (Catalogue, Inventory, POS, Payments non-existent) | Platform cannot perform basic business operations | SME Operations |
| **9** | **Silent TCP Connection Dropping** (`try_acquire_owned()` drops connection without 503) | Poor load degradation and client connection drops under peak load | Availability |
| **10** | **Sync & MRA EIS Integration Scaffolds** (No fiscalization or receipt signing) | Enterprise regulatory non-compliance in target markets | Legal & Compliance |

---

## 5. Top 10 Highest-Leverage Fixes

| Rank | Leverage Fix | Remediates | Engineering Effort |
|---|---|---|---|
| **1** | **Migrate `apps/api` to Axum & Tower Framework** | Removes raw TCP parser, adds standard routing, request buffering, and middleware support | 3–5 Days |
| **2** | **Wire `sitolo-auth` & `sitolo-authz` Middleware to API Routes** | Enforces authentication, session verification, and `AuthorizedScope` on all endpoints | 3–4 Days |
| **3** | **Implement Production Argon2id `PasswordHasher`** | Replaces SHA-256 test hasher with secure KDF using `tokio::task::spawn_blocking` | 1–2 Days |
| **4** | **Connect Handlers to `PgTenancyRepository` with RLS Context** | Enables durable database transactions and PostgreSQL Row Level Security | 4–5 Days |
| **5** | **Build Async Outbox Worker Loop in `apps/worker`** | Enables asynchronous background processing, event dispatch, and job retries | 5–7 Days |
| **6** | **Implement URI Decoding and Strict Type Parameter Route Matching** | Eliminates path traversal and routing ambiguity risks | 1–2 Days |
| **7** | **Add Backpressure and HTTP 503 Overload Handler** | Replaces silent TCP connection drops with clean 503 responses and `Retry-After` headers | 1 Day |
| **8** | **Implement Core Business Domain Engines (Phases 8–10)** | Provides real Product Catalogue, Inventory Ledger, and POS Sales capability | 10–15 Days |
| **9** | **Implement MRA EIS Fiscalization Engine** | Provides real-time receipt signing, QR generation, and tax compliance integration | 5–7 Days |
| **10** | **Fix CI Database Test Dependencies & Standardize Pre-Commit Verification** | Restores offline `./scripts/ci/verify` pass rate while retaining RLS test rigor | 1 Day |

---

## 6. Phased Remediation Plan

```text
PHASED REMEDIATION TIMELINE
===========================
[ IMMEDIATE (0-14 days) ]  --> Fix P0 Crits: Axum migration, Auth middleware, Argon2id, Worker loop
[ SHORT TERM (15-45 days) ] --> Fix P1 Highs: PgRepository, RLS context, Overload backpressure, Catalogue/Inventory
[ MEDIUM TERM (46-90 days)] --> Fix P2 Meds: POS Sales engine, MRA EIS fiscalization, Payment reconciliation
[ LONG TERM (91-180 days) ] --> Production Certification: Load testing, HA deployment, DR drills, SOC2 audit
```

### 6.1 Immediate Phase (0–14 Days) — Critical Vulnerability & Transport Remediation
1. **Axum Migration:** Refactor `apps/api` to use `axum` HTTP server framework, replacing custom TCP socket logic in `serve.rs`.
2. **Auth & Authz Middleware:** Wire `sitolo-api::auth::extract_bearer` and `establish_context` into an Axum middleware layer. Mandate valid `AuthorizedScope` context on all `/v1/*` routes.
3. **Argon2id Hasher:** Implement `Argon2idHasher` in `sitolo-auth` using the `argon2` crate with OWASP-recommended parameters inside `tokio::task::spawn_blocking`.
4. **Worker Core Engine:** Build the outbox processing worker loop in `apps/worker/src/main.rs` using transactional job claiming.

### 6.2 Short Term Phase (15–45 Days) — Persistence & Business Domain Foundations
1. **PostgreSQL Handler Integration:** Replace `InMemoryTenancyRepository` in `bootstrap.rs` with `PgTenancyRepository`, executing all database operations within SQL transactions containing `set_transaction_tenant_context()`.
2. **Product Catalogue Engine (Phase 8):** Implement product, category, pricing, and barcode domain entities, HTTP APIs, and database migrations.
3. **Inventory Ledger Engine (Phase 9):** Implement double-entry stock movement ledger, batch tracking, and branch stock allocation.
4. **Graceful Overload Handling:** Add Tower concurrency limit middleware with clean HTTP 503 response formatting.

### 6.3 Medium Term Phase (46–90 Days) — Commercial Workflows & Regulatory Compliance
1. **POS Sales Engine (Phase 10):** Implement checkout sales transactions, split payments, discount rules, and receipt generation.
2. **MRA EIS Fiscalization (Phase 15):** Implement real-time tax invoice signing, fiscal QR generation, and background transmission to Mauritius Revenue Authority servers.
3. **Payments & Reconciliation (Phase 11):** Implement mobile money (M-Pesa / Airtel Money) and card processing adapters with automated reconciliation.
4. **Offline Synchronization (Phase 12):** Build delta-based offline sync protocol for POS desktop/mobile clients.

### 6.4 Long Term Phase (91–180 Days) — Production Certification & Enterprise Hardening
1. **Load & Chaos Testing:** Perform high-throughput load testing (10,000 req/sec) and chaos engineering drills (database failover, network isolation).
2. **Distributed Tracing & Monitoring:** Integrate OpenTelemetry collectors, Prometheus metrics dashboards, and PagerDuty alert rules.
3. **Enterprise Compliance & Auditing:** Conduct third-party penetration testing, SOC 2 Type II readiness audit, and multi-region disaster recovery drills.

---

## 7. Enterprise-Grade Evaluation Matrix: Acceptable vs Non-Enterprise-Grade

| System Component | Current State | Enterprise-Grade Status | Required Standard for Production |
|---|---|---|---|
| **Configuration Engine** (`sitolo-config`) | SHA-256 fingerprinting, toolchain pinning, env validation | **ACCEPTABLE** | Maintain current design; add KMS secrets integration |
| **Tenant Scope Resolution** (`sitolo-tenancy`) | Server-authoritative requested vs trusted scope derivation | **ACCEPTABLE** | Maintain strict fail-closed scope derivation |
| **IAM & Role Hierarchy** (`sitolo-authz`) | Explicit permission sets, role inheritance, scope grants | **ACCEPTABLE** | Connect role checks to HTTP route handlers |
| **HTTP Transport** (`apps/api/src/serve.rs`) | Custom TCP socket parser, string splits | **NOT ENTERPRISE-GRADE** | Standard HTTP framework (Axum/Tower), header stream buffering |
| **API Authentication Boundary** | Unauthenticated route handler fallthrough | **NOT ENTERPRISE-GRADE** | Mandatory Bearer token validation and session establishment middleware |
| **Password Hashing** (`crates/sitolo-auth`) | Iterated SHA-256 test double | **NOT ENTERPRISE-GRADE** | Argon2id KDF executing in blocking thread pools |
| **Database Persistence** | In-memory default repository fallback | **NOT ENTERPRISE-GRADE** | PostgreSQL persistence with transaction-bound RLS tenant contexts |
| **Background Processing** | Empty `fn main() {}` in worker binary | **NOT ENTERPRISE-GRADE** | Durable outbox worker engine with DLQ and retries |
| **Business Operations Engine** | Catalogue, POS, Inventory, Tax missing | **NOT ENTERPRISE-GRADE** | Full domain engine implementations for SME operational lifecycle |
| **Observability Substrate** | Bounded ring buffers, redacted sensitive fields | **ACCEPTABLE (SUBSTRATE)** | Wire Prometheus metrics endpoints and trace propagation |

---

## 8. Conclusion

Sitolo possesses an exceptional architectural blueprint and a highly secure theoretical foundation. However, until the critical vulnerabilities in the HTTP transport, authentication middleware, password hashing, database transaction wiring, and background worker engines are fully remediated, **the codebase must remain classified as Not Ready for Production Use**.

Execution of the Phased Remediation Plan outlined in Section 6 will systematically elevate Sitolo into a secure, scalable, enterprise-grade Business Operating System.
