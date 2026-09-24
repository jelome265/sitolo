# SITOLO — PHASE 4 PART 8 AUDIT + OUTBOX IMPLEMENTATION CONTRACT

**Document:** docs/phase4_part8_audit_outbox_implementation_contract.md  
**Phase:** Phase 4 — Tenant / Organization / Branch / IAM  
**Part:** 8  
**PR:** PR-009  
**Scope:** Durable audit evidence + transactional outbox boundary  
**Repository:** jelome265/sitolo  
**Baseline:** main at faf469f5e6e0229189ed8b8cce70dd4ce4f7e117  
**Status:** Binding implementation contract  
**Implementation mode:** Direct repository implementation; no Jules delegation

---

# 0. EXECUTIVE DECISION

Phase 4 Part 8 establishes the durable security/audit evidence boundary and transactional outbox boundary required by the existing Sitolo architecture.

The canonical flow is:

~~~text
AUTHENTICATED PRINCIPAL
        |
        v
MEMBERSHIP / AUTHORIZATION / SCOPE
        |
        v
APPLICATION COMMAND
        |
        v
DOMAIN INVARIANTS
        |
        v
POSTGRES TRANSACTION
        |
        +-------------------+
        |                   |
        v                   v
 BUSINESS STATE        AUDIT RECORD
                            |
                            v
                       OUTBOX EVENT
        |                   |
        +---------+---------+
                  |
                COMMIT
                  |
                  v
          ASYNC WORKER / RELAY
                  |
                  v
        EXTERNAL / DERIVED EFFECT
~~~

The critical invariant is:

> A security-sensitive authoritative state transition and its mandatory audit/outbox evidence must share the same authoritative PostgreSQL transaction whenever the evidence describes that transition.

Therefore:

~~~text
business mutation succeeds
        +
audit evidence succeeds
        +
outbox intent succeeds
        |
        v
      COMMIT

OR

any mandatory component fails
        |
        v
     ROLLBACK
~~~

There must be no architecture of:

~~~text
COMMIT business state
        |
        +--> try audit
        +--> try publish
~~~

because process death between those operations can permanently create state without its required evidence or external intent.

The transactional outbox pattern exists specifically to make database mutation and durable event intent atomic without requiring a distributed two-phase commit. A relay can publish more than once, so downstream consumers must be idempotent.

Research:
https://microservices.io/patterns/data/transactional-outbox

---

# 1. CURRENT REPOSITORY INVESTIGATION

The repository was inspected before defining this contract.

## 1.1 Current main baseline

Current main is:

~~~text
faf469f5e6e0229189ed8b8cce70dd4ce4f7e117
~~~

This is the merge of Phase 4 PR-008, PostgreSQL RLS integration and negative tests.

PR-008 established the real PostgreSQL/RLS security boundary. Part 8 MUST build on that boundary rather than create another tenant-isolation mechanism.

## 1.2 Existing audit crate

Current audit implementation:

~~~text
crates/sitolo-audit/src/lib.rs
crates/sitolo-audit/src/event.rs
~~~

The audit crate already defines:

- AuthEventName;
- authentication event names;
- AuthenticationEvent;
- EventResult;
- AuditRequirement;
- AuditRecorder;
- AuditError;
- an in-memory sink for tests.

Important finding:

> The current audit model is primarily authentication-oriented. It is not yet the complete Phase 4 organizational/security audit model.

The existing AuthenticationEvent is intentionally secret-free. Part 8 MUST preserve that property.

## 1.3 Existing events crate

Current events boundary:

~~~text
crates/sitolo-events/src/lib.rs
~~~

It currently defines the architectural distinction between:

~~~text
domain events
integration events
outbox records
provider events
~~~

but does not yet contain the complete durable outbox implementation.

Part 8 must preserve these distinctions.

## 1.4 Existing worker

Current worker:

~~~text
apps/worker/src/main.rs
~~~

The worker is currently a Phase 1 scaffold and is not production functionality.

Part 8 defines the durable outbox contract the worker will eventually consume. It must not turn the worker into an unrelated general-purpose job platform.

## 1.5 Existing Phase 4 contracts

The existing Phase 4 specification already establishes:

- audit evidence;
- outbox processing;
- tenant-aware operation boundaries;
- organization and branch lifecycle;
- membership, role and scope changes;
- invitations;
- device and ownership security;
- concurrency;
- cache invalidation;
- operational evidence.

Earlier Phase 4 implementation contracts explicitly defer audit/outbox work to PR-009.

## 1.6 Existing architecture

The repository uses a Rust modular monolith with explicit boundaries:

~~~text
API
 |
APPLICATION
 |
DOMAIN
 |
PERSISTENCE

AUDIT
EVENTS
WORKER
INTEGRATIONS
~~~

Do not create a new service, broker, database, or messaging subsystem merely to implement Part 8.

---

# 2. REQUIRED DOCUMENT CROSS-CHECK

Before modifying source, reconcile all applicable governing documents:

~~~text
agent.md
docs/phase4_tenant_organization_branch_iam_implementation.md
docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md
docs/phase3_identity_sessions_mfa_device_identity_implementation.md
docs/phase2_config_secrets_logging_errors_telemetry_implementation.md
docs/phase1_repository_rust_workspace_ci_deep_implementation.md
docs/system_architecture_design.md
docs/security_architecture_design.md
docs/security_implementation_spec.md
docs/domain_model.md
docs/database_design.md
docs/api_contract.md
docs/auth_authorization_spec.md
docs/observability_spec.md
docs/testing_strategy.md
docs/deployment_spec.md
docs/ADR-001-025.md
~~~

Also inspect current source under:

~~~text
crates/sitolo-audit/**
crates/sitolo-events/**
crates/sitolo-application/**
crates/sitolo-persistence/**
crates/sitolo-tenancy/**
crates/sitolo-auth/**
crates/sitolo-authz/**
apps/api/**
apps/worker/**
~~~

The current source is implementation truth. Design documents define intended semantics. If documentation says something exists but source does not implement it, do not claim it exists.

---

# 3. EXACT PART 8 SCOPE

Part 8 owns:

1. audit event semantics;
2. Phase 4 security/IAM event catalogue;
3. durable audit persistence boundary;
4. transactional audit recording;
5. outbox event semantics;
6. durable outbox persistence;
7. transactionally coupled audit/outbox writes;
8. event identity;
9. event versioning;
10. aggregate ordering semantics;
11. retry state;
12. attempt accounting;
13. idempotency;
14. worker/relay consumption boundary;
15. tenant/scope metadata;
16. secret and PII minimization;
17. audit/outbox privilege boundaries;
18. retention metadata;
19. concurrency tests;
20. crash/rollback tests;
21. duplicate-delivery tests;
22. backlog/failure observability;
23. operational runbooks.

Part 8 does NOT own:

~~~text
generalized Phase 6 authorization engine
cache/versioning implementation
device binding implementation
ownership transfer implementation
support/admin separation
complete business-domain event catalogue
payment provider implementation
MRA integration
general worker/job platform
Kafka/RabbitMQ deployment
event sourcing
CQRS
microservice extraction
~~~

Do not implement future phases under the label of audit/outbox.

---

# 4. AUDIT AND OUTBOX ARE DIFFERENT

## 4.1 Audit

Audit answers:

> What happened, who or what initiated it, against which security/business scope, with what result, and when?

Audit is evidence for accountability, investigation, administrative review, compliance and reconstruction.

## 4.2 Outbox

Outbox answers:

> What durable event must be delivered to another subsystem after the authoritative transaction commits?

Outbox is durable asynchronous work/event intent.

It supports reliable publication, derived processing, notifications, integration and retry.

## 4.3 One action can create both

Example:

~~~text
OWNER changes branch manager
        |
        +--> membership state changes
        |
        +--> audit:
        |      iam.membership.role_changed
        |
        +--> outbox:
               iam.membership.role_changed.v1
~~~

The audit record is evidence.

The outbox record is a delivery mechanism.

They must not be treated as interchangeable.

---

# 5. TRANSACTIONAL INVARIANT

For an authoritative command:

~~~text
BEGIN

1. establish trusted principal/scope
2. establish transaction-local tenant context
3. validate command
4. enforce authorization
5. enforce domain invariants
6. mutate authoritative state
7. insert mandatory audit event
8. insert required outbox event

COMMIT
~~~

Only after commit may asynchronous processing occur.

If any mandatory operation fails:

~~~text
business state
audit state
outbox state
        |
        v
ROLLBACK
~~~

This MUST be tested.

Forbidden:

~~~rust
transaction.commit().await?;
audit.record(event).await?;
~~~

when the audit record is mandatory evidence of the mutation.

Also forbidden:

~~~text
BEGIN
business mutation
publish broker message
COMMIT
~~~

The broker is not the transaction authority.

Correct:

~~~text
BEGIN
business mutation
audit insert
outbox insert
COMMIT
        |
        v
relay publishes later
~~~

---

# 6. AUDIT EVENT MODEL

The audit event must be structured, bounded, typed and secret-free.

Minimum conceptual shape:

~~~text
AuditEvent
├── event_id
├── event_name
├── event_version
├── occurred_at
├── organization_id
├── branch_id
├── actor_subject_ref
├── actor_membership_ref
├── actor_device_ref
├── request_id
├── trace_id
├── target_type
├── target_ref
├── action
├── result
├── reason_class
├── assurance_level
├── source
└── metadata
~~~

Exact Rust/database representation may differ, but the semantics must remain explicit.

## 6.1 Stable identity

event_id must be globally unique.

It must not be derived from timestamps alone, organization names, usernames or client-provided sequential values.

## 6.2 Stable event names

Use lowercase dotted names.

Required Phase 4 examples:

~~~text
iam.organization.created
iam.organization.activated
iam.organization.suspended
iam.organization.resumed
iam.organization.closing_started
iam.organization.closed

iam.branch.created
iam.branch.activated
iam.branch.suspended
iam.branch.resumed
iam.branch.closing_started
iam.branch.closed

iam.membership.invited
iam.membership.invitation_accepted
iam.membership.activated
iam.membership.suspended
iam.membership.revoked

iam.role.assigned
iam.role.changed
iam.role.removed

iam.scope.granted
iam.scope.changed
iam.scope.revoked

iam.authorization.denied
iam.security.scope_violation

iam.device.bound
iam.device.unbound
iam.device.suspended
iam.device.revoked
~~~

This is a Phase 4 security/IAM catalogue, not the complete future Sitolo domain-event catalogue.

## 6.3 Version

Every durable event must carry an event schema version.

Example:

~~~text
event_name = iam.membership.role_changed
event_version = 1
~~~

Event schema version is distinct from application release version.

---

# 7. AUDIT RESULT SEMANTICS

At minimum:

~~~text
SUCCESS
FAILURE
~~~

Authorization failures can be important security evidence.

Example:

~~~text
iam.authorization.denied
result = FAILURE
reason_class = insufficient_scope
~~~

Do not persist protected-object details merely because access was denied.

---

# 8. SECRET AND SENSITIVE-DATA RULES

Never persist:

~~~text
passwords
password-reset secrets
OTP values
MFA secrets
recovery codes
access tokens
refresh tokens
API keys
client secrets
private keys
session cookies
invitation bearer tokens
raw Authorization headers
database passwords
credential-bearing connection strings
payment authentication secrets
cryptographic private material
~~~

Avoid unnecessary:

~~~text
full phone numbers
full email addresses
identity-document numbers
biometric material
raw request bodies
complete authentication headers
unbounded personal-data payloads
~~~

Use stable references, classifications, hashes or redacted forms where the existing project contract permits.

OWASP recommends logging security events while minimizing sensitive data and protecting audit records from unauthorized modification/deletion.

Reference:
https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html

---

# 9. AUTHENTICATION AUDIT COMPATIBILITY

The existing sitolo-audit crate already owns authentication events.

Part 8 MUST extend or compose that boundary rather than create a second authentication audit system.

Preserve:

~~~text
secret-free event construction
typed event names
typed outcome
async recorder boundary
mandatory vs diagnostic distinction
~~~

If a generic audit abstraction is introduced, existing Phase 3 authentication events must remain compatible without losing their existing safety properties.

Do not break Phase 3 merely to implement Phase 4.

---

# 10. MANDATORY VS DIAGNOSTIC

The existing audit contract distinguishes:

~~~text
Mandatory
Diagnostic
~~~

Preserve that distinction.

## Mandatory

Use for authoritative security/business transitions, including as applicable:

~~~text
organization creation
organization suspension
organization closure
membership activation
membership revocation
role assignment
role removal
scope grant/revocation
security-sensitive device state changes
required authorization/security violations
~~~

If mandatory audit persistence fails, the associated authoritative mutation MUST NOT commit unless an explicit higher-level contract provides another durable evidence mechanism.

## Diagnostic

Diagnostic telemetry must not normally block an unrelated transaction.

Do not confuse:

~~~text
application logs
audit evidence
metrics
traces
outbox events
~~~

They are different systems with different guarantees.

---

# 11. AUTHORIZATION DENIALS

Authorization failures are security events.

Where the security contract requires durable evidence, capture:

~~~text
event = iam.authorization.denied
result = FAILURE
actor reference
organization/branch scope if safely known
target class
requested action
reason class
request/trace reference
timestamp
~~~

Do not leak protected-object existence to the caller.

External response disclosure and internal audit evidence are separate concerns.

OWASP explicitly recommends logging authorization failures and controlling the information recorded.

Reference:
https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html

---

# 12. TENANT ISOLATION OF AUDIT DATA

Audit data is itself tenant-sensitive.

The organization/branch association must come from trusted server-side scope.

Forbidden:

~~~rust
audit.organization_id = request.organization_id;
~~~

when the request value is not independently authorized.

Correct conceptual flow:

~~~text
authenticated principal
        ↓
membership
        ↓
authorized scope
        ↓
transaction-local DB context
        ↓
mutation
        ↓
audit record
~~~

A tenant identifier is a selector, not proof of authority.

---

# 13. AUDIT ACCESS AND IMMUTABILITY

Audit records are privileged evidence.

Do not expose unrestricted audit reads.

Normal application roles MUST NOT be able to:

~~~text
UPDATE audit records
DELETE audit records
rewrite event identity
rewrite actor identity
rewrite tenant ownership
TRUNCATE audit storage
~~~

The design should prefer append-oriented semantics.

If retention requires deletion, it must be controlled by explicit retention/legal policy rather than ordinary CRUD.

The runtime role must not become the owner of security-sensitive audit tables merely because it writes them.

PostgreSQL documents that superusers, BYPASSRLS roles and table owners have special row-security behavior.

References:
https://www.postgresql.org/docs/18/ddl-rowsecurity.html
https://www.postgresql.org/docs/18/sql-createrole.html

---

# 14. OUTBOX RECORD MODEL

Minimum conceptual shape:

~~~text
OutboxEvent
├── event_id
├── aggregate_type
├── aggregate_id
├── event_name
├── event_version
├── organization_id
├── branch_id
├── occurred_at
├── payload
├── status
├── available_at
├── attempt_count
├── locked_at
├── published_at
├── last_error_class
├── deduplication_key
└── schema_version
~~~

Exact production schema must follow the repository's canonical database/migration authority.

---

# 15. OUTBOX STATE MACHINE

Minimum:

~~~text
PENDING
  |
  v
CLAIMED
  |
  +------ success ------> PUBLISHED
  |
  +------ retry --------> PENDING
  |
  +------ terminal -----> QUARANTINED
~~~

A worker lease/claim timeout must make work recoverable after worker death.

An event must never remain permanently stuck because a worker crashed.

---

# 16. DELIVERY SEMANTICS

Part 8 adopts:

> At-least-once delivery.

Do not promise exactly-once external delivery.

A relay can crash after publication and before recording successful publication:

~~~text
publish
  |
  X crash
  |
  v
database still says pending
  |
  v
event published again
~~~

Therefore consumers MUST be idempotent.

Reference:
https://microservices.io/patterns/data/transactional-outbox

---

# 17. IDEMPOTENCY

Every outbox event needs a stable identity usable for downstream deduplication.

Prefer event_id as the globally unique event identity.

Keep these concepts separate:

~~~text
command idempotency
event identity
delivery idempotency
~~~

Retrying an event must not create a new semantic event.

---

# 18. EVENT ORDERING

Do not promise global total order.

Events affecting the same aggregate must have deterministic ordering.

Example:

~~~text
membership created
      |
      v
role assigned
      |
      v
role changed
      |
      v
membership revoked
~~~

A practical implementation may use an aggregate-local sequence or another repository-approved ordering mechanism.

Wall-clock timestamps alone are not an ordering authority.

---

# 19. EVENT PAYLOAD CONTRACT

Outbox payloads MUST be:

- bounded;
- versioned;
- deterministic;
- serializable;
- secret-free;
- schema-defined;
- independent of Rust debug formatting.

Forbidden:

~~~rust
format!("{:?}", domain_object)
~~~

as the durable event contract.

Do not serialize complete database rows by default.

Publish the smallest stable contract needed by the consumer.

---

# 20. EVENT SCHEMA EVOLUTION

When an incompatible payload change occurs:

~~~text
event_version 1
       |
       v
event_version 2
~~~

Do not silently change version 1 semantics.

Already-persisted events must remain processable according to their original contract.

Consumers must handle unknown versions safely.

---

# 21. DOMAIN EVENT VS INTEGRATION EVENT VS OUTBOX

Keep the concepts separate:

~~~text
Domain Event
    |
    v
internal fact

Integration Event
    |
    v
stable asynchronous contract

Outbox Record
    |
    v
durable delivery envelope
~~~

Do not make database rows themselves the public integration contract.

---

# 22. NO EVENT-SOURCING CREEP

Part 8 is not event sourcing.

Authoritative state remains:

~~~text
PostgreSQL business state
~~~

Audit and outbox records describe/support state changes.

Do not reconstruct all business state from audit records.

Do not force every database mutation to become a domain event merely because audit exists.

---

# 23. WORKER / RELAY BOUNDARY

The worker eventually performs:

~~~text
claim bounded batch
       |
       v
validate event
       |
       v
deliver to destination
       |
       +---- success ----> mark published
       |
       +---- retryable --> release/requeue
       |
       +---- terminal --> quarantine
~~~

The worker MUST NOT:

- bypass authorization;
- become a second source of business truth;
- mutate business state merely to make delivery appear successful;
- silently delete failed evidence;
- retry forever;
- log secrets;
- use unbounded batches;
- hold database transactions across external network calls.

The worker consumes durable event intent; PostgreSQL remains authoritative.

---

# 24. CLAIMING AND CONCURRENCY

Multiple workers may process outbox events concurrently.

The claim mechanism must prevent uncontrolled simultaneous processing of the same event.

Behavioral contract:

~~~text
worker A claims event E
worker B cannot simultaneously claim E
~~~

unless E becomes eligible again after lease expiry/recovery.

Do not hold a database transaction open while waiting for external I/O.

---

# 25. RETRY CLASSIFICATION

Retries must be bounded and classified:

~~~text
RETRYABLE
TERMINAL
UNKNOWN
~~~

Retryable examples:

~~~text
temporary network failure
temporary database failure
provider unavailable
HTTP 5xx
bounded rate-limit response
~~~

Terminal examples:

~~~text
invalid event schema
unsupported event version
known malformed payload
permanent destination rejection
~~~

Unknown outcomes require explicit handling.

Do not retry every error indefinitely.

---

# 26. BACKOFF

Retry scheduling must be:

~~~text
bounded
observable
deadline-aware
jittered where appropriate
~~~

Never create a hot loop:

~~~text
failure
↓
retry immediately
↓
failure
↓
retry immediately
...
~~~

All worker resource usage remains bounded by the repository's reliability rules.

---

# 27. QUARANTINE / DEAD LETTER

A permanently unprocessable event must remain inspectable.

Do not simply delete failed events.

Preferred conceptual behavior:

~~~text
PENDING
  |
  v
attempts exhausted / terminal failure
  |
  v
QUARANTINED
  |
  +--> investigation
  +--> controlled replay
  +--> permanent disposition
~~~

Replay of the same semantic event should preserve the original event identity.

---

# 28. REPLAY SAFETY

Replay means:

> Deliver the same already-authorized durable event again.

Replay does NOT mean:

> Execute the original business command again.

Never turn an outbox record into a second authorization path.

---

# 29. ATOMICITY MATRIX

| Business mutation | Audit | Outbox | Result |
|---|---|---|---|
| fail | none | none | rollback |
| success | fail | none | rollback |
| success | success | fail | rollback |
| success | success | success | commit |
| commit | worker fails | durable outbox remains | retry |
| commit | worker publishes then crashes | duplicate possible | consumer deduplicates |

This matrix MUST become executable test coverage.

---

# 30. RLS / TENANT CONTEXT

Part 8 inherits the real PostgreSQL security boundary established by PR-008.

The implementation MUST:

- use the same trusted transaction-local tenant context;
- never use global mutable tenant state;
- never let client organization IDs override trusted scope;
- preserve app_runtime least privilege;
- preserve forced RLS where required;
- preserve setup/admin versus runtime authority separation.

PostgreSQL RLS uses USING and WITH CHECK policy expressions to control visibility and modifications. Superusers, BYPASSRLS roles and normally table owners can bypass RLS.

References:
https://www.postgresql.org/docs/18/ddl-rowsecurity.html
https://www.postgresql.org/docs/18/sql-createrole.html

---

# 31. AUDIT/OUTBOX TABLE SECURITY

For every audit/outbox table explicitly determine:

~~~text
owner
runtime privileges
worker privileges
setup/admin privileges
RLS requirement
SELECT access
INSERT access
UPDATE access
DELETE access
TRUNCATE access
role membership
~~~

Do not assume business-table RLS automatically protects these control-plane tables.

Outbox rows may contain tenant identifiers for routing while remaining inaccessible to ordinary tenant-facing APIs.

---

# 32. EVENT TENANT ID IS NOT AUTHORITY

If a worker receives:

~~~text
organization_id = org-b
~~~

that value identifies routing context.

It does not by itself prove that the worker is authorized to act for org-b.

If worker processing accesses tenant-owned state:

~~~text
event context
   ↓
validated event
   ↓
explicit service/worker authority
   ↓
transaction-local DB context
   ↓
RLS
~~~

Never blindly treat an event field as authorization proof.

---

# 33. SERVICE / WORKER PRIVILEGES

If a worker needs controlled cross-tenant processing, create an explicit service identity and privilege model.

Do NOT solve this with:

~~~text
SUPERUSER
BYPASSRLS
~~~

for convenience.

Any service identity must be:

~~~text
least privileged
explicit
auditable
bounded
revocable
~~~

---

# 34. TRANSACTION API BOUNDARY

Conceptually:

~~~rust
let mut tx = persistence.begin().await?;

authorize(&scope, &command).await?;
apply_domain_mutation(&mut tx, command).await?;
audit.record_required(&mut tx, audit_event).await?;
outbox.enqueue(&mut tx, event).await?;

tx.commit().await?;
~~~

This is illustrative only.

Use the repository's existing transaction abstraction rather than inventing a second transaction API.

The invariant is that all mandatory writes use the same authoritative transaction.

---

# 35. NO NETWORK CALLS INSIDE THE AUTHORITATIVE TRANSACTION

Never:

~~~text
BEGIN
business mutation
email provider
payment provider
webhook
wait
outbox
COMMIT
~~~

Correct:

~~~text
BEGIN
business mutation
audit
outbox
COMMIT
        |
        v
worker
        |
        +--> external call
~~~

This keeps transaction duration bounded and external failures recoverable.

---

# 36. FAILURE INJECTION

Test failures:

~~~text
after business mutation
after audit insert
after outbox insert
before commit
after commit / before worker claim
after worker claim
after external publication / before mark-published
~~~

Expected:

### Failure before commit

~~~text
business absent
audit absent
outbox absent
~~~

### Commit succeeds

~~~text
business present
audit present
outbox present
~~~

### Worker crash after claim

~~~text
event reclaimable
~~~

### Worker crash after publication

~~~text
duplicate delivery possible
consumer idempotency preserves correctness
~~~

---

# 37. SECURITY TEST MATRIX

Minimum:

~~~text
tenant A cannot read tenant B audit
tenant A cannot write tenant B audit
tenant A cannot alter audit ownership
runtime role cannot delete audit
runtime role cannot truncate audit

ordinary API role cannot claim arbitrary outbox rows
worker cannot fabricate arbitrary tenant authority

missing tenant context -> fail closed
invalid tenant context -> fail closed
stale pooled context -> fail closed

secret-bearing payload -> rejected
oversized payload -> rejected
unknown event version -> safe handling
~~~

Where outbox is intentionally internal, ordinary tenant-facing roles should not be able to query it.

---

# 38. POSITIVE TEST MATRIX

Minimum:

~~~text
authorized organization mutation
    -> one audit event
    -> one required outbox event

authorized branch mutation
    -> one audit event
    -> one required outbox event

authorized membership change
    -> one audit event
    -> one required outbox event

authorized role change
    -> one audit event
    -> one required outbox event

authorized scope change
    -> one audit event
    -> one required outbox event
~~~

Do not emit duplicate semantic events for one successful command unless the contract explicitly requires them.

---

# 39. CONCURRENCY TEST MATRIX

At minimum:

~~~text
two workers claim same event
two membership mutations race
two invitation acceptance attempts race
two lifecycle transitions race
same idempotent command retries concurrently
~~~

Assert final authoritative state, not merely absence of panics.

Use real PostgreSQL for invariants that depend on database locking/constraints.

---

# 40. DUPLICATE COMMAND SEMANTICS

For retryable commands:

~~~text
same command
+
same idempotency identity
~~~

must not create unintended duplicate state.

Security-event semantics must remain deliberate:

- repeated rejected attempts may legitimately create multiple security events;
- repeated execution of an idempotent successful command must not create duplicate business transitions;
- outbox represents the semantic transition, not every HTTP retry.

---

# 41. EVENT REGISTRY

Maintain one authoritative registry for durable event names.

Each event entry should define:

~~~text
name
version
category
producer
mandatory/diagnostic
tenant scope
payload schema
sensitive fields
consumer(s)
retry semantics
retention class
~~~

Do not scatter event definitions across unrelated handlers.

---

# 42. EVENT PRODUCER RULE

An event is emitted because an authoritative fact occurred.

Do not emit a successful mutation event before the mutation has been durably coupled to its transaction.

The outbox is the mechanism that makes event intent survive process death.

---

# 43. CONSUMER RULE

Consumers must assume:

~~~text
duplicate delivery
delayed delivery
temporary destination failure
reordered delivery across unrelated aggregates
unknown future versions
~~~

Consumers must not assume exactly once.

---

# 44. OBSERVABILITY

Required bounded metrics include, where compatible with existing observability:

~~~text
outbox_pending_count
outbox_oldest_pending_age
outbox_processing_duration
outbox_success_count
outbox_retry_count
outbox_terminal_failure_count
outbox_quarantine_count
outbox_duplicate_delivery_count

audit_write_success
audit_write_failure
mandatory_audit_failure
authorization_denial_count
audit_storage_latency
~~~

Do NOT put high-cardinality identifiers such as organization IDs, user IDs, emails or event payloads into metric labels.

---

# 45. ALERTING

Define operational alert conditions for:

~~~text
mandatory audit write failures
outbox backlog above threshold
oldest pending event age above SLO
repeated terminal failures
worker claim starvation
unexpected audit permission failures
abnormal duplicate delivery
~~~

Exact numeric thresholds belong to operational policy and must not be invented in the core domain.

---

# 46. BACKPRESSURE

The outbox is durable but not infinite.

Bound:

~~~text
batch size
concurrent deliveries
retry rate
payload size
query/page size
~~~

Do not load the entire outbox into memory.

Do not process an unbounded batch in one transaction.

---

# 47. AUDIT RETENTION

Part 8 must establish retention metadata/ownership where the existing model supports it, but must NOT invent legal retention periods.

Retention depends on jurisdiction, contractual obligations, product policy and future compliance decisions.

Do not hard-code arbitrary deletion periods.

---

# 48. PRIVACY / DATA MINIMIZATION

Auditability does not justify unlimited personal-data collection.

Apply:

~~~text
minimum necessary
purpose limitation
bounded retention
least-privilege access
~~~

Do not copy complete request bodies into durable audit records.

Do not store raw JWTs or bearer tokens for investigation convenience.

---

# 49. AUDIT INTEGRITY

Minimum architectural properties:

~~~text
append-oriented storage
restricted UPDATE/DELETE
restricted read access
separate ownership where practical
access logging
tamper-detection strategy
retention/disposition governance
~~~

Do not claim cryptographic tamper-proofing unless a real integrity mechanism is implemented and verified.

A future hash chain, immutable archive, WORM store or signing mechanism is a separate architecture decision.

---

# 50. PUBLIC API BOUNDARY

Part 8 does not require a public event broker API.

Use narrow internal abstractions such as:

~~~text
AuditRecorder
OutboxWriter
~~~

or repository-approved equivalents.

The application layer must not depend directly on broker protocols or worker lease SQL.

---

# 51. DATABASE/MIGRATION BOUNDARY

Phase 5 remains the canonical PostgreSQL schema/migration program.

Part 8 defines semantic requirements for audit/outbox persistence but must not create:

~~~text
a second migration framework
a competing schema authority
a parallel RLS model
unrelated database tables
~~~

If production schema work is needed, implement it through the canonical migration authority.

---

# 52. API / APPLICATION / DOMAIN / PERSISTENCE OWNERSHIP

The intended dependency direction remains:

~~~text
API
 ↓
APPLICATION
 ↓
DOMAIN
 ↓
PERSISTENCE
~~~

Audit and events remain explicit architectural boundaries.

Do not put SQL into HTTP handlers.

Do not put broker code into domain logic.

Do not put security policy decisions into outbox delivery code.

---

# 53. WORKER AUTHORITY

A worker consumes already-authorized durable event intent.

It must not become a second path around authorization.

Forbidden conceptual flow:

~~~text
outbox record
    ↓
pretend original user requested operation
    ↓
bypass authorization
    ↓
mutate business state
~~~

Replay is delivery, not command execution.

---

# 54. FAILURE RUNBOOK — AUDIT WRITE FAILURE

1. Fail the transaction if the mutation has not committed.
2. Capture request/trace correlation.
3. Record a safe internal error class.
4. Verify no partial business state committed.
5. Verify no orphan outbox event committed.
6. Inspect database availability/permissions.
7. Restore service only after mandatory audit persistence is healthy.
8. Preserve safe evidence without storing secrets.

Do not bypass mandatory audit persistence to restore throughput.

---

# 55. FAILURE RUNBOOK — OUTBOX BACKLOG

1. Check worker health.
2. Check database connectivity.
3. Check claim/lease contention.
4. Check destination availability.
5. Check retry classification.
6. Inspect quarantine/terminal-failure counts.
7. Increase worker capacity only within resource limits.
8. Never delete backlog to restore metrics.
9. Preserve failed events for investigation.

---

# 56. FAILURE RUNBOOK — QUARANTINED EVENT

1. Identify event ID.
2. Identify event name/version.
3. Identify aggregate and tenant context.
4. Inspect failure class.
5. Determine transient versus terminal.
6. Verify consumer compatibility.
7. Replay only through a controlled mechanism.
8. Verify consumer idempotency.
9. Audit operator action.
10. Preserve original event history.

---

# 57. FAILURE RUNBOOK — DUPLICATE DELIVERY

Duplicate delivery is a normal at-least-once failure mode, not automatically corruption.

1. Identify event ID.
2. Determine whether it was previously processed.
3. Verify idempotency guard.
4. Verify no duplicate business side effect occurred.
5. Inspect relay crash timing if frequency is abnormal.

---

# 58. FAILURE RUNBOOK — CROSS-TENANT AUDIT ACCESS

For organization A attempting to access organization B evidence:

~~~text
DENY
~~~

Verify:

~~~text
trusted principal
membership
effective scope
transaction-local context
repository predicate
RLS
public disclosure behavior
audit evidence
regression coverage
~~~

Do not repair the boundary by trusting route parameters more carefully.

---

# 59. CI AND VERIFICATION

Required repository gates remain authoritative.

At minimum:

~~~text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
./scripts/ci/verify
~~~

For database security properties, tests must use real PostgreSQL where the property depends on PostgreSQL.

No security test may silently skip because PostgreSQL is unavailable.

Do not claim a database security property from a mock.

---

# 60. REQUIRED TEST LAYERS

## Unit

Test:

- event names;
- versions;
- validation;
- secret rejection;
- reason classes;
- payload limits;
- state transitions.

## Application

Test:

- command → mutation + audit + outbox;
- audit failure;
- outbox failure;
- rollback;
- duplicate command behavior.

## PostgreSQL integration

Test:

- real privileges;
- real tenant context;
- real RLS;
- audit mutation restrictions;
- outbox claim concurrency;
- rollback atomicity.

## Worker

Test:

- retry;
- terminal failure;
- lease recovery;
- duplicate delivery;
- idempotent consumer interaction.

---

# 61. REQUIRED ATOMICITY TESTS

### Test A — business mutation succeeds, audit fails

Expected:

~~~text
rollback
business absent
audit absent
outbox absent
~~~

### Test B — business mutation succeeds, outbox fails

Expected:

~~~text
rollback
business absent
audit absent
outbox absent
~~~

### Test C — all writes succeed

Expected:

~~~text
commit
business present
audit present
outbox present
~~~

### Test D — worker crashes after claim

Expected:

~~~text
event reclaimable
~~~

### Test E — worker crashes after external publication

Expected:

~~~text
duplicate possible
consumer idempotency preserves correctness
~~~

---

# 62. REQUIRED SECURITY TESTS

Minimum:

~~~text
missing tenant context -> fail closed
invalid tenant context -> fail closed
cross-tenant audit read -> denied
cross-tenant audit write -> denied
audit UPDATE by runtime -> denied
audit DELETE by runtime -> denied
outbox administrative mutation by ordinary API role -> denied
worker cannot fabricate tenant authority
secret payload -> rejected
oversized event -> rejected
unknown version -> safely handled
~~~

---

# 63. REQUIRED CONCURRENCY TESTS

Minimum:

~~~text
two workers claim same event
two membership changes race
two invitation acceptance attempts race
two lifecycle transitions race
same idempotent command retries concurrently
~~~

Assert final state and durable evidence.

---

# 64. REQUIRED FAILURE INJECTION

Inject failure after:

~~~text
business write
audit write
outbox write
~~~

and before:

~~~text
commit
~~~

At least one real integration test must prove that a failed mandatory audit/outbox operation cannot leave committed business state.

---

# 65. LIKELY IMPLEMENTATION BOUNDARIES

Potential files:

~~~text
crates/sitolo-audit/**
crates/sitolo-events/**
crates/sitolo-application/**
crates/sitolo-persistence/**
apps/worker/**
~~~

Focused tests may live near those boundaries.

Do not modify unrelated HTTP/domain files merely to demonstrate activity.

Dependency manifests may change only for actual implementation requirements.

---

# 66. PHASE BOUNDARIES

Do not absorb:

~~~text
PR-010 cache/versioning
PR-011 device binding
PR-012 ownership transfer
PR-013 support/admin separation
Phase 5 complete PostgreSQL program
Phase 6 generalized authorization
~~~

A future feature may consume Part 8 events. That does not authorize implementing the future feature now.

---

# 67. ANTI-PATTERNS

Do NOT implement:

~~~text
post_commit_audit()
best_effort_audit()
fire_and_forget_outbox()
global_event_bus()
unbounded_event_payload
raw_debug_payload
superuser_worker
BYPASSRLS_worker
delete_failed_events
retry_forever
exactly_once_claim
in_memory_outbox_for_production
in_memory_audit_for_production
~~~

Also do not:

- publish directly from the API before commit;
- use application logs as the audit store;
- trust event tenant IDs as authorization;
- replay commands from events;
- store secrets in events;
- expose outbox rows through ordinary tenant APIs.

---

# 68. DEFINITION OF DONE

Part 8 is complete only when:

~~~text
[ ] audit semantics are explicitly typed
[ ] Phase 3 authentication audit remains intact
[ ] Phase 4 IAM/security events are defined
[ ] audit records are secret-free
[ ] mandatory audit persistence is transactional
[ ] outbox records are durable
[ ] outbox writes share the authoritative transaction
[ ] delivery is explicitly at-least-once
[ ] duplicate delivery is handled
[ ] retry classes are explicit
[ ] worker claim/recovery is bounded
[ ] quarantine behavior exists
[ ] event identity/versioning is explicit
[ ] aggregate ordering is explicit
[ ] event tenant IDs cannot forge authority
[ ] audit/outbox privileges are least-privileged
[ ] ordinary runtime cannot rewrite audit evidence
[ ] real PostgreSQL proves database security properties
[ ] rollback tests prove no orphan state
[ ] worker concurrency tests exist
[ ] observability exists without sensitive labels
[ ] secrets do not enter events/logs
[ ] no later phase is silently absorbed
[ ] canonical CI gates pass
[ ] actual test evidence is recorded
~~~

---

# 69. IMPLEMENTATION ORDER

## Step 1 — repository reconciliation

Inspect the governing documents and current source.

Do not modify code until the existing boundaries are understood.

## Step 2 — event contracts

Establish identity, name, version, category, actor, scope, target, result, reason and correlation.

## Step 3 — audit abstraction

Extend the existing audit boundary without breaking Phase 3.

## Step 4 — outbox abstraction

Define durable event intent separately from audit.

## Step 5 — transaction integration

Make mandatory mutation + audit + outbox atomic.

## Step 6 — PostgreSQL persistence

Implement the real durable boundary through the canonical database architecture.

## Step 7 — worker claim/retry

Implement bounded processing and recovery.

## Step 8 — security hardening

Verify privileges, RLS, tenant isolation and secret minimization.

## Step 9 — failure/concurrency tests

Exercise real failures and races.

## Step 10 — observability/runbooks

Add bounded metrics and operational diagnostics.

## Step 11 — final verification

Run canonical gates and report exactly what executed.

---

# 70. REVIEW QUESTIONS

### Audit

- Can a sensitive IAM transition be reconstructed?
- Is actor identity authoritative?
- Is tenant scope authoritative?
- Can runtime code rewrite evidence?
- Can secrets enter through future struct changes?

### Outbox

- Can committed state exist without its required event?
- Can worker death lose work?
- Can publication occur twice?
- Can consumers tolerate duplicates?
- Can events become permanently stuck?
- Can payload size exhaust memory?

### Tenant isolation

- Can one tenant inspect another tenant's audit?
- Can an event tenant field forge authorization?
- Can pooled connections retain stale context?
- Can a worker bypass RLS?

### Reliability

- What happens on database failure?
- What happens on worker restart?
- What happens during destination outage?
- What happens after partial network failure?
- What happens when event schema changes?

### Operations

- Can operators see backlog?
- Can they identify terminal failures?
- Can they replay safely?
- Can they investigate without secrets?
- Are audit-access operations themselves accountable?

---

# 71. REQUIRED IMPLEMENTATION REPORT

When implementation is complete, report:

~~~text
Implemented:
  exact files and semantic changes

Audit:
  event catalogue and persistence behavior

Outbox:
  event contract, delivery, retry and claim behavior

Security:
  tenant isolation, privileges, RLS and secret handling

Tests:
  exact commands and actual results

PostgreSQL:
  exact real-database evidence

Not verified:
  anything not actually exercised

Deferred:
  Phase 5/6 and later Phase 4 work

Operational impact:
  metrics, alerts, worker behavior and recovery
~~~

Never report a test as passing unless it actually executed.

---

# 72. RESEARCH BASIS

## Transactional outbox

The transactional outbox pattern stores the event in the same database transaction as the business mutation and uses a separate relay for publication. The pattern explicitly acknowledges duplicate publication after relay failure and therefore requires idempotent consumers.

https://microservices.io/patterns/data/transactional-outbox

Polling publisher and transaction-log tailing are alternative relay mechanisms. Part 8 does not force a broker or CDC technology prematurely.

https://microservices.io/patterns/data/polling-publisher.html
https://microservices.io/patterns/data/transaction-log-tailing.html

## OWASP security logging

OWASP recommends recording security-relevant events such as authentication failures, authorization failures and high-risk administrative actions while minimizing sensitive information, protecting audit records, controlling access and considering tamper detection.

https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html

https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html

## PostgreSQL security

PostgreSQL documents that superusers and BYPASSRLS roles bypass row security and that table owners normally bypass RLS unless forced. This reinforces the existing Sitolo requirement for least-privileged runtime roles and ownership separation.

https://www.postgresql.org/docs/18/ddl-rowsecurity.html
https://www.postgresql.org/docs/18/sql-createrole.html

## PostgreSQL policy inspection

PostgreSQL exposes RLS policy metadata through pg_policy and pg_policies, including command scope, roles, USING and WITH CHECK expressions.

https://www.postgresql.org/docs/18/catalog-pg-policy.html
https://www.postgresql.org/docs/18/view-pg-policies.html

---

# 73. FINAL CONTRACT

The implementation target is:

~~~text
                 AUTHORITATIVE COMMAND
                         |
                         v
                 AUTHORIZATION/SCOPE
                         |
                         v
                 DOMAIN INVARIANTS
                         |
                         v
                  POSTGRES TX
                   /    |    \
                  /     |     \
             STATE     AUDIT   OUTBOX
                  \     |    /
                   \    |   /
                     COMMIT
                        |
                        v
                  DURABLE EVENT
                        |
                        v
                     WORKER
                        |
                        v
                 EXTERNAL EFFECT
                        |
                        v
                  IDEMPOTENT
                    CONSUMER
~~~

Non-negotiable properties:

~~~text
1. PostgreSQL remains authoritative.
2. Audit is durable evidence, not ordinary logging.
3. Outbox is durable event intent, not business truth.
4. Mandatory audit/outbox writes are transactionally coupled to authoritative mutation.
5. External effects occur only after durable commit.
6. Delivery is at-least-once.
7. Consumers tolerate duplicates.
8. Tenant context is trusted security state, never event-payload authority.
9. Audit data is itself tenant/security sensitive.
10. Secrets never enter durable audit/outbox payloads.
11. Runtime privileges remain least-privileged.
12. RLS remains a defense-in-depth boundary.
13. Worker failure is recoverable.
14. Failed events remain inspectable.
15. Event schemas are versioned.
16. Ordering is explicit per aggregate, not falsely global.
17. Real PostgreSQL tests prove database security properties.
18. No later Phase 4/5/6 feature is silently absorbed.
19. Every security claim has executable evidence.
20. No fabricated test or production-readiness claim is acceptable.
~~~

**Part 8 / PR-009 is complete only when the repository can prove these properties with code, database behavior, tests and operational evidence — not merely documentation.**
