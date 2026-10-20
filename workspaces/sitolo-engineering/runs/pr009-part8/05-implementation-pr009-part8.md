# Implementation Report — PR-009 Part 8

**Stage:** 05 Implement\
**Date:** 2026-10-20\
**Source before changes:** `1223ed3aef3818f56c5b444ee22fff287546349f`\
**Current tree:** local uncommitted changes on `feat/phase4-pr009-audit-outbox`\
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
- Real PostgreSQL CI has not yet run. The new schema/test code needs its first PostgreSQL execution and any SQL/RLS corrections.
- Canonical ICM check, main-branch output-shelf cleanup, RSA advisory disposition, scanner/toolchain checks, Stage 06 findings, and exact-head required checks remain.
- Other tenancy transitions besides organization provisioning still use the in-memory reference store; this implementation does not claim their business mutations are PostgreSQL-backed.

## Human check

The requester directed continuation (“do what you have to to make it all work”) after saying the prior memo was not visible. This authorized implementation to proceed without manual file review; it is not a claim that the requester reviewed this diff. Do not merge the PR.
