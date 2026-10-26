# Versioned PostgreSQL migration source

This directory is the single repository-owned ordered SQL source for the
current Phase 3 identity authority and Phase 4 Part 8 persistence schema. The
versioned files are immutable once released: make schema changes in a new
numbered migration and append it to `manifest.toml`. The PostgreSQL integration
harnesses apply these exact source files to disposable schemas in this order.

| Version | File | Responsibility | Prerequisites |
|---|---|---|---|
| 0001 | `0001_phase3_identity_authority.sql` | Identity links, devices, and durable sessions | Provision and catalog-check `app_runtime` and `app_identity_owner`; replace `__SCHEMA__` with the validated quoted schema identifier. |
| 0002 | `0002_phase4_part8_audit_outbox.sql` | Audit events, outbox, organization-scoped aggregate sequence, relay functions, and metrics view | Provision and catalog-check `app_runtime`, `app_worker`, and `app_outbox_owner`; replace `__SCHEMA__` with the validated quoted schema identifier. |
| 0003 | `0003_phase4_part8_tenancy_provisioning.sql` | Organization, branch, membership, invitation, role, and scope state | Apply with the target application schema as `search_path`; `app_runtime` and owner roles must already exist. |
| 0004 | `0004_worker_consumer_inbox.sql` | Durable consumer idempotency and aggregate sequence guard | Apply with the target application schema as `search_path`; `app_worker` must already exist. |

The `__SCHEMA__` placeholder is an SQL identifier, not a value. A runner must
validate it against the approved schema-name contract and substitute a quoted
identifier before execution. Files without that placeholder require a
connection-local `search_path` set to the same validated target. Apply with a
separate DDL/migration identity; runtime and worker identities must never
create schemas, roles, tables, or functions. Login passwords and production
role provisioning remain the responsibility of the platform owner and secret
manager.

`manifest.toml` pins each artifact's SHA-256 and execution order. The CI
inventory check rejects missing, extra, renumbered, or modified migration
files. PostgreSQL integration CI then exercises the ordered bundle against a
disposable PostgreSQL service, including role, privilege, RLS, function, and
transaction assertions. The run must record the migration manifest version
and source revision.

## Release gates not established by this repository source

The repository does not yet identify the accountable production migration
owner or deployment/release artifact. A platform/database owner must define
the production runner, schema-version ledger behavior, preflight, rollout and
forward-fix/recovery procedure, and how the migration identity is provisioned.
The owner must then run the immutable source revision in an approved disposable
environment and verify deployed catalog privileges before production use.
This directory alone is not approval to deploy these migrations.
