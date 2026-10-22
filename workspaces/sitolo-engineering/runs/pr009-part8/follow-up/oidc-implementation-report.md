# Cognito OIDC implementation slice

**Status:** partial implementation; R2 remains open.

## Implemented

- Added a Cognito adapter in `crates/sitolo-integrations` behind the existing `IdentityProviderPort`; the domain/auth crate remains protocol-neutral.
- Validates access tokens using issuer-pinned JWKS, RS256 only, exact issuer and client ID, Cognito `token_use=access`, expiry/not-before, configured scopes, and optional resource audience.
- Bounds token and provider response sizes, request duration, JWKS size/key count/cache age, and refresh frequency. HTTPS is required and redirects are disabled.
- Rejects JWKS key-ID ambiguity and returns only `(issuer, subject)` identity facts from token validation. Tenant, membership, role and permission authority is not accepted from Cognito claims.
- Uses `SecretValue` for returned access/refresh/ID token values and redacts their `Debug` representation.

## Verification performed locally

- `cargo fmt --all -- --check` — passed after formatting.
- `cargo test -p sitolo-integrations --all-targets --locked` — 8 tests passed, including signed-token validation, negative claim/signature cases, bounded malformed-token handling and duplicate JWKS `kid` rejection.
- `cargo clippy -p sitolo-integrations -p sitolo-auth --all-targets --locked -- -D warnings` — passed.
- `cargo check --workspace --all-targets --all-features --locked` — passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — passed.
- `cargo deny check` — not run: `cargo-deny` is not installed in this environment. This is not recorded as passing.
- PostgreSQL was not installed, started, or run locally. No database verification is claimed by this report.

## Remaining R2 work

The production API does not yet compose this adapter. `AppState` has no OIDC provider, durable exact `(issuer, subject) -> UserId` mapping, persistent Sitolo session lookup, or per-request membership/tenant/permission resolver. The HTTP listener remains fail-closed for protected mutations and must stay that way until the complete verified authority path is composed. The custom TCP HTTP boundary also remains to be replaced by the approved Axum boundary before authenticated routes are enabled.

This slice does not close AUD-02 and does not claim production authentication readiness. Remaining work includes durable identity/session persistence, session/device/revocation enforcement, Axum request extraction, server-derived authorization, runtime composition, and negative database-backed CI evidence.

## Dependency rationale

- `jsonwebtoken` supplies maintained JOSE/JWT parsing and signature verification; the adapter pins RS256 and selects the AWS-LC implementation instead of accepting token-selected algorithms.
- `reqwest` supplies bounded HTTPS provider calls using rustls with certificate validation; redirects are disabled and no caller-controlled URL is fetched.

Both are workspace-pinned through `Cargo.lock`. Supply-chain approval remains subject to the repository's `cargo deny` / `cargo vet` gates.
