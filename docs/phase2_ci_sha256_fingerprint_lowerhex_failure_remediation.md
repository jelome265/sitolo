# SITOLO — PHASE 2 CI BLOCKER REMEDIATION

## SHA-256 CONFIGURATION FINGERPRINT `LowerHex` COMPILATION FAILURE

**Document:** `phase2_ci_sha256_fingerprint_lowerhex_failure_remediation.md`  
**Phase:** Phase 2 — Config + Secrets + Logging + Errors + Telemetry  
**Scope:** `sitolo-config` configuration fingerprint implementation, CI verification, regression tests, and implementation handoff  
**Severity:** Release-blocking build failure  
**Status:** Implementation-governing remediation specification  
**Baseline:** 7 September 2026  
**Primary failure:** Rust compiler error `E0277` — SHA-256 digest output does not implement `std::fmt::LowerHex`

---

# 0. Executive Decision

The current Sitolo CI failure is a real source-level compatibility defect in `crates/sitolo-config/src/fingerprint.rs`. It is not a PostgreSQL failure, not a Rust toolchain regression, and not a reason to downgrade Rust or replace the SHA-256 dependency.

The failing expression is:

```rust
format!("sha256:{:x}", Sha256::digest(data.as_bytes()))
```

CI resolves the project to `sha2 v0.11.0`, `digest v0.11.3`, `hybrid-array v0.4.14`, and Rust `1.98.1`. The current `sha2`/`digest` line represents the SHA-256 result as a `hybrid_array::Array<u8, U32>`-family type rather than the older formatting-oriented representation assumed by the code. The compiler therefore correctly rejects the attempt to apply the `LowerHex` formatting trait directly to the digest result. The CI log reports exactly this trait failure at `crates/sitolo-config/src/fingerprint.rs:25`. fileciteturn9file0L85-L98 fileciteturn9file0L118-L148

The remediation is:

```text
KEEP Rust 1.98.1
KEEP sha2 0.11
KEEP SHA-256
KEEP the non-secret fingerprint contract
REPLACE direct `{:x}` formatting of the digest output
ADD explicit byte-oriented lowercase hexadecimal encoding
ADD contract tests for digest format and secret exclusion
ENSURE every effective non-secret configuration field covered by the fingerprint contract is represented
RE-RUN the complete repository verification script
```

The preferred implementation for this repository does **not** require adding another production dependency merely to encode 32 digest bytes. The digest output exposes byte access, and `hybrid-array::Array` provides `AsRef<[T]>`/`as_slice()` semantics; explicit hexadecimal encoding therefore provides a stable, dependency-minimal boundary. The current RustCrypto documentation confirms that `Sha256::digest(...)` returns an `Array<u8, U32>`-style output in the current digest stack. citehttps://docs.rs/digest/latest/digest/ https://docs.rs/sha2/0.11.0/sha2/

---

# 1. Incident Summary

## 1.1 CI evidence

The failing workflow successfully reaches the Rust verification script, verifies Rust `1.98.1`, verifies the lockfile, passes formatting, and begins linting. During linting, Cargo reaches `sitolo-config` and then fails to compile `fingerprint.rs`. The log shows:

```text
rustc 1.98.1 (48a229cea 2026-09-01)
cargo 1.98.1 (797e8a9bc 2026-08-05)
```

followed by dependency resolution/checking for:

```text
crypto-common v0.2.2
block-buffer v0.12.1
digest v0.11.3
sha2 v0.11.0
```

and then:

```text
error[E0277]: the trait bound
`Array<...>: LowerHex` is not satisfied

--> crates/sitolo-config/src/fingerprint.rs:25:28

format!("sha256:{:x}", Sha256::digest(data.as_bytes()))
```

This sequence matters operationally. The failure occurs before the project reaches its compile, unit-test, release-build, architecture, Phase 2 policy, dependency policy, or security gates. It is therefore a **pre-feature verification blocker**: the repository cannot produce a trusted green CI result while this compiler error remains. fileciteturn9file0L85-L98 fileciteturn9file0L99-L124

## 1.2 Repository verification command

The repository's `scripts/ci/verify` script deliberately treats the build pipeline as a sequence of mandatory gates:

```text
Verify toolchain
Verify lockfile
Format check
Lint
Compile
Unit tests
Release build
Architecture check
Phase 2 policy check
Dependency and license policy
```

The lint gate is:

```bash
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

and later gates are equally mandatory. Therefore an error in a foundational crate such as `sitolo-config` blocks the entire repository rather than being isolated to a single optional feature. This behavior is correct and must not be weakened merely to obtain a green check. 

---

# 2. Authoritative Phase 2 Contract

The existing Phase 2 implementation specification establishes several non-negotiable properties for configuration:

```text
configuration      = runtime policy
secrets            = access material
errors             = semantic failure classification
telemetry          = operational evidence
PostgreSQL         = authoritative business state
audit/evidence     = durable accountability evidence
```

It explicitly states that effective configuration must be fingerprintable over **non-secret fields only**, that secret rotation must not change the fingerprint when the secret reference is unchanged, and that raw credential material must never enter the fingerprint. fileciteturn8file6L465-L476

The dedicated PostgreSQL/Phase 2 document further states that database fingerprint inputs include host, port, database, role, pool limits, acquisition timeout, and secret reference. A secret value change alone must not change the fingerprint. fileciteturn8file2L108-L132

That same contract explicitly requires a distinction between `DATABASE__PASSWORD_REF` as canonical configuration and `SITOLO__DATABASE__PASSWORD` as a development-only secret-provider input. The password value must remain outside `AppConfig` and outside the fingerprint. fileciteturn9file1L196-L222

The Phase 2 Definition of Done also requires a deterministic configuration fingerprint that excludes secret values. fileciteturn8file8L545-L582

Therefore the compiler fix must not merely make the line compile. It must preserve and strengthen the existing security and observability contract.

---

# 3. Root Cause Analysis

## 3.1 The direct cause

The source currently asks Rust formatting machinery to apply the `LowerHex` formatter to the return value of `Sha256::digest`:

```rust
Sha256::digest(data.as_bytes())
```

The compiler reports that the concrete digest type does not implement `LowerHex`.

The important point is that SHA-256 itself is functioning correctly. The failure is at the **presentation/encoding boundary** between a cryptographic digest and a textual fingerprint.

There are three distinct representations:

```text
configuration bytes
        |
        v
SHA-256 digest bytes
        |
        v
lowercase hexadecimal text
        |
        v
"sha256:<64 lowercase hex characters>"
```

The code currently collapses the last two steps into a formatting assumption:

```text
Digest output -> `{:x}`
```

That assumption is no longer valid with the current dependency stack.

## 3.2 Why the dependency change matters

Current RustCrypto documentation for `digest` explains that the SHA-256 convenience API returns an `Array<u8, U32>`-style result. Current `sha2` documentation shows the same model in examples and identifies `sha2 0.11` as the current major line used by the project. citehttps://docs.rs/digest/latest/digest/https://docs.rs/crate/sha2/latest

The `hybrid-array` API explicitly exposes byte-oriented methods such as:

```text
as_slice()
AsRef<[T]>
iter()
```

so encoding the digest as bytes is the natural operation. citehttps://docs.rs/hybrid-array/latest/hybrid_array/struct.Array.html

The CI trace confirms `hybrid-array v0.4.14` is present in this exact build. fileciteturn9file0L114-L121

## 3.3 Why `{:x}` should not be retained by forcing a type conversion

Do not solve this by introducing an arbitrary cast, transmute, unsafe representation trick, or compiler-specific workaround.

The repository forbids unsafe code in `sitolo-config`:

```rust
#![forbid(unsafe_code)]
```

and the desired output is fundamentally a protocol string. Explicit byte-to-hex conversion is clearer, auditable, portable, and independent of the concrete digest container type.

## 3.4 Why downgrading `sha2` is the wrong fix

Do **not** pin the project back to an older `sha2` solely because an older digest type happened to support `LowerHex`.

That would reverse the dependency evolution instead of fixing the interface contract. The project already has a lockfile and CI is intentionally exercising current pinned dependencies. A build fix should adapt application code to the dependency's documented result type, not weaken the cryptographic dependency merely for formatting compatibility.

The current `sha2` documentation remains the appropriate basis for SHA-256 use; the one-shot `Sha256::digest` API is explicitly supported. citehttps://docs.rs/sha2/0.11.0/sha2/

## 3.5 Why adding `hex` is not automatically the best fix

A second valid solution is:

```rust
hex::encode(Sha256::digest(data.as_bytes()))
```

The `hex` crate's `encode` function accepts `AsRef<[u8]>` and produces lowercase hexadecimal text. citehttps://docs.rs/hex/latest/hex/fn.encode.html

However, the current `sitolo-config` crate has no `hex` dependency, and this code only needs to encode exactly 32 bytes. Introducing another production dependency for this small operation increases the dependency graph, lockfile surface, supply-chain inventory, and upgrade surface without providing substantial architectural value.

For Sitolo's current Phase 2 implementation, the preferred design is therefore:

```text
sha2
  |
  v
Digest bytes
  |
  v
small local hex encoder
```

A `hex` dependency becomes reasonable only if hexadecimal encoding is already an established shared platform primitive used in multiple crates. It should then be introduced deliberately at workspace level rather than opportunistically in this one file.

---

# 4. Required Remediation

## 4.1 Correct implementation pattern

Replace the failing direct formatter with an explicit byte encoder:

```rust
use crate::AppConfig;
use sha2::{Digest, Sha256};

/// SHA-256 over an explicitly canonical non-secret representation.
pub fn config_fingerprint(c: &AppConfig) -> String {
    let data = format!(
        "v={}\\nenv={}\\nservice={}\\nversion={}\\nbind={}\\nbody={}\\ndb_host={}\\ndb_port={}\\ndb_name={}\\ndb_user={}\\ndb_ref={}\\npool_min={}\\npool_max={}\\ndb_acquire_timeout_ms={}\\notel={:?}\\notel_export_timeout_ms={}\\nsample={}\\nlog={}\\notel_max_queue={}\\nallow_local_provider={}",
        c.config_schema_version,
        c.environment,
        c.service_name,
        c.service_version,
        c.bind_address,
        c.max_request_body_bytes,
        c.db_host,
        c.db_port,
        c.db_name,
        c.db_user,
        c.db_password_ref,
        c.db_pool_min,
        c.db_pool_max,
        c.db_acquire_timeout_ms,
        c.otel_endpoint,
        c.otel_export_timeout_ms,
        c.trace_sample_ratio,
        c.log_level,
        c.otel_max_queue,
        c.allow_local_secret_provider,
    );

    let digest = Sha256::digest(data.as_bytes());
    let mut encoded = String::with_capacity(digest.as_ref().len() * 2);

    for byte in digest.as_ref() {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }

    format!("sha256:{encoded}")
}
```

The code above illustrates the intended boundary, but implementation agents MUST first confirm the exact existing fingerprint contract and tests before changing the canonical input representation. The compile fix is mandatory; broad canonicalization changes must be treated separately because changing the canonical input changes the resulting fingerprint.

## 4.2 Prefer a helper when readability benefits

A more testable implementation is:

```rust
fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }

    output
}
```

and:

```rust
let digest = Sha256::digest(data.as_bytes());
format!("sha256:{}", encode_lower_hex(digest.as_ref()))
```

For this repository, either approach is acceptable. The helper has one advantage: it isolates the textual encoding contract from the fingerprint construction, making the security property easy to test.

## 4.3 Do not use a `Debug` representation

Do not replace:

```rust
{:x}
```

with:

```rust
{:?}
```

That would generate a representation intended for debugging rather than a stable, canonical fingerprint encoding. It could introduce brackets, separators, implementation-specific syntax, or formatting changes across versions.

The required output is a stable lowercase hexadecimal digest string.

## 4.4 Do not use base64 unless the contract changes

Base64 is a valid byte encoding but is not equivalent to the existing `sha256:<hex>` fingerprint format. Changing encoding changes every fingerprint. It would therefore be an architecture/API/operational decision, not a compiler fix.

---

# 5. Canonical Representation Audit

The compile error exposes a more important engineering concern: the hash function can be correct while the **hashed representation** is incomplete or unstable.

The current `fingerprint.rs` hashes selected configuration fields. The `AppConfig` model currently contains:

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

The source model is explicit about these fields. In particular, the database acquisition timeout is a first-class property of the validated configuration. 

The Phase 2 PostgreSQL implementation contract states that the database acquisition timeout legitimately affects the database target/runtime fingerprint. fileciteturn8file2L108-L132

Therefore the implementation must perform an explicit field-coverage review rather than assuming that fixing the formatter automatically satisfies the fingerprint contract.

## 5.1 Required field-coverage rule

For every field in `AppConfig`, classify it as:

```text
fingerprint-required
fingerprint-excluded-by-contract
or
not yet assigned
```

No field is allowed to remain in the third state when Phase 2 declares the fingerprint an effective-configuration identity.

## 5.2 Likely missing fields in the current implementation

The current source visibly includes `db_acquire_timeout_ms` in `AppConfig`, but the existing fingerprint string does not include it. The same source shows `request_header_timeout_ms`, `keepalive_timeout_ms`, `otel_export_timeout_ms`, `allow_local_secret_provider`, and `config_schema_version`; the existing fingerprint expression must be checked against the security and observability contract before finalizing the canonical set.

This document does **not** authorize arbitrary inclusion of every field. Some fields may be intentionally excluded if a higher-level contract says they are operational metadata rather than effective deployment identity. What matters is that the exclusion be explicit and tested.

## 5.3 Configuration fingerprint versus deployment identity

Do not conflate:

```text
configuration fingerprint
```

with:

```text
binary/build provenance identity
```

The service version already exists as provenance in `AppConfig`. A robust deployment system may later have a separate artifact digest or build identity. The configuration fingerprint should answer:

```text
What non-secret runtime configuration was this process constructed from?
```

It should not attempt to encode the complete source tree, binary hash, container digest, dependency lockfile, or deployment platform identity.

---

# 6. Canonicalization Stability

## 6.1 Current separator concern

The current source uses string fragments such as:

```rust
"v={}\\nenv={}\\n..."
```

There is an important distinction between:

```rust
"\\n"
```

and:

```rust
"\n"
```

The first contains the two-character sequence backslash + `n` in the resulting string; the second contains an actual newline byte.

The existing file currently contains doubled backslashes in the source. That may be deliberate, or it may be an accidental artifact of prior generation/editing. The remediation agent MUST inspect the literal source file before changing it.

Do not silently change separators while fixing `LowerHex` unless the change is included in an intentional fingerprint-format migration.

## 6.2 Why separator format matters

The input to SHA-256 is not semantic configuration. It is bytes. Therefore:

```text
canonical data A != canonical data B
```

means:

```text
SHA-256(A) != SHA-256(B)
```

with overwhelming probability.

Even if the logical fields are identical, changing:

```text
literal \n
```

to:

```text
actual newline
```

changes the fingerprint.

## 6.3 Escaping requirements

A future robust canonicalization format must make field boundaries unambiguous. Naively concatenating strings with separators can create ambiguity when fields themselves can contain the separator sequence.

Example:

```text
field_a = "ab|c"
field_b = "d"
```

versus:

```text
field_a = "ab"
field_b = "c|d"
```

A delimiter-only format can become ambiguous unless values are constrained or escaped.

The current configuration model already applies length/type validation, but that does not automatically prove delimiter safety. For a stable long-lived fingerprint contract, the canonical representation should be either:

```text
length-prefixed fields
```

or:

```text
strictly escaped key/value lines
```

or:

```text
canonical serialized structure with an explicit version
```

However, changing the representation today would change the fingerprint and must therefore be treated as a versioned contract change rather than mixed into the build-failure fix.

---

# 7. Security Requirements

## 7.1 Raw secret values remain prohibited

The fingerprint input may contain:

```text
SecretRef
```

but never:

```text
database password
API token
private key
session secret
JWT signing material
payment provider secret
webhook signing secret
```

The existing `AppConfig` design is correct because it stores `db_password_ref` as a `SecretRef`, not a password. fileciteturn9file1L196-L220

## 7.2 Secret rotation invariant

Given:

```text
SecretRef = production/database/main
Password A = old credential
Password B = new credential
```

then:

```text
fingerprint(A) == fingerprint(B)
```

provided the reference and all other effective non-secret configuration remain unchanged.

This is explicitly required by the Phase 2 contract. fileciteturn8file2L136-L167

## 7.3 Secret value leakage regression test

Use a test-only sentinel such as:

```text
TEST_ONLY_DATABASE_SECRET_001
```

and assert that it does not appear in:

```text
fingerprint output
formatted AppConfig output
logs
error text
health responses
telemetry attributes
```

The existing PostgreSQL Phase 2 document explicitly calls for this class of release-blocking secret-leak regression testing. fileciteturn8file2L171-L203

## 7.4 Fingerprints are not secret hashes

The configuration fingerprint is not intended to protect a secret.

It is:

```text
configuration identity
```

not:

```text
password verifier
```

Do not place password material into the input under the assumption that hashing makes it safe. A digest of a secret is still derived secret material and can become an offline oracle or leakage vector.

---

# 8. Required Test Suite

The existing repository currently has only a basic fingerprint determinism test. The CI blocker should therefore be corrected together with a stronger test contract.

## 8.1 Test 1 — deterministic output

Repeated calls for identical `AppConfig` values must produce identical strings:

```rust
#[test]
fn fingerprint_is_deterministic() {
    let config = validate(defaults::Builder::development()).unwrap();

    assert_eq!(
        config_fingerprint(&config),
        config_fingerprint(&config)
    );
}
```

## 8.2 Test 2 — SHA-256 prefix

Assert that the fingerprint always begins with:

```text
sha256:
```

## 8.3 Test 3 — exact digest width

SHA-256 is 256 bits = 32 bytes. Lowercase hexadecimal encodes each byte into two characters. Therefore the textual payload must contain:

```text
64 hex characters
```

and the total string length must be:

```text
7 + 64 = 71
```

for:

```text
sha256:
```

plus 64 hex digits.

The test must reject:

```text
short digest
uppercase hex
non-hex characters
additional debug syntax
```

## 8.4 Test 4 — lowercase-only encoding

For every character after the prefix, assert:

```text
0-9
a-f
```

only.

## 8.5 Test 5 — known SHA-256 vector at helper level

The byte-to-hex helper should be tested against a standard SHA-256 vector. For `b"hello world"`, the expected lowercase hexadecimal digest is:

```text
b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
```

The current `sha2` documentation shows this exact vector in its examples. citehttps://docs.rs/sha2/0.11.0/sha2/

## 8.6 Test 6 — secret value independence

The repository does not store raw secret values in `AppConfig`. The strongest practical regression test is to create two equivalent configurations with the same `SecretRef` while simulating different secret-provider values outside `AppConfig`.

Expected:

```text
same effective non-secret configuration
same SecretRef
secret A != secret B

=> same fingerprint
```

## 8.7 Test 7 — secret reference change changes fingerprint

Given:

```text
ref A = development/sitolo/db
ref B = production/sitolo/db
```

and otherwise identical configuration:

```text
fingerprint(A) != fingerprint(B)
```

The Phase 2 PostgreSQL implementation contract explicitly allows/ expects a fingerprint change when the reference itself changes. fileciteturn8file2L161-L167

## 8.8 Test 8 — database acquisition timeout is covered if classified as fingerprint input

Because the dedicated Phase 2 PostgreSQL contract explicitly identifies acquisition timeout as a legitimate fingerprint input, a configuration changing only:

```text
db_acquire_timeout_ms
```

must result in a different fingerprint unless an approved architectural decision says otherwise. fileciteturn8file2L108-L124

## 8.9 Test 9 — unrelated secret material never enters fingerprint source

A source-level security test should prevent future developers from adding fields with names such as:

```text
password
secret
private_key
token
credential
api_key
client_secret
```

to the canonical fingerprint function unless an architecture review explicitly approves the field.

This can be enforced through code review, a static test, or a Phase 2 policy check.

---

# 9. Better Long-Term Fingerprint Architecture

The immediate compilation fix should be intentionally small. The durable design should be stronger.

## 9.1 Introduce an internal canonical representation function

Rather than making one large `format!` call directly inside the public function, split:

```text
AppConfig
   |
   v
canonical_non_secret_config
   |
   v
SHA-256
   |
   v
lowercase hex
   |
   v
fingerprint string
```

For example:

```rust
fn canonical_non_secret_config(c: &AppConfig) -> String {
    ...
}

fn sha256_lower_hex(data: &[u8]) -> String {
    ...
}

pub fn config_fingerprint(c: &AppConfig) -> String {
    let canonical = canonical_non_secret_config(c);
    format!("sha256:{}", sha256_lower_hex(canonical.as_bytes()))
}
```

This separation gives independent tests for:

```text
canonicalization
encoding
final public format
```

## 9.2 Consider explicit fingerprint-format versioning

The current canonical data already includes `config_schema_version`, but that is not necessarily the same thing as a fingerprint algorithm/serialization version.

A future stable contract should distinguish:

```text
configuration schema version
```

from:

```text
fingerprint canonicalization version
```

For example:

```text
fingerprint_version = 1
config_schema_version = 2
```

This is useful if the canonical serialization has to change without changing the underlying schema.

Do not implement a new public format solely as part of the current compile failure unless the project has no consumers and the team explicitly accepts the change.

## 9.3 Do not hash a database connection string

The fingerprint should continue to avoid building a credential-bearing URL such as:

```text
postgres://user:password@host/database
```

and then hashing it.

Even if the final fingerprint does not reveal the password directly, creating the credential-bearing string creates unnecessary exposure in memory, diagnostics, stack traces, debugging, and future refactors.

The existing Phase 2 PostgreSQL document explicitly rejects credential-bearing connection strings as diagnostics/fingerprint inputs and requires the raw password to remain outside configuration identity. fileciteturn8file2L237-L261

---

# 10. CI Implementation Plan

## Gate 1 — Fix compilation

Change only the digest-to-text conversion first.

Required result:

```text
sitolo-config compiles
```

## Gate 2 — Add fingerprint contract tests

Required result:

```text
deterministic
sha256 prefix
64 lowercase hex digits
secret excluded
```

## Gate 3 — Audit canonical fields

Compare the implementation against `AppConfig` and the Phase 2 fingerprint contract.

Required result:

```text
Every included field is intentional.
Every excluded field has a reason.
No effective database field accidentally omitted.
```

## Gate 4 — Format and lint

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

## Gate 5 — Compile

Run:

```bash
cargo check --workspace --all-targets --all-features --locked
```

## Gate 6 — Tests

Run:

```bash
cargo test --workspace --all-targets --all-features --locked
```

## Gate 7 — Release build

Run:

```bash
cargo build --workspace --release --locked
```

## Gate 8 — Architecture and policy gates

Run:

```bash
./scripts/ci/check-architecture
./scripts/ci/check-phase2-policy
```

## Gate 9 — Full repository verifier

Finally run:

```bash
./scripts/ci/verify
```

Do not declare the incident closed after `cargo check -p sitolo-config` passes. The repository uses broader release gates, and the project contract states that skipped mandatory gates are not equivalent to success. The security architecture explicitly treats mandatory evidence as a release condition. fileciteturn8file5L393-L437

---

# 11. CI Debugging Procedure

If the first fix still fails, use this exact order.

## 11.1 Inspect actual resolved dependency versions

Run:

```bash
cargo tree -p sitolo-config -i sha2
cargo tree -p sitolo-config -i digest
cargo tree -p sitolo-config -i hybrid-array
```

Expected current family:

```text
sha2 0.11.x
digest 0.11.x
hybrid-array 0.4.x
```

The CI log currently shows:

```text
sha2 v0.11.0
digest v0.11.3
hybrid-array v0.4.14
```

so the implementation should be compatible with that graph rather than an imagined older graph. fileciteturn9file0L118-L124

## 11.2 Inspect the concrete type

For diagnostic work only, temporarily allow the compiler to expose the type or use:

```bash
cargo check -p sitolo-config -vv
```

The compiler is already showing the relevant fact:

```text
Array<u8, ...>
```

which is sufficient to establish that the result is not directly `LowerHex`-format-compatible. fileciteturn8file0L10-L26

## 11.3 Confirm that no hidden `hex` dependency exists

Run:

```bash
cargo tree -p sitolo-config | grep -E '^hex|hex '
```

If no shared workspace policy already establishes `hex`, keep the local encoder rather than adding a new dependency.

## 11.4 Confirm the source is using `as_ref()` or `as_slice()` correctly

The current `hybrid-array` implementation provides `AsRef<[T]>` and `as_slice()`. citehttps://docs.rs/hybrid-array/latest/hybrid_array/struct.Array.html

A robust implementation should therefore compile without relying on undocumented internal fields.

---

# 12. Failure Modes to Prevent

## Failure mode A — downgrade Rust

**Do not.**

Rust `1.98.1` is the repository's declared toolchain and CI is already correctly using it. fileciteturn9file0L85-L87

## Failure mode B — downgrade `sha2`

**Do not.**

Adapt the formatting boundary to the current documented digest result type.

## Failure mode C — add `unwrap()` around formatting

Do not attempt:

```rust
format!(...).unwrap()
```

Formatting into a `String` via `write!` cannot fail under the normal `fmt::Write` implementation, but this should be represented explicitly rather than used as a blanket runtime failure strategy.

The Phase 2 policy is to use `Result` for expected runtime failures and to reserve process termination for proven internal invariants. fileciteturn9file2L302-L306

## Failure mode D — use debug formatting

**Do not.**

`{:?}` is not a stable hexadecimal protocol.

## Failure mode E — add the secret value to make the fingerprint “more unique”

**Do not.**

The fingerprint must remain non-secret.

## Failure mode F — silently change canonicalization

**Do not.**

A change in canonical bytes changes the fingerprint. Separate compile compatibility from fingerprint contract migration.

## Failure mode G — weaken CI

Do not change:

```bash
-D warnings
```

to:

```text
warnings allowed
```

Do not remove `--locked`.
Do not skip the fingerprint tests.
Do not make the failing crate optional.
Do not make CI continue after the compiler error.

These actions hide the defect rather than solve it.

---

# 13. Exact File-Level Changes

## 13.1 `crates/sitolo-config/src/fingerprint.rs`

Required:

```text
REPLACE direct `{:x}` use on Sha256 digest
ADD explicit lower-hex encoding
PRESERVE non-secret-only contract
PRESERVE public `sha256:` prefix
PRESERVE SHA-256 algorithm
REVIEW canonical field coverage
```

## 13.2 `crates/sitolo-config/src/lib.rs`

Required:

```text
ADD stronger fingerprint contract tests
```

Possible tests:

```text
fingerprint_is_deterministic
fingerprint_has_sha256_prefix
fingerprint_has_64_lower_hex_digits
fingerprint_changes_when_non_secret_db_setting_changes
fingerprint_does_not_include_secret_value
fingerprint_changes_when_secret_reference_changes
```

## 13.3 `crates/sitolo-config/Cargo.toml`

Preferred result:

```text
NO CHANGE
```

The current `sha2.workspace = true` dependency is sufficient.

## 13.4 `Cargo.toml`

Preferred result:

```text
NO CHANGE
```

Do not change the workspace SHA-256 version merely to make formatting compile.

## 13.5 `Cargo.lock`

Preferred result:

```text
NO semantic dependency change
```

If only source changes are made, the lockfile should not need regeneration. If a dependency is added for an intentionally approved shared hex primitive, update the lockfile and include that dependency change in the review.

## 13.6 `scripts/ci/verify`

Preferred result:

```text
NO CHANGE
```

The verifier is doing the right thing by catching the defect.

---

# 14. Acceptance Criteria

The incident is resolved only when all of the following are true.

```text
[ ] Rust 1.98.1 remains the toolchain.
[ ] sha2 0.11 remains the SHA-256 dependency line.
[ ] sitolo-config compiles.
[ ] `LowerHex` is no longer applied directly to the digest container.
[ ] Fingerprint output remains `sha256:<64 lowercase hex chars>`.
[ ] Fingerprint is deterministic.
[ ] Fingerprint excludes raw secret values.
[ ] Secret rotation with the same SecretRef leaves fingerprint unchanged.
[ ] Changing the SecretRef changes the fingerprint.
[ ] Effective fingerprint fields are explicitly audited.
[ ] Database acquisition timeout is handled consistently with the Phase 2 contract.
[ ] Canonicalization changes are not silently mixed into the compile fix.
[ ] Unit tests pass.
[ ] Clippy passes with `-D warnings`.
[ ] `cargo check --locked` passes.
[ ] release build passes.
[ ] architecture check passes.
[ ] Phase 2 policy check passes.
[ ] dependency/license policy remains intact.
[ ] full `./scripts/ci/verify` passes.
```

---

# 15. Implementation-Agent Directive

The implementation agent SHALL execute the following sequence and SHALL NOT substitute a weaker workaround.

### Step 1 — Inspect

Read:

```text
crates/sitolo-config/src/fingerprint.rs
crates/sitolo-config/src/lib.rs
crates/sitolo-config/src/model.rs
crates/sitolo-config/src/validate.rs
Cargo.toml
Cargo.lock
scripts/ci/verify
docs/phase2_config_secrets_logging_errors_telemetry_implementation.md
docs/phase2_postgresql_runtime_configuration_persistence_boundary_implementation.md
docs/database_design.md
docs/system_architecture_design.md
docs/ADR-001-025.md
```

The purpose is to preserve the existing architecture rather than creating a parallel fingerprint convention.

### Step 2 — Prove the dependency cause

Confirm the resolved `sha2`/`digest`/`hybrid-array` types and verify that direct `LowerHex` is unavailable.

### Step 3 — Apply the minimal code fix

Replace only the representation boundary:

```text
Digest container
        |
        v
byte slice
        |
        v
lowercase hexadecimal string
```

### Step 4 — Expand tests

Add contract tests for width, lowercase encoding, deterministic behavior, and secret exclusion.

### Step 5 — Audit field coverage

Compare every `AppConfig` field with the fingerprint contract and explicitly resolve omissions.

### Step 6 — Run the complete CI verifier

Execute:

```bash
./scripts/ci/verify
```

### Step 7 — Report evidence

The implementation report MUST include:

```text
changed files
exact compiler error resolved
dependency versions used
test results
clippy result
full verifier result
fingerprint contract changes, if any
whether the fingerprint output format changed
```

No statement such as “fixed” is acceptable without build/test evidence.

---

# 16. Recommended Minimal Patch

Where the canonical fingerprint representation is intentionally preserved, the narrowest safe source change is conceptually:

```diff
-    format!("sha256:{:x}", Sha256::digest(data.as_bytes()))
+    let digest = Sha256::digest(data.as_bytes());
+    let mut fingerprint = String::with_capacity(digest.as_ref().len() * 2);
+
+    for byte in digest.as_ref() {
+        use std::fmt::Write as _;
+        write!(&mut fingerprint, "{byte:02x}").expect("writing to String cannot fail");
+    }
+
+    format!("sha256:{fingerprint}")
```

This patch addresses exactly the compiler complaint: it no longer requests `LowerHex` on the `Array` itself. It instead formats each `u8`, which is a standard `LowerHex`-supporting primitive.

The compiler's current diagnostic explicitly indicates the absence of `LowerHex` for the digest container and lists ordinary integer types among valid implementations. fileciteturn8file0L15-L24

---

# 17. Why This Is the Correct Boundary

A digest is binary data. A fingerprint is text.

The cryptographic layer's responsibility is:

```text
bytes -> digest bytes
```

The representation layer's responsibility is:

```text
digest bytes -> hexadecimal text
```

The application contract's responsibility is:

```text
hex text -> stable fingerprint identifier
```

Keeping these boundaries separate yields a stronger implementation because a future digest type change does not force a formatting convention change.

The application code should depend on the stable property:

```text
AsRef<[u8]>
```

rather than on a concrete debug/formatting implementation of a cryptographic container.

That is particularly valuable in a multi-crate workspace using current cryptographic crates, where internal representation types may evolve while the SHA-256 algorithm and byte output contract remain stable.

---

# 18. PostgreSQL Relationship

This incident is adjacent to the PostgreSQL work but must not be misclassified as a PostgreSQL problem.

The current CI trace never reaches a PostgreSQL connection attempt. It fails while checking `sitolo-config`, before the repository's persistence work is exercised. The repository therefore has two separate tracks:

```text
TRACK A — unblock CI
    fingerprint compilation
    tests
    complete verifier

TRACK B — implement PostgreSQL persistence
    SQLx
    pool
    migrations
    schema
    roles
    RLS
    real PostgreSQL integration tests
```

The Phase 2 PostgreSQL implementation document intentionally keeps these boundaries distinct. It states that no database migration directory currently exists and that this is expected because schema/migration work belongs to Phase 5. fileciteturn9file1L224-L252

Therefore:

```text
DO NOT introduce PostgreSQL merely to solve this CI compiler error.
DO NOT introduce SQLx to solve this CI compiler error.
DO NOT create migrations to solve this CI compiler error.
DO NOT bypass the fingerprint to make PostgreSQL work appear to progress.
```

The correct sequence is to restore a trustworthy green Phase 2 build first.

---

# 19. Operational and Security Consequences of Leaving the Defect Unfixed

A compiler failure in configuration infrastructure blocks more than a convenience function.

`AppConfig` is a platform boundary consumed by later phases. The Phase 2 contract requires configuration to be deterministic, validated, redacted, fingerprintable, and safe to observe. fileciteturn8file6L478-L482

Leaving the defect unfixed creates:

```text
no trusted green build
no trustworthy release artifact
no reliable Phase 2 evidence
no safe handoff into Phase 3
higher temptation to weaken CI
higher probability of configuration divergence
```

More importantly, a rushed workaround can create future security defects if developers react by:

```text
hashing a connection string
including passwords
adding credentials to logs
using debug formatting
adding unreviewed dependencies
removing CI gates
```

The right response is therefore a precise representation-layer correction with tests.

---

# 20. Future-Proofing Recommendations

## 20.1 Standardize binary-to-text encoding

If the wider Sitolo workspace later needs many binary-to-text transformations, establish one reviewed primitive for:

```text
lowercase hexadecimal
base64url
canonical binary identifiers
```

Do not let each crate invent its own encoding policy.

## 20.2 Establish a fingerprint registry

A future platform registry should record:

```text
fingerprint name
algorithm
canonicalization version
input field set
secret exclusion policy
output format
consumer list
rotation behavior
migration policy
```

## 20.3 Add format compatibility tests

If the fingerprint ever becomes externally consumed, add fixtures such as:

```text
configuration fixture
canonical bytes
expected SHA-256
expected public fingerprint
```

These fixtures make accidental canonicalization drift visible.

## 20.4 Keep cryptographic primitives boring

For security-sensitive platform infrastructure, the preferred implementation is not the most clever one. It is the one that:

```text
is obvious
has a small API
uses well-understood primitives
has deterministic tests
has minimal dependencies
fails loudly when contracts change
```

The current byte-oriented encoding approach satisfies these properties.

---

# 21. Source and Research Basis

The remediation was based on the actual Sitolo CI evidence and the current RustCrypto API documentation.

### Repository evidence

The CI log establishes:

```text
Rust 1.98.1
sha2 0.11.0
digest 0.11.3
hybrid-array 0.4.14
E0277 at crates/sitolo-config/src/fingerprint.rs:25
```

and shows the exact failing `format!("sha256:{:x}", Sha256::digest(...))` expression. fileciteturn9file0L85-L98 fileciteturn9file0L118-L148

### RustCrypto digest API

Current `digest` documentation states that the SHA-256 result is represented as `Array<u8, U32>` in the modern API. citehttps://docs.rs/digest/latest/digest/

### SHA-2 implementation

Current `sha2 0.11` documentation confirms the `Sha256::digest` one-shot API and demonstrates the known SHA-256 test vector used above. citehttps://docs.rs/sha2/0.11.0/sha2/

### Hybrid array representation

Current `hybrid-array` documentation exposes `as_slice()` and `AsRef<[T]>`, supporting an explicit byte-oriented encoding boundary. citehttps://docs.rs/hybrid-array/latest/hybrid_array/struct.Array.html

### Optional dedicated hex crate

The current `hex` crate documentation confirms that `hex::encode` accepts `AsRef<[u8]>` and produces lowercase hexadecimal text. It is a valid option if the workspace later standardizes on a shared hex dependency. citehttps://docs.rs/hex/latest/hex/fn.encode.html

---

# 22. Final Decision Record

### Decision

Fix the configuration fingerprint compiler failure at the digest-byte-to-text boundary.

### Chosen approach

Explicitly encode SHA-256 output bytes as lowercase hexadecimal.

### Dependencies

Keep the existing `sha2 0.11` workspace dependency.

### Toolchain

Keep Rust `1.98.1`.

### Security

Preserve the rule that fingerprints contain only non-secret effective configuration and `SecretRef` metadata where contractually required.

### Compatibility

Preserve the existing public `sha256:<hex>` representation unless an explicit fingerprint-format migration is approved.

### Testing

Add deterministic, structural, encoding, secret-exclusion, and field-coverage regression tests.

### CI

Do not weaken any verification gate. The objective is a genuinely green `./scripts/ci/verify`, not a green check produced by skipping the failing policy.

### Architectural boundary

This fix does not introduce PostgreSQL, SQLx, migrations, RLS, authentication, or domain logic.

---

# 23. Definition of Done for This Incident

The issue is considered **SOLVED** only when the following statement is factually true:

> `sitolo-config` compiles under the repository's declared Rust 1.98.1 toolchain and locked dependency graph, the configuration fingerprint produces the required lowercase hexadecimal SHA-256 representation without directly requiring `LowerHex` on the digest container, raw secret values cannot enter the fingerprint, the fingerprint contract is covered by tests, and the complete repository verification pipeline passes without disabled or bypassed gates.

Anything less is partial remediation.

---

# 24. One-Line Root Cause

```text
The code assumed the SHA-256 digest container implemented `LowerHex`; sha2 0.11 returns a hybrid-array digest representation that must be explicitly encoded as bytes before hexadecimal formatting.
```

# 25. One-Line Fix

```text
Hash the canonical non-secret bytes as before, then encode the resulting digest bytes explicitly to lowercase hexadecimal instead of formatting the digest container with `{:x}`.
```

