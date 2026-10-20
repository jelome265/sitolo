# Stage 07 Remediation — PR-009 Part 8

**Input:** `06-audit-pr009-part8.md`
**Decision:** Apply the safe correction available within this slice; retain unresolved findings and require re-audit.

## Finding disposition

| Finding | Action | Result / residual |
|---|---|---|
| F-01 Unauthenticated provisioning + durable runtime | Removed `StartupContext::build()` PostgreSQL connection and `AppState::configure_postgres_tenancy`; removed API binary's direct SQLx dependency | Prevents this change from wiring the unauthenticated HTTP DTO to durable database mutations. Atomic app/persistence capability and PostgreSQL integration tests remain for a future caller. Full closure requires authenticated principal integration and server-derived owner identity. Existing in-memory HTTP tenancy routes are a pre-existing Phase 3 limitation. |
| F-02 PostgreSQL behavior / provisioning race evidence | No local PostgreSQL or supplied DB identities are available; CI PostgreSQL service is available | On `1aad95c`, integration run 781 passed the app suite; canonical Rust run 631 passed 23 Part 8 persistence tests and both 27-test legacy RLS invocations after `SET ROLE app_runtime`. Added a concurrent identical-provisioning test locally; exact-head PostgreSQL execution is pending. |
| F-03 Relay contract unresolved | No destination guessed | Not remediated; requester’s “main and sub” does not define transport/configuration. |
| F-04 dependency policy unresolved | No advisory suppression or exception added | Not remediated; requires dependency remediation or authorized governance disposition, then scanner gates. |

## Changed files

- `apps/api/src/bootstrap.rs`: removed production process database connection and role assumption.
- `apps/api/src/state.rs`: removed durable tenancy evidence-sink wiring.
- `apps/api/Cargo.toml`: removed now-unused direct SQLx dependency.
- `crates/sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql`: added a transaction-scoped advisory lock around cluster-global fixture role creation after the `dca49f8` PostgreSQL CI run exposed a unique-index race.
- `crates/sitolo-persistence/src/postgres.rs`: runtime pool now executes `SET ROLE app_runtime` on each connection, matching the explicit least-privileged runtime identity required by the RLS harness.
- `map/processes/postgres-authority-initialization.md` and `map/objects/data/persistence.md`: source revision and citations now track the exact code commit and describe the runtime role assumption; `map/objects/_index.md` was regenerated after policy run 483 found the stale source revision.
- `crates/sitolo-application/tests/part8_provision_organization_evidence_tests.rs`: added an `organizations.id`-specific count helper after the `561bbe9` PostgreSQL run exposed the generic helper's invalid `organization_id` query.
- Archived run reports: rewired relative stage-output references to sibling files under `runs/pr009-part8` after the policy checker surfaced six stale links.

The application transaction implementation remains and is still compile/test-target checked. This remediation does not claim production API atomicity: the endpoint cannot safely use it until authentication context exists. No audit or evidence requirement was weakened.

## Verification and routing

The implementation change was re-audited in Stage 06. Targeted checks pass: `cargo fmt --all -- --check`, `cargo test -p sitolo-api-bin --test startup --test tenancy_api` (11 tests), `cargo check --workspace --all-targets --locked`, the ICM cold-walk, and `python scripts/ci/check-doc-references` (0 unresolved links). Review of `561bbe9` exposed an organization count query bug, corrected in `1383233`. On `1aad95c`, application PostgreSQL integration run 781 passed, canonical Rust run 631 passed 23 Part 8 persistence tests and both 27-test RLS suite invocations, and policy run 484 passed after the map revision correction. Canonical verification then failed only at cargo audit (`RUSTSEC-2023-0071`). A two-request same-payload provisioning test has now been added locally and compiled; it still needs exact-head PostgreSQL execution. Format, all-target check, clippy, ICM, System Map, and document reference checks pass locally. Audit-stage human review was not performed; continuation is under the requester’s prior instruction to proceed autonomously, not a representation of approval. The re-audit still has unresolved findings, so Stage 08 is not cleared.
