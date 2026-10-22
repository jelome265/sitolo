# PR-009 Decision Proposals

**Date:** 2026-10-22  
**Status:** **PROPOSED — NOT OWNER-APPROVED**

This packet supplies concrete, reviewable defaults for the three decisions still blocking implementation. It is not approval from the product, security, platform or event-contract owner. Do not remove fail-closed behavior or mark findings closed solely because these proposals exist.

## Proposed baseline

| Decision | Proposed choice | Gate before implementation |
|---|---|---|
| Relay | Amazon SNS FIFO topic fan-out to one Amazon SQS FIFO queue per consumer (main + each sub-consumer); at-least-once delivery with durable consumer inbox/idempotency | Approve cloud adoption, delivery contract, consumer ownership and ack semantics |
| Lifecycle events | All five state-changing paths remain mandatory audit; introduce distinct v1 event names and stable system actors for scheduler-driven expiry | Approve names, payloads and actor conventions against the binding Phase 4 contract |
| Identity | Amazon Cognito User Pool behind existing OIDC `IdentityProviderPort`; Sitolo still owns internal principal mapping, session/device state, membership and authorization | Approve identity provider, session policy and operating/cost model |
| Production platform/secrets | ECS Fargate API/worker, RDS PostgreSQL, AWS Secrets Manager via a narrow `SecretProvider`, workload IAM roles; proposed region `af-south-1` | Approve region, cost, data handling, rotation and operations ownership |

AWS's [regional service catalogue](https://aws.amazon.com/about-aws/global-infrastructure/regional-product-services/) lists core ECS/Fargate, SNS, SQS and Secrets Manager services; AWS separately announced [Cognito availability in Cape Town](https://aws.amazon.com/about-aws/whats-new/2023/12/amazon-cognito-africa-cape-town-region/). This shows technical plausibility, not procurement approval or legal/data-residency compliance.

## Decision records

- [Relay and consumer contract](relay-delivery-contract-proposal.md)
- [IAM lifecycle event mappings](iam-lifecycle-event-mapping-proposal.md)
- [Identity, production runtime and secrets](identity-production-runtime-proposal.md)

## Explicit non-approval

No production adapter, cloud resource, Cognito token acceptance, or production secret retrieval should be enabled just because these files are committed. The accountable owner must accept or replace the exact proposal revision. Until then, these findings remain open and fail-closed guards remain required.

| Owner role | Decision | Status |
|---|---|---|
| Product/platform architecture owner | SNS/SQS FIFO, main/sub subscribers, AWS and region | Pending |
| Phase 4 domain/event-contract owner | Event names, schemas, payload and expiry actor semantics | Pending |
| Identity/security owner | Cognito/OIDC boundary and server-side session policy | Pending |
| Platform/security operations owner | ECS/RDS/Secrets Manager, IAM, rotation, recovery and SLOs | Pending |
