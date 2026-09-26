# Sitolo — Phase 15: MRA EIS Implementation Specification

**Document:** `phase15_mra_eis_implementation.md`  
**Phase:** Phase 15 — MRA EIS  
**Product:** Sitolo — Business Operating System for African SMEs  
**Baseline:** 2026-09-26  
**Status:** Implementation-governing contract  
**Program authority:** `docs/implementation_plan.md`  
**Coverage authority:** `docs/phase_contract_coverage_register.md`

> This document is a phase-specific engineering contract. It does not replace higher-authority product, domain, security, database, API, provider, regulatory or commercial sources. Where this document conflicts with a higher-authority source, the higher-authority source wins and the conflict must be recorded and reconciled.

## 0. Executive contract

Phase 15 implements Sitolo's boundary with the Malawi Revenue Authority Electronic Invoicing System. The provider boundary is externally governed: current MRA documentation describes onboarding/terminal activation, configuration, sales and utility functions, and states that third-party POS software is subject to MRA certification. Sitolo must therefore treat MRA behavior as externally verified provider/regulatory truth rather than inventing compliance semantics.

## 1. Dependency position

```text
Previous phase(s)
     ↓
Phase 15: MRA EIS
     ↓
Next phase(s)
```

The phase may not silently bypass unresolved upstream authority, security, data-integrity or migration decisions.

## 2. Mandatory sequential implementation model

Implementation is **sequential and gated**. The phase MUST NOT be implemented as one-shot work.

```text
Part 01 — Select / Reconcile
    ↓ gate
Part 02 — Domain / State
    ↓ gate
Part 03 — Persistence / Transactions
    ↓ gate
Part 04 — Security / Authority
    ↓ gate
Part 05 — Core Implementation
    ↓ gate
Part 06 — Failure / Concurrency / Adversarial Tests
    ↓ gate
Part 07 — Observability / Operations / Recovery
    ↓ gate
Part 08 — Integration / Evidence
    ↓ gate
Part 09 — Semantic Audit
    ↓ gate
Part 10 — Remediation
    ↓ gate
Part 11 — Verification / Delivery
```

**Mandatory rule:** an agent may not implement Part N+1 until the Part N exit gate is explicitly evidenced in the filesystem/PR record. A green build does not waive semantic audit.

Each part MUST produce a durable artifact in the applicable ICM stage output location.

## 3. Authority and standards

Primary internal source: `docs/mra_eis_integration_spec.md`, plus `docs/domain_model.md`, `docs/api_contract.md`, `docs/deployment_spec.md`, `docs/security_implementation_spec.md` and `docs/implementation_plan.md`.

Current MRA developer resources: https://eis-portal.mra.mw/Home/DeveloperResources and https://dev-eis-api.mra.mw/docs/. MRA's published API guide describes onboarding, configuration, sales and utilities and states that third-party POS systems must be certified; certification involves MRA inspection/testing and product/version registration. These are external authority references and must be re-verified before each production compliance claim. citeturn642578search2turn642578search4turn642578search11turn642578search13

Security/engineering reference frameworks: ISO/IEC 27001:2022, ISO 37301:2021, NIST CSF 2.0, NIST SSDF 1.1, OWASP ASVS 5.0.0. No framework or contract in this repository constitutes MRA certification.

---

## Part 01 — Select / Reconcile

### Objective

Freeze the MRA integration boundary from the current provider documentation and the repository's MRA integration specification before implementation.

### Required inputs

- mra_eis_integration_spec.md
- current MRA developer resources
- current certification requirements
- tax/domain/API/deployment/security contracts
- provider test environment if available

### Work

Verify endpoint groups and current authentication/configuration requirements from MRA primary sources. Record verification date, source URL/version, environment and unresolved questions. Separate provider facts from Sitolo assumptions. Confirm current certification process before any “MRA compliant” wording.

### Output

A bounded implementation brief identifying authoritative sources, unresolved decisions, dependencies, acceptance evidence and human approvals.

### Exit gate

```text
[ ] phase scope is explicit
[ ] canonical sources are identified
[ ] no authority conflict is hidden
[ ] dependencies are confirmed
[ ] security controls are mapped
[ ] required evidence is named
[ ] unresolved decisions are recorded
[ ] human approval boundaries are explicit
```

---

## Part 02 — Domain / State

### Objective

Define local tax-submission state independently from the merchant economic transaction.

### Canonical state model

Terminal: UNCONFIGURED / ACTIVATING / ACTIVE / SUSPENDED / REVOKED. Configuration: UNKNOWN / CURRENT / UPDATE_REQUIRED / APPLYING / FAILED. Tax submission: PENDING / SUBMITTING / ACCEPTED / RETRYABLE / REJECTED / UNKNOWN / CORRECTION_REQUIRED.

### Invariants

1. Local sale truth does not become dependent on successful EIS transport.
2. EIS response does not rewrite the economic sale.
3. Configuration version used for a submission is attributable.
4. Credentials are secret and terminal-bound according to provider rules.
5. Duplicate sale submissions are safe according to provider idempotency/evidence semantics.
6. Offline EIS submission follows the provider contract, not generic sync assumptions.
7. Rejected tax submission is explicit and operationally recoverable.
8. Compliance claims require current external evidence.

### Output

State transition and invariant matrix.

### Exit gate

```text
[ ] state machine is explicit
[ ] illegal transitions are explicit
[ ] invariants have owners
[ ] historical truth rules are explicit
[ ] idempotency boundary is explicit
[ ] cross-phase interactions are explicit
```

---

## Part 03 — Persistence / Transactions

### Objective

Persist terminal configuration state, EIS submission attempts, provider observations and certification evidence.

### Persistence authority

Store local terminal/provider identifiers, configuration version/fingerprint, submission identity, attempt state, provider response classification and reconciliation/correction linkage. Avoid storing unnecessary raw provider payloads. Preserve certification and version evidence outside transactional business tables where appropriate.

### Transaction rules

Tax work is decoupled from the sale commit through durable local state/outbox/workers. EIS HTTP occurs outside the business transaction. Status updates from provider evidence are committed atomically with audit state.

### Migration and compatibility

EIS configuration/version changes are trust-sensitive. Use explicit configuration versioning, migration windows and rollback behavior. Provider schema changes require a compatibility test phase before rollout.

### Output

Persistence contract, transaction matrix and migration-risk record.

### Exit gate

```text
[ ] authoritative tables/models are identified
[ ] constraints are defined
[ ] tenant/scope protection is defined
[ ] transaction boundaries are explicit
[ ] idempotency keys/records are durable where required
[ ] migration strategy is compatible
[ ] rollback/recovery behavior is known
```

---

## Part 04 — Security / Authority

### Objective

Secure the external tax boundary, credentials, callback/status evidence and offline behavior.

### Trust model

Treat MRA as an external trust boundary. Provider credentials are provisioned through approved secret/key management. Validate provider responses and configuration identifiers. Never use provider reachability as authorization evidence. Support access to terminal/configuration changes is explicit and audited.

### Required controls

SC-001, SC-002, SC-003, SC-005, SC-008, SC-009, SC-010, SC-012.

### Security tests

```text
invalid credentials
expired terminal credential
wrong terminal ID
wrong taxpayer identity
configuration version mismatch
replayed submission
malformed provider response
provider timeout
provider outage
cross-tenant EIS access
support scope violation
stale offline configuration
```

### Exit gate

```text
[ ] authority source is server-side
[ ] tenant/scope checks are explicit
[ ] privileged operations have policy
[ ] replay/duplicate behavior is defined
[ ] secrets are bounded
[ ] fail-closed behavior is tested
[ ] audit evidence is defined
[ ] security control IDs are mapped
```

---

## Part 05 — Core Implementation

### Objective

Implement an adapter-driven EIS gateway with explicit provider error taxonomy and durable asynchronous processing.

### Mandatory sequence

1. provider fact verification;
2. typed EIS gateway contract;
3. terminal activation/configuration workflow;
4. sale submission adapter;
5. offline submission handling;
6. status/utility adapter functions;
7. durable submission attempts;
8. retry/reconciliation worker;
9. credential/configuration rotation;
10. certification evidence collection.

### Implementation rules

Never infer current MRA behavior from a cached repository statement when the provider documentation has changed. Adapter code must isolate provider-specific types. Never expose provider credentials to clients. Do not mark a transaction compliant merely because an HTTP call succeeds.

### Exit gate

```text
[ ] core use cases implemented
[ ] production code follows approved dependency direction
[ ] no competing authority introduced
[ ] no hidden side effects
[ ] error taxonomy is preserved
[ ] operational limits are enforced
```

---

## Part 06 — Failure / Concurrency / Adversarial Verification

### Required failure classes

activation failure, configuration drift, credential rejection, provider timeout, provider unavailable, malformed response, duplicate submission, accepted-but-response-lost, rejected tax payload, offline queue saturation.

### Concurrency model

Prevent duplicate terminal activation, configuration update races and duplicate tax submission. Worker leases must be durable. Use provider-specific idempotency/evidence rules where available.

### Adversarial tests

forged terminal ID, cross-tenant terminal reference, malicious provider payload, oversized payload, stale configuration replay, credential substitution, unauthorized support reconfiguration.

### Exit gate

```text
[ ] positive and negative tests exist
[ ] retry and replay are exercised
[ ] concurrency is tested where applicable
[ ] denial tests prove no unauthorized side effect
[ ] external timeout/unknown outcomes are tested where applicable
[ ] test evidence is attributable to a revision
```

---

## Part 07 — Observability / Operations / Recovery

### Telemetry

submission success/failure class, unknown age, configuration drift, retry backlog, terminal health, provider latency and certification test results. Never emit raw secrets or sensitive tax payloads by default.

### Operational controls

MRA outage must leave local sales authoritative and create visible tax-submission backlog. Configuration drift must alert before it causes widespread submission rejection when the provider exposes such information.

### Recovery

Reconcile pending submissions from durable attempt identity and provider status. Restore terminal configuration from approved versioned state. Never blindly re-submit unknown outcomes without provider evidence/idempotency semantics.

### Runbooks

- terminal activation failure;
- configuration drift;
- provider outage;
- rejected submission;
- unknown submission outcome;
- credential rotation;
- certification regression.

### Exit gate

```text
[ ] operational metrics are bounded
[ ] critical transitions are attributable
[ ] alerts have owners
[ ] failure modes have response paths
[ ] recovery procedure is executable
[ ] recovery evidence is retained
```

---

## Part 08 — Integration / Evidence

### Integration boundaries

Phase 10 creates the economic sale; Phase 11 reconciles payment where needed; Phase 12 governs offline client commands; Phase 15 translates accepted local tax facts into MRA provider operations; Phase 16 reports tax state separately from economic state.

### Evidence package

- current MRA documentation verification record;
- provider contract fixtures;
- onboarding/configuration tests;
- sale submission tests;
- offline tests;
- credential/secret tests;
- provider failure injection;
- certification artifacts and correspondence where applicable.

### Exit gate

```text
[ ] upstream dependencies are verified
[ ] downstream contracts remain compatible
[ ] integration fixtures are current
[ ] evidence identifies source revision
[ ] no target-state statement is used as proof of implementation
```

---

## Part 09 — Semantic Audit

A separate audit MUST reconstruct this contract and challenge the actual implementation.

### Required audit procedure

```text
1. reconstruct requirements from canonical sources
2. inspect actual code paths and persistence
3. trace authority and tenant scope
4. test state transitions
5. inspect retry/idempotency behavior
6. inspect failure and recovery paths
7. inspect observability and evidence
8. challenge concurrency/race behavior
9. compare implementation with contract
10. produce requirement → implementation → evidence matrix
```

CI and unit tests are evidence only; they are not a substitute for semantic audit.

### Audit outputs

- requirement-to-implementation-to-evidence matrix;
- negative/security control matrix;
- residual-risk list;
- explicit PASS / PARTIAL / FAIL / N/A classification;
- remediation handoff.

### Gate

No phase delivery while a Critical/High semantic defect remains unresolved unless an explicit approved exception exists.

---

## Part 10 — Remediation / Re-audit

Every material finding follows:

```text
finding
 ↓
root cause
 ↓
bounded remediation
 ↓
regression test
 ↓
re-audit
 ↓
verification
```

Remediation MUST NOT silently change the contract to make the implementation appear compliant.

### Exit gate

```text
[ ] all Critical findings closed
[ ] High findings closed or explicitly risk-accepted
[ ] regression evidence exists
[ ] re-audit completed
[ ] residual risk recorded
[ ] register status updated
```

---

## Part 11 — Verification / Delivery

### Final evidence

A production certification package must include current MRA primary-source verification, the certified product/version evidence required by MRA, adapter tests, credential controls, failure handling and release artifact identity.

### Human approval boundary

Human approval is mandatory for production MRA onboarding, terminal activation, taxpayer association, credential provisioning, certification submission and any compliance claim.

### Definition of done

Phase 15 is complete only when the provider boundary is current-source verified, security-controlled, failure-safe, offline-aware, certification-evidenced and operationally supportable.

### Delivery handoff

The ICM delivery artifact MUST identify:

```text
phase
contract revision
implementation revision
test/evidence revisions
known residual risk
migration state
operational readiness
next-phase dependency
```

**End of Phase 15 contract.**

# 12. Current MRA Evidence Discipline

Because the MRA developer portal exposes current developer resources while the published API guide is dated 2024, each production change must re-check the current MRA primary-source material before a compliance statement is made. Current developer resources include the MRA EIS portal and published API documentation. The public API guide describes four functional groupings—onboarding, configuration, sales and utilities—and separately documents third-party POS certification. See:
- https://eis-portal.mra.mw/Home/DeveloperResources
- https://eis-api.mra.mw/docs/eis_api_2.htm
- https://eis-api.mra.mw/docs/api_compliance_certification.htm

The repository must record:

```text
source URL
verification date
document/version
environment
reviewer
changed requirements
impact on adapter/tests
```

# 13. Terminal Configuration Matrix

| Concern | Source of authority | Local representation | Security rule |
|---|---|---|---|
| terminal identity | MRA/provider | terminal reference | tenant-bound, protected |
| activation | MRA/provider | activation state/evidence | credential-controlled |
| configuration | MRA/provider | version/fingerprint | current-version check |
| taxpayer association | MRA/provider + authorized merchant state | explicit binding | privileged action |
| credential | approved secret authority | SecretRef | never ordinary config/log |
| product/version certification | MRA | release evidence | exact artifact identity |

# 14. EIS Submission State Machine

```text
LOCAL_TAX_FACT
   ↓
TAX_SUBMISSION_PENDING
   ↓
SUBMITTING
 ├─ ACCEPTED
 ├─ RETRYABLE
 ├─ UNKNOWN
 └─ REJECTED
        ↓
CORRECTION_REQUIRED where applicable
```

A provider acceptance does not erase the local tax evidence, and a provider rejection does not erase the local economic transaction.

# 15. Provider Configuration Drift

When MRA configuration changes:

```text
detect version change
→ quarantine incompatible work if required
→ retrieve approved configuration
→ validate
→ apply atomically
→ verify
→ resume bounded submission
```

Operating indefinitely on known-outdated configuration is prohibited where current provider rules require an update.

# 16. Offline/EIS Boundary

Offline support must follow the current MRA integration contract.

The client can preserve local work, but final fiscal authority remains governed by server/provider rules. Any EIS offline parameters, sequence rules, signatures, terminal state or later submission behavior must be explicitly sourced from the current MRA contract rather than inferred from generic sync semantics.

# 17. Certification Evidence

The certification pack should distinguish:

```text
MRA-provided requirement
→ Sitolo implementation
→ test result
→ exact artifact/version
→ certification submission
→ MRA decision/record
```

MRA's published documentation states that third-party POS systems are certified through manual inspection/testing and assigned product/version identifiers, and certification can be revoked. citeturn642578search11turn642578search13

# 18. Security and Privacy Constraints

Never expose terminal credentials, activation secrets or provider authentication material to clients or ordinary logs. Minimize tax/customer data stored in provider evidence.

Support access to EIS configuration is JIT, scoped and audited.

# 19. Failure Catalogue

- terminal activation failure;
- current configuration unavailable;
- outdated configuration;
- invalid credential;
- accepted submission with lost response;
- rejected submission;
- provider outage;
- offline queue saturation;
- provider schema drift;
- certification regression after release.

# 20. No-Go Conditions

Stop implementation/release when:

- current provider requirements cannot be verified;
- certification requirements are unknown;
- credentials are stored outside approved secret handling;
- EIS failure can mutate local economic facts;
- offline behavior is being inferred rather than source-backed;
- exact certified artifact/version cannot be identified.

# 21. Downstream Handoff

Phase 15 supplies Phase 20 with current MRA verification, certification records and exact release evidence. It supplies Phase 16 with explicit separation of economic/tax/submission state.
