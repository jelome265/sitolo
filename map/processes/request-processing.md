---
type: process
status: stub
universe: ghost
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: apps/api/src/serve.rs; crates/sitolo-api/src/auth.rs; docs/api_contract.md
source_citation: apps/api/src/serve.rs:46; crates/sitolo-api/src/auth.rs:75
---

# Authenticated Request Processing

Input: authenticated client intent.

## Movement

1. Accept and bound the HTTP request before expensive work. (apps/api/src/serve.rs:46-58; docs/api_contract.md)
2. Extract and validate authentication material where the authentication path is wired. (crates/sitolo-api/src/auth.rs:25-64)
3. Establish session/device security context before downstream authorization. (crates/sitolo-api/src/auth.rs:77-106)
4. Resolve tenant and authorization context before domain mutation. (docs/api_contract.md; docs/phase4_tenant_organization_branch_iam_implementation.md)
5. Persist authoritative state and required evidence inside the applicable transaction. (docs/database_design.md; docs/phase4_part8_audit_outbox_implementation_contract.md)

## Output

Authoritative domain result plus explicit error and side-effect state.

## Consumes

[API Boundary](../objects/runtime/api-boundary.md) · [Authentication](../objects/security/authentication.md) · [Tenancy](../objects/security/tenancy.md) · [Authorization](../objects/security/authorization.md)

## Produces

[Persistence](../objects/data/persistence.md) · [Events and Audit](../objects/reliability/events-and-audit.md)

## If you change this

### Hits

API, authentication, tenancy, authorization, domain, persistence, audit and observability.

### Does not hit

Unrelated presentation code unless the contract changes.

## Verification

Ghost/unwired as a complete end-to-end flow: the current API process contains real transport/auth helpers and tenancy operations, but the full authenticated business-mutation path is not wired as one executable sequence.

## See

docs/api_contract.md; docs/system_architecture_design.md