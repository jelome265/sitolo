# Enterprise Rust Templates

Copy these into a new workspace, then replace every `<PLACEHOLDER>`. Placeholders are deliberate: a template that still contains one must not pass the gate. Verify each tool's current configuration schema against the pinned version (`cargo deny --version`, `cargo audit --version`) before relying on it.

## 1. `rust-toolchain.toml` (BP-TC-01)

```toml
[toolchain]
channel = "1.98.1"            # exact pin; 1.98.0 is excluded (vtable miscompilation fixed in 1.98.1)
components = ["rustfmt", "clippy"]
profile = "minimal"
```

Nightly for Miri and fuzzing lives in CI only, pinned by date, for example `nightly-2026-07-01`. It is never the workspace default.

## 2. Root `Cargo.toml` (BP-CG-02, BP-CG-03, BP-CG-05, BP-DP-04)

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.98"          # placeholder MSRV: set to the oldest toolchain your code truly builds with
license = "<LICENSE-SPDX>"
repository = "<REPO-URL>"
readme = "README.md"
authors = ["<TEAM> <CONTACT>"]

[workspace.dependencies]
# Minor-level versions. Verify each with `cargo info <crate>` before adding (BP-DP-01).
thiserror = "2.0"
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "signal", "time"] }
tokio-util = "0.7"
tracing = "0.1"
clap = { version = "4.5", features = ["derive", "env"] }
secrecy = "0.10"
zeroize = { version = "1", features = ["derive"] }
subtle = "2"
# Test-only, kept out of production graphs by [dev-dependencies] in members:
proptest = "1"
insta = "1"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "unwind"              # change to "abort" only with a BP-PN-03 ADR
overflow-checks = true        # mandatory (HRD BLD-REL-01)
debug = "line-tables-only"
strip = "debuginfo"

[profile.dev]
overflow-checks = true

# Lint policy: paste the contents of
# rust-enterprise-best-practices/references/lints-workspace.toml here, or keep it in a
# separate file and mirror it. It must be present in THIS root manifest.
```

Note: the `rust-version` above is a placeholder for the MSRV you actually support. Set it to the oldest toolchain your code and dependencies truly build with, verified in CI.

## 3. Member crate `Cargo.toml` (library)

```toml
[package]
name = "<context>-domain"
description = "<one-line purpose>"
keywords = ["<kw1>", "<kw2>"]
categories = ["<crates.io category>"]
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
readme.workspace = true
authors.workspace = true

[lints]
workspace = true

[dependencies]
thiserror.workspace = true

[dev-dependencies]
proptest.workspace = true
```

Domain crates do not depend on `tokio`, `sqlx`, `axum`, `reqwest`, or any I/O crate (IMP-04).

## 4. `deny.toml` (BP-DP-02, SEC-SUP-02) — cargo-deny configuration

```toml
[graph]
all-features = true

[advisories]
version = 2
# Each ignore needs an id, a reason, and an expiry tracked outside this file.
# Example shape (do not copy the id):
# ignore = [{ id = "RUSTSEC-0000-0000", reason = "<why, owner, expiry date>" }]
ignore = []

[licenses]
version = 2
confidence-threshold = 0.93
# Organisation policy. Extend only through a recorded decision.
allow = [
  "Apache-2.0",
  "MIT",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Unicode-3.0",
  "Zlib",
]

[bans]
multiple-versions = "deny"    # BP-DP-05: duplicates must be removed or listed in `skip` with a reason
wildcards = "deny"            # no "*" version requirements
highlight = "all"
deny = [
  # Crates the organisation forbids. Example: crates with known-unsafe replacements or
  # unmaintained status that have an approved alternative. Add with a reason.
]
skip = []                      # { crate = "name@version", reason = "..." }

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []                 # any git source requires an exception record and a pinned rev (BP-DP-08)
```

## 5. `clippy.toml`, `rustfmt.toml`

```toml
# clippy.toml — framework hard bans (FW-LN-01). Copy rust-enterprise-frameworks/references/clippy.service.toml
# to the workspace root. Lint levels live in [workspace.lints]; this file lists banned paths and their reasons.
# Thresholds, if any, are added below the disallowed lists.
```

```toml
# rustfmt.toml — formatting options only.
edition = "2024"
style_edition = "2024"
```

## 6. CI workflow with pinned actions (BP-GT-05, BP-GT-06)

Pin every action by full commit SHA. Replace each `<SHA>` with the SHA of the tag you reviewed, and keep the tag as a comment.

```yaml
name: enterprise-gate
on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read            # least privilege; widen per job only when required

jobs:
  gate:
    runs-on: ubuntu-24.04
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@<SHA>   # v4.x
        with:
          persist-credentials: false
      - name: Install pinned toolchain tools
        run: |
          # Install the exact toolchain from rust-toolchain.toml, then the tools below at pinned versions.
          # Tools: cargo-nextest, cargo-deny, cargo-audit, cargo-vet, cargo-llvm-cov, cargo-semver-checks
          # Missing tools must fail the gate, never be skipped.
          echo "install with pinned versions (see CI image definition)"
      - name: Gate
        env:
          RUST_GATE_MIRI_TOOLCHAIN: nightly-<YYYY-MM-DD>
          RUST_GATE_FUZZ_SMOKE: "1"
        run: bash rust-enterprise-best-practices/scripts/rust-gate.sh .

  miri:
    if: ${{ hashFiles('**/*.rs') != '' }}
    runs-on: ubuntu-24.04
    timeout-minutes: 60
    steps:
      - uses: actions/checkout@<SHA>
        with:
          persist-credentials: false
      - name: Miri on unsafe modules
        run: echo "cargo +nightly-<YYYY-MM-DD> miri test --workspace --locked"
```

Rules for this file: no `pull_request_target` running PR code; no secrets exposed to pull requests from forks; `persist-credentials: false` on checkout; artifacts signed and provenance-attested in the release workflow (HRD BLD-SIGN, BLD-SBOM). Merge is performed by a human after review; the workflow does not auto-merge (BP-GT-01).

## 7. Fuzz target skeleton (BP-TS-03, BLD-VERIFY-03)

```rust
// fuzz/fuzz_targets/parse_<thing>.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The target must only call the public parser. It must never panic on any input.
    // A panic here is a DoS-class defect (BP-ER-02, SEC-DOS-07).
    // Result is consumed explicitly; `let _ =` is forbidden by BP-ER-01.
    drop(<context>_domain::parse_frame(data));
});
```

Each crash becomes a regression test with the minimised input committed under `tests/regressions/`, and the fix is reviewed (BP-TS-02).

## 8. Property test skeleton (BP-TS-03, BP-NU-05)

```rust
#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use super::*;

    proptest! {
        #[test]
        fn transfer_conserves_total_minor_units(
            from in 0_i64..1_000_000,
            to in 0_i64..1_000_000,
            amount in 1_i64..1_000_000,
        ) {
            // Checked arithmetic even in tests: overflow semantics are part of the property.
            let total_before = from.checked_add(to);
            prop_assert!(total_before.is_some());
            // TODO-in-your-code: apply the real transfer, then assert:
            //   1. the sum of balances is unchanged (conservation), and
            //   2. no balance went negative, and
            //   3. a transfer exceeding `from` returns the InsufficientFunds error.
            let _ = amount; // REPLACE: remove once the transfer call uses `amount`
        }
    }
}
```

The skeleton is incomplete by design: it shows the shape of a property. Complete it with real domain calls before it counts as a test. A property test that asserts nothing meaningful is a finding (AUD Pass P). The `let _ = amount;` line is a placeholder and must be removed when the transfer call is written, because BP-ER-01 forbids `let _ =` in production source.

## 9. Config struct checklist

- [ ] Every field has a `value_parser` or a validated type.
- [ ] Every timeout has a documented default, a minimum, and a maximum.
- [ ] Every secret field is a `SecretString` or `SecretBox`, and the struct has no `derive(Debug)`.
- [ ] `hide_env_values = true` on secret args.
- [ ] No `Default` impl that reads environment variables (BP-TY-12).
- [ ] Startup fails with a clear message on any invalid value (IMP-6.1).
