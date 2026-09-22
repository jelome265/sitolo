# Sitolo

**Sitolo is a business operating system for African SMEs, launching in Malawi.**

It gives a merchant one trustworthy record of how the business operates: what was procured, received, stocked, priced, sold, collected, reconciled, reported, and acted on.

```text
PROCURE → RECEIVE → STOCK → PRICE → SELL → COLLECT → RECONCILE → REPORT → DECIDE
```

The product starts with the day-to-day reality of selling and stock. It grows into payment control, reconciliation, multi-branch operations, enterprise governance, tax/compliance workflows, and approved ecosystem integrations—without forcing enterprise complexity on a small merchant.

## Project goal

Sitolo’s goal is to help a business answer the operational questions that matter:

- What came in, what went out, and what remains?
- What sold, what made money, and where is the money?
- What is unresolved and what needs action?
- Who can do what, who changed what, and what required approval?

The long-term product is one core with vertical configuration and progressively deeper enterprise controls. It is not a collection of disconnected point tools.

## Product and architecture principles

- Malawi is the initial market; regional expansion is controlled and evidence-led.
- Rust is the authoritative backend core; Flutter is Android-first mobile, Tauri supports desktop, and a limited web surface is optional where useful.
- PostgreSQL is authoritative server-side business state. SQLite is device-local operational state for offline continuity.
- Sitolo begins as a modular monolith with durable workers, outbox processing, and explicit adapter boundaries for external systems.
- Clients request; the server decides. Client inputs are never final financial or security authority.
- Tenant isolation, authorization, auditability, idempotency, bounded resources, and recoverability are product requirements—not implementation details.
- Financial and inventory history is preserved. Corrections are explicit compensating operations, never silent destructive edits.
- Payments, MRA EIS, and other providers are untrusted external boundaries. Unknown outcomes are represented and reconciled, never guessed.
- Offline capability supports continuity; it never becomes permanent client authority.

## Documentation is the source of truth

The design corpus in [docs](docs/README.md) is authoritative for implementation decisions. When sources disagree, use this hierarchy:

```text
applicable law or regulator requirement
→ current external provider contract
→ approved product decision
→ security architecture / implementation contract
→ domain model
→ database, API, sync, and integration specifications
→ testing, observability, and deployment specifications
→ ADRs
→ implementation convenience
```

Read the applicable design and phase specification before changing a boundary. [agent.md](agent.md) defines the repository-wide engineering workflow and contribution rules; it does not replace the design documents.

Start with these documents:

- [Implementation plan](docs/implementation_plan.md) — phase sequencing and non-negotiable engineering rules.
- [Business model](docs/business_model_design.md) — product scope, customers, value, and commercial constraints.
- [Commercial validation plan](docs/commercial_validation_plan.md) — 12 concrete experiments, pricing tests, CAC economics, and PMF gates.
- [System architecture](docs/system_architecture_design.md) — runtime, module, trust, and operational boundaries.
- [Domain model](docs/domain_model.md) — business semantics and invariants.
- [Security implementation specification](docs/security_implementation_spec.md) and [threat model](docs/threat_model.md) — security controls and threats.
- [Documentation index](docs/README.md) — the complete corpus and phase specifications.

## Delivery status

The repository has progressed beyond the initial engineering substrate into the tenancy and authorization foundation.

Current main-branch status includes:

- Phase 1 repository/Rust/CI/supply-chain foundation;
- Phase 2 configuration, secrets, errors, telemetry, and runtime foundation;
- Phase 3 identity/session/MFA/device-identity foundation;
- Phase 4 organization/branch APIs and repository-scope enforcement;
- the current Phase 4 RLS implementation contract.

Commercially, the product remains **pre-validation**. The business model is an enterprise target model and its pricing, ICP, channel, retention, and PMF assumptions remain hypotheses until validated through the [Commercial Validation Plan](docs/commercial_validation_plan.md).

The PostgreSQL integration-test harness remains explicitly deferred where stated by the phase documentation. Product implementation must continue to follow the applicable phase contracts and security architecture.

## Commercial direction

Sitolo's category remains **Business Operating System for African SMEs**, while the initial commercial wedge is **inventory-heavy small and growing retail** where sales, stock, cash, payment, and reconciliation problems can be measured directly.

The commercial progression is:

~~~text
SELL
→ STOCK
→ CASH / PAYMENTS
→ RECONCILIATION
→ CONTROL
→ BRANCH / ENTERPRISE
~~~

The repository intentionally separates enterprise-capable architecture from unvalidated commercial assumptions. Pricing, channels, retention, and PMF are measured through controlled experiments rather than treated as settled facts.

## Local verification

Run the canonical verifier from a clean checkout:

```sh
./scripts/ci/verify
```

On Windows:

```powershell
.\scripts\ci\verify.ps1
```

## Contributing and security

- [Contribution guide](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [License](LICENSE) — Apache License 2.0
