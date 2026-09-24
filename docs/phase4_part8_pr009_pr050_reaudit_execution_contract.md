
# SITOLO — PHASE 4 PART 8 PR-009 / PR #50 RE-AUDIT EXECUTION CONTRACT

**Primary binding contract:** docs/phase4_part8_audit_outbox_implementation_contract.md  
**PR:** #50 — Phase 4 Part 8: Audit and Outbox Implementation  
**Branch:** feat/phase4-part8-audit-outbox-12138843190386424755  
**Repository:** jelome265/sitolo  
**Current PR head reviewed:** 31ef0b92237f32f1b0a535e0c8c2a98835bed726  
**Base:** 2902dfe427963e33d90d22c0a7be1ce4c7193760  
**Review date:** 24 September 2026  
**Status:** NOT READY — substantially improved, but Part 8 security/correctness invariants are not fully proven

> This document is subordinate to the binding Part 8 implementation contract. It records the current PR #50 findings and is the execution checklist for the next implementation pass.

---

# 0. EXECUTIVE VERDICT

PR #50 is materially better than the previous PR.

Implemented now:

~~~text
PostgreSQL audit/outbox persistence boundary
transaction-aware write methods
outbox claim path using SKIP LOCKED
lease recovery
retry/quarantine transitions
worker relay library
real PostgreSQL integration-test scaffolding
atomicity rollback tests
cross-tenant audit test
Cargo.lock synchronization
green Rust workflow
green security workflow
~~~

The PR is still not complete against the main Part 8 contract.

The remaining blockers are:

~~~text
P0  app_runtime gets blanket SELECT/INSERT/UPDATE/DELETE on all fixture tables
P0  audit RLS has fail-open NULL/missing-context branches
P0  outbox RLS permits broad runtime access and mutation
P0  worker database authority is not established/proven
P0  worker executable does not actually run the relay
P0  claim ownership has no claim token or lease generation
P0  stale worker can potentially finish a newer claim
P0  no real Phase 4 application command composes business + audit + outbox
P1  aggregate ordering is absent
P1  enqueue accepts worker-owned lifecycle state
P1  payload bounds are not enforced end-to-end
P1  event registry/version/schema validation is incomplete
P1  audit metadata is an unvalidated String
P1  duplicate-delivery test is not consumer idempotency
P1  retry policy is incomplete
P1  production observability is incomplete
P1  architecture/policy CI scripts can report success after rg is unavailable
~~~

Therefore:

~~~text
CI health: green
implementation progress: substantial
Part 8 contract compliance: not proven
merge readiness: NO
~~~

---

# 1. CI EVIDENCE

For the reviewed head 31ef0b92237f32f1b0a535e0c8c2a98835bed726:

~~~text
policy    SUCCESS
rust      SUCCESS
security  SUCCESS
~~~

Rust verification ran PostgreSQL 18.6 and reported:

~~~text
audit_outbox_integration_tests: 6 passed
rls_security_tests: 27 passed
release build: passed
~~~

Security workflow reported successful CodeQL, dependency-policy, secret scanning and advisory jobs.

This is real positive evidence.

It does not prove every Part 8 property because the current tests do not exercise every required negative and failure case.

---

# 2. CI FALSE-GREEN RISK

The successful verification log contains:

~~~text
./scripts/ci/check-architecture: rg: command not found
Architecture check PASSED

./scripts/ci/check-phase2-policy: rg: command not found
Phase 2 policy check PASSED
~~~

The scripts are therefore able to report success when a required search tool is unavailable.

Required:

~~~text
install required rg tooling
or explicitly detect missing tooling
or exit non-zero when the tool is unavailable
rerun all checks
record real evidence
~~~

Do not delete the checks or weaken their failure semantics.

---

# 3. P0 — APP_RUNTIME BLANKET TABLE GRANTS

The new integration harness executes:

~~~sql
GRANT USAGE ON SCHEMA "<schema>" TO app_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE
ON ALL TABLES IN SCHEMA "<schema>" TO app_runtime;
~~~

This grants broad access to:

~~~text
iam_audit_records
outbox_events
~~~

That is incompatible with the Part 8 principle that audit and outbox are explicit security-sensitive infrastructure.

Required target:

~~~text
app_runtime
  business mutation privileges as required
  audit INSERT only
  outbox INSERT only

app_runtime must NOT:
  UPDATE audit
  DELETE audit
  TRUNCATE audit
  administratively UPDATE outbox
  DELETE outbox
  rewrite durable event contents
~~~

Use explicit table and column privileges, not blanket ALL-TABLE grants.

---

# 4. P0 — AUDIT RLS MUST FAIL CLOSED

Current audit policy is effectively:

~~~text
organization_id IS NULL
OR tenant context is NULL
OR organization_id = tenant context
~~~

This makes missing context and NULL organization values eligible.

Part 8 requires:

~~~text
missing context -> deny
invalid context -> deny
cross-tenant context -> deny
~~~

For tenant-scoped IAM evidence, prefer a non-null organization association and an exact trusted-context predicate unless the governing contract explicitly defines a global audit event.

Never turn missing security context into permission.

---

# 5. P0 — OUTBOX RLS / PRIVILEGES ARE TOO BROAD

The current outbox policy is conceptually:

~~~sql
FOR ALL
TO app_runtime
USING (true)
~~~

combined with blanket runtime UPDATE/DELETE grants from the test harness.

This leaves application runtime capable of mutating infrastructure state.

Required target:

~~~text
runtime:
  enqueue only

worker:
  claim
  mark published
  retry
  quarantine
  recover stale lease

ordinary runtime:
  no administrative outbox mutation
~~~

The tenant identifier in the event remains routing metadata, not authorization proof.

---

# 6. P0 — WORKER AUTHORITY IS NOT PROVEN

The new relay library exists, but the integration suite does not establish a dedicated worker connection and assert:

~~~sql
SELECT current_user;
~~~

The current persistence store is constructed from the runtime pool for the integration tests.

Therefore the PR does not prove that worker behavior executes under a least-privileged worker role.

Required:

~~~text
dedicated app_worker role
explicit worker credentials
current_user assertion
role attribute assertions
no BYPASSRLS
no unintended memberships
not owner of protected tables
only intended worker privileges
negative immutable-field tests
~~~

Do not call runtime-role tests worker security evidence.

---

# 7. P0 — WORKER EXECUTABLE IS STILL INERT

apps/worker/src/main.rs currently only initializes Tokio and logs startup.

It does not:

~~~text
load validated worker configuration
connect to the worker database
construct the real persistence store
construct the delivery adapter
recover stale claims
claim events
dispatch events
mark success
retry/quarantine failures
poll with bounded timing
handle graceful shutdown
~~~

The worker library is not enough.

Required executable lifecycle:

~~~text
startup
 -> validate config
 -> connect with worker authority
 -> recover stale claims
 -> claim bounded batch
 -> dispatch after durable commit
 -> mark success/retry/quarantine
 -> metrics/logging
 -> bounded wait
 -> graceful shutdown
~~~

Keep this worker narrow. Do not introduce a generic job platform.

---

# 8. P0 — CLAIM OWNERSHIP IS NOT SAFE AFTER LEASE EXPIRY

Current APIs operate approximately as:

~~~text
mark_published(event_id)
mark_failed_or_quarantined(event_id, ...)
~~~

There is no claim token or lease generation.

Race:

~~~text
worker A claims event E
        |
lease expires
        |
worker B claims E
        |
worker A finishes old delivery
        |
worker A marks E published
~~~

The current state check does not prove which worker owns the active claim.

Required:

~~~text
event_id
+
claim_token OR lease_generation
+
expected state
~~~

Every worker-owned transition must verify the current claim identity.

Required test:

~~~text
A claims
A lease expires
B reclaims
A tries completion -> rejected
B completes -> accepted
~~~

---

# 9. P0 — ATOMICITY PROOF IS DATABASE-LEVEL, NOT APPLICATION-LEVEL

The new tests now do something meaningful:

~~~text
tenant_resources mutation
+
audit insert
+
outbox insert
=
same PostgreSQL transaction
~~~

That is useful.

However, the main contract requires the actual Phase 4 application boundary:

~~~text
authenticated principal
 -> authorized scope
 -> application command
 -> domain invariant
 -> PostgreSQL transaction
 -> IAM mutation
 -> mandatory audit
 -> outbox
 -> COMMIT
~~~

No current test demonstrates a real Phase 4 IAM command through that complete application path.

Required reference implementation:

~~~text
membership activation
or membership revocation
or role assignment/removal
or scope grant/revocation
or organization/branch lifecycle transition
~~~

Then prove audit/outbox coupling through the actual application orchestration.

Do not make tenant_resources the permanent feature implementation merely because it is a convenient fixture.

---

# 10. P0 — FAILURE INJECTION IS STILL INCOMPLETE

The current rollback tests induce:

~~~text
duplicate audit event ID
duplicate outbox deduplication key
~~~

These are valid database failures.

The contract additionally requires deterministic failure injection around:

~~~text
after business write
after audit write
after outbox write
before commit
~~~

Implement test-only failure seams that exercise these exact boundaries.

Core invariants:

~~~text
audit failure
 -> business absent
 -> audit absent
 -> outbox absent

outbox failure
 -> business absent
 -> audit absent
 -> outbox absent
~~~

Do not rely solely on constraint collisions.

---

# 11. P1 — AGGREGATE ORDERING IS MISSING

Current OutboxEvent contains aggregate_type and aggregate_id, but no aggregate-local sequence.

The current integration fixture also has no aggregate_sequence and no uniqueness boundary.

Part 8 requires deterministic ordering per aggregate.

Required:

~~~text
aggregate_type
aggregate_id
aggregate_sequence
UNIQUE(aggregate_type, aggregate_id, aggregate_sequence)
~~~

Use a concurrency-safe database allocation mechanism.

Do not use occurred_at as the ordering authority.

---

# 12. P1 — ENQUEUE MUST NOT ACCEPT WORKER-OWNED STATE

The enqueue API currently persists caller-provided:

~~~text
status
attempt_count
locked_at
published_at
last_error_class
~~~

A new event must not be able to enter the durable system already CLAIMED, PUBLISHED or QUARANTINED.

Required invariant:

~~~text
new outbox event:
  PENDING
  attempt_count = 0
  locked_at = NULL
  published_at = NULL
  no worker error state
~~~

Worker processing state must be worker-owned.

---

# 13. P1 — PAYLOAD LIMIT IS NOT ENFORCED END-TO-END

The current OutboxEvent payload is a String.

The PR description claims a bounded payload, but the current model does not provide a constructor enforcing the limit and the database uses TEXT.

Required:

~~~text
bounded at event construction
bounded at persistence
bounded at worker dispatch
negative oversized test
~~~

Do not rely on caller discipline.

---

# 14. P1 — EVENT REGISTRY AND SCHEMA VALIDATION ARE INCOMPLETE

The event-name enum is useful but insufficient.

Part 8 requires an authoritative contract for at least:

~~~text
event name
version
category
producer
mandatory/diagnostic
tenant scope
payload schema
consumer
retry semantics
retention class
ordering domain
~~~

Required behavior:

~~~text
unknown event -> reject safely
unknown version -> reject/quarantine safely
invalid payload -> reject/quarantine safely
unsupported schema -> no unsafe delivery
~~~

Do not turn outbox_events into arbitrary JSON transport.

---

# 15. P1 — AUDIT METADATA IS UNVALIDATED

Current audit metadata is:

~~~text
Option<String>
~~~

It has no explicit structural bounds.

Required:

~~~text
bounded size
bounded entry count
bounded key/value sizes
safe allowed fields
deterministic serialization
secret rejection
~~~

Never persist:

~~~text
passwords
OTP values
MFA secrets
recovery codes
access tokens
refresh tokens
Authorization headers
API keys
private keys
raw request bodies
~~~

OWASP logging guidance emphasizes useful security logging while minimizing sensitive information and protecting stored audit data.

Reference:
https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html

---

# 16. P1 — SECRET-FREE TESTING IS TOO SHALLOW

Current tests mostly inspect Debug output for words such as password, secret or token.

That does not establish a structural safety boundary.

Required:

~~~text
actual event-construction validation
actual metadata validation
actual oversized payload rejection
actual secret-bearing input rejection/redaction
~~~

The test should prove sensitive values cannot cross the durable boundary, not merely that certain literals are absent from one Debug string.

---

# 17. P1 — DUPLICATE DELIVERY IS NOT CONSUMER IDEMPOTENCY

Current tests cover claim and status behavior.

They do not reproduce:

~~~text
publish succeeds
worker crashes before durable success update
event becomes eligible again
same event_id is delivered twice
consumer sees duplicate
consumer performs one semantic side effect
~~~

Implement a fake consumer with a durable idempotency boundary.

At-least-once delivery means duplicate publication is an expected failure mode; exactly-once delivery must not be claimed.

References:
https://microservices.io/patterns/data/transactional-outbox
https://microservices.io/patterns/communication-style/idempotent-consumer.html

---

# 18. P1 — RETRY POLICY NEEDS A REAL POLICY OBJECT

The current relay exposes retry classification, max attempts and a fixed backoff.

Part 8 requires retry handling to be:

~~~text
classified
bounded
observable
deadline-aware
~~~

Define policy for:

~~~text
RETRYABLE
TERMINAL
UNKNOWN
~~~

with:

~~~text
attempt budget
bounded delay
next available time
quarantine
safe error class
~~~

Do not retry every error forever.

---

# 19. P1 — WORKER RESOURCE LIMITS MUST BE VALIDATED

Validate:

~~~text
1 <= batch_size <= configured maximum
lease_duration > 0 and bounded
backoff_duration bounded
max_attempts bounded
error_class bounded
integer conversions checked
~~~

The worker must never accept configuration/input that creates unbounded query or memory behavior.

---

# 20. P1 — IN-MEMORY STORE IS TEST SUPPORT ONLY

AuditOutboxDatabase is appropriate for unit tests.

Its methods ignore the PostgreSQL transaction parameter and mutate memory immediately.

Therefore:

~~~text
in-memory tests = local behavior
real PostgreSQL tests = persistence/security/transaction evidence
~~~

Do not use the in-memory implementation to justify RLS, durability, or transaction guarantees.

---

# 21. P1 — OBSERVABILITY IS NOT COMPLETE

The current OutboxRelayMetrics uses process-local atomic counters.

That is useful for unit tests.

Production observability should cover:

~~~text
outbox pending count
oldest pending age
processing duration
publish success
retry count
terminal failure
quarantine count
duplicate delivery
audit success/failure
mandatory audit failure
audit storage latency
authorization denial count
~~~

Do not put tenant IDs, user IDs, email addresses or event payloads into metric labels.

Use the established Sitolo observability boundary.

---

# 22. P1 — OPERATIONAL RUNBOOKS

Part 8 requires concrete runbooks for:

~~~text
audit write failure
outbox backlog
quarantined event
duplicate delivery
lease recovery
cross-tenant audit access
~~~

Each should define:

~~~text
symptoms
safe diagnostics
containment
recovery
replay controls
evidence preservation
~~~

Do not invent legal retention periods in this PR.

---

# 23. POSTGRESQL SECURITY RESEARCH

The current implementation uses direct table operations rather than the previous SECURITY DEFINER-function approach. Do not reintroduce privileged functions merely to make worker operations easier.

If privileged functions are introduced later, PostgreSQL 18 requires careful SECURITY DEFINER ownership and search_path handling, and newly created functions receive EXECUTE to PUBLIC by default unless revoked.

References:
https://www.postgresql.org/docs/18/sql-createfunction.html
https://www.postgresql.org/docs/18/perm-functions.html

For the current direct-table design, the focus is explicit least privilege, role separation, RLS defense in depth and claim ownership.

PostgreSQL documents BYPASSRLS and ownership semantics that can alter the effective RLS boundary.

References:
https://www.postgresql.org/docs/18/ddl-rowsecurity.html
https://www.postgresql.org/docs/18/role-attributes.html

PostgreSQL documents SKIP LOCKED as useful for queue-like concurrent processing, but this does not itself establish long-lived worker ownership after lease expiry.

Reference:
https://www.postgresql.org/docs/18/sql-update.html

---

# 24. REQUIRED IMPLEMENTATION ORDER

## Step 1 — Repair the database authority boundary

Fix:

~~~text
app_runtime grants
audit policy
outbox policy
worker role
ownership
~~~

## Step 2 — Make worker authority explicit

Implement:

~~~text
worker credential
current_user proof
bounded transition API
claim token/generation
stale-worker rejection
lease recovery
~~~

## Step 3 — Fix outbox state/order invariants

Implement:

~~~text
PENDING-only enqueue
worker-owned state
aggregate sequence
database uniqueness
~~~

## Step 4 — Integrate one real Phase 4 IAM command

Implement:

~~~text
authorization
trusted scope
one PostgreSQL transaction
business IAM mutation
mandatory audit
outbox
commit
~~~

## Step 5 — Make the executable worker real

Implement:

~~~text
config
DB startup
claim
dispatch
success
retry
quarantine
recovery
shutdown
~~~

## Step 6 — Harden event contracts

Implement:

~~~text
registry
version validation
schema validation
payload bounds
metadata bounds
secret rejection
~~~

## Step 7 — Finish proof

Implement:

~~~text
real atomicity failures
failure injection
claim races
stale worker race
ordering race
duplicate delivery
idempotent consumer
tenant isolation
immutability
~~~

## Step 8 — Operations and telemetry

Integrate production observability and runbooks.

## Step 9 — CI evidence hardening

Make required tooling failures fail closed.

## Step 10 — Final verification

Run:

~~~text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
./scripts/ci/verify
~~~

Then run the PostgreSQL security suite explicitly.

Record exact commands and actual results.

---

# 25. ACCEPTANCE GATE

PR #50 is merge-ready only when all P0 requirements are proven and the Part 8 P1 requirements are either implemented or explicitly deferred by an authoritative phase boundary.

Required evidence:

~~~text
code
+
real PostgreSQL behavior
+
negative security tests
+
concurrency tests
+
failure injection
+
worker execution
+
idempotency proof
+
actual CI evidence
~~~

A green CI result alone is not Part 8 proof.

---

# 26. FINAL EXECUTION DIRECTIVE

Implement against:

docs/phase4_part8_audit_outbox_implementation_contract.md

The target is:

~~~text
AUTHORIZED PHASE 4 IAM COMMAND
          |
          v
TRUSTED SCOPE
          |
          v
ONE AUTHORITATIVE POSTGRESQL TRANSACTION
          |
          +---- BUSINESS MUTATION
          |
          +---- MANDATORY AUDIT
          |
          +---- OUTBOX INTENT
          |
        COMMIT
          |
          v
BOUNDED WORKER CLAIM
          |
          v
EXTERNAL DELIVERY
          |
          v
AT-LEAST-ONCE DELIVERY
          |
          v
IDEMPOTENT CONSUMER
~~~

Do not:

~~~text
grant blanket runtime table mutation
use permissive fail-open tenant policies
trust event tenant IDs as authorization
let stale workers finalize newer claims
treat fixture-only transactions as application integration
treat status tests as idempotency proof
leave the worker executable inert
swallow database errors
use unbounded payloads/metadata
allow missing CI tooling to produce success
~~~

**The implementation is complete only when the repository can prove the Part 8 invariants under concurrency, failure, tenant isolation and retry — not merely compile the new code.**

---

# 27. SHORT INSTRUCTION

> **Fix the P0s first: lock down app_runtime, establish real app_worker authority, add claim generation, make audit/outbox state ownership safe, wire one real Phase 4 IAM transaction, make the worker executable, then prove RLS/atomicity/lease races/order/failure recovery/duplicate delivery with real PostgreSQL. Green CI is necessary; executable proof is the finish line.**
