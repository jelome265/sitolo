# Sitolo Security Control Register

**Status:** Current security-control traceability authority  
**Primary owner:** Engineering for technical controls; Trust & Compliance for regulatory/security governance.

## Purpose

Trace material Sitolo security requirements from threat or obligation through control, ownership, implementation evidence, testing, CI/release enforcement, and residual risk. This register does not replace the threat model, security architecture, security implementation specification, testing strategy, or applicable law/regulation.

## Required trace

```text
Threat / obligation
    ↓
Security control
    ↓
Primary owner
    ↓
Authoritative contract
    ↓
Implementation evidence
    ↓
Security test / evidence
    ↓
CI / release gate
    ↓
Status + residual risk / exception
```

## Control ownership

| Control family | Primary owner | Consulted |
|---|---|---|
| Identity, sessions, MFA and device trust | Engineering | Trust & Compliance |
| Tenant / organization / branch isolation | Engineering | Trust & Compliance |
| Authorization and privileged access | Engineering | Trust & Compliance |
| API/input/boundary security | Engineering | Product, Trust & Compliance |
| Financial / inventory integrity | Engineering | Product, Commercial |
| Offline/synchronization security | Engineering | Product, Customer Operations |
| Payment / fiscal external trust | Trust & Compliance | Engineering, Commercial |
| Audit evidence / security telemetry | Engineering | Trust & Compliance |
| Privacy/data governance | Trust & Compliance | Engineering, Customer Operations |
| Supply-chain / release security | Engineering | Trust & Compliance |
| Security incident response | Trust & Compliance | Engineering, Customer Operations |
| Security exceptions / risk acceptance | Trust & Compliance | Engineering |

## Current register

| ID | Control | Authoritative sources | Implementation evidence | Verification / gate | Status |
|---|---|---|---|---|---|
| SC-001 | Authentication/session/device state establishes trustworthy identity before protected work | security_architecture_design.md; auth_authorization_spec.md; phase3_identity_sessions_mfa_device_identity_implementation.md | crates/sitolo-auth/ | Security harness + auth/session tests + security CI | Partial |
| SC-002 | Tenant/org/branch scope is server-derived; caller IDs never become authority | security_architecture_design.md; auth_authorization_spec.md; phase4_tenant_organization_branch_iam_implementation.md | crates/sitolo-tenancy/; crates/sitolo-application/src/tenancy.rs; crates/sitolo-api/src/tenancy.rs | tenancy API tests; real PostgreSQL RLS suite; canonical verifier | Partial |
| SC-003 | Authorization evaluates principal, session/device, membership, scope, permission, target/state, entitlement and assurance; missing facts fail closed | auth_authorization_spec.md; security_implementation_spec.md; phase6_authorization_engine_policy_enforcement_implementation.md | crates/sitolo-authz/; tenancy/IAM orchestration | security negative matrix; policy enforcement tests | Partial |
| SC-004 | Inputs and API boundaries are bounded and resistant to injection, SSRF, traversal, unsafe files/redirects and malformed state | security_implementation_spec.md; api_contract.md; threat_model.md | crates/sitolo-api/; crates/sitolo-security/ | security harness/API negatives; CodeQL where applicable | Contracted; surface-dependent |
| SC-005 | Secrets and crypto material have narrow storage, access and lifecycle boundaries | security_implementation_spec.md; deployment_spec.md; security_architecture_design.md | crates/sitolo-security/; config/secret boundary | secret tests; Gitleaks; security CI | Contracted; production deployment-specific |
| SC-006 | Financial/inventory mutations preserve authorization, transaction integrity, idempotency/replay and durable evidence | security_architecture_design.md; security_implementation_spec.md; domain_model.md; database_design.md | Current audit/security foundations; broader business engines phase-gated | security harness + future domain mutation tests | Contracted |
| SC-007 | Offline commands remain untrusted until server validation, are stable, replay-protected and reconcilable | security_architecture_design.md; security_implementation_spec.md; sync_protocol.md | crates/sitolo-sync/ is currently a contract boundary | future sync/replay suites | Contracted |
| SC-008 | External callbacks/webhooks are authenticated, replay-protected and cannot redefine internal truth | security_architecture_design.md; security_implementation_spec.md; payment_integration_spec.md; mra_eis_integration_spec.md | integrations remain contract boundary | provider/replay contract suites | Contracted |
| SC-009 | Security-sensitive/economic actions have durable audit evidence distinct from telemetry | security_architecture_design.md; phase4_part8_audit_outbox_implementation_contract.md; observability_spec.md | crates/sitolo-audit/ and current audit/outbox stream | audit/integration verification | Implemented foundation |
| SC-010 | Resource limits bound requests, concurrency, retries, queues and expensive work; uncertainty fails closed where authority matters | security_architecture_design.md; security_implementation_spec.md; deployment_spec.md; threat_model.md | runtime/security primitives and CI | resource/failure suites + canonical verifier | Contracted |
| SC-011 | Source, dependencies, toolchains and released artifacts are attributable and policy-checked | security_implementation_spec.md; deployment_spec.md; ci_enforcement.md | Cargo.lock; rust-toolchain.toml; deny.toml; pinned actions | cargo-deny; cargo-audit; CodeQL; Gitleaks; toolchain CI | Verified for configured CI surface |
| SC-012 | Privileged support/admin access is separately controlled, auditable and least-privileged | threat_model.md; security_implementation_spec.md | Support/admin control plane remains phase-gated | privileged-access security suites | Contracted |

## Mandatory engineering rule

Every material security-sensitive change must identify one or more control IDs. The run brief records applicability; the plan maps controls to implementation and tests; implementation records changed evidence; audit independently evaluates the claim; verification records executed proof and residual limitations.

A contract alone is not evidence of implementation. A passing test is evidence only when it exercises the claimed boundary.

## Known evidence limitation

The security-test corpus contains the bounded PostgreSQL testing exception `SCOPE-EXC-001`. It does not authorize production claims about database/RLS semantics and remains governed by `docs/security_test_harness.md`.

## Change triggers

Update this register when authentication, authorization, tenant isolation, sensitive data, crypto/secrets, financial/inventory integrity, offline/sync trust, provider integrations, audit evidence, privileged access, CI/release trust, recovery, or regulatory/security obligations materially change.
