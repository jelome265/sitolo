
# Sitolo — Phase Contract Coverage Register / Audit

**Document:** phase_contract_coverage_register.md  
**Role:** Authoritative reconciliation record for the Sitolo implementation-phase contract corpus  
**Scope:** Sitolo implementation phases 0–20 inclusive  
**Baseline audited:** main at ade3fb82a538823591cb177363b13c8e313937bc (2026-09-26)  
**Last audited:** 2026-09-26  
**Status:** Current audit / governance register

## 0. Authority boundary

This register is the **authoritative record for phase-contract coverage and reconciliation status**.

It answers:

- which implementation phase exists in the approved program;
- which document is the canonical phase contract, when one exists;
- which supporting sources currently feed that contract;
- what authority governs the phase;
- whether the phase contract is complete, partial or missing;
- whether the phase is reachable through the engineering ICM phase router;
- what the repository currently evidences about implementation;
- what verification evidence exists;
- what gaps remain;
- what remediation is required.

This register **does not become the source of truth for product behavior, domain rules, security controls, legal/regulatory requirements, provider behavior, database semantics, API semantics, or commercial decisions**. Those remain owned by the applicable higher-authority source.

### 0.1 Governing source hierarchy

1. Applicable law / regulator requirement
2. Current external provider contract
3. Approved product/business decision
4. Security architecture and security implementation contract
5. Domain model
6. Database / API / integration specifications
7. Testing / observability / deployment specifications
8. ADRs
9. Implementation convenience

This register classifies and reconciles the corpus; it may not silently override a higher-authority source.

### 0.2 Status vocabulary

| Field | Interpretation |
|---|---|
| **Contract completeness** | **Complete** = dedicated phase-level contract has the required implementation-governance depth. **Partial** = phase is defined but its dedicated contract is incomplete or structurally weak. **Missing** = no dedicated phase contract identified. |
| **ICM routing** | **Routed** = discoverable from the engineering phase-context router. **Partially routed** = contract exists but routing is incomplete. **Not routed** = no canonical phase entry is available. |
| **Implementation status** | Current repository state only. Contract presence never proves implementation. |
| **Evidence status** | Strength and currentness of executable or repository evidence supporting implementation claims. |
| **Gap** | A discrepancy between the implementation program and the current contract, routing or evidence corpus. |
| **Required remediation** | Concrete action needed to restore authoritative coverage. |

### 0.3 Phase/workflow separation

Sitolo has two independent numbering systems:

~~~text
Sitolo implementation program: Phase 0 → Phase 20
ICM execution workflow:       01 → 09
~~~

The implementation phases are product/platform dependency stages.

The ICM stages are the execution/control mechanism:

~~~text
01 Select
02 Research
03 Investigate
04 Plan
05 Implement
06 Audit
07 Remediate
08 Verify
09 Deliver
~~~

A phase contract is Layer-3/reference input to ICM work; it is not itself an ICM stage.

---

# 1. Executive coverage result

The implementation program defines **21 numbered phases (0 through 20 inclusive)**.

Current corpus coverage:

~~~text
Dedicated strong phase contracts:      19
Lightweight/partial phase contracts:    1
Foundational phase package, no
 single phase contract:                  1
Missing dedicated phase contracts:       0
~~~

Missing dedicated phase contracts:

~~~text
Phase 11 — Payments / Reconciliation
Phase 12 — Offline Synchronization
Phase 13 — Procurement / Suppliers
Phase 14 — Returns / Refunds / Cash
Phase 15 — MRA EIS
Phase 16 — Reporting / Exports
Phase 17 — Billing / Entitlements
Phase 18 — Admin / Support
Phase 19 — Hardening / Performance / DR
Phase 20 — Production Certification
~~~

Phase 8 has a contract but is materially lighter than the surrounding deep contracts and is not currently exposed through the engineering phase-context router.

Phase 0 has a substantial architectural package, but not a single phase-level implementation contract.

---

# 2. Phase-by-phase reconciliation matrix

| Phase | Canonical contract | Supporting sources | Authority | Contract completeness | ICM routing | Implementation status | Evidence status | Known gaps | Required remediation |
|---|---|---|---|---|---|---|---|---|---|
| **0** | No single phase-level contract. Package is anchored by implementation_plan.md plus foundational design/security/testing/ADR documents. | implementation_plan.md; domain_model.md; database_design.md; api_contract.md; auth_authorization_spec.md; sync_protocol.md; payment_integration_spec.md; mra_eis_integration_spec.md; testing_strategy.md; threat_model.md; observability_spec.md; deployment_spec.md; ADR-001-025.md; security-test/CI foundation | Higher-authority project corpus | **Partial** | Not applicable as a normal product phase entry | Architecture baseline established; no phase-complete claim | Broad repository evidence exists; no single current Phase 0 exit package identified | No canonical phase-level contract; foundational package spans many documents | Create a thin Phase 0 reconciliation contract/index without duplicating subject truth |
| **1** | docs/phase1_repository_rust_workspace_ci_deep_implementation.md | implementation_plan.md; ci_enforcement.md; repository/CI configuration; supply-chain controls | Implementation plan + repository/CI authority | **Complete** | **Routed** | Repository/workspace/CI foundation substantially implemented; phase closure not claimed here | Strong repository and CI evidence; release evidence remains run-specific | Final closure depends on current executable evidence | Keep contract current; close only with revision-attributable evidence |
| **2** | docs/phase2_config_secrets_logging_errors_telemetry_implementation.md | observability_spec.md; deployment_spec.md; security_implementation_spec.md; Phase 2 remediation records | Phase 2 contract + platform/security authority | **Complete** | **Routed** | Runtime foundation implemented in substantial portions; no blanket phase-closure claim | Current source evidence exists; old Phase 0–2 audit is explicitly historical | Historical remediation artifacts can be mistaken for current evidence | Use current source/CI evidence for closure; preserve historical classification |
| **3** | docs/phase3_identity_sessions_mfa_device_identity_implementation.md | auth_authorization_spec.md; security_implementation_spec.md; api_contract.md; observability/security contracts | Phase 3 contract + security hierarchy | **Complete** | **Routed** | Identity/device foundation exists; no phase-complete claim | Repository implementation plus contract-level evidence exist; final closure must be current-head based | No dedicated current Phase 3 closure record identified | Add/maintain explicit closure evidence when formally certified |
| **4** | docs/phase4_tenant_organization_branch_iam_implementation.md | auth_authorization_spec.md; api_contract.md; database_design.md; security_architecture_design.md; Phase 4 Part 8 records | Phase 4 contract + security/database/API authority | **Complete** | **Routed** | **Active Phase 4 work**; Part 8 / audit-outbox stream remains operationally significant | Extensive PR-specific evidence exists; snapshots must match current head | Many Part documents can look authoritative; current-status pointer has stale main SHA | Keep phase contract authoritative; classify PR snapshots; refresh exact-head status evidence |
| **5** | docs/phase5_postgresql_schema_migrations_constraints_rls_implementation.md | database_design.md; security_architecture_design.md; testing_strategy.md; Phase 5 source evidence | Phase 5 contract + database/security authority | **Complete** | **Routed** | PostgreSQL/security foundation materially implemented; full business schema/RLS phase not complete | Strong design/repository evidence; real PostgreSQL closure evidence must be current | Contract completion is ahead of complete business implementation | Enumerate unchecked exit evidence before phase closure |
| **6** | docs/phase6_authorization_engine_policy_enforcement_implementation.md | auth_authorization_spec.md; security_architecture_design.md; security_test_harness.md; Phase 5 contract | Phase 6 contract + security authority | **Complete** | **Routed** | Authorization foundation exists; full production enforcement remains phase-dependent | Strong contract/source evidence; complete endpoint/runtime proof not yet established | [X] gate markers can be misread as implementation proof | Require endpoint inventory and executable negative evidence before closure |
| **7** | docs/phase7_security_test_framework_implementation.md | security_test_harness.md; testing_strategy.md; ci_enforcement.md; threat/security contracts | Phase 7 contract + security/test authority | **Complete** | **Routed** | Security verification framework substantially specified/implemented | Strong contract and CI/test infrastructure evidence; downstream product-surface coverage remains | Framework existence does not prove all future endpoint coverage | Expand executable coverage as later phases add surfaces |
| **8** | docs/phase8_product_catalogue_implementation.md | domain_model.md; api_contract.md; database_design.md; Phase 5/6/9/10 contracts; commercial sources | Phase 8 contract + domain/security/data authority | **Partial** | **Routed** | **Target-only**; current status treats Phase 8 as later target, not active | Contract exists; no evidence that full catalogue runtime exists | Contract remains lighter than adjacent deep contracts | Deepen Phase 8 to the standard contract depth when this phase is prepared for implementation |
| **9** | docs/phase9_inventory_ledger_implementation.md | domain_model.md; database_design.md; Phase 8/10 contracts; security/testing contracts | Phase 9 contract + domain/database/security authority | **Complete** | **Routed** | Later-phase target; not active | Very strong target contract; contract itself is not implementation evidence | Runtime is not complete merely because target contract exists | Build against contract and attach executable evidence at implementation time |
| **10** | docs/phase10_pos_sales_implementation.md | domain_model.md; api_contract.md; database_design.md; Phase 9 dependencies; payment/sync/EIS interfaces | Phase 10 contract + domain/API/database/security authority | **Complete** | **Routed** | Later-phase target; not active | Strong target contract; no proof that complete POS runtime exists | Extensive target gates can be mistaken for completion | Maintain normative target; implement/verify only when dependencies are closed |
| **11** | **Missing** | payment_integration_spec.md; domain_model.md; api_contract.md; database_design.md; security_implementation_spec.md; implementation_plan.md | Implementation plan + provider/security/domain authority | **Complete** | **Routed** | Future target; not active | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 11 sequentially and collect executable payment/reconciliation evidence; update the register with revision-attributable evidence. |
| **12** | **Missing** | sync_protocol.md; domain_model.md; database_design.md; api_contract.md; security/testing/deployment contracts | Implementation plan + sync/security/data authority | **Complete** | **Routed** | Future target; not active | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 12 sequentially and collect executable sync/replay/revocation evidence; update the register with revision-attributable evidence. |
| **13** | **Missing** | domain_model.md; api_contract.md; database_design.md; agent.md; commercial supplier context | Implementation plan + domain/data/security authority | **Complete** | **Routed** | Future target; not active | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 13 sequentially and collect procurement/receipt/inventory evidence; update the register with revision-attributable evidence. |
| **14** | **Missing** | domain_model.md; api_contract.md; database_design.md; phase10_pos_sales_implementation.md; agent.md | Implementation plan + domain/financial/security authority | **Complete** | **Routed** | Future target; not active | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 14 sequentially and collect correction/refund/cash evidence; update the register with revision-attributable evidence. |
| **15** | **Missing** | mra_eis_integration_spec.md; domain_model.md; api_contract.md; deployment_spec.md; security_implementation_spec.md | Applicable MRA authority + current provider contract + Sitolo security/integration hierarchy | **Complete** | **Routed** | Future target; not active | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 15 sequentially and collect current MRA/certification evidence; update the register with revision-attributable evidence. |
| **16** | **Missing** | api_contract.md; database_design.md; observability_spec.md; security_implementation_spec.md; implementation_plan.md | Implementation plan + data/security/observability authority | **Complete** | **Routed** | Future target; not active | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 16 sequentially and collect report/export security and workload evidence; update the register with revision-attributable evidence. |
| **17** | **Missing** | docs/commercial/06_payments_finance/payment_collections_and_subscription_billing.md; domain_model.md; api_contract.md; database_design.md; security contracts | Approved commercial decisions + implementation/security hierarchy | **Complete** | **Routed** | Future target; not active | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 17 sequentially and collect billing/entitlement reconciliation evidence; update the register with revision-attributable evidence. |
| **18** | **Missing** | security_control_register.md; security_incident_response.md; security_exception_register.md; agent.md; implementation_plan.md | Security governance + implementation plan + support governance | **Complete** | **Routed** | Future target; not active | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 18 sequentially and collect privileged-access/support evidence; update the register with revision-attributable evidence. |
| **19** | **Missing** | deployment_spec.md; observability_spec.md; database_design.md; testing_strategy.md; implementation_plan.md | Implementation plan + deployment/reliability/security authority | **Complete** | **Routed** | Future target; not active | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 19 sequentially and collect measured hardening/restore/rollback evidence; update the register with revision-attributable evidence. |
| **20** | **Missing** | implementation_plan.md; ci_enforcement.md; deployment_spec.md; security_architecture_design.md; security_control_register.md; release-governance-evidence.md; external_standards_verification_register.md | All applicable higher-authority sources; production governance | **Complete** | **Routed** | Future certification target | Contract now exists; implementation/evidence remain target-state until executable proof exists. | Phase-level implementation evidence does not yet exist; supporting domain/provider/platform sources remain upstream authorities. | Implement Phase 20 only after upstream evidence is complete; collect exact-candidate certification evidence; update the register with revision-attributable evidence. |

---

# 3. Reconciliation findings

## 3.1 Phase 0

Phase 0 is information-rich but contract-structurally incomplete. The implementation plan defines a sixteen-part foundation package, and those underlying artifacts exist. The missing piece is a single phase-level reconciliation contract that tells agents how that package fits together without duplicating subject authority.

The existing phase0-2 implementation audit is explicitly historical and cannot be promoted into current Phase 0 authority.

## 3.2 Phases 1–7

The dedicated contract layer is mature. The principal remaining issue is evidence discipline:

~~~text
contract exists
      ≠
implementation exists
      ≠
verification exists
      ≠
phase is closed
~~~

This rule applies to all phase rows, including contracts containing acceptance checklists.

## 3.3 Phase 4

Part-specific Phase 4 documents are valid supporting evidence and historical implementation records. They do not replace the canonical Phase 4 contract.

The hierarchy remains:

~~~text
Phase 4 contract
    ↓
Part-specific binding contract
    ↓
current implementation/status evidence
    ↓
verification evidence
~~~

## 3.4 Phase 8

Phase 8 is both a contract-depth problem and an ICM routing problem. Its canonical contract exists, but the engineering phase-context router currently skips it.

A phase that exists only by repository-wide discovery is not sufficient for the intended bounded ICM context-loading model.

## 3.5 Phases 11–20

The repository is not missing the underlying ideas. It is missing the phase-level reconciliation boundary.

Supporting material already exists for payments, sync, procurement, returns/refunds/cash, EIS, reporting/exports, commercial billing, support/admin, hardening/DR and certification.

The remediation therefore must be **reconciliation, not invention**.

---

# 4. Cross-phase contract minimum

Every phase contract must reconcile, where applicable:

~~~text
AUTHORITY
TENANT SCOPE
STATE MACHINE
INVARIANTS
PERSISTENCE AUTHORITY
IDEMPOTENCY
AUDIT EVIDENCE
TELEMETRY
NEGATIVE TESTS
FAILURE TESTS
RECOVERY PATH
MIGRATION REVIEW
PERFORMANCE BASELINE
CI / RELEASE GATES
~~~

A phase document that omits material controls from this set is Partial even if its happy path is described in detail.

---

# 5. ICM integration rule

The phase contract belongs in ICM as bounded reference material.

~~~text
implementation phase
        ↓
phase contract
        ↓
ICM stage contract
        ↓
selected canonical references
        ↓
implementation / audit / verification artifacts
~~~

The phase register is itself a governance/reference artifact. It is **not a new ICM execution stage** and must not become a second orchestration engine.

The ICM model requires one-way references and canonical sources so that context is routed deliberately rather than rediscovered by the agent. The phase register follows the same principle.

---

# 6. Anti-drift rules

## Rule 1 — Contract existence never proves implementation

No phase may be promoted to Implemented or Verified because its contract exists.

## Rule 2 — Historical records never satisfy current coverage

Historical audits remain evidence of prior state and rationale. They require current re-verification before they can support a current-state claim.

## Rule 3 — Supporting specifications are not substitutes for phase contracts

A payment integration specification can support Phase 11 but cannot itself be treated as the complete Phase 11 implementation contract.

## Rule 4 — One canonical home per fact

Phase contracts reference authoritative domain/security/provider sources instead of copying them.

## Rule 5 — Current evidence identifies its revision

Implementation and verification evidence must be attributable to a repository revision, workflow run, artifact or other durable evidence identity.

## Rule 6 — Regulatory/provider truth remains externally governed

Phase 15 may organize the engineering work, but it cannot create regulatory approval or redefine current MRA behavior.

## Rule 7 — ICM routing must match the register

Once a phase contract becomes canonical, the engineering phase-context router must expose it.

## Rule 8 — Coverage gaps are governance gaps

A missing phase contract cannot be silently replaced by agent inference during implementation.

---

# 7. Required remediation program

~~~text
R0  Establish this register as canonical coverage authority
    ↓
R1  Add register to documentation authority/index routing
    ↓
R2  Add register to engineering ICM phase-context inputs
    ↓
R3  Make CI protect register existence
    ↓
R4  Normalize Phase 0 contract/index
    ↓
R5  Deepen + route Phase 8
    ↓
R6  Create Phase 11 contract
    ↓
R7  Create Phase 12 contract
    ↓
R8  Create Phase 13 contract
    ↓
R9  Create Phase 14 contract
    ↓
R10 Create Phase 15 contract
    ↓
R11 Create Phase 16 contract
    ↓
R12 Create Phase 17 contract
    ↓
R13 Create Phase 18 contract
    ↓
R14 Create Phase 19 contract
    ↓
R15 Create Phase 20 contract
    ↓
R16 Re-audit all 0–20 rows
    ↓
R17 Verify ICM routing and current-head evidence
~~~

Phase creation work must be driven from this register and the authoritative source hierarchy, not filename assumptions.

---

# 8. Client architecture gap recorded by this audit

The implementation program distributes mobile and desktop work across product phases rather than giving clients a dedicated implementation phase:

~~~text
Phase 3  authentication/device
Phase 4  organization/branch context
Phase 6  authorization
Phase 8  catalogue
Phase 9  inventory
Phase 10 POS
Phase 11 payments
Phase 12 offline/sync
Phase 13–18 corresponding operations
Phase 19 hardening
Phase 20 certification
~~~

This leaves an explicit program-design decision:

~~~text
Option A:
Mobile + desktop are cross-phase delivery surfaces

OR

Option B:
Introduce explicit client workstreams/phases
~~~

This register records the ambiguity but does not choose between those designs.

---

# 9. Repository integrity findings

## 9.1 Current-status SHA drift

docs/current_implementation_status.md currently records:

~~~text
623f7aed6105664d10d1a2802480fde8316ed5f8
~~~

The audited main baseline is:

~~~text
ade3fb82a538823591cb177363b13c8e313937bc
~~~

Therefore the current-status pointer should be refreshed before it is relied upon as exact-head evidence.

## 9.2 Phase 4 snapshot drift risk

PR-specific Phase 4 records must be interpreted against their recorded revision and the actual current main/PR head. They are not standalone current-state authority.

## 9.3 Phase 8 routing omission

Phase 8 is currently absent from the canonical engineering phase-context input table even though its contract exists.

---

# 10. Acceptance criteria

~~~text
[ ] Phases 0–20 enumerated exactly once
[ ] Program definitions match implementation_plan.md
[ ] Every row identifies a canonical contract or explicitly says Missing
[ ] Supporting sources are named without replacing canonical authority
[ ] Authority boundary is explicit
[ ] Contract completeness is explicit
[ ] ICM routing is explicit
[ ] Implementation status is separate from contract status
[ ] Evidence status is separate from implementation status
[ ] Known gaps are explicit
[ ] Required remediation is explicit
[ ] Historical artifacts are not promoted to current authority
[ ] Phase 8 routing defect is recorded
[ ] Missing Phases 11–20 are recorded individually
[ ] Current-head drift is recorded
[ ] Register is discoverable through canonical docs/ICM routing
[ ] CI protects register existence
~~~

---

# 11. Re-audit rule

Re-run this audit whenever any of the following changes:

- implementation_plan.md;
- a canonical phase contract;
- engineering phase-context routing;
- current implementation-status authority;
- ICM workspace topology;
- phase numbering/dependency order;
- a phase's canonical source set;
- production certification criteria.

A phase row must be re-evaluated from repository state rather than carried forward mechanically.

---

# 12. Final position

The authoritative program is:

~~~text
PHASE 0
  ↓
PHASE 1
  ↓
...
  ↓
PHASE 20
~~~

The contract corpus is currently incomplete:

~~~text
0    PARTIAL
1–7  COMPLETE CONTRACT COVERAGE
8    PARTIAL
9–10 COMPLETE CONTRACT COVERAGE
11–20 MISSING
~~~

This register is the reconciliation boundary between:

~~~text
PROGRAM DEFINITION
       ↓
CANONICAL CONTRACTS
       ↓
ICM ROUTING
       ↓
IMPLEMENTATION
       ↓
EVIDENCE
       ↓
VERIFICATION
       ↓
RELEASE
~~~

No downstream phase is contract-ready merely because its subject matter appears somewhere in the repository.

**End of Phase Contract Coverage Register.**
