# Continuation Guide — PR #49 / Phase 4 Part 8

**Purpose:** hand the next engineer/reviewer a precise starting point for the remaining work without reopening findings already fixed.  
**Current PR:** [#49](https://github.com/jelome265/sitolo/pull/49) — open, unmerged, review required.  
**Packet:** [Audit](audit.md) · [Remediation plan](remediation-plan.md) · [Mutation transaction matrix](pr49-mandatory-mutation-transaction-matrix-2026-10-23.md) · [Stage 09 delivery record](../09-delivery-pr009-part8.md).

### Current verification snapshot — 2026-10-23

The latest verified implementation source SHA is `fae7c4c6973cd55e6a4f95bb8c394301f11aa6d9`. All current checks passed for that exact SHA: [PostgreSQL integration](https://github.com/jelome265/sitolo/actions/runs/39901150740), [toolchain and verification](https://github.com/jelome265/sitolo/actions/runs/39901150869), [policy](https://github.com/jelome265/sitolo/actions/runs/39901150859), [security, dependency policy, advisories, secrets, offensive regression and CodeQL](https://github.com/jelome265/sitolo/actions/runs/39901150895), and [standalone CodeQL](https://github.com/jelome265/sitolo/runs/120244212443). This continuation-guide/remediation-report refresh changes no implementation source. PR #49 remains `OPEN`, `REVIEW_REQUIRED`, and `BLOCKED` from merge.

These checks establish only their exercised paths. They do not constitute the required independent Stage 06 audit, human review, live AWS delivery proof, or closure of the remaining AUD-01 through AUD-05 requirements.

## 1. Read the current state correctly

The repository owner approved the three choices on 2026-10-22. The [decision index](decisions/README.md) records the approval scope. Decision selection is no longer blocked; implementation, integration proof, cloud account provisioning, readiness safeguards and human PR review remain.

The `e49de425b74c0095fee0737612ae40dbc50c770a` audit and `d863bd2be3eec72fedb9f48709caf1a07a11f2f6` delivery tip below are historical snapshots, not current source evidence. Older passing checks and release dispatches do not transfer to the current head.

Treat those as scoped test evidence. The latest implementation source is `a8903d3d2aa799b50b1eeba66fc4036deed10339`; all checks, including PostgreSQL integration, passed for that exact SHA (see the top of `remediation-report.md`). The live API authenticates linked identities and owned sessions, but does not yet authorize tenant permissions or invoke protected commands. The mutation transaction matrix identifies tested PostgreSQL paths and production composition gaps. The SNS FIFO publisher and strict typed relay settings exist, but no worker production wiring or consumer inbox/effects have been composed, so no real event delivery is proven. The worker executable remains fail-closed. Production API bootstrap fails closed because no managed production secret provider or PostgreSQL pool composition is implemented.

### Implemented and evidenced (scoped)

- Restricted PostgreSQL LOGIN identities for runtime and worker connections; the administrative identity is not the normal request identity.
- Part 8 provisioning DDL and a dedicated exact-predicate verifier tied to that DDL, with negative PostgreSQL tests for tenant-blind SELECT and INSERT policies.
- Organization provisioning success, audit/outbox failure rollback, retry/divergence, and concurrent same-payload evidence in the PostgreSQL integration suite.
- PostgreSQL transaction paths and rollback/race/idempotency coverage for organization and branch lifecycles, branch creation, membership transitions, invitation issue/accept/withdraw/expiry, and role/scope assignment changes; see the current integration tests and the remediation report for their exact scope.
- Five owner-approved v1 lifecycle event names and validators. AUD-04 remains open until an independent Stage 06 audit confirms mappings and evidence.
- Live Axum request authentication with exact linked identity and session ownership checks. Protected operations still stop fail-closed because no production membership/permission authority and command composition is installed.
- PostgreSQL authorization reads for internal user ID, organization/membership/branch state, effective roles, permissions and grants within one repeatable-read tenant-scoped transaction. The authenticated-user entry point now refuses the in-memory reference store when no durable pool exists.
- PostgreSQL-backed `member_roles`, `member_grants` and `member_permissions` reads, plus an explicit `TenancyService::new_postgres` constructor. The API runtime still does not compose this constructor with authenticated tenant authorization and protected commands; reference constructors remain for tests/reference use.
- Bounded transport-neutral worker orchestration for claims, stale-lease recovery, event/version guards, retry, and quarantine.
- Concrete SNS FIFO publisher using ECS task-role credentials and strict, typed region/topic/batch/retry settings. It is not wired into the production worker executable; no SNS/SQS delivery or consumer effect is proven.
- Strict split between typed `SITOLO__` configuration and development secret values under `SITOLO_SECRET__`; example files are documentation for process-environment injection, not a dotenv loader.
- Release workflow PostgreSQL services/DSNs and its documented successful branch-dispatch test.

### Not implemented or not generally proven

- End-to-end live HTTP tenant authorization and protected command invocation, including cross-tenant/suspended membership denial and spoofed actor/owner regression with no mutation.
- Production API PostgreSQL composition and a managed production secret provider.
- Production API composition of `TenancyService::new_postgres`, verified actor provenance for mutations, and an independently reviewed transition matrix. The explicit durable constructor exists, but the live runtime does not use it yet.
- Per-consumer durable SQS inbox/effect transaction, production worker composition of the SNS publisher and database claim path, and actual delivery/acknowledgement/retry/quarantine evidence. The publisher and typed configuration exist, but are not wired to the worker executable.
- Real readiness probes for managed secret, database identities/schema, identity provider, tenancy authority and relay dependencies.
- Independent Stage 06 audit of the final source head and required human PR review.

## 2. Follow this order

1. **Complete the live authority path.** Compose production OIDC/session and PostgreSQL tenancy authority. Derive owner and actor from the verified principal; request tenant IDs remain selectors. Keep writes disabled until permission and resource scope are resolved before the command.
2. **Require one production repository path.** Separate reference/test construction from production construction. Complete the transition and actor-provenance matrix, and ensure each required mutation uses one PostgreSQL transaction for state, audit and outbox with rollback/race/idempotency evidence.
3. **Finish AUD-04 with independent audit.** Trace all five owner-approved payloads from state transition through construction, persistence and relay acceptance. Do not self-certify Stage 06.
4. **Finish delivery against the approved contract.** Wire the existing SNS FIFO adapter and strict region/topic settings into the worker after its restricted PostgreSQL runtime is available; then implement the consumer inbox/effect transaction and SQS delete-after-commit behavior. Prove service behavior in a contract-faithful environment; do not invent resource identifiers.
5. **Compose production runtime and readiness.** Add managed secrets and restricted API/worker database pools, include required schemas in controlled migrations, and fail readiness on missing identity, DB, tenancy or relay dependencies.
6. **Run exact-source verification, re-audit and human review.** Keep PR #49 open and unmerged until all required checks and reviews are complete.

The detailed tasks and acceptance criteria are in the [remediation plan](remediation-plan.md). The [mutation transaction matrix](pr49-mandatory-mutation-transaction-matrix-2026-10-23.md) maps current mutation paths and tests but explicitly records the remaining production actor/composition and independent-audit gaps.

## 3. Environment and test safety

The application reads its configuration from the process environment. Neither `.env.example` nor `.env.test.example` is automatically loaded. The canonical namespace and development-only secret provider are described in [the environment configuration reference](../../../../../docs/environment-configuration-reference.md).

Local PostgreSQL testing is deferred to the next phase. Do not install, start, or run a local database for this phase. GitHub Actions provides the approved PostgreSQL 18 service and three scoped identities (`ADMIN_DATABASE_URL`, `RUNTIME_DATABASE_URL`, and `WORKER_DATABASE_URL`). Use its exact-head integration result for database evidence; do not count compilation or a skipped database test as a pass. Do not place real credentials in any example file or commit.

Database-free targeted verification:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --all-features --locked
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo test -p sitolo-config -p sitolo-security --all-targets --all-features --locked
    cargo test -p sitolo-api-bin --test startup_env --all-features --locked
    ./scripts/ci/check-release-workflow
    ./scripts/ci/check-icm-workspace
    python scripts/ci/check-doc-references

For PostgreSQL-backed tests, push the candidate source SHA and use the repository's PostgreSQL-enabled [integration workflow](https://github.com/jelome265/sitolo/actions/workflows/integration.yml). Confirm its `headSha` exactly matches the candidate before recording a pass. Local execution of these suites belongs to the next phase.

Do not report database tests as passed if they were only compiled or skipped. Use exact-head GitHub Actions logs for PostgreSQL evidence when the local host has no suitable disposable server.

## 4. Reviewer checklist

- [ ] Current code SHA matches the code SHA named in the audit.
- [ ] Runtime/worker DSNs authenticate as the restricted roles; no SET ROLE shortcut or admin runtime fallback has returned.
- [ ] RLS verifier checks production DDL, and negative tests reject tenant-blind policies.
- [ ] Caller-supplied owner/tenant fields cannot become authority; real request tests deny unauthorized and cross-tenant actions.
- [ ] Business mutation, mandatory audit, and outbox writes roll back together for every in-scope operation.
- [ ] No unexplained transition is missing from the audit event catalogue.
- [ ] A concrete relay adapter conforms to the approved transport/consumer contract; no fake publisher is presented as end-to-end proof.
- [ ] Production secrets and runtime dependencies are real and fail closed when absent.
- [ ] CI and release-artifact evidence belongs to the exact candidate source SHA.
- [ ] The PR's remaining-risk statement and human review decision are explicit.

## 5. Research used

- [Transactional outbox pattern — AWS Prescriptive Guidance](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html): atomic mutation/outbox intent and duplicate-safe consumers.
- [PostgreSQL 18 row-security policies](https://www.postgresql.org/docs/current/ddl-rowsecurity.html): grants and RLS are complementary; owner and bypass roles need special care.
- [OWASP Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html): deny-by-default and authorization on every request.
- [OWASP Multi-Tenant Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Multi_Tenant_Security_Cheat_Sheet.html): establish tenant context from server-verified identity and test cross-tenant isolation.

These are engineering references, not authority to choose Sitolo-specific destinations or policy. The [binding Part 8 contract](../../../../../docs/phase4_part8_audit_outbox_implementation_contract.md) remains the project requirement source.
