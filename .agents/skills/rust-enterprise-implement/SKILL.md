---
name: rust-enterprise-implement
description: Enterprise Rust implementation procedure. Use when creating, extending or changing Rust code, crates, services, CLIs, libraries, workspaces or Rust components of a larger system. Enforces discovery of project rules, requirements and invariants before code, domain-first type design, test-first development, CLI-first configuration, structured errors, bounded concurrency, the enterprise gate, documentation and PR handoff. Every rule cites a BP-* rule from rust-enterprise-best-practices. Triggers include "write a Rust service", "implement in Rust", "add a feature to this crate", "build a Rust CLI", "create a Rust library", "scaffold a Rust workspace", "implement this endpoint in Rust".
---

# Enterprise Rust Implementation (IMP)

This skill builds code. It does not set the rules (rust-enterprise-best-practices does), does not add security controls beyond the baseline (rust-enterprise-harden does), and does not judge existing code (rust-enterprise-audit does). If a requirement touches authentication, money, personal data, external input or infrastructure, also apply rust-enterprise-harden before writing code.

Read `../rust-enterprise-best-practices/SKILL.md` first. Every IMP rule below is a precondition for the BP rules it cites.

Before modifying code, follow the binding execution procedure in `references/hardcore-agent-execution-contract.md`. It requires evidence-first investigation, authoritative research, a plan and acceptance tests, real production paths, honest verification, and explicit BLOCKED/NOT RUN status for missing evidence. It supplements this skill; it does not relax project-specific requirements.

## IMP-0. Non-negotiable constraints

- **IMP-00-A (BLOCKER).** Never invent requirements. If a requirement is missing and it changes behaviour, data, security or money, stop and ask. If it is minor and reversible, state the assumption explicitly in the PR description and in a code comment on the assumption.
- **IMP-00-B (BLOCKER).** Never merge a pull request. Create branches, commits and PRs; leave merging to an accountable human (BP-GT-01).
- **IMP-00-C (BLOCKER).** Never weaken a gate to get green: no `#[allow]` added to pass clippy, no `--no-verify`, no removing tests, no lowering coverage thresholds, no ignoring advisories without a `deny.toml` exception record, no `unsafe` to bypass the borrow checker.
- **IMP-00-D (BLOCKER).** Do not copy code from the internet or other projects without checking its licence and provenance against `deny.toml` and the project's policy.

## IMP-1. Phase 0 — discover the project before touching code

Run and read, in this order:

1. Project instruction files: `CLAUDE.md`, `AGENTS.md`, and the same names in `.claude/`, `.cursor/`, `.codex/` if present. These rules apply in addition to the BP rulebook. If they relax a BP rule, record the conflict and follow the stricter rule unless the user confirms an exception.
2. Toolchain and policy: `rust-toolchain.toml`, `Cargo.toml` (`[workspace]`, `[workspace.lints]`, `[workspace.dependencies]`, `[profile.*]`), `clippy.toml`, `rustfmt.toml`, `deny.toml`, `supply-chain/` (cargo-vet), `.github/workflows/` or equivalent CI, `Dockerfile`s.
3. Architecture docs: `docs/`, ADRs under `docs/adr/`, bounded-context maps, API specs (OpenAPI, protobuf, JSON Schema), event schemas.
4. Current state: `cargo metadata --locked --format-version 1 > /dev/null`, `cargo test --workspace --locked` (record baseline), `cargo clippy --workspace --all-targets --locked -- -D warnings` (record baseline).

Record the baseline. If the baseline is already red, fix or report it before adding work. Do not stack new work on a red base.

- **IMP-01 (BLOCKER).** Do not start coding until Phase 0 is complete and the baseline is recorded.

## IMP-2. Phase 1 — requirements, invariants and threat surface

Write these down before code, in the PR description or an ADR:

1. **Behaviour**: inputs, outputs, state transitions, failure modes, and what "done" means as observable behaviour.
2. **Invariants**: facts that must always hold (for example, "a ledger entry's debits equal its credits", "a shipment cannot be `Delivered` without a `delivered_at`", "a token's `exp` is in the future at issuance"). Each invariant gets a test.
3. **Boundaries**: which bounded context owns the data; which other contexts may call it and through what contract (BP-MO-02, BP-SE-01).
4. **Data ownership**: which service or module is the single writer of each record. Cross-context writes are forbidden; use events or APIs.
5. **Consistency**: strong or eventual, per domain. State the choice and its failure mode.
6. **Non-functional targets**: latency percentiles, throughput, memory ceiling, availability, RTO/RPO where persistence is involved (feeds rust-enterprise-harden OPS rules).
7. **Trust boundary**: every point where external input enters. Each entry point gets a validation plan (BP-TY-03, BP-SE-03).
8. **Failure policy**: for each external call, its timeout, retry classification (retryable or not), and degradation behaviour.

- **IMP-02 (BLOCKER).** Each invariant and each trust-boundary entry point maps to at least one planned test.

## IMP-3. Phase 2 — crate and module placement

Layout template (adapt names to the domain):

```
workspace/
├── Cargo.toml                 # [workspace] only; shared lints, deps, profiles
├── rust-toolchain.toml        # exact pin (BP-TC-01)
├── deny.toml                  # supply-chain policy (BP-DP-02)
├── clippy.toml                # framework hard bans (disallowed-methods/types); copy rust-enterprise-frameworks/references/clippy.service.toml
├── rustfmt.toml               # edition/style only
├── crates/
│   ├── <context>-domain/      # pure logic: types, invariants, state machines. No I/O, no async runtime.
│   ├── <context>-application/ # use cases, orchestration, port traits (ports the adapters implement)
│   ├── <context>-adapters/    # database, HTTP clients, queues. Implements ports. Owns I/O errors.
│   ├── <context>-api/         # HTTP/gRPC handlers, DTOs, mapping to application types. Edge only.
│   └── <app>-bin/             # main(): config, wiring, telemetry, graceful shutdown. Only binary crate.
├── fuzz/                      # cargo-fuzz targets for untrusted-input parsers (BP-TS-03)
└── docs/adr/                  # decisions (BP-DC-04)
```

- **IMP-03 (MAJOR).** Dependency direction is inward: `api` and `adapters` depend on `application`; `application` depends on `domain`; `domain` depends on nothing project-specific. Enforce with the crate graph; a reverse edge is a blocker.
- **IMP-04 (MAJOR).** Domain crates do not depend on `tokio`, `sqlx`, `axum`, `reqwest` or any I/O crate. Purity makes invariants testable without infrastructure.
- **IMP-05 (MAJOR).** Libraries (`*-domain`, shared kernels) use `thiserror` error enums. Only the binary uses `anyhow`.

## IMP-4. Phase 3 — design the types first

Before any function body:

1. Model domain identifiers as newtypes (BP-TY-02).
2. Model each state as an enum variant that carries exactly the data valid in that state (BP-TY-01). Use typestate where ordering matters (BP-TY-04).
3. Model every external input as a raw DTO type that is converted by a fallible constructor into a validated domain type (BP-TY-03). The DTO derives `Deserialize` with `#[serde(deny_unknown_fields)]` (BP-SE-02).
4. Money is `Minor` integer units with a currency newtype (BP-NU-03). No floats.
5. Public enums and structs that may grow are `#[non_exhaustive]` (BP-TY-05).
6. Write the public signatures and rustdoc with `# Errors` and `# Panics` sections (BP-DC-02) before implementing.

- **IMP-06 (BLOCKER).** A function that takes a raw primitive where a domain type exists is rejected in review (for example `fn transfer(from: i64, to: i64, amount: f64)`).

## IMP-5. Phase 4 — test list first, then failing tests

1. Write a test list: happy path, each invariant, each error variant (BP-ER-08), boundary values (BP-NU-05), malicious or malformed input for each trust-boundary entry point, concurrency interleavings if shared state exists, and idempotency (repeat the same request).
2. Write the first failing test. Run it. It must fail for the right reason. Record the failure output.
3. Implement the minimum to pass. Refactor under green.
4. Add property tests for algebraic laws and parsers (BP-TS-03). Add fuzz targets for every parser of untrusted bytes or text.
5. Add a regression test for every bug found during development (BP-TS-02).

- **IMP-07 (BLOCKER).** No production code is written for behaviour that has no failing test first, except pure type definitions and documentation.

## IMP-6. Phase 5 — implementation rules

Apply every BP rule. The rules most often violated, restated as IMP checks:

- **IMP-08 (BLOCKER).** Errors propagate with `?`. No `unwrap`, `expect`, `panic!`, `todo!` in production code (BP-ER-02). Use `Result` and typed errors. Test code may use `expect` only under the workspace test exception (BP-TS-06).
- **IMP-09 (BLOCKER).** Every external await is wrapped in a timeout from configuration (BP-AS-02). Example pattern:

```rust
let response = tokio::time::timeout(config.upstream_timeout, client.get(url).send())
    .await
    .map_err(|_| UpstreamError::Timeout)?
    .map_err(UpstreamError::Transport)?;
```

- **IMP-10 (BLOCKER).** Every spawned task has an owner. Use `JoinSet` or a stored `JoinHandle`, and await or abort at shutdown (BP-AS-04).
- **IMP-11 (BLOCKER).** Channels are bounded. Choose the capacity from configuration and document the backpressure behaviour (BP-CC-05, BP-AS-07).
- **IMP-12 (BLOCKER).** Logging uses `tracing` with structured fields. Secret-typed fields are never logged (BP-OB-01, BP-OB-02). Errors are logged once at the handling boundary (BP-OB-04).
- **IMP-13 (MAJOR).** Use `#[expect(lint, reason = "...")]` for every deliberate suppression (BP-LN-04).
- **IMP-14 (MAJOR).** Public functions that touch the clock take a clock abstraction (`fn now(&self) -> OffsetDateTime`) so tests are deterministic (BP-TS-05).

### IMP-6.1 Configuration: CLI-first, validated, no ambient state

The project rule "never bypass CLI args; never `Default` that reads env" is the enterprise standard. Configuration is parsed once, at the binary boundary, validated, and passed down as a value.

```rust
// crates/app-bin/src/config.rs
use std::{net::SocketAddr, time::Duration};
use clap::Parser;
use secrecy::SecretString;

#[derive(Parser)]                       // no derive(Debug) on a struct holding secrets (BP-TY-09)
#[command(version, about)]
pub struct Config {
    /// Address the HTTP listener binds to.
    #[arg(long, env = "APP_BIND", default_value = "127.0.0.1:8080")]
    pub bind: SocketAddr,

    /// Upstream request timeout. Must be > 0 and <= 60s.
    #[arg(long, env = "APP_UPSTREAM_TIMEOUT", value_parser = parse_timeout, default_value = "5s")]
    pub upstream_timeout: Duration,

    /// Database connection string. Secret.
    #[arg(long, env = "APP_DATABASE_URL", hide_env_values = true)]
    pub database_url: SecretString,
}

fn parse_timeout(raw: &str) -> Result<Duration, String> {
    let d = humantime::parse_duration(raw).map_err(|e| e.to_string())?;
    if d.is_zero() || d > Duration::from_secs(60) {
        return Err("timeout must be >0 and <=60s".to_owned());
    }
    Ok(d)
}
```

Notes: `humantime` is an example; verify the crate and its version with `cargo info` before adding it (BP-DP-01). `hide_env_values` keeps secrets out of `--help` output. Validation runs at parse time, so the process cannot start with an invalid configuration.

### IMP-6.2 Error model

```rust
// crates/<context>-domain/src/error.rs
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransferError {
    #[error("amount must be positive")]
    NonPositiveAmount,
    #[error("insufficient funds for account {account}")]
    InsufficientFunds { account: AccountId },
    #[error("currency mismatch")]
    CurrencyMismatch,
}

impl TransferError {
    /// Whether the caller may retry the same request unchanged.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        false // domain rejections are never retryable (BP-ER-07)
    }
}
```

Adapters wrap I/O errors in their own variant with `#[source]`, never a stringified copy (BP-ER-04). Retryability for transport errors is decided in the adapter, where the transport semantics are known.

### IMP-6.3 Graceful shutdown skeleton

The skeleton below has no `unwrap`, `expect`, or unchecked exits. Helpers it names (`telemetry::init`, `api::serve`, `wait_for_signal`) are separate, tested functions in their own modules. Printing before telemetry exists is the only boundary where direct stderr output is allowed, and it is marked for the gate.

```rust
// crates/app-bin/src/main.rs
use std::process::ExitCode;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> ExitCode {
    let config = match Config::try_parse() {
        Ok(config) => config,
        Err(err) => {
            // gate-allow: CLI usage error is shown to the operator before telemetry exists (BP-OB-01 boundary)
            if err.print().is_err() {
                return ExitCode::FAILURE;
            }
            return ExitCode::from(u8::try_from(err.exit_code()).unwrap_or(2));
        }
    };

    // The guard flushes buffered telemetry (for example OpenTelemetry spans) when dropped.
    let _telemetry = match telemetry::init(&config) {
        Ok(guard) => guard,
        Err(err) => {
            // gate-allow: telemetry failed before any logger exists; the operator must see why (BP-OB-01 boundary)
            eprintln!("telemetry initialisation failed: {err}");
            return ExitCode::FAILURE;
        }
    };

    let shutdown = CancellationToken::new();
    let server = tokio::spawn(api::serve(config.bind, shutdown.clone()));

    wait_for_signal().await; // SIGINT, and SIGTERM on Unix; implemented with tokio::signal
    shutdown.cancel();       // stop accepting new work; in-flight work drains

    match tokio::time::timeout(config.drain_timeout, server).await {
        Ok(Ok(Ok(()))) => ExitCode::SUCCESS,
        Ok(Ok(Err(err))) => {
            tracing::error!(error = %err, "server stopped with an error");
            ExitCode::FAILURE
        }
        Ok(Err(join_err)) => {
            tracing::error!(error = %join_err, "server task panicked or was cancelled");
            ExitCode::FAILURE
        }
        Err(_elapsed) => {
            tracing::error!("drain deadline exceeded; exiting with failure");
            ExitCode::FAILURE
        }
    }
}
```

Direct stderr output is permitted only at the two marked boundaries above, because no structured logger exists at that point. Every other output goes through `tracing`. `telemetry::init`, `api::serve` and `wait_for_signal` are real functions that must be written and tested in their own modules; the skeleton does not define them.

Config for this skeleton adds the drain timeout to IMP-6.1:

```rust
    /// Maximum time to drain in-flight requests after shutdown is requested.
    #[arg(long, env = "APP_DRAIN_TIMEOUT", value_parser = parse_timeout, default_value = "30s")]
    pub drain_timeout: Duration,
```

- **IMP-15 (BLOCKER).** Shutdown has a bounded drain. An unbounded drain is a defect (RES-SHUT in harden skill).

- **IMP-15 (BLOCKER).** Shutdown has a bounded drain. An unbounded drain is a defect (RES-SHUT in harden skill).

## IMP-7. Phase 6 — security-sensitive work

If the change touches any of the following, load and apply rust-enterprise-harden before implementation, not after:

- authentication, token validation, sessions, passwords, keys, signatures
- authorisation decisions, tenant isolation
- any SQL, shell, file path, template, header, or log built from external input
- money, balances, ledgers, payment state
- personal or regulated data (KYC, identity, health, financial)
- TLS, certificates, outbound HTTP clients
- deserialisation of external data, file uploads, archive extraction
- build pipelines, container images, CI configuration

## IMP-8. Phase 7 — the gate

Run `../rust-enterprise-best-practices/scripts/rust-gate.sh`. It runs format, clippy with `-D warnings`, tests, doctests, docs, supply chain, MSRV, the pattern scan, and conditional Miri and fuzz.

- **IMP-16 (BLOCKER).** The gate passes locally with the same toolchain CI uses before any PR is opened. If a tool is missing, install it. A missing tool is a failure.
- **IMP-17 (BLOCKER).** New dependencies follow BP-DP-01: justification in the PR, `cargo deny check` passing, `cargo vet` attestation or a documented audit request.

## IMP-9. Phase 8 — documentation

- Rustdoc for every public item with `# Errors`, `# Panics`, `# Safety` where applicable, and at least one runnable example (BP-DC-01, BP-DC-02).
- An ADR for every non-obvious decision: framework choice, consistency model, retry policy, data ownership, any exception to a BP rule (BP-DC-04).
- README updated for build, test, run, and configuration keys in a table (BP-DC-05).
- A runbook entry for every new alert or operational behaviour (feeds OPS rules in harden skill).

## IMP-10. Phase 9 — PR and handoff

PR description must contain:

1. Purpose and linked requirement or issue.
2. Assumptions made (IMP-00-A) and where they live in code.
3. Invariants and the tests that protect them.
4. Gate output summary: each step and its result.
5. New dependencies with justification.
6. Any BP exception requested, with owner, reason, expiry.
7. Rollout and rollback plan for anything that changes behaviour in production.
8. Explicit statement: "Not merged by the author. Awaiting human review and merge."

Commit hygiene: Conventional Commits, one logical change per commit, each commit building and passing tests where practical (BP-GT-04). Do not force-push a branch under review without saying so in the PR thread.

- **IMP-18 (BLOCKER).** The PR is opened, not merged. An automated agent that finds a PR ready must report readiness and stop (BP-GT-01).

## IMP-11. Forbidden shortcuts (each one is a finding)

| Shortcut | Why it is forbidden | Rule |
|---|---|---|
| `.clone()` added only to satisfy the borrow checker | Hides a design problem and costs memory | BP-OW-01 |
| `Arc<Mutex<T>>` as the default shared-state design | Lock contention and deadlock risk | BP-CC-04 |
| `unwrap()` "because this can't fail" | Invariants change; the panic becomes a DoS | BP-ER-02 |
| `#[allow(clippy::...)]` without reason | Silent suppression | BP-LN-04 |
| `String` for every identifier | No compile-time separation of IDs | BP-TY-02 |
| `f64` for money | Rounding errors in financial amounts | BP-NU-03 |
| `as` casts for sizes or lengths | Silent truncation | BP-NU-01 |
| `tokio::spawn` with the handle dropped | Orphaned work, lost errors | BP-AS-04 |
| `unbounded_channel` | Unbounded memory under load | BP-CC-05 |
| Reading env vars inside a library or `Default` | Hidden configuration, non-reproducible tests | BP-TY-12, IMP-6.1 |
| Writing a hand-rolled parser where a vetted crate exists | Parser bugs are security bugs | BP-DP-01, SEC-INJ |
| Disabling a test to make CI pass | Hides a regression | IMP-00-C |
| Adding a feature flag that changes behaviour silently | Untestable combinations | BP-CG-04 |
| Blocking calls inside `async fn` | Starves the runtime | BP-AS-01 |

## IMP-12. Definition of done

- [ ] The hardcore execution contract was followed; facts, assumptions, and unverified items are distinguished.
- [ ] Phase 0 baseline recorded; baseline was green before the change.
- [ ] Every invariant and trust-boundary entry point has a test.
- [ ] Test list was written before implementation; each test was seen failing first.
- [ ] Gate passes: `../rust-enterprise-best-practices/scripts/rust-gate.sh` exits 0 with the pinned toolchain.
- [ ] No `gate-allow` markers added without reviewer-visible reasons.
- [ ] No BP rule relaxed; exceptions recorded as ADRs with owner and expiry.
- [ ] Public API documented; semver impact stated for library crates (BP-MO-03).
- [ ] Observability: spans, structured logs, metrics for new paths, no sensitive fields.
- [ ] Timeouts, bounds and cancellation handled for every new I/O path.
- [ ] Rollout and rollback described.
- [ ] PR opened and left for human merge.

## IMP-13. References

- `references/templates.md`: toolchain pin, root and member `Cargo.toml`, `deny.toml`, clippy and rustfmt configuration, CI workflow with pinned actions, fuzz and property test skeletons, config checklist.
- `references/hardcore-agent-execution-contract.md`: binding end-to-end procedure for investigation, research, planning, implementation, real integration tests, evidence tracking, and PR handoff.
