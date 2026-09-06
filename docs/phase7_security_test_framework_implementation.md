# Sitolo — Phase 7 Security Test Framework Implementation Specification

**Document:** `phase7_security_test_framework_implementation.md`  
**Phase:** 7 — Security Test Framework  
**Status:** Deep implementation specification  
**Revision:** 1.0  
**Date:** 2026-09-06  
**Scope:** Rust backend, PostgreSQL, Axum API, workers, authentication/authorization, offline sync, payments, MRA EIS, Flutter/Tauri security boundaries, CI/CD and release evidence

---

## 0. Executive Position

Phase 7 converts Sitolo's security architecture into an executable, continuously maintained verification system.

This is not a generic testing checklist, a collection of scanners, or a thin wrapper around `cargo test`. It is a security verification subsystem whose job is to answer a harder question:

> **Can Sitolo continuously produce evidence that its security invariants remain true under valid, invalid, malicious, concurrent, replayed, partially failed, stale, and adversarial execution?**

The answer must be machine-verifiable.

The framework therefore treats tests as first-class security infrastructure:

```text
architecture / threat model / invariants
                |
                v
        security test contracts
                |
                v
        fixtures + identities
                |
                v
       deterministic test worlds
                |
                +------------------+
                |                  |
                v                  v
        positive scenarios   adversarial scenarios
                |                  |
                +---------+--------+
                          v
                    assertions /
                       oracles
                          |
                          v
                    evidence bundle
                          |
                          v
                     CI policy
                          |
                 +--------+--------+
                 |                 |
               PASS             BLOCK
```

The framework must test the same production boundaries that protect the actual system. It must not create a simplified security model that passes tests while production uses a different authorization path, database role, worker identity, transaction boundary, or integration adapter.

The existing Sitolo security-test harness explicitly requires executable verification for authentication, authorization, tenant isolation, database permissions/RLS, concurrency, workers, integrations, offline commands, replay, rate/resource controls, logging, secrets and supply-chain behavior. This phase turns that initial contract into the implementation architecture and operating model. fileciteturn9file4L580-L612

The existing testing strategy also requires invariant-first testing, positive/negative symmetry, deterministic tests, real PostgreSQL for critical database semantics, and real representative Android testing for release-relevant offline behavior. fileciteturn9file5L718-L817

The final system principle is:

> **No security claim without executable evidence, and no security test that can pass by bypassing the production security boundary.**

---

# 1. Governing Documents and Source Hierarchy

Phase 7 MUST consume, not redefine, the existing Sitolo contracts.

The authoritative relationship is:

```text
product scope
    ↓
business model
    ↓
system architecture
    ↓
security architecture
    ↓
security implementation specification
    ↓
domain model
    ↓
database design
    ↓
API contract
    ↓
authentication / authorization contracts
    ↓
sync / payment / MRA EIS contracts
    ↓
observability / deployment contracts
    ↓
threat model
    ↓
testing strategy
    ↓
security-test harness
    ↓
CI enforcement
    ↓
Phase 7 implementation
```

A test must not invent a new meaning for a domain invariant merely because that makes the fixture easier to construct.

The existing CI contract explicitly states that the repository must not create a second CI-specific interpretation of the product architecture. It also requires security tests, tenant-isolation tests, real PostgreSQL tests, financial/inventory invariant tests, offline security tests and artifact/security verification to remain release-enforcing. fileciteturn9file1L151-L185

## 1.1 Existing contracts Phase 7 directly implements

At minimum, Phase 7 SHALL remain aligned with:

- `security_implementation_spec.md`
- `security_architecture_design.md`
- `domain_model.md`
- `database_design.md`
- `api_contract.md`
- `auth_authorization_spec.md`
- `sync_protocol.md`
- `payment_integration_spec.md`
- `mra_eis_integration_spec.md`
- `testing_strategy.md`
- `threat_model.md`
- `observability_spec.md`
- `deployment_spec.md`
- `ci_enforcement.md`
- `security_test_harness.md`
- ADR-001 through ADR-025
- Phase 2 implementation specification
- Phase 3 implementation specification
- Phase 4 implementation specification
- Phase 5 implementation specification
- Phase 6 implementation specification

Phase 6 is particularly important because the security-test framework must test the actual authorization evaluator and its policy enforcement path, including deny-by-default, scoped access, object/property/state authorization, cache behavior, privileged actions, and fail-closed behavior.

## 1.2 Contract drift is itself a security failure

The framework MUST detect when implementation drifts from a security contract.

Examples:

```text
new endpoint added
    ↓
endpoint registry has no authorization metadata
    ↓
CI BLOCK
```

```text
new database table marked tenant-scoped
    ↓
no RLS/security test registration
    ↓
CI BLOCK
```

```text
new privileged command
    ↓
no positive + negative authorization matrix
    ↓
CI BLOCK
```

```text
new external side effect
    ↓
no idempotency / replay / failure test
    ↓
CI BLOCK
```

The framework therefore verifies not only runtime behavior, but also **security-test coverage of the architecture itself**.

---

# 2. Security Verification Objectives

Phase 7 has ten objectives.

## 2.1 Prove authorization boundaries

The framework SHALL prove:

- anonymous callers are denied when authentication is required;
- authenticated users cannot exceed their role or scope;
- users cannot cross organization boundaries;
- branch-scoped users cannot reach unauthorized branches;
- resource identifiers cannot bypass authorization;
- hidden object identifiers do not create trust;
- property-level authorization is enforced;
- function-level authorization is enforced;
- state transitions enforce actor requirements;
- step-up/MFA requirements are enforced;
- entitlements do not override security authorization;
- support access is separated from merchant authority;
- break-glass paths are bounded and audited;
- authorization cache state cannot widen privilege.

The API security threat model specifically identifies Broken Object Level Authorization and related authorization classes as first-order API risks. OWASP recommends checking object authorization on every endpoint that accesses a client-supplied identifier and not relying on identifier format or secrecy. citeturn320915search4turn320915search2

## 2.2 Prove tenant isolation

The suite SHALL establish that every tenant-scoped resource is inaccessible across tenants, whether accessed through:

- direct ID lookup;
- nested routes;
- search parameters;
- bulk operations;
- exports;
- reports;
- background jobs;
- retry paths;
- object storage references;
- cached representations;
- synchronization payloads;
- imported data;
- administrative APIs.

The assertion is stronger than “the API returned 403”. The framework SHALL also verify:

```text
no protected data leaked
no unauthorized mutation occurred
no event was emitted as a result of the denied operation
no cache entry became visible across principals
no outbox message was incorrectly created
no secondary object became discoverable
```

## 2.3 Prove database boundary defense

The suite SHALL prove that application authorization is reinforced by database permissions and PostgreSQL RLS.

Real PostgreSQL is mandatory for tests involving:

- RLS;
- role privileges;
- transaction semantics;
- row visibility;
- constraints;
- locking;
- deadlocks;
- isolation;
- query plans.

The existing CI contract explicitly rejects mock-only evidence for these properties. fileciteturn9file3L453-L492

## 2.4 Prove critical business invariants

Security testing SHALL include abuse of business logic, not only classic access control.

Examples:

- double sale finalization;
- duplicate payment application;
- invalid refund authority;
- quantity over-return;
- stock oversell under concurrency;
- unauthorized discount override;
- unauthorized price override;
- unauthorized cash adjustment;
- unauthorized tax configuration change;
- repeated payout request;
- stale approval reuse;
- approval of a changed target;
- replay of already-consumed financial action.

## 2.5 Prove failure safety

Security behavior must remain safe during:

- dependency timeout;
- database failure;
- cache outage;
- worker restart;
- process crash;
- network partition;
- provider timeout;
- provider duplicate callback;
- partial transaction failure;
- stale authorization data;
- schema migration;
- deployment transition.

The required property is normally:

```text
uncertainty
   ↓
deny / quarantine / retry safely
   ↓
never silently escalate privilege
```

## 2.6 Prove replay resistance

Tests SHALL cover replay of:

- access artifacts where applicable;
- refresh artifacts;
- MFA proof;
- recovery tokens;
- invitations;
- device enrollment artifacts;
- approvals;
- payment callbacks;
- EIS submissions;
- offline commands;
- synchronization batches;
- idempotency keys.

## 2.7 Prove adversarial input handling

The framework SHALL exercise malformed, oversized, ambiguous, boundary and hostile input.

At minimum:

- invalid UTF-8 where protocol permits raw bytes;
- Unicode confusables where relevant;
- integer overflow boundaries;
- negative quantities;
- extreme decimal values;
- large JSON objects;
- deeply nested JSON;
- unknown fields;
- duplicate fields where parser behavior matters;
- duplicate array members;
- path traversal payloads;
- SSRF payloads;
- malformed JWTs;
- algorithm confusion attempts;
- invalid signatures;
- expired tokens;
- future-dated tokens;
- wrong audience/issuer;
- oversized multipart data;
- malformed webhook payloads.

## 2.8 Prove observability safety

The suite SHALL verify that sensitive information does not leak through:

- application logs;
- traces;
- metrics labels;
- error responses;
- panic output;
- test artifacts;
- CI logs;
- snapshots;
- generated reports.

OWASP explicitly treats appropriate logging as part of authorization security and recommends centralized, consistent logging while avoiding unnecessary sensitive data. citeturn320915search2

## 2.9 Prove supply-chain and repository security controls

The framework SHALL verify security-sensitive build assumptions, including:

- dependency lock state;
- dependency vulnerability scanning;
- secret scanning;
- action pinning;
- least-privileged workflow permissions;
- artifact provenance;
- SBOM presence;
- cryptographic artifact identity.

These controls already exist as binding CI requirements and must be represented by executable gates. fileciteturn9file1L215-L239

## 2.10 Produce durable evidence

A test is useful only when an engineer can answer:

```text
what was tested?
against what build?
with what fixture state?
under what configuration?
using what identity?
what exact assertion failed or passed?
what source contract required the test?
can the result be reproduced?
```

---

# 3. Test Architecture

The framework SHALL be layered.

```text
                           SECURITY TEST SYSTEM
                                   |
        +--------------------------+---------------------------+
        |                          |                           |
     STATIC                     DYNAMIC                    EVIDENCE
        |                          |                           |
  policy lint                integration tests           reports
  endpoint registry          attack scenarios             manifests
  migration lint             concurrency                  SBOM links
  dependency policy          fuzz/property               provenance
  secret scanning            real PostgreSQL               traces
        |                          |                           |
        +--------------------------+---------------------------+
                                   |
                                   v
                             TEST ORCHESTRATOR
                                   |
                +------------------+------------------+
                |                  |                  |
            fixture DB        service runtime      providers
                |                  |                  |
                +------------------+------------------+
                                   |
                                   v
                               ORACLES
```

## 3.1 Test categories

The implementation SHALL maintain these categories as separately addressable gates:

| Category | Primary purpose | Typical cadence |
|---|---|---|
| Unit security tests | Pure policy/invariant correctness | Every commit |
| Domain/property tests | Invariant space exploration | Every PR |
| Authorization matrix tests | Positive/negative permission coverage | Every PR |
| API security tests | Endpoint boundary verification | Every PR |
| DB/RLS tests | Database enforcement | Every PR/release |
| Concurrency tests | Race/invariant integrity | PRs affecting critical paths; release |
| Integration security tests | Real subsystem interactions | Every PR/release |
| Fuzz/property tests | Malformed/adversarial input discovery | Scheduled + security-sensitive PR |
| Offline security tests | Device/sync trust boundaries | Sync-changing PR/release |
| Provider contract tests | External trust boundary behavior | Adapter-changing PR |
| Supply-chain tests | Build/repository security | Every PR/release |
| Deployment security tests | Environment/configuration | Release |
| Restore/security recovery tests | Recoverability of controls | Quarterly / major changes |
| Adversarial/manual tests | Human-driven attack discovery | Periodic |

---

# 4. Repository Layout

The repository SHALL use a structure that separates security tests by concern while keeping shared fixtures centralized.

Recommended structure:

```text
security-tests/
├── README.md
├── TESTING_CONTRACT.md
├── manifests/
│   ├── security-controls.toml
│   ├── endpoint-registry.toml
│   ├── resource-registry.toml
│   ├── role-registry.toml
│   ├── privileged-actions.toml
│   └── test-required-metadata.schema.json
├── fixtures/
│   ├── tenants/
│   ├── identities/
│   ├── memberships/
│   ├── devices/
│   ├── catalogue/
│   ├── inventory/
│   ├── sales/
│   ├── payments/
│   ├── tax/
│   └── attacks/
├── common/
│   ├── assertions/
│   ├── auth/
│   ├── authorization/
│   ├── db/
│   ├── http/
│   ├── concurrency/
│   ├── providers/
│   └── evidence/
├── integration/
│   ├── authz/
│   ├── tenant_isolation/
│   ├── rls/
│   ├── workers/
│   ├── payments/
│   ├── eis/
│   └── sync/
├── property/
├── fuzz/
├── regression/
├── abuse-cases/
├── snapshots/
└── scripts/
    ├── bootstrap.sh
    ├── provision-postgres.sh
    ├── collect-evidence.sh
    └── validate-manifests.sh
```

The exact Rust workspace path may differ according to the existing repository layout, but the conceptual separation SHALL remain.

The security test suite must be allowed to import the production domain and application crates needed to exercise real behavior. It must not reach into private implementation details merely to fake authorization success.

---

# 5. Test Environment Model

## 5.1 Disposable environment by default

Critical integration tests SHALL run in disposable environments.

```text
clean checkout
    ↓
validated toolchain
    ↓
build exact revision
    ↓
provision PostgreSQL
    ↓
apply migrations
    ↓
create realistic roles
    ↓
seed synthetic data
    ↓
start application/workers
    ↓
execute suites
    ↓
collect evidence
    ↓
destroy environment
```

No test suite may depend on production data.

## 5.2 Realistic security principals

Fixtures SHALL include at least:

```text
anonymous
OWNER_A
MANAGER_A
CASHIER_A
INVENTORY_A
AUDITOR_A
SUPPORT_AGENT
PLATFORM_SECURITY_OPERATOR
OWNER_B
MANAGER_B
CASHIER_B
DEVICE_A
DEVICE_B
REVOKED_DEVICE
EXPIRED_SESSION
MFA_REQUIRED_PRINCIPAL
```

These are synthetic identities.

The framework MUST also create distinct sessions, device identities and membership records rather than reusing one elevated test account across all scenarios.

## 5.3 Synthetic tenant topology

Minimum topology:

```text
TENANT_A
├── BRANCH_A1
│   ├── WAREHOUSE_A1
│   ├── REGISTER_A1
│   └── DEVICE_A1
└── BRANCH_A2
    ├── WAREHOUSE_A2
    ├── REGISTER_A2
    └── DEVICE_A2

TENANT_B
└── BRANCH_B1
    ├── WAREHOUSE_B1
    └── REGISTER_B1
```

Resources SHALL exist in every relevant boundary.

## 5.4 Fixture determinism

Fixtures MUST use deterministic IDs or recoverable random seeds.

Test failures must report:

```text
fixture version
random seed
database migration version
application revision
configuration fingerprint
policy version
test case identifier
```

No test should fail with “random environment” and become irreproducible.

---

# 6. Security Test Case Contract

Every security-sensitive test case SHALL have machine-readable metadata.

Example:

```toml
id = "AUTHZ-BOLA-001"
category = "authorization"
risk = "critical"
control = "SC-033"
owner = "security-platform"
requires_postgres = true
requires_network = false
release_blocking = true
```

Test metadata SHALL support:

- unique identifier;
- risk classification;
- mapped security control;
- source contract/ADR;
- owner;
- prerequisite environment;
- whether it needs real PostgreSQL;
- whether it needs concurrency;
- whether it is release-blocking;
- whether it is allowed on untrusted PRs;
- expected evidence outputs.

## 6.1 No orphan critical tests

Every Critical/High test must map to at least one architectural control.

Every Critical/High threat in the threat model must map to at least one executable test.

The threat model explicitly requires executable evidence for Critical/High threats and treats source-code intent and happy-path success as insufficient evidence. fileciteturn9file0L53-L80

## 6.2 No orphan security controls

Every security control must map to one or more of:

```text
unit test
integration test
property test
fuzz test
static analysis
manual security verification
operational control
```

The framework SHALL reject a control that has no verification mechanism.

---

# 7. Security Assertions and Oracles

A test framework is only as strong as its oracles.

A weak oracle:

```text
assert response.status == 403
```

is insufficient for high-risk operations.

A stronger oracle verifies the complete security effect:

```text
request denied
AND
no sensitive data returned
AND
state unchanged
AND
no unauthorized event emitted
AND
no unauthorized outbox message created
AND
no cache mutation
AND
no downstream external side effect
```

## 7.1 Standard denial oracle

For a forbidden command:

```text
expected:
    decision = DENY
    HTTP status = contract-defined denial
    state_before == state_after
    protected resource not disclosed
    no privileged event
```

## 7.2 No-side-effect oracle

The framework SHALL be able to snapshot critical state before and after a denied operation.

For financial resources, the oracle SHALL additionally check:

- ledger entries;
- balances;
- payment references;
- sale status;
- outbox rows;
- audit rows.

## 7.3 Cache oracle

A denied request must not populate a cache in a way that can later authorize another principal.

This extends the existing authorization cache test requirement.

## 7.4 Audit oracle

Security-relevant denial SHOULD create an appropriate audit/security event when the specification requires detective evidence, but tests must not accept a missing authorization control merely because an audit record was generated.

Audit is evidence, not authorization.

---

# 8. Authorization Test Framework

Phase 6 defines the runtime authorization engine. Phase 7 must systematically test it.

## 8.1 Decision matrix model

For each permission:

```text
principal
+ action
+ resource
+ tenant context
+ branch context
+ object state
+ attributes
+ assurance level
+ entitlement
+ time/context
--------------------------------
→ ALLOW / DENY / STEP_UP / APPROVAL_REQUIRED
```

The framework SHALL generate test cases from this matrix.

## 8.2 Positive/negative symmetry

For every allowed path:

```text
ALLOW A → resource A
```

there must be corresponding denial cases:

```text
DENY A → tenant B
DENY A → branch B
DENY insufficient role
DENY stale membership
DENY revoked device
DENY insufficient assurance
DENY forbidden property
DENY invalid state
```

This is explicitly required by the existing testing strategy. fileciteturn9file5L746-L766

## 8.3 Scope monotonicity

If authorization scope is reduced, effective authority MUST NOT increase.

Property:

```text
scope_after ⊆ scope_before
⇒
authority_after ⊆ authority_before
```

## 8.4 Revocation monotonicity

After:

```text
membership revoked
```

old authority must not generate new durable authority.

Test across:

- live session;
- stale session;
- stale access token;
- authorization cache;
- worker job;
- offline device;
- retry queue.

## 8.5 Role removal monotonicity

Removing a permission SHALL never create an allow result where a deny previously existed.

Property:

```text
permissions_after ⊆ permissions_before
⇒
authorized_actions_after ⊆ authorized_actions_before
```

## 8.6 Delegation boundedness

Delegating permission X must not imply permission Y unless explicitly defined.

Example:

```text
cashier may create sale
```

must not imply:

```text
cashier may assign role
cashier may change merchant price
cashier may issue unrestricted refund
```

## 8.7 Property-level authorization

The framework SHALL test fields individually.

Example mutation:

```json
{
  "quantity": 1,
  "unit_price": 1000,
  "discount": 999999,
  "finalized": true,
  "organization_id": "tenant-b"
}
```

The test must establish exactly which fields may be changed by the actor and verify that privileged fields cannot be mass-assigned.

OWASP classifies this as Broken Object Property Level Authorization and specifically warns against generic object binding and unrestricted field mutation. citeturn320915search6

## 8.8 State-level authorization

The test matrix SHALL combine authorization with state.

Example:

```text
CASHIER + OPEN_SALE      → permitted operations
CASHIER + FINALIZED_SALE → only explicitly allowed operations
CASHIER + REFUNDED_SALE  → restricted
OWNER + FINALIZED_SALE   → correction workflow, not silent mutation
```

---

# 9. Tenant Isolation Test System

Tenant isolation is a catastrophic-risk property and SHALL have a dedicated suite and protected CI status.

The existing CI contract already requires a dedicated tenant-isolation gate and tests both direct and indirect access paths. fileciteturn9file3L504-L557

## 9.1 Direct object tests

For every major resource family:

```text
Tenant A user
    ↓
Tenant B object ID
    ↓
DENY
```

Resources include at minimum:

```text
organizations
branches
locations
warehouses
registers
devices
products
prices
inventory
sales
cash sessions
payments
customers
suppliers
reports
exports
tax configuration
billing
support records
audit records
```

## 9.2 Enumeration tests

Do not test only known IDs.

The framework SHALL probe:

- sequential IDs where applicable;
- UUID substitution;
- nested endpoint IDs;
- query parameters;
- pagination cursors;
- sort/filter combinations;
- bulk request arrays;
- import identifiers;
- export job identifiers.

## 9.3 Query-shape isolation

A query that returns aggregate counts can leak tenant existence even when individual rows are hidden.

Therefore test:

```text
count
sum
min/max
search result count
pagination total
facets
statistics
report rows
```

against cross-tenant inputs.

## 9.4 Cache isolation

Cache keys MUST contain all security-relevant scope.

The test suite SHALL attempt:

```text
warm cache as A
request as B
```

and assert no reuse of A authorization or representation.

The existing IAM model explicitly requires cross-tenant cache contamination tests and fail-closed cache behavior. fileciteturn8file5L839-L872

---

# 10. PostgreSQL Security Test System

## 10.1 Real database required

RLS and privilege tests SHALL never be mocked.

PostgreSQL's current policy model supports default deny when RLS is enabled without applicable policies, and separates existing-row visibility (`USING`) from the validity of inserted/updated rows (`WITH CHECK`). The test suite SHALL verify both semantics where applicable. citeturn320915search3

## 10.2 Role validation

The test harness SHALL inspect database roles and fail if a runtime application role unexpectedly has:

```text
SUPERUSER
BYPASSRLS
CREATE ROLE
CREATE DATABASE
unbounded DDL privileges
ownership of security-sensitive objects
```

PostgreSQL 18 documents that superusers and roles with `BYPASSRLS` bypass row-level security; table ownership also has special RLS semantics. citeturn320915search0turn320915search3

## 10.3 RLS positive and negative matrix

For each protected table:

```text
same tenant + same scope        → ALLOW
same tenant + unauthorized scope → DENY
cross tenant                    → DENY
missing tenant context          → DENY
invalid tenant context          → DENY
stale context                   → DENY or contract-defined rejection
```

## 10.4 RLS bypass tests

The suite SHALL actively attempt to discover accidental bypasses through:

- elevated DB roles;
- owner execution;
- SECURITY DEFINER functions;
- unsafe views;
- alternate schemas;
- search path manipulation;
- reporting roles;
- maintenance roles.

PostgreSQL's documentation warns that functions, triggers and RLS policies can create security hazards and recommends tight control over object ownership and `search_path`. citeturn320915search1

## 10.5 Security-invoker view tests

Where views are used for tenant-scoped access, tests SHALL confirm whether the intended security context is based on the view owner or invoking user. PostgreSQL 18 explicitly documents the difference introduced by `security_invoker`. citeturn320915search7

## 10.6 Migration security tests

Every schema migration touching:

- RLS;
- roles;
- grants;
- authorization columns;
- tenant keys;
- indexes supporting authorization;
- security-sensitive functions;

MUST include before/after authorization verification.

---

# 11. Authentication and Session Security Tests

Phase 3 established identity/session security. Phase 7 verifies it continuously.

## 11.1 Token validation matrix

Test:

```text
valid issuer
valid audience
valid signature
valid expiration
valid not-before
valid key ID
correct algorithm
```

against:

```text
wrong issuer
wrong audience
expired token
not-yet-valid token
unknown key ID
invalid signature
wrong algorithm
missing required claim
malformed claim type
```

Every invalid artifact SHALL be rejected.

## 11.2 Session revocation

```text
login
→ session S1
→ revoke S1
→ request
→ DENY
```

Run with concurrent request timing as well.

## 11.3 Refresh rotation/replay

```text
refresh R1
→ issue R2
→ replay R1
→ detect reuse
→ contain session/family
```

Repeat with races:

```text
Thread A: redeem R1
Thread B: redeem R1
```

The expected outcome must be deterministic and documented.

## 11.4 Recovery replay

A recovery artifact must be one-time where the contract requires one-time use.

## 11.5 MFA replay

The suite must prove that consumed MFA evidence cannot be reused for unrelated privileged operations.

## 11.6 Step-up scope binding

Step-up authorization must be bound to the intended action/context.

```text
step-up for discount override
→ attempt ownership transfer
→ DENY
```

---

# 12. Privileged Operation Test System

High-risk actions SHALL be separately enumerated.

Typical categories:

```text
role changes
ownership transfer
price override
inventory adjustment
cash adjustment
refund approval
payment settlement action
API key creation
webhook configuration
MRA EIS configuration
tax configuration
export of sensitive data
support access
billing changes
security policy changes
```

Each privileged operation needs:

```text
who may initiate?
who may approve?
which assurance level?
which scope?
which state?
what limits?
what audit event?
what replay boundary?
what concurrency rule?
```

## 12.1 Separation of duties tests

The framework SHALL test that the same principal cannot self-approve a transaction where separation is required.

```text
initiator == approver
→ DENY
```

## 12.2 Approval target binding

Approval must remain bound to a specific target identity, version and relevant value.

```text
approve sale S1 value 1000
change S1 to value 100000
execute approval
→ DENY
```

---

# 13. Business Logic Abuse Testing

Authorization is not enough. Sitolo's security posture depends on preserving financial and inventory invariants under adversarial use.

## 13.1 Sale invariants

Test:

- finalize twice;
- finalize after cancellation;
- modify finalized sale;
- duplicate item rows;
- negative quantity;
- zero quantity;
- excessive discount;
- unauthorized price override;
- unauthorized tax override;
- payment amount mismatch;
- duplicate idempotency key with different payload;
- same sale submitted concurrently.

## 13.2 Inventory invariants

Test:

- oversell one unit concurrently;
- negative stock through race;
- duplicate adjustment;
- unauthorized stock correction;
- stale quantity mutation;
- cross-warehouse movement without authority;
- branch-to-branch movement outside scope.

## 13.3 Refund/return invariants

Test:

```text
refund > eligible amount → DENY
return > sold quantity   → DENY
refund twice             → DENY or idempotent replay
unauthorized branch      → DENY
```

## 13.4 Cash invariants

Test unauthorized operations on:

- opening balance;
- closing balance;
- cash adjustments;
- cash transfer;
- discrepancy recording;
- reconciliation.

---

# 14. Concurrency and Race Test Framework

Security tests must not assume serial execution where production allows concurrency.

The existing project explicitly requires race testing for membership revocation, role changes, device revocation and approvals. fileciteturn8file4L735-L779

## 14.1 Controlled scheduler

Critical concurrency tests SHOULD use a deterministic coordination mechanism.

Example:

```text
Thread A: revoke membership
Thread B: privileged mutation

A reaches transaction barrier
B reaches authorization barrier
release both

assert final state is contract-valid
```

A randomized scheduler MAY supplement deterministic schedules.

## 14.2 Required race families

At minimum:

- revoke membership vs request;
- role removal vs privileged command;
- device revocation vs offline upload;
- approval vs target mutation;
- idempotency insert vs duplicate request;
- payment callback vs local reconciliation;
- sale finalization vs inventory change;
- concurrent inventory adjustments;
- concurrent refund attempts;
- outbox insertion vs crash;
- worker retry vs manual reconciliation.

## 14.3 Race assertions

The expected result may be one of:

```text
exactly one succeeds
both safely serialize
one succeeds and one receives conflict
both become idempotent
both fail without side effect
```

“Sometimes allowed” is not an acceptable concurrency policy.

## 14.4 Deadlock testing

Critical multi-row operations SHALL be executed under repeated contention.

The suite SHALL verify:

- deterministic lock order where specified;
- no livelock;
- deadlock retry semantics where permitted;
- no duplicate financial effect after retry.

---

# 15. Property-Based Security Testing

Example-based testing catches known cases. Property-based testing searches the input space for counterexamples.

## 15.1 Required properties

At minimum:

```text
deny-by-default
scope monotonicity
revocation monotonicity
permission-removal monotonicity
delegation boundedness
idempotency
state-machine legality
money arithmetic correctness
quantity conservation
no cross-tenant result
```

## 15.2 Generated identities

Generate combinations of:

- memberships;
- role assignments;
- branch scopes;
- warehouse scopes;
- device states;
- assurance states;
- session states;
- permissions.

For each generated security context, verify authorization invariants.

## 15.3 Generated resource graphs

Generate resource relationships:

```text
organization
 └── branch
      └── warehouse
           └── inventory
```

and intentionally produce invalid cross-tenant references.

The property is:

```text
invalid ownership graph
→ cannot become an authorized graph through normal API operations
```

## 15.4 Shrinking

Failure cases MUST shrink to minimal reproducible examples.

A property failure report must include:

```text
seed
minimal generated input
policy version
resource graph
principal graph
database state
```

---

# 16. Fuzzing Strategy

Fuzzing SHALL target boundaries where parsing or policy interpretation can cause security failure.

## 16.1 API fuzz targets

Targets include:

```text
JSON request bodies
query parameters
path segments
headers
pagination cursors
sort/filter expressions
bulk command arrays
multipart metadata
webhook bodies
```

## 16.2 Authorization fuzz targets

Generate:

- unknown action names;
- invalid resource types;
- unknown permissions;
- contradictory scope claims;
- oversized context fields;
- malformed subject metadata;
- unknown policy version;
- conflicting attributes;
- stale versions.

The invariant is that malformed policy input must never produce an allow result merely because evaluation failed to parse it.

## 16.3 Parser differential testing

Where more than one serialization/parser implementation exists, the framework SHOULD compare normalized representations and reject dangerous ambiguity.

## 16.4 Crash and panic policy

Security fuzz targets SHALL treat:

- panic;
- process abort;
- memory exhaustion;
- uncontrolled recursion;
- unbounded allocation

as failures unless explicitly classified as impossible by design and proven with resource bounds.

---

# 17. API Security Test Framework

The endpoint registry SHALL be machine-readable.

Example:

```toml
[[endpoint]]
method = "POST"
path = "/v1/sales/{sale_id}/finalize"
action = "sale.finalize"
resource = "sale"
requires_authentication = true
requires_object_authorization = true
requires_scope = "branch"
release_blocking_tests = ["API-BOLA-001", "SALE-RACE-001"]
```

## 17.1 Endpoint coverage rule

A production endpoint MUST NOT ship without:

```text
authentication metadata
authorization metadata
negative test
input validation test
error contract test
```

Sensitive endpoints additionally require:

```text
replay test
concurrency test
audit test
```

## 17.2 BOLA matrix

Every endpoint receiving a resource ID SHALL have a test where the ID belongs to another tenant or unauthorized scope.

## 17.3 BFLA matrix

Every privileged endpoint SHALL have a lower-privilege principal attempt the same action.

Expected result:

```text
DENY
```

OWASP API5:2023 specifically identifies Broken Function Level Authorization and treats it as a distinct API security risk. citeturn320915search9

## 17.4 Property-level API tests

The framework SHALL verify both output and input field authorization.

Example:

```text
GET response excludes privileged field
PATCH rejects privileged field
```

---

# 18. Offline Security Testing

Offline capability is a major Sitolo trust boundary because the device can continue operating without current server connectivity.

The test framework SHALL prove that offline authorization is bounded and cannot become an indefinite server-authority mechanism.

## 18.1 Device states

Test:

```text
active
suspended
revoked
unknown
expired credentials
```

## 18.2 Offline command authenticity

Test:

- modified command;
- duplicated command;
- command from another device;
- command from another tenant;
- stale command;
- command outside device scope;
- malformed command;
- command with future timestamp;
- command replay after revocation.

## 18.3 Process death

Representative Android tests SHALL cover:

```text
create local command
→ kill process
→ restart
→ submit
```

and:

```text
revoke device on server
→ device remains offline
→ reconnect
→ attempt privileged upload
→ policy-defined denial/quarantine
```

The existing testing strategy explicitly requires representative Android devices for release-relevant offline behavior because emulator-only evidence is insufficient. fileciteturn9file5L809-L817

## 18.4 Tamper evidence

The framework SHALL test local database/queue modification where practical and verify that unauthorized changes cannot silently become authoritative server state.

---

# 19. Payment Security Test Framework

Provider integrations are external trust boundaries.

Tests SHALL assume provider inputs are untrusted evidence until verified.

Required cases:

```text
valid callback
invalid signature
wrong merchant/account
wrong amount
wrong currency
wrong reference
duplicate callback
out-of-order callback
unknown transaction
callback after cancellation
provider timeout
provider 5xx
network partition
```

## 19.1 Financial side-effect oracle

A provider callback must not create a financial state inconsistent with internal intent.

## 19.2 Replay

The same provider event must remain idempotent.

## 19.3 Unknown outcome

If a provider result is uncertain, the test SHALL verify that the system does not incorrectly infer success or failure.

---

# 20. MRA EIS Security Test Framework

Tax integration tests SHALL isolate external evidence from Sitolo's authoritative financial facts.

Test:

- invalid credentials;
- invalid payload signature where applicable;
- wrong terminal context;
- duplicate submission;
- replay;
- provider timeout;
- provider rejection;
- malformed provider response;
- unknown provider state;
- configuration version mismatch.

The existing architecture explicitly requires preservation of local sale facts when MRA submission fails; a failed tax submission must not mutate financial truth merely to make the external integration appear successful. fileciteturn9file9L1245-L1255

---

# 21. Worker Security Tests

HTTP tests alone are insufficient.

Workers SHALL be tested as independent security principals.

Every worker test must establish:

```text
worker identity
allowed queues
allowed resources
allowed tenant context
allowed actions
```

## 21.1 Job tampering

Test a job containing:

```text
wrong tenant ID
wrong object ID
wrong action
forged metadata
expired job
already completed job
```

## 21.2 Retry abuse

A retry must not become a second financial or external side effect.

## 21.3 Poison-message handling

Malformed or repeatedly failing jobs SHALL be quarantined according to the worker contract and must not create an infinite retry storm.

---

# 22. Resource Exhaustion and Abuse Testing

Security testing SHALL cover availability controls because resource exhaustion can become both an availability and security problem.

Test:

- large request body;
- large query result;
- expensive filter;
- expensive report;
- repeated expensive authorization checks;
- cache-miss storm;
- worker queue flood;
- synchronization batch flood;
- login/refresh abuse;
- webhook burst.

Assertions SHALL include:

```text
bounded CPU
bounded memory
bounded concurrency
bounded queue growth
bounded database connections
controlled retries
```

A test that merely checks HTTP 429 is incomplete if the request already consumes unacceptable server resources before the limiter executes.

---

# 23. Logging and Telemetry Security Tests

The framework SHALL intentionally trigger failures and inspect emitted logs.

Payloads SHOULD include canary secrets such as:

```text
TEST_SECRET_9f3...
TEST_TOKEN_7a...
TEST_PAYMENT_KEY_...
```

Then verify:

```text
canary secret absent from:
logs
traces
metrics
errors
CI output
artifacts
```

The logging tests SHALL also verify that correlation identifiers remain present when required but have bounded cardinality.

## 23.1 Error contract tests

For every sensitive error:

```text
client gets safe code
client does not get stack trace
client does not get SQL
client does not get provider secret
server retains diagnostic evidence separately
```

---

# 24. Secret Scanning Tests

CI SHALL scan:

```text
working tree
Git history where practical
build output
container layers
mobile packages
desktop packages
web assets
configuration samples
CI logs
uploaded artifacts
```

The existing CI contract explicitly defines these scanning targets and requires active secrets to trigger rotation/revocation and exposure analysis. fileciteturn9file3L367-L416

Security tests SHALL seed known fake secrets and confirm scanner detection to prove the scanner itself is operational.

A scanner that silently stops executing is a verification failure.

---

# 25. Static Security Analysis

Static analysis is not proof of runtime security, but it reduces the state space that dynamic testing must explore.

The framework SHALL integrate where appropriate:

```text
cargo fmt --check
cargo clippy
Rust compiler lints
cargo deny / equivalent dependency policy
secret scanner
SAST tooling
license policy
unsafe-code policy
workflow lint
IaC lint
container lint
migration lint
endpoint inventory validation
security metadata validation
```

If a static tool is mandatory and does not run, the pipeline result SHALL be `NOT RUN`, not `PASS`.

This follows the existing CI rule that distinguishes PASS, FAIL, NOT RUN, NOT APPLICABLE and EXPLICITLY WAIVED. fileciteturn9file1L149-L185

---

# 26. Dependency and Supply-Chain Tests

The security suite SHALL verify:

- lockfile consistency;
- unexpected dependency additions;
- known vulnerability status;
- dependency source policy;
- checksum/integrity state;
- high-risk dependency review;
- CI action pinning;
- reproducibility of build inputs;
- SBOM generation;
- artifact digest verification.

Dependency changes in authentication, cryptography, HTTP, serialization, SQL clients and build actions SHALL trigger targeted security regression tests. This is already required by the existing CI contract. fileciteturn9file3L361-L365

---

# 27. Security Regression Database

Every discovered vulnerability SHALL become a permanent regression test whenever technically possible.

A regression entry SHALL contain:

```text
vulnerability ID
internal incident ID
attack precondition
exploit path
fixed behavior
security control mapping
regression test ID
introduced version
fixed version
```

The test is never deleted merely because the issue is fixed.

## 27.1 Regression priority

Critical vulnerabilities SHALL receive:

```text
unit/invariant test where possible
integration test
negative test
CI release gate
```

High vulnerabilities should receive the same unless the security owner documents a justified alternative.

---

# 28. Mutation Testing

Security tests can become stale while remaining green.

Mutation testing SHALL therefore be applied to security-critical logic.

Examples of intended mutations:

```text
ALLOW → DENY
remove tenant predicate
invert branch comparison
skip MFA requirement
remove ownership check
ignore device revocation
accept stale version
skip idempotency lookup
remove RLS predicate
change approval threshold
```

A meaningful security suite must kill these mutants.

## 28.1 Mutation targets

Priority order:

1. authorization evaluator;
2. scope resolver;
3. tenant filtering;
4. state machine guards;
5. idempotency checks;
6. transaction boundary logic;
7. webhook verification;
8. sync acceptance;
9. security-sensitive DB functions;
10. privileged command handlers.

A low overall mutation score can hide catastrophic holes. Security mutation coverage SHALL therefore be reported separately from generic code mutation coverage.

---

# 29. Coverage Model

Traditional line coverage SHALL NOT be treated as the primary security metric.

Required security coverage dimensions are:

```text
control coverage
endpoint coverage
principal coverage
resource coverage
action coverage
state coverage
deny-path coverage
cross-tenant coverage
concurrency coverage
replay coverage
failure-mode coverage
```

## 29.1 Authorization matrix completeness

For every protected action, the framework should know:

```text
allowed principals
forbidden principals
allowed scopes
forbidden scopes
allowed states
forbidden states
assurance requirements
approval requirements
entitlement requirements
```

Missing rows are security debt.

---

# 30. Test Generation from Security Registries

The framework SHOULD generate parts of the suite from declarative registries.

Example:

```toml
[action]
name = "inventory.adjust"
resource = "inventory"
min_role = "inventory_manager"
scope = "warehouse"
requires_step_up = true
requires_approval = true
```

From that metadata, the framework can generate baseline checks:

```text
owner allowed
manager allowed when scoped
cashier denied
audit denied
cross-tenant denied
cross-warehouse denied
no-step-up denied
no-approval denied
replay denied
```

Generation is useful, but hand-written tests remain mandatory for complex business rules.

---

# 31. Evidence Format

Every CI security run SHALL produce a machine-readable evidence bundle.

Recommended contents:

```text
security-evidence/
├── manifest.json
├── build.json
├── toolchain.json
├── test-summary.json
├── control-results.json
├── endpoint-results.json
├── tenant-isolation-results.json
├── database-results.json
├── concurrency-results.json
├── fuzz-summary.json
├── dependency-results.json
├── secret-scan-results.json
├── artifact-digest.txt
├── sbom.json
└── logs/
```

The evidence manifest SHOULD include:

```json
{
  "revision": "<git-sha>",
  "artifact_digest": "sha256:<digest>",
  "toolchain": "<pinned-version>",
  "schema_version": "<migration-version>",
  "policy_version": "<authorization-version>",
  "security_suite_version": "<version>",
  "result": "PASS"
}
```

## 31.1 Evidence immutability

The tested artifact digest must be tied to the release artifact.

The existing CI contract requires deployment of the exact verified artifact rather than a later rebuild. fileciteturn9file1L227-L238

---

# 32. CI Test Pipeline

Recommended ordering:

```text
Stage 0 — repository integrity
    ↓
Stage 1 — static security validation
    ↓
Stage 2 — unit/domain security
    ↓
Stage 3 — authorization matrix
    ↓
Stage 4 — disposable PostgreSQL
    ↓
Stage 5 — API + tenant isolation
    ↓
Stage 6 — concurrency
    ↓
Stage 7 — integrations
    ↓
Stage 8 — fuzz/property subset
    ↓
Stage 9 — artifact/security evidence
    ↓
Stage 10 — release decision
```

## 32.1 Fail-fast vs complete evidence

Fast checks may fail early to conserve resources.

Release candidates SHOULD still collect all relevant evidence where practical rather than stopping after the first failure.

Example:

```text
tenant isolation failed
→ release decision already BLOCK
but
→ continue collecting diagnostic suites when safe
```

## 32.2 Security gate states

Every gate MUST resolve to one of:

```text
PASS
FAIL
NOT RUN
NOT APPLICABLE
EXPLICITLY WAIVED
```

The release engine SHALL treat:

```text
NOT RUN → BLOCK
```

for mandatory security controls.

---

# 33. PR Security Test Policy

Every pull request SHALL automatically classify changed areas.

Example mapping:

```text
src/auth/**
    → auth + session + recovery + revocation suites

src/authz/**
    → full authorization + tenant isolation + mutation suite

migrations/**
    → PostgreSQL + RLS + privilege suite

src/inventory/**
    → inventory concurrency + authorization suite

src/payments/**
    → replay + provider + financial invariants

src/sync/**
    → offline tamper + replay + idempotency suite

.github/**
    → workflow security + supply chain suite
```

The framework SHALL also maintain a baseline suite that runs even when a change classifier misses a dependency.

---

# 34. Test Ownership and Review

Every critical security test SHALL have an owner.

Ownership follows existing security ownership:

| Area | Owner |
|---|---|
| AuthN/session | Identity/security |
| AuthZ/IAM | Platform/security |
| Tenant isolation | Platform/domain |
| Financial invariants | POS/finance |
| Inventory | Inventory domain |
| Sync | Sync/platform |
| Payments | Integrations |
| MRA EIS | Tax/integrations |
| DB/RLS | Data/platform |
| CI/CD security | Platform/DevSecOps |
| Audit/telemetry | Security/SRE |
| DR/security recovery | SRE/platform |

The threat model explicitly assigns ownership across these boundaries. fileciteturn9file0L85-L106

A test may be reviewed by multiple owners when it crosses financial, security and infrastructure boundaries.

---

# 35. Test Flakiness Policy

A flaky security test is a security engineering defect until proven otherwise.

The test strategy explicitly says a flaky test is a reliability defect unless the test itself is proven faulty and corrected or replaced. fileciteturn9file5L704-L714

## 35.1 Prohibited practice

Do not:

```text
retry failed test blindly
quarantine critical security test indefinitely
mark flaky as non-blocking without approval
ignore intermittent tenant-isolation failure
```

## 35.2 Quarantine rule

A Critical/High security test may only be quarantined through an explicit, time-bounded waiver with:

```text
reason
owner
risk
mitigation
expiry
replacement test plan
```

An expired waiver becomes a release block.

---

# 36. Security Test Performance Engineering

Security testing must be deep without becoming operationally wasteful.

## 36.1 Fast lane

Every commit:

```text
unit authorization
critical domain invariants
endpoint metadata validation
secret scan
static analysis
small tenant matrix
```

## 36.2 Deep lane

Every pull request:

```text
real PostgreSQL
full tenant isolation
concurrency subset
integration security
property tests
```

## 36.3 Release lane

Before production:

```text
complete security suite
full tenant matrix
high-risk fuzzing
mutation sample
full provider contract suite
artifact/provenance validation
restore/security verification where required
```

The suite itself must have bounded execution and not become an excuse for developers to disable security gates.

---

# 37. Test Resource Isolation

Security tests must not contaminate one another.

Each test suite SHALL define its resource isolation level:

```text
process
container
schema
transaction
database
namespace
```

For RLS and role tests, process/database isolation is preferred where practical.

A test must not pass only because another test accidentally left permissive database state behind.

## 37.1 Reset discipline

At minimum:

```text
fresh database or deterministic reset
fresh identities
fresh caches
fresh queues
fresh provider state
```

for suites where state persistence would affect authorization outcomes.

---

# 38. Security Test Harness APIs

The common test layer SHOULD provide typed helpers.

Conceptual Rust API:

```rust
struct TestPrincipal {
    user_id: UserId,
    tenant_id: TenantId,
    session_id: SessionId,
    device_id: DeviceId,
    roles: Vec<RoleId>,
}

struct SecurityRequest {
    principal: Option<TestPrincipal>,
    action: Action,
    resource: ResourceRef,
    body: serde_json::Value,
}

struct SecurityObservation {
    decision: Decision,
    response: HttpResponseSnapshot,
    state_before: StateSnapshot,
    state_after: StateSnapshot,
    audit_events: Vec<AuditEventSnapshot>,
    outbox_events: Vec<OutboxEventSnapshot>,
}
```

The goal is to make complete security assertions easy without hiding production semantics.

---

# 39. Test Review Requirements

A security-sensitive pull request SHALL answer:

```text
What security property changed?
What attacker capability changes?
Which tests prove allowed behavior?
Which tests prove forbidden behavior?
Which race or replay cases matter?
Which database/RLS behavior changed?
What regression test protects the bug class?
What CI gate blocks recurrence?
```

A change that modifies authorization but adds only unit tests for a helper function is incomplete.

---

# 40. Threat-to-Test Traceability Matrix

The framework SHALL maintain a matrix at the repository level.

Example:

| Threat | Security control | Tests | Gate |
|---|---|---|---|
| BOLA | SC-033 | API-BOLA-001..N | Blocking |
| BFLA | SC-005 | API-BFLA-001..N | Blocking |
| Cross-tenant access | SC-006/33 | TENANT-001..N | Blocking |
| Privilege escalation | SC-005/39 | AUTHZ-ESC-* | Blocking |
| RLS bypass | SC-007 | DB-RLS-* | Blocking |
| Secret leakage | SC-011 | LOG-SECRET-* | Blocking |
| Replay | SC-24/25/39/40 | REPLAY-* | Blocking |
| Offline tampering | Sync controls | SYNC-SEC-* | Blocking |
| Payment replay | Payment controls | PAY-REPLAY-* | Blocking |
| Supply-chain compromise | SC-44 | SC-* | Blocking |

The exact numeric control mapping must reference the canonical 48-control matrix instead of inventing a second numbering system.

---

# 41. Security Regression After Incident

Incident response SHALL feed the test framework.

Workflow:

```text
incident
  ↓
contain
  ↓
understand exploit
  ↓
patch
  ↓
write regression test
  ↓
map to control/threat
  ↓
run against vulnerable reproduction
  ↓
verify fixed behavior
  ↓
add release gate if appropriate
  ↓
document residual risk
```

An incident is not fully closed when the vulnerable code is patched. The system is safer only when the regression has become durable evidence.

---

# 42. Adversarial Testing Program

Automated tests are necessary but not sufficient.

Periodic adversarial exercises SHALL target:

```text
horizontal escalation
vertical escalation
BOLA
property-level abuse
business flow abuse
offline tampering
payment replay
support impersonation
cache poisoning
RLS bypass
worker privilege abuse
CI security bypass
artifact substitution
```

The threat model explicitly states that testing does not replace independent penetration testing, provider security, cloud controls, code review, or production observability. fileciteturn9file4L654-L668

---

# 43. External Policy Engine Readiness Tests

Phase 6 deliberately retains an internal Rust authorization evaluator while leaving a clean policy-decision-point seam.

Phase 7 SHALL therefore test authorization at the decision contract boundary rather than coupling the security suite to a particular external engine.

This is important because modern policy systems such as OPA use a PDP/PEP model, while Cedar formalizes principal/action/resource/context evaluation and schema validation. Both approaches can improve policy separation, but introducing either is an architectural choice rather than a testing requirement. citeturn313118search12turn313118search11turn313118search1

## 43.1 Contract-level adapter test

If a future external PDP is introduced:

```text
same AuthorizationRequest
same policy version
same entity/context facts
       ↓
internal evaluator
external evaluator
       ↓
compare normalized decisions
```

Differences SHALL be treated as security-significant until explained.

## 43.2 Fail-closed adapter test

PDP unreachable:

```text
high-risk action
→ DENY
```

unless the specific operation is explicitly permitted by a pre-verified local authority model.

---

# 44. Why the Framework Uses Multiple Layers

A single test layer is insufficient.

Example:

```text
application test says allow
```

but:

```text
RLS blocks incorrectly
```

or:

```text
application test says deny
```

but:

```text
worker bypasses application path
```

or:

```text
API rejects attack
```

but:

```text
bulk endpoint succeeds
```

The defense therefore needs:

```text
static checks
+
unit policy tests
+
API integration tests
+
real DB/RLS tests
+
worker tests
+
concurrency tests
+
property/fuzz tests
+
operational evidence
```

---

# 45. Advantages

## 45.1 Security claims become executable

The biggest advantage is evidentiary. Security stops being a document-only property.

## 45.2 Cross-layer protection

Application authorization, database RLS, worker identity and external-boundary checks can be tested together.

## 45.3 Regression resistance

Critical failures become permanent executable tests.

## 45.4 Better incident response

The framework creates reproducible scenarios and machine-readable evidence.

## 45.5 Stronger tenant isolation

Cross-tenant access becomes an explicit release gate instead of an informal expectation.

## 45.6 Safer refactoring

A developer can refactor the authorization implementation while preserving the decision contract.

## 45.7 Better auditability

Every critical security control has a traceable owner, test and evidence path.

## 45.8 Offline security is treated as a real boundary

This avoids the common mistake of testing only online APIs while trusting the client when disconnected.

---

# 46. Disadvantages

## 46.1 Significant engineering cost

Real PostgreSQL, concurrency, representative devices, integration fixtures and evidence collection are materially more expensive than unit testing alone.

## 46.2 More complex test infrastructure

The repository acquires fixture management, test manifests, orchestration and result-processing logic.

## 46.3 Longer release verification

Deep security suites consume more compute and wall-clock time.

## 46.4 Maintenance burden

Every security-sensitive architectural change can require test updates.

## 46.5 False confidence risk

A large test suite can still miss an attacker path if the model itself is incomplete. Threat modeling and independent testing remain necessary.

## 46.6 Concurrency complexity

Race testing can be difficult to make deterministic and reproducible.

These costs are accepted because Sitolo's core risks—tenant isolation, money, inventory, mobile offline continuity, payments, tax integrations and multi-role access—are precisely the areas where shallow testing can create severe production failures.

---

# 47. Why This Over Alternatives

## Alternative A — Unit tests only

Rejected.

Unit tests cannot prove PostgreSQL RLS, real transaction behavior, worker boundaries, external replay behavior or environment-dependent security properties.

## Alternative B — Integration tests only

Rejected.

Integration-only suites become slow and can miss pure policy combinations and property-space exploration.

## Alternative C — DAST scanner as primary security control

Rejected.

DAST is useful for attack discovery but cannot prove internal business invariants, policy semantics or complex financial concurrency properties.

## Alternative D — Generic test framework with ad-hoc security cases

Rejected.

It has no architecture-level coverage model and makes omissions easy to hide.

## Alternative E — Mock PostgreSQL

Rejected for critical database properties.

The existing architecture explicitly requires real PostgreSQL for RLS, constraints, locking, isolation and transaction evidence. fileciteturn9file3L453-L492

## Alternative F — External authorization engine as the test framework

Rejected.

A policy engine can evaluate policy, but it does not prove the API, repository, database, worker, payment, sync and business-transaction boundaries around that policy.

## Alternative G — Full end-to-end UI tests only

Rejected.

UI tests are too high-level and can accidentally rely on client restrictions. Authorization must be tested directly at server boundaries.

---

# 48. CI Enforcement Rules

The following SHALL be release-blocking:

```text
missing mandatory security test
failed tenant isolation test
failed authorization matrix test
failed RLS test
unexpected DB privilege
failed privileged-action test
failed replay test
failed financial invariant
failed inventory concurrency invariant
failed sync security test for sync-changing release
failed payment security test for payment-changing release
failed secret scan
mandatory security scanner not run
artifact integrity mismatch
missing required evidence
```

The existing CI specification explicitly requires skipped mandatory checks to be treated as failures and tenant-isolation, financial and inventory regressions to block release. fileciteturn9file1L215-L239

---

# 49. Waiver System

Security waivers MUST be explicit and rare.

A waiver record SHALL include:

```text
waiver_id
control_id
test_id
reason
risk assessment
mitigation
approver
created_at
expires_at
scope
replacement plan
```

No perpetual waiver.

Critical tenant-isolation and authorization bypasses MUST NOT receive ordinary release waivers without executive/security architecture review and an explicit emergency procedure.

---

# 50. Test Data Security

Test fixtures themselves can leak secrets.

Requirements:

- synthetic customer data only;
- synthetic payment credentials;
- synthetic tax credentials;
- no production database dumps;
- no production JWT signing keys;
- no production device identifiers;
- no production PII;
- test secrets must be scoped to test systems;
- generated artifacts must be scanned.

The secret scan SHALL run against test artifacts as well as source files.

---

# 51. Security Test Naming Standard

Recommended naming:

```text
AUTHZ-*       authorization
TENANT-*      tenant isolation
DBSEC-*       database/RLS
SESSION-*     session security
MFA-*         MFA/recovery
DEVICE-*      device identity
API-*         API security
SALE-*        sales invariants
INV-*         inventory invariants
PAY-*         payments
EIS-*         MRA EIS
SYNC-*        offline/sync
WORKER-*      worker security
REPLAY-*      replay/idempotency
RACE-*        concurrency
FUZZ-*        fuzzing
LOG-*         logging/telemetry
SUPPLY-*      supply chain
DEPLOY-*      deployment security
REG-*         regression
```

Identifiers must be stable across refactors.

---

# 52. Security Test Development Workflow

When implementing a security-sensitive feature:

```text
1. identify security invariant
2. identify threat/abuse case
3. register control mapping
4. add positive test
5. add negative test
6. add boundary test
7. add race/replay test if relevant
8. add DB/RLS test if relevant
9. add regression test if bug-driven
10. wire to CI gate
11. generate evidence
12. review ownership
```

The feature is not security-complete until these steps are satisfied.

---

# 53. Definition of Test Completeness

A security-sensitive feature is complete only when:

```text
[X] threat identified
[X] invariant identified
[X] actor matrix defined
[X] positive tests exist
[X] negative tests exist
[X] boundary tests exist
[X] concurrency behavior defined where relevant
[X] replay behavior defined where relevant
[X] DB/RLS behavior tested where relevant
[X] logs/errors tested
[X] evidence generated
[X] CI gate exists
[X] owner assigned
[X] regression test exists for known defects
```

This directly reflects Sitolo's established security definition of done: code compiling is not sufficient, and security tests plus CI enforcement are mandatory completion criteria. fileciteturn9file8L1171-L1191

---

# 54. Release Certification Procedure

Before a production release, the release system SHALL be able to produce:

```text
build identity
↓
security suite result
↓
tenant-isolation result
↓
DB/RLS result
↓
concurrency result
↓
integration result
↓
secret scan result
↓
dependency/supply-chain result
↓
artifact digest
↓
SBOM/provenance
↓
approval evidence
```

A release candidate is not certified because “all tests passed” in the abstract. It is certified when the relevant mandatory security evidence corresponds to the exact artifact being promoted.

---

# 55. Production Certification Gate Matrix

| Gate | Commit | PR | Release |
|---|---:|---:|---:|
| Static security | MUST | MUST | MUST |
| Unit authorization | MUST | MUST | MUST |
| Tenant isolation smoke | SHOULD | MUST | MUST |
| Full tenant isolation | NO | MUST when relevant | MUST |
| Real PostgreSQL | targeted | MUST | MUST |
| RLS/privilege | targeted | MUST when DB touched | MUST |
| Concurrency | targeted | MUST when relevant | MUST for critical paths |
| Replay | targeted | MUST when relevant | MUST |
| Offline | NO | MUST when sync touched | MUST when release relevant |
| Provider contracts | NO | targeted | MUST when adapter relevant |
| Fuzzing | scheduled | targeted | deep subset |
| Secret scan | MUST | MUST | MUST |
| Dependency scan | MUST | MUST | MUST |
| Artifact provenance | NO | optional | MUST |
| SBOM | NO | optional | MUST |
| Restore/security drill | NO | NO | required by cadence |

---

# 56. Operational Runbooks

## 56.1 Tenant-isolation failure

```text
1. BLOCK release.
2. Preserve evidence bundle.
3. Identify exact endpoint/resource.
4. Confirm whether application authorization or DB/RLS failed.
5. Reproduce from clean fixture state.
6. Determine exploitability.
7. Contain affected deployment if already released.
8. Patch.
9. Add regression test.
10. Re-run complete tenant suite.
11. Review adjacent resource families.
12. Document residual risk.
```

## 56.2 Security scanner did not execute

```text
1. Result = NOT RUN.
2. Release remains BLOCKED.
3. Diagnose tool/environment failure.
4. Re-run from controlled environment.
5. Do not manually mark PASS.
```

## 56.3 Flaky critical security test

```text
1. Mark infrastructure/test defect.
2. Preserve failing seed/state.
3. Determine race vs test bug.
4. Fix determinism.
5. Re-run repeated schedules.
6. Do not remove release-blocking status without approval.
```

## 56.4 Secret leak discovered in test artifact

```text
1. Treat as active exposure until proven otherwise.
2. Revoke/rotate if credential could be valid.
3. identify affected artifacts/logs.
4. purge downstream copies where appropriate.
5. add regression canary test.
6. verify old credential no longer works.
```

---

# 57. Security Framework Failure Modes

## Failure Mode A — Tests bypass production code

Example:

```text
mock authorization = allow
```

while production authorization is never executed.

Mitigation:

```text
integration tests must invoke real authorization boundary
```

## Failure Mode B — Tests share an elevated DB role

Mitigation:

```text
realistic runtime role
explicit privilege assertions
```

## Failure Mode C — Only happy paths are tested

Mitigation:

```text
positive/negative symmetry enforced by metadata
```

## Failure Mode D — Test passes while side effect happened

Mitigation:

```text
state-diff + outbox + audit + provider side-effect oracle
```

## Failure Mode E — Cache masks revocation

Mitigation:

```text
revocation race suite + cache isolation + cache failure tests
```

## Failure Mode F — Fuzzing exists but findings are not regressionized

Mitigation:

```text
fuzz failure → minimized case → regression test
```

## Failure Mode G — Scanner silently fails

Mitigation:

```text
NOT RUN != PASS
```

---

# 58. Performance and Scalability of the Test Platform

The security framework itself must scale as Sitolo's endpoint and resource surface grows.

## 58.1 Partition by risk

Use a risk-ranked execution model.

```text
Critical controls → every PR
High controls     → every PR or affected PR
Medium controls   → scheduled / affected PR
Low controls      → periodic
```

## 58.2 Parallelization

Independent test worlds MAY run in parallel.

Do not parallelize tests that intentionally exercise shared database locks unless the shared schedule is itself the test subject.

## 58.3 Database pooling

The test harness must bound its own PostgreSQL connections to avoid masking application pool issues or exhausting CI infrastructure.

## 58.4 Artifact size

Evidence must be rich enough to reproduce failures but must not upload megabytes of duplicate logs for every successful test.

---

# 59. Framework Security

The security test framework is itself security-sensitive.

It can contain:

- test credentials;
- synthetic secrets;
- exploit payloads;
- privileged test fixtures;
- policy metadata;
- artifact signing configuration.

Requirements:

```text
least privilege
no production credentials
secret scanning
reviewed dependencies
pinned CI actions
restricted artifact access
controlled write permissions
```

A compromised test harness could create false assurance, which is itself a security incident.

---

# 60. Research Basis and Engineering Interpretation

The Phase 7 architecture is based on the following current security guidance and technical references.

## OWASP Authorization guidance

OWASP recommends least privilege, deny-by-default, permission validation on every request, server-side enforcement, safe failure behavior, authorization logging, and authorization-specific unit/integration tests. It also recommends attribute/relationship-aware controls where role-only authorization becomes insufficient. citeturn320915search2

## OWASP API Security Top 10

The current published API Top 10 identifies BOLA, Broken Object Property Level Authorization, Broken Function Level Authorization and unrestricted sensitive business flows as separate API risks. These categories directly shape Sitolo's endpoint, object, property and business-flow tests. citeturn320915search4turn320915search6turn320915search9

## NIST ABAC

NIST defines ABAC in terms of evaluating subject, object, requested operation and contextual attributes against policy, rules or relationships. Sitolo uses this as a conceptual basis for testing scoped and contextual authorization rather than relying on display-role assertions alone. citeturn313118search2

## NIST Zero Trust

The zero-trust approach supports treating access decisions as explicit policy decisions rather than assuming network location or prior access is inherently trustworthy. This is consistent with Sitolo's request-time authorization and worker/service boundaries.

## OPA and Cedar as future policy-engine references

OPA separates policy decision points from policy enforcement points and recommends colocating policy evaluation where practical to reduce latency and network failure exposure. Cedar models authorization as principal/action/resource/context evaluation and uses schemas to make policy structures explicit. These references support a future policy-engine adapter, not an immediate technology commitment. citeturn313118search12turn313118search11turn313118search1

## PostgreSQL security

PostgreSQL 18 documentation is particularly important to Phase 7 because security evidence must reflect real RLS, role and view semantics. RLS can default-deny rows, `USING` and `WITH CHECK` have different semantics, and superusers/BYPASSRLS roles can bypass row security. PostgreSQL also documents risks around security-definer functions and object ownership. citeturn320915search3turn320915search0turn320915search1turn320915search7

---

# 61. Implementation Sequence

The implementation SHOULD proceed in this order.

```text
PR-701
security-test crate/module foundation

PR-702
fixture and identity framework

PR-703
security metadata manifests

PR-704
authorization matrix generator

PR-705
tenant-isolation suite

PR-706
real PostgreSQL/RLS suite

PR-707
session/MFA/recovery suite

PR-708
API BOLA/BFLA/property suite

PR-709
business invariant abuse suite

PR-710
concurrency/race harness

PR-711
property-based tests

PR-712
fuzz targets

PR-713
payment/EIS security suites

PR-714
offline security suite

PR-715
worker security suite

PR-716
logging/secrets/supply-chain tests

PR-717
mutation-testing subset

PR-718
evidence bundle generation

PR-719
CI gate wiring

PR-720
release certification integration
```

Each PR must leave the repository in a stronger security-verification state and must not create a later dependency that allows an earlier insecure feature to pass.

---

# 62. Phase 7 Exit Criteria

Phase 7 is complete only when all of the following are true.

## Framework

```text
[ ] security-test framework exists
[ ] test manifest format is defined
[ ] test IDs are stable
[ ] owners are assigned
[ ] evidence format is defined
```

## Authorization

```text
[ ] positive/negative authorization matrix exists
[ ] BOLA tests cover protected object endpoints
[ ] BFLA tests cover privileged endpoints
[ ] property-level authorization is tested
[ ] state-level authorization is tested
[ ] revocation monotonicity is tested
[ ] cache isolation is tested
```

## Tenant isolation

```text
[ ] cross-tenant direct ID tests pass
[ ] cross-tenant indirect enumeration tests pass
[ ] nested-resource tests pass
[ ] aggregate/report tests pass
[ ] async worker tests pass
[ ] export tests pass
```

## Database

```text
[ ] real PostgreSQL test environment exists
[ ] runtime DB role privileges are verified
[ ] RLS tests pass
[ ] BYPASSRLS risk is tested
[ ] SECURITY DEFINER paths are tested
[ ] security-invoker/view behavior is verified where used
[ ] migration security regression tests exist
```

## Concurrency

```text
[ ] membership revocation race
[ ] role-change race
[ ] device-revocation race
[ ] approval race
[ ] idempotency race
[ ] inventory race
[ ] payment replay race
```

## Adversarial testing

```text
[ ] property tests exist
[ ] fuzz targets exist
[ ] shrink/reproduction works
[ ] malformed input suite passes
[ ] business-flow abuse suite passes
```

## Integrations

```text
[ ] payment replay/security tests pass
[ ] MRA EIS trust tests pass
[ ] offline tamper/replay tests pass
[ ] worker security tests pass
```

## Repository/CI

```text
[ ] secret scanning is blocking
[ ] dependency scan is blocking
[ ] security scanners distinguish NOT RUN from PASS
[ ] endpoint registry validation is blocking
[ ] required controls have tests
[ ] exact artifact is linked to evidence
[ ] SBOM/provenance is linked where required
```

## Operational assurance

```text
[ ] failure runbooks exist
[ ] waiver process exists
[ ] test flakiness policy exists
[ ] security regression process exists
[ ] evidence retention policy exists
```

---

# 63. Final Engineering Position

Sitolo does not become secure merely because it has authentication, authorization, PostgreSQL RLS, encrypted transport, a threat model or a large number of unit tests.

Security depends on whether those controls continue to work together when assumptions are stressed.

The Phase 7 framework therefore verifies the complete path:

```text
attacker input
    ↓
HTTP / sync / worker boundary
    ↓
identity
    ↓
session/device state
    ↓
authorization decision
    ↓
scope resolution
    ↓
resource validation
    ↓
transaction
    ↓
repository
    ↓
PostgreSQL role / RLS
    ↓
state transition / invariant
    ↓
outbox / audit
    ↓
external provider if applicable
    ↓
observable evidence
```

The most important property is not test quantity.

It is **independence of failure modes**.

If one authorization layer fails, another layer should detect or block the violation. If a cache becomes stale, the authoritative check must prevent escalation. If a client is compromised, the server must remain authoritative. If PostgreSQL receives a cross-tenant query, RLS should provide another boundary. If a payment provider sends a forged event, signature and state correlation should reject it. If a worker receives a poisoned job, worker authorization and transaction invariants should stop it. If a security scanner stops running, CI must refuse to call that a pass.

That produces the desired architecture:

```text
                 SECURITY TEST FRAMEWORK
                           |
        +------------------+------------------+
        |                  |                  |
      PROVE              BREAK              OBSERVE
        |                  |                  |
    invariants         adversarial         evidence
        |                  |                  |
        +------------------+------------------+
                           |
                           v
                    RELEASE DECISION
                           |
                    +------+------+
                    |             |
                  PASS          BLOCK
```

The result is a security system that is continuously challenged by its own repository before attackers get the opportunity to do the same thing in production.

---

# 64. Non-Negotiable Rules

1. Security tests MUST exercise production security boundaries.
2. Critical authorization behavior MUST have positive and negative tests.
3. Tenant isolation MUST be release-blocking.
4. Real PostgreSQL MUST be used for RLS, privileges, locking and isolation tests.
5. Database runtime roles MUST NOT gain accidental `SUPERUSER` or `BYPASSRLS` authority.
6. Security scanner non-execution MUST NOT be interpreted as success.
7. Critical tests MUST NOT be permanently quarantined.
8. Security failures MUST become regression tests whenever possible.
9. Denials MUST prove absence of unauthorized side effects, not merely an HTTP status.
10. Authorization caches MUST never become the authority store.
11. Revocation MUST be tested across sessions, caches, workers and devices.
12. Financial and inventory invariants MUST be tested under concurrency.
13. Provider callbacks MUST be treated as untrusted external evidence until verified and correlated.
14. Offline clients MUST NOT become final server authority.
15. Test artifacts MUST be scanned for secrets.
16. Exact release artifacts MUST be tied to security evidence.
17. A security waiver MUST be explicit, bounded and auditable.
18. Threat-model changes MUST create or update executable tests.
19. New endpoints MUST include authorization metadata and negative security coverage.
20. The framework itself is a security boundary and must be reviewed as such.

---

# 65. References

Primary external references consulted for this implementation:

- OWASP Authorization Cheat Sheet — least privilege, deny-by-default, request-time authorization, safe failure, logging and authorization testing.
- OWASP API Security Top 10 (2023) — BOLA, Broken Object Property Level Authorization, Broken Function Level Authorization and sensitive business-flow abuse.
- NIST SP 800-162 — Attribute Based Access Control concepts and subject/object/action/context policy evaluation.
- NIST Zero Trust Architecture, SP 800-207.
- Open Policy Agent documentation — PDP/PEP deployment and policy evaluation architecture.
- Cedar Policy Language documentation — principal/action/resource/context authorization model and schema validation.
- PostgreSQL 18 documentation — RLS policy semantics, role/RLS bypass behavior, function security and security-invoker views.

Current web sources used in the research pass:

- https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html
- https://owasp.org/API-Security/editions/2023/en/0xa1-broken-object-level-authorization/
- https://owasp.org/API-Security/editions/2023/en/0xa3-broken-object-property-level-authorization/
- https://owasp.org/API-Security/editions/2023/en/0x10-api-security-risks/
- https://csrc.nist.gov/pubs/sp/800/162/upd2/final
- https://www.openpolicyagent.org/docs/deploy
- https://docs.cedarpolicy.com/
- https://docs.cedarpolicy.com/auth/authorization.html
- https://www.postgresql.org/docs/current/sql-createpolicy.html
- https://www.postgresql.org/docs/current/sql-createrole.html
- https://www.postgresql.org/docs/current/perm-functions.html
- https://www.postgresql.org/docs/current/sql-createview.html

---

# 66. Document Integrity Statement

This document is an **implementation specification for Phase 7**, not a replacement for the existing Sitolo security architecture, threat model, testing strategy, initial security-test harness, or CI contract.

It exists to deepen those documents into a concrete security verification architecture and to close the gap between:

```text
security requirement
        ↓
security control
        ↓
executable test
        ↓
evidence
        ↓
CI enforcement
        ↓
release decision
```

That chain is the intended implementation boundary for Sitolo Phase 7.
