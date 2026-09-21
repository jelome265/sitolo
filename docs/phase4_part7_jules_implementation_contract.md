# SITOLO — PHASE 4 PART 7 IMPLEMENTATION CONTRACT

Repository: jelome265/sitolo
Phase: Phase 4 Part 7
Scope: PR-008 — PostgreSQL Row-Level Security integration and real negative security tests
Status: Binding implementation contract
Baseline: main after merged Phase 4 Part 6 / PR-006 (#37)

# 0. EXECUTIVE CONTRACT

Implement Phase 4 Part 7 only.

Part 7 is the transition from the Phase 4 in-memory/reference tenant-scope enforcement model to a real PostgreSQL security boundary.

NON-NEGOTIABLE: Do not simulate PostgreSQL RLS in Rust and call the simulation RLS. Prove the database boundary against a real PostgreSQL instance using a least-privileged runtime role, real policies, real transactions, trusted transaction-local tenant context, and adversarial negative tests.

Preserve PR-007 application-layer scope enforcement. Part 7 adds the database-layer defense.

Do not implement the complete Phase 5 database program. Do not implement generalized Phase 6 authorization. Do not implement PR-009 audit/outbox, PR-010 cache/versioning, PR-011 device binding, PR-012 ownership transfer, or PR-013 support/admin separation except for narrowly required test fixtures.

# 1. REQUIRED DOCUMENT CROSS-CHECK

Before changing source, inspect and reconcile:

- agent.md
- docs/phase4_tenant_organization_branch_iam_implementation.md
- docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md
- docs/phase4_part6_jules_implementation_contract.md
- docs/phase3_identity_sessions_mfa_device_identity_implementation.md
- docs/phase2_config_secrets_logging_errors_telemetry_implementation.md
- docs/phase1_repository_rust_workspace_ci_deep_implementation.md
- docs/system_architecture_design.md
- docs/security_architecture_design.md
- docs/security_implementation_spec.md
- docs/domain_model.md
- docs/database_design.md
- docs/api_contract.md
- docs/auth_authorization_spec.md
- docs/observability_spec.md
- docs/testing_strategy.md
- docs/ADR-001-025.md
- docs/phase5_postgresql_schema_migrations_constraints_rls_implementation.md
- docs/security_test_harness.md
- docs/threat_model.md
- docs/ci_enforcement.md
- all relevant current source, tests, scripts and workflows.

The older docs/enterprise_audit_and_review.md is historical evidence and must not override current main.

# 2. AUTHORITATIVE PHASE SEQUENCE

PR-001 organization + branch domain primitives
PR-002 membership persistence + lifecycle
PR-003 roles + permissions + assignment
PR-004 scope model + effective scope resolver
PR-005 invitations
PR-006 organization/branch APIs
PR-007 repository scope enforcement + PostgreSQL constraint mirror
PR-008 RLS integration + negative tests  <-- THIS CONTRACT
PR-009 audit + transactional outbox
PR-010 cache/versioning
PR-011 device/org/branch binding
PR-012 ownership transfer
PR-013 support/admin separation

Do not turn PR-008 into the entire Phase 5 program.

# 3. CURRENT BASELINE

Current main contains PR-006 and PR-007. Tenancy HTTP DTOs and transport handlers exist. Application/reference scope enforcement exists. PostgreSQL constraint intent exists as a reference/mirror. Real PostgreSQL RLS is not yet a proven production database boundary.

Inspect the actual current tree before modifying anything. If a previous Part 7 attempt exists, audit it instead of blindly layering another implementation.

# 4. HARD RULE — NO RLS SIMULATION

Any implementation that represents RLS only through Rust structs, mutexes, in-memory filters, mocked SQL, fake policy evaluators, test-only predicates, or comments is not RLS evidence.

If an existing Part 7 attempt contains a simulation: preserve useful application scope logic, remove misleading RLS claims, and replace the security proof with real PostgreSQL tests.

A test named RLS is invalid evidence if it never executes PostgreSQL RLS.

# 5. REAL POSTGRESQL TEST ENVIRONMENT

Security claims involving RLS must execute against a real disposable PostgreSQL instance.

The harness must:
- provision PostgreSQL automatically;
- apply repository-controlled migrations/schema;
- create setup/migration authority separately from runtime authority;
- create a restricted runtime role;
- seed at least two organizations;
- seed multiple branches across those organizations;
- seed representative tenant-owned resources;
- enable actual RLS;
- install actual policies;
- execute tests through the runtime role;
- establish tenant context inside real transactions;
- verify positive and negative behavior;
- inspect PostgreSQL catalogs for policy and role evidence;
- clean up deterministically.

Docker/Testcontainers or another repository-approved disposable PostgreSQL mechanism is acceptable.

Do not require a manually configured developer database for CI.
Do not silently skip the suite when PostgreSQL is unavailable.
Database provisioning failure must fail the security gate.

# 6. DATABASE SECURITY MODEL

Required defense-in-depth chain:

untrusted client
-> request bounds
-> authentication
-> membership and authorization
-> AuthorizedScope
-> persistence boundary
-> real PostgreSQL transaction
-> trusted transaction-local tenant context
-> least-privileged runtime role
-> RLS USING/WITH CHECK
-> relational constraints

RLS is defense in depth, not a replacement for application authorization.
Application authorization is not a reason to omit RLS.

# 7. REPRESENTATIVE SCHEMA

Part 7 may use the smallest representative PostgreSQL model necessary to prove the boundary, while remaining compatible with Phase 5.

Minimum conceptual model:
Organization -> Branch -> tenant-owned resource

At least two organizations and at least three branches should exist in fixtures, with branches distributed across organizations.

Do not create a second tenant model.
Do not make the database own business workflow semantics that belong in Rust.

# 8. RLS POLICY REQUIREMENTS

For every protected relation in the Part 7 scope:
- enable row-level security;
- define explicit policies;
- verify policy metadata in PostgreSQL;
- test read isolation;
- test write isolation;
- use USING for existing-row visibility/targeting where applicable;
- use WITH CHECK for inserted/updated row ownership where applicable.

Do not treat a migration file containing policy SQL as proof that the executing database has the policy. Inspect PostgreSQL metadata.

# 9. REQUIRED NEGATIVE TEST MATRIX

1. Tenant A can read A.
2. Tenant A cannot read B.
3. Tenant A can update A.
4. Tenant A cannot update B.
5. Tenant A can delete A.
6. Tenant A cannot delete B.
7. Tenant A cannot insert a B-owned row.
8. Tenant A cannot update an A row so its organization ownership becomes B.
9. A branch-scoped context cannot access an unauthorized branch.
10. A branch in organization A cannot be used as if it belonged to organization B.
11. Missing tenant context fails closed.
12. Invalid/nonexistent tenant context fails closed.
13. Unknown resource identifiers do not bypass ownership.
14. Bulk identifiers, where represented, cannot cross tenant boundaries.

Every denied mutation must verify database state remains unchanged.

# 10. CONNECTION-POOL AND TRANSACTION SAFETY

Tenant context must be transaction-local. Never store the active tenant in process-global mutable state.

Test:
A transaction -> set Tenant A context -> query A -> commit/rollback -> return connection -> new transaction -> set Tenant B context -> query B.

Repeat across bounded iterations and concurrent tasks.

Required proof:
- Tenant A context does not leak to Tenant B;
- Tenant B cannot see A;
- rollback does not leave stale context;
- pooled connections remain safe;
- concurrent transactions remain isolated.

Do not trust connection reuse merely because unit tests pass.

# 11. TRUSTED TENANT CONTEXT

Client-supplied organization_id is data, not authority.

Correct chain:
client request -> authenticated principal -> membership -> authorization -> AuthorizedScope -> trusted database transaction context.

Incorrect chain:
client organization_id -> blind database session setting -> database trust.

The exact context variable and encoding must follow existing repository conventions and the Phase 5 database contract.

# 12. RUNTIME ROLE HARDENING

Runtime database authority must be separate from migration/setup authority.

At minimum verify the runtime role has:
- no SUPERUSER;
- no BYPASSRLS;
- no CREATEROLE;
- no CREATEDB;
- no unnecessary role memberships;
- only required schema/table privileges.

Inspect PostgreSQL catalogs rather than trusting configuration files.

Do not use migration-owner credentials for normal application traffic.

# 13. BYPASSRLS AND OWNERSHIP

Explicitly assert rolbypassrls is false for the runtime role.
Explicitly assert the runtime role is not a superuser.
Inspect ownership of protected relations.

If table ownership would allow an unintended bypass, correct the role/ownership model rather than compensating in application code.

# 14. POLICY CATALOG ASSERTIONS

Verify actual PostgreSQL state:
- protected relation exists;
- row security is enabled;
- expected policy names exist;
- expected commands exist;
- expected roles are targeted;
- required USING expressions exist;
- required WITH CHECK expressions exist.

Catalog state is evidence. Migration source text alone is not.

# 15. PRIVILEGE ASSERTIONS

Verify actual runtime privileges for database, schema and protected tables.

Minimum security property:
runtime role = minimum required privileges + RLS enforcement.

Never solve a privilege failure by granting broader runtime authority.

# 16. APPLICATION + DATABASE COMPOSITION

At least one integration test must prove that application scope and PostgreSQL scope agree.

Example:
Tenant A authorized for resource A -> operation succeeds.
Tenant A targeting resource B -> application scope denies or PostgreSQL independently denies.

The database layer must remain a second boundary if application authorization contains a defect.

# 17. FAIL-CLOSED REQUIREMENTS

These conditions must never widen access:
- missing tenant context;
- invalid tenant context;
- unknown organization;
- unknown branch;
- unauthorized resource identifier;
- transaction-context initialization failure;
- runtime-role misconfiguration;
- RLS/policy setup failure;
- database connection reset;
- stale pooled connection.

Never implement missing context as an unfiltered query.
Never continue with broader access because RLS could not be initialized.

# 18. SIDE-EFFECT ASSERTIONS

A denial must prove more than an error response.

Verify, where applicable:
- no row changed;
- no row was created;
- no row was deleted;
- no unauthorized state transition occurred;
- no partial transaction was committed.

Durable audit/outbox belongs to PR-009 and must not be pulled into this PR solely for testing.

# 19. CONCURRENCY

Run independent Tenant A and Tenant B transactions concurrently.

Verify:
- no cross-tenant visibility;
- no tenant-context contamination;
- no write lands under the wrong tenant;
- no shared mutable tenant state exists.

Keep CI bounded and deterministic. This is focused concurrency verification, not full load testing.

# 20. CONSTRAINT HARDENING

Convert only the PostgreSQL constraints required by the Part 7 fixture and security boundary.

At minimum, enforce organizational consistency for branch ownership using appropriate foreign-key design.

Where the database contract requires composite consistency, use appropriate composite foreign keys rather than relying only on application checks.

Do not duplicate complex workflow rules in SQL.

# 21. SQLX / RUST BOUNDARY

If SQLx is introduced:
- keep SQL in persistence/infrastructure;
- keep domain semantics free of SQLx types;
- use explicit transactions;
- use bounded queries;
- avoid string-built tenant predicates;
- preserve existing repository abstractions where valid;
- do not create a second persistence architecture.

# 22. TEST SUPPORT ARCHITECTURE

Prefer a dedicated test-support boundary containing:
- PostgreSQL lifecycle;
- migration application;
- role setup;
- fixture seeding;
- trusted tenant-context helper;
- transaction helper;
- policy catalog assertions;
- privilege assertions;
- cleanup.

Helpers must not hide the property being tested. Every security test should make principal, tenant, target and expected database state obvious.

# 23. POSITIVE / NEGATIVE SYMMETRY

For every protected operation:
A -> A succeeds.
A -> B fails.

For writes:
A creates A -> succeeds; A creates B -> fails.
A updates A -> succeeds; A updates B -> fails.
A deletes A -> succeeds; A deletes B -> fails.

Negative isolation tests are release evidence, not optional extras.

# 24. CI ENFORCEMENT

CI must:
1. provision PostgreSQL;
2. apply migrations;
3. execute the real RLS suite;
4. execute role/privilege assertions;
5. execute cross-tenant tests;
6. execute cross-branch tests;
7. execute transaction/pool tests;
8. fail on PostgreSQL provisioning failure;
9. fail on migration failure;
10. fail if required security tests are not executed.

Never convert PostgreSQL-unavailable into a green skip.
Never weaken existing CI gates.

# 25. ERROR / SECRET SAFETY

Do not expose through public API responses:
- SQL;
- connection strings;
- passwords;
- raw database errors;
- policy expressions;
- stack traces;
- sensitive tenant data.

Follow existing Phase 2 observability and secret-redaction contracts.
Never commit real credentials or reusable secrets.

# 26. RUNBOOK — LOCAL SECURITY TEST

1. Start disposable PostgreSQL.
2. Apply repository migrations.
3. Create setup/migration authority.
4. Create restricted runtime role.
5. Seed organizations A and B.
6. Seed branches A1, A2 and B1.
7. Seed tenant resources.
8. Verify role attributes.
9. Verify RLS metadata.
10. Run the complete Part 7 security target.
11. Inspect failures.
12. Destroy the environment.

Expected: all positive tests pass, all negative isolation tests pass, role/policy assertions pass, pool-reuse tests pass, concurrency tests pass.

# 27. RUNBOOK — CROSS-TENANT FAILURE

Check in this order:
1. Is the test using real PostgreSQL?
2. Which database role executed it?
3. Is the role a superuser?
4. Is rolbypassrls false?
5. Does the runtime role own the relation?
6. Is RLS enabled?
7. Which policy applies?
8. Does USING enforce existing-row isolation?
9. Does WITH CHECK enforce write ownership?
10. Was context established inside the same transaction?
11. Was context derived from AuthorizedScope?
12. Could a pooled connection contain stale state?
13. Did a query bypass the repository boundary?
14. Did a privileged function/view change the effective security boundary?
15. Did migration application fail to install the policy?
16. Did test setup accidentally use administrative credentials?

Fix the actual security boundary. Do not hide the failure with another application-side filter.

# 28. RUNBOOK — MISSING CONTEXT

Expected: missing context -> deny.

Inspect transaction initialization, context variable, context value, NULL/empty handling, policy expression, role and connection reuse.

Never change the expected behavior to show all tenants.

# 29. RUNBOOK — ROLE PRIVILEGE FAILURE

Inspect PostgreSQL role attributes, ownership, grants, schema privileges and memberships.
Revoke unnecessary authority.
Keep setup/migration authority separate.
Rerun the complete negative suite.

# 30. RUNBOOK — CI FAILURE

Check:
1. PostgreSQL startup;
2. readiness;
3. connection configuration;
4. database creation;
5. migration application;
6. role setup;
7. fixture seeding;
8. security test execution;
9. cleanup;
10. evidence/artifacts.

Do not suppress startup, migration or test errors.

# 31. RUNBOOK — SECURITY INCIDENT

If a future change causes a cross-tenant failure:
1. block release;
2. identify commit;
3. identify relation/policy/role;
4. reproduce with isolated tenants;
5. identify failing layer;
6. add or strengthen regression coverage;
7. restore safe behavior;
8. rerun complete Part 7 suite;
9. record root cause.

Do not classify a tenant-isolation failure as a harmless flaky test without evidence.

# 32. FILES EXPECTED TO CHANGE

Preferred areas:
- migrations/
- crates/sitolo-persistence/
- crates/sitolo-domain/ only if a real boundary correction is required;
- crates/sitolo-api/ only where DB-context composition genuinely requires it;
- apps/api/ only for required wiring;
- tests/ or crate-local tests;
- scripts/ci/;
- .github/workflows/;
- Cargo.toml / Cargo.lock only when required;
- docs/ only for Part 7 evidence/runbooks.

Do not rewrite stable Phase 4 domain primitives.
Do not add frontend work.
Do not implement Phase 7 security-test framework functionality.

# 33. DEPENDENCY DISCIPLINE

Inspect existing dependencies before adding anything.
Prefer existing workspace infrastructure.
Use the smallest appropriate PostgreSQL test mechanism.
Review dependency security/license implications.
Do not add a large framework for a small fixture requirement.

# 34. DEFINITION OF DONE

Database:
- real PostgreSQL;
- clean reproducible migrations;
- actual RLS enabled;
- actual policies installed;
- catalog verification;
- least-privileged runtime role;
- no runtime BYPASSRLS;
- separate setup/migration authority.

Isolation:
- A reads A;
- A cannot read B;
- A updates A;
- A cannot update B;
- A deletes A;
- A cannot delete B;
- A cannot insert B-owned rows;
- A cannot rewrite ownership to B;
- branch isolation proven;
- missing/invalid context fails closed.

Transactions:
- context is transaction-local;
- no pool leakage;
- rollback safe;
- concurrent tenants isolated;
- repeated pool reuse passes.

Composition:
- PR-007 remains intact;
- AuthorizedScope feeds trusted DB context;
- client tenant identifiers are not trusted as authority;
- application and DB security boundaries agree;
- no second tenant model.

CI:
- PostgreSQL provisioned automatically;
- migrations executed;
- RLS tests actually run;
- unavailable database fails the gate;
- no silent skips;
- existing gates remain intact.

Security:
- no secrets;
- no raw DB errors publicly;
- no runtime superuser;
- no runtime BYPASSRLS;
- no fake RLS presented as proof;
- denied writes prove state integrity.

# 35. RELEASE BLOCKERS

Any of the following blocks the PR:

cross-tenant read; cross-tenant write; cross-tenant delete; cross-tenant insert; ownership-changing update bypass; missing-context access; disabled RLS; runtime BYPASSRLS; runtime SUPERUSER; unjustified protected-table ownership; missing policy; missing required WITH CHECK; pool context leakage; concurrent tenant contamination; skipped PostgreSQL suite; PostgreSQL unavailable but CI green; unreproducible migrations; client-controlled DB context; raw DB error disclosure; security tests that only assert HTTP status.

# 36. PR REQUIREMENTS

Open a dedicated PR.

Title:
feat(security): Phase 4 PR-008 PostgreSQL RLS integration and negative tests

Use the repository PR template.

PR description must include:
- scope and baseline;
- documents cross-checked;
- implementation summary;
- RLS policy inventory;
- runtime role model;
- test matrix;
- CI evidence;
- known limitations;
- explicitly deferred work;
- migration/rollback notes;
- release-blocker status.

Do not mix unrelated fixes.

# 37. FINAL EXECUTION GATE

inspect repository -> cross-check Phase 0–5 -> inspect PR-007 -> implement real PostgreSQL -> migrations -> least-privileged role -> real RLS -> transaction-local context -> positive tests -> negative tests -> branch tests -> pool tests -> concurrency -> catalog/privilege assertions -> CI -> exact security suite -> diff/scope review -> PR.

No green result is meaningful if PostgreSQL security tests did not execute.
No RLS claim is meaningful if PostgreSQL did not enforce the policy.
No tenant-isolation claim is meaningful if runtime authority can bypass RLS.

# 38. NON-GOALS

Defer complete Phase 5 schema, durable audit/outbox, IAM cache/versioning, device binding, ownership transfer, support/admin separation, generalized authorization engine, inventory/sales/payment persistence, sync persistence, EIS persistence, billing persistence, full production deployment topology, and the Phase 7 security-test framework.

# 39. CLOSING PRINCIPLE

A buggy application-layer tenant check must not automatically grant cross-tenant access because PostgreSQL independently enforces tenant isolation through real RLS, least-privileged runtime authority, transaction-local context, and executable negative evidence.

That is the Part 7 security boundary.