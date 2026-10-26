# Research Memo — PR-009 Part 8

**Stage:** 02 Research\
**Date:** 2026-10-20\
**Run:** `pr009-part8`\
**Source revision reviewed:** `1223ed3aef3818f56c5b444ee22fff287546349f`\
**PR:** [#49](https://github.com/jelome265/sitolo/pull/49)

## Research decision

Current external evidence is required for the supply-chain findings reported by canonical CI. No jurisdictional, regulatory, or provider-contract claim is needed for this engineering slice. The relay destination is not resolvable from external standards: it is a product/architecture decision that must come from the requester or an approved repository contract.

## Current CI evidence

GitHub checks at the reviewed head include failures in `verify`, `policy`, `non-database contract tests`, `dependency-policy`, and `advisories`. CodeQL, toolchain, offensive-regression, and secret scanning passed.

The failing job logs show:

| Finding | Evidence | Implication |
|---|---|---|
| PostgreSQL application evidence tests ran in the `non-database contract tests` job and all three failed at setup because `ADMIN_DATABASE_URL` / `DATABASE_URL` was unset. | [Workflow job 118803844605](https://github.com/jelome265/sitolo/actions/runs/39473634889/job/118803844605); test setup in `crates/sitolo-application/tests/part8_provision_organization_evidence_tests.rs` | The job must provision and configure real PostgreSQL for these tests, or a dedicated database job must run them. Silently skipping or deleting them would violate the test contract. |
| The workspace gate found the PR-009 run brief in `stages/01-select/output/` and the PR-68 audit artifact in `stages/06-audit/output/`; it reported 11 artifacts instead of 9. | [Workflow job 118803843594](https://github.com/jelome265/sitolo/actions/runs/39473634457/job/118803843594) | Stage run evidence cannot remain committed in reusable factory output shelves under the current checker contract. Preserve evidence in an allowed durable location and remove extra committed shelf artifacts; do not weaken the check. |
| `cargo audit` found `rsa 0.9.10`, advisory `RUSTSEC-2023-0071`; cargo-deny found yanked `yoke-derive 0.8.3`. | [Advisory job 118803841570](https://github.com/jelome265/sitolo/actions/runs/39473634243/job/118803841570); [dependency-policy job 118803841669](https://github.com/jelome265/sitolo/actions/runs/39473634243/job/118803841669) | Do not suppress either finding. Establish whether the RSA lock entry is reachable under supported features and remove/replace it if it is stale or unused. Update the yoke dependency graph to a non-yanked compatible release, then rerun locked build and SCA. |

## External sources and claims

| Source and date/version | Authority | Claim | Engineering implication / uncertainty |
|---|---|---|---|
| [RustSec RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html), last modified 2026-09-14 | Primary Rust ecosystem advisory database | The `rsa` crate's non-constant-time behavior can leak private-key information through observable timing; the advisory lists CVSS 5.9 Medium and no patched versions. | Treat the CI finding as unresolved until the package is removed from the relevant lock graph or a control-based disposition is approved. Local `cargo tree --locked --all-features --target all -i rsa@0.9.10` returned no dependency path, suggesting it is not in the active feature graph; this does not by itself explain or clear the lockfile scan result. Recheck against the exact lockfile and supported feature set. |
| [The Cargo Book — `cargo yank`](https://doc.rust-lang.org/cargo/reference/publishing.html#cargo-yank), checked 2026-10-20 | Primary Cargo documentation | A yank prevents newly generated lockfiles from selecting the yanked release; existing lockfiles can continue using it. | Regenerate/update the dependency resolution to a compatible non-yanked version. Do not edit the lockfile by hand or weaken the gate. |
| `cargo search yoke-derive --limit 3`, run locally 2026-10-20 | Live crates.io index through Cargo | The index currently advertises `yoke-derive 0.8.4`. | This identifies a candidate update only; compatibility and non-yanked status must be confirmed by lockfile update and the canonical SCA gates. |
| [PostgreSQL Windows downloads](https://www.postgresql.org/download/windows/), [EDB binary archive page](https://www.enterprisedb.com/download-postgresql-binaries), and [EDB's checksum response for the 18.6-5 archive](https://github.com/EnterpriseDB/edb-installers/issues/733), checked 2026-10-20 | PostgreSQL project and binary distributor | Windows binaries are available; EDB lists PostgreSQL 18.6 binaries and published a SHA-256 for its 18.6-5 archive. | A local install is technically possible, but the archive is 384,620,317 bytes and transferred at roughly 84 KB/s. The transfer was stopped after about 8.5 MB; no local server is available. Use the existing PostgreSQL 18 CI service after correcting its job wiring. |

## Repository evidence and factual limits

- The current `integration.yml` invokes workspace all-target tests while excluding only `sitolo-persistence`; it does not itself provide the database URLs required by the application evidence tests. This explains why the database-dependent tests execute in a job named `non-database contract tests` and fail at setup.
- The current `rust.yml` verification job starts PostgreSQL 18 and sets `ADMIN_DATABASE_URL` and `RUNTIME_DATABASE_URL`, but not `WORKER_DATABASE_URL`. CI wiring must cover every test identity actually required by the suite.
- The local `cargo tree` query did not find `rsa 0.9.10` even with all workspace features and all targets. This is evidence about the selected dependency graph, not proof that the lockfile is clean or that the advisory may be waived.
- The user selected a PostgreSQL-backed tenancy mutation to satisfy full business + audit + outbox atomicity, and cannot provide a PostgreSQL instance. This memo records the decision; it does not reduce the acceptance criterion.
- The user's phrase "main and sub to be specific" does not match a defined relay destination, transport, topic, endpoint, or configuration contract found in the repository search. It remains ambiguous; no relay implementation should be based on that phrase without clarification.

## Research conclusion

No external standard resolves the worker destination or business-command semantics. Those remain requester/product/architecture decisions. Supply-chain findings have concrete follow-up paths. Local PostgreSQL archive transfer was too slow to finish; CI has a PostgreSQL 18 service but its current test jobs need corrected configuration. Stage 03 may investigate the active code and failure roots; Stage 04 must keep the relay implementation blocked until destination semantics are explicit and must preserve the approved atomic business mutation.

## Human check

Requester decisions recorded: restart at Stage 01; implement PostgreSQL-backed tenancy mutation for full atomicity; requester cannot provide an existing PostgreSQL service. Relay phrase "main and sub" requires clarification before it is treated as an implementation contract.

## Exact-head update — 2026-10-20

The requester later said they cannot view this memo and directed continuation. This records their input; it is not represented as a human review of the memo. They confirmed the names “main and sub” but supplied no transport, destination identifiers/endpoints, payload/version boundary, acknowledgement behavior, or runtime configuration fields. The research decision therefore remains unchanged: no relay can be implemented without inventing product/architecture semantics.

For `dd583b24a4f548437b368d59e91fe4e438dc624a`, GitHub reports integration run 782 and policy run 485 passed. Canonical Rust run 632 passed format, lint, compile, unit/application tests, the 23-test Part 8 persistence suite, the 27-test RLS suite (including the separate invocation), release build, architecture, Phase 2 policy, and dependency/license policy; it then failed at `cargo audit` for `RUSTSEC-2023-0071` in locked `rsa 0.9.10`. Security run 639 passed dependency-policy, CodeQL, offensive-regression, and secrets; advisories failed on the same RSA finding. This confirms the earlier CI wiring fixes and exercises the concurrent provisioning test on PostgreSQL; it does not resolve the relay contract or advisory.

### Dependency remediation check

Reviewed upstream SQLx 0.9.0 release metadata and its `sqlx-mysql` manifest as a possible legitimate removal path for the lockfile-only RSA finding. SQLx 0.9.0 introduces breaking changes and its MySQL crate still declares `rsa 0.10.0-rc.18` as an optional dependency. RustSec's current advisory explicitly lists both `rsa 0.9.10` and `0.10.0-rc.18` as affected, with no patched release. A SQLx version bump therefore does not safely resolve the scanner finding and would add a major dependency migration. No version bump or scanner suppression was applied. Sources: [SQLx 0.9.0 changelog](https://github.com/transact-rs/sqlx/blob/v0.9.0/CHANGELOG.md), [SQLx 0.9.0 MySQL manifest](https://github.com/transact-rs/sqlx/blob/v0.9.0/sqlx-mysql/Cargo.toml), [RustSec RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html).

### Implemented remediation - 2026-10-21

Root cause confirmed: the `sqlx` meta-crate and `sqlx-macros` declare optional `sqlx-mysql` and `sqlx-sqlite` dependencies. Cargo records every optional package in `Cargo.lock`, so `cargo audit` scans `rsa 0.9.10` even though no enabled workspace feature or target compiles the MySQL backend.

Remediation applied in the working tree: the root workspace and both database-using crates now depend on `sqlx-core` and `sqlx-postgres` directly. `sqlx-core` enables the `_rt-tokio` runtime feature that the meta-crate previously forwarded, and `sqlx-postgres` enables `chrono` and `json`. The three `sqlx::FromRow` derives are replaced with explicit `FromRow` implementations so no macro crate is needed. `deny.toml` still has `ignore = []`, `--locked` is still used, and no CI gate was changed.

Local evidence: `Cargo.lock` dropped from 220 to 191 dependencies and no longer contains `rsa`, `sqlx`, `sqlx-mysql`, `sqlx-macros`, `sqlx-macros-core`, or `sqlx-sqlite`. `cargo audit` 0.22.2 loaded 1348 advisories and scanned 191 dependencies with no vulnerabilities. `cargo-deny` 0.20.2 reported advisories ok, bans ok, licenses ok, and sources ok. Exact-head CI at `862775c` confirms `verify`, `advisories`, `policy`, and the PostgreSQL integration job all passed.
