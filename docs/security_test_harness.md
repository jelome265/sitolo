# Sitolo — Initial Security-Test Harness

**Document:** `security_test_harness.md`  
**Phase:** 0 — Architecture / Contracts / ADR Freeze  
**File:** 15 of 16  
**Status:** Initial implementation contract  
**Revision:** 1  
**Date:** 2026-09-05  
**Scope:** Backend, database, API, workers, integrations, sync, mobile/desktop security boundaries, CI/CD and release evidence

---

# 0. Executive Purpose

This document defines the initial **security-test harness** that converts Sitolo's security architecture, implementation security contract, domain model, database model, API contract, authentication/authorization contract, offline synchronization protocol, payment integration contract, MRA EIS integration contract, testing strategy, threat model, observability specification and deployment specification into executable security verification.

The objective is not to create a collection of scanners. The objective is to create a repeatable system capable of proving that security-critical assumptions remain true as the codebase evolves.

The harness therefore treats security verification as an engineering system with:

```text
fixtures
↓
test environments
↓
trusted test identities
↓
attack scenarios
↓
assertions / oracles
↓
evidence
↓
CI policy
↓
release decision
```

The governing principle is:

> **A security control is not considered implemented until the repository can demonstrate its intended behavior under both valid and adversarial conditions.**

The existing threat model explicitly requires every Critical or High threat to map to at least one executable test, with the highest-risk classes receiving multiple independent defensive tests. The existing security architecture similarly requires cross-tenant, authorization, replay, concurrency, offline, supply-chain, API and recovery tests. fileciteturn20file0L611-L693

This harness operationalizes those requirements.

---

# 1. Scope and Non-Goals

## 1.1 In scope

The harness covers:

- authentication;
- authorization;
- tenant isolation;
- branch/resource isolation;
- object-level authorization;
- property-level authorization;
- MFA and recovery;
- device identity and revocation;
- session lifecycle;
- API request validation;
- injection resistance;
- CSRF/CORS/browser controls where applicable;
- file upload and path safety;
- SSRF controls;
- payment provider trust controls;
- webhook verification and replay resistance;
- MRA EIS integration trust controls;
- offline command authenticity/integrity;
- sync idempotency;
- sync checkpoint safety;
- database permissions and RLS;
- transaction atomicity;
- concurrency and race conditions;
- worker safety;
- rate limits and resource exhaustion;
- logging and telemetry redaction;
- secrets and credential exposure;
- CI/CD workflow security;
- dependency and artifact integrity;
- SBOM/provenance/signature verification;
- backup/restore evidence where security depends on recoverability.

## 1.2 Not in scope as a replacement

The harness is not a replacement for:

- independent penetration testing;
- MRA certification or regulatory approval;
- cloud provider security controls;
- identity provider security assurance;
- operating-system security;
- PostgreSQL vendor testing;
- manual threat modeling;
- code review;
- production observability.

The harness exists alongside these controls.

## 1.3 Guiding rule

No test framework may create a second version of the application's security model merely to make tests convenient.

The test harness must exercise the same:

```text
API boundary
application authorization
repository scoping
DB roles/RLS
worker identity
integration adapters
```

used in production.

---

# 2. Security Verification Philosophy

Sitolo's security model is not simply:

```text
authentication → access
```

It is:

```text
untrusted input
    ↓
authentication
    ↓
trusted principal
    ↓
session/device state
    ↓
tenant membership
    ↓
resource scope
    ↓
permission
    ↓
object authorization
    ↓
property/state checks
    ↓
domain invariant
    ↓
transaction
    ↓
idempotency/concurrency
    ↓
authoritative commit
    ↓
audit/outbox
    ↓
external effect
    ↓
reconciliation
```

The harness must therefore test both the individual controls and the interaction between controls.

A test that proves `sales:create` is denied to a cashier is useful.

A stronger test proves that a cashier cannot circumvent the same decision by:

```text
changing tenant_id
changing branch_id
changing sale_id
calling an undocumented route
calling the API directly instead of the UI
submitting an offline command
replaying an old command
using a revoked device
using a stale session
manipulating an approval field
```

The second category is where enterprise security evidence becomes meaningful.

---

# 3. Harness Architecture

## 3.1 Logical structure

```text
                    SECURITY TEST HARNESS
                             |
        +--------------------+--------------------+
        |                    |                    |
      Fixtures            Attack DSL          Test Oracles
        |                    |                    |
        +--------------------+--------------------+
                             |
                 Disposable Test Environment
                             |
      +-----------+----------+-----------+-----------+
      |           |                      |           |
 PostgreSQL     API                    Workers     Adapters
      |           |                      |           |
      +-----------+----------------------+-----------+
                             |
                       Evidence Store
                             |
                       CI Gate Engine
```

## 3.2 Principle of disposable environments

Security integration tests must run against isolated environments.

A test environment should be reproducible from:

```text
source commit
lockfiles
migration set
container/image references
fixture version
configuration template
```

Production credentials are prohibited in automated security tests.

## 3.3 Environment tiers

### Tier 0 — Pure unit security tests

Fast tests with no network or external process.

Use for:

- policy predicates;
- token claim validation logic;
- canonicalization;
- error mapping;
- redaction functions;
- command fingerprint calculations;
- state-machine authorization;
- parser rejection.

### Tier 1 — Service integration

Run the application with disposable dependencies.

Use for:

- API authz;
- tenant scope;
- session state;
- repository behavior;
- transaction semantics;
- provider adapters with mocks/fixtures.

### Tier 2 — Real PostgreSQL

Mandatory for:

- RLS;
- unique constraints;
- transaction atomicity;
- locks;
- deadlocks;
- isolation;
- idempotency uniqueness;
- migration validation.

The prior testing contract explicitly rejects mocks as proof for RLS, locking, uniqueness and transaction behavior. fileciteturn20file4L520-L584

### Tier 3 — Security integration / disposable staging

Use for:

- DAST;
- network boundary testing;
- browser security;
- SSRF;
- file handling;
- rate limits;
- authentication flows requiring real browser/client behavior.

### Tier 4 — Adversarial/manual

Threat-led penetration testing and abuse testing.

---

# 4. Repository Layout

Recommended initial structure:

```text
sitolo/
├── tests/
│   ├── security/
│   │   ├── fixtures/
│   │   ├── authn/
│   │   ├── authz/
│   │   ├── tenant/
│   │   ├── api/
│   │   ├── database/
│   │   ├── payments/
│   │   ├── eis/
│   │   ├── sync/
│   │   ├── workers/
│   │   ├── files/
│   │   ├── ssrf/
│   │   ├── ci/
│   │   ├── supply_chain/
│   │   ├── resource_limits/
│   │   ├── concurrency/
│   │   └── recovery/
│   ├── integration/
│   ├── property/
│   ├── fuzz/
│   └── contract/
├── crates/
│   └── ...
└── scripts/
    ├── security/
    ├── ci/
    └── evidence/
```

The exact crate/test structure can differ, but security tests must remain discoverable and independently reportable.

---

# 5. Test Identity and Fixture Model

## 5.1 Mandatory tenant fixtures

At minimum:

```text
TENANT_A
TENANT_B
```

## 5.2 Users

```text
OWNER_A
MANAGER_A
CASHIER_A
INVENTORY_A
PROCUREMENT_A
AUDITOR_A
SUPPORT_A

OWNER_B
MANAGER_B
CASHIER_B
```

## 5.3 Branches

```text
BRANCH_A1
BRANCH_A2
BRANCH_B1
```

## 5.4 Locations/registers

```text
LOCATION_A1
LOCATION_A2
REGISTER_A1
REGISTER_A2
REGISTER_B1
```

## 5.5 Devices

```text
DEVICE_A1
DEVICE_A2
DEVICE_B1
REVOKED_DEVICE
SUSPENDED_DEVICE
```

## 5.6 Sessions

```text
ACTIVE_SESSION_A
EXPIRED_SESSION_A
REVOKED_SESSION_A
WRONG_AUDIENCE_SESSION
WRONG_ISSUER_SESSION
```

## 5.7 Domain fixtures

```text
PRODUCT_A
PRODUCT_B
SKU_A
SKU_B
SALE_A
SALE_B
PAYMENT_INTENT_A
PAYMENT_INTENT_B
PAYMENT_ATTEMPT_A
EIS_SUBMISSION_A
PURCHASE_ORDER_A
GOODS_RECEIPT_A
```

## 5.8 Security fixture invariants

Fixtures themselves must be validated before tests run.

For example:

```text
CASHIER_A
  → TENANT_A
  → BRANCH_A1
  → REGISTER_A1
  → SALE_A
```

but:

```text
CASHIER_A
  ✗ TENANT_B
  ✗ BRANCH_A2 where not assigned
  ✗ REGISTER_B1
```

If fixture construction accidentally grants forbidden access, the security test suite can produce false passes.

---

# 6. Security Test Context

Every security test should be capable of expressing:

```text
principal
session
organization
branch
device
permission
resource
resource state
request
expected result
expected side effects
```

Conceptual test context:

```rust
struct SecurityContext {
    principal: PrincipalFixture,
    session: SessionFixture,
    device: DeviceFixture,
    tenant: TenantFixture,
    branch: BranchFixture,
    permissions: PermissionSet,
}
```

This is a test model only. It must not become a parallel production authorization engine.

---

# 7. Assertions: The Test Oracle Problem

A security test is incomplete if it checks only an HTTP status.

For example:

```text
403
```

is not enough if the server accidentally performed a write before returning `403`.

Security tests should assert:

```text
HTTP outcome
DB state
audit state
outbox state
external calls
metrics/events where relevant
```

For a denied mutation:

```text
response = denied
AND authoritative state unchanged
AND no financial movement
AND no outbox side effect
AND no privileged audit mutation
```

For a successful mutation:

```text
response = success
AND correct tenant
AND correct resource
AND expected domain state
AND expected audit evidence
AND expected outbox behavior
```

---

# 8. Authentication Security Suite

## 8.1 Token validation

Test rejection for:

```text
missing token
malformed token
expired token
not-before violation
wrong issuer
wrong audience
wrong signature
unknown key
algorithm mismatch
unexpected token type
```

## 8.2 Session validation

Test:

```text
active session → allow
expired session → deny
revoked session → deny
logout → session no longer accepted
credential change → configured revocation behavior
```

## 8.3 MFA

Test:

```text
MFA required → block privileged operation until satisfied
invalid factor → deny
expired factor → deny
replayed factor → deny where one-time semantics apply
MFA reset → privileged workflow + audit
```

## 8.4 Account recovery

Test:

```text
valid recovery → allowed
expired recovery → denied
reused recovery → denied
modified recovery → denied
cross-user recovery attempt → denied
account enumeration behavior → normalized
```

## 8.5 Authentication downgrade

Attempt to change:

```text
MFA required
→ lower-assurance session
```

without satisfying policy.

---

# 9. Tenant Isolation Suite

This is one of the highest-priority suites in the repository.

## 9.1 Read matrix

For every major tenant-owned resource:

```text
Tenant A principal → Tenant A object → ALLOW
Tenant A principal → Tenant B object → DENY
Tenant B principal → Tenant A object → DENY
```

Apply to:

- products;
- prices;
- inventory;
- sales;
- payments;
- refunds;
- customers;
- suppliers;
- purchase orders;
- reports;
- exports;
- audit views;
- tax records;
- billing records.

## 9.2 Mutation matrix

Repeat for:

```text
POST
PUT
PATCH where legitimate
DELETE where legitimate
command endpoints
bulk endpoints
sync ingestion
worker execution
```

## 9.3 Identifier substitution

Attempt:

```text
/path/resource/B_OBJECT
body.resource_id = B_OBJECT
body.tenant_id = B
query.tenant_id = B
header tenant selector = B
```

The server must derive authority from trusted context rather than accepting the supplied tenant identifier as authority.

## 9.4 Nested-resource bypass

Test paths such as:

```text
/org/A/sale/B
/branch/A/inventory/B
/sale/A/payment/B
/report/A/export/B
```

Nested URLs do not establish ownership.

## 9.5 Search isolation

Search endpoints are tested because cross-tenant leaks can happen through:

```text
autocomplete
barcode search
customer search
global identifiers
error messages
pagination cursors
```

## 9.6 Export isolation

Attempt to export Tenant B data using Tenant A credentials.

Assert:

```text
no rows from B
no metadata from B
no object-store artifact for B
no downloadable URL
```

---

# 10. Branch / Resource Scope Suite

Tenant isolation is not sufficient. The same tenant may contain multiple branches with restricted staff scope.

Example:

```text
CASHIER_A
allowed: BRANCH_A1
forbidden: BRANCH_A2
```

Test:

- inventory access;
- register access;
- cash operations;
- price overrides;
- sales;
- returns;
- reports;
- stock counts;
- procurement.

Verify that changing `branch_id` or `register_id` cannot broaden authority.

---

# 11. Object-Level Authorization / BOLA Suite

For every resource ID exposed by API:

```text
valid ID + unauthorized principal
```

must be tested.

Do not restrict BOLA tests to endpoints named `/{id}`. Object authorization exists in:

```text
body references
query references
nested references
command payloads
bulk arrays
file metadata
export requests
sync commands
worker payloads
```

## 11.1 BOLA mutation test

Scenario:

```text
Cashier A
sale = SALE_B
POST /refund
```

Expected:

```text
authorization denied
no refund created
no payment side effect
no inventory reversal
no audit mutation claiming success
```

---

# 12. Function-Level Authorization Suite

Test that users cannot invoke functions outside their permission set even if they know the route.

Required examples:

```text
cashier → refund high value → deny
cashier → user role change → deny
inventory clerk → tax configuration → deny
manager → tenant ownership change → deny
ordinary user → support impersonation → deny
```

The tests must call APIs directly, not through UI controls.

---

# 13. Property-Level Authorization Suite

Some operations are allowed but individual fields require stronger permissions.

Example:

```text
User may edit product name
but may not change tax classification or cost basis.
```

Test:

```text
allowed field only → allow
protected field only → deny
mixed allowed/protected request → reject protected mutation or require explicit elevated workflow
```

Never silently drop security-sensitive fields unless the API contract explicitly defines such behavior.

---

# 14. State-Machine Authorization Suite

A permission does not imply every lifecycle transition.

Example:

```text
FINALIZED sale
→ void
```

may be invalid even if the user has a general sale permission.

Test every high-risk state transition with:

```text
authorized + valid state
 authorized + invalid state
 unauthorized + valid state
 unauthorized + invalid state
```

This prevents accidental authorization bypasses that occur when code checks permission but forgets state.

---

# 15. Approval / Separation-of-Duties Suite

High-risk operations that require approval must be tested for:

```text
missing approval
fake approval
approval for wrong target
approval for wrong amount
approval from unauthorized user
self-approval where prohibited
expired approval
reused approval
modified approval after creation
```

The test must verify both authorization and business state.

Example:

```text
CASHIER requests refund
MANAGER approves REFUND_A
request payload modified from 5,000 to 50,000
```

Expected:

```text
approval no longer matches
operation denied
```

---

# 16. Device Security Suite

## 16.1 Device lifecycle

Test:

```text
registered → active → allow
suspended → deny sensitive operations
revoked → deny
replacement → new identity
```

## 16.2 Revoked device sync

A revoked device attempts:

```text
sync push
sync pull
sale command
inventory command
payment command
```

Expected:

```text
reject/quarantine according to protocol
no new authoritative mutation
security signal emitted
```

The existing threat model explicitly requires revoked-device and cloned-device offline tests. fileciteturn20file5L666-L680

## 16.3 Device substitution

Attempt to submit a command whose `device_id` differs from the authenticated device context.

Expected:

```text
reject
```

## 16.4 Cloned credential scenario

Duplicate a test device credential into a simulated second device identity.

The server must not accept authority merely because the credential material is valid if its device binding is invalid.

---

# 17. Offline Command Security Suite

## 17.1 Valid command

```text
valid identity
valid device
valid tenant
valid scope
valid schema
valid command ID
valid payload
```

→ accepted.

## 17.2 Command replay

Submit the same command twice.

Expected:

```text
first → authoritative result
second → same result/equivalent idempotent response
no second financial effect
```

## 17.3 Command ID reuse with modified payload

```text
command_id = X
payload = A

replay:
command_id = X
payload = B
```

Expected:

```text
IdempotencyConflict
```

No second effect.

## 17.4 Fingerprint mismatch

If the protocol uses canonical payload fingerprints, mutate:

```text
amount
SKU
quantity
branch
currency
```

while retaining the original command ID.

The command must be rejected.

## 17.5 Checkpoint rollback

Attempt to present an older checkpoint after the server has advanced.

Expected:

```text
controlled rejection/recovery
```

Never silently roll the authoritative checkpoint backward.

## 17.6 Batch abuse

Test:

```text
oversized command count
oversized byte size
excessive nested depth
malicious dependency graph
cycle
invalid sequence
```

Expected:

```text
bounded parse
explicit rejection
no excessive CPU/memory use
```

---

# 18. Injection Security Suite

## 18.1 SQL injection

Inject into:

- search terms;
- identifiers;
- sort fields;
- filters;
- report parameters;
- import data;
- provider references.

Assert:

```text
no semantic query change
no authorization bypass
no data leakage
```

Prefer allowlisted sort/filter fields over dynamic SQL construction.

## 18.2 Search/parser injection

Test:

```text
quotes
wildcards
regex-like payloads
very long strings
unicode confusables
null bytes
embedded control characters
```

## 18.3 JSON/protocol abuse

Test duplicate fields, unexpected fields, incorrect types, arrays where scalars are expected, nested objects, huge numbers and malformed UTF-8 handling where applicable.

---

# 19. File Security Suite

Where file handling exists, test:

```text
path traversal
absolute paths
../ traversal
encoded traversal
double encoding
null bytes
symlink-like paths
malicious archives
zip bombs
polyglot files
wrong extension
wrong content type
oversized file
excessive image dimensions
```

Assertions include:

```text
no arbitrary filesystem write
no path escape
bounded processing
no execution of uploaded content
safe object key
correct tenant ownership
```

---

# 20. SSRF Security Suite

If Sitolo fetches externally supplied URLs or remote resources, test:

```text
127.0.0.1
localhost
::1
RFC1918 addresses
link-local addresses
cloud metadata addresses
internal DNS names
redirect to internal host
DNS rebinding simulation
alternate IP representations
IPv6-mapped addresses
userinfo URL tricks
encoded hostnames
```

The test harness should provide a controlled callback service to prove whether an outbound request escaped the allowed destination policy.

Assert:

```text
blocked internal destination
blocked redirect
bounded DNS resolution
no credential forwarding
```

---

# 21. Browser Security Suite

Where browser/admin interfaces exist, test:

## 21.1 XSS

Payloads in:

```text
product name
description
customer fields
supplier fields
notes
report labels
search results
error rendering
```

Verify contextual output encoding.

## 21.2 CSRF

For cookie-authenticated browser operations, attempt cross-origin state-changing requests.

Verify required anti-CSRF controls.

## 21.3 CORS

Test:

```text
approved origin
unapproved origin
null origin
origin with credentials
preflight mismatch
```

## 21.4 Source maps/debug assets

Production build checks must verify sensitive source/debug artifacts are absent or properly restricted.

---

# 22. Payment Security Suite

Payment testing must treat the provider as an external trust boundary.

## 22.1 Client payment manipulation

Attempt:

```text
amount changed in request
currency changed
sale_id changed
provider changed
success flag forged
provider transaction ID forged
```

Expected:

```text
client data not treated as authoritative
```

## 22.2 Webhook signature

Test:

```text
valid signature → process
invalid signature → reject
missing signature → reject where required
modified body → reject
modified signature → reject
wrong secret → reject
```

## 22.3 Webhook replay

Submit the same valid event repeatedly.

Expected:

```text
one business effect
subsequent delivery = idempotent
```

## 22.4 Webhook ordering

Deliver:

```text
success
pending
success
```

or provider-supported equivalent transitions out of order.

The internal state machine must not regress to an invalid earlier state.

## 22.5 Unknown outcome

Simulate:

```text
provider receives request
provider commits
connection breaks before response
```

Expected:

```text
payment = UNKNOWN/PENDING
no blind duplicate initiation
reconciliation/query path invoked
```

## 22.6 Amount/currency/account matching

A validly signed provider event with mismatched:

```text
amount
currency
merchant account
payment intent
```

must be quarantined or rejected rather than blindly applied.

---

# 23. MRA EIS Security Suite

MRA EIS must be tested as a distinct compliance integration boundary.

## 23.1 Credential isolation

Verify EIS credentials do not appear in:

```text
mobile bundles
desktop bundles
browser responses
source
git history
logs
metrics
traces
error messages
```

## 23.2 Terminal identity

Attempt to submit using:

```text
unknown terminal
wrong tenant terminal
inactive terminal
revoked/blocked terminal
```

and verify controlled rejection.

## 23.3 Configuration version

Test:

```text
configuration refresh
configuration change
submission under old snapshot
new submission under new snapshot
```

Historical records must remain interpretable.

## 23.4 Offline EIS security

Test:

```text
expired offline window
excess cumulative amount
modified offline transaction
modified signing data
wrong terminal signing material
replayed offline submission
```

No test may ever use real production merchant credentials or generate unauthorized real tax transactions.

---

# 24. Database Security Suite

## 24.1 Runtime role privilege

Inspect actual PostgreSQL grants.

Assert runtime role cannot perform unnecessary:

```text
CREATE/DROP schema
ALTER role
CREATE role
unapproved DDL
superuser-equivalent operations
RLS bypass
```

## 24.2 RLS positive tests

```text
Tenant A → Tenant A row → ALLOW
Branch A1 → Branch A1 row → ALLOW where scoped
```

## 24.3 RLS negative tests

```text
Tenant A → Tenant B row → DENY
Branch A1 → Branch A2 row → DENY
```

## 24.4 Direct-database negative tests

Where feasible, execute queries under the actual runtime role rather than an all-powerful test role.

## 24.5 Constraint abuse

Attempt:

```text
duplicate unique key
dangling foreign key
negative amount
invalid state
invalid tenant relationship
invalid idempotency key reuse
```

Expected failure must leave the transaction in a safe state.

---

# 25. Transaction Atomicity Suite

Critical sale workflow:

```text
sale
+ inventory
+ payment linkage
+ audit
+ outbox
+ idempotency
```

must be atomic according to the domain contract.

Inject a failure after each conceptual write:

```text
after sale insert
after inventory post
after payment linkage
after audit insert
before outbox
before commit
```

Expected:

```text
no impossible partial state
```

Do not accept a test harness that merely rolls back the whole test transaction around the application. That can conceal production transaction behavior.

At least some tests must exercise actual application transaction boundaries end-to-end.

---

# 26. Concurrency and Race Security Suite

The testing strategy identifies concurrency as one of Sitolo's highest-value security/integrity areas. Required scenarios include final-stock races, refund races, webhook/polling races, worker retries, sync/manual mutation and register close races. fileciteturn20file4L458-L516

## 26.1 Last stock

```text
stock = 1
100 concurrent sale attempts
```

Expected:

```text
successful consumed quantity <= 1
no negative stock
```

## 26.2 Duplicate refund race

Two simultaneous refunds attempt to consume the same remaining entitlement.

Expected:

```text
combined refund <= eligible amount
```

## 26.3 Payment event/poll race

Deliver webhook and provider-status response concurrently.

Expected:

```text
one coherent final state
no double settlement
```

## 26.4 Worker duplicate race

Two workers claim/process the same logical work item.

Expected:

```text
one effect
or
idempotent equivalent result
```

## 26.5 Deadlock

Construct deterministic lock-order conflict.

Assert:

```text
deadlock detected
transaction safely aborted
no partial financial effect
bounded retry only if safe
observable failure after retry exhaustion
```

---

# 27. Resource Exhaustion / DoS Suite

Security and performance overlap when attackers can trigger expensive work.

## 27.1 API

Test:

```text
maximum body
above-maximum body
large query parameter
many filters
expensive sort
large pagination request
```

## 27.2 Sync

Test:

```text
maximum batch
above maximum batch
large dependency graph
deep nesting
many duplicate commands
```

## 27.3 Reports

Test:

```text
maximum permitted range
unbounded range
large export
many simultaneous exports
```

## 27.4 File parser

Test parser CPU/memory/time budgets.

## 27.5 Authentication abuse

Test:

```text
repeated login failures
MFA failures
password reset bursts
session creation bursts
```

Expect throttling without accidentally locking the entire tenant or creating an account-enumeration side channel.

---

# 28. Rate-Limit Correctness Suite

Rate limits must be tested for both enforcement and unintended bypass.

Bypass dimensions:

```text
IP changes
user changes
tenant changes
headers
IPv6/IPv4 changes
parallel connections
alternate endpoints
batch endpoints
```

For tenant quotas, ensure changing resource IDs does not bypass the appropriate logical limit.

---

# 29. Error Leakage Suite

For every important error class, test that external responses do not expose:

```text
SQL
stack traces
filesystem paths
provider secrets
EIS credentials
internal hostnames
private URLs
authorization policy internals
session data
```

Also verify that internal logs still retain enough safe information to investigate the error.

A generic `500` with no traceability is not an adequate security design; it must be safe externally and diagnosable internally.

---

# 30. Logging and Telemetry Redaction Suite

Generate errors containing synthetic secrets:

```text
ACCESS_TOKEN_TEST
REFRESH_TOKEN_TEST
WEBHOOK_SECRET_TEST
PAYMENT_API_KEY_TEST
EIS_SECRET_TEST
PRIVATE_KEY_TEST
```

Run the affected workflows and scan:

```text
application logs
worker logs
request traces
span attributes
metric exemplars if used
crash artifacts
```

Expected:

```text
secret values absent
```

Telemetry must also be checked for accidental high-cardinality identifiers.

---

# 31. Secret Exposure Suite

## 31.1 Repository scan

Scan:

```text
working tree
git diff
git history
branches where available
configuration templates
fixtures
examples
scripts
```

## 31.2 Artifact scan

Scan:

```text
container layers
mobile APK/AAB
desktop binaries/bundles
web build
source maps
SBOM
release archives
```

## 31.3 Runtime scan

Verify secrets do not appear in:

```text
health endpoints
diagnostics
metrics
logs
API errors
support exports
```

The security architecture explicitly treats secret exposure as release-blocking. fileciteturn20file1L152-L169

---

# 32. CI/CD Security Test Harness

CI itself is a privileged execution environment and therefore must be security-tested.

## 32.1 Workflow injection

Test workflows for untrusted input flowing into:

```text
shell
script
command construction
artifact names
image tags
cloud CLI arguments
```

## 32.2 Pull-request trust separation

Verify untrusted contribution contexts cannot access:

```text
production secrets
deployment credentials
signing keys
provider credentials
```

## 32.3 Workflow permission tests

Inspect workflow permissions for least privilege.

## 32.4 Action/reference pinning

Verify approved immutable references or the repository's equivalent policy.

## 32.5 Scanner-failure test

Deliberately cause a required security scanner to fail.

Expected:

```text
pipeline = FAIL
release = blocked
```

A missing report is not a pass.

---

# 33. Dependency and Supply-Chain Suite

Verify:

```text
Cargo.lock present
frontend lockfile present
approved Rust toolchain
advisory scan
license policy
SBOM generated
provenance generated
artifact signature generated
artifact verification succeeds
```

Negative tests should include intentionally vulnerable/forbidden fixtures in isolated test repositories or manifests where policy tooling can safely demonstrate rejection.

Do not introduce a known-malicious package into production dependency resolution.

---

# 34. Artifact Integrity Suite

Build an artifact and record:

```text
source commit
compiler/toolchain
dependency state
container digest
SBOM
provenance
signature
```

Then verify the artifact from a fresh environment.

Negative cases:

```text
modified artifact
wrong digest
invalid signature
mismatched provenance
unexpected builder identity
```

Expected deployment decision:

```text
REJECT
```

---

# 35. API Surface Inventory Test

Security depends on knowing what exists.

The harness should compare:

```text
documented routes
registered routes
OpenAPI/API contract
runtime exposed routes
```

Flag:

```text
undocumented endpoint
unexpected method
unexpected debug endpoint
admin endpoint exposed to normal users
route missing classification
```

This operationalizes the security requirement to inventory all production endpoints. fileciteturn20file2L299-L310

---

# 36. Security Regression Matrix — 48 Controls

The repository's 48-control baseline becomes a machine-readable matrix.

| # | Control | Harness area | Blocking? |
|---|---|---|---|
| 1 | Exposed DB credentials | secret/runtime scan | Yes |
| 2 | Public `.env` | artifact/web probe | Yes |
| 3 | Hardcoded secrets | repository scan | Yes |
| 4 | Weak authentication | auth suite | Yes |
| 5 | Missing authorization | authz matrix | Yes |
| 6 | Cross-user access | BOLA suite | Yes |
| 7 | Open DB permissions | DB privilege suite | Yes |
| 8 | Cloud misconfiguration | IaC/posture | Yes |
| 9 | Unprotected admin route | admin authz | Yes |
| 10 | Exposed debug tools | endpoint probes | Yes |
| 11 | Secret-leaking logs | redaction suite | Yes |
| 12 | Verbose errors | error leakage | Yes |
| 13 | Secrets in git | repository/history scan | Yes |
| 14 | Secrets in code | static/secret scan | Yes |
| 15 | Client-only security | direct API attacks | Yes |
| 16 | Input validation | negative/fuzz | Yes |
| 17 | SQL injection | injection suite | Yes |
| 18 | NoSQL injection | relevant adapter tests | Policy-dependent |
| 19 | XSS | browser suite | Yes where web exists |
| 20 | CSRF | browser/session suite | Yes where cookie auth exists |
| 21 | Unsafe uploads | file suite | Yes where uploads exist |
| 22 | Path traversal | filesystem suite | Yes where paths exist |
| 23 | SSRF | outbound URL suite | Yes where remote fetch exists |
| 24 | Broken password reset | recovery suite | Yes |
| 25 | Weak sessions | session suite | Yes |
| 26 | JWT secrets | key management suite | Yes |
| 27 | Permissive CORS | browser/edge suite | Yes where browser API exists |
| 28 | Missing rate limits | abuse suite | Yes for classified endpoints |
| 29 | Exposed environments | deployment probe | Yes |
| 30 | Default credentials | deployment tests | Yes |
| 31 | Unsigned webhooks | provider suite | Yes where signature contract exists |
| 32 | Frontend payment checks | direct API suite | Yes |
| 33 | IDOR/BOLA | object-auth suite | Yes |
| 34 | APIs + user input | validation/fuzz | Yes |
| 35 | Exposed logs | telemetry deployment test | Yes |
| 36 | Source maps | artifact/web scan | Policy-dependent |
| 37 | No MFA | privileged auth suite | Yes for required privileged roles |
| 38 | Account enumeration | auth/recovery suite | Yes |
| 39 | Business logic abuse | domain abuse suite | Yes |
| 40 | Race conditions | concurrency suite | Yes |
| 41 | Webhook replay | replay suite | Yes |
| 42 | Insecure CI identity | workflow suite | Yes |
| 43 | Untrusted build actions | CI trust suite | Yes |
| 44 | Unpinned build dependencies | supply chain | Yes |
| 45 | Fail-open controls | failure injection | Yes |
| 46 | Missing timeouts | resource/dependency suite | Yes |
| 47 | Sensitive browser storage | client/browser suite | Yes where browser exists |
| 48 | Insecure endpoints | surface inventory + DAST | Yes |

The baseline already specifies this 48-control mapping as a release requirement. fileciteturn20file7L936-L973

---

# 37. Mutation Security Testing

Mutation testing is mandatory for the highest-value controls.

Create deliberate test-only mutations such as:

```text
remove tenant predicate
remove branch predicate
skip permission check
accept revoked device
remove idempotency uniqueness check
skip amount verification
skip currency verification
skip webhook signature verification
advance checkpoint early
remove stock availability check
remove rate limit
ignore timeout
```

The expected behavior is:

```text
mutation introduced
↓
security suite detects it
↓
CI fails
```

A surviving mutation is itself a security finding because it demonstrates inadequate test sensitivity.

The earlier testing contract explicitly defines mutation survival this way. fileciteturn20file4L436-L454

---

# 38. Property-Based Security Testing

Property tests should target invariants rather than individual examples.

## 38.1 Tenant property

For any two distinct tenants A and B:

```text
principal(A) cannot observe or mutate authoritative resources(B)
```

subject to explicitly documented platform-admin exceptions.

## 38.2 Financial correction property

For any finalized sale:

```text
historical_sale remains unchanged
```

after any sequence of valid corrections.

## 38.3 Refund property

For any valid correction sequence:

```text
sum(refunds) <= eligible refundable amount
```

## 38.4 Inventory property

For any accepted sequence:

```text
balance = reconstruct(ledger)
```

within the defined projection model.

## 38.5 Idempotency property

For any command `C`:

```text
apply(C)
then apply(C)
```

must produce one logical effect.

## 38.6 Checkpoint property

A checkpoint must never represent a state beyond the highest durably accepted server result.

---

# 39. Fuzzing Harness

Fuzz the following targets:

```text
HTTP request parser
JSON command payloads
sync envelopes
schema versions
cursor/checkpoint parsers
canonicalization
webhook payloads
file metadata
archive parsers
query filters
provider response parsers
MRA response mappings
error translation
```

Fuzzing must be resource-bounded:

```text
CPU limit
memory limit
input size limit
execution timeout
corpus size limit
```

A parser crash is a test failure even if the process is restarted automatically.

---

# 40. Differential / Contract Testing

When an adapter maps between canonical and provider formats, test both directions.

Example:

```text
canonical request
→ provider request

provider response
→ canonical response

provider error
→ canonical error
```

Contract fixtures must include:

```text
known success
known failure
unknown fields
missing fields
null fields
boundary values
provider drift
```

Where legal and contractual constraints permit, recorded sandbox responses may become regression fixtures.

---

# 41. Security Chaos / Failure Injection

Security behavior often fails only when a dependency becomes unhealthy.

Inject:

```text
PostgreSQL unavailable
PostgreSQL slow
transaction deadlock
Redis unavailable if used
object storage unavailable
payment provider timeout
payment provider malformed response
MRA unavailable
MRA malformed response
telemetry collector unavailable
secret provider unavailable
worker crash
network partition
```

Expected security posture:

```text
fail closed for security-critical uncertainty
fail safely for non-critical telemetry
preserve durable state
avoid duplicate financial effects
surface recoverable state
```

The security architecture explicitly states that authorization-service failure should deny sensitive mutation rather than “always allow.” fileciteturn20file4L1327-L1345

---

# 42. Backup / Restore Security Tests

Backups are security-sensitive because they contain the authoritative database.

Test:

```text
backup access identity
backup encryption configuration
restore authorization
restore isolation
restored RLS policies
restored permissions
restored audit data
restored secrets references
```

A restore test must verify that restoring an older state does not accidentally recreate revoked authority without an explicit recovery procedure.

---

# 43. Audit Integrity Tests

Verify that ordinary tenant users cannot:

```text
edit audit records
delete audit records
change actor
change timestamp
change target
forge privileged audit evidence
```

Test that sensitive actions produce the expected evidence:

```text
role change
refund approval
device revoke
support access
payment reconciliation
EIS exception
export
break-glass
```

Audit evidence and telemetry must remain distinct.

---

# 44. Support / Privileged Access Security Suite

Support is a high-risk trust boundary.

Test:

```text
normal support → approved read → allow
normal support → unapproved tenant → deny
support read → write mutation → deny
expired JIT grant → deny
wrong purpose/scope → deny
break-glass without required MFA → deny
```

Verify every privileged action is attributable and auditable.

---

# 45. Production Readiness Security Exercise

Before production, execute one integrated adversarial scenario:

```text
attacker compromises cashier credentials
        ↓
tries Tenant B object IDs
        ↓
tries refund endpoint directly
        ↓
replays an old payment event
        ↓
tries revoked device sync
        ↓
tries large export
        ↓
tries support endpoint
```

Expected:

```text
authentication/session controls
↓
authorization controls
↓
tenant isolation
↓
idempotency
↓
device revocation
↓
rate/resource controls
↓
audit/detection
```

all contribute to containment.

---

# 46. CI Gate Model

Every required security control has one of these states:

```text
PASS
FAIL
NOT_RUN
WAIVED
```

`NOT_RUN` is not equivalent to `PASS`.

## 46.1 Automatic blocking

Block release on:

```text
Critical security failure
cross-tenant failure
financial integrity failure
successful BOLA
replay creating duplicate financial effect
revoked-device acceptance
secret exposure
artifact verification failure
scanner failure for mandatory control
runtime DB privilege escalation
unsafe CI secret access
```

## 46.2 Waivers

A waiver must contain:

```text
control
finding
affected asset
risk rationale
compensating control
owner
expiry
rollback plan
approval
```

Permanent security waivers are prohibited.

---

# 47. Evidence Package

Every release candidate should produce a machine-readable security evidence package.

Recommended structure:

```text
security-evidence/
├── manifest.json
├── test-results/
├── tenant-isolation/
├── authz/
├── authn/
├── database/
├── payments/
├── eis/
├── sync/
├── concurrency/
├── fuzz/
├── mutation/
├── secrets/
├── dependencies/
├── sbom/
├── provenance/
├── artifact-verification/
├── dast/
├── iaс/
└── summary.md
```

The manifest should identify:

```text
source commit
release version
toolchain
environment
database version
fixture version
test suite version
artifact digest
execution timestamp
```

This makes the security decision reproducible.

---

# 48. Security Evidence Retention

Evidence retention must be separated by sensitivity and purpose.

Retain enough information to answer:

```text
what was tested?
which code?
which environment?
which fixtures?
which controls?
what failed?
what passed?
who approved release?
```

Do not retain secrets merely because a test generated them.

Synthetic test credentials should be disposable and non-production.

---

# 49. Failure Investigation Workflow

When a security test fails:

```text
FAIL
 ↓
classify
 ↓
reproduce
 ↓
preserve evidence
 ↓
determine security impact
 ↓
contain if required
 ↓
fix
 ↓
add regression test
 ↓
rerun affected suite
 ↓
rerun dependent suites
 ↓
review
```

A “flaky security test” must not simply be disabled.

Determine whether the flakiness indicates:

```text
race condition
resource exhaustion
environment instability
unbounded timeout
nondeterministic authorization
```

Those may themselves be defects.

---

# 50. Test Isolation Rules

Tests must not accidentally influence one another through:

```text
shared tenant state
shared DB rows
shared device credentials
shared idempotency keys
shared object-store artifacts
shared queue jobs
shared ports
```

Each integration test should preferably use a unique test namespace or reset mechanism.

Security tests must not hide state by wrapping every application operation in a test-only transaction.

---

# 51. Determinism Requirements

A security test must be deterministic wherever the security property is deterministic.

Avoid:

```text
sleep(5)
hope worker processed
random tenant IDs without seed
network conditions without recording state
```

Use:

```text
explicit barriers
controlled clocks where safe
fake provider responses
real DB synchronization
known fixtures
bounded retries
```

Concurrency tests may naturally explore schedules, but must retain enough seed/context to reproduce a failure.

---

# 52. Test Data Safety

Security tests may contain sensitive-looking data, but it must be synthetic.

Rules:

```text
no real customer PII
no production payment details
no production credentials
no production MRA terminal secrets
no real private keys
no live webhook secrets
```

The harness itself is part of the attack surface and must be scanned for accidental secrets.

---

# 53. Security Test Naming Convention

Use names that communicate the threatened invariant.

Good:

```text
rejects_cross_tenant_sale_read
rejects_refund_for_other_branch
rejects_reused_command_id_with_changed_payload
rejects_revoked_device_sync
prevents_duplicate_refund_under_concurrency
rejects_payment_webhook_with_invalid_signature
blocks_scanner_failure_from_release
```

Avoid generic names such as:

```text
auth_test_1
security_test
edge_case
```

A security test name should help reviewers understand the protected property without opening the implementation.

---

# 54. Test Tags / Taxonomy

Every security test should carry a classification where the test framework supports it:

```text
SEC-AUTHN
SEC-AUTHZ
SEC-TENANT
SEC-BOLA
SEC-DEVICE
SEC-DATA
SEC-API
SEC-INJECT
SEC-FILE
SEC-SSRF
SEC-PAYMENT
SEC-EIS
SEC-SYNC
SEC-CONCURRENCY
SEC-AVAILABILITY
SEC-SECRETS
SEC-CI
SEC-SUPPLYCHAIN
SEC-AUDIT
SEC-DR
```

This allows CI to run targeted suites for affected changes.

---

# 55. Change-Based Security Test Selection

A pull request touching:

```text
auth/
```

must automatically run at least:

```text
authn
session
MFA/recovery
authorization
tenant
BOLA
```

A change touching:

```text
sync/
```

must run:

```text
sync
idempotency
device
replay
checkpoint
tenant
concurrency
resource limits
```

A change touching:

```text
payment/
```

must run:

```text
payment
webhook
replay
idempotency
concurrency
reconciliation
```

A change touching:

```text
tax/EIS
```

must run:

```text
EIS
credential isolation
submission/replay
offline
configuration
error mapping
```

Full regression remains mandatory for release candidates.

---

# 56. Security Test Coverage Requirements

Coverage must be measured at multiple dimensions.

## 56.1 Route coverage

Every production endpoint is either:

```text
covered
explicitly exempted with reason
```

## 56.2 Permission coverage

Every sensitive permission has:

```text
positive test
negative test
scope test
state test
```

## 56.3 Tenant coverage

Every tenant-owned resource has cross-tenant negative tests.

## 56.4 State-transition coverage

Every high-risk state machine transition is tested for both allowed and denied contexts.

## 56.5 Threat coverage

Every Critical/High threat maps to executable tests.

## 56.6 Control coverage

All 48 security controls map to an automated or explicitly manual evidence-producing check.

---

# 57. False-Positive / False-Negative Governance

Security testing can fail in two directions.

## False positive

A valid implementation is reported as insecure.

The remedy is not automatically to suppress the finding.

Document:

```text
why it is safe
which control proves it
which alternative attack was considered
```

## False negative

The test reports success while a vulnerability exists.

These are more dangerous and should trigger:

```text
threat-model review
oracle review
additional attack scenario
mutation test
manual verification where needed
```

---

# 58. Harness Self-Security

The test harness itself can become a vulnerability source.

Protect:

```text
test secrets
cloud credentials
signing keys
provider mocks
fixtures
artifact credentials
CI tokens
```

Do not let tests print secrets because “they are only test secrets.”

CI logs are persistent external artifacts.

Scripts must not execute arbitrary PR-controlled shell input with privileged credentials.

---

# 59. Local Developer Workflow

The local workflow should offer progressive security verification.

## Fast loop

```text
format
compile
unit tests
targeted security tests
```

## Standard loop

```text
integration tests
security suite
real PostgreSQL tests
```

## Pre-PR

```text
full targeted suite
secret scan
dependency scan
```

## Release candidate

```text
full regression
property
fuzz smoke
mutation sample
DAST
IaC
SBOM
provenance
artifact verification
restore/security checks
```

The developer workflow must make the secure path easy without weakening release gates.

---

# 60. Initial CI Pipeline

Recommended sequence:

```text
1. checkout trusted revision
2. verify toolchain
3. verify lockfiles
4. format/lint
5. compile
6. unit tests
7. security unit tests
8. dependency/advisory checks
9. secret scan
10. start disposable PostgreSQL
11. migrations
12. DB security tests
13. integration tests
14. auth/authz/tenant suite
15. payment/EIS/sync contract tests
16. concurrency suite
17. resource-limit tests
18. build artifact
19. SBOM
20. provenance
21. artifact signature
22. artifact verification
23. deploy disposable staging
24. DAST/security probes
25. collect evidence
26. enforce gate
```

Exact tools may vary; the control sequence must remain.

---

# 61. Pull Request Gate Matrix

| Change | Minimum blocking security tests |
|---|---|
| Auth | AuthN + session + MFA + recovery |
| IAM | AuthZ + tenant + BOLA + SoD |
| DB | privileges + RLS + migration + transaction |
| Sales | domain abuse + idempotency + concurrency |
| Inventory | ledger + last-unit race + scope |
| Payments | signature + replay + unknown-outcome + reconciliation |
| Sync | device + replay + checkpoint + conflict + resource limits |
| EIS | credential + terminal + submission + offline |
| File handling | traversal + parser + upload + SSRF if relevant |
| Reporting | scope + export + resource limits |
| Admin/support | MFA + JIT + scope + audit |
| CI | workflow trust + secret isolation + scanner gate |
| Deployment | artifact + secrets + environment exposure |

---

# 62. Security Release Gate

The release is blocked if any of the following is true:

```text
cross-tenant test fails
BOLA test succeeds unexpectedly
privileged MFA bypass exists
revoked device accepted sensitive command
financial replay creates duplicate effect
inventory race violates invariant
payment verification can be bypassed
EIS credential exposed
production secret detected
runtime DB privilege exceeds policy
required scanner did not execute
artifact signature verification fails
provenance cannot be established
critical DAST finding remains
restore/security prerequisite fails
```

A non-critical finding may require risk acceptance according to governance, but the exception must be explicit.

---

# 63. Security Test Harness Definition of Done

The initial harness is complete only when:

```text
[ ] test fixtures are reproducible
[ ] Tenant A/B fixtures exist
[ ] branch/resource fixtures exist
[ ] active/revoked devices exist
[ ] active/expired/revoked sessions exist
[ ] auth suite exists
[ ] authz suite exists
[ ] cross-tenant suite exists
[ ] BOLA suite exists
[ ] branch scope suite exists
[ ] DB privilege suite exists
[ ] RLS suite exists
[ ] transaction atomicity tests exist
[ ] payment replay tests exist
[ ] payment signature tests exist
[ ] EIS security tests exist
[ ] offline replay tests exist
[ ] checkpoint tests exist
[ ] concurrency tests exist
[ ] resource exhaustion tests exist
[ ] secret scans exist
[ ] log redaction tests exist
[ ] CI trust tests exist
[ ] artifact verification exists
[ ] SBOM/provenance checks exist
[ ] mutation targets exist
[ ] fuzz targets exist
[ ] evidence manifest exists
[ ] CI gate exists
[ ] waiver workflow exists
[ ] failure investigation workflow exists
```

---

# 64. Security Test Governance

Test code is production-adjacent infrastructure.

Changes to the harness itself require review because weakening a test can be equivalent to weakening a control.

Examples requiring additional review:

```text
deleting a negative test
loosening an assertion
removing a tenant fixture
changing a scanner to non-blocking
ignoring a failed test
changing fixture privileges
adding CI secret access
changing artifact verification
```

A test deletion should explain:

```text
why obsolete
what replaced it
which invariant remains protected
```

---

# 65. Incident Integration

When a production security incident occurs, the resulting regression test must enter this harness when technically possible.

Example:

```text
tenant leak
 ↓
root cause = repository query missing tenant predicate
 ↓
fix query
 ↓
add cross-tenant test
 ↓
add mutation that removes predicate
 ↓
confirm suite catches mutation
```

This creates a durable learning loop rather than relying on memory.

The existing security implementation contract explicitly requires:

```text
incident
→ fix
→ regression test
→ improve control
```

and defines that as the intended security engineering lifecycle. fileciteturn20file3L320-L332

---

# 66. Threat-to-Test Traceability Template

Every Critical/High threat should have a record equivalent to:

```yaml
threat_id: THR-TENANT-001
asset: tenant_business_data
boundary: api_to_application
attack: modify tenant_id
precondition: authenticated tenant A
expected: denied
side_effects: none
primary_control: tenant scope authorization
secondary_control: repository scope
tertiary_control: database RLS
security_test:
  - tests/security/tenant/rejects_cross_tenant_sale_read
  - tests/security/tenant/rejects_cross_tenant_sale_write
mutation_test:
  - remove_tenant_predicate
release_gate: blocking
owner: platform/domain-security
```

This should eventually become machine-readable and automatically checked for completeness.

---

# 67. Recommended Initial Implementation Order

Implement the harness in this sequence:

```text
1. fixture factory
2. test database bootstrap
3. security test context
4. tenant isolation helpers
5. authn test helpers
6. authz matrix helpers
7. HTTP attack client
8. DB privilege/RLS helpers
9. transaction/failure injection helpers
10. idempotency/replay helpers
11. provider mock/fixture layer
12. sync command attack helpers
13. concurrency barriers
14. resource-limit generators
15. secret/redaction scanner integration
16. CI gate formatter
17. evidence manifest generator
18. mutation configuration
19. fuzz harness
20. staging/DAST integration
```

This ordering builds reusable primitives before hundreds of feature-specific tests.

---

# 68. Initial Attack Utility Requirements

The harness should provide reusable attack functions such as:

```text
with_tenant_a()
as_cashier()
as_manager()
use_revoked_device()
replace_tenant_id()
replace_branch_id()
replace_resource_id()
replay_command()
modify_command()
replay_webhook()
modify_webhook_body()
corrupt_signature()
exceed_rate_limit()
exceed_body_limit()
force_timeout()
force_deadlock()
kill_worker_after_external_effect()
run_concurrently()
assert_no_financial_side_effect()
assert_no_cross_tenant_leak()
assert_audit_evidence()
```

These utilities reduce the probability that individual developers omit an important assertion when adding tests.

---

# 69. Security Review Checklist for New Tests

Every new security-sensitive test should be reviewed for:

```text
[ ] What threat does this test prove against?
[ ] What invariant is protected?
[ ] Is the attacker model realistic?
[ ] Does the test call the real security boundary?
[ ] Does it assert side effects, not just HTTP status?
[ ] Does it verify tenant/resource scope?
[ ] Does it verify audit behavior where relevant?
[ ] Does it avoid privileged test shortcuts?
[ ] Can the test be bypassed by changing the client rather than server behavior?
[ ] Is the test deterministic?
[ ] Is it resource-bounded?
[ ] Is failure evidence sufficient to debug?
```

---

# 70. Final Harness Architecture Contract

The security-test harness must enforce the same philosophy as the rest of Sitolo:

```text
UNTRUSTED INPUT
      ↓
AUTHENTICATION
      ↓
SECURITY CONTEXT
      ↓
TENANT / RESOURCE SCOPE
      ↓
AUTHORIZATION
      ↓
STATE / INVARIANT
      ↓
TRANSACTION / CONCURRENCY
      ↓
IDEMPOTENCY
      ↓
AUTHORITATIVE RESULT
      ↓
AUDIT / OUTBOX
      ↓
EXTERNAL EFFECT
      ↓
RECONCILIATION
      ↓
OBSERVABILITY
      ↓
RECOVERY
      ↓
REGRESSION TEST
```

A good security harness does not merely answer:

```text
"Did the endpoint return 403?"
```

It answers:

```text
"Could an attacker cross the boundary?
Did the server enforce the boundary?
Did the database enforce the boundary?
Did any unauthorized side effect occur?
Was the attempt observable?
Can we reproduce the result?
Would CI stop a regression?
Would the same class of bug be caught next time?"
```

That is the required standard for Sitolo.

---

# Appendix A — Minimum Security Test Command Contract

Exact commands depend on repository tooling, but the pipeline must provide equivalents for:

```text
format check
lint/static analysis
unit tests
integration tests
security tests
property tests
fuzz smoke tests
mutation tests
SQLx/query verification
secret scan
dependency scan
license/policy scan
SBOM generation
container scan
IaC scan
API contract/security tests
DAST
artifact signing
provenance generation
artifact verification
```

The security architecture already defines this command family as the minimum release posture. fileciteturn15file6L894-L917

---

# Appendix B — Mandatory Negative Security Fixture Matrix

```text
                TENANT A      TENANT B
OWNER A             ✓             ✗
MANAGER A           ✓             ✗
CASHIER A           ✓             ✗

BRANCH A1           ✓             ✗
BRANCH A2           scope         ✗
BRANCH B1           ✗             ✓

DEVICE A            ✓             ✗
REVOKED DEVICE      ✗             ✗

SALE A              ✓             ✗
SALE B              ✗             ✓

PAYMENT A           ✓             ✗
PAYMENT B           ✗             ✓
```

The exact `✓/✗` matrix must be populated according to effective role/resource scope rather than assuming every tenant user can access everything within that tenant.

---

# Appendix C — Security Gate Status Contract

```json
{
  "control_id": "SEC-TENANT-001",
  "status": "PASS",
  "commit": "<source-sha>",
  "environment": "security-ci",
  "suite": "tenant-isolation",
  "test_count": 42,
  "failed": 0,
  "timestamp": "<timestamp>",
  "evidence": "<artifact-reference>",
  "blocking": true
}
```

The actual implementation may use another serialization format, but the evidence model must remain machine-readable.

---

# Appendix D — Release Decision

A release candidate may proceed only when:

```text
security harness executed
AND mandatory controls PASS
AND no control is NOT_RUN
AND required evidence exists
AND exceptions are explicit
AND artifact identity matches tested source
```

The critical architectural rule is:

> **Testing the wrong artifact is equivalent to not testing the release.**

The security test harness must therefore bind its evidence to the exact source revision and release artifact that will be deployed.

---

# Appendix E — Relationship to Previous Sitolo Contracts

This document is intentionally subordinate to and consistent with the previous documents:

- `security_implementation_spec.md` defines implementation security requirements.
- `domain_model.md` defines business invariants that tests must prove.
- `database_design.md` defines authoritative persistence and RLS/constraint behavior.
- `api_contract.md` defines public request/response behavior and API security requirements.
- `auth_authorization_spec.md` defines identity/session/IAM behavior.
- `sync_protocol.md` defines offline command integrity and checkpoint semantics.
- `payment_integration_spec.md` defines provider trust, webhook and reconciliation semantics.
- `mra_eis_integration_spec.md` defines EIS integration and evidence requirements.
- `testing_strategy.md` defines the broader testing program.
- `threat_model.md` identifies attacker goals and high-risk threats.
- `observability_spec.md` defines telemetry evidence.
- `deployment_spec.md` defines release/runtime boundaries.
- `implementation_plan.md` places this harness in Phase 7 and makes security evidence a release prerequisite.
- `ADR-001-025.md` makes security verification and explicit failure/recovery evidence binding architecture decisions.

Where those contracts define a security invariant, this harness exists to provide executable evidence for it.

---

# Appendix F — Non-Negotiable Rules

```text
1. Do not use production credentials in tests.
2. Do not weaken authorization merely to simplify fixtures.
3. Do not use an all-powerful DB role to prove runtime security.
4. Do not treat HTTP status alone as proof of security.
5. Do not ignore side effects in denied-operation tests.
6. Do not silently skip failed scanners.
7. Do not suppress flaky security tests without investigation.
8. Do not delete a security test without replacing its invariant coverage.
9. Do not make cross-tenant tests optional.
10. Do not skip concurrency tests for financial/inventory changes.
11. Do not treat client validation as security evidence.
12. Do not treat provider callbacks as trusted without verification.
13. Do not treat offline data as authoritative merely because it originated from a registered device.
14. Do not allow test harness utilities to become privileged production utilities.
15. Do not release an artifact that differs from the artifact tested.
16. Do not accept NOT_RUN as PASS.
17. Do not store test secrets in persistent logs unnecessarily.
18. Do not disable a blocking security gate to accelerate a release.
19. Every Critical/High threat needs executable evidence.
20. Every released security control needs evidence tied to the release artifact.
```

---

**Document end — Sitolo Initial Security-Test Harness, Phase 0 / File 15 of 16.**
