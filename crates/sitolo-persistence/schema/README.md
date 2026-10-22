# Persistence schema artifacts

`phase4_part8_tenancy_provisioning.sql` is an additive administrative schema
change for the PR-009 organization-provisioning and membership-transition
transactions. Apply it with the
database migration identity after the Part 8 roles exist and before deploying
an API build that uses PostgreSQL tenancy. The API runtime identity must not
apply schema changes. The script grants scoped reads/inserts and only
column-level updates to membership `state` and `state_version`. RLS
restricts those transitions to the transaction-local tenant context; broad
table updates and deletes remain unavailable to `app_runtime`.

The SQL is also exercised through isolated per-test schemas by the PostgreSQL
integration suite. A production deployment must apply the versioned SQL through
its controlled migration process; this repository currently has no automatic
migration runner.
