# SITOLO — PHASE 4 PART 5 → PHASE 0 ENTERPRISE AUDIT & REMEDIATION PLAN

**Repository:** `jelome265/sitolo`

**Audit baseline:** `main` at `c72abe5041ccd1607e62b70461cfcb3978e17a30` — merge of Phase 4 PR-005

**Current implementation position:** Phase 4 Part 5 / PR-005 — token-bound invitation workflow

**Review scope:** Phase 4 Part 5, Phase 4 foundations already delivered through PR-001–PR-005, and retrospective remediation of Phase 3, Phase 2, Phase 1, and Phase 0.

**Artifact type:** Audit / remediation plan only. This document does not itself implement source-code changes.

**Production posture:** Not production-ready. The repository has substantially stronger Phase 3/4 foundations than the earlier enterprise audit reported, but durable persistence, production HTTP/business authorization enforcement, durable audit/outbox processing, PostgreSQL/RLS, and production-grade operational controls are still incomplete by design or by phase boundary.

---

## 0. EXECUTIVE DECISION

Sitolo should be treated as an **architecturally disciplined security-oriented foundation that is not yet an enterprise production system**.

The current repository is materially more mature than the September 2026 pre-Phase-4 enterprise audit. Phase 4 PR-001 through PR-005 now provide real organization, branch, membership, role, permission, scope-grant, and invitation state-machine primitives, persisted semantics in an in-memory reference repository, tenant-aware scope resolution, negative cross-tenant tests, token hashing, invitation single-use behavior, and abuse controls. The older audit's statements that `sitolo-domain`, `sitolo-tenancy`, and `sitolo-authz` are empty stubs are therefore no longer current and must not be used as the present-state assessment.

However, those improvements are still primarily **reference implementations and control-plane primitives**. They do not establish production readiness by themselves. The principal production blockers are now concentrated in the boundary between the domain/control-plane model and the real deployment environment:

1. **Durable persistence is still absent.** Phase 4 repositories use an in-memory database; PostgreSQL is intentionally deferred to the subsequent persistence phase.
2. **The invitation workflow is not yet exposed through the production HTTP boundary.** PR-005 explicitly defers HTTP routes/DTOs and durable delivery/outbox.
3. **Audit/outbox durability is still deferred.** Invitation and IAM mutations cannot yet provide the durable evidentiary guarantees required for enterprise operations.
4. **Actor authorization is intentionally deferred to the generalized policy layer.** The current service methods establish target-state correctness but are not the final answer to “who may perform this action?” This must remain a hard release gate before privileged API exposure.
5. **The current rate limiting is process-local.** The invitation abuse control can be bypassed horizontally across multiple service instances and must be replaced or backed by a shared distributed enforcement mechanism before production.
6. **Phase 3 security primitives remain reference-storage based.** `IdentityDatabase` uses a single mutex-backed in-memory state model and cannot support durable multi-instance operation.
7. **The API authentication boundary exists, but the full production business-route enforcement path is not yet complete.** Phase 3 explicitly deferred full route mounting and Phase 4 still precedes the later HTTP integration boundary.
8. **Observability and audit primitives are stronger than before, but operational evidence is not yet durable or complete enough for incident reconstruction across restarts and multiple instances.**
9. **CI validates structural and unit-level quality well, but successful unit tests do not prove PostgreSQL isolation, HTTP authorization, distributed rate limiting, production concurrency, rollback safety, or end-to-end tenant isolation.**
10. **Documentation has already drifted from implementation state.** The older `docs/enterprise_audit_and_review.md` describes pre-Phase-4 conditions and must be explicitly superseded by a current audit baseline to prevent agents and reviewers from implementing against stale facts.

The target state is not “more code.” The target state is a **provable authority chain** from authenticated principal to membership to organization/branch scope to authorization to domain invariants to durable transaction, with negative security properties tested at every boundary.

---

# 1. AUDIT AUTHORITY AND EVIDENCE MODEL

This plan follows the repository's documentation-first engineering rule.

## 1.1 Primary implementation authorities

The remediation work must be evaluated against, at minimum:

- `agent.md`
- `docs/phase4_tenant_organization_branch_iam_implementation.md`
- `docs/phase3_identity_sessions_mfa_device_identity_implementation.md`
- Phase 2 configuration/secrets/logging/errors/telemetry specification
- Phase 1 workspace/repository/CI architecture rules
- Phase 0 architecture/security/domain/API/database/auth contracts
- `security_architecture_design.md`
- `security_implementation_spec.md`
- `domain_model.md`
- `database_design.md`
- `api_contract.md`
- `auth_authorization_spec.md`
- `observability_spec.md`
- `testing_strategy.md`
- threat-model and ADR documents present in the repository

The source code is evidence of current behavior, not authority over documented requirements.

## 1.2 Current-state correction to the older audit

`docs/enterprise_audit_and_review.md` is now historical for several findings. It correctly identified the earlier absence of tenancy, authorization, and persistence implementations, but its current production-health table is stale because PR-001–PR-005 have since been merged.

Remediation rule:

> No future engineering agent may treat the older enterprise audit as an untouched description of the current repository.

It should remain as historical evidence of previously identified weaknesses, while this document becomes the current remediation baseline for Phase 4 Part 5 → Phase 0.

## 1.3 Confidence classes

Each finding in this document is classified conceptually as one of:

- **Observed:** directly visible in the live repository or a merged PR.
- **Documented gap:** explicitly deferred by the governing specification.
- **Derived production risk:** inevitable consequence of an observed/documented gap under real deployment conditions.
- **Future-phase gate:** a concern intentionally owned by a later phase and therefore not a defect in the current phase, but still a production blocker if the system were released before that phase.

No finding should be closed merely because the current reference implementation has a passing unit test.

---

# 2. CURRENT PHASE-4 BASELINE

## 2.1 Phase 4 PR sequence observed on `main`

The current branch history contains the following merged sequence:

| PR | Area | Current disposition |
|---|---|---|
| #15 / PR-001 | Organization, branch, membership primitives and scope resolution | Merged |
| #16 / PR-002 | Membership persistence semantics and lifecycle | Merged |
| #17 / PR-003 | Roles, permissions, role assignment lifecycle | Merged |
| #18 / PR-004 | Scope grants and effective authorization scope | Merged |
| #19 | Documentation / rustdoc warning cleanup | Merged |
| #20 / PR-005 | Token-bound invitation workflow | Merged |

PR-005 is therefore the current Phase 4 position, not PR-001.

## 2.2 What is now real

The current repository now contains real implementations for:

- Organization lifecycle
- Branch lifecycle
- Membership lifecycle
- Persisted-record scope resolution through the Phase 4 reference repository
- Role catalog
- Permission catalog
- Role assignment lifecycle
- Scope model
- Scope grant lifecycle
- Effective scope resolution
- Permission + scope authorization composition
- Token-bound invitations
- Invitation token hashing
- Invitation single-use acceptance
- Invitation expiry and revoke behavior
- Invitation acceptance role/scope materialization from server state
- Invitation abuse-class controls
- Cross-tenant negative tests
- Concurrency tests for invitation acceptance

These should be preserved and extended rather than reimplemented.

## 2.3 What remains deliberately incomplete

The following are still either later-phase work or explicit production-readiness gaps:

- PostgreSQL production repository implementation
- SQL migrations and RLS
- Durable distributed identity/session persistence
- Durable IAM persistence
- Full HTTP/API integration for the Phase 4 workflows
- Durable transactional outbox
- Durable audit event storage and delivery
- Worker processing
- Production delivery of invitations
- Generalized actor authorization / issuer-widening enforcement where explicitly deferred to Phase 6
- Full domain transaction engine
- Full financial, inventory, POS, sync, payment, and EIS production implementation
- Production deployment topology and rollback automation
- End-to-end distributed integration testing

These omissions are not all “bugs.” Some are correctly deferred. The remediation plan must distinguish missing implementation from premature scope expansion.

---

# 3. ENTERPRISE HEALTH ASSESSMENT

| Dimension | Current health | Enterprise assessment |
|---|---|---|
| Repository/dependency boundaries | Good | Strong architecture, but production-boundary enforcement remains incomplete |
| Domain state machines | Good foundation | Phase 4 model is materially improved; must gain durable persistence and API enforcement |
| Authentication primitives | Good foundation | Strong protocol/security model, but reference persistence is non-production |
| Session/device lifecycle | Good foundation | Needs durable multi-instance enforcement and adversarial integration tests |
| Tenant isolation model | Good conceptual + reference implementation | Not production-safe until DB constraints/RLS and end-to-end tests exist |
| Role/permission model | Good | Assignment is structurally safe; actor authority remains a documented future concern |
| Scope model | Good | Must be enforced from HTTP through DB, not only service-level reference tests |
| Invitation security | Good foundation | Token hashing/single-use is strong; delivery, durable state, audit and distributed abuse controls remain |
| Persistence | Red | In-memory reference implementation cannot be the production source of truth |
| Auditability | Amber | In-memory / deferred outbox semantics insufficient for production evidence |
| Observability | Amber-Good | Good telemetry substrate; operational alerting and durable correlation need completion |
| API readiness | Amber-Red | Auth boundary exists but full business route surface and policy enforcement are not yet production-complete |
| Reliability | Amber-Red | State-machine logic is careful, but process-local state and absent durable queues remain major risks |
| Scalability | Red | Mutex-backed reference stores and local rate limiting cannot support horizontal production operation |
| Performance | Amber | CPU-bound auth concerns are understood, but realistic load profiles do not yet exist |
| Testing | Amber-Good | Strong unit/reference tests; insufficient DB, HTTP, distributed and failure-injection coverage |
| Deployment readiness | Red | Persistence, migrations, rollback and operational orchestration remain incomplete |
| Maintainability | Good | Strong crate separation and documentation; stale audit documents are a governance risk |
| Compliance readiness | Red | Durable evidence, financial correctness and production audit trail remain incomplete |

---

# 4. SEVERITY MODEL

## CRITICAL

A vulnerability or reliability defect that could cause catastrophic cross-tenant disclosure, authentication bypass, irreversible state corruption, unrecoverable data loss, or invalid financial/compliance behavior.

**Release policy:** zero unresolved Critical findings.

## HIGH

A defect that can materially compromise security, availability, correctness, or operational control under plausible real-world conditions.

**Release policy:** no unresolved High findings in security/authorization/persistence paths; accepted High findings require explicit architectural sign-off and compensating controls.

## MEDIUM

A material weakness that may cause degraded reliability, observability, maintainability or operational risk but does not directly produce catastrophic compromise.

## LOW

Hardening or maintainability issue that should be remediated before scale but does not independently block the current development stage.

---

# 5. TOP 10 HIGHEST-RISK ISSUES

| Rank | ID | Severity | Risk |
|---:|---|---|---|
| 1 | P45-CRIT-01 | Critical | Phase 4 IAM state is still non-durable / in-memory; restart loses authority state |
| 2 | P45-CRIT-02 | Critical | Tenant isolation is not yet enforced at PostgreSQL/RLS boundary |
| 3 | P45-HIGH-01 | High | Privileged Phase 4 service methods are not the final actor-authorization boundary |
| 4 | P45-HIGH-02 | High | Invitation abuse control is process-local and bypassable across instances |
| 5 | P3-CRIT-01 | Critical | Phase 3 identity/session/device state is still reference-storage based |
| 6 | P3-HIGH-01 | High | Authenticator-family revocation implementation can match all sessions rather than a bounded family |
| 7 | P3-HIGH-02 | High | Authentication/audit events are not yet backed by durable production evidence |
| 8 | P2-HIGH-01 | High | Mutex-poison recovery can preserve potentially inconsistent in-memory state instead of failing closed |
| 9 | P1-HIGH-01 | High | CI verifies static/structural quality but not the critical runtime security properties |
| 10 | P0-HIGH-01 | High | Documentation/audit drift creates a governance failure where agents can implement against stale repository truth |

The remainder of this document defines each finding and its remediation.

---

# 6. PHASE 4 PART 5 — INVITATION WORKFLOW AUDIT

## P45-CRIT-01 — Phase 4 authority state is still non-durable

**Severity:** Critical

**Status:** Open

### What is wrong

PR-002 through PR-005 build Phase 4 repository semantics on `TenancyDatabase`, which is an in-memory reference implementation. This means organizations, memberships, role assignments, scope grants and invitations are not yet durable across process restart or deployment.

### Why it is wrong

The governing architecture makes PostgreSQL the authoritative server state. A multi-tenant control plane cannot rely on process-local state for the canonical source of membership and authority.

### Where it appears

- `crates/sitolo-persistence/src/memory.rs`
- Phase 4 tenancy persistence interfaces introduced by PR-002 and extended by PR-003–PR-005
- PR-016/PR-017/PR-018/PR-020 scope statements explicitly defer PostgreSQL work

### Production failure

A deployment restart can cause:

- memberships to disappear;
- roles to disappear;
- scope restrictions to disappear;
- invitation records to disappear;
- revoked authority to be forgotten;
- state versions to reset;
- previously issued invitation tokens to become invalid or, depending on future reconstruction mistakes, inconsistent.

Multiple instances cannot share a single authoritative IAM state.

### Proper fix

Implement the production TenancyStores adapter on PostgreSQL with:

- durable organization/membership/role/scope/invitation tables;
- immutable IDs;
- foreign keys;
- unique constraints for active/non-terminal membership rules;
- state/version columns;
- conditional updates for state-machine transitions;
- row locks or equivalent serialization for concurrent transitions;
- transaction boundaries matching the service invariants;
- RLS defense-in-depth where required;
- migration tests and compatibility checks.

### Acceptance criteria

- Restart does not alter authority state.
- Two API instances observe identical IAM state.
- Concurrent invitation acceptance has exactly one winner against PostgreSQL.
- Revoked membership remains revoked after restart.
- Branch scope cannot cross organization boundaries at the database layer.
- Integration tests run against real PostgreSQL.

### Priority

P0. Must be resolved before production release.

---

## P45-CRIT-02 — Tenant isolation is not yet enforced at the final data boundary

**Severity:** Critical

**Status:** Open / correctly deferred but release-blocking

### What is wrong

The current Phase 4 model performs strong scope validation inside Rust reference implementations, but the final PostgreSQL enforcement layer does not yet exist.

### Why it matters

Application-level checks alone are not sufficient for a system whose security contract explicitly treats tenant isolation as a layered invariant.

### Failure mode

A future repository query that accidentally omits organization scope could return another tenant's record unless the database independently rejects that query context.

### Proper fix

Implement a defense-in-depth chain:

```text
authenticated principal
  -> membership
  -> effective scope
  -> repository scope object
  -> SQL predicate
  -> PostgreSQL constraint
  -> RLS policy
```

The repository must not accept raw client-supplied `organization_id` as authority.

### Acceptance criteria

The automated security suite must attempt:

- A user from organization A reading organization B.
- A branch from organization A using an object from organization B.
- A revoked member reading previously authorized records.
- A branch-scoped grant accessing a sibling branch.
- An invitation token associated with one organization being combined with a second organization's identifiers.

Every path must deny without sensitive existence disclosure.

---

## P45-HIGH-01 — Actor authority for privileged IAM mutations is not yet the final boundary

**Severity:** High

**Status:** Documented future-phase gate

### What is wrong

PR-003/004/005 deliberately defer issuer authority questions to the generalized authorization engine. The current tenancy service methods enforce the target state's correctness but do not alone establish the final rule for whether an actor is allowed to issue that mutation.

Examples include:

- who may assign a role;
- who may grant a scope;
- who may revoke a grant;
- who may create or revoke an invitation;
- who may widen another member's authority.

### Why it matters

These operations are security-sensitive administrative mutations. A correct target-state model is insufficient if the caller is not authorized to produce that target state.

### Proper fix

Before exposing these methods through privileged HTTP routes, require an explicit authorization decision derived from:

```text
principal
+ active membership
+ current effective scope
+ permission
+ requested target
+ actor-vs-target widening rule
+ assurance / step-up where required
+ policy / SoD requirements
= allow or deny
```

### Acceptance criteria

A lower-privileged member cannot:

- elevate themselves;
- assign a broader role than their own authority permits;
- widen another member outside their effective scope;
- create an organization-wide grant from a branch-only context;
- revoke or replace ownership authority without the documented governance workflow.

### Release rule

Do not expose role/scope/invitation administration endpoints as production APIs until this gate is satisfied, unless a deliberately restricted bootstrap/admin path is separately governed and documented.

---

## P45-HIGH-02 — Invitation rate limiting is process-local

**Severity:** High

**Status:** Open

### What is wrong

PR-005 adds `RateLimiter` state inside `TenancyService` protected by a process-local mutex, with fallback rules when deployment policy is absent.

### Why it matters

The same attacker can distribute invitation abuse over many API instances and obtain an effective limit approximately equal to:

```text
configured_limit × active_instances
```

The control therefore does not provide a global abuse budget.

### Failure mode

An attacker repeatedly:

- creates invitations;
- accepts invitation tokens;
- probes token formats;
- consumes per-organization or per-token controls.

Horizontal scaling weakens the control as instance count increases.

### Proper fix

Separate policy from enforcement:

- policy remains in configuration;
- shared counters live in a distributed, atomic store;
- keys are cryptographically and privacy-safely derived;
- expiry is bounded;
- denial is consistent across instances;
- degraded dependency behavior is fail-closed for high-risk security actions or uses a documented conservative local emergency limiter.

The preferred architecture should be selected according to the repository's deployment contract rather than introducing an arbitrary cache product.

### Acceptance criteria

Load-test N instances and prove that the effective organization/token limit does not scale linearly with instance count.

---

## P45-MED-01 — Invitation creation accepts caller-supplied raw token material

**Severity:** Medium

**Status:** Open for architectural hardening

### What is wrong

The service API accepts `raw_token` as an argument and the caller is responsible for supplying high-entropy material.

### Risk

The PR currently relies on a caller-side invariant for token entropy. A future HTTP or background caller could accidentally use weak randomness or a predictable token generator.

### Proper fix

Move token generation into the trusted service boundary unless the repository explicitly requires caller-generated tokens for a documented reason.

The service should generate the raw token from the injected CSPRNG, hash it immediately, store only the hash, and return the one-time raw token only to the boundary that must deliver it.

### Acceptance criteria

No public application API can create an invitation without going through trusted token generation.

---

## P45-HIGH-03 — Invitation audit evidence is deferred

**Severity:** High

**Status:** Open / PR-009 dependency

### What is wrong

PR-005 explicitly defers audit emission and outbox delivery.

### Why it matters

Invitation issuance, acceptance, revocation, expiry, and abuse throttling are security-sensitive events that must be reconstructable in production.

### Failure mode

An incident investigator cannot reliably answer:

- who created an invitation;
- which organization it targeted;
- what role/scope was proposed;
- when it was accepted;
- which identity accepted it;
- whether it was rate-limited;
- whether it was revoked;
- whether acceptance raced with revocation.

### Proper fix

Implement an outbox-backed audit pipeline in the documented later phase, with durable transaction coupling:

```text
business mutation
+ audit event
+ outbox record
= one database transaction
```

Delivery is asynchronous after commit.

### Acceptance criteria

A committed invitation mutation cannot exist without a durable evidence record.

---

## P45-MED-02 — Invitation sweeper lacks an operational scheduler contract

**Severity:** Medium

**Status:** Open

### What is wrong

PR-005 exposes expiry sweeping behavior, but the actual durable worker/scheduling topology remains deferred.

### Risk

Lazy expiry protects correctness at lookup time, but stale records can accumulate and operational cleanup may never happen if no scheduler invokes the sweeper.

### Proper fix

Define the worker contract in the future outbox/worker phase:

- schedule frequency;
- batch size;
- retry semantics;
- lock ownership;
- idempotency key;
- metrics;
- lag alerting;
- graceful shutdown;
- backpressure.

### Acceptance criteria

Expired invitation backlog stays bounded and measurable under load.

---

# 7. PHASE 4 PR-004 / PR-003 CROSS-CUTTING AUDIT

## P45-HIGH-04 — Scope semantics must remain monotonic and deny-by-default after persistence migration

**Severity:** High

PR-004 correctly models grants as narrowing scope rather than widening authority. This is good and must not regress when moved to PostgreSQL.

### Required invariant

```text
effective_authority
= role_permission_set
∩ organizational_scope
∩ active_grants
∩ operating_state
```

A branch grant must never cover:

- sibling branches;
- another organization;
- an unrelated resource type;
- an inactive membership;
- a revoked grant.

The database representation must encode enough information to make impossible states difficult to create.

---

## P45-HIGH-05 — Role definition versioning must become persisted and auditable

**Severity:** High

PR-003 introduces `ROLE_DEFINITION_VERSION` and a server-side role-to-permission mapping. This is correct. The production system must also make permission-definition changes operationally observable and safely migratable.

### Risk

If a future release changes role composition without explicit version handling, the effective authority of an existing user can change silently.

### Remediation

- persist assignment semantics compatible with the definition version strategy;
- record role-definition version in audit evidence where required;
- add upgrade tests;
- provide a backward/forward compatibility policy;
- prohibit accidental permission drift from merely changing enum mapping.

---

# 8. PHASE 3 — IDENTITY, SESSIONS, MFA, RECOVERY, DEVICE IDENTITY

## P3-CRIT-01 — Identity/session/device state remains reference-storage based

**Severity:** Critical

### Evidence

`crates/sitolo-persistence/src/memory.rs` contains `IdentityDatabase` with `Mutex<IdentityState>`, holding users, sessions, devices, refresh state, MFA state and reset artifacts in process memory.

### Why it is wrong

A production identity service must survive process restart, support horizontal scaling and provide a single authoritative revocation state.

### Failure mode

- session revocations vanish after restart;
- refresh credential state diverges across instances;
- device revocation can be lost;
- password-reset artifacts disappear;
- MFA state can diverge;
- security versions are no longer authoritative.

### Proper fix

Implement PostgreSQL-backed identity repositories with transactional state transitions and unique constraints.

Distributed cache may accelerate reads but must never become the security source of truth.

### Acceptance criteria

A revoked session on instance A is denied on instance B after cache propagation boundaries are crossed.

---

## P3-HIGH-01 — Authenticator-family revocation path is too broad

**Severity:** High

### Evidence

The reference implementation's `revoke_sessions_by_scope` contains an `AuthenticatorFamily` branch that matches every session rather than filtering to a specific authenticator family.

### Why it is wrong

The method name and enum semantics imply bounded revocation, but the implementation can resolve to global session revocation.

### Production impact

A targeted MFA/authenticator security action could revoke unrelated sessions for unrelated devices or sessions. This is an integrity and availability defect and can become an operational incident during compromise response.

### Proper fix

Introduce an explicit authenticator-family relationship on session/credential state and filter strictly by that stable identifier.

Do not use `true` as a broad fallback for a security scope.

### Required tests

- Two users, two authenticator families, several sessions.
- Revoke family A.
- Family A sessions revoked.
- Family B sessions unaffected.
- Same behavior after persistence migration.
- Concurrent family revocation is idempotent.

### Priority

P0 security correctness.

---

## P3-HIGH-02 — Mandatory authentication audit evidence is not durable

### Evidence

The in-memory implementation records audit events through `InMemoryAuditSink`, and the session-establishment path does not make audit durability part of the persistence transaction.

### Risk

Security events can be lost on crash or restart, which breaks incident investigation and can violate evidence expectations.

### Proper fix

For security events that the contract marks mandatory:

```text
state transition
+ durable audit record
= one transaction
```

The event should contain pseudonymous references rather than raw credentials or secrets.

### Acceptance criteria

A successful session creation, revocation, refresh replay, MFA reset, password reset and device revocation each produce durable evidence.

---

## P3-HIGH-03 — Authentication transport boundary exists but full production mounting remains incomplete

**Severity:** High

### Evidence

PR-014 adds the transport-level bearer extraction, input bounds, security-context establishment and generic error mapping, while explicitly deferring full Axum route mounting.

### Risk

The existence of a correct helper does not create a secure production request pipeline. A route can bypass a helper unless the architecture makes the security boundary mandatory.

### Proper fix

Create a route architecture in which protected handlers cannot be mounted without an authentication/authorization layer or use typed extractors that require the required security context.

### Acceptance criteria

- Protected routes fail closed when the context is absent.
- Unauthenticated routes are explicitly marked public.
- There is no generic “optional auth” extractor that downstream code can accidentally treat as trusted.
- Integration tests enumerate protected routes and prove authentication is required.

---

## P3-MED-01 — Reference session store uses one global mutex

**Severity:** Medium/High

### Why it matters

A single lock serializes all reference identity operations and is unsuitable as a performance model for real traffic.

### Remediation

This is acceptable as a deterministic test/reference adapter only if it is clearly isolated from production wiring. The production adapter must use database transactions and indexed queries, not an in-memory global mutex.

Additionally, concurrency benchmarks should be run on the production PostgreSQL adapter before declaring session endpoints scale-ready.

---

## P3-MED-02 — Remaining internal `expect` calls require classification rather than blanket removal

**Severity:** Medium

The repository already performed a hardening pass to eliminate unsafe mutex `.expect(...)` paths and the configuration parser `unreachable!()` panic. Remaining `expect` calls are documented as static-literal/internal invariant guards.

This is acceptable only when all of the following hold:

- the constructor inputs are compile-time/static or mathematically guaranteed;
- an invariant violation indicates an actual programming defect;
- the guard cannot be reached through untrusted input;
- failure cannot corrupt or expose tenant/security state.

Every remaining panic-capable path should be reviewed and classified in a dedicated CI report.

---

## P3-MED-03 — Recovery and refresh concurrency must be proved against real persistence

Unit tests against the mutex-backed model are not enough.

The production remediation must include:

- refresh-token replay race tests;
- simultaneous logout and refresh;
- password-reset versus active-session invalidation;
- device revoke versus request authentication;
- MFA recovery versus existing session elevation;
- clock-skew tests;
- duplicate callback tests.

---

# 9. PHASE 2 — CONFIGURATION, SECRETS, LOGGING, ERRORS, TELEMETRY

Phase 2 is one of Sitolo's stronger foundations. The remediation objective is therefore **preserve and operationalize**, not rewrite.

## P2-MED-01 — Mutex-poison recovery can conceal an invariant failure

**Severity:** Medium

### Evidence

The repository hardening commit replaced several `.lock().expect(...)` calls with recovery using `poisoned.into_inner()`.

### Why this needs review

This improves availability after a panic, but it also means subsequent callers continue operating on a state container that Rust deliberately marked poisoned because a previous thread may have panicked while mutating it.

For security-sensitive or financial state, continuing may be more dangerous than failing closed.

### Remediation rule

Use poison recovery only for state where recovery is explicitly safe and documented.

For critical security/business state:

- prefer durable external state;
- reconstruct from the authoritative database where possible;
- otherwise fail closed rather than silently continuing with potentially inconsistent state.

### Acceptance criteria

Every `into_inner()` recovery site is classified by data criticality and documented.

---

## P2-MED-02 — Telemetry substrate is not equivalent to operational observability

**Severity:** Medium

### Current strength

Structured logging, redaction, trace context and typed telemetry contracts are a strong foundation.

### Gap

Production readiness also requires:

- metric exporters;
- latency histograms;
- error-rate metrics;
- database pool metrics;
- authentication failure rates;
- invitation abuse counters;
- queue depth/lag;
- audit outbox backlog;
- scope-denial rates;
- tenant-isolation violation alarms;
- stale configuration detection;
- alert thresholds;
- runbook-linked alerts.

### Remediation

Define service-level indicators and alerts before production:

```text
API availability
P95/P99 latency
5xx rate
Auth failure rate
Refresh replay rate
MFA failure rate
DB pool saturation
Transaction conflict rate
Outbox lag
Invitation issuance/acceptance rate
Rate-limit lockouts
Audit delivery failures
```

---

## P2-HIGH-01 — Production secrets management needs a real provider boundary

**Severity:** High before production deployment

The Phase 2 configuration model correctly prevents unsafe secret handling patterns, but the deployment must provide a production secret manager/KMS integration consistent with the documented deployment architecture.

### Acceptance criteria

- no plaintext production secrets in repository or CI logs;
- secret rotation without application rebuild;
- access scoped per environment/workload identity;
- audit trail for secret reads and rotations;
- emergency revocation procedure;
- startup fails closed when a required production secret cannot be resolved.

---

## P2-MED-03 — Error taxonomy must be joined to retry policy

Typed errors are necessary but not sufficient.

Every retryable dependency error needs:

- retryability classification;
- bounded exponential backoff;
- jitter;
- maximum attempts;
- timeout;
- idempotency contract;
- dead-letter/manual-remediation behavior where applicable.

No HTTP handler should blindly retry an operation that may already have committed a financial or IAM state change.

---

# 10. PHASE 1 — REPOSITORY / WORKSPACE / CI / ARCHITECTURE

## P1-HIGH-01 — CI does not yet prove runtime security properties

**Severity:** High

### Current state

CI is strong on:

- formatting;
- compiler checks;
- Clippy with warnings denied;
- unit tests;
- release build;
- architecture policy checks;
- phase policy checks.

### Gap

Those gates do not prove:

- PostgreSQL tenant isolation;
- RLS correctness;
- cross-tenant denial under real HTTP;
- distributed rate-limit behavior;
- refresh replay across instances;
- worker/outbox idempotency;
- transaction rollback behavior;
- migration compatibility;
- deployment rollback.

### Remediation

Create higher-level CI gates in later phases while keeping structural checks unchanged:

```text
Tier 0 — static policy
Tier 1 — unit/domain
Tier 2 — integration + PostgreSQL
Tier 3 — HTTP/security integration
Tier 4 — distributed/concurrency
Tier 5 — release/deployment smoke
```

A green Tier 1 build must never be interpreted as production security certification.

---

## P1-MED-01 — Dependency graph governance must be continuously enforced

The repository uses explicit crate boundaries, which is good. As the codebase grows, agents must not circumvent them with convenience dependencies.

### Required controls

- no domain → infrastructure dependency;
- no domain → Axum/HTTP dependency;
- no auth primitive → business domain dependency unless explicitly justified;
- no persistence leakage into domain aggregates;
- no circular crate references;
- no feature flags that secretly introduce prohibited edges;
- dependency additions require threat and license review.

Add a CI report showing the actual graph and diff against the approved architecture.

---

## P1-MED-02 — Dependency review checkboxes are repeatedly left incomplete

Several Phase 4 PR descriptions explicitly mark the lockfile as intentionally changed while leaving cargo advisory, license, and transitive inspection unchecked.

This is a process weakness even when the code is correct.

### Remediation

Every dependency-changing PR must include automated evidence for:

- `cargo audit` or the repository's canonical equivalent;
- `cargo deny` license/source policy;
- locked dependency tree diff;
- advisories introduced/resolved;
- source provenance.

The PR template should make the check machine-verifiable rather than relying on human checkboxes.

---

## P1-LOW-01 — Reusable engineering contracts should be machine-enforced where practical

The repository contains strong documentation and `agent.md`. The next maturity step is to convert critical architecture rules into executable policy where possible:

- prohibited dependencies;
- public crate API rules;
- security-sensitive function annotations;
- forbidden raw tenant IDs in repository APIs;
- mandatory audit emission for classified commands;
- protected-route registration checks;
- migration naming/version policy.

---

# 11. PHASE 0 — BASE CONTRACTS, THREAT MODEL, ADRs, DOCUMENTATION GOVERNANCE

## P0-HIGH-01 — Documentation/audit drift is itself an engineering risk

**Severity:** High

### Evidence

The older `docs/enterprise_audit_and_review.md` describes `sitolo-domain`, `sitolo-tenancy`, and `sitolo-authz` as empty stubs, while the live repository now contains their Phase 4 implementations.

### Why it is wrong

A documentation-driven repository is only as reliable as the freshness of its governing evidence. Stale audits cause future agents to:

- reimplement completed work;
- overlook new regressions;
- mis-rank risk;
- violate current phase boundaries;
- make incorrect security assumptions.

### Proper fix

Adopt an explicit audit-baseline protocol:

1. Record the audited commit SHA.
2. Record the phase/part.
3. Record included and excluded files.
4. Record evidence sources.
5. Mark the document as historical once the audited baseline changes materially.
6. Create a new canonical remediation/audit baseline.
7. Link superseded reports to the current report.

### Acceptance criteria

No audit document can be presented as current without an explicit repository SHA and phase baseline.

---

## P0-HIGH-02 — Security architecture must prove every declared trust boundary has an enforcement point

The Phase 0 documents are strong conceptually. The remediation must map each declared trust boundary to:

```text
boundary
→ authoritative input
→ validator
→ enforcement layer
→ failure behavior
→ audit evidence
→ test
```

For example:

```text
Client organization_id
→ untrusted selector
→ scope resolver
→ trusted EffectiveScope
→ repository API
→ RLS
→ cross-tenant test
```

Any boundary without an enforcement point should be considered incomplete.

---

## P0-MED-01 — Threat model must be kept synchronized with the actual implementation

As invitations, scope grants, device state and multi-tenant persistence were added, the threat model should be refreshed for:

- token theft;
- invitation abuse;
- invitation replay;
- tenant switching;
- role escalation;
- scope widening;
- stale cache authority;
- concurrent revocation;
- cross-instance session divergence;
- insider/support misuse;
- compromised worker identities;
- provider callbacks;
- database-privilege escalation.

A threat model that predates these features is no longer sufficient as the only security review artifact.

---

## P0-MED-02 — ADR coverage must include production persistence and distributed enforcement decisions

Before implementing the production persistence/boundary phases, record ADRs for:

- PostgreSQL transaction model;
- RLS/session-variable or equivalent scope propagation;
- connection pooling and pool sizing;
- distributed rate limiting;
- outbox storage and worker ownership;
- audit durability policy;
- cache authority and invalidation;
- deployment topology;
- rollback strategy;
- operational support/impersonation controls.

Do not make these decisions implicitly in implementation code.

---

# 12. END-TO-END TRUST-BOUNDARY AUDIT

The following flow is the canonical production path:

```text
CLIENT
  |
  | untrusted input
  v
HTTP EDGE
  |
  | TLS / limits / request ID / rate controls
  v
AUTHENTICATION
  |
  | validate credential + session + device + assurance
  v
TRUSTED PRINCIPAL
  |
  v
TENANT / MEMBERSHIP RESOLUTION
  |
  | server-authoritative organization + branch scope
  v
IAM / POLICY
  |
  | permission + scope + actor authority
  v
APPLICATION COMMAND
  |
  | validated command
  v
DOMAIN INVARIANT
  |
  | state transition
  v
DATABASE TRANSACTION
  |
  +--> audit/outbox in same transaction
  |
  v
COMMIT
  |
  +--> asynchronous side effect
  +--> telemetry
```

Any production path that skips one of these layers must have a documented reason and security review.

---

# 13. API SECURITY REMEDIATION MATRIX

## Request parsing

Required controls:

- total body size;
- field-size limits;
- JSON nesting/depth limits where relevant;
- header length limits;
- query cardinality limits;
- pagination bounds;
- content-type enforcement;
- decompression bomb protection;
- explicit timeout.

## Authentication

Required controls:

- authenticated principal extractor;
- session validation;
- device validation where bound;
- assurance checks for sensitive operations;
- credential redaction;
- generic public failures;
- rate limits before expensive CPU work.

## Authorization

Required controls:

- server-authoritative organization/scope;
- permission check;
- branch scope check;
- actor authority check;
- object-level authorization;
- state/property authorization;
- approval/SoD where required;
- entitlement where required.

## Error handling

Public errors must never leak:

- raw database messages;
- SQL text;
- tokens;
- password material;
- secret references;
- internal topology;
- stack traces;
- cross-tenant existence.

---

# 14. DATABASE AND TRANSACTION REMEDIATION

## 14.1 Required data guarantees

The production schema must encode, wherever possible:

- one membership owner/user binding;
- unique active/non-terminal membership per user/org;
- organization/branch ownership;
- grant-to-membership ownership;
- grant-to-organization containment;
- role assignment to membership;
- invitation-to-organization binding;
- invitation token-hash uniqueness;
- terminal-state protection;
- state-version monotonicity.

## 14.2 Transactional rules

The following operations must be atomic:

- organization + owner membership + default branch;
- invitation creation + membership issuance;
- invitation acceptance + membership activation + role/scope materialization;
- role revocation + effective-authority change;
- scope grant/revoke + authority update;
- session establishment + refresh family creation;
- refresh rotation + replay state update;
- security-version bump + required revocation semantics.

## 14.3 Isolation tests

Each migration PR affecting authorization state must run database tests against:

- concurrent writers;
- stale versions;
- row-lock contention;
- serialization failures;
- retry semantics;
- deadlock detection where applicable;
- transaction rollback.

---

# 15. CACHING REMEDIATION

Caching must never become the source of truth for security authority.

## Allowed cache use

- read acceleration;
- immutable/static role metadata;
- short-lived derived permissions;
- configuration that has explicit versioning.

## Forbidden cache authority

A cached membership, role, device or scope decision must never be treated as valid beyond the documented freshness/version boundary when the underlying authoritative record has been revoked.

## Required invalidation

The following events must invalidate or version-bump relevant derived authority:

- membership revoke;
- membership suspend/resume;
- role assignment/revoke;
- scope grant/revoke;
- organization suspend/close;
- branch suspend/close;
- device revoke;
- security-version bump;
- policy version change.

Tests must prove that stale cache cannot preserve authorization after revocation.

---

# 16. QUEUES, OUTBOX, JOBS, AND IDEMPOTENCY

The eventual worker/outbox implementation must satisfy:

```text
DB COMMIT
   |
   +--> OUTBOX EVENT
           |
           v
       WORKER CLAIM
           |
           v
      EXTERNAL EFFECT
           |
           +--> SUCCESS -> ACK
           |
           +--> RETRY -> BACKOFF
           |
           +--> PERMANENT FAILURE -> DEAD LETTER / MANUAL RUNBOOK
```

Every externally visible command must have an idempotency strategy.

Examples:

- invitation delivery;
- payment webhook processing;
- MRA EIS submission;
- notifications;
- receipt generation.

The worker must not infer business authority from stale job payloads. It should re-check required authoritative state when a job is security-sensitive.

---

# 17. PERFORMANCE AND CONCURRENCY REMEDIATION

## CPU-bound work

Never perform expensive password hashing or similarly expensive cryptographic work directly on Tokio worker threads if the implementation is blocking/CPU-heavy. Use the architecture's documented blocking strategy and benchmark it.

## I/O-bound work

Use async database/network clients with explicit timeouts.

## Concurrency

Every security-sensitive state transition must define:

- the conflict key;
- the serialization mechanism;
- optimistic/pessimistic locking choice;
- retry policy;
- idempotency semantics;
- failure outcome.

### Required race tests

At minimum:

- 100 concurrent invitation accepts for one token → exactly one success;
- concurrent invite attempts for same user/org → one active membership invariant;
- concurrent role grants → no duplicate effective grant;
- concurrent scope grant/revoke → deterministic final state;
- concurrent session refresh/replay → one successor and deterministic replay detection;
- revoke versus authorize → authorization must not succeed after the authoritative revoke boundary.

---

# 18. TESTING STRATEGY REMEDIATION

## Layer 1 — Unit

Keep current strong unit coverage for:

- state machines;
- token hashing;
- PKCE;
- MFA;
- role catalogs;
- scope containment;
- invitation rules.

## Layer 2 — Integration

Add real PostgreSQL tests for:

- schema constraints;
- RLS;
- transactions;
- concurrent transitions;
- migrations.

## Layer 3 — HTTP security

Add tests for:

- authentication boundary;
- protected route enforcement;
- tenant switching attempts;
- object-level authorization;
- request limits;
- error redaction;
- CSRF on browser workflows;
- header injection attempts;
- oversized bodies;
- malformed JSON.

## Layer 4 — Distributed

Test at least two service instances for:

- session revocation;
- refresh replay;
- rate limiting;
- invitation single-use;
- cache invalidation;
- outbox ownership.

## Layer 5 — Failure injection

Simulate:

- database outage during authorization;
- database outage after business commit but before response;
- audit sink failure;
- outbox publish failure;
- worker crash after external side effect;
- network timeout;
- provider timeout;
- dependency retry exhaustion;
- clock skew;
- process crash between transactional steps.

---

# 19. OBSERVABILITY / INCIDENT DIAGNOSABILITY

Every security-sensitive transaction should be traceable using correlation fields that do not disclose secrets.

Recommended evidence fields:

- request ID;
- trace ID;
- actor pseudonymous ID;
- organization ID only where policy allows it in logs;
- resource type/ID under safe logging policy;
- action code;
- policy decision;
- reason class;
- state/version transition;
- latency;
- retry count;
- database conflict indicator;
- outbox/job ID.

Never log:

- password;
- raw refresh token;
- raw invitation token;
- session secret;
- TOTP secret;
- recovery code;
- access token;
- provider credentials.

Alerting must distinguish:

```text
AUTHENTICATION FAILURE
AUTHORIZATION DENIAL
TENANT-ISOLATION VIOLATION
RATE-LIMIT SATURATION
DATABASE CONFLICT
OUTBOX LAG
AUDIT FAILURE
DEPENDENCY OUTAGE
```

---

# 20. DEPLOYMENT READINESS GATES

Sitolo is not enterprise deployable until all of the following are true.

## Gate A — Persistence

- PostgreSQL authoritative.
- Migrations automated.
- Backup strategy tested.
- Restore procedure tested.
- Connection pool limits validated.
- RLS/tenant isolation tests green.

## Gate B — Identity

- Sessions durable.
- Refresh replay durable.
- Device revocation durable.
- MFA/recovery durable.
- security-version invalidation durable.

## Gate C — IAM

- memberships durable;
- roles durable;
- grants durable;
- scopes durable;
- invitations durable;
- actor authorization enforced.

## Gate D — Audit/outbox

- mutation + evidence atomic;
- worker retry/idempotency proven;
- backlog observable;
- manual replay/runbook tested.

## Gate E — API

- protected-route enforcement mandatory;
- input bounds;
- timeout controls;
- rate controls;
- standardized errors;
- CSRF protection for cookie-auth browser paths.

## Gate F — Operations

- metrics;
- traces;
- logs;
- alerts;
- runbooks;
- rollback;
- deployment health probes;
- graceful shutdown;
- incident response.

## Gate G — Security verification

- dependency audit;
- secret scanning;
- SAST/static checks;
- integration security suite;
- cross-tenant abuse suite;
- concurrency suite.

---

# 21. TOP 10 HIGHEST-LEVERAGE FIXES

| Rank | Fix | Why it is high leverage |
|---:|---|---|
| 1 | PostgreSQL tenancy + RLS | Converts reference isolation into enforceable production isolation |
| 2 | Durable identity/session/device persistence | Prevents security state loss and enables horizontal scale |
| 3 | Mandatory actor-authorization boundary | Prevents privileged IAM self-escalation before business APIs expand |
| 4 | Durable outbox + audit | Makes side effects and evidence crash-safe |
| 5 | Fix authenticator-family revocation semantics | Removes an immediate correctness hazard in security response |
| 6 | Distributed rate limiting | Preserves abuse controls under horizontal scaling |
| 7 | Protected-route architecture | Prevents accidental bypass of authentication/authorization helpers |
| 8 | PostgreSQL integration/security test tier | Catches real isolation and concurrency bugs before release |
| 9 | Current audit/documentation baseline | Prevents agents from implementing against stale architecture facts |
| 10 | Production SLO/alert/runbook package | Converts telemetry into actual operational control |

---

# 22. PHASED REMEDIATION ROADMAP

## Immediate — P0

1. Correct the `AuthenticatorFamily` revocation implementation so it is family-scoped and race-safe.
2. Mark PostgreSQL persistence and RLS as explicit release blockers.
3. Define and lock the actor-authorization gate for role/scope/invitation administration.
4. Prevent privileged Phase 4 service methods from becoming public API surfaces without authorization enforcement.
5. Create the current audit baseline from this document and mark the older enterprise audit as historical.
6. Add CI-level classification of remaining panic-capable paths in production code.
7. Require dependency security review evidence for all lockfile-changing PRs.

## Short term — P1

1. Implement PostgreSQL tenancy repositories.
2. Implement PostgreSQL identity/session/device repositories.
3. Implement database constraints and RLS.
4. Add migration integration tests.
5. Add protected route/extractor architecture.
6. Add distributed rate-limit enforcement.
7. Add durable audit/outbox transaction coupling.
8. Add real worker scheduling and idempotency.

## Medium term — P2

1. Add complete HTTP integration coverage.
2. Add distributed concurrency tests.
3. Add cache invalidation/versioning if caching is introduced.
4. Add metrics, alerts, dashboards and runbooks.
5. Add deployment smoke tests and rollback verification.
6. Add backup/restore drills.
7. Add performance/load tests with representative merchant workloads.

## Long term — P3/P4 hardening

1. Formalize architecture conformance as machine-verifiable policy.
2. Add security regression suites for every new capability.
3. Add chaos/failure-injection tests for critical workflows.
4. Add team-scale code ownership and review controls.
5. Periodically re-run a full enterprise audit on a pinned commit baseline.

---

# 23. ACCEPTABLE VS NOT ENTERPRISE-GRADE

## Acceptable at the current development stage

- In-memory repositories as deterministic reference implementations when they are clearly not production wiring.
- Domain/unit tests before the production database exists.
- Explicit phase deferral of APIs, PostgreSQL, audit/outbox, and later authorization policy where documented.
- Reference state machines using a global lock if their purpose is deterministic testing.
- Placeholder worker binaries before worker implementation phase.

## Not enterprise-grade for production

- In-memory authority state as the canonical source of truth.
- Client-provided tenant/role/scope treated as trusted authority.
- Public administrative routes calling raw tenancy service methods without actor authorization.
- Local-only rate limits for security-sensitive abuse controls in a horizontally scaled service.
- Security events that disappear on process restart.
- Cross-tenant isolation proven only by unit tests.
- Production deployment without tested migrations and rollback.
- Database schema without durable constraints/RLS where required by the security model.
- Audit reports that do not identify the commit/phase they actually inspected.
- “All tests pass” being treated as equivalent to production readiness.

---

# 24. REMEDIATION IMPLEMENTATION ORDER

The implementation order must respect architectural dependencies.

```text
A. Freeze current Phase 4 invariants
        |
        v
B. Fix immediate security correctness findings
        |
        v
C. PostgreSQL schema + migration contracts
        |
        v
D. PostgreSQL tenancy persistence
        |
        v
E. PostgreSQL identity/session/device persistence
        |
        v
F. RLS + tenant-isolation verification
        |
        v
G. Protected HTTP route architecture
        |
        v
H. Actor authorization / policy enforcement
        |
        v
I. Durable audit + transactional outbox
        |
        v
J. Distributed rate limiting
        |
        v
K. Worker + retry + idempotency
        |
        v
L. Integration / distributed / failure tests
        |
        v
M. Deployment / rollback / recovery drills
        |
        v
N. Production readiness review
```

Do not reverse this order by exposing APIs before durable authority and authorization semantics are established.

---

# 25. DETAILED REMEDIATION CHECKLIST

## Phase 4 Part 5

- [ ] Invitation token generation moved behind trusted CSPRNG service boundary.
- [ ] Invitation token stored only as a cryptographic hash.
- [ ] Single-use acceptance proven against real PostgreSQL.
- [ ] Cross-tenant invitation attempts denied.
- [ ] Revocation/expiry race semantics proven.
- [ ] Role/scope materialization is taken only from server-side invitation state.
- [ ] Actor authorization enforced before invitation mutation.
- [ ] Distributed invitation rate limiting implemented.
- [ ] Invitation audit events durably recorded.
- [ ] Invitation delivery decoupled through transactional outbox.
- [ ] Expiry sweeper has worker/scheduler semantics and metrics.

## Phase 4 PR-004 / PR-003

- [ ] Scope containment preserved after persistence migration.
- [ ] Permission union semantics preserved.
- [ ] Role definition version persisted/observable as required.
- [ ] Role revocation remains terminal per assignment record.
- [ ] Widening rule enforced.
- [ ] Approval/SoD requirements integrated where required.
- [ ] Cross-tenant grant manipulation denied.
- [ ] Stale cached authority cannot survive a revoke boundary.

## Phase 4 PR-002 / PR-001

- [ ] Membership uniqueness enforced in DB.
- [ ] State transitions use concurrency-safe conditional writes.
- [ ] Terminal states cannot reactivate.
- [ ] Organization/branch ownership is structurally enforced.
- [ ] Provisioning bundle is atomic.
- [ ] Organization and default branch lifecycle survive restart.
- [ ] Scope resolution is based on persisted trusted identifiers.

## Phase 3

- [ ] PostgreSQL identity repositories.
- [ ] Durable sessions.
- [ ] Durable refresh rotation/replay detection.
- [ ] Durable device identity.
- [ ] Durable MFA state.
- [ ] Durable recovery artifacts.
- [ ] Durable security-version state.
- [ ] Authenticator-family revocation fixed and tested.
- [ ] Protected route mounting mandatory.
- [ ] Full authentication integration suite.
- [ ] Audit evidence durable.

## Phase 2

- [ ] Production secret-manager integration.
- [ ] Alerting and dashboards.
- [ ] Pool/backpressure metrics.
- [ ] Retry/timeout policy matrix.
- [ ] Mutex poison recovery classification.
- [ ] Security event durability semantics.
- [ ] Operational runbooks linked to alerts.

## Phase 1

- [ ] Architecture graph policy machine-enforced.
- [ ] Dependency audit automated.
- [ ] Cargo license/provenance policy automated.
- [ ] Integration test tier in CI.
- [ ] Database migration verification in CI.
- [ ] Release artifact provenance.
- [ ] Reproducible build evidence.

## Phase 0

- [ ] Current audit baseline with pinned commit SHA.
- [ ] Historical audit labeling.
- [ ] Threat model synchronized.
- [ ] ADRs updated for persistence/distributed controls.
- [ ] Every trust boundary mapped to a real enforcement point.
- [ ] Every critical invariant has a negative test.
- [ ] Production assumptions explicitly documented.

---

# 26. REGRESSION-PREVENTION RULES FOR FUTURE AGENTS

Future agents modifying Sitolo must follow these rules.

## Rule 1 — Never infer authority from identifiers

`organization_id`, `branch_id`, `membership_id`, `role`, `permission` and `device_id` are identifiers or claims until resolved into trusted state.

## Rule 2 — Never bypass the application authorization boundary

Handlers must not manually reconstruct IAM rules.

## Rule 3 — Never move authorization into the UI

UI visibility is not security.

## Rule 4 — Never use cache as the canonical authority source

Cache is a performance mechanism.

## Rule 5 — Every security mutation must be auditable

Where the phase requires durable evidence, the audit record belongs in the same transaction.

## Rule 6 — Every state machine transition needs a race test

A passing sequential unit test is not enough.

## Rule 7 — Every dependency change needs automated advisory review

Do not rely on PR checkboxes alone.

## Rule 8 — Every audit document must identify its baseline

A report without a commit/phase baseline is not authoritative.

## Rule 9 — Do not implement later-phase policy inside earlier-phase domain schema

Preserve phase boundaries.

## Rule 10 — Never declare production readiness from compilation/unit tests alone

Production readiness requires persistence, boundary, concurrency, operational and recovery evidence.

---

# 27. FINAL ENTERPRISE READINESS DECISION

## Current decision

**NOT PRODUCTION READY.**

This is not because the architecture is poor. The opposite is true: the repository now has a strong and unusually explicit conceptual security architecture, disciplined crate boundaries, careful state-machine modeling, negative tests, and a security-conscious Phase 3/4 design.

The system is not production-ready because the most important remaining boundaries are exactly the ones that convert a correct reference model into a real distributed service:

```text
REFERENCE SEMANTICS
        |
        v
DURABLE DATABASE
        |
        v
REAL HTTP BOUNDARY
        |
        v
REAL ACTOR AUTHORIZATION
        |
        v
REAL TENANT ISOLATION
        |
        v
DURABLE AUDIT / OUTBOX
        |
        v
DISTRIBUTED RATE LIMITS
        |
        v
WORKERS / RETRIES / IDEMPOTENCY
        |
        v
INTEGRATION + FAILURE TESTS
        |
        v
DEPLOYMENT / BACKUP / ROLLBACK
        |
        v
PRODUCTION
```

Until that chain is complete and proven, the correct engineering posture is **continue implementation with explicit release gates**, not production deployment.

---

# 28. DEFINITION OF DONE FOR THIS REMEDIATION PROGRAM

The Phase 4 Part 5 → Phase 0 remediation program is complete only when:

1. Every Critical finding is closed with code + test + evidence.
2. No unresolved High security finding remains on a production-exposed path.
3. PostgreSQL is the authoritative production state store.
4. Tenant isolation is defended at application and database/RLS layers.
5. Identity/session/device revocation is durable and multi-instance safe.
6. Role/scope/invitation administrative actions require authoritative actor permission.
7. Invitation security is protected by distributed abuse controls.
8. Audit evidence is durable and transactionally coupled to required mutations.
9. Outbox/worker processing is idempotent and observable.
10. Protected HTTP routes cannot bypass authentication or authorization.
11. Cross-tenant, concurrency, failure-injection and migration tests pass.
12. Deployment rollback and database restore procedures are rehearsed.
13. Alerts map to operational runbooks.
14. Documentation and audits identify their exact source baseline.
15. A new enterprise audit against the final commit can reproduce the security conclusions from repository evidence.

---

# 29. CLOSING ENGINEERING PRINCIPLE

The goal of remediation is not to maximize implementation volume.

The goal is to make the following statement objectively true:

> **For every production operation, Sitolo can prove who acted, what authority they possessed, which organization and branch they were operating in, which policy permitted the action, which domain invariant was satisfied, which durable transaction committed it, what evidence was recorded, what happens under concurrency/failure, and how the system prevents the same action from crossing a tenant boundary.**

That is the enterprise-grade bar.
