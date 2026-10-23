# Phase 4 Part 8 PR-009 remediation report

**PR:** #49 — Phase 4 Part 8 audit and outbox
**Branch:** `feat/phase4-pr009-audit-outbox`
**Current candidate source:** `43d6e04161964fc771481a325c20b7d8192f9231`
**Base:** `89fa0566ca26dbf4ea557a86e91f4d410572bdfd`
**Date:** 2026-10-23
**Status:** NOT READY — partial candidate pushed; exit criteria remain open

## Implemented in this candidate

- Production/staging API startup now resolves the database credential through the configured managed provider, validates Cognito OIDC settings, probes issuer signing keys, connects to PostgreSQL as the restricted `app_runtime` identity, verifies the role and required schema objects, and wires the persistent identity/session authority and PostgreSQL tenancy evidence service.
- API configuration includes a validated database schema and required production/staging OIDC settings. The runtime pool uses TLS verification, bounded pool settings, explicit connection lifetime/idle limits, and generic fail-closed startup errors.
- The existing branch-create HTTP route authenticates a bearer token and `session-id`, uses exact persisted external-identity mapping and server-side session ownership, then resolves persisted tenant membership, permission, and scope before invoking the durable command. HTTP-level PostgreSQL tests cover denied requests and no-mutation evidence; their database-backed outcome still requires CI on this candidate.
- SQLx was raised to the mandated 0.9 floor. Dynamic test SQL now uses explicit audited SQL-safety assertions for generated, isolated test schemas; production query values remain bound. The workspace selects rustls AWS-LC explicitly.
- Configuration, bootstrap, schema documentation, and runtime requirements were refreshed.

## Evidence for the candidate

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace --all-targets --all-features --locked`: passed before the final provider alignment; the later workspace Clippy pass compiled the provider-aligned all-target source.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: passed.
- `cargo test --workspace --lib --all-features --locked`: passed (all unit-test targets).
- `cargo test -p sitolo-integrations --lib --all-features --locked`: passed after selecting the explicit rustls provider.
- `cargo deny check`: passed after explicitly allowing SQLx 0.9's `CDLA-Permissive-2.0` webpki root bundle license.
- No local PostgreSQL instance was installed, started, or used. These results do not include database-backed integration tests.
- GitHub Actions for code candidate `43d6e04161964fc771481a325c20b7d8192f9231`: integration (`39912162255`) and security (`39912162254`) were still running at report refresh. Rust (`39912162263`) and policy (`39912162258`) failed because the verified system-map card still pinned the previous `postgres.rs` source revision. The card has now been updated to this candidate and the local process verifier passes. Fresh workflow results are required on the documentation/evidence commit before recording branch-head CI as green.

## Open exit criteria

1. **R2 coverage is partial.** Only branch creation is an authenticated, authorized mutation route. Organization creation and other protected routes remain closed; the complete requested HTTP-level authorization matrix is not established for every protected command.
2. **R3 is not complete.** Durable branch authorization and multiple transactional commands exist, but a command-by-command mandatory mutation matrix with actor source, tenant scope, event schema, transaction boundary, and proof is not yet complete. Do not infer all mandatory mutations are covered from the existing test inventory.
3. **R4 requires independent audit.** The approved names and payload validators are present and lifecycle evidence tests exist, but the full transition mapping must be checked against actual domain transitions on the final source by Stage 06. AUD-04 remains open.
4. **AUD-01 is not complete.** The bounded outbox engine and SNS FIFO publisher exist, but `apps/worker` still refuses startup. There is no concrete SQS FIFO consumer, durable inbox transaction coupled to an agreed business effect, or end-to-end delivery proof. No approved consumer effect/queue configuration is present in this repository. Do not claim relay completion.
5. **AUD-05 is partial.** API managed-secret and restricted runtime-pool composition is implemented. A separate restricted worker pool/runtime identity, worker readiness composition, and an authoritative production migration/release path for all required schemas are not implemented. Startup requires schemas to have been provisioned externally.
6. **Required human review and Stage 06 independent re-audit are pending.** No approval or audit result is claimed.

## Continuation

- First record the four GitHub Actions results against the exact candidate SHA above. Do not treat PostgreSQL CI success as proof of unimplemented worker or migration behavior.
- Obtain the owner-approved SQS consumer effect/queue contract and the deployment's migration/release contract before selecting resource identifiers or business effects. Keep worker readiness closed until its restricted pool, publisher, consumer, inbox transaction, and health checks are real.
- Complete the R2 route coverage and the R3 transition matrix with fault-injection evidence. Then run the PostgreSQL integration suite through GitHub Actions only.
- After the implementation source is final, request the independent Stage 06 audit, refresh this report and the continuation guide, and obtain the required human review. Keep PR #49 unmerged.
