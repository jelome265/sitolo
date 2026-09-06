# SITOLO — PHASE 3 IMPLEMENTATION SPECIFICATION

## Identity, Sessions, MFA, Recovery & Device Identity

**Product:** Sitolo — Business Operating System for African SMEs  
**Phase:** Phase 3 — Identity + Sessions + MFA + Device Identity  
**Status:** Implementation-governing specification  
**Primary market:** Malawi first; controlled African regional expansion  
**Backend:** Rust + Axum + Tokio  
**Persistence:** PostgreSQL authoritative server state  
**Mobile:** Flutter / Android-first  
**Desktop:** Tauri  
**Web/Admin:** TypeScript/Next.js where materially useful  
**Offline client store:** SQLite  
**Architecture:** Modular monolith first, workers/adapters at explicit boundaries  
**Predecessors:** Phase 0 contracts + Phase 1 repository/workspace/CI + Phase 2 config/secrets/logging/errors/telemetry  
**Primary dependencies:** `auth_authorization_spec.md`, `api_contract.md`, `security_implementation_spec.md`, `domain_model.md`, `database_design.md`, `observability_spec.md`, `threat_model.md`, `testing_strategy.md`, `ADR-001-025.md`

> **Binding statement:** Sitolo authentication establishes identity; session management establishes an active authenticated relationship; device identity establishes an independently revocable operational security subject; MFA establishes stronger authentication assurance; authorization and business-domain rules determine what the authenticated subject may actually do.

---

# 0. Executive Contract

Phase 3 builds the security foundation on which every later Sitolo business operation depends. This phase does **not** implement tenant authorization, catalogue, inventory, POS, payments or EIS business workflows. It establishes the identity primitives those domains are allowed to consume.

The implementation must preserve the existing Sitolo authority model:

```text
IDENTITY PROVIDER / AUTHENTICATOR
                |
                v
        IDENTITY ASSERTION
                |
                v
       RUST AUTHENTICATION
                |
                v
       SESSION ESTABLISHMENT
                |
                +--> DEVICE BINDING / STATE
                |
                +--> ASSURANCE / MFA
                |
                v
        TRUSTED PRINCIPAL
                |
                v
        TENANT / MEMBERSHIP
                |
                v
       AUTHORIZATION ENGINE
                |
                v
        DOMAIN OPERATION
```

The following are non-negotiable:

1. Sitolo MUST NOT invent a proprietary authentication protocol where a mature standards-based protocol or maintained implementation is appropriate.
2. Passwords MUST NOT be stored in plaintext or reversible form.
3. Authentication credentials MUST NOT be written to logs, traces, metrics, audit events or error responses.
4. Access tokens MUST be validated cryptographically and semantically before identity is established.
5. Token claims MUST NOT be treated as complete application authorization state.
6. Session validity MUST be independently enforceable from token validity where the chosen session architecture requires server-side state.
7. Refresh credentials MUST be rotation-aware and replay-detecting where refresh-token architecture is used.
8. MFA enrollment, challenge, recovery and reset MUST be represented as explicit stateful workflows.
9. Device identity MUST be independent from user identity and independently revocable.
10. A device identifier MUST NEVER be accepted as proof of authentication by itself.
11. Device registration MUST be scoped to an organization/security context only after authenticated enrollment rules succeed.
12. Session, authenticator and device state changes MUST be auditable.
13. Authentication and recovery flows MUST resist account enumeration and brute-force abuse.
14. High-risk operations MUST be capable of requiring stronger authentication assurance through step-up authentication.
15. Logout, revocation and security events MUST have deterministic semantics.
16. Offline capability MUST remain bounded authority. A locally cached session MUST NOT become an unrestricted offline administrator credential.
17. All security-sensitive state transitions MUST be concurrency-safe and idempotent where duplicate delivery is possible.
18. Authentication infrastructure failure MUST fail closed for operations whose authority cannot be safely established.
19. Telemetry failure MUST NOT block core authentication or business correctness except where an explicit compliance/evidence control requires durable evidence.
20. No client must ever be able to select its own security authority by supplying a tenant, role, permission, assurance, approval or device-trust value.

---

# 1. Relationship to Existing Sitolo Contracts

This file is an implementation specification, not a replacement for the existing Phase 0 contracts.

## 1.1 Governing sources

The implementation precedence remains:

```text
Applicable law / binding regulation
        |
        v
External identity/provider contract
        |
        v
security_architecture_design.md
        |
        v
system_architecture_design.md
        |
        v
security_implementation_spec.md
        |
        v
auth_authorization_spec.md
        |
        v
api_contract.md / database_design.md / observability_spec.md
        |
        v
THIS PHASE 3 IMPLEMENTATION SPEC
        |
        v
SOURCE CODE
```

The existing identity specification already establishes the core security equation:

```text
VALID PRINCIPAL
+ VALID SESSION
+ VALID DEVICE STATE
+ ACTIVE MEMBERSHIP
+ CORRECT ORGANIZATION
+ CORRECT SCOPE
+ EXPLICIT PERMISSION
+ OBJECT AUTHORIZATION
+ PROPERTY AUTHORIZATION
+ VALID DOMAIN STATE
+ ENTITLEMENT
+ REQUIRED ASSURANCE
+ REQUIRED APPROVAL
= AUTHORIZED OPERATION
```

Phase 3 owns the first three security facts in that equation. Tenant membership and authorization remain Phase 4/6 concerns.

## 1.2 Existing accepted architectural decisions

The existing ADR set explicitly accepts standards-based authentication instead of custom authentication cryptography, and it explicitly separates user, session and device identity. The implementation therefore must not introduce a new competing identity model. The existing ADRs identify standards-based authentication, independent device identity, durable workers and structured domain errors as accepted architectural directions. [Project source: `ADR-001-025.md`]

The existing authentication contract requires OAuth/OIDC-aligned protocol handling, PKCE for public native applications, server-side session control, Argon2id where passwords exist, MFA/recovery protections, security versioning and independently revocable devices. [Project source: `auth_authorization_spec.md`]

## 1.3 Current external standards baseline

The current baseline is:

- RFC 9700, OAuth 2.0 Security Best Current Practice.
- RFC 8252, OAuth 2.0 for Native Apps.
- NIST SP 800-63-4, Digital Identity Guidelines.
- OWASP Authentication, Session Management, Password Storage and related guidance.
- WebAuthn/FIDO standards where passkeys are implemented.

RFC 9700 is the current OAuth 2.0 security BCP and strengthens guidance around PKCE, redirect binding and retirement of weaker OAuth modes. citeturn130764search0

RFC 8252 requires public native applications to use PKCE and recommends using an external user agent rather than embedded web views for authorization. citeturn130764search1turn130764search2

NIST SP 800-63-4 is the current 2025 revision of the Digital Identity Guidelines and covers identity proofing, authenticators, authentication processes, federation and authenticator lifecycle management. citeturn130764search3turn130764search10

NIST's session-management guidance describes server-controlled sessions, reauthentication, overall and inactivity timeouts, and explicit session termination. citeturn130764search6

NIST's authenticator-event guidance requires lifecycle handling for binding, loss, theft, compromise, expiration and revocation. citeturn130764search7

---

# 2. Phase 3 Objective

Phase 3 is complete only when Sitolo can establish, maintain, revoke and investigate authenticated identities under normal operation and hostile conditions.

The target security property is:

```text
UNKNOWN ACTOR
    |
    v
AUTHENTICATION ATTEMPT
    |
    +--> invalid --> controlled failure
    |
    v
AUTHENTICATION SUCCESS
    |
    v
SESSION CREATED
    |
    +--> DEVICE STATE CHECK
    |
    +--> ASSURANCE CHECK
    |
    v
TRUSTED PRINCIPAL
    |
    +--> session expiration
    +--> session revocation
    +--> device revocation
    +--> membership revocation
    +--> refresh rotation
    +--> MFA state change
    |
    v
AUTHORITY RE-EVALUATED
```

The system must be safe across:

- valid login;
- wrong password;
- unknown account;
- credential stuffing;
- MFA failure;
- lost phone;
- stolen device;
- token theft;
- refresh-token replay;
- session theft;
- revoked device;
- revoked membership;
- password reset;
- MFA reset;
- provider outage;
- JWKS key rotation;
- clock skew;
- duplicate callbacks;
- partially completed enrollment;
- concurrent login/logout/revocation;
- offline client restart;
- compromised client attempts to fabricate device or assurance metadata.

---

# 3. Scope

## 3.1 In scope

This phase implements or establishes:

- identity-provider trust configuration;
- authentication boundary interfaces;
- normalized principal representation;
- OAuth/OIDC browser/native login integration boundaries;
- password authentication if enabled;
- password hashing and rehash policy;
- session creation;
- session lifecycle;
- access-token validation;
- refresh-token lifecycle;
- refresh replay detection;
- logout;
- global and selective revocation;
- MFA enrollment;
- MFA challenge/verification;
- TOTP lifecycle if enabled;
- passkey/WebAuthn integration boundary;
- recovery codes;
- password reset;
- account recovery safeguards;
- authentication assurance levels;
- step-up authentication artifacts;
- device registration;
- device lifecycle;
- device revocation;
- device replacement;
- device security versioning;
- authentication security events;
- authentication metrics;
- rate-limit hooks;
- error mappings;
- security tests;
- concurrency/idempotency tests;
- negative tests;
- audit integration points;
- observability integration;
- migration model for identity/session/device records.

## 3.2 Explicitly out of scope

The following remain later-phase responsibilities:

- full tenant and organization IAM;
- catalogue authorization;
- inventory permissions;
- POS authorization rules;
- payment entitlements;
- MRA/EIS permissions;
- support impersonation implementation;
- full entitlement engine;
- production provider-specific business contracts not yet selected;
- country-specific legal identity proofing beyond what the selected provider requires and what Sitolo explicitly needs.

---

# 4. Identity Architecture

## 4.1 Identity subjects

Sitolo distinguishes at least these security subjects:

```text
Human User
   |
   +--> authenticators
   +--> sessions
   +--> devices
   +--> memberships

Service Identity
   |
   +--> worker/service credentials

Support Identity
   |
   +--> elevated operational context

Platform Administrator
   |
   +--> administrative assurance

External Provider
   |
   +--> verified integration identity
```

The key invariant is:

```text
USER != AUTHENTICATOR != SESSION != DEVICE != MEMBERSHIP
```

Collapsing these concepts is an architectural defect because each has different lifecycle and revocation semantics.

## 4.2 Why the separation matters

A stolen Android phone should permit the operator to revoke that device without necessarily deleting the employee's identity.

A password reset should invalidate sessions without deleting historical business records.

A membership removal should immediately remove business authority even while the user's authentication credential remains valid.

An MFA reset should not silently restore previously revoked devices.

A user may have multiple active sessions and multiple devices.

A shared point-of-sale device may host different cashier sessions over time.

The data model therefore requires explicit relationships rather than a single `users` table containing dozens of security-state flags.

---

# 5. Trust Boundaries

```text
                    INTERNET
                       |
                 Untrusted Clients
                       |
              +--------+--------+
              |                 |
          Flutter           Browser/Tauri
              |                 |
              +--------+--------+
                       |
                       v
                  EDGE / TLS
                       |
                       v
                  RUST API
                       |
          +------------+-------------+
          |            |             |
          v            v             v
     Auth Adapter   Session      Device Service
          |            |             |
          v            v             v
     Identity       PostgreSQL    PostgreSQL
      Provider
          |
          v
      External IdP
```

Trust cannot be established merely because traffic arrived over a private network. Internal workers and callbacks still require identity and authorization appropriate to their role.

## 5.1 Threat boundaries

The most important Phase 3 boundaries are:

1. client ↔ API;
2. API ↔ identity provider;
3. API ↔ PostgreSQL;
4. API ↔ secure client storage;
5. authorization callback ↔ native application;
6. refresh token ↔ token endpoint;
7. MFA challenge ↔ authenticator;
8. recovery flow ↔ account state;
9. device registration ↔ existing authenticated identity;
10. administrative security action ↔ elevated assurance.

---

# 6. Authentication Strategy

## 6.1 Build versus reuse

Sitolo should build application-specific identity orchestration but reuse standards-based protocol and cryptographic implementations.

### Build internally

- normalized principal type;
- security context creation;
- session persistence semantics;
- session revocation semantics;
- device lifecycle;
- membership interaction;
- assurance policy;
- step-up policy;
- recovery business rules;
- audit event semantics;
- security telemetry;
- application-specific failure handling.

### Reuse

- OAuth/OIDC protocol implementation;
- PKCE;
- JOSE/JWT validation;
- password hashing implementation;
- WebAuthn/passkey protocol machinery;
- secure randomness;
- secure OS credential storage;
- TLS;
- secret manager/KMS.

### Explicitly prohibited

- custom JWT syntax;
- custom password hash;
- home-grown MFA cryptography;
- custom OAuth grants;
- storing browser tokens in localStorage by default;
- treating client-generated device IDs as credentials;
- putting provider SDK types into domain aggregates unless an ADR explicitly permits it.

## 6.2 Identity provider decision

The exact provider remains an implementation/commercial decision as recorded by the existing ADR. The architecture therefore uses an abstraction:

```text
IdentityProviderPort
    |
    +--> OIDC Provider Adapter
    +--> Future Self-Hosted Provider Adapter
    +--> Test Provider
```

The rest of Sitolo must not depend on vendor-specific user types.

---

# 7. Native Flutter Authentication

The Android-first client is a public OAuth client. It cannot safely hold a durable client secret.

The preferred flow is:

```text
Flutter
  |
  | generate state + PKCE verifier
  v
External browser / user agent
  |
  v
Authorization Server
  |
  | authorization code
  v
OS callback
  |
  v
Flutter validates state
  |
  v
Code + PKCE verifier
  |
  v
Token endpoint
  |
  v
Access + refresh credentials
  |
  v
Secure OS storage
```

RFC 8252 explicitly requires public native applications to use PKCE and recommends external user agents. citeturn130764search1

## 7.1 State parameter

`state` MUST be:

- unpredictable;
- transaction-specific;
- short-lived;
- stored only for the authorization attempt;
- checked exactly during callback handling;
- invalidated after successful or failed completion.

An unsolicited callback must be rejected.

## 7.2 PKCE

Use S256 PKCE.

The client generates:

```text
code_verifier = cryptographically_random_value
code_challenge = BASE64URL(SHA256(code_verifier))
```

The verifier is never transmitted until code redemption.

A verifier must not be reused between unrelated authorization transactions.

## 7.3 Browser restrictions

The client must not embed an arbitrary login webpage inside a WebView merely for convenience. Embedded credential entry creates unnecessary phishing/interception and isolation risk.

## 7.4 Callback validation

The callback handler must:

1. validate expected scheme/host/path according to the registered configuration;
2. validate `state`;
3. reject duplicate/expired authorization transactions;
4. reject unexpected error combinations;
5. avoid logging authorization codes;
6. exchange the authorization code only once;
7. clear the temporary verifier after completion.

---

# 8. Browser and Admin Authentication

Sensitive browser/admin sessions should prefer server-managed sessions or a BFF-style architecture instead of exposing long-lived bearer credentials to JavaScript.

```text
Browser
   |
   | Secure + HttpOnly cookie
   v
BFF / session boundary
   |
   v
Rust API
   |
   v
Session service
```

Cookie-based authentication requires CSRF protection for state-changing requests.

The browser must not store long-lived privileged tokens in `localStorage` merely because doing so is convenient.

Admin access should normally require stronger assurance than ordinary merchant browsing.

---

# 9. Password Authentication

Password authentication is supported only if the selected deployment requires it. Federation/passkeys may reduce password exposure, but Sitolo must not assume that every market or deployment will provide the same authenticator set.

## 9.1 Password storage

Passwords must be transformed with Argon2id through a maintained implementation.

Requirements:

```text
plaintext password
      |
      v
Argon2id
      |
      v
password verifier
```

Store only the verifier and the parameters needed to verify/upgrade it.

Never store:

- plaintext passwords;
- reversible encrypted passwords;
- passwords in user profile objects;
- passwords in logs;
- passwords in traces;
- passwords in telemetry fixtures;
- passwords in crash reports.

## 9.2 Work-factor policy

The exact Argon2id parameters must be benchmarked on the actual authentication infrastructure rather than copied from a blog.

The goal is deliberately asymmetric:

```text
acceptable per-login CPU/memory cost
        <<<<<<<<<<
attacker's aggregate guessing cost
```

But the chosen configuration must not become a self-inflicted denial-of-service vector against low-cost infrastructure.

Authentication concurrency must therefore be bounded.

## 9.3 Rehashing

Store a versioned password-hash policy identifier.

When a user successfully authenticates with an older cost configuration:

```text
verify old hash
    |
    v
authentication succeeds
    |
    v
rehash using current policy
    |
    v
atomic password-verifier update
```

A failed authentication must never trigger rehashing.

---

# 10. Session Model

A session represents an active authenticated relationship between a principal and Sitolo.

Conceptual model:

```text
Session
├── session_id
├── user_id
├── device_id?
├── created_at
├── authenticated_at
├── last_seen_at
├── expires_at
├── idle_expires_at
├── revoked_at?
├── revoke_reason?
├── assurance_level
├── authentication_method
├── security_version
├── session_family_id?
└── metadata classification
```

## 10.1 Session ID requirements

If Sitolo creates opaque session identifiers itself, they must be generated from a cryptographically secure random source.

The identifier must contain no:

- email;
- phone number;
- user ID;
- tenant ID;
- role;
- branch ID;
- timestamp encoding that exposes unnecessary information.

The identifier is an index into server state, not a portable authority claim.

## 10.2 Session state machine

```text
                 +-------------------+
                 |                   |
                 v                   |
CREATED --> ACTIVE ------------------+
  |           |                       |
  |           +--> IDLE_EXPIRED      |
  |           |                       |
  |           +--> ABSOLUTE_EXPIRED  |
  |           |                       |
  |           +--> REVOKED            |
  |           |                       |
  |           +--> SECURITY_REVOKED --+
  |                                   |
  +--> CREATION_FAILED                |
```

Only `ACTIVE` sessions may be accepted for normal authenticated requests.

## 10.3 Session invalidation triggers

At minimum:

- explicit logout;
- administrative revocation;
- password change where policy requires it;
- password reset;
- MFA reset;
- device revocation;
- security compromise event;
- user suspension;
- membership loss for business operations;
- provider revocation where detectable;
- security-version mismatch.

The system must define whether each event revokes:

- current session only;
- all sessions on current device;
- all sessions for user;
- all sessions associated with an authenticator family.

Those semantics must not be guessed by individual handlers.

---

# 11. Session Lifetime

Sitolo uses multiple controls rather than one universal timeout:

```text
overall session lifetime
        +
inactivity lifetime
        +
explicit revocation
        +
security-version invalidation
```

NIST guidance explicitly recognizes both overall and inactivity timeouts and server-side session termination. citeturn130764search6

## 11.1 Merchant POS versus administrative sessions

A merchant cashier session and a platform-admin session have different risk profiles.

Therefore configuration must be class-based:

```text
SESSION_CLASS=merchant
SESSION_CLASS=admin
SESSION_CLASS=break_glass
```

Exact durations are configuration policy and must be evidence-driven.

Do not hard-code a single lifetime into domain code.

## 11.2 Clock handling

Security decisions must use server-controlled time.

Client-supplied timestamps cannot extend:

- token lifetime;
- MFA challenge lifetime;
- session lifetime;
- recovery windows;
- device enrollment windows.

Clock skew tolerance must be explicit and tested.

---

# 12. Refresh Token Architecture

Where the chosen authentication architecture uses refresh tokens, they are treated as high-value credentials.

Preferred lifecycle:

```text
R1 issued
  |
  v
R1 used
  |
  +--> R1 consumed/revoked
  |
  v
R2 issued
```

## 12.1 Refresh token rotation

Each successful refresh results in the prior token becoming unusable under normal rotation semantics.

Persist enough protected state to detect reuse without storing unnecessary plaintext credentials.

Conceptual fields:

```text
family_id
credential_id
subject_id
issued_at
expires_at
consumed_at?
revoked_at?
replaced_by?
security_version
```

## 12.2 Replay detection

If an already-consumed refresh token is presented:

```text
REUSE DETECTED
     |
     +--> security event
     +--> increment risk signal
     +--> revoke token family according to policy
     +--> invalidate affected sessions
     +--> notify where appropriate
```

The correct response must not be "issue another token anyway."

## 12.3 Race condition

Two concurrent refresh requests can arrive using the same token.

The server must ensure only one can consume it.

Use a transactional uniqueness/locking strategy:

```text
BEGIN
  |
  v
load refresh credential FOR UPDATE
  |
  v
check unconsumed + valid
  |
  v
mark consumed
  |
  v
create successor
  |
  v
COMMIT
```

A database constraint remains the final defense against duplicate consumption.

---

# 13. JWT and Token Validation

If Sitolo consumes JWT access tokens, validation must be explicit.

Required validation:

```text
signature
issuer
audience
expiration
not-before where applicable
allowed algorithm
key identifier
key trust source
intended token type/use
```

Unsafe implementation:

```text
decode(payload)
trust role
trust tenant
trust user
```

Safe implementation:

```text
retrieve trusted metadata
        |
        v
validate signature + algorithm
        |
        v
validate issuer
        |
        v
validate audience
        |
        v
validate time claims
        |
        v
validate token use
        |
        v
normalize subject
        |
        v
resolve current application authority
```

A token claim such as `role=OWNER` can inform context but cannot replace current membership authorization.

---

# 14. JWKS and Key Rotation

Key rotation must not require application redesign or an outage.

Target behavior:

```text
trusted issuer metadata
        |
        v
JWKS retrieval
        |
        v
validated key cache
        |
        +--> current key
        +--> overlap/rotation key
        +--> retired key handling
```

## 14.1 Cache policy

The JWKS cache requires:

- bounded TTL;
- cache freshness policy;
- safe refresh on unknown `kid`;
- protection against refresh storms;
- negative caching where appropriate;
- validation that returned keys belong to the trusted issuer;
- failure semantics that do not silently accept unknown signing keys.

## 14.2 Key rotation race

During rotation, a token may use a newly published key before the application cache refreshes.

The implementation may perform a bounded metadata refresh once for the unknown key ID, but it must not accept arbitrary keys supplied by the client.

Repeated unknown-key failures become observable security events.

---

# 15. Identity Assertions and Principal Normalization

Authentication adapters must produce a typed internal representation.

Conceptual Rust model:

```rust
pub struct AuthenticatedPrincipal {
    pub subject_id: UserId,
    pub session_id: SessionId,
    pub device_id: Option<DeviceId>,
    pub authenticated_at: DateTime<Utc>,
    pub assurance: AuthenticationAssurance,
    pub security_version: SecurityVersion,
}
```

This structure MUST NOT become an untyped claim bag.

It should contain only facts required downstream.

Tenant membership and permissions remain outside the basic principal because they are current application authority, not merely identity-provider facts.

---

# 16. Authentication Assurance

Sitolo uses the existing assurance model:

```text
A0 — anonymous/public
A1 — ordinary authenticated session
A2 — strong administrative authentication
A3 — step-up authentication for high-risk operation
A4 — emergency/break-glass assurance
```

Examples:

| Operation | Minimum assurance |
|---|---:|
| Public documentation | A0 |
| Catalogue read | A1 |
| Ordinary sale | A1 |
| Password change | A1 or higher depending on risk |
| Security policy change | A2/A3 |
| MFA reset for another user | A3 |
| Payout destination change | A3 |
| Ownership transfer | A3 |
| Break-glass | A4 |

These are authorization/transaction-policy dependencies. Phase 3 provides the assurance mechanism; later phases consume it.

---

# 17. MFA Architecture

MFA is not merely a UI field called `mfa_enabled`.

The minimum internal model must distinguish:

```text
MFA enrollment state
MFA authenticator identity
MFA challenge
MFA verification result
MFA recovery
MFA reset
MFA security version
```

## 17.1 MFA factors

Potential supported factors:

- TOTP;
- passkeys/WebAuthn;
- provider-managed strong authenticators;
- recovery codes as recovery artifacts.

SMS/phone OTP may be operationally useful in the Malawi market, but it should not automatically be classified as phishing-resistant strong authentication. The existing identity contract already treats phone possession more cautiously than higher-assurance authenticators.

## 17.2 MFA state machine

```text
UNENROLLED
    |
    v
ENROLLMENT_STARTED
    |
    +--> EXPIRED
    |
    v
VERIFICATION_REQUIRED
    |
    +--> VERIFY_FAILED
    |        |
    |        +--> LOCK / RATE LIMIT
    |
    v
ACTIVE
    |
    +--> REVOKED
    +--> REPLACEMENT_REQUIRED
```

The exact state transition rules must prevent duplicate enrollment and race-based authenticator replacement.

---

# 18. TOTP

If TOTP is enabled:

- the secret is generated server-side or through a trusted enrollment component;
- the secret is never logged;
- the secret is encrypted/protected at rest;
- provisioning data is shown only during controlled enrollment;
- enrollment is not considered active until verification succeeds;
- failed attempts are rate-limited;
- replay handling follows the selected authenticator design;
- time windows are explicit;
- clock tolerance is bounded.

The server must never store an unprotected TOTP seed merely because it is convenient.

## 18.1 Enrollment rule

```text
MFA enrollment requested
        |
        v
step-up / authenticated enrollment
        |
        v
secret generated
        |
        v
provisioning artifact shown once
        |
        v
user submits verification code
        |
        v
atomic activation
        |
        v
security event
```

An enrollment object that has not completed verification must not grant MFA assurance.

---

# 19. Passkeys / WebAuthn

Passkeys/WebAuthn are the preferred high-assurance direction for platform administrators and can be progressively offered to merchant administrators.

The implementation must delegate WebAuthn protocol processing to a maintained implementation.

Sitolo owns:

- account binding;
- authenticator lifecycle;
- authorization policy;
- assurance mapping;
- audit events;
- step-up semantics.

The WebAuthn layer owns:

- challenge construction/validation;
- origin validation;
- RP ID validation;
- credential public-key handling;
- authenticator data parsing;
- protocol-level signature verification.

## 19.1 Challenge requirements

Challenges must be:

- cryptographically unpredictable;
- single-purpose;
- short-lived;
- session-bound where appropriate;
- invalidated after use.

A challenge must not be accepted twice.

---

# 20. Recovery Codes

Recovery codes are authenticators, not UI decoration.

Requirements:

```text
high entropy
single use
short/appropriate recovery scope
protected at rest
never logged
never returned after redemption
regenerated under controlled policy
```

Store a protected verifier/hash representation where practical rather than plaintext reusable codes.

The user-visible recovery-code display is a one-time event.

The server must not provide an endpoint such as:

```text
GET /my-recovery-codes
```

that simply returns existing secrets.

---

# 21. Password Reset

Password recovery is an authentication path and must be protected like login.

Target flow:

```text
REQUEST RESET
     |
     v
non-enumerating response
     |
     v
short-lived reset artifact
     |
     v
one-time atomic redemption
     |
     v
password policy validation
     |
     v
password verifier update
     |
     +--> session invalidation
     +--> refresh-family invalidation
     +--> audit
     +--> notification
```

## 21.1 Reset artifact

The reset token must be:

- cryptographically random;
- single-use;
- short-lived;
- bound to the intended account/context;
- stored as a protected/hash representation where persisted;
- invalidated after use;
- unusable after account security-version change.

## 21.2 Reset race

Two concurrent reset attempts must not both succeed with the same artifact.

The database operation must atomically claim the reset record before applying the credential transition.

---

# 22. Account Enumeration Resistance

Public authentication flows should not reveal whether a phone number, email address or other login identifier exists.

Example:

```text
Known account      -> generic response
Unknown account    -> generic response
```

The implementation may internally distinguish the outcomes for telemetry and abuse controls.

Attackers must not receive:

- different status codes solely based on account existence;
- different public error text;
- obviously different response timing where practical mitigation is justified;
- different recovery behavior visible to the caller.

Administrative support tooling can have explicit disclosure permissions inside an authenticated support boundary.

---

# 23. Rate Limiting and Abuse Controls

Authentication endpoints are high-value CPU and credential-verification resources.

Every externally reachable authentication path must have an abuse-control classification.

Examples:

```text
login
password reset request
password reset redemption
MFA verify
TOTP verify
passkey challenge completion
refresh
session creation
```

Controls may include:

- per-account subject rate limits;
- per-IP/source-class rate limits;
- device-level throttling;
- progressive backoff;
- CAPTCHA/challenge integration where required;
- temporary account/authenticator lockouts;
- anomaly detection.

Do not implement naive permanent lockouts that allow attackers to trivially deny service to merchants by repeatedly failing someone else's login.

Use layered controls.

---

# 24. Device Identity

A device is an independently revocable security subject.

Conceptual model:

```text
Device
├── device_id
├── installation_id
├── organization_id?
├── registered_by
├── platform
├── app_version
├── os_version
├── device_key_reference?
├── state
├── security_version
├── registered_at
├── last_seen_at
├── revoked_at?
└── replacement_of?
```

The exact hardware identifiers exposed by the OS must be selected conservatively. Permanent vendor/device identifiers are not automatically reliable security credentials.

## 24.1 Device lifecycle

```text
UNREGISTERED
    |
    v
REGISTRATION_STARTED
    |
    v
VERIFICATION_REQUIRED
    |
    v
REGISTERED
    |
    v
ACTIVE
  /   \
 v     v
SUSPENDED  REVOKED
              |
              v
         REPLACED / RETIRED
```

Only `ACTIVE` devices may participate in operations that require a current device context.

---

# 25. Device Registration

Device registration must occur through an authenticated enrollment flow.

It cannot be:

```text
POST /devices
{
  "organization_id": "attacker-chosen",
  "device_id": "random-string"
}
```

A secure registration flow establishes:

```text
authenticated user/session
        |
        v
organization context from trusted authority
        |
        v
registration challenge
        |
        v
device proof / OS-bound mechanism where supported
        |
        v
device record
        |
        v
audit event
```

The device receives an identifier after the server creates the authoritative record.

## 25.1 Multiple users per device

A POS device may have multiple user sessions over time.

Therefore:

```text
Device 1
  +--> Session A / User A
  +--> Session B / User B
```

does not imply User A owns all authority of Device 1 forever.

The current session principal remains the user-level security subject.

---

# 26. Device Credentials

If device-bound cryptographic credentials are introduced, they must be generated and protected through platform secure facilities where practical.

The backend should store:

- public verification material;
- credential identifier;
- device state;
- security-version/reference metadata.

It should not require exporting an OS-protected private key into ordinary application storage.

Do not treat an Android application UUID or randomly generated local ID as proof that the device is genuine.

---

# 27. Device Revocation

Revocation is authoritative server state.

```text
ACTIVE DEVICE
     |
     v
REVOCATION REQUEST
     |
     v
AUTHORIZE REQUEST
     |
     v
ATOMIC DEVICE STATE UPDATE
     |
     +--> increment device security version
     +--> revoke device sessions according to policy
     +--> invalidate offline capability
     +--> audit
     +--> telemetry
```

After revocation:

- normal authenticated API requests using the device must fail according to policy;
- synchronization must reject future normal device commands;
- no new offline capability may be granted;
- already queued offline commands remain evidence and are handled according to the sync protocol rather than silently deleted.

The existing sync and auth specifications explicitly require a revoked device to be rejected upon reconnection while preserving evidence of rejected commands.

---

# 28. Device Replacement

Replacement is not an update of the old device record.

Bad:

```text
UPDATE device SET installation_id = new_value
```

Preferred:

```text
old device
   |
   v
REVOKED / RETIRED
   |
   +--> replacement_of = old device
   |
   v
new device
   |
   v
registration lifecycle
```

This preserves historical evidence.

---

# 29. Device Theft and Loss

The lost-device workflow must be optimized for rapid containment.

```text
merchant reports lost phone
          |
          v
authenticate reporting actor
          |
          v
authorize device-revoke action
          |
          v
revoke device
          |
          +--> sessions
          +--> offline capability
          +--> future sync
          +--> device credential
          |
          v
audit + notification
```

The recovery UI must never require the lost device itself to complete its own revocation.

---

# 30. Offline Authentication Boundary

Offline support is a product requirement but must not create a permanent credential bypass.

```text
ONLINE AUTHORITY
     |
     v
bounded offline capability
     |
     +--> allowed operations
     +--> allowed scope
     +--> expiration/window
     +--> device binding
     +--> user/session constraints
```

The client may cache enough data for operational continuity, but it cannot mint new authority locally.

Offline state must not enable:

- role assignment;
- ownership transfer;
- security-policy changes;
- unrestricted payout changes;
- MFA reset for another user;
- unrestricted administrative operations.

The sync protocol remains the authoritative mechanism for reconciling offline commands.

---

# 31. Security Versioning

Security versions provide efficient invalidation of stale authority.

Potential versions:

```text
user.security_version
session.security_version
membership.security_version
server_policy_version
device.security_version
credential_family_version
```

Phase 3 owns the identity/session/device versions. Phase 4/6 can extend the model for membership and authorization.

A version mismatch must trigger denial or controlled reauthentication according to policy.

---

# 32. Authentication Context Construction

The API middleware pipeline must remain consistent with the existing Phase 0 contract:

```text
HTTP request
   |
   v
request bounds
   |
   v
auth credential extraction
   |
   v
cryptographic/token validation
   |
   v
session resolution
   |
   v
device resolution
   |
   v
principal construction
   |
   v
downstream authorization
```

Do not allow individual route handlers to reimplement token parsing or session lookup.

## 32.1 SecurityContext

Conceptual application context:

```rust
pub struct SecurityContext {
    pub principal: AuthenticatedPrincipal,
    pub session: SessionSecurityState,
    pub device: Option<DeviceSecurityState>,
    pub assurance: AuthenticationAssurance,
    pub request_id: RequestId,
    pub trace_id: TraceId,
}
```

`SecurityContext` must not contain raw bearer tokens or raw secrets after the credential-extraction boundary.

---

# 33. Rust Module Architecture

Phase 3 should fit the Phase 1 workspace.

Recommended ownership:

```text
crates/
├── sitolo-auth/
│   ├── principal
│   ├── token validation boundary
│   ├── identity provider ports
│   ├── password verification boundary
│   └── authentication policy
│
├── sitolo-session/
│   ├── session model
│   ├── lifecycle
│   ├── refresh rotation
│   └── revocation
│
├── sitolo-device/
│   ├── device model
│   ├── registration
│   ├── lifecycle
│   ├── revocation
│   └── security version
│
├── sitolo-mfa/
│   ├── enrollment
│   ├── challenge
│   ├── verification
│   ├── recovery codes
│   └── assurance mapping
│
├── sitolo-security/
│   ├── security context
│   ├── redaction
│   ├── security errors
│   └── events
│
├── sitolo-persistence/
│   └── identity/session/device repositories
│
└── sitolo-api/
    └── transport routes/middleware
```

The exact crate names must follow the actual repository layout established in Phase 1 rather than being blindly copied.

## 33.1 Dependency direction

```text
API
 |
 v
APPLICATION
 |
 +--> AUTH
 +--> SESSION
 +--> DEVICE
 +--> MFA
 |
 v
DOMAIN
 |
 v
PORTS
 |
 v
PERSISTENCE / PROVIDER ADAPTERS
```

Domain code must not depend directly on Axum, SQLx, HTTP provider clients or Flutter.

---

# 34. Persistence Model

The identity schema must reflect lifecycle semantics.

Minimum logical records:

```text
users
sessions
refresh_token_families
refresh_tokens
mfa_authenticators
mfa_challenges
mfa_recovery_codes
password_reset_requests
devices
device_credentials
authentication_events
```

Depending on the selected identity-provider architecture, some identity records may belong to the external provider and only a normalized local reference may exist.

## 34.1 Database invariants

Examples:

```text
session.id unique
refresh_token.id unique
refresh token family membership consistent
active device identity unique where required
single-use recovery artifact cannot be consumed twice
expired/revoked artifact cannot return to ACTIVE
security version monotonically changes
```

## 34.2 No secret persistence by accident

Database entities must not accidentally serialize:

- access tokens;
- refresh tokens in plaintext unless unavoidable and explicitly protected;
- password plaintext;
- OTP values;
- TOTP secrets in ordinary audit rows;
- recovery codes plaintext;
- provider private keys.

The repository layer should use dedicated secret types and explicit protected storage paths.

---

# 35. Secret Types in Rust

Secrets should have types that make accidental logging harder.

A maintained crate such as `secrecy` provides `SecretBox`/`SecretString`, explicit secret exposure traits and integration with memory wiping through `zeroize`. citeturn833127search2turn833127search5

The use of such types does **not** magically make secrets safe. It reduces accidental handling mistakes.

The design should use semantic wrappers such as:

```text
PasswordMaterial
AccessCredential
RefreshCredential
MfaSecret
RecoveryCode
ProviderSecret
```

not generic `String` everywhere.

`zeroize` is useful for clearing sensitive memory and currently documents a portable implementation intended to resist compiler optimization removing the clearing operation. citeturn833127search10

Memory zeroing remains defense in depth; process dumps, swaps, snapshots and OS-level compromise are separate threats.

---

# 36. Authentication Error Taxonomy

Phase 3 consumes the existing structured error model.

Authentication-specific semantic errors include:

```text
AuthenticationFailed
AuthenticationRateLimited
SessionInvalid
SessionExpired
SessionRevoked
RefreshTokenInvalid
RefreshTokenReused
MfaRequired
MfaFailed
MfaRateLimited
MfaEnrollmentExpired
RecoveryRequired
RecoveryArtifactInvalid
RecoveryArtifactUsed
DeviceNotRegistered
DeviceSuspended
DeviceRevoked
DeviceVerificationRequired
IdentityProviderUnavailable
IdentityProviderRejected
IdentityKeyUnavailable
ConfigurationInvalid
```

Transport mapping must never expose internal provider details.

## 36.1 Public error examples

```json
{
  "type": "https://api.sitolo.example/problems/authentication-failed",
  "title": "Authentication failed",
  "status": 401,
  "code": "AUTHENTICATION_FAILED",
  "request_id": "req_01..."
}
```

Refresh replay may be mapped to a generic authentication failure externally while internal telemetry records `REFRESH_REUSE_DETECTED`.

---

# 37. Audit Events

Authentication is security-sensitive and must emit structured evidence.

Minimum event set:

```text
LOGIN_SUCCESS
LOGIN_FAILURE
LOGIN_RATE_LIMITED
SESSION_CREATED
SESSION_REFRESHED
SESSION_REVOKED
SESSION_EXPIRED
LOGOUT
REFRESH_ROTATED
REFRESH_REUSE_DETECTED
PASSWORD_CHANGED
PASSWORD_RESET_REQUESTED
PASSWORD_RESET_COMPLETED
MFA_ENROLLMENT_STARTED
MFA_ENROLLMENT_COMPLETED
MFA_ENROLLMENT_REJECTED
MFA_SUCCESS
MFA_FAILURE
MFA_RESET
RECOVERY_CODE_REDEEMED
DEVICE_REGISTRATION_STARTED
DEVICE_REGISTERED
DEVICE_SUSPENDED
DEVICE_REVOKED
DEVICE_REPLACED
DEVICE_VERIFICATION_FAILED
JWKS_REFRESH_FAILED
TOKEN_VALIDATION_FAILED
```

Events must include useful evidence without secrets.

## 37.1 Safe authentication event fields

Potential fields:

```text
event_name
event_id
timestamp
request_id
trace_id
subject_id_pseudonymous
session_id_pseudonymous
device_id_pseudonymous
client_platform
authentication_method
assurance_level
result
reason_class
ip_class / network classification where justified
application_version
security_version
```

Never include:

```text
password
otp
mfa secret
refresh token
access token
authorization code
client secret
private key
recovery code
```

OWASP explicitly advises removing or protecting passwords, access tokens, session IDs, encryption keys, database connection strings and sensitive personal data from logs. citeturn833127search1

---

# 38. Telemetry

Phase 3 must integrate with the Phase 2 telemetry model.

Baseline metrics include:

```text
sitolo_authentication_attempts_total
sitolo_authentication_failures_total
sitolo_authentication_rate_limited_total
sitolo_mfa_attempts_total
sitolo_mfa_failures_total
sitolo_session_created_total
sitolo_session_revocations_total
sitolo_session_expirations_total
sitolo_refresh_total
sitolo_refresh_reuse_detected_total
sitolo_password_resets_total
sitolo_password_reset_failures_total
sitolo_device_registrations_total
sitolo_device_revocations_total
sitolo_device_verification_failures_total
sitolo_jwks_refresh_failures_total
```

Allowed labels must remain bounded:

```text
method
result
reason_class
client_platform
assurance_level
```

Never use:

```text
email
phone_number
user_id
session_id
trace_id
refresh_token
raw_error
```

The existing observability contract already prohibits high-cardinality identifiers in metric labels.

---

# 39. Tracing

Rust tracing should model security-relevant operations as spans:

```text
auth.request
  |
  +--> token.validate
  +--> session.resolve
  +--> device.resolve
  +--> mfa.verify
  +--> session.persist
```

Use OpenTelemetry correlation without allowing trace context to become security authority.

The current OpenTelemetry Rust documentation recommends integration with `tracing` and an OpenTelemetry bridge for logs, while current Rust support status is documented as beta for traces, metrics and logs. citeturn833127search0turn833127search3

Trace IDs and request IDs are diagnostic only.

---

# 40. Sensitive Logging Controls

Phase 2 provides centralized redaction. Phase 3 must add authentication-specific redaction fixtures.

The CI suite must prove that these never appear in emitted events:

```text
Bearer ey...
refresh_token=...
password=...
otp=123456
authorization_code=...
client_secret=...
totp_secret=...
recovery_code=...
```

Test both:

1. explicit structured fields;
2. error/debug formatting.

For example, this is dangerous:

```rust
tracing::debug!(?request, "authentication request");
```

when `request` can later gain a password/token field.

Prefer explicit safe fields:

```rust
tracing::debug!(
    method = %auth_method,
    platform = %platform,
    "authentication attempt"
);
```

The difference is architectural, not stylistic.

---

# 41. Concurrent Security Operations

Identity code is race-sensitive.

At minimum test:

- simultaneous refresh requests;
- simultaneous logout and refresh;
- simultaneous MFA enrollment completion;
- simultaneous MFA reset;
- simultaneous password reset redemption;
- simultaneous device revocation and request;
- simultaneous device registration attempts;
- simultaneous security-version changes.

Critical operations should follow:

```text
BEGIN
  |
  v
load authoritative record with correct lock
  |
  v
verify state/version
  |
  v
apply one legal transition
  |
  v
write audit/event
  |
  v
COMMIT
```

No external network call should be held open inside a long database transaction.

---

# 42. Idempotency

Identity endpoints can also be retried because mobile networks are unreliable.

Examples:

```text
logout
MFA enrollment completion
recovery-code redemption
password-reset redemption
device registration finalization
```

The implementation must distinguish:

```text
same request repeated safely
vs
same artifact reused maliciously
```

For example, a repeated API retry after a successful idempotent registration may return the previously committed result, while reuse of a single-use MFA or recovery artifact must fail and generate a security event.

---

# 43. Failure Modes

## 43.1 Identity provider unavailable

For login:

```text
IDP unavailable
   |
   v
safe authentication failure
```

Do not accept unverifiable identity assertions merely to improve availability.

For already authenticated sessions, previously issued session validity may continue until its configured limits, depending on the selected architecture.

## 43.2 JWKS unavailable

If a valid cached key remains within its safe freshness policy, the verifier may continue according to policy.

If verification cannot be performed safely:

```text
cannot validate identity
        |
        v
DENY
```

Never treat JWKS failure as permission to trust unsigned/unknown keys.

## 43.3 Database unavailable

Session creation/revocation that requires authoritative state must fail closed.

Do not issue new long-lived credentials when the server cannot persist their authoritative lifecycle.

## 43.4 Telemetry unavailable

Authentication should generally remain operational while local/alternate diagnostics continue, provided required audit semantics remain intact.

The absence of a metrics collector is not permission to skip an audit record that is required by the security contract.

---

# 44. Security Notifications

Material identity security events should support user notifications where operationally appropriate:

- new device registered;
- password changed;
- password reset completed;
- MFA enabled/disabled;
- suspicious refresh-token reuse;
- device revoked;
- administrative credential changes.

Notifications are signals, not authorization.

A notification failure must not roll back a correctly committed credential security transition unless the specific business policy explicitly requires transactional delivery semantics.

Use the existing transactional outbox architecture for asynchronous notification delivery.

---

# 45. Administrative Security Operations

Sensitive identity operations require stronger controls.

Examples:

```text
force logout user
revoke device
reset MFA
change authentication policy
unlock account
issue recovery override
```

The implementation must capture:

```text
real actor
reason
scope
target subject
authorization decision
assurance level
time
outcome
```

Support operators must never silently impersonate a user. The system must retain the distinction between the real actor and the effective subject.

---

# 46. API Surface

The exact paths remain subject to the API contract, but the identity capability must cover operations equivalent to:

```text
POST /v1/auth/login                 [provider-dependent]
POST /v1/auth/logout
POST /v1/auth/refresh
GET  /v1/auth/session
POST /v1/auth/password/change
POST /v1/auth/password/reset/request
POST /v1/auth/password/reset/confirm
POST /v1/auth/mfa/enroll
POST /v1/auth/mfa/verify
POST /v1/auth/mfa/reset                 [privileged]
GET  /v1/me/sessions
POST /v1/me/sessions/{id}/revoke
GET  /v1/me/devices
POST /v1/me/devices/register
POST /v1/me/devices/{id}/revoke
POST /v1/me/devices/{id}/replace
```

These are logical capabilities, not authorization-free route guarantees.

Every route must still have:

```text
authentication policy
authorization policy
resource scope
input schema
rate class
timeout
idempotency behavior
audit requirement
logging policy
```

---

# 47. API Resource Limits

Authentication endpoints are denial-of-service targets.

Every request must be bounded before expensive work.

Examples:

- maximum body size;
- maximum identifier length;
- maximum password length;
- maximum OTP length;
- maximum recovery code length;
- maximum callback parameter length;
- strict redirect URI validation;
- strict JSON nesting limits;
- maximum concurrency for password verification;
- bounded provider request timeouts.

Do not attempt password hashing before cheap structural validation rejects obviously invalid requests.

However, account-enumeration defenses must be considered when creating dramatically different processing paths.

---

# 48. CPU and I/O Design

Authentication is both CPU- and I/O-sensitive.

### CPU-bound work

- Argon2id password verification;
- password hashing;
- cryptographic signature verification;
- WebAuthn verification;
- some JWT operations.

### I/O-bound work

- identity-provider metadata retrieval;
- JWKS refresh;
- session persistence;
- notification enqueue;
- audit persistence.

Do not accidentally block Tokio worker threads with expensive password hashing if the selected Rust implementation is synchronous and computationally significant.

Use a bounded blocking execution strategy where appropriate.

The goal is:

```text
network / DB wait
    -> async

expensive password KDF
    -> bounded CPU/blocking capacity
```

An unlimited blocking pool is not a scalability strategy; it is an eventual resource-exhaustion path.

---

# 49. Memory Behaviour

Security credentials have a lifecycle in memory.

The implementation must minimize:

- unnecessary cloning;
- repeated UTF-8 conversion;
- logging copies;
- long-lived secret references;
- retention in error objects.

Do not retain access/refresh credentials in global caches.

Use typed secret wrappers and carefully scoped lifetimes.

Memory wiping is defense in depth, not a guarantee against all process-memory exposure.

---

# 50. Authentication Middleware Ordering

The existing API contract establishes the broader sequence. Phase 3 must implement its portion in the following order:

```text
HTTP request
   |
   v
request size / parser limits
   |
   v
rate / concurrency controls
   |
   v
credential extraction
   |
   v
cryptographic validation
   |
   v
session state
   |
   v
device state
   |
   v
principal construction
   |
   v
next-stage tenant/authz
```

The following are explicitly wrong:

```text
route handler parses JWT manually
```

```text
tenant middleware trusts a JWT role claim
```

```text
device middleware trusts x-device-id as authentication
```

```text
authentication middleware logs Authorization headers
```

---

# 51. Repository Contracts

Repositories must expose semantic operations rather than generic database mutation.

Good:

```text
create_session()
revoke_session()
consume_refresh_token()
create_refresh_successor()
register_device()
revoke_device()
start_mfa_enrollment()
complete_mfa_enrollment()
consume_recovery_code()
redeem_password_reset()
```

Bad:

```text
update_users(fields)
update_sessions(map)
set_security_state(json)
```

Generic persistence methods make forbidden state transitions too easy to express.

---

# 52. Transaction Boundaries

## 52.1 Session creation

One transaction should establish the durable application session and required audit/outbox records atomically.

```text
BEGIN
  |
  +--> validate identity state
  +--> validate device state
  +--> create session
  +--> persist security event/audit record
  +--> create notification/outbox intent where required
  |
COMMIT
```

## 52.2 Refresh

```text
BEGIN
  |
  +--> lock refresh credential
  +--> validate lifecycle
  +--> mark consumed
  +--> create successor
  +--> update session metadata
  +--> write security event
  |
COMMIT
```

## 52.3 Device revoke

```text
BEGIN
  |
  +--> authorize action
  +--> lock device record
  +--> mark revoked
  +--> increment security version
  +--> revoke dependent sessions according to policy
  +--> audit
  |
COMMIT
```

## 52.4 MFA reset

This is high risk and may require step-up authorization plus separation of duties.

The reset transition must atomically invalidate prior authenticators and dependent recovery state according to policy.

---

# 53. Database Security

The database account used by the API must not have unrestricted administrative rights.

Separate roles should be used for:

```text
migration owner
runtime application
read-only reporting where needed
operational maintenance
```

Identity tables contain sensitive material and therefore require the same tenant/security controls established by `database_design.md`.

RLS remains defense in depth. Application authorization cannot be replaced by RLS alone, and RLS cannot be treated as a substitute for correct transaction/query design.

---

# 54. Test Architecture

Phase 3 security testing is mandatory.

Recommended structure:

```text
tests/security/
├── authentication/
├── sessions/
├── refresh/
├── mfa/
├── recovery/
├── devices/
├── enumeration/
├── rate_limits/
├── concurrency/
├── jwks/
└── secrets/
```

Use real PostgreSQL for database lifecycle tests where persistence semantics matter.

Mocking the database is insufficient for proving single-use artifact and refresh-token race correctness.

---

# 55. Mandatory Authentication Tests

At minimum:

```text
invalid password -> denied
unknown account -> safe generic result
valid password -> session created
password hash upgrade -> succeeds
expired token -> denied
wrong issuer -> denied
wrong audience -> denied
wrong algorithm -> denied
unknown signing key -> denied or bounded refresh then denied
revoked session -> denied
expired session -> denied
logout -> session revoked
refresh -> token rotated
refresh replay -> family containment
MFA required -> challenge required
wrong MFA code -> denied
expired MFA code -> denied
MFA replay -> denied
recovery code reuse -> denied
reset token reuse -> denied
password reset -> sessions invalidated
revoked device -> denied
reconnect revoked device -> sync denied
concurrent refresh -> one success
concurrent reset -> one valid redemption
concurrent device revoke -> deterministic final state
```

---

# 56. Property-Based Testing

Property tests should cover invariants such as:

### Refresh rotation

```text
for all refresh credentials:
    consumed credential cannot become active again
```

### Recovery

```text
for all recovery artifacts:
    successful redemption count <= 1
```

### Device lifecycle

```text
REVOKED -> never directly becomes ACTIVE
```

### Security version

```text
security_version never decreases
```

### Session expiration

```text
expired session cannot be accepted by normal authentication context creation
```

### Callback state

```text
one authorization state cannot complete two transactions
```

These are stronger than a handful of hand-written examples.

---

# 57. Fuzz Targets

Authentication fuzzing should cover:

- JWT parsing;
- malformed JOSE headers;
- oversized token inputs;
- OIDC discovery metadata;
- JWKS documents;
- OAuth callback parameters;
- redirect URI parsing;
- password-reset artifacts;
- malformed TOTP input;
- recovery codes;
- device registration payloads;
- authentication request JSON;
- error serialization.

The goal is not just crash resistance. It is also deterministic rejection and resource bounding.

---

# 58. Browser Security Tests

For browser/admin sessions test:

- Secure cookie attribute;
- HttpOnly cookie attribute;
- intended SameSite policy;
- CSRF protection;
- session rotation;
- logout invalidation;
- cache-control where sensitive;
- absence of long-lived tokens in localStorage;
- absence of credentials in URLs;
- safe error rendering.

---

# 59. Mobile Security Tests

For Flutter:

- state mismatch callback;
- PKCE mismatch;
- duplicate callback;
- malicious callback parameters;
- secure storage failure;
- local credential clearing on logout;
- token expiration;
- revoked device;
- application reinstall behavior;
- device replacement;
- stale tenant/session cache clearing.

The mobile app must not assume that because a credential exists locally, it remains authoritative.

---

# 60. Abuse Cases

Phase 3 threat-led tests must model attacks rather than only normal usage.

## Attack: stolen refresh token

Expected:

```text
replay detected
-> token family containment
-> session invalidation
-> security event
```

## Attack: stolen phone

Expected:

```text
device revoke
-> future requests denied
-> future sync denied
```

## Attack: MFA brute force

Expected:

```text
rate limit
+ bounded attempts
+ telemetry
```

## Attack: account enumeration

Expected:

```text
external responses remain non-enumerating
```

## Attack: JWT algorithm confusion

Expected:

```text
algorithm outside allowlist -> reject
```

## Attack: fake JWKS

Expected:

```text
untrusted issuer/key source -> reject
```

## Attack: forged device ID

Expected:

```text
device id alone -> insufficient
```

---

# 61. Operational Runbooks

## 61.1 Suspected refresh-token theft

```text
1. Identify the refresh-reuse event.
2. Identify affected credential family.
3. Revoke family/session as policy requires.
4. Confirm device state.
5. Preserve audit/security evidence.
6. Notify user where applicable.
7. Review surrounding authentication events.
8. Add regression coverage when root cause is implementation-related.
```

## 61.2 Lost device

```text
1. Authenticate reporting actor.
2. Authorize device revocation.
3. Revoke device atomically.
4. Revoke dependent sessions according to policy.
5. Invalidate offline capability.
6. Confirm future sync rejection.
7. Preserve evidence.
```

## 61.3 Identity-provider outage

```text
1. Confirm external dependency health.
2. Stop treating unverifiable assertions as valid.
3. Monitor impact on new authentication.
4. Preserve active sessions only according to their valid local lifecycle.
5. Do not issue unauthorized fallback credentials.
6. Escalate according to dependency runbook.
```

## 61.4 MFA reset fraud

```text
1. Treat reset request as high risk.
2. Validate requester assurance.
3. Check recovery evidence.
4. Apply approval policy if required.
5. Reset authenticators atomically.
6. Invalidate sessions where policy requires.
7. Notify affected account.
8. Audit the complete operation.
```

---

# 62. Logging and Privacy Rules

Authentication telemetry must minimize personal information.

Use pseudonymous identifiers when exact identity is not required for diagnosis.

Never place full phone numbers or email addresses into high-volume logs by default.

When a support investigation requires identity resolution, privileged tooling should resolve the pseudonymous reference under access control rather than forcing every telemetry record to contain the person's raw identifier.

OWASP specifically recommends removing, masking, sanitizing, hashing or encrypting sensitive session, authentication and personal data in logs. citeturn833127search1

---

# 63. Configuration Contract

Phase 2 owns configuration loading. Phase 3 defines authentication-specific keys.

Logical categories include:

```text
IDENTITY_PROVIDER
OIDC_ISSUER
OIDC_AUDIENCE
OIDC_CLIENT_ID
OIDC_REDIRECT_URI
OIDC_JWKS_URL

SESSION
SESSION_IDLE_TIMEOUT
SESSION_ABSOLUTE_TIMEOUT
SESSION_ROTATION_POLICY
SESSION_REVOCATION_POLICY

REFRESH
REFRESH_TOKEN_LIFETIME
REFRESH_ROTATION_ENABLED
REFRESH_REUSE_RESPONSE

PASSWORD
PASSWORD_HASH_POLICY_VERSION
PASSWORD_MIN_LENGTH
PASSWORD_MAX_LENGTH
PASSWORD_REHASH_ENABLED

MFA
MFA_REQUIRED_FOR_ADMIN
MFA_CHALLENGE_TTL
MFA_MAX_ATTEMPTS
TOTP_WINDOW
RECOVERY_CODE_COUNT

DEVICE
DEVICE_ENROLLMENT_TTL
DEVICE_MAX_PER_USER
DEVICE_SESSION_BINDING_POLICY
DEVICE_REVOCATION_POLICY

ABUSE
LOGIN_RATE_LIMIT
MFA_RATE_LIMIT
RESET_RATE_LIMIT
REFRESH_RATE_LIMIT
```

These are policy examples, not hardcoded production values.

Secrets such as provider client secrets, encryption keys and private signing keys must use the secret-management contract from Phase 2.

---

# 64. Configuration Validation

Startup must fail if an impossible security configuration is detected.

Examples:

```text
session absolute timeout < session idle timeout relationship invalid
MFA max attempts <= 0
JWT issuer missing in JWT validation mode
JWKS URL absent when remote JWKS is required
production environment uses insecure callback URI
admin MFA requirement disabled contrary to release policy
refresh rotation enabled but token persistence schema unavailable
```

These are configuration errors, not runtime business errors.

Do not silently replace an invalid security policy with a permissive default.

---

# 65. Provider Adapter Interface

The authentication boundary should expose application-level methods.

Conceptually:

```text
IdentityProvider
├── begin_authorization()
├── exchange_code()
├── validate_id_token()
├── fetch_user_identity()
└── refresh_token() [provider dependent]
```

The provider adapter should return normalized values:

```text
ExternalIdentity
├── subject
├── verified contact attributes where applicable
├── authentication method
├── assurance hints
└── provider metadata required by policy
```

Do not leak vendor response DTOs into `sitolo-domain`.

---

# 66. Provider Timeouts and Resilience

All identity-provider network calls require:

- connection timeout;
- request deadline;
- response size bound;
- retry classification;
- bounded retry count;
- no automatic retry for non-idempotent operations unless the provider contract makes it safe;
- observability;
- circuit/backpressure strategy where needed.

A provider outage must not cause unbounded Tokio tasks.

---

# 67. JWT/JWKS Cache Stampede Prevention

A common failure mode is many concurrent requests encountering an unknown key and all refreshing JWKS simultaneously.

Bad:

```text
1,000 requests
  -> 1,000 JWKS fetches
```

Preferred:

```text
1,000 requests
       |
       v
single bounded refresh
       |
       v
shared validated cache
```

Use synchronization around refresh while keeping waiters bounded and deadlines enforced.

---

# 68. Authentication Session Cache

Caching session validity is optional.

If implemented:

```text
cache key = session_id + security_version
```

The cache must have:

- bounded TTL;
- invalidation/version checks;
- no fail-open on cache outage for sensitive operations;
- no storage of raw access credentials.

A cache is an optimization, never the source of truth.

---

# 69. Authorization Handoff

Phase 3 must hand a stable principal/session/device context to Phase 4/6.

Expected interface:

```text
Phase 3
  |
  v
SecurityContext
  |
  +--> principal identity
  +--> session validity
  +--> device validity
  +--> assurance
  +--> security version
  |
  v
Phase 4 tenant context
  |
  v
Phase 6 authorization
```

Phase 3 must not prematurely implement arbitrary domain permissions.

This keeps the architecture modular and prevents authentication code from becoming a second authorization engine.

---

# 70. Security Properties of Device Binding

Device binding may strengthen containment, but it introduces risks if over-trusted.

Advantages:

- independent revocation;
- improved anomaly detection;
- better offline containment;
- explicit device inventory;
- reduced credential portability.

Disadvantages:

- device replacement workflows become necessary;
- shared devices complicate binding;
- mobile hardware identifiers are imperfect;
- legitimate users can lose devices;
- key-storage APIs vary by platform.

Therefore device binding should be treated as a risk-control layer, not as a magic proof of legitimate ownership.

---

# 71. Why This Architecture Over Alternatives

## Alternative A — custom username/password/JWT stack

Rejected.

It creates unnecessary protocol, cryptographic, token and recovery risk. Sitolo's differentiator is merchant business correctness and offline operation, not inventing authentication standards.

## Alternative B — trust identity-provider role/tenant claims

Rejected.

Current application membership and business scope belong to Sitolo. External claims can become stale and cannot express all resource-level business authorization.

## Alternative C — device ID as credential

Rejected.

Device IDs are not proof of possession or identity and are vulnerable to extraction/spoofing.

## Alternative D — JWT-only stateless sessions with no revocation model

Rejected for sensitive Sitolo operations.

The product requires explicit session/device revocation, incident response and business security containment.

## Alternative E — database-backed session only for every client

Potentially viable for browser/BFF flows but unnecessarily constraining for native OAuth architecture. The exact hybrid architecture depends on the selected identity provider.

## Alternative F — SMS as the only MFA factor

Rejected as the highest-assurance default.

SMS can be operationally useful but does not provide the same phishing resistance as passkeys/security keys.

---

# 72. Advantages

The Phase 3 architecture provides:

1. explicit separation of identity, session and device lifecycle;
2. standards-based authentication instead of custom cryptography;
3. independently revocable sessions and devices;
4. stronger assurance for sensitive actions;
5. replay-aware refresh credentials;
6. controlled recovery;
7. safe mobile OAuth architecture;
8. clear failure semantics;
9. auditable security state changes;
10. strong foundations for tenant and authorization phases;
11. compatibility with offline operation without making offline state absolute authority;
12. testable security invariants.

---

# 73. Disadvantages

This architecture is more complex than a conventional CRUD login system.

Costs include:

- more database state;
- more state machines;
- more security tests;
- provider integration complexity;
- recovery workflows;
- device lifecycle support;
- key rotation handling;
- operational runbooks;
- higher engineering discipline requirements.

Those costs are intentional. A multi-tenant business platform that handles money, inventory, tax evidence and offline work cannot safely be built as a login page plus bearer token.

---

# 74. Security Control Mapping

Phase 3 contributes directly to the existing 48-control matrix:

| Control | Phase 3 contribution |
|---|---|
| 4 Weak authentication | standards-based authentication + assurance |
| 11 Logs leak secrets | authentication redaction tests |
| 12 Verbose production errors | auth error mapping |
| 15 Client-only security | server principal/session/device validation |
| 24 Broken password reset | secure reset state machine |
| 25 Weak sessions | server lifecycle, expiry, rotation, revocation |
| 26 JWT secrets | validation and key handling |
| 28 Rate limits | authentication abuse controls |
| 30 Default credentials | bootstrap restrictions |
| 37 No MFA | MFA policy and assurance |
| 38 Account enumeration | response equivalence |
| 40 Race conditions | refresh/reset/device/MFA concurrency tests |
| 45 Fail-open checks | identity failure semantics |
| 46 Missing timeouts | provider/session/auth endpoint deadlines |
| 47 Sensitive browser storage | browser session policy |
| 48 Insecure endpoints | identity endpoint inventory |

Phase 3 also strengthens cross-cutting controls 1, 3, 5, 6, 13, 14, 34, 35, 39, 41 and 43 through authentication-specific enforcement and evidence.

---

# 75. Definition of Phase 3 Ready

Before implementation starts, all must be defined:

```text
[X] identity subject model
[X] principal model
[X] authentication-provider boundary
[X] native OAuth flow
[X] browser session model
[X] password policy
[X] session lifecycle
[X] refresh lifecycle
[X] replay response
[X] MFA lifecycle
[X] recovery model
[X] assurance model
[X] device lifecycle
[X] device revocation
[X] security versions
[X] audit event catalogue
[X] metrics catalogue
[X] error mapping
[X] persistence ownership
[X] transaction boundaries
[X] failure semantics
[X] CPU/I/O handling
[X] security test requirements
[ ] exact identity provider selected
[ ] exact OTP provider selected
[ ] final production assurance policy approved
[ ] final client platform secure-storage implementation verified
```

Items intentionally marked unresolved must remain explicit until provider/infrastructure evidence exists.

---

# 76. Definition of Phase 3 Done

Phase 3 is not done because login succeeds.

It is done only when:

```text
[ ] login success works
[ ] login failure is safe
[ ] enumeration resistance tested
[ ] brute-force controls tested
[ ] provider errors normalized
[ ] JWT validation negative tests pass
[ ] JWKS rotation tested
[ ] session creation tested
[ ] idle expiry tested
[ ] absolute expiry tested
[ ] logout tested
[ ] global revocation tested
[ ] selective revocation tested
[ ] refresh rotation tested
[ ] refresh replay containment tested
[ ] password reset tested
[ ] password change invalidates correct sessions
[ ] MFA enrollment tested
[ ] MFA verification tested
[ ] MFA failure/rate limits tested
[ ] recovery codes tested
[ ] MFA reset tested
[ ] admin assurance policy tested
[ ] device registration tested
[ ] device replacement tested
[ ] device revocation tested
[ ] revoked-device API access rejected
[ ] revoked-device sync rejected
[ ] security-version invalidation tested
[ ] audit events persisted
[ ] secrets absent from logs
[ ] secrets absent from traces
[ ] secrets absent from errors
[ ] browser token-storage policy tested
[ ] native PKCE/state flow tested
[ ] callback replay rejected
[ ] database race tests pass
[ ] fuzz tests pass
[ ] telemetry remains bounded
[ ] failure-mode runbooks complete
[ ] CI gates enforce security tests
```

---

# 77. Release Blockers

A release MUST be blocked for any of the following:

```text
authentication bypass
JWT validation bypass
issuer/audience bypass
algorithm confusion acceptance
refresh-token replay accepted
MFA bypass
recovery bypass
session revocation bypass
device revocation bypass
unknown-signing-key acceptance without trust validation
password secret leakage
refresh credential logging
MFA secret leakage
account-enumeration regression
unbounded authentication workload
missing provider timeouts
fail-open authentication dependency behavior
security-event persistence silently skipped where required
```

A green unit-test suite does not override any of these blockers.

---

# 78. CI Enforcement

CI must machine-enforce at least:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
security authentication integration tests
refresh replay tests
MFA negative tests
device revocation tests
secret leak tests
fuzz/smoke targets where configured
secret scanning
dependency scanning
```

The existing CI policy requires security gates to fail closed. A security scanner that does not execute is not a pass.

For Phase 3, CI should also reject:

- raw `Authorization` logging;
- raw password fields in debug events;
- test fixtures containing realistic production secrets;
- missing auth route inventory metadata;
- undocumented new public authentication error codes;
- unsafe localStorage access for privileged browser credentials where policy prohibits it.

---

# 79. Evidence Requirements

Every Phase 3 release should retain evidence showing:

```text
source revision
toolchain version
dependency lock state
identity-provider configuration class
configuration validation result
security tests
auth negative tests
refresh replay tests
MFA tests
device-revocation tests
secret-scan result
dependency-scan result
migration verification
artifact digest
```

Never include secrets inside the evidence package.

---

# 80. Implementation Sequence

Phase 3 should be implemented in this order:

```text
P3-001 identity-provider ports + configuration
        |
        v
P3-002 principal + security context
        |
        v
P3-003 token/JWKS validation
        |
        v
P3-004 session persistence/lifecycle
        |
        v
P3-005 refresh rotation/replay detection
        |
        v
P3-006 logout/revocation
        |
        v
P3-007 password authentication, if enabled
        |
        v
P3-008 password reset/recovery
        |
        v
P3-009 MFA abstraction
        |
        v
P3-010 TOTP/recovery codes
        |
        v
P3-011 passkey/WebAuthn integration boundary
        |
        v
P3-012 device identity
        |
        v
P3-013 device registration/revocation/replacement
        |
        v
P3-014 telemetry/audit completion
        |
        v
P3-015 security/concurrency/fuzz tests
        |
        v
P3-016 API integration tests
        |
        v
P3-017 CI enforcement
```

Do not start with UI polish. Identity correctness must exist beneath the client.

---

# 81. Implementation PR Strategy

Recommended PR sequence:

```text
PR-020 Identity provider boundary
PR-021 Principal/security context
PR-022 Token validation + JWKS
PR-023 Session persistence
PR-024 Refresh token rotation/replay
PR-025 Logout + revocation
PR-026 Password + reset flows
PR-027 MFA framework
PR-028 TOTP + recovery
PR-029 Passkey boundary
PR-030 Device lifecycle
PR-031 Audit + telemetry
PR-032 Security test expansion
PR-033 CI release gates
```

Each PR should remain independently reviewable and should not create a temporary insecure implementation that gets “fixed later.”

---

# 82. Migration Strategy

Database migration order should permit controlled deployment.

Example:

```text
migration A
  users security-version fields

migration B
  sessions

migration C
  refresh token families

migration D
  MFA records

migration E
  recovery artifacts

migration F
  devices

migration G
  device credentials/versioning

migration H
  authentication audit indexes
```

Migrations must be backward-compatible with the currently deployed application where rolling deployments exist.

Never deploy a binary that requires a column not yet present.

Never remove a security field while an older supported binary still reads it.

---

# 83. Backup and Recovery Implications

Identity data is part of operational security state.

Backups must protect:

- sessions where retention is required;
- device lifecycle evidence;
- security events;
- authenticator metadata;
- recovery policy state.

However, backup copies of secrets require the same or stronger protection than primary storage.

Recovery procedures must account for security state consistency.

Example:

```text
restore DB
   |
   v
restore session/device/security versions
   |
   v
verify application version compatibility
   |
   v
reconcile external identity state
   |
   v
revoke/rotate where compromise cannot be excluded
```

A database restore is not automatically a safe security restore if credential state could have been compromised before the restore point.

---

# 84. Incident Investigation Model

For any identity incident, investigators should be able to reconstruct:

```text
who authenticated?
which authenticator?
which device?
which session?
what assurance level?
when did authentication occur?
which security version applied?
what security-state changes occurred?
what revocation happened?
what external provider responded?
which API operations followed?
```

Correlation IDs from Phase 2 observability link runtime telemetry to durable security events.

The audit system remains authoritative evidence; telemetry remains diagnostic evidence.

---

# 85. Security Review Questions

Every Phase 3 code review should ask:

### Authentication

- Can identity be established without cryptographic verification?
- Can a stale provider assertion create current authority?
- Can algorithm/issuer/audience validation be bypassed?

### Sessions

- Can a revoked session continue?
- Can an expired session become valid again?
- Can logout race with refresh?

### MFA

- Can an unverified enrollment grant assurance?
- Can an MFA artifact be reused?
- Can reset bypass required assurance?

### Devices

- Does a device ID itself grant access?
- Can a revoked device reconnect successfully?
- Can replacement accidentally preserve old device authority?

### Recovery

- Can reset tokens be reused?
- Can accounts be enumerated?
- Can an attacker force victim lockout?

### Secrets

- Does any debug path expose secrets?
- Does an error path retain credentials?
- Does telemetry contain authenticators?

### Concurrency

- What happens with two simultaneous requests?
- Which database constraint prevents double consumption?
- What happens if the process crashes between state transitions?

---

# 86. Non-Goals That Must Not Creep Into Phase 3

Do not turn Phase 3 into:

- a generic IAM product;
- a complete employee directory;
- a CRM identity platform;
- an arbitrary workflow engine;
- a custom policy language;
- a custom cryptographic framework;
- an authorization engine implementation before the Phase 4/6 contracts;
- a support-impersonation subsystem;
- a full enterprise SSO suite without actual customer demand.

Every expansion of the identity boundary requires evidence and, where architecture changes, ADR review.

---

# 87. Phase Dependency Graph

```text
PHASE 0
Contracts / ADR freeze
      |
      v
PHASE 1
Repository / Rust workspace / CI
      |
      v
PHASE 2
Config / Secrets / Errors / Observability
      |
      v
PHASE 3  <--- THIS DOCUMENT
Identity / Sessions / MFA / Devices
      |
      v
PHASE 4
Tenant / Organization / Branch / IAM
      |
      v
PHASE 5
PostgreSQL schema / RLS
      |
      v
PHASE 6
Authorization engine
      |
      v
PHASE 7
Security test framework expansion
```

Phase 3 can implement persistence migrations before the complete authorization engine exists, but it must expose explicit interfaces rather than inventing tenant permissions prematurely.

---

# 88. Final Engineering Contract

The Phase 3 contract is:

> **Sitolo establishes identity through standards-based authentication, normalizes that result into a typed principal, creates a separately managed authenticated session, tracks device identity as an independently revocable security subject, uses explicit authentication assurance levels, supports stronger MFA for high-risk operations, treats recovery as a security-sensitive authentication path, rotates and detects replay of refresh credentials where applicable, validates issuer/audience/signature/time/token-use semantics for JWTs, treats provider/JWKS failures as trust failures rather than reasons to fail open, persists security lifecycle state transactionally, records durable audit evidence, emits bounded telemetry, protects credentials from logs and client storage, and denies operations whenever required identity/session/device facts cannot be established safely.**

The implementation target is:

```text
AUTHENTICATE
     |
     v
VALIDATE
     |
     v
ESTABLISH SESSION
     |
     v
VERIFY DEVICE STATE
     |
     v
ESTABLISH ASSURANCE
     |
     v
HAND OFF TRUSTED PRINCIPAL
     |
     v
TENANT CONTEXT
     |
     v
AUTHORIZATION
     |
     v
BUSINESS OPERATION
```

And the failure rule is:

```text
UNKNOWN
  |
  v
DO NOT ASSUME
  |
  v
DO NOT FAIL OPEN
  |
  v
DENY / REAUTHENTICATE / RECOVER
```

---

# Appendix A — Canonical State Tables

## A.1 Session

| Current | Event | Next | Notes |
|---|---|---|---|
| CREATED | activation succeeds | ACTIVE | authoritative session established |
| CREATED | transaction fails | no session | must not partially activate |
| ACTIVE | logout | REVOKED | explicit user action |
| ACTIVE | idle timeout | EXPIRED | server time |
| ACTIVE | absolute timeout | EXPIRED | server time |
| ACTIVE | security event | REVOKED | policy controlled |
| ACTIVE | device revoked | REVOKED | where device-bound |
| ACTIVE | user security-version mismatch | REVOKED/REAUTH | policy controlled |

## A.2 Device

| Current | Event | Next |
|---|---|---|
| UNREGISTERED | registration begins | REGISTRATION_STARTED |
| REGISTRATION_STARTED | verification required | VERIFICATION_REQUIRED |
| VERIFICATION_REQUIRED | verification succeeds | REGISTERED |
| REGISTERED | activated | ACTIVE |
| ACTIVE | suspension | SUSPENDED |
| SUSPENDED | reinstatement | ACTIVE |
| ACTIVE | revoke | REVOKED |
| REVOKED | replace | RETIRED/REPLACED |

## A.3 MFA

| Current | Event | Next |
|---|---|---|
| UNENROLLED | start | ENROLLMENT_STARTED |
| ENROLLMENT_STARTED | challenge | VERIFICATION_REQUIRED |
| VERIFICATION_REQUIRED | success | ACTIVE |
| VERIFICATION_REQUIRED | expiry | EXPIRED |
| ACTIVE | revoke/reset | REVOKED |

---

# Appendix B — Authentication Error Registry Baseline

| Code | Class | Typical HTTP | Retry | Public detail |
|---|---|---:|---|---|
| AUTHENTICATION_FAILED | authentication | 401 | no | generic |
| AUTHENTICATION_RATE_LIMITED | abuse | 429 | after server delay | generic |
| SESSION_EXPIRED | session | 401 | reauthenticate | generic |
| SESSION_REVOKED | session | 401 | reauthenticate | generic |
| REFRESH_TOKEN_INVALID | session | 401 | reauthenticate | generic |
| REFRESH_TOKEN_REUSED | security | 401 | no | generic |
| MFA_REQUIRED | assurance | 403/401 policy | step-up | generic |
| MFA_FAILED | MFA | 401 | limited | generic |
| MFA_RATE_LIMITED | abuse | 429 | after delay | generic |
| DEVICE_REVOKED | device | 401/403 | no | generic |
| DEVICE_VERIFICATION_REQUIRED | device | 403 | controlled flow | generic |
| IDENTITY_PROVIDER_UNAVAILABLE | dependency | 503 | bounded retry | generic |
| INVALID_RECOVERY_ARTIFACT | recovery | 400/401 | no | generic |
| RECOVERY_ARTIFACT_USED | recovery | 409/401 | no | generic |

The final public registry must be owned centrally and enforced by CI.

---

# Appendix C — Security Test Matrix

| Test | Expected |
|---|---|
| Wrong password | authentication denied |
| Unknown user | non-enumerating response |
| Credential stuffing burst | throttled |
| Expired JWT | denied |
| Wrong issuer | denied |
| Wrong audience | denied |
| Algorithm `none` | denied |
| Untrusted key | denied |
| Refresh twice | second attempt contained |
| Session after logout | denied |
| Password reset then old session | denied where policy requires |
| MFA code replay | denied |
| Recovery code replay | denied |
| Device revoked then API request | denied |
| Device revoked then sync | rejected |
| Forged device ID | insufficient authority |
| Callback with wrong state | denied |
| Callback replay | denied |
| Password in trace fixture | CI failure |
| Token in logs fixture | CI failure |
| Concurrent refresh | one legal transition |
| Concurrent reset | one successful redemption |
| Concurrent device revoke | deterministic final state |

---

# Appendix D — Production Gate

```text
PHASE 3 PRODUCTION GATE

IDENTITY
[ ] provider trust validated
[ ] issuer/audience pinned appropriately
[ ] JWKS/key rotation tested

SESSIONS
[ ] idle timeout
[ ] absolute timeout
[ ] revocation
[ ] logout
[ ] refresh rotation
[ ] replay detection

MFA
[ ] enrollment
[ ] verification
[ ] reset
[ ] recovery
[ ] high-risk assurance

DEVICES
[ ] registration
[ ] revocation
[ ] replacement
[ ] sync interaction

SECURITY
[ ] enumeration tests
[ ] rate-limit tests
[ ] secret redaction tests
[ ] concurrency tests
[ ] fuzzing
[ ] negative API tests

OPERATIONS
[ ] telemetry
[ ] audit
[ ] alerts
[ ] runbooks
[ ] provider outage procedure
[ ] credential compromise procedure

CI
[ ] all mandatory security checks green
[ ] no skipped mandatory gate
[ ] dependency lock verified
[ ] reproducible artifact verified
```

---

# Appendix E — Research Basis

## Project evidence

The Phase 0 `auth_authorization_spec.md` establishes standards-based authentication, OAuth/OIDC alignment, PKCE for native applications, server-side sessions, password hashing requirements, MFA/recovery, assurance classes, security versioning and independent device identity. The existing ADR set independently records the same architectural decisions and explicitly rejects custom authentication cryptography and device-as-user-credential designs.

The API contract requires authentication before tenant context, authorization and business execution, defines bounded inputs and safe RFC 9457-style errors, and prohibits client-supplied authority assertions from becoming authoritative.

The observability specification requires bounded metrics, structured logs, OpenTelemetry correlation and strict exclusion of credentials from telemetry.

## External standards

- RFC 9700 — OAuth 2.0 Security Best Current Practice. citeturn130764search0
- RFC 8252 — OAuth 2.0 for Native Apps. citeturn130764search1turn130764search2
- NIST SP 800-63-4 — Digital Identity Guidelines. citeturn130764search3turn130764search4
- NIST SP 800-63B session management guidance. citeturn130764search6
- NIST authenticator event management guidance. citeturn130764search7
- OWASP Logging Cheat Sheet. citeturn833127search1
- OpenTelemetry Rust documentation. citeturn833127search0turn833127search3
- Rust `secrecy` crate documentation. citeturn833127search2
- Rust `zeroize` documentation. citeturn833127search10

---

# Appendix F — Phase 3 Exit Evidence Package

The implementation package should contain, at minimum:

```text
identity-provider configuration evidence
JWT/JWKS validation test evidence
session lifecycle tests
refresh replay test evidence
MFA lifecycle tests
recovery tests
device lifecycle tests
revoked-device tests
browser authentication tests
native PKCE/state tests
fuzz results
secret-leak results
rate-limit results
migration results
telemetry fixture results
audit fixture results
CI results
artifact provenance
```

No evidence artifact may contain live credentials, raw authentication tokens, private keys, production secrets or unbounded personal data.

---

# Document Governance

Any implementation change that alters:

- authentication protocol;
- identity-provider trust;
- token validation semantics;
- session authority;
- refresh replay semantics;
- MFA assurance;
- recovery semantics;
- device independence/revocation;
- offline security capability;
- secret handling;
- audit semantics;
- security-version semantics;
- production authentication failure behavior

requires security architecture review and, where architectural invariants change, an ADR.

Routine implementation refactors that preserve externally observable security properties may proceed without a new ADR but must preserve all tests and evidence.

---

# FINAL STATUS

**Phase:** 3  
**Domain:** Identity + Sessions + MFA + Device Identity  
**Implementation posture:** security-first, fail-closed, standards-based  
**Authority:** Sitolo server + authoritative persistence  
**Client role:** authenticated operational surface, never identity authority  
**Next dependency:** Phase 4 — Tenant / Organization / Branch / IAM

**End of `phase3_identity_sessions_mfa_device_identity_implementation.md`.**
