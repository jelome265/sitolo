---
type: object
status: verified
universe: live
cluster: security-domain
source_revision: main@623f7aed6105664d10d1a2802480fde8316ed5f8
source: crates/sitolo-security/src/lib.rs
source_citation: crates/sitolo-security/src/lib.rs:21
---
# Security Controls


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

