# PR #49 authority read path — implementation record

**Date:** 2026-10-23  
**Base:** `f73d6a3e987df309c9e2480ea68f791934e449ed`  
**Scope completed:** PostgreSQL-backed tenant authorization read path and regression coverage. This is a partial implementation of the broader PR #49 remediation brief; it is not a release or exit-criteria claim.

## Changes

- Added a single `REPEATABLE READ` transaction path for persisted organization, membership, branch, effective role, permission, and active scope-grant resolution in `TenancyService`.
- Added `authorize_user_operation`, which accepts the authenticated internal `TenantUserId` and resolves the active organization membership in PostgreSQL instead of trusting a caller-supplied membership identifier.
- When the evidence sink is configured, `effective_scope` and `authorize_operation` now use PostgreSQL as their authority source. Authority lookup and transaction failures deny with `ScopeError::AuthorityUnavailable`.
- Added a PostgreSQL integration test for allowed permission/scope, denied permission, cross-branch denial, and suspended membership.

## Verification

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass.
- `cargo test --workspace --lib --locked` — pass.
- `git diff --check` — pass.
- The new database integration test was compiled by Clippy's all-targets check; it was not run locally. No local PostgreSQL was installed, started, or used.

## Remaining requirements

The production API still authenticates but intentionally returns dependency unavailable for protected routes; it does not invoke these reads or any protected command. The in-memory `TenancyService` constructor and optional evidence sink remain, and mandatory mutation coverage/rollback proof is incomplete. No concrete SNS/SQS adapter, durable consumer inbox, managed production secret provider, PostgreSQL runtime pool composition, real readiness path, independent Stage 06 audit, exact-candidate CI, or human review is claimed by this record.
