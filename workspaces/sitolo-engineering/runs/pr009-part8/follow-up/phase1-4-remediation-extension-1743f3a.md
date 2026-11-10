---
artifact_type: remediation-report
status: remediated-reaudit-required
run_slug: "pr009-part8"
security_relevant: yes
source_head: "1743f3a198605e1e50a30776a3366d2e9e9a36f5"
date: "2026-10-25"
---

# PR #49 fingerprint inventory remediation

## Finding and change

Workstream G requires every effective non-secret `AppConfig` field to receive a fingerprint-change regression and requires future configuration fields to trigger review of the fingerprint inventory. Existing per-field tests covered the current fields but did not make additions to `AppConfig` fail until those tests were updated.

Added `assert_fingerprint_field_inventory_is_exhaustive` and a test invoking it. Its exhaustive destructure lists every `AppConfig` field without a remainder pattern. A newly added field causes a compile error until the inventory is reviewed. The existing per-field change test remains the behavior proof for the current canonical writer.

Raw secret values remain outside `AppConfig`; it stores secret references. Existing configuration tests cover the separate secret-value namespace, reference changes and exclusion from canonical fingerprint text. This change does not assert that a secret value can be changed inside `AppConfig`.

## Verification evidence

On source commit `1743f3a198605e1e50a30776a3366d2e9e9a36f5`:

- `cargo fmt --all -- --check` — PASS.
- `cargo test -p sitolo-config --locked` — PASS, 28 tests.
- `cargo clippy -p sitolo-config --all-targets --all-features --locked -- -D warnings` — PASS.
- `git diff --check` — PASS.
- Rust — PASS: https://github.com/jelome265/sitolo/actions/runs/40193380363
- Integration, including disposable PostgreSQL suites — PASS: https://github.com/jelome265/sitolo/actions/runs/40193380330
- Security, including CodeQL and offensive regression — PASS: https://github.com/jelome265/sitolo/actions/runs/40193380381
- Policy — PASS: https://github.com/jelome265/sitolo/actions/runs/40193380574
- CodeQL check — PASS: https://github.com/jelome265/sitolo/runs/121169950121

No local PostgreSQL server was started. The PostgreSQL workflow ran against its authorized disposable CI service.

## Re-audit and remaining disposition

This implementation change requires an independent Stage 06 re-audit before the workflow advances to Stage 08. This report is not that audit. The remaining consumer/effect, canonical migration/release, Phase 4 mutation evidence, Phase 3 lifecycle, branch-protection and human-review blockers remain as recorded in the current status and cross-phase matrix. PR #49 remains open and not ready to merge.
