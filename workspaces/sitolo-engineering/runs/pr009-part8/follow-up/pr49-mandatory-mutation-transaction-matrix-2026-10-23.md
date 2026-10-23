# PR #49 mandatory mutation transaction matrix

**Source basis:** `619168b3f9d8ef077deeb0bbbc1b8301a9841a8f`. This is a source-inspection and test-index matrix, not a completion claim or Stage 06 audit. Database evidence must be checked against the exact current head.

The transaction statements below describe PostgreSQL paths selected when `TenancyEvidenceSink` is configured. `TenancyService::new_postgres` now explicitly requires that durable sink, and role/grant/permission reads use its persisted authority snapshot. Reference constructors remain available, however, and the API runtime does not compose `new_postgres` with authenticated tenant authorization and protected commands. Most request-driven mutations still accept `ActorRef`; these are material gaps even where a transaction test passes.

| Command / transition | State and tenant predicate | Actor source | Audit/outbox mapping | Atomic boundary and test evidence | Status |
|---|---|---|---|---|---|
| Provision organization | Insert active organization, owner membership and default branch under new organization ID | Owner ID supplied to application method; must be principal-derived in production | `iam.organization.created` | One transaction; `provision_organization_emits_audit_and_outbox_on_success`, audit/outbox rollback, idempotent/divergent and concurrent same-payload tests | PG path tested; production authority incomplete |
| Create branch | Insert branch under organization | `ActorRef` parameter | `iam.branch.created` | One transaction when evidence configured; `branch_creation_rolls_back_state_and_evidence_on_write_failures` | PG path tested; caller actor not trusted at API boundary |
| Organization activate | `PROVISIONING → ACTIVE` | `ActorRef` parameter | `iam.organization.activated` | Transaction path; success and per-transition audit/outbox rollback in `organization_and_branch_lifecycle_changes_commit_with_evidence` and `every_organization_and_branch_transition_rolls_back_on_audit_and_outbox_failures` | PG path tested |
| Organization suspend | `ACTIVE → SUSPENDED` | `ActorRef` parameter | `iam.organization.suspended` | Same lifecycle transaction suite | PG path tested |
| Organization resume | `SUSPENDED → ACTIVE` | `ActorRef` parameter | `iam.organization.resumed` | Same lifecycle transaction suite | PG path tested |
| Organization begin close | `ACTIVE/SUSPENDED → CLOSING` | `ActorRef` parameter | `iam.organization.closing_started` | Same lifecycle transaction suite | PG path tested |
| Organization close | `CLOSING → CLOSED` | `ActorRef` parameter | `iam.organization.closed` | Same lifecycle transaction suite | PG path tested |
| Branch activate | `PROVISIONING → ACTIVE`, organization predicate | `ActorRef` parameter | `iam.branch.activated` | Same lifecycle transaction suite | PG path tested |
| Branch suspend | `ACTIVE → SUSPENDED`, organization predicate | `ActorRef` parameter | `iam.branch.suspended` | Same lifecycle transaction suite | PG path tested |
| Branch resume | `SUSPENDED → ACTIVE`, organization predicate | `ActorRef` parameter | `iam.branch.resumed` | Same lifecycle transaction suite | PG path tested |
| Branch begin close | `ACTIVE/SUSPENDED → CLOSING`, organization predicate | `ActorRef` parameter | `iam.branch.closing_started` | Same lifecycle transaction suite | PG path tested |
| Branch close | `CLOSING → CLOSED`, organization predicate | `ActorRef` parameter | `iam.branch.closed` | Same lifecycle transaction suite | PG path tested |
| Direct member invite | Create invited membership under organization | `ActorRef` parameter | `iam.membership.invited` | Transaction and dual-writer rollback; `member_invitation_and_required_evidence_commit_together`, `member_invitation_rolls_back_state_and_evidence_on_write_failures` | PG path tested |
| Mark member pending | `INVITED → PENDING_ACCEPTANCE` | `ActorRef` parameter | `iam.membership.pending_acceptance` | Transaction, retry/concurrency and audit/outbox rollback; `part8_provision_organization_evidence_tests.rs` pending tests | PG path tested; event is in five-event AUD-04 audit scope |
| Accept/activate membership | `PENDING_ACCEPTANCE → ACTIVE` | `ActorRef` parameter or token-bound identity for token acceptance | `iam.membership.activated`; token workflow may also emit invitation acceptance | Transaction and rollback; `membership_accept_resume_and_revoke_roll_back_on_audit_and_outbox_failures`; invitation acceptance tests | PG path tested |
| Suspend member | `ACTIVE → SUSPENDED` | `ActorRef` parameter | `iam.membership.suspended` | Transaction and rollback; membership transition suite | PG path tested |
| Resume member | `SUSPENDED → ACTIVE` | `ActorRef` parameter | `iam.membership.activated` | Transaction and rollback; membership transition suite | PG path tested |
| Revoke member | Active/pending member → `REVOKED` | `ActorRef` parameter | `iam.membership.revoked` | Transaction and rollback; membership transition suite | PG path tested |
| Expire direct membership | `PENDING_ACCEPTANCE → EXPIRED` | `ActorRef` parameter | `iam.membership.expired` | Transaction and dual-writer rollback; membership expiry tests | PG path tested; event is in five-event AUD-04 scope |
| Issue token invitation | Create `ISSUED` invitation plus linked membership and token hash | `ActorRef` parameter | `iam.invitation.issued` | Transaction, payload excludes raw token/contact, audit/outbox rollback; token issue tests | PG path tested; event is in five-event AUD-04 scope |
| Accept token invitation | `ISSUED → ACCEPTED`, activate linked member and materialize persisted proposed role/scope | Actor derived from locked membership/user | `iam.membership.invitation_accepted`, `iam.membership.activated`, and applicable role/scope events | One transaction and rollback; `invitation_acceptance_activates_member_and_erases_terminal_pii_atomically`, rollback and concurrent-claim tests | PG path tested |
| Withdraw token invitation | `ISSUED → WITHDRAWN`; clear contact and token hash | `ActorRef` parameter | `iam.invitation.withdrawn` | Transaction, duplicate concurrency and audit/outbox rollback; token withdrawal tests | PG path tested; event is in five-event AUD-04 scope |
| Expire invitations for one organization | Bounded `ISSUED → EXPIRED`; clear contact/hash; expire linked `INVITED` member only if transition applies | Fixed system actor `system:invitation-expiry-sweeper` | `iam.invitation.expired`, plus `iam.membership.expired` when linked member transitions | Tenant-scoped transaction with row lock/`SKIP LOCKED`; duplicate/concurrency and audit/outbox rollback tests | PG path tested; event(s) in five-event AUD-04 scope |
| Unscoped invitation expiry sweep | Cross-tenant expiry entry point | No trusted tenant argument | N/A | Explicitly fails closed when durable evidence is configured; in-memory method remains for reference store | Not production implemented |
| Assign role | Create or reuse effective organization role assignment for membership | `ActorRef` parameter | `iam.role.assigned` | Transaction, semantic idempotency/concurrency and audit/outbox rollback; role assignment tests | PG path tested |
| Revoke role | Effective assignment → `REVOKED`, retain history | `ActorRef` parameter | `iam.role.removed` | Transaction, regrant/duplicate/concurrency and audit/outbox rollback; role revocation tests | PG path tested |
| Grant scope | Create or reuse active organization/branch scope grant | `ActorRef` parameter | `iam.scope.granted` | Transaction, concurrent identical grants and audit/outbox rollback; scope grant tests | PG path tested |
| Revoke scope | Active grant → `REVOKED`, tenant predicate | `ActorRef` parameter | `iam.scope.revoked` | Transaction and audit/outbox rollback; scope revocation tests | PG path tested |

## Required closure

- Stage 06 must independently inspect this mapping against every public mutation branch, schema, and payload validator. It must not use this matrix as proof without source/test verification.
- Replace `ActorRef` supplied by callers with an actor derived from verified server context; authorize permission and target resource before each request mutation.
- Require the durable repository/evidence path in production composition; prevent a partially configured service from mutating the reference database.
- Preserve fault-injection, race and retry tests on exact candidate SHA. Existing passing cases cover specific commands and do not prove production routing or absent cases.
- No row above proves AWS delivery, consumer idempotency, managed production secrets, migration deployment, runtime identity or ready status.

## Uncommitted candidate delta — 2026-10-23

The table above remains the historical source matrix at its stated base. The current uncommitted working tree is based on commit `62eeae6ad17a78ccb062a1ab9445f5ac8e92c132`; that SHA does not contain the changes below, and no exact-head CI result applies.

| Command / transition | Current candidate addition | Evidence available now | Evidence still required |
|---|---|---|---|
| Create branch | Live HTTP route calls `create_branch_for_authenticated_user`; transaction checks persisted branch-create authority and derives the actor from authenticated context | Earlier report records route tests and evidence assertions; API/PostgreSQL integration target compiles | Exact-candidate PostgreSQL CI and Stage 06 review |
| Activate branch | Live HTTP route calls `activate_branch_for_authenticated_user`; same-transaction `BRANCH_UPDATE` authorization and `PROVISIONING → ACTIVE` transition | Earlier report records authorized OWNER, denied CASHIER, and anonymous HTTP cases; database target compiles | Exact-candidate PostgreSQL CI and Stage 06 review |
| Suspend member | Live contract-listed HTTP route calls `suspend_member_for_authenticated_user`; persisted permission check and audit actor are in the mutation transaction | Current HTTP PostgreSQL test target compiles; four-case authenticated suspend/revoke audit/outbox failure test compiles | Execute both PostgreSQL targets in CI on exact candidate SHA |
| Revoke member | Live contract-listed HTTP route calls `revoke_member_for_authenticated_user`; persisted permission check and audit actor are in the mutation transaction | Current HTTP PostgreSQL test target compiles; success/denial HTTP regression and authenticated audit/outbox failure test compile | Execute both PostgreSQL targets in CI on exact candidate SHA |
| Assign/revoke role, grant/revoke scope | Authenticated application command wrappers are present and transaction-authorized; no corresponding live HTTP routes are present in this candidate | Application PostgreSQL integration target compiles; local application unit suite passes | Contract-approved request DTOs/identifier semantics, live route integration, PostgreSQL CI |

The latest database-free suites passed locally: events 12, worker 17, application 13, and API binary library 6. This is unit evidence only. The membership application and API integration targets were compiled but not run locally under the no-local-PostgreSQL constraint. Organization and invitation mutation actor provenance remains incomplete for request-driven paths. Exact-head CI, complete Stage 06 audit, human review, and all external runtime/AWS evidence remain open.
