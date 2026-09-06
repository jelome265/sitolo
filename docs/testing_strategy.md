# Sitolo Testing Strategy
Version: 1.0.0  
Status: Phase 0 implementation contract  
Artifact: 09 of 16  
Date: 2026-09-04  
System: Sitolo — Business Operating System for African SMEs  
Primary implementation baseline: Rust backend, PostgreSQL authoritative server, SQLite offline client store, Flutter Android-first client, Tauri desktop, limited TypeScript web/admin/tooling.

This document is the executable testing strategy for Sitolo. It converts the architecture, security model, domain model, database contract, API contract, authentication/authorization specification, offline synchronization protocol and external-integration specifications into a verification system. The objective is not to maximize test count or code coverage. The objective is to produce repeatable evidence that the system preserves business truth, tenant isolation, financial integrity, inventory integrity, authorization, synchronization correctness, external-integration safety, recoverability and operational behavior under both normal and adversarial conditions.

Testing is treated as a product capability and a release control. A green build without the required security, concurrency, integration, recovery and device evidence is not a production-ready build. Conversely, a failed noncritical cosmetic test must not automatically block unrelated infrastructure work unless the failure violates a declared release policy.

## 1. Testing Doctrine
Sitolo follows a risk-first, evidence-driven testing model. Every requirement that can create material economic, security, compliance, privacy, availability or integrity harm must have a corresponding verification method. The verification method should be as close as practical to the invariant being protected.

```text
RULE 1 — Business invariants must become executable tests.
RULE 2 — Security controls require negative tests, not only happy paths.
RULE 3 — Financial mutations require idempotency and concurrency evidence.
RULE 4 — Database behavior must be tested against real PostgreSQL for database-specific claims.
RULE 5 — Offline workflows require device/process-death/network-transition testing.
RULE 6 — External integrations require contract fixtures and failure/reconciliation tests.
RULE 7 — Recovery is tested by restoration, replay and reconciliation, not by documentation alone.
RULE 8 — Test environments must be disposable, isolated and reproducible.
RULE 9 — Release evidence is immutable and attributable to a specific source revision and build artifact.
RULE 10 — A test suite is incomplete when it cannot detect a known dangerous mutation.
```

The testing strategy is aligned with the current OWASP ASVS 5.0.0 stable standard for web application verification, OWASP API Security Top 10 2023 for API-specific risks, NIST SSDF for secure development practices, Rust/Cargo native unit/integration/doc testing, PostgreSQL regression and isolation testing concepts, Flutter unit/widget/integration testing, and OpenTelemetry semantic-convention principles for test observability.

External references: OWASP ASVS 5.0.0: https://owasp.org/www-project-application-security-verification-standard/ ; OWASP API Security: https://owasp.org/API-Security/ ; NIST SP 800-218: https://csrc.nist.gov/pubs/sp/800/218/final ; Rust Cargo tests: https://doc.rust-lang.org/cargo/guide/tests.html ; Cargo test command: https://doc.rust-lang.org/cargo/commands/cargo-test.html ; PostgreSQL regression testing: https://www.postgresql.org/docs/current/regress-run.html ; Flutter integration testing: https://docs.flutter.dev/testing/integration-tests ; Flutter testing: https://docs.flutter.dev/testing ; OpenTelemetry semantic conventions: https://opentelemetry.io/docs/specs/semconv/.

## 2. Quality Model
| Dimension | What must be demonstrated | Primary evidence |
| --- | --- | --- |
| Correctness | Domain commands produce legal state transitions and preserve invariants | Unit, property, integration, concurrency tests |
| Security | Unauthorized, cross-tenant, replayed, malformed and forged requests are denied | Security regression suite, DAST, fuzzing, manual testing |
| Integrity | Money, stock, tax state and audit facts cannot be duplicated or silently mutated | DB constraints, transaction tests, idempotency, recovery tests |
| Availability | Failures degrade into explicit safe states rather than corrupting truth | Fault injection, timeout tests, chaos tests |
| Compatibility | Supported API/database/event versions remain interoperable | Contract tests, migration tests, compatibility matrix |
| Performance | Critical flows satisfy bounded latency/resource budgets under realistic load | Benchmarks, load tests, query plans, device profiling |
| Recoverability | State can be restored and external integrations reconciled | Backup restore, replay, reconciliation drills |
| Operability | Failures are diagnosable from safe telemetry and audit evidence | Observability assertions, runbook drills |
| Usability under constraint | POS remains usable on representative low/mid Android hardware and intermittent networks | Real-device integration tests, network chaos |
| Compliance evidence | Regulatory integration behavior is reproducible and auditable | MRA contract fixtures, certification evidence pack |

## 3. Test Pyramid and Test Portfolio
```text
MANUAL / EXPLOITATION / CERTIFICATION
                         ▲
                  E2E / REAL DEVICE
                         ▲
             SERVICE + PROVIDER CONTRACT TESTS
                         ▲
        DATABASE / CONCURRENCY / INTEGRATION TESTS
                         ▲
        PROPERTY / STATE MACHINE / DOMAIN TESTS
                         ▲
              UNIT / PURE FUNCTION TESTS
```

The pyramid is intentionally not a simplistic “many unit tests, few end-to-end tests” rule. Sitolo contains security, financial and synchronization properties that cannot be proven at one layer. The correct portfolio has strong unit tests for deterministic calculations, real PostgreSQL integration tests for persistence and concurrency, contract tests at trust boundaries, and device-level tests for offline continuity.

| Layer | Scope | Default speed | Release use |
| --- | --- | --- | --- |
| Unit | Pure domain logic, value objects, parsers, policy predicates | Very fast | Every commit |
| Property/state-machine | Invariant spaces, lifecycle transitions, randomized command sequences | Fast/medium | Every PR + nightly extended |
| Component | Application services with controlled dependencies | Fast/medium | Every PR |
| Database integration | Real PostgreSQL, constraints, RLS, transactions, locking | Medium | Every PR touching DB; release |
| API integration | HTTP routing, auth context, serialization, errors, rate limits | Medium | Every PR touching API |
| Contract | Provider/API/event schema interoperability | Medium | Adapter/contract changes |
| Security | Negative authorization, injection, replay, SSRF, secrets, abuse | Medium/slow | Every PR + scheduled deep runs |
| Fuzz | Untrusted parsers and protocol surfaces | Slow | PR smoke + nightly/continuous campaigns |
| Load/performance | Throughput, tail latency, DB pool, queue behavior | Slow | Scheduled + release candidate |
| E2E | Full user workflows | Slow | PR smoke + nightly + release |
| Real-device | Flutter on representative Android devices | Slow | Nightly + release |
| Chaos/DR | Dependency outages, process death, recovery/replay | Very slow | Nightly targeted + release |
| Manual/pentest | Threat-led human assessment | Very slow | Milestones and before production |

## 4. Test Environment Architecture
```text
LOCAL
  ├── deterministic unit tests
  ├── ephemeral PostgreSQL
  ├── mock/fake external providers
  └── local Flutter emulator

CI PR
  ├── isolated PostgreSQL service
  ├── Redis only when the test targets Redis behavior
  ├── disposable object storage emulator where applicable
  ├── provider contract fixtures
  └── security scanners

INTEGRATION
  ├── disposable PostgreSQL database
  ├── controlled external sandbox/stub endpoints
  ├── representative queue/worker runtime
  └── network fault injector

RELEASE CANDIDATE
  ├── production-like configuration
  ├── real device matrix
  ├── MRA/payment sandbox where authorized
  ├── load environment
  └── restore/DR environment
```

Tests must not share mutable state across unrelated test cases. Each test suite must define ownership of databases, tenants, users, devices, providers and fixtures. Test isolation is itself a security property: contamination between tests can hide authorization defects and create false positives.

Production credentials must never be used in automated functional tests. Synthetic merchant accounts, sandbox keys and isolated test tenants are mandatory. A production-connected test must be explicitly classified as a controlled operational verification, reviewed separately, and constrained against synthetic financial activity unless the external provider explicitly authorizes it.

## 5. Determinism, Time and Randomness
Business tests must control time. Domain code should receive an explicit clock abstraction rather than reading system time directly wherever time affects state transitions, expiry, idempotency windows, offline age, pricing effectiveness, approval expiration or reconciliation. Tests must be able to freeze, advance and rewind their logical test clock.

```text
TestClock
  now()
  advance(duration)
  set(timestamp)

Randomness
  seed recorded per test
  seed included in failure output
  reproduction command generated automatically
```

Randomized tests are useful only when failed cases can be replayed. Property-based and fuzz failures therefore record the seed, generated input, schema version, feature flags, database migration version and relevant fixture identifiers.

## 6. Fixture Strategy
Fixtures are data contracts, not convenient blobs. Each fixture must declare its purpose, minimum required data, ownership and cleanup behavior. Sensitive production data must never be copied into a test fixture. Prefer synthetic deterministic records that resemble production cardinality and edge conditions.

```text
Fixture categories:
  identity_fixture
  tenant_fixture
  branch_fixture
  device_fixture
  role_fixture
  catalogue_fixture
  price_fixture
  inventory_fixture
  sale_fixture
  payment_fixture
  tax_fixture
  sync_fixture
  audit_fixture
  provider_fixture
  failure_fixture
  reporting_fixture
```

| Fixture rule | Requirement |
| --- | --- |
| Minimal | Only required records are created by default |
| Explicit | Implicit “magic” global records are prohibited |
| Versioned | Schema-sensitive fixtures declare expected schema version |
| Tenant-safe | Every tenant-owned fixture has a generated or controlled tenant ID |
| Time-aware | Temporal fixtures use explicit timestamps |
| Idempotent setup | Rerunning setup does not create uncontrolled duplicates |
| Disposable | Fixtures can be destroyed without impacting other suites |
| Auditable | Tests can explain which fixture generated an asserted state |

## 7. Unit Testing Standard
Unit tests target deterministic behavior with minimal infrastructure. Rust domain modules should be tested without HTTP, PostgreSQL or provider calls whenever possible. Unit tests must not duplicate implementation details merely to increase coverage; they should assert observable contracts.

```text
Good unit test:
  given authorized refund within remaining refundable amount
  when refund command is executed
  then state transitions to REFUNDABLE_REDUCED and amount is recorded

Bad unit test:
  assert private helper X was called exactly once
```

- Value object validation: money precision, currency, quantity bounds, identifiers, timestamps, enum parsing.
- State machines: legal transitions and explicit rejection of illegal transitions.
- Authorization predicates: permission + tenant + branch + resource-state combinations.
- Pricing/tax calculations: deterministic examples and edge cases such as zero quantities, rounding boundaries and tax-inclusive/exclusive calculations.
- Inventory calculations: signed quantity semantics, unit conversion rules and non-negative-stock policy where applicable.
- Idempotency fingerprint rules: same semantic input is equivalent; materially different reuse is rejected.
- Error classification: domain rejection versus retryable infrastructure failure.
- Canonical serialization: protocol versions and hash inputs remain stable.

## 8. Domain Invariant Test Program
Every domain invariant must have at least one positive test, one direct negative test and one boundary/concurrency test where concurrency can affect it. This is the minimum definition of executable business truth.

| Domain | Required invariants | Minimum tests |
| --- | --- | --- |
| Identity | Valid identity/session lifecycle | expiry, revocation, renewal, replay |
| Tenant | No unauthorized tenant access | cross-tenant read/write attempts |
| Catalogue | SKU/product relationships remain valid | delete/disable/reference conflicts |
| Pricing | Price effective state is deterministic | overlapping price periods, boundary timestamps |
| Inventory | Movements reconcile to stock state | concurrent sale, reversal, duplicate event |
| Sales | Finalized sale facts are immutable | replay, mutation attempt, concurrent finalization |
| Payments | Provider evidence cannot exceed internal authority | amount mismatch, duplicate webhook, timeout-after-commit |
| Cash | Register/shift lifecycle remains valid | double close, wrong register, variance approval |
| Tax/EIS | Local sale truth survives external failure | submission rejection, retry, configuration change |
| Sync | Commands are durable and replay-safe | process death, duplicate push, checkpoint rollback |
| Audit | Sensitive operations produce evidence | missing audit transaction rollback |
| Billing | Entitlements cannot corrupt merchant data | payment failure, downgrade, quota exhaustion |

## 9. Property-Based Testing
Property-based testing should target domains with a large state space where example-based tests are likely to miss combinations. Suitable targets include money arithmetic, inventory ledger sequences, sale/refund sequences, idempotency, sync command sequences, pagination/cursors, authorization scope combinations and protocol parsing.

```text
Example property:
For any generated sequence of valid inventory movements:
  computed stock = initial_stock + sum(posted_movements)
provided all movements accepted by the invariant policy.

Example financial property:
For any valid sale/refund sequence:
  cumulative_refund <= refundable_amount
and no single retry can increase cumulative refund.
```

The generator must know the domain sufficiently to generate both valid and adversarial commands. Pure random bytes are more useful for fuzzing parsers than for testing business properties.

## 10. State-Machine Testing
State machines are especially important because Sitolo explicitly rejects boolean combinations such as is_paid/is_refunded/is_active as a substitute for lifecycle semantics. State-machine tests generate legal and illegal transitions and verify that invariants survive every legal sequence.

```text
Example Sale:
DRAFT -> FINALIZATION_PENDING -> FINALIZED
DRAFT -> CANCELLED
FINALIZED -> REFUND_PENDING -> PARTIALLY_REFUNDED -> FULLY_REFUNDED

Forbidden examples:
FINALIZED -> DRAFT
FULLY_REFUNDED -> FINALIZED
CANCELLED -> FINALIZED
```

State-machine tests must cover repeated commands, stale version submissions, retries after timeout, duplicate event delivery and transitions after revocation/entitlement changes where applicable.

## 11. PostgreSQL Integration Testing
Database-specific behavior must be verified against real PostgreSQL. An in-memory substitute cannot establish confidence in PostgreSQL unique constraints, transaction isolation, RLS evaluation, locks, deadlocks, deferrable constraints, query plans or PostgreSQL-specific SQL behavior. PostgreSQL itself maintains regression and isolation test suites for concurrency and database-specific behavior; Sitolo should apply the same principle to its critical database contracts.

Every database integration suite should start from a known migration state. The test harness must apply migrations exactly as production deployment would, then create the minimum required roles and test data.

```text
DB test lifecycle:
  provision ephemeral PostgreSQL
  apply migrations
  create runtime roles
  seed synthetic tenants
  run isolation/integrity tests
  collect logs/query plans where required
  destroy database
```

| Database claim | Verification |
| --- | --- |
| Unique business identity | Concurrent insert test + UNIQUE constraint |
| Tenant isolation | RLS/role tests with two tenants |
| Transaction atomicity | Induced failure mid-operation and rollback assertion |
| Lock correctness | Concurrent transaction test with timing barriers |
| Deadlock handling | Known opposing lock order scenario |
| Idempotency | Same command twice concurrently |
| Migration correctness | Upgrade from previous schema snapshots |
| Index correctness | EXPLAIN-based assertion for critical query paths |
| Retention/archive | Synthetic volume + policy verification |

## 12. Transaction and Atomicity Tests
The central sale transaction must be tested as an atomic unit. The canonical finalized-sale transaction updates authoritative sale state, inventory postings, payment linkage where applicable, audit evidence and outbox state without leaving a partially committed financial record.

```text
Test A:
  begin finalization
  inject failure after inventory write
  assert sale not finalized
  assert inventory not advanced
  assert audit/outbox side effects from failed transaction absent

Test B:
  commit DB transaction
  fail external EIS submission
  assert sale remains finalized
  assert tax state becomes pending/retryable
  assert retry evidence exists
```

Tests must deliberately inject failures at transaction boundaries. Mocking every repository call is insufficient because the dangerous defect is often transaction ownership, lock behavior or ordering rather than an individual function result.

## 13. Concurrency Test Program
Concurrency testing is release-blocking for financial and inventory paths. The test harness must use deterministic barriers/latches where possible so race windows can be reproduced rather than relying entirely on timing.

```text
Scenario: two POS devices sell the last unit
  stock = 1
  Tx A attempts sale qty=1
  Tx B attempts sale qty=1
  synchronize arrival before stock lock
  release both
  assert exactly one sale consumes stock
  assert other is rejected/conflicted according to policy
  assert stock never becomes negative
```

```text
Scenario: duplicate payment webhook
  deliver identical event concurrently to N workers
  assert one provider event identity is recorded
  assert only one business payment effect exists
  assert all duplicate workers converge on same canonical result
```

```text
Scenario: duplicate offline command
  push same command concurrently from retry workers
  assert one authoritative command application
  assert stable command result returned to all equivalent retries
```

| Race target | Required assertion |
| --- | --- |
| Inventory decrement | No negative/incorrect stock |
| Sale finalization | Exactly one finalized state |
| Refund | Cumulative refund cannot exceed eligible amount |
| Payment event | One logical payment effect |
| Cash close | One authoritative close |
| Idempotency record | Unique identity under concurrent creation |
| Sync checkpoint | Checkpoint never advances past uncommitted result |
| Entitlement/quota | No double-grant or double-consume |
| Audit/outbox | Commit and evidence are transactionally consistent |

## 14. Deadlock and Lock-Order Testing
Deadlocks are not automatically correctness failures if the application safely aborts and retries an operation. The actual requirement is bounded recovery without duplicate financial effects. Tests should intentionally exercise opposing operation order and verify deadlock classification, retry policy and idempotency.

```text
Procedure:
  construct transactions with controlled lock order
  force cycle
  observe PostgreSQL deadlock error
  verify transaction aborts safely
  verify retry classification
  retry using stable operation identity
  assert one final business effect
```

## 15. API Integration Testing
API tests validate the translation boundary between untrusted HTTP and trusted application commands. They must assert authentication context, authorization, validation, serialization, error mapping, pagination, rate limits, idempotency and response semantics.

- Every endpoint has tests for unauthenticated access where denial is expected.
- Every tenant-owned endpoint has cross-tenant negative tests.
- Every privilege-sensitive route has role/permission matrix coverage.
- Every object identifier is tested with an object owned by another tenant and another branch.
- Every body field that can influence authority is tested for tampering.
- Every idempotent mutation is tested with same-key/same-payload and same-key/different-payload.
- Pagination tests include empty results, maximum limits, invalid cursors, cursor replay and high-cardinality data.
- Errors are tested for stable public codes and absence of internal SQL/provider details.

## 16. Authentication and Session Tests
Authentication tests must validate both successful and hostile flows. Tests should include password/identity-provider paths as actually implemented, MFA, session issuance, refresh rotation where used, expiry, logout, revocation, device state, recovery and step-up authentication.

| Scenario | Expected result |
| --- | --- |
| Expired access token | 401/defined auth failure; no business execution |
| Revoked refresh/session | refresh denied |
| Revoked device | device-sensitive/high-risk action denied |
| MFA challenge replay | denied after successful consumption/expiry |
| Recovery token reuse | denied |
| Wrong tenant membership | no access to tenant data |
| Stale membership cache | authorization must not persist beyond defined revocation window |
| Step-up missing for sensitive operation | operation denied or challenge required |
| Brute-force threshold | bounded response and telemetry |
| Malformed token/JWT | safe rejection without parser crash |

## 17. Authorization and Tenant-Isolation Testing
Tenant isolation is a security boundary. It is not enough to test the UI. The test suite must construct adversarial requests directly against HTTP and database layers. OWASP API1/API3/API5 risks are particularly relevant because object-level, property-level and function-level authorization failures can expose or mutate merchant data.

```text
Mandatory negative matrix:
  User U1 belongs to Tenant A
  Tenant B contains equivalent resource IDs
  Attempt:
    GET B resource using A session
    POST mutation referencing B resource
    PATCH field that changes ownership/scope
    invoke privileged function unavailable to U1
  Expected:
    no resource disclosure
    no state mutation
    safe error response
    audit/security telemetry
```

At database level, run the same cases using the actual runtime role. Where RLS is part of defense-in-depth, tests must prove that application-layer mistakes do not automatically become cross-tenant access.

## 18. OWASP Security Regression Suite
Security testing is mapped to the existing Sitolo security control inventory and should remain traceable as requirements evolve. The suite must include both automated tests and periodic human assessment.

| Area | Minimum automated coverage |
| --- | --- |
| Secrets | repository, history, generated artifacts, mobile bundles, logs, test output |
| Authentication | credential abuse, token parsing, session lifecycle, MFA/recovery |
| Authorization | BOLA, property-level auth, function-level auth, tenant/branch scope |
| Injection | SQL, search, template, shell/command-like inputs where applicable |
| Web | XSS, CSRF where cookie auth exists, CORS, unsafe redirects |
| Files | path traversal, archive bombs, parser abuse, content-type confusion |
| SSRF | private ranges, loopback, cloud metadata, redirects, DNS changes |
| Business abuse | refund abuse, discount abuse, stock manipulation, export abuse |
| Replay | webhooks, payment attempts, sync commands, approvals |
| Availability | rate limits, bounded payloads, expensive query controls |
| CI/CD | workflow injection, unsafe permissions, dependency provenance, secret exposure |
| Cloud/IaC | public storage, database exposure, network rule drift |
| DR | restore/rebuild and data-integrity validation |

## 19. Fuzz Testing
Fuzzing targets untrusted or highly stateful parsers where example cases cannot cover the input space. Initial targets are protocol envelopes, API error bodies, identifiers, cursors, compressed payloads, JSON/CBOR-like decoders if introduced, canonicalization, cryptographic input framing, CSV imports and provider responses.

```text
Fuzz requirements:
  no panic on attacker-controlled input
  bounded memory growth
  bounded CPU/time
  parser rejects malformed structure deterministically
  unsupported versions fail safely
  canonicalization remains stable
  no secret material appears in crash artifacts
```

A fuzz target is not considered valuable merely because it runs. It must have an explicit security or correctness property. Corpus minimization and crash reproduction artifacts are required.

## 20. Mutation Testing
Mutation testing is a core quality control for security and business invariants. Known dangerous mutations should be deliberately introduced in test builds to verify that the suite fails.

```text
Required mutations:
  remove tenant predicate
  skip permission check
  disable idempotency lookup
  accept replayed webhook
  advance sync checkpoint before commit
  allow refunded amount above original amount
  allow negative stock when policy forbids it
  trust client-supplied sale total
  skip provider verification
  ignore device revocation
  log secret value
  bypass RLS/runtime scope
  silently retry permanent external rejection
```

Mutation testing does not need to run on every PR. A curated mutation corpus should run on a scheduled basis and before major release milestones. Every escaped high-risk mutation requires either a new test or an explicit justification approved through the architecture/security review process.

## 21. Payment Testing
The payment specification defines external provider systems as trust boundaries. Tests must therefore distinguish “provider callback arrived” from “Sitolo proved the payment.” Current PayChangu documentation instructs merchants to validate webhook signatures and re-query transactions before fulfilling an order; the test program must encode those behaviors as contract-level controls.

| Scenario | Required assertion |
| --- | --- |
| Valid webhook | signature validates, event persists once, verification occurs as required |
| Invalid signature | no business side effect |
| Duplicate webhook | same business result, no second payment effect |
| Same event ID/different body | idempotency conflict or explicit rejection |
| Provider timeout after request | payment remains indeterminate/pending until safe reconciliation |
| Provider says success but amount differs | do not mark expected payment settled |
| Wrong merchant/account | reject or exception; no fulfillment |
| Unknown provider reference | quarantine/exception |
| Refund replay | no second refund effect |
| Provider outage | merchant sale state remains intact; payment state explicit |
| Worker crash after external success | reconciliation discovers success without duplicate initiation |

## 22. MRA EIS Testing
MRA EIS testing must remain contract-driven and must never confuse a successful mock response with regulatory readiness. Tests must cover the adapter boundary, terminal state, configuration lifecycle, online sales, offline transactions, receipt-signing artifacts, offline submission, external rejection and evidence retention. The exact production contract remains subject to the MRA version and certification process documented in mra_eis_integration_spec.md.

```text
Minimum MRA test scenarios:
  terminal not activated
  terminal activated but configuration stale
  configuration changes between sales
  online sale accepted
  online sale rejected
  timeout after submission
  duplicate retry
  offline transaction within threshold
  offline transaction over threshold
  offline transaction too old
  offline signature generation/validation
  offline transaction upload after reconnect
  upload duplicate
  terminal blocked/disabled
  invalid product/tax configuration
  correction/void/credit/debit note according to enabled MRA contract
  evidence preserved for every accepted/rejected attempt
```

For current MRA-specific fields and endpoint semantics, contract fixtures must be generated from the approved MRA interface version rather than handwritten assumptions. Certification evidence is a separate release artifact and cannot be synthesized by test success alone.

## 23. Offline and Synchronization Testing
Offline synchronization is one of Sitolo’s highest-risk subsystems because the client can operate without continuous authority from the server. The test program therefore treats the local command journal, command identity, idempotency, device authorization, tenant scope, business invariants, checkpoints and crash recovery as one system.

```text
Required chaos sequences:
  create local sale -> kill process -> restart -> sync
  sync request reaches server -> response lost -> retry same command
  upload batch partially accepted -> disconnect -> retry
  pull response received -> process dies before checkpoint persist -> restart
  device revoked while commands are pending -> reconnect -> verify policy
  local clock moved forward/backward -> server ignores client time for authority
  duplicate commands arrive in same batch
  same logical sale arrives from multiple devices
  schema version unsupported -> controlled rejection
```

| Sync property | Test |
| --- | --- |
| Durability | process death at every local journal transition |
| Idempotency | same command repeated serially and concurrently |
| Checkpoint safety | crash before/after response and before/after checkpoint |
| Ordering | dependency-aware command sequence |
| Tenant isolation | cross-tenant command forged |
| Device revocation | pending local commands after revoke |
| Bounded resources | oversized batch/payload and high queue depth |
| Schema evolution | old supported client/new server and vice versa |
| Conflict semantics | stock, price, role, cash, tax examples |
| Resync | corrupt/stale local cursor and authoritative snapshot recovery |

## 24. Flutter Mobile Testing
Flutter testing is divided into unit, widget and integration tests. The official Flutter integration_test package supports execution on physical devices/emulators and Firebase Test Lab, while widget testing validates widget behavior in isolation. Sitolo requires all three because low-connectivity operation and process death cannot be established by widget tests alone.

```text
Flutter test suites:
  test/
    domain/
    application/
    persistence/
    sync/
    security/
  integration_test/
    auth_flow_test.dart
    offline_sale_test.dart
    reconnect_sync_test.dart
    payment_state_test.dart
    device_revocation_test.dart
    migration_test.dart
    low_storage_test.dart
```

Widget tests should assert loading/error/permission/empty states, not merely screenshots. Integration tests should execute user-visible workflows against a controlled backend or test server. The mobile suite must include background/foreground transitions, process death, storage exhaustion, network changes and device clock manipulation where platform constraints allow.

## 25. Real Device Matrix
The product explicitly targets low- and mid-range Android devices and intermittent connectivity. Therefore at least one representative lower-memory/older Android device class and one current mainstream Android device class must be included in release verification. Exact models should be maintained as an owned device matrix and updated based on observed customer hardware.

| Profile | Required tests |
| --- | --- |
| Low-memory Android | cold start, catalog load, POS sale, sync, background/foreground, memory pressure |
| Mainstream Android | full regression, biometric/secure storage where used, camera/barcode where used |
| Slow storage profile | SQLite transaction throughput, local queue growth, migration |
| Intermittent 3G/4G-like network | latency, packet loss, reconnection, duplicate requests |
| Offline extended | multiple sales, restart, reconnect, sync backlog |
| Battery constrained | background behavior, queue persistence, resumed sync |

## 26. Tauri/Desktop Testing
Tauri is tested separately because desktop capabilities introduce filesystem, navigation, update and native-command attack surfaces that are not equivalent to Flutter mobile. Tests must verify restricted command surfaces, navigation allowlists, CSP, credential persistence and update integrity as implemented.

- No untrusted UI action can invoke an unrestricted shell/filesystem bridge.
- Credentials are stored through the approved secure mechanism and are not written to ordinary logs.
- Navigation is restricted to approved origins/routes.
- Update packages are authenticated according to the deployment/update contract.
- Opening malicious local files or deep links cannot escape the application security boundary.
- Final production bundles contain no test secrets, debug settings or unintended source artifacts.

## 27. Contract Testing
Every boundary where Sitolo consumes or publishes a protocol needs a contract suite. Contract tests should run against versioned fixtures and, where possible, a real sandbox. Consumer-driven tests are useful for stable internal contracts; schema/fixture tests are essential for providers controlled by third parties.

```text
Canonical adapter contract:
  canonical request -> provider request mapping
  provider success -> canonical success
  provider rejection -> canonical rejection
  provider timeout -> indeterminate/retryable state
  malformed response -> unsafe-to-trust classification
  duplicate response/event -> idempotent handling
  signature verification -> pass/fail
  redaction -> no secrets in captured diagnostics
```

## 28. API Schema and Compatibility Testing
Public API changes are tested as compatibility changes, not merely compilation changes. Every release must classify endpoints and schemas as additive, behavior-preserving, deprecated or breaking. Contract tests must verify supported previous clients where backward compatibility is promised.

| Change | Required test |
| --- | --- |
| New optional field | old client can still consume response |
| New enum value | clients handle unknown value safely where contract requires |
| Removed field | deprecation period and compatibility tests |
| Changed error code | explicit API version/contract update |
| Changed idempotency semantics | security and replay regression |
| Changed pagination | cursor compatibility and boundary tests |
| Changed auth scope | authorization matrix regression |

## 29. Migration Testing
Every PostgreSQL migration is tested from the immediately previous schema version and, for meaningful releases, from representative supported older versions. The suite must test forward migration, application startup, data invariants, query compatibility and rollback/recovery procedure as applicable. A migration that executes successfully but leaves semantic data corruption is a failed migration.

```text
Migration verification:
  restore representative pre-migration DB
  apply migration
  run integrity queries
  run critical application tests
  compare row/count/checksum metrics where safe
  validate indexes/constraints
  validate application startup
  capture migration timing/locks
  verify recovery plan
```

## 30. Reporting and Export Testing
Reporting is derived state. Tests must prove that reports do not become an alternate financial authority and cannot cross tenant boundaries. Large reports/exports are also a resource-exhaustion surface.

- Report totals reconcile with authoritative transactional queries for controlled datasets.
- Historical sales use stored historical facts rather than current product price/tax configuration.
- Exports enforce tenant/branch scope and authorization.
- Large date ranges are bounded or routed to asynchronous jobs according to the API contract.
- Export filenames, metadata and contents do not leak data from other tenants.
- Repeated export requests do not create uncontrolled duplicate jobs.
- CSV/JSON generation handles delimiters, formulas and encoding safely.

## 31. Observability Assertions in Tests
Critical paths must produce safe, structured evidence. Tests should assert presence and correlation of important telemetry without hard-coding unstable implementation details. OpenTelemetry defines semantic conventions for common telemetry attributes and events; Sitolo should adopt stable conventions for request, worker, provider and failure context.

```text
Critical transaction correlation:
  request_id / trace_id
  tenant-safe actor context
  operation name
  outcome class
  duration
  dependency status
  idempotency/command reference where safe
  audit reference where applicable
```

Tests must also assert negative observability properties: secrets, authentication tokens, provider signatures, full payment credentials, raw command bodies where prohibited and unnecessary PII must not appear in logs, traces or test failure artifacts.

## 32. Performance Engineering
Performance targets are defined by business criticality rather than arbitrary universal millisecond claims. POS operations must remain responsive on low-end hardware and under realistic network conditions, while reports and bulk operations may be asynchronous. Performance tests must measure p50/p95/p99, throughput, error rate, connection-pool wait, database lock wait, CPU, memory and queue depth where applicable.

| Workload | Primary metric | Failure signal |
| --- | --- | --- |
| POS sale | p95 end-to-end and commit latency | tail latency / lock contention |
| Catalogue search | p95 query latency | unbounded query / missing index |
| Inventory posting | transaction duration + lock wait | contention/deadlock |
| Sync push | commands/sec + p95 batch time | queue growth / timeout |
| Sync pull | events/sec + payload size | memory/network blowup |
| Payment webhook | ingestion latency | backlog |
| EIS queue | oldest pending age | retry saturation |
| Reporting | job completion time | resource starvation |
| Mobile startup | cold-start time and memory | OOM / excessive initialization |

## 33. Load Testing
Load profiles must resemble the product rather than hypothetical internet-scale traffic. Start with a small number of concurrent merchants and registers performing realistic sales, catalogue reads, synchronization and reporting. Scale until a declared limit or bottleneck is reached, then document the limiting resource.

```text
Baseline profile:
  70% catalogue/read traffic
  20% POS mutations
  5% sync traffic
  3% payment/tax callbacks
  2% reports/exports

Stress profile:
  double concurrent merchant/register load
  inject provider latency
  inject DB connection pressure
  measure degradation and recovery
```

Load tests must preserve tenant diversity. A single giant tenant can hide per-tenant contention and index behavior. Tests should include many tenants, multiple branches and concurrent registers where the architecture supports them.

## 34. Resource Exhaustion and Abuse Testing
Unrestricted resource consumption is both a security and reliability problem. Tests must attempt oversized bodies, giant arrays, deep JSON structures, expensive filters, broad report windows, huge exports, excessive sync batches, long cursors, repeated login failures and high-frequency webhook delivery.

| Attack/workload | Control to prove |
| --- | --- |
| Oversized request | HTTP/body limit |
| Oversized sync batch | batch count/bytes limit |
| Huge export | async job + quota |
| Expensive query | bounded filter/date range + timeout |
| Connection flood | rate limit/connection pool protection |
| Queue flood | backpressure and priority isolation |
| Webhook flood | signature gate + dedupe + bounded worker capacity |
| Repeated failed login | authentication abuse control |
| Large file upload | size/type/storage quotas and scanning path |

## 35. Chaos and Fault Injection
Chaos testing validates that dependency failure results in explicit safe state. The objective is not random destruction for entertainment; every injected failure must map to a declared failure mode and expected recovery behavior.

```text
Inject:
  PostgreSQL connection loss
  transaction timeout
  deadlock
  Redis unavailable (if enabled)
  object storage timeout
  payment provider timeout
  payment provider malformed response
  MRA EIS timeout/rejection
  DNS failure
  packet loss / latency
  process crash
  disk-full condition
  clock skew
```

| Failure | Expected behavior |
| --- | --- |
| Database unavailable | unsafe writes fail closed; no fake success |
| Provider timeout | state becomes pending/indeterminate, not falsely failed/succeeded |
| EIS unavailable | local sale truth preserved; tax submission queued/retryable |
| Redis unavailable | only safe degradation allowed because Redis is non-authoritative |
| Worker crash | lease expires; operation can be safely retried |
| Disk full | local durable-write path reports explicit failure; no silent data loss |
| Network loss | offline capability used only where policy permits |
| Clock skew | server/database authority used for authoritative time decisions |

## 36. Disaster Recovery Testing
Recovery is a testable behavior. A backup file existing on storage is not sufficient evidence that Sitolo can recover. DR tests must restore into an isolated environment and verify migrations, tenant counts, critical transaction counts, inventory balances, pending integration jobs, audit evidence and application startup.

```text
Quarterly DR drill:
  select recovery point
  restore backup/PITR into isolated environment
  verify schema/version
  run integrity checks
  calculate critical reconciliation metrics
  start application with restored DB
  test representative sale/read/report paths
  reconcile payment/EIS pending states
  record RTO/RPO achieved
  create corrective actions
```

The system must not claim zero data loss for device-local unsynchronized commands. Recovery evidence must distinguish server-authoritative committed state from local unsynchronized state.

## 37. Backup Integrity Tests
Backup jobs have their own success criteria. Tests should verify that backups are complete, restorable, encrypted according to deployment requirements and retained according to policy. Restore automation should fail loudly when backup metadata is corrupt or incompatible.

- At least one automated restore validation for each backup mechanism.
- Periodic full restore rehearsal, not only checksum validation.
- Validation of WAL/PITR chain where used.
- Protection against restoring production data into unsafe shared test environments.
- Post-restore reconciliation queries for business-critical tables.
- Captured restore duration and resulting RTO.

## 38. Security Scanning and Supply-Chain Tests
CI must combine source analysis with artifact and dependency verification. No single scanner is sufficient. The baseline includes secret scanning, dependency advisory scanning, static analysis/lints, SBOM generation, container/image scanning where containers are used, IaC scanning where infrastructure-as-code exists, and production artifact verification.

```text
PR security gate:
  format/lint
  unit + integration tests
  secret scan
  dependency audit
  static security analysis
  API security regression
  tenant isolation suite

Release gate:
  full security suite
  SBOM
  artifact provenance/signing verification
  DAST against disposable deployment
  migration tests
  performance baseline
  restore verification where required
```

NIST SSDF provides the secure-development framework vocabulary for integrating security practices into the SDLC. The test system should map each material practice to a repository control and evidence artifact rather than treating the standard as narrative documentation.

## 39. Rust-Specific Test Program
Rust/Cargo natively supports unit tests, integration tests and documentation tests. Sitolo should use these native mechanisms as the foundation while adding property testing, fuzzing and integration harnesses where needed.

```text
Suggested repository structure:
crates/
  domain/
    src/...
    tests/...
  application/
  infrastructure/
  api/
  integration-tests/
    tests/...
fuzz/
benches/
scripts/test/
```

Rust tests should use typed test fixtures rather than arbitrary JSON blobs where possible. Integration tests should instantiate public interfaces and avoid reaching private implementation details. Critical database suites should use an actual PostgreSQL service. Benchmark code must not be used as a substitute for correctness tests.

## 40. Benchmarking
Benchmarks are used for trend detection and regression diagnosis, not as release proof by themselves. Every benchmark records commit SHA, compiler/toolchain version, machine profile, database version/configuration, dataset size and benchmark parameters. Benchmarks should focus on hot paths such as inventory posting, pricing calculation, serialization, sync command validation and high-cardinality catalogue search.

```text
Benchmark evidence:
  benchmark name
  input cardinality
  p50/p95 where supported
  throughput
  allocations / memory where measurable
  compiler profile
  hardware profile
  baseline comparison
  regression threshold
  noise/confidence notes
```

## 41. SQL Query Testing
Critical queries require both correctness tests and query-plan review. Tests should cover tenant scope, indexes, null behavior, empty results, date boundaries, pagination and data volume. Where performance is a release concern, automated plan assertions should detect catastrophic plan changes without overfitting to every minor planner detail.

```text
Required query assertions:
  correct tenant predicate/scope
  expected result cardinality
  no duplicate rows from joins
  stable ordering for cursor pagination
  bounded date/filter range
  no full-table scan on critical indexed lookup unless intentionally justified
  no accidental N+1 query pattern in service orchestration
```

## 42. API Security Dynamic Testing
DAST should run against a disposable deployment with synthetic tenants and seeded permissions. Tests should directly exercise endpoints rather than depending only on browser UI paths. The security test account matrix should include ordinary cashier, manager, tenant administrator, support/JIT role and disabled/revoked identities according to the implemented IAM model.

- Object identifier substitution across tenants and branches.
- HTTP method tampering such as changing GET/POST/PUT/PATCH semantics where relevant.
- Content-type mismatches and parser confusion.
- Parameter pollution and duplicate-key handling.
- Unexpected JSON fields and mass-assignment attempts.
- Cursor manipulation and pagination abuse.
- Rate-limit evasion using alternate identifiers/paths.
- Authentication bypass attempts on internal/admin endpoints.
- Error-path probing for stack traces, SQL fragments or provider secrets.

## 43. File Upload and SSRF Tests
Any future import, receipt, image, attachment or URL-fetch capability must enter the security suite immediately. Tests must cover filename normalization, path traversal, archive expansion, MIME confusion, malicious SVG/HTML where applicable, oversized files and parser failures. URL fetching requires SSRF controls including IP/range validation, DNS rebinding resistance, redirect handling and strict allowlisting.

```text
SSRF negative corpus:
  127.0.0.1
  localhost
  RFC1918 ranges
  link-local addresses
  cloud metadata addresses where applicable
  IPv6 loopback/private ranges
  encoded IP forms
  redirect to private address
  DNS record that changes between validation and connect
```

## 44. Audit-Test Strategy
Audit evidence is part of the domain contract for sensitive operations. Tests must verify that audit records are created transactionally with the business operation when required, contain sufficient context for investigation, avoid sensitive secret material, and cannot be modified by ordinary merchant flows.

| Operation | Audit evidence to assert |
| --- | --- |
| Login/security event | actor/device/session/outcome/timestamp |
| Role change | actor, target, old/new authorization effect, reason |
| Sale finalization | sale reference, actor/device, relevant state transition |
| Refund | actor, approval if required, amount, original sale reference |
| Inventory adjustment | actor, reason, quantity delta, approval |
| Payment exception | provider reference, verification result, resolution |
| EIS exception | submission reference, config version, error class, resolution |
| Support/JIT access | support identity, tenant, scope, reason, expiration |

## 45. Error-Handling Tests
The API and domain model distinguish business rejection from infrastructure uncertainty because retry behavior differs. Tests must therefore verify error taxonomy, mapping and retry classification.

```text
Domain rejection:
  invalid transition / insufficient stock / authorization denied
  -> stable business error
  -> no unsafe retry

Infrastructure uncertainty:
  timeout / connection reset / dependency unavailable
  -> explicit unavailable/indeterminate state
  -> bounded retry where safe
  -> idempotency/reconciliation
```

Internal stack traces, SQL errors and provider response bodies must not be exposed to untrusted clients. At the same time, diagnostic telemetry must preserve enough structured evidence for engineers to identify the underlying failure.

## 46. Test Data Security and Privacy
Test datasets should assume logs, CI artifacts and crash reports can eventually be accessed by more people than production data. Therefore synthetic PII is preferred. Any approved sensitive test data must have explicit authorization, retention limits and cleanup. Test snapshots must be treated as data assets and cannot be casually uploaded to issue trackers.

- No production customer data in local developer fixtures.
- No production secrets in CI variables unless the specific operational test requires them and policy explicitly allows it.
- No payment credentials in fixtures.
- No MRA production terminal secrets in test fixtures.
- No auth tokens in golden HTTP recordings.
- Redact sensitive headers and payload fields in snapshots.
- Securely delete temporary exports after test execution.

## 47. Test Observability and Failure Artifacts
Every failed integration/security test should produce a reproducible artifact package without leaking secrets. Minimum metadata includes commit SHA, test name, environment build ID, migration version, seed where relevant, timestamps, correlation IDs and a concise failure classification.

```text
test-artifact/
  metadata.json
  junit.xml
  logs/
  traces/
  screenshots/          # UI/device only
  http-cassettes/       # sanitized only
  db-diagnostics/       # sanitized
  reproduction.md
```

Artifacts have retention rules. High-volume test logs should not become an unbounded data store. The retention window must be sufficient for release investigation and incident review.

## 48. CI Pipeline Design
```text
STAGE 0 — Repository safety
  secret scan
  dependency/policy checks
  workflow/static configuration checks

STAGE 1 — Fast correctness
  format
  lint
  unit tests
  docs/tests

STAGE 2 — Domain/security
  property/state-machine tests
  auth/authz/tenant isolation
  API regression

STAGE 3 — Real infrastructure
  PostgreSQL migrations
  DB integration
  concurrency
  worker/outbox/sync

STAGE 4 — Boundary contracts
  payment fixtures
  MRA fixtures
  external schema checks

STAGE 5 — Security dynamic
  DAST
  fuzz smoke
  resource-exhaustion smoke

STAGE 6 — Build/release evidence
  package/bundle
  SBOM
  provenance/signature
  artifact scan
```

The CI pipeline should be fail-closed for security-critical gates. If a required security suite cannot run because its environment is unavailable, the correct default is not to silently skip the suite for a production release. The release should either be blocked or use an explicitly approved emergency exception with recorded compensating controls.

## 49. Test Classification and Gate Policy
| Class | Example | PR gate | Release gate |
| --- | --- | --- | --- |
| P0 Critical | tenant escape, duplicate financial effect, secret exposure | Block | Block |
| P1 High | major auth bypass, inventory corruption, unsafe external side effect | Block | Block |
| P2 Material | important workflow defect with safe failure | Usually block affected changeset | Block if unresolved in affected scope |
| P3 Normal | localized UI/analytics defect | May warn | Track |
| P4 Cosmetic | nonfunctional visual difference | No | No |

Severity and gating are independent of test type. A unit-test failure caused by a critical invariant is P0 even though it is fast; a flaky screenshot comparison may be P3. Tests must be assigned owners and expected remediation windows.

## 50. Flaky Test Governance
A flaky test is a broken signal, not a normal property of distributed software. Tests may be quarantined only under an explicit process. Quarantine records the reason, owner, affected gate, detection time, issue reference and expiry date. Critical security/integrity tests may not be permanently quarantined.

```text
Flake process:
  detect repeated non-deterministic result
  classify environmental vs product defect
  capture seed/artifacts
  assign owner
  quarantine only with expiry
  add remediation test or infrastructure correction
  remove quarantine
  review stale quarantines weekly
```

## 51. Test Coverage Metrics
Coverage is one signal, not the definition of quality. Line/branch coverage should be monitored, but Sitolo prioritizes requirement and invariant coverage. A code path can be 100% executed while a tenant isolation defect remains untested.

| Metric | Meaning | Use |
| --- | --- | --- |
| Line/branch coverage | Executed code structure | Regression signal |
| Requirement coverage | Requirements linked to executable tests | Completeness |
| Invariant coverage | Domain invariants with positive/negative/boundary tests | Business correctness |
| Security-control coverage | Security control mapped to passing evidence | Security assurance |
| Mutation score | Dangerous mutations detected by tests | Test effectiveness |
| Contract coverage | External operations and failure classes represented | Integration safety |
| Recovery coverage | Failure modes with executable recovery tests | Resilience |
| Device coverage | Representative hardware/network states | Mobile reliability |

## 52. Test Traceability Matrix
Each high-value requirement receives a stable test identifier. The matrix should link requirement → design section → implementation module → automated test(s) → CI gate → evidence artifact. This makes security and operational claims auditable.

```text
Example:
REQ-SALE-IMMUTABLE
  -> domain_model § Sales / immutability
  -> sales aggregate
  -> TEST-SALE-IMMUTABLE-001..007
  -> gate: financial-integrity
  -> evidence: junit + DB assertion
```

The 48 existing security controls must be traceable into this matrix. The matrix is version-controlled and changes to security controls should trigger review of corresponding tests.

## 53. Test IDs and Naming
```text
TEST-UNIT-*       pure/domain tests
TEST-DOM-*        invariants/state machines
TEST-DB-*         PostgreSQL behavior
TEST-API-*        API integration
TEST-AUTH-*       authentication/session
TEST-AZ-*         authorization/tenant isolation
TEST-PAY-*        payment provider
TEST-EIS-*        MRA EIS
TEST-SYNC-*       offline/synchronization
TEST-SEC-*        cross-cutting security
TEST-FUZZ-*       fuzz targets
TEST-PERF-*       performance
TEST-DR-*         backup/recovery
TEST-MOB-*        real-device/mobile
TEST-E2E-*        end-to-end business flows
```

Test names should describe an invariant or scenario, not implementation syntax. A test named test_service_method_works provides weak diagnostic value; a test named TEST-AZ-003-cross-tenant-update-is-denied explains the security property directly.

## 54. End-to-End Business Scenarios
End-to-end tests represent merchant outcomes and therefore intentionally cross modules. The critical baseline scenario is the complete economic loop.

```text
Scenario E2E-CORE-001
  onboard tenant
  register branch/register/device
  create products and price
  receive stock
  open cash register/shift if configured
  sell item for cash
  sell item via mobile money/payment provider path
  reconcile payment
  produce tax/EIS submission or pending state according to environment
  view report
  close shift
  inspect audit evidence
```

Additional E2E scenarios must cover partial payment/split tender if supported, refund, stock adjustment, offline sale/reconnect, device revocation, permission denial and branch isolation.

## 55. Release Candidate Test Suite
```text
Release candidate minimum:
  all P0/P1 tests green
  full unit/property/state-machine suite
  PostgreSQL migration + integration suite
  authorization/tenant isolation suite
  payment contract suite
  MRA EIS contract suite where feature enabled
  sync crash/replay suite
  API compatibility suite
  security scans
  DAST
  fuzz smoke
  performance baseline
  representative Android device regression
  backup restore validation according to release tier
  SBOM + artifact verification
  no unauthorized production secrets/artifacts
```

A release candidate is not promoted based solely on aggregate pass percentage. The release gate is a policy evaluation over mandatory suites, severity, exceptions, evidence freshness and scope.

## 56. Production Certification Gate
Production certification requires an evidence pack. The exact evidence list may be expanded for regulated integrations, but the baseline includes source revision, build identifier, dependency inventory, test summary, security findings, migration verification, performance results, restore evidence, device regression results, external integration certification evidence and approved exceptions.

| Evidence | Owner |
| --- | --- |
| Source/build provenance | Engineering/Release |
| Automated test report | Engineering |
| Security verification report | Security |
| Tenant isolation report | Security/Backend |
| DB migration report | Backend/DBA |
| Performance report | Engineering/SRE |
| Device matrix report | Mobile |
| Payment provider contract evidence | Payments |
| MRA certification/integration evidence | Compliance/Integration |
| Restore drill evidence | SRE |
| Open-risk exception register | Engineering leadership |

## 57. Incident-to-Test Feedback Loop
Every production incident that exposes a missing control should create a regression test when technically possible. The post-incident sequence is: detect → contain → preserve evidence → recover → verify → reproduce → add regression → strengthen control → measure recurrence.

```text
Incident: duplicate external side effect
  immediate containment
  identify exact race/retry window
  add deterministic failing test
  patch implementation
  add database uniqueness/transaction guard if appropriate
  rerun broader replay/concurrency suite
  add monitoring signal
  close only after evidence
```

## 58. Manual Penetration Testing
Automated testing cannot replace human threat-led assessment. Before major production milestones, a qualified tester should examine tenant isolation, privilege escalation, payment/tax boundaries, offline trust, support/JIT access, admin functions, exposed infrastructure, business abuse and chained vulnerabilities.

- Manual testing must use an isolated environment and synthetic data.
- Credentials are scoped to the exact assessment environment.
- Findings must be reproduced and added to regression testing where appropriate.
- Critical findings block production until remediated or formally accepted under an exceptional governance process.
- The assessment must include direct API access rather than only UI interaction.

## 59. Test Tooling Recommendations
Tools are implementation choices. The strategy defines control objectives, not permanent vendor lock-in.

| Capability | Preferred direction |
| --- | --- |
| Rust unit/integration | cargo test |
| Rust lint/quality | cargo fmt + clippy with project policy |
| Property testing | proptest or equivalent |
| Fuzzing | cargo-fuzz/libFuzzer or equivalent |
| Database | real PostgreSQL in ephemeral CI environment |
| API functional | Rust HTTP test harness + external API test tooling as needed |
| Dynamic security | OWASP-aligned DAST tooling |
| Secret scanning | dedicated repository/history scanner |
| Dependencies | RustSec/Cargo audit ecosystem and equivalent package scanners |
| Mobile | flutter_test + integration_test |
| Load | k6, Gatling, Locust or equivalent depending on repository standards |
| Tracing | OpenTelemetry-compatible instrumentation |
| SBOM | CycloneDX/SPDX-compatible generator |
| Container scan | established image scanner if containers are deployed |

## 60. Test Harness Design
The initial test harness should make dangerous integration tests easy to write. The harness should provide helpers for creating tenant A/B, users with different roles, devices, products, stock, sales, payments and external-provider fixtures. Helpers must not bypass the very security controls being tested unless the test specifically prepares trusted preconditions.

```text
TestContext
  clock
  db
  api_client
  tenant_a
  tenant_b
  users
  devices
  provider_stub
  audit_reader
  event_store
  fault_injector

Helpers:
  create_tenant()
  create_user(role)
  authenticate(user, device)
  seed_product()
  receive_stock()
  finalize_sale()
  create_payment()
  push_sync_batch()
  assert_no_cross_tenant_access()
  assert_audit_event()
```

Test helpers should expose safe high-level operations. Direct database writes may be available for fixture setup, but tests of application behavior must exercise the public command path. Otherwise the suite can accidentally prove only that the database accepts data, not that the application correctly authorizes and validates the command.

## 61. Security Test Data Matrix
| Identity | Tenant | Branch | Device | Expected |
| --- | --- | --- | --- | --- |
| cashier A | A | A-1 | trusted | allowed within cashier scope |
| cashier A | A | A-2 | revoked | denied sensitive/offline operations per policy |
| manager A | A | A-2 | trusted | allowed according to manager policy |
| admin A | A | A-* | trusted | tenant admin only |
| user A | B | B-1 | trusted | no access to B |
| support JIT | target tenant | approved scope | approved device/session | only explicitly authorized support actions |
| expired user | A | A-1 | trusted | denied |
| disabled user | A | A-1 | trusted | denied |

## 62. Offline Security Matrix
| Attack | Expected control |
| --- | --- |
| Forge tenant_id | server derives/validates tenant from trusted identity/device scope |
| Forge role | role claims do not become authority without server verification |
| Replay command | stable command ID/idempotency blocks duplicate effect |
| Modify local payload | integrity/fingerprint validation detects mismatch where configured |
| Rollback checkpoint | server/client checkpoint logic prevents duplicate or skipped state |
| Use revoked device | sensitive sync denied |
| Oversized batch | bounded rejection |
| Unsupported protocol version | explicit rejection |
| Tamper with cached price | server validates authoritative price according to offline policy |
| Local clock manipulation | authoritative server semantics are not replaced by device time |

## 63. Payment and Tax Cross-Domain Testing
Payment and tax are deliberately asynchronous external effects. Tests must prove that each external subsystem can fail independently without corrupting the core sale. This is a critical architectural property.

```text
Sale transaction SUCCESS
  -> inventory SUCCESS
  -> payment intent state explicit
  -> EIS submission state explicit

Payment FAILURE
  -> does not roll back already-authoritative sale unless business workflow explicitly requires precondition before sale

EIS FAILURE
  -> sale remains authoritative
  -> tax state pending/retry/exception
  -> reconciliation path remains available
```

## 64. Worker and Queue Testing
Workers must be treated as at-least-once executors. Tests should assume duplicate execution, crashes after side effects, leases expiring and jobs being delivered out of order where dependencies allow it.

```text
Worker tests:
  claim job
  perform operation
  crash before ack -> lease expires -> retry
  retry same job -> idempotent outcome
  permanent failure -> dead-letter/exception according to policy
  dependency unavailable -> bounded retry/backoff
  queue backlog -> fairness protects critical financial jobs
  poison message -> does not block entire queue
```

## 65. Feature Flag and Configuration Testing
Configuration and feature flags can change business behavior without code changes. Tests must cover default, enabled and disabled states, tenant/plan scopes, invalid configuration and rollout/rollback. Security-critical flags must fail closed when configuration is missing or invalid.

## 66. Time-Based Tests
Temporal behavior is a high-risk source of edge cases. Tests must cover daylight/UTC boundaries where relevant, token/session expiry, price effective timestamps, MRA offline age thresholds, payment retry windows, queue backoff, report date ranges and retention cutoffs. Use UTC/internal canonical timestamps unless an explicit domain requirement says otherwise.

## 67. Localization and Currency Testing
Malawi-first operation requires currency and locale correctness. Money calculations must use the domain money model, not floating-point shortcuts. Tests should cover MWK formatting, rounding, negative corrections, zero values, large values and serialization boundaries. Regionalization must not silently change the currency of historical transactions.

## 68. Accessibility and UX Verification
Accessibility tests are not primarily a regulatory checkbox. For a merchant POS, readability, tap targets, error visibility and recovery states affect operational correctness. Automated widget semantics and focused manual testing should cover critical workflows. Accessibility regressions that can cause incorrect sales or cash actions should be treated as functional defects.

## 69. Browser/Web Admin Security Testing
The web/admin surface is a high-privilege interface even though it is not the core merchant client. Tests must include CSP, cookie/session settings where used, CSRF where relevant, clickjacking protection, dangerous redirects, source-map/debug exposure in production, admin route authorization, tenant selection tampering and privileged operation auditing.

## 70. Test Environment Drift
A test suite that passes only because the environment accidentally contains extra permissions or services is misleading. Environment definitions, roles, database extensions, feature flags, toolchain versions and service dependencies must be versioned. CI should compare expected environment invariants against actual test setup where practical.

## 71. Toolchain and Compiler Verification
Rust toolchain versions are part of build reproducibility. The repository should pin or explicitly declare the supported stable toolchain and CI should verify it. A dependency or compiler upgrade must rerun the full relevant suite, especially serialization, crypto-adjacent code, unsafe/FFI boundaries and performance-sensitive paths.

## 72. Test Failure Triage
Failure messages should classify the defect rather than dumping raw infrastructure noise. Recommended categories include ASSERTION, AUTHORIZATION, DATA_INTEGRITY, CONCURRENCY, DEPENDENCY, FLAKE, INFRASTRUCTURE, PERFORMANCE, SECURITY and COMPATIBILITY.

```text
Triage order:
  Is the test itself deterministic and valid?
  Did the product violate a requirement?
  Is the environment missing a declared dependency?
  Did a provider/schema change?
  Is there a race or ordering dependency?
  Is the failure security-sensitive?
  Does the issue require a new regression test?
```

## 73. Definition of Test Complete
A feature is test-complete only when its risk-appropriate verification exists and passes. Minimum criteria: executable requirements, positive/negative tests, state-machine tests where lifecycle complexity exists, database tests for persistence claims, authorization tests for scoped operations, concurrency tests for shared financial/inventory state, integration contracts for external dependencies, observability assertions, migration tests, and recovery tests for material failure modes.

```text
[ ] requirements mapped to tests
[ ] critical invariants executable
[ ] positive + negative paths
[ ] tenant/branch isolation covered
[ ] auth/session/security coverage
[ ] persistence constraints tested
[ ] concurrency/idempotency tested
[ ] external contract tested
[ ] offline behavior tested where applicable
[ ] failure/retry/recovery tested
[ ] metrics/logging evidence verified
[ ] performance budget measured where material
[ ] test artifacts reproducible
[ ] CI gate configured
```

## 74. Definition of Security Test Done
```text
[ ] no critical/high authorization bypass found
[ ] no cross-tenant data access
[ ] no financial replay side effect
[ ] no secrets in source/artifacts/logs
[ ] injection regression suite green
[ ] SSRF/file defenses tested where feature exists
[ ] resource limits tested
[ ] webhook/payment verification tested
[ ] offline command replay/revocation tested
[ ] CI/CD security controls tested
[ ] DAST completed for required milestone
[ ] mutation checks detect core security mutations
[ ] findings triaged with evidence
```

## 75. Definition of Production Testing Done
```text
[ ] release candidate is immutable and identified
[ ] mandatory automated suites green
[ ] no unapproved P0/P1 findings
[ ] migration path tested
[ ] backward compatibility evaluated
[ ] real-device matrix passed
[ ] performance baseline passed
[ ] external-provider contracts verified
[ ] DR/restore evidence current for release tier
[ ] SBOM/provenance evidence generated
[ ] monitoring/alerts verified
[ ] rollback procedure exercised or validated
[ ] certification evidence complete for regulated integrations
[ ] release owner signs evidence pack
```

## 76. Nightly Test Program
```text
Nightly:
  extended property tests
  fuzz smoke/short campaigns
  full PostgreSQL isolation suite
  concurrency stress
  sync crash/replay matrix
  payment/EIS contract suite
  DAST
  representative Android integration suite
  moderate load test
  backup restore smoke where environment permits
  mutation sample
  dependency/security scans
```

Nightly failures should not silently disappear into a dashboard. Owners, severity and remediation state must be assigned. Security and integrity regressions automatically reopen release risk even if ordinary feature builds still pass.

## 77. Weekly/Periodic Deep Verification
A periodic deep suite should perform broader fuzzing, larger load tests, full mutation campaigns, more extensive device coverage, dependency review and manual security checks. The exact cadence can be adjusted as operational maturity grows, but periodic deep verification must exist independently of PR velocity.

## 78. Test Architecture Decision Rules
- Prefer the smallest environment capable of proving the property.
- Escalate to real infrastructure when infrastructure semantics matter.
- Never mock away the security boundary being tested.
- Never mock away the transaction boundary when testing atomicity.
- Never let a provider stub assert only its own stub behavior; assert Sitolo’s handling of provider evidence.
- Prefer deterministic fault injection over timing sleeps.
- Keep production behavior and test behavior structurally similar where feasible.
- Build fixtures through stable APIs whenever testing the application path.
- Keep dangerous tests isolated from external production systems.

## 79. Known Testing Anti-Patterns
- Coverage theater: optimizing percentage instead of risk coverage.
- Happy-path monopoly: only testing successful requests.
- UI-only security: assuming the client enforces authorization.
- Mock everything: never executing real PostgreSQL transactions.
- Test-data pollution: shared mutable state across tests.
- Flake normalization: treating intermittent failures as harmless.
- Snapshot worship: accepting large snapshots without semantic assertions.
- Silent skips: marking security suites skipped when infrastructure is unavailable.
- Timing races: using arbitrary sleeps for concurrency tests.
- Production credentials in tests.
- Provider trust by fixture: treating a stubbed “success” as proof of real integration.
- No incident regression: fixing production bugs without adding tests.

## 80. Governance and Change Control
The test strategy changes when system invariants change. Changes to authentication, authorization, tenant isolation, inventory semantics, financial semantics, payment lifecycle, offline acceptance, tax evidence, external integrations, database transaction boundaries or production deployment controls require test-impact review and may require an ADR.

Every new module should answer before merge: what is the authoritative state; what can an attacker control; what can race; what can be replayed; what external dependency can fail; what data must never leak; what can be recovered; and which test proves each answer.

## 81. Initial Implementation Backlog
```text
TST-001  establish test workspace and CI commands
TST-002  establish PostgreSQL ephemeral harness
TST-003  implement tenant-isolation fixtures
TST-004  implement auth/session fixtures
TST-005  implement domain invariant helpers
TST-006  implement concurrency barrier utilities
TST-007  implement API security matrix
TST-008  implement idempotency/replay suite
TST-009  implement payment provider contract fixtures
TST-010  implement MRA EIS contract fixtures
TST-011  implement sync crash/replay harness
TST-012  implement Flutter integration harness
TST-013  establish low-end Android device profile
TST-014  establish fuzz targets
TST-015  establish DAST disposable environment
TST-016  establish SBOM/artifact evidence
TST-017  establish performance baseline
TST-018  establish restore test automation
TST-019  establish mutation corpus
TST-020  establish requirement/security traceability matrix
```

## 82. Phase Alignment
| Implementation phase | Testing that must exist before phase exit |
| --- | --- |
| Phase 1 Repository/CI | format/lint/unit/test harness/secret scan |
| Phase 2 Config/telemetry | config validation, redaction, telemetry assertions |
| Phase 3 Identity | auth/session/MFA/device tests |
| Phase 4 Tenant/IAM | tenant/branch/authz negative matrix |
| Phase 5 Database | migration/RLS/constraint/concurrency suite |
| Phase 6 Authorization | full policy matrix + mutation tests |
| Phase 7 Security framework | security harness operational and release-blocking |
| Phase 8 Catalogue | catalogue/pricing invariants |
| Phase 9 Inventory | ledger/concurrency/race suite |
| Phase 10 POS | atomic sale/e2e/idempotency |
| Phase 11 Payments | provider contract/replay/reconciliation |
| Phase 12 Sync | offline/crash/checkpoint/conflict suite |
| Phase 13 Procurement | receiving/stock/vendor invariants |
| Phase 14 Returns/Refunds/Cash | state-machine/approval/concurrency tests |
| Phase 15 MRA EIS | current contract + certification evidence |
| Phase 16 Reporting | scope/reconciliation/resource tests |
| Phase 17 Billing | entitlement/metering/payment tests |
| Phase 18 Admin | JIT/SoD/support audit tests |
| Phase 19 Hardening/DR | load/chaos/restore/device regression |
| Phase 20 Production certification | complete evidence pack |

## 83. Relationship to Other Sitolo Contracts
This document depends on, and must remain consistent with, security_implementation_spec.md, domain_model.md, database_design.md, api_contract.md, auth_authorization_spec.md, sync_protocol.md, payment_integration_spec.md and mra_eis_integration_spec.md. The testing strategy does not supersede those contracts. It is the verification layer that determines whether implementation satisfies them.

The existing architecture requires that domains are not considered done merely because code exists: their invariants, authorization, tenant boundaries, constraints, concurrency, idempotency, audit, events, offline behavior, API behavior, failure semantics, migration behavior and observability need executable evidence. This document formalizes that requirement.

## 84. External Standard Mapping
| Reference | How Sitolo uses it |
| --- | --- |
| OWASP ASVS 5.0.0 | web/API security verification baseline and requirement identifiers |
| OWASP API Security Top 10 2023 | API-specific threat/regression categories including BOLA and function/property authorization |
| NIST SP 800-218 | secure SDLC practices, evidence and defect prevention |
| Rust Cargo testing model | unit, integration and documentation tests as repository foundation |
| PostgreSQL testing model | real database regression, isolation and recovery testing concepts |
| Flutter testing model | unit/widget/integration and real-device verification |
| OpenTelemetry semantic conventions | stable telemetry fields used in test evidence and diagnostics |

## 85. Test Strategy Risk Register
| Risk | Impact | Control |
| --- | --- | --- |
| False confidence from mocks | Critical | real PostgreSQL + contract + E2E layers |
| Cross-tenant authorization regression | Critical | automated negative matrix on every relevant PR |
| Duplicate financial side effect | Critical | idempotency + concurrency + mutation tests |
| Offline data loss | Critical | durability/process-death/recovery tests |
| Provider contract drift | High | versioned contract fixtures + sandbox verification |
| Mobile performance failure | High | device matrix + profiling |
| Flaky tests hide regressions | High | flake governance + deterministic harness |
| CI bypass | Critical | release-blocking machine-enforced gates |
| Unrestorable backups | Critical | scheduled restore drills |
| Test environment drift | High | versioned infrastructure + reproducible setup |
| Missing compliance evidence | High | certification evidence pack and traceability |

## 86. Final Testing Contract
Sitolo testing is complete only when the implementation can demonstrate, with repeatable evidence, that hostile inputs meet explicit boundaries; authorized inputs meet explicit business invariants; concurrent operations cannot corrupt money or inventory; external dependencies cannot silently redefine internal truth; offline clients cannot become permanent authority; migrations preserve historical meaning; failures become explicit safe states; recovered systems can reconcile with external reality; and every material security or integrity control has a regression test capable of detecting its removal.

```text
BUILD
  -> STATIC CHECKS
  -> UNIT
  -> DOMAIN PROPERTIES
  -> SECURITY NEGATIVE TESTS
  -> REAL POSTGRESQL
  -> CONCURRENCY
  -> CONTRACTS
  -> SYNC/RECOVERY
  -> DEVICE
  -> PERFORMANCE
  -> DAST/FUZZ
  -> ARTIFACT VERIFICATION
  -> RELEASE EVIDENCE
  -> PRODUCTION
```

A passing unit suite is not the finish line. The finish line is evidence that the whole system preserves the contracts that matter: tenant isolation, identity and authorization, financial integrity, inventory integrity, synchronization correctness, payment and tax trust boundaries, auditability, recoverability and controlled availability.

## Appendix A — Minimum Commands
```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc
# property tests: repository-selected command
# fuzz smoke: repository-selected command
# Flutter unit/widget/integration: repository-selected command
# security scans: repository-selected commands
# DAST: disposable deployment only
# migration/DB suite: disposable PostgreSQL
```

The exact command names for property testing, fuzzing, scanners and load testing are implementation choices. CI must expose stable wrapper commands so developers do not need to memorize tool-specific flags. The wrappers become part of the repository contract.

## Appendix B — CI Evidence Schema
```text
{
  "commit_sha": "...",
  "build_id": "...",
  "toolchain": "...",
  "migration_version": "...",
  "suite": "security|domain|db|api|mobile|e2e|performance|dr",
  "status": "passed|failed|blocked",
  "test_count": 0,
  "failure_count": 0,
  "security_findings": 0,
  "artifact_digests": [],
  "environment_id": "...",
  "created_at": "..."
}
```

## Appendix C — Mandatory Negative Tests
```text
cross-tenant read
cross-tenant write
cross-branch write
forged role
forged approval
expired session
revoked device
replayed command
replayed webhook
same idempotency key / different payload
client-controlled sale amount
negative/overflow quantity
unauthorized refund
double refund
concurrent last-stock sale
checkpoint rollback
oversized sync batch
unsupported schema
invalid signature
provider amount mismatch
EIS timeout after local commit
SQL injection payload
SSRF private address
path traversal
secret-in-log regression
CI secret exposure regression
```

## Appendix D — References
1. OWASP Application Security Verification Standard (ASVS), stable 5.0.0: https://owasp.org/www-project-application-security-verification-standard/  
2. OWASP API Security Top 10: https://owasp.org/API-Security/  
3. NIST SP 800-218 Secure Software Development Framework: https://csrc.nist.gov/pubs/sp/800/218/final  
4. Rust Cargo testing guide: https://doc.rust-lang.org/cargo/guide/tests.html  
5. Rust cargo-test command: https://doc.rust-lang.org/cargo/commands/cargo-test.html  
6. PostgreSQL 18 regression testing: https://www.postgresql.org/docs/current/regress-run.html  
7. Flutter testing documentation: https://docs.flutter.dev/testing  
8. Flutter integration testing: https://docs.flutter.dev/testing/integration-tests  
9. OpenTelemetry Semantic Conventions: https://opentelemetry.io/docs/specs/semconv/  
10. OpenTelemetry general semantic conventions: https://opentelemetry.io/docs/specs/semconv/general/

Document end — Sitolo Testing Strategy, Phase 0 / File 09 of 16.

