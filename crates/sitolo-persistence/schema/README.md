# Persistence schema source

`migrations/` is the repository's ordered, versioned SQL source for the current
identity, tenancy, audit/outbox, and consumer-inbox schema. The release order,
role prerequisites, schema substitution rules, and remaining production-owner
gates are documented in [`migrations/README.md`](migrations/README.md).

PostgreSQL integration harnesses apply these same migration files to fresh,
disposable schemas. Test-only cluster role setup lives separately in
`crates/sitolo-persistence/tests/fixtures/role_bootstrap.sql`; it is not a
production migration and does not provide production credentials.

This source bundle does **not** establish an accountable production migration
owner, deployment artifact, execution identity, rollout window, or recovery
procedure. Do not represent production migration/release readiness as complete
until the platform/database owner approves and verifies those controls.
