# SITOLO — PHASE 4 PART 7 / PR-008 REMEDIATION EXECUTION CONTRACT

## Status

**Binding remediation contract for PR #39.**

Repository: `jelome265/sitolo`
Phase: **Phase 4 Part 7 only**
PR: **PR-008**
Target PR: **#39**

This document supplements `docs/phase4_part7_jules_implementation_contract.md` with concrete change-to-test mapping and mandatory evidence requirements.

---

# 1. ABSOLUTE SCOPE

The implementation remains **Phase 4 Part 7**.

The purpose is to establish and prove the **real PostgreSQL RLS security boundary** for the Phase 4 tenant/organization/branch scope model.

This is not Phase 5.

Phase 5 documentation is reference-only for compatibility and boundary decisions. It is not an implementation authorization.

Do not build the production PostgreSQL migration/persistence foundation ahead of Phase 5.

Do not turn this PR into a generalized database migration program.

Do not implement later Phase 4 parts or Phase 5 business persistence.

---

# 2. REQUIRED REMEDIATION TARGETS

The existing PR #39 must be corrected in all of these areas:

1. Remove or reduce premature production PostgreSQL persistence/migration architecture.
2. Keep only the PostgreSQL infrastructure necessary to execute a real Part 7 security proof.
3. Make `WITH CHECK` tests prove RLS rather than accidentally proving foreign-key rejection.
4. Remove arbitrary `AuthorizedScope` widening.
5. Exercise the actual Phase 4 authorization/scope path in at least one composition test.
6. Complete PostgreSQL policy catalog assertions.
7. Complete runtime role privilege and ownership assertions.
8. Remove hardcoded reusable database credentials.
9. Make test database/schema lifecycle deterministic and isolated.
10. Prove transaction-local context across pool reuse and rollback.
11. Add concurrent write isolation coverage.
12. Complete positive/negative operation symmetry.
13. Align or explicitly document the PostgreSQL version baseline.
14. Fix the currently failing CI/security gate.
15. Preserve all existing repository verification gates.
16. Rewrite stale PR claims so every statement is backed by evidence.

---

# 3. CONCRETE CHANGE → TEST MAPPING

Every implementation change must map to one or more executable tests below. A code change without a mapped test is incomplete unless it is purely mechanical and does not alter behavior.

## 3.1 Remove production Phase-5 creep

### Change
Remove/reduce any production-looking migration history, production PostgreSQL CRUD architecture, or permanent schema implementation that is outside Part 7.

### Required tests/evidence

- Clean Part 7 security harness provisions its own disposable PostgreSQL security fixture.
- No production API depends on the Part 7 fixture schema merely to compile or run unrelated Phase 4 tests.
- Existing Phase 4 tests continue to pass without the new production persistence layer.
- Diff inspection proves no unrelated Phase 5 business tables/workflows were introduced.

### Evidence

- changed-file inventory;
- diff scope report;
- relevant existing Phase 4 test output;
- Part 7 harness startup log;
- explicit list of deferred Phase 5 work.

---

## 3.2 Real PostgreSQL RLS

### Change
Execute protected operations using a real PostgreSQL server and restricted runtime role.

### Required tests

- `rls_real_database_enforcement`
- `rls_runtime_role_enforcement`
- `rls_policy_catalog_verification`

### Required proof

A broad query executed by the runtime role with Tenant A context must not expose Tenant B rows even when the application does not add a tenant predicate that would independently filter B.

### Evidence

- PostgreSQL server version;
- runtime role name;
- transaction context used;
- SQL operation class;
- returned row identifiers/counts;
- policy catalog rows;
- PASS/FAIL result.

---

## 3.3 `USING` read isolation

### Change
Ensure RLS `USING` prevents unauthorized existing-row visibility/targeting.

### Required tests

- Tenant A reads A successfully.
- Tenant A cannot read B.
- Tenant B reads B successfully.
- Tenant B cannot read A.
- Broad identifier query cannot bypass RLS.

### Evidence

For every case record:

```text
principal: tenant-A/runtime-role
context: organization=A, branch=<scope>
target: resource-B
operation: SELECT
application_predicate: absent where direct-RLS proof is required
expected: denied/hidden
observed_rows: 0
result: PASS
```

---

## 3.4 `WITH CHECK` INSERT isolation

### Change
Ensure inserts are rejected by RLS `WITH CHECK` when the row ownership is outside the active tenant.

### Required test design

The malicious row must be **relationally valid**.

Do not use a fake/nonexistent branch merely to make insertion fail.

The row must reference an existing valid branch/organization combination so foreign keys do not independently reject it.

Then attempt insertion under the wrong tenant context.

### Required proof

The operation reaches the RLS security boundary and is denied because the row fails the tenant policy.

### Evidence

Record:

```text
fixture organization: B
fixture branch: valid B branch
active tenant context: A
operation: INSERT
expected: RLS denial
observed error class: <actual PostgreSQL result>
rows before: N
rows after: N
foreign-key-valid-fixture: true
result: PASS
```

---

## 3.5 `WITH CHECK` ownership-changing UPDATE

### Change
Prevent an authorized tenant from rewriting an existing row so that its tenant ownership changes to another tenant.

### Required test design

The attempted final row must be relationally valid.

Do not rely on an invalid organization/branch combination to trigger a foreign-key violation.

### Required proof

The mutation is denied by the RLS `WITH CHECK` boundary and database state remains unchanged.

### Evidence

```text
existing row owner: A
active context: A
attempted new owner: B
new branch/tenant references: relationally valid
operation: UPDATE
expected: RLS denial
rows before: N
rows after: N
final owner: A
result: PASS
```

---

## 3.6 Update/delete isolation

### Change
Ensure RLS prevents cross-tenant mutations.

### Required tests

- A updates A → success.
- A updates B → denial.
- A deletes A → success.
- A deletes B → denial.
- B mirrors the same checks.

### Evidence

For each denied mutation:

```text
target owner
active context
operation
expected denial
observed denial
rows affected
state-before hash/identity
state-after hash/identity
result
```

No denied operation may alter durable state.

---

## 3.7 Branch isolation

### Change
Enforce branch-scoped access without permitting branch scope to escape its organization.

### Required tests

- A organization scope sees authorized A branches.
- A branch-scoped context sees only its authorized branch.
- A branch-scoped context cannot see another A branch.
- A cannot use a B branch.
- B cannot use an A branch.
- Cross-organization branch/resource combinations fail closed.

### Evidence

Record organization, branch, resource, active scope, expected visibility, observed visibility, and result for every case.

---

## 3.8 `AuthorizedScope` integrity

### Change
Remove any public/internal constructor or mutator that can attach an arbitrary branch to an authorized organization without validation.

### Required tests

- Valid effective scope produces valid `AuthorizedScope`.
- Invalid organization/branch combination cannot produce a trusted scope.
- Arbitrary branch IDs cannot widen a trusted scope.
- Client-supplied organization/branch values cannot directly become trusted DB context.

### Evidence

- compile/type-level evidence where applicable;
- unit test of scope construction;
- integration test from actual scope resolution;
- negative attempt to widen scope;
- final API surface/diff review.

---

## 3.9 Application → DB composition

### Change
Bind the existing Phase 4 trusted scope to the PostgreSQL transaction context.

### Required test

Exercise the actual Phase 4 path:

```text
principal
→ membership
→ authorization/scope resolution
→ AuthorizedScope
→ DB transaction
→ transaction-local context
→ runtime role
→ RLS
```

Do not manually manufacture the trusted scope for the only composition test.

### Evidence

Record:

```text
principal: <fixture principal>
resolved organization: A
resolved branch: A1
AuthorizedScope source: actual resolver
DB context: A/A1
runtime role: restricted role
operation: <operation>
expected: allowed/denied
observed: allowed/denied
result: PASS
```

---

## 3.10 Missing/invalid context

### Change
Fail closed when PostgreSQL tenant context is absent or invalid.

### Required tests

- no organization context → no tenant rows;
- invalid organization context → no tenant rows;
- nonexistent organization → no tenant rows;
- invalid branch context → no unauthorized rows;
- transaction-context initialization failure → operation fails safely.

### Evidence

Record context value, SQL operation, expected zero access, observed rows/error, and database state.

---

## 3.11 Connection-pool isolation

### Change
Guarantee tenant context is transaction-local and cannot leak across pooled connections.

### Required test sequence

```text
transaction A
→ SET LOCAL tenant A
→ read A
→ commit/rollback

return connection to pool

transaction B
→ SET LOCAL tenant B
→ read B
→ verify A invisible
```

Repeat enough bounded iterations to exercise connection reuse.

### Evidence

Record:

```text
iteration
connection reuse observed
transaction A context
transaction B context
rows returned to A
rows returned to B
cross-tenant rows observed
result
```

Any cross-tenant row is an immediate failure.

---

## 3.12 Rollback safety

### Change
Prove rollback cannot leave tenant context or unauthorized state behind.

### Required tests

- tenant context inside rolled-back transaction disappears with transaction;
- next transaction establishes its own context;
- rolled-back mutation leaves state unchanged;
- pooled connection remains safe afterward.

### Evidence

Before/after state plus context behavior across transaction boundaries.

---

## 3.13 Concurrent tenant isolation

### Change
Prove tenant context is not process-global and cannot contaminate concurrent transactions.

### Required tests

Run bounded concurrent Tenant A and Tenant B operations.

Include both reads and writes.

### Required proof

```text
A transaction → A context → A operations
B transaction → B context → B operations
```

No A operation may observe or mutate B state and vice versa.

### Evidence

For every worker/transaction record:

```text
worker
transaction
tenant context
operation
target
expected tenant
observed tenant/state
cross-tenant observation
result
```

---

## 3.14 Policy catalog verification

### Change
Verify actual PostgreSQL catalog state rather than merely inspecting migration source.

### Required assertions

For every protected relation:

- relation exists;
- RLS enabled;
- FORCE RLS enabled where required;
- exact policy name;
- policy command;
- permissive/restrictive semantics where relevant;
- target role;
- exact/normalized `USING` expression;
- exact/normalized `WITH CHECK` expression.

### Evidence

Persist the catalog query result or CI artifact containing the verified policy metadata.

Migration text alone is not evidence.

---

## 3.15 Runtime role and ownership

### Change
Prove actual runtime database authority.

### Required assertions

```text
rolsuper = false
rolbypassrls = false
rolcreaterole = false
rolcreatedb = false
```

Also verify:

- role memberships;
- database CONNECT;
- schema USAGE;
- protected table privileges;
- required sequence privileges where applicable;
- protected relation ownership;
- absence of unintended privilege grants.

### Evidence

Catalog query output must be captured for the final CI run.

---

## 3.16 Credential/configuration boundary

### Change
Remove hardcoded reusable database credentials from source.

### Required tests/evidence

- source scan finds no committed test password/connection string matching the old hardcoded credentials;
- test configuration comes through the approved test configuration boundary;
- credentials are ephemeral or test-scoped;
- secrets do not appear in logs/artifacts.

### Evidence

```text
secret scan: PASS
credential source: <approved mechanism>
log redaction check: PASS
```

Never include actual secret values in evidence.

---

## 3.17 Test lifecycle isolation

### Change
Replace shared dirty test state with deterministic disposable lifecycle.

### Required proof

Two independent executions of the security suite must produce equivalent fixture state and results without relying on data from the previous execution.

### Evidence

Record environment creation, migration/setup completion, fixture seed count, test result, cleanup completion.

---

## 3.18 PostgreSQL version

### Change
Resolve the CI/test PostgreSQL version mismatch intentionally.

### Evidence

Record:

```text
required/documented version: <version>
tested CI version: <version>
compatibility decision: aligned | explicitly justified
reason/evidence: <short statement>
```

Do not silently claim support for an untested database version.

---

## 3.19 Dependency/license gate

### Change
Fix the current security workflow failure caused by the SQLx dependency graph/license policy.

### Required behavior

Do not disable cargo-deny.

Do not broadly relax license policy merely to make CI green.

Use the narrowest repository-approved dependency/license correction.

### Evidence

Capture the final cargo-deny result including:

```text
command
commit SHA
exit code
policy result
unapproved licenses: none
result: PASS
```

---

# 4. MANDATORY GATE EVIDENCE FORMAT

Every verification gate must produce evidence in the following exact logical format.

```text
GATE: <stable gate name>
CATEGORY: <build|test|security|database|policy|architecture|ci>
COMMIT: <40-char SHA>
ENVIRONMENT: <local|CI>
PLATFORM: <platform>
DATABASE: <version or N/A>
COMMAND: <exact command>
SCOPE: <what was verified>
EXPECTED: <specific expected property>
OBSERVED: <specific observed result>
EXIT_CODE: <integer>
ARTIFACT: <artifact/path/URL or NONE>
STATUS: PASS | FAIL | NOT_EXECUTED
FAILURE_REASON: <required only when FAIL or NOT_EXECUTED>
REMEDIATION: <required when FAIL>
```

Do not replace evidence with:

```text
looks good
passed locally
CI green
works
all tests pass
```

Those statements are insufficient evidence.

---

# 5. REQUIRED GATE INVENTORY

The final PR must provide evidence for **every gate below**.

## GATE-01 — Repository formatting

Expected:

```text
formatter succeeds with zero diff
```

Evidence must contain exact command, SHA, exit code, and result.

---

## GATE-02 — Clippy / static analysis

Expected:

```text
no new warnings/errors under repository policy
```

Evidence must contain command and complete status.

---

## GATE-03 — Workspace compilation

Expected:

```text
workspace compiles successfully
```

No partial package-only claim unless the repository gate itself is package-scoped.

---

## GATE-04 — Existing workspace tests

Expected:

```text
all applicable existing tests pass
```

Report test count and failures.

---

## GATE-05 — Real PostgreSQL security suite

Expected:

```text
real PostgreSQL provisioned
RLS tests executed
no silent skip
all Part 7 security tests pass
```

This is a release-blocking gate.

---

## GATE-06 — RLS catalog verification

Expected:

```text
all required relations and policies match the intended database state
```

Attach/catalog evidence.

---

## GATE-07 — Runtime role privilege verification

Expected:

```text
runtime role cannot bypass RLS
runtime role has only required privileges
protected relation ownership cannot bypass intended RLS
```

Attach catalog evidence.

---

## GATE-08 — Cross-tenant negative tests

Expected:

```text
A cannot read/update/delete/insert B-owned data
B cannot read/update/delete/insert A-owned data
```

Every denied mutation must show unchanged state.

---

## GATE-09 — Branch isolation tests

Expected:

```text
branch-scoped access cannot cross branch or organization boundaries
```

---

## GATE-10 — Missing/invalid context tests

Expected:

```text
missing/invalid context never widens access
```

---

## GATE-11 — Pool/transaction safety

Expected:

```text
SET LOCAL tenant context cannot leak across pooled transactions
rollback cannot leave stale tenant context
```

---

## GATE-12 — Concurrent isolation

Expected:

```text
concurrent tenant reads and writes remain isolated
```

---

## GATE-13 — Application/DB composition

Expected:

```text
actual Phase 4 authorization scope feeds the trusted database context
```

---

## GATE-14 — Dependency/license policy

Expected:

```text
cargo-deny/security policy passes without policy weakening
```

---

## GATE-15 — Audit/security static checks

Expected:

```text
no hardcoded reusable DB credentials
no secret leakage
no accidental SQL/debug exposure
```

---

## GATE-16 — Release build

Expected:

```text
repository release build remains successful
```

---

## GATE-17 — Final scope audit

Expected:

```text
only Phase 4 Part 7 changes remain
no Phase 5 implementation creep
no unrelated refactor
no weakened existing security boundary
```

Evidence must be a final changed-file/diff inventory.

---

# 6. SECURITY TEST EVIDENCE MATRIX

The final security artifact must contain at least this table structure:

| ID | Security property | Operation | Active tenant | Target tenant | Branch | Expected | Observed | State unchanged | Evidence | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| RLS-001 | Read isolation | SELECT | A | A | A1 | allow | | N/A | | |
| RLS-002 | Read isolation | SELECT | A | B | B1 | deny | | N/A | | |
| RLS-003 | Read isolation | SELECT | B | A | A1 | deny | | N/A | | |
| RLS-004 | Update isolation | UPDATE | A | A | A1 | allow | | yes | | |
| RLS-005 | Update isolation | UPDATE | A | B | B1 | deny | | yes | | |
| RLS-006 | Delete isolation | DELETE | A | B | B1 | deny | | yes | | |
| RLS-007 | Insert `WITH CHECK` | INSERT | A | B | valid B branch | deny | | yes | | |
| RLS-008 | Ownership `WITH CHECK` | UPDATE | A | B | valid final relation | deny | | yes | | |
| RLS-009 | Branch isolation | SELECT | A/A1 | A/A2 | A2 | deny | | N/A | | |
| RLS-010 | Cross-org branch | SELECT | A | B | B1 | deny | | N/A | | |
| RLS-011 | Missing context | SELECT | none | A | A1 | deny/empty | | N/A | | |
| RLS-012 | Invalid context | invalid | A | A | A1 | deny/empty | | N/A | | |
| RLS-013 | Pool leakage | SELECT | B after A | A | A1 | deny | | N/A | | |
| RLS-014 | Concurrent write | UPDATE/INSERT | A | B | B1 | deny | | yes | | |

Extend this matrix for every protected relation and operation actually implemented.

---

# 7. DATABASE EVIDENCE ARTIFACT

The final CI artifact must include database-security evidence sufficient to independently inspect:

```text
PostgreSQL version
runtime role attributes
runtime role memberships
runtime database privileges
runtime schema privileges
runtime table privileges
protected relation owners
RLS enabled/forced state
policy names
policy commands
policy target roles
USING expressions
WITH CHECK expressions
fixture organization IDs
fixture branch IDs
security test results
```

Never include credentials or secrets.

---

# 8. CI FAILURE RULES

A gate is **FAIL**, not PASS, when:

- PostgreSQL cannot start;
- migrations/setup cannot execute;
- security tests are skipped;
- test discovery finds zero Part 7 tests unexpectedly;
- required policy metadata is absent;
- runtime role can bypass RLS;
- a cross-tenant operation succeeds;
- denied state changes;
- tenant context leaks across transactions;
- dependency/license policy fails;
- the repository does not compile;
- existing required tests fail;
- evidence cannot identify the tested commit;
- the final PR description claims success without evidence.

There is no acceptable green-by-skip path.

---

# 9. RELEASE EVIDENCE BUNDLE

The final PR must contain or link to an artifact bundle with:

```text
01-gate-summary.txt
02-security-matrix.txt
03-postgres-catalog.txt
04-runtime-role-privileges.txt
05-migration-fixture-lifecycle.txt
06-concurrency-pool-results.txt
07-dependency-license-result.txt
08-test-summary.txt
09-scope-diff.txt
```

Each file must identify the tested commit SHA.

If GitHub Actions artifacts are used, the PR must identify the artifact name and workflow run.

---

# 10. FINAL PR CHECKLIST

Before declaring the PR complete:

```text
[ ] PR #39 still contains only Phase 4 Part 7 work
[ ] Phase 5 production implementation has not been pulled forward
[ ] real PostgreSQL is used
[ ] runtime role is restricted
[ ] runtime role cannot bypass RLS
[ ] protected relation ownership is verified
[ ] RLS catalog metadata is verified
[ ] USING is directly exercised
[ ] WITH CHECK INSERT is directly exercised
[ ] WITH CHECK UPDATE is directly exercised
[ ] malicious fixtures are relationally valid
[ ] AuthorizedScope cannot be widened arbitrarily
[ ] actual Phase 4 scope resolution is exercised
[ ] client tenant IDs are not authority
[ ] missing context fails closed
[ ] invalid context fails closed
[ ] branch isolation passes
[ ] cross-tenant read isolation passes
[ ] cross-tenant write isolation passes
[ ] denied state remains unchanged
[ ] pool leakage test passes
[ ] rollback test passes
[ ] concurrent read isolation passes
[ ] concurrent write isolation passes
[ ] no hardcoded reusable credentials remain
[ ] test lifecycle is deterministic
[ ] PostgreSQL version baseline is explicit
[ ] cargo-deny passes
[ ] existing CI gates pass
[ ] release build passes
[ ] no security test is silently skipped
[ ] every gate has evidence in the required format
[ ] PR description contains only verified claims
[ ] final diff has been manually audited
```

Any unchecked item means the PR is not complete.

---

# 11. COPY-PASTE JULES IMPLEMENTATION COMMAND

The following instruction is the execution command. Paste it into Jules exactly.

---

## JULES COMMAND

```text
HARD STOP. DO NOT MERGE. DO NOT DECLARE SUCCESS.

Remediate the existing Phase 4 Part 7 / PR-008 implementation in PR #39.

Repository: jelome265/sitolo
PR: #39
Contract: docs/phase4_part7_jules_implementation_contract.md
authoritative remediation contract: docs/phase4_part7_pr008_remediation_execution_contract.md

FOLLOW BOTH DOCUMENTS EXACTLY.

This is a correction task against the existing implementation. It is not permission to redesign the architecture, reinterpret the requirements, skip difficult work, or move defects to a later phase.

FIRST:
1. Inspect the complete PR #39 diff.
2. Inspect the actual current repository state.
3. Read all governing Phase 0–4 security/architecture/testing documentation.
4. Read the Phase 5 document only to preserve the phase boundary and compatibility. DO NOT IMPLEMENT PHASE 5.
5. Inspect PR-007 and the existing Phase 4 scope/authorization implementation.
6. Build a concrete defect list before editing.

THEN FIX EVERYTHING.

MANDATORY REMEDIATION:

- Remove/reduce premature production PostgreSQL migration/persistence architecture that belongs to Phase 5.
- Keep only the PostgreSQL infrastructure necessary to execute a real Part 7 security proof.
- Use a real disposable PostgreSQL instance.
- Use a restricted runtime role.
- Prove SUPERUSER=false.
- Prove BYPASSRLS=false.
- Prove CREATEROLE=false.
- Prove CREATEDB=false.
- Prove actual role memberships and database/schema/table privileges.
- Prove protected relation ownership.
- Enable real PostgreSQL RLS.
- Verify actual RLS catalog state.
- Verify exact policy names, commands, roles, USING expressions, and WITH CHECK expressions.
- Do not accept policy source text as database evidence.

FIX THE FALSE-POSITIVE SECURITY TESTS.

The cross-tenant INSERT test MUST use an existing valid branch/organization fixture. Do not use a fake branch ID that can cause a foreign-key failure before RLS is evaluated.

The ownership-changing UPDATE test MUST produce a relationally valid final row. Do not depend on a foreign-key violation to make the test pass.

The tests must prove the RLS boundary itself.

Do not use generic is_err() assertions when an unrelated database constraint could satisfy the assertion. Where necessary, verify the failure class and independently verify database state.

FIX AuthorizedScope.

Remove any arbitrary branch mutator/constructor that allows an already trusted scope to be widened with an unvalidated branch ID.

A client-supplied organization_id or branch_id is not authority.

The trusted database context must derive from the real Phase 4 authorization/scope-resolution path.

Add at least one integration test proving:

principal -> membership -> authorization/scope resolution -> AuthorizedScope -> transaction-local DB context -> restricted PostgreSQL role -> RLS

Do not manufacture the only trusted scope directly inside the integration test.

FIX TRANSACTION SAFETY.

Tenant context must be transaction-local.

Prove pool reuse, rollback, repeated transactions, and bounded concurrent Tenant A/Tenant B activity.

Add concurrent WRITE coverage. Read-only concurrency is insufficient.

Prove no wrong-tenant write can commit.

FIX FAIL-CLOSED BEHAVIOR.

Missing context, invalid context, unknown tenant, unknown branch, context initialization failure, pooled connection reuse, and database setup failure must never widen access.

PostgreSQL/security test provisioning failure MUST fail CI.

NEVER silently skip the security suite.

FIX CREDENTIALS.

Remove hardcoded reusable database credentials from source.

Use the repository-approved test configuration boundary or ephemeral test credentials.

Do not expose credentials in source, logs, artifacts, or PR text.

FIX TEST LIFECYCLE.

Do not leave security fixtures accumulating in a globally shared database.

Make the security environment deterministic and disposable.

FIX CI.

The current security/license gate is failing because the SQLx dependency graph is not accepted by the repository's current cargo-deny policy.

Resolve the actual dependency/license issue.

DO NOT disable cargo-deny.
DO NOT broadly relax license policy.
DO NOT hide the failure.
DO NOT add continue-on-error.
DO NOT skip the security workflow.

Resolve the dependency policy correctly and rerun the complete gate.

FIX THE DATABASE VERSION BASELINE.

The current CI PostgreSQL version and the documented database target must not silently disagree. Align them or document and technically justify the compatibility baseline.

COMPLETE THE SECURITY MATRIX.

Every protected operation needs positive and negative evidence:

A reads A -> PASS
A reads B -> DENY
A creates A -> PASS
A creates B -> DENY
A updates A -> PASS
A updates B -> DENY
A deletes A -> PASS
A deletes B -> DENY

Also prove:

- ownership-changing UPDATE denial;
- branch isolation;
- missing context;
- invalid context;
- pool leakage;
- rollback safety;
- concurrent reads;
- concurrent writes;
- direct database RLS enforcement independent of an application tenant WHERE predicate.

Every denied mutation must prove durable state is unchanged.

EVIDENCE IS MANDATORY.

For EVERY gate produce evidence using exactly this logical format:

GATE: <name>
CATEGORY: <category>
COMMIT: <40-char SHA>
ENVIRONMENT: <environment>
PLATFORM: <platform>
DATABASE: <version or N/A>
COMMAND: <exact command>
SCOPE: <what was verified>
EXPECTED: <specific expected property>
OBSERVED: <specific observed result>
EXIT_CODE: <integer>
ARTIFACT: <artifact/path/URL or NONE>
STATUS: PASS | FAIL | NOT_EXECUTED
FAILURE_REASON: <required when failed/not executed>
REMEDIATION: <required when failed>

Do not report 'looks good', 'works', or 'all tests pass' without the actual evidence fields.

REQUIRED GATES:

GATE-01 formatting
GATE-02 clippy/static analysis
GATE-03 workspace compilation
GATE-04 existing workspace tests
GATE-05 real PostgreSQL RLS security suite
GATE-06 PostgreSQL policy catalog verification
GATE-07 runtime role privilege/ownership verification
GATE-08 cross-tenant negative tests
GATE-09 branch isolation
GATE-10 missing/invalid context
GATE-11 transaction/pool/rollback safety
GATE-12 concurrent tenant isolation including writes
GATE-13 application authorization -> DB composition
GATE-14 dependency/license policy
GATE-15 security/secret static checks
GATE-16 release build
GATE-17 final scope/diff audit

The final PR must contain evidence sufficient to identify the exact commit tested and the exact command executed for every gate.

Do not claim a gate passed if it was not executed.

Do not claim PostgreSQL RLS is proven if the test did not actually execute against PostgreSQL.

Do not claim WITH CHECK is proven if the malicious fixture can be rejected by an unrelated foreign key.

Do not claim tenant isolation is proven if application-side filtering is the only protection exercised.

Do not claim runtime RLS is safe if the runtime role can bypass RLS or owns protected relations in a way that defeats the intended boundary.

Do not claim CI is green while any required gate is red.

After fixing everything:

1. Run the complete repository verification suite.
2. Run the complete real PostgreSQL Part 7 security suite.
3. Capture evidence for every gate.
4. Inspect the full final diff.
5. Remove unrelated changes.
6. Confirm Phase 5 has not been implemented prematurely.
7. Update the PR description so every claim is backed by the final evidence.
8. Push all remediation commits to PR #39.

DO NOT STOP AFTER THE FIRST FIX.

DO NOT DECLARE COMPLETION UNTIL EVERY CHECKLIST ITEM IN docs/phase4_part7_pr008_remediation_execution_contract.md IS SATISFIED.

The acceptance condition is executable evidence, not source-code intent.
```

---

# 12. FINAL ACCEPTANCE RULE

The PR is merge-ready only when the implementation and evidence demonstrate all of the following simultaneously:

```text
Phase 4 trusted authorization
        ↓
AuthorizedScope integrity
        ↓
transaction-local trusted DB context
        ↓
least-privileged runtime role
        ↓
real PostgreSQL
        ↓
real RLS USING/WITH CHECK
        ↓
independent negative security evidence
        ↓
pool/concurrency safety
        ↓
all repository CI gates green
```

A source implementation that merely appears to provide this chain is insufficient.

A passing test that can succeed for the wrong reason is insufficient.

A green CI run with skipped security tests is insufficient.

A PostgreSQL test run using a privileged bypass role is insufficient.

A policy migration file without PostgreSQL catalog evidence is insufficient.

A PR description without gate evidence is insufficient.

**The security boundary must be real, narrow, adversarially tested, and reproducibly evidenced.**
