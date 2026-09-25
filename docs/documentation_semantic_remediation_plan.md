# Sitolo — Semantic Documentation Remediation Plan

**Document status:** Current remediation plan
**Audit date:** 2026-09-25
**Repository baseline:** `feat/agentic-workflow` at the revision containing the ICM workflow and reference-path repairs
**Purpose:** inventory Markdown references, classify authority, reconcile documented intent with current implementation evidence, and repair stale documentation without creating competing sources of truth.

---

## 1. Audit rule

A filename existing is not enough.

Every referenced document is classified by both **authority** and **truth state**:

| Class | Meaning |
|---|---|
| Canonical contract | Defines intended behavior or governing policy |
| Current implementation evidence | Describes what the repository actually implements now |
| Phase contract | Normative work for a defined future/current phase |
| Historical evidence | Retained for audit/history; must not be routed as current state |
| Compatibility entry | Preserves an old path while pointing to the canonical source |
| Generated/navigation artifact | Derived routing or index; never becomes business authority |

The source code is evidence of current implementation. It does not silently override a normative contract. When a contract and implementation disagree, the finding is an **implementation gap** unless the governing contract itself is stale.

---

## 2. Reference inventory

The initial audit snapshot contained 86 Markdown files in the scoped documentation/repository set. Current counts must always be derived from the repository tree; this document is not the inventory source. The reference checker must inventory:

1. explicit Markdown links;
2. backtick/path-qualified Markdown files references;
3. document-local bare filenames;
4. repository-wide unique basename matches;
5. ambiguous basename matches;
6. missing targets;
7. references to historical documents;
8. compatibility entries.

The inventory must distinguish a true missing target from a reference that is intentionally routed through a compatibility entry.

`scripts/ci/check-doc-references` has now been upgraded to resolve explicit links, path-qualified references, local bare names, unique repository-wide basenames, missing targets, and ambiguous basenames. CI runs it for documentation changes. The checker is the mechanical gate; semantic correctness still requires the authority classification below.

---

## 3. Authority hierarchy

The repository's existing hierarchy remains the governing order:

```text
1. Applicable law and regulatory requirements
2. Signed external-provider contracts and current provider behavior
3. Security architecture invariants
4. System architecture invariants
5. Domain model decisions
6. Database design decisions
7. Product requirements
8. UI assumptions
9. Implementation convenience
```

ICM routing documents do not change this hierarchy. They only control which source is loaded.

---

## 4. High-impact findings

### DOC-001 — Historical enterprise audit is stale
**File:** `docs/enterprise_audit_and_review.md`
**State:** fixed
**Reason:** The document was replaced with a current-state audit tied to the repository baseline and explicitly separates current evidence from future-phase gaps.
**Action:** keep the audit pinned to a dated source revision and refresh it when major implementation boundaries change.
**Priority:** P0

### DOC-002 — API architecture contract vs current transport implementation
**Sources:** `docs/system_architecture_design.md`, `docs/api_contract.md`, `docs/implementation_plan.md`, `agent.md`
**State:** implementation/documentation divergence
**Evidence:** the contracts name Axum as the API boundary, but `apps/api/Cargo.toml` has no Axum dependency and `apps/api/src/serve.rs` contains a direct TCP listener/request parser.
**Action:** do not rewrite the contracts as though custom TCP is intended. Record the divergence and resolve it through the appropriate implementation or ADR decision.
**Priority:** P0

### DOC-003 — Phase 8 catalogue language is written as present-tense runtime behavior
**File:** `docs/phase8_product_catalogue_implementation.md`
**State:** fixed
**Reason:** The contract now explicitly identifies itself as Phase 8 target state and says catalogue runtime/API existence must be proven by source and tests.
**Action:** promote individual requirements only when executable evidence exists.
**Priority:** P1

### DOC-004 — Old Phase 4 enterprise remediation plan is now historical
**File:** `docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md`
**State:** fixed
**Reason:** The document is now explicitly historical and points readers to the current enterprise audit.
**Action:** retain as historical evidence only.
**Priority:** P1

### DOC-005 — ICM reference-integrity document contains now-outdated “missing reference” framing
**File:** `docs/icm_reference_integrity.md`
**State:** fixed
**Reason:** It is now the standing integrity policy and routes semantic remediation through this plan.
**Priority:** P1

### DOC-006 — ICM workflow guide still describes the missing-reference state as current
**File:** `docs/agentic_workflow.md`
**State:** fixed
**Action:** workflow guide now describes the restored authority paths, reference validation, and walk-test requirement.
**Priority:** P1

### DOC-007 — MRA EIS dated transition language must remain historical
**Files:** commercial operating model and any residual EFD/transition references
**State:** fixed at document-structure level
**Evidence:** a dated regulatory verification register now separates current regulator facts from historical transition dates and defines reverification triggers.
**Action:** reverify the register before production certification or external compliance claims.
**Priority:** P1

### DOC-008 — External standards need dated verification metadata
**Targets:** Rust, PostgreSQL, SLSA, OWASP, NIST, OpenTelemetry and provider/regulatory sources
**State:** fixed for the core standards set
**Evidence:** `docs/external_standards_verification_register.md` records Rust 1.98.1, PostgreSQL 18.6, SLSA 1.2, OpenTelemetry Semantic Conventions 1.44.0, OWASP ASVS 5.0.0, and the distinction between final NIST SSDF 1.1 and draft SSDF 1.2, with primary sources.
**Action:** refresh the register when external standards change.
**Priority:** P1

### DOC-009 — Documentation-to-source completion claims are mixed with future-phase contracts
**Targets:** Phase 5, 6, 7, 8, 9, 10 contracts; testing/deployment/observability contracts
**State:** semantic classification needed
**Action:** classify each contract statement as implemented evidence, current requirement, future-phase requirement, or acceptance criterion. No acceptance criterion is evidence merely because it appears as `[X]` or imperative text.
**Priority:** P1

### DOC-010 — Completed PR-specific contracts can be mistaken for active work
**Files:** Phase 4 Part 6/7 contracts and superseded PR-008/PR-009 review documents
**State:** fixed for current routing
**Action:** current `docs/README.md` and ICM phase routing now classify these as historical/snapshot records; completed Part 6/7 contracts carry explicit lifecycle banners, and PR-009 has a current-status pointer.
**Priority:** P1

---

## 5. Known current implementation baseline

The current source evidence establishes the following:

| Area | Current evidence | Classification |
|---|---|---|
| Identity/authentication | `sitolo-auth` contains real protocol/state primitives | implemented foundation |
| Tenancy | `sitolo-tenancy` and `sitolo-domain::tenancy` contain organization/branch/scope primitives | implemented foundation |
| Authorization | `sitolo-authz` contains role, permission, assignment, scope and invitation primitives | implemented foundation |
| API transport | `sitolo-api` contains request validation/error/auth helpers and tenancy transport; `apps/api` serves via direct TCP | partial / implementation gap vs Axum contract |
| Persistence | `sitolo-persistence` contains memory/reference stores and PostgreSQL authority/runtime primitives | partial; Phase 5 schema work remains |
| Observability | `sitolo-observability` contains structured telemetry primitives and bounded dimensions | implemented substrate |
| Domain | domain crate currently exposes tenancy; sales/inventory/etc. are contractual/future work | partial |
| Workers | `apps/worker/src/main.rs` is a Phase 1 scaffold and currently has no worker loop | not implemented |
| Events | `sitolo-events` is a contract crate with no substantive runtime event implementation | not implemented |
| Integrations | `sitolo-integrations` is a contract boundary with no provider runtime implementation | not implemented |
| Offline sync | `sitolo-sync` is a contract boundary with no substantive runtime implementation | not implemented |
| PostgreSQL schema/RLS | Phase 5 contract exists, but current baseline still precedes full schema/migration delivery | future/current phase gate |
| Mobile/desktop clients | not present in current repository topology | future structure |
| Business model | canonical commercial corpus under `docs/commercial/` | canonical reference |

---

## 6. Semantic remediation order

Repairs should proceed in this order:

```text
1. Current enterprise audit
      ↓
2. Authority/history classification
      ↓
3. Reference-resolution and inventory enforcement
      ↓
4. Product/API/security contract temporal-state cleanup
      ↓
5. External-version and regulatory freshness cleanup
      ↓
6. Documentation → source drift audit
      ↓
7. System Map expansion / process verification
      ↓
8. Final documentation integrity verification
```

A documentation repair must not claim implementation completion when the source remains incomplete.

---

## 7. Closure criteria

The documentation audit is closed only when:

- every internal Markdown files reference resolves or is explicitly classified as historical/compatibility;
- no ambiguous bare basename reference remains in a governing document;
- every canonical document has one source of truth;
- every historical audit is clearly marked historical;
- every phase contract distinguishes target requirements from implemented evidence;
- current external versions and regulatory facts have dated official-source verification;
- the current enterprise audit matches the actual repository baseline;
- implementation/documentation divergences have an explicit disposition;
- the System Map has evidence for every object/process marked verified;
- CI reference checks pass on the exact remediation head.

---

## 8. Source notes

ICM's current canonical repository defines one-stage/one-job contracts, layered context loading, plain-text handoffs, selective reference routing and canonical-source discipline. Its current `icm-architect` repository also defines the System Map form for repositories later agents must edit.

Those principles justify keeping business/commercial knowledge in Layer 3 and keeping implementation evidence separate from future-phase contracts.

Official ICM references:
- https://github.com/RinDig/Interpretable-Context-Methodology
- https://github.com/RinDig/icm-architect
