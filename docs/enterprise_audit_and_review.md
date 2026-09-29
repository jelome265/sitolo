# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-25
**Source baseline:** `main` at `e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc`
**Status:** Current-state comprehensive enterprise audit
**Scope:** Full end-to-end codebase evaluation across architecture, security, reliability, scalability, performance, maintainability, observability, testing, deployment readiness, and operational risk.

---

## 1. Executive Summary & Overall Codebase Health

Sitolo is currently **NOT production-ready**, but possesses an unusually strong, security-conscious architectural substrate. The repository demonstrates exceptional engineering rigor in its foundational crates (`sitolo-security`, `sitolo-auth`, `sitolo-authz`, `sitolo-tenancy`, `sitolo-config`, `sitolo-observability`, and `sitolo-persistence`), but is critically incomplete as an operational business system for African SMEs.

### Core Strengths:
1. **Security & Cryptographic Foundations:** Zero `unsafe` code (`#![forbid(unsafe_code)]`), explicit secret redaction via `SecretValue` / `SecretRef`, memory wiping, deterministic password hashing bounds, and strict ID validation (hostile character rejection, path traversal protection).
2. **Tenant Isolation Model:** Robust server-authoritative scope resolution (`AuthorizedScope`, `bind_organization`, `resolve_effective_scope`) preventing tenant scope widening. PostgreSQL Row-Level Security (RLS) and least-privileged `app_runtime` database role enforcement proven by integration security tests.
3. **CI/CD & Governance Automation:** Canonical CI pipeline enforcing strict formatting (`cargo fmt`), clippy warnings as errors (`-D warnings`), dependency auditing (`cargo deny`, `cargo audit`), workspace architecture rules, and System Map reference integrity.

### Critical Production Blockers & Systemic Gaps:
1. **Transport Boundary Vulnerability & Architecture Divergence:** The primary HTTP service (`apps/api`) implements a custom, raw `TcpListener` request parser in `apps/api/src/serve.rs` instead of using the specified production framework (Axum/Hyper). The implementation reads raw TCP streams into a single buffer (`stream.read(&mut buf)`), failing on fragmented HTTP requests, missing TLS/HTTPS termination, lacking keep-alive support, and omitting HTTP authentication/authorization middleware entirely.
2. **Absence of Core Business Domain Engines:** `sitolo-domain` contains only tenancy models (`Organization`, `Branch`, `Membership`, `Invitation`). Core operational domain engines—Product Catalogue, Inventory Ledger (double-entry FIFO costing), POS Sales & Cart, Payments & Reconciliation, MRA EIS Tax Engine, Procurement, Returns, Reporting, and Billing—are entirely absent from source code.
3. **Unimplemented Worker & Asynchronous Infrastructure:** `apps/worker/src/main.rs` is an empty stub (`fn main() {}`). `sitolo-events`, `sitolo-integrations`, and `sitolo-sync` are empty shell crates. No background job processing, transaction outbox worker, DLQ mechanism, or offline synchronization exists.
4. **Persistence Integration Seam:** While PostgreSQL RLS security proofs pass in tests, the live application state relies on in-memory repositories (`MemoryDatabase` / `TenancyDatabase`). Production transactions, migrations management, and outbox event persistence are not connected to application service commands.

---

## 2. Assessment Across 19 System Dimensions

| # | Dimension | Status | Current-State Assessment & Evidence |
|---|---|---|---|
| 1 | **Repository Structure & Dependency Boundaries** | **Acceptable** | Monorepo layout with clear separation between `apps/` and `crates/`. Rust workspace dependency graph enforces acyclic, layered boundaries (`sitolo-security` $\rightarrow$ `sitolo-config` $\rightarrow$ `sitolo-domain` $\rightarrow$ `sitolo-application` $\rightarrow$ `apps/api`). |
| 2 | **Application Architecture & Module Separation** | **Partial** | Hexagonal/ports-and-adapters architecture defined. Application services orchestrate tenancy and IAM, but lack command/query handlers for business operations. |
| 3 | **API Design, Request Flow & Trust Boundaries** | **Unacceptable** | Bounded DTOs (`sitolo-api`) enforce `deny_unknown_fields` and max body size (32 KiB). However, `apps/api/src/serve.rs` parses raw TCP streams manually, lacks HTTP auth middleware, and omits header/chunk parsing. |
| 4 | **Authentication, Authorization & Session Handling** | **Partial** | Cryptographic token models, session snapshots, MFA enrollment, Argon2 password hashing, and role/scope catalogs exist in `sitolo-auth`/`sitolo-authz`. However, HTTP endpoints in `apps/api` do not extract or verify session tokens. |
| 5 | **Input Validation, Sanitization & Data Integrity** | **Acceptable Substrate** | Domain identifiers enforce strict ASCII regex checks (`^[a-zA-Z0-9_-]+$`), rejecting CR/LF injection, control characters, and path traversal attempts. Body size limits enforced. |
| 6 | **Injection Risks, XSS, CSRF, SSRF, IDOR & Defaults** | **Partial / High Risk** | IDOR protection is strong in tenancy scope resolution. XSS/CSRF are minimized by JSON-only APIs. However, raw TCP HTTP parsing introduces HTTP request smuggling risk, and missing HTTP auth leaves endpoints exposed. |
| 7 | **Error Handling, Retry Behavior & Failure Isolation** | **Partial** | Errors map cleanly to RFC 7807 `ProblemDetails`. However, custom TCP reader silently drops connection errors without logging or retry handling. No provider retry or circuit breaker logic exists. |
| 8 | **CPU-bound vs. I/O-bound Bottlenecks** | **Partial** | Argon2 password hashing is configured with CPU/memory cost bounds. However, blocking synchronous disk or crypto operations in async Tokio threads could starve the runtime if not offloaded via `spawn_blocking`. |
| 9 | **Async Behavior, Blocking Ops & Concurrency** | **Partial** | Tokio async runtime used correctly in reference services. However, `apps/api/src/serve.rs` connection loop uses a fixed concurrency semaphore without backpressure or queueing metrics. |
| 10 | **Database Design, Transactions, Migrations & RLS** | **Partial / Gate** | PostgreSQL 18 RLS policies, least-privileged `app_runtime` role, and transaction GUC context (`app.organization_id`) are verified in `rls_security_tests.rs`. Full schema, migrations, and live DB execution for business entities remain phase-gated. |
| 11 | **Caching Strategy & Consistency Tradeoffs** | **Not Implemented** | No caching layer (Redis or in-memory LRU) exists. Architecture treats database as single source of truth, which is safe but unoptimized for read-heavy catalogue traffic. |
| 12 | **Queueing, Background Jobs & Idempotency** | **Unacceptable** | `apps/worker` is empty (`fn main() {}`). No transactional outbox worker, job queue, retry backoff, or idempotency table processor exists. |
| 13 | **Observability: Logs, Metrics, Traces & Alerts** | **Partial** | `sitolo-observability` provides structured JSON logging via `tracing-subscriber` and bounded trace context propagation. Missing Prometheus/OpenTelemetry metric exporters and health endpoints metrics. |
| 14 | **Test Coverage, Test Quality & Edge Cases** | **Partial** | High test quality and negative test coverage for security, config, tenancy, and PostgreSQL RLS. Zero test coverage for non-existent business domain engines. |
| 15 | **Configuration Management & Secrets Handling** | **Acceptable** | Pinned toolchain (Rust 1.98.1), strongly-typed configuration structs (`sitolo-config`), and zero-leakage `SecretValue` wrapper in `sitolo-security`. Production provider (Vault/AWS KMS) needs implementation. |
| 16 | **Deployment Safety, Rollback & Release Discipline** | **Partial** | Pinned dependencies and reproducible builds. Missing container release manifests, helm/k8s deployment specs, DB migration rollbacks, and zero-downtime deployment scripts. |
| 17 | **Code Quality, Naming & Technical Debt** | **Acceptable** | Clean Rust code conforming to strict clippy rules (`#![forbid(unsafe_code)]`). Minimal dead code outside stub crates. |
| 18 | **Maintainability Under Team Growth & Ownership** | **Acceptable** | Excellent workspace boundaries and governance context (`AGENTS.md`, System Map, CI enforcement) allowing modular ownership. |
| 19 | **Compliance & Enterprise Operational Expectations** | **Unacceptable** | Missing MRA EIS tax integration, fiscal digital signature engine, audit log outbox exporter, and PCI-DSS payment isolation boundaries. |

---

## 3. Severity-Ranked Findings List

### CRITICAL SEVERITY FINDINGS

#### FINDING CRIT-01: Raw TCP HTTP Parsing in API Binary (HTTP Request Smuggling, Fragment Denial of Service)
- **What is wrong:** `apps/api/src/serve.rs` implements custom HTTP request parsing directly on a raw `tokio::net::TcpStream` instead of using a standard, battle-tested HTTP framework (e.g., Axum / Hyper).
- **Why it is wrong:** Custom HTTP parsers fail to handle TCP fragmentation, HTTP keep-alive, chunked transfer encoding, header continuation, and header validation rules specified in RFC 9112. A single TCP read (`stream.read(&mut buf)`) assumes the entire HTTP request header and body arrive in a single packet payload.
- **Where it appears:** `apps/api/src/serve.rs`, function `handle_connection` (lines 53–87).
- **How it fails in production:**
  1. Any request split across multiple TCP packets (due to network latency, large payloads, or MTU fragmentation) will have its headers or body truncated, causing random HTTP 422 errors or malformed request processing.
  2. Malicious attackers can exploit custom line/header splitting to perform HTTP Request Smuggling, header injection, or resource exhaustion.
  3. No HTTP/2 or HTTP/3 support, forcing inefficient single-request TCP connections (`connection: close`).
- **What a proper fix looks like:** Replace raw TCP socket handling in `apps/api` with `axum` and `hyper` routing layers, using `tower` middleware for body limits, timeouts, and header parsing.
- **Priority & Blast Radius:** Priority P0 | Blast Radius: Global API Availability & Security.

#### FINDING CRIT-02: HTTP API Route Surface Lacks Authentication and Authorization Middleware
- **What is wrong:** The HTTP request handler `dispatch_request` in `apps/api/src/serve.rs` routes incoming HTTP POST requests directly to application handles (`handle_provision_organization`, `handle_create_branch`, etc.) without passing through authentication token extraction or session context validation.
- **Why it is wrong:** Any anonymous network client capable of reaching the API port can execute administrative tenant operations (such as creating organizations or suspending branches) without presenting a valid Bearer token, session cookie, or API key.
- **Where it appears:** `apps/api/src/serve.rs`, function `dispatch_request` (lines 89–210).
- **How it fails in production:** An unauthenticated attacker can send a curl command to `/v1/organizations/{id}/suspend` or `/v1/organizations/{id}/branches` and shut down legitimate business tenants or inject unauthorized branches.
- **What a proper fix looks like:** Implement an Axum middleware layer (`sitolo-api::middleware::authenticate`) that extracts Bearer tokens, validates sessions against `sitolo-auth::session`, resolves `AuthorizedScope`, and enforces `sitolo-authz` policy checks before invoking route handlers.
- **Priority & Blast Radius:** Priority P0 | Blast Radius: System-Wide Security & Authorization Bypass.

#### FINDING CRIT-03: Complete Absence of Core SME Business Domain Engines
- **What is wrong:** The domain layer (`crates/sitolo-domain`) contains only tenancy models (`tenancy.rs`). The core business capabilities required for an SME Business Operating System are absent.
- **Why it is wrong:** A business operating system cannot process sales, record stock movements, calculate taxes, or process payments without domain entities and state machines.
- **Where it appears:** `crates/sitolo-domain/src/lib.rs` and `crates/sitolo-domain/src/`.
- **How it fails in production:** The system cannot serve POS clients, record inventory, calculate MRA EIS tax signatures, or reconcile cash/card payments.
- **What a proper fix looks like:** Sequentially implement domain modules for Product Catalogue (Phase 8), Inventory Ledger (Phase 9), POS Sales (Phase 10), Payments (Phase 11), Offline Sync (Phase 12), Procurement (Phase 13), Returns (Phase 14), and MRA EIS Tax (Phase 15).
- **Priority & Blast Radius:** Priority P0 | Blast Radius: Core Business Functionality.

#### FINDING CRIT-04: Empty Background Worker Binary and Asynchronous Infrastructure
- **What is wrong:** `apps/worker/src/main.rs` contains only `fn main() {}`. `sitolo-events`, `sitolo-integrations`, and `sitolo-sync` contain no executable event/job loops.
- **Why it is wrong:** Asynchronous tasks—such as sending outbox events to MRA EIS tax systems, processing payment webhooks, generating daily reconciliation reports, and synchronizing offline mobile POS devices—will never execute.
- **Where it appears:** `apps/worker/src/main.rs`, `crates/sitolo-events/src/lib.rs`, `crates/sitolo-integrations/src/lib.rs`, `crates/sitolo-sync/src/lib.rs`.
- **How it fails in production:** Outbox audit tables will grow indefinitely without being consumed; integration side effects will silently fail; offline POS devices will never receive catalog or price updates.
- **What a proper fix looks like:** Build a polling/listen worker loop in `apps/worker` that claims outbox records using `FOR UPDATE SKIP LOCKED`, processes tasks with idempotency guarantees, and dispatches external provider calls with exponential backoff and DLQ handling.
- **Priority & Blast Radius:** Priority P0 | Blast Radius: Asynchronous Operations, Integrations & Compliance.

---

### HIGH SEVERITY FINDINGS

#### FINDING HIGH-01: In-Memory Repositories Used as Production State in API Runtime
- **What is wrong:** `apps/api` bootstraps application services with `MemoryDatabase` / `TenancyDatabase` in `apps/api/src/state.rs`. Live application data is stored in in-memory HashMaps protected by `std::sync::RwLock`.
- **Why it is wrong:** All tenant organization and branch mutations are volatile. Restarting the API container or deploying a new build completely wipes all tenant state.
- **Where it appears:** `apps/api/src/state.rs` and `crates/sitolo-persistence/src/memory.rs`.
- **How it fails in production:** Data loss on every process restart or crash; failure to scale horizontally across multiple API instances (instance A does not see state created on instance B).
- **What a proper fix looks like:** Wire `sitolo-persistence::postgres` pools into `AppState` and replace `MemoryDatabase` with PostgreSQL database repositories executing SQL transactions with RLS context.
- **Priority & Blast Radius:** Priority P1 | Blast Radius: Persistence & Multi-Node Scalability.

#### FINDING HIGH-02: Missing TLS/HTTPS Termination Boundary in Listener Component
- **What is wrong:** `apps/api/src/serve.rs` opens a plain TCP socket (`TcpListener::bind`) without TLS options or mandatory reverse-proxy header verification (`X-Forwarded-Proto`, `TLS-Client-Cert`).
- **Why it is wrong:** Plain HTTP transmits session credentials, tokens, and sensitive business transaction data in plaintext across the network, violating security baselines and enterprise compliance standards.
- **Where it appears:** `apps/api/src/serve.rs`, function `serve`.
- **How it fails in production:** Interception of user authentication tokens, customer PII, and financial transaction data over public or untrusted networks (man-in-the-middle attacks).
- **What a proper fix looks like:** Configure `rustls` for direct TLS termination or enforce strict reverse-proxy headers validation with proxy trust subnet restrictions.
- **Priority & Blast Radius:** Priority P1 | Blast Radius: Data Confidentiality & Transport Security.

#### FINDING HIGH-03: Lack of Prometheus / OpenTelemetry Exporters in Observability Infrastructure
- **What is wrong:** `sitolo-observability` writes structured JSON logs to stdout/stderr, but provides no HTTP metrics endpoint (`/metrics`) or OpenTelemetry OTLP trace exporter.
- **Why it is wrong:** Operations teams cannot set up real-time alerting on error rates, HTTP request latencies (p50/p99), database pool exhaustion, or memory/CPU usage.
- **Where it appears:** `crates/sitolo-observability/src/lib.rs`.
- **How it fails in production:** Incidents (such as database connection pool starvation or high 500 error rates) go unnoticed until customers report outages.
- **What a proper fix looks like:** Integrate `metrics-exporter-prometheus` or `opentelemetry-otlp` into `sitolo-observability` and expose a restricted `/metrics` endpoint in `apps/api`.
- **Priority & Blast Radius:** Priority P1 | Blast Radius: System Observability & Incident Response.

---

### MEDIUM SEVERITY FINDINGS

#### FINDING MED-01: Incomplete Secret Provider Integration for Production Vault / KMS
- **What is wrong:** Production configuration falls back to `EnvironmentSecretProvider` or requires custom provider code.
- **Why it is wrong:** Storing database passwords or JWT signing keys directly in environment variables increases the risk of secret leaks via process dumps, environment inspection, or log outputs.
- **Where it appears:** `crates/sitolo-security/src/provider.rs`.
- **How it fails in production:** Secret leakage if environment variables are exposed in crash reports or system monitoring tool outputs.
- **What a proper fix looks like:** Implement a dedicated HashiCorp Vault or AWS KMS secret provider in `sitolo-security` with cached token renewal and zero-memory-leakage guarantees.
- **Priority & Blast Radius:** Priority P2 | Blast Radius: Secrets Governance.

#### FINDING MED-02: Missing Distributed Rate Limiting Across API Instances
- **What is wrong:** Rate limiting primitives in `sitolo-auth::ratelimit` operate in-memory per node without a distributed Redis backend.
- **Why it is wrong:** In a multi-instance API deployment, brute-force login attempts or API abuse distributed across N instances effectively multiplies the rate limit threshold by N.
- **Where it appears:** `crates/sitolo-auth/src/ratelimit.rs`.
- **How it fails in production:** Credential stuffing or DDoS attacks succeed by spreading requests across multiple load-balanced API pods.
- **What a proper fix looks like:** Add a Redis-backed sliding-window rate limiter with fallback to local in-memory leaky-bucket limits.
- **Priority & Blast Radius:** Priority P2 | Blast Radius: API Protection & DDoS Resilience.

---

### LOW SEVERITY FINDINGS

#### FINDING LOW-01: Dead Code Warnings in Uncompiled Helper Methods
- **What is wrong:** `sitolo-persistence::postgres` generated minor unused item warnings during build when certain features or test flags were toggled.
- **Why it is wrong:** Increases build output noise and can obscure meaningful compiler warnings.
- **Where it appears:** `crates/sitolo-persistence/src/postgres.rs` (`PgAuthorityError`, `PgAuthorityPools` helpers).
- **How it fails in production:** No operational failure in production; developer ergonomics issue.
- **What a proper fix looks like:** Add appropriate `#[allow(dead_code)]` or refine module export visibility for helper constructs.
- **Priority & Blast Radius:** Priority P3 | Blast Radius: Code Cleanliness.

---

## 4. Top 10 Highest-Risk Issues

1. **Raw TCP HTTP Request Handling (`apps/api/src/serve.rs`):** High probability of HTTP request truncation, parsing crashes, and request smuggling under real network traffic.
2. **Unauthenticated HTTP API Surface:** Zero HTTP middleware verifying Bearer tokens or sessions on incoming organization/branch API routes.
3. **Missing Business Domain Engines:** Complete lack of POS sales, inventory ledger, pricing, and catalogue engines prevents operational use.
4. **Volatile In-Memory Application Database:** Production API uses `MemoryDatabase`, causing total data loss on container restart.
5. **Unimplemented Worker Process:** `apps/worker` is empty, leaving all outbox events, background jobs, and integration sync unexecuted.
6. **No MRA EIS Tax Engine or Digital Signatures:** Regulatory non-compliance in target African markets (e.g., Mauritius Revenue Authority EIS) leading to legal operational failure.
7. **Lack of TLS / Transport Layer Encryption:** Plain TCP listening opens network communications to credential interception.
8. **Missing Distributed Rate Limiting:** In-memory rate limiting allows brute-force attacks across multi-node API deployments.
9. **Absence of Real-Time Metrics & Prometheus Exporter:** Inability to monitor request rates, p99 latencies, and database pool saturation.
10. **Unchecked Blocking Operations on Async Worker Threads:** Potential thread starvation in Tokio runtime if password hashing (Argon2) or heavy crypto runs on main event loops without `spawn_blocking`.

---

## 5. Top 10 Highest-Leverage Fixes

1. **Migrate `apps/api` to Axum Framework:** Instant gain in HTTP parsing correctness, body limits, header safety, TLS support, and ecosystem middleware compatibility.
2. **Implement Axum Authentication & Scope Middleware:** Enforce session token extraction, `AuthorizedScope` resolution, and RBAC policy evaluation globally.
3. **Wire PostgreSQL Repositories into `AppState`:** Connect `sitolo-persistence::postgres` to replace `MemoryDatabase` with durable, RLS-protected transactions.
4. **Implement Transactional Outbox Worker Loop:** Build the event polling loop in `apps/worker` using `FOR UPDATE SKIP LOCKED` for reliable async execution.
5. **Implement Product Catalogue & Inventory Domain Engines (Phases 8 & 9):** Establish core business data structures for products, stock ledgers, and FIFO movements.
6. **Implement POS Sales & Cart Engine (Phase 10):** Enable transaction processing, receipt generation, and cash/card sales posting.
7. **Integrate MRA EIS Compliance Module (Phase 15):** Add fiscal invoice signing, QR code generation, and automated tax reporting.
8. **Expose Prometheus `/metrics` Endpoint:** Integrate operational metrics (`metrics-exporter-prometheus`) for immediate infrastructure visibility.
9. **Add HashiCorp Vault / AWS KMS Secret Provider:** Transition secret resolution from environment variables to enterprise secret stores.
10. **Implement Offline Sync Protocol Engine (Phase 12):** Enable mobile and desktop POS devices to operate seamlessly during Internet outages.

---

## 6. Phased Remediation Plan

```text
PHASED REMEDIATION ROADMAP
┌─────────────────────────────────────────────────────────────────────────┐
│ Immediate (Days 1–14)                                                   │
│ 1. Migrate apps/api from raw TCP to Axum + Hyper framework.             │
│ 2. Add HTTP auth middleware & AuthorizedScope extraction.               │
│ 3. Connect PostgreSQL persistence layer & replace MemoryDatabase.       │
├─────────────────────────────────────────────────────────────────────────┤
│ Short-Term (Month 1–2)                                                  │
│ 1. Implement Phase 8 (Product Catalogue) & Phase 9 (Inventory Ledger). │
│ 2. Implement Phase 10 (POS Sales) & Phase 11 (Payments).               │
│ 3. Build transactional outbox worker in apps/worker.                    │
├─────────────────────────────────────────────────────────────────────────┤
│ Medium-Term (Month 3–4)                                                 │
│ 1. Implement Phase 12 (Offline Sync) & Phase 15 (MRA EIS Tax Engine).  │
│ 2. Expose Prometheus metrics & OpenTelemetry tracing exporters.          │
│ 3. Implement Redis-backed distributed rate limiting.                   │
├─────────────────────────────────────────────────────────────────────────┤
│ Long-Term (Month 5–6)                                                   │
│ 1. Complete Phase 19 (Hardening, Performance & DR Drills).             │
│ 2. Execute Phase 20 (Production Certification & Enterprise Audit).     │
│ 3. Integrate Vault/KMS secret provider & automated DB failover.         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Identification: Acceptable vs. Not Enterprise-Grade

### What is ACCEPTABLE & ENTERPRISE-GRADE Today:
- **Zero Unsafe Code:** Strictly enforced workspace-wide (`#![forbid(unsafe_code)]`).
- **Domain Identity Validation:** Hostile string rejection, path traversal prevention, and bounded string formats.
- **Tenant Scope Resolution:** `AuthorizedScope` typed boundary preventing privilege escalation and scope widening.
- **PostgreSQL RLS Security Infrastructure:** Multi-tenant database isolation enforced at PostgreSQL schema level with least-privileged runtime roles (`app_runtime`).
- **Configuration & Secret Models:** Strongly typed configuration parsing with zero-leakage `SecretValue` redaction.
- **CI/CD Quality Gates:** Enforced formatting, linting, dependency auditing (`cargo deny`, `cargo audit`), and architecture verification scripts.

### What is NOT ENTERPRISE-GRADE Today:
- **HTTP Transport Layer:** Custom TCP parser in `apps/api/src/serve.rs` is fragile and vulnerable to fragmentation and request smuggling.
- **Unprotected HTTP Endpoints:** Absence of HTTP auth middleware on API routes.
- **Core Business Feature Absence:** Zero implementation of POS sales, inventory, tax compliance, or payments engines.
- **Volatile Application Storage:** Reliance on in-memory maps in production API binary.
- **Empty Worker Process:** `apps/worker` cannot execute asynchronous or background tasks.
- **Missing Enterprise Observability:** Lack of Prometheus metrics endpoint and distributed tracing exporters.
