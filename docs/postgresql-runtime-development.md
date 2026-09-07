# PostgreSQL 18 runtime development

Phase 2 defines how Sitolo treats PostgreSQL as a typed runtime dependency; it does not create schema, migrations, RLS, or application tables. Those are Phase 5 work.

Use a disposable PostgreSQL 18 container or an equivalent local PostgreSQL 18 installation. Configure only the canonical non-secret connection identity:

```sh
docker run --rm --name sitolo-postgres -p 5432:5432 \
  -e POSTGRES_DB=sitolo -e POSTGRES_USER=sitolo_app_runtime \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  postgres:18
```

`trust` is only acceptable for this disposable loopback development container;
it is never a deployment setting.

```text
SITOLO__DATABASE__HOST=localhost
SITOLO__DATABASE__PORT=5432
SITOLO__DATABASE__NAME=sitolo
SITOLO__DATABASE__USER=sitolo_app_runtime
SITOLO__DATABASE__PASSWORD_REF=development/sitolo/db
```

For isolated development only, the environment secret provider resolves the synthetic password from `SITOLO__DATABASE__PASSWORD`. Do not commit that value, `.env` files, connection URLs, or provider credentials. The local provider is rejected in production.

Use a separate future migration-owner role and runtime application role. The runtime role must not be a PostgreSQL superuser, `BYPASSRLS`, role creator, database creator, or unrestricted schema owner. Phase 5 will define actual grants, schemas, migrations, RLS, and real PostgreSQL integration tests.

Verify the Phase 2 boundary without requiring PostgreSQL schema work:

```sh
cargo test -p sitolo-config -p sitolo-persistence --all-targets --all-features --locked
./scripts/ci/check-phase2-policy
```
