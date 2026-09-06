# Sitolo Observability Specification

**Document:** `observability_spec.md`  
**Phase:** 0 — Architecture / Contracts / ADR Freeze  
**File:** 11 of 16  
**Status:** Implementation-bound specification  
**System:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first, controlled African regionalization  
**Backend:** Rust / Tokio / Axum  
**Database:** PostgreSQL 18  
**Mobile:** Flutter / SQLite  
**Desktop:** Tauri  
**Telemetry model:** OpenTelemetry-compatible traces, metrics and logs with Prometheus-compatible metric consumption  
**Normative language:** MUST / MUST NOT / REQUIRED / SHOULD / SHOULD NOT / MAY are intentional requirements.

---

## 0. Executive Position

Sitolo must be observable as a business system, not merely as a web service.

The platform runs a chain of economically meaningful operations:

```text
identity
  -> organization / branch / device
  -> catalogue / pricing
  -> procurement / receiving
  -> inventory
  -> sale
  -> payment / cash
  -> reconciliation
  -> tax / EIS
  -> reporting
```

A production incident can therefore have at least four simultaneous dimensions:

```text
TECHNICAL FAILURE
    ↓
BUSINESS FAILURE
    ↓
SECURITY SIGNAL
    ↓
RECOVERY CONSEQUENCE
```

Example:

```text
PostgreSQL connection exhaustion
    ↓
Sale commit latency increases
    ↓
Clients retry
    ↓
Retry volume increases duplicate-risk pressure
    ↓
Idempotency path becomes hot
    ↓
POS throughput collapses
```

A useful observability system must allow an engineer to answer, quickly and defensibly:

1. Is the system healthy?
2. Which capability is failing?
3. Which tenants / branches / devices are affected?
4. Is the problem technical, business-rule, authorization, dependency, data, or abuse related?
5. Did a transaction commit before the failure?
6. Could an external side effect have occurred?
7. Is the platform safe to retry?
8. What evidence exists for reconciliation?
9. What is the blast radius?
10. What recovery action is safe?

The central design rule is:

> **Observability must make failures diagnosable without becoming a second source of business truth.**

Operational telemetry can report that a sale request succeeded. It MUST NOT become the authoritative source for whether that sale exists. PostgreSQL domain state, append-oriented audit evidence, and durable integration records remain authoritative according to the domain model.

This specification therefore defines five cooperating evidence layers:

```text
                 ┌─────────────────────────┐
                 │   BUSINESS AUTHORITIES   │
                 │ PostgreSQL domain state  │
                 │ audit / integration data │
                 └────────────┬────────────┘
                              │
                    correlation identifiers
                              │
       ┌──────────────────────┼──────────────────────┐
       │                      │                      │
       ▼                      ▼                      ▼
    METRICS                TRACES                  LOGS
       │                      │                      │
       └──────────────┬───────┴───────────┬──────────┘
                      ▼                   ▼
                 DASHBOARDS           ALERTING
                      │                   │
                      └─────────┬─────────┘
                                ▼
                        INCIDENT RESPONSE
```

The architecture must support this relationship without forcing every event into every signal.

---

# 1. Scope

## 1.1 In scope

This document governs observability for:

- Rust API and application services.
- PostgreSQL and transaction behavior visible to the application.
- Background workers.
- Transactional outbox processing.
- Offline synchronization.
- Payment integrations.
- MRA EIS integration.
- Authentication and authorization paths.
- Tenant and branch isolation signals.
- Inventory and sales critical paths.
- Cash and reconciliation workflows.
- Reporting/export jobs.
- Object storage interactions.
- Redis, if enabled, as a non-authoritative dependency.
- Flutter client operational telemetry.
- Tauri desktop operational telemetry.
- Build and deployment telemetry relevant to runtime correctness.
- Synthetic probes and release verification.
- Audit evidence and incident correlation.

## 1.2 Out of scope

This document does not redefine:

- business-domain invariants;
- API resource contracts;
- payment provider protocol details;
- MRA EIS protocol details;
- database schema ownership;
- identity protocol semantics;
- legal retention periods that must be established externally;
- organizational incident command policy beyond technical observability requirements.

Those contracts remain owned by their respective specifications.

## 1.3 Relationship to existing specifications

Observability MUST implement the contracts established by:

- `security_implementation_spec.md`
- `domain_model.md`
- `database_design.md`
- `api_contract.md`
- `auth_authorization_spec.md`
- `sync_protocol.md`
- `payment_integration_spec.md`
- `mra_eis_integration_spec.md`
- `testing_strategy.md`
- `threat_model.md`

The observability system may expose additional diagnostic attributes, but it MUST NOT redefine authority boundaries established by these documents.

---

# 2. Observability Principles

## 2.1 Three questions, three evidence classes

Every production investigation should distinguish:

```text
WHAT HAPPENED?
    -> logs / traces

HOW OFTEN / HOW MUCH?
    -> metrics

WHAT IS ACTUALLY TRUE?
    -> authoritative domain state / audit / integration state
```

Confusing these classes creates dangerous operational decisions.

For example:

```text
metric: sale_requests_total = 10,000
```

does not prove:

```text
10,000 sales committed
```

A request may have been rejected, timed out, retried, or failed before commit.

## 2.2 Observability is not logging everything

The objective is not maximum telemetry volume.

The objective is maximum diagnostic information per unit of telemetry cost and risk.

The system SHOULD prefer:

- structured events;
- stable semantic fields;
- low-cardinality dimensions;
- deterministic correlation;
- sampling for high-volume traces;
- targeted retention;
- aggregation before export where safe.

The system MUST avoid indiscriminate capture of:

- passwords;
- access tokens;
- refresh tokens;
- payment secrets;
- provider credentials;
- terminal secret material;
- MFA secrets;
- private keys;
- complete payment payloads;
- complete customer PII where unnecessary;
- complete sale bodies at high volume.

## 2.3 Observability must survive partial failure

The platform must remain diagnosable when one telemetry component fails.

Examples:

```text
OTLP collector down
    -> business request may continue
    -> critical audit still commits
    -> local process logs remain available
    -> telemetry-loss metric / health signal fires
```

The telemetry backend MUST NOT become a dependency of financial correctness.

Conversely:

```text
PostgreSQL unavailable
    -> financial mutation fails closed
    -> safe degraded response
    -> process emits failure telemetry if possible
```

## 2.4 Audit is not telemetry

Audit evidence answers:

> Who performed what sensitive business action, against which resource, under which authorization context, and what outcome resulted?

Operational telemetry answers:

> What was the runtime behavior of the system?

They overlap in correlation, not in authority.

Audit records MUST be persisted in the authoritative system or an intentionally durable evidence store. They MUST NOT be represented solely as application logs.

## 2.5 Security telemetry must support detection without creating a PII lake

Security signals should capture:

- failure count;
- actor class;
- tenant scope where safe;
- resource type;
- reason code;
- source category;
- device category;
- network/risk metadata appropriate for detection;
- correlation IDs.

Security telemetry SHOULD avoid raw personal identifiers where a pseudonymous or internal identifier is sufficient.

---

# 3. Signal Architecture

## 3.1 Three primary signals

Sitolo will use:

1. Metrics.
2. Traces.
3. Structured logs.

These will be correlated through common resource and context attributes.

OpenTelemetry provides common semantic conventions across traces, metrics, logs, profiles and resources; the implementation SHOULD use the applicable current conventions rather than inventing equivalent names. The current OpenTelemetry semantic conventions registry is versioned and includes HTTP, database, messaging, RPC, resource, trace, metric and log conventions. [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/) citeturn997793search12turn997793search5

## 3.2 Correlation model

Required identifiers include, where applicable:

```text
trace_id
span_id
request_id
operation_id
command_id
idempotency_key_hash
outbox_id
job_id
sale_id
payment_intent_id
provider_transaction_id
terminal_id
sync_batch_id
sync_checkpoint
exception_id
```

Not every signal must carry every identifier.

The rule is:

```text
high-volume signal -> low-cardinality identifiers
high-value diagnostic record -> precise correlation IDs
authoritative business record -> complete required identifiers
```

## 3.3 W3C trace propagation

For HTTP and compatible distributed boundaries, the platform SHOULD use W3C Trace Context propagation using `traceparent` and, where appropriate, `tracestate`. The standard defines these fields specifically to preserve trace context across intermediaries and independently instrumented components. [W3C Trace Context](https://www.w3.org/TR/trace-context/) citeturn997793search11

Trace context MUST NOT be treated as authentication or authorization evidence.

A caller can forge a trace ID. It is a diagnostic correlation value, not a trust assertion.

## 3.4 Resource identity

Every telemetry-producing service/process SHOULD identify its deployment resource with stable attributes such as:

```text
service.name
service.version
service.instance.id
service.namespace
cloud.provider
cloud.region
deployment.environment.name
host.name / container identity where appropriate
```

Identifiers must be stable enough for operational grouping but not leak sensitive infrastructure details to untrusted users.

---

# 4. Telemetry Trust Boundary

The telemetry path is itself an attack surface.

```text
Application
   |
   | OTLP / local exporter
   v
Telemetry collector
   |
   +--> metrics backend
   +--> trace backend
   +--> log backend
```

Threats include:

- telemetry injection;
- forged correlation IDs;
- PII leakage;
- token leakage;
- log-volume denial of service;
- cardinality explosion;
- collector compromise;
- unbounded local buffering;
- sensitive data retention beyond policy;
- tenant data leakage through dashboards;
- cross-environment contamination.

Therefore:

1. Telemetry endpoints MUST be authenticated when remotely reachable.
2. Telemetry exporters MUST use TLS for remote transport.
3. Collector access MUST be network-restricted.
4. Dashboard authorization MUST follow least privilege.
5. Tenant-sensitive dashboards MUST enforce tenant scope.
6. Untrusted input MUST NOT directly become unrestricted metric label values.
7. Log bodies MUST be size-bounded.
8. Collector queues MUST be bounded.
9. Telemetry failure MUST not block critical domain transactions unless explicitly required for an evidence workflow.

---

# 5. Metrics Specification

## 5.1 Metric design rule

Metrics answer population-level questions efficiently.

Use metrics for:

- rates;
- counts;
- durations;
- queue depth;
- resource utilization;
- error ratios;
- saturation;
- state counts;
- availability objectives;
- business process health at aggregate level.

Prometheus recommends stable names with clear units and warns strongly against high-cardinality labels. Every unique label combination creates a distinct time series. Prometheus guidance recommends keeping cardinality low and specifically warns against labels such as user IDs or email addresses. [Prometheus metric and label naming](https://prometheus.io/docs/practices/naming/) [Prometheus instrumentation guidance](https://prometheus.io/docs/practices/instrumentation/) citeturn997793search0turn997793search15

## 5.2 Metric naming

Where Prometheus exposition is used, the metric design SHOULD follow a consistent application namespace.

Example:

```text
sitolo_http_requests_total
sitolo_http_request_duration_seconds
sitolo_db_pool_connections
sitolo_sales_committed_total
```

Use base units such as:

- seconds;
- bytes;
- ratio;
- count.

Do not mix seconds and milliseconds in similarly named measurements.

## 5.3 Approved label dimensions

Default safe dimensions include:

```text
service
route_template
method
status_class
result
operation
integration
region
environment
job_type
queue_class
error_family
```

Potentially bounded domain dimensions include:

```text
payment_provider
tax_provider
sync_result
device_platform
app_major_version
```

The following MUST NOT be used as metric labels:

```text
user_id
email
phone_number
sale_id
payment_intent_id
command_id
trace_id
request_id
free-form_error_message
provider_transaction_id
terminal_serial_number
```

These may exist in traces/logs/audit records where justified.

## 5.4 HTTP metrics

Required baseline metrics:

```text
sitolo_http_requests_total
sitolo_http_request_duration_seconds
sitolo_http_request_body_bytes
sitolo_http_response_body_bytes
sitolo_http_in_flight_requests
```

Recommended dimensions:

```text
method
route_template
status_class
```

Avoid raw request URL labels.

OpenTelemetry's current HTTP semantic conventions define conventions for HTTP spans, metrics and logs; instrumentations may be in a transition between older experimental and newer stable conventions, so Sitolo MUST pin and document the convention version used by each major instrumentation path rather than silently changing telemetry semantics. [OpenTelemetry HTTP Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/http/) citeturn997793search1

## 5.5 Application error metrics

Required:

```text
sitolo_errors_total
```

Dimensions:

```text
service
operation
error_family
recoverability
```

Example `error_family` values:

```text
validation
authorization
not_found
conflict
invariant
concurrency
idempotency
external_unavailable
external_rejected
tax_rejected
internal
```

Do not put the raw exception message into labels.

## 5.6 Authentication metrics

Required:

```text
sitolo_authentication_attempts_total
sitolo_authentication_failures_total
sitolo_mfa_attempts_total
sitolo_mfa_failures_total
sitolo_session_refresh_total
sitolo_session_revocations_total
sitolo_device_revocations_total
```

Dimensions may include:

```text
method
result
reason_class
client_platform
```

Reason classes must be bounded:

```text
invalid_credentials
expired_session
invalid_otp
revoked_device
risk_block
rate_limited
unknown
```

## 5.7 Authorization metrics

Required:

```text
sitolo_authorization_denials_total
sitolo_policy_evaluations_total
sitolo_policy_evaluation_duration_seconds
```

Dimensions:

```text
operation
resource_type
result
reason_class
```

The metric MUST NOT expose the full resource ID or user identifier.

## 5.8 Tenant isolation metrics

High-value security metrics:

```text
sitolo_scope_resolution_failures_total
sitolo_cross_scope_access_denials_total
sitolo_rls_denials_total
sitolo_tenant_context_missing_total
```

An unexpected increase is a security alert candidate.

These metrics must not be used to infer that a tenant was actually compromised. They indicate policy events requiring investigation.

## 5.9 Database metrics

Application-observable database metrics:

```text
sitolo_db_pool_size
sitolo_db_pool_acquired
sitolo_db_pool_idle
sitolo_db_pool_wait_seconds
sitolo_db_query_duration_seconds
sitolo_db_query_errors_total
sitolo_db_transactions_total
sitolo_db_transaction_duration_seconds
sitolo_db_deadlocks_total
sitolo_db_lock_wait_seconds
sitolo_db_timeouts_total
```

Query labels MUST identify safe logical operation families, not full SQL text.

Example:

```text
operation="sale_finalize"
```

not:

```text
query="SELECT ... tenant_id='abc' ..."
```

OpenTelemetry's database semantic conventions define conventions for database spans, metrics and logs. Sitolo SHOULD use them where the implementation supports them, while keeping business operation names under explicit application control. [OpenTelemetry Database Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/db/) citeturn997793search3

## 5.10 Transaction metrics

Financially important operations SHOULD expose:

```text
sitolo_domain_transaction_duration_seconds
sitolo_domain_transaction_failures_total
sitolo_domain_transaction_commits_total
```

Dimensions:

```text
operation
result
```

Examples:

```text
operation="sale_finalize"
operation="inventory_receive"
operation="refund_create"
operation="cash_close"
```

## 5.11 Sales metrics

Required:

```text
sitolo_sales_attempted_total
sitolo_sales_committed_total
sitolo_sales_rejected_total
sitolo_sales_duration_seconds
```

Useful bounded dimensions:

```text
channel
result
offline_online_mode
payment_mode
```

DO NOT expose monetary value as a high-cardinality metric label.

Aggregate financial reporting belongs in domain/reporting data, not in arbitrary telemetry labels.

## 5.12 Inventory metrics

Required:

```text
sitolo_inventory_postings_total
sitolo_inventory_conflicts_total
sitolo_inventory_insufficient_stock_total
sitolo_inventory_adjustments_total
```

Dimensions:

```text
operation
result
reason_class
```

Use authoritative database state for exact stock balances.

## 5.13 Payment metrics

Required:

```text
sitolo_payment_intents_total
sitolo_payment_attempts_total
sitolo_payment_provider_requests_total
sitolo_payment_provider_failures_total
sitolo_payment_webhook_events_total
sitolo_payment_webhook_verification_failures_total
sitolo_payment_duplicate_events_total
sitolo_payment_reconciliation_exceptions_total
sitolo_payment_unknown_outcome_total
```

Dimensions:

```text
provider
operation
result
failure_class
```

Never place account numbers or provider transaction IDs into labels.

## 5.14 EIS metrics

Required:

```text
sitolo_eis_submissions_total
sitolo_eis_submission_duration_seconds
sitolo_eis_rejections_total
sitolo_eis_retryable_failures_total
sitolo_eis_unknown_outcome_total
sitolo_eis_configuration_refresh_total
sitolo_eis_offline_submission_total
sitolo_eis_tax_exception_total
sitolo_eis_queue_depth
```

Dimensions:

```text
operation
result
failure_class
environment
```

Exact terminal IDs belong in logs/audit/evidence, not broad metrics.

## 5.15 Sync metrics

Required:

```text
sitolo_sync_push_batches_total
sitolo_sync_commands_received_total
sitolo_sync_commands_committed_total
sitolo_sync_commands_rejected_total
sitolo_sync_conflicts_total
sitolo_sync_retryable_total
sitolo_sync_queue_depth
sitolo_sync_oldest_pending_age_seconds
sitolo_sync_checkpoint_advances_total
sitolo_sync_checkpoint_regressions_total
sitolo_sync_revoked_device_attempts_total
```

Dimensions:

```text
result
command_type
conflict_class
platform
```

Do not label by command ID.

## 5.16 Worker metrics

Required:

```text
sitolo_worker_jobs_claimed_total
sitolo_worker_jobs_completed_total
sitolo_worker_jobs_failed_total
sitolo_worker_job_duration_seconds
sitolo_worker_job_queue_depth
sitolo_worker_job_oldest_age_seconds
sitolo_worker_lease_expirations_total
sitolo_worker_dead_letter_total
```

Dimensions:

```text
job_type
queue_class
result
```

## 5.17 Cache / Redis metrics

If Redis is deployed:

```text
sitolo_cache_hits_total
sitolo_cache_misses_total
sitolo_cache_errors_total
sitolo_cache_operation_duration_seconds
```

Cache state MUST NOT be treated as authoritative.

## 5.18 Object storage metrics

```text
sitolo_object_store_requests_total
sitolo_object_store_failures_total
sitolo_object_store_duration_seconds
sitolo_object_store_bytes_total
```

## 5.19 Report/export metrics

Reports can create resource-exhaustion risk.

Required:

```text
sitolo_report_jobs_total
sitolo_report_job_duration_seconds
sitolo_report_job_failures_total
sitolo_export_bytes_total
sitolo_export_queue_depth
sitolo_export_oldest_age_seconds
```

Large reports MUST be asynchronous.

## 5.20 Rate-limit metrics

```text
sitolo_rate_limit_rejections_total
sitolo_rate_limit_buckets_exhausted_total
sitolo_abuse_blocks_total
```

## 5.21 Telemetry pipeline metrics

Telemetry itself needs observability:

```text
sitolo_telemetry_export_failures_total
sitolo_telemetry_export_queue_depth
sitolo_telemetry_export_dropped_total
sitolo_telemetry_log_redactions_total
sitolo_telemetry_oversize_records_total
```

The application MUST be able to distinguish:

```text
business telemetry unavailable
```

from:

```text
business dependency unavailable
```

---

# 6. Tracing Specification

## 6.1 Why traces matter

Traces reconstruct a single execution path across:

```text
mobile request
  -> API gateway
  -> authn
  -> authz
  -> domain operation
  -> PostgreSQL
  -> outbox
  -> worker
  -> provider
  -> reconciliation
```

A trace is especially valuable when latency and failure emerge from a chain of bounded operations.

## 6.2 Sampling strategy

Default trace sampling SHOULD be adaptive.

Suggested policy:

```text
successful high-volume requests -> sampled aggressively
slow requests                 -> retained
5xx requests                  -> retained
security anomalies            -> retained
financial exceptions          -> retained
payment unknown outcomes      -> retained
MRA rejected submissions      -> retained
sync conflicts                -> retained
```

The exact sampling rate is an operational parameter, not a business rule.

Critical evidence MUST NOT depend solely on sampled traces.

## 6.3 Span naming

Span names SHOULD describe the logical operation, not the raw URL.

Good:

```text
sale.finalize
inventory.post
payment.intent.create
payment.provider.verify
eis.sale.submit
sync.command.apply
report.generate
```

Bad:

```text
POST /api/v1/tenants/ab123/sales/998/complete
```

## 6.4 Required span attributes

Application spans should include bounded attributes such as:

```text
sitolo.operation
sitolo.component
sitolo.tenant_scope_class
sitolo.branch_scope_class
sitolo.command_type
sitolo.result
sitolo.error.family
```

Use direct resource identifiers only for diagnostic spans when the trace store is appropriately access-controlled and retention is justified.

## 6.5 HTTP server spans

Capture:

- HTTP method;
- route template;
- status code;
- status class;
- request/response size where safe;
- server duration;
- error status;
- trace context.

Do not record:

- authorization headers;
- cookies containing credentials;
- raw request bodies containing sensitive financial or personal data.

## 6.6 Database spans

Database spans should identify:

```text
db.system
server.address where appropriate
db.operation
db.namespace where safe
```

Do not attach:

- literal SQL with bound secrets;
- full query parameter values;
- tenant-sensitive row contents.

SQL statement capture SHOULD be restricted or redacted where its presence creates material privacy or security risk.

## 6.7 Worker traces

A worker should continue correlation from the originating event where possible.

Example:

```text
Trace A: sale.finalize
   |
   +--> outbox row
          |
          +--> worker Trace B
                 |
                 +--> eis.sale.submit
```

The worker trace MUST contain a durable correlation to the domain event/outbox ID even if trace sampling causes the original trace to be absent from the backend.

## 6.8 External integration spans

Each external call SHOULD create a client span.

Example:

```text
eis.sale.submit
   |
   +--> HTTP client span
   |
   +--> response classification
```

Record:

```text
provider
operation
result
HTTP status if available
retry classification
latency
```

Do not record credentials, signatures, or entire external response payloads.

## 6.9 Retry tracing

Retries MUST be distinguishable from independent operations.

Use:

```text
attempt.number
retry.reason
retry.backoff_ms
```

as bounded span attributes.

A retry must not create the illusion of a new business intent.

---

# 7. Structured Logging Specification

## 7.1 JSON structured logs

Production logs MUST be structured.

Recommended baseline fields:

```json
{
  "timestamp": "...",
  "severity": "INFO",
  "service": "sitolo-api",
  "service_version": "...",
  "environment": "production",
  "trace_id": "...",
  "span_id": "...",
  "request_id": "...",
  "operation": "sale.finalize",
  "event": "sale_commit_completed",
  "result": "success"
}
```

The exact field names MAY follow OpenTelemetry log/resource conventions as the implementation evolves. The current OpenTelemetry log data model is stable and provides a common representation for log records; Sitolo SHOULD preserve semantic interoperability while retaining explicit application fields for business diagnostics. [OpenTelemetry Logs Data Model](https://opentelemetry.io/docs/specs/otel/logs/data-model/) citeturn997793search13

## 7.2 Severity

Minimum levels:

```text
TRACE
DEBUG
INFO
WARN
ERROR
FATAL
```

Production defaults:

```text
INFO for lifecycle/business operation summaries
WARN for degraded but controlled behavior
ERROR for failed operations requiring investigation
FATAL for process-level integrity failure
```

DEBUG/TRACE SHOULD be dynamically controllable under strict authorization and bounded duration.

## 7.3 Event names

Use stable event names:

```text
request_started
request_completed
authorization_denied
sale_commit_completed
sale_commit_failed
inventory_posted
payment_attempt_created
payment_unknown_outcome
payment_webhook_rejected
eis_submission_accepted
eis_submission_rejected
sync_command_rejected
sync_conflict_detected
worker_job_failed
backup_validation_failed
```

Avoid free-form prose as the primary diagnostic field.

## 7.4 Safe identifiers

Permitted examples:

```text
sale_id
job_id
outbox_id
command_id
trace_id
request_id
exception_id
```

These identifiers SHOULD be treated as sensitive operational data and access-controlled accordingly.

## 7.5 Sensitive fields

The logger MUST redact or reject fields matching classes such as:

```text
password
secret
token
access_token
refresh_token
client_secret
api_key
private_key
otp
mfa_secret
signature
authorization
cookie
provider_credential
terminal_secret
```

The exact implementation SHOULD use structural redaction rather than regex-only sanitization.

## 7.6 Error logging

An error log MUST include:

```text
stable error code
error family
operation
retryability
correlation identifiers
safe causal summary
```

It MUST NOT expose:

- SQL credentials;
- connection strings;
- cryptographic keys;
- raw authorization headers;
- secret-bearing provider responses.

## 7.7 Exception chains

Internal logs MAY retain error chains for diagnosis.

The public API MUST still return stable domain errors as defined by the API contract.

Example:

```text
Public:
EXTERNAL_DEPENDENCY_UNAVAILABLE

Internal:
provider timeout
  <- reqwest timeout
  <- socket error
```

---

# 8. Business Observability

## 8.1 Principle

Traditional infrastructure monitoring answers whether CPU, memory and HTTP are functioning.

Sitolo additionally needs business health telemetry.

Core business process:

```text
PROCURE
  ↓
RECEIVE
  ↓
STOCK
  ↓
PRICE
  ↓
SELL
  ↓
COLLECT
  ↓
RECONCILE
  ↓
REPORT
```

The observability design should expose failures at each transition.

## 8.2 Sale lifecycle telemetry

Required states:

```text
DRAFT
AUTHORIZED
COMMITTED
PAYMENT_PENDING
PAYMENT_CONFIRMED
TAX_PENDING
TAX_ACCEPTED
TAX_EXCEPTION
VOIDED / CORRECTED where domain rules permit
```

The exact states remain domain-owned.

Telemetry should report transitions, not invent parallel state.

## 8.3 Inventory health

Track:

- posting failure rate;
- insufficient-stock rejection rate;
- concurrency-conflict rate;
- adjustment volume;
- receiving discrepancy rate;
- ledger repair alerts;
- negative-stock exception count where policy permits detection.

Exact balances come from authoritative inventory state.

## 8.4 Payment health

A healthy payment system is not merely:

```text
provider API reachable
```

It is:

```text
intent creation healthy
+ provider initiation healthy
+ callback verification healthy
+ reconciliation healthy
+ unknown outcomes bounded
+ duplicate events bounded
```

## 8.5 Tax/EIS health

Required aggregate views:

```text
online submission success
retryable failures
permanent rejections
unknown outcomes
pending age
queue depth
configuration staleness
offline backlog
```

An EIS outage should not be mistaken for a sales outage if local sales can safely continue under policy.

## 8.6 Sync health

Primary dimensions:

```text
pending commands
oldest pending age
rejection rate
conflict rate
checkpoint movement
retry rate
revoked-device attempts
schema incompatibility
```

A growing sync backlog can indicate:

- server degradation;
- network degradation;
- client version skew;
- schema incompatibility;
- authorization changes;
- fraud/tampering;
- domain conflicts.

## 8.7 Tenant-noisy-neighbor health

Sitolo is multi-tenant.

The system SHOULD expose aggregate per-tenant operational budgets without turning unbounded tenant IDs into public metric cardinality.

Recommended pattern:

```text
metrics:
  tenant_tier
  region
  operation

logs/traces/support views:
  exact tenant ID
```

For internal SRE dashboards, bounded tenant cohorts MAY be used where cardinality is controlled.

---

# 9. Audit and Evidence Correlation

## 9.1 Evidence chain

A sensitive action should be reconstructable through:

```text
actor
  ↓
session/device
  ↓
request
  ↓
operation
  ↓
authorization decision
  ↓
domain transaction
  ↓
audit event
  ↓
outbox / integration effect
  ↓
external reference
```

## 9.2 Required cross-domain correlation

Examples:

### Sale

```text
sale_id
request_id
command_id
trace_id
actor_id
branch_id
register_id
payment_intent_id if applicable
eis_submission_id if applicable
```

### Payment

```text
payment_intent_id
attempt_id
provider_transaction_id
webhook_event_id if applicable
trace_id
reconciliation_exception_id
```

### EIS

```text
sale_id
terminal_id
configuration_version
submission_id
external_reference
trace_id
```

### Sync

```text
device_id
command_id
batch_id
checkpoint
trace_id
server_result
```

## 9.3 Correlation does not imply authority

A trace may show:

```text
payment.provider.verify -> 200
```

but only the payment state machine and reconciliation rules can decide whether that payment is economically accepted.

---

# 10. Dashboard Architecture

Dashboards should be operational decision surfaces, not metric museums.

## 10.1 Executive service health dashboard

Show:

```text
availability
request rate
error rate
p95/p99 latency
DB health
worker health
sync backlog
payment health
EIS health
critical alerts
```

No raw PII.

## 10.2 API dashboard

Panels:

- request throughput;
- error percentage;
- latency distribution;
- saturation;
- top failing route templates;
- top error families;
- auth failures;
- authorization denials;
- rate-limit events.

## 10.3 PostgreSQL dashboard

Panels:

- active connections;
- pool wait;
- transaction latency;
- deadlocks;
- lock waits;
- query error rate;
- transaction age;
- connection exhaustion;
- replication lag if replicas exist;
- autovacuum health;
- storage growth.

## 10.4 Worker dashboard

Panels:

- queue depth;
- oldest job age;
- success/failure;
- retries;
- lease expiration;
- dead-letter growth;
- worker utilization.

## 10.5 Payment dashboard

Panels:

- payment attempts;
- provider latency;
- provider error rate;
- webhook verification failures;
- duplicate webhooks;
- unknown outcomes;
- reconciliation exceptions;
- pending-age distribution.

## 10.6 EIS dashboard

Panels:

- submission success;
- submission latency;
- reject rate;
- retryable errors;
- tax exceptions;
- pending backlog;
- oldest tax submission age;
- offline queue depth;
- configuration refresh failures.

## 10.7 Sync dashboard

Panels:

- connected devices;
- push rate;
- command acceptance;
- rejection rate;
- conflicts;
- oldest pending command;
- checkpoint advancement;
- schema-version rejection;
- revoked-device attempts.

## 10.8 Security dashboard

Panels:

- authentication failures;
- MFA failures;
- authorization denial spikes;
- cross-scope denial attempts;
- rate-limit blocks;
- webhook signature failures;
- suspicious device events;
- secret-scan / deployment security events;
- telemetry redaction failures.

## 10.9 Tenant support dashboard

Support views MAY expose exact tenant identifiers, but access MUST require appropriate support authorization and be audited.

Support MUST NOT gain unrestricted access simply because a dashboard contains operational data.

---

# 11. Service Level Objectives

SLOs must be tied to user-visible capabilities rather than infrastructure vanity targets.

## 11.1 API availability

Example initial target:

```text
Critical POS API monthly availability >= 99.9%
```

This is an engineering target, not a contractual SLA until commercially adopted.

## 11.2 Sale finalization latency

Target metrics should be evaluated by environment and device/network conditions.

Example:

```text
p95 <= 500 ms under nominal server/network conditions
p99 <= 1.5 s under nominal conditions
```

These are performance targets to validate, not facts about the final product.

## 11.3 EIS submission freshness

Measure:

```text
sale commit -> EIS accepted
```

Do not define “success” purely as request initiation.

## 11.4 Sync freshness

Measure:

```text
command local commit -> server acknowledgement
server event -> client availability
```

Offline operation should be excluded from online availability calculations where appropriate, but offline backlog age MUST remain observable.

## 11.5 Error budget

Each critical capability SHOULD have an error budget.

Examples:

```text
API availability
payment reconciliation freshness
EIS backlog age
sync backlog age
report completion latency
```

Error-budget exhaustion SHOULD trigger controlled engineering action rather than a cosmetic dashboard state.

---

# 12. Alerting Specification

## 12.1 Alert philosophy

Every alert must answer:

```text
What is wrong?
Why does it matter?
Who responds?
What should they inspect?
What action is safe?
```

An alert without an owner or response path is noise.

## 12.2 Severity

Recommended operational severity:

```text
SEV-1  immediate integrity / isolation / widespread transaction impact
SEV-2  major capability degradation
SEV-3  material localized degradation
SEV-4  informational / non-urgent
```

## 12.3 Critical alerts

Examples:

```text
PostgreSQL unavailable
cross-tenant authorization anomaly
mass authorization denials
payment unknown-outcome spike
payment webhook verification failures spike
EIS backlog beyond policy threshold with business impact
sync protocol incompatibility spike
critical backup failure
telemetry redaction failure
secret exposure detection
```

## 12.4 Warning alerts

Examples:

```text
latency trend increasing
connection pool saturation
worker queue aging
EIS retry growth
sync backlog growth
report queue growth
object storage error trend
```

## 12.5 Alert anti-patterns

DO NOT alert on:

```text
every 5xx request
any single failed login
any single EIS rejection
any single payment timeout
```

Use rates, ratios, burn rates, persistence windows, or aggregation.

---

# 13. Burn-Rate Alerts

For SLO-backed capabilities, alerts SHOULD use burn-rate concepts instead of only absolute thresholds.

Conceptual example:

```text
fast burn:
  severe error budget consumption over short window

slow burn:
  sustained degradation over longer window
```

This prevents alerting on harmless transient noise while still detecting sustained damage.

Exact windows and thresholds are deployment parameters.

---

# 14. Distributed Context Propagation

## 14.1 Context fields

A trusted runtime context may contain:

```text
trace context
request ID
actor ID
session ID
device ID
tenant ID
branch ID
operation
```

Security-sensitive fields must be derived from trusted authentication/authorization state, not accepted as caller-provided metadata.

## 14.2 Trace vs security context

These are distinct:

```text
trace context = diagnostic propagation
security context = authorization authority
```

Never use:

```text
traceparent
```

to identify a user or authorize a request.

## 14.3 Asynchronous propagation

For queues/outbox/workers, propagate a bounded correlation envelope.

Recommended:

```text
traceparent if applicable
causation_id
correlation_id
aggregate_id
event_type
schema_version
```

Do not copy entire request headers into jobs.

---

# 15. Sampling and Retention

## 15.1 General policy

Telemetry retention must be intentionally classified.

Suggested classes:

```text
metrics: longer operational horizon
traces: moderate horizon with sampled retention
logs: moderate horizon with security/event exceptions
audit: domain/regulatory retention policy
integration evidence: domain-defined retention
```

Exact durations MUST be set by deployment and legal requirements.

## 15.2 High-value trace retention

Retain or tail-sample traces associated with:

- errors;
- high latency;
- payment exceptions;
- EIS rejection;
- sync conflict;
- authorization anomaly;
- financial integrity incident;
- support investigation.

## 15.3 PII retention

PII minimization must occur before ingestion wherever feasible.

Do not solve excessive PII exposure with retention policy alone.

The preferred sequence is:

```text
DO NOT COLLECT
   ↓
MINIMIZE
   ↓
REDACT
   ↓
RESTRICT ACCESS
   ↓
RETAIN ONLY AS REQUIRED
   ↓
DELETE / ARCHIVE PER POLICY
```

---

# 16. Cardinality Governance

## 16.1 Principle

Metric cardinality is a finite operational resource.

Prometheus explicitly warns that label combinations create time series and that high-cardinality labels can cause major resource growth. It advises avoiding unbounded labels and generally keeping most metrics low-cardinality. citeturn997793search0turn997793search9

## 16.2 Cardinality classes

### Safe

```text
method
status_class
result
service
operation
```

### Controlled

```text
provider
job_type
command_type
app_major_version
region
```

### High

```text
tenant_id
user_id
branch_id
device_id
```

### Unbounded

```text
email
phone
sale_id
trace_id
request_id
free_text
URL
SQL
provider_transaction_id
```

High and unbounded dimensions MUST NOT be used in standard Prometheus metrics.

## 16.3 Pre-deployment cardinality review

Every new metric PR must document:

```text
metric name
unit
type
labels
expected cardinality
maximum cardinality
aggregation meaning
retention impact
```

## 16.4 Runtime cardinality protection

The telemetry layer SHOULD provide:

- label allowlists;
- bounded value normalization;
- dropped-label counters;
- exporter backpressure;
- event size limits.

---

# 17. Privacy and Data Classification

## 17.1 Telemetry data classes

Classify telemetry as:

```text
PUBLIC_OPERATIONAL
INTERNAL_OPERATIONAL
SECURITY_SENSITIVE
PERSONAL_DATA
FINANCIAL_SENSITIVE
CREDENTIAL_SECRET
```

Secrets MUST never be emitted.

## 17.2 Tenant isolation in telemetry

Telemetry storage is a separate data system but can still leak tenant information.

Controls:

- dashboard authorization;
- tenant-scoped support views;
- access logging;
- environment segregation;
- retention policy;
- export restrictions;
- no customer-visible raw trace store.

## 17.3 Customer-visible diagnostics

Merchant-facing diagnostics should expose safe summaries such as:

```text
Payment pending
Tax submission retrying
Sync requires connection
Report queued
```

Do not expose internal stack traces or security signals.

---

# 18. Error Taxonomy and Telemetry Mapping

The domain error taxonomy must map consistently into telemetry.

| Domain / operational class | Metric family | Log event | Trace status | Retryable? |
|---|---|---|---|---|
| Validation | validation error | validation_failed | error/unset by policy | No |
| Authorization denied | authz denial | authorization_denied | error | No |
| Not found | not-found | resource_not_found | unset/error by contract | No |
| State conflict | conflict | state_conflict | error | Usually no |
| Invariant violation | invariant | invariant_violation | error | No |
| Concurrency conflict | concurrency | concurrency_conflict | error | Controlled |
| Idempotency conflict | idempotency | idempotency_conflict | error | No |
| External unavailable | dependency | dependency_unavailable | error | Yes |
| External rejected | dependency | dependency_rejected | error | Usually no |
| Tax rejected | EIS | tax_submission_rejected | error | No / exception |
| Internal failure | internal | internal_error | error | Unknown |

The mapping table is part of the contract.

---

# 19. Database Observability

## 19.1 Application-level requirements

The database layer must expose enough information to diagnose:

- connection exhaustion;
- slow queries;
- lock contention;
- deadlocks;
- long transactions;
- rollback storms;
- pool starvation;
- failed migrations;
- replication lag if used;
- storage pressure.

## 19.2 Query naming

Each important repository operation SHOULD expose a logical query name:

```text
sale.finalize.load
sale.finalize.commit
inventory.post
payment.get_intent
sync.apply_command
```

This enables query-family metrics without raw SQL cardinality.

## 19.3 Transaction observability

Critical transaction spans SHOULD include:

```text
operation
transaction duration
commit / rollback
retry classification
lock wait contribution
```

## 19.4 Lock monitoring

The operational dashboard must expose lock contention.

Alert when:

```text
lock wait duration
```

or:

```text
deadlock rate
```

exceeds tested operating thresholds.

## 19.5 Connection pool monitoring

Track:

```text
configured pool size
in-use
idle
waiters
wait duration
timeouts
```

Connection exhaustion is both a performance and availability concern.

---

# 20. Runtime Resource Observability

Required host/process/container signals include:

```text
CPU
memory
RSS
file descriptors
network throughput
network errors
disk usage
disk I/O
process restarts
thread/task pressure
```

Rust/Tokio-specific operational signals SHOULD include:

- runtime blocking detection;
- task latency where instrumentable;
- worker saturation;
- blocking pool pressure;
- event-loop delay where measurable.

Do not emit a metric per Tokio task ID.

---

# 21. Mobile Observability

The Flutter client is a core operational component because offline capability lives partly on the device.

## 21.1 Required client telemetry

Track aggregate:

```text
app starts
crash-free sessions
sync attempts
sync failures
offline duration distribution
local queue depth
local migration failures
secure-storage failures
API latency bands
API error bands
battery / background limitations where safe
```

## 21.2 Privacy

Client telemetry MUST NOT include:

- raw sales payloads;
- customer contact lists;
- payment credentials;
- access tokens;
- local database dumps.

## 21.3 Offline evidence

The server must receive authoritative synchronization metadata when commands arrive.

Client telemetry is diagnostic only.

A device saying:

```text
sale completed
```

does not make the server believe it.

## 21.4 Device-loss observability

When a device is revoked, observe:

```text
revocation issued
revocation propagated
post-revocation command attempts
sync state
replacement enrollment
```

---

# 22. Tauri Desktop Observability

Desktop telemetry SHOULD cover:

- application startup;
- update verification;
- native command failures;
- storage/database failures;
- synchronization health;
- API connectivity;
- crash loops.

Do not expose filesystem paths containing personal usernames or secrets unless required for internal support.

---

# 23. Payment Observability

The payment specification requires explicit handling of:

```text
provider timeout
unknown outcome
webhook replay
webhook signature failure
provider duplicate
reconciliation mismatch
refund mismatch
```

Observability MUST make each state distinguishable.

Example:

```text
payment.intent.create
      |
      +--> timeout
             |
             +--> payment_unknown_outcome_total += 1
             +--> reconciliation job scheduled
```

The system must never convert:

```text
request timeout
```

directly into:

```text
payment failed
```

without provider/domain evidence.

---

# 24. MRA EIS Observability

MRA EIS integration requires observability around:

```text
terminal state
configuration version
submission state
offline state
retry state
external response
```

The architecture already requires EIS failure to be isolated from sale correctness: the local sale can remain committed while EIS remains pending or enters an explicit tax exception. Observability must make this split visible.

Required dashboard state:

```text
LOCAL SALE SUCCESS
EIS PENDING
EIS RETRY
EIS ACCEPTED
EIS REJECTED
TAX EXCEPTION
```

Never alert “sales failed” merely because EIS failed.

---

# 25. Offline Synchronization Observability

## 25.1 Required lifecycle

```text
LOCAL_COMMIT
   ↓
QUEUED
   ↓
SENDING
   ↓
ACKED / RETRYABLE / CONFLICT / REJECTED
   ↓
CHECKPOINT ADVANCED
```

Observe every transition class.

## 25.2 Critical sync metrics

```text
pending_count
oldest_pending_age
rejection_rate
conflict_rate
retry_rate
checkpoint_lag
schema_mismatch_rate
device_revocation_attempts
```

## 25.3 Sync incident diagnosis

The first investigation questions should be:

1. Is connectivity available?
2. Is the device authenticated?
3. Is the device active?
4. Is the schema compatible?
5. Is the command valid?
6. Is the command authorized?
7. Is idempotency rejecting a duplicate?
8. Is the domain state conflicting?
9. Is the server overloaded?
10. Is checkpoint progression healthy?

---

# 26. Security Observability

## 26.1 Detection signals

Security telemetry should detect:

```text
credential attack
session abuse
MFA attack
BOLA probing
cross-tenant access attempts
cross-branch access attempts
forged approval attempts
replay attempts
webhook forgery
rate-limit abuse
export abuse
device compromise indicators
secret exposure
CI/CD anomalies
```

## 26.2 Correlation

A security event should be correlatable to:

```text
actor or anonymous principal class
session/device if known
tenant scope if known
operation
resource type
result
trace/request ID
```

## 26.3 Security anomaly metrics

Examples:

```text
sitolo_auth_failures_per_principal_class
sitolo_cross_scope_denials_total
sitolo_replay_attempts_total
sitolo_signature_failures_total
sitolo_abuse_blocks_total
```

Use bounded dimensions only.

---

# 27. Health Endpoints

## 27.1 Liveness

Liveness answers:

> Is the process alive enough to be restarted or continue serving?

It MUST remain cheap.

It MUST NOT execute broad database business queries.

## 27.2 Readiness

Readiness answers:

> Can this instance safely accept the class of traffic it is configured to handle?

Checks may include:

- database connectivity;
- required schema compatibility;
- critical dependency state.

Readiness semantics MUST be documented so orchestration does not create cascading restart loops.

## 27.3 Deep health

Administrative deep-health checks MAY test:

- database transaction path;
- provider connectivity;
- object storage;
- worker scheduling.

Deep checks must be rate-limited and authenticated.

---

# 28. Synthetic Monitoring

Production should use safe synthetic operations that do not mutate customer financial state.

Examples:

```text
GET health
GET public config
authenticated read-only tenant probe
read-only catalogue query
non-financial database transaction probe where approved
payment sandbox probe in non-production
EIS certification/sandbox probe where permitted
```

Never use synthetic tests that create real merchant sales, payments or tax filings without explicit authorization and test controls.

---

# 29. Deployment and Release Observability

## 29.1 Deployment markers

Every deployment must emit:

```text
service_version
build_id
commit_sha
environment
deployment_id
migration_version
```

## 29.2 Canary comparison

For canary deployments compare:

```text
error rate
latency
authorization denials
DB load
queue depth
payment failure rate
EIS failure rate
sync rejection rate
```

against the stable population.

## 29.3 Regression detection

A release should be automatically flagged when it causes statistically significant regressions in critical SLO indicators.

## 29.4 Rollback correlation

Rollbacks must emit:

```text
rollback_id
from_version
to_version
reason
operator
approval/reference
```

---

# 30. Migration Observability

Database migrations are production-impacting operations.

Observe:

```text
migration started
migration completed
migration duration
migration failure
lock acquisition delay
backfill progress
rows affected where safe
```

A migration failure MUST NOT be hidden as a generic deployment error.

---

# 31. Backup and Disaster Recovery Observability

Required metrics:

```text
backup_jobs_total
backup_failures_total
backup_age_seconds
backup_size_bytes
restore_test_total
restore_test_failures_total
restore_duration_seconds
point_in_time_recovery_validation_total
```

The system SHOULD alert on:

```text
backup freshness breach
backup job failure
restore verification failure
unexpected backup size change
```

A backup system without restore evidence is not sufficient operational assurance.

---

# 32. Telemetry Failure Modes

## 32.1 Collector unavailable

Expected:

```text
application continues
bounded local buffering
telemetry loss metric increments
no infinite retry loop
```

## 32.2 Metrics backend unavailable

Expected:

```text
application continues
bounded export buffer
controlled telemetry loss
no request-path blocking
```

## 32.3 Trace backend unavailable

Expected:

```text
critical logs remain
request ID remains available
business correctness unaffected
```

## 32.4 Log backend unavailable

Expected:

```text
local structured logs retained within bounded buffer
critical audit unaffected
no unbounded disk growth
```

---

# 33. Backpressure and Resource Controls

Telemetry must be bounded.

Controls:

- maximum log event size;
- maximum trace batch size;
- bounded exporter queue;
- bounded retry count;
- exponential backoff;
- circuit breaking around remote telemetry exporters;
- local disk quotas;
- dropped-record counters.

Telemetry overload must never become a self-inflicted denial of service.

---

# 34. Observability of External Dependencies

Each external dependency should expose:

```text
availability
latency
failure rate
retry rate
unknown-outcome rate
circuit state
queue depth if asynchronous
```

Dependencies include:

- payment providers;
- MRA EIS;
- object storage;
- identity provider;
- notification provider;
- optional Redis;
- observability backend.

The dependency abstraction MUST preserve provider-specific capability differences rather than forcing all providers into the same metric model.

---

# 35. Queue Observability

Every durable queue should expose:

```text
ready depth
in-flight count
oldest item age
processing rate
failure rate
retry count
dead-letter count
lease expirations
```

Queue classes should remain distinguishable:

```text
critical financial/integration side effects
high-priority reconciliation
normal notifications
bulk reporting
maintenance
```

Bulk work must not hide critical queue starvation.

---

# 36. Exception Management Observability

The platform's explicit exception model must be observable.

Examples:

```text
payment_unmatched
payment_duplicate
tax_rejected
stock_mismatch
sync_conflict
provider_timeout
refund_approval_required
device_revoked_pending_events
receiving_discrepancy
```

Required metrics:

```text
sitolo_exceptions_open_total
sitolo_exceptions_created_total
sitolo_exceptions_resolved_total
sitolo_exceptions_oldest_age_seconds
```

Dimensions:

```text
exception_type
priority
status
```

Avoid free-form reason labels.

---

# 37. Financial Integrity Observability

Critical invariant failures should generate both:

```text
operational telemetry
```

and:

```text
durable audit/evidence where required
```

Examples:

```text
sale transaction rolled back
inventory posting mismatch
duplicate financial command
payment double-credit prevention triggered
refund overage rejected
cash close conflict
```

Metrics are useful for detecting patterns. The database state is authoritative for reconciliation.

---

# 38. Fraud and Insider-Risk Observability

Initial fraud analytics MAY be rules-based.

Telemetry indicators include:

```text
unusually high refund frequency
frequent price overrides
large stock adjustments
repeated voids
cash variance patterns
sales outside expected hours
multiple devices per actor
rapid role changes
repeated authorization failures
```

These should feed a separate risk/exception workflow, not directly mutate business state.

---

# 39. Tenant Isolation Observability

A multi-tenant system requires explicit isolation signals.

## 39.1 Required events

```text
tenant_context_resolved
tenant_context_missing
scope_mismatch
cross_tenant_denied
branch_scope_denied
rls_denied
support_scope_override
```

## 39.2 High-risk condition

If a request appears to reference a resource belonging to another tenant, log the denial and security metadata, but do not expose the existence of the resource back to the caller.

## 39.3 Dashboarding

Security operations may view cross-scope denial rates globally.

Tenant administrators should not gain visibility into other tenants' security telemetry.

---

# 40. Observability Test Requirements

Observability itself must be tested.

## 40.1 Unit tests

Test:

- field extraction;
- redaction;
- error classification;
- label normalization;
- event naming;
- trace context parsing.

## 40.2 Integration tests

Test:

- metric emission;
- trace propagation;
- structured log schema;
- exporter failure behavior;
- queue backpressure;
- correlation across worker boundaries.

## 40.3 Security tests

Test:

```text
secret does not appear in logs
access token does not appear in traces
PII minimization works
attacker-controlled labels are bounded
forged trace headers do not affect authorization
cross-tenant dashboard access is denied
```

## 40.4 Failure injection

Simulate:

- collector unavailable;
- metrics backend timeout;
- trace backend timeout;
- log sink unavailable;
- exporter overload;
- oversized log record;
- exporter DNS failure.

The core application must remain safe.

---

# 41. Telemetry Contract Tests

Every service SHOULD run contract tests asserting:

```text
required resource attributes exist
required operation names exist
metric names are stable
units are correct
labels are approved
sensitive fields are absent
HTTP spans are present for configured routes
DB spans do not leak query secrets
errors map to expected families
```

A telemetry schema change should be treated as a contract change where downstream dashboards/alerts depend on it.

---

# 42. Golden Telemetry Fixtures

The repository SHOULD contain fixtures such as:

```text
sale_success.json
sale_timeout.json
payment_unknown_outcome.json
eis_rejection.json
sync_conflict.json
authz_denial.json
worker_retry.json
secret_redaction.json
```

These fixtures allow schema regression tests.

---

# 43. Alert Tests

Alerts must have automated tests where possible.

Examples:

```text
synthetic high 5xx rate -> API alert fires
DB exhaustion -> database alert fires
EIS backlog threshold -> EIS alert fires
sync backlog threshold -> sync alert fires
authorization anomaly -> security alert fires
backup failure -> DR alert fires
```

False-positive tests SHOULD also exist.

Example:

```text
one isolated EIS rejection
-> no major outage alert
```

---

# 44. Runbooks Linked to Alerts

Each high-severity alert MUST link to a runbook containing:

```text
symptom
scope check
first queries
safe actions
unsafe actions
containment
recovery
verification
escalation
post-incident actions
```

Examples:

```text
Database unavailable
Payment unknown outcome spike
EIS rejection spike
Sync backlog spike
Tenant isolation anomaly
Telemetry pipeline failure
```

---

# 45. Incident Investigation Workflow

Recommended sequence:

```text
1. Identify alert / symptom.
2. Establish incident start time.
3. Check deployment timeline.
4. Check service-level metrics.
5. Identify affected capability.
6. Drill into traces for representative requests.
7. Inspect structured logs.
8. Check authoritative database/integration state.
9. Determine whether external side effect occurred.
10. Establish blast radius.
11. Contain unsafe behavior.
12. Recover.
13. Reconcile business state.
14. Preserve evidence.
15. Create regression tests / corrective controls.
```

---

# 46. Common Incident Patterns

## 46.1 HTTP 5xx spike

Investigate:

```text
route
error family
database health
dependency health
recent deployment
resource saturation
```

## 46.2 Payment unknown outcomes

Investigate:

```text
provider latency
provider reference
intent state
verification job
webhook arrival
reconciliation queue
```

Do not assume failure.

## 46.3 EIS rejection spike

Investigate:

```text
configuration version
terminal activation state
payload contract changes
MRA response codes
recent deployment
current MRA requirements
```

Keep local sale state separate from tax submission state.

## 46.4 Sync backlog

Investigate:

```text
connectivity
client version
schema version
authorization changes
server latency
queue depth
conflict distribution
```

## 46.5 Authorization anomaly

Investigate:

```text
operation
actor/session/device
scope
resource type
recent role changes
rate patterns
```

Never inspect raw customer data unnecessarily.

---

# 47. Observability for DR and Restore

After database restore:

```text
1. Verify application version.
2. Verify migration state.
3. Verify telemetry configuration.
4. Verify authoritative watermarks.
5. Reconcile payment states.
6. Reconcile EIS submission states.
7. Reconcile outbox processing.
8. Inspect sync checkpoints.
9. Verify dashboards and alerts.
10. Record recovery evidence.
```

A restored database without restored observability is operationally incomplete.

---

# 48. Multi-Region / Regionalization Considerations

Malawi is the initial market.

Regionalization may introduce:

```text
region
currency
locale
tax regime
provider set
time zone
```

Telemetry should treat region as a bounded dimension.

Currency MUST NOT be inferred from telemetry alone.

Business configuration remains authoritative.

---

# 49. Time Semantics

Use synchronized server timestamps for authoritative operational records.

Telemetry MAY contain:

```text
event time
observed time
export time
```

When diagnosing offline flows, distinguish:

```text
client occurrence time
server receipt time
server commit time
external provider time when known
```

Client clock time is not authoritative.

---

# 50. Clock Skew Observability

Monitor:

```text
client/server timestamp drift
service clock synchronization
provider timestamp differences
```

Large clock drift can affect:

- token validity;
- signed events;
- offline age checks;
- anomaly detection;
- trace ordering.

The system should detect improbable timestamps without trusting them as authoritative.

---

# 51. Configuration Observability

Configuration changes must be observable.

Track:

```text
configuration_key_class
old_version
new_version
changed_at
changed_by
source
```

Do not log secret values.

For high-risk configuration:

```text
audit event + telemetry
```

are required.

Examples:

- payment credentials changed;
- EIS terminal configuration refreshed;
- rate-limit policy changed;
- feature entitlement changed;
- authorization policy changed.

---

# 52. Feature Flag Observability

If feature flags are introduced, every evaluation SHOULD be explainable through:

```text
flag key
flag version
scope class
result
```

Avoid logging full customer targeting rules to every request.

Critical policy flags require auditability.

---

# 53. API Contract Observability

Every externally reachable operation should map to:

```text
route template
operation ID
security classification
SLO class
telemetry policy
```

Example:

```text
POST /api/v1/sales/{sale_id}/finalize
operation = sale.finalize
criticality = CRITICAL_FINANCIAL
```

This enables complete endpoint inventory and alert ownership.

---

# 54. Endpoint Inventory Metric

The platform SHOULD maintain a machine-generated endpoint inventory containing:

```text
method
route_template
operation_id
authentication_required
authorization_class
tenant_scope
sensitive_data_class
rate_limit_class
telemetry_class
```

Unknown/unregistered endpoints in production are a security and observability defect.

---

# 55. Performance Observability

Performance telemetry must separate:

```text
queue delay
application processing
DB wait
external dependency wait
serialization
network
```

A 900 ms request is not actionable until the system knows where the 900 ms went.

Example trace decomposition:

```text
request 900 ms
├─ auth 20 ms
├─ DB 80 ms
├─ inventory lock wait 100 ms
├─ payment provider 650 ms
└─ serialization 50 ms
```

This enables correct remediation.

---

# 56. Tail Latency

Always inspect p50/p95/p99 where meaningful.

Averages can conceal:

- overloaded tenants;
- lock contention;
- cold starts;
- provider timeouts;
- mobile network variability.

Critical POS operations should use percentile latency dashboards.

---

# 57. Saturation Signals

Monitor:

```text
DB pool utilization
worker concurrency
CPU
memory
queue depth
connection count
rate-limit exhaustion
external provider concurrency
```

Saturation is often the earliest signal before outright failure.

---

# 58. Capacity Planning Signals

Retain time series for:

- transactions per second;
- active devices;
- active tenants;
- DB connection utilization;
- storage growth;
- queue throughput;
- telemetry volume.

Capacity plans MUST use observed workloads rather than speculative enterprise numbers.

---

# 59. Cost Observability

Observability itself has cost.

Track:

```text
telemetry volume
trace samples
log bytes
metric series count
collector CPU/memory
storage usage
retention footprint
```

A sudden telemetry cost spike can itself be an incident.

---

# 60. Cardinality Incident Runbook

When metric volume spikes:

```text
1. Identify new metric/label dimensions.
2. Inspect deployment changes.
3. Estimate series growth.
4. Disable or restrict offending dimension.
5. Preserve necessary diagnostic telemetry through logs/traces.
6. Verify exporter recovery.
7. Add a cardinality regression test.
```

Never solve cardinality by blindly increasing infrastructure indefinitely.

---

# 61. Log Flood Incident Runbook

When log volume spikes:

```text
1. Identify event name.
2. Identify service/version.
3. Check retry loops.
4. Check exception loops.
5. Apply temporary rate limiting if available.
6. Preserve high-value error events.
7. Correct source.
8. Verify normal volume.
```

---

# 62. Sensitive Telemetry Incident Runbook

If secrets/PII are detected in telemetry:

```text
1. Stop further emission.
2. Contain affected exporter/view.
3. Determine data class.
4. Revoke/rotate credentials if present.
5. Restrict telemetry access.
6. Determine retention/removal obligations.
7. Identify affected versions.
8. Patch redaction.
9. Backfill detection tests.
10. Verify old secret no longer works.
```

---

# 63. OpenTelemetry Compatibility Policy

OpenTelemetry semantic conventions evolve.

Sitolo MUST pin instrumentation/semantic-convention versions at release time.

Telemetry semantic changes SHOULD be treated like API changes for dashboards and alerts.

The repository should record:

```text
OTel SDK version
collector version
semantic convention version
export protocol
backend compatibility
```

OpenTelemetry's semantic conventions currently span HTTP, database, messaging, RPC, metrics, logs and resources, but some areas remain mixed or development status. Sitolo therefore MUST document which conventions are relied upon and avoid assuming every semantic field is permanently stable. citeturn997793search12turn997793search1turn997793search3

---

# 64. Prometheus Compatibility Policy

If Prometheus is used:

- metric names MUST be stable;
- units MUST be explicit;
- labels MUST be bounded;
- recording rules MUST be version-controlled;
- dashboards MUST use tested queries;
- alert rules MUST be code-reviewed;
- series count MUST be monitored.

Prometheus itself models data as labeled time series, making label governance fundamental rather than cosmetic. citeturn997793search7turn997793search0

---

# 65. Observability Data Pipeline

Reference architecture:

```text
            ┌──────────────────────────┐
            │        SIT0LO             │
            │ API / Workers / Clients   │
            └────────────┬─────────────┘
                         │
                  OpenTelemetry SDK
                         │
                         ▼
               ┌───────────────────┐
               │ Telemetry Gateway │
               │ / Collector       │
               └───────┬───────────┘
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
       Metrics       Traces        Logs
          │            │            │
          ▼            ▼            ▼
       Metrics DB    Trace DB     Log Store
          │            │            │
          └────────────┼────────────┘
                       ▼
                  Dashboards
                       |
                    Alerts
```

Exact vendors remain an infrastructure decision.

---

# 66. Collector Placement

The default should be:

```text
application -> local/nearby collector -> remote backend
```

rather than every service maintaining complex backend-specific export logic.

This supports:

- consistent redaction;
- batching;
- routing;
- sampling;
- version management;
- backend portability.

The collector itself becomes production infrastructure and requires monitoring.

---

# 67. Collector Security

Collector controls:

- authenticated inputs where remote;
- encrypted transport;
- network allowlists;
- bounded queues;
- tenant-aware routing where required;
- configuration versioning;
- secret isolation;
- regular patching;
- audit logging for administrative changes.

---

# 68. Dashboard Authorization

Dashboards must follow the same principle as application interfaces:

```text
WHO
WHAT
TENANT
SCOPE
PURPOSE
```

Roles MAY include:

```text
platform SRE
security analyst
support operator
engineering developer
merchant administrator
```

A merchant administrator must not gain access to raw platform-wide traces.

---

# 69. Support Tool Observability

Support actions must themselves be observable.

Track:

```text
support operator
case/ticket reference
tenant scope
action
reason
result
before/after where safe
```

JIT/break-glass support access must emit high-priority audit evidence.

---

# 70. Privacy-Safe Correlation

When precise identity is unnecessary, use stable pseudonymous references.

For example:

```text
actor_hash
```

may be useful for aggregate investigation, but the hashing strategy MUST avoid reversible disclosure and should not substitute for authoritative identity data when identity is required.

Prefer internal IDs with access control over ad hoc public identifiers.

---

# 71. Observability for Data Integrity Checks

Periodic integrity jobs may emit:

```text
integrity_check_runs_total
integrity_check_failures_total
integrity_anomalies_total
```

Examples:

- inventory ledger reconciliation;
- financial consistency checks;
- orphan detection;
- outbox consistency;
- duplicate detection;
- checkpoint consistency.

These checks must not mutate financial history simply to make the check pass.

---

# 72. Outbox Observability

Required:

```text
outbox_pending_count
outbox_oldest_age_seconds
outbox_processing_total
outbox_failures_total
outbox_retry_total
outbox_dead_letter_total
```

Each outbox processing trace should correlate to:

```text
aggregate type
aggregate ID
outbox ID
attempt
result
```

Exact aggregate identifiers belong in controlled diagnostic data, not metric labels.

---

# 73. Idempotency Observability

Track:

```text
idempotency_hits_total
idempotency_conflicts_total
idempotency_store_errors_total
idempotency_replays_total
```

A spike in idempotency hits can indicate:

- normal mobile retry behavior;
- network instability;
- client bugs;
- upstream retry storm;
- attack behavior.

The metric is a detection signal, not proof of any particular cause.

---

# 74. Retry Observability

Every retrying subsystem should expose:

```text
attempt count
retry cause
backoff
final outcome
```

Retries MUST be bounded.

A retry storm is itself a failure mode.

---

# 75. Circuit Breaker Observability

Where circuit breakers exist:

```text
closed
open
half_open
```

Track:

```text
state transitions
open duration
rejected calls
successful probes
```

Circuit state should be visible in dependency dashboards.

---

# 76. Rate Limiting and Backpressure

Observe both:

```text
requests rejected
```

and:

```text
requests delayed / queued
```

A rising rejection rate with normal capacity can indicate abuse or a policy change. A rising rejection rate with resource saturation can indicate actual overload.

---

# 77. API Security Telemetry

For each route class, observe:

- authentication failures;
- authorization failures;
- rate limiting;
- validation rejection;
- suspicious resource access;
- upload rejection;
- SSRF blocking;
- CSRF validation where applicable;
- signature verification failure.

Do not expose attacker payloads wholesale to logs.

Store bounded excerpts only when explicitly justified and sanitized.

---

# 78. SSRF / File Upload Observability

For remote-fetch features:

```text
ssrf_request_attempts_total
ssrf_blocked_total
ssrf_resolution_failures_total
```

For uploads:

```text
upload_attempts_total
upload_rejections_total
upload_size_bytes
parser_failures_total
malware_scan_failures_total where enabled
```

Never log the full file contents.

---

# 79. Export and Data Exfiltration Signals

Exports can be legitimate but can also be abuse channels.

Observe:

```text
export jobs
export rows
export bytes
export failures
export authorization denials
export frequency
```

Potential anomaly patterns:

```text
sudden bulk export
unusual tenant scope
large number of exports by one principal
exports immediately after role escalation
```

These signals feed security investigation, not automatic punishment unless separate policy authorizes it.

---

# 80. Observability for Authentication and Sessions

Observe:

```text
login success/failure
MFA success/failure
refresh success/failure
session revocation
device revocation
recovery attempts
step-up attempts
```

Do not expose raw credentials or OTP values.

---

# 81. Device and Mobile Security Telemetry

Observe aggregate:

```text
unknown device attempts
revoked device requests
attestation failures if implemented
secure-storage failures
app version skew
schema migration failures
```

Telemetry must not become a shadow identity system.

---

# 82. Worker Reliability

Workers must be designed to run twice.

Telemetry should therefore distinguish:

```text
first execution
retry execution
idempotent no-op
actual duplicate prevention
```

This helps identify whether retries are expected or pathological.

---

# 83. Observability for Long-Running Jobs

Long-running jobs must emit progress safely.

Example:

```text
job started
phase=extract
phase=transform
phase=write
job completed
```

Avoid emitting one log per record.

Progress MAY be sampled or reported every N records / time interval.

---

# 84. Version Skew Observability

Track:

```text
client major/minor version
API version
schema version
sync protocol version
event schema version
```

This is essential for offline clients.

A spike in:

```text
unsupported_schema_version
```

must trigger compatibility investigation.

---

# 85. Schema Evolution Telemetry

Events should record:

```text
schema_version
```

Telemetry should surface:

```text
unsupported event version
fallback parser used
migration compatibility path
```

Do not silently accept unknown versions.

---

# 86. Observability Build Integration

CI MUST validate:

```text
metric naming
label allowlists
log schema fixtures
sensitive-field redaction
alert syntax
recording rules
OTel configuration validity
```

A production telemetry configuration change should not be merged solely because the application compiles.

---

# 87. Static Checks

Recommended CI checks:

```text
metric schema lint
alert rule lint
dashboard query validation
OTel configuration validation
secret scan
PII pattern test
telemetry fixture test
```

---

# 88. Runtime Self-Diagnostics

The service SHOULD expose internally observable status such as:

```text
build
version
migration
telemetry exporter status
DB pool health
worker status
feature/config versions
```

Public endpoints should expose only the minimum safe subset.

---

# 89. Redaction Testing

Mandatory test cases:

```text
password -> REDACTED / absent
JWT -> REDACTED / absent
API key -> REDACTED / absent
payment signature -> REDACTED / absent
terminal secret -> REDACTED / absent
phone number -> minimized/absent as policy requires
full payment payload -> absent
```

Tests should operate on generated logs and traces, not just source-code inspection.

---

# 90. Log Injection Protection

Structured logging MUST prevent attacker-controlled input from breaking log structure.

Prefer:

```text
JSON encoded field
```

over concatenated strings.

Newlines, terminal escape sequences and control characters should be normalized or safely encoded.

---

# 91. Telemetry Tamper Considerations

Telemetry is operational evidence but may not be tamper-proof.

For high-assurance audit evidence, use durable application-controlled stores.

An attacker who compromises a service may potentially alter process logs; therefore incident investigations must correlate logs against:

- database audit events;
- provider records;
- deployment records;
- identity/session records;
- external evidence.

---

# 92. Clock and Ordering Semantics

Logs/traces from multiple systems can arrive out of order.

Use:

```text
server timestamp
monotonic duration where applicable
sequence / event IDs
causation IDs
```

Do not rely on log arrival order as business event order.

---

# 93. Trace-to-Event Correlation

Critical domain events SHOULD retain a correlation reference to the originating operation.

Example:

```text
sale_completed event
  correlation_id
  causation_id
  command_id
```

The trace system can then locate the request path, while the domain event remains authoritative.

---

# 94. Metrics for Business Reconciliation

Reconciliation should expose:

```text
payment entries unmatched
payment entries matched
tax submissions unmatched
inventory discrepancies
cash variances
exception backlog
```

These metrics indicate workload and risk; exact reconciliation results remain in authoritative records.

---

# 95. Support Search Correlation

Support search SHOULD be possible using safe identifiers such as:

```text
tenant ID
receipt number
sale ID
device ID
provider transaction ID
terminal ID
request ID
```

The architecture already identifies these as useful support lookup identifiers. Search access should remain tightly scoped and audited.

---

# 96. Observability During Incident Containment

During severe incidents, telemetry may need elevated collection.

Examples:

```text
increase trace retention
increase debug level for one operation
capture additional DB timing
capture provider error classifications
```

Such changes MUST be:

- authenticated;
- time-limited;
- audited;
- bounded;
- reversible.

Do not globally enable unbounded debug logging in production.

---

# 97. Dynamic Log-Level Controls

Dynamic log-level changes should support:

```text
scope
operator
reason
expiration
```

Example:

```text
service=sitolo-api
level=DEBUG
expires=15 minutes
reason=INC-1234
```

Permanent production DEBUG should require explicit review.

---

# 98. Observability for Security Exceptions

If a security control is temporarily bypassed under break-glass authority, telemetry must record:

```text
control
operator
approval/reference
start
expiration
scope
```

The exception MUST NOT silently disappear into operational logs.

---

# 99. Data Retention Architecture

Retention should be layered.

```text
HOT
  recent dashboards / incident response

WARM
  trend analysis / investigation

COLD / ARCHIVE
  limited high-value evidence where required

DELETE
  expired telemetry
```

Audit and regulatory records follow their own domain retention requirements.

---

# 100. Deletion Safety

Telemetry deletion must not delete authoritative business records.

Deletion jobs should be observable and audited where sensitive data is involved.

---

# 101. Telemetry Access Logging

Access to high-sensitivity observability data should itself produce audit records.

Examples:

```text
support user opened payment exception trace
security analyst queried tenant isolation incident
engineer enabled debug logging
operator changed retention policy
```

---

# 102. Cost and Quota Controls

Observability backends may enforce quotas.

The application SHOULD expose telemetry loss indicators when limits are hit.

Examples:

```text
telemetry_dropped_total
trace_sampled_out_total
log_rate_limited_total
metric_rejected_total
```

A zero-value dashboard caused by backend failure must not look identical to true zero activity.

---

# 103. Zero Telemetry vs Zero Activity

Dashboards must distinguish:

```text
no events occurred
```

from:

```text
no telemetry was received
```

Use heartbeat / scrape-health / exporter metrics to make absence meaningful.

---

# 104. Blackout Detection

The platform SHOULD detect:

```text
service active
BUT
telemetry absent
```

Possible alert:

```text
telemetry blackout for critical service > threshold
```

---

# 105. Runbook — Telemetry Blackout

```text
1. Confirm service health independently.
2. Check collector health.
3. Check exporter errors.
4. Check network path.
5. Check backend ingestion.
6. Check credentials/certificates.
7. Check quota/retention limits.
8. Restore telemetry.
9. Verify dashboards repopulate.
10. Assess missing evidence window.
```

If critical audit evidence was also affected, treat as a separate integrity incident.

---

# 106. Runbook — High Error Rate

```text
1. Check error budget / alert severity.
2. Identify route templates.
3. Identify error families.
4. Check deployment marker.
5. Check database saturation.
6. Check external dependencies.
7. Check authorization anomalies.
8. Stop unsafe release/change if applicable.
9. Contain.
10. Recover.
11. Validate business integrity.
12. Add regression evidence.
```

---

# 107. Runbook — Database Saturation

```text
1. Check connection pool.
2. Check PostgreSQL connections.
3. Check lock waits/deadlocks.
4. Check long transactions.
5. Check top logical query families.
6. Identify bulk jobs.
7. Shed non-critical work.
8. Preserve POS capacity.
9. Verify financial operations.
10. Restore normal load.
```

---

# 108. Runbook — Payment Dependency Degradation

```text
1. Check provider latency/error rates.
2. Check unknown outcomes.
3. Confirm provider reference states.
4. Prevent unsafe retries.
5. Keep payment pending where appropriate.
6. Continue safe cash/offline operations according to policy.
7. Reconcile after recovery.
```

---

# 109. Runbook — EIS Degradation

```text
1. Confirm local sales are committing.
2. Inspect EIS queue.
3. Check terminal/configuration state.
4. Check current MRA response classes.
5. Separate retryable from permanent rejection.
6. Preserve submission evidence.
7. Continue safe operations under offline policy.
8. Reconcile pending tax submissions.
9. Validate no sale records were altered to compensate.
```

---

# 110. Runbook — Sync Degradation

```text
1. Check sync queue.
2. Check oldest pending age.
3. Check server/API health.
4. Check device/auth state.
5. Check protocol/schema versions.
6. Check rejection/conflict distributions.
7. Inspect one representative command trace.
8. Avoid manual data edits.
9. Trigger controlled resync if required.
10. Verify convergence.
```

---

# 111. Runbook — Tenant Isolation Alert

```text
1. Treat as high severity.
2. Freeze affected administrative paths where necessary.
3. Identify operation and scope.
4. Preserve telemetry/audit evidence.
5. Validate authorization and RLS behavior.
6. Determine blast radius.
7. Revoke compromised credentials if indicated.
8. Patch.
9. Run full isolation regression suite.
10. Verify telemetry and audit integrity.
```

---

# 112. Runbook — Telemetry Secret Exposure

Covered by the security incident response but specifically:

```text
1. Identify secret class.
2. Rotate/revoke.
3. Restrict telemetry access.
4. Stop emission.
5. Remove exposed data where policy allows.
6. Patch source.
7. Scan historical telemetry exposure.
8. Add regression test.
```

---

# 113. Release Gate — Observability

A production release MUST NOT pass unless:

```text
[ ] required metrics exist
[ ] metric names/units are valid
[ ] cardinality review passed
[ ] traces are correlated
[ ] structured log schema passes
[ ] secret redaction tests pass
[ ] alert rules pass validation
[ ] dashboards render expected queries
[ ] health endpoints work
[ ] telemetry failure is non-fatal to core correctness
[ ] critical audit evidence remains durable
[ ] runbooks exist for critical alerts
[ ] deployment markers exist
[ ] telemetry versioning is recorded
```

---

# 114. Definition of Observability Done

A feature is observably complete when:

```text
[ ] critical operations have metrics
[ ] critical operations have traces
[ ] critical operations have structured logs
[ ] errors map to stable families
[ ] correlation IDs exist
[ ] sensitive data is redacted
[ ] tenant scope is safe
[ ] dashboard exists if operationally important
[ ] alert exists where action is required
[ ] runbook exists for high-severity failure
[ ] tests validate telemetry
[ ] failure of telemetry does not corrupt business state
[ ] retention is defined
[ ] access controls are defined
[ ] cardinality is reviewed
```

---

# 115. Observability Anti-Patterns

## 115.1 Log Everything

Forbidden as a general strategy.

## 115.2 User ID as Metric Label

Forbidden for high-volume metrics.

## 115.3 Trace ID as Business ID

Forbidden.

## 115.4 Telemetry as Source of Truth

Forbidden.

## 115.5 Dashboard-Only Operations

If an operational action cannot be investigated without a manually curated dashboard, the underlying telemetry is probably incomplete.

## 115.6 Silent Telemetry Failure

Forbidden for critical telemetry infrastructure.

## 115.7 Raw Provider Payload Logging

Forbidden unless a narrowly scoped, access-controlled evidence workflow explicitly requires it.

## 115.8 Dynamic Debug Forever

Forbidden.

## 115.9 Unbounded Exporter Retry

Forbidden.

## 115.10 Alert Without Action

Avoid.

---

# 116. Example End-to-End Trace

A successful sale may look conceptually like:

```text
TRACE 4fd...
│
├─ POST /sales/{id}/finalize
│  ├─ authenticate
│  ├─ authorize sale.finalize
│  ├─ load sale
│  ├─ lock inventory
│  ├─ post inventory
│  ├─ commit sale transaction
│  └─ create outbox event
│
└─ worker: sale.completed
   └─ optional downstream integration
```

Associated metrics:

```text
sales_attempted_total += 1
sales_committed_total += 1
sale_duration_seconds.observe(...)
```

Associated audit evidence:

```text
sale finalized
actor
branch
register
authorization evidence reference
command_id
```

The three evidence classes remain separate but correlated.

---

# 117. Example Payment Failure Trace

```text
payment.intent.create
   |
   +--> provider.request
          |
          +--> timeout
```

Metrics:

```text
payment_provider_failures_total += 1
payment_unknown_outcome_total += 1
```

Domain state:

```text
PAYMENT_PENDING / UNKNOWN_OUTCOME
```

Recovery workflow:

```text
provider verification
   ↓
reconciliation
```

No premature “failed” assumption is permitted.

---

# 118. Example EIS Failure Trace

```text
sale.finalize
   |
   +--> local commit
   |
   +--> outbox
           |
           +--> eis.sale.submit
                  |
                  +--> MRA rejected
```

Business truth:

```text
sale = COMMITTED
tax = REJECTED / EXCEPTION
```

Observability must display this as two related states rather than one collapsed failure.

---

# 119. Example Sync Conflict Trace

```text
sync.push
  |
  +--> authenticate
  +--> authorize
  +--> idempotency
  +--> domain validation
  +--> concurrency check
         |
         +--> conflict
```

Metrics:

```text
sync_conflicts_total += 1
```

Logs:

```text
sync_conflict_detected
command_id
conflict_class
resource_type
```

Authoritative result:

```text
CONFLICT
```

---

# 120. Telemetry Governance

Changes to any of the following require review:

- telemetry fields;
- metric names;
- metric labels;
- semantic convention version;
- log schema;
- trace propagation;
- retention;
- dashboard authorization;
- redaction rules;
- alert thresholds for critical capabilities;
- export configuration.

Changes affecting business incident response SHOULD be documented in the implementation change or ADR process.

---

# 121. Ownership Model

Suggested ownership:

```text
Application metrics -> service owner
Domain/business metrics -> domain owner
Security telemetry -> security owner
Database telemetry -> platform/data owner
Client telemetry -> mobile/desktop owner
Collector -> platform/SRE
Dashboards -> capability owner
Alerts -> operational owner
Runbooks -> operational owner
Audit evidence -> domain/security governance
```

Every critical alert MUST have a human owner.

---

# 122. Operational Readiness Checklist

```text
ARCHITECTURE
[ ] signal model defined
[ ] correlation model defined
[ ] telemetry trust boundary defined

METRICS
[ ] core metrics implemented
[ ] labels bounded
[ ] cardinality reviewed
[ ] units standardized

TRACES
[ ] HTTP propagation
[ ] DB spans
[ ] worker correlation
[ ] external calls

LOGS
[ ] structured JSON
[ ] event names stable
[ ] secret redaction
[ ] PII minimization

ALERTING
[ ] SLOs defined
[ ] critical alerts defined
[ ] burn-rate strategy where applicable
[ ] runbooks linked

SECURITY
[ ] telemetry access control
[ ] dashboard authorization
[ ] collector authentication
[ ] telemetry injection protections

RELIABILITY
[ ] exporter backpressure
[ ] telemetry failure non-fatal
[ ] blackout detection
[ ] retention limits

TESTING
[ ] unit tests
[ ] integration tests
[ ] redaction tests
[ ] alert tests
[ ] cardinality tests
[ ] chaos/failure tests

OPERATIONS
[ ] deployment markers
[ ] backup/restore telemetry
[ ] version skew telemetry
[ ] incident evidence workflow
```

---

# 123. Machine-Enforceable CI Requirements

The CI pipeline should reject changes when:

```text
metric uses unapproved label
metric has undocumented unit
metric name violates naming policy
sensitive test fixture leaks secrets
alert rule syntax invalid
required runbook reference absent
telemetry schema fixture broken
instrumentation version drift is uncontrolled
```

Exact tooling can vary; the control objective is mandatory.

---

# 124. Observability Regression Matrix

| Area | Regression | Required result |
|---|---|---|
| Metrics | new unbounded label | CI fails |
| Logs | secret emitted | CI fails |
| Traces | lost propagation | CI fails |
| Auth | forged trace accepted as authority | request denied based on auth, trace ignored |
| Database | raw SQL secrets emitted | test fails |
| Payment | provider ref missing from evidence | integration test fails |
| EIS | tax failure hides sale success | integration test fails |
| Sync | command correlation lost | integration test fails |
| Tenant | dashboard crosses scope | access denied |
| Telemetry backend | collector unavailable | business transaction remains safe |
| Alerting | critical alert rule invalid | CI fails |
| Deployment | version marker missing | release gate fails |

---

# 125. Final Architecture Statement

Sitolo's observability architecture is intentionally subordinate to the business model while being rich enough to explain failures in a security-sensitive, offline-capable, multi-tenant system.

The architecture is:

```text
AUTHORITATIVE DOMAIN STATE
          ↓
CORRELATION CONTEXT
          ↓
METRICS + TRACES + LOGS
          ↓
DASHBOARDS + ALERTS
          ↓
INCIDENT RESPONSE
          ↓
RECOVERY / RECONCILIATION
          ↓
REGRESSION TESTS
```

The implementation must preserve these boundaries:

```text
metrics != business truth
traces != authorization
logs != audit authority
telemetry != customer data warehouse
provider response != internal financial truth
client telemetry != server authority
```

The most important operational invariant is:

> **A failure must be diagnosable without requiring the operator to guess whether a side effect committed.**

That means critical workflows must expose enough evidence to reconstruct:

```text
request
→ authorization
→ transaction
→ commit/rollback
→ durable event
→ external side effect
→ reconciliation
```

For Sitolo this is particularly important because the system intentionally permits controlled degradation:

```text
network unavailable
    → offline continuity where authorized

payment provider unavailable
    → pending / reconciliation

MRA EIS unavailable
    → tax pending / retry / exception

telemetry backend unavailable
    → bounded telemetry degradation
```

The platform must never confuse graceful degradation with “always allow.”

## Definition of Observability Complete

Observability is complete only when the system can answer, with machine-readable evidence:

```text
WHAT happened?
WHEN?
WHERE?
FOR WHOM?
UNDER WHICH SCOPE?
UNDER WHICH VERSION?
DID THE TRANSACTION COMMIT?
DID THE EXTERNAL SIDE EFFECT OCCUR?
WHAT IS AUTHORITATIVE?
WHAT IS RETRYABLE?
WHAT IS THE BLAST RADIUS?
WHAT IS THE SAFE RECOVERY ACTION?
```

And when a failure is resolved:

```text
DETECT
  ↓
CONTAIN
  ↓
PRESERVE EVIDENCE
  ↓
RECOVER
  ↓
RECONCILE
  ↓
VERIFY
  ↓
ADD REGRESSION CONTROL
```

This is the observability contract required for Sitolo to operate as a serious business system rather than a conventional application with dashboards attached.

---

# Appendix A — Core Metric Registry Starter Set

| Metric | Type | Unit | Key labels |
|---|---|---|---|
| `sitolo_http_requests_total` | Counter | count | method, route_template, status_class |
| `sitolo_http_request_duration_seconds` | Histogram | seconds | method, route_template |
| `sitolo_errors_total` | Counter | count | operation, error_family |
| `sitolo_authentication_failures_total` | Counter | count | method, reason_class |
| `sitolo_authorization_denials_total` | Counter | count | operation, resource_type |
| `sitolo_db_query_duration_seconds` | Histogram | seconds | operation |
| `sitolo_db_pool_wait_seconds` | Histogram | seconds | pool |
| `sitolo_db_deadlocks_total` | Counter | count | operation |
| `sitolo_sales_committed_total` | Counter | count | channel, result |
| `sitolo_sales_duration_seconds` | Histogram | seconds | channel |
| `sitolo_inventory_conflicts_total` | Counter | count | operation, reason_class |
| `sitolo_payment_provider_failures_total` | Counter | count | provider, operation, failure_class |
| `sitolo_payment_unknown_outcome_total` | Counter | count | provider, operation |
| `sitolo_payment_reconciliation_exceptions_total` | Gauge/derived | count | provider, exception_type |
| `sitolo_eis_rejections_total` | Counter | count | operation, failure_class |
| `sitolo_eis_queue_depth` | Gauge | count | operation |
| `sitolo_sync_commands_rejected_total` | Counter | count | command_type, reason_class |
| `sitolo_sync_conflicts_total` | Counter | count | command_type, conflict_class |
| `sitolo_sync_oldest_pending_age_seconds` | Gauge | seconds | command_class |
| `sitolo_worker_jobs_failed_total` | Counter | count | job_type, queue_class |
| `sitolo_worker_job_duration_seconds` | Histogram | seconds | job_type |
| `sitolo_worker_job_queue_depth` | Gauge | count | job_type |
| `sitolo_exceptions_open_total` | Gauge | count | exception_type, priority |
| `sitolo_rate_limit_rejections_total` | Counter | count | operation, reason_class |
| `sitolo_telemetry_export_failures_total` | Counter | count | signal, backend |
| `sitolo_telemetry_dropped_total` | Counter | count | signal, reason |

The registry MUST be version-controlled.

---

# Appendix B — Approved Metric Label Vocabulary

```text
service
service_version_major
environment
region
method
route_template
status_class
operation
result
error_family
reason_class
resource_type
provider
integration
job_type
queue_class
command_type
command_class
conflict_class
failure_class
channel
offline_online_mode
platform
app_major_version
signal
backend
exception_type
priority
```

Any addition requires a cardinality review.

---

# Appendix C — Redaction Vocabulary

```text
password
passphrase
secret
token
access_token
refresh_token
client_secret
api_key
private_key
signing_key
mfa_secret
otp
authorization
cookie
set-cookie
signature
payment_secret
terminal_secret
provider_credential
connection_string
DSN
```

---

# Appendix D — Required Incident Queries

Operational tooling SHOULD make these query patterns available:

```text
requests by route and status
errors by operation and family
slowest operations
DB pool wait
lock contention
payment unknown outcomes
EIS backlog
sync oldest pending
authorization denials
cross-scope denials
worker dead letters
backup failures
telemetry loss
recent deployment markers
```

---

# Appendix E — External Standards and References

1. OpenTelemetry Semantic Conventions 1.44.0: https://opentelemetry.io/docs/specs/semconv/  
2. OpenTelemetry HTTP Semantic Conventions: https://opentelemetry.io/docs/specs/semconv/http/  
3. OpenTelemetry Database Semantic Conventions: https://opentelemetry.io/docs/specs/semconv/db/  
4. OpenTelemetry General Logs Semantic Conventions: https://opentelemetry.io/docs/specs/semconv/general/logs/  
5. OpenTelemetry Logs Data Model: https://opentelemetry.io/docs/specs/otel/logs/data-model/  
6. Prometheus Metric and Label Naming: https://prometheus.io/docs/practices/naming/  
7. Prometheus Instrumentation: https://prometheus.io/docs/practices/instrumentation/  
8. Prometheus Data Model: https://prometheus.io/docs/concepts/  
9. Prometheus Zen of Prometheus: https://prometheus.io/docs/practices/the_zen/  
10. W3C Trace Context: https://www.w3.org/TR/trace-context/

---

# Appendix F — Governance Rule

Any implementation that materially changes:

- telemetry trust boundaries;
- audit/evidence semantics;
- tenant visibility;
- secret redaction;
- metric cardinality strategy;
- trace propagation semantics;
- critical SLO definitions;
- incident evidence retention;
- external integration observability;
- deployment/release telemetry;

must undergo architecture/security review and, where it changes a documented invariant, an ADR.

The standard is not “many dashboards.”

The standard is:

> **repeatable, secure, low-noise, high-value evidence that lets engineering determine what happened, what is authoritative, what is safe to retry, and how to recover without corrupting business truth.**

---

**Document end — Sitolo Observability Specification, Phase 0 / File 11 of 16.**
