---
artifact_type: verification-report
status: complete
run_slug: "pr009-part8"
security_relevant: yes
---

# Verification Report — PR #49 Phase 4 Part 8 remediation (exact-head follow-up)

**Branch:** `feat/phase4-pr009-audit-outbox`
**Current source basis:** `b095768f9903c86b399c13d788714ebd0294c2a2` (2026-10-25)
**Previous source basis:** `3ee27a3a660557975e602f48535916bd44e82b90` (2026-10-25)
**Remediation input:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-remediation-2026-10-24-report.md`
**Follow-up findings input:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-read-only-re-audit-2026-10-25.md`
**PR:** [#49](https://github.com/jelome265/sitolo/pull/49) — Open, unmerged, base `main`, head `b095768`.
**Verification window:** 2026-10-25
**PR #49 merge state:** Left unmerged. This report records evidence only; it is not a merge approval and does not satisfy the independent Stage 06 re-audit.

## Latest PR-head CI rerun — documentation head `a5dffa4`

The following workflows completed against PR head `a5dffa4fe734cb82717a783f552d4f7489bfd4ff`. The implementation under test includes the payload fix `b095768`; the `a5dffa4` changes in this checkpoint are documentation-only.

| Workflow | Result | Run | Finding |
|---|---|---|---|
| Policy | **PASS** | [40161977349](https://github.com/jelome265/sitolo/actions/runs/40161977349) | ICM and repository policy checks pass |
| Security | **PASS** | [40161977235](https://github.com/jelome265/sitolo/actions/runs/40161977235) | Security workflow passes |
| Integration | **PASS** | [40161977330](https://github.com/jelome265/sitolo/actions/runs/40161977330) | PostgreSQL integration is green; membership transaction tests 44/44 and provision/invitation evidence tests 18/18 |
| Rust | **FAIL** | [40161977327](https://github.com/jelome265/sitolo/actions/runs/40161977327) | The only reported Rust failure is `test_part8_relation_ownership_and_rls_metadata`: four observed policies vs three expected |

The RLS metadata failure is associated with `consumer_inbox_tenant_policy` on `consumer_inbox`, targeting `app_worker`. The policy was introduced earlier in `0cdd771`; neither the one-key lifecycle payload correction nor this documentation-only checkpoint changes its SQL or the assertion. This provenance does not itself settle the contract question: before merge, determine whether the test should scope its policy inventory to Part 8 evidence tables or the consumer-inbox policy violates the governing table/role contract. Do not relax the test or alter the policy without that contract-backed decision.

These are the CI results for `a5dffa4`, not a claim that every workflow passed. The Integration gate is green; Rust remains red. The prior implementation-head evidence below is retained to preserve test-level provenance for fixes `3ee27a3` and `b095768`.

## 1. Exact-head GitHub Actions evidence (head `b095768`)

| Workflow | Result | Run | Evidence and scope |
|---|---|---|---|
| Policy | **PASS** | [40160964864](https://github.com/jelome265/sitolo/actions/runs/40160964864) | ICM workspace, workflow policy, system map, doc references, release workflow, policy files present |
| Security | **PASS** | [40160964726](https://github.com/jelome265/sitolo/actions/runs/40160964726) | cargo-deny dependency/license policy, cargo-audit advisories, CodeQL, secrets, offensive-regression |
| Integration | **PASS** | [40160964822](https://github.com/jelome265/sitolo/actions/runs/40160964822) | application and contract tests with PostgreSQL |
| Rust | **FAIL** | [40160964745](https://github.com/jelome265/sitolo/actions/runs/40160964745) | architecture, crate-boundary, format, strict Clippy, compile, lib unit tests pass; one PostgreSQL-backed persistence test fails (see §1.3) |

## 1.1 Fix: `organization_id` removed from membership lifecycle event payload (commit `b095768`)

**Root cause:** `transition_membership_with_evidence` included `organization_id` in the outbox/audit payload. The strict `iam.membership.pending_acceptance` schema enforced by `validate_iam_payload` (crates/sitolo-events/src/payload.rs:143-156) allows only `membership_id`, `from_state`, `to_state`, `state_version`. The extra `organization_id` field failed `validate_keys` → `OutboxError::InvalidSchema` → `EvidencePersistenceFailed`, so `mark_member_pending` evidence writes failed.

This single defect cascaded into all three remaining PostgreSQL failures:
- `mark_member_pending_commits_state_audit_and_outbox_together` — evidence write failed, so no outbox row → test's `fetch_one` returned `RowNotFound`.
- `expire_invitation_commits_membership_audit_and_outbox_together` — it calls `mark_member_pending(...).await?` first, which failed with `EvidencePersistenceFailed`.
- `concurrent_mark_member_pending_emits_one_semantic_event` — both concurrent calls failed evidence persistence → 0 successes instead of 1.

**Fix:** Removed `"organization_id": organization_id.as_str()` from the payload built in `transition_membership_with_evidence` (crates/sitolo-application/src/tenancy.rs). `organization_id` is already a dedicated `audit_events` column; the contract (docs §511) scopes membership lifecycle payloads to `membership_id`, `from_state`, `to_state`, `state_version`.

## 1.2 The three provision failures — FIXED and proven green

The `tests/part8_provision_organization_evidence_tests.rs` target now reports **18 passed, 0 failed** on exact head `b095768` (Integration run 40160964822). Previously it reported 15 passed, 3 failed. All three failures are resolved at the semantic boundary with no weakened expected event counts:

| Test | Prior failure | Fix | CI result (`b095768`) |
|---|---|---|---|
| `mark_member_pending_commits_state_audit_and_outbox_together` | `RowNotFound` (no outbox row) | Payload no longer carries the schema-rejected `organization_id`; evidence written atomically | **PASS** |
| `expire_invitation_commits_membership_audit_and_outbox_together` | `EvidencePersistenceFailed` | `mark_member_pending` precondition now succeeds; `expire_invitation` payload was already schema-compliant | **PASS** |
| `concurrent_mark_member_pending_emits_one_semantic_event` | 0 successes instead of 1 | One concurrent call transitions (writes evidence), the other returns `InvalidTransition` | **PASS** |

## 1.3 Remaining Rust failure — pre-existing, out of scope for this slice

The `tests/part8_postgres_integration_tests.rs` target reports a failure in
`test_part8_relation_ownership_and_rls_metadata`: it expects exactly 3 Part 8
RLS policies but finds 4.

**Root cause (not caused by this remediation):** `crates/sitolo-persistence/schema/consumer_inbox.sql` (added in `0cdd771`, applied in `setup_test_context`) creates `consumer_inbox_tenant_policy` on the `consumer_inbox` table, targeting `app_worker` (consumer_inbox.sql:32). The Part 8 test asserts exactly 3 policies on the Part 8 evidence tables and that no table policy targets the worker role. `consumer_inbox` is not a Part 8 evidence table, so its policy is out of scope for this assertion, but the test's `pg_policies` query is unscoped and counts it.

This is a **pre-existing** finding unrelated to the role-assignment / provision remediation (the commit `b095768` touches only a JSON payload string in `tenancy.rs`, no SQL). It sits in the SQS consumer / transactional-inbox area (AUD-01), which requires an owner-approved consumer contract and must not be altered or invented here. It is recorded as a remaining blocker, not waived and not treated as passed.

## 1.4 The six assigned membership failures — FIXED and proven green

The `tests/membership_transaction_integration.rs` target reports **44 passed, 0 failed** on exact head `3ee27a3` (and re-confirmed at `b095768`). All six failures identified by the re-audit are resolved at the semantic boundary, with no weakened expected event counts:

| Test | Prior failure | Fix | CI result |
|---|---|---|---|
| `authenticated_role_assignment_authorizes_and_records_server_actor` | `RowNotFound` | Reconciled audit-target contract: `target_ref` is the affected membership; assignment identity proven through outbox `aggregate_id` | **PASS** |
| `authenticated_membership_suspend_authorizes_and_records_server_actor` | denial ≠ `ScopeError::PermissionDenied` | Denied suspend/revoke authenticate as the suspended/revoked membership; authority lookup resolves to `PermissionDenied` | **PASS** |
| `authenticated_membership_revoke_authorizes_and_records_server_actor` | denial ≠ `ScopeError::PermissionDenied` | Same as above | **PASS** |
| `concurrent_role_assignment_retries_emit_one_semantic_event` | counts `(1, 2, 2)` vs `(1, 1, 1)` | `RoleAssignmentOutcome` (`Created`/`Replayed`); evidence only on real transition; `lock_organization` serializes retries | **PASS** |
| `role_assignment_commits_with_evidence_and_is_semantically_idempotent` | counts `(2, 2)` vs `(1, 1)` | Same created/replayed outcome | **PASS** |
| `role_revocation_preserves_history_and_regrant_is_duplicate_safe` | counts `(2, 1, 3, 3, 1)` vs `(2, 1, 2, 2, 1)` | Revoked-record replay returns `Replayed`, no new evidence; new-id regrant creates one assignment + one evidence pair | **PASS** |

## 2. Local non-database gates (all green)

Executed against `b095768` with a clean working tree, Rust 1.98.1, no local PostgreSQL:

| Command | Result | Scope |
|---|---|---|
| `cargo fmt --all -- --check` | PASSED | Whole workspace |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASSED | Strict policy unchanged; no suppression |
| `cargo check --workspace --all-targets --all-features --locked` | PASSED | Whole workspace, all targets |
| `cargo test --workspace --lib --all-features --locked` | PASSED — 259 tests, 0 failed | Library unit tests across 17 packages |
| `cargo build --workspace --release --locked` | PASSED | Whole workspace, release profile |
| `cargo test -p sitolo-api-bin --test axum_boundary` | PASSED — 3 tests | R2 deny-by-default over every declared protected route |
| `cargo test -p sitolo-application --test lifecycle_event_contract` | PASSED — 2 tests | R4 five-event catalogue/validator consistency |
| `./scripts/ci/check-architecture` | PASSED | Exhaustive crate-boundary contract, 17 packages classified |
| `./scripts/ci/check-phase2-policy` | PASSED | Configuration/secret/logging policy |
| `./scripts/ci/check-workflow-policy` | PASSED | GitHub Actions workflow policy |
| `./scripts/ci/check-release-workflow` | PASSED | Release workflow provenance and promotion policy |
| `./scripts/ci/check-security-icm-policy` | PASSED | Security ICM policy |
| `python scripts/ci/check-doc-references` | PASSED — 0 errors | Documentation link integrity |
| `./scripts/ci/check-system-map-processes` | PASSED — 2 source-grounded cards | System Map process/source hop |
| `cargo deny check` | PASSED | Advisories, bans, licenses, sources |
| `./scripts/ci/check-icm-workspace` | PASSED — 9 stages, exact handoffs, no synthetic approval | ICM routing, stage contracts, mechanical cold walk |
| `cargo metadata --format-version 1 --locked` | PASSED | Lockfile is current |

Per-crate library results: `sitolo-api` 20, `sitolo-api-bin` 7, `sitolo-audit` 4, `sitolo-auth` 53, `sitolo-authz` 22, `sitolo-config` 24, `sitolo-domain` 11, `sitolo-events` 12, `sitolo-integrations` 12, `sitolo-observability` 10, `sitolo-persistence` 41, `sitolo-security` 18, `sitolo-tenancy` 8, `sitolo-worker-bin` 17.

## 3. Change summary

Commit `b095768`: `fix(phase4-part8): remove organization_id from membership lifecycle event payload` — 1 file, 1 deletion.

Combined with commit `3ee27a3` (`fix(phase4-part8): role-assignment evidence idempotency and denial variant`, 5 files, +84/−42), all remediation is implemented. No expected event counts were relaxed, no grants broadened, no denial accepted as arbitrary, no lint suppressed, `#[allow]` remains banned.

## 4. Limitations

- Database-backed suites require the CI PostgreSQL service and three DSNs; not run locally. Exact-head GitHub Actions is the authority.
- `cargo-audit` is not installed locally; the CI `advisories` job (Security workflow, run 40160964726) is authoritative and passed.
- The `test_part8_relation_ownership_and_rls_metadata` failure (§1.3) is pre-existing and out of scope for this slice (consumer/AUD-01 area).
- The independent Stage 06 re-audit required before `08-verify` has **not** been performed. This report claims no audit approval, no security sign-off, and no release readiness.

## 5. Completion condition

Local non-database gates are green and the crate-boundary, lint, route-inventory, and ICM contracts are enforced. The three provision failures and all six membership failures are fixed and proven green on exact head `b095768` (Integration run 40160964822 PASS; membership tests 44/44). Rust CI remains red solely due to the pre-existing `consumer_inbox_tenant_policy`/`app_worker` RLS policy-count finding (§1.3), which is unrelated to this remediation and sits in the AUD-01 consumer area.

Verification is **not** complete for release: the pre-existing persistence test failure and the independent Stage 06 re-audit plus human review are still required. Control returns to `06-audit` for the re-audit; `09-deliver` is not entered. PR #49 remains open and unmerged.
