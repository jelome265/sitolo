# Remediation Report — PR #49 / Phase 4 Part 8

**Date:** 2026-10-22  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Base SHA:** `5d22468713011911734938e293b28ef0a58764fb`  
**Disposition:** Partial remediation. Re-audit is required for the changed event catalogue. AUD-01 through AUD-05 are not closed.

## Evidence-first baseline

The branch was fetched and fast-forward checked before edits; it was already at the origin head and clean. `cargo metadata --locked --format-version 1` and `cargo clippy --workspace --all-targets --locked -- -D warnings` passed before the change. `cargo test --workspace --locked` compiled, and database-free tests passed, but the suite failed when the PostgreSQL integration harness required the three local test DSNs. The first concurrent test run also hit Windows paging-file exhaustion; the serial retry completed compilation. No PostgreSQL service or Docker executable is available on this host, so database integration evidence is **BLOCKED**, not passed.

## Finding disposition

| Finding | Requirement / root cause | Owner and dependency | Change and proof | Status |
|---|---|---|---|---|
| AUD-01 relay | Approved SNS FIFO/SQS FIFO contract requires a real publisher and durable consumer inbox; current worker has no AWS destination adapter or consumer integration. | Platform operations and consumer owners; depends on production AWS account/resources, R2/R3 and R5. | No adapter or fake integration was added. The executable remains fail-closed. | **OPEN / NOT IMPLEMENTED** |
| AUD-02 authority | HTTP commands need a verified OIDC principal and server-derived membership/tenant authorization; current live boundary does not provide it. | Identity/security owner; owner-approved Cognito choice recorded, but route/application design and real token validation remain. | No authorization shortcut or client-derived authority was added. | **OPEN / NOT IMPLEMENTED** |
| AUD-03 atomicity | Each mandatory state mutation, audit row and outbox row must share one authoritative PostgreSQL transaction; most tenancy mutations still update the in-memory store before separately recording evidence. | Engineering/persistence owner; depends on durable operation boundaries and trusted actor context (R2). | Not changed because adding event writes after current store mutations would preserve the finding rather than fix it. PostgreSQL fault-injection tests cannot run without the approved disposable database. | **OPEN / BLOCKED** |
| AUD-04 event mapping | The approved five lifecycle transitions were absent from the binding catalogue and `IamEventName`. | Engineering/domain owner; mappings are owner-approved. | Added five distinct v1 variants and stable dotted names to `IamEventName::ALL` / `as_str()`, updated the binding contract catalogue and minimal payload constraints, and corrected the decision record’s implementation status. A registry test first failed because `iam.membership.pending_acceptance` was missing, then all four `sitolo-audit` library tests passed. Transition wiring, payload validation and transactional evidence remain incomplete. | **PARTIAL / RE-AUDIT REQUIRED** |
| AUD-05 runtime | Production composition needs real Secrets Manager, PostgreSQL pools and restricted identities; current bootstrap correctly refuses an absent managed provider. | Platform/security owner; requires AWS account, IAM, secret references and database provisioning. | No production fallback or fabricated integration was introduced. Existing fail-closed guards remain. | **OPEN / NOT IMPLEMENTED** |

## Changed files

- `crates/sitolo-audit/src/iam_event.rs` — approved event registry values and regression test.
- `docs/phase4_part8_audit_outbox_implementation_contract.md` — canonical catalogue and payload constraints.
- `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/decisions/iam-lifecycle-event-mapping-proposal.md` — status now distinguishes catalogue implementation from remaining wiring/atomicity proof.

## Verification

| Check | Result |
|---|---|
| Failing-before-fix registry test | **FAILED as expected**: missing `iam.membership.pending_acceptance` |
| `cargo test -p sitolo-audit --lib --locked` | **PASS**, 4 tests |
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
| `cargo test --workspace --locked` | **BLOCKED** at PostgreSQL integration tests: `ADMIN_DATABASE_URL` absent; Docker/PostgreSQL tooling unavailable locally |
| Exact-source CI / re-audit | **NOT RUN** |

## Next required work

Implement R2/R3 transaction and authenticated-authority boundaries before wiring the approved transition methods. Then add database-backed fault, race and idempotency tests for each mapping, compose R5 dependencies without enabling production fallback, and implement R1/R6 adapter/consumer integration against the approved contract. Re-audit the resulting exact source head in Stage 06 before Stage 08 verification. Keep PR #49 open and unmerged pending required human review.
