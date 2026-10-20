# Implementation Plan — PR-009 Part 8

**Stage:** 04 Plan\
**Date:** 2026-10-20\
**Inputs:** `01-select-pr009-part8-completion-brief.md`, `02-research-pr009-part8.md`, `03-investigation-pr009-part8.md`\
**Target:** PR #49, branch `feat/phase4-pr009-audit-outbox`; leave open and unmerged.

## Objective

Continue PR-009 Part 8 toward merge readiness while preserving every P0/P1 acceptance item in the run brief. Implement the selected organization provisioning command so the organization, owner membership, default branch, mandatory audit row, and outbox event are committed or rolled back together in one PostgreSQL transaction. Resolve current CI wiring and supply-chain failures without weakening gates. Do not infer a worker relay destination from “main and sub”; record the exact external contract still needed and do all independent work.

## Authority and assumptions

- Binding: `docs/phase4_part8_audit_outbox_implementation_contract.md`; execution order/closure: `docs/phase4_part8_pr009_remediation_execution_contract.md`.
- Security controls carried: SC-002, SC-006, SC-009, SC-010.
- Requester approved PostgreSQL-backed atomic tenancy provisioning and cannot provide PostgreSQL. Canonical PostgreSQL CI must supply real-engine evidence; no mock substitutes.
- Requester asked us to proceed without requiring them to open the Stage 02 memo. This is continuation authorization, not an assertion that they reviewed its contents.
- “Main and sub” is not a complete relay contract. No broker, target, delivery protocol, schema, or credential/config names may be invented.
- No PR merge. No test removal, scanner suppression, advisory suppression without approved exception, or CI policy weakening.

## Bounded contexts and owners

| Area | Owner | Planned impact |
|---|---|---|
| Domain tenancy | `sitolo-domain` | Reuse organization/branch/member constructors and validation; no SQL dependencies |
| Application orchestration | `sitolo-application` | Own one PG transaction for selected provisioning + audit + outbox |
| Persistence | `sitolo-persistence` | Add narrow provisioning insert/read/idempotency operations that accept caller transaction; add durable schema/deployment artifact consistent with repo database contract |
| API bootstrap | `apps/api`, `sitolo-api` | Wire PostgreSQL-backed tenancy path explicitly; never silently fall back to memory when durable mode is selected |
| Worker | `apps/worker` | Leave external publish adapter unimplemented until destination contract is supplied; continue only destination-independent evidence/report/docs |
| CI / evidence | `.github/workflows/`, stage outputs | Run DB-dependent suites against PostgreSQL with distinct admin/runtime/worker identities; repair policy/supply-chain findings without suppressions |

## Transaction contract

For the selected command:

1. Validate all bounded IDs/names and construct the domain bundle before opening a transaction.
2. Begin a transaction using the API/runtime identity.
3. For a create operation, use the reviewed bootstrap authorization boundary; do not apply an ordinary tenant context that presupposes an organization row already exists. Any `SECURITY DEFINER` helper must be narrowly scoped, pinned to catalog plus exact schema, owned by `app_outbox_owner`, have PUBLIC execute revoked, and grant only the required runtime role. If direct runtime inserts are safe under the schema's bootstrap RLS policy, prefer them and prove that policy; do not grant broad schema/table ownership.
4. Insert organization, owner membership, and default branch under PK/FK/unique constraints, returning typed rows. Exact same identifiers and semantics may return the existing complete bundle; partial or divergent pre-existing rows conflict and must not be treated as success.
5. Build mandatory audit and outbox records from the validated command/result; write both through the existing transaction-taking ports.
6. Commit once. Any business, audit, outbox, serialization, role, timeout, or commit error returns failure; transaction drop/rollback leaves zero rows from this command.

The existing in-memory store remains an explicitly local/test reference. The API's durable path must be explicit and startup must fail closed when durable configuration is missing. Do not claim other tenancy mutations have gained PostgreSQL atomicity unless they are actually moved in this run.

## Persistence / schema

- Define the minimum durable organization, membership, branch schema needed for the selected provisioning bundle, with explicit bounds, foreign keys, uniqueness/idempotency semantics, and compatible tenant predicates.
- Put schema in the repository's deployable database artifact, not only in a test fixture. Keep test schemas isolated per run.
- Reuse the audit/outbox table contract and transaction-aware repositories.
- Add no unrelated tenancy CRUD, invitation flow, role-assignment persistence, generic migration framework, event broker, or schema expansion beyond the selected bundle.
- If project evidence reveals an existing authoritative migration owner before editing, use it instead of introducing a parallel schema path.

## Failure, replay, and concurrency behavior

- Audit failure after business inserts: all business inserts roll back; no outbox row.
- Outbox failure after business+audit inserts: all three categories roll back.
- Business constraint or serialization failure: no evidence commit.
- Same-command retry: one complete bundle and one event pair only; same ID/different payload returns conflict.
- Concurrent same-ID requests: database uniqueness/transaction handling determines a single durable bundle; losing transaction returns the documented retry/conflict result without duplicate evidence.
- Bound pool/statement/lock/transaction lifetime according to current runtime config; no unbounded operation or best-effort evidence fallback.

## Test and evidence contract

Real PostgreSQL tests must prove:

- successful command has exactly one organization, owner membership, default branch, audit row, and outbox row;
- injected mandatory audit failure leaves none of those rows;
- injected outbox failure leaves none of those rows, including already-staged audit;
- same-payload retry is idempotent and divergent ID reuse conflicts;
- cross-organization membership/branch bindings and unauthorized role/path are denied;
- required runtime identity executes under the intended role and cannot mutate audit/outbox outside approved repository functions;
- integration tests use per-test schemas and real transactions for `SET LOCAL`.

CI must expose explicit `ADMIN_DATABASE_URL`, `RUNTIME_DATABASE_URL`, and `WORKER_DATABASE_URL` identities, assert role identity, and run all DB-dependent app + persistence tests on the PostgreSQL service. Non-DB jobs must select only genuinely DB-independent tests; DB assertions are not deleted or silently skipped.

Also repair the Stage 02 CI blockers: workspace policy artifact layout, toolchain availability, yanked transitive `yoke-derive` resolution via a legitimate compatible lockfile update, and `rsa` advisory after confirming its dependency path/applicability. Do not state “fixed” until exact-head CI passes.

## Worker and relay boundary

- Do not add publication to an assumed “main/sub” topic, queue, endpoint, or service.
- Search repository/config history for an already-approved contract. If none exists, document the exact missing fields and keep the relay's destination adapter, end-to-end duplicate/idempotent-consumer proof, and destination-specific runbook explicitly blocked.
- Destination-independent worker runtime, bounded claim loop, retry/quarantine, metrics, event registry/version validation, and operator procedures may proceed only if they do not imply delivery semantics or a destination. Must not claim full P0/P1 completion while the required runtime/destination proof is absent.

## Security mapping

| Control | Threat / requirement | Change | Negative test / evidence | Gate | Residual |
|---|---|---|---|---|---|
| SC-002 | Forged organization selectors or cross-tenant relation bindings | Server-side validation, FK/unique constraints, narrow bootstrap access, no client authority from IDs | Foreign organization branch/member binding rejected; wrong DB role denied | Real PG integration + canonical CI | Unauthenticated API placeholder in current source remains a separate Phase 3 integration limitation; do not portray it as solved |
| SC-006 | Partial provisioning, replay, or evidence loss | App-owned single transaction; database idempotency and constraints | Audit/outbox failure rollback; duplicate retry and conflict tests | Real PG integration + workspace tests | Only selected organization provisioning command is converted in this run |
| SC-009 | Mutation without authoritative evidence | Required audit and outbox are in same transaction | Both persistence-failure injection cases leave zero business/audit/outbox rows | Real PG integration + security audit | External delivery remains at-least-once and needs the approved consumer contract |
| SC-010 | Unbounded work or fail-open dependency behavior | Bounded SQL/runtime controls; durable-mode startup fails closed | Missing durable config and invalid limits rejected; no DB identity fallback | CI, negative tests, canonical verifier | Relay config remains unprovided |

## Rollback / disable path

No data-destructive migration. Preserve current test/reference constructor for local tests. The production API path is enabled only when durable database configuration is explicitly selected; missing credentials/config fail startup or the command closed. Roll back code and schema additively without dropping existing audit/outbox evidence. Do not silently route production mutations to memory.

## Unresolved decisions

1. Relay transport, destination names/endpoints, payload contract, acknowledgement behavior, and runtime configuration remain unspecified; “main and sub” does not resolve them.
2. No requester-managed PostgreSQL exists. Real integration proof depends on repairing CI PostgreSQL setup and getting checks to execute on this exact head.
3. No production tenancy migration directory was found. Use any authoritative schema path discovered during implementation; otherwise document a narrowly scoped deployable SQL artifact and its invocation/ownership, with the project database contract as authority.

## Sequence and exit criteria

1. Implement and test PostgreSQL-backed organization provisioning plus full atomic business/audit/outbox rollback.
2. Wire API durable provisioning explicitly; verify fail-closed startup/config behavior.
3. Correct CI PostgreSQL identities and test partitioning; correct policy output layout and legitimate supply-chain/toolchain blockers.
4. Complete destination-independent worker/P1 gaps where contract permits; leave destination-specific pieces pending.
5. Run targeted checks, Stage 06 independent audit, canonical verification and exact-head GitHub checks; update evidence and PR description to remove any claim not proven.
6. Leave PR open. Merge readiness may only be reported if every P0/P1 gate is met; otherwise enumerate exact unresolved inputs and evidence.

## Plan disposition

This plan follows the requester's instruction to proceed autonomously. It does not treat a CI result as human review, and it does not authorize inventing a relay destination. Continue to Stage 05 for the independently authorized atomic provisioning work; keep external-relay-dependent changes out until its contract is defined.

## Implementation follow-up

Stage 05 implemented the transaction, RLS-scoped provisioning schema, API
runtime wiring, and PostgreSQL failure/retry tests. See
`05-implementation-pr009-part8.md` for changed files and
verification gaps. Relay semantics and exact-head evidence remain open.

## Audit-driven implementation deviation

The Stage 05 API database wiring was removed during Stage 07 after Stage 06
confirmed the current HTTP route has no authenticated principal and accepts a
request-supplied owner identity. The application-level PostgreSQL transaction
capability remains, but production API wiring is deferred until authentication
derives owner, actor, and tenant authority server-side. The re-audit leaves
this integration requirement open and does not clear Stage 08.

## Destination-independent worker work — 2026-10-20

The requester’s “main and sub” response still provides no transport or delivery contract. Per the planned destination-independent slice, add a transport-neutral worker processing engine with bounded claim/recovery, event name/version/envelope validation, typed retry/quarantine handling, and stable event-ID handoff to a future publisher adapter. Keep the executable fail-closed until an approved adapter/configuration is available. This does not satisfy actual relay delivery, consumer deduplication, or production runtime composition; those remain blocked on the exact destination contract.

## Advisory remediation decision - 2026-10-21

The lockfile-only RSA advisory is removed by dependency construction, not by waiver:

1. Replace the `sqlx` meta-crate and its `macros` feature with direct `sqlx-core` and `sqlx-postgres` dependencies. This is within the approved stack because both are published SQLx 0.8 backend crates, and the workspace only uses PostgreSQL.
2. Keep the runtime behavior identical: `sqlx-core` carries the `_rt-tokio` feature the meta-crate forwarded; `sqlx-postgres` carries `chrono` and `json`.
3. Replace the three `sqlx::FromRow` derives with explicit `FromRow` implementations, because retaining `sqlx-macros` would keep the optional MySQL package in `Cargo.lock`.
4. Do not add an advisory ignore, do not remove `--locked`, and do not change the CI gate.

Exit criteria: locked workspace check, clippy, all non-PostgreSQL tests, release build, `cargo-deny`, and `cargo audit` pass locally; exact-head CI `verify` and `advisories` pass after push. PostgreSQL behavior remains CI-evidenced only.
