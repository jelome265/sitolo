# SITOLO — Authorization Engine & Policy Enforcement Implementation Specification

**Document:** `phase6_authorization_engine_policy_enforcement_implementation.md`  
**Phase:** Phase 6 — Authorization engine / policy enforcement  
**Product:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first; controlled African regionalization  
**Backend:** Rust + Axum + Tokio  
**Persistence:** PostgreSQL authoritative; SQLite client continuity  
**Architecture:** Modular monolith first; workers and explicit adapters  
**Status:** Implementation-governing Phase 6 contract  
**Normative language:** MUST / MUST NOT / REQUIRED / SHOULD / SHOULD NOT / MAY are normative.  
**Depends on:** Phase 0 contracts, Phase 1 repository/workspace/CI, Phase 2 runtime substrate, Phase 3 identity/sessions/MFA/device identity, Phase 4 tenant/org/branch/IAM, Phase 5 PostgreSQL/RLS.

---

# 0. Executive Contract

Sitolo's authorization system is not a role check attached to HTTP routes. It is the decision system that converts authenticated identity, current organizational membership, scoped authority, requested action, target resource, resource state, property set, entitlement, authentication assurance, approvals, and business policy into an explicit decision.

The core equation is:

```text
VALID PRINCIPAL
    + VALID SESSION
    + VALID DEVICE STATE
    + ACTIVE MEMBERSHIP
    + CORRECT ORGANIZATION
    + CORRECT SCOPE
    + EXPLICIT PERMISSION
    + OBJECT AUTHORIZATION
    + PROPERTY AUTHORIZATION
    + VALID DOMAIN STATE
    + ENTITLEMENT
    + REQUIRED ASSURANCE
    + REQUIRED APPROVAL
    + CURRENT POLICY
    = AUTHORIZED OPERATION
```

Any required fact that is unknown, stale, revoked, inconsistent, unavailable, or outside the decision contract causes denial or a controlled non-authorizing result. Authorization MUST fail closed.

The authorization engine is therefore a **policy enforcement subsystem**, not merely a library. It has four conceptual roles:

```text
Policy Authority
    ↓
Policy Evaluator / Decision Engine
    ↓
Policy Enforcement Points (PEPs)
    ↓
Business operation
```

Within Sitolo, policy authority is distributed across explicit authoritative sources:

```text
Identity / session state
Tenant / membership / scope state
Role / permission assignments
Resource state
Domain invariants
Entitlements
Approval state
Security policy
    ↓
Authorization Request
    ↓
Authorization Decision
    ↓
Enforcement
    ↓
Audit / telemetry where required
```

The engine MUST NOT become a second business database. It derives decisions from authoritative state and may cache reusable policy projections, but cache contents never become permanent authority.

---

# 1. Relationship to Existing Sitolo Contracts

This document deepens, rather than replaces, the existing Sitolo contracts.

## 1.1 Existing authority hierarchy

The implementation follows the existing project hierarchy:

```text
Business model
    ↓
Product/domain specification
    ↓
System architecture
    ↓
Security architecture
    ↓
Security implementation contract
    ↓
Domain model
    ↓
Database contract
    ↓
API contract
    ↓
Phase 2 runtime substrate
    ↓
Phase 3 identity
    ↓
Phase 4 tenant/IAM
    ↓
Phase 5 PostgreSQL/RLS
    ↓
THIS DOCUMENT
    ↓
Implementation
```

The existing API contract already requires function-level, object-level and property-level authorization and explicitly states that a client-controlled tenant ID is not proof of authority. The existing identity contract separately distinguishes authentication from authorization and treats the principal as insufficient by itself. The existing Phase 4 contract establishes organizations, branches, memberships, scopes and role assignments. The Phase 5 contract establishes database constraints and RLS as defense in depth.

The existing security architecture also maps the 48 mandatory controls to executable security tests, including missing authorization, BOLA/IDOR, privilege escalation, fail-open checks, race conditions, client-only security and endpoint completeness. These tests remain release-blocking.

## 1.2 What Phase 6 adds

Phase 4 defined **who can belong where and what authority can be assigned**. Phase 5 persisted and constrained that model. Phase 6 defines **how that authority is evaluated consistently at runtime**.

Phase 6 therefore owns:

- authorization request models;
- decision evaluation;
- policy composition;
- permission resolution;
- scope evaluation;
- object/resource authorization;
- property authorization;
- state-aware policy checks;
- entitlement checks;
- approval requirements;
- assurance/step-up requirements;
- policy versions;
- authorization cache semantics;
- fail-closed behavior;
- PEP integration contracts;
- audit and denial evidence;
- authorization-specific telemetry;
- deterministic policy testing.

Phase 6 does **not** redefine:

- identity provider protocol;
- password hashing;
- session cryptography;
- tenant database schema;
- financial domain semantics;
- inventory ledger mathematics;
- payment provider protocol;
- MRA EIS protocol;
- client UI permission rendering.

Those remain owned by their existing documents.

---

# 2. Design Objectives

The engine must simultaneously satisfy seven objectives.

## 2.1 Correctness

For every protected operation, the same inputs under the same policy version should produce the same decision. Authorization MUST be deterministic for the evaluated snapshot.

## 2.2 Security

The default is deny. Missing permissions, incomplete context, cache failures, stale policy, policy evaluation errors and dependency ambiguity cannot silently become `ALLOW`.

OWASP's current authorization guidance explicitly recommends least privilege, deny-by-default, validation of permissions on every request, authorization testing, safe failure and stronger attribute/relationship-based controls where RBAC is insufficient. citeturn339893search2

## 2.3 Business correctness

A permission cannot make an illegal domain transition legal. Example:

```text
PERMISSION = REFUND_APPROVE
SALE_STATE = FULLY_REFUNDED

result = DENY / STATE_CONFLICT
```

Authorization and domain validity are separate predicates.

## 2.4 Performance

Authorization executes on high-volume POS paths. The engine MUST avoid unnecessary network hops, unbounded relationship traversals, synchronous remote policy calls for ordinary operations, and pathological cache amplification.

## 2.5 Auditability

Every high-impact decision must be reconstructable from its decision inputs, policy version, actor context and outcome without storing secret material.

## 2.6 Evolvability

The engine should support later richer relationship/attribute rules without forcing Sitolo into a premature distributed Zanzibar-like authorization platform.

## 2.7 Operational resilience

A telemetry outage or ordinary cache outage must not silently grant access. A policy data corruption event must produce denial and evidence rather than undefined behavior.

---

# 3. Authorization Model

Sitolo uses a layered authorization model rather than pure RBAC.

```text
                         +--------------------+
                         | Authentication     |
                         +---------+----------+
                                   |
                                   v
                         +--------------------+
                         | Session / Device   |
                         +---------+----------+
                                   |
                                   v
                         +--------------------+
                         | Membership / Scope |
                         +---------+----------+
                                   |
                                   v
                         +--------------------+
                         | Permission / Role  |
                         +---------+----------+
                                   |
                    +--------------+--------------+
                    |              |              |
                    v              v              v
              Object Check   Property Check   State Check
                    |              |              |
                    +--------------+--------------+
                                   |
                                   v
                         +--------------------+
                         | Entitlement        |
                         +---------+----------+
                                   |
                                   v
                         +--------------------+
                         | Assurance / StepUp |
                         +---------+----------+
                                   |
                                   v
                         +--------------------+
                         | Approval / SoD      |
                         +---------+----------+
                                   |
                                   v
                         +--------------------+
                         | Decision            |
                         +--------------------+
```

This model deliberately uses **RBAC for vocabulary** and **scope, relationship and attribute rules for actual authority**.

A role is not authority by itself.

```text
role = MANAGER
```

is insufficient to answer:

```text
Which organization?
Which branch?
Which warehouse?
Which register?
Which resource?
Which operation?
Which fields?
Which state?
Which monetary threshold?
Which assurance level?
Which approval policy?
```

The evaluator must answer those questions from current authoritative state.

---

# 4. Authorization Terminology

## 4.1 Principal

A normalized authenticated subject established by Phase 3. It identifies the actor and authentication/session context but does not itself confer tenant business authority.

## 4.2 Permission

An atomic capability such as:

```text
SALE_CREATE
SALE_VOID
REFUND_CREATE
REFUND_APPROVE
INVENTORY_ADJUST
PRICE_OVERRIDE
USER_INVITE
ROLE_ASSIGN
EXPORT_CREATE
PAYMENT_RECONCILE
EIS_CONFIGURE
```

## 4.3 Role

A named bundle of permissions and default policy semantics.

## 4.4 Scope

The organizational and resource boundary where a permission applies.

Examples:

```text
organization-wide
branch-specific
warehouse-specific
register-specific
resource-owner-specific
```

## 4.5 Subject

The actor for whom the decision is evaluated.

## 4.6 Resource

The target object or logical resource being accessed.

## 4.7 Action

The business operation being attempted.

## 4.8 Property set

The exact fields the caller is attempting to change or view.

## 4.9 Policy

A deterministic rule set defining when a permission is valid.

## 4.10 Policy version

An immutable identifier for the policy snapshot used to make a decision.

## 4.11 Policy input

The structured facts provided to the evaluator. Client-controlled values may be included as request intent but never as trusted authority facts.

## 4.12 Decision

The explicit evaluator result. It must distinguish at least:

```text
ALLOW
DENY
STEP_UP_REQUIRED
APPROVAL_REQUIRED
ENTITLEMENT_REQUIRED
CONTEXT_INVALID
POLICY_UNAVAILABLE
RESOURCE_UNAVAILABLE
```

Only `ALLOW` authorizes execution.

---

# 5. Decision Object Contract

The decision object is internal but strongly typed.

Conceptual Rust model:

```rust
pub enum AuthorizationDecision {
    Allow(AllowDecision),
    Deny(DenyDecision),
    StepUpRequired(StepUpRequirement),
    ApprovalRequired(ApprovalRequirement),
    EntitlementRequired(EntitlementRequirement),
    ContextInvalid(ContextError),
}
```

A decision SHOULD contain:

```text
policy_version
principal_subject_id
organization_id
resource_type
action
scope_result
decision_code
reason_class
evaluation_id
required_assurance?
required_approval?
expires_at?
```

Sensitive identifiers are protected according to the telemetry and logging policy. The evaluator MUST NOT place credentials, raw tokens, secrets or complete PII records in the decision object merely because they were available upstream.

The decision object MUST be immutable after construction.

---

# 6. Decision Reasons

Internal reason codes SHOULD be bounded and stable:

```text
NOT_AUTHENTICATED
SESSION_INVALID
SESSION_REVOKED
DEVICE_REVOKED
MEMBERSHIP_INACTIVE
TENANT_MISMATCH
BRANCH_OUT_OF_SCOPE
RESOURCE_OUT_OF_SCOPE
PERMISSION_MISSING
ROLE_INACTIVE
PROPERTY_FORBIDDEN
STATE_FORBIDDEN
ENTITLEMENT_MISSING
ASSURANCE_INSUFFICIENT
APPROVAL_REQUIRED
SEPARATION_OF_DUTIES_VIOLATION
POLICY_UNAVAILABLE
POLICY_INVALID
CONTEXT_INCOMPLETE
RESOURCE_NOT_FOUND
RESOURCE_STATE_UNAVAILABLE
CONCURRENCY_CONFLICT
```

Reasons must not be exposed indiscriminately to callers. A client may receive a safe public problem code while detailed reason metadata remains in protected audit/diagnostic channels.

---

# 7. Evaluation Input Contract

The evaluator accepts a strongly typed request rather than arbitrary JSON.

Conceptual structure:

```text
AuthorizationRequest
├── principal
│   ├── subject_id
│   ├── session_id
│   ├── device_id
│   ├── assurance
│   └── security_version
├── organization
│   └── organization_id
├── scope
│   ├── branches
│   ├── warehouses
│   ├── registers
│   └── other scopes
├── action
├── resource
│   ├── resource_type
│   ├── resource_id
│   └── state snapshot/reference
├── properties
├── request metadata
├── entitlement context
├── approval context
└── policy version hint
```

The key rule is that **trusted authority facts are resolved server-side**.

For example, the client can request:

```text
organization_id = org_B
sale_id = sale_123
```

but it cannot assert:

```text
I am OWNER of org_B
```

The engine obtains actual membership and permissions from authoritative server state.

---

# 8. Policy Evaluation Pipeline

Every decision follows the same conceptual sequence.

```text
1. Validate evaluator input
2. Validate principal/session/device state
3. Resolve active membership
4. Resolve effective organization context
5. Resolve effective scope
6. Resolve permission set
7. Verify function permission
8. Resolve resource
9. Verify object-level authorization
10. Verify property-level authorization
11. Verify current domain state
12. Evaluate entitlement
13. Evaluate authentication assurance
14. Evaluate approval/SoD
15. Produce explicit decision
16. Emit required evidence
```

The engine SHOULD short-circuit on decisive denial conditions where doing so does not leak resource existence.

For enumeration-sensitive resources, lookup and authorization must be composed so that unauthorized existence is not revealed.

---

# 9. Policy Composition

Policies are composed from independent predicates rather than one giant function.

Conceptually:

```text
principal_valid
AND session_valid
AND device_valid
AND membership_active
AND permission_granted
AND scope_allows
AND resource_allows
AND properties_allowed
AND state_allows
AND entitlement_allows
AND assurance_allows
AND approval_allows
```

An implementation should make each predicate independently testable.

Bad design:

```rust
fn authorize_everything(...) -> bool {
    // hundreds of branches
}
```

Preferred:

```rust
let principal = principal_policy.evaluate(&context)?;
let membership = membership_policy.evaluate(&context)?;
let permission = permission_policy.evaluate(&context)?;
let scope = scope_policy.evaluate(&context)?;
let resource = resource_policy.evaluate(&context)?;
let properties = property_policy.evaluate(&context)?;
let state = state_policy.evaluate(&context)?;
let entitlement = entitlement_policy.evaluate(&context)?;
let assurance = assurance_policy.evaluate(&context)?;
let approval = approval_policy.evaluate(&context)?;

Decision::combine([
    principal,
    membership,
    permission,
    scope,
    resource,
    properties,
    state,
    entitlement,
    assurance,
    approval,
])
```

The actual implementation may optimize the call graph, but the semantic decomposition must remain observable in tests.

---

# 10. Permission Resolution

Permission resolution starts with the actor's current effective memberships and assigned roles.

```text
User
  ↓
Memberships
  ↓
Active roles
  ↓
Role permissions
  ↓
Explicit grants/denials where supported
  ↓
Effective permissions
```

A user may have multiple roles. The result must be normalized into a bounded permission set.

## 10.1 Permission deny precedence

Sitolo SHOULD avoid a design in which an accidental explicit deny/allow precedence becomes implicit framework behavior. Precedence must be documented.

Recommended baseline:

```text
invalid context      → deny
explicit security deny → deny
missing prerequisite  → deny
positive grants       → allow only if all required predicates pass
```

Future explicit deny policies, if introduced, MUST have a documented precedence rule and tests for conflicting grants.

---

# 11. Scope Evaluation

Scope is hierarchical but not assumed to be automatically inherited unless the domain explicitly defines inheritance.

Example:

```text
Organization
   ├── Branch A
   │    ├── Warehouse A1
   │    └── Register A1
   └── Branch B
```

A branch-scoped manager does not automatically receive Branch B authority.

For each operation the evaluator derives:

```text
Requested Scope
      ↓
Effective Scope
      ↓
Permission Scope
      ↓
Intersection
      ↓
Scope Decision
```

Mathematically, an operation is in scope only where:

```text
requested_resource_scope
∈ effective_actor_scope
```

and the relevant organization context is consistent.

---

# 12. Object-Level Authorization

Every caller-controlled resource ID is untrusted until authorization succeeds.

Unsafe:

```sql
SELECT * FROM sales WHERE id = $1;
```

Preferred sequence:

```text
authenticate
→ tenant context
→ branch scope
→ permission
→ resource retrieval under trusted scope
→ state validation
```

The repository layer SHOULD expose scope-aware accessors such as:

```rust
get_sale(authz_context, sale_id)
```

rather than:

```rust
get_sale(sale_id)
```

This turns missing authorization context into a type/interface problem instead of merely a reviewer convention.

OWASP specifically recommends ensuring guessed lookup IDs do not bypass authorization, and treats object-level authorization as a distinct API security concern. citeturn339893search2

---

# 13. Property-Level Authorization

Property authorization is mandatory for privileged resources.

A cashier may be permitted to submit:

```text
quantity
product_id
payment_method
```

but not:

```text
organization_id
approved_by
settled_at
internal_cost
payment_verified
mra_receipt_number
role
```

The API request model must therefore be command-specific.

Property policy can be expressed as:

```text
actor permission
+ resource state
+ field class
+ action
= writable field set
```

Unknown fields SHOULD be rejected for sensitive mutations.

---

# 14. State-Aware Authorization

Authorization is not purely static.

Examples:

```text
SALE_VOID
+ SALE already voided
= DENY
```

```text
REFUND_APPROVE
+ refund exceeds refundable amount
= DENY
```

```text
INVENTORY_ADJUST
+ lot expired
+ normal adjustment prohibited
= DENY / controlled workflow
```

```text
PAYMENT_REFUND
+ payment unknown outcome
= reconciliation required
```

The engine MUST consume authoritative state or a transactionally consistent state snapshot for high-impact decisions. A stale cache must never be treated as sufficient evidence when current state materially changes authorization.

---

# 15. Monetary and Threshold Authorization

Some permissions depend on values, not merely resource types.

Examples:

```text
REFUND_CREATE <= cashier limit
REFUND_APPROVE <= manager threshold
PRICE_OVERRIDE <= approved deviation
CASH_ADJUST <= configured threshold
```

Threshold policies must use exact domain money semantics from the existing domain/database model.

Never use binary floating-point comparisons for authorization thresholds involving money.

The evaluator receives normalized money values, for example:

```text
amount_minor: i64
currency: CurrencyCode
```

and MUST verify currency compatibility before comparing thresholds.

---

# 16. Entitlement Enforcement

Authorization and product entitlements are separate predicates.

Example:

```text
permission = EXPORT_CREATE
entitlement = REPORTING_ADVANCED
```

Both are required where the feature is plan-gated.

The engine must distinguish:

```text
PERMISSION_MISSING
```

from:

```text
FEATURE_NOT_ENTITLED
```

This distinction matters operationally and for support.

Entitlement data may be cached, but an entitlement cache outage cannot grant premium capabilities by default.

---

# 17. Authentication Assurance and Step-Up

A valid session may still be insufficient for a sensitive operation.

```text
A1 normal session
     ↓
A3 required
     ↓
STEP_UP_REQUIRED
     ↓
MFA/passkey
     ↓
narrow elevated context
     ↓
authorization re-evaluated
```

The step-up artifact MUST be:

- short-lived;
- bound to the session/principal;
- bound to the intended operation class;
- non-replayable;
- invalidated on use or expiration;
- auditable.

A step-up for `PAYOUT_DESTINATION_CHANGE` must not authorize `OWNERSHIP_TRANSFER`.

---

# 18. Approval and Separation of Duties

High-risk workflows may require independent approval.

```text
REQUEST
  ↓
PENDING_APPROVAL
  ↓
AUTHORIZED_APPROVER
  ↓
APPROVED
  ↓
RE-EVALUATE CURRENT STATE
  ↓
POST
```

The approval itself does not automatically authorize execution.

At execution time the system must revalidate:

```text
request still exists
request still valid
request not expired
resource state still compatible
approver authority still active
requester != approver where SoD applies
policy version compatible
```

This prevents approval of obsolete state followed by execution after material changes.

---

# 19. Policy Versioning

Authorization decisions must identify the policy version under which they were made.

```text
policy_v42
   ↓
role/scope rules
threshold rules
approval rules
assurance rules
```

Policy changes must be versioned rather than mutating the meaning of an existing version invisibly.

A decision cache key should therefore incorporate enough policy state to prevent reuse across incompatible rules.

Example conceptual key:

```text
principal_version
+ membership_version
+ device_version
+ policy_version
+ organization_id
+ scope_hash
+ action
+ resource_type
+ resource_id where safe
+ property_set_hash where required
```

Policy versioning is especially important for revocation. Removing a permission must not leave an old cached decision authorizing future actions.

---

# 20. Authorization Cache

Caching is an optimization only.

The cache MUST NOT become an authority store.

## 20.1 What can be cached

Suitable cache candidates include:

- effective permission projections;
- low-risk read decisions;
- stable policy metadata;
- role-to-permission maps;
- bounded scope projections.

## 20.2 What should not be cached as a stale truth

Avoid treating these as durable authorization authority:

- active session state without versioning;
- membership revocation state without invalidation;
- device revocation state without invalidation;
- high-risk financial operation decisions;
- approval decisions detached from target state.

## 20.3 Fail-closed cache behavior

```text
cache hit
  ↓
validate version/scope/freshness
  ↓
use if valid

cache miss
  ↓
authoritative evaluation

cache unavailable
  ↓
authoritative evaluation

authority unavailable
  ↓
DENY
```

This is consistent with the existing Phase 4 contract, which explicitly prohibits cache outages from becoming privilege escalation paths. fileciteturn8file5L839-L872

---

# 21. Local Engine vs External Policy Engine

Phase 6 SHOULD begin with an in-process Rust authorization engine.

This is intentional.

## 21.1 Why local first

Sitolo starts as a modular monolith. An in-process evaluator:

- avoids a network round trip for every POS operation;
- keeps transaction-local state close to application logic;
- reduces distributed failure modes;
- makes offline/online semantics easier to reason about;
- avoids introducing another production dependency before workload evidence exists.

## 21.2 External policy engine boundary

OPA is a mature general-purpose policy engine that separates policy decision-making from policy enforcement and can be deployed close to enforcement points for low-latency decisions. citeturn339893search3turn339893search9

However, OPA or another external PDP is **not mandated for Phase 6**.

If later adopted, the boundary should be:

```text
Rust PEP
   ↓
Typed authorization input
   ↓
PDP adapter
   ↓
Policy decision
   ↓
Local semantic validation
   ↓
Rust enforcement
```

Provider policy technology must not leak into the domain model.

## 21.3 Zanzibar-style systems

Google Zanzibar demonstrates that globally consistent relationship authorization is feasible at massive scale, using relationship tuples, caching and explicit consistency semantics. Its design is relevant as research, not as a reason to copy a distributed authorization control plane into an early-stage SME operating system. citeturn339893search48turn339893search49

A distributed Zanzibar-style service becomes justified only if measured evidence demonstrates that the modular-monolith evaluator cannot satisfy required scale, organizational complexity or cross-service consistency requirements.

---

# 22. Policy Definition Strategy

Policies SHOULD be represented in typed Rust structures or controlled configuration data rather than ad-hoc strings spread through handlers.

Example conceptual model:

```text
PolicyRule
├── id
├── version
├── resource_type
├── action
├── subject_requirements
├── scope_requirements
├── property_requirements
├── state_requirements
├── threshold_requirements
├── assurance_requirements
├── approval_requirements
└── outcome
```

The policy representation must be:

- deterministic;
- reviewable;
- testable;
- versioned;
- diffable;
- bounded in execution complexity.

User-controlled data MUST NOT be executable as policy.

---

# 23. Policy Complexity Limits

Authorization itself can become a denial-of-service surface.

The engine MUST impose bounded evaluation behavior.

Limits should exist for:

```text
maximum rule count evaluated
maximum relationship traversal depth
maximum collection expansion
maximum external dependency calls
maximum recursion depth
maximum policy execution duration
maximum policy input size
```

A malformed or adversarial resource relationship graph must not cause unbounded CPU consumption.

Do not introduce general recursive policy evaluation merely because it makes the model look expressive.

---

# 24. Rust Crate / Module Architecture

The preferred Phase 6 structure builds on the Phase 1 workspace.

```text
crates/
├── sitolo-domain/
├── sitolo-application/
├── sitolo-auth/
├── sitolo-authz/
│   ├── src/
│   │   ├── context.rs
│   │   ├── action.rs
│   │   ├── resource.rs
│   │   ├── permission.rs
│   │   ├── scope.rs
│   │   ├── policy.rs
│   │   ├── decision.rs
│   │   ├── evaluator.rs
│   │   ├── cache.rs
│   │   ├── requirements.rs
│   │   ├── version.rs
│   │   └── audit.rs
│   └── tests/
├── sitolo-tenancy/
├── sitolo-persistence/
├── sitolo-audit/
├── sitolo-observability/
└── sitolo-api/
```

`siteolo-authz` owns policy semantics and evaluation interfaces. It must not depend directly on Axum handlers.

It MAY depend on application ports/interfaces but should avoid direct dependence on provider-specific infrastructure.

---

# 25. Application Ports

The authorization engine should consume ports such as:

```text
PrincipalProvider
MembershipProvider
PermissionProvider
ScopeProvider
ResourceAuthorizer
EntitlementProvider
ApprovalProvider
PolicyProvider
Clock
AuditSink
```

These ports allow:

- unit testing without infrastructure;
- integration testing with PostgreSQL;
- future policy-provider replacement;
- controlled caching.

Infrastructure implementations live outside the policy core.

---

# 26. API Enforcement Point Contract

Axum handlers and application services are PEPs.

Preferred pattern:

```rust
let decision = authz.authorize(AuthorizationRequest::for_command(...)).await?;
decision.require_allow()?;
application.execute(command).await
```

The application service MUST NOT assume that a route middleware check was sufficient for privileged business operations. Authorization should be enforced at the business-command boundary as well for high-impact mutations.

This prevents internal call paths from bypassing HTTP middleware.

---

# 27. Repository Enforcement

Repository access must remain scope aware.

A repository SHOULD require an authorization-scoped query object or trusted tenant context for tenant-owned resources.

Example:

```rust
pub struct AuthorizedQueryContext {
    pub organization_id: OrganizationId,
    pub branch_scope: BranchScope,
    pub actor: SubjectId,
}
```

This prevents casual calls to unconstrained CRUD methods.

Database RLS remains defense in depth. PostgreSQL itself documents that RLS policies restrict rows selected or modified and that tables with enabled RLS and no applicable policy default to deny; however, table owners and roles with `BYPASSRLS` can bypass RLS, so runtime database identities must be designed accordingly. citeturn339893search0turn339893search1

---

# 28. Transaction Boundary Rules

Authorization evaluation can occur before a transaction, inside a transaction, or both depending on the operation.

## 28.1 Low-risk read

```text
authorize
→ query
→ project
```

## 28.2 High-risk mutation

```text
authenticate
→ coarse authorization
→ BEGIN
→ reload authoritative state
→ fine authorization
→ domain invariant checks
→ mutate
→ audit/outbox
→ COMMIT
```

This two-stage pattern avoids making every read transaction-heavy while ensuring that critical mutation authorization is evaluated against state that cannot change invisibly between decision and write.

---

# 29. Authorization + PostgreSQL RLS

The relationship is defense in depth, not duplication for its own sake.

```text
HTTP/API PEP
   ↓
AuthZ engine
   ↓
application service
   ↓
repository scope
   ↓
PostgreSQL RLS
   ↓
row returned/mutated
```

A request that bypasses application authorization should still be constrained by database policy where the resource is RLS-protected.

RLS must never be used as the sole explanation of authorization behavior. RLS can silently return zero rows, which may obscure application bugs; therefore explicit authorization and negative tests remain mandatory.

PostgreSQL's current documentation confirms the separate roles of `USING` and `WITH CHECK`, and that absent applicable policies produce default deny when RLS is enabled. citeturn339893search0turn339893search1

---

# 30. SECURITY DEFINER Boundary

Any PostgreSQL function used by authorization or RLS MUST receive extraordinary review.

PostgreSQL warns that `SECURITY DEFINER` functions run with the owner's privileges and require careful `search_path` handling to prevent object shadowing by untrusted users. citeturn339893search4turn339893search7

Rules:

```text
NO arbitrary SECURITY DEFINER functions
NO writable schema in trusted search_path
NO unqualified security-sensitive object references
NO dynamic SQL from untrusted identifiers
NO privilege escalation through helper functions
```

A database function is not automatically safe because it lives inside PostgreSQL.

---

# 31. Worker and Background Enforcement

Workers are PEPs too.

A worker job must carry enough security context to answer:

```text
Which tenant?
Which organization?
Which actor initiated it?
Which capability authorized it?
Which operation?
Which policy version?
Which resource?
```

A worker must not infer tenant authority from:

- global mutable process state;
- job payload fields alone;
- an administrator-like service account.

The job handler must revalidate the security conditions necessary for its operation.

For example, an asynchronous export must not retain indefinite authority just because the user was authorized when the job was requested.

---

# 32. Support and Impersonation

Support access has a special authorization mode.

```text
real actor = support operator
        ↓
approved support scope
        ↓
effective subject = merchant user
        ↓
normal merchant authorization applied
        ↓
additional support-policy constraints
```

Both identities must remain visible in audit evidence.

Support must not convert impersonation into invisible identity substitution.

A support session should have:

- ticket/reason;
- target tenant;
- narrow capability set;
- expiration;
- optional approval;
- complete audit;
- no default financial mutation privilege.

---

# 33. Break-Glass Authorization

Break-glass is an emergency path, not a shortcut.

```text
separate identity
+ MFA
+ explicit reason
+ narrow capability
+ narrow scope
+ short expiration
+ enhanced audit
+ post-incident review
```

The engine should recognize break-glass as a distinct authentication/authorization class so that normal policy does not accidentally inherit emergency authority.

---

# 34. Offline Authorization Boundary

Offline support is bounded authority.

The local client may possess an offline capability profile, but the server remains authoritative.

Offline profiles MUST be:

- device-bound;
- organization-bound;
- scope-bound;
- operation-bound;
- time-bounded where applicable;
- versioned;
- revocable upon reconnection.

The offline profile must not include capabilities for:

```text
ROLE_ASSIGN
OWNERSHIP_TRANSFER
PLATFORM_SECURITY_CHANGE
UNRESTRICTED_PAYOUT_ADMIN
```

unless a future explicit architecture decision establishes a cryptographically controlled workflow.

When a device reconnects, current authorization is re-evaluated. Existing offline commands are accepted, rejected or quarantined according to the sync contract; the offline client cannot force authority retroactively.

---

# 35. Authorization Cache Invalidation

Authority-changing events must invalidate or version-bump affected decisions.

Mandatory invalidation triggers include:

```text
membership revoked
membership scope changed
role assigned
role removed
permission changed
organization policy changed
device revoked
session revoked
security version changed
entitlement downgraded
approval revoked/expired
```

Versioning is preferable to relying solely on best-effort deletion.

Example:

```text
membership_version = 17
cached decision built under 16
↓
cache entry invalid
↓
authoritative evaluation
```

This provides resilience against partial cache invalidation.

---

# 36. Concurrency Semantics

Authorization races are real business races.

Required scenarios include:

```text
role revoke + privileged mutation
membership revoke + privileged mutation
branch scope removal + request
approval revoke + execution
entitlement downgrade + feature use
device revoke + sync request
step-up expiry + execution
```

The authorization engine itself should avoid mutable shared global policy state.

Where policy snapshots are immutable, concurrent reads are cheap and safe.

For hot-path caching, synchronization should favor read-heavy concurrency primitives and bounded memory rather than global locks.

The CPU/I/O rule is:

```text
policy evaluation = CPU-bound deterministic work
membership/resource lookup = I/O
external policy provider = network I/O
cache access = network/local I/O depending on topology
```

Do not offload tiny deterministic authorization predicates to Tokio blocking pools; do not perform blocking network/database calls directly inside async tasks.

---

# 37. Time Semantics

Authorization must use a trusted clock abstraction.

Client timestamps are never authority.

The existing Sitolo ADR explicitly establishes server time as authoritative for server-side security decisions, while client time may be retained as offline evidence.

The evaluator must support tests for:

```text
expired session
future client timestamp
clock jump
expired step-up
expired invitation
expired approval
expired entitlement
policy effective_at boundary
```

Use dependency-injected clocks in unit tests and controlled server time in integration tests.

---

# 38. Error Mapping

Internal policy failures map to existing domain/API error contracts.

Examples:

```text
PERMISSION_MISSING
    → AUTHORIZATION_DENIED

RESOURCE_OUT_OF_SCOPE
    → NOT_FOUND or AUTHORIZATION_DENIED depending on enumeration policy

STEP_UP_REQUIRED
    → STEP_UP_REQUIRED

APPROVAL_REQUIRED
    → APPROVAL_REQUIRED

FEATURE_NOT_ENTITLED
    → FEATURE_NOT_ENTITLED

POLICY_UNAVAILABLE
    → AUTHORIZATION_UNAVAILABLE / safe 503 where appropriate
```

Do not leak:

- rule source;
- policy file content;
- internal SQL;
- stack traces;
- evaluator implementation details;
- cache topology;
- hidden resources.

The API contract already defines RFC 9457-style problem details and prohibits raw internal diagnostic material from client responses.

---

# 39. Audit Requirements

Not every read requires a durable audit record. High-risk authorization events do.

At minimum, audit:

```text
high-risk authorization allow
high-risk authorization deny
role changes
scope changes
membership changes
support access
break-glass access
step-up events
approval decisions
ownership operations
security policy changes
mass permission changes
```

Audit record should capture:

```text
who
acting-as-whom if applicable
what action
what resource
organization/scope
policy version
assurance level
approval reference
result
reason class
request/correlation ID
timestamp
```

Never store secrets in authorization audit records.

---

# 40. Telemetry

The authorization engine exposes aggregate metrics such as:

```text
sitolo_authorization_decisions_total
sitolo_authorization_denials_total
sitolo_authorization_evaluation_duration_seconds
sitolo_authorization_policy_errors_total
sitolo_authorization_cache_hits_total
sitolo_authorization_cache_misses_total
sitolo_authorization_cache_errors_total
sitolo_authorization_step_up_required_total
sitolo_authorization_approval_required_total
```

Safe label dimensions:

```text
operation
resource_type
result
reason_class
policy_version_family where bounded
```

Never use:

```text
user_id
email
phone_number
sale_id
trace_id
request_id
free-form_reason
```

These rules align with the existing observability contract, which prohibits unbounded authorization identifiers as metric labels.

---

# 41. Performance Budgets

Authorization is on the hot path.

Baseline engineering targets should be expressed as budgets, not vague claims.

Suggested initial server-side budgets:

```text
pure policy evaluation p95        <= 1 ms target
local cached authorization p95   <= 3 ms target
DB-backed authorization p95      <= 25 ms target
external PDP authorization        = exceptional / measured separately
```

These are **engineering targets**, not SLA claims. They must be validated under realistic workloads.

The correct optimization order is:

```text
profile
→ remove redundant I/O
→ batch authoritative lookups
→ cache safe projections
→ reduce allocation
→ reduce policy traversal
→ only then consider lower-level optimization
```

Do not optimize by weakening authorization freshness.

---

# 42. Cache Stampede Protection

When a popular policy projection expires, thousands of requests must not simultaneously rebuild it.

Use bounded single-flight/coalescing behavior where justified:

```text
cache miss
   ↓
first request obtains refresh lease
   ↓
other requests wait briefly / use prior safe value where permitted
   ↓
refresh
   ↓
publish versioned entry
```

For high-risk writes, stale authorization must not be served merely to avoid waiting.

---

# 43. Negative Caching

Negative results can reduce repeated forbidden checks, but they can create authorization-staleness risks.

Negative decisions may only be cached where:

- the underlying policy/version is included;
- TTL is bounded;
- revocation/invalidation semantics are acceptable;
- the resource existence policy does not create an oracle.

High-risk denial states SHOULD prefer authoritative or strongly versioned evaluation.

---

# 44. Policy Data Integrity

Policy configuration is security-sensitive configuration.

Policy files/configuration MUST be:

- version controlled where appropriate;
- reviewed;
- syntactically validated;
- semantically validated;
- signed/provenance-controlled for production distribution where policy is externally packaged;
- deployment-attributed.

CI must reject:

```text
invalid policy syntax
unknown permission
unknown resource type
unknown action
duplicate rule IDs
unbounded rule constructs
conflicting policy precedence without declaration
missing policy version
```

---

# 45. Policy Rollout

Policy changes should use staged deployment where operational risk warrants it.

```text
policy authored
 ↓
static validation
 ↓
unit/property tests
 ↓
shadow evaluation where useful
 ↓
staging
 ↓
production rollout
 ↓
monitor deny/allow deltas
 ↓
finalize
```

Shadow mode MUST NOT affect authorization until explicitly switched into enforcement mode.

Shadow decisions must never be represented to users as authority.

---

# 46. Policy Rollback

Rollback must be possible without rebuilding application artifacts when policy is separately deployable.

However, policy rollback is not automatically safe.

Before rollback, determine whether:

- a permission would become newly granted;
- a security revocation would be undone;
- an approval threshold would widen;
- a tenant scope would expand.

Security policy rollback may itself require approval and incident review.

---

# 47. Why Not Role Checks in Every Handler

This pattern is prohibited:

```rust
if user.role == "MANAGER" {
    // allow
}
```

Reasons:

- ignores tenant scope;
- ignores branch scope;
- ignores object state;
- ignores property authorization;
- becomes inconsistent across endpoints;
- creates stringly typed security logic;
- makes policy changes difficult to audit;
- invites privilege escalation.

The existing Sitolo contracts explicitly reject role-only authorization.

---

# 48. Why Not Put Authorization Entirely in PostgreSQL

Rejected as the primary model.

Database policies are excellent defense in depth, but the business authorization model also depends on:

- application actions;
- state machines;
- entitlements;
- assurance levels;
- approvals;
- provider/integration state;
- support access;
- client command semantics.

Those are application/domain concerns.

PostgreSQL RLS enforces row visibility/mutation boundaries; it does not replace the business policy engine.

---

# 49. Why Not Fully Externalize Authorization

A remote PDP creates additional I/O and availability dependencies on every decision.

```text
POS request
→ application
→ network
→ PDP
→ network
→ application
→ PostgreSQL
```

This can be excellent at scale, but it adds:

- network latency;
- timeout semantics;
- version synchronization;
- policy distribution complexity;
- extra failure modes;
- another operational plane.

For Sitolo's modular-monolith-first architecture, local policy evaluation is the safer default until measurements show otherwise.

---

# 50. Authorization API Surface

The internal authorization API should remain small.

Recommended operations:

```text
authorize(request)
require_allow(request)
check_permission(context, action)
check_resource(context, action, resource)
check_properties(context, action, properties)
check_state(context, action, resource)
require_step_up(context, requirement)
require_approval(context, requirement)
```

Avoid exposing low-level mutable policy internals to domain code.

The domain should consume semantic authorization outcomes, not manipulate cache records or raw policy maps.

---

# 51. Example: Sale Void

A sale void request demonstrates the complete model.

```text
REQUEST
  ↓
Principal valid?
  ↓ yes
Session active?
  ↓ yes
Device active?
  ↓ yes
Membership active?
  ↓ yes
Organization scope valid?
  ↓ yes
Permission SALE_VOID?
  ↓ yes
Sale belongs to authorized branch?
  ↓ yes
Sale state allows void?
  ↓ yes
Property set valid?
  ↓ yes
Any approval threshold crossed?
  ↓ no
Required assurance sufficient?
  ↓ yes
ALLOW
```

At transaction execution the sale is reloaded and the final state is rechecked before mutation.

---

# 52. Example: High-Value Refund

```text
REFUND_CREATE
↓
permission exists
↓
resource in scope
↓
refund amount <= actor threshold?
    NO
     ↓
APPROVAL_REQUIRED
     ↓
approval created
     ↓
manager evaluates independently
     ↓
manager permission + threshold + scope
     ↓
SoD satisfied
     ↓
APPROVED
     ↓
transaction reloads sale/payment state
     ↓
final authorization
     ↓
post refund
```

This prevents a cashier from converting an approval-capable UI into an unrestricted refund endpoint.

---

# 53. Example: Inventory Transfer

Inventory transfer checks may include:

```text
TRANSFER_CREATE
+ source branch scope
+ destination branch permission
+ warehouse relationship
+ SKU visibility
+ lot restrictions
+ quantity rules
+ approval threshold
+ device state
+ offline capability if offline
```

The authorization engine does not calculate inventory truth. It asks whether the actor is permitted to request the operation; the inventory domain then proves the transfer is economically valid.

---

# 54. Example: Export

Exports are high-risk because they create data-exfiltration capability.

Authorization must include:

```text
EXPORT_CREATE
+ tenant scope
+ dataset type
+ requested columns
+ time range
+ maximum rows
+ entitlement
+ assurance
+ potentially approval
```

The export worker revalidates the job authority when execution begins.

A previously valid export request must not acquire indefinite rights to newly created sensitive data.

---

# 55. Service Identity Authorization

Workers and future service extractions use service identities.

A service identity should receive narrowly scoped permissions.

Example:

```text
EIS_WORKER
  EIS_SUBMIT
  READ_APPROVED_TAX_PAYLOAD
  AUDIT_APPEND

NOT:
  ROLE_ASSIGN
  USER_DELETE
  TENANT_DELETE
  PAYMENT_PROVIDER_CONFIGURE
```

The same evaluator can be used with a typed subject type:

```text
HumanUser
ServiceIdentity
SupportIdentity
PlatformAdministrator
```

Different subject classes may have different allowed policy domains.

---

# 56. Tenant Safety Properties

The engine must satisfy the following properties.

### Property A — Tenant monotonicity

Adding access to Tenant B does not remove existing access to Tenant A unless an explicit policy says so.

### Property B — Revocation monotonicity

Revoking membership cannot increase authorization.

### Property C — Permission removal monotonicity

Removing a permission cannot increase authorization.

### Property D — Scope narrowing monotonicity

Narrowing branch/warehouse scope cannot grant resources outside the new scope.

### Property E — Assurance monotonicity

A lower assurance level cannot satisfy a higher assurance requirement.

### Property F — Approval integrity

Approval cannot expand the approver's authority beyond their permission and scope.

These are ideal candidates for property-based testing.

---

# 57. Security Test Matrix

At minimum, Phase 6 must implement:

```text
unauthenticated → protected resource → deny
invalid session → deny
revoked session → deny
revoked device → deny
inactive membership → deny
Tenant A → Tenant B resource → deny
Branch A → Branch B resource → deny
permission absent → deny
object out of scope → deny
forbidden property → deny
invalid state → deny
insufficient assurance → step-up
missing approval → approval required
self approval under SoD → deny
cache outage → no privilege escalation
policy unavailable → deny
stale policy version → reject/re-evaluate
role removed during request → documented deterministic result
device revoked during sync → deny future authority
worker missing tenant context → reject
support scope exceeded → deny
break-glass without MFA → deny
```

---

# 58. Property-Based Authorization Testing

Generate combinations of:

```text
roles
permissions
organizations
branches
resource states
assurance levels
entitlements
approval states
policy versions
revocation states
```

The test oracle is expressed as properties rather than only individual examples.

Example:

```text
forall authorization contexts C,
if permission P is removed from C,
then authorize(C, action(P)) != ALLOW
unless another explicitly independent permission path exists.
```

The test framework must distinguish legitimate independent authority from accidental privilege escalation.

---

# 59. Mutation Testing

Authorization tests are weak if breaking the security engine does not fail the suite.

Mutation targets include deliberate removal of:

```text
tenant predicate
branch predicate
permission predicate
property predicate
state predicate
assurance predicate
approval predicate
entitlement predicate
revocation check
policy-version check
cache freshness check
fail-closed branch
```

Every security-critical mutation that survives must be treated as a test-design defect.

---

# 60. Fuzzing

Fuzz targets include:

- authorization request decoding;
- opaque cursor/resource identifiers;
- scope sets;
- permission identifiers;
- policy documents if configuration is parsed;
- property-set encodings;
- policy input normalization;
- approval references;
- step-up artifacts;
- cache key construction.

Fuzzing must specifically seek:

```text
panics
stack overflow
unbounded loops
allocation explosions
unexpected ALLOW
policy parser confusion
integer overflow
string normalization inconsistencies
```

Unexpected `ALLOW` is the most serious authorization fuzzing outcome.

---

# 61. Decision Logging and Privacy

Development logging may expose more evaluator detail in controlled environments, but production policy should log a bounded security event rather than a full request object.

Safe production decision event:

```json
{
  "event": "AUTHZ_DENY",
  "operation": "REFUND_APPROVE",
  "resource_type": "refund",
  "reason_class": "PERMISSION_MISSING",
  "policy_version": "p42",
  "request_id": "req_..."
}
```

Resource IDs may be retained in protected audit evidence where required, but must not automatically become high-cardinality metrics.

---

# 62. Alerting

Authorization alerts should focus on behavior changes, for example:

```text
spike in cross-tenant denials
spike in privilege denials
unusual role-assignment volume
mass scope changes
repeated step-up failures
repeated approval failures
policy evaluation errors
policy checksum mismatch
cache corruption/error spikes
worker authorization failures
```

A denial spike is not proof of compromise. It is a signal for investigation.

---

# 63. Availability vs Security

The authorization engine must explicitly prioritize trust over availability for security-sensitive mutations.

```text
LOW-RISK READ
cache unavailable
→ authoritative lookup
→ continue if safe

HIGH-RISK WRITE
policy dependency unavailable
→ DENY / controlled unavailable
```

The engine MUST NOT implement:

```rust
if authorization_service_is_down() {
    return Allow;
}
```

That is a catastrophic security design.

---

# 64. DoS Resistance

Attackers may attempt to force expensive authorization evaluations.

Controls include:

- request rate limits;
- concurrency limits;
- policy evaluation deadlines;
- bounded graph traversal;
- bounded scope expansion;
- bounded batch authorization sizes;
- cache stampede protection;
- circuit breakers around external PDP adapters;
- no user-defined policy execution.

Authorization limits must be integrated with the Phase 2 resource-budget framework.

---

# 65. Batch Authorization

Batch authorization can improve throughput for list screens, but creates risk if implemented as one giant unconstrained operation.

Batch APIs must define:

```text
maximum resources
maximum property set size
maximum evaluation time
partial-result semantics
```

For list endpoints, prefer filtering resources by authorized scope in the database rather than loading every resource and authorizing one by one in application memory.

However, property-level or resource-state policy that cannot safely be expressed in SQL may still require per-resource evaluation.

Measure before choosing.

---

# 66. Authorization for Search

Search must not become a tenant oracle.

The correct model is:

```text
query
↓
authorized scope filters
↓
resource eligibility
↓
search execution
↓
safe projection
```

Never search globally and then remove unauthorized results only in presentation code if the underlying database query could reveal counts, timings, ranking or metadata across tenant boundaries.

---

# 67. Authorization for Aggregates and Counts

Counts can leak data even when individual rows are hidden.

Example:

```text
GET /inventory?branch=B&count=true
```

If branch B is unauthorized but the API reveals `total=873`, the caller has learned protected information.

Authorization therefore applies to aggregate queries and metadata as well as row-level access.

---

# 68. Authorization for Caches and Read Models

Caches and read models must encode tenant and scope boundaries.

Bad:

```text
products:{search_term}
```

when the result differs by tenant.

Preferred conceptual key:

```text
products:{organization}:{scope_hash}:{query_hash}:{projection_version}
```

Read models must never become an authority store merely because they are easier to query.

---

# 69. Authorization for Object Storage

File/object access follows the same pattern.

A presigned URL must not become an independent authorization bypass.

Before issuing a file access capability:

```text
authenticate
→ tenant scope
→ file resource authorization
→ file state/policy
→ issue narrow capability
```

The capability should be short-lived and scoped to the intended object/action.

---

# 70. Authorization for Notifications

Notification jobs can expose business information.

A worker must verify that the destination and content remain authorized at execution time where the sensitivity warrants it.

Do not treat queued notification payloads as permanently trusted simply because the enqueue request was authorized earlier.

---

# 71. Authorization for Payment Operations

Payment operations are particularly sensitive.

Authorization controls must not be based on client-side payment status.

For example:

```text
PAYMENT_REFUND
+ permission
+ payment belongs to tenant
+ provider state supports refund
+ amount eligible
+ threshold/approval
+ assurance
= allow
```

Provider reconciliation remains separate from authorization. A user can be authorized to request a refund even when the payment outcome is currently unknown; the domain then determines whether the action is operationally safe.

---

# 72. Authorization for MRA EIS Operations

EIS configuration and submission capabilities are separate from ordinary sale permissions.

Examples:

```text
EIS_SUBMIT
EIS_CONFIGURE
EIS_VIEW
EIS_EXCEPTION_RESOLVE
```

Terminal and taxpayer configuration must not become ordinary merchant-editable state merely because a user has a broad manager role.

Regulatory evidence remains a separate integration concern.

---

# 73. Policy Decision Auditing vs Application Audit

The engine should not duplicate every business event into authorization logs.

Use:

```text
AUTHZ audit
= security decision evidence

Domain audit
= business state transition evidence
```

For a sale:

```text
AUTHZ_ALLOW_HIGH_RISK
        +
SALE_FINALIZED
        +
OUTBOX_EVENT_CREATED
```

These records answer different questions and are correlated rather than merged into one overloaded structure.

---

# 74. Formal Security Properties

Phase 6 should document the following invariants as executable properties.

## P1 — No unknown authority

If required authorization input is unknown, the decision cannot be `ALLOW`.

## P2 — Revocation safety

After security-version change, prior authorization cache entries are invalid.

## P3 — Tenant isolation

A subject with authority only in organization A cannot obtain an allow decision for organization B.

## P4 — Scope narrowing

Removing a branch from effective scope cannot grant an operation in that branch.

## P5 — SoD integrity

A request/approval pair requiring separation of duties cannot be approved by the requester.

## P6 — Assurance integrity

A lower assurance level cannot satisfy a higher level requirement.

## P7 — Entitlement integrity

Permission without entitlement cannot authorize entitlement-gated features.

## P8 — State safety

Permission without valid resource state cannot authorize illegal state transitions.

## P9 — Cache safety

Cache inconsistency cannot create a new authorization path that authoritative evaluation would deny.

## P10 — Fail closed

Policy evaluator errors produce non-authorizing outcomes.

---

# 75. Integration Tests with Real PostgreSQL

The test harness must combine application authorization with actual RLS.

Minimum matrix:

```text
TENANT_A → TENANT_A → allow
TENANT_A → TENANT_B → deny
BRANCH_A1 → BRANCH_A1 → allow
BRANCH_A1 → BRANCH_A2 → deny
SUPPORT → approved scope → allow
SUPPORT → unrelated tenant → deny
SERVICE_IDENTITY → allowed table → allow
SERVICE_IDENTITY → privileged unrelated table → deny
```

Also verify that the runtime role cannot silently bypass RLS through ownership or `BYPASSRLS` privileges. PostgreSQL explicitly documents these bypass behaviors. citeturn339893search1

---

# 76. Authorization Test Harness Fixtures

Create reusable fixtures for:

```text
TenantA
TenantB
BranchA1
BranchA2
BranchB1
OwnerA
ManagerA
CashierA
AuditorA
SupportAgent
PlatformAdmin
DeviceA
DeviceB
RevokedDevice
ActiveMembership
RevokedMembership
ActiveEntitlement
ExpiredEntitlement
PendingApproval
ApprovedApproval
InvalidApproval
A1Session
A3Session
```

The fixtures must be deterministic and contain no production data.

---

# 77. Security Regression Catalogue

Each confirmed authorization defect becomes:

```text
incident
↓
root cause
↓
minimal reproducer
↓
regression test
↓
policy correction
↓
negative test
↓
mutation test where useful
↓
threat-model review
↓
ADR review if architecture changed
```

No authorization bug fix is considered complete without a test demonstrating that the previous bypass no longer exists.

---

# 78. CI Enforcement

CI MUST block on:

```text
authorization compilation failures
authz unit test failure
authz integration failure
cross-tenant test failure
cross-branch test failure
property authorization failure
policy parser failure
policy version mismatch
mutation-test regression where mandated
security test harness failure
policy artifact integrity failure
```

The project already requires security checks to fail closed. A missing authorization scanner/test environment is not equivalent to a passing result.

---

# 79. Policy Diff Review

A policy change review must summarize semantic impact, not just textual diff.

Required report:

```text
permissions added
permissions removed
scope widened/narrowed
threshold changed
assurance changed
approval requirement changed
resources affected
roles affected
tenants affected
client versions affected
security controls affected
rollback behavior
```

For high-risk changes, include representative allow/deny decision matrices before and after the change.

---

# 80. Policy Static Analyzer

The initial analyzer should detect:

```text
unknown permissions
unknown resource types
unknown actions
duplicate rules
unreachable rules
conflicting rules
missing default deny
unbounded wildcard scope
wildcard privileged actions
missing assurance on high-risk actions
missing approval for configured threshold classes
```

A policy that compiles but accidentally grants `TENANT_DELETE` to a cashier must be rejected semantically.

---

# 81. Security Control Traceability

Phase 6 directly contributes to the existing 48-control matrix.

Major coverage:

```text
4   authentication boundary
5   missing authorization
6   cross-user access
7   DB privilege defense
9   protected admin routes
15  client-only security
16  input validation
24  recovery authorization
25  session state
28  rate/resource controls
32  server-side payment authority
33  BOLA/IDOR
34  API/user input
37  MFA for privileged actions
38  enumeration resistance
39  business logic abuse
40  race conditions
41  replay handling
45  fail-closed behavior
46  timeouts/resource limits
48  complete endpoint authorization inventory
```

The authoritative 48-control ownership remains in the existing security architecture and security-test harness.

---

# 82. Endpoint Authorization Inventory

Every protected endpoint must map to:

```text
HTTP method
route
operation/action
resource type
authentication requirement
minimum assurance
permission
scope model
property model
state requirements
entitlement requirement
approval requirement
idempotency requirement
rate class
timeout
error mapping
audit class
owner
```

This inventory should be machine-readable so CI can detect routes without declared authorization metadata.

---

# 83. Code Review Rules

Reviewers MUST reject:

```text
role string comparisons in handlers
client-trusted tenant IDs
client-trusted permissions
generic privileged PATCH endpoints
authorization checks only in UI
authorization only in middleware for critical commands
raw SQL without tenant/scope reasoning
cache decisions without versioning
fail-open authorization fallbacks
unbounded policy recursion
unreviewed policy wildcard grants
```

Reviewers should explicitly ask:

```text
Who is the actor?
What is the authority source?
What is the scope?
What resource is targeted?
What state is it in?
What fields are writable?
What happens if policy is unavailable?
What happens if membership changes concurrently?
What is logged?
What proves the control works?
```

---

# 84. Advantages

## Strong tenant isolation

Authority is evaluated from explicit organization and scope state instead of copied role checks.

## Business-aware security

State, thresholds, approvals and entitlements can participate in decisions.

## High performance

An in-process Rust evaluator avoids a network hop on common requests.

## Evolvability

The policy interface can later support an external PDP without leaking provider types into the domain.

## Better testing

Decision predicates are independently testable and suitable for property/mutation testing.

## Better operational evidence

Policy version and reason classes make authorization behavior diagnosable.

---

# 85. Disadvantages

## Implementation complexity

This is materially more complex than `role == MANAGER`.

## Cache invalidation burden

Versioning and revocation semantics must be engineered correctly.

## Policy review burden

Authorization policy is production security configuration and needs governance.

## More integration work

API, application, database, workers, support tools and offline sync all become PEPs.

## Potential latency cost

Resource-state authorization may require database access on critical commands.

These costs are justified because Sitolo's business model is multi-tenant, financial, offline-capable and audit-sensitive.

---

# 86. Why This Over Alternatives

## Alternative A — role-only RBAC

Rejected because role names do not encode scope, resource, state or thresholds.

## Alternative B — authorization entirely in UI

Rejected categorically. The API is directly reachable and clients can be modified or bypassed.

## Alternative C — authorization entirely in PostgreSQL RLS

Rejected as sole authority because application actions, assurance, approvals, entitlements and domain state exceed row visibility semantics.

## Alternative D — remote PDP for every decision

Deferred. It introduces a network dependency and operational complexity before measurements justify it.

## Alternative E — Zanzibar-style distributed authorization service

Deferred. Valuable research direction, but excessive architecture for the initial modular-monolith scale.

## Alternative F — ad-hoc middleware checks

Rejected because internal application paths and workers would bypass them and policy semantics would drift.

---

# 87. Operational Runbook — Authorization Outage

```text
1. Determine whether failure is evaluator, database, cache, policy data or deployment related.
2. Confirm sensitive mutations are failing closed.
3. Confirm low-risk reads have not accidentally gained fail-open behavior.
4. Inspect policy version and checksum.
5. Inspect authorization error rate.
6. Inspect cache health and invalidation lag.
7. Inspect database membership/scope health.
8. Restore authoritative policy source.
9. Re-run authorization smoke tests.
10. Verify cross-tenant denial.
11. Verify privileged action denial when context is unavailable.
12. Close incident with regression evidence.
```

---

# 88. Operational Runbook — Suspected Privilege Escalation

```text
1. Declare security incident.
2. Identify affected policy/permission/resource.
3. Freeze unsafe policy rollout if necessary.
4. Preserve policy version and decision evidence.
5. Revoke compromised sessions/devices/memberships if indicated.
6. Determine blast radius.
7. Run cross-tenant and privilege-escalation test suite.
8. Correct policy.
9. Deploy through protected release path.
10. Verify old authorization path no longer allows.
11. Add regression and mutation tests.
12. Update threat model/ADR/runbook when architecture changed.
```

---

# 89. Operational Runbook — Stale Authorization Cache

```text
1. Compare cached policy/security version with authoritative version.
2. Determine affected tenants/resources.
3. Invalidate namespace or bump version.
4. Force authoritative evaluation for high-risk writes.
5. Confirm revocations take effect.
6. Inspect invalidation delivery metrics.
7. Add regression test if stale authority survived beyond policy.
```

---

# 90. Production Readiness Checklist

```text
[ ] authorization request model typed
[ ] decision object typed
[ ] deny-by-default implemented
[ ] permission catalog integrated
[ ] scope evaluation implemented
[ ] object authorization implemented
[ ] property authorization implemented
[ ] state-aware authorization implemented
[ ] entitlement checks integrated
[ ] assurance/step-up integration implemented
[ ] approval/SoD integration implemented
[ ] policy versioning implemented
[ ] cache versioning implemented
[ ] fail-closed paths tested
[ ] worker authorization tested
[ ] support access tested
[ ] break-glass tested
[ ] offline authority bounded
[ ] PostgreSQL/RLS integration verified
[ ] runtime DB role cannot bypass RLS unintentionally
[ ] decision telemetry bounded
[ ] authorization audit integrated
[ ] endpoint authorization inventory complete
[ ] property-based tests passing
[ ] mutation tests passing where required
[ ] fuzz targets installed
[ ] concurrency tests passing
[ ] CI gates blocking
[ ] rollback path tested
[ ] incident runbooks reviewed
```

---

# 91. Definition of Done

Phase 6 is complete only when:

1. Every protected operation maps to a declared authorization action.
2. Every action maps to explicit permissions and scope semantics.
3. Object-level authorization is enforced.
4. Property-level authorization is enforced where required.
5. State-aware authorization is enforced.
6. Entitlement checks are explicit.
7. Assurance/step-up requirements are executable.
8. Approval/SoD requirements are executable.
9. Policy versions are observable.
10. Authorization caches cannot grant stale authority.
11. Failures fail closed.
12. Workers and support tools use the same security model.
13. PostgreSQL defense-in-depth is active where designed.
14. Cross-tenant and cross-branch tests pass against real PostgreSQL.
15. Race-condition tests cover revocation and privilege changes.
16. Property-based security properties pass.
17. Mutation testing demonstrates meaningful test strength.
18. Authorization events are auditable.
19. Telemetry is bounded and secret-free.
20. CI rejects undeclared/invalid policy changes.
21. Endpoint inventory is complete.
22. Operational runbooks are exercised.
23. No unresolved high-risk authorization bypass remains.

---

# 92. Phase 6 Exit Gate

```text
ARCHITECTURE
[X] policy enforcement boundary defined
[X] local engine selected for initial implementation
[X] external PDP boundary defined but not required

AUTHORIZATION
[X] permission resolution
[X] scope evaluation
[X] object authorization
[X] property authorization
[X] state authorization
[X] entitlement
[X] assurance
[X] approvals / SoD

SECURITY
[X] deny by default
[X] fail closed
[X] tenant isolation
[X] branch isolation
[X] worker enforcement
[X] support/break-glass separation
[X] offline bounded authority

DATABASE
[X] repository scope contract
[X] RLS defense in depth
[X] runtime-role review
[X] SECURITY DEFINER review rules

RELIABILITY
[X] cache versioning
[X] revocation invalidation
[X] timeout/deadline
[X] bounded policy complexity
[X] concurrency semantics

TESTING
[X] negative authorization suite
[X] property tests
[X] mutation tests
[X] fuzz targets
[X] real PostgreSQL tests
[X] race tests

OPERATIONS
[X] telemetry
[X] audit
[X] policy version evidence
[X] incident runbooks
[X] CI gates
```

Phase 7 may begin only after the Phase 6 gate passes and authorization controls are integrated into the security-test framework.

---

# 93. Research and Standards Basis

This specification is grounded in the current Sitolo contracts and current external engineering guidance.

## Project sources

- `business_model_design.md`
- `sitolo.md`
- `system_architecture_design.md`
- `security_architecture_design.md`
- `security_implementation_spec.md`
- `domain_model.md`
- `database_design.md`
- `api_contract.md`
- `auth_authorization_spec.md`
- `phase2_config_secrets_logging_errors_telemetry_implementation.md`
- `phase3_identity_sessions_mfa_device_identity_implementation.md`
- `phase4_tenant_organization_branch_iam_implementation.md`
- `phase5_postgresql_schema_migrations_constraints_rls_implementation.md`
- `testing_strategy.md`
- `threat_model.md`
- `observability_spec.md`
- `ADR-001-025.md`

## External research

OWASP Authorization guidance establishes least privilege, deny-by-default, per-request validation, resource/object authorization, safe failure and authorization testing as core practices. citeturn339893search2

PostgreSQL 18 documentation defines RLS policy behavior, default deny, command-specific policies, `USING` and `WITH CHECK`, and the privilege bypass implications of superusers, `BYPASSRLS` and table owners. citeturn339893search0turn339893search1

PostgreSQL's security guidance requires special care for `SECURITY DEFINER` functions and trusted `search_path` handling. citeturn339893search4turn339893search7

OPA documentation establishes a standard policy decision point / policy enforcement point model and describes local deployment as a way to reduce decision latency and network failure coupling. citeturn339893search3turn339893search5turn339893search9

Google's Zanzibar paper provides evidence for relationship-based authorization at very large scale, including explicit consistency considerations and caching, but does not justify adopting that architecture prematurely in Sitolo. citeturn339893search48turn339893search49

---

# 94. Final Engineering Position

Sitolo's authorization engine is a security-critical execution boundary.

The desired end state is:

```text
                     UNTRUSTED REQUEST
                             |
                             v
                    AUTHENTICATED SUBJECT
                             |
                             v
                 CURRENT SESSION / DEVICE
                             |
                             v
                    MEMBERSHIP / TENANT
                             |
                             v
                    EFFECTIVE SCOPE
                             |
                             v
                    PERMISSION / ROLE
                             |
                             v
                    OBJECT AUTHORIZATION
                             |
                             v
                   PROPERTY AUTHORIZATION
                             |
                             v
                    DOMAIN STATE CHECK
                             |
                             v
                       ENTITLEMENT
                             |
                             v
                     ASSURANCE / MFA
                             |
                             v
                       APPROVAL / SOD
                             |
                             v
                         DECISION
                       /          \
                    ALLOW         DENY
                      |              |
                      v              v
                 TRANSACTION      SAFE ERROR
                      |
                      +--> AUDIT
                      +--> OUTBOX
```

The implementation standard is deliberately strict:

> **Authorization must be explicit, typed, scoped, state-aware, versioned, auditable, testable and fail-closed. A role is not authority. A tenant ID is not authority. A device ID is not authority. A cache entry is not authority. A UI decision is not authority. A policy provider being reachable is not authority. Only a successful evaluation of current trusted security and business context produces authority.**

This gives Sitolo one coherent enforcement model across HTTP requests, application services, PostgreSQL repositories, workers, support tools, exports, payments, EIS, offline synchronization and future service extraction.

The platform must remain capable of answering the fundamental security question for every sensitive operation:

```text
WHO
WHAT
WHERE
TO WHAT
UNDER WHICH POLICY
WITH WHICH ASSURANCE
UNDER WHICH STATE
WITH WHICH APPROVAL
AND WHY
```

That is the required Phase 6 foundation.

**END OF `phase6_authorization_engine_policy_enforcement_implementation.md`**
