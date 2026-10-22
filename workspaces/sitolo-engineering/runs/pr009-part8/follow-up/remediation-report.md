# Remediation Report — PR #49 / Phase 4 Part 8

## Exact-head update — `8eaaf18530027a3fdbb854361a1aa4a0ba580e30` (2026-10-22)

This update supersedes earlier remediation statements below when they conflict with the current source. The branch was fetched before work and remains `feat/phase4-pr009-audit-outbox`; PR #49 is open and unmerged.

The invitation acceptance PostgreSQL tests now use token fixtures that meet the existing minimum token length. A new PostgreSQL integration regression races token acceptance, invitation withdrawal, and the expiry sweeper against an already-expired invitation. It verifies acceptance is rejected, exactly one terminal invitation transition wins, terminal contact/token-hash fields are deleted, and the corresponding invitation event is emitted once. The accepted v1 membership-expiry schema permits `INVITED → EXPIRED`, and the sweep emits `iam.membership.expired` only when it actually changes the linked membership.

Exact-head GitHub Actions passed all nine reported checks: PostgreSQL integration [39737396713](https://github.com/jelome265/sitolo/actions/runs/39737396713), Rust verification [39737395716](https://github.com/jelome265/sitolo/actions/runs/39737395716), policy [39737395738](https://github.com/jelome265/sitolo/actions/runs/39737395738), security [39737395727](https://github.com/jelome265/sitolo/actions/runs/39737395727), and CodeQL in the security run. The PostgreSQL integration output includes the new race test passing and 34 membership transaction integration tests passing. Local PostgreSQL was not started or used; database evidence is from GitHub Actions only.

**Current disposition:** AUD-04 has implementation and exact-head PostgreSQL evidence for the five approved lifecycle mappings, but remains **PARTIAL pending independent Stage 06 re-audit**. AUD-03 remains **OPEN** until the independent review confirms transaction and rollback coverage across every mandatory IAM mutation and actor/tenant provenance; the API route remains fail-closed because production OIDC validation, session lookup, and server-side membership/permission resolution are not composed. AUD-01, AUD-02, and AUD-05 remain open. No merge or production integration was performed.

## Exact-head implementation update — 2026-10-22

Verified code source head: `75ff3e4c1612a170aaaa88b1ba89932ee4d4efd4`. On this source head, all nine reported checks passed: PostgreSQL integration [39729489081](https://github.com/jelome265/sitolo/actions/runs/39729489081), Rust/workspace verification [39729489126](https://github.com/jelome265/sitolo/actions/runs/39729489126), policy [39729489089](https://github.com/jelome265/sitolo/actions/runs/39729489089), and security [39729489097](https://github.com/jelome265/sitolo/actions/runs/39729489097), including CodeQL. No result is carried forward from prior source heads.

The latest persistence changes serialize organization-scoped mutation and lifecycle transactions through a tenant-checked transaction advisory lock. The organization and branch transition functions take the same lock. Their dedicated owner role now has a narrowly scoped tenant SELECT policy required by forced RLS for `UPDATE ... RETURNING`; `app_runtime` retains no direct organization or branch UPDATE grant. PostgreSQL integration and policy verification pass on the exact head.

**AUD-03 remains OPEN.** This corrects organization/branch concurrency and RLS boundaries, but does not implement durable transactions for role/scope grants, invitation acceptance and expiry sweep, or prove every required mutation's rollback/race matrix. **AUD-04 remains PARTIAL.** The new event mappings are still not wired across every approved transition; invitation expiry sweeping and integration proof remain outstanding. AUD-01, AUD-02 and AUD-05 remain open. The protected HTTP mutation path remains fail-closed until verified identity and server-derived authority are composed.

No local PostgreSQL was started or used. Database-backed proof above is from GitHub Actions only. PR #49 remains open and unmerged; exact-head re-audit and human review remain required.

## Implementation addendum — source SHA `b2d0bb876886e636063c8d19c8fbb6c9ed9623c8`

The current source adds PostgreSQL transactions coupling direct branch creation and member activation, suspension, resumption, and revocation with their audit and outbox writes. Existing transactional paths remain for organization provisioning, member pending acceptance, and membership expiry. PostgreSQL integration tests exercise member suspension success and rollback when either audit or outbox writing fails. Exact-head CI is in progress; at the time of this addendum the PostgreSQL application workflow [39724526083](https://github.com/jelome265/sitolo/actions/runs/39724526083), Rust verification [39724526062](https://github.com/jelome265/sitolo/actions/runs/39724526062), and policy workflow [39724526094](https://github.com/jelome265/sitolo/actions/runs/39724526094) passed; the security workflow remains in progress. Do not infer final checks from previous SHAs.

Operations without a transaction-backed PostgreSQL path now fail before changing the reference store whenever the evidence sink is enabled. This prevents further split-brain state/evidence writes, but it is containment, not implementation of those operations.

**AUD-03 remains OPEN.** Role/scope operations, invitations, organization lifecycle, branch lifecycle transitions and the broader fault/race matrix are not yet backed by PostgreSQL state/audit/outbox transactions. **AUD-04 remains PARTIAL.** The five names and schemas are registered; member pending and membership expiry are wired, while invitation issuance, withdrawal and per-invitation expiry remain unwired. **AUD-02 remains OPEN:** the live HTTP boundary still denies protected writes because verified identity/session/tenant authorization is not composed. AUD-01 and AUD-05 remain open as recorded below.

No local PostgreSQL instance was used. It is deferred to the next phase; the repository's PostgreSQL-enabled GitHub Actions workflow is the database verification environment for this phase. The PR remains open and unmerged.

## Current implementation addendum — 2026-10-22

This addendum supersedes historical implementation and verification details below where they conflict. Current code head: `7752636695576ad49d57175eb0d9a114fd911539`. All 10 required CI checks passed on that exact SHA: PostgreSQL integration [run 39721887375](https://github.com/jelome265/sitolo/actions/runs/39721887375), Rust verification [run 39721886981](https://github.com/jelome265/sitolo/actions/runs/39721886981), policy [run 39721886982](https://github.com/jelome265/sitolo/actions/runs/39721886982), security [run 39721887010](https://github.com/jelome265/sitolo/actions/runs/39721887010), and CodeQL [check 119637951175](https://github.com/jelome265/sitolo/runs/119637951175). Earlier exact-head CI at `e04b4fcd5e7e43a151daa129b8d9c7b0e125c055` also passed all 10 checks; no results are carried forward between source heads.

### Current changes

- `mark_member_pending` and `expire_invitation` now update membership state/version and write their approved audit/outbox events in one tenant-scoped PostgreSQL transaction. Integration coverage exercises success, duplicate and concurrent transitions, and rollback when audit or outbox insertion fails.
- PostgreSQL audit/outbox event-name constraints now accept the approved `pending_acceptance` event name. Exact-head CI confirmed the new path passes against PostgreSQL.
- The approved retention rule is implemented in the in-memory invitation lifecycle: accepted, withdrawn and expired invitations erase contact/token-hash fields, and the token lookup index is removed. Replays after terminalization are reported as invalid/unknown.
- Local PostgreSQL execution has not been performed, as intended for this phase. Database-backed verification is in CI.

### Current finding disposition

| Finding | Current status | Remaining work |
|---|---|---|
| AUD-01 relay | **OPEN** | Production SNS/SQS publisher, consumer inbox/idempotency, infrastructure and real delivery proof. |
| AUD-02 authority | **OPEN** | Verified OIDC principal and server-derived API actor/owner/tenant authority at the live HTTP boundary. |
| AUD-03 atomicity | **OPEN** | Only organization provisioning and the two membership transitions above have PostgreSQL transaction coupling. Other mandatory IAM mutations remain incomplete; invitation storage and all-path rollback/race coverage are outstanding. |
| AUD-04 event mapping | **PARTIAL** | Five v1 names and strict payload schemas exist; membership pending and expiry are wired. Invitation issue, withdrawal and per-invitation expiry sweep still need authoritative PostgreSQL transitions and integration evidence. |
| AUD-05 runtime | **OPEN** | Managed production secret provider and fully composed API/worker runtime remain incomplete; fail-closed guards stay. |

The decision record documents the owner-approved terminal invitation retention rule. These changes do not close AUD-03/AUD-04 or make PR #49 merge-ready. Stage 06 re-audit and human review remain required.

## Exact-head implementation update — 2026-10-22

Verified implementation head: `8a4a58bb55512343eeb31056a83610a34bc57b00`. All 10 required PR checks passed on this exact head: PostgreSQL integration [39731541826](https://github.com/jelome265/sitolo/actions/runs/39731541826), Rust/workspace verification [39731541832](https://github.com/jelome265/sitolo/actions/runs/39731541832), policy [39731541896](https://github.com/jelome265/sitolo/actions/runs/39731541896), security [39731541825](https://github.com/jelome265/sitolo/actions/runs/39731541825), and CodeQL [119668322293](https://github.com/jelome265/sitolo/runs/119668322293). No check result is carried over from another source head.

Since the preceding update, PostgreSQL-backed invitation issuance, withdrawal, and bounded expiry sweeping now persist invitation state together with mandatory audit and outbox events. The sweep uses tenant-scoped transactions and `FOR UPDATE SKIP LOCKED`; it removes contact and token-hash data on terminalization, expires a linked `INVITED` membership when it actually transitions, and is safe under repeated/concurrent sweeps. Integration tests cover state/event outcomes and rollback on audit/outbox failures. Direct membership expiry now also has audit/outbox rollback regression coverage. The owner-approved v1 `iam.membership.expired` schema permits `INVITED → EXPIRED` in addition to `PENDING_ACCEPTANCE → EXPIRED`, as recorded in the lifecycle mapping decision.

**AUD-03 remains OPEN.** These additions expand PostgreSQL transaction coverage but do not establish authoritative transaction handling and complete rollback/race coverage for every mandatory IAM operation (including outstanding role/scope grant and acceptance paths), nor do they resolve the live API authority gap. **AUD-04 implementation is present for the five approved mappings and has PostgreSQL integration evidence on this head; independent Stage 06 re-audit is still required before closure.** AUD-01, AUD-02 and AUD-05 remain open. Protected HTTP writes remain fail-closed until authenticated identity and server-derived authority are composed.

GitHub Actions PostgreSQL integration is the database evidence for this phase. **No local PostgreSQL instance was started or used; local PostgreSQL verification is deferred to the next phase.** PR #49 remains open and unmerged. Exact-source re-audit and human review remain required.

**Date:** 2026-10-22  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Verified implementation SHA:** `441ca1fd7974bfbbac2042ac900c82cea1fd3c63`
**Disposition:** Partial remediation. Re-audit is required for the changed event catalogue. AUD-01 through AUD-05 are not closed.

## Evidence-first baseline

The branch was fetched and fast-forward checked before edits; it was already at `7326098b9b2da614bc6c99dd08ef2b9a46c01820`, the remote PR head. The implementation changes were committed as `441ca1fd7974bfbbac2042ac900c82cea1fd3c63`. All 10 PR checks passed for that exact source SHA, including the new PostgreSQL integration regression. Local PostgreSQL execution is planned for the next engineering phase and is not claimed here.

## Finding disposition

| Finding | Requirement / root cause | Owner and dependency | Change and proof | Status |
|---|---|---|---|---|
| AUD-01 relay | Approved SNS FIFO/SQS FIFO contract requires a real publisher and durable consumer inbox; current worker has no AWS destination adapter or consumer integration. | Platform operations and consumer owners; depends on production AWS account/resources, R2/R3 and R5. | No adapter or fake integration was added. The executable remains fail-closed. | **OPEN / NOT IMPLEMENTED** |
| AUD-02 authority | HTTP commands need a verified OIDC principal and server-derived membership/tenant authorization; current live boundary does not provide it. | Identity/security owner; owner-approved Cognito choice recorded, but route/application design and real token validation remain. | No authorization shortcut or client-derived authority was added. | **OPEN / NOT IMPLEMENTED** |
| AUD-03 atomicity | Each mandatory state mutation, audit row and outbox row must share one authoritative PostgreSQL transaction; most tenancy mutations still update the in-memory store before separately recording evidence. | Engineering/persistence owner; depends on durable operation boundaries and trusted actor context (R2). | This patch does not close the broad IAM transaction-coverage gap. The specific enqueue-boundary hole is closed by validating mapped payloads before sequence allocation or outbox writes; transactional IAM state coverage remains outstanding. | **OPEN** |
| AUD-04 event mapping | The approved five lifecycle transitions were absent from the binding catalogue and `IamEventName`. | Engineering/domain owner; mappings are owner-approved. | Added five distinct v1 variants and stable dotted names to `IamEventName::ALL` / `as_str()`, updated the binding contract catalogue, and added strict v1 payload validators enforced at event construction, PostgreSQL enqueue, and relay acceptance. A mutated malformed event is rejected before database writes; malformed relay events are quarantined before publication. Actual transition wiring and transaction/race coverage remain incomplete. | **PARTIAL / RE-AUDIT REQUIRED** |
| AUD-05 runtime | Production composition needs real Secrets Manager, PostgreSQL pools and restricted identities; current bootstrap correctly refuses an absent managed provider. | Platform/security owner; requires AWS account, IAM, secret references and database provisioning. | No production fallback or fabricated integration was introduced. Existing fail-closed guards remain. | **OPEN / NOT IMPLEMENTED** |

## Changed files

- `crates/sitolo-audit/src/iam_event.rs` — approved event registry values and regression test.
- `docs/phase4_part8_audit_outbox_implementation_contract.md` — canonical catalogue and payload constraints.
- `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/decisions/iam-lifecycle-event-mapping-proposal.md` — status now distinguishes catalogue implementation from remaining wiring/atomicity proof.
- `crates/sitolo-events/src/payload.rs` — strict allowlisted payload validation for the five approved v1 lifecycle schemas.
- `crates/sitolo-events/src/outbox.rs`, `crates/sitolo-events/Cargo.toml`, and `apps/worker/src/relay.rs` — enforce schemas at construction and relay acceptance; reject/quarantine malformed mapped payloads.
- `crates/sitolo-persistence/src/outbox_repository.rs` and `crates/sitolo-persistence/tests/part8_postgres_integration_tests.rs` — independently validate at PostgreSQL enqueue and cover post-construction event mutation before persistence.

## Verification

| Check | Result |
|---|---|
| Failing-before-fix registry test | **FAILED as expected**: missing `iam.membership.pending_acceptance` |
| `cargo test -p sitolo-audit --lib --locked` | **PASS**, 4 tests |
| `cargo test -p sitolo-events --lib --locked` | **PASS**, 12 tests |
| `cargo test -p sitolo-worker-bin --lib --locked` | **PASS**, 7 tests, including malformed lifecycle payload quarantine |
| `cargo clippy -p sitolo-events -p sitolo-worker-bin --all-targets --locked -- -D warnings` | **PASS** |
| `cargo test -p sitolo-persistence --test part8_postgres_integration_tests --no-run --locked` | **PASS**, integration target compiles; DB execution is delegated to PostgreSQL CI |
| `cargo clippy -p sitolo-persistence --all-targets --locked -- -D warnings` | **PASS** |
| `cargo fmt --all -- --check` | **PASS** |
| `cargo check --workspace --all-targets --all-features --locked -j 1` | **PASS** |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS** |
| `cargo test -p sitolo-config -p sitolo-security --all-targets --all-features --locked -j 1` | **PASS**, 42 tests |
| `cargo test -p sitolo-api-bin --test startup_env --all-features --locked -j 1` | **PASS**, 2 tests |
| `python scripts/ci/check-doc-references` | **PASS**, 221 Markdown files / 1,027 references; zero errors |
| `scripts/ci/check-workflow-policy.ps1` | **PASS** |
| `scripts/ci/check-icm-workspace` | **PASS**, 9-stage mechanical cold walk |
| `scripts/ci/check-release-workflow` | **PASS**, PostgreSQL services and scoped DSNs are present in the release workflow |
| Baseline `cargo metadata --locked --format-version 1` | **PASS** |
| Baseline `cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS** |
| Local `cargo test --workspace --locked` | **NOT RUN TO COMPLETION**: database-free tests passed, but PostgreSQL tests require local `ADMIN_DATABASE_URL`, `RUNTIME_DATABASE_URL`, and `WORKER_DATABASE_URL`; local execution is deferred to the next phase |
| Exact-source PR CI at `441ca1f` | **PASS**, all 10 checks, including the mutated-event PostgreSQL enqueue regression: [Rust/toolchain and verification](https://github.com/jelome265/sitolo/actions/runs/39715813157), [PostgreSQL integration](https://github.com/jelome265/sitolo/actions/runs/39715813182), [security](https://github.com/jelome265/sitolo/actions/runs/39715813194), [policy](https://github.com/jelome265/sitolo/actions/runs/39715813153), [CodeQL](https://github.com/jelome265/sitolo/runs/119618293552) |
| Stage 06 re-audit | **NOT RUN**; required after implementation changes |

## Next required work

Implement R2/R3 transaction and authenticated-authority boundaries before wiring the approved transition methods. Then add database-backed fault, race and idempotency tests for each mapping, compose R5 dependencies without enabling production fallback, and implement R1/R6 adapter/consumer integration against the approved contract. Re-audit the resulting exact source head in Stage 06 before Stage 08 verification. Keep PR #49 open and unmerged pending required human review.
