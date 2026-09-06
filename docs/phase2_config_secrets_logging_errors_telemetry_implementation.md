# SITOLO — PHASE 2 IMPLEMENTATION SPECIFICATION

**Document:** `phase2_config_secrets_logging_errors_telemetry_implementation.md`  
**Phase:** Phase 2 — Config + Secrets + Logging + Errors + Telemetry  
**Product:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first; controlled African regional expansion  
**Backend:** Rust + Axum + Tokio  
**Persistence:** PostgreSQL authoritative; SQLite client continuity  
**Clients:** Flutter / Android-first, Tauri desktop, TypeScript where materially useful  
**Architecture:** Modular monolith first, durable workers/outbox, explicit external adapters  
**Baseline:** 6 September 2026  
**Status:** Implementation-governing contract


# 0. Executive Decision

This document is the implementation-governing contract for Sitolo Phase 2: Configuration + Secrets + Logging + Errors + Telemetry. It converts already-frozen architectural decisions into executable runtime rules. Sitolo is not a conventional CRUD POS. It is a mobile-first, offline-capable Business Operating System for African SMEs, beginning in Malawi, with multi-tenant organizations, branches, devices, catalogue, inventory, POS, cash, payments, reconciliation, MRA EIS, reporting, billing and administrative controls. The runtime substrate therefore has to be deterministic under weak networks, concurrent transactions, provider ambiguity, retries, device loss, secret rotation, partial outages and adversarial input.

The governing separation is:

```text
configuration      = runtime policy
secrets            = access material
errors             = semantic failure classification
telemetry          = operational evidence
PostgreSQL         = authoritative business state
audit/evidence     = durable accountability evidence
```

None of the first four may silently become a competing business authority. Telemetry can state that a sale request succeeded; it cannot decide that a sale exists. A config value can select a timeout; it cannot redefine a merchant's historical tax rule. A secret provider can authorize a payment adapter; it cannot grant a user permission. An internal exception can explain a database timeout; it cannot automatically become a client-facing stack trace.

# 1. Existing Sitolo Contract Baseline

The following project artifacts are already present and are treated as upstream contracts rather than regenerated here: `business_model_design.md`, `sitolo.md`, `system_architecture_design.md`, `security_architecture_design.md`, `security_implementation_spec.md`, `domain_model.md`, `database_design.md`, `api_contract.md`, `auth_authorization_spec.md`, `sync_protocol.md`, `payment_integration_spec.md`, `mra_eis_integration_spec.md`, `testing_strategy.md`, `threat_model.md`, `observability_spec.md`, `deployment_spec.md`, `implementation_plan.md`, `ADR-001-025.md`, `security_test_harness.md`, `ci_enforcement.md`, and the Phase 1 repository/workspace/CI implementation document.

The upstream contracts establish Rust + Axum + Tokio as the backend baseline, PostgreSQL as authoritative server state, SQLite as client continuity state, Flutter as the Android-first client, Tauri for desktop, TypeScript only where materially useful, modular-monolith-first architecture, durable outbox/workers, explicit external adapter boundaries, append-oriented financial corrections, ledger-backed inventory, server-enforced authorization, tenant isolation, fail-closed security and release-blocking tests. The API contract already requires RFC 9457-style errors, request correlation and explicit retry semantics. The observability contract already defines metrics, traces, structured logs, correlation IDs, cardinality constraints, telemetry trust boundaries and the rule that telemetry is not business truth. Phase 2 implements these existing commitments rather than creating alternative conventions.

# 2. Scope and Non-Goals

In scope: typed runtime configuration; configuration source precedence; validation; production environment separation; secret references; secret providers; secret rotation; secret failure handling; startup readiness; safe redaction; structured Rust logging; request correlation; trace propagation; error taxonomy; transport error mapping; retryability; metrics; tracing; OpenTelemetry integration boundaries; telemetry backpressure; exporter failure handling; health/readiness semantics; database/worker/integration instrumentation; schema registries; security tests; CI checks; operational runbooks.

Not in scope: redefining identity protocols, MFA design, tenant membership, authorization vocabulary, database schema, payment wire contracts, MRA wire contracts, synchronization semantics, legal retention requirements, or final hosting-provider selection. Those remain owned by upstream contracts. Where this document mentions them, it only defines the runtime hooks those later phases consume.

# 3. Normative Principles

The following are binding unless changed through the existing ADR process.

1. Configuration is centralized and typed. Feature code must not freely call `std::env::var`.
2. Configuration is validated before normal traffic is accepted.
3. Secret values are distinct from ordinary configuration and must never be serialized or logged accidentally.
4. Production must not depend on developer-local secret files.
5. Business state is not environment configuration.
6. Error types retain semantic distinction across module boundaries.
7. Public errors are explicit allowlists, not serialized internal exceptions.
8. Retryability is an explicit property.
9. Unknown external outcomes are not assumed to be failures or successes merely from a timeout.
10. Logging is structured and field-selected, not arbitrary request dumping.
11. Metrics use bounded dimensions only.
12. Trace context is untrusted diagnostic input and never authorization evidence.
13. Telemetry is bounded and may degrade without corrupting business transactions.
14. Mandatory audit/evidence paths are separate from best-effort telemetry.
15. Every critical dependency has timeout and failure semantics.
16. CI must enforce the security properties that can be mechanically enforced.
17. Any exception to these rules is explicit, reviewable, tested and observable.

# 4. Phase 2 Architecture

```text
Deployment / Environment
        |
        v
+---------------------+
| Typed Config Loader |
+----------+----------+
           |
    parse/merge/validate
           |
     +-----+------+----------------+
     |            |                |
     v            v                v
 Config       Secrets         Telemetry
 Fingerprint  Runtime         Runtime
     |            |                |
     +------------+----------------+
                  |
                  v
          +---------------+
          | Runtime State |
          +-------+-------+
                  |
      +-----------+-----------+
      |           |           |
      v           v           v
   Axum/API    Workers    Integrations
      |           |           |
      +-----------+-----------+
                  |
           Error / Context
                  |
       +----------+----------+
       |          |          |
       v          v          v
   Safe HTTP    Audit     Telemetry
   response     evidence      |
                         +----+----+----+
                         |    |    |    |
                        logs metrics traces
```

The runtime substrate is deliberately outside the pure domain. `sitolo-domain` owns business semantics, while `sitolo-config`, `sitolo-observability`, `sitolo-security`, `sitolo-api`, `sitolo-application`, and `sitolo-persistence` enforce the corresponding infrastructure boundaries. Domain code must not import Axum, SQLx, OpenTelemetry exporters, HTTP provider SDKs, or OS environment access.

# 5. Configuration Taxonomy

Every runtime setting belongs to exactly one operational class.

### A — Static process configuration
Examples: service name, bind address, listener limits, request-size ceilings, telemetry destination, deployment environment, build metadata.

### B — Secret references
Examples: database password reference, OIDC client secret reference, payment credential reference, webhook secret reference, MRA secret reference, telemetry exporter credential reference. A reference is configuration; the secret value is not.

### C — Operational tunables
Examples: pool size, worker concurrency, retry ceiling, telemetry sampling ratio, queue capacity. These are configuration of infrastructure, not merchant business state.

### D — Business/domain state
Examples: approval limits, merchant pricing policies, branch behavior, tax configuration, stock policy. These belong to authoritative domain/persistence state and must not be encoded as environment variables just because they are convenient to developers.

The hard rule is: if changing a value changes what the merchant legally or economically did, it is probably domain state, not process configuration.

# 6. Configuration Precedence

The canonical baseline is:

```text
compiled safe defaults
      ↓
non-secret environment/profile file
      ↓
environment variables / deployment injection
      ↓
secret references resolved through secret authority
      ↓
approved runtime overrides, only where explicitly supported
      ↓
parse → normalize → validate → security-policy validate
```

An override can replace a value only inside the allowed domain of that setting. It cannot bypass hard ceilings. Example: if the architecture defines a 25 MiB maximum body-size ceiling, an operator setting `1 GiB` is an invalid configuration, not a new policy. Configuration merging must be deterministic, testable and independent from request execution.

# 7. Configuration Source Rules

Use one canonical namespace, for example:

```text
SITOLO__RUNTIME__ENVIRONMENT
SITOLO__HTTP__BIND_ADDRESS
SITOLO__HTTP__MAX_BODY_BYTES
SITOLO__DATABASE__HOST
SITOLO__DATABASE__PORT
SITOLO__DATABASE__NAME
SITOLO__DATABASE__USER
SITOLO__DATABASE__PASSWORD_REF
SITOLO__TELEMETRY__OTLP_ENDPOINT
SITOLO__TELEMETRY__TRACE_SAMPLE_RATIO
```

Avoid a proliferation of aliases such as `DB_URL`, `DATABASE_URL`, `POSTGRES_URL`, `PG_URL` unless a documented compatibility period exists. Multiple names create a latent production failure mode where two instances believe they are configured identically but consume different variables.

The Rust `config` crate supports layered defaults/files/environment/programmatic sources and is a viable implementation option, but Sitolo must own the semantic precedence and validation contract rather than outsourcing architecture to a library. citeturn487252search3turn487252search10

# 8. Configuration Schema

Each field must have machine-readable metadata:

```text
name
class (A/B/C/D)
required?
default
source(s)
allowed range / syntax
secret classification
reloadability
owner
startup failure behavior
runtime failure behavior
```

A schema should reject type ambiguity. A setting called `TIMEOUT=5` is weak because units are unclear. Prefer explicit durations or names such as `REQUEST_TIMEOUT_MS`. Numeric bounds must be enforced before the value reaches a client, worker, pool, or exporter.

# 9. Startup Validation and Readiness

Startup is a security boundary. A service must not become ready while mandatory trust assumptions are invalid.

```text
load config
  ↓
parse
  ↓
validate schema
  ↓
validate security ceilings
  ↓
resolve mandatory secrets
  ↓
initialize redaction
  ↓
initialize telemetry
  ↓
initialize critical dependencies
  ↓
READY
```

Not every dependency is a startup blocker. PostgreSQL is normally a blocker for an API that must execute authoritative writes. A telemetry collector may be temporarily unavailable without making financial data incorrect. A payment provider outage should not prevent a catalogue endpoint from starting. Capability health therefore needs to be distinct from process liveness.

# 10. Configuration Fingerprint and Drift

Generate a deterministic hash of canonicalized, non-secret effective configuration. The fingerprint must exclude raw secret values, private keys, session material and PII. It can be included in logs and traces as a low-cardinality deployment attribute.

Example:

```text
config_fingerprint = SHA-256(canonical_non_secret_effective_config)
```

This lets operators compare two instances during a rolling deployment and detect accidental drift. It is evidence, not authority. The fingerprint does not replace deployment records or source provenance.

# 11. Dynamic Configuration Policy

Default to restart-required for trust-sensitive settings. Dynamic reload is permitted only after an explicit design and test cycle.

```text
JWT issuer / JWKS trust / TLS verification / auth policy
    → restart required by default

worker counts / sampling ratio / resource tuning
    → may be dynamically adjustable later

merchant pricing / tax / approval policy
    → domain/database operation
```

The Rust `config` library supports live watching, but availability of a library feature is not evidence that the capability is safe for Sitolo. Live-changing a trust setting can make the process operate under a policy different from the one reviewed at deployment time. Any future reload system must have versioning, audit, concurrency semantics, rollback and failure behavior.

# 12. Secret Management Contract

OWASP recommends centralized storage, provisioning, auditing and rotation of secrets rather than scattering credentials through source and configuration files. citeturn487252search13 Sitolo follows that model while retaining a provider abstraction.

Required secret classes include database credentials, identity credentials, payment credentials, webhook verification secrets, MRA credentials/terminal secret material, object-storage credentials, telemetry exporter credentials and internal service credentials.

Architecture:

```text
typed SecretRef
      ↓
SecretProvider trait
      ↓
selected provider adapter
      ↓
SecretValue
      ↓
narrow consumer capability
```

No general `HashMap<String, String>` of secrets should be injected into every service. That makes least privilege nearly impossible and creates accidental logging hazards.

# 13. Secret Provider Boundary

Conceptual interface:

```rust
#[async_trait]
pub trait SecretProvider: Send + Sync {
    async fn get(&self, reference: &SecretRef) -> Result<SecretValue, SecretError>;
}
```

Potential implementations can include a development-only local provider and an approved production provider. The interface is stable; the implementation is replaceable. The domain does not know whether the credential came from a cloud secret manager, Vault-like service, injected environment value, or development fixture.

The production provider must be selected through deployment policy, not developer preference. A local-file provider must hard-fail when the runtime environment is production.

# 14. Secret Types and Memory Hygiene

Raw secret values should be distinct from ordinary strings. Secret-bearing types should not implement broad `Debug`, `Display` or `Serialize` behavior accidentally. Where justified, zeroization can reduce residual-memory exposure, but it is defense in depth; correct architecture matters more.

Never place raw secrets inside a long-lived request context, error message, metric attribute, tracing field or domain object. A secret should exist only in the narrowest component that needs it and for the shortest practical lifetime.

# 15. Secret Rotation

Rotation must support an explicit lifecycle:

```text
issue K2
   ↓
propagate K2
   ↓
validate K2
   ↓
switch consumers
   ↓
observe stable operation
   ↓
revoke K1
```

For providers supporting overlap, K1 and K2 may coexist temporarily. If overlap is impossible, the rotation runbook must explicitly describe the expected availability window. No component is allowed to retain an old credential forever merely because rotation is inconvenient.

The telemetry for rotation may include secret class, workload, operation ID and outcome, but never the secret value.

# 16. Secret Failure Modes

Expected behavior:

| Failure | Required behavior |
|---|---|
| secret missing | startup failure if mandatory |
| secret provider unavailable | capability fails closed |
| secret invalid | startup/capability failure |
| secret expired | fail closed according to capability |
| provider authorization denied | security event + failure |
| stale cached secret | only inside explicit TTL policy |
| rotation mismatch | bounded failure, never silent fallback |

Forbidden pattern:

```rust
if secret_missing { use_default_secret(); }
```

A secret fallback embedded in source is not resilience; it is credential compromise waiting to happen.

# 17. Logging Architecture

Rust services should use `tracing` and `tracing-subscriber` rather than direct `println!` for application diagnostics. `tracing` provides structured spans/events, while `tracing-subscriber` provides filtering/formatting facilities. citeturn487252search11 The current `EnvFilter` API supports strict environment parsing; security-sensitive production filtering should use strict validation rather than silently accepting malformed directives. citeturn487252search5

Libraries emit events. Executables own process-global subscriber initialization. This avoids duplicate subscribers and allows API and worker executables to configure exporters appropriately.

# 18. Structured Log Schema

A base event should look like:

```json
{
  "timestamp":"2026-09-06T07:00:00Z",
  "severity":"INFO",
  "service.name":"sitolo-api",
  "service.version":"0.1.0",
  "deployment.environment.name":"staging",
  "event.name":"sale.finalize.completed",
  "operation":"sale_finalize",
  "request_id":"req_01J...",
  "trace_id":"4f3...",
  "result":"success"
}
```

Events add only fields justified by the event. Do not create a giant common schema that automatically carries every possible business field. The more generic the serializer, the easier it becomes for a later refactor to introduce a secret or customer dataset into production logs.

# 19. Forbidden Log Data

At minimum, never log raw passwords, password hashes, access tokens, refresh tokens, JWTs, authorization headers, cookies, OTPs, TOTP seeds, recovery codes, API keys, payment secrets, MRA terminal secret material, private keys, raw encryption keys, CVV/CVC or complete payment instruments.

Avoid unnecessary copies of complete PII, identity documents, full sale bodies and raw provider payloads. OWASP's logging guidance treats security logging as a first-class control and explicitly emphasizes controlling sensitive data in logs, protecting the log system and validating the resulting implementation. citeturn487252search12

# 20. Redaction Model

Redaction is defense in depth, not the primary data-classification mechanism. Use layers:

```text
typed secret values
  ↓
explicit field selection
  ↓
safe serialization
  ↓
redaction policy
  ↓
collector policy
  ↓
CI regression tests
```

If redaction fails, the unsafe event must not be emitted. Emit a smaller safe failure event instead. Never choose the fallback `redaction failed → emit raw payload` because that turns a subsystem defect directly into credential disclosure.

# 21. Request IDs and Correlation

Every inbound HTTP request gets a bounded opaque request ID. A client-supplied request ID can be accepted only after length/format validation and remains diagnostic data; it is never authority. The same ID must appear consistently in safe API errors and corresponding server logs.

The correlation graph may include:

```text
request_id
trace_id
span_id
command_id
sale_id
payment_intent_id
outbox_id
job_id
provider_transaction_id
sync_batch_id
```

These identifiers are useful in logs/audit but are not default metric labels because unbounded identifiers would create high-cardinality time series.

# 22. Trace Context

Use W3C Trace Context for compatible HTTP boundaries. The W3C model exists to propagate trace context across independently instrumented components. Trace context must always be treated as untrusted input: an attacker can forge it, and a forged trace ID must not affect authorization, tenant scope or business state.

```text
traceparent → correlation only
tracestate  → correlation only
```

Authentication and authorization are established independently from trace propagation.

# 23. Error Taxonomy

Use typed semantic error layers:

```text
DomainError
ValidationError
AuthenticationError
AuthorizationError
ApplicationError
PersistenceError
IntegrationError
ConfigurationError
SecretError
TelemetryError
TransportError
```

The hierarchy matters because the client, retry engine, telemetry, metrics and incident response all need different semantics. A raw `sqlx::Error` is not a useful public contract. Conversely, turning an `INSUFFICIENT_STOCK` business rejection into a generic 500 destroys the distinction between expected business behavior and platform failure.

# 24. Retryability and Unknown Outcome

Every failure crossing an operation boundary should have explicit retry semantics:

```text
NotRetryable
Retryable
RetryAfter(duration)
UnknownOutcome
```

`UnknownOutcome` is critical for payments and external integrations. If a request was sent and the connection timed out, Sitolo may not know whether the provider accepted it. Automatic retry can create a duplicate side effect. The correct next action may therefore be reconciliation, not retry.

This mirrors the project's existing payment and EIS approach: external failure must not rewrite or invent local business truth.

# 25. Problem Details Mapping

The API contract already requires RFC 9457-style problem details. Phase 2 supplies the internal mapping rather than inventing a parallel response shape.

```text
internal error
    ↓
classify
    ↓
choose client-safe code
    ↓
choose status
    ↓
choose retryability
    ↓
attach request_id
    ↓
Problem Details
```

Production responses never include stack traces, SQL text, environment variables, filesystem paths, raw provider responses, private network addresses or secret values. The current API contract explicitly defines these restrictions.

# 26. Error Registry

Every client-visible error code has a registry record:

```text
code
meaning
HTTP mapping
retryability
security sensitivity
safe detail policy
telemetry family
owner
introduced version
deprecation status
```

Changing code, status, retryability or field-error semantics is an API compatibility change and must follow the existing API versioning policy. A new public error code without a registry entry must fail CI.

# 27. Panic and Unwrap Policy

Expected runtime failure must use `Result`, not panic. `unwrap`/`expect` are acceptable only when a proven internal invariant makes process termination the intentionally correct behavior. They are inappropriate for request data, environment configuration, network I/O, secret retrieval, provider responses and database availability.

The goal is not to ban panic. The goal is to prevent accidental process-wide failure where a controlled error state is required.

# 28. Provider Error Normalization

External providers are untrusted inputs and may return huge or hostile payloads. Normalize provider errors into bounded internal classifications:

```text
Timeout
Unavailable
AuthenticationFailed
RateLimited
Rejected(code)
InvalidResponse
UnknownOutcome
```

Store provider-specific details only where they are needed for reconciliation or debugging. Bound the length of provider messages before telemetry. Never mirror raw provider bodies to API consumers.

# 29. Metrics Contract

Metrics answer population-level questions: rate, duration, count, saturation, queue depth and aggregate error class. The existing observability specification defines Sitolo's broader metric families. Phase 2 supplies the registration and instrumentation substrate.

Baseline names include:

```text
sitolo_http_requests_total
sitolo_http_request_duration_seconds
sitolo_http_in_flight_requests
sitolo_errors_total
sitolo_configuration_load_failures_total
sitolo_secret_access_failures_total
sitolo_telemetry_export_failures_total
sitolo_telemetry_dropped_records_total
sitolo_db_pool_wait_seconds
sitolo_db_query_duration_seconds
sitolo_db_query_errors_total
sitolo_db_transaction_duration_seconds
sitolo_worker_jobs_total
```

The exact later-domain metrics remain governed by `observability_spec.md`.

# 30. Metric Cardinality

Metric labels must come from closed or tightly bounded vocabularies. Prometheus guidance warns that every unique label combination creates a time series and specifically discourages unbounded user identifiers or similar values. citeturn487252search12

Safe examples:

```text
method
route_template
status_class
operation
result
error_family
integration
job_type
```

Forbidden as general metric labels:

```text
user_id
email
phone_number
sale_id
payment_intent_id
request_id
trace_id
command_id
provider_transaction_id
free-form_error_message
```

Exact identifiers belong in logs/traces/audit when justified.

# 31. Tracing Strategy

Trace meaningful boundaries rather than every function. Recommended spans:

```text
HTTP request
  ├─ auth
  ├─ tenant context
  ├─ authorization
  ├─ application command
  │    ├─ transaction
  │    ├─ audit
  │    └─ outbox
  └─ response
```

Workers have job execution, persistence and external-call spans. Database spans use logical operation names such as `sale_finalize`, not raw SQL text. Traces may be sampled, but sampling must never control whether a business mutation, audit record or payment record is persisted.

# 32. OpenTelemetry Baseline

OpenTelemetry's current Rust documentation provides traces, metrics and logs through the Rust ecosystem and currently marks the major language components as beta. The current getting-started guidance also shows the `tracing` bridge for log records. citeturn487252search0turn487252search6 The Logs SDK specification is stable at the specification level except where explicitly noted. citeturn487252search4

Sitolo should therefore use OpenTelemetry for interoperability but preserve its own event, metric and redaction contracts. Versions of the OpenTelemetry crates, collector compatibility and semantic convention version must be pinned and recorded. A dependency upgrade is not merely a Cargo update; it is a telemetry schema compatibility event.

# 33. Telemetry Backpressure

Telemetry is a potentially unbounded producer. The runtime must impose:

```text
max queue records
max queue bytes where practical
max event size
max attributes
max attribute value length
max batch size
export timeout
retry budget
shutdown flush deadline
```

An unlimited exporter queue is unacceptable because an OTLP outage could become a memory exhaustion incident. Tokio's asynchronous model does not make queued futures free; a growing queue still consumes memory and scheduling capacity.

# 34. Telemetry Failure State Machine

```text
HEALTHY
  ↓ repeated export failure
DEGRADED
  ↓ queue threshold
PRESSURED
  ↓ buffer capacity reached
DROPPING
  ↓ sustained inability to export
BLACKOUT
  ↓ exporter recovers
RECOVERING
  ↓ stable exports
HEALTHY
```

Telemetry may degrade without blocking an ordinary sale, but authoritative audit evidence cannot be replaced by telemetry. Local fallback logging remains redacted and bounded. Never switch to dumping complete request payloads merely because remote telemetry is unavailable.

# 35. Telemetry Trust Boundary

```text
Application
   ↓ authenticated / controlled export
Collector
   ↓ authorized routing
Metrics backend / trace backend / log backend
   ↓ least-privilege access
Operators
```

The telemetry plane itself is attackable through injection, PII leakage, cardinality explosion, resource exhaustion and cross-tenant visibility mistakes. Remote collectors require appropriate authentication and TLS. Dashboards that expose tenant-sensitive data must enforce the same least-privilege principles as application data access.

# 36. Audit versus Telemetry

Audit is durable accountability evidence; telemetry is operational evidence.

```text
AUDIT
- durable
- authoritative evidence
- tied to actor/effective subject
- tied to business/security operation
- must survive exporter failure

TELEMETRY
- diagnostic
- sampled where acceptable
- bounded
- may be dropped under controlled pressure
- never authoritative
```

A refund approval may produce one durable audit record, one trace, several structured logs and one metric increment. Those records are related through identifiers but have different authority and retention characteristics.

# 37. Health and Readiness

Liveness is intentionally cheap and asks whether the process is alive. Readiness asks whether the instance should receive normal traffic. Capability health can represent individual degraded dependencies.

A health endpoint must never disclose full configuration, DB passwords, provider credentials, raw connection strings, stack traces or unrestricted dependency responses.

Preferred decomposition:

```text
/process/live
/process/ready
/capabilities/summary
```

The exact paths remain an API/deployment decision. The contract is the information boundary and the fact that diagnostic detail is not automatically public.

# 38. Timeout and Resource Policy

Every network/I/O boundary needs a bounded timeout unless a documented exception exists. At minimum: secret provider, PostgreSQL acquisition/query paths, telemetry exporters, external payment/EIS calls, worker operations and HTTP clients.

A timeout is a resource-control mechanism:

```text
slow provider
  ↓
waiting tasks
  ↓
concurrency saturation
  ↓
queue growth
  ↓
memory pressure
  ↓
outage
```

Hard timeout ceilings prevent operators from turning a transient dependency problem into a resource-exhaustion problem by setting pathological timeout values.

# 39. CPU versus I/O

The runtime must distinguish CPU-heavy work from I/O waits. Redaction, large serialization and cryptographic operations can be CPU intensive; secret retrieval, PostgreSQL access and remote telemetry are I/O-bound. Use async tasks for I/O, bounded worker execution for truly CPU-heavy workloads and `spawn_blocking` only for blocking operations that cannot be made asynchronous.

A common anti-pattern is to push every operation into a worker thread for perceived safety. That adds scheduling overhead and can make an I/O-bound system slower. The opposite anti-pattern is performing large CPU work directly on latency-critical Tokio executor threads. Phase 2 should provide instrumentation so these decisions are measurable rather than ideological.

# 40. Worker Correlation

Durable outbox/worker execution crosses process and restart boundaries. Correlation must therefore survive persistence. A request-local context is not enough.

```text
request_id / trace_id
      + command_id
      + outbox_id
      ↓ persisted envelope
worker execution
      ↓
job_id + child trace/span
```

A worker must never trust that an in-memory context corresponds to the durable business event after a restart. Persist only the identifiers actually required; do not serialize arbitrary request context.

# 41. Offline Sync Telemetry

Offline operation creates a second correlation surface. Client commands can carry command IDs and sync-batch identifiers, but these are selectors/correlation data, not authority. Server telemetry should capture aggregate queue age, command outcomes, conflict classes, retries and revoked-device attempts. Exact payloads belong in the sync evidence model when required.

The implementation must preserve the fundamental invariant from the project: offline capability is bounded authority and PostgreSQL remains authoritative server-side.

# 42. Payment and EIS Telemetry Hooks

Payment instrumentation must distinguish sale, payment intent, attempt, provider transaction, webhook event and reconciliation records. EIS instrumentation must distinguish local sale, fiscal submission, terminal, configuration version and external fiscal outcome. These are not aliases.

Unknown external outcomes should be visible as first-class signals:

```text
payment.unknown_outcome
EIS.unknown_outcome
```

No telemetry path can turn such an outcome into an assumed success merely to make dashboards look healthy.

# 43. Configuration and Secret Telemetry

Safe events include:

```text
configuration.loaded
configuration.validation.failed
secret.access.denied
secret.rotation.started
secret.rotation.completed
secret.rotation.failed
```

These events may include environment, workload, secret class, config schema version, config fingerprint and operation ID. They must never include the raw credential or a complete reference if exposing the reference would itself create unnecessary infrastructure disclosure.

# 44. Event and Metric Registries

Maintain machine-readable registries such as:

```text
docs/telemetry/events.yaml
docs/telemetry/metrics.yaml
docs/telemetry/redaction.yaml
```

An event definition should specify name, schema version, classification, owner and allowed fields. A metric definition should specify name, type, unit and approved labels. A redaction policy should define forbidden classes. CI can then reject unregistered events, undocumented metrics and new unapproved label dimensions.

# 45. Error Serialization Tests

Every release must test actual serialized HTTP responses, not only internal Rust types. Assertions include:

```text
500 contains no stack trace
500 contains no SQL
500 contains no filesystem path
500 contains no secret
401 does not enumerate accounts
403 does not expose policy internals
external error does not mirror raw provider body
field errors remain bounded
```

Unknown internal errors must still produce a safe `INTERNAL_ERROR` response with a request ID. Internally, the system should record that an unexpected mapping occurred so the issue cannot hide behind the generic response forever.

# 46. Secret Redaction Tests

Synthetic test values must be injected through error, logging, tracing and serialization paths. A successful test proves the exact secret byte sequence cannot be found in the resulting sink payload. Test nested serialization, not only top-level fields.

Examples:

```text
TEST_SECRET_DATABASE=TEST_ONLY_DATABASE_SECRET_001
TEST_SECRET_PAYMENT=TEST_ONLY_PAYMENT_SECRET_002
TEST_SECRET_MRA=TEST_ONLY_MRA_SECRET_003
```

The same values should be searched for in generated logs, traces, metrics, HTTP responses, startup diagnostics and exported fixtures.

# 47. Configuration Fuzzing

Fuzz configuration parsing with malformed URLs, huge numeric values, invalid durations, negative bounds, unknown keys, duplicate keys, weird Unicode, missing required fields, invalid enum variants and pathological string sizes. The required property is deterministic behavior: either a valid normalized configuration is produced or an explicit bounded validation failure occurs. Parser behavior must not become an indirect availability vulnerability.

# 48. Error Fuzzing

Fuzz provider error text, parser failures, nested error chains and arbitrary external error codes. The safe serializer must remain bounded, valid and non-secret under hostile input. This is particularly important because external dependencies are allowed to return data that the application did not generate and therefore cannot automatically trust.

# 49. Telemetry Fuzzing

Fuzz event fields, user-controlled labels, traceparent values, request identifiers and large nested diagnostic values. The test properties are:

```text
no panic
bounded allocation behavior for configured test limits
no forbidden fields
no dynamic metric name creation
valid serialized telemetry
trace context cannot alter authorization
```

The fuzz harness must operate against the real sanitization/serialization code, not a simplified mock implementation.

# 50. CI Enforcement

Phase 2 is incomplete if its rules live only in Markdown. CI should fail on:

```text
hardcoded secret patterns
unsafe production secret provider
unregistered metric labels
missing metric units
unregistered public error code
missing event schema version
redaction regression
unsafe production debug setting
configuration schema failure
telemetry schema failure
```

The inherited Phase 1 gates remain mandatory: format, lint, build/check, unit/integration tests, lockfile policy, secret scanning, supply-chain checks and artifact/provenance verification. A failed scanner or missing security job is not green by default.

# 51. Static Architecture Checks

Where practical, automated policy checks should detect:

```text
std::env::var outside sitolo-config
println!/dbg! in production paths covered by policy
secret-bearing types deriving Serialize/Debug unintentionally
tracing!(?request) for sensitive request DTOs
dynamic metric names
dynamic unbounded metric labels
provider raw-body passthrough into responses
```

These checks must have narrowly scoped legitimate allowlists. A grep rule that developers disable because it creates noise is not a security control.

# 52. Operational Runbooks

Phase 2 must ship with at least four runbooks.

### Secret compromise
Identify class → determine blast radius → rotate/revoke → invalidate dependent credentials if required → search exposure evidence → verify replacement → deploy regression → record incident.

### Telemetry blackout
Confirm business paths are still healthy → inspect queue pressure → confirm bounded drops → verify local safe logs → restore exporter → inspect gaps → ensure no telemetry-dependent business behavior was accidentally introduced.

### Configuration poisoning
Freeze rollout → identify fingerprint → compare approved revision → revert → inspect deployment identity → preserve evidence → investigate trust boundary.

### Error storm
Check deploy/config changes → classify error family → inspect DB pool/locks/provider state → inspect retry amplification → protect authoritative transaction paths → capture reproducer.

# 53. Release Evidence

A production release should be attributable to:

```text
source revision
artifact digest
Rust toolchain
Cargo.lock dependency resolution
config schema version
config fingerprint
telemetry library versions
semantic convention version
security/test results
secret-provider implementation identity
```

This fits the existing Sitolo release model in which artifacts and deployments must be reproducible and traceable. Runtime telemetry should make the running identity visible, while deployment evidence remains the authoritative source for release provenance.

# 54. Advantages

Centralized runtime contracts prevent every feature from inventing configuration loading, credential access, error mapping and instrumentation. Typed errors preserve business semantics. Structured telemetry enables low-cost correlation. Explicit secret boundaries simplify least privilege and rotation. Bounded exporter queues prevent observability outages from becoming memory outages. CI enforcement turns documentation into executable engineering policy. Most importantly, later identity, tenant, inventory, POS, payment, sync and EIS phases inherit the same substrate instead of creating divergent infrastructure.

# 55. Disadvantages

This architecture has real costs: more types, more startup validation, more review surface, telemetry registry maintenance, controlled dependency upgrades, stronger CI and more deliberate operational procedures. It can also make development feel slower than putting values in `.env` and printing objects. Those costs are intentional because Sitolo handles financial facts, multi-tenant state, offline retries and regulated integration boundaries. The alternative is not “less complexity”; it is undocumented complexity that appears later as incidents.

# 56. Why This Over Alternatives

### Ad-hoc env reads vs typed config
Typed config provides one validation point, reproducibility and inspectable dependencies.

### `println!` vs `tracing`
Structured tracing supports spans/events and composable filtering instead of text-only output. citeturn487252search11

### Raw logging vs OpenTelemetry-compatible signals
OpenTelemetry provides interoperability across traces, metrics and logs while Sitolo retains control over its own event and security contracts. The current Rust implementation documents these capabilities and their present maturity. citeturn487252search0

### `anyhow::Error` everywhere vs typed errors
Typed errors preserve domain, retry and transport semantics.

### Scattered credentials vs secret provider
Centralized provisioning and rotation reduce credential sprawl and are directly aligned with OWASP secret-management guidance. citeturn487252search13

### Unlimited telemetry vs bounded queues
Bounded queues preserve service availability under exporter failure.

### Generic CRUD diagnostics vs explicit operation telemetry
Explicit operation names (`sale_finalize`, `inventory_receive`, `refund_create`) are safer and more useful than leaking tables, SQL and raw payloads.

# 57. Phase 2 Implementation Order

```text
01 config crate and schema
02 source precedence and normalization
03 startup validation and hard ceilings
04 config fingerprint
05 secret reference abstraction
06 production secret adapter
07 secret rotation hooks
08 redaction primitives
09 tracing subscriber setup
10 request ID/correlation
11 typed error taxonomy
12 Problem Details mapping
13 metric registry
14 OpenTelemetry integration
15 telemetry backpressure
16 database/worker/integration instrumentation
17 event/metric/redaction registries
18 security/config/error/telemetry test harness
19 CI enforcement
20 runbooks and production verification
21 Phase 2 certification
```

This order prevents feature modules from establishing local conventions before platform semantics are frozen.

# 58. Phase 2 Definition of Ready

Before coding starts, all upstream dependencies are identified; no duplicate architecture contract is being created; error semantics are aligned with the existing API contract; telemetry semantics are aligned with the existing observability specification; secret ownership boundaries are understood; Phase 1 workspace boundaries are available; open decisions are recorded rather than guessed.

# 59. Phase 2 Definition of Done

```text
[ ] typed configuration exists
[ ] precedence is deterministic
[ ] startup validation is fail-closed
[ ] config fingerprint excludes secrets
[ ] secret references are typed
[ ] production provider is enforced
[ ] local secret provider cannot run in production
[ ] rotation path exists
[ ] redaction is active
[ ] structured logging is centralized
[ ] request correlation is consistent
[ ] W3C trace context is handled as untrusted diagnostic data
[ ] typed errors exist
[ ] API Problem Details mapping exists
[ ] retryability is explicit
[ ] provider unknown-outcome semantics exist
[ ] metrics are centrally registered
[ ] labels are bounded
[ ] telemetry exporters have timeouts and bounds
[ ] telemetry outage has tested degradation behavior
[ ] database/worker/integration hooks exist
[ ] event/metric/redaction registries exist
[ ] redaction tests pass
[ ] config fuzz/property tests pass
[ ] error serialization tests pass
[ ] CI blocks defined policy failures
[ ] runbooks exist
[ ] release evidence is recorded
```

The final gate is evidence, not confidence or document length.

# 60. Architecture Decision Boundary

Any future change that modifies the trust model, secret source, error semantics, telemetry authority, tenant visibility through diagnostics, timeout safety model, or mandatory evidence path must undergo architecture review and may require an ADR. Internal refactoring that preserves these observable contracts does not require architectural reinvention.

Do not silently “fix” Phase 2 by introducing a second configuration crate, second logger, second error envelope, or provider-specific secret path. That fragments the platform and recreates the exact inconsistencies this phase exists to eliminate.

# 61. Final Contract and Phase 3 Handoff

At the end of Phase 2, a Rust executable should be able to answer before serving traffic: what configuration is active, what secrets are required, what artifact is running, how trust-sensitive startup validation passed, how a request will be correlated, how errors will be classified, how telemetry will be bounded and how failures will degrade.

Phase 3 may then implement identity, sessions, MFA and devices assuming these facilities already exist. It must consume the common configuration, secret, error and telemetry contracts instead of introducing parallel infrastructure.

The final invariant is:

```text
configuration selects runtime behavior
secrets provide controlled access
errors preserve semantics
telemetry explains behavior
PostgreSQL preserves business truth
audit preserves accountability
```

Anything that violates this hierarchy is an architecture defect, even if the code compiles.

# 62. External Research Basis

The Phase 2 design was cross-checked against current public guidance available in September 2026:

- Rust `config` documentation for layered configuration and environment providers. citeturn487252search3turn487252search10
- `tracing-subscriber` formatting and `EnvFilter` documentation. citeturn487252search11turn487252search5
- OpenTelemetry Rust language documentation and current Rust getting-started material. citeturn487252search0turn487252search6
- OpenTelemetry Logs SDK specification. citeturn487252search4
- OWASP Logging Cheat Sheet. citeturn487252search12
- OWASP Secrets Management Cheat Sheet. citeturn487252search13
- NIST SP 800-218 SSDF 1.1. citeturn487252search8
- NIST SP 800-218 Rev. 1 / SSDF 1.2 initial public draft as a current governance reference. citeturn487252search14


# Appendix A — Canonical Runtime Configuration Catalogue

The following catalogue is the implementation baseline. Exact values are deliberately not hardcoded where they depend on deployment capacity, but the semantic type, security class and validation behavior are fixed.

| Configuration | Type | Class | Required | Example | Hard rule |
|---|---|---|---:|---|---|
| `environment` | enum | A | yes | `production` | unknown value fails startup |
| `service_name` | string | A | yes | `sitolo-api` | bounded and non-empty |
| `service_version` | string | A | yes | `2026.09.06` | provenance field; immutable at runtime |
| `bind_address` | socket address | A | yes | `0.0.0.0:8080` | parser validates |
| `max_request_body_bytes` | u64 | A | yes | `2097152` | bounded hard maximum |
| `request_header_timeout_ms` | duration | C | yes | `5000` | positive; hard ceiling |
| `keepalive_timeout_ms` | duration | C | yes | `30000` | positive; hard ceiling |
| `db_host` | host | A | yes | `postgres.internal` | no credentials |
| `db_port` | u16 | A | yes | `5432` | valid TCP port |
| `db_name` | string | A | yes | `sitolo` | bounded |
| `db_user` | string | A | yes | `sitolo_api` | least-privileged role required |
| `db_password_ref` | SecretRef | B | yes | `prod/sitolo/db` | raw password prohibited |
| `db_pool_min` | u32 | C | yes | `5` | upper bounded |
| `db_pool_max` | u32 | C | yes | `20` | must be >= min and hard bounded |
| `db_acquire_timeout_ms` | duration | C | yes | `2000` | bounded |
| `otel_endpoint` | URL | A/B | conditional | `https://collector` | TLS required for remote |
| `otel_export_timeout_ms` | duration | C | yes | `3000` | bounded |
| `otel_max_queue` | u32 | C | yes | `2048` | bounded |
| `trace_sample_ratio` | decimal | C | yes | `0.1` | 0..=1 |
| `log_level` | enum | C | yes | `info` | production ceiling applies |
| `allow_local_secret_provider` | bool | A/security | yes | `false` | production must be false |
| `config_schema_version` | integer | A | yes | `2` | unknown schema fails |

The implementation should expose a safe diagnostic representation of this catalogue in tests. The safe representation reports presence, source and validation state but never raw secret values.

# Appendix B — Configuration Validation Matrix

Validation is layered rather than performed by a single permissive parser.

```text
syntax validation
      ↓
type validation
      ↓
semantic validation
      ↓
security validation
      ↓
environment validation
      ↓
cross-field validation
      ↓
final AppConfig
```

Examples of cross-field validation:

```text
pool_max >= pool_min
trace_sample_ratio <= 1
retry_delay <= retry_ceiling
request_timeout <= hard_request_timeout_ceiling
local_secret_provider == false when environment == production
TLS required when telemetry endpoint is remote
secret references required when corresponding capability is enabled
```

A field may be individually valid and the configuration still be invalid as a whole. Phase 2 therefore requires both field-level and cross-field validation.

# Appendix C — Configuration Security Invariants as Tests

The following should become executable tests rather than comments:

```text
production cannot use development secret provider
production cannot enable unrestricted debug responses
negative timeout rejected
zero timeout rejected where timeout is mandatory
negative pool size rejected
pool max below pool min rejected
sampling ratio above 1 rejected
sampling ratio below 0 rejected
untrusted remote telemetry without TLS rejected
required secret reference absent rejected
empty production service identity rejected
unknown critical environment rejected
configuration fingerprint changes after non-secret effective config change
configuration fingerprint does not change because only secret value changed
```

That last pair is particularly important. Secret rotation should normally not force a new fingerprint containing the secret. The fingerprint represents non-secret effective configuration, while secret versions are tracked separately and only as safe metadata.

# Appendix D — Secret Capability Model

Do not hand one global secret object to the application.

Prefer capability-specific handles:

```text
DatabaseCredential
PaymentCredential
WebhookVerificationSecret
MraTerminalSecret
TelemetryExporterCredential
SigningKeyHandle
```

Each capability should expose only operations required by its consumer. A payment adapter may need to sign or authenticate a provider call; it does not need to inspect arbitrary MRA credentials.

Conceptually:

```rust
pub struct PaymentCredential(/* intentionally non-Debug */);
pub struct MraTerminalSecret(/* intentionally non-Serializable */);
```

This is an architectural guardrail. Rust's type system cannot stop every memory disclosure, but distinct secret types reduce accidental generic handling.

# Appendix E — Secret Rotation State Machine

```text
ACTIVE(K1)
    |
    | rotation started
    v
ROTATING
  /    \
 /      \
K1+K2    K2_ONLY
  |        |
  |        | validation
  |        v
  |     ACTIVE(K2)
  |
  | rollback
  v
ACTIVE(K1)
```

An implementation that cannot prove K2 is usable must not revoke K1 prematurely when the provider supports overlap.

Rotation events should be idempotent. Repeating a rotation command must not accidentally generate unbounded keys or leave multiple active credentials without explicit policy.

# Appendix F — Secret Access Audit Fields

Where secret access itself is audited, the record can contain:

```text
operation_id
workload_id
environment
secret_class
provider_class
reference_fingerprint
result
occurred_at
```

Do not store raw secret material in the audit event. A secret reference fingerprint can be useful for correlation without disclosing the reference string itself where that matters.

# Appendix G — Logging Event Catalogue

Baseline operational events:

```text
service.starting
service.started
service.ready
service.shutdown.started
service.shutdown.completed
configuration.loaded
configuration.validation.failed
secret.access.failed
secret.rotation.started
secret.rotation.completed
http.request.started
http.request.completed
application.error
worker.job.started
worker.job.completed
worker.job.failed
telemetry.export.failed
telemetry.buffer.pressured
telemetry.record.dropped
database.pool.saturated
database.timeout
external.request.started
external.request.completed
external.request.unknown_outcome
```

Every event must define an owner and a documented field set.

# Appendix H — Security Event Catalogue

Authentication/security events inherited by later phases:

```text
LOGIN_SUCCESS
LOGIN_FAILURE
MFA_SUCCESS
MFA_FAILURE
SESSION_CREATED
SESSION_REVOKED
SESSION_EXPIRED
REFRESH_ROTATED
REFRESH_REUSE_DETECTED
PASSWORD_CHANGED
PASSWORD_RESET_REQUESTED
PASSWORD_RESET_COMPLETED
MFA_ENROLLED
MFA_RESET
DEVICE_REGISTERED
DEVICE_REVOKED
AUTHZ_DENY
AUTHZ_ALLOW_HIGH_RISK
ROLE_ASSIGNED
ROLE_REMOVED
MEMBERSHIP_SUSPENDED
MEMBERSHIP_REVOKED
SCOPE_CHANGED
APPROVAL_REQUESTED
APPROVAL_GRANTED
APPROVAL_REJECTED
STEP_UP_REQUIRED
STEP_UP_COMPLETED
SUPPORT_ACCESS_GRANTED
SUPPORT_ACCESS_EXPIRED
```

These names match the project's identity/security contract and should be emitted through the Phase 2 event mechanism rather than each phase building a separate logging convention.

# Appendix I — Error Family Catalogue

```text
validation
authentication
authorization
not_found
conflict
concurrency
idempotency
invariant
rate_limit
dependency_unavailable
external_rejected
unknown_outcome
configuration
secret
telemetry
internal
```

The error family is a bounded operational classification. It is not a free-form sentence.

# Appendix J — Public Error Code Examples

```text
AUTHENTICATION_FAILED
AUTHORIZATION_DENIED
TENANT_SCOPE_DENIED
RESOURCE_NOT_FOUND
VALIDATION_ERROR
RATE_LIMITED
CONFLICT
PRECONDITION_FAILED
IDEMPOTENCY_KEY_REUSED_WITH_DIFFERENT_REQUEST
DEPENDENCY_UNAVAILABLE
INTERNAL_ERROR
PAYMENT_PROVIDER_UNAVAILABLE
PAYMENT_UNKNOWN_OUTCOME
EIS_REJECTED
EIS_UNKNOWN_OUTCOME
```

Public error codes must be documented before clients depend on them. The wording of `detail` may change when it remains safe, but the code's semantic meaning must remain stable.

# Appendix K — Error-to-HTTP Matrix

| Internal class | Example | HTTP | Public code |
|---|---|---:|---|
| validation | invalid quantity | 422 | `VALIDATION_ERROR` |
| authentication | invalid credential | 401 | `AUTHENTICATION_FAILED` |
| authorization | missing permission | 403 | `AUTHORIZATION_DENIED` |
| anti-enumeration | hidden object | 404 | `RESOURCE_NOT_FOUND` |
| conflict | stale version | 409/412 | `CONFLICT` / `PRECONDITION_FAILED` |
| concurrency | stock conflict | 409 | domain-specific conflict |
| idempotency | key reused | 409 | `IDEMPOTENCY_KEY_REUSED_WITH_DIFFERENT_REQUEST` |
| rate limit | quota reached | 429 | `RATE_LIMITED` |
| dependency | DB unavailable | 503 | `DEPENDENCY_UNAVAILABLE` |
| upstream protocol | malformed provider response | 502 | `UPSTREAM_INVALID_RESPONSE` |
| upstream timeout | provider timeout | 504 where appropriate | provider-specific safe code |
| internal | unexpected bug | 500 | `INTERNAL_ERROR` |

The matrix does not eliminate route-specific semantics; it establishes a default that prevents arbitrary mapping.

# Appendix L — Error Mapping Rules

1. Mapping must be total: every internal error reaches a safe external class.
2. Mapping must be deterministic.
3. Mapping must preserve retryability internally.
4. Mapping must attach request correlation.
5. Mapping must never serialize arbitrary error source chains.
6. Mapping must redact provider/external data before constructing client details.
7. Mapping must not convert unknown outcomes into ordinary retryable failures.
8. Mapping must not use raw exception text as a stable public code.

# Appendix M — Request Correlation Contract

Every request has:

```text
server_request_id
optional client_request_id
trace_id when tracing active
operation_name
```

The server-generated request ID is the authoritative support correlation value returned to clients.

Validation constraints:

```text
maximum length: bounded
allowed alphabet: restricted
no whitespace/control characters
no credentials
no embedded PII
```

# Appendix N — Trace Propagation Contract

Inbound:

```text
read traceparent
validate syntax
accept as diagnostic context
never authorize from it
```

Outbound:

```text
create child span
propagate context
```

At process boundaries:

```text
request context
    ≠
credential context
```

Trace context may be propagated through workers and provider requests, but secrets/authorization state must be independently resolved.

# Appendix O — Metric Registry Rules

Each metric definition must state:

```text
name
kind
unit
description
owner
allowed labels
forbidden labels
cardinality expectation
```

The registry should be version controlled with source changes. A metric rename is a compatibility change for dashboards and alerts and should therefore be treated as an operational API change.

# Appendix P — Telemetry Resource Budget

Initial defaults must be measurable and tunable rather than hard-coded into individual event sites.

Suggested controls:

```text
max_event_bytes
max_attribute_bytes
max_attributes_per_event
max_metric_labels
max_label_value_bytes
max_spans_per_request
max_queued_records
max_queued_bytes
export_timeout
max_export_retries
shutdown_flush_deadline
```

Resource limits must be tested at their boundary values and one past their boundary.

# Appendix Q — Telemetry Loss Policy

Telemetry classes have different loss tolerances.

```text
ordinary trace sample            → may be lost
ordinary debug log              → may be lost under controlled pressure
aggregate metric sample         → minimize loss, but bounded
security event                  → stronger durability requirement
financial audit                 → authoritative persistence, not telemetry
regulatory evidence             → governed by integration contract
```

This avoids the false choice between “never lose telemetry” and “telemetry does not matter.” Different evidence types have different correctness properties.

# Appendix R — Telemetry Backpressure Algorithm

Conceptually:

```text
queue < 50%       → normal
50–75%            → warning signal
75–90%            → pressure state
90–100%           → shedding low-priority records
100%              → bounded drop policy
```

Priority classes may be:

```text
P0 security-critical
P1 high-value business diagnostics
P2 normal operational telemetry
P3 verbose/debug telemetry
```

If shedding is required, discard lower-priority telemetry first. Never drop authoritative database/audit writes because the telemetry queue is full.

# Appendix S — Retry/Backoff Policy

Exponential backoff should have:

```text
base delay
maximum delay
attempt ceiling
jitter
classification
```

Example conceptual sequence:

```text
250ms
500ms
1s
2s
4s
max 30s
```

Values are examples, not frozen production numbers.

Retry policy must be tied to operation semantics. A retry is safe only when duplicate effects are impossible or idempotency/reconciliation semantics explicitly make it safe.

# Appendix T — Jitter Requirement

Workers performing the same telemetry/provider retry simultaneously can create a thundering herd.

Use bounded jitter where retries are appropriate:

```text
backoff = deterministic_base + bounded_random_jitter
```

The random source should be cryptographically strong only where security requires it. Ordinary retry jitter generally needs unpredictability for distribution, not cryptographic secrecy. Avoid wasting expensive cryptographic primitives where not needed.

# Appendix U — Database Connection Pool Observability

The pool should expose:

```text
configured_max
currently_open
currently_idle
currently_checked_out
waiters
acquire_wait
acquire_timeout
```

The application should derive alerts from saturation patterns rather than from one isolated slow query.

A common chain:

```text
pool saturated
  ↓
request latency
  ↓
client retries
  ↓
more pool demand
  ↓
positive feedback
```

This is why Phase 2 telemetry must correlate database pool pressure with HTTP latency and retry behavior.

# Appendix V — Shutdown Acceptance Tests

Tests must prove:

```text
SIGTERM stops new work
in-flight safe work completes or is cancelled according to policy
worker stops claiming jobs
DB connections close
telemetry flush is attempted
flush has a hard deadline
process exits even when collector is unavailable
no duplicate job execution is created merely by shutdown
```

# Appendix W — Health Endpoint Acceptance Tests

Assert that production-mode health responses do not contain:

```text
DATABASE_URL
DATABASE_PASSWORD
JWT_SECRET
OTEL credential
MRA credential
payment secret
filesystem path
SQL statement
stack trace
private IP inventory
customer data
```

Assert that readiness correctly changes state when a critical dependency is unavailable.

# Appendix X — Secret Leak Regression Search

The security harness should search serialized artifacts for synthetic test secrets after:

```text
unit test run
integration test run
HTTP test run
telemetry export simulation
startup failure simulation
provider failure simulation
panic/error rendering test
```

This catches leaks that unit-level object inspection misses.

# Appendix Y — Operational Investigation Workflow

The standard investigation chain is:

```text
Alert
  ↓
request / trace / operation correlation
  ↓
service + version + artifact
  ↓
config fingerprint
  ↓
error family
  ↓
database/provider/worker state
  ↓
authoritative business state
  ↓
safe recovery action
  ↓
regression test
```

Never begin by modifying production business rows. First establish what actually happened and which state is authoritative.

# Appendix Z — Phase 2 Security Certification Checklist

```text
CONFIGURATION
[ ] all sources documented
[ ] precedence tested
[ ] schema validated
[ ] hard ceilings tested
[ ] cross-field validation tested
[ ] production restrictions tested
[ ] fingerprint deterministic

SECRETS
[ ] no source secrets
[ ] no artifact secrets
[ ] typed secret references
[ ] production provider enforced
[ ] rotation path tested
[ ] secret failures fail closed
[ ] redaction tests pass

ERRORS
[ ] error taxonomy implemented
[ ] public registry complete
[ ] Problem Details mapping complete
[ ] retryability explicit
[ ] unknown outcome explicit
[ ] 5xx leakage tests pass

LOGGING
[ ] structured events
[ ] request correlation
[ ] sensitive fields prohibited
[ ] log-size limits
[ ] production filter policy

TELEMETRY
[ ] metrics registry
[ ] bounded labels
[ ] trace propagation
[ ] exporter timeout
[ ] queue bounds
[ ] backpressure
[ ] telemetry outage test
[ ] self-monitoring

OPERATIONS
[ ] health/readiness safe
[ ] shutdown tested
[ ] runbooks approved
[ ] alert owners assigned
[ ] release evidence captured

CI
[ ] secret scanner
[ ] config tests
[ ] redaction tests
[ ] error tests
[ ] telemetry schema tests
[ ] static policy tests
[ ] fuzz targets
[ ] release-blocking status
```

# Appendix AA — Phase 2 Failure Matrix

| Failure | Detection | User effect | Internal response | Data authority |
|---|---|---|---|---|
| malformed config | startup validator | no traffic | fail startup | deployment config |
| missing DB secret | startup validator | service not ready | page operator | secret provider |
| OTLP outage | exporter metrics | normally none | bounded queue/shedding | PostgreSQL/audit |
| DB outage | DB errors/readiness | affected operations fail | alert/restore | PostgreSQL when available |
| secret rotation mismatch | secret validation | affected capability degraded | rotation incident | secret authority |
| provider timeout | integration telemetry | ambiguous operation possible | reconcile | domain + provider evidence |
| log sink outage | sink telemetry | none | local bounded fallback | audit/domain |
| error mapping bug | regression test/runtime counter | safe generic error | hotfix + regression | domain/database |
| metric cardinality explosion | registry/monitoring | telemetry degraded | shed/reject | business state unaffected |
| trace injection | parser/security tests | none | ignore as authority | auth/domain state |
| config drift | fingerprint comparison | depends | investigate/reconcile | deployment record |

# Appendix AB — Security Properties That Must Be Impossible to Accidentally Disable

The following should not be a runtime “feature flag” in production:

```text
redaction off
secret fallback on
TLS verification off for required production channels
unbounded telemetry queue
arbitrary metric names
production stack traces
raw request logging
anonymous privileged telemetry
```

A developer should not be able to change these with a normal `.env` edit. Where temporary exceptions exist, they require explicit deployment-level policy and review.

# Appendix AC — Configuration and Dependency Injection Anti-Patterns

Forbidden patterns:

```rust
fn handler() {
    let secret = std::env::var("PAYMENT_SECRET").unwrap();
}
```

```rust
static mut CONFIG: Option<AppConfig> = None;
```

```rust
let db_url = format!("postgres://{}:{}@{}", user, password, host);
tracing::debug!(db_url = %db_url, "database configured");
```

```rust
tracing::error!(?request, "failed");
```

The safer pattern is dependency injection of typed, narrow capabilities and explicit instrumentation fields.

# Appendix AD — Example Safe Error Flow

```text
Client sends FinalizeSale
        ↓
request parser succeeds
        ↓
authentication succeeds
        ↓
tenant context resolved
        ↓
authorization succeeds
        ↓
domain checks stock
        ↓
stock insufficient
        ↓
DomainError::InsufficientStock
        ↓
ApplicationError::Domain
        ↓
metric increment
        ↓
structured event
        ↓
HTTP 409 Problem Details
        ↓
request_id returned
```

The event/log can contain `operation=sale_finalize`, `error_family=invariant`, and a bounded reason class. The client does not receive SQL, stack trace or internal Rust type names.

# Appendix AE — Example Infrastructure Failure Flow

```text
Client sends FinalizeSale
        ↓
request/authz succeed
        ↓
DB pool acquisition times out
        ↓
PersistenceError::PoolTimeout
        ↓
ApplicationError::DependencyUnavailable
        ↓
DB timeout metric
        ↓
error log with operation + request_id
        ↓
HTTP 503
```

No database password is emitted. The operation is not represented as a business rejection. The caller can retry only according to the API's explicit retry semantics and idempotency contract.

# Appendix AF — Example Unknown Outcome Flow

```text
Payment request sent
        ↓
provider connection lost
        ↓
client received no response
        ↓
provider commitment unknown
        ↓
IntegrationError::UnknownOutcome
        ↓
payment state = ambiguous / pending according to payment contract
        ↓
reconciliation required
        ↓
telemetry: payment.unknown_outcome
```

This is deliberately more complicated than returning 500. A 500 only describes the HTTP request outcome; it does not prove the business effect did not happen.

# Appendix AG — Example Telemetry Blackout Flow

```text
collector unavailable
        ↓
export retries bounded
        ↓
queue increases
        ↓
pressure metric fires
        ↓
low-priority debug events shed
        ↓
critical diagnostic events prioritized
        ↓
business transaction continues
        ↓
audit remains durable
        ↓
collector recovery
        ↓
normal state
```

This is the intended resilience property. A monitoring outage must not automatically become a merchant outage.

# Appendix AH — Security Review Questions for Every New Field

Before adding any field to a log, trace, metric or error, ask:

```text
1. Is the field actually necessary?
2. Is it bounded?
3. Is it user-controlled?
4. Is it PII?
5. Is it authentication material?
6. Is it authorization-sensitive?
7. Is it a secret?
8. Is it an identifier with unbounded cardinality?
9. Which sink receives it?
10. What is its retention?
11. Who can query it?
12. Could its meaning change later?
13. Does it belong in audit instead?
```

A field that fails these questions should not be added casually.

# Appendix AI — Phase 2 Ownership Matrix

| Area | Primary owner | Review partners |
|---|---|---|
| config schema | platform | security, SRE |
| secret provider | platform/security | SRE |
| error taxonomy | application/platform | domain owners, API |
| Problem Details | API | security |
| logging | platform | security, SRE |
| metrics | platform/SRE | domain owners |
| tracing | platform/SRE | application owners |
| redaction | security | all owners |
| event registry | platform + domain owners | security |
| telemetry access | security/SRE | platform |
| alerting | SRE/operations | capability owners |
| runbooks | operations | service owners |

No single engineer should be able to silently redefine the whole platform's trust/telemetry model through an application PR.

# Appendix AJ — Change Classification

### Low-risk change

Internal event wording change that preserves fields and semantics.

### Medium-risk change

Additive metric or log field that is non-sensitive and bounded.

### High-risk change

Change to:

```text
secret provider
redaction
public error semantics
telemetry access
metric cardinality policy
trust-sensitive config
mandatory audit path
```

High-risk changes require security/architecture review and likely an ADR depending on whether an architectural invariant changes.

# Appendix AK — Exit Criteria for Phase 2 to Phase 3

Phase 3 may start when the project can demonstrate, on a clean checkout:

```text
1. Application starts with validated configuration.
2. Required secrets are resolved through the approved boundary.
3. Invalid configuration fails before readiness.
4. Structured logs are emitted without forbidden data.
5. Request IDs and trace context are correlated.
6. Errors map deterministically to the existing API contract.
7. Retryability is explicit.
8. Metrics are registered and bounded.
9. Telemetry export has timeout/backpressure behavior.
10. Telemetry failure cannot corrupt business persistence.
11. Security leakage tests pass.
12. Configuration/telemetry/error tests pass.
13. CI blocks the policy violations it can detect.
14. Runbooks exist for secret, config, telemetry and error incidents.
15. Release evidence identifies artifact, config schema and effective non-secret configuration.
```

Only after this gate passes should identity/session/MFA/device implementation become the dominant work. This preserves the project sequence and prevents later phases from creating duplicate runtime infrastructure.

