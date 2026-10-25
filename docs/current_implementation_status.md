# Sitolo — Current Implementation Status

**Status:** Current implementation-state pointer
**Date:** 2026-10-25
**Main baseline:** `623f7aed6105664d10d1a2802480fde8316ed5f8`

## Active product implementation

The active Sitolo product-development stream is **Phase 4 Part 8 / PR-009: durable audit evidence + transactional outbox**.

PR #49 is the active implementation PR for that work. It remains open and is not the same work as PR #52.

### PR #49 latest implementation checkpoint

The latest implementation candidate is `e48b12a5995b4d79aca86a7443b014144d893164` on `feat/phase4-pr009-audit-outbox`. Rust, Integration, Policy and Security workflows pass on that exact source SHA. The branch includes a fail-closed disposable-database target guard and complete secret-free optional-field fingerprint encoding (format version 2).

PR #49 remains **not ready for merge or Phase 4 closure**. Production consumer/effect composition, migration/release authority, full mutation/identity evidence, organization lifecycle disposition, testkit and CI policy reconciliation, independent Stage 06 review, and human review remain open. See [`phase4_part8_pr009_current_status.md`](phase4_part8_pr009_current_status.md) for current evidence and blockers.

## PR #52

PR #52 (`feat/agentic-workflow`) was the initial **engineering workflow/documentation architecture change**; the current mainline ICM conformance pass is tracked separately.

It adds and hardens the ICM filesystem workflow, context routing, System Map, documentation authority classification, reference integrity, and documentation remediation controls.

It does **not** move Sitolo's product implementation to Phase 8.

## Phase 8

`docs/phase8_product_catalogue_implementation.md` is a later-phase **target-state contract**.

Its presence does not mean Phase 8 is active, implemented, or verified.

The contract exists so future Phase 8 work has a stable requirement definition before implementation begins.

## Current Phase 4 sequence

```text
Phase 4 Part 1 → completed baseline
Phase 4 Part 2 → completed baseline
Phase 4 Part 3 → completed baseline
Phase 4 Part 4 → completed baseline
Phase 4 Part 5 → completed baseline
Phase 4 Part 6 → completed baseline
Phase 4 Part 7 → completed baseline
Phase 4 Part 8 → ACTIVE / PR-009
```

Current product work must use the active Part 8 binding contract and the current PR-009 status document rather than later Phase 8 contracts.

## Authority

For current implementation state:

1. current source tree;
2. active phase/PR contract;
3. current verification evidence;
4. historical audits only as historical evidence.

A contract for a future phase is not implementation evidence. A PR-specific review snapshot is not current state unless it matches the current PR head.

## ICM workflow position

The ICM pipeline itself is workflow infrastructure:

```text
01-select
02-research
03-investigate
04-plan
05-implement
06-audit
07-remediate
08-verify
09-deliver
```

That pipeline is independent of the Sitolo product phase sequence. It is the mechanism used to perform engineering work safely; it is not a replacement for product-phase planning.
