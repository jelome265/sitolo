# SITOLO — PHASE 4 TENANT / ORGANIZATION / BRANCH / IAM IMPLEMENTATION SPECIFICATION

**Document:** `phase4_tenant_organization_branch_iam_implementation.md`  
**Phase:** Phase 4 — Tenant / Organization / Branch / IAM  
**Product:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first; controlled African expansion  
**Backend:** Rust + Axum + Tokio  
**Persistence:** PostgreSQL authoritative; SQLite client continuity  
**Clients:** Flutter Android-first; Tauri desktop; TypeScript web/admin where materially useful  
**Architecture:** Modular monolith first; durable workers; explicit integration adapters  
**Security posture:** Zero trust; deny by default; least privilege; server authoritative; tenant isolation; fail closed  
**Status:** Implementation-governing specification  
**Baseline date:** 6 September 2026  

> This document defines the implementation contract for Sitolo's organizational tenancy and IAM layer. It does not replace the existing authentication, authorization, domain, database, API, security, sync, payment, EIS, testing, observability or deployment specifications. It converts those contracts into a concrete implementation boundary for Phase 4.

---

# 0. EXECUTIVE DECISION

Sitolo shall implement a **hierarchical, multi-tenant organizational control plane** in which a user identity, an organization membership, an organizational scope, a branch assignment, a device association, an explicit permission, and a domain operation remain separate concepts.

The canonical authority chain is:

```text
AUTHENTICATED PRINCIPAL
        |
        v
ACTIVE MEMBERSHIP
        |
        v
ORGANIZATION
        |
        +-------------------------+
        |                         |
        v                         v
     BRANCH                    ORG-WIDE
        |
        +-------------------+
        |                   |
        v                   v
   WAREHOUSE             REGISTER
        |
        v
      DEVICE

membership + role + scope + permission + target + state + assurance
                           |
                           v
                    IAM DECISION
                           |
                           v
                   APPLICATION COMMAND
                           |
                           v
                    DOMAIN INVARIANT
                           |
                           v
                      TRANSACTION
```

The most important implementation rule is:

> **A tenant identifier is a selector, never proof of authority. A role is a vocabulary, never the complete authorization decision. A branch is a scope boundary, never merely a UI filter. IAM exists to establish the organizational authority under which domain operations execute.**

This phase therefore owns **organizational identity and authority topology**, not every authorization rule in the platform. Phase 6 will build the generalized policy/authorization engine over this foundation.

---

# 1. RELATIONSHIP TO THE EXISTING SITOLO CONTRACTS

## 1.1 Existing documents are authoritative inputs

Phase 4 derives from the already-existing Sitolo package:

```text
business_model_design.md
        |
sitolo.md
        |
system_architecture_design.md
        |
security_architecture_design.md
        |
security_implementation_spec.md
        |
domain_model.md
        |
database_design.md
        |
api_contract.md
        |
auth_authorization_spec.md
        |
phase2_config_secrets_logging_errors_telemetry_implementation.md
        |
phase3_identity_sessions_mfa_device_identity_implementation.md
        |
                ↓
     PHASE 4 TENANT / IAM
```

The existing contracts explicitly establish that Sitolo is multi-tenant, multi-branch, server-authoritative, and that membership/scope must be resolved independently of raw client claims. The API contract also requires every object identifier to be checked against trusted organizational scope, while the authentication contract distinguishes `User`, `Membership`, `Session`, and `Device`. fileciteturn6file3L500-L542 fileciteturn4file1L1086-L1127

The database contract defines PostgreSQL as authoritative, with tenant scope and branch scope represented inside the persistence model and protected by constraints and defense-in-depth RLS. fileciteturn3file7L786-L830

## 1.2 What Phase 4 adds

Phase 4 adds implementation depth around:

- organization lifecycle;
- business entity representation;
- branch lifecycle;
- branch/location/warehouse/register relationships;
- membership lifecycle;
- role assignment lifecycle;
- scope grants;
- invitations and enrollment;
- membership suspension/revocation;
- organization switching;
- delegated administration;
- ownership transfer;
- branch manager/cashier scope;
- device-to-organization association;
- service identity scope;
- organization-level and branch-level invariants;
- IAM transaction boundaries;
- concurrency handling;
- cache invalidation/versioning;
- IAM audit evidence;
- IAM-specific telemetry;
- tenant-aware repository APIs;
- negative security testing;
- migration strategy;
- operational runbooks.

## 1.3 What Phase 4 does not redefine

Phase 4 does not redefine:

- authentication protocol;
- password hashing;
- MFA cryptography;
- refresh token semantics;
- public HTTP error shape;
- generic policy-expression language;
- sales rules;
- inventory rules;
- payment provider semantics;
- MRA EIS protocol;
- synchronization protocol;
- billing entitlements;
- legal tax identities;
- production hosting topology.

Where Phase 4 requires a future policy-engine decision, that decision remains a Phase 6 concern and must not be prematurely embedded into the organization schema.

---

# 2. BUSINESS PURPOSE OF THE TENANT MODEL

Sitolo is not a generic SaaS database with an `organization_id` column added to tables. The tenant model represents a real merchant business operating across people, locations, stock points, registers and devices.

The minimum business relationship is:

```text
ONE PERSON
   |
   +---- may belong to many organizations
   |
   +---- may use many devices

ONE ORGANIZATION
   |
   +---- may have many branches
   |
   +---- has members
   |
   +---- owns catalogue/pricing/business configuration
   |
   +---- owns operational records

ONE BRANCH
   |
   +---- belongs to exactly one organization
   |
   +---- may contain location metadata
   |
   +---- may contain warehouses
   |
   +---- may contain registers
   |
   +---- may contain devices
   |
   +---- may have scoped staff authority
```

This topology directly supports the Sitolo operating loop:

```text
PROCURE
   ↓
RECEIVE
   ↓
STOCK
   ↓
PRICE
   ↓
SELL
   ↓
COLLECT
   ↓
RECONCILE
   ↓
REPORT
```

Organizational scope determines **where** each operation is allowed to occur, but does not independently determine **whether** the operation is economically valid. For example:

```text
Cashier may be authorized for Branch A
        |
        +--> sale belongs to Branch A
        |
        +--> stock must exist for Branch A
        |
        +--> register must be valid
        |
        +--> device must be active
        |
        +--> sale state must permit finalization
        |
        +--> transaction must pass domain invariants
```

IAM is therefore a prerequisite, not a replacement, for domain correctness.

---

# 3. TERMINOLOGY — CANONICAL VOCABULARY

The following terms are normative.

## 3.1 User

A globally identifiable human account represented by the identity subsystem.

A user can exist without belonging to any organization.

```text
User != Organization
User != Membership
User != Session
User != Device
```

## 3.2 Organization

The top-level tenant boundary for merchant business data and business authority.

An organization owns or controls business resources and memberships within the Sitolo platform.

## 3.3 Business Entity

The legal/commercial identity represented within an organization where the business model needs a distinction between the platform tenant and an operating/legal business entity.

This distinction exists because one organization can later support more complex structures without changing the tenant boundary.

## 3.4 Branch

An operational subdivision of an organization.

A branch has a defined business scope and is the primary organizational boundary used for branch-specific stock, sales, cash, devices, registers and personnel authority.

## 3.5 Location

Physical or logical location metadata associated with a branch or operational facility.

A location is not itself an authorization principal.

## 3.6 Warehouse

An inventory holding point under an organization, generally scoped to a branch or explicitly governed as an organization-level facility.

## 3.7 Register

A point-of-sale operational station within a branch.

## 3.8 Device

A separately managed client identity introduced by Phase 3. Devices are security subjects and can be independently revoked.

## 3.9 Membership

The explicit relationship between a user and an organization.

Membership is the foundation from which business authority is derived.

## 3.10 Role

A named bundle of permissions, such as `OWNER`, `MANAGER`, `CASHIER`, or `INVENTORY_CLERK`.

Roles are not complete authorization decisions.

## 3.11 Permission

An atomic operation capability, such as `SALE_CREATE`, `REFUND_APPROVE`, or `ROLE_ASSIGN`.

## 3.12 Scope Grant

A restriction or grant specifying where a permission applies, such as organization-wide, branch-specific, warehouse-specific or register-specific scope.

## 3.13 Invitation

A controlled enrollment object allowing a person to become a member of an organization under proposed role and scope terms.

## 3.14 Ownership

A high-privilege organizational relationship that defines who has authority to perform ownership-sensitive actions.

Ownership transfer is a controlled state transition, never a role string mutation.

---

# 4. CANONICAL ORGANIZATIONAL HIERARCHY

The canonical topology is:

```text
                         PLATFORM
                            |
             +--------------+--------------+
             |                             |
         USER IDENTITY               SERVICE IDENTITY
             |
             v
        ORGANIZATION
             |
      +------+-------------------+
      |                          |
      v                          v
 BUSINESS ENTITY              MEMBERSHIPS
      |                          |
      |                          +--> ROLES
      |                          +--> SCOPE GRANTS
      |                          +--> STATUS
      |
      +-------------------+
      |                   |
      v                   v
    BRANCH             ORG-LEVEL RESOURCES
      |
      +-----------------------------+
      |             |                |
      v             v                v
  LOCATION      WAREHOUSE         REGISTER
                                     |
                                     v
                                   DEVICE
```

## 4.1 Required cardinality invariants

The implementation MUST enforce:

```text
User → Organization Membership       = 0..N
Organization → Membership             = 0..N
Organization → Branch                 = 0..N
Branch → Organization                 = exactly 1
Warehouse → Organization              = exactly 1
Register → Branch                     = exactly 1
Device → Organization                 = exactly 1 when registered
Membership → User                     = exactly 1
Membership → Organization             = exactly 1
Role Assignment → Membership         = exactly 1
Scope Grant → Organization            = exactly 1
```

A resource must never silently belong to multiple organizations through mutable foreign-key combinations.

---

# 5. TENANT ISOLATION IS AN INVARIANT

Tenant isolation is not merely an authorization middleware feature.

It is a layered invariant:

```text
          CLIENT
             |
             v
          API ROUTE
             |
             v
       SECURITY CONTEXT
             |
             v
       IAM / AUTHORIZATION
             |
             v
        APPLICATION
             |
             v
        REPOSITORY
             |
             v
       POSTGRESQL SCOPE
             |
             v
             RLS
```

Every layer should fail if a lower or upper layer is bypassed.

## 5.1 Negative invariant

For users `U`, organizations `A` and `B`:

```text
Membership(U, A) = ACTIVE
Membership(U, B) = none

U + object owned by B

READ   → DENY
WRITE  → DENY
EXPORT → DENY
SEARCH → no protected disclosure
SYNC   → DENY
ADMIN  → DENY
CACHE  → no leakage
REPORT → DENY
```

## 5.2 No implicit global context

The following patterns are forbidden:

```rust
let organization_id = request.organization_id;
repo.find_product(organization_id, product_id).await?;
```

where `organization_id` is merely user input.

The correct pattern is conceptually:

```rust
let scope = iam.resolve_scope(&principal, requested_context).await?;
let product = repo.find_product(&scope, product_id).await?;
```

The repository should require an authorization/scope context so developers cannot accidentally construct unrestricted lookups.

---

# 6. ORGANIZATION LIFECYCLE

Organization lifecycle is explicit.

```text
PROVISIONING
     |
     v
ACTIVE
  |    |
  |    +--------> SUSPENDED
  |                    |
  |                    +----> ACTIVE
  |
  +-------------------------> CLOSING
                               |
                               v
                            CLOSED
```

## 6.1 `PROVISIONING`

Used while required setup is incomplete.

Allowed operations should be tightly limited to onboarding/configuration operations.

No production financial workflow should execute against an organization that is not operationally activated.

## 6.2 `ACTIVE`

Normal business operation.

## 6.3 `SUSPENDED`

Business operations may be restricted because of:

- owner action;
- security incident;
- compliance issue;
- billing/entitlement policy;
- platform administrative action.

Suspension semantics must specify which operations remain available. For example, viewing account status or exporting required records might remain allowed while new sales are blocked.

## 6.4 `CLOSING`

A controlled transition toward closure.

The system should prevent new long-lived authority from being created while allowing necessary settlement, export, reconciliation and evidence operations.

## 6.5 `CLOSED`

Terminal state for ordinary platform operations.

Historical records remain preserved according to legal/business retention requirements.

A closed organization must not be treated as deleted merely because its normal interactive access is disabled.

---

# 7. ORGANIZATION CREATION

Organization creation is a security-sensitive provisioning workflow.

Minimum steps:

```text
AUTHENTICATED USER
       |
       v
CREATE ORGANIZATION REQUEST
       |
       v
VALIDATE INPUT
       |
       v
CREATE ORGANIZATION
       |
       v
CREATE OWNER MEMBERSHIP
       |
       v
CREATE INITIAL POLICY PROFILE
       |
       v
CREATE DEFAULT BRANCH / CONFIGURATION
       |
       v
AUDIT
       |
       v
COMMIT
       |
       v
POST-COMMIT SIDE EFFECTS
```

## 7.1 Transaction boundary

Organization and the initial owner membership must be created atomically.

The following state is invalid:

```text
organization exists
BUT
no valid owner membership exists
```

Unless a deliberately designed provisioning subsystem temporarily holds this state, it must not be visible as a normal tenant.

## 7.2 Default branch decision

Sitolo should support a single-branch merchant without forcing them to understand multi-branch architecture. The internal model may nevertheless create one default branch.

This preserves a clean invariant:

```text
sales / stock / register operations
        ↓
       branch
```

The UI can hide unnecessary complexity while the backend retains the stronger model.

## 7.3 Organization naming

Names are presentation/business data, not unique security identifiers.

Uniqueness should use a stable identifier and only apply name uniqueness where a real product requirement exists.

Do not make security decisions using organization names.

---

# 8. MEMBERSHIP LIFECYCLE

Membership lifecycle:

```text
INVITED
   |
   v
PENDING_ACCEPTANCE
   |
   +---- expired ----> EXPIRED
   |
   v
ACTIVE
  |
  +---- owner/admin action ----> SUSPENDED
  |                                  |
  |                                  +----> ACTIVE
  |
  +---- revoke -----------------> REVOKED
```

A membership that is not `ACTIVE` does not possess ordinary business authority.

## 8.1 Membership invariants

1. A membership belongs to exactly one organization.
2. A membership references exactly one user.
3. A user may have at most one active membership per organization unless an explicit historical/versioned model exists.
4. Membership status is server authoritative.
5. A revoked membership cannot be reactivated through arbitrary client mutation.
6. Historical membership evidence remains auditable.
7. Membership changes invalidate stale authorization state.
8. A membership cannot grant authority broader than the actor assigning it.
9. A user cannot assign themselves a broader role.
10. Role changes are not destructive history; effective authority changes are auditable.

---

# 9. INVITATION MODEL

Invitation is a controlled transition from no membership to pending membership.

```text
ISSUER
  |
  v
INVITATION
  |
  +--> organization
  +--> proposed role
  +--> proposed scope
  +--> expiry
  +--> target contact
  +--> nonce/token reference
  |
  v
RECIPIENT
  |
  v
IDENTITY PROOF
  |
  v
ACCEPT
  |
  v
MEMBERSHIP ACTIVE
```

## 9.1 Security requirements

Invitation tokens MUST be:

- high entropy;
- single use;
- expiry bounded;
- protected at rest if persisted;
- excluded from logs;
- excluded from metrics labels;
- bound to the invitation record rather than carrying trusted authority entirely in the token.

The server must load the invitation record before granting membership authority.

## 9.2 Role tampering

The invitation token must never be trusted to define role or scope.

```text
TOKEN → identifies invitation
SERVER → loads authoritative proposed role/scope
```

A malicious client cannot alter:

```text
role=OWNER
scope=ALL_ORGANIZATIONS
```

by modifying client-side invitation state.

## 9.3 Invitation race

Two concurrent acceptance attempts must result in exactly one successful membership activation.

Use a transactional claim/update protected by uniqueness/state conditions.

---

# 10. ROLE MODEL

Roles provide the canonical human vocabulary for authority.

Initial built-in roles should include:

```text
OWNER
ORG_ADMIN
BRANCH_MANAGER
CASHIER
INVENTORY_CLERK
PROCUREMENT_CLERK
ACCOUNTING_USER
VIEWER
AUDITOR
```

Exact role names may evolve through ADR review, but the model must not hard-code authorization behavior into string comparisons scattered through handlers.

## 10.1 Permission model

Permissions remain atomic capabilities.

Representative examples:

```text
ORG_VIEW
ORG_UPDATE
ORG_CLOSE
MEMBERSHIP_VIEW
MEMBERSHIP_INVITE
MEMBERSHIP_SUSPEND
MEMBERSHIP_REVOKE
ROLE_ASSIGN
SCOPE_GRANT
BRANCH_CREATE
BRANCH_UPDATE
BRANCH_ARCHIVE
WAREHOUSE_CREATE
REGISTER_CREATE
DEVICE_REGISTER
DEVICE_REVOKE
SALE_CREATE
SALE_VIEW
SALE_VOID
REFUND_CREATE
REFUND_APPROVE
INVENTORY_VIEW
INVENTORY_ADJUST
INVENTORY_ADJUST_APPROVE
INVENTORY_TRANSFER
REPORT_VIEW
EXPORT_CREATE
AUDIT_VIEW
PAYMENT_VIEW
PAYMENT_RECONCILE
EIS_SUBMIT
EIS_CONFIGURE
```

These align with the existing authorization vocabulary rather than replacing it. fileciteturn3file0L1469-L1503

## 10.2 Role does not equal permission

A role should be resolved into permissions using a versioned, server-side role definition.

```text
membership
   ↓
role assignments
   ↓
role definitions
   ↓
permissions
   ↓
scope grants
   ↓
IAM decision
```

This makes role policy inspectable and testable.

---

# 11. SCOPE MODEL

Sitolo needs more than RBAC.

The minimum scope hierarchy is:

```text
ORGANIZATION
   |
   +--> BRANCH
          |
          +--> LOCATION
          +--> WAREHOUSE
          +--> REGISTER
                 |
                 +--> DEVICE
```

## 11.1 Scope inheritance

A broad scope may imply authority over narrower descendants only when the permission explicitly allows inheritance.

Example:

```text
ORG-WIDE INVENTORY_VIEW
       |
       +--> Branch A inventory
       +--> Branch B inventory
       +--> Warehouse X
```

But:

```text
BRANCH A SALE_CREATE
```

must never imply:

```text
BRANCH B SALE_CREATE
```

## 11.2 Scope widening rule

A user cannot grant a scope they do not possess.

Formally:

```text
GRANTED_SCOPE ⊆ ISSUER_EFFECTIVE_SCOPE
```

unless the operation is performed through a separate platform-controlled provisioning authority.

## 11.3 Scope intersection

Effective access should be derived by intersection:

```text
ROLE PERMISSIONS
        ∩
MEMBERSHIP SCOPE
        ∩
RESOURCE SCOPE
        ∩
POLICY CONDITIONS
        ∩
ASSURANCE
        ∩
ENTITLEMENT
```

This avoids the common mistake of unioning privileges from multiple assignments without considering boundaries.

---

# 12. OWNER MODEL

Ownership is exceptional authority.

It must not be implemented as:

```text
role = "OWNER"
```

alone.

Owner authority may include:

- ownership-sensitive organization settings;
- control over organization administrators;
- ownership transfer;
- organization closure;
- high-risk billing/payment controls;
- security policy administration.

## 12.1 Multiple owners

The domain must explicitly decide whether an organization may have:

```text
one owner
```

or:

```text
multiple owners
```

The implementation must not let an accidental unique constraint or UI workflow decide this implicitly.

For the current baseline, Sitolo should treat **one primary legal/account owner per organization** as the default business model while allowing carefully designed additional high-privilege administrators without conflating them with ownership.

## 12.2 Ownership transfer

Ownership transfer must be a state machine:

```text
REQUESTED
   |
   v
VERIFICATION_REQUIRED
   |
   v
APPROVAL / STEP-UP
   |
   v
PENDING_EFFECTIVE
   |
   v
TRANSFERRED
```

Rejected/expired requests are preserved as evidence.

The existing authentication specification already requires step-up for ownership-sensitive actions. fileciteturn4file1L1227-L1250

---

# 13. ORGANIZATION ADMINISTRATION

Administrative authority must be decomposed.

Avoid a universal `ADMIN` permission.

Instead:

```text
ORG_ADMIN
    |
    +--> organization configuration
    +--> membership administration
    +--> role assignment within allowed scope
    +--> branch management

SECURITY_ADMIN
    |
    +--> security policy
    +--> MFA administrative workflows
    +--> device/security state

BILLING_ADMIN
    |
    +--> billing settings
    +--> entitlements

AUDITOR
    |
    +--> read-only evidence
```

The existing security architecture explicitly distinguishes platform administration, security administration, billing administration, support, audit and break-glass access. Phase 4 must preserve this separation. fileciteturn0file13L1317-L1334

---

# 14. BRANCH MODEL

Branch is a first-class business boundary, not merely an address record.

A branch can own or scope:

- inventory;
- registers;
- cashier sessions;
- devices;
- branch staff;
- branch-specific price configuration where supported;
- operational reports;
- stock transfers;
- branch cash balances.

## 14.1 Branch lifecycle

```text
PROVISIONING
    |
    v
ACTIVE
  |
  +----> SUSPENDED
  |
  +----> CLOSING
             |
             v
          CLOSED
```

## 14.2 Branch closure

Closing a branch must not destructively delete history.

Before closure, the system should verify:

```text
open registers = 0
pending cash sessions = 0
unresolved stock transfer = 0
unresolved reconciliation = 0
pending critical sync = policy-dependent
pending tax obligations = policy-dependent
```

The exact checklist belongs to domain modules, but IAM should prevent accidental authorization against a closed branch.

## 14.3 Branch reassignment

Moving a warehouse/register/device/user from one branch to another is not a generic foreign-key update.

It is a controlled transition because it changes scope semantics.

Historical records must retain the original branch context.

---

# 15. REGISTER AND DEVICE AUTHORITY

A register belongs to a branch.

A device may be bound to a register or branch according to the client model, but device identity remains independent as required by Phase 3.

```text
Organization
    ↓
Branch
    ↓
Register
    ↓
Device
```

The security model is:

```text
USER SESSION
     +
DEVICE STATE
     +
REGISTER STATE
     +
BRANCH SCOPE
     +
USER MEMBERSHIP
     =
VALID OPERATIONAL CONTEXT
```

A valid user with a revoked device is not authorized.

A valid device with a suspended user is not authorized.

A valid device attached to Branch A cannot be used to fabricate Branch B authority.

---

# 16. DEVICE-TO-TENANT BINDING

The Phase 3 device identity model establishes a device as an independent security subject. Phase 4 binds that subject into the organizational topology.

Required relationship:

```text
device.organization_id
```

but this field alone is not enough.

The server must verify:

```text
DEVICE ACTIVE
+
DEVICE ORGANIZATION = REQUESTED ORGANIZATION
+
SESSION VALID
+
USER MEMBERSHIP ACTIVE
+
USER DEVICE RELATIONSHIP VALID
+
REQUESTED BRANCH IN USER SCOPE
+
REQUESTED BRANCH IN DEVICE ALLOWED SCOPE
```

This matters especially for offline synchronization.

The existing authorization specification explicitly requires revoked devices to be rejected during synchronization after reconnect. fileciteturn3file0L1823-L1853

---

# 17. ORGANIZATION SWITCHING

Users can belong to multiple organizations.

The client may show:

```text
Organization A
Organization B
Organization C
```

but the server must establish the active context.

```text
SESSION
  |
  v
MEMBERSHIP LIST
  |
  v
SELECT ORGANIZATION
  |
  v
SERVER VALIDATES MEMBERSHIP
  |
  v
ISSUE / DERIVE EFFECTIVE CONTEXT
  |
  v
REQUESTS USE CONTEXT
```

## 17.1 Context switching security

Switching context should not require a new login every time, but it must invalidate stale scope assumptions in the client.

The application must ensure:

```text
current_org = server-approved org
current_branch = server-approved branch
```

not:

```text
current_org = arbitrary client string
```

## 17.2 Cached data isolation

When switching organizations, client caches must not leak:

- product data;
- inventory balances;
- sales;
- payment history;
- staff lists;
- customer data;
- exports;
- audit records.

A tenant switch should therefore include cache namespace separation or invalidation.

---

# 18. DELEGATED ADMINISTRATION

Managers need practical authority without requiring owner-level rights.

Delegation should be explicitly bounded:

```text
Owner
  ↓
Manager
  ↓
may administer staff
within Branch A
```

not:

```text
Manager
  ↓
becomes unrestricted admin
```

## 18.1 Delegation invariant

For an actor `A` assigning a role/scope to subject `B`:

```text
assignable_permissions(B)
    ⊆
permissions(A)
```

and:

```text
assigned_scope(B)
    ⊆
effective_scope(A)
```

unless a dedicated privileged workflow says otherwise.

## 18.2 Anti-escalation tests

The security harness must prove that:

```text
CASHIER → cannot assign MANAGER
BRANCH_MANAGER(A) → cannot grant BRANCH_B
MANAGER → cannot grant OWNER
AUDITOR → cannot modify staff
```

---

# 19. ROLE ASSIGNMENT STATE MACHINE

```text
REQUESTED
   |
   v
VALIDATING
   |
   +---- invalid ----> REJECTED
   |
   v
APPROVAL_REQUIRED
   |
   +----> APPROVED
   |
   v
EFFECTIVE
   |
   +----> REVOKED
```

Not every role change requires explicit approval, but high-risk role transitions do.

The policy should classify:

```text
LOW RISK
MEDIUM RISK
HIGH RISK
CRITICAL
```

and use step-up/approval accordingly.

---

# 20. MEMBERSHIP SUSPENSION AND REVOCATION

Suspension and revocation are materially different.

## 20.1 Suspension

Temporary removal of authority with potential reactivation.

## 20.2 Revocation

Permanent termination of the membership relationship for ordinary purposes.

## 20.3 Required effects

A membership revocation should trigger:

```text
membership state change
        +
security version bump
        +
relevant session invalidation
        +
authorization cache invalidation
        +
audit event
        +
notification where configured
```

The operation should be transactionally persisted before asynchronous notifications are emitted.

---

# 21. IAM TRANSACTION BOUNDARIES

IAM has several concurrency-sensitive operations.

The standard mutation pattern is:

```text
BEGIN
  |
  v
LOAD AUTHORIZED MEMBERSHIP
  |
  v
LOCK / SERIALIZE TARGET
  |
  v
REVALIDATE ACTOR AUTHORITY
  |
  v
REVALIDATE TARGET STATE
  |
  v
APPLY IAM TRANSITION
  |
  +--> SECURITY AUDIT
  +--> OUTBOX EVENT
  |
  v
COMMIT
```

This aligns with the existing security implementation contract for race-sensitive mutations. fileciteturn0file13L1160-L1198

## 21.1 No external calls inside transaction

Invitation email/SMS, push notifications or external identity-provider operations must not be performed while holding a long database transaction.

Instead:

```text
DB transaction
  ↓
commit organization/membership state
  +
outbox intent
  ↓
worker
  ↓
external notification
```

The existing system explicitly adopts transactional outbox processing for durable side effects. fileciteturn0file13L1202-L1246

---

# 22. CRITICAL IAM RACE CONDITIONS

## 22.1 Concurrent role changes

Two administrators may edit the same membership simultaneously.

Mitigation:

- version column or optimistic concurrency token;
- transaction lock for critical transitions;
- revalidation under transaction;
- deterministic conflict result.

## 22.2 Concurrent revocation and privileged operation

Scenario:

```text
Request A: privileged operation starts
Request B: membership revoked
```

The operation must not execute using stale authority if the business/security contract requires revocation to take effect immediately.

The exact behavior should use transaction isolation and/or security version checks appropriate to the mutation.

## 22.3 Concurrent invitation acceptance

Use atomic state transitions plus uniqueness constraints.

## 22.4 Concurrent owner transfer

Only one ownership transfer may become effective.

Use a transactionally protected state machine.

## 22.5 Concurrent branch closure

A branch closure must not race with creation of a new register/device membership that assumes the branch is active.

---

# 23. SECURITY VERSIONING

Phase 3 establishes security-version concepts.

Phase 4 must operationalize them for IAM:

```text
organization.security_version
membership.security_version
role_assignment.security_version
scope.security_version
branch.security_version
```

When authority changes materially:

```text
version++
```

Cached authorization state must include the relevant version.

Example:

```text
CACHE KEY
principal
+
organization
+
branch
+
permission set
+
organization_security_version
+
membership_security_version
+
policy_version
```

Do not cache:

```text
user_id -> is_admin=true
```

The existing IAM model explicitly requires version-aware authorization caching and fail-closed behavior. fileciteturn3file0L1764-L1803

---

# 24. AUTHORIZATION CACHE RULES

A cache is a performance optimization, never the authority store.

Requirements:

1. bounded TTL;
2. scoped cache key;
3. version-aware invalidation;
4. fail-closed for sensitive decisions;
5. explicit invalidation event on authority changes;
6. no cache persistence of secrets;
7. bounded memory usage;
8. negative results may be cached only where safe;
9. cross-tenant cache contamination tests;
10. cache backend outage must not produce privilege escalation.

## 24.1 Availability decision

For ordinary low-risk reads, a cache outage may reduce performance.

For high-risk writes:

```text
authorization cache unavailable
        ↓
authoritative lookup
        ↓
if authority unknown
        ↓
DENY
```

Do not fail open because Redis is down.

---

# 25. DATABASE MODEL

The exact columns must follow the existing `database_design.md`, but the logical tables are:

```text
organizations
business_entities
organization_memberships
roles
permissions
role_permissions
membership_roles
scope_grants
branches
locations
warehouses
registers
devices
invitations
organization_security_events
membership_security_events
ownership_transfer_requests
```

Depending on the already-frozen schema, some concepts may be represented by normalized variants rather than these exact names. The implementation must not duplicate an existing table merely because this conceptual model is clearer.

## 25.1 Hard database constraints

The database must protect at minimum:

- foreign-key integrity;
- organization/branch ownership consistency;
- membership uniqueness;
- role-assignment validity;
- scope organization consistency;
- active-state constraints;
- device organization association;
- invitation uniqueness/replay constraints;
- effective-date constraints;
- audit immutability where defined.

## 25.2 Cross-tenant foreign-key consistency

A classic tenant leak is:

```text
branch.organization_id = A
warehouse.organization_id = B
warehouse.branch_id = branch_A
```

This must be impossible at the database level where practical.

Use composite foreign keys or equivalent constraints so child rows cannot reference parent resources across tenant boundaries.

---

# 26. POSTGRESQL RLS STRATEGY

RLS is defense in depth, not the only authorization layer.

The intended model is:

```text
APPLICATION IAM
      +
TRUSTED DB SESSION CONTEXT
      +
POSTGRESQL RLS
      =
DEFENSE IN DEPTH
```

The database should receive trusted request context derived by the application, not raw client assertions.

Example conceptual context:

```text
app.organization_id
app.user_id
app.session_id
app.request_id
```

The implementation must ensure that application-controlled session settings cannot be arbitrarily supplied by an untrusted API caller.

## 26.1 RLS must not silently hide correctness bugs

RLS returning zero rows can conceal an authorization mistake.

Therefore critical repository operations must have explicit negative tests proving:

```text
cross-tenant → denied
cross-branch → denied
```

and explicit authorization tests at the application layer.

The existing database policy specifically requires real PostgreSQL testing for RLS rather than relying on mocks. fileciteturn0file12L1110-L1132

---

# 27. REPOSITORY API DESIGN

The persistence layer should make unsafe access difficult to express.

Preferred:

```rust
pub async fn get_branch(
    tx: &mut Transaction<'_, Postgres>,
    scope: &AuthorizedScope,
    branch_id: BranchId,
) -> Result<Option<Branch>, PersistenceError>
```

Avoid:

```rust
pub async fn get_branch(
    branch_id: BranchId,
) -> Result<Option<Branch>, PersistenceError>
```

The second API invites accidental unrestricted access.

## 27.1 Scope-aware query composition

Repositories must use explicit tenant predicates for tenant-owned resources even when RLS exists.

Example:

```sql
SELECT id, organization_id, name, status
FROM branches
WHERE organization_id = $1
  AND id = $2;
```

Do not depend entirely on RLS because:

- local integration tests may bypass the expected context;
- a privileged DB role may behave differently;
- future background workers may use different DB roles;
- application-level authorization remains necessary.

---

# 28. API SURFACE FOR PHASE 4

Representative endpoints:

```text
POST   /v1/organizations
GET    /v1/organizations
GET    /v1/organizations/{organization_id}
PATCH  /v1/organizations/{organization_id}

POST   /v1/organizations/{organization_id}/invitations
GET    /v1/organizations/{organization_id}/invitations
POST   /v1/invitations/{invitation_id}/accept
POST   /v1/invitations/{invitation_id}/revoke

GET    /v1/organizations/{organization_id}/members
GET    /v1/organizations/{organization_id}/members/{membership_id}
PATCH  /v1/organizations/{organization_id}/members/{membership_id}
POST   /v1/organizations/{organization_id}/members/{membership_id}/suspend
POST   /v1/organizations/{organization_id}/members/{membership_id}/revoke

POST   /v1/organizations/{organization_id}/branches
GET    /v1/organizations/{organization_id}/branches
GET    /v1/organizations/{organization_id}/branches/{branch_id}
PATCH  /v1/organizations/{organization_id}/branches/{branch_id}
POST   /v1/organizations/{organization_id}/branches/{branch_id}/close

POST   /v1/organizations/{organization_id}/members/{membership_id}/roles
DELETE /v1/organizations/{organization_id}/members/{membership_id}/roles/{role_id}

POST   /v1/organizations/{organization_id}/members/{membership_id}/scope-grants
DELETE /v1/organizations/{organization_id}/members/{membership_id}/scope-grants/{grant_id}

POST   /v1/organizations/{organization_id}/ownership-transfers
POST   /v1/organizations/{organization_id}/ownership-transfers/{id}/approve
POST   /v1/organizations/{organization_id}/ownership-transfers/{id}/reject
```

These are examples of business commands, not a mechanical requirement that every conceptual table become a public endpoint.

The existing API contract explicitly rejects table-shaped CRUD endpoints and favors explicit commands for state-changing operations. fileciteturn4file0L122-L175

---

# 29. REQUEST DTO POLICY

Every IAM endpoint uses dedicated DTOs.

Example:

```text
CreateOrganizationRequest
InviteMemberRequest
ChangeMembershipRoleRequest
ChangeMembershipScopeRequest
CreateBranchRequest
CloseBranchRequest
RequestOwnershipTransfer
```

Never deserialize request bodies directly into:

```text
Organization
Membership
Branch
RoleAssignment
ScopeGrant
```

This blocks property-level privilege injection.

The existing API contract explicitly requires named request DTOs and rejects direct persistence-model deserialization. fileciteturn3file0L632-L700

---

# 30. ERROR CONTRACT

Phase 4 consumes the Phase 2/3 error model.

IAM-specific errors should include stable codes such as:

```text
ORGANIZATION_NOT_FOUND
ORGANIZATION_SUSPENDED
ORGANIZATION_CLOSED
MEMBERSHIP_NOT_FOUND
MEMBERSHIP_INACTIVE
MEMBERSHIP_ALREADY_EXISTS
INVITATION_EXPIRED
INVITATION_ALREADY_ACCEPTED
INVITATION_INVALID
ROLE_ASSIGNMENT_FORBIDDEN
SCOPE_ASSIGNMENT_FORBIDDEN
SCOPE_OUTSIDE_ACTOR_AUTHORITY
BRANCH_NOT_FOUND
BRANCH_CLOSED
OWNERSHIP_TRANSFER_REQUIRED
OWNERSHIP_TRANSFER_INVALID_STATE
IAM_CONCURRENCY_CONFLICT
```

The client must not see:

- SQL errors;
- internal role IDs unless explicitly public;
- stack traces;
- database topology;
- token material;
- internal policy expression details.

The existing ADR-017 defines the stable structured domain-error taxonomy and requires transport mapping rather than leaking raw database/provider errors. fileciteturn6file4L406-L469

---

# 31. AUDIT CONTRACT

IAM mutations require durable audit evidence.

Minimum event classes:

```text
ORGANIZATION_CREATED
ORGANIZATION_UPDATED
ORGANIZATION_SUSPENDED
ORGANIZATION_REACTIVATED
ORGANIZATION_CLOSED

INVITATION_CREATED
INVITATION_ACCEPTED
INVITATION_REVOKED
INVITATION_EXPIRED

MEMBERSHIP_CREATED
MEMBERSHIP_ROLE_CHANGED
MEMBERSHIP_SCOPE_CHANGED
MEMBERSHIP_SUSPENDED
MEMBERSHIP_REACTIVATED
MEMBERSHIP_REVOKED

BRANCH_CREATED
BRANCH_UPDATED
BRANCH_CLOSED

ROLE_ASSIGNED
ROLE_REMOVED
SCOPE_GRANTED
SCOPE_REVOKED

OWNERSHIP_TRANSFER_REQUESTED
OWNERSHIP_TRANSFER_APPROVED
OWNERSHIP_TRANSFER_REJECTED
OWNERSHIP_TRANSFER_COMPLETED
```

Audit records must preserve the distinction between:

```text
real_actor
+
effective_subject
+
organization
+
branch/scope
+
action
+
target
+
old_state
+
new_state
+
policy_version
+
request_id
+
trace_id
+
result
```

The existing auth specification explicitly requires support access and actor/effective-subject distinction to remain auditable. fileciteturn3file0L1880-L1905

---

# 32. OBSERVABILITY

IAM telemetry must support both operational and security investigations.

Baseline metrics:

```text
sitolo_organizations_created_total
sitolo_organizations_suspended_total
sitolo_organizations_closed_total

sitolo_membership_invites_total
sitolo_membership_acceptances_total
sitolo_membership_revocations_total
sitolo_membership_suspensions_total

sitolo_role_assignments_total
sitolo_scope_changes_total
sitolo_cross_scope_denials_total
sitolo_iam_concurrency_conflicts_total
sitolo_iam_cache_misses_total
sitolo_iam_cache_failures_total
```

Safe labels:

```text
operation
result
reason_class
scope_type
client_platform
```

Do not use:

```text
user_id
phone
email
membership_id
organization_id
branch_id
invitation_token
```

as broad metric labels.

This follows the existing observability specification's explicit prohibition on unbounded identifiers as metric labels. fileciteturn3file4L2442-L2530

---

# 33. LOGGING AND REDACTION

IAM logs must never include:

```text
invitation tokens
session tokens
refresh tokens
passwords
MFA secrets
recovery codes
private keys
secret provider values
```

For debugging membership changes, log structured references:

```text
organization_id_hash
membership_id_hash
actor_id_hash
operation
result
reason_class
request_id
trace_id
```

Exact identifiers may be placed in access-controlled audit records where required for reconciliation, but should not automatically be copied into high-volume application logs.

OWASP guidance explicitly recommends removing, masking, hashing or encrypting tokens, credentials, session identifiers and sensitive personal data rather than logging them directly. citeturn888031search1

---

# 34. IAM WITH OFFLINE OPERATION

Offline operation complicates organizational authority.

The core rule remains:

```text
ONLINE IAM AUTHORITY
        ≠
OFFLINE DEVICE CAPABILITY
```

A device may retain a bounded operational profile allowing normal business work, but it must never gain offline authority to:

- create unrestricted administrators;
- transfer ownership;
- broaden organizational scope;
- modify security policy;
- revoke another tenant's security control;
- create unrestricted cross-branch access.

The existing authentication specification explicitly defines offline operation as bounded authority rather than permanent authority. fileciteturn3file0L1823-L1835

## 34.1 Offline role changes

Role changes should be online-required by default.

An offline client may cache existing effective capabilities but must not authoritatively create new server membership authority.

## 34.2 Reconnection

After reconnect:

```text
device state
+
user session
+
membership
+
organization status
+
branch status
+
policy version
```

must be revalidated before synchronization acceptance.

---

# 35. SERVICE IDENTITY SCOPING

Workers are first-class security subjects.

Examples:

```text
EIS_WORKER
PAYMENT_RECONCILER
NOTIFICATION_WORKER
EXPORT_WORKER
SYNC_WORKER
```

A worker identity must receive only the minimum organization data access required by the job.

A service processing one tenant's export should not implicitly gain unrestricted organization-admin capability across all tenants.

The existing ADR-020 explicitly treats workers as first-class reliability/security components with bounded permissions. fileciteturn6file2L247-L287

---

# 36. SUPPORT AND IMPERSONATION BOUNDARY

Support staff are not merchant administrators.

Support access should be modeled as a distinct control-plane authorization context:

```text
SUPPORT OPERATOR
      |
      v
SUPPORT ACCESS GRANT
      |
      +--> ticket/reason
      +--> tenant
      +--> scope
      +--> duration
      +--> permissions
      |
      v
EFFECTIVE SUPPORT SESSION
```

When impersonation exists:

```text
real_actor       = support user
represented_user = merchant user
```

Both identities must appear in audit evidence.

Support access must never silently grant ownership transfer, unrestricted financial mutation or security-policy changes.

---

# 37. ADMIN/API ENDPOINT SECURITY

Every IAM endpoint must have an inventory entry containing:

```text
method
path
owner
authentication
required_permission
scope_type
sensitivity
step_up
approval_requirement
idempotency
request_limits
rate_class
error_codes
audit_event
telemetry_operation
```

The existing API contract requires endpoint inventory at exactly this kind of granularity. fileciteturn3file6L714-L737

---

# 38. RATE LIMITING

IAM operations require distinct rate classes.

Example:

```text
ORGANIZATION_CREATE       strict
INVITATION_CREATE         moderate
INVITATION_ACCEPT         moderate
MEMBERSHIP_LOGIN_CONTEXT  normal
ROLE_ASSIGN               strict
SCOPE_CHANGE              strict
OWNERSHIP_TRANSFER        very strict
```

The limits must defend against:

- invitation spam;
- account probing;
- privilege-change flooding;
- ownership-transfer abuse;
- resource exhaustion.

Exact numerical limits remain environment/configuration policy and must be load-tested rather than invented as universal constants.

---

# 39. INPUT VALIDATION

IAM input is untrusted.

Validate:

- identifiers;
- names;
- role IDs;
- scope types;
- branch associations;
- expiration times;
- invitation target contact fields;
- reason fields;
- pagination;
- filter/sort inputs.

Reject oversized strings and unbounded arrays.

The existing API security architecture mandates maximum body size, string length, pagination and other resource budgets. fileciteturn3file6L739-L756

---

# 40. CONCURRENCY CONTROL STRATEGY

IAM should use the weakest mechanism that safely protects the invariant:

```text
optimistic versioning
       ↓
when sufficient
       |
       v
transaction lock
       ↓
when required
       |
       v
serialized critical transition
```

Do not blanket-lock all membership operations.

Broad locking causes:

- unnecessary contention;
- latency spikes;
- deadlocks;
- poor mobile experience;
- reduced throughput.

But critical transitions such as ownership transfer and final membership revocation should prioritize correctness over throughput.

---

# 41. CPU VS I/O CONSIDERATIONS

IAM is predominantly I/O-bound:

```text
HTTP
 ↓
identity state
 ↓
PostgreSQL
 ↓
policy data
 ↓
cache
```

Rust/Tokio should therefore avoid blocking the async runtime with:

- synchronous filesystem work;
- CPU-heavy cryptography outside intended library behavior;
- expensive policy compilation;
- unbounded JSON serialization.

CPU-heavy operations such as large export authorization analysis should be moved to bounded worker execution when necessary.

The goal is not to create workers for every function. It is to prevent CPU-heavy or externally blocking workloads from starving request processing.

---

# 42. MEMORY BEHAVIOR

IAM code should avoid:

- storing full role graphs unnecessarily per request;
- copying large organization structures;
- cloning principal claims repeatedly;
- retaining unbounded membership lists;
- loading complete organization datasets for simple authorization queries.

Prefer compact representations:

```text
Principal
MembershipSummary
ScopeSet
PermissionSet
PolicyVersion
```

The request context should carry only what downstream handlers need.

---

# 43. FAILURE MODES

## 43.1 Database unavailable

```text
IAM request
  ↓
DB unavailable
  ↓
NO AUTHORITY PROOF
  ↓
DENY SENSITIVE OPERATION
```

## 43.2 Cache unavailable

Fallback to authoritative data where safe.

Never use cache failure as a reason to assume permission.

## 43.3 Identity provider unavailable

New authentication may fail, but already issued sessions may remain valid according to the Phase 3 session contract if local authoritative session validation remains available.

Do not invent offline global IAM authority because an IdP is unavailable.

## 43.4 Outbox unavailable

If audit/outbox are required for atomic mutation evidence, commit semantics must follow the database contract. A sensitive mutation must not silently succeed while its required durable audit intent is lost.

## 43.5 Concurrent modification

Return a stable concurrency conflict and require client refresh/retry according to command semantics.

---

# 44. THREAT MODEL FOR PHASE 4

## Threat T1 — Horizontal tenant breakout

Attacker has valid organization-A credentials and supplies organization-B IDs.

Controls:

- membership-derived scope;
- repository scope predicates;
- RLS;
- negative tests;
- safe 404/403 behavior.

## Threat T2 — Vertical privilege escalation

Cashier attempts to assign manager/owner permissions.

Controls:

- assignable-permission intersection;
- authorization engine;
- approval policy;
- audit;
- negative tests.

## Threat T3 — Branch breakout

Branch-A manager manipulates branch-B identifiers.

Controls:

- branch scope authorization;
- composite database integrity;
- resource-level checks;
- negative tests.

## Threat T4 — Invitation tampering

Attacker modifies role/scope in an invitation payload.

Controls:

- server-side invitation record;
- opaque single-use token;
- signed/hashed protected representation where appropriate;
- server-authoritative role/scope.

## Threat T5 — Membership race

Revoked user races a privileged request.

Controls:

- transactional revalidation;
- security versions;
- appropriate locking.

## Threat T6 — Cache privilege persistence

Role revoked while cached authorization remains active.

Controls:

- version-aware key;
- invalidation;
- bounded TTL;
- fresh check for high-risk operations.

## Threat T7 — Closed-branch mutation

User submits cached command for a closed branch.

Controls:

- authoritative branch state validation;
- sync revalidation;
- state-machine errors.

## Threat T8 — Support privilege abuse

Support operator uses tenant access outside ticket scope.

Controls:

- JIT scope;
- expiry;
- audit;
- explicit effective subject;
- deny-by-default.

---

# 45. SECURITY TEST MATRIX

Mandatory tests include:

```text
[ ] cross-tenant GET
[ ] cross-tenant POST
[ ] cross-tenant PATCH
[ ] cross-tenant export
[ ] cross-tenant search
[ ] cross-branch read
[ ] cross-branch write
[ ] revoked membership request
[ ] suspended membership request
[ ] revoked device request
[ ] closed organization request
[ ] closed branch request
[ ] self-escalation attempt
[ ] cashier → manager escalation
[ ] manager → owner escalation
[ ] scope widening attempt
[ ] invitation replay
[ ] invitation expiry
[ ] invitation role tampering
[ ] concurrent invitation acceptance
[ ] concurrent role modification
[ ] concurrent membership revocation
[ ] ownership transfer race
[ ] stale authorization cache
[ ] cache outage fail-closed
[ ] DB outage fail-safe
[ ] malformed identifiers
[ ] oversized IAM request
[ ] authorization timing/enumeration behavior
[ ] audit event generated
[ ] required outbox record generated
[ ] audit does not contain secrets
```

These tests directly correspond to the existing 48-control security matrix, especially cross-user access, weak sessions, BOLA/IDOR, input validation, race conditions, endpoint security and fail-closed checks. fileciteturn3file1L135-L179 fileciteturn3file2L190-L225

---

# 46. PROPERTY-BASED IAM TESTS

Property testing should verify invariants over generated organization trees.

Example property:

```text
For every generated organization graph:

child.organization_id == parent.organization_id
```

and:

```text
For every generated actor:

effective_scope(actor, organization_A)
never grants access to organization_B
```

Another property:

```text
grant(actor, permission, scope)
→ granted_scope ⊆ actor_scope
```

These tests should run across thousands of generated combinations rather than a handful of manually selected examples.

---

# 47. FUZZING TARGETS

Recommended fuzz targets:

```text
organization creation DTO
membership mutation DTO
invitation DTO
scope expression/parser if one exists
role assignment payload
branch creation payload
authorization-context serialization
opaque cursor parsing
identifier parsing
```

Fuzzing is especially valuable for parser-level authority confusion and malformed hierarchical identifiers.

The existing testing strategy explicitly recommends fuzzing request parsing and other hostile input boundaries. fileciteturn3file2L227-L257

---

# 48. MIGRATION STRATEGY

IAM schema migrations must be forward-compatible.

Required process:

```text
schema design
   ↓
constraint analysis
   ↓
backward-compatible migration
   ↓
deploy code compatible with old/new schema
   ↓
backfill
   ↓
validate invariants
   ↓
enable new behavior
   ↓
remove obsolete structure later
```

Never combine an irreversible destructive migration with the first deployment of a security-critical authorization feature unless the deployment strategy has explicitly proven rollback safety.

---

# 49. BOOTSTRAP SECURITY

The first organization owner is a highly sensitive bootstrap path.

It must not rely on:

```text
first user in database = owner
```

unless the bootstrap process explicitly defines and constrains that rule.

Production bootstrap must have:

- authenticated provisioning;
- anti-automation controls;
- audit evidence;
- deterministic owner creation;
- no default passwords;
- no universal admin token;
- no debug override.

The security architecture expressly rejects default credentials and production debug access. fileciteturn3file1L169-L177

---

# 50. LOCAL DEVELOPMENT MODEL

Local development must resemble production authority semantics without requiring production secrets.

Recommended fixture topology:

```text
ORG-A
 ├── Branch-A1
 │    ├── Register-A1
 │    └── Device-A1
 └── Branch-A2

ORG-B
 └── Branch-B1
      └── Register-B1
```

Users:

```text
user-owner-A
manager-A1
cashier-A1
viewer-A
owner-B
```

Required fixtures include both valid and adversarial relationships.

Examples:

```text
cashier-A1 → attempts branch-A2
cashier-A1 → attempts org-B
owner-A → views org-B object by ID
```

All must fail appropriately.

---

# 51. RUST MODULE CONTRACT

Recommended Phase 4 crates/modules:

```text
sitolo-tenancy
    organization lifecycle
    branch lifecycle
    membership lifecycle
    hierarchy resolution

sitolo-authz
    role resolution
    permission sets
    scope evaluation
    decision interfaces

sitolo-domain
    organization/branch state machines
    IAM-related domain invariants

sitolo-persistence
    tenant-aware repositories
    membership repositories
    scope repositories
    transaction helpers

sitolo-audit
    IAM audit events

sitolo-events
    IAM event envelopes

sitolo-outbox
    durable notification/integration intents

sitolo-api
    IAM HTTP routes and DTOs

sitolo-testkit
    tenant fixtures
    multi-branch fixtures
    authorization fixtures
```

The exact crate names may follow the already-created Phase 1 workspace. The Phase 1 contract explicitly defines a modular workspace and keeps tenancy/authz separate from HTTP and persistence concerns. fileciteturn2file7L970-L1025

---

# 52. DEPENDENCY DIRECTION

Required direction:

```text
API
 ↓
APPLICATION
 ↓
DOMAIN

PERSISTENCE ─────┐
EVENTS ──────────┤
AUDIT ───────────┤
OUTBOX ──────────┤
                  ↓
             APPLICATION PORTS
```

The domain must not depend on:

- Axum;
- SQLx;
- PostgreSQL client internals;
- Redis;
- HTTP clients;
- provider SDKs.

The Phase 1 repository contract explicitly establishes this dependency rule. fileciteturn2file7L1029-L1059

---

# 53. IAM SERVICE INTERFACES

The application layer should expose explicit use-case interfaces such as:

```text
CreateOrganization
InviteMember
AcceptInvitation
SuspendMembership
RevokeMembership
AssignRole
RevokeRole
GrantScope
RevokeScope
CreateBranch
CloseBranch
RequestOwnershipTransfer
ApproveOwnershipTransfer
SwitchOrganizationContext
ResolveEffectiveScope
```

Avoid a generic method:

```text
updateOrganizationAccess(object, changes)
```

Generic mutation APIs become privilege-escalation magnets.

---

# 54. EFFECTIVE AUTHORIZATION CONTEXT

A request context should be immutable after construction.

Conceptual structure:

```rust
struct EffectiveIamContext {
    principal_id: PrincipalId,
    session_id: SessionId,
    device_id: Option<DeviceId>,
    organization_id: OrganizationId,
    membership_id: MembershipId,
    branch_scope: ScopeSet,
    permissions: PermissionSet,
    assurance: AssuranceLevel,
    policy_version: PolicyVersion,
    organization_security_version: SecurityVersion,
    membership_security_version: SecurityVersion,
}
```

This structure is an internal security object, not an API payload.

Downstream handlers should not be able to mutate it to become more privileged.

---

# 55. PREVENTING CONTEXT CONFUSION

A common architecture bug is mixing:

```text
requested organization
trusted organization
effective organization
```

These should have distinct types where practical.

Example concept:

```text
RequestedOrganizationId
TrustedOrganizationId
```

or at least distinct semantic wrappers in the domain/application layer.

This is particularly useful in Rust because the type system can make authority confusion harder to express.

---

# 56. IDEMPOTENCY

IAM commands that can be retried must use explicit idempotency semantics.

High-value examples:

```text
POST /organizations
POST /invitations
POST /role assignments
POST /scope grants
POST /ownership transfers
```

For example:

```text
same idempotency key
+
same request semantics
→ original result
```

while:

```text
same idempotency key
+
different request semantics
→ conflict
```

The existing API contract defines this exact duplicate-safe semantics requirement. fileciteturn3file0L844-L883

---

# 57. EVENTS

IAM events are facts, not commands.

Examples:

```text
MembershipRevoked
RoleAssigned
ScopeChanged
BranchCreated
OrganizationSuspended
DeviceBoundToOrganization
```

Event envelopes should contain:

```text
event_id
schema_version
event_type
occurred_at
organization_id?
actor_id?
correlation_id
causation_id?
data
```

Never include credentials or raw secret material.

Events are emitted through the transactional outbox when durable post-commit processing is required.

---

# 58. EVENT VERSIONING

Event meaning must remain stable.

Breaking changes require:

```text
new schema version
or
new event type
```

Do not silently reinterpret:

```text
MembershipRevoked
```

to mean something materially different.

Historical event evidence must remain understandable after software upgrades.

---

# 59. BUSINESS CORRECTNESS VS IAM CORRECTNESS

A successful IAM decision does not mean the business action is valid.

Example:

```text
CASHIER
 + SALE_CREATE
 + Branch A
 + valid session
 = AUTHORIZED TO ATTEMPT SALE
```

But the sale still requires:

```text
product exists
+
price valid
+
stock rule valid
+
register active
+
payment semantics valid
+
domain state valid
```

Conversely:

```text
OWNER
 + sale reversal permission
```

does not permit a reversal if the domain rule forbids the transition.

The existing authentication specification explicitly distinguishes authentication, authorization and domain-state legality. fileciteturn4file1L1037-L1069

---

# 60. IAM AND ENTITLEMENTS

Billing and entitlements are Phase 17, but the IAM architecture must leave a seam for them.

The future decision chain is:

```text
identity
+
membership
+
scope
+
permission
+
domain state
+
entitlement
+
assurance
+
approval
= decision
```

Phase 4 must not permanently encode billing status into membership roles.

Bad:

```text
PREMIUM_OWNER
FREE_CASHIER
```

Better:

```text
role = OWNER
entitlement = PROFESSIONAL_PLAN
```

---

# 61. IAM AND FUTURE MULTI-COUNTRY SUPPORT

The tenant model must remain country-neutral at the organizational layer.

Country-specific concerns such as:

- tax identity;
- currency;
- payment rails;
- legal entity details;
- regulatory obligations

must remain domain/configuration concepts.

Avoid:

```text
malawi_organization
malawi_branch
```

as schema concepts.

Use:

```text
organization
country_code
branch
```

with country-specific extensions where necessary.

This aligns with the existing ADR-025 multi-country extensibility decision. fileciteturn3file0L53-L90

---

# 62. DATA MINIMIZATION

IAM does not need to store every user attribute.

Keep only data necessary for:

- authority;
- lifecycle;
- auditing;
- operational management;
- business identity.

Do not copy identity-provider profiles wholesale into every organization membership.

Use references to the identity subject and maintain organization-specific metadata separately.

This reduces privacy exposure and synchronization complexity.

---

# 63. PII AND IAM SEARCH

Administrative search can expose sensitive membership data.

Search results must:

- require authorization;
- be tenant-scoped;
- be bounded;
- avoid unrestricted wildcard scans;
- avoid account enumeration across tenants;
- use minimal projections.

A user searching members of Organization A must never receive Organization B records because a global search endpoint accidentally omitted the tenant predicate.

---

# 64. CACHE ISOLATION TEST

Every cache test should include:

```text
write cache entry for org A
request org B
assert no hit containing org A authority
```

Also test stale entries:

```text
role OWNER
 ↓
cache
 ↓
role revoked
 ↓
request
 ↓
assert denied
```

This should run against the real cache implementation where feasible, not only a mock.

---

# 65. DATABASE PRIVILEGE MODEL

Application database credentials must not have unrestricted administrative authority.

At minimum separate:

```text
migration role
application role
worker role(s)
read/reporting role where needed
```

The application should not be able to bypass security simply because it can connect as a superuser.

The database design explicitly requires least-privileged runtime credentials and rejects production use of unrestricted database access. fileciteturn0file12L1110-L1132

---

# 66. IAM AUDIT INTEGRITY

Audit records must not be editable through ordinary IAM routes.

An administrator must not be able to execute:

```text
DELETE audit_events
```

as an ordinary organization operation.

The database contract already defines audit as a separately owned security/platform area and requires immutable evidence for important transitions. fileciteturn0file12L1081-L1106

---

# 67. SUPPORTING INCIDENT RESPONSE

When an IAM incident occurs, operators need to answer:

```text
Which user?
Which session?
Which device?
Which organization?
Which membership?
Which branch?
Which permission?
Which scope?
Which policy version?
What changed?
When?
Who changed it?
Did cache invalidate?
Did sessions revoke?
Did any business transaction occur after authority loss?
```

Phase 4 therefore requires correlation across:

```text
IAM audit
+
application logs
+
traces
+
security metrics
+
database state
```

The observability contract already establishes that telemetry is diagnostic evidence and authoritative business state remains in the domain/database layer. fileciteturn4file2L2050-L2141

---

# 68. OPERATIONAL RUNBOOK — UNAUTHORIZED CROSS-TENANT ATTEMPT

```text
1. Identify request_id / trace_id.
2. Confirm authenticated principal.
3. Confirm effective organization.
4. Identify requested resource.
5. Confirm denial reason.
6. Confirm database/RLS behavior.
7. Determine whether any response leaked object existence.
8. Check whether the same principal repeated attempts.
9. Preserve audit evidence.
10. Add regression test if a bypass occurred.
```

Do not manually modify business rows to “repair” an authorization issue.

---

# 69. OPERATIONAL RUNBOOK — PRIVILEGE ESCALATION

```text
1. Freeze suspicious high-risk operations if required.
2. Identify the real actor.
3. Identify effective membership.
4. Compare old/new role and scope.
5. Determine whether assignment exceeded issuer authority.
6. Revoke unauthorized membership/role/scope.
7. Revoke affected sessions/devices where required.
8. Invalidate caches/security versions.
9. Search for transactions performed under unauthorized authority.
10. Reconcile financial/inventory effects through domain workflows.
11. Preserve evidence.
12. Add regression coverage.
```

---

# 70. OPERATIONAL RUNBOOK — LOST DEVICE

```text
1. Locate device identity.
2. Verify organization/branch association.
3. Revoke device.
4. Increment device security version.
5. Invalidate relevant sessions/capability state.
6. Block future synchronization.
7. Preserve pending offline commands according to sync policy.
8. Investigate recent activity.
9. Re-register replacement device through normal enrollment.
```

This extends the Phase 3 device revocation model into organizational scope.

---

# 71. OPERATIONAL RUNBOOK — ORGANIZATION COMPROMISE

```text
1. Declare security incident.
2. Identify impacted organization.
3. Determine suspected principals/devices.
4. Revoke compromised sessions/devices.
5. Freeze high-risk organization administration.
6. Rotate affected credentials according to provider contract.
7. Review role/scope changes.
8. Review financial/inventory effects.
9. Preserve audit and telemetry evidence.
10. Restore safe authority state.
11. Verify tenant isolation.
12. Add regression controls.
```

---

# 72. PERFORMANCE REQUIREMENTS

IAM operations should generally remain lightweight and predictable.

Targets should be established from load testing rather than fabricated promises.

Measure:

```text
membership lookup latency
scope resolution latency
permission resolution latency
organization switching latency
role-assignment transaction latency
cache hit rate
DB lock wait
serialization conflicts
```

The performance objective is not maximum theoretical authorization throughput. It is predictable authorization under the same conditions that matter to Sitolo:

- many small merchants;
- low-end Android devices;
- unstable networks;
- occasional multi-branch businesses;
- bursty POS traffic;
- high-concurrency financial operations.

---

# 73. LOAD TEST SCENARIOS

Minimum scenarios:

```text
1. 1,000 organizations, low traffic
2. 10,000 organizations, steady traffic
3. one large tenant with many branches
4. many concurrent invitation requests
5. role-change burst
6. branch listing burst
7. authorization cache cold start
8. cache outage
9. database failover behavior
10. concurrent membership revocation + API requests
```

Measure both latency and correctness.

A faster authorization layer that occasionally leaks tenant data is a failure, not an optimization.

---

# 74. CI ENFORCEMENT

CI must enforce Phase 4 contracts.

Mandatory checks include:

```text
architecture dependency check
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
security integration tests
cross-tenant negative tests
RLS tests
property tests
migration tests
error registry validation
telemetry schema validation
secret scanning
```

The Phase 1 CI contract already requires locked dependency resolution, format/lint/compile/test gates and security scanning. fileciteturn2file12L1514-L1559

---

# 75. RELEASE BLOCKERS

A Phase 4 release must fail when any of the following is true:

```text
cross-tenant access succeeds
cross-branch access succeeds
revoked membership retains authority
revoked device retains IAM authority
role escalation succeeds
scope escalation succeeds
invitation replay succeeds
ownership transfer bypasses required assurance
IAM cache fails open
RLS test suite unavailable
required audit event missing
required outbox record missing
secret appears in logs
new IAM endpoint missing security inventory
unknown error code reaches clients
```

These conditions are release-blocking regardless of general business-test success.

---

# 76. DEFINITION OF READY

Phase 4 implementation is ready to begin only when:

```text
[X] Phase 3 identity/session/device contract available
[X] organization hierarchy agreed
[X] membership lifecycle agreed
[X] scope dimensions agreed
[X] role/permission vocabulary imported from existing auth spec
[X] branch semantics agreed
[X] ownership semantics recorded
[X] database ownership mapped
[X] API surfaces mapped
[X] audit events mapped
[X] security tests identified
[X] observability signals identified
[X] open policy-engine decisions preserved
```

---

# 77. DEFINITION OF DONE

Phase 4 is done only when:

```text
[ ] organization creation works transactionally
[ ] owner bootstrap is secure
[ ] membership lifecycle works
[ ] invitations are replay-safe
[ ] roles are assigned safely
[ ] scopes are bounded
[ ] branch lifecycle works
[ ] device-to-organization binding works
[ ] organization switching works
[ ] support access is separated
[ ] ownership transfer is controlled
[ ] authorization context is immutable
[ ] repositories require scope context
[ ] database constraints prevent cross-tenant corruption
[ ] RLS tests pass
[ ] cross-tenant negative suite passes
[ ] cross-branch negative suite passes
[ ] concurrency tests pass
[ ] audit evidence is durable
[ ] outbox behavior passes
[ ] cache invalidation passes
[ ] cache outage fails closed
[ ] API error registry is complete
[ ] telemetry is bounded
[ ] production logs are secret-free
[ ] migrations are tested
[ ] rollback strategy is documented
[ ] runbooks exist
[ ] CI gates are release-blocking
```

---

# 78. ARCHITECTURE TRADEOFFS

## Advantages

### Strong tenant isolation

The organizational hierarchy becomes explicit at every critical layer rather than relying on UI assumptions.

### Supports one-shop and multi-branch merchants

A small merchant can operate through one default branch while the backend naturally supports expansion.

### Safer delegated administration

Managers receive scoped authority without receiving universal admin power.

### Offline-compatible security model

Device identity and organizational scope integrate naturally with the existing offline model.

### Future regionalization

Country-specific concerns do not contaminate the tenant model.

### Better auditability

Role/scope changes become explicit business/security events.

## Disadvantages

### More schema complexity

A real IAM model requires more tables, constraints and lifecycle states than a simple `users.organization_id` design.

### More transaction logic

Membership and role transitions have concurrency and audit requirements.

### Higher implementation cost

Scope-aware repositories, negative tests and RLS tests require significant engineering effort.

### Cache complexity

Authorization caching introduces invalidation and versioning requirements.

### Product/UI complexity

The client must represent context switching, branch selection and scoped permissions correctly.

These costs are justified because they protect Sitolo's actual business model rather than adding decorative enterprise complexity.

---

# 79. WHY THIS OVER ALTERNATIVES

## Alternative A — `users.organization_id`

Rejected.

It cannot naturally model users belonging to several businesses, scoped managers, branch authority or independent device membership.

## Alternative B — Role-only RBAC

Rejected.

`MANAGER` does not explain **which organization**, **which branch**, **which warehouse**, **which resource**, or **which state**.

## Alternative C — Client-controlled tenant IDs

Rejected categorically.

It is an obvious horizontal-authorization failure.

## Alternative D — One global super-admin role for platform operations

Rejected for normal merchant administration.

It destroys separation of duties and expands blast radius.

## Alternative E — Fully external IAM controlling tenant/business authorization

Not accepted as the complete model.

An identity provider can establish identity; Sitolo still owns merchant membership, organizational scope, branch authority and business policy. This is an explicit existing architectural decision. fileciteturn6file2L473-L524

## Alternative F — Full policy engine now

Deferred.

Phase 4 should establish clean data and context boundaries. Phase 6 can introduce a more general policy engine only after real use cases justify the complexity.

---

# 80. OPEN DECISIONS

The following remain explicitly open rather than being guessed:

1. exact final built-in role catalog;
2. whether organizations can have multiple legal/business entities in the first production release;
3. whether an organization can have multiple legal owners;
4. exact branch closure requirements by vertical;
5. exact warehouse scope inheritance rules;
6. exact register/device one-to-one policy;
7. final JIT support implementation;
8. external enterprise SSO integration;
9. exact policy-engine technology, if any;
10. exact cache implementation/topology;
11. final organization naming/slug uniqueness policy;
12. exact organization data retention rules by jurisdiction.

These are not deficiencies in the Phase 4 architecture. They are intentionally isolated decisions that should be closed only when business, regulatory or infrastructure evidence exists.

---

# 81. TRACEABILITY TO THE 48 SECURITY CONTROLS

Phase 4 materially covers:

```text
4   weak authentication boundary
5   missing authorization
6   cross-user access
7   database privilege
9   unprotected admin routes
11  secret-leaking logs
12  verbose errors
15  client-only security
16  input validation
24  recovery/session relationship
25  weak sessions
28  rate limits
30  default credentials/bootstrap
33  IDOR/BOLA
34  APIs + user input
35  exposed logs
37  MFA policy for privileged actions
38  account enumeration
39  business-logic abuse
40  race conditions
42  CI identity
44  pinned dependencies
45  fail-closed checks
46  timeouts/resource controls
48  endpoint inventory
```

The complete 48-control matrix remains owned by the security architecture and test harness. fileciteturn3file1L135-L179

---

# 82. PHASE 4 IMPLEMENTATION SEQUENCE

Implementation should proceed in this order:

```text
PR-001
organization + branch domain primitives

PR-002
membership persistence + lifecycle

PR-003
roles + permissions + assignment model

PR-004
scope model + effective scope resolver

PR-005
invitation workflow

PR-006
organization/branch APIs

PR-007
repository scope enforcement + PostgreSQL constraints

PR-008
RLS integration and negative tests

PR-009
audit + outbox events

PR-010
cache/versioning

PR-011
device/org/branch binding

PR-012
ownership transfer workflow

PR-013
support/admin separation

PR-014
concurrency/property/fuzz tests

PR-015
observability + operational runbooks

PR-016
release gates
```

Each PR should be independently reviewable and should not introduce a security dependency that is only validated several PRs later.

---

# 83. IMPLEMENTATION CHECKLIST

```text
ARCHITECTURE
[ ] organizational hierarchy implemented
[ ] module ownership clear
[ ] no domain → infrastructure coupling

IDENTITY
[ ] principal consumed from Phase 3
[ ] sessions validated
[ ] device state validated

TENANT
[ ] organization lifecycle
[ ] branch lifecycle
[ ] membership lifecycle
[ ] invitation lifecycle

IAM
[ ] role catalog
[ ] permission catalog
[ ] scope model
[ ] delegation constraints
[ ] ownership workflow

DATABASE
[ ] FK integrity
[ ] uniqueness
[ ] cross-tenant constraints
[ ] indexes
[ ] RLS
[ ] real PostgreSQL tests

API
[ ] explicit DTOs
[ ] object authorization
[ ] error registry
[ ] idempotency
[ ] endpoint inventory

SECURITY
[ ] cross-tenant tests
[ ] cross-branch tests
[ ] privilege escalation tests
[ ] invitation replay tests
[ ] race tests
[ ] fail-closed tests

AUDIT
[ ] durable IAM events
[ ] actor/effective-subject distinction
[ ] outbox integration

OBSERVABILITY
[ ] IAM metrics
[ ] logs
[ ] traces
[ ] bounded labels
[ ] cache telemetry

OPERATIONS
[ ] revoke member runbook
[ ] lost device runbook
[ ] privilege escalation runbook
[ ] organization compromise runbook

RELEASE
[ ] all security gates passing
[ ] migration evidence
[ ] rollback evidence
[ ] CI enforcement active
```

---

# 84. FINAL ARCHITECTURE

The Phase 4 architecture is:

```text
                    SITOLO PLATFORM
                           |
                    AUTHENTICATED USER
                           |
                    SESSION + DEVICE
                           |
                    ACTIVE MEMBERSHIP
                           |
                    ORGANIZATION TENANT
                           |
              +------------+------------+
              |                         |
       ORG-LEVEL AUTHORITY          BRANCH SCOPE
              |                         |
        roles / permissions       branch / warehouse
              |                    register / device
              +------------+------------+
                           |
                    EFFECTIVE IAM
                           |
              +------------+------------+
              |                         |
          API COMMAND             INTERNAL WORKER
              |                         |
              +-------------+-----------+
                            |
                       DOMAIN RULES
                            |
                     DATABASE TRANSACTION
                       /           \
                    AUDIT        OUTBOX
                       \           /
                         COMMIT
                            |
                    OBSERVABILITY
                            |
                       EVIDENCE
```

The intended operating property is:

```text
ONE USER
   ↓
MANY ORGANIZATIONS
   ↓
MANY BRANCHES
   ↓
EXPLICIT MEMBERSHIP
   ↓
EXPLICIT ROLE
   ↓
EXPLICIT SCOPE
   ↓
SERVER-AUTHORITATIVE DECISION
   ↓
DOMAIN-CORRECT TRANSACTION
```

And never:

```text
CLIENT-SUPPLIED organization_id
        ↓
magic ADMIN role
        ↓
raw SQL
```

---

# 85. PHASE 4 EXIT GATE

Phase 4 may hand control to Phase 5 only when:

```text
[X] identity/session/device foundation exists
[X] organization hierarchy is frozen
[X] membership model is frozen
[X] branch scope model is frozen
[X] role/permission vocabulary is traceable to existing IAM contracts
[X] organization lifecycle is explicit
[X] membership lifecycle is explicit
[X] invitation lifecycle is explicit
[X] ownership transition is explicit
[X] IAM transaction boundaries are explicit
[X] concurrency rules are explicit
[X] tenant isolation is enforced at API/application/database layers
[X] cross-tenant negative tests exist
[X] cross-branch negative tests exist
[X] audit semantics exist
[X] outbox semantics exist
[X] telemetry semantics exist
[X] CI release blockers exist

[ ] all implementation tests pass
[ ] all PostgreSQL/RLS tests pass
[ ] all migrations pass from clean state
[ ] rollback evidence exists
[ ] production deployment evidence exists
```

The `[X]` items are architectural contracts established by this document; implementation evidence remains `[ ]` until the actual Phase 4 code passes the corresponding gates.

---

# 86. GOVERNANCE RULE

Any change that alters one of the following requires architecture review and, where the invariant changes, an ADR:

```text
organization boundary
membership semantics
branch scope semantics
role/permission meaning
ownership semantics
scope inheritance
security-version behavior
RLS tenant boundary
invitation authority
support authority
cross-tenant query behavior
device-to-tenant trust relationship
```

Routine refactoring that preserves observable authorization behavior does not require a new ADR.

---

# 87. RESEARCH BASIS

This implementation contract is based on the existing Sitolo architecture and current security guidance.

Project-specific evidence includes:

- the existing authentication/authorization contract, which separates user identity, memberships, sessions and devices and defines organization/branch scope; fileciteturn4file1L1086-L1127
- the existing API contract, which requires server-derived tenant context and object-level authorization; fileciteturn3file0L500-L542
- the existing security architecture, which requires fail-closed authorization and a complete 48-control verification model; fileciteturn3file1L135-L179
- the existing observability contract, which mandates bounded metric cardinality and explicit telemetry trust boundaries; fileciteturn4file2L2398-L2439
- the existing database contract, which establishes PostgreSQL authority, tenant isolation, RLS and least-privilege runtime database access; fileciteturn3file7L804-L830
- the existing ADR set, which explicitly rejects custom identity protocols and requires separate device identity and durable workers. fileciteturn6file2L139-L173 fileciteturn6file2L190-L243

External guidance used for security implementation includes OWASP logging guidance on secret/PII exclusion and tamper/access protection, and current Rust `secrecy` / `zeroize` documentation for sensitive in-memory values. citeturn888031search1turn888031search2turn888031search7

---

# 88. FINAL CONTRACT

> **Sitolo organization and IAM authority is derived from authenticated identity, active membership, explicit organizational scope, server-side role/permission resolution, current resource state, device state, required assurance and domain policy. No client-controlled tenant identifier, role, scope, device ID or cached UI state can grant authority. Every tenant-owned resource is protected by explicit application scope checks and database defense in depth. Organization, membership, role, scope, branch, invitation, ownership and device-binding transitions are explicit stateful operations with transactional audit and durable side-effect intent. Authorization caches are optimizations, never sources of truth, and fail closed where authority cannot be established. Offline capability remains bounded authority. Support and platform administration remain separate control planes. Every IAM boundary has negative security tests, observability and release-blocking evidence.**

The intended result is not merely a multi-tenant database.

It is a business operating system in which:

```text
THE RIGHT PERSON
       |
       v
IN THE RIGHT ORGANIZATION
       |
       v
IN THE RIGHT BRANCH/SCOPE
       |
       v
WITH THE RIGHT PERMISSION
       |
       v
ON THE RIGHT RESOURCE
       |
       v
IN THE RIGHT STATE
       |
       v
WITH THE REQUIRED ASSURANCE
       |
       v
CAN PERFORM THE RIGHT BUSINESS OPERATION
       |
       v
AND THE SYSTEM CAN PROVE IT LATER.
```

**END OF `phase4_tenant_organization_branch_iam_implementation.md`**
