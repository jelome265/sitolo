# SITOLO — PHASE 4 PART 8 PR-009 RE-AUDIT + IMPLEMENTATION EXECUTION CONTRACT

**Primary binding contract:** docs/phase4_part8_audit_outbox_implementation_contract.md
**PR:** #49 — Phase 4 Part 8 audit and outbox implementation
**Branch:** feat/phase4-pr009-audit-outbox
**Repository:** jelome265/sitolo
**Re-audit head:** ee8c599af916ec2668219c91d1452f90acc82456
**PR base:** 30c7f5ae1a09c2142939b4535f8bf3d87fad9b41
**Re-audit date:** 24 September 2026
**Status:** NOT READY — P0 implementation and proof defects remain

> This document is subordinate to the binding Part 8 contract. It records the current PR state and converts the remaining gaps into an executable implementation order.

---

# 0. EXECUTIVE VERDICT

The second implementation pass is materially better than the initial type-only pass.

Newly present:

- PostgreSQL audit repository
- PostgreSQL outbox repository
- real PostgreSQL integration-test scaffolding
- claim/lease/retry/quarantine SQL functions
- aggregate-sequence storage
- worker-role fixture
- expanded RLS fixture

The PR is still not compliant with the binding Part 8 contract.

The remaining blockers are primarily correctness, security-boundary, and proof failures:

1. canonical CI currently fails;
2. chrono is referenced as a workspace dependency but is absent from the root workspace dependency set;
3. the atomicity tests do not perform a real business mutation;
4. no real Phase 4 application command is wired through business + audit + outbox in one authoritative transaction;
5. apps/worker remains an empty Phase 1 scaffold;
6. SECURITY DEFINER functions are not safely isolated/owned;
7. default PUBLIC EXECUTE on security-definer functions is not revoked;
8. claim ownership is not protected after lease expiry;
9. aggregate sequence generation is race-prone and database errors are swallowed;
10. worker parameters are insufficiently bounded;
11. worker test credentials can silently fall back to runtime credentials;
12. SET LOCAL is used outside a transaction in at least one integration test;
13. test functions are globally/unqualified and can race between parallel tests;
14. duplicate-delivery testing does not test an idempotent consumer;
15. retry/backoff policy is not fully executable;
16. event registry, bounded metadata, and complete schema validation are still missing;
17. observability and operational evidence remain incomplete.

Therefore:

PR #49 is progressing, but it is still a DO NOT MERGE state.

---

# 1. CI EVIDENCE

Latest workflow runs for the current head:

rust:
- run 35973718828
- verify job 107549131701
- FAILED

security:
- run 35973718897
- dependency-policy job 107549132282
- FAILED

The dependency-policy failure is concrete:

crates/sitolo-persistence/Cargo.toml inherits chrono from workspace.dependencies.chrono, but the root Cargo.toml has no workspace dependency named chrono.

The current branch Cargo.lock also has no chrono package entry.

Fix the workspace dependency correctly and regenerate the lockfile.

Do not remove --locked or bypass cargo-deny to hide the failure.

Required verification:

cargo check --locked
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
./scripts/ci/verify

Only actual execution counts as evidence.

---

# 2. ATOMICITY PROOF IS STILL INVALID

The binding contract requires:

business mutation + mandatory audit + outbox intent = one authoritative PostgreSQL transaction.

The current tests named around atomicity mostly insert audit rows and roll back. They do not mutate authoritative business state.

That means they do not prove the required invariant.

Implement one real Phase 4 mutation path through:

authenticated principal
 -> AuthorizedScope
 -> transaction-local tenant context
 -> authorization
 -> domain validation
 -> business mutation
 -> mandatory audit insert
 -> outbox insert
 -> commit

Failure at audit or outbox persistence must leave no committed business state and no orphan evidence.

Use the established PR-008 transaction-local tenant context rather than creating a second context mechanism.

---

# 3. APPLICATION INTEGRATION IS STILL MISSING

Repositories existing is not equivalent to application integration.

A real Phase 4 command must establish the reference composition pattern.

The application layer owns transaction orchestration.

The domain owns invariants.

Persistence owns SQLx/PostgreSQL.

Audit remains an explicit evidence boundary.

Outbox remains an explicit durable-delivery boundary.

Do not move business authorization into the worker.

Do not make the worker replay commands.

Do not add HTTP between modules in this modular monolith.

---

# 4. WORKER RUNTIME IS STILL EMPTY

apps/worker/src/main.rs remains a Phase 1 scaffold.

Part 8 requires the narrow relay behavior:

load configuration
 -> claim bounded batch
 -> validate event
 -> external delivery
 -> mark published
 -> retry or quarantine
 -> bounded logs/metrics

Implement only the relay needed by Part 8.

Do not create a generic job platform.

Do not introduce Kafka/RabbitMQ just because the outbox exists.

---

# 5. SECURITY-DEFINER FUNCTION HARDENING IS A P0

The current fixture uses SECURITY DEFINER functions.

PostgreSQL executes these functions with the privileges of the owning role. PostgreSQL recommends a secure function search_path that excludes untrusted/writable schemas and places pg_temp last. New functions also receive EXECUTE for PUBLIC by default unless that privilege is explicitly revoked.

References:
https://www.postgresql.org/docs/18/sql-createfunction.html
https://www.postgresql.org/docs/18/perm-functions.html

Required:

- explicit trusted schema;
- explicit secure search_path;
- intentional non-login owner;
- owner is not an accidental superuser;
- REVOKE EXECUTE FROM PUBLIC;
- GRANT EXECUTE only to the worker role;
- catalog tests for owner and grants.

Do not use the test container's postgres superuser as the production security model.

---

# 6. CLAIM OWNERSHIP IS NOT SAFE AFTER LEASE EXPIRY

Current logic identifies a claimed event by event_id + status = CLAIMED.

Race:

worker A claims E
 -> lease expires
 -> worker B reclaims E
 -> worker A calls mark_published(E)

The current API has no claim identity proving that worker A still owns the claim.

That permits a stale worker to potentially complete a newer claim.

Required:

event_id + claim_token or lease_generation + expected state

must be required for final state transitions.

A stale worker must receive a safe conflict/no-op and must never publish the result of another worker's claim.

---

# 7. AGGREGATE ORDERING IS CURRENTLY RACE-PRONE

The repository computes aggregate_sequence using MAX(aggregate_sequence) + 1.

Two concurrent writers can derive the same value.

There is also no database uniqueness boundary proving that one aggregate cannot contain duplicate sequence values.

The code also converts sequence-query errors into sequence 1 through unwrap_or(1), which hides authoritative database failures.

Required:

- concurrency-safe aggregate sequencing;
- UNIQUE(aggregate_type, aggregate_id, aggregate_sequence);
- no swallowed persistence errors;
- aggregate sequence preserved in the loaded OutboxEvent;
- explicit ordering semantics for same-aggregate events.

Wall-clock occurred_at is not an ordering authority.

---

# 8. WORKER INPUTS MUST BE BOUNDED

Validate all worker controls:

- batch size: positive and bounded;
- lease duration: positive and bounded;
- retry backoff: bounded;
- attempt budget: bounded;
- error classification: bounded and controlled;
- integer conversions: checked, never silent truncation.

Do not let arbitrary callers create unbounded SQL LIMITs, huge leases, or hot retry loops.

PostgreSQL supports bounded concurrent row-locking patterns, including SKIP LOCKED, but the application still has to bound resource usage.

Reference:
https://www.postgresql.org/docs/18/sql-update.html

---

# 9. WORKER TEST IDENTITY MUST BE EXPLICIT

The integration harness currently allows WORKER_DATABASE_URL to fall back to RUNTIME_DATABASE_URL.

That means a worker security test can silently execute as app_runtime.

Required:

- explicit worker credential for worker tests;
- SELECT current_user;
- assert expected worker identity;
- fail closed when worker credentials are missing.

Never use the wrong database role and still call the test a worker privilege proof.

---

# 10. TRANSACTION-LOCAL TENANT TEST IS INCORRECT

At least one integration test executes SET LOCAL app.organization_id against a pool instead of an explicit transaction.

SET LOCAL is transaction-local and must be executed within the authoritative transaction.

Reference:
https://www.postgresql.org/docs/18/sql-set.html
https://www.postgresql.org/docs/18/functions-admin.html

Use the existing set_transaction_tenant_context transaction boundary.

---

# 11. TEST-HARNESS FUNCTION ISOLATION IS REQUIRED

The fixture currently creates outbox functions without schema qualification.

Because the integration tests use isolated schemas but globally named functions, parallel tests can replace each other's functions.

That creates a shared mutable test-global and can produce flaky or cross-test behavior.

Required:

- schema-qualified function creation;
- schema-qualified function calls;
- or one carefully initialized shared test schema;
- deterministic teardown;
- no public global function replacement from parallel tests.

PostgreSQL creates an unqualified function in the current schema.

Reference:
https://www.postgresql.org/docs/18/sql-createfunction.html

---

# 12. DUPLICATE-DELIVERY TEST DOES NOT TEST IDEMPOTENCY

The current duplicate-delivery test:

claim
 -> mark published
 -> claim again

only proves that PUBLISHED rows are not claimable.

It does not prove:

publish succeeds
 -> worker crashes before state update
 -> same event is delivered again
 -> consumer receives duplicate
 -> consumer performs one semantic side effect

The binding contract explicitly adopts at-least-once delivery.

Required:

- stable event_id;
- simulated duplicate publication;
- idempotent consumer boundary;
- one semantic side effect for duplicate delivery.

Reference:
https://microservices.io/patterns/data/transactional-outbox
https://microservices.io/patterns/communication-style/idempotent-consumer.html

---

# 13. RETRY / BACKOFF / QUARANTINE MUST BE REAL POLICY

The type RetryClass exists, but the final worker must implement:

RETRYABLE
TERMINAL
UNKNOWN

with:

- bounded attempts;
- bounded backoff;
- next-available scheduling;
- safe error classification;
- terminal transition;
- quarantine;
- recovery after lease expiry.

Do not let the database function caller arbitrarily choose retry intervals outside the worker's policy.

---

# 14. EVENT CONTRACT REMAINS TOO LOOSE

The current model uses many unconstrained String fields.

The binding contract requires bounded and schema-defined durable events.

Implement:

- bounded event identity;
- bounded actor references;
- bounded request/trace identifiers;
- bounded action/source;
- bounded metadata;
- explicit event registry;
- event/version validation;
- deterministic serialization;
- invalid JSON/schema rejection.

Do not create an unbounded generic metadata map.

Do not serialize Rust Debug output as a durable event contract.

---

# 15. EVENT REGISTRY IS STILL REQUIRED

One authoritative registry must define at least:

name
version
category
producer
mandatory/diagnostic
tenant scope
payload contract
sensitive fields
consumers
retry behavior
retention class
ordering domain

Do not duplicate these facts across handlers.

---

# 16. AUDIT MODEL NEEDS BOUNDED METADATA

The binding contract's conceptual audit model includes metadata.

Current IamAuditEvent does not.

Add bounded metadata only after identifying the actual required metadata.

Never persist:

passwords
OTP values
MFA secrets
recovery codes
access tokens
refresh tokens
API keys
private keys
raw Authorization headers
credential-bearing connection strings
raw request bodies

OWASP's logging guidance requires security-relevant logging while minimizing sensitive-data collection and protecting stored audit information.

Reference:
https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html

---

# 17. AUDIT/OUTBOX PRIVILEGE PROOF

Final real-PostgreSQL tests must verify:

- app_runtime identity and privilege boundary;
- worker identity and privilege boundary;
- table ownership;
- RLS enabled/forced where required;
- exact RLS policy roles;
- cross-tenant audit read denied;
- cross-tenant audit write denied;
- audit UPDATE denied;
- audit DELETE denied;
- audit TRUNCATE denied;
- ordinary API access to outbox denied;
- worker immutable-field mutation denied;
- PUBLIC execution of privileged functions denied;
- worker only executes intended functions;
- worker cannot obtain BYPASSRLS or unintended role membership.

PostgreSQL role and RLS semantics are part of the actual security boundary.

References:
https://www.postgresql.org/docs/18/sql-createrole.html
https://www.postgresql.org/docs/18/ddl-rowsecurity.html

---

# 18. OBSERVABILITY IS NOT COMPLETE

The outbox_metrics view is useful, but it is not the complete worker/audit telemetry required by Part 8.

Implement bounded metrics for:

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
audit_storage_latency
authorization_denial_count

Do not put organization IDs, user IDs, emails, or event payloads into metric labels.

---

# 19. FAILURE-INJECTION PROOF

Implement deterministic tests for failure:

- after business write;
- after audit write;
- after outbox write;
- before commit;
- after worker claim;
- after external publication before mark-published.

The core invariant is:

mandatory audit/outbox failure -> no committed authoritative mutation.

Do not replace failure injection with manual rollback that never exercises the failing component.

---

# 20. CONCURRENCY PROOF

The final suite must prove:

- two workers cannot simultaneously own the same claim;
- stale workers cannot finalize newer claims;
- expired leases are reclaimable;
- same-aggregate sequences are unique;
- concurrent aggregate writers cannot duplicate ordering;
- invitation acceptance races converge safely;
- lifecycle races preserve final invariants;
- concurrent idempotent command retries do not double-apply state.

Use real PostgreSQL whenever the invariant depends on database locks, constraints, RLS, or transaction visibility.

---

# 21. CODE QUALITY / BUILD CLEANUP

Before final verification inspect for:

- unused PostgresAuditWriter pool state;
- unused OutboxEventRow fields;
- unused error variants;
- unsafe integer casts;
- swallowed SQL errors;
- duplicate OutboxWriter/OutboxReader abstractions;
- duplicated event definitions.

Run clippy with -D warnings.

Do not leave correctness or architecture warnings until the end.

---

# 22. IMPLEMENTATION ORDER

1. Fix root workspace dependency + Cargo.lock.
2. Make cargo metadata/check/clippy pass.
3. Harden SECURITY DEFINER ownership, search_path, function grants.
4. Add explicit worker identity and privilege tests.
5. Implement claim token/generation and stale-worker rejection.
6. Replace MAX()+1 ordering with a concurrency-safe database boundary.
7. Add unique aggregate ordering constraint and preserve sequence in events.
8. Implement bounded worker controls and real retry policy.
9. Wire one real Phase 4 application command through business + audit + outbox in one transaction.
10. Implement the narrow worker relay.
11. Add real atomicity/failure-injection tests.
12. Add real duplicate-delivery/idempotent-consumer tests.
13. Add event registry, bounded metadata, schema validation.
14. Add observability and operational runbooks.
15. Run canonical CI and record exact evidence.

---

# 23. DEFINITION OF DONE

Do not merge until all are true:

[P0] cargo check --locked passes
[P0] cargo fmt --check passes
[P0] clippy passes with -D warnings
[P0] workspace tests pass
[P0] ./scripts/ci/verify passes
[P0] real business + audit + outbox transaction exists
[P0] mandatory audit failure rolls back business state
[P0] outbox failure rolls back business state
[P0] worker runtime exists
[P0] security-definer functions are safely owned
[P0] PUBLIC execute is revoked
[P0] stale workers cannot finalize newer claims
[P0] aggregate ordering is concurrency-safe
[P0] persistence errors are never swallowed
[P0] worker credentials are explicit
[P0] SET LOCAL tenant tests use real transactions
[P0] integration schemas/functions are isolated
[P1] retry/backoff/quarantine is executable
[P1] duplicate delivery reaches an idempotent consumer
[P1] event registry/version/schema boundary exists
[P1] audit metadata and field bounds exist
[P1] worker/audit observability exists
[P1] operational runbooks exist
[P1] exact test evidence is recorded

---

# 24. FINAL EXECUTION DIRECTIVE

Implement this against:

docs/phase4_part8_audit_outbox_implementation_contract.md

The current target is not “more code”. The target is executable proof of the Part 8 invariants.

Do not:
- fake atomicity tests;
- use superuser behavior as least-privilege evidence;
- use a SECURITY DEFINER shortcut without hardening;
- swallow persistence failures;
- use MAX()+1 without a concurrency boundary;
- allow stale worker claims to finalize;
- call a PUBLISHED-state assertion idempotency;
- let worker tests silently run as app_runtime;
- mutate globally named test functions;
- invent retention periods;
- claim a CI gate passed without execution.

PR #49 remains NOT READY until the repository can prove Part 8 with code, PostgreSQL behavior, concurrency/failure tests, and actual CI evidence.
