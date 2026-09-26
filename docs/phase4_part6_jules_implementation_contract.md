# SITOLO — PHASE 4 PART 6 IMPLEMENTATION CONTRACT

**Repository:** `jelome265/sitolo`  
**Target:** Phase 4 Part 6 / PR-006 only  
**Scope name:** Organization + Branch HTTP API transport boundary  
**Status:** Historical implementation contract  
**Baseline:** Current `main` at the time this contract is created  
**Critical baseline fact:** PR-006 was already merged once as #25, reverted as #27, and reimplemented as #29. Current `main` also contains PR-007/#30. This contract exists to make the Part 6 boundary explicit and prevent scope drift, duplicate architecture, stale-baseline implementation, or accidental Phase 5/6 work.

---
> **Lifecycle classification:** Historical implementation contract. This document governed a completed PR-specific change and is retained for traceability. It is not a current implementation task. Current state must be verified against the repository source and the current phase/PR status documents.

# 1. NON-NEGOTIABLE EXECUTION RULE

Implement **Phase 4 Part 6 only**.

Part 6 means:

> **Organization and Branch HTTP APIs and DTOs, transport validation, transport error mapping, application-service wiring, route dispatch, and focused tests.**

Do not reinterpret Part 6 as “complete Phase 4”.

Do not implement later Phase 4 PRs.

Do not implement Phase 5.

Do not implement the generalized Phase 6 authorization engine.

Do not create a second tenancy model.

Do not replace the existing Rust modular architecture.

Do not duplicate code that already satisfies this contract.

The current repository state is the source of implementation evidence. The Phase 4 specification is the source of implementation meaning. Earlier phases remain architectural constraints.

---

# 2. REQUIRED DOCUMENT CROSS-CHECK

Before changing source, inspect and reconcile all applicable documents. At minimum:

- `agent.md`
- `docs/phase4_tenant_organization_branch_iam_implementation.md`
- `docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md`
- `docs/phase3_identity_sessions_mfa_device_identity_implementation.md`
- `docs/phase2_config_secrets_logging_errors_telemetry_implementation.md`
- `docs/phase1_repository_rust_workspace_ci_deep_implementation.md`
- `docs/system_architecture_design.md`
- `docs/security_architecture_design.md`
- `docs/security_implementation_spec.md`
- `docs/domain_model.md`
- `docs/database_design.md`
- `docs/api_contract.md`
- `docs/auth_authorization_spec.md`
- `docs/observability_spec.md`
- `docs/testing_strategy.md`
- `docs/ADR-001-025.md`
- relevant current source under `crates/sitolo-api` and `apps/api`

The older `docs/enterprise_audit_and_review.md` is historical evidence and must not override the current repository state.

The current Phase 4 sequence is:

```
PR-001 organization + branch domain primitives
PR-002 membership persistence + lifecycle
PR-003 roles + permissions + assignment model
PR-004 scope model + effective scope resolver
PR-005 invitation workflow
PR-006 organization/branch APIs          <-- THIS CONTRACT
PR-007 repository scope enforcement       <-- already present; DO NOT REIMPLEMENT
PR-008 RLS
PR-009 audit + outbox
PR-010 cache/versioning
PR-011 device/org/branch binding
PR-012 ownership transfer
PR-013 support/admin separation
```

Part 6 ends at PR-006.

---

# 3. CURRENT REPOSITORY FACTS

Current `main` contains:

- Phase 4 PR-001 through PR-007;
- `crates/sitolo-api/src/tenancy.rs`;
- tenancy DTOs and transport handlers;
- `apps/api/src/state.rs` tenancy-service wiring;
- `apps/api/src/serve.rs` tenancy route dispatch;
- scoped tenancy repository behavior from PR-007.

The existing PR-006 implementation was merged in #29.

The implementation must therefore be treated as **existing baseline code to verify and correct**, not as permission to blindly recreate the feature.

If the existing code already satisfies a requirement, leave it intact.

If existing code partially satisfies a requirement, patch only the missing behavior.

If existing code conflicts with this contract, correct the conflict.

If a proposed change belongs to PR-007+, stop that change and leave it for its governing phase.

---

# 4. EXACT PART 6 SCOPE

## 4.1 Request DTOs

Implement and preserve dedicated request DTOs:

```rust
CreateOrganizationRequest
CreateBranchRequest
```

Required organization request fields:

```text
organization_id
organization_name
owner_membership_id
owner_user_id
default_branch_id
default_branch_name
```

Required branch request fields:

```text
branch_id
name
```

DTOs must:

- deserialize only the declared properties;
- reject unknown properties;
- never deserialize directly into domain or persistence entities;
- remain transport models rather than business authority objects.

Required serde behavior:

```text
#[serde(deny_unknown_fields)]
```

## 4.2 Response DTOs

Implement and preserve:

```
OrganizationResponse
BranchResponse
ProvisionedOrganizationResponse
```

Responses are read projections.

Responses must not expose:

- credentials;
- invitation tokens;
- internal secrets;
- stack traces;
- database details;
- internal policy expressions.

State values must serialize deterministically.

---

# 5. TRANSPORT VALIDATION

Validation occurs before domain/application work.

Minimum bounds already established by the Phase 4 contract:

```text
request body          <= 32 KiB
identifier length     <= 128 bytes
display name length   <= 256 bytes
```

Identifier validation must reject:

- empty values;
- oversized values;
- whitespace;
- CR/LF injection;
- unsupported identifier characters.

Name validation must reject:

- empty/whitespace-only values;
- oversized values.

Malformed input must never reach a persistence operation.

Body limits must be enforced before JSON deserialization.

Do not add unbounded arrays, recursive payloads, or generic dynamic JSON structures.

---

# 6. EXACT HTTP COMMAND SURFACE

Implement only these Part 6 commands.

## Organization provisioning

```http
POST /v1/organizations
```

Semantics:

```text
request validation
    ↓
typed IDs
    ↓
TenancyService::provision_organization
    ↓
organization + owner membership + default branch
    ↓
201 Created
```

The provisioning bundle must remain atomic according to the existing application/persistence contract.

## Organization lifecycle

```http
POST /v1/organizations/{organization_id}/activate
POST /v1/organizations/{organization_id}/suspend
POST /v1/organizations/{organization_id}/resume
POST /v1/organizations/{organization_id}/begin_close
POST /v1/organizations/{organization_id}/close
```

Each command must:

1. validate the path identifier;
2. construct the typed `OrganizationId`;
3. call the existing `TenancyService` operation;
4. map domain failure into the existing public error model;
5. return the bounded response projection.

## Branch creation

```http
POST /v1/organizations/{organization_id}/branches
```

Semantics:

```text
validate org id
validate branch DTO
    ↓
typed OrganizationId + BranchId
    ↓
TenancyService::create_branch
    ↓
BranchResponse
    ↓
201 Created
```

The owning organization must be enforced by the existing scoped tenancy semantics.

## Branch lifecycle

```http
POST /v1/organizations/{organization_id}/branches/{branch_id}/activate
POST /v1/organizations/{organization_id}/branches/{branch_id}/suspend
POST /v1/organizations/{organization_id}/branches/{branch_id}/resume
POST /v1/organizations/{organization_id}/branches/{branch_id}/begin_close
POST /v1/organizations/{organization_id}/branches/{branch_id}/close
```

A branch identifier from another organization must never be treated as belonging to the path organization.

---

# 7. DO NOT IMPLEMENT THE REST OF THE PHASE 4 API INVENTORY

The Phase 4 specification contains a larger representative API surface.

Part 6 does **not** authorize implementation of:

```text
GET organization collection
GET organization
PATCH organization
invitation APIs
membership APIs
role APIs
scope-grant APIs
ownership-transfer APIs
support APIs
device-binding APIs
cache APIs
audit APIs
outbox APIs
```

The representative inventory is architectural guidance. The PR-006 sequence defines the implementation boundary.

Do not inflate Part 6 into a generic CRUD API.

---

# 8. ERROR MAPPING

Consume the existing Phase 2/3 error architecture.

Required public mapping:

| Domain condition | Public response |
|---|---:|
| invalid identifier/name/input | 422 |
| malformed JSON / unknown fields | 422 |
| organization or branch not found | 404 |
| conflict | 409 |
| illegal lifecycle transition | 409 |
| terminal-state mutation | 409 |
| rate-limited tenancy operation | 429 |
| unexpected internal failure | 500 |

Never leak:

```text
SQL errors
database topology
Rust panic text
stack traces
filesystem paths
provider credentials
internal policy expressions
secret material
```

Do not invent a second error system beside `AppError`.

Do not silently turn security-sensitive failures into success responses.

---

# 9. ROUTE DISPATCH REQUIREMENTS

The existing probe server is still a constrained reference HTTP boundary.

Preserve:

- bounded connection handling;
- request-size limits;
- timeout behavior;
- concurrency/backpressure limits;
- deterministic shutdown.

Route matching must be exact.

Reject unknown route shapes rather than interpreting partial prefixes as valid commands.

Path segments must not accidentally shift a branch identifier into an action slot.

Unknown actions return the existing generic not-found behavior.

No client-controlled path selector becomes proof of authority.

---

# 10. SECURITY HARD RULES

## 10.1 Tenant identifiers are selectors

A request path such as:

```
/v1/organizations/org-b/branches/branch-b1
```

does not prove authority over `org-b`.

The existing service/repository scope semantics remain authoritative.

Never add:

```text
if path organization matches body organization
    then allow
```

as an authorization rule.

Matching identifiers are still only selectors.

## 10.2 Owner identity is not a client trust boundary

The current reference DTO contains `owner_user_id`.

That field must not become a production authorization mechanism.

Do not implement fake authentication.

Do not trust arbitrary client-supplied identity as proof of ownership.

Do not invent a security context inside Part 6.

Preserve the documented reference-implementation placeholder until the governing authentication/authorization phases provide the real authenticated principal boundary.

## 10.3 Cross-tenant behavior

For organization/branch operations:

```text
valid identifier
+
wrong tenant binding
=
deny
```

No cross-tenant existence disclosure.

The existing PR-007 predicate enforcement must be preserved.

## 10.4 No property-level privilege injection

Do not deserialize JSON directly into:

```
Organization
Branch
Membership
RoleAssignment
ScopeGrant
```

Dedicated DTOs are mandatory.

---

# 11. APPLICATION WIRING

The HTTP boundary must call the existing:

```
TenancyService
```

Do not move business invariants into:

```
apps/api/src/serve.rs
```

The API transport layer owns:

```
decode
validate
type conversion
dispatch
error mapping
response projection
```

The application layer owns:

```
use-case orchestration
transaction semantics
domain invocation
```

The domain owns:

```
state machines
business invariants
typed identifiers
```

The persistence layer owns:

```
repository semantics
tenant predicates
reference storage
```

Keep this direction:

```
API
 ↓
APPLICATION
 ↓
DOMAIN
```

Do not reverse it.

---

# 12. FILES ALLOWED TO CHANGE

Primary Part 6 implementation files:

```
crates/sitolo-api/src/tenancy.rs
crates/sitolo-api/src/lib.rs
apps/api/src/state.rs
apps/api/src/serve.rs
```

Focused tests may live in:

```
crates/sitolo-api/src/tenancy.rs
apps/api/src/serve.rs
apps/api/tests/*
```

Dependency manifests may change only when an actual Part 6 compile-time requirement is missing:

```
crates/sitolo-api/Cargo.toml
apps/api/Cargo.toml
Cargo.lock
```

A lockfile change requires an actual dependency graph change.

---

# 13. FILES THAT MUST NOT CHANGE FOR PART 6

Do not modify these for feature expansion:

```
crates/sitolo-domain/**
crates/sitolo-tenancy/**
crates/sitolo-application/**
crates/sitolo-persistence/**
migrations/**
infra/**
apps/worker/**
```

Exception:

A strictly necessary compile-fix may touch an upstream interface only when the existing Part 6 implementation cannot compile against the already-merged Phase 4 contracts.

That exception does not permit semantic redesign.

Do not add:

```
PostgreSQL migrations
RLS policies
Redis/cache
audit outbox
worker processing
ownership transfer
support impersonation
generalized policy engine
```

---

# 14. TEST REQUIREMENTS

Part 6 is not complete because DTO unit tests pass.

Minimum focused coverage:

## DTO correctness

```
unknown JSON field → reject
malformed JSON → reject
missing required field → reject
oversized identifier → reject
invalid identifier characters → reject
oversized name → reject
empty name → reject
oversized body → reject
```

## Handler correctness

```
validation failure stops before service call
valid provisioning reaches TenancyService
valid branch creation reaches TenancyService
domain NotFound maps to 404
domain Conflict maps to 409
invalid transition maps to 409
terminal state maps to 409
```

## Route correctness

```
POST /v1/organizations
POST /v1/organizations/{org}/branches
POST organization lifecycle action
POST branch lifecycle action
unknown route → 404
unknown action → 404
malformed route → 404/controlled failure
oversized body → 422
```

## Tenant boundary

At minimum:

```
organization A + branch from organization B → denied
organization A lifecycle operation targeting unknown org → generic 404
```

## State-machine behavior

The API tests must rely on the existing Phase 4 domain/application semantics rather than rebuilding state machines inside the HTTP layer.

## Integration boundary

Use the existing bounded TCP/reference server for end-to-end route dispatch coverage where appropriate.

No test may claim PostgreSQL/RLS behavior unless a real PostgreSQL test actually executes it. That belongs to Phase 5/PR-008.

---

# 15. CONCURRENCY AND ASYNC REQUIREMENTS

HTTP handlers are async.

Do not introduce blocking work into Tokio request paths.

Do not hold application locks across external I/O.

Do not add global mutable HTTP state to fake durability.

Part 6 should remain predominantly I/O-bound:

```
HTTP parse
 ↓
validation
 ↓
application call
 ↓
repository I/O
 ↓
response
```

CPU-heavy functionality does not belong in this phase.

The current in-memory reference store remains a reference implementation. Do not pretend process-local synchronization is production distributed coordination.

---

# 16. RESPONSE SAFETY

Successful responses must be generated from typed response projections.

Do not construct large hand-written JSON strings for business objects.

Use the existing serde serialization boundary.

For simple probe responses, retain the existing safe escaping.

For problem details, preserve the existing structured error contract.

No raw user-controlled field may become a JSON syntax context without proper serialization.

---

# 17. IDENTITY AND AUTHORIZATION BOUNDARY

Part 6 exposes transport paths around tenancy operations.

Part 6 does **not** establish the full actor authorization system.

Therefore:

```
AUTHENTICATION
    ↓
AUTHORIZATION
    ↓
TENANCY OPERATION
```

remains the architectural target, but Part 6 must not invent the missing generalized policy engine.

Existing reference handlers may call the already-defined tenancy service.

Do not add:

```
is_owner
is_admin
role == "OWNER"
```

checks scattered across HTTP handlers.

Those are architecture debt, not authorization.

---

# 18. IDEMPOTENCY BOUNDARY

The existing tenancy implementation has record-level idempotency semantics through explicit identifiers.

Preserve that behavior.

For repeated organization provisioning with identical identifiers and semantics:

```
same semantic request
→ same existing result
```

For conflicting semantics under the same identifiers:

```
same identifier
+
different semantic payload
→ conflict
```

Do not create a second organization or branch because a POST was retried.

Do not invent a separate HTTP idempotency subsystem inside Part 6 unless an already-frozen API contract requires one and the existing implementation is missing it.

---

# 19. RUNBOOK — ROUTE VALIDATION FAILURE

Trigger:

```
422 VALIDATION_ERROR
```

Procedure:

```
1. Capture request_id.
2. Identify endpoint family.
3. Classify failure as body, JSON, identifier, or name validation.
4. Confirm request stopped before domain execution.
5. Confirm no sensitive payload was logged.
6. Confirm no persistence mutation occurred.
7. Add regression coverage if malformed input reached business logic.
```

Do not weaken validation to make a client request pass.

---

# 20. RUNBOOK — CROSS-TENANT BRANCH TARGET

Trigger:

```
organization A
+
branch belonging to organization B
→
DENY / 404
```

Procedure:

```
1. Capture request_id / trace_id when available.
2. Identify authenticated/reference principal context.
3. Record requested organization and branch selectors in controlled debugging evidence only.
4. Verify branch ownership binding through TenancyService/persistence semantics.
5. Confirm response does not disclose branch existence to the wrong tenant.
6. Search for any accidental unscoped repository call.
7. Add a regression test if the path succeeded.
```

Do not repair the problem by adding a second authorization check to the route handler.

Fix the broken boundary.

---

# 21. RUNBOOK — LIFECYCLE CONFLICT

Trigger:

```
409 CONFLICT
```

Procedure:

```
1. Capture request_id.
2. Identify organization/branch identifier.
3. Identify requested lifecycle action.
4. Read authoritative current state.
5. Determine whether the transition is illegal, terminal, or concurrent.
6. Return the stable conflict contract.
7. Do not mutate state merely to satisfy the request.
8. Add regression coverage for the transition if absent.
```

A 409 is not permission to bypass the state machine.

---

# 22. RUNBOOK — DEPLOYMENT ROLLBACK

Part 6 has no new persistent schema.

Rollback path:

```
1. Stop rollout.
2. Redeploy the previous known-good application artifact.
3. Confirm readiness/liveness.
4. Exercise organization/branch smoke routes.
5. Confirm no persistent migration rollback is required.
6. Compare API route behavior against the previous release.
7. Preserve CI and deployment evidence.
```

Do not introduce database rollback steps into a Part 6-only PR.

---

# 23. CI / VERIFICATION GATE

Required verification:

```
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
./scripts/ci/verify
```

Use the repository's canonical verification path where supported.

Also verify:

```
cargo doc --workspace --no-deps
```

when the repository's documentation gate requires it.

A PR description must report actual executed results.

Never report “passed” without execution evidence.

---

# 24. FAILURE MODES TO CHECK BEFORE PR

Explicitly inspect for:

```
JSON parsed before body bound
unknown JSON fields silently accepted
path IDs used as authority
branch looked up without organization binding
tenant mismatch returning another tenant's object
domain errors leaked directly
panic/unwrap introduced in runtime path
blocking code inside async request handling
duplicated tenancy business rules in HTTP
raw JSON assembled unsafely
new dependency added without necessity
existing PR-007 scope semantics bypassed
later-phase functionality mixed into Part 6
```

Any one of these is grounds for rejecting the implementation.

---

# 25. PR SCOPE LOCK

The implementation PR must state explicitly:

```text
Phase: 4
Part: 6
Area: Organization + Branch HTTP APIs and DTOs
Base: current main
```

The PR must also state what remains out of scope:

```text
Phase 5 PostgreSQL/RLS
Phase 6 generalized authorization
PR-008 RLS
PR-009 audit/outbox
PR-010 cache
PR-011 device binding
PR-012 ownership transfer
PR-013 support/admin
```

Use the repository PR template.

Open a PR. Do not push an unreviewed direct-to-main implementation.

---

# 26. DEFINITION OF DONE

Part 6 is done only when all conditions are true:

```
[ ] Dedicated organization/branch DTOs exist and reject unknown fields
[ ] Request bounds are enforced before business execution
[ ] Provisioning command is wired to TenancyService
[ ] Branch creation command is wired to TenancyService
[ ] Organization lifecycle commands are wired
[ ] Branch lifecycle commands are wired
[ ] Typed IDs are used at the application boundary
[ ] Domain errors map to stable public AppError responses
[ ] Cross-tenant branch binding remains enforced
[ ] HTTP routing is exact and bounded
[ ] No business invariant is duplicated in serve.rs
[ ] No secrets or stack traces reach responses
[ ] Focused transport tests pass
[ ] Cross-tenant negative coverage exists
[ ] Workspace verification passes
[ ] Existing PR-007 semantics remain intact
[ ] No Phase 5/6 functionality was added
[ ] PR uses the repository template
[ ] PR body documents tests, security, tenant impact, operational impact, and rollback
```

---

# 27. FINAL EXECUTION COMMAND

Implement the contract above against the **current `main`**.

Inspect first.

Patch only what Part 6 requires.

Do not duplicate an already-correct implementation.

Do not widen the scope.

Run the real verification gates.

Open the PR using the repository template.

Anything outside this contract stays out of the PR.

**Part 6 means HTTP transport for organization and branch operations. Nothing more.**
