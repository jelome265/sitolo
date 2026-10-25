# Phase 4 Part 8 PR-009 Continuation Guide

**PR:** #49
**Branch:** `feat/phase4-pr009-audit-outbox`
**Current source basis:** `5a128c4f95db61f7f6cf9fd885d22911b78553c1` (2026-10-25)
**Prior validated head:** `98d49304c8318f708e79019d79449fe33f0069d3` (local non-database gates only)
**Status:** NOT READY; do not merge

The previous 2026-09-24 execution contract is preserved at `docs/phase4_part8_pr009_remediation_execution_contract_2026-09-24_historical.md`. Its head-specific findings and CI statements are historical. This file is the current continuation guide.

## Current implementation state

The API process composes the managed production secret provider, Cognito OIDC issuer validation, PostgreSQL `app_runtime` pool and schema checks, persisted identity/session authority, and the durable tenancy service. `TenancyService::new_postgres` is constructed with `PgTenancyUnitOfWork`, so the application owns the ports and persistence implements them; authorization, the business mutation, the audit row and the outbox row share one transaction.

The reviewed route inventory declares six protected routes. Five are wired to authenticated application commands:

- `POST /v1/organizations` (owner derived from the verified principal)
- `POST /v1/organizations/{organization_id}/branches`
- `POST /v1/organizations/{organization_id}/branches/{branch_id}/activate`
- `POST /v1/organizations/{organization_id}/members/{membership_id}/suspend`
- `POST /v1/organizations/{organization_id}/members/{membership_id}/revoke`

The sixth, `POST /v1/organizations/{organization_id}/activate`, is explicitly closed in `CLOSED_PROTECTED_ROUTES`. It cannot be served: `provision_organization` calls `Organization::activate()` before the insert, so no persisted `PROVISIONING` organization exists for the route to transition. The handler authenticates, then returns `503`; any other undeclared protected route is a startup `InvalidRouteInventory` error.

The worker composes the bounded outbox relay with the SNS FIFO publisher and runs until shutdown. It does **not** yet run the SQS consumer: there is no owner-approved queue/consumer contract, no durable inbox deduplication coupled to an agreed business effect, and no restricted worker migration/release path beyond the existing `app_worker` pool.

## Immediate next work

1. Push the branch and verify exact-head GitHub Actions on `5a128c4` (or its successor). Record each job result; do not substitute older green runs. This requires explicit user authorization to push.
2. Request the independent Stage 06 re-audit against the exact head. Do not self-certify AUD-03, AUD-04, R2, R3 or R4.
3. Resolve the owner-approved SQS queue/consumer contract and exact consumer business effect. Implement durable inbox deduplication in the same transaction as that effect; keep worker readiness closed until complete.
4. Implement the restricted worker database role/pool and documented migration/release path covering identity, tenancy, audit, outbox and inbox schemas.
5. Decide the organization-activation route: either add a real `PROVISIONING` persistence path with its own contract and tests, or remove the route from the inventory. Do not re-open the closed handler without the persistence path.
6. Add live request routes for role/scope and invitation commands only after an owner-approved request contract and authenticated wrapper exist.
7. After the final source change, update the remediation report, verification report and this guide to that SHA. Leave PR #49 unmerged unless the user later asks otherwise.

## Current candidate local evidence

At `5a128c4` with a clean working tree:

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — passed, exit 0.
- `cargo check --workspace --all-targets --all-features --locked` — passed.
- `cargo test --workspace --lib --all-features --locked` — 259 tests, 0 failed.
- `cargo test -p sitolo-api-bin --test axum_boundary` — 3 passed (R2 deny-by-default and exact closure partition).
- `cargo test -p sitolo-application --test lifecycle_event_contract` — 2 passed (R4 catalogue/validator consistency).
- `cargo build --workspace --release --locked` — passed.
- `cargo deny check` — advisories, bans, licenses and sources passed.
- Crate-boundary, architecture, Phase 2, workflow, release-workflow, security-ICM, doc-reference, system-map, ICM-workspace and ICM-contract checks — all passed.

Database-backed suites (`rls_security_tests`, `part8_postgres_integration_tests`, the application PostgreSQL integration targets) were compiled but not run locally; they require the CI PostgreSQL service and three DSNs. `cargo-audit` is not installed locally; the CI `advisories` job is authoritative. See `docs/phase4_part8_pr009_remediation_report.md`, `docs/phase4_part8_audit_outbox_implementation_contract.md`, and the run archive at `workspaces/sitolo-engineering/runs/pr009-part8/follow-up/`.
