# Implementation Report — PR-009 Part 8

**Stage:** 05 Implement\
**Date:** 2026-10-20\
**Source before changes:** `1223ed3aef3818f56c5b444ee22fff287546349f`\
**Current tree:** `0b851583030687cc7d663289b995c63991224dc2` plus uncommitted System Map source-revision updates on `feat/phase4-pr009-audit-outbox` (PR #49 open)\
**Goal:** PostgreSQL transaction for organization + owner membership + default branch + audit + outbox; repair database-test wiring; keep PR open.

## Implemented

- Added `crates/sitolo-persistence/src/tenancy_postgres.rs`, which inserts a validated provisioning bundle inside a caller-owned PostgreSQL transaction. It treats a complete same-payload existing bundle as a retry and rejects divergent/pre-existing partial rows.
- Updated `TenancyService::provision_organization`: when the durable evidence sink is configured, it constructs the domain bundle, applies transaction-local organization scope, inserts the three business rows, writes mandatory audit and outbox evidence through the same transaction, and commits once. A matching retry returns without adding another semantic event. The in-memory reference path remains for non-durable tests.
- Added `crates/sitolo-persistence/schema/phase4_part8_tenancy_provisioning.sql` and schema README. The schema creates bounded tenancy tables, forced RLS, scoped runtime read/insert policies, and only the required runtime grants. Administrative deployment is required before runtime startup; the API never creates schema objects.
- API process wiring was initially added, then removed during audit remediation because the live HTTP provisioning route has no authenticated principal and accepts caller-provided `owner_user_id`. The durable application transaction capability remains, but the API must wait for authenticated, server-derived identity before it can safely be wired to PostgreSQL.
- Reworked the application PostgreSQL integration fixture to use the full Part 8 role/RLS schema plus the tenancy schema artifact. Tests now assert a complete bundle and evidence on success; outbox and audit failures each leave no business rows; same-payload retry emits one evidence pair and divergent reuse conflicts.
- Updated Rust and integration GitHub workflows to provide PostgreSQL 18 and required database URLs to suites that include PostgreSQL-backed tests. The runtime and worker pools connect with the PostgreSQL service login and assume their `NOLOGIN` test roles.
- Updated the locked `yoke-derive` resolution from yanked `0.8.3` to non-yanked `0.8.4`.

## Verification

| Command / evidence | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo metadata --format-version 1 --locked` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS; integration targets compile |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --lib --locked` | PASS (workspace library tests) |
| `cargo test -p sitolo-application --test part8_provision_organization_evidence_tests --locked --no-run` | PASS (test target compiles) |
| `cargo test -p sitolo-api-bin --test startup --test tenancy_api` (after audit remediation) | PASS (11 tests) |
| `cargo fmt --all -- --check` (after audit remediation) | PASS |
| `cargo check --workspace --all-targets --locked` (after audit remediation) | PASS |
| Real PostgreSQL application integration suite | NOT RUN locally: no PostgreSQL server or database URLs are available; CI workflow is configured but has not run on these changes |
| Full canonical verification / exact-head CI | NOT RUN on these changes |
| `cargo audit`, `cargo deny check` | NOT RUN locally; `cargo-audit` installation failed while unpacking `aws-lc-sys` because the system C: volume is full. Prior CI reported the inactive locked `rsa 0.9.10` advisory, which remains unresolved pending an applicability/remediation decision |

The unit/build checks do not prove RLS, role, SQL transaction, or rollback behavior. Those claims remain unverified until the real PostgreSQL workflow passes on the exact pushed revision.

## Remaining gates

- The worker relay destination remains undefined. “Main and sub” still does not identify a transport, endpoints/topics/queues, event delivery contract, or runtime fields. No relay target has been invented.
- Destination-dependent relay, idempotent consumer delivery, worker runtime completion, and associated operator evidence remain incomplete.
- Application PostgreSQL CI passed on `1383233`; the persistence/RLS suite and its catalog, privilege, and concurrency assertions still await the canonical verifier.
- RSA advisory disposition and the exact-head canonical verifier remain. The 0b85158 policy run found a stale verified System Map source revision after the `postgres.rs` change; the process/object cards and generated index are corrected locally, and exact-head policy rerun is pending.
- Other tenancy transitions besides organization provisioning still use the in-memory reference store; this implementation does not claim their business mutations are PostgreSQL-backed.

## Human check

The requester directed continuation (“do what you have to to make it all work”) after saying the prior memo was not visible. This authorized implementation to proceed without manual file review; it is not a claim that the requester reviewed this diff. Do not merge the PR.

## Exact-head PostgreSQL follow-up — 2026-10-20

Commit `1383233691ec2aa0b294ea0311d8143836ca0d19` fixes the organization assertion helper to query `organizations.id`. GitHub integration run 779 passed on this exact commit. All four `part8_provision_organization_evidence_tests` passed against PostgreSQL, including successful provisioning, audit-failure rollback, outbox-failure rollback, same-payload retry, and divergent-ID conflict. The separate persistence/RLS suite under the canonical verifier has not yet completed; these application tests do not prove its role/catalog/concurrency checks. The requester has no local PostgreSQL instance, which remains true.

Canonical verifier run 629 then passed all 23 Part 8 PostgreSQL persistence tests but failed the legacy `rls_security_tests`: `PgAuthorityPools` opened the runtime pool as `postgres` and did not assume `app_runtime`, so every test failed its explicit identity precondition before exercising its body. The local remediation adds `SET ROLE app_runtime` in the runtime pool's `after_connect` hook. Local format, all-target check, clippy, and diff checks pass after that change; an exact-head PostgreSQL rerun is pending.

## Exact-head verification follow-up — 2026-10-20

On commit `1aad95cf03e98d63f610c7fe8bc268aae45b2e4c`, policy run 484 passed, application PostgreSQL integration run 781 passed, and canonical Rust run 631 passed the all-target test set, 23 Part 8 PostgreSQL tests, and the legacy RLS suite (27 tests, also passed in its explicit standalone invocation). The verifier then failed at `cargo audit` on `RUSTSEC-2023-0071`; all preceding canonical gates passed, including cargo-deny. The security workflow's dependency-policy, CodeQL, offensive-regression, and secrets jobs passed, while its advisory job failed on the same RSA finding.

A concurrent identical-provisioning test was added after that run to exercise two PostgreSQL transactions racing on the same organization ID. It formats, compiles, and passes clippy locally, but cannot run without a local PostgreSQL service; exact-head CI must execute it after push. The current `1aad95c` pass does not cover this new test.
