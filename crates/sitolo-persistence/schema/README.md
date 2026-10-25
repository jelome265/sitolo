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

The runtime release order is: provision the cluster roles required by the
Phase 3 identity SQL, apply `phase3_identity_authority.sql` with the schema
placeholder replaced by the selected application schema, apply the canonical
Phase 4 organization/branch and authorization schema from the Phase 5 migration
authority, then apply `phase4_part8_tenancy_provisioning.sql`. Install the
audit/outbox schema from the Phase 5 migration authority before this final
step. The API checks for the complete set of identity, tenancy, audit, outbox,
and session functions at startup. This repository does not yet contain a
production migration runner or a versioned canonical Phase 5 migration bundle;
an operator must not point the runtime at a partially provisioned schema.

`consumer_inbox.sql` adds the consumer-side durable idempotency table. It
grants only SELECT and INSERT on that table to the existing restricted
`app_worker` role and applies forced tenant RLS; the consumer transaction sets
the organization context locally, inserts an uncommitted marker, applies its
approved business effect, and commits both together. Before inserting a new
marker, the adapter takes a transaction-scoped advisory lock for that
consumer/organization/aggregate and requires the next contiguous aggregate
sequence. A gap leaves the message unacknowledged, so an earlier quarantined
sequence blocks later effects until it is replayed or an audited skip operation
is implemented and authorized. The production migration must also scope
`outbox_aggregate_sequences` and the outbox aggregate-sequence uniqueness
constraint by organization to match the approved SNS FIFO message group. These
schema changes remain subject to the externally controlled migration release;
no production migration owner or deployment mechanism is defined here.
