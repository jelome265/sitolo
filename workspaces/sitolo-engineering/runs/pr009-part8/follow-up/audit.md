# Follow-up Audit — PR #49 / Phase 4 Part 8

**Audit date:** 2026-10-22  
**Repository:** jelome265/sitolo  
**PR:** [#49 — Phase 4 Part 8 audit and outbox](https://github.com/jelome265/sitolo/pull/49)  
**Code head reviewed:** `e49de425b74c0095fee0737612ae40dbc50c770a`  
**Delivery/documentation tip at review start:** `d863bd2be3eec72fedb9f48709caf1a07a11f2f6`  
**Base:** `main` at `d2dbcce6606db49b1b87669f364721905c6509f8`  
**Disposition:** **Review-ready, not merge-ready. No merge performed.**

This is a fresh follow-up review of the post-Stage-08 code state. The original Stage 06 and Stage 07 files remain historical run records; this report does not rewrite them. The current findings are linked to source paths and the verification evidence recorded in the [Stage 09 delivery record](../09-delivery-pr009-part8.md).

## 1. Executive assessment

The repository owner approved the three decision records on 2026-10-22: [relay/consumer contract](decisions/relay-delivery-contract-proposal.md), [lifecycle event mappings](decisions/iam-lifecycle-event-mapping-proposal.md), and [identity/production runtime](decisions/identity-production-runtime-proposal.md). This closes the decision-selection part of the previous goal blocker. The remaining findings are engineering, integration-test, deployment and review blockers.

A concrete baseline for the three owner-decision gaps is now documented in the [decision proposal pack](decisions/README.md): SNS FIFO/SQS FIFO fan-out, five distinct v1 lifecycle mappings, and Cognito with ECS/RDS/Secrets Manager. These are proposals only. They do not represent cloud procurement, event-contract approval, or permission to enable production adapters.

The follow-up fixes for database-role identity, the Part 8 RLS verifier, secret/config namespace separation, release-workflow database setup, environment references, and the SQLx advisory path are reported closed. The delivery record reports **10/10 PR checks passing at `d863bd2`** and a successful branch dispatch of the release workflow at `144a284` (run [39703940077](https://github.com/jelome265/sitolo/actions/runs/39703940077)).

Those checks demonstrate the covered test paths only. They do not establish that the live API has authenticated authority, that every IAM mutation is coupled atomically to mandatory audit/outbox rows, or that an event has been delivered through a real destination. The worker executable intentionally refuses to start without an approved relay adapter.

**No unresolved P0 static finding was identified in this scoped follow-up.** The open P1 integration and contract gaps below still block merge and production-readiness claims.

## 2. Scope and review method

Reviewed the PR metadata and recorded check evidence, the binding Part 8 contract, Stage 08/09 records, the API bootstrap/state/tenancy handlers, the tenancy application service, the Part 8 PostgreSQL DDL and evidence tests, the worker relay, the environment configuration reference, and the release workflow. Research sources were consulted for transactional-outbox semantics, PostgreSQL row-level security, and authorization/tenant-context practices.

This is a source-and-evidence review, not a production penetration test, a deployment exercise, or human approval. No new application code was changed by this documentation packet. Exact verification claims below are attributed to the recorded PR CI and artifact-dispatch evidence.

## 3. Prior findings verified closed

| Finding / area | Current result | Basis and boundary |
|---|---|---|
| Original F-01 — runtime/worker database identity | **Closed** | Runtime and worker pools authenticate directly as restricted LOGIN identities, not as an administrator followed by SET ROLE. Runtime identity verification checks both current_user and session_user; negative escalation evidence is recorded in the delivery report. |
| Original F-02 — production Part 8 DDL and RLS verifier mismatch | **Closed** | Part 7 fixture verification is separated from the Part 8 production-DDL verifier. The latter binds to the three provisioning tables, operation-specific SELECT/INSERT policies, and the restricted runtime grant set. Negative PostgreSQL tests reject tenant-blind SELECT and INSERT predicates. |
| Secret/config namespace collision | **Closed** | Development secret values use SITOLO_SECRET__ while typed configuration uses SITOLO__. Unknown ordinary configuration remains fail-closed; bootstrap/config tests cover coexistence and rejection of the former secret key. |
| Release artifact job database/tooling gap | **Closed for the exercised branch dispatch** | Both release jobs have their own PostgreSQL service and the three scoped DSNs. The release verifier installs its required policy tools; the structural workflow checker is enforced by CI. Run 39703940077 passed both jobs and exact-source test execution. |
| Environment examples/reference | **Closed** | .env.example, .env.test.example, and the environment configuration reference document process-environment injection. No automatic dotenv loader is claimed. |
| RustSec dependency finding | **Closed in the recorded evidence** | The SQLx optional MySQL/SQLite lockfile path was removed without adding an advisory suppression; the delivery report records cargo-audit and cargo-deny success. |

The source of truth for detailed job IDs and the full evidence inventory is the [delivery record](../09-delivery-pr009-part8.md). These closure results should not be read as closure of the independent gaps below.

## 4. Open findings

### AUD-01 — Approved relay contract not yet implemented or verified

**Severity:** P1 — release blocker  
**Evidence:** `apps/worker/src/main.rs`; `apps/worker/src/relay.rs`; [binding Part 8 contract](../../../../../docs/phase4_part8_audit_outbox_implementation_contract.md).

The worker library has a bounded claim/publish/transition loop, stale-claim recovery, event/version guards, retry backoff, quarantine, and stable event-ID handoff. The executable exits with status 78 until a destination adapter and configuration are approved. The repository owner approved the SNS FIFO topic → separate SQS FIFO queues for the main and sub-consumers, at-least-once delivery, durable consumer inbox, per-aggregate ordering, acknowledgement semantics, bounded retry/quarantine and ownership requirements. See the [approved relay record](decisions/relay-delivery-contract-proposal.md). The concrete adapter, infrastructure, IAM policies and end-to-end delivery proof are not implemented; the worker correctly remains fail-closed. This is an implementation/release blocker, not a missing decision.

The outbox pattern permits duplicate publication around failure windows; a stable event ID in the producer is not proof that a real consumer deduplicates its business effect. The external transactional-outbox guidance explicitly calls for idempotent consumers and says not to publish an event for a rolled-back transaction. See [AWS transactional outbox guidance](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html).

**Required closure:** an authorized owner approves a versioned destination/consumer contract; a concrete adapter and configuration are implemented against it; and an end-to-end test proves acknowledgement, retry, duplicate, restart/lease-recovery, and consumer-idempotency behavior. No broker or endpoint should be selected by the implementer without that decision.

### AUD-02 — The live HTTP command path has no authenticated principal-derived authority

**Severity:** P1 — security/release blocker  
**Evidence:** `crates/sitolo-api/src/tenancy.rs`; `apps/api/src/serve.rs`; `apps/api/src/state.rs`.

The tenancy API layer defines an `unauthenticated_actor()` placeholder. The organization request DTO still carries an owner user identifier for the in-memory/reference path, with comments saying production must derive that identity from an authenticated principal. The current API state constructs an in-memory `TenancyDatabase`; the durable provisioning path is not wired into this request composition.

This means the current PR does not prove authenticated, server-derived authority at the HTTP boundary. Do not reconnect durable mutations to these handlers while owner, actor, membership, tenant scope, and permissions remain caller-controlled or represented by a placeholder. The absence of durable HTTP wiring is a containment decision, not evidence that API authorization is complete.

**Required closure:** thread a verified principal/security context through the route and application boundary; derive the owner and audit actor server-side; authorize the requested tenant and operation on every request; reject unauthenticated and cross-tenant calls; and test the real HTTP boundary before durable API wiring is enabled. This is consistent with the [OWASP Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html) and [OWASP Multi-Tenant Security guidance](https://cheatsheetseries.owasp.org/cheatsheets/Multi_Tenant_Security_Cheat_Sheet.html).

### AUD-03 — Atomic business-state plus audit/outbox coupling is only demonstrated for organization provisioning

**Severity:** P1 — integrity/audit blocker  
**Evidence:** `crates/sitolo-application/src/tenancy.rs`; `crates/sitolo-application/tests/part8_provision_organization_evidence_tests.rs`.

The durable `provision_organization` branch opens a PostgreSQL transaction, establishes transaction-local organization context, inserts the organization/owner membership/default branch, writes the required audit record and outbox event through the same transaction, and commits only after those writes succeed. The reported PostgreSQL tests cover success, audit/outbox failure rollback, idempotent retry, divergence, and concurrent same-payload provisioning.

That proof is path-specific. Other methods such as branch creation, membership changes, role assignment/revocation, scope changes, and lifecycle transitions call a store mutation and then call `record_evidence`. The latter opens and commits a separate pool transaction for audit plus outbox. The business mutation is therefore not generally atomic with its evidence; an evidence failure can occur after the preceding state change. In addition, the current API composition does not inject the durable evidence sink.

**Required closure:** move every in-scope durable IAM mutation onto an application-owned PostgreSQL transaction that includes the business write and both required evidence writes, or record an explicit contract-approved exclusion. Add fault-injection proof for each operation family that shows a failed audit or outbox insert leaves no business mutation. Do not claim universal Part 8 atomicity based on the organization-provisioning test alone.

### AUD-04 — Approved lifecycle event mappings not yet wired into the contract and code

**Severity:** P1 — approved mappings need implementation and atomicity evidence  
**Evidence:** comments and method bodies in `crates/sitolo-application/src/tenancy.rs`; event catalogue in `crates/sitolo-audit/src/iam_event.rs`; binding Part 8 contract.

The service explicitly documents transitions not wired to evidence because the current `IamEventName` catalogue does not define distinct events for them. The repository owner approved the v1 matrix for member pending acceptance, membership expiry, invitation issuance, invitation withdrawal and per-item invitation expiry during the sweep. The approved names and payloads are recorded in [the event mapping decision](decisions/iam-lifecycle-event-mapping-proposal.md), but they are not yet in the binding contract or source catalogue. Reusing an existing event name for a materially different operation would misstate the audit record; implement the approved vocabulary before wiring these transitions.

**Required closure:** update the binding contract, event enum, catalogue validation and payload schemas to the approved transition-to-event matrix. Add positive, concurrency and failure-path tests. The matrix must also show which transitions are transactionally coupled as required by AUD-03.

### AUD-05 — Production runtime composition is incomplete and correctly fails closed

**Severity:** P1 — production release blocker  
**Evidence:** `apps/api/src/bootstrap.rs`; `apps/api/src/state.rs`; `apps/worker/src/main.rs`; [environment configuration reference](../../../../../docs/environment-configuration-reference.md).

The bootstrap code has no managed production secret provider and rejects production startup rather than falling back to the development provider. The application state currently builds an in-memory tenancy store; it records database runtime intent but does not compose the production PostgreSQL runtime for the API. The worker executable also refuses to start without its approved adapter. The repository owner approved Cognito/OIDC, ECS Fargate services, RDS PostgreSQL and a narrow Secrets Manager provider with workload IAM roles, subject to regional/service compatibility, account setup and operational safeguards. See [identity/runtime proposal](decisions/identity-production-runtime-proposal.md). The environment reference correctly warns that example files are not production deployment manifests.

**Required closure:** approve and implement the managed secret-provider contract, compose real database pools with the restricted runtime and worker LOGIN identities, keep the administrative identity limited to provisioning/migration, and make readiness contingent on the actual required dependencies. Production startup tests must prove that missing provider/credentials, the wrong database identity, or a missing relay adapter fails closed. Do not remove these guards to make a smoke test pass.

## 5. Requirement and control coverage

| Requirement/control | Status | Current evidence or reason |
|---|---|---|
| SC-002 — server-derived tenant/actor authority | **PARTIAL** | PostgreSQL RLS/runtime-role checks are present, but the live HTTP principal and permission boundary is not integrated. |
| SC-006 — transaction integrity/idempotency | **PARTIAL** | The organization-provisioning transaction and its retry/concurrency cases are tested; atomic evidence coupling is not proved for every IAM mutation. |
| SC-009 — durable audit evidence | **PARTIAL** | Audit/outbox rows are coupled on the tested provisioning path; some lifecycle transitions lack event mappings and general transaction coupling. |
| SC-010 — bounded/fail-closed behavior | **PARTIAL** | The worker engine is bounded and the executable fails closed without an adapter; real production delivery/composition is absent. |
| SC-011 — dependency integrity | **PASS in recorded CI evidence** | Advisory and dependency-policy gates are reported green with no new suppression. |
| Environment/configuration boundary | **PASS for documented/tested contract** | Separate secret/config namespaces, strict unknown configuration, examples/reference, and process-environment semantics are tested/documented. |
| Release workflow wiring | **PASS for recorded branch dispatch** | PostgreSQL services, scoped DSNs, verifier tools, exact-source tests, artifact/SBOM/provenance steps are recorded as passing at run 39703940077. |

## 6. Decision and exit criteria

The repository owner's instruction dated 2026-10-22 records approval of the decisions in the three linked decision documents. The decision-selection gate is closed at repository-owner level. AUD-01 through AUD-05 remain open because the approved contracts have not been fully implemented and verified. The relevant code paths must be re-audited at the resulting code head. Human PR review remains mandatory; this approval is not a merge approval and makes no claim of production certification.

PR #49 remains open and unmerged. Before a merge decision, AUD-01 through AUD-05 need implementation closure or a separately recorded human disposition accepted under repository governance. The relevant code paths must then be re-audited at the resulting code head. Human review remains mandatory; repository-owner approval of the architectural choices is not merge approval and does not establish production certification.

The implementation must preserve the binding contract's transaction boundary, least-privilege database identities, strict config/secret separation, RLS defaults, and fail-closed behavior. A green CI result alone does not close the uncovered paths.

## 7. Research references

- [AWS Prescriptive Guidance — Transactional outbox pattern](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html): dual-write atomicity, rollback behavior, duplicates, and idempotent consumers.
- [PostgreSQL 18 — Row security policies](https://www.postgresql.org/docs/current/ddl-rowsecurity.html): row policies supplement grants; table owners and BYPASSRLS/superuser roles have special bypass behavior.
- [PostgreSQL 18 — CREATE FUNCTION](https://www.postgresql.org/docs/current/sql-createfunction.html): SECURITY DEFINER functions require careful ownership and search_path handling.
- [OWASP — Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html): deny by default and validate permissions for each request.
- [OWASP — Multi-Tenant Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Multi_Tenant_Security_Cheat_Sheet.html): server-verified tenant context, RLS-role testing, and cross-tenant negative tests.

These sources ground the review criteria; they do not choose Sitolo's product-specific relay destination or replace the repository's binding contract.
