# R2 Authenticated Authority — Implementation Slice

**Date:** 2026-10-22  
**Branch:** `feat/phase4-pr009-audit-outbox`  
**Status:** Partial implementation. AUD-02 remains **OPEN**; protected HTTP routes remain fail-closed.

## Implemented

- Added `RequestIdentityAuthority`, the minimal server-side persistence contract used after external token verification.
- Added explicit, exact `(issuer, subject) -> UserId` binding and lookup in the in-memory reference store. Binding rejects unknown users and conflicting reassignment. Request authentication never auto-links by email or creates a Sitolo user from token claims.
- Added `authenticate_request` to the API boundary. It bounds and parses the Session-ID selector before provider I/O, validates the bearer through `IdentityProviderPort`, resolves the exact mapped identity, requires an active session owned by that user, validates user security version/suspension and bound-device state, and constructs the existing `SecurityContext` without retaining raw bearer material.
- Added `PgRequestIdentityAuthority` and `phase3_identity_authority.sql` for exact external-identity lookup and persistent session acceptance. A narrow non-login owner role owns the identity tables. Runtime access is limited to security-definer lookup/acceptance functions; session selection verifies owner, active state, account state, security version, device state, and both expiry clocks, then updates idle activity atomically.
- Added API regressions for accepted mapped identity, unknown identity denial, and cross-user session-selector denial. Added a PostgreSQL integration regression for persistent exact mapping, active-session acceptance, idle activity update, and cross-user denial. PostgreSQL integration target compiles locally; its database execution is left to PostgreSQL-enabled CI.
- Updated the Phase 3 public error registry with `IDENTITY_AUTHORITY_UNAVAILABLE` (generic 503 mapping).

## Verification performed on the working tree

- `cargo fmt --all` — passed.
- `cargo check --workspace --all-targets --all-features --locked` — passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — passed.
- `cargo test -p sitolo-api --lib --locked` — passed, 20 tests including the three new authentication-flow cases.
- `cargo test -p sitolo-persistence --test part8_postgres_integration_tests --no-run --locked` — passed; compilation only, not database evidence.
- `git diff --check` — passed.
- No local PostgreSQL instance was started or used. The new SQL fixture has not yet executed against PostgreSQL on this source revision; exact-source CI is required.

## Remaining R2 blockers

- `apps/api/src/serve.rs` still uses the custom raw TCP boundary. The new authentication function is not invoked by live routing.
- `AppState` and production bootstrap still do not compose the Cognito verifier, a PostgreSQL pool, or `PgRequestIdentityAuthority`. Production startup continues to fail closed without its managed secret provider.
- Durable user/session/device creation, identity-link provisioning, and revocation mutations are not implemented through the PostgreSQL authority. The new schema defines lookup/acceptance storage only; it is not yet part of a production migration/release workflow.
- Membership, tenant, operation permission, and resource authorization are not resolved at the live application command boundary. Authentication alone must never enable a protected command.
- The Session-ID header is an index only; the validated bearer and database ownership checks establish identity/session authority. Do not accept a client-selected `UserId`, tenant, role, or permission.
- Do not compose `TestIdentityProvider` into a served runtime.

The follow-up contract still requires an Axum route inventory with per-route permission/public reason, request-body limit, timeout, and snapshot; live HTTP negative/positive authorization tests; same-transaction command authorization; exact-source PostgreSQL CI; and Stage 06 re-audit. AUD-02 is not closed by this slice.
