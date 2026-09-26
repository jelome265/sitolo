# ICM Reference Integrity

This document is the standing policy for internal Markdown reference integrity in the Sitolo repository.

## Current state

The previously missing authority paths have been restored:

- `security_architecture_design.md`
- `auth_authorization_spec.md`
- `ci_enforcement.md`
- `phase8_product_catalogue_implementation.md`
- `sitolo.md`
- `business_model_design.md` at its former root `docs/` location

The root business-model file is a compatibility entry to the canonical commercial document. It is not a second business-model source.

## Integrity classes

Every document reference must be understood as one of:

| Class | Meaning |
|---|---|
| Canonical | Current source of truth for the subject |
| Contract | Normative requirement for a current or future phase |
| Evidence | Observed implementation or verification evidence |
| Historical | Retained evidence from an earlier baseline; not current state |
| Compatibility | Old path preserved only to route readers to the canonical source |
| Navigation | Index/router derived from other sources |

A filename resolving successfully does not establish authority.

## Required checks

Reference integrity includes:

1. explicit Markdown links resolve from the referring file;
2. Markdown reference-style links and wikilinks resolve deterministically;
3. HTML `href`/`src` Markdown references are checked when present;
4. path-qualified document references resolve deterministically;
3. bare document names resolve locally or to exactly one repository-wide basename;
4. ambiguous bare basenames are rejected;
5. references escaping the repository are rejected;
6. one-home-per-fact is maintained for canonical knowledge;
7. historical documents are not routed as current implementation evidence;
8. target-state contracts are not mistaken for implementation evidence;
9. external version/regulatory claims carry dated verification;
10. generated indexes and routing files remain consistent with the tree.

## Automation

`scripts/ci/check-doc-references` is the mechanical gate. It inventories repository Markdown files and checks explicit links, path-qualified references, bare names, missing targets and ambiguous basename resolution.

Use:

```text
python3 scripts/ci/check-doc-references
python3 scripts/ci/check-doc-references --inventory
```

The checker is a reference-integrity gate, not an authority engine. CI must fail if the generated inventory file is empty or lacks the inventory summary line.

## Semantic audit rule

A resolved reference can still be wrong in meaning.

When a document and current implementation disagree:

- classify whether the document is historical, target-state or current;
- classify the source-of-truth level using `agent.md`'s hierarchy;
- do not silently weaken a governing contract to match an incomplete implementation;
- do not claim implementation completion merely because a contract exists;
- record implementation gaps in the current audit/remediation plan.

## Phase contract coverage authority

`docs/phase_contract_coverage_register.md` is the canonical reconciliation record for the implementation-phase contract corpus. It owns coverage classification, ICM routing status, implementation/evidence status and remediation tracking for Phases 0–20. It does not override higher-authority product, security, legal, regulatory, provider, domain, database, API or commercial sources.

## Canonical semantic remediation

The current semantic audit and repair sequence is documented in:

`docs/documentation_semantic_remediation_plan.md`

The current repository-wide state assessment is:

`docs/enterprise_audit_and_review.md`

Both are subordinate to the project's governing source hierarchy.

## Agent rule

When encountering a missing or ambiguous reference, stop the authority chain at that point. Record the defect, identify the nearest authoritative source, and repair the path or create an explicitly identified compatibility/contract document with evidence. Never fabricate historical recovery.