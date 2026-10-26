---
artifact_type: remediation-extension
status: implemented-partial
run_slug: "pr009-part8"
security_relevant: yes
source_before: "49affa5a118339ad6056cb7f4d1520820d14ebc8"
implementation_source: "0385aadd2697b9900e77eecd47ab0f71f886c66a"
verification_head: "0385aadd2697b9900e77eecd47ab0f71f886c66a"
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

No local PostgreSQL was started. The database regression executed in the
approved disposable PostgreSQL CI environment; all required exact-head checks
passed on `0385aadd2697b9900e77eecd47ab0f71f886c66a`:

- Rust canonical verifier, including the session authority PostgreSQL test:
  [40298701887](https://github.com/jelome265/sitolo/actions/runs/40298701887).
- PostgreSQL application/contract integration:
  [40298701841](https://github.com/jelome265/sitolo/actions/runs/40298701841).
- Security: [40298701912](https://github.com/jelome265/sitolo/actions/runs/40298701912).
- Policy: [40298701810](https://github.com/jelome265/sitolo/actions/runs/40298701810).
- CodeQL: [121493288342](https://github.com/jelome265/sitolo/runs/121493288342).

The audit correction in
`phase3-session-expiry-audit-remediation-2026-10-26.md` supersedes this
implementation. The results above belong only to source `0385aadd`, whose
expiry-state change lacked required audit evidence; do not use them to verify
the corrected candidate. Phase 3 remains partial, and independent Stage 06
review and human PR approval are not claimed.
