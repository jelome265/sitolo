# Persistence schema artifacts

`phase4_part8_tenancy_provisioning.sql` is an additive administrative schema
change for the PR-009 organization-provisioning transaction. Apply it with the
database migration identity after the Part 8 roles exist and before deploying
an API build that uses PostgreSQL tenancy. The API runtime identity must not
apply schema changes. The script intentionally grants only `INSERT` to
`app_runtime`; table reads and lifecycle updates are outside this slice.

The SQL is also exercised through isolated per-test schemas by the PostgreSQL
integration suite. A production deployment must apply the versioned SQL through
its controlled migration process; this repository currently has no automatic
migration runner.
