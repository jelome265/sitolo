# Sitolo — Phase 4 Part 8 PR-009 Current Status

**Status:** NOT READY — exact-head PostgreSQL integration failures; do not merge  
**PR:** [#49 — Phase 4 Part 8 audit and outbox](https://github.com/jelome265/sitolo/pull/49)  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Implementation code commit:** `5a128c4f95db61f7f6cf9fd885d22911b78553c1`  
**Candidate head with CI evidence:** `68f5ae5be7cdb8659e0cbd4f90fae16a7ee24a85`  
**Date:** 2026-10-25  
**Binding contract:** `docs/phase4_part8_audit_outbox_implementation_contract.md`  
**Continuation guide:** `docs/phase4_part8_pr009_remediation_execution_contract.md`  
**Exact-head follow-up findings:** [ICM follow-up record](../workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-read-only-re-audit-2026-10-25.md)

## Exact-head CI result

The runs below evaluated candidate head `68f5ae5be7cdb8659e0cbd4f90fae16a7ee24a85`. They supersede earlier “CI pending” statements for that candidate.

| Workflow | Result | Run |
|---|---|---|
| Policy | **PASS** | [40156547558](https://github.com/jelome265/sitolo/actions/runs/40156547558) |
| Rust | **FAIL** | [40156547515](https://github.com/jelome265/sitolo/actions/runs/40156547515) |
| Integration | **FAIL** | [40156547478](https://github.com/jelome265/sitolo/actions/runs/40156547478) |
| Security | **PASS** | [40156547500](https://github.com/jelome265/sitolo/actions/runs/40156547500) |

The PostgreSQL-backed application transaction suite reports **38 passed, 6 failed**. Failures cover role-assignment audit lookup, membership suspend/revoke authorization error mapping, duplicate audit/outbox evidence on assignment retry/idempotent replay, and duplicate evidence during revoke/regrant. See the linked follow-up record for each test name, expected/actual values, source trace, and ordered correction plan.

## What is materially improved

- Application-owned ports live in `crates/sitolo-application/src/identity_ports.rs` and `tenancy_ports.rs`; PostgreSQL implementation is behind `sitolo-persistence`.
- The metadata-driven crate-boundary contract covers all 17 workspace packages. The independent architecture/boundary job passes on the evaluated candidate.
- Strict workspace Clippy passes on the evaluated candidate with `--all-targets --all-features --locked -- -D warnings`.
- The R2 route-boundary tests pass: all six declared protected routes reject missing credentials.
- The R4 schema/catalogue tests pass for five lifecycle event names and version-1 shape validation. Full transition-to-persisted-event/relay proof remains open.
- The Security workflow passes, including its advisories/cargo-audit job.

These passes do not override the failed database-backed tests or close the remaining semantic audit gates.

## Open exit criteria

1. Fix role assignment/revocation outcome semantics so identical retries and no-op transitions cannot emit duplicate audit/outbox events.
2. Align role-assignment audit target identity with the approved contract; correct suspend/revoke permission-denial mapping; prove denied requests leave state and evidence unchanged.
3. Make the full PostgreSQL-backed Rust and Integration workflows pass on the same final source head.
4. Finish the owner-approved SQS FIFO queue/consumer contract and compose the existing consumer/inbox infrastructure with an approved concrete business effect in the worker runtime. AUD-01 remains open.
5. Complete the restricted worker role and documented migration/release path across identity, tenancy, audit, outbox and inbox schemas. AUD-05 remains open/partial.
6. Resolve the organization-activation route: implement a real persisted `PROVISIONING` transition or remove the route from the declared inventory/contract.
7. Complete the source-grounded R3 mutation matrix and R4 transition/evidence trace; AUD-04 remains open.
8. Obtain the independent Stage 06 re-audit and required human review. Verify the architecture/boundary job is required by repository branch protection.
9. Refresh the PR description after the source fixes and final exact-head evidence are available; its existing head/CI statements are stale.

## Evidence precedence

The 2026-10-24 remediation and verification reports document earlier source/local checks and remain historical evidence. They are not proof that the PostgreSQL suite passed on candidate head `68f5ae5`. The new ICM follow-up records the current failed exact-head tests; it does not claim to be the independent Stage 06 audit. No merge or Phase 4 Part 8 closure is approved.
