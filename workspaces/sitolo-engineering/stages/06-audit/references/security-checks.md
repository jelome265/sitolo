# Security Audit Checks

Apply only the checks relevant to the changed trust boundaries. Use the security control register for control ownership and evidence authority.

## Identity and authorization

- Can an attacker invoke the operation without a trustworthy principal/session/device state?
- Can caller-controlled identifiers select another tenant, organization, branch, resource, or privileged action?
- Does missing membership, scope, entitlement, assurance, or approval fail closed?
- Can a lower-privileged actor reach an administrative or support path?

## Input and boundary security

- Are untrusted inputs bounded, normalized, and validated at the real enforcement boundary?
- Are injection, traversal, SSRF, unsafe file/redirect, malformed-state, or parser ambiguity paths possible?
- Are external callbacks authenticated and replay-protected?

## Integrity and replay

- Can a mutation be replayed, reordered, duplicated, or partially applied?
- Is idempotency enforced at the authoritative boundary?
- Can offline or stale state bypass server validation?

## Secrets and data

- Can secrets, credentials, tokens, signing material, or sensitive data cross an unintended boundary?
- Is sensitive evidence exposed through logs, errors, telemetry, exports, or audit records?

## Availability and abuse

- Can attacker-controlled input trigger unbounded work, concurrency, retries, memory, queue growth, or expensive downstream calls?
- Do failure paths fail closed where authority matters?

## Evidence rule

A security finding is not closed merely because a test exists. Establish that the test exercises the claimed production boundary and proves the required property.
