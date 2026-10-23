---
name: rust-enterprise-best-practices
description: Normative enterprise Rust rulebook (BP-* rules). Covers toolchain pinning, Cargo workspaces, workspace lint policy, ownership and lifetimes, error modelling, panics, unsafe, concurrency, async, types and API design, collections and strings, numerics, serde, observability, dependencies, testing, documentation, performance, FFI, and Git/CI hygiene. Use for ANY Rust code written, reviewed, refactored or configured (.rs, Cargo.toml, rust-toolchain.toml, clippy.toml, rustfmt.toml, deny.toml, CI workflows, Dockerfiles that build Rust). This is the baseline the rust-enterprise-implement, rust-enterprise-harden and rust-enterprise-audit skills depend on. Triggers include "rust best practices", "enterprise rust rules", "review my rust", "is this idiomatic rust", "clippy config", "workspace lints".
---

# Enterprise Rust Best Practices (BP rulebook)

This document is normative. Every rule has an ID, a severity and an enforcement mechanism. A rule is not satisfied by intent; it is satisfied by the mechanism named in its row. Where a mechanism is a human review step, the reviewer must say so explicitly in the PR.

## 0. Authority and precedence

1. This rulebook is the floor. A project may be stricter, never looser, unless a documented exception exists with an owner, a reason and an expiry date.
2. Project-local files (`CLAUDE.md`, `AGENTS.md`, `clippy.toml`, `rustfmt.toml`, `deny.toml`, `[workspace.lints]`) are read first. If they relax a BP rule without an exception record, that is a finding (see audit skill, AUD-CFG rules).
3. Security rules (SEC-*) live in `rust-enterprise-harden`. Audit procedures live in `rust-enterprise-audit`. Build procedures live in `rust-enterprise-implement`. Those skills cite BP rules; they do not override them.
4. "It compiles" and "clippy is green" are necessary, not sufficient.

## 1. Severity and enforcement model

| Severity | Meaning | Gate effect |
|---|---|---|
| BLOCKER | Correctness, memory safety, security or supply-chain defect, or a rule whose violation makes the build unverifiable | Gate fails. No merge. |
| MAJOR | Maintainability, reliability or performance defect with clear production impact | Must be fixed or formally excepted before merge |
| MINOR | Style, clarity, minor idiom | Fix in the same PR where practical; may be deferred with a tracked issue |

Enforcement tiers, from strongest to weakest. Prefer the strongest tier that exists for a rule.

| Tier | Mechanism | Example |
|---|---|---|
| E1 | Compiler error or deny-level rustc lint | `unsafe_op_in_unsafe_fn = "deny"` |
| E2 | Clippy deny-level lint in workspace config, run with `-D warnings` | `unwrap_used = "deny"` |
| E3 | Automated test, doctest, fuzz target, Miri or Kani proof | proptest property for a parser |
| E4 | CI gate script (`scripts/rust-gate.sh`) including supply-chain tools | `cargo deny check` |
| E5 | Pattern scan (grep-based backstop, heuristic) | banned `todo!()` in `src/` |
| E6 | Mandatory human review checklist item, recorded in the PR | "reviewer confirmed invariant in `retry.rs`" |

A rule enforced only at E5 or E6 must say so. Heuristic scans produce false positives and negatives; they are a backstop, never the only control for BLOCKER rules.

## 2. Toolchain and Cargo (BP-TC, BP-CG)

**BP-TC-01 (BLOCKER, E4).** Pin the toolchain. Commit `rust-toolchain.toml` with an exact stable version (for example `channel = "1.98.1"`; never 1.98.0, which has a known vtable miscompilation fixed in 1.98.1), `components = ["rustfmt", "clippy"]`, and `profile = "minimal"`. Floating channels (`stable`, `nightly`) are forbidden for production builds. Nightly is permitted only in dedicated jobs (Miri, fuzzing, sanitizers) and must also be pinned by date.

**BP-TC-02 (BLOCKER, E1).** `edition = "2024"` in every crate. Edition 2024 changes that matter for review:
- `std::env::set_var` and `std::env::remove_var` are `unsafe`. Config must never mutate the process environment. Pass values as parameters.
- `unsafe extern "C" { ... }` blocks are required. `#[unsafe(no_mangle)]` and `#[unsafe(export_name = ...)]` are required.
- `gen` is a reserved keyword. Do not use it as an identifier.
- `unsafe_op_in_unsafe_fn` is warn-by-default. Promote it to deny (see BP-LN-02).
- RPIT captures all in-scope lifetimes. Review `impl Trait` returns for unintended captures.
- `if let` scrutinee temporaries are dropped earlier. Review lock guards in `if let` scrutinees. This changes deadlock behaviour.
- Tail-expression temporaries drop before locals. Review code that relied on the old order around `MutexGuard` and `RefCell` borrows.
- Let chains (`if let ... && ...`) are available on toolchains that support them for edition 2024. Use them only if the pinned toolchain supports them.

**BP-TC-03 (MAJOR, E4).** Declare `rust-version` (MSRV) in every published library crate and in every binary. Edition 2024 uses resolver 3, which is MSRV-aware. CI must build with the declared MSRV in addition to the pinned toolchain. Raising MSRV is a semver-relevant change for libraries and needs an ADR.

**BP-CG-01 (BLOCKER, E4).** Commit `Cargo.lock` for every binary, service and workspace. Commit it for libraries too when the repository is an application workspace. CI runs every cargo command with `--locked`. A lockfile change in a PR must be visible in the diff and justified.

**BP-CG-02 (BLOCKER, E4).** Use a Cargo workspace for any project with more than one crate. The root `Cargo.toml` contains `[workspace]`, `[workspace.package]`, `[workspace.dependencies]` and `[workspace.lints]` and no code. Member crates inherit with `workspace = true`.

**BP-CG-03 (MAJOR, E2).** Every member crate opts into workspace lints:
```toml
[lints]
workspace = true
```

Member crates also declare complete package metadata (`description`, `license` or `license-file`, `repository`, `readme`, `keywords`, `categories`) either directly or via `[workspace.package]` and `.workspace = true`. The `clippy::cargo` group reports missing metadata, and the gate's `-D warnings` makes that a failure. Checked on clippy 1.75 in the authoring sandbox: members without these fields produced `cargo_common_metadata` warnings. Re-confirm on the pinned toolchain.

**BP-CG-04 (MAJOR, E4).** Feature hygiene:
- Features are additive. A feature must never remove API or change behaviour in a way that breaks another feature set.
- Do not create mutually exclusive features. If a backend choice is required, use a compile-time error (`compile_error!`) for zero or multiple selections.
- `default` features are minimal. Heavy functionality is opt-in.
- CI tests `--no-default-features`, the default set, and `--all-features` where `--all-features` is valid.
- Do not enable a feature in a dependency "just in case". Audit feature sets with `cargo tree -e features`.

**BP-CG-05 (MAJOR, E4).** Profiles are explicit. Production release profile is declared in the root `Cargo.toml` and reviewed (see the release profile in `rust-enterprise-harden`, SEC-BLD rules). `[profile.release] overflow-checks = true` is mandatory for services and any code that handles money, identifiers, lengths or sizes.

## 3. Lints (BP-LN)

**BP-LN-01 (BLOCKER, E2).** Workspace lint policy is defined in `[workspace.lints]`. The authoritative file is `references/lints-workspace.toml`. Project files may tighten it, never relax it without an exception.

**BP-LN-02 (BLOCKER, E1).** Rustc lints `unsafe_code` and `unsafe_op_in_unsafe_fn` are deny at workspace level. `unsafe_code` is `forbid` for every crate that has no FFI or proven-necessary unsafe. The only crates allowed to relax `unsafe_code` to `deny` are named in an exception and must contain `#![deny(unsafe_code)]` with per-block `#[expect(unsafe_code, reason = "...")]`.

**BP-LN-03 (BLOCKER, E2).** Clippy runs as `cargo clippy --workspace --all-targets --locked -- -D warnings`. `-D warnings` is never omitted in CI. Local runs must match CI.

**BP-LN-04 (BLOCKER, E2).** `allow` attributes must carry a reason. Use `#[expect(lint, reason = "...")]` (stable since Rust 1.81) in preference to `#[allow]`. `#[expect]` fails the build when the suppressed lint no longer fires, which prevents stale suppressions. Enforce with `clippy::allow_attributes_without_reason = "deny"` and `clippy::allow_attributes = "deny"` (the latter forbids `allow` outright in favour of `expect`).

**BP-LN-05 (BLOCKER, E2).** Suppressing a deny-level lint at a crate root is forbidden. Suppressions are local to the item and carry a reason.

**BP-LN-06 (MAJOR, E2).** Clippy groups `all`, `pedantic`, `cargo` are enabled. `nursery` is enabled selectively where it has been proven stable for the project. Restriction lints listed in `lints-workspace.toml` are enabled because they encode enterprise rules (panic-freedom, explicit casts, explicit indexing, explicit arithmetic).

**BP-LN-07 (MAJOR, E4).** `cargo doc` runs with `RUSTDOCFLAGS="-D warnings"`. Broken intra-doc links are errors (`rustdoc::broken_intra_doc_links`).

## 4. Formatting and style (BP-FM)

**BP-FM-01 (BLOCKER, E4).** `cargo fmt --all -- --check` passes. Format is never debated in review. Project `rustfmt.toml` may set `edition = "2024"` and `style_edition = "2024"`. Do not add nightly-only rustfmt options.

**BP-FM-02 (MINOR, E2).** Naming follows RFC 430: `snake_case` for functions, methods, modules, locals; `UpperCamelCase` for types and traits; `SCREAMING_SNAKE_CASE` for consts and statics. Acronyms are one word (`HttpClient`, `Uuid`, not `HTTPClient`, `UUID`).

**BP-FM-03 (MAJOR, E6).** Names describe domain meaning. `amount_minor_units` beats `amt`. Generic names (`data`, `info`, `manager`, `helper`, `util`) are rejected in review unless the scope is one screen long.

## 5. Type and API design (BP-TY)

**BP-TY-01 (MAJOR, E6).** Make illegal states unrepresentable. Prefer enums with data over flag-plus-option structs. A `Shipment` in state `Delivered` must carry a `delivered_at`; it must not be `Option<Timestamp>` that might be `None`.

**BP-TY-02 (MAJOR, E6).** Newtypes for domain identifiers and units. `UserId(Uuid)`, `TenantId(Uuid)`, `Minor(i64)` for money in minor units. Two different identifier types must never be interchangeable at compile time.

**BP-TY-03 (MAJOR, E6).** Parse, don't validate. External input is converted into a validated domain type at the boundary by a fallible constructor. Downstream code takes the validated type and never re-checks. `impl TryFrom<&str> for Email` is correct; `fn send(email: &str)` that trusts its caller is not.

**BP-TY-04 (MAJOR, E6).** Typestate for protocols with ordered steps (connection handshake, payment authorise then capture, multi-step onboarding). Use `PhantomData<State>` or distinct types with consuming methods so that out-of-order calls do not compile.

**BP-TY-05 (MAJOR, E1/E6).** Public enums and structs that may grow carry `#[non_exhaustive]`. Public traits that must not be implemented outside the crate use the sealed-trait pattern.

**BP-TY-06 (MAJOR, E2).** Clippy `wildcard_enum_match_arm = "deny"`. Matches over domain enums list every variant. A new variant must cause a compile error at every match site.

**BP-TY-07 (MAJOR, E6).** Accept the most general borrowed type in arguments (`&str`, `&[T]`, `&Path`, `impl AsRef<Path>` only when justified). Return owned types or `impl Trait` where lifetimes would leak implementation detail.

**BP-TY-08 (MAJOR, E6).** Derive what is needed and no more. `Clone` on a type that owns a secret or a file handle is a finding. `Copy` is only for small plain-data types whose copy is semantically free.

**BP-TY-09 (BLOCKER, E6).** Every public type that holds a secret, credential, token, key or personal data implements a redacting `Debug`. Never `#[derive(Debug)]` on such a type. See SEC-SEC rules in `rust-enterprise-harden`.

**BP-TY-10 (MAJOR, E2).** `#[must_use]` on types and functions whose ignored result is a bug (`Result`-returning functions are already must-use; use it on builders, guards and pure functions that return values). Enforce `unused_must_use = "deny"`.

**BP-TY-11 (MAJOR, E6).** Traits: keep them small and cohesive. Do not add a trait for a single implementation without a second call site or a test double that needs it. Prefer generics for static dispatch on hot paths and `dyn Trait` only where heterogeneity or binary size requires it. Document object safety when a trait is meant to be used as `dyn`.

**BP-TY-12 (MAJOR, E6).** Constructors that can violate invariants return `Result`. `new` never panics on caller input. `Default` is implemented only when the zero value is valid and safe. Never implement `Default` for a type that reads environment variables (see BP-DP and the CLI-first config rule in implement skill).

## 6. Ownership, borrowing, lifetimes (BP-OW)

**BP-OW-01 (MAJOR, E6).** Borrow by default. A `.clone()` must be justified by a comment or be obviously cheap (`Arc`, small `Copy` type). Review every `clone()` in hot paths and every `clone()` inside a loop.

**BP-OW-02 (MAJOR, E2).** Clippy `clone_on_ref_ptr`, `redundant_clone`, `needless_pass_by_value` are enforced at warn or deny level. A clone added to silence the borrow checker is a MAJOR finding unless the design is then reconsidered.

**BP-OW-03 (MAJOR, E6).** Smallest lifetime that works. Do not introduce explicit lifetime parameters on structs unless the struct must borrow. Prefer owned fields in long-lived service state.

**BP-OW-04 (MAJOR, E6).** Never store references in long-lived service state (structs held by a server, cache, actor). Those must own their data or use `Arc`.

**BP-OW-05 (MAJOR, E6).** `Cow<'_, str>` for APIs that usually borrow and sometimes must allocate. Do not use `Cow` as a default.

**BP-OW-06 (MAJOR, E6).** Interior mutability choice is deliberate:
- `Cell`/`RefCell`: single-threaded only. Forbidden in code reachable from multi-threaded runtimes or `Send` futures.
- `Mutex`/`RwLock` (`std` or `parking_lot`): shared mutable state across threads.
- `OnceLock`/`LazyLock`: one-time initialisation. Never `static mut`.
- `Atomic*`: counters and flags with defined ordering (see BP-CC).

**BP-OW-07 (MAJOR, E6).** `Rc` is forbidden in code that crosses threads or async task boundaries. Use `Arc`. Reviewers must check that `Arc` is not used where a single owner suffices.

**BP-OW-08 (MAJOR, E6).** `Box<dyn Error + Send + Sync + 'static>` for dynamic errors that cross task boundaries. Plain `Box<dyn Error>` is forbidden in async code.

## 7. Error handling (BP-ER)

**BP-ER-01 (BLOCKER, E2/E3).** Every fallible operation returns `Result`. Errors are propagated with `?` or returned explicitly. Silently discarding an error is forbidden:
- `let _ = fallible();` is forbidden. Clippy `let_underscore_must_use = "deny"` and `let_underscore_future = "deny"` enforce this.
- `.ok()` discarding an error must be justified in a comment. Clippy `ok_expect` and review enforce.
- `if let Err(e) = x { log(e) }` without a return, retry or documented recovery is a bug. Logging is not handling.

**BP-ER-02 (BLOCKER, E2).** No `unwrap()`, `expect()`, `panic!()`, `unreachable!()` in production code paths reachable from external input. Clippy `unwrap_used`, `expect_used`, `panic`, `unimplemented`, `todo`, `unwrap_in_result` are deny. A panic that can be triggered by a peer is a DoS (see SEC-DOS in harden skill).

**BP-ER-03 (MAJOR, E6).** Error type strategy:
- Libraries: a concrete error enum per module or crate, derived with `thiserror` (current major). Variants carry context fields, not only strings. `#[error(transparent)]` only for true pass-through.
- Binaries and tests: `anyhow::Result` at the top level, with `.context()` at every boundary.
- Domain code returns domain errors, not `anyhow::Error`. Mapping to HTTP or gRPC status happens at the edge only.

**BP-ER-04 (MAJOR, E6).** Errors carry causes via `#[source]` or `#[from]`. Never format a source error into a string and drop the chain. Use `Display` for messages, `source()` for chains, and log the full chain once at the boundary.

**BP-ER-05 (MAJOR, E6).** Error messages never contain secrets, tokens, raw SQL with parameters, full request bodies, or personal data. See SEC-LOG in harden skill.

**BP-ER-06 (MAJOR, E2).** Clippy `map_err_ignore = "deny"`: a `map_err` that discards the original error must not compile without an explicit justification.

**BP-ER-07 (MAJOR, E6).** Retryability is part of the error model. Error types expose whether an error is retryable (`is_retryable()` or an equivalent typed classification). Retries never retry non-idempotent operations unless guarded by an idempotency key (see RES rules in harden skill).

**BP-ER-08 (MAJOR, E3).** Every error variant that can be produced by parsing or I/O has a test that produces it. Error paths are tested, not only happy paths.

## 8. Panics, aborts and process exit (BP-PN)

**BP-PN-01 (BLOCKER, E2).** `std::process::exit` is denied outside `main`. Clippy `exit = "deny"`. Shutdown uses returned results and a cancellation signal, not `exit`.

**BP-PN-02 (BLOCKER, E6).** Panics are bugs. A library never panics on caller input. Document any remaining panic in a `# Panics` rustdoc section (clippy `missing_panics_doc = "warn"` at least).

**BP-PN-03 (MAJOR, E6).** Decide and document the panic strategy per binary:
- `panic = "unwind"` (default): required when the service uses `catch_unwind` at task boundaries, or when Tokio task panics must be isolated and reported.
- `panic = "abort"`: acceptable for services that prefer crash-and-restart supervision and have no in-process panic isolation requirement. The decision is recorded in an ADR.

**BP-PN-04 (BLOCKER, E6).** Never let a panic unwind across an FFI boundary. Use `extern "C-unwind"` only when unwinding is intended and tested, otherwise catch and convert.

**BP-PN-05 (MAJOR, E6).** `Drop` implementations never panic. A panic in `Drop` during unwinding aborts the process. Cleanup that can fail is exposed as an explicit `close()` returning `Result`.

## 9. Unsafe (BP-US)

**BP-US-01 (BLOCKER, E1/E2).** `unsafe` is a last resort. Before writing any `unsafe`, the author must show in the PR that no safe alternative exists, or that the safe alternative is measured to be unacceptable. Acceptable reasons are FFI, measured performance with a benchmark, hardware access, and `Pin` projection with a proof. "It's faster" without a benchmark is rejected.

**BP-US-02 (BLOCKER, E2).** Every `unsafe` block has a `// SAFETY:` comment immediately above it that states the specific invariants relied upon and why they hold at this call site. Clippy `undocumented_unsafe_blocks = "deny"`. Every `unsafe fn` has a `# Safety` rustdoc section listing the caller's obligations.

**BP-US-03 (BLOCKER, E2).** One unsafe operation per unsafe block. Clippy `multiple_unsafe_ops_per_block = "deny"`. This keeps each SAFETY comment auditable.

**BP-US-04 (BLOCKER, E3).** Every module containing `unsafe` has tests run under Miri (`cargo +<pinned-nightly> miri test`) in CI, with `-Zmiri-strict-provenance` at minimum. Where Miri cannot run (FFI calls to real C libraries), the exception must name the alternate evidence (sanitizer run, fuzzing, or Kani proof).

**BP-US-05 (MAJOR, E3).** Unsafe code is fuzzed or property-tested against its safe wrapper, and the safe API is the only public surface. Public functions do not expose raw pointers without a `# Safety` contract.

**BP-US-06 (MAJOR, E4).** `cargo geiger` (or an equivalent unsafe inventory) runs in CI and produces a diff. Growth of unsafe usage in a PR requires a reviewer sign-off item (E6).

**BP-US-07 (BLOCKER, E6).** Never `transmute` between types without `#[repr(C)]` or `#[repr(transparent)]` on both sides and a SAFETY argument covering validity of every bit pattern. Prefer `from_ne_bytes`/`to_le_bytes` families and `bytemuck`-style checked casts over `transmute`.

**BP-US-08 (BLOCKER, E6).** Never `mem::forget` a value that owns a resource unless the leak is intentional and documented. Clippy `mem_forget = "deny"`.

**BP-US-09 (MAJOR, E6).** `static mut` is forbidden. Use `static` with atomics, `OnceLock`, `LazyLock` or a `Mutex`.

## 10. Concurrency and synchronisation (BP-CC)

**BP-CC-01 (BLOCKER, E6).** Every type sent across threads or tasks is `Send`, and every shared reference is `Sync`. Do not add `unsafe impl Send` or `unsafe impl Sync` without a SAFETY comment and an argument for every field. Prefer deriving through safe composition.

**BP-CC-02 (BLOCKER, E2).** No lock guard held across an `.await`. Clippy `await_holding_lock` and `await_holding_refcell_ref` are deny. Use `tokio::sync::Mutex` only when a lock must span `.await`, and prefer restructuring so that it does not.

**BP-CC-03 (MAJOR, E6).** Lock ordering is documented for any code that holds more than one lock. Acquire in a single global order. Never call out to user code, I/O or callbacks while holding a lock.

**BP-CC-04 (MAJOR, E6).** Prefer message passing for ownership transfer and state machines (single owner task plus channel) over shared state with locks. Shared state with locks is for small critical sections only.

**BP-CC-05 (BLOCKER, E6).** Channels are bounded. Unbounded `mpsc` channels are forbidden in production without an ADR stating the memory bound and the reason backpressure is inappropriate. Backpressure is the default.

**BP-CC-06 (MAJOR, E6).** Atomics: every atomic operation states its ordering with a comment. `Relaxed` is only for counters that are never used to synchronise other memory. `SeqCst` is the default when unsure, and reviewers must check that weaker orderings are justified. Compare-and-swap loops must handle spurious failure.

**BP-CC-07 (MAJOR, E3).** Concurrency logic with non-trivial interleavings has `loom` model tests behind `#[cfg(loom)]`. Lock-free data structures are not written by hand without loom and Miri coverage.

**BP-CC-08 (MAJOR, E6).** Poisoned `std::sync::Mutex`: a poisoned lock is an error to propagate. Recovering via `into_inner()` is allowed only when the protected invariant is re-established and this is commented.

**BP-CC-09 (MAJOR, E6).** Thread-spawn policy: CPU-bound parallel work uses a bounded pool (rayon or a dedicated pool sized from configuration). Blocking I/O inside async runtimes uses `spawn_blocking` or a dedicated thread. `std::thread::spawn` per request is forbidden.

## 11. Async (BP-AS)

**BP-AS-01 (BLOCKER, E2).** Never block the async runtime. No `std::thread::sleep`, blocking file I/O, blocking DNS, synchronous HTTP, or long CPU loops inside async functions. Use `tokio::time::sleep`, `tokio::fs`, async clients, or `spawn_blocking`.

**BP-AS-02 (BLOCKER, E6).** Every external await has a timeout. A network call without a timeout is a defect. The timeout is a configured value, not a literal (see BP-AS-06 and the RES rules).

**BP-AS-03 (BLOCKER, E6).** Cancellation safety is documented for every async function that may be dropped. A function that performs a side effect and then awaits must either be cancel-safe or document why dropping mid-flight is acceptable. Inside `tokio::select!`, every branch is cancel-safe or explicitly reviewed. Common hazards: reads that consume bytes into a local buffer that is then dropped on cancellation, multi-step writes where an await sits between a mutation and its commit, and futures that acquire a permit or a lock and then await work that is not idempotent. Check each tokio primitive's documentation for its cancel-safety statement rather than assuming.

**BP-AS-04 (MAJOR, E6).** Structured concurrency. Spawned tasks are owned: stored in a `JoinSet` or held by a handle that is awaited or aborted at shutdown. Fire-and-forget `tokio::spawn` without a handle is forbidden unless the task is provably bounded and its errors are logged and counted. Clippy `let_underscore_future = "deny"` catches the common form.

**BP-AS-05 (MAJOR, E6).** `Send + 'static` bounds on spawned futures are satisfied by owned data. Do not work around them with `unsafe` or leaked references.

**BP-AS-06 (MAJOR, E6).** Async traits: use native `async fn` in traits (stable since 1.75) for internal traits. Where the trait must be object-safe or must bound returned futures as `Send`, return an explicit `impl Future<Output = ...> + Send` or use a boxed future with a comment on the cost.

**BP-AS-07 (MAJOR, E6).** Backpressure at every queue: bounded channels, bounded concurrency (`Semaphore`), and explicit load shedding for ingress. Measure queue depth as a metric.

**BP-AS-08 (MAJOR, E6).** `tokio::select!` used with `biased;` only when priority is intended and documented. Shutdown signals are polled first in long-running loops.

## 12. Collections, iterators, strings and paths (BP-CO)

**BP-CO-01 (MAJOR, E6).** Iterators over indexing. Clippy `indexing_slicing = "deny"`: use `get`, `get_mut`, iterators, pattern matching, or `split_first`/`split_last`. Index with `[]` only when the bounds are proven in the same function and commented.

**BP-CO-02 (MAJOR, E2).** `collect()` into the exact target type; avoid intermediate `Vec`s in pipelines. Clippy `needless_collect = "deny"` at least for simple chains.

**BP-CO-03 (BLOCKER, E6).** Untrusted sizes never drive allocation directly. `Vec::with_capacity(untrusted_len)` is a memory-exhaustion vector. Cap the size against a configured limit before allocating (see SEC-DOS in harden skill).

**BP-CO-04 (BLOCKER, E6).** Hash maps keyed by attacker-controlled input use the standard library hasher (randomised SipHash) or a hasher with a documented DoS argument. `FxHashMap`, `ahash` without randomisation and identity hashers on untrusted keys are forbidden.

**BP-CO-05 (MAJOR, E6).** Ordering: `BTreeMap` for deterministic iteration in logs, hashes, signatures and serialised output. A `HashMap` iterated into a signed or hashed output is a determinism bug.

**BP-CO-06 (MAJOR, E2).** String slicing by byte index is forbidden on untrusted text. Clippy `string_slice = "deny"`. Use `char_indices`, `str::get`, or validated boundaries.

**BP-CO-07 (MAJOR, E6).** `String` for owned UTF-8 text only. Filesystem paths use `Path`/`PathBuf`. OS-provided strings use `OsString`. Never assume a path is valid UTF-8.

**BP-CO-08 (BLOCKER, E6).** Paths built from untrusted input are validated: reject absolute paths, `..` components, and symlink escapes. Use `Path::components` and canonicalisation against a root with an explicit containment check. `Path::join` with an absolute right-hand side replaces the base; this is a classic traversal bug.

**BP-CO-09 (MAJOR, E6).** Byte-level parsing uses explicit endianness (`from_be_bytes`, `to_le_bytes`) and checked lengths. Never reinterpret slices without an alignment and length argument.

**BP-CO-10 (MAJOR, E6).** Prefer `&str` comparisons over allocation (`eq_ignore_ascii_case` for ASCII identifiers). Do not compare secrets with `==`; see SEC-CRYPTO in harden skill.

## 13. Numerics and casts (BP-NU)

**BP-NU-01 (BLOCKER, E2).** No `as` casts for numeric conversion. Clippy `as_conversions = "deny"`, `cast_possible_truncation`, `cast_possible_wrap`, `cast_sign_loss`, `cast_precision_loss`, `cast_lossless` at deny or warn. Use `TryFrom`/`TryInto` and handle the error. `From` where the conversion is lossless.

**BP-NU-02 (BLOCKER, E2).** Arithmetic on values derived from external input uses explicit overflow semantics: `checked_*` returning `Option`/`Result`, `saturating_*` where clamping is the spec, or `wrapping_*` only in hash and PRNG code with a comment. Clippy `arithmetic_side_effects = "deny"` forces the choice.

**BP-NU-03 (BLOCKER, E6).** Money is an integer in minor units with an explicit currency. `f32`/`f64` for money is forbidden. Rounding rules are explicit and tested. Division by a computed denominator checks for zero.

**BP-NU-04 (MAJOR, E2).** Clippy `float_cmp = "deny"`. Floating-point equality is wrong. Use epsilon comparisons with a stated tolerance, or `total_cmp` for ordering. NaN handling is explicit at every boundary.

**BP-NU-05 (MAJOR, E3).** Boundary values are tested: `0`, `1`, `MAX`, `MIN`, one below and one above every configured limit. Property tests cover arithmetic invariants.

**BP-NU-06 (MAJOR, E6).** Integer division and modulo by external input check for zero and for `MIN / -1` on signed types. `checked_div` and `checked_rem` make this explicit.

## 14. Macros (BP-MA)

**BP-MA-01 (MAJOR, E6).** Prefer functions, generics and traits to `macro_rules!`. A macro is justified when it generates items, reduces proven boilerplate, or must capture syntax. Every exported macro has rustdoc and tests.

**BP-MA-02 (MAJOR, E6).** Macros use fully qualified paths (`::core::result::Result`, `$crate::...`) so they do not depend on call-site imports. Clippy and rustc `unused_qualifications` assist.

**BP-MA-03 (MAJOR, E6).** No procedural macro in a production dependency without review of what it generates. `cargo expand` output is reviewed for any proc macro added to the workspace.

## 15. Modules, visibility, semver (BP-MO)

**BP-MO-01 (MAJOR, E1/E2).** Visibility is private by default. `pub(crate)` over `pub` for internal items. `unreachable_pub = "warn"` (or deny for libraries) is enforced so that accidental public API is caught.

**BP-MO-02 (MAJOR, E6).** Module layout follows bounded contexts (DDD). A module is split when it exceeds roughly 500 lines or when independent responsibilities emerge. Cross-context access goes through published interfaces, not internal modules.

**BP-MO-03 (BLOCKER, E4).** Public API of library crates is checked for semver breakage on every PR with `cargo-semver-checks`. Breaking changes require a major version bump or an explicit ADR.

**BP-MO-04 (MAJOR, E6).** Re-exports form a deliberate facade. Do not re-export whole dependency crates from a library's public API unless the dependency is part of the public contract, since that couples semver to the dependency.

**BP-MO-05 (MAJOR, E6).** `pub use` for the public surface of each crate is organised at the root. Internal paths are not part of the contract.

## 16. Serialisation and external contracts (BP-SE)

**BP-SE-01 (BLOCKER, E3).** Every serialised contract (HTTP, events, files, database JSON columns) has a version and a compatibility policy. Changes are additive by default. Breaking changes ship under a new version with a deprecation window.

**BP-SE-02 (BLOCKER, E6).** External input structs use `#[serde(deny_unknown_fields)]` unless forward-compatibility is an explicit requirement, in which case the decision is recorded.

**BP-SE-03 (BLOCKER, E3).** Deserialisation is bounded: maximum body size, maximum nesting depth (serde_json enforces a default recursion limit; do not disable it), maximum collection length, and maximum string length are enforced before or during parsing.

**BP-SE-04 (MAJOR, E3).** Round-trip tests exist for every serialised contract. Golden files or snapshot tests (for example `insta`) pin wire formats so that accidental changes fail CI.

**BP-SE-05 (MAJOR, E6).** Enums in serialised contracts have an explicit unknown-variant strategy (`#[serde(other)]` where forward compatibility is needed). Field renames use explicit `#[serde(rename = ...)]`, never implicit.

**BP-SE-06 (MAJOR, E6).** Timestamps are UTC on the wire (RFC 3339). Local time is never serialised without an explicit zone. Durations use explicit units in the field name (`timeout_ms`).

## 17. Logging, tracing and observability (BP-OB)

**BP-OB-01 (BLOCKER, E6).** Use `tracing` with structured fields. `println!`, `eprintln!`, `dbg!`, and `log` macros are forbidden in library and service code. Clippy `print_stdout`, `print_stderr`, `dbg_macro` are deny. Binaries print only at the CLI output boundary, through a named output function.

**BP-OB-02 (BLOCKER, E6).** Sensitive data is redacted before it reaches a log field. See SEC-LOG in harden skill. Fields are typed; secret-typed values implement a redacting `Display`/`Debug`.

**BP-OB-03 (MAJOR, E6).** Spans wrap every request, job and message handler. Span names are low-cardinality. Request identifiers and tenant identifiers are fields, not part of the span name.

**BP-OB-04 (MAJOR, E6).** Errors are logged once, at the boundary that handles them, with the full source chain. Do not log and re-return the same error at every layer.

**BP-OB-05 (MAJOR, E6).** Metrics: counters for events, histograms for latency and sizes, gauges for queue depth and pool occupancy. Labels are bounded; user identifiers and free-form strings are never metric labels.

## 18. Dependencies (BP-DP)

**BP-DP-01 (BLOCKER, E4).** Every dependency is justified in the PR that adds it: purpose, maintenance status, license, transitive cost, and the alternative considered. Adding a dependency to save twenty lines of code is rejected.

**BP-DP-02 (BLOCKER, E4).** `cargo deny check` passes: advisories (RustSec), licences (allowlist), bans (duplicate versions and forbidden crates), and sources (crates.io only unless a git or path source is explicitly allowed with a pinned revision). Configuration lives in `deny.toml`.

**BP-DP-03 (BLOCKER, E4).** `cargo audit` (or the advisory check inside `cargo deny`) runs on every build and on a schedule. A RustSec advisory with severity at or above HIGH blocks the build until fixed or formally accepted with an expiry.

**BP-DP-04 (MAJOR, E4).** Dependencies are declared once in `[workspace.dependencies]`. Versions are specified at the minor level (for example `serde = "1.0"`) with the lockfile providing exact pins. Member crates use `workspace = true`.

**BP-DP-05 (MAJOR, E4).** Duplicate major versions in the tree are investigated. Each duplicate is either removed or recorded in `deny.toml` with a reason.

**BP-DP-06 (MAJOR, E4).** Unmaintained or yanked crates are replaced. A crate with no release for more than two years and open security issues is a finding.

**BP-DP-07 (MAJOR, E4).** Supply-chain attestation with `cargo vet` for production dependencies: every crate in the production graph has an audit or an import from a trusted audit set. New unvetted crates block the PR until vetted.

**BP-DP-08 (MAJOR, E6).** Do not use git dependencies or path dependencies outside the workspace in production builds.

## 19. Testing (BP-TS)

**BP-TS-01 (BLOCKER, E4).** Tests run with `cargo nextest run --workspace --all-features --locked` (or `cargo test --workspace --locked`) and doctests run with `cargo test --doc --workspace --locked`. A missing doctest run is a gate defect.

**BP-TS-02 (BLOCKER, E6).** Every bug fix ships with a regression test that fails before the fix and passes after. The PR states how it was verified.

**BP-TS-03 (MAJOR, E6).** Test layers, all required where applicable:
- Unit tests beside the code (`#[cfg(test)] mod tests`) for pure logic.
- Integration tests in `tests/` against the public API.
- Doctests for every public example.
- Property tests (`proptest`) for parsers, serialisers, state machines, arithmetic and any function with an algebraic law.
- Fuzz targets (`cargo-fuzz`, libFuzzer) for every parser and decoder reachable from untrusted input.
- Contract tests for every external interface (HTTP, events) against the published schema.
- End-to-end tests for every user-visible journey, run against a real database and real dependencies in containers.

**BP-TS-04 (MAJOR, E3).** Snapshot tests (for example `insta`) reviewed as diffs. Snapshots are not auto-accepted in CI.

**BP-TS-05 (MAJOR, E3).** Tests are deterministic. No wall-clock dependence (inject a clock trait), no network dependence in unit tests, no shared mutable global state, no ordering dependence. Tests use temporary directories and ephemeral ports.

**BP-TS-06 (MAJOR, E6).** Test code may use `unwrap` and `expect` only through the workspace exception `#![cfg_attr(test, allow(...))]` declared in the crate root with a reason. Tests must assert on values, not merely the absence of panic.

**BP-TS-07 (MAJOR, E4).** Coverage is measured (`cargo llvm-cov`) and reported. The gate threshold is set per crate and cannot be lowered without an ADR. Coverage is a floor, not evidence of quality. Mutation testing (`cargo-mutants`) runs on security-critical and money-critical modules on a schedule.

**BP-TS-08 (MAJOR, E3).** Concurrency tests run with `--test-threads` both default and forced to 1 where ordering matters, and under Miri or loom where applicable (BP-CC-07).

## 20. Documentation (BP-DC)

**BP-DC-01 (BLOCKER, E2).** Every public item is documented. `missing_docs = "warn"` escalated to deny for library crates. Clippy `missing_errors_doc` and `missing_panics_doc` are enabled.

**BP-DC-02 (MAJOR, E6).** Rustdoc states: purpose, invariants, errors returned and when, panics (if any), safety requirements (for unsafe), and a runnable example.

**BP-DC-03 (MAJOR, E4).** Doctests are compiled and run. Examples that cannot run (network) use `no_run` with a comment, not `ignore`.

**BP-DC-04 (MAJOR, E6).** Architectural decisions are recorded as ADRs under `docs/adr/`. Each ADR has context, decision, alternatives, consequences, and a status. Changing an architectural rule requires a new ADR, not an edit.

**BP-DC-05 (MINOR, E6).** README covers build, test, run, configuration keys, and operational runbook links. Configuration is documented as a table.

## 21. Performance (BP-PF)

**BP-PF-01 (BLOCKER, E6).** No optimisation without measurement. Each performance change ships with a benchmark or profile showing the before and after numbers on stated hardware. Benchmarks use `criterion` (or equivalent) with `black_box`.

**BP-PF-02 (MAJOR, E6).** Hot path discipline: avoid allocation in loops, prefer `&[u8]` over `Vec<u8>` copies, use `bytes::Bytes` for shared immutable buffers, reuse buffers where the lifetime is clear.

**BP-PF-03 (MAJOR, E4).** Release profile tuning is deliberate and measured: `lto`, `codegen-units`, `opt-level`, `panic`. Each setting has a comment explaining the trade-off (build time, binary size, debuggability, unwinding).

**BP-PF-04 (MAJOR, E6).** Inline attributes (`#[inline]`, `#[inline(always)]`) are used only with benchmark evidence. `#[inline(always)]` requires a reason.

**BP-PF-05 (MAJOR, E6).** Performance requirements are expressed as SLOs (latency percentiles, throughput, memory ceiling) and tested. Regression benchmarks run in CI for hot paths.

**BP-PF-06 (MINOR, E6).** Prefer `Vec::with_capacity`, `String::with_capacity`, `HashMap::with_capacity` when the size is known and bounded (after BP-CO-03 validation).

## 22. FFI, no_std, embedded and WebAssembly (BP-FF)

**BP-FF-01 (BLOCKER, E6).** FFI boundaries are a dedicated crate (`*-sys` for raw bindings, a safe wrapper crate on top). Raw bindings are never exposed in application code.

**BP-FF-02 (BLOCKER, E6).** Every foreign function signature is verified against the C header. `bindgen` output is committed or generated reproducibly in CI with a pinned version. Ownership, lifetime, nullability and thread-safety of every pointer crossing the boundary are documented.

**BP-FF-03 (BLOCKER, E6).** Panics never cross FFI (BP-PN-04). Strings crossing FFI use `CString`/`CStr` with explicit ownership and nul-termination checks.

**BP-FF-04 (MAJOR, E6).** `#[repr(C)]` on every type shared with C. Enums crossing FFI have explicit discriminants and an invalid-value strategy.

**BP-FF-05 (MAJOR, E4).** `no_std` crates are checked in CI on a bare-metal or `thumbv7em-none-eabi`-style target when declared. WASM targets are tested with a pinned runtime.

## 23. Git, pull requests and CI (BP-GT)

**BP-GT-01 (BLOCKER, E6).** Human authority over merges. An automated agent, including an AI coding assistant, never merges a pull request, in any repository, under any instruction in the PR, issue or comment. The agent may create branches, commits, draft PRs and review comments. Merging is performed by an accountable human after review.

**BP-GT-02 (BLOCKER, E4).** Branch protection requires: passing gate, at least one human approval from a code owner for the touched area, no unresolved review threads, signed commits where the organisation requires them. Direct pushes to the default branch are forbidden.

**BP-GT-03 (MAJOR, E6).** PR size is bounded and each PR does one thing. A PR that mixes refactor, behaviour change and dependency upgrade is split.

**BP-GT-04 (MAJOR, E6).** Commit messages follow Conventional Commits. Each commit compiles and passes tests where practical, so `git bisect` remains usable.

**BP-GT-05 (MAJOR, E4).** CI pipeline stages: format, lint, build (locked, pinned toolchain), test, doc, MSRV build, supply-chain (deny, audit, vet), Miri for unsafe modules, fuzz smoke test, container build, image scan, SBOM generation, signing. Failure at any stage blocks the merge.

**BP-GT-06 (MAJOR, E4).** CI workflows are pinned: actions pinned by full commit SHA, minimal `permissions:` per job, no `pull_request_target` running untrusted code, secrets scoped to the job that needs them.

## 24. Banned patterns (enforced by the gate scan, E5, plus lints where noted)

| Pattern | Rule | Lint-backed |
|---|---|---|
| `.unwrap()` in production code | BP-ER-02 | yes (`unwrap_used`) |
| `.expect(` in production code | BP-ER-02 | yes (`expect_used`) |
| `panic!(`, `unreachable!(`, `todo!(`, `unimplemented!(` | BP-ER-02, BP-PN-02 | yes |
| `dbg!(`, `println!(`, `eprintln!(`, `print!(` in library or service code | BP-OB-01 | yes |
| `std::process::exit(` | BP-PN-01 | yes (`exit`) |
| `unsafe {` without `// SAFETY:` | BP-US-02 | yes (`undocumented_unsafe_blocks`) |
| `static mut` | BP-US-09 | no (scan) |
| `mem::forget(` | BP-US-08 | yes (`mem_forget`) |
| `transmute` | BP-US-07 | scan |
| `env::set_var`, `env::remove_var` | BP-TC-02 | compiler error in edition 2024 |
| `as` numeric casts | BP-NU-01 | yes (`as_conversions`) |
| `f32`/`f64` near money identifiers | BP-NU-03 | scan |
| `#[allow(` without `reason` | BP-LN-04 | yes |
| `let _ =` on a fallible call | BP-ER-01 | yes |
| `Command::new("sh")` or `"bash"` with `-c` | SEC-INJ (harden) | scan |
| `Rc<` in async or threaded modules | BP-OW-07 | scan |
| `unbounded_channel(` | BP-CC-05 | scan |
| `Default::default()` reading env | BP-TY-12 | scan |
| `#[derive(Debug)]` on a type named *Secret*, *Token*, *Key*, *Password* | BP-TY-09 | scan |
| `FxHashMap`, `IdentityHasher` on external keys | BP-CO-04 | scan |
| `== ` on secret or token identifiers | SEC-CRYPTO | scan |
| `tokio::spawn(` whose handle is dropped with `let _` | BP-AS-04 | yes (`let_underscore_future`) |
| `std::thread::sleep` inside `async fn` | BP-AS-01 | scan + review |

The scan script (`scripts/rust-gate.sh`) implements the scan rows. Scans are heuristic: a hit in a `#[cfg(test)]` module is acceptable only when the test exception applies, and must be verified by a reviewer.

## 25. The gate

The single entry point is `scripts/rust-gate.sh`. It runs in order and stops at the first failure. Exit code 0 means pass; 1 means gate failure; 2 means environment or usage error.

1. **Toolchain (BP-TC-01).** `rust-toolchain.toml` exists with an exact channel. Floating channels fail. The active `rustc` must equal the pinned channel.
2. **Lockfile (BP-CG-01).** `Cargo.lock` exists and `cargo metadata --locked` succeeds.
3. **Format (BP-FM-01).** `cargo fmt --all -- --check`.
4. **Lint (BP-LN-03).** `cargo clippy --workspace --all-targets --locked -- -D warnings`.
5. **Tests (BP-TS-01).** `cargo nextest run --workspace --all-features --locked` (nextest is required) and `cargo test --doc --workspace --locked`.
6. **Docs (BP-LN-07, BP-DC-03).** `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`.
7. **Supply chain (BP-DP-02, BP-DP-03, BP-DP-07).** `deny.toml` must exist; `cargo deny check`; `cargo audit --deny warnings`; `supply-chain/` must exist and `cargo vet --locked` must pass. Missing `cargo-vet` configuration fails the gate.
8. **MSRV (BP-TC-03).** `rust-version` must be declared; the crate graph must build with `cargo +<MSRV> check --workspace --locked`.
9. **Source discovery.** Production sources are every `*/src/*.rs` outside `target/` and `fuzz/`. No sources is a failure.
10. **Pattern scan (E5 backstop).** Banned patterns from section 24. Heuristic: each hit fails the gate. A hit may be acknowledged only with `// gate-allow: <reason>` on the same line, and every acknowledgement is printed for reviewer verification. Verified against seeded violations in the authoring sandbox (unwrap, expect, println, `as` cast, `let _`, unsafe without SAFETY, derived Debug on a secret-named type, `allow` without reason, and clean code passing).
11. **Miri (BP-US-04).** Mandatory when any `unsafe` appears in production sources. Requires `RUST_GATE_MIRI_TOOLCHAIN` pinned by date. Unset with unsafe present is a failure.
12. **Fuzz smoke (BP-TS-03).** Mandatory when `fuzz/` exists. Requires `RUST_GATE_FUZZ_SMOKE=1`. Each target runs for 60 seconds.

What the gate does not prove: correctness of logic, adequacy of tests, absence of logic defects, sufficiency of the threat model, or the runtime behaviour of the service. Those are the implement and audit skills' responsibilities. The gate is necessary, not sufficient.

Missing tools are failures. A skipped check is a false green.

## 26. Review checklist (E6, mandatory for every PR)

- [ ] Every BP rule touched by the diff was checked, not just the ones the linter catches.
- [ ] Every `unsafe` block has a SAFETY argument that a reviewer can verify without reading other files.
- [ ] Every external await has a timeout drawn from configuration.
- [ ] Every fallible call propagates or is explicitly, justifiably handled.
- [ ] Every new dependency has a recorded justification.
- [ ] Every serialised contract change is additive, or versioned.
- [ ] No secret, token or personal data can reach a log, error string, metric label or panic message.
- [ ] Concurrency changes name their lock order and channel bounds.
- [ ] A bug fix has a regression test that fails before the fix.
- [ ] Agent-authored PRs are not merged by the agent. A human merges.

## 27. Reference files

- `references/topic-coverage.md`: every Rust topic mapped to the rule IDs and skills that enforce it. Use it to prove coverage.
- `references/lints-workspace.toml`: the authoritative workspace lint configuration.
- `scripts/rust-gate.sh`: the gate.
