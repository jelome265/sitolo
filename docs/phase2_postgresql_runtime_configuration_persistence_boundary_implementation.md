# SITOLO — PHASE 2 POSTGRESQL RUNTIME CONFIGURATION & PERSISTENCE BOUNDARY IMPLEMENTATION

**File:** `docs/phase2_postgresql_runtime_configuration_persistence_boundary_implementation.md`  
**Phase:** Phase 2 — Configuration, Secrets, Logging, Errors, Telemetry  
**Concern:** PostgreSQL runtime configuration, credential resolution, persistence boundary, database lifecycle, failure semantics, observability, and handoff to Phase 5  
**Database target:** PostgreSQL 18.x  
**Backend:** Rust + Axum + Tokio  
**Database access layer:** SQLx when database implementation begins  
**Authority model:** PostgreSQL is the authoritative server-side business store; client SQLite is continuity/offline state only  
**Security model:** zero trust, least privilege, fail closed, defense in depth  
**Status:** Implementation-governing specification  
**Prepared:** 7 September 2026

---

## 0. Executive Decision

This document defines the **PostgreSQL portion of Phase 2**. It does **not** replace the existing Phase 2 specification and it does **not** pull Phase 5 forward.

The required separation is:

```text
PHASE 2

configuration
  -> typed PostgreSQL settings
  -> SecretRef
  -> secret provider selection
  -> configuration validation
  -> safe diagnostics
  -> pool/timeout policy
  -> persistence capability boundary
  -> readiness/lifecycle semantics
  -> typed database errors
  -> observability hooks

PHASE 5

PostgreSQL implementation
  -> database/schema bootstrap
  -> roles/privileges
  -> migrations
  -> tables
  -> indexes
  -> constraints
  -> RLS
  -> tenant database context
  -> SQLx repositories
  -> transactions
  -> concurrency controls
  -> real PostgreSQL security/integrity tests
```

### Absolute rule

Phase 2 prepares PostgreSQL as a **secure infrastructure dependency**. Phase 5 makes PostgreSQL the concrete persistence implementation.

Do not satisfy a Phase 2 checklist by creating an incomplete schema, fake RLS policy, empty migrations, or a privileged `postgres` connection. That creates the appearance of progress while weakening the architecture.

The resulting authority chain must remain:

```text
client
  -> requests intent

API/application
  -> authenticates
  -> authorizes
  -> validates
  -> invokes persistence operation

persistence
  -> owns database infrastructure

PostgreSQL
  -> owns authoritative durable server state
```

Telemetry explains behavior. It never decides business truth.

---

# 1. Repository Baseline and Findings

The uploaded repository was inspected before this specification was written.

## 1.1 Workspace

The workspace currently contains `sitolo-config`, `sitolo-security`, `sitolo-persistence`, API/worker binaries and the other modular crates defined by the architecture.

The repository pins Rust `1.98.1` and uses edition `2024`.

## 1.2 Current persistence state

`crates/sitolo-persistence` currently contains a scaffold whose module documentation already says that it will own:

```text
SQLx/PostgreSQL implementation
repositories
transactions
mapping
```

but its `Cargo.toml` currently has no SQLx dependency and its library is intentionally skeletal.

This is acceptable at Phase 2.

It becomes a problem only if later code bypasses this boundary or if Phase 2 claims PostgreSQL implementation has already been completed.

## 1.3 Current configuration state

`AppConfig` already contains:

```text
db_host
db_port
db_name
db_user
db_password_ref
db_pool_min
db_pool_max
db_acquire_timeout_ms
```

The canonical parser namespace already includes:

```text
SITOLO__DATABASE__HOST
SITOLO__DATABASE__PORT
SITOLO__DATABASE__NAME
SITOLO__DATABASE__USER
SITOLO__DATABASE__PASSWORD_REF
SITOLO__DATABASE__POOL_MIN
SITOLO__DATABASE__POOL_MAX
SITOLO__DATABASE__ACQUIRE_TIMEOUT_MS
```

That naming must remain canonical.

## 1.4 Current secret boundary

The security crate already contains a `SecretProvider` abstraction and a development `EnvSecretProvider`.

`AppConfig` stores a `SecretRef`, not the database password. That is the correct architectural direction and must be preserved.

## 1.5 Current development-provider caveat

The development secret provider currently maps the database secret class to:

```text
SITOLO__DATABASE__PASSWORD
```

This value is acceptable only as a **development secret-provider input**. It must not be promoted into the canonical configuration contract.

Therefore the distinction must remain:

```text
CANONICAL CONFIG
SITOLO__DATABASE__PASSWORD_REF

DEVELOPMENT-ONLY SECRET INPUT
SITOLO__DATABASE__PASSWORD
```

The production runtime must not use the environment-backed secret provider.

## 1.6 Current database implementation state

The repository currently has no database migration directory and no PostgreSQL schema implementation in the persistence crate.

That is not a defect for Phase 2. It is the expected Phase 5 boundary.

---

# 2. Upstream Contracts That Override This Document

This specification must be interpreted with the existing Sitolo contracts, especially:

```text
docs/system_architecture_design.md
docs/database_design.md
docs/phase2_config_secrets_logging_errors_telemetry_implementation.md
docs/phase5_postgresql_schema_migrations_constraints_rls_implementation.md
docs/security_implementation_spec.md
docs/security_test_harness.md
docs/testing_strategy.md
docs/threat_model.md
docs/deployment_spec.md
docs/implementation_plan.md
docs/ADR-001-025.md
```

In the event of a wording difference, an existing higher-level architectural decision remains authoritative unless a superseding ADR explicitly changes it.

This file is a focused Phase 2 PostgreSQL implementation contract, not a replacement database design.

---

# 3. PostgreSQL Authority Model

PostgreSQL owns durable server-side truth for the business.

That includes, eventually:

```text
organization/tenant state
identity references and memberships
catalogue
pricing
procurement
inventory ledger and balances
sales
returns/refunds/reversals
cash
payments
reconciliation
tax/EIS state
audit evidence
idempotency records
sync command state
outbox state
background job state
reporting/read-model inputs
billing/entitlement state
```

The API process may cache information, but a cache must never become a second write authority.

The client may maintain SQLite continuity state for offline workflows, but it must not silently replace PostgreSQL as the server source of truth.

### Prohibited fallback

```rust
connect_postgres()
    .await
    .unwrap_or_else(|_| connect_sqlite_as_authority())
```

This is architecturally invalid.

---

# 4. Phase 2 Scope

## 4.1 Required

Phase 2 PostgreSQL work shall establish:

```text
typed DB configuration
configuration parsing
configuration validation
secret references
secret-provider boundary
safe configuration fingerprinting
safe logging/diagnostics
pool bounds
acquisition timeout
future timeout hooks
persistence initialization errors
DB capability lifecycle
readiness semantics
shutdown semantics
DB observability hooks
architecture guards
test coverage
local development documentation
Phase 5 handoff
```

## 4.2 Explicitly deferred

The following remain Phase 5 or later:

```text
PostgreSQL schemas
migrations
DDL
roles and GRANT statements
RLS policies
tenant context implementation
SQLx repositories
business transactions
inventory concurrency implementation
outbox tables
idempotency tables
production backups/PITR
production failover
load/capacity certification
```

---

# 5. Canonical Configuration Contract

The Phase 2 database configuration shall remain component-based rather than connection-string-based.

Canonical fields:

```text
host
port
database name
username
password reference
pool minimum
pool maximum
acquire timeout
```

The existing environment variable names are:

```text
SITOLO__DATABASE__HOST
SITOLO__DATABASE__PORT
SITOLO__DATABASE__NAME
SITOLO__DATABASE__USER
SITOLO__DATABASE__PASSWORD_REF
SITOLO__DATABASE__POOL_MIN
SITOLO__DATABASE__POOL_MAX
SITOLO__DATABASE__ACQUIRE_TIMEOUT_MS
```

Do not introduce the following as additional canonical aliases:

```text
DATABASE_URL
DB_URL
POSTGRES_URL
PG_URL
DATABASE_PASSWORD
POSTGRES_PASSWORD
```

Multiple aliases are a production drift hazard because two deployments can appear equivalent while consuming different inputs.

---

# 6. Why the Configuration Must Not Be `DATABASE_URL`

A complete PostgreSQL URL commonly embeds the credential:

```text
postgres://user:password@host:5432/sitolo
```

That makes accidental leakage easy.

The URL can end up in:

```text
Debug output
error messages
panic payloads
telemetry
CI logs
process inspection
support bundles
```

Sitolo therefore separates:

```text
non-secret connection identity
```

from:

```text
secret authority reference
```

If SQLx eventually requires a DSN/URL internally, build it in the infrastructure layer from validated configuration and a resolved secret. Do not make that assembled string the application configuration model.

---

# 7. DatabaseRuntimeConfig Boundary

Implement a narrow typed database configuration view if useful to the codebase.

Conceptual example:

```rust
pub struct DatabaseRuntimeConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password_ref: SecretRef,
    pub pool_min: u32,
    pub pool_max: u32,
    pub acquire_timeout: Duration,
}
```

It may be exposed through:

```rust
impl AppConfig {
    pub fn database_runtime(&self) -> DatabaseRuntimeConfig { ... }
}
```

or through a dedicated conversion implementation.

Do not perform network I/O when constructing this value.

The object represents **validated intent**, not a live connection.

---

# 8. Secret Separation Invariant

The most important Phase 2 PostgreSQL invariant is:

```text
AppConfig
    contains SecretRef
    does not contain password
```

Never replace:

```rust
pub db_password_ref: SecretRef
```

with:

```rust
pub db_password: String
```

for convenience.

That one change would contaminate:

```text
config debugging
fingerprinting
serialization
error handling
telemetry
application state
```

and would violate the Phase 2 secret model.

---

# 9. Secret Resolution Boundary

The credential flow must remain:

```text
AppConfig
  |
  v
SecretRef
  |
  v
SecretProvider
  |
  v
SecretValue
  |
  v
PostgreSQL connection capability
```

The persistence layer must not call:

```rust
std::env::var(...)
```

for database credentials.

It must receive a `SecretProvider` capability through dependency injection.

This preserves:

```text
development provider
staging provider
production managed provider
```

as replaceable infrastructure.

---

# 10. Development Secret Provider Rules

The environment-backed provider is acceptable only for isolated development/test workflows.

Its contract must be explicit:

```text
production = false
```

for the provider to resolve secrets.

Production must hard-fail if the selected provider is local/environment-backed.

The development provider may read a synthetic password from:

```text
SITOLO__DATABASE__PASSWORD
```

but that key must not appear in the canonical field catalogue as an ordinary configuration field.

This distinction prevents developers from accidentally creating a production deployment where the password is considered normal configuration.

---

# 11. SecretRef Semantics

The `SecretRef` must identify a secret authority location, not contain a secret.

The current reference model already enforces bounded syntax.

Required properties:

```text
non-empty
bounded length
no whitespace
no control characters
no semicolon
no unbounded arbitrary URL syntax
```

Invalid references must fail before secret resolution.

Do not pass invalid references to a production provider and hope that provider rejects them.

Defense in depth means validation occurs before the provider boundary.

---

# 12. Database Configuration Validation

Validation must occur before runtime assembly.

Required checks:

```text
host != empty
database name != empty
username != empty
port != 0
password reference exists
password reference valid
pool_min <= pool_max
pool_min <= configured hard ceiling
pool_max <= configured hard ceiling
acquire_timeout > 0
acquire_timeout <= configured hard ceiling
production cannot use local secret provider
```

The current repository already defines hard pool ceilings and acquisition timeout ceilings. These are **policy ceilings**, not recommendations for production values.

---

# 13. Bounded String Validation

Database host, database name and database user are operator-provided configuration.

They must have bounded lengths.

Do not use arbitrary huge string allocations for a configuration field.

Recommended validation properties:

```text
non-empty
bounded length
no NUL/control characters
```

Do not over-validate DNS syntax in the configuration parser if doing so would reject valid managed-provider hostnames.

Reachability is a runtime problem, not a parser problem.

---

# 14. Port Validation

The type `u16` prevents integer overflow but does not make `0` a meaningful PostgreSQL service port.

Reject:

```text
0
```

unless the deployment architecture explicitly introduces a dynamically assigned port contract, which Sitolo does not require for the PostgreSQL service.

`5432` is a normal development default but should never override an explicit production configuration silently.

---

# 15. Pool Minimum and Maximum

The repository currently uses development defaults approximately equivalent to:

```text
pool_min = 5
pool_max = 20
```

The current hard policy ceilings are much larger than these defaults.

The distinction must remain:

```text
DEFAULT != RECOMMENDED PRODUCTION CAPACITY
CEILING != RECOMMENDED PRODUCTION CAPACITY
```

Production pool sizing must be calculated from total deployment demand.

For example:

```text
4 API replicas * 25 = 100
2 worker replicas * 15 = 30
reporting = 10

aggregate requested = 140
```

This is only arithmetic illustration. Actual limits must be based on PostgreSQL capacity and workload evidence.

---

# 16. Pool Sizing Is an Aggregate Deployment Problem

A single API process may safely use a pool of `20` and still overload PostgreSQL when horizontally scaled.

Therefore the capacity calculation is:

```text
sum(pool_max of all concurrent consumers)
```

not:

```text
pool_max of one process
```

Consumers can include:

```text
API replicas
worker replicas
reporting processes
maintenance tasks
migration processes
administrative tooling
```

Migration/admin capacity should normally be temporary and must not be silently treated as part of the steady-state application pool.

---

# 17. Connection Acquisition Timeout

The existing `db_acquire_timeout_ms` is the first explicit database resource guard.

It answers:

> How long can application code wait to acquire a usable connection from the pool?

It is not the same as:

```text
TCP connection timeout
statement timeout
lock timeout
transaction timeout
```

Do not collapse all of these into one setting.

---

# 18. Future Timeout Layers

When PostgreSQL implementation begins, the database layer should distinguish:

```text
pool acquisition timeout
connection establishment timeout
statement execution timeout
lock wait timeout
transaction deadline
idle-in-transaction timeout
```

PostgreSQL 18 documents `statement_timeout`, `transaction_timeout` and `idle_in_transaction_session_timeout`, each with different purposes. [PostgreSQL 18 Client Connection Defaults](https://www.postgresql.org/docs/18/runtime-config-client.html)

This supports a more precise failure model than one universal timeout.

---

# 19. Do Not Remove Timeouts to “Improve Performance”

Removing a timeout does not make an operation faster.

It allows an operation to consume a resource for longer.

Under failure:

```text
slow DB
 -> pool wait
 -> request wait
 -> client timeout
 -> retry
 -> more DB load
```

Bounded timeouts limit the duration of that amplification.

Any timeout change must therefore be reviewed as an availability/security change.

---

# 20. PostgreSQL TLS Policy

Production connectivity must use the deployment-approved transport security model.

The baseline expectation is:

```text
private/controlled network path
+
TLS where required
+
server identity verification
+
secret-managed credentials
```

Do not add a production default that silently permits unauthenticated transport.

A future `sslmode`/TLS representation may be introduced as typed configuration, but its exact values must match the selected infrastructure provider.

Do not scatter provider-specific TLS code throughout repositories.

---

# 21. Database Target Identity

The runtime should have a safe, non-secret database identity:

```rust
pub struct DatabaseTarget {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
}
```

The application may derive a safe target fingerprint for telemetry.

Example:

```text
db.target_id=db_3fd2...
```

Do not use the raw credential-bearing URL as the target identity.

---

# 22. Database Role Identity

`DATABASE__USER` represents a database role, not a Sitolo employee.

The application database role should eventually be something like:

```text
sitolo_app_runtime
```

while migration capability is separate:

```text
sitolo_migration_owner
```

This separation is essential because PostgreSQL roles are independent security principals.

---

# 23. PostgreSQL Role Separation Handoff

Phase 2 freezes the intent for Phase 5:

```text
migration_owner
app_runtime
reporting_readonly
backup_operator
break_glass
```

`app_runtime` must not become:

```text
SUPERUSER
BYPASSRLS
CREATEROLE
CREATEDB
unrestricted schema owner
```

PostgreSQL 18 explicitly documents `SUPERUSER` and `BYPASSRLS` as privileged attributes and describes connection limits as a role-level control. [CREATE ROLE](https://www.postgresql.org/docs/18/sql-createrole.html)

---

# 24. Why Role Separation Is Required in Phase 2 Planning

If the API is later given the migration owner credential, then a compromise of the API process can become:

```text
application compromise
 -> arbitrary DDL
 -> drop/alter tables
 -> create privileged roles
 -> bypass security controls
```

With a least-privileged runtime role:

```text
application compromise
 -> narrower database capability
```

Least privilege therefore reduces the blast radius of failures and exploits.

---

# 25. RLS Handoff

Phase 2 does not implement RLS.

It must preserve the future contract:

```text
application authorization
+
least-privileged DB role
+
RLS where materially valuable
+
constraints
```

RLS is not a substitute for application authorization.

PostgreSQL 18 documents that RLS is enabled explicitly, that no applicable policy produces default-deny behavior, and that superusers/`BYPASSRLS` roles bypass RLS. [Row Security Policies](https://www.postgresql.org/docs/18/ddl-rowsecurity.html)

---

# 26. Tenant Context Must Never Come From the Client

Do not design the future RLS context as:

```text
HTTP header
 -> SET tenant_id
```

Correct flow:

```text
authenticated principal
 -> membership
 -> organization/branch scope
 -> application authorization
 -> trusted DB context
 -> RLS
```

The client request is data. It is not database security authority.

---

# 27. Connection Pool Tenant Leakage Hazard

PostgreSQL connections are pooled and reused.

A session-scoped tenant variable can therefore leak across requests if it is not reset.

Unsafe:

```text
request A
SET tenant_id = A
return connection

request B
borrow connection
tenant_id is still A
```

Phase 5 should prefer transaction-local state or another design with provable reset semantics.

`SET LOCAL` provides transaction-scoped configuration semantics, which is relevant to the future RLS tenant-context design. [SET ROLE](https://www.postgresql.org/docs/18/sql-set-role.html)

---

# 28. Phase 2 Must Not Implement Tenant RLS Context

Do not create a half-working helper such as:

```text
set_tenant_context(org_id)
```

in Phase 2 merely to prepare for RLS.

That API would freeze security semantics before identity/tenant/authorization phases are complete.

Phase 2 should instead keep the persistence lifecycle and transaction ownership flexible enough for Phase 5 to implement the final model.

---

# 29. Persistence Boundary

All future PostgreSQL access must terminate in:

```text
crates/sitolo-persistence
```

The conceptual direction is:

```text
API
 -> Application
 -> persistence port
 -> PostgreSQL adapter
 -> SQLx
 -> PostgreSQL
```

The HTTP layer must not know how PostgreSQL connections are created.

The domain layer must not know PostgreSQL exists.

---

# 30. Domain Dependency Rule

The domain crate must not import:

```text
sqlx
axum
OS environment APIs
HTTP provider clients
filesystem/network implementation details
```

If a domain type contains SQLx types, the architecture has been inverted.

Domain semantics should remain portable and testable without infrastructure.

---

# 31. Application Dependency Rule

Application services may depend on domain types and persistence ports.

They should not directly construct `PgPool`.

This makes the business operation testable independently from actual database connectivity while still allowing integration tests to exercise the real persistence adapter.

---

# 32. API Dependency Rule

`apps/api/src/main.rs` should remain an assembly point.

It may perform:

```text
load config
build provider
build persistence capability
build application services
build API state
start server
```

It should not perform:

```text
raw SQL
migration DDL
transaction business logic
secret reads from environment
PostgreSQL role management
```

---

# 33. Worker Dependency Rule

The worker binary should use the same configuration and persistence abstractions as the API where applicable.

Do not create a second database configuration parser.

The worker may have different pool/tuning values, but the configuration semantics must remain centralized.

---

# 34. SQLx Timing

Do not add SQLx to `sitolo-persistence` simply to make Phase 2 look more complete.

SQLx should enter when the repository begins the actual PostgreSQL implementation.

At that point, the expected stack is:

```text
sqlx::PgPool
sqlx::postgres::PgPoolOptions
sqlx::Transaction<Postgres>
sqlx::migrate!
```

Current SQLx documentation exposes PostgreSQL-specific pools, PostgreSQL transactions, query APIs and migration support. [SQLx docs](https://docs.rs/sqlx/latest/sqlx/)

---

# 35. Future Pool Construction Contract

When Phase 5 begins, the construction boundary should look conceptually like:

```rust
pub struct PostgresFactory;

impl PostgresFactory {
    pub async fn connect(
        config: &DatabaseRuntimeConfig,
        secrets: &dyn SecretProvider,
    ) -> Result<PostgresDatabase, PersistenceInitError> {
        // resolve secret
        // construct bounded pool
        // apply safe settings
        // validate connection capability
    }
}
```

The factory must be infrastructure-owned.

The API should consume the resulting capability rather than the credential or connection-building inputs.

---

# 36. SQLx Pool Rules

The future SQLx pool must be explicitly configured rather than relying on accidental defaults for critical resource policy.

At minimum review:

```text
max_connections
min_connections
acquire_timeout
idle_timeout
max_lifetime
connection establishment behavior
session initialization
```

SQLx documents that pool sizing must consider the database's own connection capacity and that finite connection lifetime is preferable to infinite lifetime in normal operation. [PoolOptions](https://docs.rs/sqlx/latest/sqlx/pool/struct.PoolOptions.html)

---

# 37. Why `max_lifetime` Matters

Long-lived connections may accumulate server-side session state and can interact poorly with:

```text
credential rotation
network proxies
server maintenance
session-level configuration
resource leakage
```

Do not set a tiny lifetime merely because it sounds safer; frequent churn can itself become an outage source.

Choose it from observed deployment behavior.

---

# 38. Database Session Initialization

When Phase 5 creates a PostgreSQL connection, the persistence layer may need controlled session settings such as:

```text
application_name
statement_timeout
lock_timeout
transaction/session behavior
safe search_path
```

These settings must be owned by infrastructure code.

An HTTP header must never become a PostgreSQL session setting.

---

# 39. Search Path Security Handoff

Security-sensitive SQL must not depend on an unsafe mutable `search_path`.

Phase 5 must explicitly review:

```text
schema qualification
function lookup
SECURITY DEFINER behavior
attacker-created objects
extension behavior
```

This matters particularly for RLS helpers and privileged functions.

---

# 40. Connection State and Transaction State

A database connection can hold session state and can become transaction-aborted after an error.

The persistence layer must therefore own transaction lifecycle carefully.

When SQLx is used, explicit transaction objects should be passed through the operation boundary rather than hidden inside repositories.

SQLx documents that a transaction is completed with `commit` or `rollback`; dropping an unfinished transaction rolls it back. [SQLx Transaction](https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html)

---
# 41. Transaction Boundary Handoff

Phase 2 must establish the rule now even though the complete transaction API belongs to Phase 5:

> If one business operation changes multiple authoritative records and the records must agree, the persistence operation must execute within one PostgreSQL transaction.

Typical later examples:

```text
sale finalization
 + inventory decrement
 + audit evidence
 + outbox event

payment state transition
 + payment attempt evidence
 + reconciliation state

cash close
 + final cash state
 + close evidence
```

Repositories must not silently start independent transactions that can partially commit.

The application service should own the business transaction boundary while the persistence layer owns PostgreSQL transaction mechanics.

---

# 42. No External I/O Inside an Authoritative DB Transaction

Never hold a PostgreSQL transaction open while waiting for:

```text
payment provider
MRA EIS
HTTP API
object storage
webhook callback
email/SMS service
```

Correct:

```text
BEGIN
  write durable intent/state
COMMIT

worker
  call external provider
  record outcome
```

Incorrect:

```text
BEGIN
  call payment provider
  wait 10 seconds
  update payment
COMMIT
```

The incorrect model holds a connection and transaction open across unreliable external latency and can magnify lock contention.

---

# 43. Unknown Outcome Semantics

A database error does not always mean the mutation did not happen.

The most dangerous case is:

```text
client -> PostgreSQL
PostgreSQL executes
connection dies before response
client sees timeout
```

The application must not infer:

```text
no response == no commit
```

For mutation operations, this is why Phase 5 must combine transaction semantics with idempotency and durable operation identity.

Phase 2 must preserve an explicit error category for **unknown outcome**.

---

# 44. Retry Rules

Do not implement:

```rust
for attempt in 0..3 {
    retry_everything();
}
```

Retry rules must depend on:

```text
error category
operation semantics
idempotency
whether execution began
whether the operation is externally visible
```

Examples:

```text
unique conflict -> usually no retry
check violation -> no retry
permission denied -> no retry
schema mismatch -> no retry
serialization failure -> may retry if operation is designed for it
deadlock -> may retry if operation is safe
unknown outcome -> reconcile/idempotency first
```

---

# 45. Database Error Classification Requirements

The persistence layer should eventually classify SQLx/PostgreSQL errors using structured database information, preferably SQLSTATE where available.

Avoid brittle string matching such as:

```rust
error.to_string().contains("duplicate key")
```

because server messages can vary and may contain sensitive values.

A useful internal mapping includes:

```text
unique_violation
foreign_key_violation
check_violation
serialization_failure
deadlock_detected
lock_not_available
query_canceled
connection_exception
```

The resulting internal error should carry the business-safe category, not the raw PostgreSQL message as the public response.

---

# 46. Public Error Mapping

The API must map persistence errors into the existing Sitolo API error contract.

Example:

```text
PersistenceError::ConnectionFailure
    -> ApplicationError::DependencyUnavailable
        -> HTTP 503 Problem Details
```

Example:

```text
PersistenceError::UniqueConflict
    -> ApplicationError::Conflict
        -> HTTP 409 Problem Details
```

Example:

```text
PersistenceError::UnknownOutcome
    -> ApplicationError::OperationIndeterminate
        -> explicit retry/reconciliation guidance
```

Do not send the PostgreSQL SQLSTATE, hostname, role, relation name or full server message to ordinary clients unless the API contract explicitly requires a safe subset.

---

# 47. Database Error Observability

Internal telemetry should preserve enough detail for incident response:

```text
operation
error_category
retryability
unknown_outcome
request_id
trace_id
service_version
database_target_id
```

Do not attach:

```text
password
connection URL
arbitrary query parameters
customer payload
full payment payload
```

Raw low-level errors may be available in protected logs only after redaction and according to the logging policy.

---

# 48. Pool Saturation and Retry Amplification

The system must recognize this failure loop:

```text
PostgreSQL latency increases
        |
        v
pool connections stay busy
        |
        v
acquisition waits increase
        |
        v
requests time out
        |
        v
clients retry
        |
        v
more requests enter the pool
        |
        v
PostgreSQL latency increases again
```

Pool sizing, acquisition timeout, application deadlines and retry policy must be considered as one system.

Do not solve pool saturation by automatically raising `pool_max`.

---

# 49. Database Pool Metrics

The observability layer should make at least these quantities available:

```text
configured maximum
configured minimum
current in-use
idle/available
acquisition wait duration
acquisition timeout count
connection establishment failures
```

Useful bounded metrics include:

```text
sitolo_db_pool_wait_seconds
sitolo_db_pool_timeout_total
sitolo_db_connection_failure_total
```

Avoid labels containing:

```text
organization_id
sale_id
customer_id
request_id
raw SQL
```

---

# 50. Database Query Metrics

Query timing should use logical operation names:

```text
sale_finalize
inventory_read
inventory_mutate
membership_lookup
sync_command_apply
payment_lookup
```

Do not use the SQL statement itself as the metric label.

Metric cardinality must remain bounded.

---

# 51. Database Tracing

A database span may contain:

```text
db.operation=sale_finalize
db.target_id=db_xxx
db.system=postgresql
```

It should not contain arbitrary query arguments.

A trace is not a secure database audit trail.

The trace should answer:

```text
which logical operation ran?
how long did it take?
succeeded or failed?
why did it fail?
```

It should not become a copy of the database contents.

---

# 52. Audit Versus Telemetry

Never confuse:

```text
tracing
metrics
logs
```

with:

```text
authoritative audit records
```

For example:

```text
log: sale finalized
```

does not prove that the sale transaction committed.

A durable audit record should be associated with the authoritative operation according to the database design.

---

# 53. Configuration Fingerprint Requirements

The existing configuration fingerprinting model should remain deterministic and non-secret.

Database changes that legitimately affect the fingerprint include:

```text
host
port
database
role
pool limits
acquisition timeout
secret reference
```

A secret value change alone must not change the fingerprint.

The fingerprint must never be derived from:

```text
raw password
```

or another credential-bearing string.

---

# 54. Secret Rotation and Fingerprints

Consider:

```text
old secret value = A
new secret value = B
same SecretRef
```

Expected:

```text
config fingerprint remains unchanged
```

The deployment may still emit a security event:

```text
secret rotation completed
class=database
```

without storing either secret.

If the reference itself changes:

```text
old ref != new ref
```

the fingerprint may change because the effective configuration changed.

---

# 55. Database Secret Leak Regression Tests

Use a unique synthetic test secret such as:

```text
TEST_ONLY_DATABASE_SECRET_001
```

Exercise:

```text
configuration loading
secret resolution
initialization failure
logging
health output
error serialization
telemetry generation
```

Then assert the sentinel does not appear in:

```text
stdout
stderr
JSON response
structured log
metric label
trace attribute
configuration fingerprint
```

This should be a release-blocking security test once the infrastructure is wired.

---

# 56. AppConfig Debug Safety

The current configuration model is comparatively safe because it contains:

```text
SecretRef
```

rather than:

```text
SecretValue
```

Preserve this property.

It should be possible to run:

```rust
format!("{config:?}")
```

without exposing a database password.

However, do not assume every future runtime database object will be safe to `Debug` merely because `AppConfig` is safe.

Credential-bearing runtime capability types should be reviewed separately.

---

# 57. Health/Readiness Response Safety

Never expose:

```text
DATABASE_URL
DATABASE_PASSWORD
PostgreSQL connection string
raw authentication failure
internal host inventory
SQL statements
stack traces
```

A safe failure response is closer to:

```json
{
  "status": "not_ready",
  "dependency": "database",
  "reason": "connection_unavailable"
}
```

The exact response remains governed by the API health contract.

---

# 58. Startup Readiness States

Use explicit internal state such as:

```text
Starting
Ready
Degraded
NotReady
ShuttingDown
```

Database-specific states may include:

```text
Unconfigured
Configured
Connecting
Ready
Unavailable
Incompatible
```

Do not mark the application `Ready` merely because a `PgPool` object exists.

A pool object can be constructed while the underlying database is unavailable until the first connection is attempted.

---

# 59. Database Capability Verification

When real PostgreSQL is introduced, decide explicitly whether startup should perform:

```text
connection establishment
lightweight health query
schema compatibility check
```

Do not perform a heavy application query merely to prove the database exists.

The health contract must be cheap enough to execute safely during startup and, if used for readiness, during repeated probes.

---

# 60. Schema Readiness Is Later Than Connectivity

These are different states:

```text
network reachable
credentials accepted
role exists
schema compatible
application ready
```

Phase 2 can model the distinction conceptually.

Phase 5 becomes responsible for actual schema compatibility checks once migrations exist.

Do not fake migration state in Phase 2.

---

# 61. Production Database Endpoint Policy

The production endpoint must be operator-controlled infrastructure.

The application must not accept merchant/user input as the database host.

The client never chooses PostgreSQL connectivity.

The correct topology is:

```text
Mobile/Desktop client
       |
       v
HTTPS API
       |
       v
private database network
       |
       v
PostgreSQL
```

Not:

```text
Mobile client
       |
       v
PostgreSQL
```

---

# 62. PostgreSQL Must Not Be Publicly Exposed as an Application Requirement

A public PostgreSQL endpoint materially increases the attack surface.

The deployment should instead use:

```text
private network
firewall/security group
TLS
least privilege
secret manager
```

Provider-specific networking details are deferred to deployment infrastructure, but the application architecture should never require a public database endpoint.

---

# 63. Database Role Blast Radius

A compromised application process should not automatically be able to:

```text
create roles
change passwords
change schema
create arbitrary extensions
remove audit tables
bypass RLS
```

This is why the runtime database role must be narrower than the migration/admin role.

---

# 64. Migration Role Boundary

The migration owner may perform controlled DDL.

It must not be the API's normal runtime identity.

Later CI should explicitly validate both:

```text
migration role has required migration privileges
runtime role lacks unnecessary migration privileges
```

This is more meaningful than merely inspecting migration SQL.

---

# 65. Reporting Role Boundary

Reporting workloads should be read-only where possible.

A reporting credential should not automatically have the same write privileges as the API.

Potential architecture:

```text
API -> app_runtime
reporting -> reporting_readonly
migration -> migration_owner
```

The exact physical deployment may evolve later.

---

# 66. Break-Glass Role Boundary

Emergency administrative access must be separate.

A break-glass role should be:

```text
rarely used
strongly authenticated
separately audited
operationally controlled
not embedded in normal application configuration
```

The API must never automatically fall back to break-glass access.

---

# 67. PostgreSQL RLS Security Facts

PostgreSQL 18 documents that RLS must be enabled on a table before its policies apply, that an enabled table with no applicable policy behaves as default-deny, and that superusers and roles with `BYPASSRLS` bypass RLS. [PostgreSQL 18 Row Security Policies](https://www.postgresql.org/docs/18/ddl-rowsecurity.html)

Therefore a future Sitolo RLS design must never use an application role that can simply bypass the policies it is supposed to enforce.

---

# 68. `USING` Versus `WITH CHECK` Handoff

Phase 5 must distinguish:

```text
USING
= which existing rows a role may access

WITH CHECK
= which new/updated rows may be produced
```

Read isolation without write isolation is incomplete.

A tenant policy must not allow:

```text
tenant A can read only A
but can insert/update rows belonging to B
```

This is specifically why actual RLS testing is required.

---

# 69. RLS Is Defense in Depth

The primary decision chain remains:

```text
authentication
 -> session/device trust
 -> tenant membership
 -> authorization
 -> domain rules
 -> persistence
```

RLS adds a database-side defense against accidental cross-tenant access.

It does not replace the application authorization engine.

---

# 70. Real PostgreSQL Requirement

Security claims involving PostgreSQL must be proven against real PostgreSQL.

This applies to:

```text
RLS
role privileges
UNIQUE races
foreign keys
transaction isolation
locking
statement timeout
session state
```

A fake repository can validate control flow but cannot prove database semantics.

---

# 71. SQLite Restriction

The client-side SQLite implementation must remain distinct from server PostgreSQL.

Do not add:

```text
server SQLite mode
SQLite fallback when PostgreSQL fails
shared repository that silently swaps engines
```

SQLite exists for client continuity and offline operation.

PostgreSQL exists for authoritative server state.

---

# 72. Local PostgreSQL Development Standard

The repository should provide one documented local path, preferably containerized PostgreSQL 18.

Conceptual environment:

```text
postgres image 18.x
port 5432 bound to localhost only
database sitolo
runtime role sitolo_app_runtime
migration role sitolo_migration_owner
```

Use synthetic credentials.

Do not publish the database port to all interfaces unless the developer explicitly needs remote access.

---

# 73. Local Database Bootstrap

The future local bootstrap should approximately perform:

```text
create database
create migration role
create runtime role
set role passwords from local secret input
apply migrations
verify privileges
```

Use a superuser only for controlled bootstrap operations.

After bootstrap, application tests must connect as the runtime role.

---

# 74. Local Role Verification

A healthy local environment should be able to prove:

```text
runtime role works
migration role works
runtime cannot perform migration-only operations
```

This prevents local development from hiding privilege problems that would fail in production.

---

# 75. Local Secret Input

A developer may configure:

```text
SITOLO__DATABASE__PASSWORD
```

for the environment-backed development secret provider.

That value must:

```text
not be committed
not appear in .env.example
not be printed
not be reused in production
```

The canonical configuration remains `PASSWORD_REF`.

---

# 76. `.env.example` Contract

A safe example should contain no credential:

```text
SITOLO__RUNTIME__ENVIRONMENT=development
SITOLO__DATABASE__HOST=localhost
SITOLO__DATABASE__PORT=5432
SITOLO__DATABASE__NAME=sitolo
SITOLO__DATABASE__USER=sitolo_app_runtime
SITOLO__DATABASE__PASSWORD_REF=development/sitolo/db
SITOLO__DATABASE__POOL_MIN=5
SITOLO__DATABASE__POOL_MAX=20
SITOLO__DATABASE__ACQUIRE_TIMEOUT_MS=2000
```

Do not include:

```text
SITOLO__DATABASE__PASSWORD=...
```

in a committed example file.

---

# 77. Docker Compose Boundary

If a Compose file is introduced as part of the DB implementation work, it must be clearly identified as development/test infrastructure.

Suggested constraints:

```text
PostgreSQL 18.x
persistent local volume
localhost-only host port binding
healthcheck
no production credentials
```

The Compose service must not be presented as the production deployment architecture.

---

# 78. CI PostgreSQL Service Handoff

Once Phase 5/7 begins, CI should create real PostgreSQL 18 for tests.

The CI pipeline should roughly be:

```text
checkout
  |
  v
Rust 1.98.1
  |
  v
PostgreSQL 18.x
  |
  +--> bootstrap roles
  +--> run migrations
  +--> run persistence tests
  +--> run RLS tests
  +--> run concurrency tests
  |
  v
security/integrity evidence
```

If PostgreSQL cannot start, the required database test job must fail rather than silently skip database tests.

---

# 79. SQLx Compile-Time Checking Strategy

When SQLx is introduced, the repository must document how compile-time checking is performed.

Possible models are:

```text
DATABASE_URL + controlled schema
SQLx offline metadata
CI PostgreSQL service
```

The exact model should be selected once the Phase 5 migration workflow is implemented.

Do not create a hidden developer-only dependency on a personal PostgreSQL server.

---

# 80. SQLx Migration Source Control

When migration files exist, they must be version-controlled.

SQLx's `migrate!()` embeds migrations into the binary and documents migration-file tracking considerations under Cargo builds. [SQLx migrate!](https://docs.rs/sqlx/latest/sqlx/macro.migrate.html)

If the repository uses embedded migrations, changes to the migration directory must be visible to the build system.

The final implementation must document this explicitly.

---

# 81. Migration Execution Policy

Do not assume every API replica should automatically migrate production.

A controlled deployment may use:

```text
migration job/stage
    -> migration_owner
    -> verify

API replicas
    -> app_runtime
```

This reduces migration privilege exposure and avoids races between multiple application instances.

The final decision belongs to deployment architecture.

---

# 82. Schema Drift Handoff

A database can be reachable but wrong for the application version.

Example:

```text
application expects schema 42
actual schema 41
```

Phase 5 should expose migration/schema state separately from simple TCP/database reachability.

The Phase 2 readiness abstraction must be able to represent that future distinction.

---

# 83. Database Connectivity Is Not Schema Compatibility

Do not implement a health check that returns:

```text
healthy=true
```

because `SELECT 1` succeeded while required application tables are missing.

Once the migration system exists, schema compatibility must be verified according to the migration contract.

---

# 84. Database Application Name

Future SQLx connections should identify their workload through PostgreSQL `application_name` where useful:

```text
sitolo-api
sitolo-worker
sitolo-migration
```

This makes server-side investigation easier through PostgreSQL activity/lock tooling.

The value must be controlled by trusted service configuration.

It must not come from:

```text
HTTP Host
User-Agent
request header
merchant input
```

---

# 85. Connection Role Verification

The Phase 5 test harness should verify the real session identity:

```sql
SELECT current_user;
```

and compare it with the expected runtime role.

The result can be used for test evidence but should not be exposed through public health endpoints.

---

# 86. Connection State Reset

Pooled PostgreSQL connections can retain session state.

Future code must carefully handle:

```text
SET ROLE
SET search_path
SET LOCAL tenant context
session timeouts
application_name
transaction state
```

Transaction-local state is preferable for tenant security context when appropriate.

The Phase 2 persistence boundary must not force future RLS code into unsafe global session state.

---

# 87. Connection Pool and Tenant Context Test

Phase 5 must execute a sequence such as:

```text
request A -> tenant A
return connection
request B -> tenant B
reuse same physical connection if possible
```

and prove:

```text
B never observes A's context
```

This must be tested under actual connection pooling.

---

# 88. Session-Leak Threat Model

Potentially dangerous session state includes:

```text
tenant identifier
role
search_path
transaction state
statement timeout
application name
custom configuration parameters
```

A reused connection is shared infrastructure.

Anything that must not cross requests must be transaction-local or reliably reset.

---

# 89. Search Path and Privileged Functions

If Phase 5 introduces `SECURITY DEFINER` functions, safe `search_path` handling becomes critical.

The function must not resolve attacker-controlled objects unexpectedly.

The final database implementation should use explicit schema qualification where security-sensitive.

---

# 90. Extension Privilege Handoff

Do not grant `app_runtime` arbitrary extension-creation privileges.

Extensions should be approved through an architecture/security process and supported by the selected PostgreSQL environment.

The runtime account must remain narrow.

---

# 91. Production Secret Provider

The exact provider is intentionally not frozen here.

Acceptable patterns include:

```text
managed cloud secret manager
Vault-like secret service
orchestrator-managed secret integration
OS-backed credential provider
```

The application's dependency remains:

```text
SecretProvider
```

This avoids hard-coding vendor APIs into the persistence layer.

---

# 92. Secret Provider Capability Routing

Secret classes remain separate:

```text
Database
Identity
Payment
Webhook
MraTerminal
Telemetry
SigningKey
ObjectStorage
Service
```

Do not create one giant secret called:

```text
INFRASTRUCTURE_CREDENTIALS
```

Database access should require `SecretClass::Database`.

---

# 93. Database Secret Resolution Failure Matrix

| Failure | Internal result | Startup/runtime behavior |
|---|---|---|
| missing secret | `Missing` | fail initialization |
| provider unavailable | `Unavailable` | fail required capability |
| provider denied | `ProviderAuthorizationDenied` | fail closed |
| invalid secret | `Invalid` | fail capability |
| expired secret | `Expired` | fail/rotate according to ops policy |
| local provider in production | `LocalProviderForbidden` | hard fail |

Never replace these failures with a default credential.

---

# 94. Database Authentication Failure

A PostgreSQL authentication failure may result from:

```text
wrong password
expired credential
wrong role
wrong database
rotation race
provider misconfiguration
```

The internal error category should preserve enough information for investigation while the public API returns a generic dependency failure.

Do not reveal the database role/password details to customers.

---

# 95. Database TLS Failure

A TLS error must not be logged as a raw connection URL.

Safe metadata:

```text
error_category=tls_failure
database_target_id=db_...
```

Potentially sensitive values such as certificates, private keys or full provider URLs must stay out of normal logs.

---

# 96. DNS/Network Failure

Network failures should be classified separately from authentication failures where practical.

For example:

```text
DNS failure
connection refused
connection timeout
TLS failure
authentication failure
```

This makes operations more efficient and avoids unnecessary credential rotations during network incidents.

---

# 97. Database Permission Failure

A permission error should be treated as an infrastructure/deployment defect, not silently retried forever.

Potential response:

```text
internal: PermissionDenied
external: 503 or controlled server error
```

The final mapping depends on whether the failure is a deploy-time capability defect or an operation-specific authorization outcome.

---

# 98. Constraint Violation Versus Infrastructure Failure

A `UNIQUE` violation generally means:

```text
application/business conflict
```

It is not the same class as:

```text
PostgreSQL unavailable
```

This distinction matters because a conflict should not trigger:

```text
retry storm
health failure
database restart
```

---

# 99. Database Timeouts Versus Business Rejections

A statement timeout indicates:

```text
infrastructure/query-duration failure
```

An insufficient-stock rejection indicates:

```text
expected business state rejection
```

They must remain separate throughout:

```text
persistence
application
API
telemetry
```

---

# 100. Database Cancellation

When a request is cancelled:

```text
client disconnect
request deadline
server shutdown
```

the persistence layer must not accidentally continue a mutation in a detached task unless the operation is explicitly durable/asynchronous.

A financial mutation must either:

```text
complete under the designed transaction
or
rollback / become safely recoverable
```

not enter an undocumented third state.

---

# 101. Shutdown Contract

The process shutdown sequence should be:

```text
stop receiving new work
   |
   v
stop worker claims
   |
   v
drain/cancel in-flight work
   |
   v
close DB capability
   |
   v
flush bounded telemetry
   |
   v
exit before hard deadline
```

Do not wait forever for PostgreSQL or telemetry.

---

# 102. Database Shutdown Metrics

Useful events include:

```text
database.shutdown.started
database.shutdown.completed
database.shutdown.timeout
```

Do not block termination simply to guarantee telemetry export.

---

# 103. Database Backpressure Priority

When resources are constrained, prioritize:

```text
authoritative business persistence
```

over:

```text
optional telemetry
large reports
non-critical exports
```

The system should reduce load before dropping authoritative correctness.

---

# 104. Configuration Poisoning

Configuration must be treated as untrusted deployment input.

A malicious or invalid configuration might attempt:

```text
DB host -> attacker endpoint
pool_max -> extreme value
acquire timeout -> huge value
secret provider -> local development source
TLS -> disabled
```

Validation and deployment policy must prevent these states.

---

# 105. Hostname Safety

Database host strings should be bounded and reject obvious control characters.

Do not attempt to encode full network security policy in a generic string parser.

The deployment network boundary should additionally control:

```text
where the process can connect
which database endpoint is allowed
```

---

# 106. Database Configuration Provenance

A configuration diagnostic system should be able to identify a field's source:

```text
compiled default
environment/deployment configuration
secret provider
```

without revealing the value where it is sensitive.

This helps diagnose:

```text
unexpected environment override
wrong deployment manifest
secret reference drift
```

---

# 107. Safe Configuration Snapshot

A privileged diagnostic may show:

```text
DATABASE__HOST -> source=environment -> configured
DATABASE__PORT -> source=environment -> 5432
DATABASE__NAME -> source=environment -> sitolo
DATABASE__USER -> source=environment -> sitolo_app_runtime
DATABASE__PASSWORD_REF -> source=environment -> ref_...
DATABASE__POOL_MIN -> source=default -> 5
DATABASE__POOL_MAX -> source=default -> 20
```

It must not show the password.

---

# 108. Database Configuration API Tests

The test suite should verify deterministic behavior for equivalent input orderings.

For example:

```text
pairs A,B,C
pairs C,A,B
```

should produce the same effective configuration if no duplicate-key conflict exists.

The current use of a deterministic map in parsing is useful for this property.

---

# 109. Duplicate Environment Key Policy

If duplicate keys can enter the parser input, define deterministic semantics.

Do not let process environment ordering decide which database host wins.

Recommended behavior is either:

```text
last source wins according to an explicit ordered layer
```

or:

```text
duplicate key is rejected
```

The rule must be documented and tested.

---

# 110. Unknown Configuration Key Policy

The canonical namespace should reject unknown keys.

Example typo:

```text
SITOLO__DATABASE__POLL_MAX
```

must not silently result in:

```text
pool_max = default
```

because that can produce unexpected production capacity.

---

# 111. Unit Clarity

Use explicit unit-bearing names:

```text
ACQUIRE_TIMEOUT_MS
```

rather than:

```text
TIMEOUT=2000
```

The validator must know the units.

Future settings should follow the same convention.

---

# 112. Database Configuration Schema Evolution

Adding a database configuration field is a schema change to the process configuration contract.

The change should update:

```text
model
parser
validator
field catalogue
fingerprint
redaction/diagnostics
tests
documentation
```

Do not add a parser field and forget validation.

---

# 113. Configuration Schema Version

The existing `config_schema_version` remains the process configuration compatibility control.

A change that materially changes the configuration contract should consider:

```text
schema version bump
compatibility handling
migration notes
```

Do not use the version field merely as decorative metadata.

---

# 114. Database Pool Ceiling Changes

Raising the hard ceiling is a high-risk change.

Before doing so, document:

```text
why existing ceiling is insufficient
expected maximum replicas
aggregate DB connections
memory impact
failure behavior
load test evidence
```

Never increase the ceiling just because one developer's workload hit it locally.

---

# 115. Timeout Ceiling Changes

Raising timeout ceilings can worsen outage recovery because resources remain occupied longer.

Document:

```text
query/operation workload
DB capacity
lock behavior
retry interaction
observed latency
```

before changing safety ceilings.

---

# 116. Database Connection Budget Model

The deployment should calculate:

```text
API replicas * API pool_max
+
worker replicas * worker pool_max
+
reporting consumers
+
maintenance overhead
<
PostgreSQL safe capacity
```

The value should leave operational headroom.

Do not plan to consume the database's maximum connection capacity as normal steady state.

---

# 117. Why More Connections Can Be Slower

Increasing connections can increase:

```text
context switching
memory use
lock contention
query concurrency
cache contention
```

The database is not a linear throughput machine where doubling connections doubles performance.

Benchmark before increasing pool capacity.

---

# 118. API Versus Worker Resource Classes

API workloads generally prioritize:

```text
latency
bounded per-request work
```

Workers generally prioritize:

```text
throughput
controlled concurrency
```

Therefore their pools may eventually need different values.

The common configuration contract should support this without duplicate parsers.

---

# 119. Reporting Workload Isolation

Reporting can create long-running scans and aggregations.

If measured impact becomes significant, options include:

```text
separate pool
read model
read replica
materialized view
warehouse/export path
```

Do not choose one solely because it sounds “enterprise.”

Measure first.

---

# 120. Phase 2 Does Not Implement Read Replicas

No read replica should be introduced merely to satisfy this document.

The authority model is:

```text
PostgreSQL authoritative state
```

A replica, when later introduced, remains derived/read-only from the application's perspective.

It must never become a competing write authority.

---

# 121. Database Caching Boundary

A future cache may accelerate reads.

It must not become the only copy of a mutable business state that the application relies on for correctness.

For example:

```text
inventory cache
```

cannot authorize a sale if PostgreSQL is unavailable.

The authoritative stock decision remains database-backed.

---

# 122. Financial Truth Boundary

Do not store financial truth solely in:

```text
Redis
cache
metrics
logs
client SQLite
```

The authoritative financial state belongs in PostgreSQL according to the database design.

Phase 2 configuration must not create a fallback that changes this rule.

---

# 123. Inventory Authority Boundary

The future inventory implementation will require atomic database operations.

Phase 2 must not introduce a configuration option such as:

```text
inventory.use_local_cache=true
```

that would let the API bypass the authoritative inventory path.

---

# 124. Audit Integrity Boundary

Audit evidence should be durable and attributable.

A telemetry outage must not delete or redefine the database audit contract.

A PostgreSQL outage means authoritative audit writes may be unavailable; they must not be replaced with a best-effort log and declared successful.

---

# 125. Database and Outbox Handoff

The future outbox pattern should allow:

```text
business state change
+
outbox record
```

to commit atomically.

The external worker can then deliver the side effect.

Phase 2 only needs to ensure the persistence boundary can later participate in explicit transactions and that observability can connect the original operation to the worker.

---

# 126. Database and Idempotency Handoff

The future idempotency design should use durable uniqueness at the database layer for critical mutations.

Do not use:

```text
process-local HashMap
```

as the authoritative duplicate detector for a horizontally scaled system.

---

# 127. Database and Webhook Handoff

Provider callbacks can arrive:

```text
once
multiple times
out of order
after retries
after process restart
```

The final PostgreSQL design must persist provider event identity and deduplicate atomically.

Phase 2 provides the error/telemetry boundary for those later operations.

---

# 128. Database and EIS Handoff

MRA/EIS workflows may have ambiguous external states.

The database should later distinguish:

```text
intent
attempt
acknowledgement
reconciliation
```

The application must not consider a telemetry span an authoritative EIS submission record.

---

# 129. Data Retention Handoff

Phase 2 does not choose retention periods for business/audit data.

It must, however, ensure configuration does not turn database cleanup into an arbitrary runtime environment toggle that can destroy authoritative evidence.

Retention belongs to governed database/domain/operations policy.

---

# 130. Database Cleanup Handoff

Later cleanup workers must use:

```text
bounded batches
indexed predicates
short transactions
observable progress
retry-safe behavior
```

They must not compete with transactional workloads without resource controls.

---

# 131. Migration Lock Safety

Schema migrations can require locks that affect live traffic.

Phase 5 migrations must document:

```text
lock type
expected duration
affected tables
backfill strategy
rollback/recovery
```

Phase 2's timeout/telemetry foundation should make these failures visible.

---

# 132. Expand-and-Contract Handoff

Future incompatible schema changes should generally use:

```text
expand
 -> deploy compatible code
 -> backfill
 -> migrate reads/writes
 -> contract
```

Avoid requiring an application rollback that depends on a schema immediately removed by the new version.

This is part of the Phase 5 migration contract.

---

# 133. Database Deployment Compatibility

A code rollback should remain compatible with the deployed database during the documented rollback window.

Do not design migration deployment so:

```text
new binary + migration
```

works but:

```text
previous verified binary + migrated DB
```

fails immediately.

---

# 134. Database Version Policy

The Sitolo baseline is PostgreSQL 18.x.

PostgreSQL's official 18 documentation lists 18 as the current major release line and documents later 18.x maintenance releases. [PostgreSQL 18 release notes](https://www.postgresql.org/docs/18/release.html)

The repository should keep major-version expectations explicit across:

```text
development
CI
migration testing
production
```

---

# 135. PostgreSQL 18 Feature Restraint

PostgreSQL 18 includes features such as asynchronous I/O improvements and UUIDv7 support. [PostgreSQL 18 release notes](https://www.postgresql.org/docs/18/release-18.html)

These capabilities do not justify introducing complexity without evidence.

Use PostgreSQL capabilities when they materially improve Sitolo's correctness, operability or performance and the implications are understood.

---

# 136. UUIDv7 and Sitolo IDs

Do not move all identifier generation into PostgreSQL simply because PostgreSQL 18 provides `uuidv7()`.

Offline operation can require client-generated identifiers.

The exact distribution of identifier authority must be decided with the domain and sync contracts in Phase 5.

---

# 137. PostgreSQL 18 AIO Does Not Remove Resource Limits

Server-side I/O improvements can improve certain workloads, but application-level limits remain required:

```text
bounded pool
bounded concurrency
query timeouts
pagination
```

Do not interpret server performance improvements as permission to remove resource controls.

---

# 138. Database Query Limits Handoff

Phase 5 should define query result limits for API operations.

For example:

```text
ordinary list endpoint -> bounded page
report export -> job-based
large history -> cursor/pagination
```

Do not allow a normal HTTP request to materialize an unbounded table into memory.

---

# 139. Database Memory Boundary

Even with PostgreSQL doing most data processing, Rust code can still allocate excessively when mapping rows.

Repositories should therefore use:

```text
bounded result sets
streaming where justified
pagination
appropriate projection columns
```

Do not fetch entire rows when only a few fields are needed.

---

# 140. N+1 Query Handoff

Repository design should guard against accidental N+1 database access.

A page of:

```text
100 products
```

should not automatically trigger:

```text
1 query + 100 additional queries
```

unless the workload is intentionally designed and measured.

---

# 141. Query Plan Handoff

Phase 5 performance review must inspect query plans for critical paths.

The configuration layer only supplies resource/time budgets.

Query correctness and indexes belong to the database implementation.

---

# 142. Transaction Isolation Handoff

Do not globally select the strongest possible isolation level merely to sound secure.

The existing database contract prefers:

```text
appropriate isolation per workflow
```

Default `READ COMMITTED` may be sufficient for many operations.

Stronger isolation should be used where the business invariant requires it and tested under concurrency.

---

# 143. Deadlock Prevention Handoff

Phase 5 should keep lock order deterministic and transactions short.

A deadlock is not solved by simply increasing timeouts.

The final implementation should:

```text
classify
observe
reduce lock scope
standardize ordering
retry only where safe
```

---

# 144. Configuration Fuzzing

Fuzz or property-test:

```text
DB host
DB name
DB user
DB port strings
pool values
timeout values
secret references
unknown keys
```

Assert:

```text
no panic
bounded error
no credential echo
no accepted invalid configuration
```

---

# 145. Secret Reference Fuzzing

Generate invalid inputs containing:

```text
spaces
newlines
NUL/control bytes
semicolons
very long values
unexpected punctuation
```

The `SecretRef` constructor should reject them without unbounded behavior.

---

# 146. Error Serialization Fuzzing

Database-related errors should not be able to produce a serialized response containing:

```text
password
connection URL
internal host inventory
stack trace
raw SQL
```

Fuzz error payloads and mapping code where practical.

---

# 147. Architecture Check — Domain

A CI check should fail if a domain crate introduces a dependency on:

```text
sqlx
postgres client
filesystem/network runtime
OS environment
```

unless a future ADR explicitly changes the architecture.

---

# 148. Architecture Check — API

A CI check should detect direct SQL execution under:

```text
apps/api
crates/sitolo-api
```

The persistence boundary should remain the only SQLx owner.

A coarse grep is useful but should supplement, not replace, dependency graph checks and code review.

---

# 149. Architecture Check — Credentials

CI should detect obvious prohibited patterns:

```text
DATABASE_URL in source
DATABASE_PASSWORD as AppConfig field
postgres://... password ...
password in tracing fields
password in println!/debug
```

False positives must be handled without disabling the security check globally.

---

# 150. Test Environment Fail-Closed Rule

A database integration test must never report success because PostgreSQL was unavailable.

Wrong:

```text
try PostgreSQL
if unavailable -> skip
exit 0
```

Correct:

```text
PostgreSQL required
if unavailable -> test failure
```

unless the test itself is explicitly categorized as a non-DB unit test.

---

# 151. Test Categorization

Separate:

```text
unit tests
configuration tests
architecture tests
PostgreSQL integration tests
security integration tests
concurrency tests
```

A clean unit test run must not be represented as proof that PostgreSQL RLS works.

---

# 152. Evidence Quality

Every database security claim should identify:

```text
source revision
toolchain
PostgreSQL version if applicable
command/test suite
result
```

This makes security evidence reproducible.

---

# 153. What the Phase 2 Engineer Must Not Claim

Do not claim:

```text
RLS is secure
schema is production-ready
migrations are safe
inventory race is solved
backup is working
restore is validated
```

unless the relevant Phase 5/operations evidence exists.

---

# 154. File-by-File Implementation Guidance

## `crates/sitolo-config/src/model.rs`

Maintain the existing typed database fields. Add a narrow database configuration view only if it reduces coupling without creating a second source of truth.

If a view is added:

```text
copy validated fields
carry SecretRef
carry typed timeout/duration
carry bounded pool settings
```

Do not resolve the secret here.

## `crates/sitolo-config/src/parse.rs`

Keep canonical `SITOLO__DATABASE__*` parsing. Reject unknown fields. Do not add `DATABASE_URL`.

## `crates/sitolo-config/src/validate.rs`

Add/strengthen database-specific validation. Keep failures deterministic and aggregate validation problems rather than panicking.

## `crates/sitolo-config/src/fingerprint.rs`

Ensure all database configuration fields that define effective behavior are represented, while secret values remain excluded.

## `crates/sitolo-config/src/field.rs`

Keep the field catalogue synchronized with parser/model/validation.

## `crates/sitolo-security/src/provider.rs`

Keep environment-backed resolution explicitly development-only. Do not let it become a production provider through an accidental default.

## `crates/sitolo-persistence/src/lib.rs`

Expose the infrastructure-owned persistence boundary and initialization/error types without prematurely implementing all Phase 5 repositories.

## `apps/api/src/main.rs`

Only wire the common runtime assembly when the Phase 2 runtime is implemented. Do not place SQL or secret resolution logic directly in `main`.

## `apps/worker/src/main.rs`

Consume the same configuration/secret/persistence contract rather than duplicating environment parsing.

---

# 155. Suggested Persistence Modules

A reasonable Phase 2 module structure is:

```text
crates/sitolo-persistence/src/
    lib.rs
    config.rs
    error.rs
```

Optional later modules:

```text
    postgres.rs
    transaction.rs
    health.rs
    repositories/
    queries/
```

Do not create empty modules for every future concept.

Create a module when there is actual ownership and behavior.

---

# 156. Suggested Error Module Shape

Conceptual:

```rust
#[derive(Debug, thiserror::Error)]
pub enum PersistenceInitError {
    #[error("invalid database configuration")]
    Configuration(#[source] DatabaseConfigurationError),

    #[error("database secret resolution failed")]
    Secret(#[source] SecretError),

    #[error("database connection capability unavailable")]
    Connection(#[source] DatabaseConnectionError),
}
```

The exact categories may be adapted to existing error architecture.

The critical properties are:

```text
typed
non-secret
stable internally
mappable externally
```

---

# 157. Suggested Database Target Type

Conceptual:

```rust
#[derive(Clone)]
pub struct DatabaseTarget {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
}
```

Prefer implementing a custom redacted diagnostic representation rather than serializing arbitrary values by default.

---

# 158. Suggested Runtime Config Construction

Conceptual:

```rust
impl AppConfig {
    pub fn database_runtime(&self) -> DatabaseRuntimeConfig {
        DatabaseRuntimeConfig {
            host: self.db_host.clone(),
            port: self.db_port,
            database: self.db_name.clone(),
            username: self.db_user.clone(),
            password_ref: self.db_password_ref.clone(),
            pool_min: self.db_pool_min,
            pool_max: self.db_pool_max,
            acquire_timeout: Duration::from_millis(self.db_acquire_timeout_ms),
        }
    }
}
```

This is illustrative; adapt naming and module boundaries to the codebase.

---

# 159. Suggested Runtime Initialization Sequence

The future runtime should do:

```text
load config
  |
validate config
  |
construct secret provider
  |
resolve DB SecretRef
  |
construct DB configuration/capability
  |
perform required DB capability check
  |
register DB with application state
  |
mark readiness according to policy
```

The secret should not be resolved before configuration validation proves which secret is required.

---

# 160. Database Provider Injection

Conceptual:

```rust
pub struct RuntimeDependencies {
    pub secrets: Arc<dyn SecretProvider>,
    pub database: DatabaseCapability,
}
```

Keep the credential itself out of broad application state.

The database capability is the thing application code needs; the raw password is not.

---

# 161. Credential Non-Propagation Rule

Once a database capability has been created, ordinary application services should not need access to:

```text
database password
SecretProvider
connection string
```

unless they explicitly own secret-management responsibilities.

The normal business path should receive:

```text
DatabaseCapability / Repository
```

instead.

---

# 162. Database Capability Type

A future abstraction might be:

```rust
pub struct Database {
    pool: PgPool,
}
```

but this should remain infrastructure-private as much as practical.

Repository traits should expose business-oriented operations.

Do not expose raw `PgPool` throughout the entire application if that creates architecture coupling.

---

# 163. Raw PgPool Exposure Tradeoff

A raw `PgPool` can be convenient, but broad exposure increases the number of places that can execute arbitrary SQL.

The preferred design is:

```text
application
 -> repositories/ports
 -> persistence implementation
```

rather than:

```text
application
 -> arbitrary PgPool.execute()
```

The latter is harder to govern and test.

---

# 164. Repository Transaction Ownership

A repository method must clearly state whether it expects:

```text
pool
or
active transaction
```

For multi-repository atomic operations, the application should be able to provide the same transaction context to every repository involved.

Avoid hidden independent transactions.

---

# 165. Unit-of-Work Handoff

A future unit-of-work abstraction may simplify:

```text
begin
 -> perform repositories
 -> commit
```

but it should not hide transaction boundaries so completely that reviewers cannot see which operations are atomic.

Transparency matters for financial correctness.

---

# 166. Database Operations Must Be Bounded

Every future repository query should have:

```text
known operation purpose
bounded inputs
bounded results
expected indexes
transaction semantics
error mapping
```

Do not create generic repository methods such as:

```text
find_any_table
execute_arbitrary_query
```

---

# 167. Dynamic SQL Rules

Identifiers cannot be bound as normal SQL parameters.

If a dynamic column/order choice is required:

```text
client value
  -> enum/allowlist
  -> trusted SQL fragment
```

not:

```text
format!("ORDER BY {}", client_input)
```

---

# 168. SQL Injection Defense Stack

Sitolo should layer:

```text
input validation
+
SQLx bound parameters
+
allowlists for identifiers
+
least-privileged DB role
+
RLS where applicable
+
constraints
```

An application bug should not automatically become unrestricted database administration.

---

# 169. Database Security Testing Matrix — Future Phase 5

The following must be executed against PostgreSQL:

| Security property | Test |
|---|---|
| runtime privilege | runtime cannot DDL |
| RLS | tenant A cannot read tenant B |
| RLS write | tenant A cannot insert B row |
| missing context | access denied |
| pool reuse | context cannot leak |
| role isolation | reporting cannot mutate |
| migration isolation | runtime cannot migrate |
| SQL injection | malicious input remains data |
| constraint integrity | invalid state rejected |
| replay | duplicate idempotency key handled once |

---

# 170. Database Integrity Testing Matrix — Future Phase 5

Include:

```text
foreign key enforcement
unique conflict
check violation
rollback
commit
serialization
lock timeout
deadlock
concurrent stock decrement
cash-close race
webhook replay
```

---

# 171. Database Recovery Testing Matrix — Later Operations

Include:

```text
backup success
restore success
point-in-time recovery
application reconnect
migration compatibility after restore
audit integrity after restore
idempotency state after restore
external reconciliation after restore
```

A backup that has never been restored is not sufficient evidence of recoverability.

---

# 172. Disaster Recovery Credential Consideration

Restoring PostgreSQL may restore data and schema but not necessarily the current credential authority state.

The runtime secret reference therefore must remain externally managed and replaceable.

This is another reason not to store credentials inside application source/config snapshots.

---

# 173. Incident Runbook — Database Unavailable

Use:

```text
1. identify service version
2. identify database target fingerprint
3. inspect pool pressure
4. inspect connection failure category
5. inspect network/TLS state
6. inspect secret-provider state
7. inspect recent configuration changes
8. avoid blind retries
9. recover DB capability
10. verify readiness
11. reconcile unknown outcomes
```

---

# 174. Incident Runbook — Wrong Credential

Do not immediately assume PostgreSQL itself is broken.

Check:

```text
SecretRef
secret provider
role
credential rotation timing
pool connection lifetime
deployment version
```

Verify with safe diagnostics only.

---

# 175. Incident Runbook — Pool Exhaustion

Check:

```text
current pool usage
acquisition waits
long-running transactions
slow queries
locks
reporting jobs
retries
```

Do not immediately raise pool maximum.

---

# 176. Incident Runbook — Unknown Mutation Outcome

Check:

```text
operation ID
idempotency record
authoritative state
external provider state if applicable
retry policy
```

Do not rerun the mutation until its safety is established.

---

# 177. Phase 2 Implementation Acceptance Tests

The repository should eventually prove all of these:

```text
[ ] valid DB config accepted
[ ] invalid DB config rejected
[ ] password cannot enter AppConfig
[ ] missing SecretRef rejected
[ ] invalid SecretRef rejected
[ ] production local provider rejected
[ ] pool inversion rejected
[ ] pool ceiling rejected
[ ] zero timeout rejected
[ ] excessive timeout rejected
[ ] config fingerprint excludes secret value
[ ] safe DB diagnostics contain no password
[ ] secret provider errors are typed
[ ] persistence boundary exists
[ ] domain does not depend on SQLx
[ ] API does not directly access SQLx
[ ] shutdown path is bounded
```

---

# 178. Phase 2 Non-Functional Requirements

The implementation must be:

```text
deterministic
observable
secure
bounded
testable
maintainable
portable across approved secret providers
compatible with future SQLx/PostgreSQL implementation
```

It must not introduce unnecessary framework complexity.

---

# 179. Phase 2 Performance Requirements

Configuration parsing should be negligible compared with process startup.

Secret resolution and database connection are I/O operations and should be async where the API runtime is async.

Telemetry must be bounded.

Database pools must not be unbounded.

---

# 180. Phase 2 Security Requirements

The database credential must:

```text
never be committed
never be returned to clients
never be a metric label
never be a trace attribute
never be included in fingerprints
never be stored in AppConfig
never be exposed through normal Debug output
```

---

# 181. Phase 2 Maintainability Requirements

An engineer should be able to find:

```text
configuration -> sitolo-config
secrets -> sitolo-security
persistence -> sitolo-persistence
HTTP mapping -> sitolo-api
business orchestration -> sitolo-application
business semantics -> sitolo-domain
```

Avoid introducing hidden coupling across crates.

---

# 182. Phase 2 Observability Requirements

An operator should be able to correlate:

```text
configuration fingerprint
service version
request ID
trace ID
database target identifier
pool pressure
DB error category
```

without exposing credentials.

---

# 183. Phase 2 Reliability Requirements

A PostgreSQL outage must not cause:

```text
fake success
silent SQLite authority
uncapped retries
unbounded connection creation
secret fallback
process hang forever
```

The failure must be explicit and recoverable according to later domain semantics.

---

# 184. Phase 2 Documentation Requirements

The repository documentation must contain a short developer path explaining:

```text
PostgreSQL 18 target
local container option
canonical config names
secret reference usage
how to run tests
what is deferred to Phase 5
```

Do not copy production credentials or provider-specific secret instructions into the repository.

---

# 185. Suggested Documentation Example

A concise developer page may contain:

```text
Start PostgreSQL 18 locally.
Create the development database and runtime role.
Set SITOLO__DATABASE__HOST/PORT/NAME/USER.
Set SITOLO__DATABASE__PASSWORD_REF=development/sitolo/db.
Provide the synthetic local secret through the development secret provider.
Run the Phase 2 tests.
Do not commit .env or the password.
```

Once Phase 5 exists, add migration/bootstrap instructions there.

---

# 186. Why the Repository Can Be Implemented Without a Native PostgreSQL Installation

The architecture does not require PostgreSQL to be installed directly on the developer's host OS.

It requires **real PostgreSQL semantics** when database behavior is being tested.

Therefore a container is valid because the process is still speaking to PostgreSQL.

This is materially different from substituting SQLite.

---

# 187. Why This Matters for the Current Repository

The uploaded repository already has the correct Phase 2 config skeleton and the Phase 5 database specification.

The missing engineering step is to make the boundary between those two phases explicit and executable.

That is the purpose of this document.

---

# 188. Implementation Guardrail: Do Not Rewrite Existing Architecture

Do not respond to the database configuration work by:

```text
creating another config crate
moving all environment parsing to persistence
putting secrets in API state
introducing an ORM
creating a parallel database abstraction
adding a second repository layer
```

The existing architecture is already modular enough.

The task is to complete the intended contracts, not redesign the platform.

---

# 189. Implementation Guardrail: Avoid Premature Abstraction

Do not create:

```rust
trait DatabaseEngine
```

just to support future MySQL/SQLite servers.

PostgreSQL is a deliberate architectural choice.

Abstract only where there is an actual dependency inversion or testability requirement.

---

# 190. Implementation Guardrail: Do Not Hide SQL Behind a Generic Executor

Do not create:

```rust
trait SqlExecutor {
    async fn execute(&self, sql: &str) -> ...;
}
```

and then allow application code to call arbitrary SQL through it.

That would destroy the intended repository boundary and greatly expand the attack surface.

---

# 191. Implementation Guardrail: Do Not Expose `SecretValue`

A persistence constructor should consume a secret capability rather than return the raw credential to application code.

Prefer:

```text
SecretProvider -> connection capability
```

not:

```text
SecretProvider -> AppState.db_password -> PostgreSQL factory
```

The second design spreads secret exposure across the application.

---

# 192. Implementation Guardrail: Do Not Log the Connection Builder

Never add:

```rust
tracing::debug!(url = %url, "creating postgres pool");
```

Even in development.

Log structured safe metadata instead.

---

# 193. Implementation Guardrail: Do Not Use Process Environment in Domain Logic

The domain must remain independent from:

```text
SITOLO__DATABASE__*
```

Configuration enters through typed values.

This is necessary for deterministic tests and architecture enforcement.

---

# 194. Implementation Guardrail: Do Not Read Business Policy from Config

Do not introduce database settings like:

```text
SITOLO__INVENTORY__ALLOW_OVERSOLD
```

if the value defines merchant/business semantics.

Those decisions belong in domain/application/database state, not deployment config.

---

# 195. Implementation Guardrail: Do Not Disable Security for Tests

Never make test code do:

```text
if cfg!(test) { bypass_rls = true; }
```

or connect as superuser simply to “make the fixture easier.”

Security tests must exercise the real boundary.

---

# 196. Implementation Guardrail: Do Not Swallow Initialization Errors

Never:

```rust
let db = connect().await.ok();
```

and continue as if the capability exists.

Required dependencies must either initialize successfully or produce an explicit failure state.

---

# 197. Implementation Guardrail: Do Not Create Retry Loops in Constructors

Database construction should not contain a long unbounded retry loop.

If startup retry is desired, it must be:

```text
bounded
observable
deadline-aware
```

and justified by deployment requirements.

---

# 198. Implementation Guardrail: Do Not Make Health Expensive

A readiness check should not run:

```text
large report
full inventory count
full reconciliation
```

Use the smallest reliable capability check.

---

# 199. Implementation Guardrail: Do Not Confuse Health With Authorization

A database being reachable does not mean:

```text
user is authorized
```

Health is infrastructure state.

Authorization is an application security decision.

---

# 200. Final Phase Boundary

The final boundary is:

```text
                         PHASE 2

SITOLO__DATABASE__*
        |
        v
validated AppConfig
        |
        +---- SecretRef
        |
        v
SecretProvider
        |
        v
Database capability boundary
        |
        +---- safe errors
        +---- readiness
        +---- telemetry
        +---- bounded resource policy
        |
        v
                         PHASE 5

PostgreSQL 18
        |
        +---- roles
        +---- schemas
        +---- migrations
        +---- constraints
        +---- indexes
        +---- RLS
        +---- SQLx
        +---- transactions
        +---- concurrency
        +---- real security tests
```

This is the intended implementation boundary.

---

# 201. Final Non-Negotiable Contract

> **PostgreSQL is Sitolo's authoritative server-side transactional store. Phase 2 must make PostgreSQL a secure, typed, observable and fail-closed runtime dependency without implementing the Phase 5 database itself. Database identity is configuration; credentials are secret capabilities; business state is authoritative PostgreSQL data; SQLite is client continuity state; API handlers do not execute SQL; domain code does not depend on SQLx; runtime database access is least-privileged; failures are typed; retries are semantics-aware; telemetry remains non-authoritative; and all PostgreSQL security claims are eventually proven against real PostgreSQL.**

---

# 202. Exact Coding-Agent Instruction Block

The following is intended to be handed directly to the implementation agent.

```text
IMPLEMENTATION DIRECTIVE — SITOLO PHASE 2 POSTGRESQL

Read the complete repository before modifying code.

Treat these as governing sources:
- docs/system_architecture_design.md
- docs/database_design.md
- docs/phase2_config_secrets_logging_errors_telemetry_implementation.md
- docs/phase5_postgresql_schema_migrations_constraints_rls_implementation.md
- docs/security_implementation_spec.md
- docs/security_test_harness.md
- docs/testing_strategy.md
- docs/threat_model.md
- docs/deployment_spec.md
- docs/implementation_plan.md
- docs/ADR-001-025.md

Implement only Phase 2 PostgreSQL runtime preparation.

DO:
- preserve the SITOLO__DATABASE__* namespace;
- keep database credentials as SecretRef in configuration;
- strengthen DB config validation;
- establish a narrow DatabaseRuntimeConfig or equivalent view if useful;
- establish typed persistence initialization/error boundaries;
- keep secret resolution behind SecretProvider;
- keep the development environment provider explicitly non-production;
- add safe database diagnostics/fingerprinting;
- add pool and acquisition-timeout policy enforcement;
- add readiness/shutdown lifecycle hooks where Phase 2 requires them;
- add database configuration/security/redaction tests;
- enforce dependency direction and architecture checks;
- document local PostgreSQL 18 development setup without committing secrets;
- leave a clean Phase 5 handoff.

DO NOT:
- add Phase 5 tables;
- add RLS policies;
- add tenant RLS context;
- add production migrations;
- make the API runtime a migration owner;
- add DATABASE_URL as the canonical config API;
- store a password in AppConfig;
- use SQLite as a server fallback;
- put SQL in handlers;
- import SQLx into domain crates;
- expose raw PgPool broadly without architectural reason;
- log passwords or connection URLs;
- use superuser credentials for ordinary application execution;
- silently skip required database tests;
- fabricate test results.

For every changed file, verify that ownership remains correct.
For every new security property, add executable evidence where practical.
For every environment limitation, document it honestly.
At completion, report changed files, tests executed, tests not executable, and explicit Phase 5 work left deferred.
```

---

# 203. Expected Implementation Outcome

After implementation, a reviewer should be able to inspect the repository and answer “yes” to all of these:

```text
Is PostgreSQL configuration centralized?
Is the credential represented only by SecretRef in ordinary configuration?
Is production prevented from using the development secret provider?
Are pool and acquisition settings bounded?
Can invalid DB config fail startup deterministically?
Can the runtime report DB failures without leaking secrets?
Is database connectivity owned by the persistence boundary?
Can API code avoid knowing PostgreSQL credential details?
Is the domain still infrastructure-agnostic?
Is SQLite still clearly client-only?
Is PostgreSQL still the server authority?
Can Phase 5 add SQLx and migrations without redesigning configuration?
Are security claims backed by tests rather than comments?
```

If any answer is “no”, Phase 2 PostgreSQL preparation is incomplete.

---

# 204. Final Phase 2 Sign-Off Checklist

```text
CONFIGURATION
[ ] canonical DB keys are present and documented
[ ] no DATABASE_URL canonical alias exists
[ ] DB fields are typed
[ ] DB strings are bounded
[ ] port is validated
[ ] pool_min <= pool_max
[ ] pool limits have hard ceilings
[ ] acquisition timeout has hard ceiling

SECRETS
[ ] AppConfig contains SecretRef, not password
[ ] invalid SecretRef fails
[ ] missing secret fails
[ ] provider failure is typed
[ ] production cannot use local provider
[ ] password is absent from diagnostics/fingerprint

PERSISTENCE
[ ] persistence owns DB infrastructure
[ ] domain has no SQLx dependency
[ ] API handlers contain no SQL
[ ] DB capability is dependency-injected
[ ] raw credential does not propagate through business state

RELIABILITY
[ ] DB acquisition is bounded
[ ] initialization failures are explicit
[ ] readiness semantics exist
[ ] shutdown semantics exist
[ ] unknown outcome is represented
[ ] retries are not universal/blind

OBSERVABILITY
[ ] pool pressure is observable
[ ] DB error category is observable
[ ] DB target has safe identifier
[ ] no sensitive SQL/credentials in normal telemetry

DEVELOPMENT
[ ] PostgreSQL 18 local path documented
[ ] local role separation documented
[ ] synthetic development secret model documented
[ ] no secrets committed

CI/SECURITY
[ ] configuration tests pass
[ ] secret redaction tests pass
[ ] architecture tests pass
[ ] required DB tests are not silently skipped
[ ] real PostgreSQL testing is explicitly deferred only where Phase 5 owns it

PHASE BOUNDARY
[ ] no schema implementation added
[ ] no migrations added unless Phase 5 explicitly begins
[ ] no RLS implementation added
[ ] Phase 5 handoff documented
```

---

# 205. Sources and Research Basis

This focused implementation specification was prepared from the actual Sitolo repository structure and governing project documents, then cross-checked against current primary documentation.

## PostgreSQL

**PostgreSQL 18 — CREATE ROLE**  
https://www.postgresql.org/docs/18/sql-createrole.html

Used for role privilege boundaries, `SUPERUSER`, `CREATEROLE`, `CREATEDB`, `BYPASSRLS`, and connection-limit semantics.

**PostgreSQL 18 — Row Security Policies**  
https://www.postgresql.org/docs/18/ddl-rowsecurity.html

Used for RLS enablement, default-deny behavior, `USING`/`WITH CHECK`, role behavior and RLS bypass semantics.

**PostgreSQL 18 — Client Connection Defaults**  
https://www.postgresql.org/docs/18/runtime-config-client.html

Used for database timeout distinctions including `statement_timeout`, `transaction_timeout` and `idle_in_transaction_session_timeout`.

**PostgreSQL 18 — Release 18**  
https://www.postgresql.org/docs/18/release-18.html

Used to verify the PostgreSQL 18 feature/version baseline.

**PostgreSQL 18 — Release Notes**  
https://www.postgresql.org/docs/18/release.html

Used to verify the current 18.x maintenance line.

## SQLx

**SQLx documentation**  
https://docs.rs/sqlx/latest/sqlx/

Used for the PostgreSQL pool, query and transaction API boundary.

**SQLx `PgPoolOptions`**  
https://docs.rs/sqlx/latest/sqlx/postgres/type.PgPoolOptions.html

Used for PostgreSQL-specific pool construction.

**SQLx `PoolOptions`**  
https://docs.rs/sqlx/latest/sqlx/pool/struct.PoolOptions.html

Used for connection pool sizing, acquisition timeout and connection lifetime guidance.

**SQLx `Transaction`**  
https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html

Used for transaction lifecycle guidance.

**SQLx `migrate!`**  
https://docs.rs/sqlx/latest/sqlx/macro.migrate.html

Used for the future Phase 5 migration/build integration handoff.

---

# 206. Final Statement

This document is the Phase 2 PostgreSQL bridge.

It answers the engineering question:

> **“What exactly must be implemented now so that PostgreSQL is configured and treated correctly in Phase 2, while leaving the actual PostgreSQL schema/security implementation to Phase 5?”**

The answer is:

```text
centralized typed configuration
+
secret reference/capability boundary
+
strict validation
+
bounded pool/timeouts
+
safe diagnostics
+
typed persistence errors
+
readiness/lifecycle
+
observability
+
architecture enforcement
+
realistic development setup
+
explicit Phase 5 handoff
```

Do that completely.

Do not pretend that schema, RLS, migrations or transactional business correctness are complete until the Phase 5 implementation and real PostgreSQL test evidence exist.

That separation is the architecture.
