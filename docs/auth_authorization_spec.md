# Sitolo — Authentication and Authorization Contract

**Status:** reconstructed canonical contract from Phase 3 identity/session/device work, Phase 4 tenant/IAM work, Phase 6 authorization enforcement, security implementation and current API/domain boundaries.

## Authentication

Authentication establishes the principal. A valid request requires applicable identity and session state; revoked, expired or otherwise invalid state cannot authorize work.

Authentication context includes, as applicable:

- principal identity
- session identity/state
- device identity/state
- authentication assurance
- organization/tenant context

Authentication is a prerequisite, not permission.

## Authorization decision

```text
principal
+ session
+ device state
+ active membership
+ organization scope
+ branch/resource scope
+ action/permission
+ target resource
+ target state
+ property constraints
+ entitlement
+ assurance
+ approval/context
= decision
```

The decision is fail-closed. Missing, stale, revoked, contradictory or unavailable mandatory facts cannot silently become allow.

## Enforcement points

Authorization must be enforced at the server-side operation boundary, not solely by UI visibility or route naming. Database/RLS protections provide an additional boundary for tenant-owned data.

## Scope model

```text
Principal
  -> Organization/Tenant
     -> Membership / role / permission
        -> Branch or resource scope
           -> operation
```

A caller cannot select a tenant, organization, branch or resource outside its authorized scope merely by changing an identifier in a request.

## Object and property authorization

Authorization may depend on the target resource and its current state, not only on a role. Sensitive fields/actions may require additional property-level controls or elevated assurance.

## Entitlements

Plan/module entitlements are a product-control input. They do not replace security authorization. A user can be authorized for an operation but still be outside the enabled commercial capability; conversely, a plan never grants authority to an unauthorized principal.

## Mutations

Material mutations must preserve transaction integrity and use idempotency/replay protection where the operation can be retried or originated offline. Authorization is evaluated against current authoritative state at the enforcement boundary.

## Offline commands

Offline-originated commands are treated as untrusted input. They require stable command identity, authenticated/scope context, replay protection, server validation and deterministic reconciliation. Local UI state is not an authorization source.

## Audit

Security-sensitive authorization outcomes and privileged mutations must have sufficient evidence for operational investigation and compliance requirements. Audit evidence is append-only where the governing audit contract requires it.

## Canonical references

- `phase3_identity_sessions_mfa_device_identity_implementation.md`
- `phase4_tenant_organization_branch_iam_implementation.md`
- `phase6_authorization_engine_policy_enforcement_implementation.md`
- `security_architecture_design.md`
- `security_implementation_spec.md`
- `threat_model.md`

This contract restores the filename referenced by the existing documentation corpus and consolidates the already-established identity/authorization rules without introducing a second policy model.