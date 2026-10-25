---
artifact_type: follow-up-index
status: current-snapshot-index
run_slug: "pr009-part8"
security_relevant: yes
candidate_head: "db4fcbaa69f12b96971d95d8bc6c2786c978858f"
date: "2026-10-25"
---

# PR #49 / Phase 1–4 Audit Follow-up Index

This index retains the historical cross-phase audit artifacts and points to the current remediation checkpoint. The deep audit, original plan and copy-paste guide remain pinned to source snapshot `a378beec2bc5097bb642825d2e8fcf4c967c105d`; the current implementation and exact-head verification are recorded separately below.

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
