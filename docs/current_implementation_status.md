# Sitolo — Current Implementation Status

**Status:** Current implementation-state pointer
**Date:** 2026-10-25
**Main baseline:** `623f7aed6105664d10d1a2802480fde8316ed5f8`

## Active product implementation

The active Sitolo product-development stream is **Phase 4 Part 8 / PR-009: durable audit evidence + transactional outbox**.

PR #49 is the active implementation PR for that work. It remains open and is not the same work as PR #52.

### PR #49 latest implementation checkpoint

The latest implementation source is `4858341fcd16459aad10494d8b75ddb684859861` on `feat/phase4-pr009-audit-outbox`. Rust, Integration, Policy and Security workflows pass on exact documentation head `d53e05297963b5457d44006e236522c4a179d257`, which includes that source. The branch includes fail-closed disposable-database target validation, versioned secret-free configuration fingerprint encoding, strict lint/CI policy enforcement, shared testkit tenancy fixtures, and removal of the unavailable organization activation route.

PR #49 remains **not ready for merge or Phase 4 closure**. Production consumer/effect composition, migration/release authority, full mutation/Phase 3 identity evidence, administrator branch-protection evidence, independent Stage 06 review, and human review remain open. The organization activation route was removed from the active API surface because no persisted provisioning transition exists; shared testkit fixtures and automated lint/CI policy enforcement are implemented. See [`phase4_part8_pr009_current_status.md`](phase4_part8_pr009_current_status.md) and the [Phase 1–4 requirements matrix](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/phase1-4-requirements-matrix-2026-10-25.md).

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
