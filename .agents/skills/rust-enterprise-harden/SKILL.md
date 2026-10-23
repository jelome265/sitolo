---
name: rust-enterprise-harden
description: Enterprise hardening of Rust systems covering security (input boundaries, OAuth2 OIDC and JWT validation, RBAC, rustls TLS, cryptography, secrets and zeroisation, DoS limits, injection and path defence, log redaction, supply chain), resilience (timeouts, budgeted retries with jitter, circuit breakers, bulkheads, load shedding, degradation, idempotency, transactional outbox, graceful shutdown), observability (tracing, OpenTelemetry, RED and USE metrics, probes, SLOs, audit logs), operations (backups, DR, migrations, IaC, FinOps), and build and container hardening (release profile, reproducible builds, SBOM, signing, sanitisers, distroless non-root images). Use for any Rust service, API, worker or CLI that handles untrusted input, money, personal data or credentials, or runs in production. Trigger phrases include harden this, production ready, security review before launch, add auth to this Rust API, circuit breaker, rate limit, Dockerize this Rust service, SBOM and threat model.
---

# Enterprise Rust Hardening (SEC, RES, OBS, OPS, BLD, CTR)

Hardening is a set of controls added to a system so that it fails safely, resists attack, degrades predictably, can be operated, and can be proven to be what it claims to be. Hardening is not a checklist of crates. Each control below has a rule ID, a verification method, and a failure mode it prevents.

Read `../rust-enterprise-best-practices/SKILL.md` first. This skill cites its BP rules and adds controls on top. When a control here conflicts with a BP rule, the stricter one applies and the conflict is reported.

## HRD-0. Method

1. **Threat model first.** Complete HRD-SEC-TM before adding controls, so every control maps to a named threat.
2. **Facts, assumptions, risks.** Separate what the code and config prove, what is assumed (for example "the load balancer strips client-supplied `X-Forwarded-For`"), and what is an open risk. An assumption without an owner is a risk.
3. **Verify, don't trust.** Every control has a test or a runtime check. A control that is documented but not tested is not a control.
4. **Least privilege everywhere.** Process, database role, cloud IAM, network, file system, container capabilities, token scopes.
5. **Fail closed for security decisions, fail open only where availability is the explicit policy and it is recorded.** Authentication, authorisation, signature verification, and schema validation fail closed.
6. **Compliance is a mapping, not an assumption.** Map controls to the regulation that applies to the specific system (for example data protection law, payment card rules, central bank directives, SOC 2 criteria). Do not assume which regimes apply. Record the mapping. Never state that a system is compliant; state which controls exist and which have been verified.

## HRD-SEC. Security

### SEC-TM — threat modelling (BLOCKER before launch)

- **SEC-TM-01 (BLOCKER).** A data-flow diagram exists with trust boundaries, data classification (public, internal, confidential, restricted), and entry points.
- **SEC-TM-02 (BLOCKER).** STRIDE applied per trust boundary: Spoofing, Tampering, Repudiation, Information disclosure, Denial of service, Elevation of privilege. Each threat has a control, an accepted-risk record with owner and expiry, or a non-applicability justification.
- **SEC-TM-03 (MAJOR).** Abuse cases written for each money-moving, identity-changing, or data-export flow (for example "a user replays a payment confirmation", "a tenant enumerates another tenant's IDs", "a partner submits a file that expands to 10 GB").
- **SEC-TM-04 (MAJOR).** The threat model is versioned in the repository and updated when a trust boundary changes.

### SEC-IN — input boundary (BLOCKER)

- **SEC-IN-01.** Every external byte is untrusted: HTTP bodies, headers, query strings, cookies, files, message queue payloads, webhook payloads, gRPC messages, CLI arguments, environment variables that come from other systems, database rows written by other services.
- **SEC-IN-02.** Parse, do not validate (BP-TY-03). A raw DTO becomes a domain type through a fallible constructor. Domain code never accepts a raw string where a validated type exists.
- **SEC-IN-03.** Limits are enforced before allocation: maximum body size (`axum::extract::DefaultBodyLimit` or equivalent, set per route), maximum header count and size (set at the server layer), maximum string length, maximum collection length, maximum JSON depth, maximum file size, maximum decompressed size for compressed input (zip bomb defence).
- **SEC-IN-04.** Unknown fields are rejected on input DTOs (`#[serde(deny_unknown_fields)]`, BP-SE-02). Mass-assignment is impossible because DTO fields are explicit.
- **SEC-IN-05.** Numeric input is range-checked into a newtype; no `as` casts (BP-NU-01).
- **SEC-IN-06.** Input normalisation happens once, at the boundary, with a documented canonical form (Unicode normalisation form for identifiers and emails if the domain requires it; case folding rules; trimming). Comparisons after normalisation only.
- **SEC-IN-07.** Identifiers from clients are treated as capabilities only after authorisation; a valid-looking ID is not proof of access (IDOR defence, see SEC-AUTHZ-03).

### SEC-AUTH — authentication (BLOCKER)

- **SEC-AUTH-01.** Use OpenID Connect (OIDC) for user authentication and OAuth 2.0 for delegated access. Do not implement login protocols by hand. Use the authorization code flow with PKCE for public clients (mobile, desktop, SPA). Use client credentials for service-to-service.
- **SEC-AUTH-02.** Access tokens are JWT (JWS) or opaque. If JWT:
  - Pin the expected algorithm list. Reject `none`. Never accept the algorithm from the token header as the sole decision input.
  - Select the verification key by a `kid` from a trusted JWKS endpoint fetched over TLS, cached with a bounded TTL, and refreshed on unknown `kid` with rate limiting.
  - Validate `iss` (exact match), `aud` (exact match against this service), `exp` (with a small, configured leeway, at most 60 seconds), `nbf` if present, `iat` sanity, and `typ` where profiles define it.
  - Require the claims the service depends on (`sub`, `aud`, `iss`, `exp`, scopes or roles) and reject tokens missing them.
  - Never log full tokens. Never put tokens in URLs or query strings.
- **SEC-AUTH-03.** Opaque tokens are checked via token introspection with a bounded cache. Revocation latency is documented.
- **SEC-AUTH-04.** Refresh tokens are rotated on use, bound to the client, and revoked on reuse detection.
- **SEC-AUTH-05.** Passwords, if the system stores them (prefer delegating to the IdP): Argon2id with explicit parameters meeting current OWASP guidance for the chosen memory, iterations and parallelism, a per-password random salt, and a `password-hash`-style encoding that records parameters. Upgrade parameters on login when they are below policy. Compare with constant-time primitives. Never store reversible encryption of passwords.
- **SEC-AUTH-06.** Session cookies: `Secure`, `HttpOnly`, `SameSite=Lax` or `Strict` unless a flow requires `None` with a documented CSRF defence, a bounded lifetime, and server-side revocation.
- **SEC-AUTH-07.** Multi-factor authentication for administrative and high-value actions, with phishing-resistant factors where the threat model requires them.
- **SEC-AUTH-08.** Rate limit and lock out (progressive delay, not permanent lockout by default) on authentication endpoints, keyed on account and on source, with monitoring for credential stuffing.
- **SEC-AUTH-09.** Failure responses are uniform: no distinction between "user not found" and "wrong password" in the body, status, or timing (compare against a dummy hash when a user does not exist).

### SEC-AUTHZ — authorisation (BLOCKER)

- **SEC-AUTHZ-01.** Deny by default. Every route, handler, and domain operation declares its required permission. A route without a declared permission fails the build (enforced by a test that enumerates routes).
- **SEC-AUTHZ-02.** RBAC: roles map to permissions in a versioned policy. Permissions are verbs on resources (`payments:capture`, `kyc:read`). Roles are never checked by name in business code; permissions are checked. Check at the domain operation, not only at the HTTP edge.
- **SEC-AUTHZ-03.** Object-level authorisation (IDOR defence): every access to a resource checks that the resource belongs to the caller's tenant and that the caller holds the permission on that resource. Queries include the tenant predicate. Enumerable identifiers are avoided where practical (UUIDv4 or UUIDv7 with random component).
- **SEC-AUTHZ-04.** Multi-tenant isolation is tested: a test suite runs every endpoint as tenant A against a resource of tenant B and asserts denial.
- **SEC-AUTHZ-05.** Administrative actions are logged to the audit log with actor, action, target, tenant, decision, and reason (OBS-AUDIT).
- **SEC-AUTHZ-06.** Service accounts have scoped credentials, short-lived tokens, and no human-equivalent privileges.

### SEC-TLS — transport (BLOCKER)

- **SEC-TLS-01.** TLS 1.2 minimum; TLS 1.3 preferred; no SSLv3, TLS 1.0 or 1.1. Use `rustls`. Do not use OpenSSL bindings unless a documented requirement (FIPS, specific ciphers) justifies it.
- **SEC-TLS-02.** Choose a single vetted crypto provider. Verify the provider's FIPS status if the regulator or contract requires it. Configure the provider explicitly rather than relying on defaults that may change across versions.
- **SEC-TLS-03.** Certificate validation is never disabled. No custom `ServerCertVerifier` that accepts all certificates. Certificate pinning, if used, pins a public key on a documented rotation plan and has a backup pin.
- **SEC-TLS-04.** Every outbound HTTP client sets: connect timeout, total timeout, a connection pool bound, and the root store explicitly (system roots or a configured private CA bundle). Redirect policy is explicit; cross-origin redirects do not forward credentials.
- **SEC-TLS-05.** Internal service-to-service traffic uses mutual TLS or equivalent workload identity. Plaintext is not permitted across a trust boundary.
- **SEC-TLS-06.** Certificates have automated rotation with alerting at 30 and 7 days before expiry.
- **SEC-TLS-07.** HSTS, enforced at the edge with a documented max-age and includeSubDomains decision.

### SEC-CRYPTO — cryptography (BLOCKER)

- **SEC-CRYPTO-01.** No custom cryptography. Use audited crates with a documented primitive choice. A pull request that implements a cipher, MAC, KDF, signature scheme or PRNG is rejected.
- **SEC-CRYPTO-02.** Symmetric encryption: AEAD only (AES-256-GCM, or XChaCha20-Poly1305 where nonce-misuse resistance is needed). Unauthenticated modes (CBC without MAC, ECB, CTR without MAC) are forbidden.
- **SEC-CRYPTO-03.** Nonces are unique per key. Random 96-bit nonces are only safe below the documented message count per key; longer-lived keys use XChaCha20 with 192-bit random nonces, or a counter with persistent, crash-safe state. The chosen scheme is recorded.
- **SEC-CRYPTO-04.** Key derivation: HKDF for deriving keys from high-entropy input; Argon2id for passwords and low-entropy secrets. PBKDF2 only where a standard requires it, with an iteration count meeting current guidance.
- **SEC-CRYPTO-05.** Signatures: Ed25519 for new designs unless interoperability requires another algorithm. Verify before parsing claims. Reject malleable or non-canonical encodings where the library exposes the choice.
- **SEC-CRYPTO-06.** Random numbers for keys, nonces, tokens, IDs and salts come from the operating system CSPRNG (`getrandom`, or the OS-backed generator of `rand`). Never seed from time, process ID, or a user-supplied value. The `rand::thread_rng` default is acceptable for non-security randomness only (jitter, shuffling).
- **SEC-CRYPTO-07.** Comparisons of secrets, MACs, tokens and digests use constant-time equality (`subtle::ConstantTimeEq`). A `==` on these values is a finding (gate scan SEC-CRYPTO).
- **SEC-CRYPTO-08.** Hashing for integrity uses SHA-2 or SHA-3 families; MD5 and SHA-1 are forbidden for security purposes. Non-cryptographic hashes are never used for keying or authentication.
- **SEC-CRYPTO-09.** Algorithm and key-size choices are versioned in the data (for example a `kid`, algorithm tag or version byte in ciphertext) so that migration is possible without a flag day.
- **SEC-CRYPTO-10.** Keys are managed by a KMS or HSM. Application processes receive data keys or perform operations through the KMS API. Envelope encryption for data at rest: a data key encrypts the record, and the KMS wraps the data key. Key rotation has a tested runbook.

### SEC-SEC — secrets and memory hygiene (BLOCKER)

- **SEC-SEC-01.** Secrets are not in source, images, Cargo features, logs, error messages, metrics labels, panic messages, crash dumps (where controllable), or environment variables visible in process listings for long-lived services where a secret manager with file or in-process injection is available.
- **SEC-SEC-02.** Secret-typed values use `secrecy::SecretString` or `SecretBox` (or equivalent) so that `Debug` and `Display` are redacted and exposure requires an explicit `expose_secret()` call. Each `expose_secret()` call site is reviewed.
- **SEC-SEC-03.** Secret material in memory is zeroised on drop with `zeroize` (`Zeroize`, `ZeroizeOnDrop`). Note the limit: zeroisation does not defeat compiler copies or swapped pages. Use `mlock`-style protection only with a measured threat that justifies the cost.
- **SEC-SEC-04.** Secrets are injected by the platform's secret manager (cloud KMS-backed secret store, Vault, Kubernetes secret mounted as a file with restricted permissions). Rotation is automated and the application reloads without a restart where the design allows, or restarts through the orchestrator with a rollout strategy.
- **SEC-SEC-05.** Secret scanning runs on every commit and every build (for example a secrets detector in CI and a pre-commit hook). A detected secret is treated as compromised: rotate, then remove from history.
- **SEC-SEC-06.** Debug endpoints, pprof endpoints, admin consoles, and metrics endpoints are not exposed on public interfaces. They require authentication and network restriction.

### SEC-DOS — denial of service (BLOCKER)

- **SEC-DOS-01.** Every resource that grows with input has a configured ceiling: connections, concurrent requests per client, request bodies, streams, queue lengths, in-memory caches, retry queues, regex inputs, JSON nesting, decompression ratio, log line length, number of open files, and number of spawned tasks.
- **SEC-DOS-02.** Regular expressions run on untrusted input only with a linear-time engine (the `regex` crate). Backtracking engines (`fancy-regex`, PCRE bindings) are forbidden on untrusted input.
- **SEC-DOS-03.** Hash-flooding: maps keyed by external input use the standard randomised hasher (BP-CO-04).
- **SEC-DOS-04.** CPU-bound work per request has a budget. Work exceeding the budget is cancelled or moved to a bounded pool with a queue limit (BP-CC-09).
- **SEC-DOS-05.** Algorithmic complexity is reviewed for every input-driven loop. Nested loops over attacker-controlled collections are findings unless bounded.
- **SEC-DOS-06.** Slowloris and slow-body defence: header read timeout, body read timeout, minimum transfer rate where the server supports it.
- **SEC-DOS-07.** Panics triggered by input are DoS bugs (BP-ER-02). A fuzz campaign on each input parser is a release criterion (BP-TS-03).

### SEC-INJ — injection and interpreter defence (BLOCKER)

- **SEC-INJ-01 (SQL).** All queries use bound parameters. String-built SQL from input is forbidden. With `sqlx`, use `query!`, `query_as!` or `query_scalar!` macros or the `query` builder with `.bind()`. Dynamic identifiers (column or table names) come from an allowlist enum, never from input. Compile-time checked queries run against a committed offline cache in CI (`SQLX_OFFLINE=true`).
- **SEC-INJ-02 (command).** `std::process::Command` with an argument list and no shell. Never `Command::new("sh").arg("-c")` with input. Program paths are absolute or from an allowlist. The environment is cleared and rebuilt explicitly for child processes.
- **SEC-INJ-03 (headers and HTTP).** Header values built from input reject CR and LF. Outbound URLs built from input are validated against an allowlist of schemes and hosts (SSRF defence): no link-local addresses (`169.254.0.0/16`, including the cloud metadata address), no loopback or private ranges unless explicitly allowed, DNS rebinding considered (resolve once, connect to the resolved address, verify the address class before connecting).
- **SEC-INJ-04 (logs).** Log injection: user input containing newlines or control characters is escaped or stored as a structured field value, never concatenated into a message line.
- **SEC-INJ-05 (templates and serialisation).** Output encoding is contextual: HTML, JSON, CSV (formula injection for spreadsheet exports: prefix cells starting with `=`, `+`, `-`, `@` with a single quote or reject them), and XML (external entities disabled).
- **SEC-INJ-06 (deserialisation).** No polymorphic deserialisation driven by untrusted type tags. Enums with explicit variants only.

### SEC-PATH — filesystem (BLOCKER)

- **SEC-PATH-01.** Paths from input are resolved against a fixed root. The resolved path is canonicalised and must start with the canonical root. Reject absolute input, `..` components, drive prefixes, and NUL bytes before resolution (BP-CO-08).
- **SEC-PATH-02.** Symlinks inside the root are a TOCTOU risk. Open files with flags that refuse symlinks where the platform supports it, or open the root directory handle and resolve relative to it (`openat`-style handling via a vetted crate).
- **SEC-PATH-03.** Uploaded files: generate server-side file names; never trust the client file name for storage; store outside the web root; scan with an anti-malware pipeline where the threat model requires it; serve with a correct `Content-Type` and `Content-Disposition: attachment` for untrusted types.
- **SEC-PATH-04.** Archive extraction (zip, tar): reject entries with absolute paths, `..`, symlinks pointing outside the root, and enforce total expanded size and entry count limits (SEC-IN-03).
- **SEC-PATH-05.** Temporary files use a secure temp API with restrictive permissions (0600) and are removed on all paths, including errors (RAII guard).

### SEC-LOG — logging, errors, redaction (BLOCKER)

- **SEC-LOG-01.** Fields that are never logged: passwords, tokens (access, refresh, ID, API keys), session identifiers, private keys, full card numbers, national identity numbers, full bank account numbers, full authentication headers, request bodies unless a sanitised schema says otherwise.
- **SEC-LOG-02.** Personal data is minimised in logs. Identifiers are pseudonymised with a keyed hash where correlation is needed. Retention periods for logs are defined and enforced.
- **SEC-LOG-03.** Error types that carry sensitive data implement redacting `Display` and `Debug` (BP-TY-09). Error messages returned to clients are generic and carry a correlation ID. Internal detail is logged server-side only.
- **SEC-LOG-04.** Stack traces and internal error chains are never returned in HTTP responses in production.
- **SEC-LOG-05.** Log integrity: logs are shipped to a centralised, access-controlled store. Audit logs are append-only and tamper-evident (OBS-AUDIT).

### SEC-SUP — supply chain (BLOCKER)

- **SEC-SUP-01.** `Cargo.lock` committed and enforced (BP-CG-01). Builds use `--locked`.
- **SEC-SUP-02.** `cargo deny check` enforces advisories, licences, bans, and sources (BP-DP-02). Licence allowlist is defined per organisation policy.
- **SEC-SUP-03.** `cargo vet` attestations cover the production dependency graph (BP-DP-07).
- **SEC-SUP-04.** Dependency updates are automated (Dependabot or Renovate) with grouped PRs, tests on every PR, and a human review. Security updates for HIGH and CRITICAL advisories are prioritised with a stated SLA.
- **SEC-SUP-05.** Build scripts (`build.rs`) and procedural macros are reviewed on addition and on version change. Build scripts do not access the network and do not write outside `OUT_DIR`.
- **SEC-SUP-06.** Vendoring or a private registry mirror is used for production builds so that a registry outage or a deleted crate does not break releases.
- **SEC-SUP-07.** Typosquatting defence: crate names are checked against the intended name before adding; `cargo deny` sources policy forbids unexpected registries.
- **SEC-SUP-08.** Containers and CI actions are pinned by digest or full commit SHA (BP-GT-06, CTR-04).

## HRD-RES. Resilience

### RES-TO — timeouts and deadlines (BLOCKER)

- **RES-TO-01.** Every outbound call has a connect timeout, a per-attempt timeout, and an overall deadline that bounds retries. Values come from configuration with documented defaults and validated ranges.
- **RES-TO-02.** Deadlines propagate. A request's remaining budget is passed to downstream calls (for example as a `grpc-timeout` header or an internal deadline value). A downstream call never runs longer than the caller's remaining budget.
- **RES-TO-03.** Every inbound request has a server-side deadline, enforced by middleware, returning a documented status when exceeded.
- **RES-TO-04.** Database statements have a server-side timeout (for example `statement_timeout` on PostgreSQL) and the client has a timeout on acquiring a pool connection.

### RES-RETRY — retries (BLOCKER)

- **RES-RETRY-01.** Retry only retryable errors, as classified by the error model (BP-ER-07). Validation failures, authorisation failures, and domain rejections are never retried.
- **RES-RETRY-02.** Exponential backoff with full jitter: `sleep = random(0, min(cap, base * 2^attempt))`. Cap and base are configured. Retry-After headers from the server override the computed delay when present.
- **RES-RETRY-03.** Bounded attempts (typically 3 including the first) and a retry budget: retries cannot exceed a fixed fraction of recent request volume (for example 10%) so that retries cannot amplify an outage.
- **RES-RETRY-04.** Non-idempotent operations are retried only with an idempotency key (RES-IDEM).
- **RES-RETRY-05.** Retries are visible: each retry increments a metric labelled by dependency and error class (bounded labels), and emits a span event.

### RES-CB — circuit breakers (MAJOR)

- **RES-CB-01.** Every dependency that can fail in bulk has a circuit breaker with explicit states: closed, open (fail fast), half-open (limited probes). Thresholds are configured per dependency: failure ratio over a rolling window, minimum request count, open duration, probe count.
- **RES-CB-02.** A breaker's open state returns a typed error (`DependencyUnavailable`) that the caller maps to degradation (RES-DEG) rather than a generic 500.
- **RES-CB-03.** Breaker state transitions are logged and exported as a metric. An open breaker pages only when it blocks a critical path (OBS-ALERT).

### RES-BULK — bulkheads and concurrency limits (MAJOR)

- **RES-BULK-01.** Concurrency to each dependency is limited with a semaphore sized from measured capacity. Separate limits for separate dependencies so that one slow dependency cannot consume all workers.
- **RES-BULK-02.** Separate connection pools per dependency and per tenant class where isolation is a requirement.
- **RES-BULK-03.** Background work (batch jobs, reports, exports) runs in a separate process or pool from the request path, with its own limits.

### RES-SHED — load shedding and rate limiting (BLOCKER)

- **RES-SHED-01.** Ingress has a concurrency limit. When the limit is reached, new requests are rejected quickly (HTTP 503 with `Retry-After`) rather than queued without bound (BP-AS-07).
- **RES-SHED-02.** Rate limits are enforced per client identity (authenticated principal, API key, or IP where unauthenticated) with a shared store when instances are horizontally scaled. Per-instance limits alone are not a control in a multi-instance deployment; document the effective limit.
- **RES-SHED-03.** Priority: health probes, authentication and payment-critical paths retain capacity when shedding. The priority order is documented.
- **RES-SHED-04.** Queue depth and wait time are metrics. Alerts fire on sustained queueing.

### RES-DEG — graceful degradation (MAJOR)

- **RES-DEG-01.** For each dependency, the degraded behaviour is written down: serve stale with a maximum staleness, return a reduced feature set, queue the write for later with a durable outbox, or fail the request with a clear error. "Crash" is not a degradation policy.
- **RES-DEG-02.** Feature kill switches exist for expensive or risky features, operable without a deployment, audited, and tested.
- **RES-DEG-03.** Caches are disposable. A cache outage must not cause an outage of the core flow unless the cache is the source of truth, in which case it is not a cache.

### RES-IDEM — idempotency and exactly-once effects (BLOCKER for money and state changes)

- **RES-IDEM-01.** Every state-changing endpoint that a client may retry accepts an `Idempotency-Key` (or equivalent domain key). The server stores the key, a hash of the request, the response status and body, and a TTL.
- **RES-IDEM-02.** Replaying the same key with the same request returns the stored response without re-executing. The same key with a different request body returns a conflict error.
- **RES-IDEM-03.** The idempotency record and the business effect commit in the same database transaction, or the effect is guarded by a unique constraint on the idempotency key. A check-then-insert without a constraint is a race and is a finding.
- **RES-IDEM-04.** Message consumers are idempotent: duplicate delivery is expected and produces no second effect. Use a processed-message table keyed by message ID with the effect in the same transaction.
- **RES-IDEM-05.** Money movement is modelled with double-entry ledger entries where the domain requires auditability. Balances are derived or checked against the ledger. Amounts are integer minor units (BP-NU-03). Reversals are new entries, never edits.

### RES-OUTBOX — events and messaging (MAJOR)

- **RES-OUTBOX-01.** Events that must follow a database change are written to a transactional outbox in the same transaction as the change. A relay publishes from the outbox with at-least-once semantics. Dual writes (database then broker, without an outbox) are forbidden.
- **RES-OUTBOX-02.** Kafka or RabbitMQ is used when asynchronous messaging improves decoupling, resilience or throughput (per the project's architecture policy). Choice is recorded in an ADR with the delivery guarantee, ordering scope, retention, and consumer lag SLO.
- **RES-OUTBOX-03.** Every consumer defines: idempotency (RES-IDEM-04), a retry policy with backoff, a dead-letter queue, a poison-message procedure, and an alert on dead-letter growth.
- **RES-OUTBOX-04.** Message schemas are versioned contracts (BP-SE-01). Schema evolution is backward compatible for the consumer's lifetime. A schema registry or contract tests enforce it.
- **RES-OUTBOX-05.** Ordering is guaranteed only within the partition or queue key that the domain requires. Cross-key ordering is not assumed.

### RES-STATE — statelessness and scaling (MAJOR)

- **RES-STATE-01.** Services are stateless with respect to request handling. Session state lives in a shared store or in signed tokens. Sticky sessions are not a design choice; they are a finding.
- **RES-STATE-02.** Any in-process state (caches, connection pools, rate-limit buckets, background schedulers) is either disposable or coordinated. A scheduler that runs on every replica must use leader election or a distributed lock with a lease and fencing tokens.
- **RES-STATE-03.** Horizontal scale is tested: the service runs with N replicas in a staging environment under load, and the design's correctness does not depend on a single replica.

### RES-SHUT — graceful shutdown and restart (BLOCKER)

- **RES-SHUT-01.** On SIGTERM: stop accepting new work, mark readiness false, drain in-flight requests within a bounded deadline, flush telemetry, close pools, exit (implement skill, IMP-6.3).
- **RES-SHUT-02.** Startup does not serve traffic until dependencies are verified and migrations are confirmed (readiness gate).
- **RES-SHUT-03.** Background tasks observe the cancellation token and stop at safe points. Work in progress is either completed, checkpointed, or re-queued.

## HRD-OBS. Observability and operations of the software

### OBS-LOG — structured logs

- **OBS-LOG-01.** Logs are JSON or a structured format, emitted through `tracing` with a subscriber that adds timestamp (UTC), level, service name, version, environment, instance ID, trace ID and span ID.
- **OBS-LOG-02.** Level policy: ERROR means an operation failed and needs attention or a retry decision; WARN means degraded but handled; INFO means significant lifecycle events; DEBUG and TRACE are off in production by default and never contain secrets (SEC-LOG).
- **OBS-LOG-03.** Log volume is budgeted. Per-request INFO logs are sampled or limited to one per request with summary fields.

### OBS-TRACE — distributed tracing

- **OBS-TRACE-01.** OpenTelemetry for traces, with W3C `traceparent` propagation on every inbound and outbound call, including messages (trace context carried in message headers).
- **OBS-TRACE-02.** Spans cover inbound requests, outbound calls, database queries, cache operations, queue publish and consume, and background jobs. Span names are low-cardinality (BP-OB-03).
- **OBS-TRACE-03.** Sampling policy is written down. Tail-based sampling at a collector keeps all error and slow traces. Sensitive attributes are never added to spans (SEC-LOG).
- **OBS-TRACE-04.** Trace IDs are returned to clients in a response header on errors to support support-ticket correlation.

### OBS-MET — metrics

- **OBS-MET-01.** RED metrics per endpoint and per dependency: request rate, error rate (by class), duration histogram with explicit buckets aligned to SLOs.
- **OBS-MET-02.** USE metrics per resource: utilisation, saturation (queue depth, pool wait), errors for pools, threads, file descriptors, memory, and disk.
- **OBS-MET-03.** Business metrics for the domain: transactions per minute, value processed (in minor units), reconciliation mismatches, failed payouts. Business metrics feed alerts for silent failures.
- **OBS-MET-04.** Label cardinality budget: labels are bounded enums. No user IDs, tenant IDs at high cardinality, URLs with identifiers, or free text as labels (BP-OB-05).
- **OBS-MET-05.** Runtime metrics: Tokio task count, poll times where available, allocator metrics where available, GC-free but track memory growth over time.

### OBS-HEALTH — health probes

- **OBS-HEALTH-01.** Liveness reports only that the process can make progress. It never depends on external dependencies (a database outage must not restart every pod).
- **OBS-HEALTH-02.** Readiness reports whether the instance can serve traffic now: dependencies reachable within a short timeout, startup migrations confirmed, not draining. Readiness failure removes the instance from rotation without restart.
- **OBS-HEALTH-03.** Startup probe for slow-start services with a bounded startup budget.
- **OBS-HEALTH-04.** Health endpoints are unauthenticated only if they expose no internal detail; detailed diagnostics require authentication.

### OBS-SLO — service levels

- **OBS-SLO-01.** Each service defines SLIs (availability as successful requests over valid requests; latency as proportion of requests below threshold), SLOs (targets over a window), and an error budget policy (what happens when the budget is spent: feature freeze, reliability work).
- **OBS-SLO-02.** SLAs (contractual) are a strict subset of SLOs with margin. Contracts are reviewed by the business owner, not inferred by engineering.
- **OBS-SLO-03.** Capacity planning is derived from SLO targets and load tests, not from guesses. Load tests are repeatable and versioned.

### OBS-ALERT — alerting

- **OBS-ALERT-01.** Alerts are symptom-based (user-visible error budget burn, latency), not cause-based (CPU at 80%), except for capacity forecasts.
- **OBS-ALERT-02.** Multi-window, multi-burn-rate alerts for SLOs (fast burn pages, slow burn tickets).
- **OBS-ALERT-03.** Every alert has an owner, a severity, a runbook link, and a documented first action. Alerts without runbooks are removed.
- **OBS-ALERT-04.** Alert fatigue is measured. Alerts that fire without action for a month are retuned or removed.

### OBS-AUDIT — audit trail (BLOCKER for regulated actions)

- **OBS-AUDIT-01.** Regulated and security-relevant actions produce audit events: authentication success and failure, authorisation denials, privilege changes, data exports, money movements, configuration changes, key operations.
- **OBS-AUDIT-02.** An audit event contains: timestamp (UTC), actor (principal and type), action, target (type and ID), tenant, outcome, reason, source (IP or workload identity, pseudonymised where required), request ID, and version of the code that produced it.
- **OBS-AUDIT-03.** Audit events are written through a path that cannot be disabled by application configuration in production. Write failures for audit events fail the operation for high-value actions (fail closed), and are alerted for others.
- **OBS-AUDIT-04.** Audit storage is append-only with integrity protection (for example hash chaining with periodic anchoring, or WORM storage). Access to audit logs is itself audited.
- **OBS-AUDIT-05.** Retention for audit logs follows the applicable legal requirement, recorded as a decision, not a default.

## HRD-OPS. Operations, data and delivery

- **OPS-DR-01 (BLOCKER).** Each service records RTO and RPO. Values come from the business impact analysis and are approved by the owner. Engineering does not invent them.
- **OPS-DR-02 (BLOCKER).** Backups are encrypted with keys separate from the data's keys, stored off-site or in another failure domain, and restore-tested on a schedule. An untested backup is not a backup.
- **OPS-DR-03 (MAJOR).** Point-in-time recovery is enabled for transactional databases where RPO requires it. Restore procedures are runbooks with timings measured in drills.
- **OPS-DR-04 (MAJOR).** Regional or zonal failure is covered by a documented procedure, rehearsed at least annually (game day).
- **OPS-MIG-01 (BLOCKER).** Schema migrations use expand-and-contract: add nullable columns and new tables first, dual-write or backfill, switch reads, then remove old columns in a later release. Breaking schema changes in a single step are forbidden for live tables.
- **OPS-MIG-02 (MAJOR).** Migrations run as a separate job with a lock timeout and statement timeout set. Long-running backfills run in batches with progress tracking and are resumable.
- **OPS-MIG-03 (MAJOR).** Every migration has a tested rollback or a documented forward-fix plan.
- **OPS-IAC-01 (BLOCKER).** Infrastructure is defined as code, reviewed through the same PR process, with state stored encrypted and access-controlled. Manual production changes are detected (drift detection) and either reverted or codified.
- **OPS-IAC-02 (MAJOR).** Policy as code runs in CI: no public database endpoints, no unencrypted storage, no wildcard IAM actions, required tags for cost allocation.
- **OPS-IAC-03 (MAJOR).** Least-privilege cloud IAM for workloads: each service has its own identity with only the permissions it uses, reviewed quarterly.
- **OPS-FINOPS-01 (MAJOR).** Unit economics are measured: cost per thousand requests, cost per transaction, cost per active tenant. Resource requests and limits are set from measured usage. Autoscaling has minimum and maximum bounds. Idle resources are reported.
- **OPS-FINOPS-02 (MINOR).** Cost anomalies alert. Non-production environments shut down outside working hours where the business accepts it.
- **OPS-CFG-01 (BLOCKER).** Configuration follows 12-factor principles: environment-specific values are injected, not compiled in. Configuration is validated at startup and fails fast (IMP-6.1).
- **OPS-CFG-02 (BLOCKER).** Feature flags for risky changes are versioned, audited, and removed after rollout.
- **OPS-REL-01 (MAJOR).** Progressive delivery: canary or blue-green with automated analysis against SLO signals and automatic rollback on regression.
- **OPS-REL-02 (MAJOR).** Release notes and change records are generated from commits and PRs, with links to ADRs for behaviour changes.

## HRD-BLD. Build, release and verification

### BLD-REL — release profile

Declare in the root `Cargo.toml`. Each setting is a decision recorded in an ADR or a comment:

```toml
[profile.release]
opt-level = 3
lto = "thin"              # measured; "fat" only with build-time budget approval
codegen-units = 1         # smaller, slower-to-build; keep for shipped binaries
panic = "unwind"          # set to "abort" only per BP-PN-03 ADR; default kept for task isolation
overflow-checks = true    # mandatory: silent wraparound in release is a correctness and security defect
debug = "line-tables-only"  # symbolicated stack traces without shipping full debug info
strip = "debuginfo"       # debug symbols archived separately for crash analysis
incremental = false
```

- **BLD-REL-01 (BLOCKER).** `overflow-checks = true` in release for every shipped binary.
- **BLD-REL-02 (BLOCKER).** Debug symbols are archived per release, keyed by build ID, so that crashes can be symbolicated.

### BLD-REPRO — reproducible builds (MAJOR)

- **BLD-REPRO-01.** Builds are performed in a hermetic container with a pinned toolchain, pinned base images by digest, and `--locked`.
- **BLD-REPRO-02.** Path and timestamp noise is removed: `SOURCE_DATE_EPOCH` set from the commit timestamp, `RUSTFLAGS="--remap-path-prefix=$PWD=/src --remap-path-prefix=$CARGO_HOME=/cargo"` (adjust paths for the build environment).
- **BLD-REPRO-03.** Two independent builds of the same commit produce byte-identical artifacts, verified in CI. A mismatch fails the release.
- **BLD-REPRO-04.** Build scripts do not read the clock, network or non-declared environment variables. Declared inputs use `cargo:rerun-if-changed` and `cargo:rerun-if-env-changed`.

### BLD-SBOM — software bill of materials and provenance (BLOCKER for release)

- **BLD-SBOM-01.** `cargo auditable build` embeds dependency information into the binary so that the shipped artifact can be audited later with `cargo audit bin`.
- **BLD-SBOM-02.** A CycloneDX or SPDX SBOM is generated for each release (for example with `cargo-cyclonedx` or an equivalent tool), attached to the release, and stored with the artifact.
- **BLD-SBOM-03.** SLSA-style provenance is generated for the build: builder identity, source commit, build parameters, and input digests. Verification happens before deployment.

### BLD-SIGN — signing (BLOCKER for release)

- **BLD-SIGN-01.** Container images and binaries are signed with keyless or KMS-backed signatures (for example Sigstore cosign). Deployment verifies signatures and provenance and refuses unsigned artifacts (admission control).
- **BLD-SIGN-02.** Signing keys are never in the source repository or CI variables in plaintext. Signing identity is bound to the CI workflow identity.

### BLD-VERIFY — verification tooling (conditional, mandatory when the trigger applies)

- **BLD-VERIFY-01 Miri (BLOCKER when `unsafe` exists).** Run on every module with unsafe code, with strict provenance (BP-US-04).
- **BLD-VERIFY-02 Sanitisers (MAJOR for unsafe or FFI).** AddressSanitizer, ThreadSanitizer, and MemorySanitizer runs on nightly, using the pinned nightly toolchain and `-Zsanitizer=...`. Results are gate-reviewed.
- **BLD-VERIFY-03 Fuzzing (BLOCKER for untrusted-input parsers).** cargo-fuzz targets for every parser, decoder and deserialiser that consumes untrusted data. Continuous fuzzing on a schedule with a corpus stored and minimised. Crashes become regression tests (BP-TS-02).
- **BLD-VERIFY-04 Model checking (MAJOR for invariants that matter).** Kani proofs for bounded properties (arithmetic overflow freedom in ledger code, parser termination on bounded inputs). loom for concurrency models (BP-CC-07).
- **BLD-VERIFY-05 Mutation testing (MAJOR for money and security modules).** cargo-mutants on a schedule; surviving mutants in critical modules are findings.
- **BLD-VERIFY-06 Dynamic analysis of services.** Load tests (for example a repeatable generator with recorded profiles), chaos experiments on dependencies (latency, errors, partition), and soak tests for memory growth.

### BLD-SAN-PROFILE — dependency surface

- **BLD-SAN-01 (MAJOR).** `cargo geiger` (or equivalent) reports unsafe usage per crate; growth is reviewed.
- **BLD-SAN-02 (MAJOR).** Binary size and dependency count are tracked. Growth is justified (BP-DP-01).

## HRD-CTR. Containers and runtime

- **CTR-01 (BLOCKER).** Multi-stage build: the build stage has the toolchain; the runtime stage has only the binary and required runtime libraries. No compilers, package managers, shells or source code in the runtime image.
- **CTR-02 (BLOCKER).** Runtime runs as a non-root user with a fixed numeric UID. The filesystem is read-only except for declared writable volumes. No privileged mode, no host networking, no host PID namespace.
- **CTR-03 (BLOCKER).** Linux capabilities dropped to none unless a specific capability is required and justified. `allowPrivilegeEscalation: false`. Seccomp profile set to `RuntimeDefault` or a stricter profile.
- **CTR-04 (BLOCKER).** Base images pinned by digest. The build tool version pinned. Images rebuilt on base-image security updates through the same pipeline.
- **CTR-05 (BLOCKER).** Image vulnerability scan on every build; HIGH and CRITICAL findings with a fix available block the release.
- **CTR-06 (MAJOR).** Resource requests and limits set. Memory limit sized from measured peak with headroom; Rust services do not have a GC, so memory growth is a leak or a cache policy problem, not a collector setting.
- **CTR-07 (MAJOR).** Network policies: default deny ingress and egress; explicit allow for each dependency.
- **CTR-08 (MAJOR).** Graceful termination period set to exceed the drain timeout (RES-SHUT-01).

Reference Dockerfile (replace digests with real pinned values from your registry; the placeholders are deliberate and must not ship):

```dockerfile
# syntax=docker/dockerfile:1
FROM rust:1.98.1-bookworm@sha256:<PIN_DIGEST> AS build
WORKDIR /src
COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
COPY crates ./crates
# Pass the commit timestamp at build time: --build-arg SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}
RUN cargo build --locked --release --bin app-bin

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:<PIN_DIGEST>
COPY --from=build /src/target/release/app-bin /usr/local/bin/app-bin
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/app-bin"]
```

Use a static musl target with `scratch` or `distroless/static` when no dynamic libraries are needed. A healthcheck is performed by the orchestrator's probes (OBS-HEALTH), not by shell commands inside the image.

## HRD-DB. Database access (BLOCKER for persistent data)

- **DB-01.** Each service owns its schema. Other services do not read or write its tables. Cross-service reads go through APIs or events (BP-MO-02, RES-OUTBOX).
- **DB-02.** The application database role has the minimum privileges: no DDL in runtime, no superuser, no ability to drop tables. Migrations use a separate role used only by the migration job.
- **DB-03.** Connections use TLS with certificate verification (SEC-TLS-03). Connection pool bounds and acquire timeouts are set (RES-TO-04).
- **DB-04.** Row-level security or equivalent tenant predicate is applied for multi-tenant data, backed by SEC-AUTHZ-03 tests.
- **DB-05.** Transactions are short. No network calls inside a transaction. Isolation level is explicit; serialisable or explicit locking is used where invariants require it, and the choice is recorded per domain (consistency by domain).
- **DB-06.** Sensitive columns are encrypted at the application layer with envelope encryption (SEC-CRYPTO-10) when the data classification requires it, and are excluded from logs and default queries.
- **DB-07.** Queries are reviewed with `EXPLAIN` for new access paths. Unbounded `SELECT` without `LIMIT` on user-driven paths is a finding (SEC-DOS-01).

## HRD-CFG. Runtime configuration baseline

- **CFG-01.** Configuration validated at startup; invalid configuration prevents start (IMP-6.1).
- **CFG-02.** Insecure options are rejected in production builds or environments: disabled certificate verification, debug endpoints, permissive CORS, plaintext listeners, default credentials.
- **CFG-03.** CORS is an explicit origin allowlist. `Access-Control-Allow-Origin: *` with credentials is forbidden.
- **CFG-04.** Security response headers set on every response: `Content-Security-Policy` where HTML is served, `X-Content-Type-Options: nosniff`, `Referrer-Policy`, `Cache-Control: no-store` for authenticated responses, `Strict-Transport-Security` at the edge.
- **CFG-05.** Panic output and default error pages do not leak internal detail (SEC-LOG-04).

## HRD-OUT. Output contract for a hardening engagement

Every hardening change produces:

1. The threat model delta (SEC-TM) and the controls mapped to threats.
2. A control inventory table: control ID, rule, implementation location, verification method (test ID, gate step, or runbook), owner.
3. Accepted risks with owner, expiry, and compensating control.
4. Residual risk statement written as facts, assumptions and open risks, separated.
5. Operational runbooks for new alerts, rotations, and recovery.
6. Gate output with the new checks enabled.

## HRD-DONE. Hardening done criteria

- [ ] Threat model exists, is current, and every trust boundary has controls (SEC-TM).
- [ ] Every entry point parses into domain types with limits (SEC-IN).
- [ ] Authentication and authorisation are deny-by-default and tested per route and per tenant (SEC-AUTH, SEC-AUTHZ).
- [ ] TLS is rustls-based, verification enabled, timeouts on every client (SEC-TLS, RES-TO).
- [ ] Secrets are typed, redacted, injected, rotated, and scanned (SEC-SEC).
- [ ] Input-driven resources have ceilings; fuzz campaigns run on parsers (SEC-DOS, BLD-VERIFY-03).
- [ ] Retries are budgeted; non-idempotent operations are protected by idempotency keys; breakers and bulkheads exist for each dependency (RES).
- [ ] Health probes separate liveness and readiness; SLOs and burn-rate alerts exist with runbooks (OBS).
- [ ] Audit events exist for regulated actions and are tamper-evident (OBS-AUDIT).
- [ ] Backups are encrypted and restore-tested; RTO and RPO are approved (OPS-DR).
- [ ] Release binaries are reproducible-checked, SBOM'd, signed, and provenance-verified (BLD).
- [ ] Container runs non-root, read-only, minimal, pinned, and scanned (CTR).
- [ ] Supply chain gates pass: deny, audit, vet (SEC-SUP).
