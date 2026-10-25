# PR #49 mandatory mutation transaction matrix

**Source basis:** `18dce7e59c945cedcccf793b2e69b354b9b86e4d` (2026-10-25). This is a source-inspection and test-index matrix, not a completion claim or Stage 06 audit. Exact-head Rust, Integration, Security and Policy workflows pass on this source. No local PostgreSQL was started for this matrix.

The transaction statements below describe the PostgreSQL path selected when the durable unit of work is configured. `TenancyService::new_postgres` requires a `TenancyEvidenceSink`, and the API bootstrap composes it with `PgTenancyUnitOfWork` (an application-owned port implemented by `sitolo-persistence`) over the `app_runtime` pool. Authorization, the business mutation, the audit row and the outbox row share one PostgreSQL transaction on every row marked as durable.

Two actor sources remain distinguishable:

- **Authenticated wrapper** — the actor is derived from the verified `SecurityContext`, and the persisted permission is checked inside the mutation transaction.
- **`ActorRef` parameter** — the application method accepts a caller-supplied actor. These are not production request paths until a wrapper exists; they remain reference/test entry points.

## Command matrix

| Command / transition | State and tenant predicate | Actor source | Audit/outbox mapping | Atomic boundary and test evidence | Status |
|---|---|---|---|---|---|
| Provision organization | Insert active organization, owner membership and default branch under new organization ID | HTTP route derives `owner_user_id` from the verified principal; application method still takes it as a parameter | `iam.organization.created` | One transaction; `provision_organization_emits_audit_and_outbox_on_success`, audit/outbox rollback, idempotent/divergent and concurrent same-payload tests | Durable-path tests included in passing exact-head CI `18dce7e`; independent Stage 06 trace still required |
| Create branch | Insert branch under organization; persisted `BRANCH_CREATE` authority at organization scope | Authenticated wrapper `create_branch_for_authenticated_user`; live route | `iam.branch.created` | One transaction; `branch_creation_rolls_back_state_and_evidence_on_write_failures` | Durable-path tests included in passing exact-head CI `18dce7e`; independent Stage 06 trace still required |
| Organization activate | `PROVISIONING → ACTIVE` | Application method takes `ActorRef`; no authenticated wrapper | `iam.organization.activated` | Transaction path and lifecycle rollback suite exist | **No active HTTP operation.** Provisioning calls `Organization::activate()` before insert, so no persisted `PROVISIONING` organization exists. The unavailable endpoint was removed from the route inventory and API contract examples; the boundary test asserts the path returns 404. Any future incomplete-provisioning API requires a separately approved workflow contract and persistence tests. |
| Organization suspend | `ACTIVE → SUSPENDED` | `ActorRef` parameter; no live route | `iam.organization.suspended` | Same lifecycle transaction suite | Durable path tested; no request path |
| Organization resume | `SUSPENDED → ACTIVE` | `ActorRef` parameter; no live route | `iam.organization.resumed` | Same lifecycle transaction suite | Durable path tested; no request path |
| Organization begin close | `ACTIVE/SUSPENDED → CLOSING` | `ActorRef` parameter; no live route | `iam.organization.closing_started` | Same lifecycle transaction suite | Durable path tested; no request path |
| Organization close | `CLOSING → CLOSED` | `ActorRef` parameter; no live route | `iam.organization.closed` | Same lifecycle transaction suite | Durable path tested; no request path |
| Branch activate | `PROVISIONING → ACTIVE`, organization predicate | Authenticated wrapper `activate_branch_for_authenticated_user`; live route | `iam.branch.activated` | Same lifecycle transaction suite; authorized-owner and denied-cashier route tests | Durable-path tests included in passing exact-head CI `18dce7e`; independent Stage 06 trace still required |
| Branch suspend | `ACTIVE → SUSPENDED`, organization predicate | `ActorRef` parameter; no live route | `iam.branch.suspended` | Same lifecycle transaction suite | Durable path tested; no request path |
| Branch resume | `SUSPENDED → ACTIVE`, organization predicate | `ActorRef` parameter; no live route | `iam.branch.resumed` | Same lifecycle transaction suite | Durable path tested; no request path |
| Branch begin close | `ACTIVE/SUSPENDED → CLOSING`, organization predicate | `ActorRef` parameter; no live route | `iam.branch.closing_started` | Same lifecycle transaction suite | Durable path tested; no request path |
| Branch close | `CLOSING → CLOSED`, organization predicate | `ActorRef` parameter; no live route | `iam.branch.closed` | Same lifecycle transaction suite | Durable path tested; no request path |
| Direct member invite | Create invited membership under organization | `ActorRef` parameter; no live route | `iam.membership.invited` | Transaction and dual-writer rollback; `member_invitation_and_required_evidence_commit_together`, `member_invitation_rolls_back_state_and_evidence_on_write_failures` | Durable path tested; no request path |
| Mark member pending | `INVITED → PENDING_ACCEPTANCE` | `ActorRef` parameter; no live route | `iam.membership.pending_acceptance` | Transaction, retry/concurrency and audit/outbox rollback; `part8_provision_organization_evidence_tests.rs` pending tests | Durable path tested; no request path; event is in five-event AUD-04 scope |
| Accept/activate membership | `PENDING_ACCEPTANCE → ACTIVE` | `ActorRef` parameter, or token-bound identity for token acceptance | `iam.membership.activated`; token workflow may also emit invitation acceptance | Transaction and rollback; `membership_accept_resume_and_revoke_roll_back_on_audit_and_outbox_failures`; invitation acceptance tests | Durable path tested; no live route |
| Suspend member | `ACTIVE → SUSPENDED` | Authenticated wrapper `suspend_member_for_authenticated_user`; live route | `iam.membership.suspended` | Transaction and rollback; `authenticated_membership_suspend_and_revoke_roll_back_on_evidence_failures` uses test-only failing unit-of-work wrappers | Durable-path tests included in passing exact-head CI `18dce7e`; independent Stage 06 trace still required |
| Resume member | `SUSPENDED → ACTIVE` | `ActorRef` parameter; no live route | `iam.membership.activated` | Transaction and rollback; membership transition suite | Durable path tested; no request path |
| Revoke member | Active/pending member → `REVOKED` | Authenticated wrapper `revoke_member_for_authenticated_user`; live route | `iam.membership.revoked` | Transaction and rollback; membership transition suite; authenticated evidence-failure test | Durable-path tests included in passing exact-head CI `18dce7e`; independent Stage 06 trace still required |
| Expire direct membership | `PENDING_ACCEPTANCE → EXPIRED` | `ActorRef` parameter; no live route | `iam.membership.expired` | Transaction and dual-writer rollback; membership expiry tests | Durable path tested; no request path; event is in five-event AUD-04 scope |
| Issue token invitation | Create `ISSUED` invitation plus linked membership and token hash | `ActorRef` parameter; no live route | `iam.invitation.issued` | Transaction, payload excludes raw token/contact, audit/outbox rollback; token issue tests | Durable path tested; no request path; event is in five-event AUD-04 scope |
| Accept token invitation | `ISSUED → ACCEPTED`, activate linked member and materialize persisted proposed role/scope | Actor derived from locked membership/user | `iam.membership.invitation_accepted`, `iam.membership.activated`, and applicable role/scope events | One transaction and rollback; `invitation_acceptance_activates_member_and_erases_terminal_pii_atomically`, rollback and concurrent-claim tests | Durable path tested; no live route |
| Withdraw token invitation | `ISSUED → WITHDRAWN`; clear contact and token hash | `ActorRef` parameter; no live route | `iam.invitation.withdrawn` | Transaction, duplicate concurrency and audit/outbox rollback; token withdrawal tests | Durable path tested; no request path; event is in five-event AUD-04 scope |
| Expire invitations for one organization | Bounded `ISSUED → EXPIRED`; clear contact/hash; expire linked `INVITED` member only if transition applies | Fixed system actor `system:invitation-expiry-sweeper` | `iam.invitation.expired`, plus `iam.membership.expired` when linked member transitions | Tenant-scoped transaction with row lock/`SKIP LOCKED`; duplicate/concurrency and audit/outbox rollback tests | Durable path tested; no live route; event(s) in five-event AUD-04 scope |
| Unscoped invitation expiry sweep | Cross-tenant expiry entry point | No trusted tenant argument | N/A | Explicitly fails closed when durable evidence is configured; in-memory method remains for reference store | Not production implemented |
| Assign role | Create or reuse effective organization role assignment for membership | Authenticated wrapper `assign_role_for_authenticated_user`; no live route | `iam.role.assigned` | Transaction, semantic idempotency/concurrency and audit/outbox rollback; role assignment tests | Durable path tested; no request path |
| Revoke role | Effective assignment → `REVOKED`, retain history | Authenticated wrapper `revoke_role_for_authenticated_user`; no live route | `iam.role.removed` | Transaction, regrant/duplicate/concurrency and audit/outbox rollback; role revocation tests | Durable path tested; no request path |
| Grant scope | Create or reuse active organization/branch scope grant | Authenticated wrapper `grant_scope_for_authenticated_user`; no live route | `iam.scope.granted` | Transaction, concurrent identical grants and audit/outbox rollback; scope grant tests | Durable path tested; no request path |
| Revoke scope | Active grant → `REVOKED`, tenant predicate | Authenticated wrapper `revoke_scope_grant_for_authenticated_user`; no live route | `iam.scope.revoked` | Transaction and audit/outbox rollback; scope revocation tests | Durable path tested; no request path |

## Live protected-route inventory at this head

The reviewed inventory declares five protected routes, all wired to authenticated application commands. The unavailable organization activation route is absent from both the inventory and router; provisioning atomically persists a complete active organization, owner membership and default branch.

| Route | Handler | Evidence |
|---|---|---|
| `POST /v1/organizations` | `provision_organization` with principal-derived owner | R2 missing-credentials denial; route tests |
| `POST /v1/organizations/{organization_id}/branches` | `create_branch_for_authenticated_user` | R2 denial; authorized-owner and denied-cashier route tests |
| `POST /v1/organizations/{organization_id}/branches/{branch_id}/activate` | `activate_branch_for_authenticated_user` | R2 denial; route tests |
| `POST /v1/organizations/{organization_id}/members/{membership_id}/suspend` | `suspend_member_for_authenticated_user` | R2 denial; four-case evidence-failure test |
| `POST /v1/organizations/{organization_id}/members/{membership_id}/revoke` | `revoke_member_for_authenticated_user` | R2 denial; four-case evidence-failure test |

`apps/api/tests/axum_boundary.rs` proves, from the inventory itself, that every declared protected route rejects a request with no credentials. `organization_activation_route_is_not_exposed` asserts the unavailable organization activation path returns 404 and is absent from `declared_routes()`. An undeclared route is a startup `InvalidRouteInventory` error.

## R4 five-event catalogue evidence

`crates/sitolo-application/tests/lifecycle_event_contract.rs` pins the catalogue to the validator without a database:

- every approved name exists in `IamEventName::ALL`;
- each accepts exactly its v1 payload;
- each rejects a non-v1 version and an unknown field (for example an email address);
- catalogue names are unique.

This proves registry consistency, not that every transition emits the event. Transition emission remains part of the Stage 06 source trace.

## Current-head lifecycle disposition (2026-10-25)

The API contract examples do not declare a separate organization activation endpoint. Phase 4 §7 requires organization, initial owner membership, policy profile and default branch/configuration to be created atomically and says an organization without a valid owner must not be visible as a normal tenant unless an explicit provisioning subsystem is designed. Current `provision_organization` persists the organization as `ACTIVE` in the same transaction as its owner membership and default branch. Accordingly, the dead activation HTTP route was removed rather than changed to unconditional success. This resolves the declared-route gap; it does not claim coverage of all domain lifecycle transitions in the mandatory matrix. `organization_activation_route_is_not_exposed` asserts `404` and absence from the live route inventory. Exact-head Rust, Integration, Security and Policy workflows pass on `18dce7e`.

## Required closure

- Stage 06 must independently inspect this mapping against every public mutation branch, schema and payload validator. It must not use this matrix as proof without source/test verification.
- Organization activation has been removed from the active HTTP surface. The domain lifecycle and other mandatory mutations still require the complete independent source and PostgreSQL matrix review.
- Role/scope, invitation and most lifecycle commands still have no live request route. Adding one requires an owner-approved request contract and an authenticated wrapper.
- Require the durable repository/evidence path in production composition; the API bootstrap currently does so, and the worker composition must be checked separately.
- Preserve fault-injection, race and retry tests on the exact candidate SHA. Existing passing cases cover specific commands and do not prove production routing or absent cases.
- No row above proves AWS delivery, consumer idempotency, managed production secrets, migration deployment, runtime identity or ready status.
- The database-backed suites (`rls_security_tests`, `part8_postgres_integration_tests`, the application PostgreSQL integration targets) were compiled locally but not executed; they require the CI PostgreSQL service and three DSNs.

## Local database-free evidence at this head

`cargo fmt --all -- --check`, workspace Clippy with `-D warnings`, `cargo check --all-targets`, `cargo test --workspace --lib` (259 tests), `cargo build --workspace --release`, `cargo deny check`, and the crate-boundary, architecture, Phase 2, workflow, release-workflow, security-ICM, doc-reference, system-map, ICM-workspace and ICM-contract checks all pass. `cargo-audit` is not installed locally; the CI `advisories` job is authoritative. This is local evidence only; exact-head CI and independent Stage 06 audit remain open.
