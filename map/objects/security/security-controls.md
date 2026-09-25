# Security Controls

- type: object
- status: verified
- source revision: Sitolo main at e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
- cluster: security-domain

## Why this shape

Cross-cutting security primitives enforce fail-closed behavior and guard trust boundaries without owning business semantics.

## Shape

Primary implementation: crates/sitolo-security. Canonical implementation controls: docs/security_implementation_spec.md and docs/threat_model.md.

## Connected to

Authentication, authorization, tenancy, API, persistence and CI/security gates.

## If you change this

### Hits

Security invariants, boundary controls, negative tests and release gates.

### Does not hit

Product behavior that consumes the unchanged security boundary.

## Surfaces

API/application security paths, workers and security tests.

## See

docs/security_implementation_spec.md
