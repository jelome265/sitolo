# SITOLO — Phase 0–2 Implementation Audit & Remediation Contract

**Repository:** `jelome265/sitolo`  
**Branch audited:** `main`  
**Audit context:** Phase 3 development in progress  
**Audit scope:** Phase 0, Phase 1, Phase 2 implementation correctness and Phase 3 readiness  
**Primary evidence:** current repository implementation plus the existing Sitolo Phase 0–2 contracts  
**Artifact type:** implementation audit + remediation contract  
**Status:** Action required before Phase 3 is allowed to rely on the audited foundations

---

## 0. Executive Decision

This audit compares the current `main` repository implementation against the existing Sitolo source-of-truth documents for:

- Phase 0 — architecture, domain, security, contracts and ADR freeze;
- Phase 1 — repository, Rust workspace, CI, reproducibility and supply-chain controls;
- Phase 2 — configuration, secrets, logging, errors, telemetry and PostgreSQL runtime boundaries.

The audit deliberately does **not** attempt to redesign Phase 3 identity/session/MFA/device semantics. Phase 3 is considered only where its implementation consumes, bypasses, weakens or contradicts a Phase 0–2 contract.

The central result is:

> **Sitolo has a strong architectural/document baseline, but the current implementation is not yet an evidence-complete realization of the Phase 0–2 contracts. The largest defect is not one isolated vulnerability; it is the incomplete vertical integration of the foundation.**

The repository contains the expected security/configuration/observability/persistence boundary crates and a substantial CI policy surface. However, the API executable itself remains a nine-line Phase 1 scaffold with no runtime dependencies and no Phase 2 startup path. That means the Phase 2 operational plane exists primarily as reusable library code rather than as an actually assembled service.

The current CI system also has a material release-assurance problem: the release artifact workflow builds and tests the workspace, but it does not execute the full canonical verification/security chain required by the Phase 1 contract before packaging an immutable release artifact. This creates two subtly different definitions of “verified source”: the normal CI path and the tag/release path.

A second class of defects affects individual Phase 2 primitives:

- correlation identifiers are not globally unique across instances and expose sequence information;
- W3C `traceparent` validation accepts values outside the protocol's semantic validity rules;
- the public operational-event emission API accepts an unconstrained operation string, creating an unbounded diagnostic input surface;
- telemetry queue priority is modeled but not actually enforced when the queue is saturated, allowing the most important event classes to be dropped;
- event schema versions are registered but emission hard-codes version `1`;
- the Phase 2 secret-provider test does not execute the asynchronous provider operation and therefore does not prove the documented fail-closed behavior;
- configuration defaults are layered from development defaults even when the selected runtime environment is production, which makes configuration provenance weaker than the contract requires;
- the PostgreSQL boundary is intentionally still only a boundary, which is acceptable for Phase 2, but there is currently no application bootstrap that consumes the boundary.

This document converts those findings into a remediation contract. It is intended to be executable by an engineering agent without inventing new architecture.

---

# 1. Audit Authority and Source-of-Truth Hierarchy

The audit follows the repository's own authority model rather than treating current code as the truth.

The documented precedence is:

```text
PRODUCT / BUSINESS SCOPE
        ↓
BUSINESS MODEL
        ↓
PRODUCT / DOMAIN CONTRACT
        ↓
SYSTEM ARCHITECTURE
        ↓
SECURITY ARCHITECTURE
        ↓
SECURITY IMPLEMENTATION CONTRACT
        ↓
DOMAIN MODEL
        ↓
DATABASE / API / AUTH / SYNC / INTEGRATION CONTRACTS
        ↓
TESTING / OBSERVABILITY / DEPLOYMENT CONTRACTS
        ↓
THREAT MODEL
        ↓
CI / SECURITY-TEST ENFORCEMENT
        ↓
IMPLEMENTATION
```

The security implementation specification explicitly states that code does not redefine the business model, routes do not redefine authorization, migrations do not silently redefine the domain model, and CI convenience does not override a release gate.

The current Phase dependency is:

```text
PHASE 0
Architecture / contracts / ADR freeze
        ↓
PHASE 1
Repository / Rust workspace / CI
        ↓
PHASE 2
Config / secrets / logging / errors / telemetry
        ↓
PHASE 3
Identity / sessions / MFA / devices
```

Phase 3 therefore inherits Phase 0–2 invariants. It must not compensate for missing platform foundations by creating parallel infrastructure.

### Governing implementation principle

Every externally reachable operation must ultimately pass through:

```text
UNTRUSTED INPUT
    ↓
AUTHENTICATION
    ↓
TRUSTED SECURITY CONTEXT
    ↓
TENANT / SCOPE
    ↓
VALIDATION
    ↓
DOMAIN INVARIANTS
    ↓
TRANSACTION / CONCURRENCY CONTROL
    ↓
IDEMPOTENCY WHERE REQUIRED
    ↓
AUTHORITATIVE COMMIT
    ↓
AUDIT / OUTBOX
    ↓
EXTERNAL SIDE EFFECT
    ↓
RECONCILIATION
    ↓
OBSERVABILITY
    ↓
RECOVERY
    ↓
REGRESSION TEST
```

At Phase 0–2, the business-domain stages are largely future work. The foundation stages must already exist as reusable, fail-closed infrastructure.

---

# 2. Audit Scope

## 2.1 Included

This audit reviews:

### Phase 0
- architectural boundaries;
- domain authority;
- security invariants;
- source-of-truth precedence;
- trust boundaries;
- error semantics;
- audit/observability authority;
- maintainability constraints;
- ADR and contract adherence.

### Phase 1
- repository topology;
- Rust workspace boundaries;
- dependency direction;
- pinned toolchain;
- locked dependencies;
- CI execution;
- security scanning;
- artifact generation;
- SBOM/provenance;
- release promotion;
- failure propagation;
- supply-chain controls.

### Phase 2
- configuration parsing;
- configuration precedence;
- environment isolation;
- production-safe defaults;
- secret references;
- secret provider semantics;
- redaction;
- correlation identifiers;
- trace context;
- structured events;
- metric definitions;
- telemetry buffering;
- persistence boundaries;
- startup/readiness semantics;
- runtime integration.

### Phase 3 readiness
Only the prerequisites inherited from Phase 0–2 are reviewed:
- whether Phase 3 can consume the common runtime infrastructure;
- whether Phase 3 would currently be forced to introduce duplicate config/error/telemetry/security infrastructure;
- whether current Phase 3 work risks bypassing the existing contracts.

## 2.2 Excluded by design

The following are **not** treated as Phase 0–2 defects merely because they are not implemented yet:

- PostgreSQL schema and RLS implementation;
- complete tenant management;
- authorization engine;
- production identity provider;
- full session persistence in PostgreSQL;
- payment workflows;
- inventory;
- POS;
- offline synchronization;
- MRA EIS;
- reporting;
- billing;
- support impersonation.

These belong to later phases unless an implementation already exists that conflicts with an upstream contract.

---

# 3. Current Repository Baseline

The current repository contains:

- `apps/api`
- `apps/worker`
- modular foundation crates such as:
  - `sitolo-config`
  - `sitolo-security`
  - `sitolo-observability`
  - `sitolo-persistence`
  - `sitolo-auth`
  - `sitolo-domain`
  - `sitolo-application`
  - `sitolo-api`
  - `sitolo-authz`
  - `sitolo-tenancy`
  - `sitolo-events`
  - `sitolo-audit`
  - `sitolo-integrations`
  - `sitolo-sync`
  - `sitolo-testkit`
- six GitHub Actions workflows covering:
  - policy;
  - Rust verification;
  - integration;
  - security;
  - artifact;
  - release promotion.

The current repository is therefore structurally aligned with the intended modular-monolith direction, but structural presence must not be confused with runtime completeness.

---

# 4. Risk Rating Model

| Rating | Meaning | Release treatment |
|---|---|---|
| P0 / Critical | Can invalidate the trust model, release assurance or service authority | Must block Phase 3 progression |
| P1 / High | Material security/correctness/integrity weakness | Must be remediated before dependent Phase 3 work |
| P2 / Medium | Significant maintainability, compatibility, observability or correctness risk | Remediate before Phase 3 exit |
| P3 / Low | Local quality issue or future drift risk | Can be sequenced, but must be recorded |
| Evidence Gap | Cannot prove compliance from repository evidence | Treat as unverified until evidence exists |

Severity is based on **impact × exploitability × contract significance × blast radius**, not on how small the code defect appears.

---

# 5. Findings Summary

| ID | Finding | Area | Severity | Phase | Required action |
|---|---|---|---|---|---|
| F-001 | API binary is still an empty scaffold; Phase 2 is not vertically integrated | Runtime bootstrap | P0 | 2 | Build a single application bootstrap path consuming Phase 2 contracts |
| F-002 | Production environment can inherit development configuration defaults | Configuration | P1 | 2 | Make environment selection occur before environment-specific defaults |
| F-003 | Secret-reference provenance is not strongly bound to runtime environment/provider | Secrets | P1 | 2 | Validate reference/provider compatibility and prohibit ambiguous production references |
| F-004 | OTLP TLS validation is bypassable by scheme-less remote endpoints | Telemetry config | P2 | 2 | Make transport security explicit and fail closed for remote collectors |
| F-005 | Service name/version do not reject control characters | Telemetry/logging | P3 | 2 | Apply the same structured-text safety validation used elsewhere |
| F-006 | `traceparent` parser validates shape but not semantic W3C constraints | Observability | P2 | 2 | Reject forbidden version/zero identifiers and test protocol semantics |
| F-007 | Server request IDs are process-local sequential identifiers | Observability | P2 | 2 | Use globally unique server-generated correlation IDs |
| F-008 | Operational event emission accepts an unbounded operation field | Logging/telemetry | P1 | 2 | Introduce bounded typed operation values and reject raw attacker-sized strings |
| F-009 | Event registry schema versions are ignored at emission time | Event registry | P2 | 2 | Emit the registry-defined schema version |
| F-010 | Telemetry priority model does not protect P0/P1 records under saturation | Telemetry | P1 | 2 | Reserve capacity and implement priority-aware shedding |
| F-011 | Telemetry buffer is a counter, not a durable record queue | Telemetry | P2 | 2 | Either explicitly mark as contract-only or implement record lifecycle required by Phase 2 |
| F-012 | Secret-provider asynchronous fail-closed behavior is not actually tested | Security testing | P1 | 2 | Execute provider calls in tests and verify failure semantics |
| F-013 | Development provider ignores the SecretRef path | Secrets | P2 | 2 | Make provider semantics explicit: class-only mapping or reference-aware mapping |
| F-014 | Secret-missing errors expose secret-reference paths | Error handling | P2 | 2 | Redact or classify references before logs/telemetry |
| F-015 | Release workflow bypasses canonical Phase 1 verification/security chain | CI/CD | P0 | 1 | Make release artifacts a product of the same mandatory verification gate |
| F-016 | Artifact workflow grants broad write permissions at workflow level | CI security | P1 | 1 | Narrow permissions to job/action needs and tag-only release jobs |
| F-017 | Artifact path identity uses two different commit variables | Release logic | P2 | 1 | Compute source/artifact path once and reuse it |
| F-018 | PostgreSQL integration is an explicit Phase 1 exception, but exception enforcement is weak | CI/testing | P1 | 1 | Record a durable exception/expiry and define mandatory Phase 2/5 completion gates |
| F-019 | Canonical Phase 1 repository topology is only partially realized | Repository architecture | P2 | 1 | Record accepted omissions or create required foundation directories |
| F-020 | No single executable startup contract verifies config, secrets, telemetry, DB intent and readiness | Runtime correctness | P0 | 2 | Add deterministic startup orchestration and shutdown lifecycle |
| F-021 | Phase 3 risks creating duplicate infrastructure if it proceeds before F-001/F-020 | Architecture governance | P1 | 3 readiness | Freeze duplicate platform implementations |
| F-022 | CI evidence does not itself prove protected branch/repository settings | Governance | Evidence Gap | 0/1 | Validate branch protection and required checks as repository-admin evidence |

---

# 6. Detailed Findings and Remediation Contracts

# F-001 — API binary is still an empty scaffold

**Severity:** P0 / Critical  
**Category:** Runtime integration, architecture, maintainability, release correctness  
**Phase:** Phase 2  
**Status:** Blocking

## Evidence

The API application currently contains only a minimal `main.rs` scaffold and the application package declares no runtime dependencies.

The source explicitly labels itself as a Phase 1 scaffold, and the process does not assemble:

- `AppConfig`;
- secret providers;
- configuration validation;
- request correlation;
- structured logging;
- telemetry state;
- error mapping;
- persistence runtime intent;
- graceful shutdown;
- readiness.

The repository therefore contains the pieces of the platform but no single executable that composes them into the service lifecycle.

## Why this is a defect

Phase 2 is not merely “create utility crates”.

The Phase 2 contract requires that a Rust executable can determine, before serving traffic:

- what configuration is active;
- what secrets are required;
- what artifact is running;
- whether startup trust validation passed;
- how requests are correlated;
- how errors are classified;
- how telemetry is bounded;
- how telemetry failures degrade;
- how persistence lifecycle is represented.

Without the executable bootstrap, none of those properties is demonstrated at the application boundary.

This also creates a maintainability trap: Phase 3 developers are likely to wire identity directly into `main.rs` or introduce a second application bootstrap, bypassing the intended shared platform contracts.

## Failure modes

### Failure mode A — Phase 3 duplicates Phase 2

A Phase 3 authentication implementation may create its own:

- tracing subscriber;
- request ID middleware;
- configuration loading;
- environment secret reads;
- error conversion;
- readiness endpoint.

The result becomes:

```text
platform implementation
+
auth-specific implementation
+
eventually business-specific implementation
```

This is precisely the fragmentation the Phase 2 specification forbids.

### Failure mode B — insecure startup

A service can compile and run without proving:

```text
production configuration
+
secret policy
+
telemetry policy
+
error policy
+
persistence boundary
```

## Remediation

Create a single application bootstrap layer with an explicit lifecycle:

```text
load raw environment
        ↓
parse typed configuration
        ↓
validate configuration
        ↓
initialize redaction/security primitives
        ↓
initialize structured logging
        ↓
create correlation/telemetry infrastructure
        ↓
construct secret provider according to environment
        ↓
construct database runtime intent
        ↓
validate startup dependencies
        ↓
construct router
        ↓
publish readiness
        ↓
serve traffic
        ↓
graceful shutdown
        ↓
close persistence/telemetry resources
```

Do not move business logic into the bootstrap.

The bootstrap should own composition, not domain decisions.

## Required code structure

Target shape:

```text
apps/api/src/
    main.rs
    bootstrap.rs
    state.rs
    shutdown.rs
```

Suggested responsibilities:

### `main.rs`

Only process-level orchestration:

```text
main
  -> bootstrap
  -> run
  -> shutdown
  -> process exit
```

### `bootstrap.rs`

Own:

- configuration;
- startup validation;
- logging;
- secrets;
- telemetry;
- persistence intent;
- router/application state.

### `state.rs`

Own only immutable runtime dependencies shared with request handlers.

### `shutdown.rs`

Own:

- SIGTERM;
- SIGINT;
- cancellation;
- orderly subsystem shutdown.

## Acceptance criteria

Phase 2 is not considered vertically integrated until:

```text
[ ] API executable loads typed configuration
[ ] configuration validation is fail-closed
[ ] secret provider is environment appropriate
[ ] logging initializes exactly once
[ ] correlation context initializes exactly once
[ ] telemetry buffer/export policy initializes exactly once
[ ] error boundary is installed
[ ] persistence intent is constructed
[ ] startup failure stops the service
[ ] readiness is not published before startup checks complete
[ ] shutdown executes in deterministic order
[ ] no duplicate platform implementation exists in Phase 3
[ ] startup integration tests execute the real bootstrap
```

---

# F-002 — Production environment can inherit development configuration defaults

**Severity:** P1 / High  
**Category:** Security, configuration correctness  
**Phase:** Phase 2

## Evidence

The environment loader starts from the development defaults builder and then overlays values from `SITOLO__...` environment variables.

This means environment selection is effectively:

```text
development defaults
        ↓
environment override
```

rather than:

```text
environment selector
        ↓
environment-appropriate defaults
        ↓
explicit overrides
```

The validation layer prevents `allow_local_secret_provider=true` in production, which is good. It does not, however, make every default field environment-aware.

## Why this matters

An operator can set:

```text
SITOLO__RUNTIME__ENVIRONMENT=production
```

without necessarily replacing every development-flavored default.

That creates a dangerous class of configuration:

```text
environment = production
security policy = production
database/secret reference = inherited default
```

A production process should never rely on accidental developer defaults.

## Failure modes

- production points at development resource identity;
- production secret namespace references a development location;
- future fields added to the development preset automatically become production defaults;
- a new unsafe development field can silently become a production default before a validator is written.

The last case is the most dangerous because the system becomes less safe as the configuration model evolves.

## Remediation

Make environment selection a first-class parsing step.

Recommended algorithm:

```text
1. Read only the environment selector.
2. Parse environment.
3. Choose the corresponding baseline:
   development / staging / production.
4. Apply explicit environment variables.
5. Perform cross-field validation.
6. Produce AppConfig.
```

Do not instantiate development defaults and then mutate the environment enum.

Alternatively, remove environment-specific mutable defaults entirely and require every environment-sensitive value to be explicit.

## Additional invariant

A production AppConfig must be impossible to construct from development-only defaults unless each default is explicitly documented as production-safe.

## Acceptance tests

Add tests for:

```text
production environment + no DB override
production environment + no secret-reference override
production environment + development telemetry endpoint
production environment + local provider flag
production environment + verbose logs
```

The expected result is either:

```text
valid because the default is explicitly production-safe
```

or:

```text
startup rejected with an actionable configuration error
```

There must be no ambiguous middle state.

---

# F-003 — Secret-reference provenance is not strongly bound to environment/provider

**Severity:** P1 / High  
**Category:** Secrets, configuration, trust boundaries  
**Phase:** Phase 2

## Evidence

The configuration contains a typed `SecretRef`, which is correct.

The development environment provider maps a secret class to an environment variable and does not use the actual `SecretRef` path to select the environment value.

The current contract therefore distinguishes:

```text
secret class
```

but not necessarily:

```text
secret authority
secret namespace
secret environment
secret version
```

## Why this matters

A secret reference is supposed to identify controlled access material, not merely label the type of secret.

A production-safe reference model should prevent states such as:

```text
production runtime
    ↓
development secret namespace
```

or:

```text
production runtime
    ↓
generic secret class
    ↓
provider chooses an unrelated default
```

## Remediation

Extend the semantic model so a secret reference contains sufficient provenance to enforce environment compatibility.

At minimum:

```text
SecretRef
├── class
├── authority/provider
├── namespace/path
└── version selector (when supported)
```

The exact shape must follow the existing architecture rather than inventing a new provider-specific API.

The provider must verify:

```text
requested environment
        ==
allowed secret authority/environment
```

Production must not accept local development authorities.

## Important constraint

Do not put resolved secret values inside `AppConfig`.

The current design correctly keeps secret material out of typed application configuration. Preserve that.

## Acceptance criteria

```text
[ ] production cannot select local secret provider
[ ] production cannot use development namespace
[ ] secret reference is validated before network/database initialization
[ ] secret provider cannot silently reinterpret reference authority
[ ] secret material never appears in errors
[ ] secret material never appears in telemetry
[ ] tests cover wrong-environment references
```

---

# F-004 — OTLP TLS validation is bypassable by scheme-less endpoints

**Severity:** P2 / Medium  
**Category:** Configuration security  
**Phase:** Phase 2

## Evidence

The current validator rejects non-HTTPS endpoints only when the configured endpoint contains a URI scheme.

A value such as:

```text
collector.example.internal:4317
```

has no `://` and therefore bypasses the explicit HTTPS check.

The configuration model states that remote telemetry endpoints require TLS.

## Why this matters

A hostname/port pair is still a remote endpoint.

Security validation should not depend on whether the operator writes:

```text
https://collector.example
```

versus:

```text
collector.example:4317
```

if both ultimately represent remote transport.

## Remediation

Make transport security a first-class setting.

Preferred:

```text
TelemetryEndpoint {
    address,
    transport_security
}
```

where production remote telemetry requires:

```text
transport_security = tls
```

If the OTLP library already has explicit TLS configuration, validate that configuration instead of guessing from URL syntax.

## Acceptance criteria

```text
[ ] production remote endpoint without TLS configuration is rejected
[ ] localhost development collector can be explicitly allowed in development
[ ] staging behavior is explicit
[ ] tests cover URL and host:port forms
```

---

# F-005 — Service name/version permit control characters

**Severity:** P3 / Low  
**Category:** Structured logging, telemetry hygiene  
**Phase:** Phase 2

## Evidence

Service name and service version are bounded by length but are not subjected to the control-character rejection applied to database identity fields.

## Risk

These fields are likely to enter:

- logs;
- OpenTelemetry resource attributes;
- metrics metadata;
- diagnostic output.

Control characters can damage log readability, complicate downstream parsing, or create log-forging ambiguity in poorly configured sinks.

## Remediation

Use a common bounded-structured-string validator for configuration fields that reach logging or telemetry.

At minimum reject:

```text
NUL
CR
LF
TAB where inappropriate
other ASCII control characters
```

Prefer a documented allowed grammar for:

- service name;
- service version.

A version field should normally be compatible with the repository's chosen release identity format.

---

# F-006 — `traceparent` parser validates shape but not full W3C semantics

**Severity:** P2 / Medium  
**Category:** Protocol correctness, observability integrity  
**Phase:** Phase 2

## Evidence

The current parser checks:

- four hyphen-separated fields;
- expected field lengths;
- hexadecimal characters.

That is only syntactic validation.

The W3C trace-context protocol also has semantic constraints, including restrictions around:

- reserved version values;
- zero trace identifiers;
- zero parent identifiers.

## Why this matters

Trace context is explicitly treated as **untrusted diagnostic data**.

That does not make invalid protocol data harmless. Invalid trace context can:

- break correlation;
- cause interoperability problems;
- pollute distributed traces;
- undermine anomaly detection.

## Remediation

Implement semantic validation:

```text
version:
    exactly two hex digits
    reject reserved invalid value

trace-id:
    exactly 32 hex chars
    reject all-zero

parent-id:
    exactly 16 hex chars
    reject all-zero

flags:
    exactly two hex chars
```

Preserve unknown future versions only if the W3C protocol semantics permit doing so; otherwise reject at the boundary.

## Test matrix

```text
valid 00 traceparent
wrong field count
wrong field length
non-hex
all-zero trace-id
all-zero parent-id
reserved version
upper/lowercase hex
flags variants
```

---

# F-007 — Server request IDs are process-local sequential values

**Severity:** P2 / Medium  
**Category:** Observability, distributed correctness  
**Phase:** Phase 2

## Evidence

Server request IDs are generated from a process-local atomic counter.

The current shape is effectively:

```text
req-0000000000000001
req-0000000000000002
...
```

The counter is safe for concurrent threads inside one process, but it is not globally unique.

## Problems

### Multi-instance collision

Two application instances can emit:

```text
req-0000000000000001
```

at the same time.

### Information leakage

Sequential IDs disclose relative request volume and execution ordering.

### Long-term wraparound

`AtomicU64` eventually wraps.

The practical probability of actual wraparound is negligible, but a foundation layer should not encode avoidable semantics when a robust identifier is inexpensive.

## Remediation

Use a globally unique, non-predictable identifier for server-generated request correlation.

Suitable families:

- UUID v4;
- ULID;
- another cryptographically random 128-bit identifier.

The chosen format must remain:

- bounded;
- log-safe;
- HTTP-header-safe;
- cheap to generate.

Keep client-supplied correlation IDs only as external diagnostic input. Do not confuse them with authoritative server-generated identifiers.

Recommended model:

```text
server_request_id = globally unique

client_correlation_id = optional external correlation value
```

Both can coexist.

---

# F-008 — Operational event emission accepts an unbounded operation string

**Severity:** P1 / High  
**Category:** Logging security, telemetry cardinality, resource exhaustion  
**Phase:** Phase 2

## Evidence

The public event emission function accepts:

```text
operation: &str
```

and logs it directly.

This conflicts with the stated design that events are:

- registered;
- field-selected;
- bounded;
- controlled.

## Why this matters

Any request path that eventually passes attacker-controlled data into `operation` can produce:

- large log records;
- high-cardinality telemetry;
- PII leakage;
- control-character pollution;
- log storage pressure;
- expensive downstream indexing.

A field being “structured” does not automatically make it safe.

## Remediation

Replace raw operation strings with a bounded operation type.

Preferred approach:

```rust
pub struct OperationName(SmallString);
```

or an enum for known platform operations.

At minimum:

- maximum byte length;
- allowed grammar;
- no control characters;
- no secrets;
- no arbitrary request/body content.

If operations are metrics labels, use an explicit registry:

```text
auth.login
auth.refresh
auth.logout
health.ready
db.acquire
...
```

Unknown operations should be rejected or mapped to a bounded `unknown` bucket.

## Acceptance criteria

```text
[ ] attacker-sized operation value is rejected
[ ] CR/LF rejected
[ ] arbitrary JSON rejected
[ ] secrets are not accepted
[ ] unknown operation is bounded
[ ] metric labels use registry values only
```

---

# F-009 — Event schema registration is ignored during emission

**Severity:** P2 / Medium  
**Category:** Maintainability, observability contract integrity  
**Phase:** Phase 2

## Evidence

The registry stores:

```text
name
schema_version
owner
```

but the emission function outputs a hard-coded schema version of `1`.

## Why this matters

The registry becomes partly decorative.

If an event later changes to:

```text
schema_version = 2
```

the emitted payload still claims:

```text
event_schema_version = 1
```

That is a data-contract bug.

## Remediation

Resolve the event definition once:

```text
definition = event(name)
```

and emit:

```text
definition.schema_version
```

Do not duplicate schema-version constants across call sites.

## Additional improvement

Expose an immutable event-definition reference to the emitter rather than accepting free-form event names plus separately resolved metadata.

---

# F-010 — Telemetry priority model does not protect high-priority records under saturation

**Severity:** P1 / High  
**Category:** Security evidence, business evidence, resilience  
**Phase:** Phase 2

## Evidence

The telemetry layer defines:

```text
P0Security
P1Business
P2Normal
P3Debug
```

but the buffer currently only checks:

```text
queued < capacity
```

When full, the incoming event is dropped regardless of priority.

## Why this is a serious defect

The architectural intent is clearly:

```text
low-value diagnostic telemetry
    ↓
shed first

security/business evidence
    ↓
preserve longer
```

The current implementation does not provide that guarantee.

At saturation:

```text
P3 debug
```

and:

```text
P0 security
```

have identical admission behavior.

That is not priority-aware telemetry.

## Remediation

Implement explicit capacity reservation.

For example:

```text
capacity = total

reserved_security_capacity
reserved_business_capacity
normal_capacity
debug_capacity
```

Or implement weighted eviction:

```text
when full:
    drop P3 first
    then P2
    then P1
    never silently discard P0 unless the system is in an explicit catastrophic state
```

For P0 security evidence, a durable fallback path may be necessary.

Do not make telemetry failure block business transactions unless the architecture explicitly requires it.

The correct principle is:

```text
telemetry must not become business authority
+
security evidence must not be silently discarded
```

---

# F-011 — TelemetryBuffer is currently a counter, not an actual queue

**Severity:** P2 / Medium  
**Category:** Implementation completeness  
**Phase:** Phase 2

## Evidence

The current buffer tracks:

- capacity;
- queue count;
- dropped counts.

It does not contain telemetry records.

The export operation decrements a counter.

## Why this matters

As a contract-level primitive this can be useful.

As an implementation-level telemetry queue it cannot:

- retain event payloads;
- maintain ordering;
- associate priority with a specific record;
- retry a failed export;
- identify which record was exported;
- preserve P0/P1 records.

Therefore it should not be presented as if it were a complete telemetry queue.

## Remediation choice

One of two approaches is acceptable.

### Option A — Contract-only

Rename/annotate the type to make it explicit that it is a capacity/state model:

```text
TelemetryCapacityModel
```

and defer actual queue implementation to a later integration layer.

### Option B — Implement queue semantics

Use a bounded asynchronous queue containing:

```text
TelemetryRecord {
    event
    priority
    timestamp
    correlation_id
    payload
}
```

with explicit:

- admission;
- shedding;
- export;
- retry;
- shutdown drain;
- failure counters.

Given the Phase 2 Definition of Done, Option B is the stronger interpretation if Phase 2 claims exporter behavior is implemented.

---

# F-012 — Secret provider fail-closed behavior is not actually tested

**Severity:** P1 / High  
**Category:** Security testing  
**Phase:** Phase 2

## Evidence

The development environment secret provider contains a production guard.

However, the existing test checks the internal `production` flag rather than executing the asynchronous `get()` operation and proving:

```text
production provider
    ↓
get()
    ↓
LocalProviderForbidden
```

## Why this matters

A security invariant that is tested indirectly is not equivalent to executable evidence of the actual boundary.

A future refactor could move the guard below environment access or alter `get()` semantics without breaking the current test.

## Remediation

Use a minimal asynchronous test runtime and execute the actual method.

Required tests:

```text
production provider rejects get()
development provider resolves configured env value
missing env value returns Missing
empty env value returns Missing
class mapping is correct
secret reference does not leak value
```

Also assert that forbidden production access does not read the environment first.

---

# F-013 — Development secret provider ignores the SecretRef path

**Severity:** P2 / Medium  
**Category:** Maintainability, semantic ambiguity  
**Phase:** Phase 2

## Evidence

The provider receives a full `SecretRef`, but the development provider selects an environment variable using only `SecretClass`.

Therefore:

```text
Database -> ref A
Database -> ref B
```

both map to the same development environment variable.

## Why this matters

That may be intentional for local development, but the semantics are currently implicit.

A developer could believe that changing:

```text
SecretRef("dev/db-primary")
```

to:

```text
SecretRef("dev/db-test")
```

changes the selected credential, when it does not.

That ambiguity is dangerous during integration testing.

## Remediation

Document and encode one explicit model.

### Model 1 — class-only development secrets

State formally:

```text
development provider ignores path by design
```

and introduce separate test mappings when multiple credentials are required.

### Model 2 — reference-aware environment provider

Derive environment variables from the reference:

```text
SITOLO_SECRET_DATABASE_DEV_DB_PRIMARY
```

The provider must sanitize the transformation safely.

Do not implement a secret-reference grammar that creates shell/environment injection risk.

---

# F-014 — Missing-secret errors expose secret-reference paths

**Severity:** P2 / Medium  
**Category:** Information disclosure  
**Phase:** Phase 2

## Evidence

The secret error contains the reference path.

The path is not the secret value, which is good, but secret-reference namespaces can still expose:

- environment names;
- project/service naming;
- infrastructure structure;
- provider naming conventions.

## Remediation

Separate operator diagnostics from externally visible errors.

Preferred internal representation:

```text
SecretError {
    class,
    stable_code,
    redacted_reference_identity
}
```

The raw reference path should not be emitted into public API errors.

It may be retained in security-restricted diagnostics only when required and appropriately classified.

## Acceptance criteria

```text
[ ] no secret value in error
[ ] no credential-bearing connection string
[ ] no secret path in client-facing Problem Details
[ ] telemetry contains only classified secret identity
[ ] logs use approved redaction policy
```

---

# F-015 — Release workflow bypasses the canonical verification/security chain

**Severity:** P0 / Critical  
**Category:** Supply-chain security, release integrity  
**Phase:** Phase 1  
**Status:** Blocking

## Evidence

The artifact workflow performs:

```text
cargo build --workspace --release --locked
cargo test --workspace --all-targets --all-features --locked
```

and then packages the release.

The Phase 1 contract is broader:

```text
format
→ lint
→ build
→ tests
→ PostgreSQL integration
→ security verification
→ immutable artifact
→ SBOM
→ provenance
→ verification
→ promotion
```

The release artifact job does not itself execute the complete canonical verification path before packaging.

## Why this matters

There are now effectively two build definitions:

### Normal CI

```text
scripts/ci/verify
```

### Release artifact

```text
cargo build
cargo test
package
```

A tag could therefore produce an artifact using a path that has not executed every required policy control.

The fact that the commit may have passed normal CI is not sufficient because the release workflow itself is the trust boundary that turns source into an artifact.

## Remediation

Make the release artifact job invoke the canonical verification entrypoint before packaging.

Required:

```text
./scripts/ci/verify
```

Then add any release-only controls:

```text
cargo build --workspace --release --locked
SBOM
provenance
artifact digest
```

The correct flow is:

```text
SOURCE
  ↓
CANONICAL VERIFY
  ↓
RELEASE BUILD
  ↓
PACKAGE
  ↓
DIGEST
  ↓
SBOM
  ↓
ATTEST
  ↓
RELEASE
  ↓
PROMOTION
```

Do not duplicate the canonical checks manually if the repository already has a single verification script.

## Acceptance criteria

```text
[ ] release build cannot bypass scripts/ci/verify
[ ] verification failure blocks package creation
[ ] SBOM is generated only after successful verification
[ ] attestation is generated only after package digest exists
[ ] exact verified artifact is promoted
```

---

# F-016 — Artifact workflow grants overly broad write permissions

**Severity:** P1 / High  
**Category:** CI security  
**Phase:** Phase 1

## Evidence

The artifact workflow grants workflow-level permissions including:

```text
contents: write
id-token: write
attestations: write
```

even though `workflow_dispatch` is also enabled.

## Why this matters

Least privilege should apply to CI infrastructure exactly as it applies to runtime services.

The broader the token authority:

```text
workflow compromise
    ↓
repository write capability
    ↓
release manipulation / source modification
```

A manually invoked release-build workflow should not automatically have repository write authority unless the specific job requires it.

## Remediation

Split the workflow into jobs with minimal permissions.

Recommended:

```text
verify_build:
    contents: read

attest:
    contents: read
    id-token: write
    attestations: write

publish_release:
    contents: write
```

The release-publication job should:

- run only for trusted tag events;
- require successful verification job completion;
- use the minimum necessary write authority.

Avoid granting `contents: write` at workflow scope.

---

# F-017 — Artifact identity uses two commit-derived variables

**Severity:** P2 / Medium  
**Category:** Release logic correctness  
**Phase:** Phase 1

## Evidence

The package filename is derived from:

```text
git rev-parse HEAD
```

while later steps address the package using:

```text
github.sha
```

These are expected to match in normal tag execution, but the relationship is not enforced.

## Why this matters

Future changes to checkout behavior, merge workflows or trigger contexts can produce mismatches.

Then:

```text
package created as A
SBOM expects B
attestation targets B
```

and the workflow fails or, worse, if naming is changed inconsistently, verification binds the wrong artifact.

## Remediation

Compute once:

```text
SOURCE_SHA
ARTIFACT_PATH
```

and export them to subsequent steps.

Every later step must consume the exact values.

Do not recompute artifact identity independently.

---

# F-018 — PostgreSQL integration is an explicit Phase 1 exception, but exception enforcement is weak

**Severity:** P1 / High  
**Category:** CI/testing governance  
**Phase:** Phase 1

## Evidence

The integration workflow explicitly records that PostgreSQL integration testing is deferred by an approved Phase 1 scope exception.

This is consistent with the current staged implementation in which SQLx/schema/RLS are deferred to later phases.

The problem is not the existence of an exception.

The problem is that a textual echo line does not by itself prove:

- which ADR approved it;
- who owns it;
- what expiry condition exists;
- which phase closes it;
- which protected status check prevents accidental release without the exception being revisited.

## Remediation

Represent the exception as an explicit governance object.

At minimum:

```text
exception ID
owner
reason
approved by
date
expiry/review date
replacement phase
required evidence
release restriction
```

The exception should be referenced in the testing/implementation documentation.

When Phase 5 starts, this exception must be automatically considered expired.

## Acceptance criteria

```text
[ ] exception has owner
[ ] exception has documented scope
[ ] exception has review/expiry condition
[ ] CI identifies the exception
[ ] later phase removes the exception
[ ] production certification refuses unresolved temporary exceptions
```

---

# F-019 — Canonical Phase 1 repository topology is only partially realized

**Severity:** P2 / Medium  
**Category:** Repository architecture  
**Phase:** Phase 1

## Evidence

The Phase 1 specification describes a canonical tree including:

```text
apps/mobile
apps/desktop
migrations
config
tests
fuzz
infra
```

The current repository is narrower and currently centers on:

```text
apps/api
apps/worker
crates/*
docs
scripts/ci
```

## Important distinction

Not every missing directory is a defect.

Phase 1 explicitly says empty structure should not be created merely to satisfy a diagram, and future product phases own many directories.

The actual defect is **ambiguity**.

The repository currently does not make every canonical omission obviously intentional.

## Remediation

For every omitted canonical component, choose one:

### Accepted future structure

Record:

```text
Not created until Phase N
Owner: Phase N
Reason: no current executable responsibility
```

### Required foundation

Create the directory now if the directory already has Phase 1 responsibilities.

Do not create empty placeholder modules solely for aesthetics.

---

# F-020 — No single executable startup contract proves Phase 2 readiness

**Severity:** P0 / Critical  
**Category:** Runtime correctness  
**Phase:** Phase 2  
**Status:** Blocking

This finding is closely related to F-001 but is evaluated separately because it is a lifecycle/invariant problem rather than merely an empty binary.

The Phase 2 system needs a deterministic startup contract:

```text
parse
→ validate
→ construct security context
→ initialize observability
→ resolve required startup secrets
→ construct persistence intent
→ validate all required boundaries
→ bind HTTP server
→ readiness
```

The current repository exposes many of these pieces independently but does not provide evidence that the sequence exists as one coherent transaction-like startup process.

## Why this matters

Ordering is security-sensitive.

Incorrect:

```text
start listener
↓
initialize telemetry
↓
discover invalid config
```

Correct:

```text
validate trust-sensitive startup state
↓
only then accept traffic
```

Likewise:

```text
readiness = true
```

must never occur before required dependencies are initialized.

## Remediation

Define a `StartupContext` or equivalent immutable bootstrap result.

The construction should be:

```text
StartupContext::build()
```

and should fail as a complete unit.

The HTTP listener must not start until the context exists.

---

# F-021 — Phase 3 must not create duplicate platform infrastructure

**Severity:** P1 / High  
**Category:** Architecture governance  
**Phase:** Phase 3 readiness

Phase 3 is allowed to implement:

- identity;
- sessions;
- MFA;
- device identity;
- recovery;
- authentication assurance.

Phase 3 is **not** allowed to invent:

- another configuration system;
- another secret provider;
- another logger;
- another request-ID model;
- another error envelope;
- another telemetry registry;
- another startup model.

The Phase 3 specification explicitly requires consuming the common Phase 2 runtime facilities.

## Required gate

Phase 3 feature branches must depend on:

```text
sitolo-config
sitolo-security
sitolo-observability
sitolo-persistence boundary
```

through the shared application composition layer.

If a Phase 3 feature requires a missing primitive, remediate that primitive in Phase 2 rather than introducing an authentication-specific copy.

---

# F-022 — CI evidence cannot prove repository-admin settings

**Severity:** Evidence Gap  
**Category:** Governance  
**Phase:** Phase 0/1

The repository contains policy workflows and required-check logic, but repository code alone cannot prove:

- branch protection;
- required status checks actually configured in GitHub;
- who may bypass required reviews;
- whether force pushes are disabled;
- production environment approval configuration;
- environment secret restrictions;
- tag creation permissions.

These are repository administration controls, not source files.

## Remediation

Create a release-governance evidence record containing:

```text
default branch
branch protection mode
required checks
required review count
force-push setting
deletion setting
admin enforcement
production environment approvers
deployment branch restrictions
tag protection
workflow permission policy
```

Export evidence without exposing secrets.

---

# 7. Cross-Finding Architectural Problems

The individual findings reveal broader systemic issues.

## 7.1 The foundation is currently library-complete but application-incomplete

The repository has many correct pieces:

```text
config
security
observability
persistence
auth
events
audit
```

but the executable composition layer is missing.

That is a classic early-platform failure mode:

> **The architecture exists in crates but not in the running system.**

The remediation should prioritize vertical integration over adding additional isolated primitives.

---

# 8. Security Review

## 8.1 Authentication boundary readiness

Phase 3 will require:

```text
identity provider
↓
cryptographic validation
↓
principal
↓
session
↓
device
↓
assurance
```

The current Phase 0–2 foundation must already guarantee:

- secrets can be loaded safely;
- authentication errors have a common error boundary;
- authentication telemetry cannot leak tokens;
- correlation IDs are safe;
- startup cannot silently use development secret behavior in production.

F-002, F-003, F-008 and F-012 directly affect Phase 3 security.

---

## 8.2 Authorization boundary readiness

Although full authorization belongs later, the Phase 0–2 runtime must not make tenant/security identity impossible to carry safely.

The correct future path is:

```text
transport
↓
authenticated principal
↓
security/session context
↓
tenant/scope
↓
authorization
```

Do not put tenant authorization into:

- telemetry;
- configuration;
- environment variables;
- raw client IDs;
- request IDs.

---

## 8.3 Secrets

The current secret design has several good properties:

- raw secret values are excluded from AppConfig;
- production local provider use is explicitly rejected;
- secret provider is abstracted behind an interface;
- redaction exists as defense in depth.

The defects are mainly semantic/test gaps:

- environment provenance;
- reference meaning;
- asynchronous test coverage;
- diagnostic exposure.

These are exactly the kinds of gaps that become difficult to fix once authentication and integrations start depending on them.

---

## 8.4 Logging

Structured logging is the correct direction.

The major danger is not the structured formatter itself.

The danger is uncontrolled **field semantics**.

Safe:

```text
operation = "auth.login"
```

Unsafe:

```text
operation = request.body
```

The emitter must enforce the former.

---

## 8.5 Telemetry

Telemetry must remain:

```text
diagnostic evidence
```

and never:

```text
business authority
```

The queue must therefore tolerate exporter outages without changing business semantics.

At the same time:

```text
security evidence
business evidence
```

must receive stronger retention than:

```text
debug noise
```

That is why F-010 is a real security finding rather than a performance optimization.

---

# 9. Maintainability Review

## 9.1 Good existing direction

The repository already uses separate crates for:

- domain;
- application;
- configuration;
- security;
- observability;
- persistence;
- integrations;
- auth.

This is consistent with the modular-monolith-first architecture.

## 9.2 Maintainability risks

### Risk A — duplicated startup logic

F-001/F-020.

### Risk B — registry data not actually authoritative

F-009.

### Risk C — type-safe boundaries weakened by raw strings

F-008.

### Risk D — configuration semantics hidden inside defaults

F-002.

### Risk E — “contract stubs” that look implemented

F-011.

The last risk is particularly important.

A repository can look highly engineered because it contains many interfaces and types while the actual runtime path remains unimplemented.

The audit standard must therefore be:

```text
contract exists
+
implementation exists
+
runtime consumes it
+
tests execute it
+
failure path is observed
```

---

# 10. Performance and Resource Review

Phase 0–2 does not require premature optimization, but it does require bounded behavior.

## 10.1 Positive controls

The current configuration includes hard ceilings for:

- request body size;
- header timeout;
- keepalive;
- DB acquisition;
- OTEL export timeout;
- telemetry queue;
- pool sizes.

This is correct.

## 10.2 Remaining resource risks

### Unbounded operation field

F-008.

### Telemetry queue semantics

F-010/F-011.

### Release-time verification duplication

F-015.

### Configuration string safety

F-005.

---

# 11. Concurrency Review

Phase 2 itself has limited business-state concurrency, but its foundational components still need deterministic concurrent behavior.

## Required tests

### Request IDs

Many concurrent request-ID generations must be:

```text
unique within process
```

and globally safe by design.

### Telemetry

Concurrent producers must never exceed capacity.

### Shutdown

Repeated shutdown calls must be safe.

### Secrets

Parallel reads must not mutate shared state unsafely.

### Startup

Only one application runtime should be assembled per process.

---

# 12. Error Handling Review

The Phase 0–2 error model correctly distinguishes:

```text
internal cause
↓
semantic error
↓
safe external representation
```

The implementation must continue to prevent:

- stack traces;
- SQL strings;
- secret references;
- provider credentials;
- filesystem paths;
- raw external responses

from crossing the API boundary.

## Required property

An internal error can be highly detailed internally while the public error remains stable:

```json
{
  "type": "...",
  "title": "...",
  "status": 503,
  "detail": "...",
  "instance": "...",
  "request_id": "..."
}
```

The exact shape remains governed by `api_contract.md`.

---

# 13. Business Logic Review

There is intentionally very little implemented business logic in the Phase 0–2 code.

That is correct.

Do **not** treat absence of:

- inventory;
- sales;
- payments;
- reporting

as findings in this audit.

The business-logic risk is instead architectural:

> **The foundation must not make future business invariants difficult to enforce.**

Examples:

### Financial operations

Future financial commands require:

```text
transaction
+
idempotency
+
append-oriented facts
+
audit
```

The persistence boundary must therefore remain capable of expressing unknown outcomes.

### Inventory

Future inventory must be ledger-backed.

Phase 2 must not introduce generic CRUD repository patterns that encourage silent overwrite semantics.

### Offline operation

Clients are continuity surfaces, not permanent authorities.

Phase 2 infrastructure must not create a request path that trusts local client state as authoritative.

---

# 14. Required Remediation Order

Do not fix findings randomly.

The dependency order is:

```text
1. F-001  API bootstrap
2. F-020  startup contract
3. F-002  environment-safe configuration
4. F-003  secret-reference/provider semantics
5. F-015  release verification gate
6. F-016  CI permissions
7. F-018  integration-test exception governance
8. F-010  priority-aware telemetry
9. F-008  bounded operation semantics
10. F-012 secret-provider executable tests
11. F-006 traceparent semantics
12. F-007 global request IDs
13. F-009 event schema authority
14. F-014 secret-reference error redaction
15. F-004 telemetry endpoint security
16. F-005 structured-string validation
17. F-011 telemetry queue completeness
18. F-017 artifact identity cleanup
19. F-019 topology governance
20. F-022 repository-admin evidence
```

---

# 15. Implementation Rules

## Rule 1 — Do not create parallel infrastructure

There must be exactly one:

```text
configuration system
secret boundary
error mapping boundary
logging bootstrap
telemetry registry
request ID model
startup lifecycle
```

---

## Rule 2 — Do not turn contracts into fake completeness

An interface with no executable implementation may be valid when explicitly documented as a future boundary.

It must not be marked complete merely because:

```text
the crate compiles
```

---

## Rule 3 — Security tests must call the real boundary

Do not test:

```text
production_flag == true
```

when the actual invariant is:

```text
provider.get()
→ LocalProviderForbidden
```

Test the second.

---

## Rule 4 — Prefer types over strings

Raw:

```text
&str
```

is acceptable at a transport boundary.

It is weaker inside security-critical operational infrastructure.

Use bounded domain types for:

- operation names;
- event names;
- secret references;
- request identifiers;
- service identifiers.

---

## Rule 5 — Configuration must be monotonic with security

An operator may tune within policy.

An operator may not turn configuration into a new security authority.

Examples:

```text
timeout:
    configurable within ceiling

body size:
    configurable within ceiling

local secret provider:
    forbidden in production

authorization:
    never configurable through environment
```

---

# 16. Required Test Matrix

## 16.1 Configuration

```text
[ ] development defaults valid
[ ] staging defaults valid
[ ] production defaults valid
[ ] production cannot inherit unsafe development-only values
[ ] unknown environment rejected
[ ] unknown config key rejected
[ ] malformed integer rejected
[ ] oversized string rejected
[ ] control character rejected
[ ] cross-field pool bounds rejected
[ ] invalid trace ratio rejected
[ ] invalid telemetry endpoint rejected
[ ] production verbose logging rejected
[ ] production local secret provider rejected
```

---

# 17. Secret Tests

```text
[ ] development provider works
[ ] production provider is forbidden
[ ] missing secret returns stable error
[ ] empty secret returns stable error
[ ] wrong secret class is rejected
[ ] reference/environment mismatch rejected
[ ] provider failure is fail-closed
[ ] secret values are absent from error strings
[ ] secret values are absent from logs
[ ] secret values are absent from telemetry
```

---

# 18. Redaction Tests

At minimum:

```text
[ ] exact secret match
[ ] overlapping secret match
[ ] multiple secrets
[ ] UTF-8 input
[ ] empty secret entry
[ ] large input within limit
[ ] over-limit input
[ ] output over-limit after replacement
[ ] field-name redaction
[ ] quoted values
[ ] malformed serialization
[ ] authentication tokens
[ ] refresh tokens
[ ] authorization headers
[ ] code_verifier
```

The existing redaction implementation correctly avoids empty-pattern loops. Preserve that property.

---

# 19. Observability Tests

## Request IDs

```text
[ ] safe server ID
[ ] safe client ID
[ ] invalid client ID rejected
[ ] newline rejected
[ ] oversize rejected
[ ] concurrent generation unique
[ ] multi-instance-safe format
```

## Trace context

```text
[ ] valid traceparent
[ ] malformed traceparent
[ ] zero trace ID
[ ] zero parent ID
[ ] reserved version
[ ] invalid hex
[ ] invalid length
```

## Event emission

```text
[ ] unknown event rejected
[ ] known event emitted
[ ] registry schema version emitted
[ ] bounded operation accepted
[ ] oversized operation rejected
[ ] control characters rejected
[ ] unregistered operation rejected when required
```

---

# 20. Telemetry Saturation Tests

The following must be deterministic:

```text
Given capacity = N

push N normal events
→ accepted

push debug event when full
→ dropped

push security event when full
→ preserved or routed to reserved capacity

export one
→ exact capacity/accounting transition

export failure
→ queue state consistent

shutdown
→ bounded drain behavior
```

The test must verify **priority semantics**, not only queue count.

---

# 21. Startup Integration Tests

The application bootstrap needs integration tests that use the real composition path.

## Successful startup

```text
valid config
+
valid secret boundary
+
telemetry initialized
+
database intent valid
→ startup succeeds
→ readiness available
```

## Invalid production config

```text
production
+
local secret provider
→ startup fails
→ listener does not become ready
```

## Secret failure

```text
required secret unavailable
→ startup fails
→ no request serving
```

## Telemetry failure

If telemetry export is non-critical by contract:

```text
exporter unavailable
→ service remains available
→ bounded degradation state
→ no unbounded retry
```

If telemetry is security-critical for a specific event class:

```text
security evidence cannot be recorded
→ explicit bounded policy
```

The behavior must be defined rather than inferred.

---

# 22. Release Pipeline Acceptance

A release must satisfy:

```text
[ ] canonical verification passed
[ ] dependency policy passed
[ ] advisory scan passed
[ ] secret scan passed
[ ] CodeQL/security checks passed
[ ] tests passed
[ ] locked dependency graph verified
[ ] toolchain verified
[ ] artifact built from exact verified source
[ ] artifact digest recorded
[ ] SBOM generated
[ ] provenance attested
[ ] release asset matches digest
[ ] promotion verifies exact artifact
```

The artifact workflow must not independently redefine correctness.

---

# 23. CI Security Review

## Action pinning

The current workflows already use immutable SHA references for major third-party actions. Preserve that behavior.

Do not replace them with floating:

```text
@main
@master
@vX
```

unless the security policy explicitly changes.

## Permissions

Apply:

```text
read by default
write only where required
```

at the narrowest level.

## Untrusted pull requests

Do not expose:

- production credentials;
- deployment tokens;
- privileged write tokens

to untrusted fork execution.

---

# 24. Database Boundary Review

The current Phase 2 persistence runtime intentionally stops before:

- SQLx;
- schema;
- migrations;
- RLS;
- tenant context.

That is correct according to the phase dependency model.

The important requirement is that Phase 3/4/5 consume the boundary rather than bypass it.

The current persistence contract should remain:

```text
Application
    ↓
semantic persistence port
    ↓
opaque capability
    ↓
future PostgreSQL adapter
```

Do not expose:

```text
raw Pool<Postgres>
```

to domain code.

Do not move:

```text
PostgreSQL-specific SQL
```

into `sitolo-domain`.

---

# 25. What Is Already Good

The audit is not a claim that the repository is fundamentally unsound.

Several decisions are strong and should be preserved.

## 25.1 Rust safety boundary

`#![forbid(unsafe_code)]` at the API/observability/security boundary is an appropriate default.

## 25.2 Typed configuration

Using a typed `AppConfig` instead of passing environment variables around request handlers is correct.

## 25.3 Hard ceilings

Configuration ceilings are treated as architecture policy rather than tuning hints.

That is exactly the right model.

## 25.4 Secret value separation

`AppConfig` contains references rather than raw credentials.

Preserve this.

## 25.5 Structured operational registries

A closed event/metric registry is safer than allowing arbitrary production telemetry.

The current implementation simply needs to enforce the model all the way through emission.

## 25.6 Opaque persistence capability

Keeping credentials and raw database pools out of the consumer surface is correct.

## 25.7 CI action pinning

Pinned action SHAs materially reduce mutable-action supply-chain exposure.

Do not regress this.

---

# 26. Things Not To “Fix”

Do not make changes merely because something is absent from the current repository if it belongs to a later phase.

Do **not** prematurely add:

- tenant RLS;
- full authorization policy engine;
- POS transaction engine;
- inventory ledger;
- payment adapters;
- MRA EIS;
- complete offline sync;
- support impersonation.

Also do not create giant generic abstractions merely because future phases exist.

The right approach is:

```text
stable narrow boundary
+
explicit ownership
+
testable contract
```

not:

```text
generic framework for every possible future feature
```

---

# 27. Phase 3 Entry Gate

Phase 3 should not be considered blocked because every later-phase concern is unfinished.

It is blocked only if Phase 2 infrastructure cannot safely be consumed.

Therefore the Phase 3 entry gate is:

```text
[ ] F-001 closed
[ ] F-020 closed
[ ] F-002 closed
[ ] F-003 closed
[ ] F-015 closed
[ ] F-016 closed
[ ] F-010 closed
[ ] F-008 closed
[ ] F-012 closed

[ ] common bootstrap exists
[ ] common error boundary exists
[ ] common request-ID boundary exists
[ ] common telemetry boundary exists
[ ] common secret boundary exists
[ ] common persistence boundary exists

[ ] Phase 3 does not duplicate Phase 2 infrastructure
```

The following may remain in progress during early Phase 3 implementation if they do not weaken the trust boundary:

```text
F-004
F-005
F-006
F-007
F-009
F-011
F-014
F-017
F-019
F-022
```

They must still be tracked.

---

# 28. Suggested Repository Changes

The following is the intended shape, not permission to invent a new architecture:

```text
apps/
  api/
    src/
      main.rs
      bootstrap.rs
      state.rs
      shutdown.rs

crates/
  sitolo-config/
  sitolo-security/
  sitolo-observability/
  sitolo-persistence/
  sitolo-api/
  sitolo-application/
  sitolo-audit/
  sitolo-events/
  ...

scripts/
  ci/
    verify
    check-workflow-policy
```

The goal is to create one composition root.

---

# 29. Definition of Done for This Audit

The audit is resolved only when evidence exists for the following:

## Runtime

```text
[ ] executable startup path exists
[ ] startup is deterministic
[ ] startup fails closed
[ ] readiness follows startup success
[ ] shutdown is bounded and graceful
```

## Configuration

```text
[ ] environment-aware defaults
[ ] production-safe provenance
[ ] cross-field validation
[ ] bounded strings
[ ] schema compatibility
```

## Secrets

```text
[ ] typed references
[ ] provider enforcement
[ ] production restriction
[ ] executable tests
[ ] redacted diagnostics
```

## Observability

```text
[ ] globally unique request IDs
[ ] semantic traceparent validation
[ ] bounded event fields
[ ] authoritative event schema version
[ ] priority-aware shedding
```

## CI/CD

```text
[ ] release uses canonical verification
[ ] least-privilege permissions
[ ] exact source/artifact identity
[ ] security checks are release-relevant
[ ] exceptions are governed
```

## Evidence

```text
[ ] test output retained
[ ] CI status recorded
[ ] configuration tests recorded
[ ] secret tests recorded
[ ] telemetry tests recorded
[ ] release evidence recorded
```

---

# 30. Final Engineering Position

The repository is architecturally ahead of its executable integration.

That is both the strength and the current risk.

The design already establishes:

```text
PostgreSQL = authoritative server state
clients = continuity surfaces
security = fail closed
tenant isolation = security boundary
external providers = explicit trust boundaries
financial history = append-oriented
offline = bounded authority
CI/CD = production security boundary
```

The current implementation must now make those rules **observable in executable behavior**.

The correct next move is not to add more features.

The correct move is to close the foundation gap:

```text
DOCUMENTED CONTRACT
        ↓
IMPLEMENTED PRIMITIVE
        ↓
APPLICATION BOOTSTRAP
        ↓
EXECUTABLE TEST
        ↓
CI ENFORCEMENT
        ↓
RELEASE EVIDENCE
```

Anything short of that is a documented intention, not a proven control.

Phase 3 should build identity/session/MFA/device functionality only on top of this common runtime path.

The project should not move toward production certification while F-001, F-015 and F-020 remain unresolved, because together they mean the system's documented platform controls are not yet demonstrated as one verifiable running service.

---

# Appendix A — Evidence Sources

The audit used the current public `main` repository and the existing Sitolo Phase 0–3 documentation set.

## Current implementation paths reviewed

```text
apps/api/src/main.rs
apps/api/Cargo.toml

crates/sitolo-config/src/
crates/sitolo-security/src/
crates/sitolo-observability/src/
crates/sitolo-persistence/src/

.github/workflows/
```

## Current workflow paths reviewed

```text
.github/workflows/policy.yml
.github/workflows/rust.yml
.github/workflows/integration.yml
.github/workflows/security.yml
.github/workflows/artifact.yml
.github/workflows/release.yml
```

## Governing documentation

```text
security_implementation_spec.md
threat_model.md
ci_enforcement.md
implementation_plan.md
phase1_repository_rust_workspace_ci_deep_implementation.md
phase2_config_secrets_logging_errors_telemetry_implementation.md
phase3_identity_sessions_mfa_device_identity_implementation.md
domain_model.md
database_design.md
api_contract.md
auth_authorization_spec.md
observability_spec.md
deployment_spec.md
testing_strategy.md
security_test_harness.md
```

---

# Appendix B — Contract Traceability

| Finding | Upstream contract impacted | Primary invariant |
|---|---|---|
| F-001 | Phase 2 implementation | Runtime must consume common platform contracts |
| F-002 | Phase 2 implementation | Production must not inherit unsafe development semantics |
| F-003 | Security implementation | Secret authority must be explicit |
| F-004 | Phase 2 telemetry | Remote telemetry must use protected transport |
| F-005 | Observability | Telemetry fields must be bounded and safe |
| F-006 | Observability | Untrusted trace context must be valid protocol data |
| F-007 | Observability | Correlation must work across instances |
| F-008 | Threat model + observability | Attacker-controlled diagnostics must be bounded |
| F-009 | Event registry | Registry is authoritative |
| F-010 | Threat model + observability | Security/business evidence must not be treated as disposable debug noise |
| F-011 | Phase 2 DoD | Telemetry degradation must be executable, not nominal |
| F-012 | Security implementation | Critical controls require executable evidence |
| F-013 | Secrets contract | Secret references must have unambiguous semantics |
| F-014 | Security logging | Diagnostics must not disclose protected material |
| F-015 | CI enforcement | Release artifacts must follow verified build chain |
| F-016 | CI enforcement | Workflow permissions are least privilege |
| F-017 | CI/release | Artifact identity must be cryptographically attributable |
| F-018 | Testing/CI | Exceptions must be explicit and bounded |
| F-019 | Phase 1 repository contract | Repository topology is an ownership model |
| F-020 | Phase 2 DoD | Runtime startup behavior must be executable |
| F-021 | Phase 3 contract | Phase 3 consumes Phase 2; it does not replace it |
| F-022 | CI governance | Repository-admin controls require evidence outside source |

---

# Appendix C — Non-Regression Rules

After remediation:

1. No secrets enter application configuration.
2. No secret values enter logs or traces.
3. No request/body data becomes a metric label.
4. No unbounded strings become telemetry dimensions.
5. No production startup can select the local secret provider.
6. No release artifact bypasses canonical verification.
7. No release workflow has unnecessary repository write authority.
8. No Phase 3 module creates a second configuration/logger/error/telemetry system.
9. No domain crate gains infrastructure dependencies merely to simplify Phase 3.
10. No PostgreSQL schema/RLS behavior is introduced early merely to satisfy a local test.
11. No telemetry state becomes business state.
12. No client-provided diagnostic identifier becomes an authorization identity.
13. No platform control is declared complete without executable evidence.

---

# Appendix D — Final Release Gate

The Phase 0–2 foundation is ready for normal Phase 3 progression only when:

```text
P0 FINDINGS
    ↓
CLOSED
    ↓
P1 FINDINGS
    ↓
CLOSED / EXPLICITLY WAIVED WITH EXPIRY
    ↓
P2 FINDINGS
    ↓
TESTED
    ↓
CANONICAL CI
    ↓
EVIDENCE
    ↓
PHASE 3 CONTINUES
```

The system should never transition from:

```text
"the code looks correct"
```

to:

```text
"the platform is trusted"
```

without the intermediate evidence.

---

**End of Phase 0–2 Implementation Audit & Remediation Contract**
