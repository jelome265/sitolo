# Sitolo — Security Architecture Design

**Status:** reconstructed canonical security-architecture baseline from the existing threat model, security implementation specification, authorization/identity phase contracts, database/RLS contract, deployment contract and current source boundaries.

## Security objectives

Sitolo uses Zero Trust, least privilege, deny-by-default, server-authoritative decisions, tenant isolation, fail-closed behavior and evidence-driven verification.

## Trust boundaries

```text
Client
  -> TLS/API boundary
     -> authentication/session
        -> tenant + organization + branch scope
           -> authorization
              -> domain/application mutation
                 -> PostgreSQL
                    -> audit/outbox
                       -> worker/integration boundary
```

External payment/tax providers, client offline stores, exported files and operational tooling are separate trust boundaries.

## Security control families

### Identity and session

Authentication establishes a principal. Sessions and device state must be validated and revocable. Authentication never substitutes for authorization.

### Authorization

Authorization evaluates principal, session, membership, organization/branch scope, permission, target resource/state, properties, entitlements, required assurance and contextual policy. Unknown or stale required facts fail closed.

### Tenant isolation

Every tenant-owned operation must carry authoritative tenant context. Application checks and PostgreSQL/RLS controls are defense in depth; client-supplied tenant identifiers are never trusted as authority.

### Data protection

TLS protects network transport. Secrets remain outside source control and are isolated from ordinary configuration. Sensitive data must be redacted from logs and protected at rest according to deployment requirements.

### Input and boundary security

Validate request structure and business invariants at the appropriate boundary. Prevent injection, SSRF, path traversal, unsafe file handling and untrusted redirect/callback behavior.

### Financial/inventory integrity

Money and stock mutations require authorization, transaction integrity, idempotency/replay protection where applicable, and durable audit evidence for material actions.

### Offline security

Offline commands are untrusted until server validation. Commands carry stable identity and scope, are replay-protected, and converge through server-authoritative rules.

### Integrations

Provider credentials are isolated. Webhooks/callbacks are authenticated and validated. Unknown external outcomes are represented as unknown rather than guessed success/failure. Retries must be bounded and idempotent.

### Audit and observability

Security-relevant actions must produce sufficient audit evidence. Logs/metrics/traces are operational evidence and never become authorization authority.

### Supply chain and release

Pinned toolchains/actions, dependency governance, lockfile integrity, vulnerability scanning, formatting/lint/test gates and security tests are release controls.

## Security governance and evidence

Security controls are managed through `security_control_register.md`. Material changes are traced from threat/obligation → control ID → owner → implementation evidence → security test/evidence → CI/release gate → residual risk or exception. Incident handling, exceptions and cryptographic key lifecycle are governed by the dedicated security operations documents routed through the Engineering security context.

## Security verification

Security controls are release-blocking where the governing implementation contracts mark them mandatory. Negative tests must cover cross-tenant access, scope escalation, stale/revoked identity, replay, authorization bypass, unsafe integration behavior and relevant data-boundary failures.

## Canonical references

- `threat_model.md`
- `security_implementation_spec.md`
- `phase3_identity_sessions_mfa_device_identity_implementation.md`
- `phase4_tenant_organization_branch_iam_implementation.md`
- `phase6_authorization_engine_policy_enforcement_implementation.md`
- `phase7_security_test_framework_implementation.md`
- `database_design.md`
- `deployment_spec.md`

This file restores the security-architecture authority referenced throughout the corpus; implementation details remain in the subordinate contracts.