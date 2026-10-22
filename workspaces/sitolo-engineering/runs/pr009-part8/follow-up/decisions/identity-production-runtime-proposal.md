# Proposed Identity, Production Runtime and Secret Provider

**Status:** **PROPOSED — NOT APPROVED**  
**Governing sources:** [Phase 3 identity/session specification](../../../../../../docs/phase3_identity_sessions_mfa_device_identity_implementation.md) and [Phase 2 configuration/secrets specification](../../../../../../docs/phase2_config_secrets_logging_errors_telemetry_implementation.md).

## 1. Concrete baseline proposal

| Capability | Proposed choice |
|---|---|
| Region | AWS Africa/Cape Town, `af-south-1`, after latency/cost/data-handling review |
| API and worker compute | Separate ECS Fargate services/tasks and task IAM roles |
| Authoritative DB | Amazon RDS for PostgreSQL, major version 18 after engine/extension compatibility checks |
| Identity provider | Amazon Cognito User Pool behind the existing protocol-neutral `IdentityProviderPort` |
| Production secrets | AWS Secrets Manager via a real production `SecretProvider` adapter |
| Workload auth | Short-lived credentials supplied by ECS task IAM roles; no static AWS keys |
| Relay | SNS FIFO topic + SQS FIFO subscriber queues per [relay proposal](relay-delivery-contract-proposal.md) |
| Schema changes | Restricted migration job/role; API and worker never run migrations or use an administrator DSN |

AWS lists PostgreSQL 18 versions in its current [RDS release notes](https://docs.aws.amazon.com/AmazonRDS/latest/PostgreSQLReleaseNotes/postgresql-versions.html), lists managed-service availability in its [regional catalogue](https://aws.amazon.com/about-aws/global-infrastructure/regional-product-services/), and announced [Cognito availability in Cape Town](https://aws.amazon.com/about-aws/whats-new/2023/12/amazon-cognito-africa-cape-town-region/). Check exact service/engine-minor availability in the selected region before provisioning. Region selection alone does not establish legal compliance or data-residency sufficiency.

## 2. Identity and session model

Use **Amazon Cognito User Pool** as the first upstream OIDC adapter; do not introduce Cognito-specific user types into the Sitolo domain.

- Flutter/native public clients use OAuth 2.0 Authorization Code with PKCE through an external browser. No durable client secret is embedded in the app.
- APIs accept the configured access-token type only. Validate signature with trusted issuer JWKS; issuer; allowed signing algorithm; token use/type; expected client/resource audience; expiry/not-before; and required scopes/assurance. Reject malformed tokens, unknown issuer/key/algorithm, or missing required claims. Bound JWKS caching; refresh on unknown key IDs; never fetch a caller-supplied JWKS URL.
- Map immutable exact `(issuer, subject)` to Sitolo's internal `UserId`. Email/display name are not stable authorization identifiers. A request-body `owner_user_id` cannot establish authority.
- Proposed access token TTL: **10 minutes**, to be validated against the chosen Cognito settings. Short TTL does not replace server-side Sitolo session validity/revocation or per-request authorization.
- Sitolo still owns normalized principals, its server-side session lifecycle, device binding/revocation, MFA/assurance policy, membership, role/scope resolution, tenant authorization and resource/property authorization. Cognito authenticates upstream identity; it does not grant Sitolo organization membership.
- All HTTP commands derive owner and audit actor server-side from the authenticated principal and Sitolo state. Request tenant/resource IDs are selectors, not permission. Automated work uses a stable system actor, never a fabricated human.

The project’s Phase 3 contract already says to use standard OIDC/JWT implementations and keep identity-provider choice behind an adapter. See [OpenID Connect Core](https://openid.net/specs/openid-connect-core-1_0.html) and [RFC 8725 JWT Best Current Practices](https://www.rfc-editor.org/rfc/rfc8725.html). Apply token-type-specific rules for Cognito access tokens; do not blindly apply an ID-token validation recipe to access tokens.

## 3. Secret-provider contract

Implement an AWS Secrets Manager-backed `SecretProvider` that resolves only allowlisted reference classes. The AWS SDK obtains short-lived identity from the ECS task role. Secrets are held in narrow secret types and must not enter logs, tracing, metrics, config fingerprints, debug output, serialized domain values or error responses.

Proposed logical references (examples only, no real ARN/account/secret value):

| Consumer | Reference | Permission boundary |
|---|---|---|
| API runtime | `prod/sitolo/api/runtime-db-password` | Read that secret only |
| Worker | `prod/sitolo/worker/worker-db-password` | Read that secret only |
| Migration job | `prod/sitolo/migrations/admin-db-credential` | Migration-only role; never available to API/worker |
| Future integration | `prod/sitolo/<service>/<credential-class>` | Adapter-specific minimum permission |

Verify that the typed secret-reference type can represent the approved logical reference safely. Do not put values in `.env.example`, GitHub Actions output, repository variables, or config snapshots.

### Workload identity boundaries

- **API task role:** read its runtime database secret and specifically approved API secret classes. No SNS publish and no queue consumer permissions.
- **Worker task role:** read the worker database secret and publish only to the approved SNS topic. No admin secret, queue deletion, or broad secrets access.
- **Consumer task role:** consume/delete only from its assigned queue and approved DLQ.
- **Migration role:** separate, constrained, used only in a controlled migration job/operator flow.
- **Deployment role:** if automated deployment is added, prefer GitHub Actions OIDC federation with tightly scoped repository/environment/branch trust instead of long-lived repository AWS keys.

Use TLS with certificate verification to PostgreSQL, private database networking, and narrow inbound security group access. Scope `secretsmanager:GetSecretValue` and any `kms:Decrypt` to exact secrets/keys and workload principals.

## 4. Rotation and fail-closed behavior

- Before production, name the rotation operator, policy/cadence, alerting and recovery procedure, including whether old/new credentials can overlap.
- Rotation provisions and verifies a replacement, updates the secret, rolls/reconnects the applicable ECS tasks/pools, verifies health, then revokes the old credential. No old credential remains valid indefinitely.
- Missing provider, unresolved/invalid required secret, invalid DB credentials, or untrusted configuration prevents readiness.
- Never fall back to `SITOLO_SECRET__...`, committed defaults, an in-memory database or administrator credentials. Existing production fail-closed guards stay until provider and real database composition are implemented and tested.
- Secret references remain ordinary typed config (for example, `SITOLO__DATABASE__PASSWORD_REF`); the raw value remains provider-owned and excluded from the effective config fingerprint.

## 5. Runtime composition

API startup must validate config, build the approved provider, resolve the database password, build the actual PostgreSQL pool as `app_runtime`, assert both `current_user` and `session_user` are `app_runtime`, verify required dependency capability, and only then publish readiness.

Worker startup must resolve its own secret, connect as `app_worker`, validate the outbox/claim capabilities and approved relay config, and publish readiness only afterwards. The admin/migration identity is never a fallback.

Current `apps/api/src/state.rs` creates an in-memory tenancy store and bootstrap retains database runtime intent instead of composing the production pool. This requires application code, migrations and tests; it is not solved by adding env keys.

## 6. Required evidence

1. Valid provider access token succeeds; invalid signature, issuer/client, token use, expiry, JWKS key, inactive/revoked Sitolo session and insufficient assurance fail closed.
2. Caller-supplied owner/tenant fields cannot override principal mapping; cross-tenant operations are denied.
3. Missing provider/reference, denied IAM permission, provider outage and bad DB credentials prevent readiness without exposing secret values.
4. API cannot read worker/admin secrets or publish; worker cannot read admin credentials or consume/delete subscriber messages.
5. Each database pool proves its actual restricted login identity.
6. Rotation updates pool/task state safely and has recovery evidence.
7. Private networking, TLS, narrow IAM, secret-free logs, backups/restore and RTO/RPO are documented and exercised.
8. Exact-source integration/security/release checks pass and a human accepts the platform decision.

## Approval

This is an explicit baseline for review, not an approved choice. The accountable owner must accept it or replace it with another concrete provider/deployment record. If cost/ownership rejects AWS, keep the existing provider/adapter boundaries and choose one alternative deliberately; do not introduce a multi-cloud framework just to avoid the decision.
