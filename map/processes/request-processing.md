# Authenticated Request Processing

- type: process
- status: stub
- source: docs/system_architecture_design.md; docs/api_contract.md

## Input

Authenticated client intent.

## Movement

1. API receives the request.
2. Authentication establishes the principal.
3. Tenant and authorization context is resolved.
4. Application and domain validation enforce invariants.
5. PostgreSQL commits authoritative state when applicable.
6. Audit, outbox or derived work is registered when required.

## Output

Authoritative domain result plus explicit error and side-effect state.

## If you change this

### Hits

API, authentication, tenancy, authorization, domain, persistence, audit and observability.

### Does not hit

Unrelated presentation code unless the contract changes.

## See

docs/system_architecture_design.md
