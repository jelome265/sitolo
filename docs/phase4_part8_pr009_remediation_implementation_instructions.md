# SITOLO — PR-009 / PHASE 4 PART 8 IMPLEMENTATION AUDIT + REMEDIATION INSTRUCTIONS

**Primary contract:** `docs/phase4_part8_audit_outbox_implementation_contract.md`  
**Reviewed PR:** #49 — `feat(security): Phase 4 Part 8 audit and outbox implementation`  
**PR head reviewed:** `6cfab2b219dd74384115546380ba98ca4c19b4bb`  
**PR base:** `main` at `30c7f5ae1a09c2142939b4535f8bf3d87fad9b41`  
**Review date:** 2026-09-24  
**Status:** **NOT READY — implementation is materially incomplete**  
**Purpose:** Concrete remediation contract for completing PR-009 against the main Part 8 implementation contract.

---

# 1. EXECUTIVE FINDING

PR #49 implements the **type-level beginnings** of the Part 8 audit/outbox boundary, but it does **not implement the durable audit/outbox system required by the binding contract**.

The PR currently adds:

- IAM audit event types;
- an outbox event type;
- outbox traits;
- a PostgreSQL fixture schema;
- four small tests.

It does **not** currently implement the critical runtime path:

```text
authorized command
      ↓
authoritative PostgreSQL transaction
      ↓
business mutation
      +
mandatory audit insert
      +
outbox insert
      ↓
COMMIT
      ↓
worker claim
      ↓
external publication
      ↓
PUBLISHED / retry / quarantine
```

The current PR has no production PostgreSQL audit repository, no production PostgreSQL outbox repository, no transactionally coupled application mutation path, and no worker relay implementation.

The PR description claims:

> “Comprehensive Tests”

and describes atomicity, security and PostgreSQL behavior that the diff does not actually implement or prove.

That claim must not remain uncorrected in implementation evidence.

---

# 2. CI FACT

The PR head's canonical Rust verification failed.

Workflow:

```
rust
run: 35970982273
verify job: 107540403686
```

The failure occurs immediately at the repository lockfile gate:

```
error: cannot update the lock file .../Cargo.lock because --locked was passed to prevent this
```

Therefore:

```
canonical verification = FAIL
```

Security workflow:

```
run: 35970982283
conclusion: success
```

Security success does **not** compensate for the failed canonical Rust verification.

The PR must not be treated as passing until the lockfile is synchronized and the complete verification pipeline passes.

---

# 3. CONTRACT COMPLIANCE MATRIX

| Main contract requirement | PR #49 | Status |
|---|---|---|
| IAM event catalogue | enum added | Partial |
| Secret-free audit model | struct exists | Partial |
| Durable audit persistence | no implementation | **FAIL** |
| Transactional audit recording | no implementation | **FAIL** |
| Durable outbox persistence | no implementation | **FAIL** |
| Same DB transaction as mutation | no implementation | **FAIL** |
| At-least-once delivery | documented only | **FAIL** |
| Worker claim/recovery | trait only | **FAIL** |
| Retry classification | enum only | **FAIL** |
| Backoff | absent | **FAIL** |
| Quarantine | trait only | **FAIL** |
| Duplicate delivery handling | absent | **FAIL** |
| Aggregate ordering | absent | **FAIL** |
| Event registry | absent | **FAIL** |
| Event schema validation | minimal/absent | **FAIL** |
| Real PostgreSQL integration | fixture only | **FAIL** |
| RLS tenant isolation | insert-only fixture policies | Partial |
| Audit immutability | no DB enforcement of all required operations | **FAIL** |
| Worker privilege boundary | broad `USING (true)` / `WITH CHECK (true)` | **FAIL** |
| Failure injection | absent | **FAIL** |
| Concurrency tests | absent | **FAIL** |
| Atomicity tests | absent | **FAIL** |
| Observability | absent | **FAIL** |
| Operational runbooks | contract only | Partial |
| Canonical CI | failed | **FAIL** |
| Phase 3 compatibility | source unchanged | Partial |
| Phase 5 boundary | fixture placed under test-support, but production persistence absent | Partial |

---

# 4. CRITICAL GAP — THIS IS CURRENTLY AN API/TYPE PR, NOT A DURABLE IMPLEMENTATION

The new `OutboxWriter` and `OutboxReader` traits do not constitute an implementation.

Current:

```rust
pub trait OutboxWriter {
    async fn enqueue(...);
}

pub trait OutboxReader {
    async fn claim_batch(...);
    async fn mark_published(...);
    async fn release(...);
    async fn quarantine(...);
}
```

Required:

```text
application transaction
        ↓
PostgreSQL transaction object
        ↓
business repository
        ↓
AuditRepository::record_required(&mut tx, ...)
        ↓
OutboxRepository::enqueue(&mut tx, ...)
        ↓
COMMIT
```

The traits must be wired into real persistence and application code.

Do not stop at interfaces.

---

# 5. CRITICAL GAP — NO TRANSACTIONAL COUPLING

The main contract explicitly requires:

> business mutation + mandatory audit + outbox intent share the same authoritative PostgreSQL transaction.

PR #49 contains no application/service implementation proving this.

Implement at least one real Phase 4 mutation path end-to-end and use it as the reference composition pattern.

Required shape:

```text
BEGIN
  |
  +-- trusted scope
  |
  +-- authorization
  |
  +-- business mutation
  |
  +-- mandatory audit insert
  |
  +-- outbox insert
  |
COMMIT
```

If any operation fails:

```text
ROLLBACK
  |
  +-- no business mutation
  +-- no audit record
  +-- no outbox record
```

Do not simulate this with mocks.

---

# 6. CRITICAL GAP — POSTGRESQL SCHEMA IS ONLY A TEST FIXTURE

The file:

```
crates/sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql
```

creates tables, but no production persistence implementation consumes them.

The contract permits test fixtures where appropriate, but the fixture cannot substitute for production durability.

Implement the persistence boundary through the repository's canonical PostgreSQL architecture.

Do not create a second migration framework.

If Phase 5 owns canonical migrations, keep the production schema authority consistent with that boundary and do not silently fork migration ownership.

---

# 7. CRITICAL GAP — THE CLAIMED “INTEGRATION TESTS” ARE NOT INTEGRATION TESTS

The current file:

```
crates/sitolo-persistence/tests/part8_audit_outbox_tests.rs
```

contains four ordinary Rust tests.

They do not:

- connect to PostgreSQL;
- execute the fixture schema;
- create/use runtime roles;
- establish RLS context;
- insert audit rows;
- insert outbox rows;
- test transaction rollback;
- test row-level security;
- test worker claims;
- test concurrent claims.

Rename/restructure the tests only if necessary, but more importantly **make them real integration tests**.

A test proving:

```rust
OutboxStatus::Pending.as_str() == "PENDING"
```

does not prove an outbox implementation.

---

# 8. CRITICAL GAP — AUDIT IMMUTABILITY

The main contract requires normal runtime roles to be unable to:

```text
UPDATE audit
DELETE audit
TRUNCATE audit
rewrite actor
rewrite tenant
rewrite event identity
```

The fixture currently grants only INSERT to `app_runtime`, which is directionally correct, but there are no executable privilege assertions proving the negative cases.

Add real PostgreSQL tests:

```text
app_runtime INSERT audit -> allowed
app_runtime SELECT audit -> according to intended access contract
app_runtime UPDATE audit -> denied
app_runtime DELETE audit -> denied
app_runtime TRUNCATE audit -> denied
app_runtime ALTER audit -> denied
app_runtime ownership manipulation -> denied
```

Also prove the table owner is not `app_runtime`.

---

# 9. CRITICAL GAP — WORKER RLS POLICY IS TOO BROAD

The fixture contains:

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

This is a major security boundary.

The contract explicitly says:

> event tenant IDs are routing metadata, not authority.

A worker may legitimately require broader internal access than a tenant-facing runtime role, but `true/true` must be justified by an explicit worker trust model, privilege review and constrained operations.

Do not blindly accept this policy.

Investigate:

1. Does the worker need cross-tenant visibility?
2. Is the worker dedicated to outbox relay only?
3. Can it UPDATE arbitrary columns?
4. Can it change organization_id?
5. Can it change event_id?
6. Can it change payload?
7. Can it delete rows?
8. Can it grant itself broader access?
9. Is it the table owner?
10. Does it have BYPASSRLS?
11. Can an application path authenticate as it?

Prefer narrow SQL operations/procedures or restricted UPDATE privileges if compatible with the architecture.

At minimum, prove that the worker cannot use its privilege to rewrite immutable event identity or tenant ownership.

---

# 10. CRITICAL GAP — OUTBOX STATE MACHINE IS ONLY AN ENUM

The contract requires actual state transitions:

```text
PENDING
  ↓
CLAIMED
  ↓
PUBLISHED

PENDING
  ↓
CLAIMED
  ↓
PENDING   (retry)

PENDING
  ↓
CLAIMED
  ↓
QUARANTINED
```

The PR currently defines only:

```rust
enum OutboxStatus
```

Implement actual transition rules.

Invalid transitions must fail:

```text
PUBLISHED -> PENDING
PUBLISHED -> CLAIMED
QUARANTINED -> PUBLISHED
QUARANTINED -> CLAIMED
```

unless a deliberately specified administrative replay path exists.

Replay must preserve event identity.

---

# 11. CRITICAL GAP — CLAIMING CONCURRENCY IS NOT IMPLEMENTED

The contract requires:

> worker A claims event E; worker B cannot simultaneously claim E.

Implement real PostgreSQL claiming.

The design should use PostgreSQL concurrency primitives appropriate to the repository's architecture, typically a bounded claim query using row locking semantics rather than an application-only mutex.

The implementation must:

- claim only eligible events;
- bound batch size;
- prevent simultaneous claims;
- record lease/lock state;
- allow lease recovery;
- avoid holding a DB transaction during external network I/O.

Add a multi-worker integration test.

---

# 12. CRITICAL GAP — LEASE RECOVERY IS ABSENT

The contract requires recovery after worker death.

Current model has `locked_at`, but no implementation.

Define:

```text
CLAIMED
+
lease expired
        ↓
eligible again
        ↓
PENDING/CLAIMABLE
```

The exact representation can use the existing status model, but the invariant must be explicit.

Test:

1. worker A claims;
2. worker A disappears;
3. lease expires;
4. worker B reclaims;
5. event is delivered;
6. no permanent loss occurs.

---

# 13. CRITICAL GAP — RETRY/BACKOFF IS ABSENT

The PR defines:

```text
Retryable
Terminal
Unknown
```

but provides no actual retry policy.

Implement:

- retry classification;
- attempt increment;
- bounded retry count/deadline;
- next available time;
- exponential/backoff policy;
- jitter where appropriate;
- terminal classification;
- quarantine after exhaustion;
- safe error classification without secrets.

Do not implement infinite retry.

---

# 14. CRITICAL GAP — EVENT PAYLOAD CONTRACT IS WEAK

The Rust type uses:

```rust
payload: String
```

while the database fixture uses:

```sql
payload JSONB
```

This creates an avoidable representation mismatch.

Use a typed serialization boundary that guarantees:

```text
valid JSON
bounded size
stable schema
versioned payload
secret-free content
deterministic serialization
```

Do not rely on arbitrary strings.

Validate the event name/version/payload before persistence.

---

# 15. CRITICAL GAP — SECRET-FREE TEST IS NOT A SECURITY PROPERTY

Current test:

```rust
let debug = format!("{:?}", event);
assert!(!debug.contains("password"));
assert!(!debug.contains("Bearer"));
assert!(!debug.contains("secret"));
```

This is insufficient.

It only proves those literal strings do not appear in one Debug rendering of one test object.

It does not prevent future developers from adding:

```text
password_value
access_token
otp
refresh_token
raw request body
```

to the event model.

Implement a construction boundary that makes sensitive fields unavailable to the durable event type, or explicitly validates/redacts structured metadata.

Add tests for representative secret-bearing inputs.

Never log the secret itself merely to prove it was rejected.

---

# 16. CRITICAL GAP — METADATA FIELD IS MISSING

The main contract's conceptual audit shape includes:

```text
metadata
```

The current `IamAuditEvent` has no metadata.

Do not blindly add an unbounded `HashMap<String, String>`.

Define bounded, typed metadata appropriate to the existing architecture.

Requirements:

- bounded size;
- bounded key count;
- bounded key/value lengths;
- no secrets;
- no raw request body;
- deterministic serialization.

---

# 17. CRITICAL GAP — EVENT REGISTRY IS ABSENT

The contract requires one authoritative registry defining:

```text
name
version
category
producer
mandatory/diagnostic
tenant scope
payload schema
sensitive fields
consumers
retry semantics
retention class
```

Currently event names are an enum and strings, but there is no complete registry.

Implement the smallest repository-consistent registry that makes durable event contracts discoverable and testable.

Do not create duplicated registries.

---

# 18. CRITICAL GAP — EVENT ORDERING IS ABSENT

The contract explicitly rejects wall-clock time as the ordering authority.

Current outbox schema has `occurred_at`, but no aggregate sequence/order mechanism.

Define an aggregate-local ordering strategy.

Requirements:

- deterministic ordering for events belonging to the same aggregate;
- no false promise of global ordering;
- concurrent mutations cannot silently create ambiguous order;
- consumer can identify ordering position.

Use PostgreSQL-backed ordering if the ordering invariant depends on database concurrency.

---

# 19. CRITICAL GAP — AUDIT/OUTBOX CORRELATION IS INCOMPLETE

Audit includes:

```text
request_id
trace_id
```

but constructors always set them to `None`.

The application transaction must be able to propagate correlation context into the durable evidence.

Do not make tracing mandatory if the existing architecture does not require it, but where the repository already has request/trace identity, do not discard it at the audit boundary.

---

# 20. CRITICAL GAP — FAILURE INJECTION IS ABSENT

The main contract requires testing failures after:

```text
business mutation
audit insert
outbox insert
before commit
after worker claim
after external publication before mark-published
```

Implement deterministic failure injection in test support.

The most important invariant:

```text
business mutation succeeds
audit fails
        ↓
ROLLBACK
```

and:

```text
business mutation succeeds
audit succeeds
outbox fails
        ↓
ROLLBACK
```

Prove database state after each failure.

---

# 21. CRITICAL GAP — DUPLICATE DELIVERY IS ABSENT

At-least-once delivery means:

```text
publish
↓
worker crashes
↓
event remains/reappears eligible
↓
publish again
```

Implement a deterministic consumer/idempotency test.

Do not claim exactly-once delivery.

The durable event ID must remain stable across retry/replay.

---

# 22. CRITICAL GAP — WORKER IS STILL A PHASE 1 SCAFFOLD

Current:

```text
apps/worker/src/main.rs
fn main() {}
```

The Part 8 contract explicitly defines the worker/relay boundary.

The implementation must now provide the minimum worker functionality required by PR-009:

```text
load eligible events
      ↓
claim bounded batch
      ↓
validate
      ↓
publish through an explicit delivery boundary
      ↓
mark published
      ↓
retry / quarantine on failure
```

Do not build a generic job platform.

Do not add Kafka/RabbitMQ merely because an outbox exists.

A narrow internal relay abstraction is sufficient for this phase.

---

# 23. CRITICAL GAP — OBSERVABILITY IS ABSENT

The main contract requires bounded operational metrics.

Implement at minimum the repository's existing observability equivalent for:

```text
pending outbox count
oldest pending age
processing duration
success count
retry count
terminal failure count
quarantine count
duplicate delivery count
mandatory audit failures
audit write latency
```

Do not put organization IDs, user IDs, email addresses or event payloads into metric labels.

Use the existing observability crate/boundary.

---

# 24. CRITICAL GAP — AUDIT EVENT CATALOGUE IS INCOMPLETE IN SEMANTICS

The enum contains the named events, which is good, but names alone do not define:

```text
mandatory vs diagnostic
actor semantics
target semantics
tenant scope
payload contract
producer
retention class
consumer
failure semantics
```

Add tests and registry metadata for these semantics.

Do not expand the event catalogue beyond Phase 4 unless an existing governing document requires it.

---

# 25. CRITICAL GAP — PHASE 4 DEVICE EVENTS ARE PREMATURE

The PR adds:

```text
iam.device.bound
iam.device.unbound
iam.device.suspended
iam.device.revoked
```

but device binding is explicitly a later Phase 4 part.

It is acceptable to define future event names only if they are clearly registry placeholders and do not imply the device feature has been implemented.

Do not add device runtime behavior to PR-009.

---

# 26. LOCKFILE MUST BE FIXED

The PR adds dependencies to `sitolo-events`:

```text
async-trait
sitolo-domain
thiserror
```

but the repository lockfile is stale.

Synchronize `Cargo.lock` intentionally.

Then run:

```text
cargo check --locked
./scripts/ci/verify
```

Do not remove `--locked` from CI to hide the problem.

---

# 27. SECURITY RESEARCH UPDATE — POSTGRESQL 18.6

Current CI is using PostgreSQL 18.6.

That is important because PostgreSQL published CVE-2026-14666 describing stale row-security policy caching after role modifications. PostgreSQL lists 18.6 as the fixed release. PR-009 must preserve the repository's existing least-privilege/RLS discipline and should include connection/role-transition tests where relevant to its worker/runtime role model.

Source:
https://www.postgresql.org/support/security/CVE-2026-14666/

Do not claim that upgrading to 18.6 alone makes the application secure. It only establishes that this particular PostgreSQL vulnerability is addressed by the tested server version.

---

# 28. SECURITY RESEARCH UPDATE — POSTGRESQL LOGGING

PostgreSQL's current documentation warns that statement logging can expose sensitive values, including plaintext passwords when statements/bind parameters are logged.

Therefore PR-009 must ensure audit payloads and worker diagnostics never depend on raw SQL logging as evidence and must not introduce secret-bearing event payloads.

Source:
https://www.postgresql.org/docs/18/runtime-config-logging.html

---

# 29. TRANSACTIONAL OUTBOX RESEARCH CONFIRMATION

Current AWS guidance confirms the core transactional-outbox invariant:

- business update and outbox write occur in the same transaction;
- rolled-back transactions must not produce published events;
- duplicate publication is possible;
- consumers should be idempotent.

Source:
https://docs.aws.amazon.com/en_en/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html

This independently supports the main contract's transaction and delivery model.

---

# 30. REQUIRED IMPLEMENTATION PLAN

Implement in this order.

## Phase A — fix foundation

1. Synchronize Cargo.lock.
2. Ensure all new crates compile.
3. Preserve dependency direction.
4. Reconcile event types with existing Phase 3 audit semantics.

## Phase B — harden event contracts

5. Add typed event metadata.
6. Define durable event registry.
7. Define schema/version semantics.
8. Define payload serialization/validation.
9. Define aggregate ordering.
10. Define secret rejection/redaction.

## Phase C — production audit persistence

11. Implement PostgreSQL audit repository.
12. Use the existing transaction abstraction.
13. Insert mandatory audit records inside the caller's transaction.
14. Prove append-only privilege boundaries.
15. Prove tenant isolation.

## Phase D — production outbox persistence

16. Implement PostgreSQL outbox repository.
17. Implement atomic enqueue.
18. Implement claim with real PostgreSQL concurrency semantics.
19. Implement lease recovery.
20. Implement mark-published.
21. Implement retry scheduling.
22. Implement quarantine.

## Phase E — application composition

23. Select representative Phase 4 commands.
24. Integrate mutation + audit + outbox in one transaction.
25. Prove rollback on audit failure.
26. Prove rollback on outbox failure.
27. Prove successful commit produces exactly the expected durable evidence.

## Phase F — worker

28. Implement narrow relay.
29. Add bounded batch processing.
30. Add delivery abstraction.
31. Add retry/terminal classification.
32. Add duplicate-delivery behavior.
33. Add recovery after worker death.

## Phase G — security/concurrency tests

34. Real PostgreSQL tenant isolation.
35. Real privilege assertions.
36. Worker claim race.
37. Lease expiry recovery.
38. Atomicity failure injection.
39. Duplicate delivery.
40. Invalid event/schema handling.
41. Secret-bearing payload rejection.

## Phase H — observability

42. Add bounded metrics.
43. Add safe structured worker diagnostics.
44. Add backlog/age visibility.
45. Add failure/quarantine visibility.

## Phase I — verification

46. Run formatting.
47. Run clippy with warnings denied.
48. Run workspace tests.
49. Run real PostgreSQL integration suite.
50. Run canonical `./scripts/ci/verify`.
51. Run cargo audit/deny gates.
52. Record exact successful run IDs.
53. Review the final diff against the main Part 8 contract again.

---

# 31. REQUIRED TEST MATRIX

The implementation is not complete until these are executable.

### Atomicity

```text
business fail
business + audit success
business success + audit fail
business + audit success + outbox fail
business + audit + outbox success
```

### Tenant isolation

```text
A -> A audit allowed according to access contract
A -> B audit denied
A -> B outbox denied
wrong organization_id in event -> denied
missing tenant context -> denied
invalid tenant context -> denied
```

### Privileges

```text
runtime INSERT audit -> allowed
runtime UPDATE audit -> denied
runtime DELETE audit -> denied
runtime TRUNCATE audit -> denied
ordinary API claim outbox -> denied
worker claim -> allowed
worker rewrite immutable identity -> denied
```

### Concurrency

```text
worker A / worker B same event
worker crash after claim
lease expiry
concurrent membership mutations
concurrent idempotent command retry
```

### Delivery

```text
success -> published
temporary failure -> retry
terminal failure -> quarantine
publish + crash -> duplicate possible
duplicate -> consumer remains correct
```

### Data safety

```text
oversized payload -> rejected
invalid schema -> rejected
unknown version -> safe failure/quarantine
secret-bearing payload -> rejected
malformed event -> cannot poison worker
```

---

# 32. REQUIRED EVIDENCE

Final implementation evidence must contain:

```text
implementation commit SHA
base SHA
CI run IDs
exact commands
PostgreSQL version
number of tests
number passed
number failed
number skipped
security assertions
RLS assertions
privilege assertions
atomicity evidence
worker concurrency evidence
retry/quarantine evidence
duplicate-delivery evidence
```

Never write “comprehensive tests” unless the tests actually execute the properties claimed.

---

# 33. DO NOT DO

Do not:

- modify the PR description merely to make it look complete;
- mark a unit test as a PostgreSQL integration test;
- add a mock and call it RLS evidence;
- bypass RLS with superuser privileges;
- give the worker BYPASSRLS for convenience;
- implement a generic event bus;
- introduce Kafka/RabbitMQ;
- create a second migration system;
- build Phase 5 wholesale;
- implement device binding;
- implement ownership transfer;
- implement support/admin separation;
- implement generalized Phase 6 authorization;
- add arbitrary retention periods;
- claim exactly-once delivery;
- store secrets in events;
- use raw Debug output as a security boundary;
- suppress the failing lockfile gate.

---

# 34. FINAL ACCEPTANCE CRITERIA

PR-009 can be considered ready only when the following are true:

```text
[ ] Cargo.lock synchronized
[ ] canonical CI passes
[ ] real audit persistence exists
[ ] real outbox persistence exists
[ ] mandatory audit is transactionally coupled
[ ] outbox enqueue is transactionally coupled
[ ] rollback is proven
[ ] PostgreSQL RLS/tenant isolation is proven
[ ] audit immutability is proven
[ ] worker privileges are proven least-privileged
[ ] claim concurrency is proven
[ ] lease recovery is proven
[ ] retry/backoff is implemented
[ ] quarantine is implemented
[ ] duplicate delivery is tested
[ ] event identity is stable
[ ] event versioning is enforced
[ ] aggregate ordering is explicit
[ ] payloads are bounded and typed
[ ] secret rejection is structural/tested
[ ] event registry exists
[ ] worker relay exists
[ ] bounded observability exists
[ ] failure injection exists
[ ] operational recovery paths are documented
[ ] no later Phase 4/5/6 work is absorbed
[ ] final evidence matches the actual implementation
```

---

# 35. SHORT EXECUTION INSTRUCTION

**Implement PR-009 against `docs/phase4_part8_audit_outbox_implementation_contract.md` — do not merely add types. Build the real PostgreSQL transaction path, durable audit/outbox persistence, worker claim/retry/quarantine, tenant/privilege boundaries, failure/concurrency tests, and evidence. Fix Cargo.lock first. No mocks for database security properties, no fake integration tests, no superuser/BYPASSRLS shortcuts, no future-phase creep. Keep the implementation narrow, production-grade, and prove every contract requirement with executable tests before calling PR-009 complete.**
