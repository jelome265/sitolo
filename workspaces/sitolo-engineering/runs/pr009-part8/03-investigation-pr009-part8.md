# Investigation — PR-009 Part 8

**Stage:** 03 Investigate\
**Date:** 2026-10-20\
**Revision inspected:** `1223ed3aef3818f56c5b444ee22fff287546349f`\
**Continuation:** requester directed restart at Stage 01, selected PostgreSQL-backed atomic tenancy provisioning, cannot supply a PostgreSQL environment, and asked us to proceed without requiring them to view the Stage 02 memo.

## Scope and controlling requirements

The Stage 01 brief and Stage 02 research memo are the working inputs. Binding sources are `docs/phase4_part8_audit_outbox_implementation_contract.md` and `docs/phase4_part8_pr009_remediation_execution_contract.md`. The latter requires a real Phase 4 command to write business state, mandatory audit evidence, and outbox intent in one PostgreSQL transaction; it explicitly says audit or outbox failure must leave no business mutation. The run brief keeps this requirement in P0 and leaves the PR open for human review and merge.

Security applicability remains `yes`. Carried controls: SC-002 (server-derived tenant scope), SC-006 (transaction integrity/idempotency/durable evidence), SC-009 (durable audit distinct from telemetry), and SC-010 (bounded resources and fail-closed behavior).

## Existing implementation and evidence

### Provision command and transaction boundary

- `crates/sitolo-application/src/tenancy.rs`: `TenancyService` owns an `Arc<TenancyDatabase>` and an optional `TenancyEvidenceSink`. `provision_organization` calls `TenancyStores::provision_organization` first, then calls `record_evidence`. The latter opens a separate `PgPool` transaction and calls the audit and outbox writers before committing. Thus only audit+outbox are atomic; the successful in-memory organization/owner-membership/default-branch bundle is already committed if evidence writing fails.
- `crates/sitolo-persistence/src/tenancy.rs`: `TenancyDatabase` is a mutex-protected in-memory reference store. Its provisioning operation validates and atomically inserts organization, owner membership, and default branch only inside that process-local lock.
- `crates/sitolo-application/tests/part8_provision_organization_evidence_tests.rs`: current test names and comments explicitly document the business mutation gap. Its failure test asserts that memory state survives evidence failure. The suite has a real-PG setup that panics when `ADMIN_DATABASE_URL`/`DATABASE_URL` is absent; replacing it with a mock or silent skip would invalidate the requested proof.
- `apps/api/src/state.rs`: API bootstrap constructs `TenancyDatabase::new()` and a `TenancyService` without evidence wiring. The running API therefore does not use PostgreSQL as the tenancy authority today.

### Reusable persistence boundaries

- `crates/sitolo-persistence/src/audit_repository.rs`: `AuditWriter::record_required` accepts a caller-owned `&mut Transaction<Postgres>`; `PostgresAuditWriter` is stateless and inserts into the current transaction.
- `crates/sitolo-persistence/src/outbox_repository.rs`: `OutboxWriter::enqueue` accepts that same transaction; aggregate sequence allocation and the outbox insert use it.
- `crates/sitolo-persistence/src/postgres.rs`: `set_transaction_tenant_context` applies `app.organization_id` and `app.branch_id` with transaction-local `set_config`. Existing security fixture/tests cover that helper and database role boundaries.
- `crates/sitolo-persistence/src/lib.rs` says PostgreSQL tenancy implementations arrive in Phase 5. Repository search found no production migration directory or tenancy schema migration; only test fixtures exist (`rls_schema.sql`, Part 8 evidence schema, and the application evidence schema). The selected atomic provisioning therefore needs a durable schema owner and deployment path, not only test DDL.

### Test and CI environment

- The application evidence fixture currently creates only `audit_events` and `outbox_events`; it has no organization, membership, or branch tables.
- The Stage 02 memo records that `.github/workflows/rust.yml` supplies PostgreSQL 18 and admin/runtime URLs but lacks `WORKER_DATABASE_URL`; `.github/workflows/integration.yml` runs all-target workspace tests without a database while excluding only the persistence crate. Canonical CI failure logs show PostgreSQL-dependent tests fail during setup when URLs are absent.
- Requester cannot supply a database. The repository's CI PostgreSQL service is the available path for real-database proof, after correcting job wiring. This is not yet evidence that CI has run successfully.

### Worker destination

`apps/worker/src/main.rs` remains an empty scaffold. The binding contract calls for a narrow relay and downstream idempotency, but neither the working documents nor the source tree define a transport, endpoint/topic/queue, payload contract, or runtime configuration. The user's phrase “main and sub” does not identify those fields. The implementation must not create a broker, endpoint, or inferred mapping. Atomic tenancy provisioning can proceed independently; relay delivery and its duplicate-consumer proof remain dependent on a concrete destination contract.

## Findings and design constraints

1. The current provisioning path is not the authoritative PostgreSQL mutation; it cannot pass the P0 atomicity requirement.
2. Existing audit/outbox writers already support application-owned transaction orchestration. A bounded PostgreSQL provisioning repository can participate by accepting the same caller-owned transaction, while the application service begins, scopes, invokes the repository, records audit/outbox, and commits.
3. That change must persist the organization, owner membership, and default branch as one business bundle and return the existing `ProvisionedOrganization` domain result. It must preserve record-level idempotency and reject divergent reuse of identifiers.
4. The application API currently uses only the in-memory reference store. A production claim requires an explicit PostgreSQL-backed service wiring path and schema/deployment artifact; adding test-only SQL would not complete the selected requirement.
5. Tenant creation has no pre-existing organization row on which ordinary tenant RLS can authorize the insert. The investigation must preserve the established transaction-local scope model and use a narrowly privileged, reviewed creation path rather than pretending ordinary tenant RLS already solves bootstrap authorization.
6. Real PostgreSQL tests must inject audit and outbox failures after staging the business writes and assert that all three business rows and all evidence rows are absent. Success must assert the complete three-row bundle plus exactly one audit and one outbox row. Reused-ID same-payload and divergent-payload behavior need database-backed assertions.
7. The output policy failure and supply-chain CI failures in the Stage 02 memo remain separate merge blockers and need remediation/evidence in later stages. Do not suppress advisories or weaken workspace policy.

## Control traceability

| Control | Requirement / threat | Implementation implication | Required negative evidence | Gate / residual risk |
|---|---|---|---|---|
| SC-002 | Caller-provided tenant selectors cannot grant authority; bootstrap is especially sensitive | Validate IDs in domain; bind every organization/branch/membership insert; keep creation entrypoint narrowly scoped; no cross-tenant branch binding | Cross-org branch or membership binding is rejected; normal runtime identity cannot invoke privileged setup paths | Real PostgreSQL RLS/role tests; relay destination remains separate unresolved input |
| SC-006 | Partial authoritative provisioning or replay can create inconsistent topology | One PostgreSQL transaction for org+owner membership+default branch+audit+outbox; enforce unique/FK constraints and safe same-payload retry | Audit failure and outbox failure each roll back all business rows; divergent duplicate IDs conflict; concurrent retry does not duplicate | Real PostgreSQL transaction/concurrency tests; API currently uses memory and must be wired deliberately |
| SC-009 | Missing or forged audit evidence for a successful transition | Keep mandatory audit insert in the transaction and pair with a versioned outbox event | Missing audit/outbox relation or insert error makes command fail and rolls business rows back | Exact-head real PostgreSQL tests; event registry/version boundary still absent |
| SC-010 | Unbounded retry or worker work; uncertainty must fail closed | Keep SQL parameters bounded; do not add relay runtime until destination/config are explicit | Invalid limits and absent credentials fail closed; no fallback to runtime DB identity | CI role/URL setup and worker scope remain open |

## Unresolved decisions / dependencies

- Relay destination and contract: the phrase “main and sub” is insufficient to identify transport, endpoint/topic/queue, event schema, delivery acknowledgement, or runtime fields. Do not infer it.
- PostgreSQL availability: requester cannot supply URLs. Configure canonical CI PostgreSQL 18 and required identities; a local real-PG run remains unavailable.
- Schema deployment: repository contains no tenancy production migration mechanism. Stage 04 must select the smallest durable schema artifact compatible with repository ownership and deployment policy, without claiming a test fixture is production schema.

## Stage 03 disposition

Investigation is complete. Continue to Stage 04 under the requester's instruction to proceed autonomously. The design must preserve the full atomicity requirement and keep relay implementation explicitly pending its destination contract.

## Implementation follow-up

Stage 05 has since added the selected PostgreSQL provisioning path and runtime
wiring. Its source/test evidence is recorded in
`../05-implement/output/pr009-part8-implementation.md`. The findings above
remain a dated description of the pre-implementation revision; see the
implementation report for current status.
