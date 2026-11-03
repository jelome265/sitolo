---
artifact_type: copy-paste-execution-guide
status: ready-for-controlled-execution
run_slug: "pr009-part8"
security_relevant: yes
candidate_head: "a378beec2bc5097bb642825d2e8fcf4c967c105d"
date: "2026-10-25"
---

# PR #49 / Phases 1–4 — Copy/Paste Agent Execution Guide

## How to use

Copy the prompt below into Codex, Claude Code or your implementation agent from a checked-out repository. The first pass is read-only. If remediation is separately authorized, use the bounded work order and acceptance criteria; never make CI green by weakening controls.

## Copy/paste prompt

You are the senior Rust systems/security engineer working in jelome265/sitolo.

### Objective

Close the proven Phase 1–4 gaps for PR #49 on branch feat/phase4-pr009-audit-outbox. Start by checking the live branch HEAD and reading:

- workspaces/sitolo-engineering/runs/pr009-part8/follow-up/phase1-4-deep-audit-2026-10-25.md
- workspaces/sitolo-engineering/runs/pr009-part8/follow-up/phase1-4-remediation-plan-2026-10-25.md
- docs/phase1_repository_rust_workspace_ci_deep_implementation.md
- all Phase 2 specifications/remediation docs listed in the audit report
- docs/phase3_identity_sessions_mfa_device_identity_implementation.md
- docs/phase4_tenant_organization_branch_iam_implementation.md
- docs/phase4_part8_audit_outbox_implementation_contract.md
- docs/phase_contract_coverage_register.md
- docs/testing_strategy.md, docs/security_test_harness.md and docs/security_exception_register.md
- ICM stages 01-select through 09-deliver and all current follow-ups; treat reports pinned to older SHAs as historical.
- all source/test/CI files named by each finding.

Do not assume the audit’s pinned source is still HEAD. Record exact HEAD before reading code and re-check before delivery.

### Mandatory operating constraints

1. Preserve existing user work. Do not force-push, run git reset --hard, rewrite published history or discard uncommitted changes. Check git status first.
2. Initial pass is read-only. Inspect source, contracts, tests, CI, dependency graph and ICM. Do not edit code until a specific finding and acceptance criteria are established.
3. Keep Rust-first layered architecture. Domain is infrastructure-independent; Application owns use cases/ports; PostgreSQL/SQLx adapters stay in Persistence; API/worker crates compose layers only where the boundary contract permits.
4. Never use mocks/fakes/in-memory adapters as proof of PostgreSQL, RLS, transaction, locking or tenant-isolation behavior. Those tests require real disposable PostgreSQL. Narrow provider fakes can support local adapter unit tests only and never count as end-to-end external delivery evidence.
5. No production mock, no no-op business effect, no fake authorization success and no in-memory production authority fallback. Do not wire a consumer until the domain owner approves the exact effect.
6. Keep Clippy/Rust policy strict. Never add blanket allow, disable all lints, use --cap-lints allow, use an expect without a narrow reason, permit unsafe code to silence a failure or weaken test assertions. Any expect must be narrow, justified, reviewed and included in exception inventory.
7. Do not turn missing DB/env/provider/tool into success, skip required tests or hide exit status. Never publish a green report if a required test did not run.
8. Never run destructive tests against personal, shared, staging or production databases. Do not commit secrets/DSNs. Before any DB test connects or performs DDL, prove its target is disposable; if it cannot, stop before network/DDL.
9. POSTGRES_HOST_AUTH_METHOD=trust is limited to isolated disposable CI containers. Do not reuse it in production or shared DB.
10. Do not use curl | sh, arbitrary downloaded scripts, unreviewed broad deletion, DROP DATABASE, or wildcard cleanup. Scoped temp cleanup may remove only the dedicated temp directory created by that run; schema cleanup only its generated schema after target validation.
11. Do not invent AWS account IDs, ARNs, queue names, secret refs, provider semantics, migration owners or a domain business effect. If a decision is absent, keep code fail-closed and record the blocker.
12. Do not merge, approve or close PR #49. Do not claim phase completion before evidence and independent review. Do not alter the independent Stage 06 output shelf; this is a supplemental report, not independent approval.
13. Do not install/start local PostgreSQL for this PR scope unless the governing contract is formally changed. Use the approved disposable PostgreSQL GitHub Actions service for required DB integration proof.
14. Distinguish verified facts, assumptions, risks and decisions. Every finding includes exact path/symbol, source SHA, reproduction/evidence, impact and pass criterion.

### First deliverable: read-only re-audit

Before editing:
- Record branch, HEAD SHA, dirty state, base SHA and CI status for that exact HEAD.
- Inventory all Phase 1–4 phase-specific docs and linked support/remediation records. Preserve historical records and do not treat them as current truth.
- Verify whether each cited finding still exists at current HEAD.
- Inspect every workspace and per-crate Cargo lint declaration, .cargo/config.toml, build flags, CI workflows, lint suppressions, unsafe blocks, SQL DDL and destructive test commands.
- Trace mandatory Phase 4 mutation paths from route/consumer -> authenticated principal -> authorization/scope -> use case -> application port -> PostgreSQL transaction -> state + audit + outbox -> relay -> consumer inbox/effect -> acknowledgement. Record absent edges.
- Check workflow head SHA, not just a green run with similar name.
- Produce a source-grounded finding table: ID, priority, current status, file/symbol, evidence, impact, change, test and owner role.
- Stop after reporting if the request is report-only. Do not modify code during that pass.

### Remediation order after authorization

**Unit A — database target safety**
- Parse all database DSNs and validate host, dbname, role and consistent target before connecting or DDL.
- Reject production-like targets, unexpected remote hosts, conflicting DSNs and missing opt-in before network/DDL.
- Keep real PostgreSQL tests. Add negative tests proving invalid targets fail before DDL.

**Unit B — organization lifecycle**
- Keep activation closed until there is an approved persisted PROVISIONING state/transition and transactional audit/outbox contract.
- Either implement real lifecycle end-to-end with authorization and PostgreSQL tests or explicitly defer/remove the route from the current API contract.
- Never replace 503 with unconditional success.

**Unit C — mutation transaction matrix**
- Derive full command list from Phase 4/Part 8 binding contracts.
- For each mandatory command, test authorized success, denied/no-write, audit failure rollback, outbox failure rollback, exact replay, concurrent retries and event schema/version/target identity.
- Business state, mandatory audit and outbox must share one PostgreSQL transaction.
- Preserve exact event-count assertions; do not weaken them.

**Unit D — consumer and effect**
- Confirm the owner-approved SNS FIFO/SQS FIFO/DLQ contract and get approval for the concrete business effect, resource identifiers, permissions and queue policy.
- Add strict typed config and fingerprint tests.
- Wire real consumer processor, SQS transport, inbox and approved effect in worker composition; no no-op default.
- Commit inbox marker and effect in one PG transaction, ack only after commit.
- Test duplicate, concurrency, crash before/after commit, ack failure, retry exhaustion, DLQ/redrive and aggregate ordering. A fake publisher is not end-to-end proof.
- Do not create production AWS resources without explicit authorization and infrastructure review.

**Unit E — migration/release**
- Define canonical migration source and immutable version/hash, release procedure and owner.
- Separate migration, API runtime, relay and consumer identities. Verify grants, RLS and definer functions from DB catalogs.
- Runtime does not execute DDL; readiness fails on incomplete schema.
- Test clean setup, supported upgrade and interrupted-migration recovery in disposable PostgreSQL.

**Unit F — testkit**
- Inventory duplicate fixtures; move only genuinely shared helpers into sitolo-testkit.
- Use testkit through dev-dependencies only and enforce no production dependency edges.
- Keep DB security proof on real PostgreSQL; do not fake RLS in memory.
- Update System Map only when real consumers exist. If no shared value exists, approve removal/deferment and amend the contract.

**Unit G — fingerprint and lint/CI policy**
- Enumerate every fingerprint field and test each field plus secret exclusion/reference behavior.
- Pin cargo-deny/cargo-audit versions.
- Add suppression and compiler-flag inventory for allow, expect, cfg_attr, lint overrides, unsafe production code and --cap-lints allow.
- Reconcile SCOPE-EXC-001 with the real mandatory PostgreSQL suites and verify required merge checks.

### Verification commands

Inspect the canonical verifier before running any command. The repository’s intended locked verification includes:

    git status --short --branch
    git rev-parse HEAD
    rustc --version
    cargo --version
    cargo metadata --format-version 1 --locked
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo check --workspace --all-targets --all-features --locked
    cargo test --workspace --all-targets --all-features --locked
    cargo test -p sitolo-persistence --test rls_security_tests --all-features --locked
    cargo build --workspace --release --locked
    ./scripts/ci/check-crate-boundaries.py
    ./scripts/ci/check-icm-workspace
    ./scripts/ci/check-phase2-policy
    ./scripts/ci/check-workflow-policy
    ./scripts/ci/check-security-icm-policy
    ./scripts/ci/check-release-workflow
    python scripts/ci/check-doc-references
    cargo deny check
    cargo audit

Run the canonical verifier in its approved test environment. Never run destructive PG tests while developer environment variables might name a non-disposable DB. Required database suites must use only the approved disposable PostgreSQL workflow. Some scripts may have different exact names/executable conventions at future HEAD: inspect the repository and execute only commands that actually exist. A missing script/tool is a failure to report, not a pass.

After any source change:
- Show the diff and identify the binding contract.
- Run targeted tests, then the complete required gate.
- Push only the intended feature branch and never force-push.
- Verify each workflow run’s head_sha equals the candidate source SHA.
- Record exact workflow results and safe logs.
- Refresh the matrix/status/PR description; label old runs as historical.
- Request an independent Stage 06 audit of the final source commit and separate human review.

### Stop/go acceptance gate

Do not report completion until:
- [ ] No relevant P0/P1 gap remains without authorized written exception.
- [ ] DB target validation blocks unintended target before DDL.
- [ ] All mandatory IAM commands have real PostgreSQL success/denial/rollback/replay/concurrency evidence.
- [ ] Organization lifecycle route is implemented correctly or explicitly deferred.
- [ ] Real consumer/effect uses inbox transaction and ack-after-commit with retry/DLQ/order evidence.
- [ ] Production migration/release source and owner are explicit.
- [ ] Testkit is real and test-only, or an approved deprecation/removal is documented.
- [ ] Fingerprint coverage and lint-suppression/flag inventory pass.
- [ ] CI scope exception and branch protection are verified.
- [ ] Rust, Integration, Security and Policy pass on the same final source SHA.
- [ ] Current-status pointers and PR description match that evidence.
- [ ] Independent Stage 06 audit and human review are complete.

Final response must name files, exact commit, tests/workflow links, unresolved risks and next safe action. Do not present planned work as completed.
