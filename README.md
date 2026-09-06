# Sitolo

**Business Operating System for African SMEs.**

Sitolo turns fragmented merchant operations into a coherent business record:

```text
PROCURE
  -> RECEIVE
  -> STOCK
  -> PRICE
  -> SELL
  -> COLLECT
  -> RECONCILE
  -> REPORT
  -> DECIDE
```

Sitolo is a Rust-first, modular-monolith business operating system. PostgreSQL is the authoritative server-side state; SQLite is the device-local operational store for offline continuity. Primary market is Malawi, with controlled African regional expansion planned.

## Architecture

```text
Flutter Mobile / Tauri Desktop / limited Web
                 |
              HTTPS/API
                 |
                 v
       Rust Application Core
        Axum + Tokio
                 |
      +----------+----------+
      |          |          |
      v          v          v
 PostgreSQL   Workers    Integrations
  authority    /outbox    /adapters
      |
      +---- audit / idempotency / read models
```

## Core invariants

- Clients request; the server decides. Client input is never final security or financial authority.
- Tenant isolation is a security boundary, not a convention.
- Financial and inventory facts are high-integrity data. Corrections use explicit compensating operations.
- External providers are untrusted trust boundaries.
- Offline operation continues safely but never becomes permanently authoritative.
- Security verification is release-blocking.
- PostgreSQL invariants are proven against real PostgreSQL, not mocks.

## Governance

The **governing engineering contract** for all AI coding agents and human contributors working in this repository is [`agent.md`](agent.md). Read it before making any change.

- [Documentation index](docs/README.md)
- [Contribution and repository policy](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [License](LICENSE) — Apache License 2.0

## Repository status

The repository is currently in **Phase 1** of the implementation program: establishing the engineering substrate (repository ownership, Rust workspace, dependency governance, reproducible builds, CI, security enforcement, artifact identity) before business-domain implementation begins. The governing Phase 1 specification is [`docs/phase1_repository_rust_workspace_ci_deep_implementation.md`](docs/phase1_repository_rust_workspace_ci_deep_implementation.md).

## Getting started

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the developer bootstrap path.

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE).