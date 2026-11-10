---
artifact_type: remediation-extension
status: exact-head-passed-with-open-blockers
run_slug: "pr009-part8"
security_relevant: yes
source_head: "7f176661b205650871d293b185e27715a0caf00c"
date: "2026-10-25"
---

# Phase 1–4 remediation extension — consumer and producer aggregate ordering

## Finding and change

Review of the approved relay delivery proposal found that consumer inbox deduplication did not enforce aggregate sequence continuity. A later event could be recorded and committed while its predecessor had not committed, allowing consumer effects to overtake a quarantined or delayed event. The producer sequence key and outbox predecessor query also omitted organization scope despite the approved organization-scoped SNS FIFO group identity.

Commit `7f176661b205650871d293b185e27715a0caf00c` now serializes consumer inbox processing with a transaction-scoped advisory lock keyed by consumer, organization, aggregate type and aggregate ID. Existing event identity is checked before sequence continuity, preserving safe duplicate handling. New rows must be the next contiguous sequence; gaps fail with a sanitized error and remain unacknowledged under the worker contract. Producer sequence allocation, uniqueness, and claim predecessor blocking now use organization-scoped aggregate identity. The test schema enforces tenant RLS on sequence rows. The persistence schema README records that the externally controlled production migration must carry the corresponding schema change.

## Evidence

- Test-first regression on `7aa7abbf22ae56648dfb835456dd190ea4072a27` failed in the canonical Rust PostgreSQL suite at the expected assertion: later aggregate sequence must wait for its predecessor ([run 40196875334](https://github.com/jelome265/sitolo/actions/runs/40196875334)).
- On `7f176661b205650871d293b185e27715a0caf00c`, local `cargo fmt --all -- --check`, `cargo test -p sitolo-persistence --test part8_postgres_integration_tests --no-run --locked`, strict targeted Clippy, and `git diff --check` passed. Compile-only does not establish database behavior.
- Exact-head Rust [40197666873](https://github.com/jelome265/sitolo/actions/runs/40197666873), Integration [40197666692](https://github.com/jelome265/sitolo/actions/runs/40197666692), Security [40197666922](https://github.com/jelome265/sitolo/actions/runs/40197666922), and Policy [40197666960](https://github.com/jelome265/sitolo/actions/runs/40197666960) passed on `7f176661b205650871d293b185e27715a0caf00c`. Rust canonical verification includes the required PostgreSQL persistence suites and is the database proof. Integration excludes direct `sitolo-persistence` tests. Security included passing CodeQL, advisory, secret-scanning and offensive PostgreSQL regression jobs.
- Documentation reference inventory passed locally (251 Markdown files, 1127 resolved references, no unresolved/ambiguous/escaping references). ICM security-policy and workspace-contract checks passed locally.
- No local PostgreSQL server was started. No production AWS resources were created.

## Limits and disposition

This change closes only the identified sequence-gap and organization-scope defects in the adapters and test schema. It does not implement or approve the consumer's concrete business effect, production queue/resource identities, operator-authorized replay/skip, or production migration/release. The approved proposal calls for quarantined sequences to block successors; an audited skip path remains unimplemented and requires authorization. The full mutation and Phase 3 evidence gaps, branch-protection evidence, independent Stage 06 re-audit and human review remain open. PR #49 remains open and unmerged; no phase is declared complete.
