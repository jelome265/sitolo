---
artifact_type: verification-report
status: complete
run_slug: "pr009-part8"
security_relevant: yes
---

# Verification Report — PR #49 Phase 4 Part 8 remediation (exact-head follow-up)

**Branch:** `feat/phase4-pr009-audit-outbox`
**Current source basis:** `3ee27a3a660557975e602f48535916bd44e82b90` (2026-10-25)
**Previous source basis:** `5a128c4f95db61f7f6cf9fd885d22911b78553c1` (2026-10-24)
**Remediation input:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-remediation-2026-10-24-report.md`
**Follow-up findings input:** `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/pr49-phase4-part8-read-only-re-audit-2026-10-25.md`
**PR:** [#49](https://github.com/jelome265/sitolo/pull/49) — Open, unmerged, base `main`, head `3ee27a3`.
**Verification window:** 2026-10-25
**PR #49 merge state:** Left unmerged. This report records evidence only; it is not a merge approval and does not satisfy the independent Stage 06 re-audit.

## 1. Exact-head GitHub Actions evidence (head `3ee27a3`)

| Workflow | Result | Run | Evidence and scope |
|---|---|---|---|
| Policy | **PASS** | [40158776546](https://github.com/jelome265/sitolo/actions/runs/40158776546) | ICM workspace, workflow policy, system map, doc references, release workflow, policy files present |
| Security | **PASS** | [40158776577](https://github.com/jelome265/sitolo/actions/runs/40158776577) | cargo-deny dependency/license policy, cargo-audit advisories, CodeQL, secrets, offensive-regression |
| Rust | **FAIL** | [40158776568](https://github.com/jelome265/sitolo/actions/runs/40158776568) | architecture, crate-boundary, format, strict Clippy, compile and lib unit tests pass; workspace PostgreSQL-backed tests fail |
| Integration | **FAIL** | [40158776564](https://github.com/jelome265/sitolo/actions/runs/40158776564) | application and contract tests with PostgreSQL |

### 1.1 The six assigned membership failures — FIXED and proven green

The `tests/membership_transaction_integration.rs` target reports **44 passed, 0 failed** on exact head `3ee27a3`. All six failures identified by the re-audit are resolved at the semantic boundary, with no weakened expected event counts:

| Test | Prior failure | Fix | CI result (`3ee27a3`) |
|---|---|---|---|
| `authenticated_role_assignment_authorizes_and_records_server_actor` | `RowNotFound` looking up `target_ref = 'assignment-target'` | Reconciled audit-target semantics with the binding contract: `target_ref` is the affected membership; the assignment identity is proven through the outbox `aggregate_id`. Denied-request evidence queries now use the membership `target_ref`. | **PASS** |
| `authenticated_membership_suspend_authorizes_and_records_server_actor` | denial did not match `ScopeError::PermissionDenied` | Denied suspend/revoke now authenticate as the suspended/revoked membership itself, so the persisted authority lookup resolves to `ScopeError::PermissionDenied` for a member lacking the required permission; denied requests leave state, audit rows and outbox rows unchanged. | **PASS** |
| `authenticated_membership_revoke_authorizes_and_records_server_actor` | denial did not match `ScopeError::PermissionDenied` | Same as above. | **PASS** |
| `concurrent_role_assignment_retries_emit_one_semantic_event` | counts `(1, 2, 2)` vs `(1, 1, 1)` | `insert_role_assignment` now reports `RoleAssignmentOutcome::Created` vs `RoleAssignmentOutcome::Replayed`; mandatory `iam.role.assigned` audit/outbox evidence is written only on a real transition. `lock_organization` serializes concurrent identical requests to one transition and one evidence pair. | **PASS** |
| `role_assignment_commits_with_evidence_and_is_semantically_idempotent` | counts `(2, 2)` vs `(1, 1)` | Same created/replayed outcome; repeated identical request replays without recording evidence. | **PASS** |
| `role_revocation_preserves_history_and_regrant_is_duplicate_safe` | counts `(2, 1, 3, 3, 1)` vs `(2, 1, 2, 2, 1)` | Replaying an already-revoked assignment returns `Replayed` and records no new `iam.role.assigned` evidence; regrant with a new assignment id creates one effective assignment and one evidence pair. | **PASS** |

### 1.2 Remaining CI failures — pre-existing, out of scope for this slice

The `tests/part8_provision_organization_evidence_tests.rs` target reports **15 passed, 3 failed** on exact head `3ee27a3`:

- `expire_invitation_commits_membership_audit_and_outbox_together` — `EvidencePersistenceFailed`
- `mark_member_pending_commits_state_audit_and_outbox_together` — `RowNotFound`
- `concurrent_mark_member_pending_emits_one_semantic_event` — evidence-count assertion (`left: 0`, `right: 1`)

These three failures are **pre-existing and not caused by this remediation commit**. The commit `3ee27a3` touches only the role-assignment path (`insert_role_assignment`, `assign_role_with_evidence`, the `RoleAssignmentOutcome` port) and its tests. It does not touch `expire_invitation`, `mark_member_pending`, `transition_membership_with_evidence`, or `transition_membership`. The three failing tests exercise the invitation/provision membership-lifecycle evidence path, which is part of the still-open AUD-01 (consumer/invitation evidence) and R4 (lifecycle-event emission) gates. They are therefore recorded here as remaining open work, correctly attributed, and not relaxed or suppressed.

## 2. Local non-database gates (all green)

Executed against `3ee27a3` with a clean working tree, Rust 1.98.1, no local PostgreSQL:

| Command | Result | Scope |
|---|---|---|
| `cargo fmt --all -- --check` | PASSED | Whole workspace |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASSED | Strict policy unchanged; no suppression |
| `cargo check --workspace --all-targets --all-features --locked` | PASSED | Whole workspace, all targets |
| `cargo test --workspace --lib --all-features --locked` | PASSED — 259 tests, 0 failed | Library unit tests across 17 packages |
| `cargo build --workspace --release --locked` | PASSED — finished in ~4m 25s | Whole workspace, release profile |
| `cargo test -p sitolo-api-bin --test axum_boundary` | PASSED — 3 tests | R2 deny-by-default over every declared protected route; exact closed-route partition |
| `cargo test -p sitolo-application --test lifecycle_event_contract` | PASSED — 2 tests | R4 five-event catalogue/validator consistency |
| `./scripts/ci/check-architecture` | PASSED | Exhaustive crate-boundary contract, 17 packages classified |
| `./scripts/ci/check-phase2-policy` | PASSED | Configuration/secret/logging policy |
| `./scripts/ci/check-workflow-policy` | PASSED | GitHub Actions workflow policy |
| `./scripts/ci/check-release-workflow` | PASSED | Release workflow provenance and promotion policy |
| `./scripts/ci/check-security-icm-policy` | PASSED | Security ICM policy |
| `python scripts/ci/check-doc-references` | PASSED — 238 files, 1054 references, 0 errors | Documentation link integrity |
| `./scripts/ci/check-system-map-processes` | PASSED — 2 source-grounded cards | System Map process/source hop |
| `cargo deny check` | PASSED | Advisories, bans (incl. `serde_json/unbounded_depth`), licenses, sources |
| `./scripts/ci/check-icm-workspace` | PASSED — 9 stages, exact handoffs, no synthetic approval | ICM routing, stage contracts, mechanical cold walk |
| `cargo metadata --format-version 1 --locked` | PASSED | Lockfile is current |

Per-crate library results: `sitolo-api` 20, `sitolo-api-bin` 7, `sitolo-audit` 4, `sitolo-auth` 53, `sitolo-authz` 22, `sitolo-config` 24, `sitolo-domain` 11, `sitolo-events` 12, `sitolo-integrations` 12, `sitolo-observability` 10, `sitolo-persistence` 41, `sitolo-security` 18, `sitolo-tenancy` 8, `sitolo-worker-bin` 17. `sitolo-application`, `sitolo-sync` and `sitolo-testkit` have no library tests; their proof is the integration targets exercised in CI.

## 3. Change summary (commit `3ee27a3`)

`fix(phase4-part8): role-assignment evidence idempotency and denial variant` — 5 files, +84/−42:

- `crates/sitolo-application/src/tenancy_ports.rs`: added the typed `RoleAssignmentOutcome` (`Created` / `Replayed`) and changed `TenancyTransaction::insert_role_assignment` to return it, so the application port explicitly distinguishes a new state transition from an identical replay. Conflict on divergent semantics is unchanged (`TenancyError::Conflict`).
- `crates/sitolo-persistence/src/tenancy_unit_of_work.rs`: `insert_role_assignment` returns `RoleAssignmentOutcome::Created` only when it inserts a new effective row; an identical existing record yields `RoleAssignmentOutcome::Replayed` with no new mutation and no evidence.
- `crates/sitolo-application/src/tenancy.rs`: `assign_role_with_evidence` records `iam.role.assigned` audit/outbox evidence only for `RoleAssignmentOutcome::Created`; replays commit no evidence. Revoked-record replays also produce no new transition evidence.
- `crates/sitolo-application/src/lib.rs`: re-exports `RoleAssignmentOutcome`.
- `crates/sitolo-application/tests/membership_transaction_integration.rs`: reconciled the role-assignment audit-target contract (membership `target_ref` + outbox `aggregate_id`), corrected the denied suspend/revoke callers to authenticate as the suspended/revoked membership so `ScopeError::PermissionDenied` is the contract-defined denial, and aligned denied-request evidence queries to the membership `target_ref`.

No expected event counts were relaxed, no grants were broadened, no denial was accepted as arbitrary, no lint was suppressed, and `#[allow]` remains banned.

## 4. Security control evidence

| Control ID | Gate/test | Result | Limitation |
|---|---|---|---|
| SC-002 Tenant scope is server-derived | Port inversion; `check-crate-boundaries`; application tenancy integration targets | PASSED locally for compilation and unit scope | Real PostgreSQL RLS suite and application transaction tests require CI PostgreSQL service |
| SC-003 Authorization fails closed | `sitolo-authz` 22 tests; strict invitation-policy matcher | PASSED | Negative matrix against a live database remains CI-only |
| SC-005 Secrets stay narrow | `sitolo-security` 18 tests; presence-flag-only `Debug` impls | PASSED | Production secret resolution is deployment-specific |
| SC-009 Durable audit evidence | `FailingUnitOfWork`/`FailingTransaction` rollback targets; membership assignment evidence now transition-scoped | PASSED for compiled and unit scope | Evidence-failure rollback against PostgreSQL runs only in CI |
| SC-010 Resource limits and fail-closed uncertainty | Exhaustive protected-route inventory; R2 missing-credentials denial | PASSED | End-to-end load shedding is CI/runtime evidence |
| SC-011 Supply chain attributable and policy-checked | `cargo deny check`; `cargo metadata --locked` | PASSED | `cargo-audit` not installed locally; CI `advisories` job is authoritative |

## 5. Limitations

- Database-backed suites (`rls_security_tests`, `part8_postgres_integration_tests`, and the application PostgreSQL integration targets) require the CI PostgreSQL service and three DSNs; they were **not** run locally. Exact-head GitHub Actions remains the authority.
- `cargo-audit` is not installed locally; the CI `advisories` job (Security workflow, run 40158776577) is authoritative and passed.
- `scripts/ci/verify` was not executed end-to-end locally because it includes the PostgreSQL suite and `cargo audit`; its non-database steps were executed individually and are recorded in section 2.
- The three `part8_provision_organization_evidence_tests.rs` failures (section 1.2) remain open; they are pre-existing, out of scope for this six-test remediation slice, and relate to the open AUD-01/AUD-05/R4 gates.
- The independent Stage 06 re-audit required before `08-verify` has **not** been performed. This report claims no audit approval, no security sign-off, and no release readiness.

## 6. Completion condition

Local non-database gates are green and the crate-boundary, lint, route-inventory, and ICM contracts are enforced. The six assigned PostgreSQL membership failures are fixed and proven green on exact head `3ee27a3` (integration run 40158776564, `membership_transaction_integration.rs` 44/44). Verification is **not** complete for release: the three pre-existing provision/invitation evidence failures keep the database-backed CI jobs red, and the independent Stage 06 re-audit plus human review are still required. Control returns to `06-audit` for the re-audit; `09-deliver` is not entered. PR #49 remains open and unmerged.
