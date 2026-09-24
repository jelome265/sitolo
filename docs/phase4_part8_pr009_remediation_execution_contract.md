# SITOLO — PHASE 4 PART 8 PR-009 REMEDIATION + IMPLEMENTATION EXECUTION CONTRACT

**Document:** `docs/phase4_part8_pr009_remediation_execution_contract.md`  
**Primary contract:** `docs/phase4_part8_audit_outbox_implementation_contract.md`  
**PR:** #49 — `feat(security): Phase 4 Part 8 audit and outbox implementation`  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Repository:** `jelome265/sitolo`  
**Status:** Remediation / execution instruction  
**Current reviewed head:** `6cfab2b219dd74384115546380ba98ca4c19b4bb`  
**Current PR base:** `30c7f5ae1a09c2142939b4535f8bf3d87fad9b41`  
**Reviewed:** 24 September 2026

> This document is subordinate to the binding implementation contract named above. It does not redefine Part 8 requirements. It converts the contract into an implementation and verification sequence for PR #49 and records the gaps found in the current PR.

---

# 0. EXECUTIVE RESULT

PR #49 is **not complete against the Part 8 implementation contract**.

The current PR establishes useful type-level foundations:

- Phase 4 IAM event names;
- IAM audit record shape;
- an outbox record shape;
- an outbox reader/writer abstraction;
- a test SQL fixture;
- basic unit-level tests.

However, the PR does **not yet prove or implement the authoritative behavior required by the contract**.

The largest missing capabilities are:

1. authoritative business mutation + audit + outbox transaction coupling;
2. real PostgreSQL audit/outbox persistence;
3. real PostgreSQL security tests for the new tables;
4. worker claim/lease/recovery behavior;
5. bounded retry/backoff/quarantine behavior;
6. duplicate-delivery/idempotency verification;
7. aggregate ordering semantics;
8. event registry/schema contract;
9. observability and operational runbooks;
10. failure-injection proof;
11. privilege/ownership separation for the new tables;
12. safe worker authority.

There is also a concrete security defect in the new SQL fixture:

`app_worker` is granted `SELECT, UPDATE` on `outbox_events` and the worker RLS policies use:

```sql
USING (true)
WITH CHECK (true)
```

That is broader than the Part 8 contract allows. It permits the worker to update arbitrary rows and potentially rewrite tenant/event fields unless another database boundary prevents it. PostgreSQL RLS remains a policy boundary, but a policy of `true` is intentionally permissive. PostgreSQL documents that RLS policies constrain rows/commands, while table owners and BYPASSRLS roles have special behavior; forced RLS must therefore be paired with an actually least-privileged policy design. citeturn147448search3turn147448search1

This remediation document is therefore the execution target for the remainder of PR #49.

---

# 1. GOVERNING CONTRACT

The authoritative source is:

`docs/phase4_part8_audit_outbox_implementation_contract.md`

The implementation MUST preserve its non-negotiable properties:

```text
PostgreSQL remains authoritative.
Audit is durable evidence.
Outbox is durable event intent.
Mandatory audit/outbox writes are transactionally coupled to authoritative mutation.
External effects occur only after durable commit.
Delivery is at-least-once.
Consumers tolerate duplicates.
Tenant context is trusted security state, not event-payload authority.
Secrets never enter durable audit/outbox payloads.
Runtime privileges remain least-privileged.
RLS remains defense in depth.
Worker failure is recoverable.
Failed events remain inspectable.
Event schemas are versioned.
Aggregate ordering is explicit.
Real PostgreSQL proves database security properties.
No future phase is silently absorbed.
Every security claim has executable evidence.
```

Do not replace these with weaker approximations.

---

# 2. CURRENT PR SNAPSHOT

## 2.1 Current PR

PR #49 is open and has one implementation commit at the reviewed head:

```text
6cfab2b219dd74384115546380ba98ca4c19b4bb
```

Changed files:

```text
crates/sitolo-audit/src/iam_event.rs
crates/sitolo-audit/src/lib.rs
crates/sitolo-events/Cargo.toml
crates/sitolo-events/src/lib.rs
crates/sitolo-events/src/outbox.rs
crates/sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql
crates/sitolo-persistence/tests/part8_audit_outbox_tests.rs
```

No CI statuses were returned for the reviewed head. The PR discussion also contains an automated security-evidence note stating that check publication was unavailable because the GitHub app lacks the required Checks write permission. Do not treat that note as test evidence.

---

# 3. INVESTIGATION FINDINGS

## 3.1 Audit model — partial

The PR correctly extends `sitolo-audit` rather than creating a second authentication audit crate.

Positive:

- IAM event names are typed.
- Dotted lowercase names are stable.
- Event version exists.
- Actor references are pseudonymous references.
- Authentication audit remains intact.

Missing/weak:

- no metadata field despite the contract's minimum conceptual model;
- fields are mostly unbounded `String`;
- no centralized event registry;
- no explicit category;
- no explicit mandatory/diagnostic property in the IAM event itself;
- no persistence implementation;
- no request/trace population path;
- no validation path for event content;
- no durable audit writer;
- no access/immutability verification.

The contract requires the semantics of a structured, bounded and secret-free audit record. Merely having secret-free field names is not enough to prove future payload safety.

---

## 3.2 Outbox model — partial

The PR correctly introduces:

- `PENDING`;
- `CLAIMED`;
- `PUBLISHED`;
- `QUARANTINED`;
- retry classes;
- attempt count;
- lease timestamp;
- published timestamp;
- deduplication key;
- event version.

Missing:

- no concrete SQL repository implementation;
- no transaction-aware writer;
- no claim implementation;
- no lease recovery implementation;
- no retry scheduling implementation;
- no bounded backoff implementation;
- no quarantine persistence path;
- no duplicate-delivery consumer contract;
- no aggregate sequence/order field;
- no event registry;
- no schema validation beyond byte-count payload size;
- no real JSON/schema serialization boundary.

The existence of an `OutboxReader` trait does not prove that a worker can safely claim or recover events.

---

## 3.3 Transaction atomicity — missing

This is the largest functional gap.

The contract requires:

```text
BEGIN
  establish trusted tenant context
  authorize
  validate
  mutate business state
  insert mandatory audit
  insert required outbox
COMMIT
```

The current PR contains no application-layer transaction integration and no persistence repository that accepts the existing `sqlx::Transaction` boundary.

The existing repository already has a trusted transaction-local context function:

```rust
set_transaction_tenant_context(
    tx: &mut Transaction<'_, Postgres>,
    scope: &AuthorizedScope,
)
```

and existing PR-008 tests exercise the same transaction pattern.

PR #49 must extend that established boundary instead of inventing another transaction mechanism.

Forbidden outcome:

```text
business commit
   |
   +--> audit later
   +--> outbox later
```

Required outcome:

```text
one authoritative PostgreSQL transaction
   |
   +--> business state
   +--> audit evidence
   +--> outbox intent
   |
 COMMIT
```

The transactional outbox pattern exists specifically to couple database mutation and durable event intent, while accepting duplicate publication after relay failure. Consumers therefore need idempotent processing. citeturn138896search0turn138896search1

---

# 4. DATABASE FINDINGS

## 4.1 Fixture is not production persistence

The SQL file is currently:

`crates/sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql`

This is useful for tests, but by itself it does not implement production persistence.

Do not call the feature production durable merely because a test fixture contains the table definitions.

Respect the repository's canonical PostgreSQL schema/migration authority. Do not introduce a second migration framework.

---

## 4.2 Audit table policy

Current fixture gives `app_runtime`:

```sql
GRANT INSERT ON audit_events TO app_runtime;
```

and an INSERT RLS policy.

That is directionally correct for least privilege.

Still required:

- prove `app_runtime` does not own the table;
- prove `app_runtime` cannot UPDATE;
- prove `app_runtime` cannot DELETE;
- prove `app_runtime` cannot TRUNCATE;
- prove cross-tenant INSERT fails;
- prove missing tenant context fails closed;
- prove forced RLS is enabled;
- inspect policy metadata through PostgreSQL catalogs;
- prove ordinary tenant-facing roles cannot read audit storage.

PR-008 already established a strong pattern for catalog verification. Reuse it.

---

## 4.3 Outbox worker policy is too broad

Current fixture:

```sql
GRANT SELECT, UPDATE ON outbox_events TO app_worker;
```

with:

```sql
CREATE POLICY outbox_events_worker_select_policy ON outbox_events
    FOR SELECT
    TO app_worker
    USING (true);

CREATE POLICY outbox_events_worker_update_policy ON outbox_events
    FOR UPDATE
    TO app_worker
    USING (true)
    WITH CHECK (true);
```

This is unacceptable as the final security boundary.

With UPDATE access and a permissive policy, the worker can potentially modify fields beyond the state transition it should own, including:

```text
organization_id
branch_id
aggregate identifiers
event name/version
payload
deduplication key
schema version
published timestamp
attempt count
status
```

The contract does not permit a worker to fabricate tenant authority.

Required remediation:

### Preferred boundary

Use a narrow worker repository/database routine that allows only the intended claim and state-transition columns.

For example, conceptually:

```text
claim:
  status
  locked_at
  attempt_count

success:
  status
  published_at
  locked_at

retry:
  status
  available_at
  attempt_count
  locked_at
  last_error_class

quarantine:
  status
  attempt_count
  last_error_class
  locked_at
```

Immutable event identity and routing fields must not be writable by the worker.

Do not grant blanket table UPDATE merely because one worker needs a few state transitions.

If SQL functions are used, apply PostgreSQL function-security discipline: tightly control who owns/creates the function, use trusted objects, and keep the callable surface narrow. citeturn147448search2

The final implementation must also prove the worker is not:

```text
SUPERUSER
BYPASSRLS
member of an unintended privileged role
owner of the protected tables
```

PostgreSQL explicitly documents that `BYPASSRLS` bypasses row security and that table ownership normally bypasses RLS unless forced. citeturn147448search1turn147448search3

---

# 5. OUTBOX CLAIMING DESIGN

Implement a bounded claim algorithm against PostgreSQL.

Required behavior:

```text
worker A claims E
worker B cannot simultaneously claim E
```

unless E becomes eligible again after its lease expires.

The claim path should use row-locking semantics suitable for concurrent workers. PostgreSQL supports `FOR UPDATE` row locking and `SKIP LOCKED` for avoiding contention between concurrent consumers. citeturn147448search0

A practical pattern is:

```text
BEGIN
  select a bounded set of eligible PENDING rows
  with row locks
  using SKIP LOCKED
  mark them CLAIMED
  assign lease timestamp
  increment attempt count
COMMIT
```

Do not:

- hold the claim transaction while performing external I/O;
- load the entire table;
- claim unlimited rows;
- rely on application-side mutexes;
- rely on one process instance;
- rely on wall-clock order as an aggregate ordering authority.

The lease must be recoverable after worker death.

---

# 6. RETRY / BACKOFF / QUARANTINE

The current `RetryClass` enum is only a type. Implement real behavior.

Required model:

```text
RETRYABLE
TERMINAL
UNKNOWN
```

Retry scheduling must be:

```text
bounded
observable
deadline-aware
jittered where appropriate
```

The worker must not hot-loop:

```text
failure
retry immediately
failure
retry immediately
...
```

Required data behavior:

```text
attempt_count += 1
available_at = computed next eligible time
last_error_class = bounded classification
```

A terminal/attempt-exhausted event moves to:

```text
QUARANTINED
```

Do not delete permanently failed events.

---

# 7. EVENT IDENTITY / VERSION / ORDERING

## 7.1 Event identity

`event_id` must be globally unique and stable.

The PR currently accepts arbitrary `String` values and the test passes `"evt-1"`.

That proves neither uniqueness nor safe production generation.

Use the repository's existing ID facility or another repository-approved generator.

Do not derive event identity from:

- timestamps alone;
- organization names;
- usernames;
- client-supplied sequential values.

---

## 7.2 Version semantics

Every durable event must carry a stable event schema version.

Keep separate:

```text
application release
event version
database schema version
```

Do not silently change version 1 payload semantics.

The current outbox model has both `event_version` and `schema_version`, but `schema_version = event_version` is hard-coded in the constructor. Do not keep two fields that happen to mean the same thing unless the repository explicitly defines two distinct version domains.

Resolve the semantic distinction before implementation.

---

## 7.3 Aggregate ordering

The PR does not implement ordering.

The contract forbids pretending wall-clock timestamps provide aggregate order.

Add an explicit repository-approved ordering mechanism, such as an aggregate-local monotonically increasing sequence.

Required property:

```text
aggregate A:
E1 -> E2 -> E3

delivery of unrelated aggregate B may interleave
```

Do not promise global ordering.

---

# 8. EVENT PAYLOAD SAFETY

Current `OutboxEvent::new` only enforces:

```text
payload.len() <= 65,536
```

That is necessary but insufficient.

The durable payload contract must be:

```text
bounded
versioned
deterministic
serializable
secret-free
schema-defined
stable
```

Do not serialize domain objects using Rust Debug formatting.

The payload must not become:

```text
complete database row
request body
JWT
Authorization header
authentication secret
credential object
unbounded arbitrary metadata
```

Use explicit event DTOs/contracts.

Reject malformed JSON/schema input before durable enqueue.

Add tests for:

- invalid JSON;
- unknown event name;
- unsupported event version;
- oversized payload;
- prohibited secret-bearing fields/patterns;
- deterministic serialization.

OWASP recommends logging security events while excluding or sanitizing credentials, access tokens, passwords, connection strings and sensitive PII. citeturn138896search4

---

# 9. IAM EVENT CATALOGUE

The current enum covers the required Phase 4 examples.

The next implementation step is to make the catalogue authoritative rather than merely enumerated.

Each durable event definition should declare:

```text
name
version
category
producer
mandatory/diagnostic
tenant scope
payload schema
sensitive fields
consumer
retry semantics
retention class
ordering domain
```

Do not scatter the same event name across unrelated handlers.

Do not create the full future Sitolo event catalogue here.

---

# 10. AUDIT RECORD COMPLETENESS

The minimum audit semantics from the primary contract are:

```text
event_id
event_name
event_version
occurred_at
organization_id
branch_id
actor_subject_ref
actor_membership_ref
actor_device_ref
request_id
trace_id
target_type
target_ref
action
result
reason_class
assurance_level
source
metadata
```

The current `IamAuditEvent` is missing the explicit metadata field.

Do not add an unbounded `HashMap<String, Value>` and call the requirement complete.

Metadata must itself be:

- bounded;
- typed or schema-constrained;
- safe;
- minimal;
- observable without leaking secrets.

---

# 11. MANDATORY VS DIAGNOSTIC

The existing Phase 3 audit boundary already distinguishes:

```text
Mandatory
Diagnostic
```

Preserve it.

For mandatory operations:

```text
business state
+
audit
+
outbox
```

are one consistency boundary.

If mandatory audit persistence fails, the authoritative mutation must not commit.

Diagnostic telemetry may degrade independently unless a higher contract says otherwise.

Do not conflate:

```text
logs
audit
metrics
traces
outbox
```

They have different guarantees.

---

# 12. APPLICATION TRANSACTION INTEGRATION

Integrate Part 8 into the existing application/persistence boundary.

Use the repository's current transaction abstraction.

Existing PR-008 code already establishes transaction-local scope from `AuthorizedScope`. Preserve that mechanism.

Target conceptual structure:

```rust
let mut tx = persistence.begin_scoped(&scope).await?;

authorize(...).await?;
validate(...).await?;
apply_domain_mutation(&mut tx, ...).await?;
audit.record_required(&mut tx, audit_event).await?;
outbox.enqueue(&mut tx, outbox_event).await?;

tx.commit().await?;
```

This is conceptual. Use repository-approved names and traits.

Never move authorization into the worker.

Never make the worker re-execute the command.

Never publish externally before commit.

---

# 13. REQUIRED ATOMICITY TESTS

Implement executable tests for every row of the contract matrix.

## Test A — business succeeds, audit fails

Expected:

```text
business absent
audit absent
outbox absent
transaction rolled back
```

## Test B — business succeeds, outbox fails

Expected:

```text
business absent
audit absent
outbox absent
transaction rolled back
```

## Test C — all writes succeed

Expected:

```text
business present
audit present
outbox present
commit succeeds
```

## Test D — worker crashes after claim

Expected:

```text
event becomes reclaimable after lease expiry
```

## Test E — worker publishes then crashes before mark-published

Expected:

```text
duplicate publication possible
consumer idempotency prevents duplicate business side effect
```

These must not be represented by unit tests alone.

At least the transaction/RLS properties must execute against the real PostgreSQL security boundary.

---

# 14. REAL POSTGRESQL SECURITY TESTS

Reuse the established PR-008 harness patterns.

Required checks:

```text
real PostgreSQL connected
runtime identity = app_runtime where appropriate
worker identity is explicit
worker is least privileged
new tables are not owned by app_runtime
RLS enabled
FORCE RLS where required
policy roles are exact
missing tenant context fails closed
invalid tenant context fails closed
tenant A cannot read tenant B audit
tenant A cannot write tenant B audit
runtime cannot update audit
runtime cannot delete audit
runtime cannot truncate audit
ordinary API role cannot query outbox
worker cannot rewrite immutable event fields
worker cannot fabricate tenant authority
```

The repository already has catalog-driven RLS verification patterns in PR-008. Extend those rather than creating a weaker test harness.

Do not accept mocks as evidence for PostgreSQL security properties.

---

# 15. REQUIRED CONCURRENCY TESTS

Implement:

```text
two workers claim the same event
two membership changes race
two invitation acceptance attempts race
two lifecycle transitions race
same idempotent command retries concurrently
```

For worker claiming specifically, assert:

```text
one event
two workers
one simultaneous claim
no duplicate claim ownership
recoverability after lease expiry
```

Assert final durable state and evidence, not merely lack of panic.

---

# 16. DUPLICATE DELIVERY / IDEMPOTENCY

At-least-once means duplicate delivery is expected.

The relay can publish and crash before recording success. This is a normal failure mode of the transactional outbox pattern. citeturn138896search0

Consumers need a durable idempotency guard appropriate to their domain.

The implementation must separate:

```text
command idempotency
event identity
delivery idempotency
```

Do not make retry create a fresh semantic event.

For idempotent consumers, a processed-message identity or equivalent durable guard is a standard pattern. citeturn138896search1

---

# 17. TENANT / SCOPE RULES

The event's `organization_id` is routing metadata.

It is not authorization proof.

Required flow:

```text
authenticated principal
 -> membership
 -> authorized scope
 -> trusted transaction-local context
 -> authoritative mutation
 -> audit/outbox
```

Never:

```rust
event.organization_id = request.organization_id;
```

and assume the value is trusted.

The PR-008 database boundary must remain the foundation.

Do not introduce:

- global mutable tenant context;
- connection-global mutable state;
- client-authoritative tenant switching;
- worker BYPASSRLS shortcuts.

---

# 18. WORKER SECURITY MODEL

The worker is a durable delivery component, not an authorization engine.

The worker may:

```text
claim durable event
validate event
deliver event
mark published
requeue retryable failure
quarantine terminal failure
```

The worker must NOT:

```text
re-authorize user commands
change tenant ownership
change event identity
change immutable payloads
pretend to be the original user
mutate arbitrary business entities merely to complete delivery
use SUPERUSER/BYPASSRLS for convenience
```

Replay means:

```text
deliver the same durable event again
```

not:

```text
execute the original business command again
```

---

# 19. OBSERVABILITY

Add bounded operational metrics consistent with the existing observability architecture:

```text
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
```

Do not use organization IDs, user IDs, emails or event payloads as metric labels.

Add worker logs with:

```text
event_id
event_name
attempt_count
safe error class
request/trace correlation when available
```

Do not log secrets.

OWASP recommends protecting logging infrastructure from unauthorized access/modification/deletion and avoiding credentials/tokens and unnecessary sensitive data. citeturn138896search4

---

# 20. ALERTING

Add policy placeholders rather than inventing universal thresholds.

Required alert conditions:

```text
mandatory audit write failures
outbox backlog above policy threshold
oldest event age above SLO
repeated terminal failures
claim starvation
unexpected audit permission errors
abnormal duplicate delivery
```

Numeric thresholds belong in operational policy, not in domain semantics.

---

# 21. RETENTION / PRIVACY

The current implementation has `created_at` but does not establish the required retention metadata/ownership model.

Add explicit retention classification/ownership only where supported by the repository's privacy/compliance architecture.

Do NOT invent legal retention periods.

Do NOT add destructive TTL deletion by convenience.

The contract requires:

```text
minimum necessary
purpose limitation
bounded retention
least privilege
controlled disposition
```

---

# 22. AUDIT IMMUTABILITY

The implementation must prove:

```text
append-oriented
restricted UPDATE
restricted DELETE
restricted TRUNCATE
restricted reads
separate ownership from runtime
```

Do not claim cryptographic tamper-proofing unless a real integrity mechanism is implemented.

Do not add a speculative hash chain merely to tick a checkbox.

---

# 23. FAILURE INJECTION

Inject failure at:

```text
after business write
after audit write
after outbox write
before commit
after commit / before claim
after claim
after external publication / before mark-published
```

Each failure point must have an explicit invariant.

The most important security property is:

```text
mandatory evidence failure
        ->
no committed authoritative mutation
```

At least one real PostgreSQL integration test must prove it.

---

# 24. TEST FILE STRUCTURE

The current test file:

`crates/sitolo-persistence/tests/part8_audit_outbox_tests.rs`

currently behaves as a unit test file despite its name.

Rename or refactor so the test structure communicates the real boundary.

Recommended split:

```text
crates/sitolo-persistence/tests/
  part8_audit_outbox_unit_tests.rs
  part8_audit_outbox_postgres_tests.rs
  part8_audit_outbox_worker_tests.rs
```

Exact naming may follow repository conventions.

The key requirement is semantic clarity: a test claiming PostgreSQL security must actually connect to PostgreSQL.

---

# 25. IMPLEMENTATION ORDER

Execute in this order.

## Step 1 — Reconcile repository contracts

Inspect before changing source:

```text
agent.md
docs/phase4_part8_audit_outbox_implementation_contract.md
docs/phase4_tenant_organization_branch_iam_implementation.md
docs/phase4_part7_pr008_remediation_execution_contract.md
docs/phase3_identity_sessions_mfa_device_identity_implementation.md
docs/phase2_config_secrets_logging_errors_telemetry_implementation.md
docs/testing_strategy.md
docs/database_design.md
docs/system_architecture_design.md
docs/security_architecture_design.md
docs/api_contract.md
docs/auth_authorization_spec.md
docs/observability_spec.md
docs/deployment_spec.md
docs/ADR-001-025.md
```

Inspect current source before adding duplicate abstractions.

## Step 2 — Fix the event contract

Implement:

```text
bounded event fields
explicit metadata contract
stable event identity
event registry
event/version semantics
safe serialization
secret validation
aggregate ordering semantics
```

## Step 3 — Fix the outbox contract

Implement:

```text
transaction-aware writer
repository-backed persistence
claim
lease
retry
backoff
quarantine
recovery
immutable event fields
```

## Step 4 — Integrate one authoritative transaction

Wire:

```text
business mutation
audit write
outbox write
commit
```

through the same PostgreSQL transaction.

## Step 5 — Implement database boundary

Use the canonical PostgreSQL authority.

For every new relation determine:

```text
owner
app_runtime privileges
worker privileges
setup/admin privileges
RLS
SELECT
INSERT
UPDATE
DELETE
TRUNCATE
role membership
```

## Step 6 — Harden worker SQL authority

Remove the blanket worker UPDATE policy.

Use the narrowest viable claim/state transition boundary.

## Step 7 — Implement worker behavior

Add:

```text
bounded batches
claim
lease recovery
external call after commit
success transition
retry transition
quarantine
safe logging
metrics
```

## Step 8 — Add real PostgreSQL tests

Extend PR-008 infrastructure.

Prove:

```text
tenant isolation
role separation
RLS metadata
atomicity
rollback
claim concurrency
lease recovery
worker privilege
audit immutability
```

## Step 9 — Add failure injection

Prove no orphan business state exists when mandatory audit/outbox persistence fails.

## Step 10 — Add duplicate-delivery tests

Prove consumer idempotency.

## Step 11 — Add operational documentation

Document:

```text
audit write failure
backlog
quarantine
duplicate delivery
cross-tenant access investigation
worker recovery
```

## Step 12 — Run canonical verification

At minimum:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
./scripts/ci/verify
```

Also run the real PostgreSQL security suite with its required environment.

Do not report a test as passing unless it actually executed.

---

# 26. ACCEPTANCE MATRIX

| Contract area | Current PR #49 | Required state |
|---|---|---|
| IAM event names | Partial | Typed + registry-backed + tested |
| Audit record | Partial | Bounded, complete, persisted |
| Phase 3 compatibility | Partial | Explicit regression proof |
| Secret minimization | Weak proof | Structural + serialization tests |
| Durable audit | Missing | Real PostgreSQL repository |
| Transactional audit | Missing | Same transaction as mutation |
| Outbox record | Partial | Real durable repository |
| Outbox claim | Missing | Concurrent PostgreSQL claim |
| Lease recovery | Missing | Crash-safe reclaim |
| Retry classification | Partial | Executable retry policy |
| Backoff | Missing | Bounded + observable |
| Quarantine | Missing | Durable + inspectable |
| Event identity | Partial | Stable uniqueness boundary |
| Versioning | Partial | Schema contract + compatibility |
| Ordering | Missing | Aggregate-local sequencing |
| Idempotency | Missing | Consumer duplicate guard |
| Tenant RLS | Unsafe/incomplete | Real PG proof |
| Audit immutability | Missing | Privilege + negative tests |
| Worker least privilege | Unsafe | Narrow update authority |
| Failure injection | Missing | Real rollback evidence |
| Concurrency tests | Missing | Real PostgreSQL races |
| Metrics | Missing | Bounded metrics |
| Alerts | Missing | Policy-linked conditions |
| Runbooks | Missing | Operational procedures |
| CI evidence | Not available | Actual execution recorded |

---

# 27. DEFINITION OF DONE FOR PR #49

Do not merge until the repository can demonstrate all of the following:

```text
[ ] Phase 3 authentication audit remains compatible
[ ] Phase 4 IAM event catalogue is typed and authoritative
[ ] event identity is stable and unique
[ ] event version is explicit
[ ] aggregate ordering is explicit
[ ] audit fields are bounded
[ ] audit payloads cannot contain secrets
[ ] mandatory audit is transactional
[ ] outbox is durable
[ ] outbox insert shares the authoritative transaction
[ ] outbox delivery is explicitly at-least-once
[ ] worker claim is concurrency-safe
[ ] worker lease recovery exists
[ ] retries are classified
[ ] backoff is bounded
[ ] terminal failures quarantine
[ ] failed events remain inspectable
[ ] duplicate delivery is safe
[ ] tenant context is trusted
[ ] event tenant IDs cannot forge authority
[ ] app_runtime remains least privileged
[ ] worker is explicitly least privileged
[ ] runtime cannot rewrite audit evidence
[ ] worker cannot rewrite immutable outbox fields
[ ] RLS is real and forced where required
[ ] real PostgreSQL tests prove RLS behavior
[ ] real PostgreSQL proves rollback atomicity
[ ] worker concurrency is tested
[ ] failure injection is tested
[ ] backlog/latency/worker metrics exist
[ ] secrets do not enter logs
[ ] runbooks exist
[ ] no Phase 5/6 scope creep
[ ] canonical CI gates actually pass
[ ] actual command evidence is recorded
```

---

# 28. IMPLEMENTATION REPORT REQUIRED IN THE PR

Before marking PR #49 ready, update the PR body with:

```text
Implemented:
  exact source files and behavior

Audit:
  catalogue, persistence, mandatory semantics

Outbox:
  schema, claim, retry, quarantine, delivery semantics

Security:
  tenant isolation, RLS, roles, immutability, secret handling

Transaction:
  exact business + audit + outbox atomicity path

Tests:
  exact commands and actual results

PostgreSQL:
  exact environment and real-database evidence

Worker:
  claim, lease, retry and recovery behavior

Observability:
  metrics, logs, alerts

Not verified:
  anything not actually exercised

Deferred:
  explicit Phase 5/6 work

Operational impact:
  recovery and failure behavior
```

No placeholder checkbox is evidence.

No documentation-only claim is evidence.

---

# 29. RESEARCH BASIS

The remediation keeps the repository aligned with the established architectural choice and current authoritative guidance.

## Transactional outbox

The transactional outbox pattern stores the event in the same database transaction as the state change and uses a separate relay. The relay can publish more than once, so consumers must be idempotent. citeturn138896search0

## Idempotent consumers

A consumer can maintain durable processed-message identity so repeated delivery does not repeat the business side effect. citeturn138896search1

## OWASP security logging

OWASP recommends capturing security-relevant events such as authorization failures while minimizing or removing passwords, access tokens, connection strings and sensitive personal data, and protecting stored logging data from unauthorized modification/deletion. citeturn138896search4turn138896search6

## PostgreSQL RLS and role security

PostgreSQL RLS can restrict SELECT/INSERT/UPDATE/DELETE independently by policy; table owners normally bypass RLS unless forced, and roles with `BYPASSRLS` always bypass it. That makes ownership and privilege separation part of the actual security boundary, not administrative detail. citeturn147448search1turn147448search3

## PostgreSQL concurrent claiming

PostgreSQL row locks and `SKIP LOCKED` can be used for bounded concurrent work claiming when lock contention must not serialize worker selection. citeturn147448search0turn147448search5

---

# 30. FINAL EXECUTION DIRECTIVE

Implement this document **against**:

`docs/phase4_part8_audit_outbox_implementation_contract.md`

Do not reinterpret the contract.

Do not add superficial code solely to increase the diff.

Do not replace real PostgreSQL behavior with mocks.

Do not use permissive worker privileges to make implementation easier.

Do not publish externally inside the authoritative transaction.

Do not make the worker re-run commands.

Do not invent retention periods.

Do not claim production readiness without executable evidence.

The target architecture is:

```text
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
AUTHORITATIVE POSTGRES TX
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
              WORKER/RELAY
                  |
                  v
          IDEMPOTENT CONSUMER
```

**The job is not to make PR #49 look complete. The job is to make the repository prove Part 8 is actually implemented.**
