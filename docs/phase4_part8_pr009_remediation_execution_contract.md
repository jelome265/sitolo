# Phase 4 Part 8 PR-009 Continuation Guide

**PR:** #49
**Branch:** `feat/phase4-pr009-audit-outbox`
**Implementation source:** `43d6e04161964fc771481a325c20b7d8192f9231`
**Exact-source validation head:** `5c7e49ae2f8fe7b550fc3bb9ce863771f29a7e26`
**Status:** NOT READY; do not merge

The previous 2026-09-24 execution contract is preserved at `docs/phase4_part8_pr009_remediation_execution_contract_2026-09-24_historical.md`. Its head-specific findings and CI statements are historical. This file is the current continuation guide.

## Current implementation state

The API process now composes the managed production secret provider, Cognito OIDC issuer validation, PostgreSQL `app_runtime` pool and schema checks, persisted identity/session authority, and a durable tenancy evidence service. The live branch-create route authenticates bearer plus `session-id` and uses persisted membership, permission, and scope before its command. Other protected routes remain closed.

The existing bounded outbox relay engine and SNS FIFO publisher are present, but `apps/worker` still refuses startup. There is no SQS consumer, no inbox transaction coupled to an agreed business effect, no complete worker runtime/pool, and no authoritative migration/release path. These are implementation gaps, not CI-only gaps.

## Immediate next work

1. Verify GitHub Actions run results for source SHA `43d6e04161964fc771481a325c20b7d8192f9231`; record each result and do not substitute older green runs.
2. Resolve the owner-approved SQS queue/consumer contract and exact consumer business effect. Implement durable inbox deduplication in the same transaction as that effect; keep worker readiness closed until complete.
3. Implement the restricted worker database role/pool and documented migration/release path covering identity, tenancy, audit, outbox, and inbox schemas.
4. Expand protected HTTP operations to the requested full route matrix and finish the R3 command/transition coverage matrix, including audit/outbox failure rollback for every mandatory mutation.
5. Verify all five lifecycle event mappings and payload construction, persistence, and relay acceptance. Run the PostgreSQL integration suite through GitHub Actions only; do not start local PostgreSQL.
6. After final source changes, update the report and this guide to the final SHA, request an independent Stage 06 audit, obtain the required human review, and leave PR #49 unmerged unless the user later asks otherwise.

## Current candidate local evidence

`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --lib --all-features --locked`, `cargo test -p sitolo-integrations --lib --all-features --locked`, and `cargo deny check` passed. Database-backed tests were not run locally. Exact-source GitHub Actions, including PostgreSQL integration, passed on `5c7e49ae2f8fe7b550fc3bb9ce863771f29a7e26`; see `docs/phase4_part8_pr009_remediation_report.md` for run IDs and the precise evidence boundary.
