---
artifact_type: remediation-execution-extension
status: partial-remediation-reaudit-required
run_slug: "pr009-part8"
security_relevant: yes
source_head: "15147ef45126917ad4184ca1326282db6631078a"
date: "2026-10-25"
---

# R-04 extension — PostgreSQL-backed audit/outbox failure rollback

## Confirmed gap

`crates/sitolo-application/tests/membership_transaction_integration.rs` previously used `FailingUnitOfWork` and `FailingTransaction` wrappers that returned an error before the PostgreSQL evidence writers ran. Although the business mutation used a real transaction and the tests queried real database state, that setup did not exercise an actual audit or outbox insert failure. The mission requires database-bound rollback evidence from real PostgreSQL.

## Remediation

Removed both fake wrappers. The rollback cases now use the production `PgTenancyUnitOfWork` with the real `app_runtime` role. Each isolated integration schema installs audit and outbox `BEFORE INSERT` triggers. Separate runtime pools select the trigger behavior through their validated `application_name`; the trigger raises a PostgreSQL constraint error on the intended insert. For outbox failure, the real audit insert occurs first and the outbox insert then fails within the same transaction. Existing assertions query persisted business and evidence state after the error.

This is a test-only fault injector in an approved disposable PostgreSQL schema. It creates no production behavior or resource and uses no mocked database or transaction implementation.

## Evidence

Local database-free checks on `15147ef45126917ad4184ca1326282db6631078a`:

- `cargo fmt --all -- --check` — PASS.
- `cargo test -p sitolo-application --test membership_transaction_integration --no-run --locked` — PASS (compilation only; not represented as integration execution).
- `cargo clippy -p sitolo-application --test membership_transaction_integration --all-features --locked -- -D warnings` — PASS.
- `git diff --check` — PASS.
- No local PostgreSQL server was started.

Exact-head CI on `15147ef45126917ad4184ca1326282db6631078a`:

- Rust, including canonical verification — PASS: https://github.com/jelome265/sitolo/actions/runs/40195430303
- Integration, including the real disposable PostgreSQL application/contract suite — PASS: https://github.com/jelome265/sitolo/actions/runs/40195430299
- Security, including CodeQL, advisories, dependency policy, secrets and offensive regression — PASS: https://github.com/jelome265/sitolo/actions/runs/40195430302
- Policy — PASS: https://github.com/jelome265/sitolo/actions/runs/40195430306
- CodeQL check — PASS: https://github.com/jelome265/sitolo/runs/121176342832

## Scope and remaining work

This closes the false failure-injection mechanism for the covered rollback cases. It does not prove every mandatory mutation's complete source trace, authorization/scope behavior, denial/no-write result, replay or concurrency semantics. The mutation matrix remains `PARTIAL`. Independent Stage 06 re-audit is required before Stage 08; this report is not independent review. PR #49 remains open and not ready to merge.
