# Continuation Guide — PR #49 / Phase 4 Part 8

**Purpose:** hand the next engineer/reviewer a precise starting point for the remaining work without reopening findings already fixed.  
**Current PR:** [#49](https://github.com/jelome265/sitolo/pull/49) — open, unmerged, review required.  
**Packet:** [Audit](audit.md) · [Remediation plan](remediation-plan.md) · [Stage 09 delivery record](../09-delivery-pr009-part8.md).

### Current verification snapshot — 2026-10-22

The current PR head is `bf6b7b04edc5de9fb5fe459d72d0d42d7755814b`. GitHub reports all **10/10** associated check runs completed successfully on this exact SHA: `verify`, `toolchain`, PostgreSQL integration, policy, dependency policy, advisories, secrets, CodeQL workflow, offensive regression, and the separate CodeQL check. The PR remains `OPEN`, with `REVIEW_REQUIRED` and merge state `BLOCKED`.

This updates the earlier verification snapshot below; it does not update the code head reviewed by the static audit (`e49de42`) or close AUD-01 through AUD-05. The checks establish only their exercised paths. The relay contract and adapter, authenticated API authority, full IAM transaction coverage, event decisions, and managed production composition still require their stated closure evidence.

## 1. Read the current state correctly

The repository owner approved the three choices on 2026-10-22. The [decision index](decisions/README.md) records the approval scope. Decision selection is no longer blocked; implementation, integration proof, cloud account provisioning, readiness safeguards and human PR review remain.

The code head reviewed for this packet is `e49de425b74c0095fee0737612ae40dbc50c770a`; the delivery/documentation tip at the start was `d863bd2be3eec72fedb9f48709caf1a07a11f2f6`. The delivery record lists 10/10 passing PR checks at that tip and the separate successful release-workflow dispatch, run [39703940077](https://github.com/jelome265/sitolo/actions/runs/39703940077), on `144a284`.

Treat those as scoped test evidence. Do not infer that the live API has authentication, that all IAM transitions are transactionally audited, or that a real event reached a consumer. The current worker executable refuses to start without an approved relay adapter. Production API bootstrap also fails closed because no managed production secret provider is implemented.

### Implemented and evidenced

- Restricted PostgreSQL LOGIN identities for runtime and worker connections; the administrative identity is not the normal request identity.
- Part 8 provisioning DDL and a dedicated exact-predicate verifier tied to that DDL, with negative PostgreSQL tests for tenant-blind SELECT and INSERT policies.
- Organization provisioning success, audit/outbox failure rollback, retry/divergence, and concurrent same-payload evidence in the PostgreSQL integration suite.
- Bounded transport-neutral worker orchestration for claims, stale-lease recovery, event/version guards, retry, and quarantine.
- Strict split between typed `SITOLO__` configuration and development secret values under `SITOLO_SECRET__`; example files are documentation for process-environment injection, not a dotenv loader.
- Release workflow PostgreSQL services/DSNs and its documented successful branch-dispatch test.

### Not implemented or not generally proven

- Authenticated principal-derived owner/actor/tenant authority in the live HTTP route.
- Production API PostgreSQL composition and a managed production secret provider.
- Atomic state + audit + outbox coupling for every mandatory IAM mutation. The proof is strongest for organization provisioning; many other methods mutate through the current store before the separate evidence transaction.
- Canonical audit-event mapping for every IAM lifecycle transition.
- An approved destination contract, concrete publisher adapter, production worker composition, real acknowledgement semantics, and idempotent consumer evidence.

## 2. Follow this order

1. **Review the proposals and capture explicit owner approval.** Start with [the decision index](decisions/README.md), then review the [relay contract](decisions/relay-delivery-contract-proposal.md), [lifecycle event mapping](decisions/iam-lifecycle-event-mapping-proposal.md), and [identity/runtime/secret provider](decisions/identity-production-runtime-proposal.md). These documents recommend a concrete AWS baseline but do not approve it. Replace any choice that fails the actual hosting, commercial or operational constraints.
2. **Establish trusted authority.** Connect the verified session/principal to each command. Derive the actor and initial owner on the server; tenant IDs supplied by a client select a resource but do not authorize it.
3. **Close the transaction boundary.** Refactor every mandatory state mutation so that business state, the audit row, and outbox intent use one authoritative PostgreSQL transaction. Add rollback tests for audit and outbox failures, not just success tests.
4. **Compose real runtime dependencies.** Implement the approved production secret provider and actual API/worker database pools, preserving separate runtime and worker identities. Keep startup fail-closed until all required dependencies are valid.
5. **Build delivery against the approved contract.** Use the real adapter and prove the acknowledgement point, duplicate behavior, retry/lease recovery, quarantine, and consumer idempotency.
6. **Run exact-source verification and re-audit.** Update evidence against the final code SHA, request human review, and leave merge to the authorized reviewer.

The detailed tasks and acceptance criteria are in the [remediation plan](remediation-plan.md).

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
