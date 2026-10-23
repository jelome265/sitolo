# Environment configuration reference

This page documents Sitolo's environment contracts and the two reference files
that accompany them:

- `.env.example` — local application configuration and the development secret
  value placeholder.
- `.env.test.example` — PostgreSQL integration-test harness inputs.

Neither file is an application loader. Sitolo reads configuration directly from
the process environment at startup. There is no automatic `.env` file loader
and no dotenv dependency; exporting the variables (or supplying them through a
process supervisor) is what configures the process.

The canonical configuration contract is defined by `crates/sitolo-config`.
The examples are derived from that contract and do not create a second
configuration mechanism.

## Two disjoint namespaces

| Namespace | Owner | Purpose | Unknown keys |
|---|---|---|---|
| `SITOLO__` | `sitolo-config` | Ordinary typed configuration | Fail startup |
| `SITOLO_SECRET__` | Development `EnvSecretProvider` | Secret values for a development process | Not parsed as configuration |

`SITOLO_SECRET__` is deliberately disjoint from `SITOLO__`: the single
underscore after `SITOLO` means it cannot match the `SITOLO__` prefix. The
configuration loader therefore ignores it, and the development secret provider
reads it. The former `SITOLO__DATABASE__PASSWORD` name is **not** accepted: it
is an unknown configuration key and fails startup before any secret can be
resolved. Use `SITOLO_SECRET__DATABASE__PASSWORD` instead.

This separation preserves two different contracts:

- A **secret reference** such as `SITOLO__DATABASE__PASSWORD_REF` is ordinary
  configuration. It names where a secret lives and is safe to fingerprint.
- A **secret value** such as `SITOLO_SECRET__DATABASE__PASSWORD` is credential
  material. It is never parsed as configuration and never enters the
  configuration fingerprint.

## Supplying values

For a local POSIX shell:

```sh
set -a
. ./.env.example
set +a
cargo run -p sitolo-api-bin
```

Replace every placeholder first. The `source` step is a shell convenience; the
application itself only sees the exported process environment. In deployment,
use the process supervisor or secret-injection mechanism appropriate to the
environment. Configuration is read once at startup and is not hot-reloaded.

## Application configuration keys

All keys below use the `SITOLO__` prefix. The defaults are the compiled
development defaults; staging and production deliberately require explicit
database identity and a production secret reference.

| Key (without `SITOLO__`) | Default | Notes |
|---|---|---|
| `RUNTIME__ENVIRONMENT` | `development` | `development`, `staging`, or `production`; unknown values fail startup |
| `RUNTIME__SERVICE_NAME` | `sitolo` | Bounded non-empty service identity |
| `RUNTIME__SERVICE_VERSION` | `0.0.0+dev` | Bounded non-empty provenance value |
| `RUNTIME__CONFIG_SCHEMA_VERSION` | `2` | Only known schema version `2` is accepted |
| `HTTP__BIND_ADDRESS` | `0.0.0.0:8080` | Socket address |
| `HTTP__MAX_BODY_BYTES` | `2097152` | At most 26,214,400 bytes |
| `HTTP__REQUEST_HEADER_TIMEOUT_MS` | `5000` | At most 30,000 ms |
| `HTTP__KEEPALIVE_TIMEOUT_MS` | `30000` | At most 300,000 ms |
| `DATABASE__HOST` | `localhost` | Explicit host required in staging/production |
| `DATABASE__PORT` | `5432` | Non-zero port |
| `DATABASE__NAME` | `sitolo` | Explicit database name required in staging/production |
| `DATABASE__USER` | `sitolo_api` | Local examples use `sitolo_app_runtime` |
| `DATABASE__PASSWORD_REF` | `development/sitolo/db` | Typed secret reference; staging/production reject the development namespace |
| `DATABASE__POOL_MIN` | `5` | At most 256 |
| `DATABASE__POOL_MAX` | `20` | At least `POOL_MIN`, at most 512 |
| `DATABASE__ACQUIRE_TIMEOUT_MS` | `2000` | At most 60,000 ms |
| `TELEMETRY__OTLP_ENDPOINT` | absent (disabled) | Remote endpoints require TLS; loopback HTTP is development-only |
| `TELEMETRY__EXPORT_TIMEOUT_MS` | `3000` | At most 60,000 ms |
| `TELEMETRY__MAX_QUEUE` | `2048` | At most 1,048,576 |
| `TELEMETRY__TRACE_SAMPLE_RATIO` | `0.1` | Inclusive range `0.0..=1.0` |
| `LOG__LEVEL` | `info` | `trace`, `debug`, `info`, `warn`, `error`; production ceiling is `info` |
| `SECRETS__ALLOW_LOCAL_PROVIDER` | `true` | Must be `false` in production; validation rejects a local provider there |
| `SECRETS__AWS_REGION` | absent | Required by production process bootstrap for ECS-role-backed AWS Secrets Manager lookup; bounded AWS region identifier |

The source of truth for this table is
`crates/sitolo-config/src/field.rs`; the typed model and defaults live in
`crates/sitolo-config/src/model.rs`.

## Development secret values

The development provider maps each secret class to a fixed environment name.
The database value used by the API startup probe is:

```text
SITOLO_SECRET__DATABASE__PASSWORD
```

The other classes map to `SITOLO_SECRET__IDENTITY__CLIENT_SECRET`,
`SITOLO_SECRET__PAYMENT__CREDENTIAL`, `SITOLO_SECRET__WEBHOOK__VERIFICATION_SECRET`,
`SITOLO_SECRET__MRA__TERMINAL_SECRET`,
`SITOLO_SECRET__TELEMETRY__EXPORTER_CREDENTIAL`,
`SITOLO_SECRET__SIGNING_KEY`, `SITOLO_SECRET__OBJECT_STORAGE__CREDENTIAL`, and
`SITOLO_SECRET__SERVICE__CREDENTIAL`. The complete mapping is in
`crates/sitolo-security/src/provider.rs`.

Secret values are placeholders only in `.env.example`. Never commit a real
credential. The provider selects the variable by secret class, not by the
reference path, so two references of the same class resolve through the same
variable by design.

## Production limits

- Production rejects `SITOLO__SECRETS__ALLOW_LOCAL_PROVIDER=true`.
- Production API bootstrap uses AWS Secrets Manager with explicit ECS task-role
  credentials when `SECRETS__AWS_REGION` is set. It resolves only the configured
  database reference and never falls back to `SITOLO_SECRET__...`.
- The worker is a separate binary. An SNS FIFO publisher exists, but the worker
  executable remains fail-closed until its restricted database and relay runtime
  are composed. `.env.example` is an API configuration reference, not a
  production deployment manifest.
- Production and staging require explicit database host, name, user, and secret
  reference; the development defaults and the `development/` secret namespace
  are rejected.

## PostgreSQL test harness inputs

The integration tests use three direct harness inputs from the process
environment, documented in `.env.test.example`:

```text
ADMIN_DATABASE_URL
RUNTIME_DATABASE_URL
WORKER_DATABASE_URL
```

These are not application configuration. The harnesses fail closed when any is
absent. Use a disposable PostgreSQL 18 database: the harness creates and drops
isolated schemas, and its fixture creates the cluster-scoped `app_runtime`,
`app_worker`, and `app_outbox_owner` roles. The admin URL must be able to create
and drop schemas and those roles. The CI service uses trust authentication on
loopback only; a local cluster with password authentication needs matching
credentials in the URLs.

## Verification

The namespace boundary is covered by:

```sh
cargo test -p sitolo-config --test env_namespace --all-features --locked
cargo test -p sitolo-config -p sitolo-security --all-targets --all-features --locked
```

The release workflow's PostgreSQL wiring is checked by:

```sh
./scripts/ci/check-release-workflow
```

For local PostgreSQL setup, see
[PostgreSQL 18 runtime development](postgresql-runtime-development.md).
