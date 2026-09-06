# SITOLO — Offline Synchronization Protocol

**Document:** `sync_protocol.md`  
**Phase:** 0 — Architecture / Contracts / ADR Freeze  
**Sequence:** File 06 of 16  
**Status:** Implementation-governing distributed-synchronization specification  
**Product:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first; controlled African regional expansion  
**Backend authority:** Rust + Axum + Tokio  
**Authoritative server persistence:** PostgreSQL  
**Offline operational persistence:** SQLite  
**Primary mobile client:** Flutter / Android-first  
**Desktop:** Tauri  
**Web/admin:** TypeScript where materially useful  
**Architecture:** Modular monolith first; workers and adapters; selective service extraction only when justified

> Sitolo's offline system is not “CRUD plus a retry queue.” It is a distributed business-state protocol. A merchant may continue selling without connectivity, but money, stock, authorization, tax evidence, audit history and provider state must still converge to a trustworthy authoritative state.

---

# 0. Executive Decision

Sitolo will use a **durable client command journal + authoritative server command processor + explicit acknowledgement/rejection/conflict protocol**.

It will **not** use generic last-write-wins replication for business truth.

```text
USER INTENT
    |
    v
FLUTTER DOMAIN COMMAND
    |
    v
LOCAL SQLITE TRANSACTION
    +------------------------------+
    | local projection             |
    | durable command journal      |
    +------------------------------+
    |
    v
PENDING_SYNC
    |
    | connectivity available
    v
SYNC TRANSPORT
    |
    v
RUST AUTHENTICATION
    |
    v
DEVICE + SESSION + MEMBERSHIP
    |
    v
COMMAND VALIDATION
    |
    v
TENANT / SCOPE AUTHORIZATION
    |
    v
BUSINESS INVARIANT VALIDATION
    |
    v
IDEMPOTENCY / DEDUPLICATION
    |
    v
POSTGRESQL TRANSACTION
    |
    +--> authoritative state
    +--> audit event
    +--> outbox event
    +--> command result
    |
    v
ACK / REJECT / CONFLICT / RETRY
    |
    v
LOCAL RESULT PERSISTED
    |
    v
CHECKPOINT ADVANCEMENT
```

The central contract is:

> **Offline capability provides continuity of operation, not permanent authority.**

The device is allowed to preserve intent locally. The server remains responsible for determining whether that intent is authorized, valid, financially legal, and safe against the authoritative current state.

---

# 1. Relationship to the Existing Sitolo Architecture

The existing Sitolo architecture makes several decisions that this protocol MUST preserve:

- Flutter is the primary mobile client.
- SQLite is the local operational store.
- Rust owns the transactional business decision engine.
- PostgreSQL is authoritative for server-side business truth.
- Sitolo uses a modular monolith first.
- The offline protocol is custom and domain-aware rather than generic last-write-wins replication.
- Transactional outbox is preferred before introducing a separate message broker.
- Financial facts are append-only and corrected with explicit compensating operations.
- Tenant authorization is server-enforced.

The architectural sequence is intentionally:

```text
FOUNDATION
   ↓
identity + tenant + authorization
   ↓
core domain state
   ↓
offline local store
   ↓
custom synchronization
   ↓
external integrations
```

This means synchronization cannot become an independent technical feature that bypasses domain contracts.

---

# 2. Why Generic Replication Is Insufficient

A generic replicated document system commonly assumes that two replicas can merge independently generated changes.

Sitolo contains operations where merge semantics are not generic:

```text
stock = 1

Device A sells 1
Device B sells 1
```

Both commands can be locally valid. They cannot both consume the same final unit under a non-negative-stock policy.

Likewise:

```text
sale = 10,000 MWK
refund A = 7,000 MWK
refund B = 7,000 MWK
```

Both commands may be valid user intents, but the combined financial result is invalid.

The correct model is therefore:

```text
LOCAL INTENT
     ↓
AUTHORITATIVE VALIDATION
     ↓
DOMAIN INVARIANTS
     ↓
COMMIT / REJECT / CONFLICT
```

rather than:

```text
replica A + replica B → merge fields
```

---

# 3. Synchronization Goals

The protocol MUST provide:

1. durable local continuity;
2. stable command identity;
3. safe retries;
4. duplicate resistance;
5. explicit conflict handling;
6. explicit rejection handling;
7. tenant isolation;
8. branch/resource isolation;
9. bounded offline authority;
10. crash recovery;
11. checkpoint safety;
12. schema evolution;
13. device revocation;
14. resource limits;
15. deterministic convergence;
16. financial integrity;
17. inventory integrity;
18. auditability;
19. observability;
20. controlled recovery and resynchronization.

---

# 4. Non-Goals

This protocol does not make the client authoritative for:

- global inventory truth;
- payment settlement;
- tax acceptance;
- organization membership;
- role assignment;
- security policy;
- platform administration;
- unrestricted exports;
- provider credentials;
- legal/compliance certification.

It also does not attempt to solve arbitrary collaborative editing for arbitrary documents. The product domain defines which values can be merged, which commands can continue offline, and which operations require strong online authority.

---

# 5. Consistency Classes

Different Sitolo domains require different consistency models.

| Domain | Model |
|---|---|
| Product display | Eventual |
| Catalogue metadata | Eventual |
| Low-risk reference data | Eventual |
| Sale finalization | Strong transactional server consistency |
| Inventory decrement | Strong server transaction |
| Financial corrections | Strong + append-only |
| Cash events | Durable ordered business facts |
| Provider transaction identity | Strong uniqueness |
| Payment settlement | Provider evidence + reconciliation |
| Tax/EIS status | External/eventual |
| Notifications | Eventual |
| Analytics | Eventual |
| Search | Eventual |
| Device local command queue | Local durable |
| Synchronization acknowledgement | Server authoritative |
| Audit | Durable |
| Outbox processing | At-least-once with idempotent consumers |

There is deliberately no “one consistency policy for the whole platform.”

---

# 6. Core Terminology

## 6.1 Command

A command is an immutable expression of actor intent.

Examples:

```text
CreateSale
FinalizeSale
ReceiveGoods
AdjustStock
CreateStockCount
CloseRegister
CreateReturn
RequestRefund
CreatePurchaseOrder
```

## 6.2 Event

An event records an accepted fact that has occurred.

Examples:

```text
SaleFinalized
InventoryMovementPosted
RefundApproved
RegisterClosed
```

## 6.3 Projection

A projection is derived state used for efficient local or server reads.

## 6.4 Checkpoint

A checkpoint records safe synchronization progress.

## 6.5 Conflict

A conflict exists when the requested operation cannot safely produce its intended business outcome under authoritative state.

## 6.6 Retry

A retry is another delivery attempt when the final server outcome is not yet known or the failure is explicitly transient.

## 6.7 Replay

Replay is redelivery of an already seen command or event.

---

# 7. Trust Model

The following components are distinct security subjects:

```text
Flutter client
Tauri client
Web/admin client
Rust API
Rust workers
PostgreSQL
Object storage
Payment provider
Tax authority
Notification provider
CI runner
Support operator
```

No subject is trusted merely because it runs “inside Sitolo.”

In particular:

> **A compromised mobile client is an untrusted client that can generate syntactically valid requests.**

The server must still validate every privileged operation.

---

# 8. Synchronization Trust Boundaries

```text
BOUNDARY A
User → Client

UI input is untrusted.

BOUNDARY B
Client → Local DB

Local state is compromiseable.

BOUNDARY C
Client → API

Network input is untrusted.

BOUNDARY D
API → Domain

DTOs must be validated and converted to trusted domain types.

BOUNDARY E
Domain → PostgreSQL

Transactions and database constraints protect authority.

BOUNDARY F
PostgreSQL → Outbox/Workers

Messages are durable but may be delivered repeatedly.

BOUNDARY G
Workers → External Providers

Provider responses are untrusted until verified.
```

---

# 9. Local Store Architecture

The Flutter application should maintain a durable SQLite operational database containing only the data required for its authorized operating scope.

Recommended logical areas:

```text
identity/session metadata
active organization
branch scope
device metadata
capability profile
catalogue projection
pricing projection
inventory projection
local drafts
command journal
command results
sync receipts
checkpoints
conflicts
rejections
schema metadata
```

The local database MUST NOT contain:

```text
PostgreSQL credentials
cloud credentials
provider master secrets
MRA master credentials
private deployment keys
platform administrator credentials
server signing private keys
```

---

# 10. Local Data Minimization

A normal cashier device should not contain the complete organization's database merely because it belongs to that organization.

The bootstrap/synchronization scope should be restricted to the actual operational scope:

```text
organization
  ↓
branch
  ↓
locations/registers
  ↓
assigned operational data
```

This reduces:

- privacy exposure;
- storage use;
- synchronization traffic;
- breach blast radius;
- cache complexity.

---

# 11. Local Transaction Contract

When a user performs an operation offline, local persistence should occur as a SQLite transaction.

```text
BEGIN
  validate local command schema
  validate local operational rules
  generate command_id
  generate client sequence
  persist command journal
  update local projection if safe
  persist local metadata
COMMIT
```

Only after the local transaction commits may the UI tell the merchant:

```text
Saved locally
```

This is distinct from:

```text
Server confirmed
```

SQLite provides atomic transactional behavior, including crash recovery semantics; final durability settings must be selected deliberately for Sitolo's actual Android deployment and tested against representative hardware. SQLite's WAL documentation also makes clear that WAL improves reader/writer concurrency but has durability configuration tradeoffs.

---

# 12. Local State vs Server State

There are two fundamentally different states:

```text
LOCAL_COMMITTED
SERVER_COMMITTED
```

The UI must not collapse them into a single success state.

Recommended user-visible statuses:

```text
LOCAL_ONLY
PENDING_SYNC
SERVER_CONFIRMED
REJECTED
CONFLICT
QUARANTINED
```

---

# 13. Command Identity

Every durable command receives a `command_id` before it is persisted.

Properties:

- immutable;
- stable across retries;
- stable across process restarts;
- stable across network transitions;
- unique;
- retained through recovery;
- never regenerated because a request timed out.

A UUIDv7 or ULID-style identifier is appropriate because it provides practical uniqueness and useful ordering characteristics without pretending that identifier ordering is the same thing as business causality.

---

# 14. Idempotency Identity

For commands where the business intent maps one-to-one to a command identity:

```text
idempotency_key = command_id
```

is acceptable.

The server stores enough information to distinguish:

```text
same command
```

from:

```text
same key, different command
```

The latter is a protocol violation.

---

# 15. Command Fingerprint

The server may calculate a canonical semantic fingerprint over important command fields.

Conceptually:

```text
fingerprint = H(
    protocol_version,
    schema_version,
    command_type,
    canonical_payload,
    security-relevant context
)
```

If a command ID is reused with a different fingerprint:

```text
REJECT_IDEMPOTENCY_MISMATCH
```

This prevents command-identity hijacking.

---

# 16. Command Envelope

Illustrative envelope:

```json
{
  "protocol_version": 1,
  "schema_version": 1,
  "command_id": "01J...",
  "device_id": "01J...",
  "client_sequence": 1842,
  "command_type": "FINALIZE_SALE",
  "created_at_client": "2026-09-04T15:00:00Z",
  "organization_hint": "01J...",
  "branch_hint": "01J...",
  "base_cursor": "...",
  "dependencies": [],
  "payload": {},
  "integrity": {}
}
```

Important: any tenant, branch, user, role, approval or security information inside the envelope is a claim to be validated, not authoritative identity.

---

# 17. Actor Identity

The authoritative actor comes from authenticated server context:

```text
access/session
    ↓
principal
    ↓
membership
    ↓
scope
```

The client MUST NOT be allowed to impersonate another actor by changing:

```text
user_id
role
permission
approver_id
```

inside a command payload.

---

# 18. Device Identity

Each installation has a server-registered device identity.

```text
Device
├── device_id
├── organization_id
├── registration state
├── user/session linkage
├── branch scope
├── app version
├── last seen
├── last checkpoint
├── capability version
├── security state
└── revocation state
```

Device ID is not an authentication credential.

---

# 19. Device State Machine

```text
PENDING_REGISTRATION
        ↓
ACTIVE
   ├────┼────┐
   ↓    ↓    ↓
LOST  SUSPENDED  COMPROMISED
   \    |    /
    \   |   /
      REVOKED
        ↓
      RETIRED
```

Not all operational implementations need every intermediate state, but the domain must support at least an explicit active/revoked lifecycle.

---

# 20. Device Registration

A device registration process binds:

```text
authenticated user
+
organization
+
device identity
+
application installation
+
allowed scope
```

together.

Registration MUST NOT automatically import all tenant data or grant administrator privileges.

---

# 21. Device-Key Option for High-Risk Commands

For higher-assurance offline operations, the device may possess a device-bound private key.

The server retains corresponding public-key registration data.

Conceptually:

```text
canonical_command
      ↓
device_private_key
      ↓
signature
      ↓
server verifies public key
```

This should use established cryptographic libraries; Sitolo must not invent cryptographic primitives.

Device signatures are an additional control. They do not replace authorization, replay detection or server-side business validation.

---

# 22. Offline Capability Profiles

A device receives a bounded capability profile.

Example:

```text
Cashier Device
  allowed commands:
    FINALIZE_SALE
    SMALL_STOCK_COUNT
  branch scope:
    BRANCH_A
  value thresholds:
    configured
  expires:
    configured
  policy version:
    12
```

This is safer than a generic:

```text
offline_mode = true
```

---

# 23. Offline Capability Fields

A capability may contain:

```text
capability_id
organization
branch
role/profile
command allowlist
monetary limits
quantity limits
issued_at
expires_at
policy_version
security_version
device_id
```

Capabilities are not a replacement for normal server authorization.

---

# 24. High-Risk Operations Generally Online-Only

Unless later risk analysis proves otherwise:

- changing organization ownership;
- changing MFA policy;
- resetting another user's MFA;
- assigning privileged platform roles;
- changing payout credentials;
- creating deployment/service identities;
- disabling security controls;
- changing tax authority credentials;
- unrestricted bulk exports;
- break-glass operations;
- privileged support impersonation.

---

# 25. Potential Offline Operations

Candidates include:

- ordinary sales;
- low-risk inventory observations;
- configured stock counts;
- permitted cash events;
- operational drafts;
- catalogue browsing;
- low-risk receiving preparation.

The actual command registry determines eligibility.

---

# 26. Offline Capability Is Time-Bounded Authority

A capability can expire without the device being reconnected.

The server SHOULD NOT rely on device clock alone to enforce expiry. Instead:

```text
capability issued
server time / bounded validity
```

is evaluated against server-known state when synchronization resumes.

For security-sensitive offline operations, capability lifetime must be deliberately bounded.

---

# 27. Command Lifecycle

```text
LOCAL_DRAFT
    ↓
LOCAL_COMMITTED
    ↓
PENDING_SYNC
    ↓
SENDING
    ├── RETRY_WAIT → PENDING_SYNC
    ├── ACCEPTED → SERVER_CONFIRMED
    ├── REJECTED → TERMINAL
    ├── CONFLICT → REVIEW
    └── QUARANTINED → SECURITY_REVIEW
```

There must be no silent transition from `REJECTED` or `CONFLICT` to `SERVER_CONFIRMED`.

---

# 28. Sync Session Lifecycle

```text
AUTHENTICATE
     ↓
DEVICE VALIDATION
     ↓
SYNC HANDSHAKE
     ↓
NEGOTIATE VERSION / CAPABILITY
     ↓
UPLOAD BATCH
     ↓
PROCESS COMMANDS
     ↓
RETURN RESULTS
     ↓
PULL SERVER DELTAS
     ↓
APPLY DELTAS LOCALLY
     ↓
ADVANCE CHECKPOINT
     ↓
CLOSE / PAUSE
```

---

# 29. Sync Handshake

Handshake SHOULD communicate:

```text
protocol version
client version
server version
schema compatibility
server time
minimum supported client version
maximum batch count
maximum batch bytes
device status
capability version
security version
server change cursor
```

No handshake response may reveal unauthorized tenant data.

---

# 30. Sync Authentication

Synchronization uses the same trust model as other protected API operations.

The client presents authenticated session credentials appropriate for the selected platform.

The server establishes:

```text
principal
session
organization
membership
scope
device
```

before processing protected commands.

---

# 31. Sync Authorization Pipeline

```text
transport limits
      ↓
schema validation
      ↓
authentication
      ↓
session state
      ↓
device state
      ↓
organization membership
      ↓
offline capability
      ↓
function permission
      ↓
object scope
      ↓
property authorization
      ↓
state transition validation
      ↓
idempotency
      ↓
domain transaction
```

Exact implementation can reorder some safe checks for performance, but must never weaken the trust model.

---

# 32. Idempotency Lookup Security

A duplicate command lookup must still be authorization-safe.

An attacker who guesses a valid command ID must not learn:

```text
which tenant owns it
what financial result it produced
which user submitted it
```

simply by probing the sync endpoint.

The server must establish enough context before returning duplicate results.

---

# 33. Batch Limits

Every sync request must have both:

```text
maximum command count
maximum bytes
```

Example policy values can be established through load testing, but they must be hard limits.

Do not rely on:

```text
“normal clients will only send small requests.”
```

---

# 34. Decompression Safety

If compressed synchronization is supported, the server must enforce logical limits on:

```text
compressed size
expanded size
CPU cost
```

Compression must not become a decompression-bomb attack surface.

---

# 35. Batch Atomicity

A transport batch is not automatically a single database transaction.

Default:

```text
batch transport
    !=
transaction
```

Each command receives its own domain transaction unless a documented command explicitly owns a larger atomic unit.

This prevents one huge offline queue from creating a giant long-running database transaction.

---

# 36. Per-Command Atomicity

A financially meaningful command should be atomic.

Example:

```text
FinalizeSale
  ├── sale
  ├── sale lines
  ├── inventory movement(s)
  ├── inventory balance update
  ├── payment intent/linkage
  ├── audit
  ├── command result
  └── outbox
```

All appropriate authoritative records commit together.

---

# 37. Acknowledgement Contract

An acknowledgement means:

> The server has durably classified the command.

It does not necessarily mean external side effects are complete.

Possible result states:

```text
ACCEPTED
ALREADY_ACCEPTED
REJECTED
CONFLICT
RETRYABLE
QUARANTINED
```

---

# 38. Command Result

A result should include stable machine-readable information:

```json
{
  "command_id": "...",
  "status": "ACCEPTED",
  "result_reference": {
    "type": "SALE",
    "id": "..."
  },
  "reason_code": null,
  "server_cursor": "..."
}
```

For rejection/conflict, return stable codes rather than free-form operational details.

---

# 39. The Most Important Failure Sequence

```text
CLIENT
  |
  | FinalizeSale(command_id=A)
  v
SERVER
  |
  | PostgreSQL COMMIT
  |
  X response lost
  |
CLIENT TIMEOUT
  |
  | retry same command_id=A
  v
SERVER
  |
  | idempotency lookup
  v
ALREADY_ACCEPTED
```

Expected invariant:

```text
one command_id
→ one business effect
```

---

# 40. Same Idempotency Key, Different Payload

If:

```text
command_id = A
payload = X
```

was already accepted, then later:

```text
command_id = A
payload = Y
```

must produce:

```text
SYNC_IDEMPOTENCY_MISMATCH
```

It must not execute Y.

---

# 41. Response-Loss Retry

Any command sent across an unreliable connection must be treated as ambiguous after timeout.

Unsafe:

```text
timeout → create a new command
```

Safe:

```text
same command_id → retry
```

This is central to offline financial correctness.

---

# 42. Retry Classification

## Retryable

Examples:

- temporary connection failure;
- transient server overload;
- temporary service unavailable;
- known transient database serialization conflict where the command is safe to retry.

## Terminal rejection

Examples:

- malformed command;
- unauthorized scope;
- expired capability;
- invalid state;
- unsupported schema;
- refund exceeds eligible amount.

## Conflict

Examples:

- inventory unavailable;
- stale version;
- simultaneous incompatible state transition;
- count reconciliation mismatch.

## Security quarantine

Examples:

- invalid high-assurance signature;
- revoked device abuse;
- suspicious replay behavior;
- impossible device/tenant relationship;
- corrupted security metadata.

---

# 43. Retry Backoff

Use:

```text
exponential backoff
+
jitter
+
maximum delay
+
retry budget
```

The client must stop automatically retrying a permanently rejected command.

A mobile device with poor connectivity must not turn one failure into an infinite battery/network drain.

---

# 44. Retry After Authentication Expiry

When authentication expires:

```text
pause sync
   ↓
refresh / reauthenticate
   ↓
resume same commands
```

The client must not generate new business commands merely because an access token changed.

---

# 45. Retry After Process Termination

At startup:

```text
inspect commands in SENDING
       ↓
mark PENDING_SYNC
       ↓
resume with same command_id
```

The client must assume the previous request may have committed on the server.

---

# 46. Retry After Device Reboot

Same rule:

```text
command_id preserved
```

The local journal is authoritative for local intent until server acknowledgement is known.

---

# 47. Retry After Revocation

If a device becomes revoked:

```text
stop normal sync
preserve evidence
reject/quarantine commands according to policy
```

Do not silently transfer them to a new device as fresh operations.

---

# 48. Client Sequence Numbers

Each device SHOULD maintain a monotonically increasing client sequence:

```text
device D
1001
1002
1003
```

The sequence helps detect gaps and diagnose client corruption.

It is not:

- a globally unique ID;
- an authentication credential;
- sufficient replay protection;
- proof of authorization.

---

# 49. Sequence Gaps

If the server sees:

```text
1001
1002
1004
```

it may record the gap.

A gap is not automatically a rejection because prior commands might be:

- still pending;
- terminally rejected;
- intentionally missing;
- lost due to local corruption.

Business dependencies must be explicit.

---

# 50. Business Causality

If command B logically depends on command A, the client expresses the dependency.

```text
A = CreateSale
B = FinalizeSale
B depends on A
```

The server must not infer causality merely from network arrival order.

---

# 51. Dependency Handling

If:

```text
B depends on A
A is unresolved
```

then B may enter:

```text
WAITING_DEPENDENCY
```

subject to bounded storage and time limits.

Dependency cycles must be detected:

```text
A → B
B → A
```

and classified without entering an infinite loop.

---

# 52. Pull / Download Architecture

Synchronization is bidirectional:

```text
UPLOAD durable commands
+
DOWNLOAD authoritative changes
```

The downloaded data is scoped by current authorization.

A client does not receive the entire tenant event stream simply because it authenticated.

---

# 53. Server Change Cursor

Use an opaque server-controlled cursor/position rather than assuming that timestamps alone are a reliable synchronization sequence.

Example:

```text
cursor = 1842231
```

A cursor is:

- opaque;
- durable;
- scoped;
- resumable;
- non-authorizing.

It is not itself a security credential.

---

# 54. Checkpoint Contract

A client advances a checkpoint only after it has durably persisted all state represented by that checkpoint.

Safe:

```text
receive delta
  ↓
apply local transaction
  ↓
commit
  ↓
advance checkpoint
```

Unsafe:

```text
receive delta
  ↓
advance checkpoint
  ↓
crash
  ↓
data missing
```

---

# 55. Checkpoint and Crash Recovery

If the app crashes during delta application, the checkpoint must remain at the last safely committed point.

The client can then replay the same delta batch.

Therefore server delta application should be idempotent.

---

# 56. Delta Event Identity

Every authoritative delta that must not be applied twice should have stable event identity.

Example:

```text
event_id = E
```

Client behavior:

```text
if E already applied:
    no-op
else:
    apply + record E
```

---

# 57. Missing Delta

If the client expects:

```text
100
```

and receives:

```text
102
```

without knowing that 101 is intentionally absent, it must not blindly advance beyond the missing position.

Possible recovery:

```text
request missing range
```

or:

```text
scoped resync
```

---

# 58. Delta Retention

The server must retain synchronization history long enough to support expected offline clients.

When a client is too far behind:

```text
delta history unavailable
```

the correct behavior is explicit recovery:

```text
RESYNC_REQUIRED
```

not fabricated completeness.

---

# 59. Snapshot / Bootstrap

A bootstrap snapshot should include enough metadata to verify consistency:

```text
snapshot_id
schema_version
scope
created_at
server_cursor
checksum
```

The client should validate the snapshot before treating it as authoritative local projection input.

---

# 60. Full Resynchronization

Possible recovery sequence:

```text
pause normal projection writes
      ↓
preserve durable command journal
      ↓
resolve/reconcile safe commands
      ↓
obtain authoritative snapshot
      ↓
replace/rebuild local projections
      ↓
establish checkpoint
      ↓
resume synchronization
```

Do not delete unresolved business commands merely because rebuilding local state is easier.

---

# 61. Projection Replacement

A local projection is derived state.

The architecture should make it possible to rebuild it from authoritative server information.

Therefore:

```text
projection corruption
```

must not imply:

```text
irrecoverable business truth corruption
```

---

# 62. Last-Write-Wins Prohibition

Last-write-wins MUST NOT be used for:

- finalized sales;
- inventory balance truth;
- financial corrections;
- payment settlement;
- cash events;
- approval decisions;
- permission state;
- device revocation;
- tax submission truth.

It may be considered for low-risk presentation metadata only after explicit domain review.

---

# 63. Conflict Categories

Use stable categories such as:

```text
STALE_VERSION
INSUFFICIENT_STOCK
INVALID_STATE_TRANSITION
PERMISSION_CHANGED
BRANCH_SCOPE_CHANGED
ENTITY_ARCHIVED
DUPLICATE_BUSINESS_REFERENCE
DEVICE_REVOKED
COMMAND_EXPIRED
SCHEMA_UNSUPPORTED
EXTERNAL_STATE_MISMATCH
CONFLICT_REQUIRES_REVIEW
```

---

# 64. Conflict State Machine

```text
OPEN
 ↓
CLASSIFIED
 ├── AUTO_RESOLVED
 ├── USER_ACTION_REQUIRED
 ├── APPROVAL_REQUIRED
 ├── RECONCILIATION_REQUIRED
 └── SECURITY_REVIEW
            ↓
         RESOLVED
            ↓
         CLOSED
```

A conflict is not “an error message.” It is a durable business/security state.

---

# 65. Conflict Record

Persist enough data to investigate:

```text
conflict_id
command_id
device_id
user_id
organization_id
branch_id
entity_type
entity_id
category
client_version
server_version
timestamps
state
resolution
resolved_by
resolved_at
evidence references
```

The original command must remain reconstructable.

---

# 66. Inventory Conflict Example

```text
authoritative stock = 1

Device A offline:
  Sale A = 1

Device B offline:
  Sale B = 1
```

After synchronization:

```text
Sale A → ACCEPTED
Sale B → CONFLICT / INSUFFICIENT_STOCK
```

The exact winner must be deterministic according to the server's processing and domain policy.

The system must not silently manufacture inventory.

---

# 67. Inventory Race Transaction

The authoritative transaction should resemble:

```text
BEGIN
  authenticate/scope
  lock or atomically update inventory position
  re-read authoritative stock
  validate availability
  post inventory movement
  update balance projection
  write sale linkage
  write audit
  write outbox
  write command result
COMMIT
```

The critical rule is server-side atomicity.

---

# 68. Price Conflict

An offline device can contain an older price version.

The command should preserve the price/configuration context used locally where the business model requires historical explanation.

Server policy determines whether that version remains valid.

The server must never rewrite a completed local sale by silently replacing its historical commercial facts.

---

# 69. Business-Day Semantics

The command can contain:

```text
created_at_client
```

but the server records:

```text
received_at_server
accepted_at_server
```

The domain determines which timestamp controls business reporting.

Client time is evidence, not an authority for security-sensitive expiry or privilege.

---

# 70. Offline Sale

```text
cashier
  ↓
local cart
  ↓
local price/stock validation
  ↓
FinalizeSale command
  ↓
SQLite transaction
  ↓
LOCAL_COMMITTED
  ↓
receipt/UX according to offline policy
  ↓
PENDING_SYNC
  ↓
server validation
  ↓
PostgreSQL transaction
  ↓
SERVER_CONFIRMED or explicit rejection/conflict
```

The merchant experience should remain simple even though the underlying protocol is sophisticated.

---

# 71. Offline Sale Does Not Equal Payment Settlement

An offline sale can record a payment method intent.

For external payment rails:

```text
sale recorded
```

does not prove:

```text
provider settlement confirmed
```

The payment subsystem resolves that separately.

---

# 72. Offline Cash

Offline cash operations may be allowed because physical cash is immediately observable.

Still enforce:

- register scope;
- user scope;
- amount thresholds;
- command identity;
- reconciliation;
- anti-duplication.

---

# 73. Offline Refunds

Refunds move money and therefore receive stronger controls.

Default policy should be:

```text
ordinary low-risk refund → policy-defined offline capability
high-risk refund          → online and/or approval
```

The server computes refundable value from authoritative state.

The client cannot choose an arbitrary refund amount and have the server trust it.

---

# 74. Offline Stock Adjustments

Stock adjustment is higher risk than ordinary selling.

Recommended model:

```text
small adjustment
    → potentially offline

large adjustment
    → online / approval
```

Exact thresholds are configuration.

---

# 75. Offline Stock Count

A stock count should preserve:

```text
count_session_id
location
scope
snapshot/base version
counted quantities
actor
device
time
```

The server reconciles the count against movements occurring after the snapshot.

Never use unconditional last-write-wins for stock counts.

---

# 76. Offline Procurement Receiving

Where allowed, receiving commands must include enough evidence to reconstruct:

```text
supplier
purchase order
receiving location
actual quantity
lot/batch
expiry
cost
actor
device
```

Server validates current purchase-order and inventory state.

---

# 77. Offline Transfer

A stock transfer is one business operation:

```text
source -Q
     +
destination +Q
```

The two movements cannot be independently considered completed.

If one side fails authoritative validation, the transfer remains unresolved rather than silently creating or destroying stock.

---

# 78. Offline Register Closing

Closing a register requires the server to reason about:

```text
register session
cash events
pending commands
cash variance
approval requirements
```

The client can provide the local state but cannot fabricate a server-approved close.

---

# 79. Synchronization and Payment Events

The system must support different arrival orders:

```text
sale first → provider callback later
```

and:

```text
provider evidence first → sale/linkage later
```

Provider event identity and reconciliation semantics determine how these converge.

---

# 80. Synchronization and EIS

Tax submission is an external state machine.

A local sale may be:

```text
TAX_PENDING
```

while the sale remains a finalized commercial fact.

Do not modify sale financial facts simply because tax submission failed.

---

# 81. Approval Synchronization

An approval is its own authoritative state transition.

```text
REQUESTED
   ↓
PENDING_APPROVAL
   ↓
APPROVED
   ↓
EXECUTED
```

A client payload such as:

```json
{"approved":true}
```

is not approval authority.

The server must identify the approver from authenticated membership and policy.

---

# 82. Separation of Duties Offline

A cashier cannot make themselves an approver by editing local state.

A restricted adjustment can require a second actor even when the first action was captured offline.

The synchronization system carries the evidence forward; the server decides final validity.

---

# 83. Authorization Changes While Offline

Example:

```text
09:00 cashier has permission
09:15 owner revokes permission
09:20 device reconnects
```

The server must re-evaluate queued commands against current authorization and the command's allowed historical policy.

For high-risk actions, newer security state normally wins and the command is rejected or quarantined.

---

# 84. Membership Versioning

A tenant membership or security policy version can accelerate cache invalidation.

Example:

```text
authorization_version = 27
```

Device has:

```text
26
```

Result:

```text
refresh capability
```

Version numbers are control metadata, not credentials.

---

# 85. Device Revocation

A revoked device MUST NOT continue normal synchronization.

Possible flow:

```text
server revoke
   ↓
device attempts sync
   ↓
SYNC_DEVICE_REVOKED
   ↓
security event
   ↓
commands preserved/quarantined
```

---

# 86. Stolen Device Response

```text
detect
  ↓
revoke device
  ↓
revoke device sessions
  ↓
invalidate capabilities
  ↓
preserve command/audit evidence
  ↓
issue new device identity
  ↓
scoped re-bootstrap
```

Do not trust the stolen device to perform its own secure cleanup.

---

# 87. Local Tampering Model

An attacker with access to a compromised mobile device may attempt to modify:

```text
stock
price
user
role
branch
payment state
command amount
command timestamp
checkpoint
```

The server must treat these values as untrusted.

---

# 88. Server Authority Against Local Tampering

Examples:

```text
local stock = 1,000,000
server stock = 2
```

Server evaluates stock using PostgreSQL authority.

```text
local role = OWNER
server role = CASHIER
```

Server evaluates as CASHIER.

```text
local paid = true
provider evidence = pending
```

Server does not settle the payment.

---

# 89. Local Integrity Metadata

Where useful, the local journal can maintain:

```text
content_hash
previous_command_hash
sequence
signature/MAC
```

This can detect local corruption or manipulation.

However:

> Client-side integrity metadata is not a substitute for server authority.

A fully compromised endpoint can potentially alter both data and local metadata.

---

# 90. Command Canonicalization

Commands that are signed or fingerprinted require deterministic canonical representation.

Define:

- field ordering;
- exact numeric representation;
- timestamp encoding;
- null vs absent semantics;
- enum representation;
- string normalization policy;
- binary encoding.

Never assume semantically equivalent JSON strings have identical bytes.

---

# 91. Replay Protection

Replay defense is layered:

```text
command_id uniqueness
+
idempotency record
+
state-machine validation
+
command age policy where appropriate
+
device state
+
capability validity
+
audit
```

No single control is sufficient.

---

# 92. Checkpoint Rollback Defense

A client must not be able to arbitrarily claim:

```text
“I am at cursor 100.”
```

and thereby cause unsafe historical reprocessing or cross-scope data exposure.

The server validates checkpoint relationships and scope.

---

# 93. Delta Ordering

The server must provide deterministic ordering for its change stream.

The client applies authoritative deltas in that order unless explicit parallelism is proven safe.

For dependent events, causal buffering may be required.

---

# 94. Event Ordering Example

```text
SaleFinalized
    ↓
SaleReversed
```

A projection must not incorrectly display:

```text
reversal exists
sale does not exist
```

because it applied the dependent event first without a safe model.

---

# 95. Projection Merge Rules

When server state conflicts with pending local state, the client may:

```text
apply
merge
wait
conflict
reject local command
```

It must not default to:

```text
overwrite
```

for financial or inventory domains.

---

# 96. Local Cache Revocation

When user scope decreases, the device may need to purge or invalidate local records no longer authorized.

Examples:

```text
branch access removed
employee access removed
customer-data policy changed
```

Cached authorization must not become a permanent entitlement.

---

# 97. Offline Multi-User Device

If multiple staff share one device, command actor identity must follow the authenticated user/session.

```text
cashier A logout
    ↓
cashier B login
    ↓
new commands belong to B
```

Previously queued commands remain attributed to A.

---

# 98. Logout and Pending Commands

Logout must not silently erase pending business commands.

It should however clear or protect authentication/session secrets according to the client security policy.

Queued commands remain linked to their original authenticated actor and are re-evaluated when synchronization resumes.

---

# 99. Storage Exhaustion

When local storage is low:

```text
protect durable business commands
protect sync receipts
protect required security metadata
purge low-value cache first
```

The application should surface an explicit degraded state before critical business persistence fails.

---

# 100. Queue Compaction

Only transient local drafts may be freely compacted.

Example:

```text
Draft edited 20 times
→ one current draft
```

But:

```text
Finalized sale command
```

must not be compacted into a form that destroys business/audit identity.

---

# 101. Command Archive

Terminal commands may move from a hot queue to an archive after successful reconciliation.

Archive retention must follow business and audit requirements.

The local archive is never the authoritative financial ledger.

---

# 102. Queue Garbage Collection

A command can leave the hot queue when:

```text
terminal server outcome persisted
+
required local evidence retained
```

Unresolved conflict or security-quarantine records must remain available for the required retention period.

---

# 103. Large Offline Backlog

A device can return after a long offline period with an unusually large queue.

The server must not accept unbounded work.

Possible strategy:

```text
bounded chunks
+
priority
+
rate limits
+
resync where required
+
command age policy
```

A queue of 50,000 commands must not monopolize the entire API.

---

# 104. Fairness Across Tenants

The sync subsystem must protect against noisy-neighbor abuse.

Limits may be applied across:

```text
IP
user
organization
device
command class
bytes
concurrency
```

A single broken device must not consume all sync worker capacity.

---

# 105. Background Sync Scheduling

Flutter synchronization should consider:

- connectivity;
- battery;
- app lifecycle;
- pending work;
- retry backoff;
- queue age;
- priority;
- network quality.

Do not use an unbounded tight loop.

---

# 106. Priority Classes

A possible policy:

```text
P0 security/session state
P1 financial/business mutations
P2 inventory
P3 reference metadata
P4 telemetry/low-value background work
```

The exact queue ordering can evolve from production measurements.

---

# 107. Sync Concurrency

Only one logical sync coordinator should normally own a device's local command queue.

However, server-side deduplication remains mandatory because local locking cannot protect against:

- duplicated background jobs;
- app restarts;
- network retries;
- multiple installations;
- malicious replay.

---

# 108. Network Transition

The protocol is independent of the transport network:

```text
Wi-Fi
 ↓
mobile data
 ↓
offline
 ↓
Wi-Fi
```

must not change command identity.

---

# 109. Network Chaos

Test synchronization under:

```text
latency
packet loss
connection reset
DNS failure
TLS failure
partial response
server timeout
provider timeout
intermittent connectivity
```

The command journal must remain correct.

---

# 110. Server Timeouts

Every sync operation has a server-side deadline.

Timeouts protect:

- API threads/tasks;
- database connections;
- memory;
- worker capacity.

A slow client must not keep server resources indefinitely.

---

# 111. Database Timeouts

Synchronization operations require bounded database work:

```text
statement timeout
transaction timeout
connection timeout
```

These are aligned with the broader Sitolo availability/security contract.

---

# 112. Retry Storm Protection

When the server is failing, clients may all retry simultaneously.

Controls:

```text
exponential backoff
jitter
server retry hints
client retry budget
rate limits
circuit breaking where useful
```

---

# 113. Dependency Isolation

Sync should not depend synchronously on:

```text
payment provider
MRA EIS
notification service
```

for core acknowledgement unless the domain explicitly requires that dependency.

Use PostgreSQL outbox/worker architecture for asynchronous external work.

---

# 114. Outbox Relationship

Example:

```text
FinalizeSale
   ↓
PostgreSQL transaction
   ├── Sale
   ├── InventoryMovement
   ├── Audit
   ├── CommandResult
   └── OutboxEvent
          ↓
        COMMIT
          ↓
      worker
       ├── EIS
       ├── payment reconciliation
       └── notifications
```

The mobile client does not wait for every external side effect before receiving core command acknowledgement unless the business contract explicitly requires it.

---

# 115. Event Replay vs Command Replay

These are different:

```text
command replay
→ attempts business intent again
```

```text
event replay
→ rebuilds a derived projection from facts already accepted
```

Event replay must never be treated as a fresh financial command.

---

# 116. Event Idempotency

Consumers should use stable event IDs:

```text
if event already consumed:
   no-op
else:
   process + record
```

Outbox delivery is therefore safely at-least-once.

---

# 117. Payment Event Ordering

Payment providers can emit:

```text
duplicate events
out-of-order events
late events
```

The reconciliation subsystem must handle these without duplicating financial state.

---

# 118. Tax Event Ordering

Tax submission can similarly progress independently:

```text
sale finalized
   ↓
EIS submission pending
   ↓
provider unavailable
   ↓
retry
   ↓
accepted/rejected
```

The sale remains economically authoritative.

---

# 119. Rejection Semantics

A rejected command must have:

```text
stable code
human-safe message
command_id
server timestamp
optional evidence reference
```

Example:

```text
INSUFFICIENT_STOCK
```

Do not return raw SQL errors or internal stack traces.

---

# 120. Terminal vs Temporary Rejection

A rejection can be:

```text
terminal
```

or:

```text
retryable
```

The distinction must be explicit.

For example:

```text
403 authorization denied
→ terminal under current identity

503 service unavailable
→ retryable
```

---

# 121. User Experience for Rejection

Bad:

```text
Sync failed.
```

Better:

```text
Sale saved locally.
Server stock validation rejected the sale.
Review the stock conflict before continuing.
```

The user must not be told that a rejected server command succeeded.

---

# 122. User Experience for Retry

Example:

```text
Connection unavailable.
Your sale is saved and will sync automatically.
```

This reflects the real state.

---

# 123. User Experience for Device Revocation

Example:

```text
This device is no longer authorized.
Your pending work has been preserved for recovery.
```

Do not disclose internal security details.

---

# 124. Conflict UX

Example:

```text
Sale saved locally.
Server stock changed before synchronization.
Review required.
```

Do not automatically overwrite important financial/inventory state just to make the screen green.

---

# 125. Security Events

Useful security events include:

```text
DEVICE_REVOKED_SYNC_ATTEMPT
COMMAND_REPLAY
COMMAND_SIGNATURE_FAILURE
CAPABILITY_EXPIRED
TENANT_SCOPE_MISMATCH
BRANCH_SCOPE_MISMATCH
CHECKPOINT_ANOMALY
SYNC_FLOOD
SCHEMA_MISMATCH
```

These events should flow into the security telemetry system.

---

# 126. Audit Events

Business audit should preserve:

```text
command_id
user_id
device_id
organization_id
branch_id
command_type
result
reason_code
received_at
accepted_at
correlation_id
```

Sensitive payload values should not be dumped into ordinary logs.

---

# 127. Forensic Chain

A disputed offline transaction should be reconstructable:

```text
User
 ↓
Session
 ↓
Device
 ↓
Command
 ↓
Local timestamp
 ↓
Server receipt
 ↓
Authorization version
 ↓
Policy version
 ↓
Database transaction
 ↓
Sale / Inventory / Payment
 ↓
Audit
 ↓
Outbox
 ↓
External provider evidence
```

This is especially important for fraud and merchant disputes.

---

# 128. Local Privacy

The local store should minimize PII.

For example, a cashier may need:

```text
customer display name
```

but not necessarily:

```text
all customer historical records
all identity documentation
```

The exact retention policy is defined by product/privacy requirements.

---

# 129. Local Sensitive Data Retention

Define retention separately for:

- customer data;
- employee data;
- financial snapshots;
- receipts;
- operational caches;
- security diagnostics.

Offline retention should not silently become indefinite retention.

---

# 130. Logout, Device Wipe and Retirement

On retirement or secure wipe:

```text
invalidate local sessions
remove secret material
clear or cryptographically retire sensitive data
preserve required recoverable command evidence
```

The exact sequence must account for whether unsynchronized business commands still require recovery.

---

# 131. Device Replacement Recovery

```text
old device
   ↓
revoked
   ↓
new device registration
   ↓
authenticated user
   ↓
scope verification
   ↓
bootstrap
   ↓
reconcile outstanding server state
```

No old device identifier is reused.

---

# 132. Local Database Corruption

If SQLite integrity fails:

```text
stop normal writes
preserve diagnostics
extract durable recoverable command state if possible
revoke device when compromise is plausible
perform scoped/full resync
verify checkpoint
resume
```

Do not “fix” financial truth by editing local history manually.

---

# 133. Queue Poisoning

One malformed command must not permanently block the entire queue.

The system should isolate the poison item:

```text
QUARANTINED
```

while allowing safe independent work to continue.

---

# 134. Dead-Letter / Quarantine

Quarantined commands need:

- reason;
- owner;
- evidence;
- retry/replay policy;
- retention;
- audit.

A dead-letter state is not “deleted.”

---

# 135. Quarantine Replay

If an operator replays a quarantined command, use the same original command identity where business semantics allow it, and record:

```text
operator
reason
approval
previous state
replay timestamp
result
```

High-risk replay may require step-up or approval.

---

# 136. Command Dependency Cycles

The dependency graph must be bounded.

If:

```text
A depends B
B depends C
C depends A
```

classify the cycle rather than retry forever.

---

# 137. Command Age

Some commands need maximum age limits because their meaning becomes unsafe over time.

Examples:

- privileged approval;
- temporary capability;
- short-lived security operation.

Normal offline retail commands can have different retention semantics.

The policy must be command-class specific.

---

# 138. Protocol Versioning

All synchronized commands carry protocol/schema versions.

Example:

```text
protocol_version = 1
schema_version = 3
```

Unknown versions must not be silently interpreted.

---

# 139. Schema Compatibility

Controlled compatibility may use:

```text
current = v3
accept v2 + v3
migrate clients
measure v2 usage
retire v2
```

A stale offline mobile installation may remain disconnected longer than a normal browser session, so deprecation must be deliberate.

---

# 140. Downgrade Protection

A client must not be able to choose an older protocol merely to disable newer security controls.

The server maintains:

```text
minimum_protocol_version
minimum_client_version
```

and rejects unsafe downgrade paths.

---

# 141. Bootstrap Compatibility

A newer server may provide an explicit bootstrap format that older clients cannot interpret.

Therefore the server must negotiate supported versions before sending a large snapshot.

---

# 142. Rolling Deployments

During server rollout, overlapping server versions must understand all supported command schemas.

Do not deploy a server version that immediately makes the current supported mobile fleet unable to synchronize.

---

# 143. Mobile App Upgrades

App upgrades must preserve pending command identity.

Test:

```text
old client
100 pending commands
↓
upgrade app
↓
new client
↓
sync
```

Expected:

```text
no command loss
no duplicate effects
```

---

# 144. API Compatibility

The sync endpoint follows the API contract, including:

- authentication;
- request size limits;
- error contract;
- idempotency;
- correlation IDs;
- versioning;
- rate limiting.

---

# 145. Browser/Tauri/Flutter Relationship

Flutter is the primary offline operational client.

Tauri may support local operational workflows but must use the same command/domain semantics.

The web/admin surface should generally rely on online API operations except where an explicit offline design exists.

No client should invent a different business truth model.

---

# 146. Tauri Local Sync

If Tauri requires durable local operation:

- use the same command contracts;
- protect native IPC boundaries;
- scope local data;
- protect local secrets;
- reuse protocol semantics.

Do not implement a second proprietary sync protocol inside the desktop client.

---

# 147. Web Admin

Web admin should not casually cache sensitive business state offline.

Where browser synchronization is introduced later, it requires a separate threat review because browser storage and service-worker persistence have different compromise characteristics.

---

# 148. Sync and Authentication Architecture

Authentication is delegated to the identity/session system.

Synchronization consumes trusted authentication context:

```text
principal
session
organization
membership
scope
```

It does not reimplement password hashing or OAuth/OIDC logic.

---

# 149. Sync and Authorization Architecture

Authorization remains server-enforced.

Sync adds an additional dimension:

```text
is this operation permitted
```

must become:

```text
is this operation permitted
AND
is this operation permitted in offline mode
AND
is this device still allowed to execute it
```

---

# 150. Sync and Entitlements

Subscription entitlements can constrain the command registry.

However, the client's cached feature flag is not authority.

The server evaluates current entitlement where required.

---

# 151. Sync and Feature Flags

Feature flags included in offline capability profiles must be versioned.

A stale client flag cannot unlock a newer security-sensitive command.

---

# 152. Sync and Country Configuration

Country-specific policy may affect:

- tax;
- currency;
- numbering;
- receipt rules;
- retention;
- payment methods.

Configuration must therefore be versioned where it materially affects business interpretation.

---

# 153. Configuration References

Instead of embedding huge configuration snapshots in every command, store compact references:

```text
pricing_policy_version
payment_policy_version
tax_policy_version
offline_capability_version
```

The authoritative history remains server-side.

---

# 154. Business Invariants Enforced During Sync

Examples:

```text
refund <= eligible refundable amount
stock >= configured minimum
provider event unique
sale finalized once per command
cash close only when legal state allows
cashier cannot approve own restricted action
expired lot cannot enter prohibited sale allocation
revoked device cannot continue normal sync
```

These are security controls as well as business controls.

---

# 155. Financial Immutability During Sync

A finalized offline sale that becomes server-confirmed is not later edited destructively.

Correction uses:

```text
return
refund
reversal
adjustment
replacement
```

according to the domain model.

---

# 156. Inventory Ledger During Sync

The client may maintain an estimated/operational projection.

The server owns the authoritative ledger.

The client must never send:

```text
“set stock = 50”
```

as authority merely because its local projection says 50.

Use domain commands such as:

```text
AdjustStock
ReceiveGoods
TransferStock
FinalizeSale
```

that the server interprets against authoritative state.

---

# 157. Read Model Convergence

After successful synchronization:

```text
local projection
→ should converge toward authoritative server state
```

Derived values such as analytics and search can lag without violating core business truth.

---

# 158. No Hidden Second Ledger

The local SQLite database must not become a second financial ledger with a different correction model.

Local records are operational projections and commands.

The authoritative financial history remains the server domain ledger.

---

# 159. Offline Payment Safety

The payment workflow distinguishes:

```text
payment intent
provider evidence
internal settlement state
```

A client payload cannot directly change final settlement.

This prevents frontend payment tampering and fraudulent offline confirmation.

---

# 160. Webhook Replay Relation

A payment webhook may arrive independently of synchronization.

Both pathways converge on the same authoritative payment state and uniqueness constraints.

```text
offline sale command
         |
         v
     PaymentIntent
         ^
         |
provider webhook
```

Neither pathway is allowed to create duplicate financial effects.

---

# 161. Sync Security Against Cross-Tenant Access

Mandatory test:

```text
Device A / Tenant A
  ↓
modify payload → Tenant B
```

Expected:

```text
DENY
no Tenant B data
no state mutation
no side-channel success
```

Repeat for every command class.

---

# 162. Cross-Branch Security

A device scoped to Branch A cannot synchronize commands for Branch B unless current policy grants Branch B access.

Local branch identifiers are not authority.

---

# 163. Cross-User Security

A user cannot embed another user's identity into a command to perform an action as that user.

The authenticated principal remains authoritative.

---

# 164. Property-Level Security

Suppose a command payload contains:

```json
{
  "discount": 90,
  "approval_required": false
}
```

If the current role may not modify approval semantics, the server rejects or ignores the unauthorized field according to the API's explicit mass-assignment policy.

The client cannot bypass property authorization through synchronization.

---

# 165. Security-Sensitive Fields

Security-sensitive fields include:

```text
user_id
organization_id
branch_id
role_id
permission
approval_id
provider_status
settlement_state
security_policy
```

These must be derived or revalidated server-side.

---

# 166. Rate Limits

Sync limits should include:

```text
per device
per user
per organization
per IP where useful
per command class
per byte volume
per concurrency
```

A carrier NAT may contain many legitimate users, so IP-only throttling is insufficient.

---

# 167. Rate Limit Failure

Rate-limit responses should communicate retry behavior without leaking sensitive policy details.

The mobile client honors bounded backoff.

---

# 168. Sync Authentication Throttling

Repeated failed synchronization authentication can indicate:

- stolen token;
- compromised device;
- automated probing.

Apply security controls and produce security telemetry.

---

# 169. Endpoint Exposure

Synchronization endpoints are protected.

Do not expose an unauthenticated endpoint that can:

```text
upload commands
inspect checkpoints
enumerate device state
```

---

# 170. Health Endpoints

Public health endpoints must not expose:

- tenant information;
- queue contents;
- database credentials;
- internal sync state;
- raw configuration.

Detailed synchronization health remains restricted operational telemetry.

---

# 171. Sync API Resource Model

Conceptual API families:

```text
POST /v1/sync/sessions
POST /v1/sync/batches
GET  /v1/sync/deltas
POST /v1/sync/resync
POST /v1/sync/acknowledge
```

The exact route structure is governed by `api_contract.md`.

---

# 172. Sync Handshake Endpoint

The handshake creates or refreshes the server-side synchronization context.

It may return:

```text
sync_session_id
protocol capabilities
server cursor
capability version
server time
limits
```

---

# 173. Submit Batch Endpoint

The batch endpoint accepts commands and returns per-command classifications.

Example:

```text
A → ACCEPTED
B → ALREADY_ACCEPTED
C → CONFLICT
D → RETRYABLE
E → REJECTED
```

The entire transport request does not need to succeed uniformly.

---

# 174. Delta Endpoint

The client supplies an opaque cursor and receives a bounded delta page.

The server validates:

```text
cursor scope
current device scope
current authorization
```

before returning data.

---

# 175. Resync Endpoint

A resync endpoint initiates recovery from an invalid/expired checkpoint or incompatible local state.

It must not silently grant broader scope than normal synchronization.

---

# 176. Sync Error Codes

At minimum:

```text
SYNC_AUTH_REQUIRED
SYNC_DEVICE_REVOKED
SYNC_SCOPE_DENIED
SYNC_BATCH_TOO_LARGE
SYNC_PAYLOAD_TOO_LARGE
SYNC_PROTOCOL_UNSUPPORTED
SYNC_SCHEMA_UNSUPPORTED
SYNC_CAPABILITY_EXPIRED
SYNC_IDEMPOTENCY_MISMATCH
SYNC_COMMAND_REPLAY
SYNC_COMMAND_REJECTED
SYNC_CONFLICT
SYNC_RETRYABLE
SYNC_QUARANTINED
SYNC_CHECKPOINT_INVALID
SYNC_RESYNC_REQUIRED
```

---

# 177. Safe Error Responses

The client should get enough information to act safely, but not internal details.

Bad:

```text
Postgres unique violation on table sync_command...
```

Good:

```text
SYNC_IDEMPOTENCY_MISMATCH
```

with a stable human-safe message.

---

# 178. Correlation

Every sync request should have:

```text
request_id
trace_id
sync_session_id
batch_id
command_id
```

where applicable.

This allows a failure to be traced across:

```text
mobile → API → DB → outbox → provider
```

---

# 179. Metrics

Track:

```text
sync_sessions
commands_created
commands_sent
commands_accepted
commands_rejected
commands_conflicted
commands_quarantined
retry_count
batch_size
payload_bytes
sync_duration
checkpoint_lag
oldest_pending_age
revoked_device_attempts
```

Avoid high-cardinality raw merchant/customer values in metric labels.

---

# 180. Alerting

Alert on:

- rapidly growing pending queues;
- conflict spikes;
- revoked-device sync attempts;
- replay spikes;
- protocol mismatch spikes;
- checkpoint stagnation;
- unusual command volume;
- high sync latency;
- server-side quarantine growth.

---

# 181. Device Health

Device sync health can track:

```text
last_successful_sync
oldest_pending_command
pending_command_count
pending_bytes
last_cursor
protocol_version
app_version
authorization_version
device_state
```

This allows operators to detect unhealthy merchant devices before failures become data-reconciliation problems.

---

# 182. Fleet Health

Aggregate views can identify:

```text
same version failing everywhere
same command class conflicting everywhere
same provider blocking downstream reconciliation
```

These are operational signals, not merchant-facing data.

---

# 183. Synchronization Test Architecture

The test model is multi-layered:

```text
pure state tests
     ↓
property tests
     ↓
SQLite tests
     ↓
API sync integration
     ↓
real PostgreSQL tests
     ↓
concurrency tests
     ↓
network chaos
     ↓
device tests
     ↓
manual adversarial review
```

---

# 184. Mandatory Test Fixture Topology

Create deterministic fixtures:

```text
TENANT_A
TENANT_B

BRANCH_A
BRANCH_B

CASHIER_A
MANAGER_A
CASHIER_B

DEVICE_A
DEVICE_B

SKU_X
STOCK_1

SALE_A
SALE_B
```

The same fixtures should be reusable across domain/security/sync suites.

---

# 185. Normal Offline Sale Test

```text
GIVEN network is unavailable
AND cashier has valid offline capability
AND local catalogue state is valid

WHEN cashier finalizes a sale

THEN SQLite commit succeeds
AND command is pending
AND restart preserves the command
AND reconnect uploads the same command_id
AND server classifies the command
AND the client persists the result
```

---

# 186. Timeout-After-Commit Test

```text
server commits sale
response is lost
client times out
client retries same command_id
```

Expected:

```text
existing result returned
no duplicate sale
```

This is release-blocking.

---

# 187. Cross-Tenant Test

```text
Device A belongs Tenant A
payload altered to Tenant B
```

Expected:

```text
DENY
no Tenant B data
no Tenant B write
```

---

# 188. Cross-Branch Test

```text
Device A → Branch A
payload altered → Branch B
```

Expected:

```text
DENY
```

---

# 189. Forged Role Test

```text
local role = OWNER
server membership = CASHIER
```

Expected:

```text
CASHIER authority
```

not owner authority.

---

# 190. Revoked Device Test

```text
device active
queue commands
server revokes device
device reconnects
```

Expected:

```text
normal synchronization denied
security event created
commands preserved/quarantined
```

---

# 191. Duplicate Command Test

Submit identical command 100 times.

Expected:

```text
one business effect
many safe duplicate results
```

---

# 192. Duplicate Event Test

Apply the same server event repeatedly.

Expected:

```text
one local effect
```

---

# 193. Modified Payload Test

```text
original command:
  amount = 10000

retry:
  same command_id
  amount = 1000000
```

Expected:

```text
IDEMPOTENCY_MISMATCH
```

---

# 194. Inventory Race Test

```text
stock = 1

Device A sells 1
Device B sells 1
```

Concurrent server processing must result in a domain-valid outcome without inventory fabrication.

---

# 195. Refund Race Test

```text
refund eligibility = 5,000

request A = 5,000
request B = 5,000
```

Expected:

```text
only one can consume the same refundable amount
```

unless an explicit domain policy provides another legal split.

---

# 196. Checkpoint Crash Test

```text
receive delta
crash before checkpoint commit
restart
```

Expected:

```text
delta safely reapplied
no data loss
no duplicate projection effect
```

---

# 197. Missing Delta Test

If delta 101 is missing and 102 arrives, client requests recovery rather than blindly advancing.

---

# 198. Schema Compatibility Test

Test:

```text
old supported command
new supported command
unsupported command
future version command
```

Expected results must be explicit.

---

# 199. Clock Manipulation Test

Modify device clock dramatically.

Expected:

```text
server security/freshness rules remain authoritative
```

---

# 200. Large Backlog Test

Simulate a device returning with thousands of commands.

Verify:

- bounded memory;
- bounded batch size;
- fair resource usage;
- no retry storm;
- progress or explicit recovery.

---

# 201. Low Storage Test

Fill device storage close to capacity.

Verify:

```text
critical command persistence protected
low-value cache evicted first
UI warns operator
```

---

# 202. App Kill Test

Kill the process at:

```text
before local commit
after local commit
while preparing request
after request transmission
before response
while applying delta
before checkpoint
```

Every point should map to an explicit expected state.

---

# 203. Network Chaos Matrix

| Failure | Expected behavior |
|---|---|
| no connectivity | local queue continues where policy allows |
| connection reset | same command retried |
| server 503 | bounded retry |
| response timeout | retry same command |
| DNS failure | retain queue |
| TLS handshake failure | retain queue; alert if persistent |
| partial batch response | process known results, retain unresolved work |
| provider timeout | external state remains pending |

---

# 204. Property-Based Testing

Generate command sequences and verify invariants such as:

```text
duplicate(command) is equivalent to one command
```

and:

```text
projection after convergence == authoritative projection
```

where the domain declares convergence valid.

Generate randomized:

- duplicate delivery;
- ordering changes;
- inventory sequences;
- return/refund sequences;
- configuration versions;
- network failures.

---

# 205. Model-Based Testing

Create a simplified reference model:

```text
state + command → next state
```

and compare selected production behavior against it.

This is particularly valuable for:

- sale state;
- inventory state;
- command lifecycle;
- checkpoint progression.

---

# 206. Mutation Testing

Deliberately mutate security-sensitive implementation decisions:

```text
remove tenant predicate
remove idempotency check
advance checkpoint early
ignore device revocation
allow stale capability
```

The test suite must detect the mutation.

---

# 207. Fuzzing Targets

Fuzz:

```text
command envelope
command decoder
schema version parser
dependency graph
cursor parser
checkpoint parser
compressed payload handling
canonicalization
conflict parser
```

Fuzz tests must have CPU/memory/time limits.

---

# 208. Real PostgreSQL Requirement

Mocks cannot prove:

- unique idempotency constraints;
- transaction behavior;
- locking;
- RLS behavior;
- deadlock handling.

Therefore critical sync integration tests run against a real PostgreSQL instance.

---

# 209. Real Device Requirement

The test program should include representative low/mid-range Android hardware because Sitolo explicitly targets lower-cost devices and intermittent connectivity.

Test:

- memory pressure;
- storage limits;
- battery behavior;
- app suspension;
- process death;
- network transition.

---

# 210. Security Regression Matrix

Mandatory cases include:

```text
cross-tenant command
cross-branch command
forged user
forged role
forged approval
revoked device
expired capability
replay
modified payload
checkpoint rollback
oversized batch
oversized command
dependency cycle
unsupported schema
protocol downgrade
invalid signature
local stock tampering
local price tampering
payment-state tampering
```

All must be machine-enforceable where possible.

---

# 211. Definition of Sync Security Done

Synchronization is not complete until:

```text
[ ] durable local journal
[ ] stable command identity
[ ] duplicate resistance
[ ] timeout-after-commit safety
[ ] authentication
[ ] device validation
[ ] tenant validation
[ ] branch validation
[ ] offline capability validation
[ ] idempotency mismatch detection
[ ] batch limits
[ ] payload limits
[ ] retry budgets
[ ] bounded resource use
[ ] explicit conflicts
[ ] explicit rejections
[ ] checkpoint crash safety
[ ] event deduplication
[ ] resync
[ ] revocation
[ ] schema compatibility
[ ] downgrade protection
[ ] audit
[ ] telemetry
[ ] concurrency tests
[ ] fuzz tests
[ ] device tests
[ ] CI enforcement
```

---

# 212. Failure-Closed Rules

The synchronization subsystem must fail closed when security-critical evidence is unavailable.

Examples:

```text
missing tenant context → deny
missing device state → deny sensitive sync
missing capability verification → deny high-risk command
unsupported schema → reject
invalid signature → reject
idempotency store unavailable → do not execute duplicate-prone financial command
checkpoint invalid → controlled recovery
```

Availability can be sacrificed before financial/integrity trust.

---

# 213. Secure Degradation

Safe degradation means:

```text
payment provider unavailable
→ payment pending

EIS unavailable
→ tax pending

network unavailable
→ local operation where permitted

authorization service unavailable
→ sensitive mutation denied
```

Do not confuse graceful degradation with “always allow.”

---

# 214. Recovery Runbook — Stuck Queue

```text
1. Confirm connectivity.
2. Check authentication/session.
3. Check device state.
4. Inspect oldest pending command.
5. Inspect command result/conflict.
6. Check protocol/schema compatibility.
7. Check server health.
8. Retry boundedly.
9. Trigger resync if explicitly required.
10. Preserve evidence.
```

Never start by deleting the queue.

---

# 215. Recovery Runbook — Repeated Conflict

```text
identify command
→ identify category
→ compare client/server versions
→ inspect concurrent device activity
→ inspect policy version
→ determine automatic/manual resolution
→ preserve evidence
→ resolve
→ add regression test if systemic
```

---

# 216. Recovery Runbook — Device Revocation

```text
detect
→ confirm device ID
→ revoke session/device
→ preserve attempted command IDs
→ inspect security telemetry
→ rotate credentials if needed
→ issue replacement device
```

---

# 217. Recovery Runbook — Corrupt Checkpoint

```text
stop normal sync
→ preserve local state
→ validate cursor/checkpoint
→ attempt scoped recovery
→ rebuild projection if required
→ establish new checkpoint
→ resume
```

---

# 218. Recovery Runbook — Suspected Local Tampering

```text
stop trusting local projection
→ preserve command journal
→ revoke device when compromise plausible
→ inspect security evidence
→ re-bootstrap
→ reconcile pending commands
→ issue replacement device
```

---

# 219. Recovery Runbook — Server Restore

After PostgreSQL recovery:

```text
restore
→ integrity checks
→ command uniqueness checks
→ outbox checks
→ cursor/checkpoint checks
→ inventory/financial reconciliation
→ external provider reconciliation
→ reopen sync
```

A database restore must preserve the idempotency/control-plane state needed to survive mobile retries after recovery.

---

# 220. Backup Requirements

Backups must include the synchronization control-plane records required to make previously accepted commands remain duplicate-safe.

Recovering:

```text
sale state
```

without recovering:

```text
command/idempotency state
```

can reintroduce duplicate-effect risk.

---

# 221. Change Management

Changes to any of these require protocol review:

```text
command identity
idempotency semantics
offline command allowlist
capability lifetime
checkpoint semantics
cursor semantics
schema versions
conflict rules
device revocation
local retention
financial offline behavior
inventory offline behavior
```

Material trust-boundary changes require an ADR.

---

# 222. ADRs Required for Sync

Minimum sync-specific ADRs:

```text
ADR-SYNC-001 command-based synchronization
ADR-SYNC-002 local SQLite durability settings
ADR-SYNC-003 command identity/idempotency
ADR-SYNC-004 cursor/checkpoint design
ADR-SYNC-005 offline capability profiles
ADR-SYNC-006 conflict classification
ADR-SYNC-007 resync strategy
ADR-SYNC-008 device revocation handling
ADR-SYNC-009 delta retention
ADR-SYNC-010 protocol version compatibility
```

Each ADR should capture:

```text
context
decision
alternatives
security impact
operational impact
tradeoffs
reversibility
evidence
consequences
```

---

# 223. Build-vs-Buy Boundary

Sitolo should build the semantics that create its moat:

```text
offline command model
inventory conflict rules
financial command semantics
authorization-aware synchronization
reconciliation
```

Reuse mature components for:

```text
SQLite
TLS
cryptography
serialization
secure OS storage
observability
```

Do not create a custom database engine or custom cryptographic system merely because synchronization is complex.

---

# 224. Why CRDTs Are Not the Default

CRDTs can be excellent for conflict-free collaborative data.

They are not a universal solution for:

```text
inventory depletion
financial refunds
cash reconciliation
payment settlement
approval authority
```

These require business invariants and authoritative decisions.

Sitolo can use mergeable data structures selectively for low-risk collaboration, but not as the financial truth model.

---

# 225. Why Last-Write-Wins Is Unsafe

“Latest” does not mean “correct.”

A later stock update cannot legitimately create physical inventory.

A later refund cannot exceed eligibility merely because its timestamp is newer.

A later client role cannot become authoritative because it was written after the real server role.

---

# 226. Why the Client Cannot Be Authority

The endpoint can be:

- offline;
- stolen;
- rooted;
- reverse engineered;
- modified;
- replayed;
- disconnected from the server.

Therefore the server remains the authoritative boundary.

---

# 227. Why the Client Must Still Be Durable

Server authority does not mean the client should lose merchant work when connectivity disappears.

The product requirement is:

```text
network fails
→ business continues
```

Therefore local command durability is essential.

---

# 228. Core Invariants

The protocol must maintain these invariants:

```text
one command_id
→ at most one authoritative command effect

checkpoint advanced
→ required local state durably persisted

server-confirmed command
→ durable authoritative transaction exists

rejected command
→ no protected business effect

revoked device
→ no normal new privileged synchronization

cross-tenant command
→ no tenant-B access

same event twice
→ one local application

financial correction
→ explicit compensating event
```

---

# 229. End-to-End Example — Offline Sale

```text
Cashier scans SKU-X
      ↓
Local catalogue lookup
      ↓
Local availability check
      ↓
Build FinalizeSale command
      ↓
Generate command_id C1
      ↓
SQLite BEGIN
      ↓
write command journal
      ↓
update local operational projection
      ↓
COMMIT
      ↓
UI: “Sale saved locally”
      ↓
network unavailable
      ↓
queue remains pending
      ↓
network returns
      ↓
POST /sync/batches
      ↓
Rust authenticates user/session/device
      ↓
checks tenant + branch + capability
      ↓
checks idempotency
      ↓
loads authoritative inventory
      ↓
executes transaction
      ↓
Sale + inventory movement + audit + outbox + command result
      ↓
COMMIT
      ↓
ACK C1 = ACCEPTED
      ↓
client persists result
      ↓
client applies authoritative delta
      ↓
checkpoint advances
```

---

# 230. End-to-End Example — Timeout After Commit

```text
C1 submitted
 ↓
server commits C1
 ↓
response lost
 ↓
client timeout
 ↓
C1 remains pending
 ↓
retry C1
 ↓
server finds existing result
 ↓
ALREADY_ACCEPTED
 ↓
client marks confirmed
```

No duplicate sale.

---

# 231. End-to-End Example — Inventory Conflict

```text
Server stock = 1

Device A offline:
  sale A = 1

Device B offline:
  sale B = 1

A syncs first
  → ACCEPTED
  → stock = 0

B syncs
  → INSUFFICIENT_STOCK / CONFLICT
```

The system preserves an explicit record that B was locally recorded but could not become authoritative under current stock policy.

---

# 232. End-to-End Example — Permission Revocation

```text
cashier authorized at 09:00
 ↓
creates ordinary sale locally
 ↓
owner revokes membership at 09:10
 ↓
device reconnects at 09:20
 ↓
sale command is evaluated under defined historical/offline policy
 ↓
new privileged command
 ↓
DENY
```

The system must document which class of already-created offline work remains acceptable after revocation; this is not something the client decides.

---

# 233. End-to-End Example — Payment Race

```text
offline sale command
        ↓
PaymentIntent
        ↓
webhook arrives
        ↓
provider event dedupe
        ↓
reconciliation
        ↓
server authoritative payment state
```

A duplicate webhook must not create another payment allocation.

---

# 234. End-to-End Example — EIS Failure

```text
offline sale
 ↓
sale sync accepted
 ↓
outbox event
 ↓
EIS worker
 ↓
provider unavailable
 ↓
TAX_PENDING / RETRY
```

Sale financial facts remain unchanged.

---

# 235. End-to-End Example — Corrupt Local DB

```text
SQLite corruption detected
 ↓
stop normal sync
 ↓
preserve command journal if recoverable
 ↓
revoke device when compromise plausible
 ↓
new authenticated device/session
 ↓
authoritative bootstrap
 ↓
reconcile recoverable commands
 ↓
resume
```

---

# 236. Security Test Gate

A sync release MUST fail when:

- a required sync suite did not execute;
- a tenant-negative test fails;
- duplicate financial processing is possible;
- a revoked device is accepted;
- checkpoint advancement can lose data;
- schema validation can be bypassed;
- malformed input causes unsafe processing;
- critical resource limits are absent.

A missing or skipped test is not a pass.

---

# 237. CI Commands — Conceptual Set

The implementation repository should expose equivalent commands for:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --test sync_* 
cargo test --test security_* 
cargo test --test concurrency_*
property-test-suite
fuzz-smoke-suite
secret-scan
dependency-audit
SBOM generation
API contract/security tests
```

The exact tooling can evolve while the control objectives remain mandatory.

---

# 238. Performance Testing

Measure:

```text
commands/sec
batch latency
P95/P99 server processing
SQLite commit latency
checkpoint application latency
memory usage
CPU usage
battery impact
payload bandwidth
```

Use realistic merchant workloads rather than one synthetic tenant generating all traffic.

---

# 239. Low-End Device Performance

Test synchronization on representative lower-cost Android hardware.

Measure:

```text
cold start
SQLite open time
journal write time
sync time
memory
battery
storage
```

The target market makes these first-order product requirements.

---

# 240. Offline Duration Testing

Test:

```text
1 hour offline
1 day offline
3 days offline
7 days offline
longer periods according to realistic merchant behavior
```

The protocol must remain bounded and recoverable.

---

# 241. Reordering Tests

Generate command sequences where network arrival order differs from creation order.

Verify that:

```text
explicit dependencies
```

are respected.

---

# 242. Duplicate Burst Tests

Send the same command:

```text
10 times
100 times
1000 times
```

Expected:

```text
one business effect
```

and controlled resource consumption.

---

# 243. Tenant Flood Tests

Simulate one device/tenant generating a huge command stream while many other tenants are active.

Expected:

```text
no platform-wide starvation
```

---

# 244. Projection Convergence Test

After all accepted commands and deltas have been processed:

```text
client projection
```

must match the server-authoritative read model for the client's scope.

Any known eventual-consistency exceptions must be documented.

---

# 245. Reconciliation Invariant

After convergence:

```text
server accepted commands
```

must correspond to:

```text
authoritative business events / state transitions
```

The count may differ for non-business projection events, but every financial/inventory command must have a traceable authoritative outcome.

---

# 246. No Orphaned Financial Commands

A finalized financial command must not end in:

```text
unknown
```

without a recoverable classification.

Terminal state options include:

```text
accepted
rejected
conflict
quarantined
```

according to policy.

---

# 247. Idempotency Retention

The server must retain idempotency protection long enough to cover the realistic replay window for the command's business effect.

For permanent financial operations, durable business uniqueness should provide an additional layer.

Do not treat idempotency records as short-lived cache entries when deletion could re-enable duplicate effects.

---

# 248. Business Uniqueness vs Command Identity

A command identity protects transport/application replay.

Business uniqueness protects business semantics.

Example:

```text
command_id = C1
sale_number = SALE-000100
provider_event_id = P1
```

Each may require independent uniqueness constraints.

---

# 249. Temporary Local IDs

The client may generate local IDs for UI continuity:

```text
local_sale_id
```

The server may later assign:

```text
server_sale_id
server_sale_number
```

The mapping must be durable:

```text
local_sale_id
   ↓
command_id
   ↓
server_sale_id
```

---

# 250. Offline Sale Numbering

Global receipt/sequence numbering needs a separate business/regulatory design.

Never assume that independent offline devices can safely generate one global sequential sequence using only local counters.

Possible strategies include:

- server-preallocated blocks;
- globally unique IDs plus display sequence;
- device/branch-qualified local sequence;
- provider/regulatory-specific mechanisms.

The final strategy must be compatible with current legal and EIS requirements.

---

# 251. Receipt State

A local receipt may need to indicate:

```text
LOCAL
SYNC_PENDING
SERVER_CONFIRMED
TAX_PENDING
TAX_ACCEPTED
```

as appropriate.

Do not imply tax acceptance merely because a local receipt was printed.

---

# 252. Offline Tax Claims

The system must never make a legal/compliance claim solely because:

```text
offline receipt generated
```

Actual certification and provider behavior govern.

---

# 253. Sync and Reporting

Reports generated from partially synchronized data must communicate their status.

Example:

```text
Branch report
Last synchronized: 14:32
Pending device events: 7
```

Analytics must not silently present an incomplete data window as complete truth.

---

# 254. Sync and Exports

Exports should use authoritative server data where possible.

A local offline export can exist only where product policy explicitly defines its semantics.

Bulk export is an exfiltration capability and remains permission-scoped.

---

# 255. Sync and Search

Search projections are eventually consistent.

A newly synchronized sale may appear in the transactional source before appearing in a search index.

This must not affect financial authority.

---

# 256. Sync and Cache

Cache is derived data.

The client must invalidate or refresh cached values when the server sends authoritative updates that affect them.

Cached authorization state requires especially strict invalidation.

---

# 257. Sync and RLS

If PostgreSQL Row-Level Security is used, sync transactions must operate under an application runtime role that cannot unexpectedly bypass RLS.

Tenant context must be established safely before accessing tenant-owned rows.

---

# 258. Sync Database Tables — Logical

The database design should include logical persistence for:

```text
sync_devices
sync_commands
sync_command_results
sync_conflicts
sync_checkpoints / device_cursors
sync_consumed_events
sync_stream_metadata
```

Exact physical names belong in `database_design.md`.

---

# 259. Sync Command Record

Conceptual fields:

```text
command_id
organization_id
device_id
actor_user_id
command_type
schema_version
client_sequence
payload_digest
received_at
processing_state
result_reference
reason_code
accepted_at
created_at_client
```

Do not store arbitrary secret-bearing payloads simply for convenience.

---

# 260. Sync Conflict Record

Conceptual fields:

```text
conflict_id
command_id
entity_type
entity_id
category
state
created_at
detected_at
resolved_at
resolved_by
resolution_code
evidence_reference
```

---

# 261. Checkpoint Record

Conceptual fields:

```text
device_id
scope
cursor
schema_version
updated_at
```

A checkpoint cannot be moved backward without an explicit recovery procedure.

---

# 262. Delta Application Receipt

The local client should record enough data to make event application idempotent.

Conceptually:

```text
event_id
cursor
applied_at
```

The actual local schema may combine receipts with projection transactions.

---

# 263. Bootstrap Integrity

A snapshot must be validated before becoming the base state for synchronization.

Checks can include:

```text
schema version
scope
checksum
cursor consistency
row count sanity
```

---

# 264. Security Side Channels

Synchronization can leak information through:

```text
different errors
different response sizes
command result timing
conflict counts
cursor behavior
```

Avoid designs where an unauthorized user can infer the existence of another tenant's object merely by probing IDs.

---

# 265. Enumeration Resistance

Do not return:

```text
“command exists but belongs to Tenant B.”
```

Prefer generic authorization-safe semantics.

---

# 266. Support Tool Access

Support tools must not silently modify or replay offline commands.

A support operator may need controlled capabilities:

```text
view status
inspect evidence
resolve conflict
replay with approval
```

High-risk actions require explicit permission and audit.

---

# 267. Break-Glass Sync Access

Break-glass access is exceptional.

Use:

```text
MFA
approval
scoped access
time limit
audit
```

Do not create a permanent hidden synchronization backdoor for operators.

---

# 268. Incident Response — Sync Compromise

If a sync vulnerability is discovered:

```text
detect
→ classify impact
→ disable affected command class/path
→ revoke affected devices/credentials
→ preserve command/audit data
→ identify affected tenants
→ reconcile authoritative truth
→ deploy fix
→ rerun security regression suite
→ reopen safely
```

---

# 269. Incident Response — Suspected Duplicate Financial Effects

```text
freeze affected operation
→ identify command IDs
→ inspect idempotency records
→ compare ledger state
→ inspect audit/outbox
→ determine provider effects
→ reconcile
→ preserve evidence
→ patch and regression-test
```

Do not “fix” duplicates through arbitrary database updates that destroy history.

---

# 270. Incident Response — Queue Flood

```text
identify device/tenant
→ throttle
→ revoke if malicious
→ isolate command class if necessary
→ preserve forensic records
→ restore service capacity
```

---

# 271. Operational SLO Candidates

The exact SLOs are established after real load data, but candidate measures include:

```text
P95 sync latency
P99 sync latency
oldest pending command age
conflict rate
rejection rate
retry rate
checkpoint lag
```

A system can be technically “available” while merchant queues remain permanently stuck.

---

# 272. Performance Budgets

Define budgets for:

```text
request CPU
request memory
DB connection time
DB transaction time
batch size
command count
payload size
queue depth
retry volume
```

---

# 273. Memory Safety

Rust provides a strong memory-safety foundation, but synchronization can still trigger application-level resource exhaustion through huge payloads, dependency graphs or giant queues.

Rust does not remove the need for explicit resource limits.

---

# 274. CPU Safety

Avoid expensive server operations on unbounded synchronization input.

Examples:

- arbitrary regex evaluation;
- huge dependency graphs;
- expensive canonicalization on enormous payloads;
- decompression bombs.

---

# 275. Storage Safety

A device should not be allowed to accumulate unlimited failed commands.

When a queue approaches a defined ceiling:

```text
warn
→ prioritize
→ stop low-value work
→ recover/resync
```

Critical commands must never be silently discarded.

---

# 276. Network Efficiency

Use:

- compact command payloads;
- delta synchronization;
- pagination;
- compression where useful;
- bounded retries.

Do not download unchanged catalogue/inventory data repeatedly.

---

# 277. Payload Projection

The server should return only the fields the client needs for the requested synchronization scope.

Do not include:

```text
internal secrets
support metadata
unrelated financial data
```

---

# 278. Tenant-Scoped Deltas

All server deltas are evaluated against the current scope.

If a user loses access to a branch, the server may need to emit removal/tombstone information or require a scoped refresh so the client no longer treats old data as currently authorized.

---

# 279. Tombstones

Tombstones can communicate that an object is no longer part of an active projection.

Example:

```text
SKU X ARCHIVED
```

A tombstone does not necessarily expose why or reveal confidential historical information.

---

# 280. Tombstone Retention

Tombstones must live long enough for expected offline clients.

When history is too old, force a resync instead of retaining tombstones forever.

---

# 281. Synchronization Contract for Financial Data

The server must be able to prove:

```text
command
→ accepted business event
→ financial record
→ inventory effect if applicable
→ payment linkage if applicable
→ audit
```

This is part of Sitolo's economic correctness moat.

---

# 282. Synchronization Contract for Inventory

The server must be able to prove:

```text
command
→ inventory movement
→ authoritative balance/projection
→ source transaction
→ actor/device
```

An inventory balance without explainable movement history is insufficient for the target architecture.

---

# 283. Synchronization Contract for Cash

The server must preserve:

```text
register session
cash command
actor/device
cash movement
closing/reconciliation
variance
```

---

# 284. Synchronization Contract for Audit

Audit records should identify:

```text
what
who
where
when
which device
which command
what result
```

without leaking unnecessary sensitive payload data.

---

# 285. Synchronization Contract for Privacy

The local operational store is subject to the same data-classification model as server storage.

The fact that data is on a phone does not make it non-sensitive.

---

# 286. Synchronization Contract for Compliance

The synchronization protocol supports compliance evidence but does not itself certify compliance.

Tax and regulated-domain behavior must be verified against current authority requirements.

---

# 287. Synchronization and Pharmacy

Pharmacy extensions may require stricter handling of:

- expiry;
- lot/batch;
- quarantine;
- recalls;
- authorized staff;
- dispensing evidence.

Offline availability must not bypass those controls.

---

# 288. Synchronization and Agro-Dealer

Agro workflows may require:

- batch/formulation data;
- unit conversion;
- expiry/recommended use;
- supplier traceability.

The core sync engine remains generic while command semantics remain domain-specific.

---

# 289. Synchronization and Wholesale

Wholesale may introduce:

- bulk quantities;
- customer-specific pricing;
- larger credit flows;
- larger inventory adjustments.

These should use the same command/idempotency/security framework with different domain policies.

---

# 290. No Direct Local SQL Authority

The client should never communicate:

```text
UPDATE inventory SET quantity = ...
```

as a synchronization operation.

It communicates intent:

```text
AdjustStock
```

The server decides the actual state transition.

---

# 291. No Arbitrary Remote Procedure Calls

The command registry must be closed and typed.

Avoid a generic command payload such as:

```json
{
  "function": "delete_anything",
  "args": {}
}
```

Command types are explicit and mapped to approved domain services.

---

# 292. Rust Command Dispatcher

Conceptual structure:

```text
SyncHandler
   ↓
SyncApplicationService
   ↓
CommandDispatcher
   ↓
Typed domain handler
   ↓
Repository/application service
   ↓
PostgreSQL transaction
```

The sync layer is not a back door around domain logic.

---

# 293. Typed Sync Context

Rust should use types such as:

```text
SyncSessionId
CommandId
DeviceId
OrganizationId
BranchId
SyncCursor
ClientSequence
CapabilityVersion
ConflictId
```

Do not treat every identifier as an interchangeable `String`.

---

# 294. Typed Command Payloads

Each command type should deserialize into an explicit DTO before conversion into domain values.

This prevents arbitrary JSON from becoming persistence state.

---

# 295. Domain Constructors

High-risk domain values should reject impossible states.

Examples:

```text
Money
Quantity
DiscountRate
RefundAmount
UnitConversion
```

This adds defense against malformed synchronization payloads.

---

# 296. No Floating-Point Financial Values

Financial commands must use exact monetary representation.

A floating-point client value must not be trusted as financial truth.

---

# 297. No Client-Controlled Tenant Authority

The following is not accepted as proof:

```json
{"organization_id":"victim"}
```

The server derives authority from authenticated membership/device scope.

---

# 298. No Client-Controlled Approval Authority

The following is not authority:

```json
{"approved_by":"owner"}
```

The authenticated approver must actually possess the required permission and scope.

---

# 299. No Client-Controlled Provider Settlement

The client cannot synchronize:

```json
{"payment_status":"PAID"}
```

into authoritative settlement without provider/domain evidence.

---

# 300. No Client-Controlled Tax Acceptance

Likewise:

```json
{"tax_status":"ACCEPTED"}
```

is only a claim unless supported by verified EIS evidence.

---

# 301. Sync and Reconciliation

Reconciliation is the mechanism for ambiguous external states.

Example:

```text
provider says transaction P1
Sitolo says PaymentIntent I1
sale = S1
```

The reconciliation system links these using provider IDs and defined matching rules.

Synchronization must not bypass reconciliation simply because a client supplied a provider reference.

---

# 302. Ambiguous External State

When evidence conflicts:

```text
preserve evidence
mark exception
stop unsafe automatic state change
```

Do not “pick whichever update arrived last.”

---

# 303. Reconciliation Queue

Unresolved synchronization/payment discrepancies can create:

```text
ReconciliationCase
```

with:

- references;
- evidence;
- reason;
- state;
- owner;
- resolution.

---

# 304. Sync and Incident Evidence

Never delete an unusual command simply because it looks malicious.

Preserve:

```text
command_id
actor/device
scope
received_at
reason
security event
```

according to retention policy.

---

# 305. Privacy vs Forensics

The system should retain enough evidence for security investigations without logging entire payloads unnecessarily.

Prefer structured identifiers and reason codes.

---

# 306. Queue Encryption

If local database encryption is used, the encryption key belongs in OS secure storage.

The protocol must not store the key next to the SQLite database.

---

# 307. Rooted/Jailbroken Device

The protocol assumes that a hostile operator may control the endpoint.

Controls focus on:

```text
minimal local data
bounded offline authority
short-lived capabilities where needed
server revalidation
device revocation
```

Do not claim that endpoint attestation eliminates all endpoint compromise risk.

---

# 308. Device Security Signals

Where supported, the client can expose security signals such as:

```text
OS version
app version
installation integrity signal
root/jailbreak signal
```

These are signals for policy, not unquestionable truth.

---

# 309. App Integrity

The application may use platform integrity mechanisms where justified.

The protocol must still assume that a sufficiently compromised endpoint can forge ordinary application behavior.

---

# 310. Offline Authorization Expiry

A device capability can require online refresh after a defined period.

When it expires:

```text
ordinary low-risk work may continue only if policy permits
high-risk work denied
sync refresh required
```

---

# 311. Security Version Invalidation

A tenant/device security version can force immediate capability refresh after:

- role change;
- device revocation;
- policy change;
- incident response.

---

# 312. Sync and Branch Deactivation

If a branch becomes disabled:

```text
new branch commands
→ deny
```

Pending commands are handled according to domain-specific historical policy.

They are never accepted solely because the device still has cached branch data.

---

# 313. Sync and Organization Suspension

If the organization account is suspended:

server determines whether:

```text
read-only sync

or

no sync
```

is allowed.

Business/legal/security reasons must be represented explicitly.

---

# 314. Sync and Subscription Suspension

Subscription state can affect entitlements but must not corrupt historical merchant data.

For example:

```text
subscription suspended
→ new premium commands denied
→ historical financial records remain intact
```

---

# 315. Safe Subscription Downgrade

A device may hold cached features from a higher plan.

Server entitlement enforcement prevents cached UI capabilities from becoming unauthorized server commands.

---

# 316. Synchronization Audit Query

Support/security tooling should allow authorized operators to answer:

```text
What commands were pending for device D?
Which were accepted?
Which were rejected?
Which conflicted?
Which were retried?
```

without requiring raw database access.

---

# 317. No Generic Admin SQL Repair

Operators must not “repair” synchronization by directly editing business tables as a normal support workflow.

Use explicit recovery tools that create auditable compensating actions.

---

# 318. Sync Repair Tools

Repair tools should be command-oriented:

```text
RebuildProjection
ReissueCapability
RevokeDevice
ResolveConflict
ReconcilePayment
RequestResync
```

rather than arbitrary SQL.

---

# 319. Production Deployment Gate

Before a synchronization change reaches production:

```text
[ ] protocol compatibility checked
[ ] command registry reviewed
[ ] migrations backward-compatible
[ ] duplicate tests pass
[ ] cross-tenant tests pass
[ ] revocation tests pass
[ ] checkpoint tests pass
[ ] concurrency tests pass
[ ] fuzz smoke pass
[ ] low-end device validation pass
[ ] observability verified
[ ] rollback tested
```

---

# 320. Rollback Considerations

Rolling a server version backward can be dangerous if it cannot understand commands already accepted by the newer version.

Therefore rollback must consider:

```text
protocol version
command schema
stored command results
outbox events
client checkpoint
client app versions
```

Database rollback is not automatically safe merely because application deployment rollback is possible.

---

# 321. Forward-Compatible Design

Prefer additive schema changes where feasible.

For command schemas:

```text
add field
server tolerates during compatibility window
client rollout
remove old field later
```

Destructive changes require migration planning.

---

# 322. Protocol Deprecation

Deprecation requires:

- telemetry;
- communicated deadline where necessary;
- upgrade path;
- supported migration;
- safe rejection behavior;
- recovery for abandoned devices.

---

# 323. Device Fleet Upgrade Monitoring

Track:

```text
client version distribution
protocol version distribution
stale devices
unsupported devices
```

Do not deprecate a protocol blindly while a meaningful production fleet still depends on it.

---

# 324. Sync Documentation Contract

Every command definition must state:

```text
name
purpose
actor
scope
online/offline policy
payload
size limits
preconditions
idempotency
retry behavior
conflict behavior
result
emitted events
audit
```

---

# 325. Command Registry Example

| Command | Offline | Risk | Conflict | Idempotent |
|---|---|---|---|---|
| FinalizeSale | bounded | medium | yes | yes |
| AdjustStock | limited | high | yes | yes |
| CloseRegister | limited | high | yes | yes |
| RequestRefund | restricted | high | yes | yes |
| ChangeRole | no | critical | yes | yes |
| ChangePayout | no | critical | yes | yes |
| ReceiveGoods | policy | medium | yes | yes |
| SubmitTax | adapter-specific | high | yes | yes |

This registry is an implementation contract, not a UI menu.

---

# 326. Security Review of New Command

Every new command requires review of:

```text
identity
scope
permission
offline eligibility
resource limits
idempotency
state machine
financial effects
inventory effects
external effects
audit
conflict behavior
rollback
security tests
```

---

# 327. New Command “Definition of Ready”

A command may enter implementation only when:

```text
[ ] domain purpose defined
[ ] actor defined
[ ] scope defined
[ ] state transitions defined
[ ] invariant defined
[ ] offline policy defined
[ ] payload schema defined
[ ] limits defined
[ ] idempotency defined
[ ] retry behavior defined
[ ] conflict behavior defined
[ ] audit defined
[ ] tests specified
```

---

# 328. New Command “Definition of Done”

```text
[ ] Rust domain implementation
[ ] persistence implementation
[ ] API implementation
[ ] sync implementation
[ ] unit tests
[ ] integration tests
[ ] security negatives
[ ] concurrency tests if relevant
[ ] observability
[ ] documentation
[ ] rollback/migration plan
```

---

# 329. Integration Contract — Payments

The payment subsystem consumes synchronized sales and produces independent provider state.

The sync protocol supplies stable references; the payment subsystem owns provider reconciliation.

---

# 330. Integration Contract — MRA EIS

EIS integration consumes accepted business facts and manages external submission state.

The synchronization protocol preserves local/server sale continuity even while EIS is unavailable, subject to the applicable current regulatory contract.

---

# 331. Integration Contract — Notifications

Notifications are eventual side effects.

A notification failure does not generally invalidate an otherwise committed sale.

---

# 332. Integration Contract — Object Storage

Receipt/document uploads referenced by offline commands use object IDs or controlled storage references rather than arbitrary client filesystem paths.

---

# 333. Integration Contract — Search

Search/index updates are derived asynchronous effects.

A search lag must not alter the authoritative transaction state.

---

# 334. Integration Contract — Reporting

Reports may lag synchronization and must expose appropriate freshness semantics.

---

# 335. Integration Contract — Audit

Every command with business significance must produce adequate evidence to reconstruct what happened.

---

# 336. Threat Model — Stolen Device

Attack:

```text
device stolen
→ attacker replays commands
```

Controls:

```text
device revocation
session revocation
bounded offline capability
command identity
server authorization
```

---

# 337. Threat Model — Modified Local Database

Attack:

```text
stock / price / role modified locally
```

Controls:

```text
server authority
integrity signals
bounded local scope
server revalidation
```

---

# 338. Threat Model — Replay Flood

Attack:

```text
same valid command replayed thousands of times
```

Controls:

```text
idempotency
unique constraints
rate limits
concurrency limits
```

---

# 339. Threat Model — Cross-Tenant Command

Attack:

```text
valid command identity
+ victim tenant ID
```

Controls:

```text
authenticated tenant membership
server-derived scope
repository tenant predicates
RLS where justified
negative tests
```

---

# 340. Threat Model — Offline Privilege Escalation

Attack:

```text
edit local role → OWNER
```

Controls:

```text
server principal
membership
capability profile
authorization
```

---

# 341. Threat Model — Clock Manipulation

Attack:

```text
set device clock backward/forward
```

Controls:

```text
server time
bounded capability validity
server acceptance timestamps
```

---

# 342. Threat Model — Queue Exhaustion

Attack:

```text
million commands generated offline
```

Controls:

```text
local queue limits
server batch limits
device quotas
tenant quotas
command-type limits
```

---

# 343. Threat Model — Dependency Cycle

Attack/bug:

```text
A depends B
B depends A
```

Controls:

```text
dependency graph bounds
cycle detection
quarantine
```

---

# 344. Threat Model — Checkpoint Loss

Bug:

```text
checkpoint advanced before local commit
```

Control:

```text
single local transaction
```

with checkpoint update committed only after delta application.

---

# 345. Threat Model — Server Rollback

Bug:

```text
database restored
idempotency records lost
```

Control:

```text
backup/restore includes sync control plane
restore verification
```

---

# 346. Threat Model — Schema Confusion

Attack:

```text
new command encoded to look like old command
```

Controls:

```text
explicit version
strict schema
canonicalization
compatibility matrix
```

---

# 347. Threat Model — Support Abuse

Attack:

```text
support agent replays/changes commands silently
```

Controls:

```text
JIT access
least privilege
approval
audit
explicit repair commands
```

---

# 348. Threat Model — Malicious Client

Assume the client can fully manipulate its own:

- code;
- local database;
- timestamps;
- request payloads.

The security boundary is the server.

---

# 349. Security Regression Mapping

This protocol directly contributes controls for:

```text
input validation
IDOR/BOLA
authorization
client-only security
rate limits
race conditions
replay
business logic abuse
sensitive browser/local storage
insecure endpoints
missing timeouts
payment tampering
```

It inherits the full 48-control security baseline from `security_architecture_design.md`.

---

# 350. Production Gate — Offline Business Continuity

Before claiming offline readiness:

```text
[ ] merchant can sell without network
[ ] command survives process restart
[ ] command survives app update
[ ] network return resumes sync
[ ] duplicate transmission is harmless
[ ] response loss after commit is harmless
[ ] conflicts are explicit
[ ] revoked devices stop synchronizing
[ ] local tampering cannot create server authority
[ ] large backlog is bounded
```

---

# 351. Production Gate — Financial Integrity

```text
[ ] no duplicate sale
[ ] no duplicate refund
[ ] no over-refund
[ ] no stock fabrication
[ ] financial corrections append-only
[ ] payment confirmation server-authoritative
[ ] audit reconstruction works
```

---

# 352. Production Gate — Security

```text
[ ] cross-tenant negative tests pass
[ ] cross-branch negative tests pass
[ ] revoked-device tests pass
[ ] replay tests pass
[ ] tamper tests pass
[ ] checkpoint tests pass
[ ] fuzz smoke passes
[ ] rate/size limits pass
[ ] timeout tests pass
[ ] fail-closed behavior verified
```

---

# 353. Production Gate — Operational Recovery

```text
[ ] resync works
[ ] queue recovery works
[ ] checkpoint recovery works
[ ] device replacement works
[ ] database restore preserves idempotency
[ ] outbox recovery works
[ ] incident runbooks tested
```

---

# 354. Final System Contract

Sitolo's synchronization system is formally defined as:

> **A client-side durable command journal that preserves merchant intent while disconnected, combined with a server-authoritative synchronization processor that revalidates identity, device state, tenant scope, offline capability, authorization, object ownership, property permissions, domain state, financial/inventory invariants, idempotency and resource limits before committing business truth. PostgreSQL commits authoritative state, command outcome, audit and outbox information transactionally. The client advances local checkpoints only after safe durable application. Duplicate commands are harmless, timeout-after-commit retries use the original command identity, conflicts are explicit, rejected work is preserved as evidence, device revocation is enforceable, protocol evolution is versioned, and no client-local state becomes global business authority.**

---

# 355. Engineering North Star

```text
NETWORK FAILS
      ↓
BUSINESS CONTINUES
      ↓
LOCAL WORK SURVIVES CRASH
      ↓
NETWORK RETURNS
      ↓
SAME COMMANDS RETRY
      ↓
SERVER REVALIDATES
      ↓
DATABASE COMMITS ONCE
      ↓
DELTAS RETURN
      ↓
LOCAL STATE CONVERGES
      ↓
MONEY + STOCK RECONCILE
      ↓
AUDIT EXPLAINS WHAT HAPPENED
```

The protocol is successful when this remains true under ordinary merchant conditions, malicious client behavior, provider outages, duplicate delivery, process crashes and concurrent operations.

---

# 356. Implementation Readiness Checklist

```text
ARCHITECTURE
[ ] command-based sync chosen
[ ] server authority preserved
[ ] no LWW for critical domains

LOCAL
[ ] SQLite schema designed
[ ] durable command journal
[ ] secure storage
[ ] local data minimization
[ ] queue limits

IDENTITY
[ ] session binding
[ ] device identity
[ ] device revocation
[ ] capability profiles

COMMANDS
[ ] command IDs
[ ] idempotency
[ ] fingerprints
[ ] schema versions
[ ] client sequence
[ ] causality metadata

SERVER
[ ] sync handshake
[ ] batch endpoint
[ ] delta endpoint
[ ] checkpoint handling
[ ] resync
[ ] command dispatcher

SECURITY
[ ] tenant scope
[ ] branch scope
[ ] object authorization
[ ] property authorization
[ ] replay protection
[ ] rate limits
[ ] payload limits
[ ] timeout limits
[ ] fail-closed behavior

BUSINESS
[ ] sales
[ ] inventory
[ ] cash
[ ] returns/refunds
[ ] payments
[ ] tax/EIS
[ ] approvals

RECOVERY
[ ] app restart
[ ] process kill
[ ] timeout-after-commit
[ ] device replacement
[ ] database restore
[ ] queue poison
[ ] checkpoint corruption

TESTING
[ ] unit
[ ] property
[ ] integration
[ ] real PostgreSQL
[ ] concurrency
[ ] fuzz
[ ] network chaos
[ ] low-end Android
[ ] security negatives
[ ] CI release gates
```

---

# 357. References and Research Basis

The implementation decisions in this specification are grounded in the existing Sitolo architecture and product documents and supplemented by current technical documentation for the components used by the design.

Primary external technical references:

1. SQLite — Atomic Commit: https://www.sqlite.org/atomiccommit.html
2. SQLite — Write-Ahead Logging: https://www.sqlite.org/wal.html
3. SQLite — PRAGMA / synchronous behavior: https://sqlite.org/pragma.html
4. Flutter architecture/offline-first guidance: https://docs.flutter.dev/app-architecture
5. PostgreSQL documentation: https://www.postgresql.org/docs/current/
6. OAuth 2.0 Security Best Current Practice, RFC 9700: https://www.rfc-editor.org/rfc/rfc9700.html
7. OAuth 2.0 for Native Apps, RFC 8252: https://www.rfc-editor.org/rfc/rfc8252.html
8. HTTP Semantics, RFC 9110: https://www.rfc-editor.org/rfc/rfc9110.html
9. OWASP API Security: https://owasp.org/API-Security/
10. OWASP Application Security Verification Standard: https://owasp.org/www-project-application-security-verification-standard/

Internal sources governing this file:

- `sitolo.md`
- `business_model_design.md`
- `system_architecture_design.md`
- `security_architecture_design.md`
- `security_implementation_spec.md`
- `domain_model.md`
- `database_design.md`
- `api_contract.md`
- `auth_authorization_spec.md`

---

# 358. Document Governance

This document must be versioned with the code.

Changes that materially alter any of the following require architecture/security review:

```text
offline command authority
command identity
idempotency
checkpoint semantics
conflict resolution
financial offline behavior
inventory offline behavior
device revocation
capability lifetime
protocol compatibility
local sensitive-data retention
```

Ordinary implementation refactoring is allowed when all documented invariants remain unchanged.

**End of `sync_protocol.md`.**
