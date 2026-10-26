---
artifact_type: remediation-extension
status: implemented-awaiting-exact-head-postgresql-verification
run_slug: "pr009-part8"
security_relevant: yes
source_before: "49affa5a118339ad6056cb7f4d1520820d14ebc8"
date: "2026-10-26"
---

# Phase 3 durable session expiry remediation

## Contract gap

The Phase 3 session state machine defines `IDLE_EXPIRED` and
`ABSOLUTE_EXPIRED` as terminal transitions. Before this change, the PostgreSQL
session authority denied an expired session but left its durable row in
`active`. The runtime therefore had correct request denial but incomplete
persisted lifecycle state.

## Implementation

- Added migration `0005_identity_session_expiry_state.sql`; migration 0001
  remains unchanged. When a matching user presents an active session past its
  idle or absolute deadline, the security-definer function persists the
  corresponding terminal state before returning the ordinary authentication
  denial. Absolute expiry takes precedence when both deadlines have elapsed.
- Added the migration to the ordered SHA-256 manifest and migration README.
- Extended the real PostgreSQL identity-authority integration test with
  separate idle-expired and absolute-expired rows. It asserts authentication
  is denied and the corresponding state is stored. The persistence harness
  applies migration 0005 after migration 0004.

## Local verification

- `cargo fmt --all -- --check` — pass.
- `cargo check -p sitolo-persistence --all-targets --all-features --locked` — pass.
- `cargo test -p sitolo-persistence --test part8_postgres_integration_tests --locked --no-run` — pass (target compiled; not executed locally).
- `cargo clippy -p sitolo-persistence --all-targets --all-features --locked -- -D warnings` — pass.
- `python scripts/ci/check-schema-migrations.py` — pass.
- `git diff --check` — pass.

No local PostgreSQL was started. The database behavior still needs execution in
the approved disposable PostgreSQL CI service on the exact pushed source head.
Phase 3 remains partial: this change does not implement durable session
creation, logout/revocation, refresh rotation, MFA, recovery or device
registration/revocation. Independent Stage 06 review and human PR approval are
not claimed.
