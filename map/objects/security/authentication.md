---
type: object
status: verified
cluster: security-domain
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: crates/sitolo-auth/src/lib.rs
source_citation: crates/sitolo-auth/src/lib.rs:30
---
# Authentication


## Why this shape

Authentication establishes the principal before authorization and never substitutes for authorization.

## Shape

Primary implementation: crates/sitolo-auth. Related identity and session contracts live in docs/phase3_identity_sessions_mfa_device_identity_implementation.md.

## Connected to

Sessions, devices, tenancy, authorization, API and audit.

## If you change this

### Hits

Principal identity, sessions, device trust, authorization context and security tests.

### Does not hit

Business domain rules unrelated to identity.

## Surfaces

API authentication boundary, application services and security tests.

## See

docs/phase3_identity_sessions_mfa_device_identity_implementation.md

