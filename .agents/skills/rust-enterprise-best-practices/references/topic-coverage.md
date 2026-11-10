# Rust Topic Coverage Matrix

Every topic below is mandatory review scope for enterprise Rust guidance. This means each topic must be considered and mapped to a rule/enforcement mechanism where applicable; it does **not** mean every language feature must be used in every codebase. Record a REQUIRED, USED, NOT APPLICABLE, or DEFERRED disposition with a brief reason for material topics. A topic is covered when at least one rule exists and an enforcement mechanism is named. Skill codes:

- **BP** = rust-enterprise-best-practices (normative rules)
- **IMP** = rust-enterprise-implement (build procedure)
- **HRD** = rust-enterprise-harden (hardening controls, SEC/RES/OBS/BLD/CTR rules)
- **AUD** = rust-enterprise-audit (audit procedure and finding taxonomy)

Mechanism codes follow the E1–E6 model in BP §1.

## A. Language core

| Topic | Rules | Skills | Mechanism |
|---|---|---|---|
| Ownership, moves, copies | BP-OW-01, BP-OW-02, BP-TY-08 | BP, IMP, AUD | E2 (clippy `redundant_clone`, `clone_on_ref_ptr`), E6 |
| Borrowing and aliasing | BP-OW-01, BP-OW-03, BP-OW-04 | BP, AUD | E6 |
| Lifetimes (elision, explicit, `'static`, HRTB) | BP-OW-03, BP-OW-04, BP-AS-05, BP-TY-07 | BP, AUD | E6 |
| `Cow` | BP-OW-05 | BP | E6 |
| Smart pointers: `Box`, `Rc`, `Arc`, `Weak` | BP-OW-07, BP-LN (`rc_mutex`) | BP, AUD | E2, E5 |
| Interior mutability: `Cell`, `RefCell`, `OnceCell`, `OnceLock`, `LazyLock` | BP-OW-06, BP-LN (`await_holding_refcell_ref`) | BP, AUD | E2, E6 |
| `Drop`, RAII, drop order | BP-PN-05, BP-TY-01, BP-ER-04 | BP, IMP, HRD (SEC-SEC zeroize) | E6, E3 |
| Pattern matching, exhaustiveness | BP-TY-01, BP-TY-06 | BP | E1, E2 (`wildcard_enum_match_arm`) |
| `if let`, `let else`, let chains | BP-TC-02 (edition 2024 scrutinee rules) | BP, AUD | E6 |
| Enums with data, `Option`, `Result` combinators | BP-TY-01, BP-ER-01 | BP | E2, E6 |
| `#[must_use]` | BP-TY-10 | BP | E1 (`unused_must_use`) |
| Constants, statics, `static mut` | BP-US-09, BP-TY-12 | BP | E5 |
| Const generics, const fn | BP-TY-02 | BP | E6 |
| Closures: `Fn`, `FnMut`, `FnOnce`, captures | BP-CC-03 (no callbacks under locks) | BP, AUD | E6 |
| Traits, supertraits, blanket impls, coherence | BP-TY-05, BP-TY-11 | BP, IMP | E6 |
| Associated types and constants | BP-TY-11 | BP | E6 |
| Generics and monomorphisation trade-offs | BP-TY-11, BP-PF-03 | BP, HRD (BLD binary size) | E6 |
| Trait objects, object safety | BP-TY-11, BP-AS-06 | BP | E6 |
| `impl Trait` in argument and return position | BP-TC-02 (RPIT capture), BP-TY-07 | BP | E6 |
| Sealed traits, `#[non_exhaustive]` | BP-TY-05 | BP | E6 |
| Newtype pattern | BP-TY-02 | BP, IMP | E6 |
| Typestate pattern | BP-TY-04 | BP, IMP | E6 |
| Parse-don't-validate | BP-TY-03 | BP, IMP, HRD (input boundary) | E6 |
| Make illegal states unrepresentable | BP-TY-01 | BP, IMP | E6 |
| Operator overloading (`Add`, `Index`, `Deref`) | BP-TY-07, BP-CO-01 | BP, AUD | E6 (`Deref` misuse flagged in review) |
| `Deref` as inheritance anti-pattern | BP-TY-07 | BP | E6 |
| `AsRef`, `Borrow`, `From`, `TryFrom`, `Into` | BP-NU-01, BP-TY-03 | BP | E2 |
| `Default` semantics | BP-TY-12 | BP | E5, E6 |
| `PartialEq`/`Eq`/`PartialOrd`/`Ord`/`Hash` contracts | BP-CO-04, BP-NU-04 | BP, AUD | E6 |
| Iterators, adaptors, laziness | BP-CO-01, BP-CO-02, BP-CO-03 | BP, AUD | E2 (`needless_collect`) |
| Slices, arrays, `Vec`, `Box<[T]>` | BP-CO-01, BP-CO-03 | BP | E2, E6 |
| `String`, `&str`, UTF-8 validity, `char` | BP-CO-06, BP-CO-07 | BP, AUD | E2 (`string_slice`) |
| `Path`, `PathBuf`, `OsString`, `CString`, `CStr` | BP-CO-07, BP-CO-08, BP-FF-03 | BP, HRD (SEC-PATH), AUD | E6 |
| Hash maps and sets, hashing DoS | BP-CO-04, BP-CO-05 | BP, HRD (SEC-DOS), AUD | E5, E6 |
| `BTreeMap` determinism | BP-CO-05 | BP | E6 |
| `VecDeque`, `BinaryHeap`, `LinkedList` choice | BP-PF-02 | BP | E6 |
| Byte handling: `bytes::Bytes`, endianness | BP-CO-09, BP-PF-02 | BP | E6 |
| Numerics: integer overflow, `checked_*`, `wrapping_*`, `saturating_*` | BP-NU-02, BP-NU-05, BP-NU-06 | BP, HRD (BLD overflow-checks), AUD | E2 (`arithmetic_side_effects`), E3 |
| Casting: `as`, `TryFrom` | BP-NU-01 | BP | E2 (`as_conversions`) |
| Floats: `total_cmp`, NaN, epsilon | BP-NU-04 | BP | E2 (`float_cmp`) |
| Money as minor-unit integers | BP-NU-03 | BP, HRD, AUD | E6 |
| Macros: `macro_rules!`, hygiene, `$crate` | BP-MA-01, BP-MA-02 | BP | E6 |
| Procedural macros and derive | BP-MA-03 | BP | E6 |
| Modules, visibility, `pub(crate)`, re-exports | BP-MO-01, BP-MO-04, BP-MO-05 | BP, IMP | E1, E2 (`unreachable_pub`) |
| Editions, edition 2024 migration rules | BP-TC-02 | BP | E1 |
| Attributes: `#[cfg]`, `#[cfg_attr]`, `#[expect]`, `#[allow]` | BP-LN-04, BP-LN-05 | BP | E2 |
| `unsafe` (blocks, fns, traits, impls, extern) | BP-US-01 to 09 | BP, HRD (BLD Miri), AUD | E1, E2, E3 |
| Raw pointers, provenance | BP-US-05, BP-US-04 | BP, AUD | E3 (Miri strict provenance) |
| `transmute`, unions, `MaybeUninit` | BP-US-07, BP-LN (`uninit_vec`, `mem_replace_with_uninit`) | BP, AUD | E2, E6 |
| `Pin`, `Unpin`, self-referential types | BP-US-01, BP-AS-03 | BP, AUD | E6 |
| Panics, `catch_unwind`, unwinding, `panic = "abort"` | BP-PN-01 to 05 | BP, HRD (BLD), AUD | E2, E6 |
| `Result`-based error design | BP-ER-01 to 08 | BP, IMP | E2, E3 |
| `thiserror`, `anyhow`, `#[source]`, `#[from]` | BP-ER-03, BP-ER-04 | BP, IMP | E6 |
| Error chains and `Display` discipline | BP-ER-04, BP-ER-05 | BP, HRD (SEC-LOG) | E6 |
| Retryable vs fatal error classification | BP-ER-07 | BP, HRD (RES) | E6, E3 |
| `std::process::exit`, `main` return types | BP-PN-01 | BP, IMP | E2 |
| Threads: `std::thread`, scoped threads | BP-CC-09 | BP | E5, E6 |
| `Send` and `Sync` | BP-CC-01 | BP, AUD | E6 |
| `Mutex`, `RwLock`, poisoning | BP-CC-02, BP-CC-03, BP-CC-08 | BP, AUD | E2 (`await_holding_lock`), E6 |
| Atomics, memory ordering | BP-CC-06 | BP, AUD | E6, E3 (loom) |
| Condition variables, `Barrier`, `Once` | BP-CC-04 | BP | E6 |
| Channels: `mpsc`, bounded vs unbounded | BP-CC-05 | BP, HRD (RES bulkhead), AUD | E5, E6 |
| Lock-free structures, ABA, reclamation | BP-CC-07 | BP, AUD | E3 (loom, Miri) |
| Deadlock and lock ordering | BP-CC-03 | BP, AUD | E6 |
| Data races and their UB | BP-CC-01, BP-US-04 | BP, AUD | E3 (Miri, TSan) |
| `async`/`await`, `Future`, `Poll`, executors | BP-AS-01 to 08 | BP, IMP, AUD | E2, E6 |
| Tokio runtime, `spawn`, `spawn_blocking`, `JoinSet` | BP-AS-01, BP-AS-04, BP-CC-09 | BP, IMP | E2, E5 |
| `select!` and cancellation safety | BP-AS-03, BP-AS-08 | BP, AUD | E6 |
| Async traits and boxing | BP-AS-06 | BP | E6 |
| Timeouts and cancellation tokens | BP-AS-02, HRD (RES) | BP, HRD, IMP | E6 |
| Streams and backpressure | BP-AS-07 | BP, HRD (RES) | E6 |
| Blocking-in-async detection | BP-AS-01 | BP, AUD | E5, E6 |
| `Send` bounds for spawned futures | BP-AS-05 | BP | E1 (compile) |
| Graceful shutdown and signals | HRD (RES-SHUT rules), IMP | HRD, IMP | E6, E3 (integration) |

## B. Tooling and build

| Topic | Rules | Skills | Mechanism |
|---|---|---|---|
| `rustup`, toolchain pinning, `rust-toolchain.toml` | BP-TC-01 | BP, IMP | E4 |
| Edition and MSRV, `rust-version` | BP-TC-02, BP-TC-03 | BP | E4 |
| Cargo workspaces, `[workspace.dependencies]` | BP-CG-02, BP-DP-04 | BP, IMP | E4 |
| `Cargo.lock`, `--locked`, `--frozen`, `--offline` | BP-CG-01 | BP, HRD (BLD) | E4 |
| Features: additive design, `default`, `--no-default-features` | BP-CG-04 | BP | E4 |
| Build scripts (`build.rs`), `rerun-if-changed` | HRD (BLD: build.rs review, no network in build) | HRD, AUD | E6 |
| Profiles: `dev`, `release`, `lto`, `codegen-units`, `opt-level`, `strip`, `debug`, `overflow-checks`, `panic` | BP-CG-05, BP-PF-03, HRD (BLD-REL) | BP, HRD | E4, E6 |
| Cross compilation and targets | BP-FF-05 | BP, HRD (BLD) | E4 |
| Clippy | BP-LN-01 to 07 | BP | E2 |
| rustfmt | BP-FM-01 | BP | E4 |
| rustdoc, intra-doc links, doctests | BP-DC-01 to 03 | BP | E2, E4 |
| `cargo doc` warnings as errors | BP-LN-07 | BP | E4 |
| `cargo tree`, `cargo metadata` | BP-DP-05 | BP, AUD | E4 |
| `cargo deny` (advisories, licences, bans, sources) | BP-DP-02 | BP, HRD (SEC-SUP) | E4 |
| `cargo audit` (RustSec) | BP-DP-03 | BP, HRD, AUD | E4 |
| `cargo vet` | BP-DP-07 | BP, HRD | E4 |
| `cargo auditable` (embedded dependency info in binaries) | HRD (BLD-SBOM) | HRD | E4 |
| `cargo geiger` (unsafe inventory) | BP-US-06 | BP, AUD | E4 |
| `cargo semver-checks` | BP-MO-03 | BP | E4 |
| `cargo bloat`, `cargo llvm-lines` (binary size analysis) | BP-PF-03 | BP | E6 |
| `cargo expand` (macro expansion review) | BP-MA-03 | BP, AUD | E6 |
| Build reproducibility (`SOURCE_DATE_EPOCH`, `--remap-path-prefix`) | HRD (BLD-REPRO) | HRD | E4, E6 |
| Workspace `[lints]` inheritance | BP-CG-03 | BP | E2 |
| Cross-platform builds, `cfg(target_os)` | BP-CG-04 | BP | E4 |
| Cargo `[patch]` and `[replace]` | BP-DP-08 | BP, AUD | E4 |
| Dependency features and `resolver = "3"` | BP-TC-02, BP-CG-04 | BP | E1 |

## C. Testing and verification

| Topic | Rules | Skills | Mechanism |
|---|---|---|---|
| Unit tests, `#[cfg(test)]` | BP-TS-03 | BP, IMP | E4 |
| Integration tests (`tests/`) | BP-TS-03 | BP, IMP | E4 |
| Doctests | BP-TS-01, BP-DC-03 | BP | E4 |
| `nextest` | BP-TS-01 | BP | E4 |
| Property testing (`proptest`, `quickcheck`) | BP-TS-03, BP-NU-05 | BP, IMP | E3 |
| Snapshot testing (`insta`) | BP-TS-04, BP-SE-04 | BP | E3 |
| Fuzzing (`cargo-fuzz`, libFuzzer) | BP-TS-03, BP-US-05 | BP, HRD (BLD-FUZZ), AUD | E3, E4 |
| Miri (undefined behaviour, provenance, data races) | BP-US-04 | BP, HRD, AUD | E4 |
| Sanitizers (ASan, TSan, MSan) | HRD (BLD-SAN) | HRD, AUD | E4 |
| `loom` (concurrency model checking) | BP-CC-07 | BP, AUD | E3 |
| Kani (bounded model checking) | BP-US-05 (where applicable) | BP, AUD | E3 |
| Coverage (`cargo llvm-cov`) | BP-TS-07 | BP | E4 |
| Mutation testing (`cargo-mutants`) | BP-TS-07 | BP, AUD | E4 (scheduled) |
| Test determinism, injected clocks | BP-TS-05 | BP | E6 |
| Regression tests for fixes | BP-TS-02 | BP, IMP | E6 |
| Contract testing | BP-TS-03, BP-SE-01 | BP | E3 |
| End-to-end testing with containers | BP-TS-03 | BP, HRD (CTR-04, CTR-07 for test networks) | E3 |
| Benchmarks (`criterion`, `black_box`) | BP-PF-01 | BP | E6 |
| Test-only lint exceptions | BP-TS-06 | BP | E2 |

## D. Serialisation, I/O and interop

| Topic | Rules | Skills | Mechanism |
|---|---|---|---|
| `serde` derive, attributes, `deny_unknown_fields` | BP-SE-02, BP-SE-05 | BP, IMP | E6 |
| Versioned contracts and compatibility | BP-SE-01 | BP | E3, E6 |
| Bounded deserialisation (size, depth, length) | BP-SE-03, HRD (SEC-DOS) | BP, HRD, AUD | E3, E6 |
| Timestamps, durations, time zones | BP-SE-06, BP-OB-05 | BP, AUD | E6 |
| Monotonic vs wall clock | BP-TS-05 | BP, AUD | E6 |
| File I/O, `BufReader`, `BufWriter`, atomic writes | BP-AS-01 | BP, IMP | E6 |
| Networking: `std::net`, TCP, UDP | BP-AS-02 | BP, HRD (RES, SEC-TLS) | E6 |
| TLS (`rustls`), certificate validation | HRD (SEC-TLS) | HRD, AUD | E6 |
| Databases (`sqlx`, parameter binding, pools) | HRD (SEC-DB) | HRD, AUD | E6, E3 |
| Environment, CLI args (`clap`), config layering | IMP (CLI-first config) | IMP, HRD | E6 |
| Logging and tracing (`tracing`, spans, subscribers) | BP-OB-01 to 04 | BP, HRD (OBS) | E2, E6 |
| Metrics (counters, histograms, gauges) | BP-OB-05 | BP, HRD (OBS) | E6 |
| OpenTelemetry | HRD (OBS-OTEL) | HRD | E6 |
| Process exit codes, signals | BP-PN-01, HRD (RES-SHUT) | HRD, IMP | E6 |
| FFI (`extern "C"`, `bindgen`, `cbindgen`) | BP-FF-01 to 04 | BP, AUD | E6 |
| `no_std`, `alloc`, embedded | BP-FF-05 | BP | E4 (target build) |
| WebAssembly | BP-FF-05 | BP | E4 |
| Cryptography (`rustls`, `ring`/`aws-lc-rs`, `argon2`, `zeroize`, `subtle`, `secrecy`, RNG) | HRD (SEC-CRYPTO, SEC-SEC) | HRD, AUD | E6, E5 |
| Authentication (OIDC, JWT, sessions) | HRD (SEC-AUTH) | HRD, AUD | E6, E3 |
| Authorisation (RBAC, deny-by-default) | HRD (SEC-AUTHZ) | HRD, AUD | E6, E3 |
| Input validation and injection (SQL, command, path, header, log) | HRD (SEC-INJ, SEC-PATH, SEC-LOG) | HRD, AUD | E5, E6 |
| Denial of service (regex, parsing, allocation, CPU) | HRD (SEC-DOS) | HRD, AUD | E3, E6 |
| Secrets management and zeroisation | HRD (SEC-SEC) | HRD, AUD | E5, E6 |
| Timing side channels | HRD (SEC-CRYPTO) | HRD, AUD | E6 |

## E. Architecture, operations and delivery

| Topic | Rules | Skills | Mechanism |
|---|---|---|---|
| DDD bounded contexts in module layout | BP-MO-02 | BP, IMP | E6 |
| Hexagonal/ports-and-adapters boundaries | IMP (architecture rules) | IMP | E6 |
| Stateless horizontal scaling | IMP, HRD (RES-STATE) | IMP, HRD | E6 |
| Idempotency keys and retries | HRD (RES-IDEM), BP-ER-07 | HRD, AUD | E6, E3 |
| Circuit breakers, bulkheads, timeouts, backoff with jitter | HRD (RES-CB, RES-BULK, RES-TO, RES-RETRY) | HRD, AUD | E6, E3 |
| Health and readiness checks | HRD (OBS-HEALTH) | HRD | E6, E3 |
| Graceful degradation | HRD (RES-DEG) | HRD | E6 |
| Load shedding and rate limits | HRD (RES-SHED) | HRD | E6, E3 |
| SLO, SLA, error budgets | HRD (OBS-SLO) | HRD | E6 |
| Alerting | HRD (OBS-ALERT) | HRD | E6 |
| Backups, DR, RTO, RPO | HRD (OPS-DR) | HRD | E6 |
| Database migrations, expand/contract | HRD (OPS-MIG) | HRD | E6 |
| Containers: multi-stage, distroless, non-root, read-only FS | HRD (CTR rules) | HRD, AUD | E4, E6 |
| Image scanning, signing, SBOM | HRD (CTR-SIGN, BLD-SBOM) | HRD | E4 |
| Infrastructure as code | HRD (OPS-IAC) | HRD | E6 |
| CI/CD, pinned actions, least-privilege tokens | BP-GT-05, BP-GT-06 | BP, HRD | E4, E6 |
| Git flow, PR hygiene, Conventional Commits | BP-GT-03, BP-GT-04 | BP, IMP | E6 |
| Human-only merges | BP-GT-01, BP-GT-02 | BP, IMP, AUD | E6 |
| ADRs | BP-DC-04 | BP, IMP | E6 |
| Threat modelling (STRIDE) | HRD (SEC-TM), AUD | HRD, AUD | E6 |
| FinOps (cost per request, resource right-sizing) | HRD (OPS-FINOPS) | HRD | E6 |
| Compliance and audit logging | HRD (OBS-AUDIT) | HRD, AUD | E6 |
| Performance SLOs and regression benchmarks | BP-PF-05 | BP | E6 |
| Money, precision, rounding, currency | BP-NU-03 | BP, HRD, AUD | E6, E3 |
| Time-based logic (time zones, DST, leap seconds) | BP-SE-06, BP-TS-05 | BP, AUD | E6, E3 |

## Coverage verdict rule

A topic is only considered covered when: (a) it appears above, (b) at least one rule ID exists, and (c) its mechanism is implemented in the gate or the review checklist. An auditor who finds a topic in code with no matching rule raises an AUD-COV finding (coverage gap in the rulebook), not silence.

## F. Frameworks (enforced by rust-enterprise-frameworks)

| Topic | Rules | Skills | Mechanism |
|---|---|---|---|
| Tokio runtime, single runtime, spawn, blocking, shutdown | FW-TOK-01 to 08 | FW, BP, IMP | E2 (disallowed-methods), E4 |
| axum routing, body limits, deny-by-default routes | FW-AXM-01 to 08, FW-INV-01 to 03 | FW, HRD, AUD | M1, M2, E6 |
| tower and tower-http middleware, layer order, timeouts | FW-TWR-01 to 04 | FW | M1 (deprecated TimeoutLayer::new), E6 |
| hyper protocol, header read timeouts, hyper 0.14 ban | FW-HYP-01 to 03 | FW | E4, E6 |
| tonic gRPC limits, TLS, interceptors | FW-TNC-01 to 05 | FW, HRD | E6 |
| sqlx queries, offline cache, pools, statement timeouts | FW-SQX-01 to 07 | FW, HRD (DB) | M3 (gate), E5, E6 |
| reqwest client policy, timeouts, blocking-client ban | FW-RQW-01 to 05 | FW, HRD | M1, M2 |
| rustls, provider choice, verifier bypass ban | FW-RLS-01 to 03 | FW, HRD (SEC-TLS) | M1, E4 |
| serde limits, deny_unknown_fields, recursion limit | FW-SRD-01 to 04 | FW, BP | M1, E6 |
| clap configuration, try_parse, env reading | FW-CLP-01 to 03 | FW, IMP | M1 |
| tracing subscriber placement and field discipline | FW-TRC-01 to 04 | FW, BP | E6 |
| OpenTelemetry pairing, provider shutdown | FW-OTL-01 to 04 | FW, HRD (OBS) | E4, E6 |
| Banned frameworks (actix-web, rocket, warp, diesel, sea-orm, native TLS) | FW-0, FW-GATE-01 | FW | M3 |
| Framework version floors, majors, MSRV floor | FW-GATE-02 to 04, FW-VER-01 to 04 | FW | M3, E4 |
| Duplicate framework versions | FW-GATE-03 | FW | M3 |
