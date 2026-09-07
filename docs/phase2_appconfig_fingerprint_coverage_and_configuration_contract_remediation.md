# Sitolo Phase 2 — AppConfig Fingerprint Coverage & Configuration Contract Remediation

**Status:** Implementation specification  
**Phase:** Phase 2 — Configuration, Secrets, Logging, Errors & Telemetry  
**Finding:** Effective `AppConfig` contains configuration fields that are not represented by the configuration fingerprint, while the configuration field catalogue is incomplete relative to the typed parser/model.  
**Severity:** High for configuration-drift detection and operational correctness; Medium for immediate confidentiality/security impact  
**Priority:** P0 before declaring Phase 2 configuration work complete  
**Scope:** `crates/sitolo-config` and its configuration-contract tests  
**Primary files:** `model.rs`, `parse.rs`, `field.rs`, `fingerprint.rs`, `validate.rs`, `lib.rs`, associated tests  
**Out of scope:** PostgreSQL schema/migrations/RLS implementation, domain authorization, payment provider protocols, mobile sync semantics

---

## 1. Executive Summary

The Sitolo configuration subsystem already has the correct architectural direction in several important areas:

- configuration is represented by a typed `AppConfig`;
- environment variables use the canonical `SITOLO__...` namespace;
- database passwords are represented by `SecretRef`, not stored as raw password values;
- configuration is validated before becoming effective runtime configuration;
- a deterministic SHA-256 fingerprint is intended to represent effective non-secret configuration;
- configuration fingerprints are explicitly intended for operational drift detection.

However, an implementation-consistency defect exists.

`AppConfig` currently contains more fields than the fingerprint implementation represents. In particular, the current fingerprint omits configuration that materially changes runtime behavior, including:

- `request_header_timeout_ms`;
- `keepalive_timeout_ms`;
- `db_acquire_timeout_ms`;
- `otel_export_timeout_ms`;
- `allow_local_secret_provider`;
- `config_schema_version`.

The configuration field catalogue is also incomplete relative to the actual parser/model. The parser accepts these configuration keys, and the model stores them, but the catalogue does not describe all of them.

This creates two related defects:

1. **Fingerprint false equality:** two instances can produce the same fingerprint despite having materially different effective configuration.
2. **Schema metadata drift:** the authoritative machine-readable configuration catalogue does not fully describe the configuration contract actually accepted by the binary.

These are not merely documentation problems. The fingerprint is operational evidence used to detect deployment drift. If an effective setting is omitted, the fingerprint can falsely assert that two instances have equivalent configuration.

The remediation is therefore to establish one explicit configuration-field contract and derive both:

- fingerprint coverage; and
- catalogue coverage

from that contract.

The implementation must not solve this by blindly adding fields to the existing `format!` string. The canonical representation itself must be deliberately specified, deterministic, non-secret, versioned, testable, and stable.

---

# 2. Evidence and Current State

The CI job confirms that the repository is compiling against Rust `1.98.1` and currently resolves:

- `sha2 v0.11.0`;
- `digest v0.11.3`;
- `hybrid-array v0.4.14`.

The immediate CI compiler failure is in:

```text
crates/sitolo-config/src/fingerprint.rs:25
```

with:

```text
error[E0277]: the trait bound
Array<...>: LowerHex is not satisfied
```

The failing expression is:

```rust
format!("sha256:{:x}", Sha256::digest(data.as_bytes()))
```

The CI log shows the repository passed toolchain verification and reached compilation before failing in `sitolo-config`. It did not fail because PostgreSQL was unavailable. The job uses Rust `1.98.1`, and the dependency versions above were actually compiled before the failure.

The existing Phase 2 configuration contract states that the fingerprint must be:

```text
SHA-256(canonical_non_secret_effective_config)
```

and that raw secret values, private keys, session material, and PII must be excluded.

The current typed model already contains the following effective configuration fields:

```text
environment
service_name
service_version
bind_address
max_request_body_bytes
request_header_timeout_ms
keepalive_timeout_ms
db_host
db_port
db_name
db_user
db_password_ref
db_pool_min
db_pool_max
db_acquire_timeout_ms
otel_endpoint
otel_export_timeout_ms
otel_max_queue
trace_sample_ratio
log_level
allow_local_secret_provider
config_schema_version
```

The current parser also accepts the corresponding canonical environment keys.

The existing fingerprint implementation currently represents only a subset:

```text
config_schema_version
environment
service_name
service_version
bind_address
max_request_body_bytes
db_host
db_port
db_name
db_user
db_password_ref
db_pool_min
db_pool_max
otel_endpoint
trace_sample_ratio
log_level
otel_max_queue
```

Therefore the following fields are currently absent from fingerprint input:

```text
request_header_timeout_ms
keepalive_timeout_ms
db_acquire_timeout_ms
otel_export_timeout_ms
allow_local_secret_provider
```

`config_schema_version` is already present and must remain present.

---

# 3. Why This Finding Matters

## 3.1 The fingerprint is not merely a hash

A hash has no useful operational semantics by itself.

The Sitolo fingerprint is intended to answer a specific question:

> Are these processes operating under the same effective, non-secret configuration contract?

That means the hash input must faithfully represent every effective non-secret configuration value whose change should make two deployments operationally distinguishable.

A cryptographically correct SHA-256 implementation can therefore still produce an architecturally incorrect fingerprint.

For example:

```text
Instance A:
db_acquire_timeout_ms = 2000

Instance B:
db_acquire_timeout_ms = 60000
```

If `db_acquire_timeout_ms` is omitted from the canonical fingerprint input, both instances can produce the same fingerprint.

That is a false negative in configuration-drift detection.

---

# 4. Configuration Contract Layers

The remediation must distinguish four related but different concepts.

## 4.1 Accepted configuration key

Example:

```text
DATABASE__ACQUIRE_TIMEOUT_MS
```

This is the external configuration interface.

## 4.2 Typed builder field

Example:

```rust
db_acquire_timeout_ms: u64
```

This is the mutable configuration-loading representation.

## 4.3 Effective AppConfig field

Example:

```rust
pub db_acquire_timeout_ms: u64
```

This is validated runtime configuration.

## 4.4 Fingerprint field

Example:

```text
db_acquire_timeout_ms=<effective value>
```

This is the operationally significant representation.

All four must agree.

A configuration key that can affect runtime behavior must not silently disappear between these layers.

---

# 5. Required Canonical Configuration Set

The effective non-secret fingerprint contract must include the following fields.

## Runtime

```text
config_schema_version
environment
service_name
service_version
```

## HTTP

```text
bind_address
max_request_body_bytes
request_header_timeout_ms
keepalive_timeout_ms
```

## Database

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

## Telemetry

```text
otel_endpoint
otel_export_timeout_ms
otel_max_queue
trace_sample_ratio
```

## Logging

```text
log_level
```

## Secret-provider policy

```text
allow_local_secret_provider
```

No raw secret values are permitted.

---

# 6. Secret Boundary

The fingerprint must never include:

```text
database password
JWT secret
private key
session token
API secret value
provider credential
cookie secret
encryption key
```

The database password reference may be included because it is a reference, not the secret value.

For example:

```text
db_password_ref=database:development/sitolo/db
```

is acceptable if `SecretRef` guarantees that its displayed representation contains only the reference identity and never resolves the underlying secret.

The following is prohibited:

```rust
format!("{:?}", resolved_database_password)
```

The following is also prohibited:

```rust
format!("password={}", password)
```

The fingerprint must remain safe to emit as a low-cardinality operational attribute.

---

# 7. Configuration Catalogue Remediation

The current catalogue must be brought into parity with the actual accepted configuration contract.

At minimum, add entries for:

```text
HTTP__REQUEST_HEADER_TIMEOUT_MS
HTTP__KEEPALIVE_TIMEOUT_MS
DATABASE__ACQUIRE_TIMEOUT_MS
TELEMETRY__EXPORT_TIMEOUT_MS
LOG__LEVEL
SECRETS__ALLOW_LOCAL_PROVIDER
RUNTIME__CONFIG_SCHEMA_VERSION
```

The catalogue should also be reviewed against every `AppConfig` field and every `parse.rs` match arm.

The invariant should become:

```text
accepted canonical configuration key
        ↓
parser
        ↓
Builder field
        ↓
validation
        ↓
AppConfig field
        ↓
catalogue metadata
        ↓
fingerprint policy
```

No accepted key should exist only in one of these layers.

---

# 8. Catalogue Metadata Requirements

Each `ConfigField` entry must continue to describe at least:

```text
key
class
required
reloadable
owner
```

The existing Phase 2 contract additionally requires configuration-schema metadata covering:

```text
name
class
required?
default
source(s)
allowed range / syntax
secret classification
reloadability
owner
startup failure behavior
runtime failure behavior
```

If the current Rust structure intentionally remains minimal, the implementation should not pretend that the five-field `ConfigField` struct contains all of this metadata.

Either:

1. extend `ConfigField` to represent the full required schema; or
2. introduce a separate strongly typed metadata structure while retaining `ConfigField` as the public compatibility surface if necessary.

The implementation must choose deliberately and document the compatibility decision.

---

# 9. Fingerprint Canonicalization Contract

The fingerprint input must be canonical.

A valid canonical representation must satisfy:

```text
same effective configuration
        =>
same canonical bytes
        =>
same SHA-256
        =>
same fingerprint
```

and:

```text
materially different effective configuration
        =>
different canonical bytes
        =>
different fingerprint
```

subject to the normal cryptographic collision assumption.

The canonical format must have:

- fixed field ordering;
- explicit field names;
- explicit separators;
- stable scalar formatting;
- explicit representation of optional values;
- no locale-dependent formatting;
- no debug formatting;
- no map iteration whose order is unspecified;
- no secret values;
- no environment-variable source names unless they are themselves part of the intended semantics.

---

# 10. Recommended Canonical Representation

Use an explicit line-oriented representation.

Conceptually:

```text
v=2
environment=development
service_name=sitolo
service_version=0.0.0+dev
bind_address=0.0.0.0:8080
max_request_body_bytes=2097152
request_header_timeout_ms=5000
keepalive_timeout_ms=30000
db_host=localhost
db_port=5432
db_name=sitolo
db_user=sitolo_api
db_password_ref=database:development/sitolo/db
db_pool_min=5
db_pool_max=20
db_acquire_timeout_ms=2000
otel_endpoint=<none>
otel_export_timeout_ms=3000
otel_max_queue=2048
trace_sample_ratio=0.1
log_level=info
allow_local_secret_provider=true
config_schema_version=2
```

The actual implementation must use real newline characters, not an accidental textual sequence representing a backslash followed by `n`.

Do not depend on `Debug` formatting for the canonical contract.

For example, avoid:

```rust
format!("{:?}", c.otel_endpoint)
```

Prefer explicit semantic encoding:

```text
otel_endpoint=<none>
```

or a deliberately specified escaped representation.

---

# 11. Important Existing Canonicalization Defect

The current fingerprint source contains:

```rust
"v={}\\nenv={}\\nservice={}..."
```

In a Rust string literal, `\\n` represents a literal backslash followed by `n`.

It does not represent a newline.

If the intended canonical format is line-oriented, the implementation must use:

```rust
"\n"
```

rather than:

```rust
"\\n"
```

This must be verified against the actual source before changing it because changing canonical serialization changes fingerprints.

This is precisely why the canonical representation needs a versioned contract.

---

# 12. Fingerprint Versioning

The fingerprint format must be treated as a contract.

The existing:

```text
v=<schema>
```

field is useful, but the implementation must distinguish:

```text
configuration schema version
```

from:

```text
fingerprint serialization version
```

They are related but not necessarily identical.

A future change such as:

```text
field ordering
escaping
optional-value encoding
numeric serialization
```

can change the resulting hash without changing the semantic configuration.

Therefore the long-term design should permit a fingerprint serialization version.

For example:

```text
fingerprint_format=1
config_schema_version=2
...
```

Do not introduce this solely to fix the current issue if doing so would unnecessarily alter the existing contract. If introduced, document it as a deliberate compatibility change.

---

# 13. Do Not Add Hash Dependencies Merely to Fix This

The immediate CI failure is caused by attempting:

```rust
{:x}
```

on the `Array` returned by the current `sha2` API.

Do not:

- downgrade Rust;
- downgrade `sha2`;
- replace SHA-256;
- introduce unsafe conversion;
- disable the lint;
- disable the test;
- weaken CI;
- add an unnecessary cryptographic library solely to obtain hex encoding.

The digest bytes should be explicitly encoded as lowercase hexadecimal.

A dependency such as a dedicated hexadecimal encoding crate is acceptable only if the workspace dependency policy justifies it. For a fixed 32-byte digest, a small explicit encoding helper is entirely reasonable and avoids dependency expansion.

---

# 14. Required Fingerprint Implementation Properties

The implementation must satisfy all of the following:

## 14.1 Deterministic

Identical effective configurations produce identical fingerprints.

## 14.2 Complete

Every effective non-secret configuration field is represented.

## 14.3 Secret-free

No secret value is represented.

## 14.4 Stable

Field ordering and scalar formatting are explicit.

## 14.5 Low-cardinality

The fingerprint is suitable for operational telemetry.

## 14.6 Cryptographically strong

SHA-256 remains the hashing primitive.

## 14.7 Version-aware

Changes to canonical serialization must be deliberate and reviewable.

## 14.8 Independent of configuration source

These should fingerprint identically when their effective values are identical:

```text
compiled default
environment variable
future configuration file
future secret-reference provider
```

The fingerprint represents effective configuration, not where it came from.

---

# 15. Required Regression Tests

The current test:

```rust
fingerprint_is_deterministic_and_excludes_secret_value
```

is insufficient to prove coverage.

Add targeted tests.

## 15.1 Determinism

```text
same config -> same fingerprint
```

## 15.2 SHA-256 prefix

Fingerprint must begin with:

```text
sha256:
```

## 15.3 Lowercase hexadecimal

The digest portion must:

```text
contain only [0-9a-f]
contain exactly 64 hexadecimal characters
```

for SHA-256.

## 15.4 Different service name

Changing:

```text
service_name
```

must change the fingerprint.

## 15.5 Different request-header timeout

Changing:

```text
request_header_timeout_ms
```

must change the fingerprint.

## 15.6 Different keepalive timeout

Changing:

```text
keepalive_timeout_ms
```

must change the fingerprint.

## 15.7 Different database acquisition timeout

Changing:

```text
db_acquire_timeout_ms
```

must change the fingerprint.

This is the primary regression test for the finding.

## 15.8 Different telemetry export timeout

Changing:

```text
otel_export_timeout_ms
```

must change the fingerprint.

## 15.9 Different telemetry queue

Changing:

```text
otel_max_queue
```

must change the fingerprint.

## 15.10 Different sampling ratio

Changing:

```text
trace_sample_ratio
```

must change the fingerprint.

## 15.11 Different logging level

Changing:

```text
log_level
```

must change the fingerprint.

## 15.12 Different secret-provider policy

Changing:

```text
allow_local_secret_provider
```

must change the fingerprint.

This setting affects the security posture of the process and must not be invisible to drift detection.

## 15.13 Different schema version

Changing:

```text
config_schema_version
```

must change the fingerprint.

## 15.14 Secret-value independence

Two configurations referencing the same secret identity but resolving to different secret values must have identical fingerprints.

The test must not place actual secret material into the configuration fingerprint API.

## 15.15 Optional telemetry endpoint

These must produce different fingerprints:

```text
otel_endpoint=None
otel_endpoint=Some("https://collector.example")
```

## 15.16 Field-completeness regression

Introduce a test that explicitly enumerates the fields that must participate in fingerprinting.

This is stronger than relying only on pairwise tests because a future developer could add a field to `AppConfig` and forget to update the fingerprint.

---

# 16. Stronger Architectural Test

The preferred long-term test strategy is to centralize canonical serialization.

Conceptually:

```rust
fn canonical_non_secret_config(c: &AppConfig) -> String
```

then:

```rust
pub fn config_fingerprint(c: &AppConfig) -> String {
    let canonical = canonical_non_secret_config(c);
    let digest = Sha256::digest(canonical.as_bytes());
    encode_lower_hex(&digest)
}
```

This gives the system two independently testable boundaries:

```text
AppConfig
   ↓
canonical_non_secret_config()
   ↓
stable bytes
   ↓
SHA-256
   ↓
hex encoding
   ↓
fingerprint
```

The canonical serializer can then be tested independently from the cryptographic encoding.

---

# 17. Configuration Catalogue Completeness Test

The catalogue should be checked against the parser.

At minimum, tests should verify that all canonical keys accepted by `parse.rs` have catalogue entries.

The inverse should also be tested:

```text
catalogue key
    =>
parser understands key
```

This prevents:

```text
catalogue says supported
but parser rejects
```

and:

```text
parser accepts
but catalogue omits
```

The second condition is the current class of defect.

---

# 18. AppConfig Field Inventory Test

Maintain an explicit inventory of the effective fields.

Required inventory:

```text
environment
service_name
service_version
bind_address
max_request_body_bytes
request_header_timeout_ms
keepalive_timeout_ms
db_host
db_port
db_name
db_user
db_password_ref
db_pool_min
db_pool_max
db_acquire_timeout_ms
otel_endpoint
otel_export_timeout_ms
otel_max_queue
trace_sample_ratio
log_level
allow_local_secret_provider
config_schema_version
```

Every addition to `AppConfig` must trigger a review of:

```text
parser
defaults
validation
catalogue
fingerprint
redaction
documentation
tests
```

A configuration-field change must therefore be treated as a cross-cutting contract change.

---

# 19. Database Acquisition Timeout Must Be Included

This is the most operationally important omitted field identified by the original finding.

The configuration contains:

```text
db_acquire_timeout_ms
```

This controls how long callers wait for a database connection from the pool.

It affects:

- request latency;
- saturation behavior;
- backpressure;
- timeout propagation;
- overload behavior;
- failure rates;
- database pool observability;
- availability during contention.

Two API instances with:

```text
db_acquire_timeout_ms=2000
```

and:

```text
db_acquire_timeout_ms=60000
```

do not have equivalent runtime behavior.

Therefore:

```text
db_acquire_timeout_ms
```

is mandatory fingerprint input.

---

# 20. HTTP Timeout Fields Must Be Included

The same reasoning applies to:

```text
request_header_timeout_ms
keepalive_timeout_ms
```

These values affect connection behavior and resource occupancy.

They influence:

- slow-client exposure;
- connection lifetime;
- worker/resource consumption;
- backpressure;
- request admission behavior;
- operational latency.

Omitting them creates another class of false fingerprint equality.

---

# 21. Telemetry Export Timeout Must Be Included

The current configuration includes:

```text
otel_export_timeout_ms
```

This determines exporter behavior and can materially change:

- telemetry delivery latency;
- exporter resource usage;
- shutdown behavior;
- failure timing.

It is therefore part of effective non-secret configuration.

Telemetry is not business truth, but telemetry runtime behavior is still part of the process configuration and should be represented in the configuration fingerprint.

---

# 22. Local Secret Provider Policy Must Be Included

The setting:

```text
allow_local_secret_provider
```

has direct security significance.

Production validation already rejects:

```text
environment=production
allow_local_secret_provider=true
```

Nevertheless, the effective value belongs in the fingerprint.

The fingerprint is not a security control replacing validation. It is operational evidence.

If an instance unexpectedly runs with:

```text
allow_local_secret_provider=true
```

that state must be visible through configuration drift evidence.

---

# 23. Configuration Schema Version Must Remain Included

The current implementation already includes:

```text
config_schema_version
```

Keep it.

An instance interpreting configuration according to a different schema must not silently appear equivalent.

---

# 24. Do Not Fingerprint Raw Environment Variables

Do not implement coverage by hashing:

```rust
std::env::vars()
```

Reasons:

1. it violates the typed configuration architecture;
2. environment variable ordering is not the effective semantic model;
3. unrelated environment variables could change the fingerprint;
4. secret variables could accidentally be included;
5. aliases could create misleading differences;
6. source provenance would be confused with effective configuration.

The fingerprint must operate on validated `AppConfig`.

---

# 25. Do Not Use Debug Serialization as the Contract

Avoid:

```rust
format!("{:?}", c)
```

or:

```rust
serde_json::to_string(c)
```

unless serialization is deliberately defined as the canonical configuration contract.

Debug formatting is not a stable configuration protocol.

Generic serialization also introduces questions around:

- field ordering;
- optional-value representation;
- compatibility;
- future derived fields;
- secret fields;
- implementation details.

An explicit canonical serializer is safer.

---

# 26. Unicode and Delimiter Handling

The configuration contains bounded strings such as:

```text
service_name
service_version
db_host
db_name
db_user
otel_endpoint
```

The canonical format must remain unambiguous if values contain:

```text
newline
backslash
equals sign
control characters
Unicode
```

Validation currently bounds several values, but bounded does not automatically mean canonicalization-safe.

The implementation should either:

1. constrain configuration syntax so the chosen canonical line format is unambiguous; or
2. implement explicit escaping.

Do not create a format where:

```text
a|b
```

and:

```text
a
b
```

can serialize ambiguously.

The canonical serializer is security-sensitive because ambiguity can produce false equivalence or false divergence.

---

# 27. Validation Interaction

Fingerprint generation must occur only after successful validation.

Required lifecycle:

```text
raw sources
    ↓
parse
    ↓
validate
    ↓
AppConfig
    ↓
canonicalize
    ↓
hash
```

Never fingerprint the raw builder before validation.

Otherwise two semantically invalid configurations could receive fingerprints and become operationally confusing.

The fingerprint must represent effective accepted configuration, not arbitrary input.

---

# 28. Failure Behavior

Fingerprint generation itself should not perform network I/O.

It must be:

- CPU-local;
- deterministic;
- bounded;
- infallible after canonical configuration validation, unless the chosen serializer deliberately returns a bounded serialization error.

It must not:

- query PostgreSQL;
- resolve a secret;
- contact telemetry;
- call an external service;
- read arbitrary files;
- inspect mutable process state.

This keeps the fingerprint operation cheap and deterministic.

---

# 29. CPU vs I/O Considerations

SHA-256 over a small configuration document is CPU work.

The canonical configuration is tiny compared with ordinary request payloads, so hashing it is negligible relative to:

- network I/O;
- PostgreSQL acquisition;
- PostgreSQL query execution;
- external integration calls.

Do not introduce asynchronous execution merely for fingerprint calculation.

The correct model is:

```text
CPU:
canonical serialization
SHA-256
hex encoding

I/O:
none
```

This function should therefore remain synchronous.

---

# 30. Concurrency Semantics

`AppConfig` is intended to be immutable effective process configuration.

Fingerprint generation should operate on an immutable reference:

```rust
&AppConfig
```

No global mutable configuration should be introduced.

Do not use:

```rust
static mut
```

or runtime mutation to make fingerprinting easier.

If future dynamic configuration is introduced, a configuration snapshot must be explicitly defined. The fingerprint should correspond to one coherent effective snapshot rather than a mixture of fields read from independently changing sources.

---

# 31. Dynamic Configuration Future-Proofing

Current Phase 2 policy defaults trust-sensitive configuration to restart-required behavior.

If future dynamic configuration is introduced, the system must define whether the fingerprint represents:

```text
startup configuration
```

or:

```text
current effective configuration
```

For the current architecture, it should represent the validated process configuration snapshot.

Dynamic reload must not be retrofitted into fingerprint semantics without defining:

- atomic snapshot boundaries;
- versioning;
- concurrency;
- rollback;
- audit;
- telemetry;
- failure behavior.

---

# 32. Required Source Changes

## `crates/sitolo-config/src/fingerprint.rs`

Implement:

1. canonical serialization;
2. complete field coverage;
3. explicit optional-value representation;
4. explicit lowercase hexadecimal encoding;
5. SHA-256 prefix;
6. regression tests.

Remove direct:

```rust
{:x}
```

formatting of the `sha2` digest.

---

## `crates/sitolo-config/src/field.rs`

Add missing catalogue entries.

At minimum:

```text
HTTP__REQUEST_HEADER_TIMEOUT_MS
HTTP__KEEPALIVE_TIMEOUT_MS
DATABASE__ACQUIRE_TIMEOUT_MS
TELEMETRY__EXPORT_TIMEOUT_MS
LOG__LEVEL
SECRETS__ALLOW_LOCAL_PROVIDER
RUNTIME__CONFIG_SCHEMA_VERSION
```

Then audit every parser key against the catalogue.

---

## `crates/sitolo-config/src/model.rs`

No unnecessary model redesign is required.

The current model already contains the fields required by the finding.

Review documentation/comments to ensure every field clearly states:

- whether it is secret or non-secret;
- whether it is reloadable;
- its security significance;
- its unit;
- its fingerprint participation.

---

## `crates/sitolo-config/src/parse.rs`

No semantic redesign is required.

Verify every accepted key has catalogue coverage.

Ensure canonical names remain unchanged.

Do not introduce aliases such as:

```text
DATABASE_URL
DB_URL
POSTGRES_URL
PG_URL
```

---

## `crates/sitolo-config/src/validate.rs`

No validation weakening is permitted.

Ensure the fields added to the fingerprint are validated before they can reach `AppConfig`.

Existing security ceilings must remain enforced.

---

## `crates/sitolo-config/src/lib.rs`

Expand contract tests to cover:

- fingerprint coverage;
- catalogue/parser parity;
- secret exclusion;
- canonical digest formatting.

---

# 33. Cargo Dependency Policy

The existing workspace uses:

```toml
sha2 = { version = "0.11", default-features = false }
```

Do not alter this dependency solely because the digest container does not implement `LowerHex`.

The problem is formatting, not cryptographic correctness.

The preferred implementation should continue using the existing `sha2` dependency.

If a hexadecimal crate is introduced, justify it explicitly and ensure:

- no default features that violate workspace policy;
- no unnecessary dependency tree expansion;
- lockfile is updated;
- security audit remains clean.

A hand-written 32-byte lowercase hexadecimal encoder is acceptable if implemented clearly and tested.

---

# 34. Exact Hex Encoding Contract

The resulting fingerprint must have:

```text
prefix = "sha256:"
digest length = 64 characters
alphabet = 0-9a-f
```

Therefore:

```text
len("sha256:" + digest) = 71
```

The digest must not contain:

```text
A-F
0x
spaces
hyphens
base64 characters
debug formatting
```

---

# 35. Example Encoding Algorithm

A dependency-free implementation may conceptually do:

```rust
fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut out = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }

    out
}
```

Then:

```rust
let digest = Sha256::digest(canonical.as_bytes());
format!("sha256:{}", encode_lower_hex(&digest))
```

This should be used only after verifying compatibility with the actual resolved `sha2` API and project lint rules.

---

# 36. Fingerprint Regression Matrix

The test suite should establish:

| Mutation | Fingerprint must change? |
|---|---:|
| environment | Yes |
| service name | Yes |
| service version | Yes |
| bind address | Yes |
| max request body | Yes |
| request header timeout | Yes |
| keepalive timeout | Yes |
| database host | Yes |
| database port | Yes |
| database name | Yes |
| database user | Yes |
| password reference | Yes |
| password value behind same reference | No |
| pool minimum | Yes |
| pool maximum | Yes |
| database acquire timeout | Yes |
| OTLP endpoint | Yes |
| OTLP export timeout | Yes |
| OTLP queue | Yes |
| trace sample ratio | Yes |
| log level | Yes |
| local secret-provider policy | Yes |
| configuration schema version | Yes |

The password-value row is intentionally different because the fingerprint must not depend on secret material.

---

# 37. Catalogue Regression Matrix

The following must all exist in the catalogue:

| Canonical key | Parser | Builder | AppConfig | Catalogue | Fingerprint |
|---|---:|---:|---:|---:|---:|
| `RUNTIME__ENVIRONMENT` | Yes | Yes | Yes | Yes | Yes |
| `RUNTIME__SERVICE_NAME` | Yes | Yes | Yes | Yes | Yes |
| `RUNTIME__SERVICE_VERSION` | Yes | Yes | Yes | Yes | Yes |
| `HTTP__BIND_ADDRESS` | Yes | Yes | Yes | Yes | Yes |
| `HTTP__MAX_BODY_BYTES` | Yes | Yes | Yes | Yes | Yes |
| `HTTP__REQUEST_HEADER_TIMEOUT_MS` | Yes | Yes | Yes | **Must add** | Yes |
| `HTTP__KEEPALIVE_TIMEOUT_MS` | Yes | Yes | Yes | **Must add** | Yes |
| `DATABASE__HOST` | Yes | Yes | Yes | Yes | Yes |
| `DATABASE__PORT` | Yes | Yes | Yes | Yes | Yes |
| `DATABASE__NAME` | Yes | Yes | Yes | Yes | Yes |
| `DATABASE__USER` | Yes | Yes | Yes | Yes | Yes |
| `DATABASE__PASSWORD_REF` | Yes | Yes | Yes | Yes | Yes |
| `DATABASE__POOL_MIN` | Yes | Yes | Yes | Yes | Yes |
| `DATABASE__POOL_MAX` | Yes | Yes | Yes | Yes | Yes |
| `DATABASE__ACQUIRE_TIMEOUT_MS` | Yes | Yes | Yes | **Must add** | Yes |
| `TELEMETRY__OTLP_ENDPOINT` | Yes | Yes | Yes | Yes | Yes |
| `TELEMETRY__EXPORT_TIMEOUT_MS` | Yes | Yes | Yes | **Must add** | Yes |
| `TELEMETRY__MAX_QUEUE` | Yes | Yes | Yes | Yes | Yes |
| `TELEMETRY__TRACE_SAMPLE_RATIO` | Yes | Yes | Yes | Yes | Yes |
| `LOG__LEVEL` | Yes | Yes | Yes | **Must add** | Yes |
| `SECRETS__ALLOW_LOCAL_PROVIDER` | Yes | Yes | Yes | **Must add** | Yes |
| `RUNTIME__CONFIG_SCHEMA_VERSION` | Yes | Yes | Yes | **Must add** | Yes |

---

# 38. Security Review

The remediation must preserve:

## Confidentiality

No secret values enter:

- canonical configuration;
- fingerprint;
- logs;
- metrics;
- traces;
- error responses.

## Integrity

Fingerprint input must be derived from validated configuration.

## Availability

Fingerprinting must be bounded and local.

No network dependency may be introduced.

## Tenant isolation

Configuration fingerprinting is process-level infrastructure and must not include tenant identifiers, organization IDs, branch IDs, merchant data, or request-specific values.

## Auditability

Configuration drift can be detected from fingerprint changes without exposing secrets.

---

# 39. Observability Rules

The fingerprint may be attached to low-cardinality telemetry such as:

```text
config_fingerprint
```

Do not attach:

```text
raw configuration
database password
secret value
tenant_id
user_id
request_id
session_id
```

A fingerprint is suitable as an operational deployment attribute precisely because it compresses the effective configuration into a bounded identifier.

---

# 40. Logging Rules

Safe:

```text
config_fingerprint=sha256:...
```

Safe:

```text
configuration_loaded=true
config_schema_version=2
```

Unsafe:

```text
db_password=...
```

Unsafe:

```text
DATABASE_URL=postgres://user:password@...
```

Unsafe:

```text
resolved_secrets={...}
```

The fingerprint must never become an excuse to log the underlying canonical configuration document.

---

# 41. Backward Compatibility

Changing fingerprint input fields can change fingerprints for every deployment.

That is expected if the old fingerprint was incomplete.

However, this must be treated as an operational contract change.

Before rollout:

1. document the reason;
2. document the new coverage;
3. update tests;
4. update operational expectations;
5. communicate that old and new fingerprints are not directly comparable unless the format/version is known.

Do not preserve a defective fingerprint merely to avoid changing its value.

Correctness takes priority over continuity of an incorrect diagnostic value.

---

# 42. Migration Strategy

Implement in this order:

### Step 1 — Freeze the field inventory

List every effective `AppConfig` field.

### Step 2 — Reconcile parser

Confirm every field has an accepted canonical source where appropriate.

### Step 3 — Reconcile catalogue

Ensure every accepted key has metadata.

### Step 4 — Define canonical representation

Specify field ordering and scalar/optional encoding.

### Step 5 — Implement canonical serializer

Keep it pure and synchronous.

### Step 6 — Implement SHA-256 encoding

Hash canonical bytes and encode the digest explicitly as lowercase hexadecimal.

### Step 7 — Add mutation tests

Each fingerprint-relevant field must be proven to affect the fingerprint.

### Step 8 — Add secret-independence tests

Changing secret values must not affect the fingerprint.

### Step 9 — Add catalogue/parser parity tests

Prevent future metadata drift.

### Step 10 — Run formatting and CI

The project must pass its existing CI gate.

---

# 43. CI Verification

The existing CI verifies:

```text
rustc 1.98.1
cargo 1.98.1
lockfile
format
lint
workspace compilation/tests
```

The CI failure currently occurs during the lint/build stage while compiling `sitolo-config`.

After remediation, the agent must run the repository's prescribed verification command:

```text
./scripts/ci/verify
```

and, where appropriate:

```text
cargo test --workspace --all-targets --all-features --locked
```

The exact commands must remain subordinate to the repository's CI script.

Do not claim success unless the command actually succeeds.

---

# 44. PostgreSQL Independence

This remediation does not require PostgreSQL to be installed.

The finding exists entirely within:

```text
configuration model
parser
validation
catalogue
fingerprinting
tests
```

The database acquisition timeout is represented as configuration even though PostgreSQL integration is implemented later.

Therefore:

```text
Phase 2 configuration correctness
```

must be fixed now.

This does not mean implementing:

- migrations;
- RLS;
- schema;
- PostgreSQL repositories;
- database transactions.

Those remain governed by the Phase 5 boundary.

---

# 45. Implementation Anti-Patterns

Do not:

### A. Downgrade Rust

```text
No.
```

Rust `1.98.1` is the repository's declared toolchain.

### B. Downgrade SHA-2

```text
No.
```

The current API is valid; the formatting code is wrong.

### C. Remove fingerprinting

```text
No.
```

Fingerprinting is an explicit Phase 2 operational requirement.

### D. Remove omitted fields from AppConfig

```text
No.
```

The fields are legitimate runtime configuration.

### E. Remove fields from the catalogue

```text
No.
```

The catalogue must catch up with the actual contract.

### F. Hash raw environment variables

```text
No.
```

That violates the typed effective-configuration model.

### G. Hash resolved secrets

```text
Absolutely not.
```

### H. Serialize `AppConfig` using debug formatting

```text
No.
```

### I. Perform I/O during fingerprinting

```text
No.
```

### J. Add dynamic configuration as part of this fix

```text
No.
```

That is a separate design problem.

---

# 46. Acceptance Criteria

The finding is resolved only when all conditions below are true.

## Configuration model

- [ ] Every effective configuration field is inventoried.
- [ ] No required field is removed to make the fingerprint easier.
- [ ] Units and semantic meaning are documented.

## Parser

- [ ] Every canonical accepted key is known.
- [ ] No unauthorized aliases are introduced.
- [ ] Parser/model parity is tested.

## Catalogue

- [ ] Every accepted key has metadata.
- [ ] Missing timeout fields are added.
- [ ] Logging configuration is catalogued.
- [ ] Secret-provider policy is catalogued.
- [ ] Schema version is catalogued.

## Fingerprint

- [ ] Every effective non-secret field is represented.
- [ ] `db_acquire_timeout_ms` is represented.
- [ ] HTTP timeout fields are represented.
- [ ] OTLP export timeout is represented.
- [ ] local secret-provider policy is represented.
- [ ] schema version is represented.
- [ ] raw secret values are excluded.
- [ ] SHA-256 is retained.
- [ ] digest is encoded as 64 lowercase hexadecimal characters.
- [ ] fingerprint begins with `sha256:`.
- [ ] canonical field order is deterministic.
- [ ] optional values have explicit semantics.

## Tests

- [ ] Determinism test passes.
- [ ] Hex-format test passes.
- [ ] Each fingerprint field mutation changes the fingerprint.
- [ ] Secret-value changes do not change the fingerprint.
- [ ] Parser/catalogue parity tests pass.
- [ ] Workspace tests pass.

## CI

- [ ] `cargo fmt --check` passes.
- [ ] Clippy/lint passes.
- [ ] `cargo test --workspace --all-targets --all-features --locked` passes where supported by the repository.
- [ ] `./scripts/ci/verify` passes.
- [ ] No PostgreSQL requirement is introduced into this configuration-only remediation.

---

# 47. Definition of Done

This finding is not done merely because:

```text
cargo check
```

passes.

It is done when the system can demonstrate:

```text
effective AppConfig
        ↓
complete canonical non-secret representation
        ↓
stable SHA-256
        ↓
safe operational fingerprint
```

and:

```text
parser
  ↕
catalogue
  ↕
Builder
  ↕
AppConfig
  ↕
fingerprint
```

are contractually aligned.

The most important regression is:

```text
db_acquire_timeout_ms = 2000
        ≠
db_acquire_timeout_ms = 60000
```

therefore:

```text
fingerprint(A) != fingerprint(B)
```

Likewise:

```text
request_header_timeout_ms
keepalive_timeout_ms
otel_export_timeout_ms
allow_local_secret_provider
```

must not be invisible to configuration drift detection.

---

# 48. Coding-Agent Directive

An implementation agent working on this finding must:

1. inspect the current `sitolo-config` implementation before editing;
2. treat existing Phase 2 documentation as the governing contract;
3. inventory `AppConfig`, `Builder`, parser keys, catalogue entries, defaults and fingerprint fields;
4. preserve the canonical `SITOLO__` namespace;
5. preserve `SecretRef`;
6. preserve production rejection of the local secret provider;
7. preserve all existing validation ceilings;
8. add missing catalogue metadata;
9. implement complete fingerprint coverage;
10. fix digest encoding for `sha2 0.11`;
11. avoid unnecessary dependency changes;
12. avoid raw secret values;
13. avoid environment-variable hashing;
14. add regression tests for every fingerprint-relevant field;
15. add parser/catalogue parity tests;
16. run formatting;
17. run the repository's CI verification;
18. report actual command results;
19. never claim tests passed without executing them;
20. do not implement PostgreSQL schema/RLS as part of this finding.

---

# 49. Final Technical Position

The original CI compiler failure and the AppConfig coverage finding are related but distinct.

The compiler failure is:

```text
sha2::Digest output
        ↓
{:x}
        ↓
LowerHex not implemented
        ↓
E0277
```

The configuration-contract finding is:

```text
AppConfig has effective fields
        ↓
parser accepts them
        ↓
validation handles them
        ↓
fingerprint omits some
        ↓
false configuration equivalence
```

Both must be fixed.

The correct end state is not simply “make CI compile.”

The correct end state is:

```text
typed configuration
    +
complete configuration metadata
    +
validated effective state
    +
canonical non-secret serialization
    +
SHA-256
    +
explicit lowercase hexadecimal encoding
    +
regression coverage
    =
trustworthy configuration fingerprint
```

That is the standard required for Sitolo's Phase 2 configuration subsystem.
