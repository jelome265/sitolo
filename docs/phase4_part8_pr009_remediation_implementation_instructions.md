# SITOLO — PR-009 / Phase 4 Part 8 Remediation Implementation Instructions

**Status:** Superseded review artifact  
**Reviewed PR:** #49 — Phase 4 Part 8 audit/outbox implementation  
**Original review scope:** remediation of the first PR-009 implementation pass  
**Superseded by:** `docs/phase4_part8_audit_outbox_implementation_contract.md`  
**Disposition:** Historical review evidence; do not use as the active PR-009 implementation plan.

---

## 1. Why this file is superseded

This document reviewed an earlier PR-009 head and identified missing durable audit/outbox implementation, transaction coupling, worker runtime, privilege hardening, claim ownership, aggregate ordering, bounded worker inputs, test isolation, duplicate-delivery proof and observability gaps.

PR #49 subsequently received a later implementation pass and a dedicated re-audit execution contract.

The old reviewed commit/base values remain useful for tracing the evolution of the PR but are not the current PR state.

## 2. Current PR-009 authority

The active remediation contract is:

`docs/phase4_part8_audit_outbox_implementation_contract.md`

That document records the later PR head, the remaining P0/P1 defects, implementation order and definition of done.

The binding Phase 4 Part 8 contract remains:

`docs/phase4_part8_audit_outbox_implementation_contract.md`

## 3. Current-state rule

Agents must not combine this superseded document with the newer execution contract and infer a larger or different task.

Use this file only to understand historical review findings and how the remediation evolved.

## 4. Preservation

This artifact is retained because PR reviews are evidence. When the PR closes, the final state must still be evaluated from the merged source, current tests and governing contract rather than from either historical review document.