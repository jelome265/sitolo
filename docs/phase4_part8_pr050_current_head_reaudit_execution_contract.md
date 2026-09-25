# SITOLO — PHASE 4 PART 8 PR #50 CURRENT-HEAD RE-AUDIT + EXECUTION CONTRACT

Document: docs/phase4_part8_pr050_current_head_reaudit_execution_contract.md
Binding contract: docs/phase4_part8_audit_outbox_implementation_contract.md
PR: #50
Repository: jelome265/sitolo
Current reviewed head: 15dd78081ebbbedd4e3cd6ba28c9d50e29ef4cba
Owner reference snapshot: 41a298f997af3cce896c8e56168af7ee28778a7e
Base: main at 2902dfe427963e33d90d22c0a7be1ce4c7193760
Status: Binding execution contract for remediation of the current PR head

## 1. EXECUTIVE VERDICT

PR #50 is not merge-ready at the current head.

The current head is two commits ahead of the owner-requested 41a298f... snapshot. The substantive regression is 8e366857a0cde1f94443219ccc6a4c28648ec6fa; 15dd780... is a whitespace-only follow-up.

The regression removed previously repaired Part 8 properties: the real worker executable, dedicated app_worker authority, claim ownership identity, aggregate-local ordering, application-level IAM transaction integration, worker-role test setup, and several negative/concurrency tests.

The current CI is green, but it proves compilation and the current test suite, not the full Part 8 security/correctness contract.

Required finish line:

    authorized IAM command
      -> trusted scope
      -> one authoritative PostgreSQL transaction
      -> business mutation + mandatory audit + outbox
      -> commit
      -> dedicated worker
      -> bounded claim with ownership
      -> validated dispatch
      -> publish/retry/quarantine
      -> at-least-once delivery
      -> idempotent consumer

## 2. CURRENT PR STATE

PR #50 is open, non-draft, and currently reported mergeable=true.

Current head: 15dd78081ebbbedd4e3cd6ba28c9d50e29ef4cba
Requested reference: 41a298f997af3cce896c8e56168af7ee28778a7e

Comparison from 41a298f... to the current head is ahead by two commits. The changed paths include:

    apps/worker/src/main.rs
    crates/sitolo-events/src/outbox.rs
    crates/sitolo-persistence/src/audit_outbox.rs
    crates/sitolo-persistence/tests/audit_outbox_integration_tests.rs
    crates/sitolo-persistence/tests/fixtures/rls_schema.sql
    docs/phase4_part8_pr009_pr050_reaudit_execution_contract.md

The previous remediation document was deleted by the regression and must be replaced by this document.

## 3. CI EVIDENCE

Current-head workflow evidence:

    rust     run 36089672011   success
    policy   run 36089671992   success
    security run 36089671977   success

The Rust verification job ran against PostgreSQL 18 and completed successfully. The workflow checked out PR merge ref 8a08a5b4ac4045f8fe1fbec3e75c392607e5343a for that run.

Interpretation:

    green CI = positive evidence
    green CI != Part 8 compliance

The current integration suite can still self-return when required database environment variables are absent. Security evidence must never silently become a no-op.

CI hardening must also preserve fail-closed behavior for required tooling and policy checks.

## 4. P0 — WORKER EXECUTABLE IS INERT

Current apps/worker/src/main.rs only starts Tokio and logs initialization.

It does not:

    validate worker configuration
    connect using worker credentials
    construct the real PostgreSQL store
    recover stale claims
    claim events
    dispatch events
    record publication
    retry or quarantine
    poll
    handle graceful shutdown

Required lifecycle:

    startup
    -> validate config
    -> connect as app_worker
    -> recover stale work
    -> claim bounded batch
    -> dispatch after commit
    -> mark published/retry/quarantine
    -> emit bounded telemetry
    -> poll
    -> graceful shutdown

Keep the worker narrowly scoped to the Part 8 relay. Do not introduce a generic job platform.

## 5. P0 — DEDICATED WORKER AUTHORITY WAS REMOVED

PgAuditOutboxStore now contains a single pool, and the PostgreSQL integration harness constructs it from the runtime pool.

The test setup no longer establishes a real app_worker connection.

Therefore the PR does not prove that worker operations execute with worker authority.

Required database proof:

    SELECT current_user;

must resolve to app_worker in worker tests.

Also inspect pg_roles and role memberships to prove at minimum:

    NOSUPERUSER
    NOBYPASSRLS
    NOINHERIT unless explicitly required by the reviewed design
    NOCREATEDB
    NOCREATEROLE
    NOREPLICATION
    no unintended memberships
    not owner of protected tables

PostgreSQL 18 documents that BYPASSRLS bypasses row-security policies and that table ownership has special RLS semantics.

References:
https://www.postgresql.org/docs/18/sql-createrole.html
https://www.postgresql.org/docs/18/ddl-rowsecurity.html
https://www.postgresql.org/docs/18/view-pg-roles.html

## 6. P0 — APP_RUNTIME BROAD TABLE PRIVILEGES ARE BACK

The integration harness grants app_runtime USAGE plus SELECT, INSERT, UPDATE and DELETE on all tables in the test schema.

This covers Part 8 infrastructure tables and prevents the suite from proving least privilege.

Required authority model:

    app_runtime
      -> ordinary application/business privileges
      -> audit INSERT where required
      -> outbox INSERT where required
      -> no arbitrary audit UPDATE/DELETE
      -> no administrative outbox mutation

    app_worker
      -> claim
      -> lease recovery
      -> publication state
      -> retry
      -> quarantine

Use explicit grants. Never use blanket ALL-TABLE grants to simplify tests.

References:
https://www.postgresql.org/docs/18/ddl-priv.html
https://www.postgresql.org/docs/18/ddl-rowsecurity.html

## 7. P0 — AUDIT RLS IS FAIL-OPEN

Current audit policy allows rows when organization_id is NULL or when the organization context is NULL.

That creates a fail-open path for missing tenant context.

Required invariant for tenant-scoped evidence:

    missing context  -> deny
    empty context    -> deny
    invalid context  -> deny
    wrong tenant     -> deny
    trusted match    -> allow according to role policy

Do not use NULL as a generic escape hatch. Truly global events must be explicit and independently modeled.

PostgreSQL 18 states that RLS is restrictive once enabled and defaults to deny when no applicable policy exists; it also documents owner and BYPASSRLS semantics.

Reference:
https://www.postgresql.org/docs/18/ddl-rowsecurity.html

## 8. P0 — OUTBOX RLS AND OBJECT AUTHORITY ARE TOO BROAD

Current outbox policy is FOR ALL to app_runtime with USING(true), and the test harness grants broad UPDATE and DELETE privileges.

This lets the runtime role act as an administrator over worker-owned state.

Required separation:

    runtime -> create durable event intent
    worker  -> claim/status/retry/quarantine/lease recovery

Worker-owned state must not be writable through an ordinary runtime path.

## 9. P0 — CLAIM OWNERSHIP IS UNSAFE AFTER LEASE EXPIRY

Current completion APIs use event_id alone.

Failure sequence:

    worker A claims E
    lease expires
    worker B reclaims E
    worker A finishes an old delivery
    worker A marks E published

The database cannot distinguish stale ownership from the active claim.

Required claim identity:

    event_id + claim_token
or
    event_id + lease_generation

Every worker-owned transition must verify:

    event_id
    claim identity
    expected current state

Required real-PostgreSQL test:

    A claims E -> T1
    lease expires
    B claims E -> T2
    A finalization with T1 -> rejected / zero rows
    B finalization with T2 -> accepted

PostgreSQL SKIP LOCKED is useful for concurrent queue processing, but it does not solve long-lived ownership after lease expiry.

Reference:
https://www.postgresql.org/docs/18/sql-update.html

## 10. P1 — AGGREGATE ORDERING WAS REMOVED

OutboxEvent contains aggregate_type and aggregate_id but no aggregate-local sequence.

Claim ordering is currently based on occurred_at.

occurred_at is not the ordering authority for events from the same aggregate.

Required:

    aggregate_sequence
    UNIQUE(aggregate_type, aggregate_id, aggregate_sequence)

Allocation must be concurrency-safe and database-authoritative.

Do not use current max + 1 without proper locking, process-local counters, timestamps, or worker-local counters.

Reference:
https://microservices.io/patterns/data/transactional-outbox

## 11. P1 — ENQUEUE ACCEPTS WORKER-OWNED STATE

enqueue_outbox_tx persists caller-supplied status, attempt_count, locked_at, published_at and last_error_class.

A newly created outbox event must always begin as:

    status = PENDING
    attempt_count = 0
    locked_at = NULL
    published_at = NULL
    last_error_class = NULL

Worker state must be owned by worker transitions and database invariants, not caller discipline.

## 12. P0 — APPLICATION-LEVEL IAM ATOMICITY WAS REMOVED

Current PostgreSQL tests prove useful database-level coupling using tenant_resources + audit + outbox in one transaction.

They no longer prove the actual Phase 4 application command boundary.

Required reference operation:

    authenticated principal
    -> membership / authorization / trusted scope
    -> real Phase 4 application command
    -> domain invariant
    -> authoritative IAM mutation
    -> mandatory audit
    -> outbox
    -> COMMIT

Use an actual organization, branch, membership, role, scope, or lifecycle operation already owned by Phase 4.

Do not make tenant_resources the permanent feature implementation merely because it is a convenient test table.

## 13. P0 — FAILURE INJECTION COVERAGE IS INCOMPLETE

Duplicate-key failures are valid database failures but do not replace explicit failure injection at the transaction boundaries.

Required failure points:

    after business write
    after audit write
    after outbox write
    before commit

Expected invariant:

    any mandatory failure -> no partial authoritative state

Test both audit and outbox failure paths through the real application transaction.

## 14. P1 — DUPLICATE DELIVERY AND IDEMPOTENT CONSUMER PROOF IS MISSING

Part 8 adopts at-least-once delivery. Exactly-once external delivery is not promised.

Required failure sequence:

    publish succeeds
    worker crashes before durable success recording
    event becomes eligible again
    duplicate delivery occurs
    consumer applies one semantic effect
    second delivery is recognized as already processed

Implement a durable consumer idempotency boundary using event identity or the repository-approved equivalent.

References:
https://microservices.io/patterns/data/transactional-outbox
https://microservices.io/patterns/communication-style/idempotent-consumer.html

## 15. P1 — EVENT VALIDATION IS TOO WEAK

OutboxEvent is largely free-form String data. OutboxEventId rejects only empty/whitespace input.

Add explicit validation for:

    identifier length
    aggregate type/id length
    event name
    event version
    schema version
    payload size
    deduplication key
    error class
    tenant metadata consistency

Bound the payload in construction, persistence, and worker dispatch.

Unknown events and unsupported schemas must fail safely rather than being blindly delivered.

## 16. P1 — AUDIT METADATA SAFETY IS SHALLOW

Current metadata is an Option<String>, and the tests rely partly on Debug-string checks.

That does not establish a structural security boundary.

Required:

    bounded metadata size
    bounded key/value sizes
    deterministic serialization
    explicit allowed fields
    secret rejection or redaction

Never persist credentials, OTPs, MFA secrets, recovery codes, tokens, Authorization headers, API keys, private keys, database credentials, raw request bodies, or biometric material.

Reference:
https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html

## 17. P1 — AUDIT IMMUTABILITY IS NOT PROVEN

RLS does not itself make audit records append-only.

Add real negative tests proving the normal runtime cannot:

    UPDATE audit
    DELETE audit
    rewrite tenant ownership
    rewrite actor identity
    rewrite event identity
    TRUNCATE audit storage

Use explicit SQL privileges plus RLS as defense-in-depth.

PostgreSQL documents that TRUNCATE is not subject to row security.

Reference:
https://www.postgresql.org/docs/18/ddl-rowsecurity.html

## 18. P1 — TENANT ISOLATION PROOF IS INCOMPLETE

Existing cross-tenant audit read coverage is useful but insufficient.

Required matrix:

    Tenant A cannot read Tenant B audit
    Tenant A cannot insert Tenant B audit
    Tenant A cannot update/delete Tenant B audit
    Tenant A cannot read Tenant B outbox
    Tenant A cannot enqueue Tenant B outbox
    Tenant A cannot administratively alter Tenant B outbox
    missing context cannot access tenant audit
    missing context cannot access tenant outbox

Remember: a request organization identifier is a selector, not authority proof.

## 19. P1 — INTEGRATION TESTS MUST NOT SILENTLY SELF-SKIP

The current setup returns Ok(None) when database environment variables are missing, and test wrappers return without failing.

That can make a security integration target appear successful without executing the proof.

Required test policy:

    unit target -> excludes database integration suite
    integration/security target -> missing prerequisites = FAIL

Do not report skipped security proof as successful evidence.

## 20. P1 — RETRY POLICY NEEDS A CENTRAL POLICY

Current code exposes classification and fixed configuration, but the full policy is not explicit.

Define policy for:

    RETRYABLE
    TERMINAL
    UNKNOWN

with:

    attempt budget
    bounded backoff
    next available time
    quarantine
    bounded error class
    explicit unknown-error behavior

Never retry indefinitely.

## 21. P1 — WORKER RESOURCE LIMITS NEED VALIDATION

Validate:

    1 <= batch_size <= configured maximum
    lease_duration > 0 and bounded
    max_attempts > 0 and bounded
    backoff_duration bounded
    payload and metadata bounded
    integer conversions checked

Reject invalid configuration at startup.

## 22. P1 — PRODUCTION DISPATCHER IS NOT WIRED

InMemoryDispatcher is suitable for unit tests.

The executable worker currently has no production dispatch path.

Wire the repository-approved integration/derived-effect boundary without introducing a new broker or generic messaging subsystem.

Validate events before dispatch.

## 23. P1 — OBSERVABILITY IS NOT COMPLETE

Process-local AtomicU64 counters are useful for unit tests but are not the production observability boundary.

Required measures include:

    pending count
    oldest pending age
    claim count
    publish success/failure
    retry count
    terminal failure
    quarantine count
    processing duration
    duplicate delivery
    audit success/failure
    mandatory audit failure
    audit storage latency
    authorization denial count

Do not use tenant/user identifiers or payloads as metric labels.

## 24. P1 — OPERATIONAL RUNBOOKS ARE REQUIRED

Document at least:

    audit write failure
    outbox backlog
    quarantined event
    duplicate delivery
    stale lease recovery
    cross-tenant audit access attempt

Each runbook needs symptoms, safe diagnostics, containment, recovery, replay controls, and evidence preservation.

Do not invent legal retention periods.

## 25. REQUIRED IMPLEMENTATION ORDER

### Step 1 — Restore database authority

Fix app_runtime privileges, audit policy, outbox policy, app_worker role, ownership and memberships.

### Step 2 — Restore worker ownership

Add worker credentials, current_user proof, claim token or generation, expected-state transitions, stale-worker rejection and lease recovery.

### Step 3 — Restore aggregate ordering

Add aggregate_sequence, uniqueness and concurrency-safe sequence allocation.

### Step 4 — Restore real Phase 4 application integration

Wire one real IAM command through authorization, trusted scope, transaction, authoritative mutation, audit and outbox.

### Step 5 — Restore executable worker

Implement validated startup, worker DB connection, polling, claiming, dispatch, completion/retry/quarantine and graceful shutdown.

### Step 6 — Harden event contracts

Implement registry, version/schema validation, payload bounds, metadata bounds, secret handling, deterministic serialization and worker-state ownership.

### Step 7 — Finish proof

Add real PostgreSQL tests for atomicity, failure injection, stale-worker races, ordering, tenant isolation, audit immutability, duplicate delivery and idempotent consumers.

### Step 8 — Operations and CI

Integrate observability, runbooks and fail-closed test prerequisites.

### Step 9 — Final verification

Run:

    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --workspace --all-targets
    ./scripts/ci/verify

Then run the PostgreSQL security/integration suite explicitly and record the actual results.

## 26. REQUIRED TEST MATRIX

| Area | Required proof |
| --- | --- |
| runtime audit | insert only where authorized; update/delete denied |
| audit RLS | missing/wrong tenant context denied |
| runtime outbox | enqueue allowed; admin mutation denied |
| worker authority | current_user=app_worker; role attributes verified |
| claim ownership | token/generation prevents stale completion |
| lease recovery | expired claim is recoverable |
| aggregate ordering | sequence allocation is unique and ordered under concurrency |
| application atomicity | IAM mutation + audit + outbox commit together |
| failure injection | mandatory failure produces full rollback |
| payload/schema | oversized/unknown/invalid event is rejected or quarantined |
| audit safety | sensitive input cannot cross durable boundary |
| audit immutability | runtime cannot rewrite/delete/truncate evidence |
| duplicate delivery | consumer applies one semantic effect |
| retry policy | bounded retry and terminal quarantine |
| worker lifecycle | real executable starts, polls and shuts down |
| integration harness | missing security prerequisites do not silently pass |

## 27. ACCEPTANCE GATE

PR #50 is merge-ready only when all P0 requirements are proven and remaining Part 8 requirements are implemented or explicitly deferred by an authoritative phase boundary.

Mandatory evidence:

    code
    + real PostgreSQL behavior
    + negative security tests
    + concurrency tests
    + failure injection
    + actual worker execution
    + idempotency proof
    + real CI evidence

Green CI alone is not sufficient.

## 28. IMPLEMENTATION RULES

1. Implement against docs/phase4_part8_audit_outbox_implementation_contract.md.
2. Preserve existing Phase 3 audit/authentication safety.
3. Do not weaken RLS to make tests pass.
4. Do not broaden app_runtime privileges for convenience.
5. Do not claim worker security using the runtime connection.
6. Do not use occurred_at as aggregate ordering authority.
7. Do not finalize worker state by event_id alone after lease expiry.
8. Do not use in-memory tests as proof of PostgreSQL security or atomicity.
9. Do not delete failing security tests; fix the implementation.
10. Do not introduce Kafka, RabbitMQ, CQRS, event sourcing, microservice extraction, or a generic job platform.

## 29. FINAL EXECUTION DIRECTIVE

Restore the repaired security/correctness design represented by 41a298f... where it satisfies the binding main contract, then complete the remaining proof gaps above.

Do not perform another broad refactor.
Do not optimize for green CI at the expense of the contract.
Do not remove security boundaries because they make the worker harder to implement.

The finish line is executable proof under real PostgreSQL, concurrency, tenant isolation, lease expiry, worker failure, transaction failure and duplicate delivery.

## 30. SHORT INSTRUCTION

STOP REFACTORING THE ARCHITECTURE. RESTORE THE PART 8 INVARIANTS: DEDICATED APP_WORKER AUTHORITY, FAIL-CLOSED RLS, CLAIM GENERATION, AGGREGATE ORDERING, WORKER-OWNED STATE, ONE REAL PHASE 4 IAM TRANSACTION, REAL WORKER EXECUTION, AND POSTGRESQL PROOF FOR STALE-WORKER RACES, ATOMICITY, TENANT ISOLATION, RETRIES AND DUPLICATE DELIVERY. DO NOT DELETE TESTS OR WEAKEN THE CONTRACT TO MAKE CI GREEN.