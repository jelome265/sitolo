---
artifact_type: remediation-report
status: complete
run_slug: "pr009-part8"
---

# Remediation Report

**PR:** #49 — Phase 4 Part 8 audit and outbox
**Branch:** `feat/phase4-pr009-audit-outbox`
**Audit input:** `docs/phase4_part8_pr009_remediation_report.md` (source `a1d29ae`)
**Remediation contract:** `docs/phase4_part8_pr009_remediation_execution_contract.md`
**Current source basis:** `5a128c4f95db61f7f6cf9fd885d22911b78553c1` (2026-10-25)
**Security relevant:** yes

## Audit input provenance

The ICM stage directories contain no `01-select`–`06-audit` artifacts for this run; every
`output/` directory holds only `.gitkeep`. The audit evidence for this run is therefore the
project-knowledge report named above, not a stage artifact. This report does **not** claim a
Stage 06 audit result and does not substitute for the independent re-audit required before
verification. `06-audit` was not retro-filled, because fabricating audit approval is forbidden
by the ICM contract and checked by `scripts/ci/check-icm-workspace`.

## Artifact location

`scripts/ci/check-icm-workspace` forbids committed run artifacts inside the reusable
`stages/*/output/` shelf, so this report is archived with the run at
`workspaces/sitolo-engineering/runs/pr009-part8/follow-up/`. The stage contract still names its
`output/` directory as the runtime write target; that directory remains `.gitkeep`-only in version
control.

## Findings addressed

Ordered by `workspaces/sitolo-engineering/stages/07-remediate/references/remediation-order.md`:
release-blocking integrity first, then contract violations, then transaction/concurrency defects,
then missing tests, then operational gaps, then documentation drift.

| Finding | Root cause | Fix | Proof |
|---|---|---|---|
| C-1 (integrity, release-blocking) Branch activation returned `permission denied for table branches` and surfaced HTTP 503 | The runtime role holds only the least-privilege grants; `UPDATE` on `branches` was never granted, and widening the grant would have broken least privilege | Branch activation now routes through the existing `SECURITY DEFINER transition_branch_state` function instead of widening grants | `crates/sitolo-persistence/src/tenancy.rs` activation path; DB proof runs in GitHub Actions only |
| C-2 (contract) Application named persistence types; the dependency edge pointed the wrong way | Ports and DTOs were defined in `sitolo-persistence`, so application depended on persistence | Atomic inversion: `sitolo-application` now owns `identity_ports.rs` and `tenancy_ports.rs`; `sitolo-persistence` implements them and re-exports nothing of its own. Persistence depends on application, never the reverse | `python scripts/ci/check-crate-boundaries.py`; `crates/sitolo-application/src/tenancy_ports.rs` |
| C-3 (contract) Boundary rule was a hand-maintained allow-list, so a new unclassified crate could pass | Contract was a sample, not exhaustive | Contract is now exhaustive and metadata-driven from `Cargo.toml`, and runs as an early independent CI job before format/lint/test | `scripts/ci/check-crate-boundaries.py`; `.github/workflows/rust.yml` `architecture` job |
| C-4 (contract) Test-only hybrid tenancy composition leaked persistence types into application | Evidence-only writes were typed against the concrete persistence adapter | Composition now accepts `Arc<dyn TenancyUnitOfWork>` for evidence-only writes, so application never names a persistence type | `crates/sitolo-application/tests/part8_provision_organization_evidence_tests.rs` |
| C-5 (transaction) Evidence-write failure rollback proofs were attached to deleted failing sinks | The failing sinks disappeared with the port inversion | Replaced by a test-only `FailingUnitOfWork`/`FailingTransaction` that injects failure in `record_evidence` while delegating every other transaction method | `membership_transaction_integration.rs`, `part8_provision_organization_evidence_tests.rs` |
| C-6 (transaction) Durable tenancy commands executed SQL inside `sitolo-application` | Service reached past its own port | `TenancyService` now uses `durable()`/`reference()` accessors only; all SQL moved behind `TenancyUnitOfWork`/`TenancyTransaction` | `crates/sitolo-application/src/tenancy.rs`; persistence adapter |
| C-7 (build) Release profile had lost overflow checks and LTO | Profile keys were dropped during an earlier edit | Restored `overflow-checks = true` for dev and release, `lto = "thin"`, `debug = "line-tables-only"` | workspace `Cargo.toml` |
| C-8 (CI, release-blocking) `cargo clippy --workspace --all-targets --all-features -- -D warnings` fails | `[lints] workspace = true` was added to `apps/api` and `apps/worker` in `0cdd771`, exposing pre-existing lint debt on the binary surface that had never been checked; the audit report's green clippy claim predates that change | Strict policy kept and defects fixed at the root: missing docs, `must_use`, discarded errors, unchecked arithmetic and indexing, long functions | `cargo clippy --workspace … -D warnings` exit 0 (see Tests) |
| C-9 (contract) Invitation policy verification accepted a spurious `USING` clause when the contract expected none | Lax match arm treated any non-matching pair as acceptable | Tightened to strict `(Some, None) => false` matching | `crates/sitolo-persistence/src/postgres.rs` `verify_part8_provisioning_rls_metadata` |
| C-10 (test quality) `verify_part8_provisioning_rls_metadata` was a 598-line unverifiable block; concurrency tests joined threads as they were created | Structure, not intent | Split into an orchestrator plus focused helpers; concurrency tests now spawn all threads before joining any, so they prove real contention | `crates/sitolo-persistence/src/postgres.rs`; `tenancy.rs` concurrency tests |
| C-11 (operational) 17 packages carried no real metadata | Missing package fields | Every package carries description, repository, readme, keywords and categories; `multiple_crate_versions` is allowed at workspace level with a written reason | `cargo metadata`; workspace `Cargo.toml` |

Not closed by this report: R3 command-by-command mutation matrix, R4/AUD-04 transition mapping,
AUD-01 consumer completion, AUD-05 worker runtime/migration path. These need the owner-approved
consumer and migration contracts and an independent audit; see Remaining risks.

## Follow-up hardening at `5a128c4`

After the first remediation commit, the framework-ban policy and the protected-route inventory were
reconciled. These changes do not weaken any gate; they make the existing policy executable.

| Change | Reason | Proof |
|---|---|---|
| Activated the reachable framework bans from the enterprise reference policy (`tokio::spawn`, runtime construction, raw `reqwest` clients, `Router::route`, `TimeoutLayer::new`, clap exit-on-error parsing, direct environment reads, rustls dangerous APIs, blocking HTTP) | The workspace policy claimed `disallowed_methods`/`disallowed_types` were deny but the call-site bans were incomplete | `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0 with the bans active |
| Documented and proved two deliberate omissions | `Runtime::block_on` fires on all 183 `#[tokio::test]` expansions, so a workspace ban would need 183 suppressions; `disable_recursion_limit` is unreachable without the `unbounded_depth` feature | Scratch-crate probes; `[[bans.features]]` for `serde_json/unbounded_depth` in `deny.toml` fires with `error[feature-banned]`, exit 2 |
| Added item-scoped `#[expect(..., reason = ...)]` at the two runtime entry points, the reviewed route registry, the local-only secret adapter, and bounded test spawn/DSN helpers | Genuine structural exceptions only; no blanket suppression | `cargo clippy` clean; each reason states why the exception is safe |
| Made route wiring exhaustive and the organization-activate gap explicit | A wildcard handler hid which protected routes were unimplemented, and provisioning never persists `PROVISIONING`, so the activate route is unreachable | `CLOSED_PROTECTED_ROUTES`, `declared_routes()`, `InvalidRouteInventory` on an undeclared route; R2 partition test |
| Added R2 deny-by-default proof over the inventory | Route coverage was previously asserted per-route and could drift | `apps/api/tests/axum_boundary.rs`: all six protected routes reject missing credentials; exactly five are wired and one is closed |
| Added R4 catalogue/validator consistency proof | The five approved names and the payload validator are separate registries that could drift | `crates/sitolo-application/tests/lifecycle_event_contract.rs`: each name accepts only its v1 shape and rejects wrong versions and unknown fields |

R4 transition emission (which domain transition constructs which payload and persists it) is still a
source-trace item for Stage 06; the new test proves registry consistency, not emission coverage.

## Tests

Evidence is recorded only for what was actually executed locally. No local PostgreSQL instance was
installed, started, or used.

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | PASSED |
| Lint | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASSED (exit 0, no diagnostics) |
| Compile | `cargo check --workspace --all-targets --all-features --locked` | PASSED |
| Release build | `cargo build --workspace --release --locked` | PASSED (finished in 5m 21s) |
| Unit tests | `cargo test --workspace --lib --all-features --locked` | PASSED — 259 tests across 17 packages, 0 failed |
| R2 route coverage | `cargo test -p sitolo-api-bin --test axum_boundary --all-features --locked` | PASSED — 3 tests: every declared protected route rejects missing credentials; closed set is exactly the documented gap |
| R4 event contract | `cargo test -p sitolo-application --test lifecycle_event_contract --all-features --locked` | PASSED — 2 tests: five approved names, v1-only shapes, unknown-field rejection, unique names |
| Crate boundaries | `python scripts/ci/check-crate-boundaries.py` | PASSED (17 workspace packages classified) |
| Architecture | `./scripts/ci/check-architecture` | PASSED (pure-crate graph, single logging installer, single fingerprint owner, boundary contract) |
| Phase 2 policy | `./scripts/ci/check-phase2-policy` | PASSED |
| Workflow policy | `./scripts/ci/check-workflow-policy`; `./scripts/ci/check-release-workflow` | PASSED |
| Security ICM policy | `./scripts/ci/check-security-icm-policy` | PASSED |
| Documentation | `./scripts/ci/check-doc-references` | PASSED — 238 Markdown files, 1054 references, 0 unresolved |
| System Map | `./scripts/ci/check-system-map-processes` | PASSED — 2 source-grounded process cards |
| Dependency policy | `cargo deny check` | PASSED (advisories, bans, licenses, sources) |
| ICM workspace | `./scripts/ci/check-icm-workspace` | PASSED (9 stages, exact handoffs, no synthetic approval) |

Per-crate unit-test counts observed locally: `sitolo-api` 20, `sitolo-api-bin` 7, `sitolo-audit` 4,
`sitolo-auth` 53, `sitolo-authz` 22, `sitolo-config` 24, `sitolo-domain` 11, `sitolo-events` 12,
`sitolo-integrations` 12, `sitolo-observability` 10, `sitolo-persistence` 41, `sitolo-security` 18,
`sitolo-tenancy` 8, `sitolo-worker-bin` 17. `sitolo-application`, `sitolo-sync` and
`sitolo-testkit` have no library tests; their proof is the integration targets listed below.

Database-backed suites are not run locally and are not claimed: `rls_security_tests`,
`part8_postgres_integration_tests`, and the application PostgreSQL integration tests require the
CI PostgreSQL service and the three DSNs. `cargo-audit` is not installed on this host; the CI
`advisories` job runs it. This report therefore claims local evidence only for the non-database
gates it lists, and the exact-head GitHub Actions run remains the authority for the database and
security-advisory gates.

## Re-audit required

Yes. Implementation changed, so this returns to `06-audit` for an independent Stage 06 re-audit
before `08-verify`. This report claims no audit result, no approval, and no release readiness.

## Remaining risks

- R3 command-by-command matrix is refreshed at `5a128c4` but remains a source-inspection index, not
  independent audit evidence. Organization activation is marked unreachable and its route is
  explicitly closed; it must not be re-opened without a real `PROVISIONING` persistence path.
- R4 registry consistency is proven locally; transition emission and relay acceptance still require
  a Stage 06 source trace.
- AUD-01 and AUD-05 remain open. The worker now runs the bounded relay and SNS FIFO publisher, but
  the SQS consumer, durable inbox deduplication and restricted worker migration/release path are not
  implemented. Relay completion must not be inferred from a compiling consumer module.
- The full PostgreSQL and HTTP-level proof only exists once GitHub Actions runs against the exact
  head. A local green lint/test result is evidence only for the code paths it exercised.
- Exact-head CI has not been run for `5a128c4`; the branch has not been pushed.
