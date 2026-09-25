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

The current repository contains **86 Markdown files** across the repository, documentation corpus, governance files, and public repository guides. The reference checker must inventory:

1. explicit Markdown links;
2. backtick/path-qualified `.md` references;
3. document-local bare filenames;
4. repository-wide unique basename matches;
5. ambiguous basename matches;
6. missing targets;
7. references to historical documents;
8. compatibility entries.

The inventory must distinguish a true missing target from a reference that is intentionally routed through a compatibility entry.

The current `scripts/ci/check-doc-references` implementation is too conservative for this purpose: its regex captures only bare filenames, so a path-qualified reference such as `docs/foo.md` can be reduced to `foo.md`; nested commercial documents can also be falsely treated as local-only references. The checker therefore needs a reference-resolution upgrade before the corpus can be considered mechanically complete.

---

## 3. Authority hierarchy

The repository's existing hierarchy remains the governing order:

```text
1. Applicable law / regulator requirement
2. Current external provider contract
3. Approved product / business decision
4. Security architecture and security implementation contract
5. Domain model
6. Database / API / integration specifications
7. Testing / observability / deployment specifications
8. ADRs
9. Implementation convenience
```

ICM routing documents do not change this hierarchy. They only control which source is loaded.

---

## 4. High-impact findings

### DOC-001 — Historical enterprise audit is stale
**File:** `docs/enterprise_audit_and_review.md`
**State:** stale
**Reason:** It still says tenancy/authz/domain/persistence/integrations/sync are empty stubs, while current source contains Phase 4 tenancy/authz/domain work and PostgreSQL authority primitives.
**Action:** replace with a current-state enterprise audit based on the current repository baseline; preserve the older findings only as explicitly historical evidence where useful.
**Priority:** P0

### DOC-002 — API architecture contract vs current transport implementation
**Sources:** `docs/system_architecture_design.md`, `docs/api_contract.md`, `docs/implementation_plan.md`, `agent.md`
**State:** implementation/documentation divergence
**Evidence:** the contracts name Axum as the API boundary, but `apps/api/Cargo.toml` has no Axum dependency and `apps/api/src/serve.rs` contains a direct TCP listener/request parser.
**Action:** do not rewrite the contracts as though custom TCP is intended. Record the divergence and resolve it through the appropriate implementation or ADR decision.
**Priority:** P0

### DOC-003 — Phase 8 catalogue language is written as present-tense runtime behavior
**File:** `docs/phase8_product_catalogue_implementation.md`
**State:** temporal/semantic ambiguity
**Reason:** the contract says the API exposes catalogue operations while the current API implementation does not expose a product catalogue surface.
**Action:** change present-tense implementation claims to normative future/phase language and keep current implementation evidence separate.
**Priority:** P1

### DOC-004 — Old Phase 4 enterprise remediation plan is now historical
**File:** `docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md`
**State:** historical
**Reason:** its metadata identifies Phase 4 Part 5 / PR-005 as the current implementation position, while the repository has moved through later Phase 4 work.
**Action:** mark it explicitly historical and prevent current workflow routing from treating it as the current baseline.
**Priority:** P1

### DOC-005 — ICM reference-integrity document contains now-outdated “missing reference” framing
**File:** `docs/icm_reference_integrity.md`
**State:** partially stale
**Reason:** the missing-file class has been repaired; remaining work is semantic integrity, source precedence, historical/current separation, and automated inventory.
**Action:** update its scope to become the standing integrity policy and link this remediation plan.
**Priority:** P1

### DOC-006 — ICM workflow guide still describes the missing-reference state as current
**File:** `docs/agentic_workflow.md`
**State:** partially stale
**Action:** update validation/integrity language to reflect the restored authority paths and the new semantic audit stage.
**Priority:** P1

### DOC-007 — MRA EIS dated transition language must remain historical
**Files:** commercial operating model and any residual EFD/transition references
**State:** semantic freshness
**Evidence:** the repository already records the 2026 EIS transition as complete; the live MRA developer site continues to provide EIS API documentation.
**Action:** current product/compliance language must describe EIS as the current integration path for affected taxpayers; old transition dates may remain only as dated historical context.
**Priority:** P1

### DOC-008 — External standards need dated verification metadata
**Targets:** Rust, PostgreSQL, SLSA, OWASP, NIST, OpenTelemetry and provider/regulatory sources
**State:** recurring freshness risk
**Action:** each externally versioned requirement must carry a verified date and official source; stale versions become historical rather than silently remaining current.
**Priority:** P1

### DOC-009 — Documentation-to-source completion claims are mixed with future-phase contracts
**Targets:** Phase 5, 6, 7, 8, 9, 10 contracts; testing/deployment/observability contracts
**State:** semantic classification needed
**Action:** classify each contract statement as implemented evidence, current requirement, future-phase requirement, or acceptance criterion. No acceptance criterion is evidence merely because it appears as `[X]` or imperative text.
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

- every internal `.md` reference resolves or is explicitly classified as historical/compatibility;
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
