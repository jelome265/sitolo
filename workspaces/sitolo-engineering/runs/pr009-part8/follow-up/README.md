---
artifact_type: follow-up-index
status: current-snapshot-index
run_slug: "pr009-part8"
security_relevant: yes
candidate_head: "76154ffd639ff6467729f3e4ab508f554a72f919"
date: "2026-10-26"
---

# PR #49 / Phase 1–4 Audit Follow-up Index

This index retains the historical cross-phase audit artifacts and points to the current remediation checkpoint. The deep audit, original plan and copy-paste guide remain pinned to source snapshot `a378beec2bc5097bb642825d2e8fcf4c967c105d`; the current implementation and exact-head verification are recorded separately below.

## Current implementation checkpoint — `76154ff`

The Rust lint policy now inventories all 45 existing distinct reasoned `#[expect]` path/attribute pairs and rejects direct or conditional `#[allow]` and `#[warn]` downgrades. Six regression tests cover allowed, missing, stale, changed, and lint-lowering cases. Rust [40290717776](https://github.com/jelome265/sitolo/actions/runs/40290717776), Integration [40290717827](https://github.com/jelome265/sitolo/actions/runs/40290717827), Security [40290718200](https://github.com/jelome265/sitolo/actions/runs/40290718200), and Policy [40290717731](https://github.com/jelome265/sitolo/actions/runs/40290717731) passed on exact head `76154ffd639ff6467729f3e4ab508f554a72f919`. See the [lint downgrade extension](phase1-4-remediation-extension-76154ff.md). Other Phase 1–4 and review blockers remain.

## Current implementation checkpoint — `9d6ba1c`

Rust lint exceptions now require an exact path-and-attribute inventory record, including the lint and inline reason. The 45 existing distinct `#[expect]` records are inventoried; policy CI regression tests reject missing, changed and stale entries. Rust [40289598902](https://github.com/jelome265/sitolo/actions/runs/40289598902), Integration [40289598918](https://github.com/jelome265/sitolo/actions/runs/40289598918), Security [40289599101](https://github.com/jelome265/sitolo/actions/runs/40289599101), and Policy [40289598905](https://github.com/jelome265/sitolo/actions/runs/40289598905) passed on exact head `9d6ba1c58601691fce4195b76ec700ad207d2cdb`. See the [remediation extension](phase1-4-remediation-extension-9d6ba1c.md). Other Phase 1–4 and independent-review blockers remain.

## Current implementation checkpoint — `216eb36`

The worker now renews SQS message visibility during processing, receives one FIFO message at a time per worker, and rejects a message-group ID that does not match the event's organization/aggregate. The targeted worker tests (19), strict Clippy, workspace Clippy and workspace check pass locally. Exact-head Rust [40287439503](https://github.com/jelome265/sitolo/actions/runs/40287439503), Integration [40287439518](https://github.com/jelome265/sitolo/actions/runs/40287439518), Security [40287439599](https://github.com/jelome265/sitolo/actions/runs/40287439599), and Policy [40287439504](https://github.com/jelome265/sitolo/actions/runs/40287439504) passed on `216eb369cd01e107c14b8e654ce6f56ff305416b`. See the [consumer visibility remediation extension](phase1-4-remediation-extension-216eb36.md). These checks do not prove live AWS behavior or production consumer composition. PR #49 remains open, unmerged and review-required.

## Current implementation checkpoint — `15147ef`

Replaced the PostgreSQL integration suite's fake evidence-failure unit-of-work wrappers with the real `PgTenancyUnitOfWork` plus disposable-schema triggers that reject actual audit or outbox inserts. The existing rollback cases assert state and evidence rollback. Local formatting, strict focused Clippy and integration-target compilation pass. Exact-head Rust [40195430303](https://github.com/jelome265/sitolo/actions/runs/40195430303), Integration [40195430299](https://github.com/jelome265/sitolo/actions/runs/40195430299), Security [40195430302](https://github.com/jelome265/sitolo/actions/runs/40195430302), and Policy [40195430306](https://github.com/jelome265/sitolo/actions/runs/40195430306) all pass; CodeQL [121176342832](https://github.com/jelome265/sitolo/runs/121176342832) passes. This closes only the database-trigger failure-injection gap; the full mutation matrix remains partial, and PR #49 remains open and review-required.

## Current implementation checkpoint — `1743f3a`

The fingerprint test module now exhaustively destructures every `AppConfig` field, so adding a configuration field causes compilation to require review of the fingerprint inventory. Focused config tests (28), strict package Clippy and formatting pass locally. Exact-head Rust [40193380363](https://github.com/jelome265/sitolo/actions/runs/40193380363), Integration [40193380330](https://github.com/jelome265/sitolo/actions/runs/40193380330), Security [40193380381](https://github.com/jelome265/sitolo/actions/runs/40193380381), and Policy [40193380574](https://github.com/jelome265/sitolo/actions/runs/40193380574) all pass on `1743f3a198605e1e50a30776a3366d2e9e9a36f5`; CodeQL [121169950121](https://github.com/jelome265/sitolo/runs/121169950121) passes. This strengthens only the test inventory; outstanding Phase 1–4, independent audit and human-review blockers remain.

## Current implementation checkpoint — `23d8e14`

Rust [40192070689](https://github.com/jelome265/sitolo/actions/runs/40192070689), Integration [40192070759](https://github.com/jelome265/sitolo/actions/runs/40192070759), Security [40192070695](https://github.com/jelome265/sitolo/actions/runs/40192070695), and Policy [40192070679](https://github.com/jelome265/sitolo/actions/runs/40192070679) workflows all pass on exact source head `23d8e14aa357dc213fb4e42a80c94c0c6e010445`. PostgreSQL integration now explicitly exercises suspended-identity and security-version session invalidation. This does not resolve the production consumer, migration/release, full mutation, remaining Phase 3 lifecycle, independent Stage 06 or human-review blockers. PR #49 remains open and review-required.

## Latest implementation and verification checkpoint — `d8032ef`

The shared disposable PostgreSQL guard is consumed by every identified real PostgreSQL integration harness, including `apps/api/tests/tenancy_api.rs`; its debug output redacts DSN credentials. Testkit tests pass (8), the API integration target compiles locally, and all exact-head workflows pass on `d8032eff0422edb9447239ccc13f403f285c00db`: Rust [40190324443](https://github.com/jelome265/sitolo/actions/runs/40190324443), Integration [40190324475](https://github.com/jelome265/sitolo/actions/runs/40190324475), Security [40190324472](https://github.com/jelome265/sitolo/actions/runs/40190324472), and Policy [40190324477](https://github.com/jelome265/sitolo/actions/runs/40190324477). PR #49 remains open and review-required; production and independent-review blockers remain as recorded below.

## Latest implementation and verification checkpoint — `18dce7e`

The latest source commit centralizes disposable PostgreSQL target and generated-schema validation in `sitolo-testkit::postgres`, and wires the application, Part 8 persistence and RLS integration harnesses through the shared guard. Testkit unit tests (6), affected integration-target compilation, strict focused Clippy, architecture, crate boundary, Rust policy, doc reference and Phase 2 policy checks pass. All required exact-head GitHub workflows pass on `18dce7e59c945cedcccf793b2e69b354b9b86e4d`: Rust [40182743696](https://github.com/jelome265/sitolo/actions/runs/40182743696), Integration [40182743665](https://github.com/jelome265/sitolo/actions/runs/40182743665), Security [40182743667](https://github.com/jelome265/sitolo/actions/runs/40182743667), Policy [40182743657](https://github.com/jelome265/sitolo/actions/runs/40182743657). PR #49 remains open and review-required; this test-harness fix does not resolve the owner/release/review blockers below.

## Latest implementation and verification checkpoint — `db4fcba`

The latest focused implementation validates generated Part 8 schema identifiers before any network connection or DDL. Its parser-only regression, strict focused Clippy and formatting checks pass. All four required workflows passed on exact head `db4fcbaa69f12b96971d95d8bc6c2786c978858f`: Rust [40180985925](https://github.com/jelome265/sitolo/actions/runs/40180985925), Integration [40180985800](https://github.com/jelome265/sitolo/actions/runs/40180985800), Security [40180985807](https://github.com/jelome265/sitolo/actions/runs/40180985807), and Policy [40180985960](https://github.com/jelome265/sitolo/actions/runs/40180985960). See the [verification follow-up](pr49-phase4-part8-verification-follow-up-2026-10-25.md). This does not close the consumer, migration/release, complete mutation evidence, Phase 3, independent Stage 06 or human-review gates.

## Current implementation checkpoint — source `4858341`, verified head `d53e052`

The [remediation execution report](phase1-4-remediation-execution-2026-10-25.md) records the database target guards, fingerprint fix, lint/CI policy, shared testkit fixtures and removal of the unavailable organization activation endpoint. The [Phase 1–4 requirements matrix](phase1-4-requirements-matrix-2026-10-25.md) distinguishes implemented evidence from remaining owner and review blockers. Rust [40179488202](https://github.com/jelome265/sitolo/actions/runs/40179488202), Integration [40179488191](https://github.com/jelome265/sitolo/actions/runs/40179488191), Security [40179488204](https://github.com/jelome265/sitolo/actions/runs/40179488204) and Policy [40179488198](https://github.com/jelome265/sitolo/actions/runs/40179488198) passed on exact head `d53e052`.

## Later implementation checkpoint — `e48b12a`

The [remediation execution report](phase1-4-remediation-execution-2026-10-25.md) records two subsequent fixes: fail-closed PostgreSQL test target validation and collision-safe optional-field configuration fingerprints (format version 2). Rust, Integration, Policy and Security passed on exact candidate `e48b12a5995b4d79aca86a7443b014144d893164`. The original deep audit, plan and copy-paste guide remain pinned to `a378beec` as historical audit artifacts.

The two fixes do not close the remaining production consumer, migration/release, complete mutation-evidence, Phase 3, organization lifecycle, testkit, CI governance, independent-review or human-review gates.

## New report-only artifacts

1. [Phase 1–4 deep audit](phase1-4-deep-audit-2026-10-25.md) — document inventory, exact-head evidence, findings, real-vs-test-only distinctions, Clippy/unsafe and command-safety review, overall disposition.
2. [Phase 1–4 remediation plan](phase1-4-remediation-plan-2026-10-25.md) — prioritized workstreams, acceptance evidence and Phase 4 exit gate.
3. [Copy/paste agent execution guide](phase1-4-copy-paste-guide-2026-10-25.md) — constrained execution prompt, safety rules, remediation order and verification gates.

## Source snapshot evidence observed

- PR: [#49](https://github.com/jelome265/sitolo/pull/49), observed open/unmerged.
- Source candidate: [a378beec2bc5097bb642825d2e8fcf4c967c105d](https://github.com/jelome265/sitolo/commit/a378beec2bc5097bb642825d2e8fcf4c967c105d)
- [Rust run 40163505238](https://github.com/jelome265/sitolo/actions/runs/40163505238) — passed.
- [Integration run 40163505245](https://github.com/jelome265/sitolo/actions/runs/40163505245) — passed.
- [Security run 40163505240](https://github.com/jelome265/sitolo/actions/runs/40163505240) — passed.
- [Policy run 40163505254](https://github.com/jelome265/sitolo/actions/runs/40163505254) — passed.

These results belong only to the listed candidate SHA. The report-only documentation commit creates a newer HEAD and requires its own run-state check. Do not copy old statuses as proof for that newer HEAD.

## Existing follow-up records: retain provenance

Older SHA-pinned reports must not be rewritten to look as if they reviewed newer source:
- pr49-phase4-part8-read-only-re-audit-2026-10-25.md
- pr49-phase4-part8-remediation-2026-10-23-plan.md
- pr49-phase4-part8-remediation-2026-10-24-report.md
- pr49-phase4-part8-verification-2026-10-24-report.md
- pr49-phase4-part8-verification-2026-10-25-report.md
- pr49-mandatory-mutation-transaction-matrix-2026-10-23.md
- decisions/iam-lifecycle-event-mapping-proposal.md
- decisions/identity-production-runtime-proposal.md
- decisions/relay-delivery-contract-proposal.md

Historical Part 6/7/PR-008 records also remain historical evidence. Use binding phase contracts and current source for requirements and current truth.

## Review boundary

These artifacts are supplemental, not an independent Stage 06 audit, approval or human review. Do not copy them into the independent output shelf or treat them as closure. Phase 4 Part 8 remains not ready until blockers have implementation evidence, current-head workflow results, independent Stage 06 and human review.
