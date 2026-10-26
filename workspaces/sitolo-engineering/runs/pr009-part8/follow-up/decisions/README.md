# PR-009 Decision Proposals

**Date:** 2026-10-22  
**Status:** **OWNER-APPROVED — 2026-10-22**

**Approval record — 2026-10-22:** the repository owner explicitly instructed that the decision in this record be labelled owner-approved. This records repository-owner approval of the proposal as written. It does not claim separate signatures from distinct platform, security or domain role holders unless those roles are delegated to the repository owner.

The decision-selection gate is now closed at repository-owner level. Implementation, deployment provisioning, verification and human PR review remain outstanding.

## Approved baseline

| Decision | Proposed choice | Gate before implementation |
|---|---|---|
| Relay | Amazon SNS FIFO topic fan-out to one Amazon SQS FIFO queue per consumer (main + each sub-consumer); at-least-once delivery with durable consumer inbox/idempotency | Approve cloud adoption, delivery contract, consumer ownership and ack semantics |
| Lifecycle events | All five state-changing paths are mandatory audit; distinct v1 names and the documented system actors | **Approved 2026-10-22**; remaining work is implementation and evidence |
| Invitation retention | Delete contact data and token-hash data when invitations become accepted, withdrawn or expired; remove token lookup indexes in the same transition | **Approved 2026-10-22** |
| Identity | Amazon Cognito User Pool behind existing OIDC `IdentityProviderPort`; Sitolo still owns internal principal mapping, session/device state, membership and authorization | Approve identity provider, session policy and operating/cost model |
| Production platform/secrets | ECS Fargate API/worker, RDS PostgreSQL, AWS Secrets Manager via a narrow `SecretProvider`, workload IAM roles; proposed region `af-south-1` | Approve region, cost, data handling, rotation and operations ownership |

AWS's [regional service catalogue](https://aws.amazon.com/about-aws/global-infrastructure/regional-product-services/) lists core ECS/Fargate, SNS, SQS and Secrets Manager services; AWS separately announced [Cognito availability in Cape Town](https://aws.amazon.com/about-aws/whats-new/2023/12/amazon-cognito-africa-cape-town-region/). This shows technical plausibility, not procurement approval or legal/data-residency compliance.

## Decision records

- [Relay and consumer contract](relay-delivery-contract-proposal.md)
- [IAM lifecycle event mappings](iam-lifecycle-event-mapping-proposal.md)
- [Identity, production runtime and secrets](identity-production-runtime-proposal.md)

## Approval boundary

The approved choices authorize implementation against these contracts; they do not authorize an unreviewed production rollout or bypass of release gates. Live resources still require infrastructure review, cost/ownership, least-privilege policies, integration tests and release approval. Keep fail-closed guards until implementation is complete.

| Owner role | Decision | Status |
|---|---|---|
| Repository owner | SNS/SQS FIFO, main/sub subscribers, AWS and region | **Approved 2026-10-22** |
| Repository owner | Event names, schemas, payload and expiry actor semantics | **Approved 2026-10-22** |
| Repository owner | Delete invitation contact/token-hash data at terminal state | **Approved 2026-10-22** |
| Repository owner | Cognito/OIDC boundary and server-side session policy | **Approved 2026-10-22** |
| Repository owner | ECS/RDS/Secrets Manager baseline, IAM boundaries, rotation and recovery requirements | **Approved 2026-10-22; implementation review remains** |
