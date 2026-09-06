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
- [System architecture](docs/system_architecture_design.md) — runtime, module, trust, and operational boundaries.
- [Domain model](docs/domain_model.md) — business semantics and invariants.
- [Security implementation specification](docs/security_implementation_spec.md) and [threat model](docs/threat_model.md) — security controls and threats.
- [Documentation index](docs/README.md) — the complete corpus and phase specifications.

## Delivery status

The repository currently has the Phase 1 engineering substrate and Phase 2 runtime foundation:

- pinned Rust workspace, locked dependency graph, CI/security policy, artifact identity, SBOM, provenance, and protected release-promotion verification;
- typed configuration and secret boundaries, safe error mapping, structured telemetry contracts, and operational runbooks.

The PostgreSQL integration-test harness is explicitly deferred. The next planned product foundations are identity, sessions, MFA, device identity, tenant/organization/branch scope, IAM, PostgreSQL schema/RLS, and authorization enforcement, as defined by the phase documents.

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
