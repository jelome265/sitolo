# Sitolo — Deployment Specification

**Document:** 12 / 16
**Phase:** Phase 0 — Architecture, Contracts & ADR Freeze
**Status:** Normative implementation contract
**Version:** 1.0
**Date:** 2026-09-04

> This document defines the deployment, runtime, release, infrastructure, configuration, disaster-recovery, rollback, supply-chain, and operational boundaries for Sitolo. It converts the architectural and security decisions into deployable infrastructure rules.

---

## 1. Purpose

Sitolo is a mobile-first business operating system whose deployment model must support:

- reliable merchant transaction processing;
- intermittent client connectivity;
- authoritative server-side financial and inventory state;
- multi-tenant isolation;
- payment and tax integrations;
- durable asynchronous work;
- auditable administration;
- secure software delivery;
- controlled degradation;
- recoverable infrastructure failure;
- predictable release and rollback behavior;
- future multi-region expansion without pretending that multi-region is required for MVP.

Deployment is therefore treated as an engineering control plane, not merely a method for copying a binary onto a server.

The deployment system MUST preserve these architectural truths:

```text
Flutter / Tauri / Web
        |
        v
   Edge / API
        |
        v
Rust Application
        |
        +--> PostgreSQL  <-- authoritative server state
        |
        +--> Job / Outbox workers
        |
        +--> Object storage
        |
        +--> Optional Redis / cache
        |
        +--> Payment providers
        |
        +--> MRA EIS
        |
        +--> Observability pipeline
```

The deployment layer MUST NOT become a second business-logic layer.

---

## 2. Deployment Principles

### 2.1 Correctness before availability

A deployment must prefer controlled denial over accepting an operation whose authorization, financial state, inventory state, idempotency state, or integrity cannot be established safely.

### 2.2 PostgreSQL is authoritative

PostgreSQL is the authoritative server-side transactional store. No deployment component may introduce another silently competing source of truth.

### 2.3 Stateless API where practical

API processes should be replaceable and horizontally scalable without depending on local process memory for authoritative state.

Allowed local process state:

- connection pools;
- bounded caches;
- compiled configuration;
- transient request state;
- short-lived cryptographic material loaded through approved secret interfaces.

Forbidden authoritative process state:

- merchant balances;
- inventory quantities;
- payment truth;
- tax submission truth;
- tenant membership;
- durable job ownership.

### 2.4 Durable work is explicit

Anything that must survive process death becomes durable state: outbox records, integration intents, job records, idempotency records, sync commands, audit events, and reconciliation state.

### 2.5 External side effects are asynchronous unless deliberately justified

Database transactions MUST NOT remain open across external HTTP calls. The canonical pattern is:

```text
DB transaction
  -> durable intent/outbox
  -> commit
  -> worker
  -> external side effect
  -> durable result
  -> reconciliation
```

### 2.6 Kubernetes is not mandatory

A Kubernetes deployment may be adopted later where its operational benefits justify the additional control-plane complexity. The application contract must remain platform-neutral enough to run on managed containers, VMs, or Kubernetes.

The deployment design must not introduce Kubernetes-specific assumptions into application code.

### 2.7 Infrastructure must be reproducible

Production infrastructure MUST be represented as version-controlled configuration and machine-applicable automation wherever practical.

### 2.8 Immutable artifacts

Deployments use immutable build artifacts identified by cryptographic digest or equivalent immutable identifier. Mutable `latest`-style production deployment references are prohibited.

---

## 3. Environment Model

Sitolo MUST maintain clearly separated environments.

Minimum environments:

```text
local
    |
    v
CI / ephemeral integration
    |
    v
development
    |
    v
staging / certification
    |
    v
production
```

### 3.1 Local

Purpose:

- developer iteration;
- migration development;
- unit/integration tests;
- local dependency simulation.

Production secrets MUST never be copied into local configuration.

### 3.2 CI / ephemeral

Purpose:

- isolated pull-request validation;
- integration suites;
- security tests;
- disposable PostgreSQL;
- API contract tests;
- migration tests;
- build reproducibility checks.

Ephemeral environments MUST have bounded lifetimes and automatic cleanup.

### 3.3 Development

Purpose:

- shared feature validation;
- integration with non-production services;
- exploratory UI/backend work.

Development data MUST be synthetic or sanitized.

### 3.4 Staging / certification

Purpose:

- production-like deployment verification;
- release candidate validation;
- payment sandbox testing;
- MRA certification/testing where applicable;
- load and resilience tests that are not safe in production.

Staging should approximate production architecture sufficiently that release behavior is meaningful.

### 3.5 Production

Production is the authoritative customer-serving environment.

Production credentials, databases, signing keys, provider accounts, MRA terminal secrets, customer data, and operational identifiers MUST remain isolated from all lower environments.

---

## 4. Environment Isolation

At minimum, environments MUST be separated by:

- credentials;
- secret stores/namespaces;
- databases or database clusters as appropriate;
- object-storage namespaces/buckets;
- payment provider accounts or modes;
- MRA test versus production configuration;
- telemetry destinations where customer data may appear;
- deployment identity;
- network policy;
- administrative authorization.

A staging compromise must not provide a credential path to production.

A production credential MUST NOT be valid in development or staging.

---

## 5. Runtime Components

The initial production topology SHOULD be a modular Rust application plus separately managed workers rather than a large microservice fleet.

Conceptual components:

```text
                         Internet
                            |
                            v
                    CDN / Edge / WAF
                            |
                            v
                     Load Balancer
                            |
             +--------------+--------------+
             |                             |
             v                             v
        Rust API A                     Rust API B
             |                             |
             +--------------+--------------+
                            |
                            v
                        PostgreSQL
                            ^
                            |
               +------------+-----------+
               |                        |
               v                        v
          Worker pool               Reporting jobs
               |
      +--------+---------+
      |        |         |
      v        v         v
 Payment     MRA EIS   Notifications
 adapters    adapter    adapter

Other infrastructure:
- object storage
- optional Redis
- telemetry collector/backend
- secret manager
```

This topology may run on VMs, managed containers, or Kubernetes.

---

## 6. API Runtime Requirements

The API runtime MUST be:

- horizontally replaceable;
- health-checkable;
- bounded by CPU and memory limits;
- configured exclusively through approved configuration sources;
- observable through structured telemetry;
- resistant to request resource exhaustion;
- graceful under dependency failure;
- able to drain during deployment;
- free of authoritative mutable local state.

### 6.1 Startup validation

An API process MUST validate required startup configuration before serving traffic.

Required categories may include:

- environment identifier;
- database connectivity configuration;
- cryptographic verification/signing configuration where required;
- secret-provider connectivity;
- telemetry endpoint/configuration;
- public origin configuration;
- feature/configuration version;
- integration configuration references.

Startup SHOULD fail rather than run with missing security-critical configuration.

### 6.2 Dependency initialization

Optional dependencies must be explicitly classified as optional.

Example:

```text
PostgreSQL unavailable
    -> application not ready

Redis unavailable
    -> continue only if feature semantics permit

Telemetry collector unavailable
    -> continue with bounded local buffering/drop policy

Payment provider unavailable
    -> payments become pending/unavailable, not fabricated success
```

---

## 7. Health Checks

Every deployable process MUST expose health semantics appropriate to its role.

### 7.1 Liveness

Liveness answers:

> Is the process alive and able to participate in normal scheduling?

It MUST NOT perform expensive external dependency checks.

### 7.2 Readiness

Readiness answers:

> Should this process receive production traffic right now?

Readiness MAY verify critical dependencies such as PostgreSQL connectivity or migration compatibility state.

### 7.3 Startup

Startup checks prevent a newly created instance from receiving traffic before initialization is complete.

### 7.4 Health-check anti-patterns

Do not:

- make liveness depend on every external provider;
- cause cascading restarts because of temporary provider outages;
- put expensive reports in readiness probes;
- treat telemetry backend availability as business readiness;
- use health endpoints to expose secrets or configuration.

---

## 8. Graceful Shutdown

Deployable processes MUST support graceful shutdown.

Sequence:

```text
termination signal
      |
      v
stop accepting new work
      |
      v
drain in-flight requests
      |
      v
finish safe local state transitions
      |
      v
release leases / connections
      |
      v
exit
```

Workers MUST also release or expire job leases safely.

A process killed during a job MUST leave the durable job state recoverable by another worker.

---

## 9. API Availability and Draining

During rolling deployment:

1. instance becomes not-ready;
2. traffic routing drains;
3. in-flight requests complete up to bounded shutdown timeout;
4. process terminates;
5. replacement reaches readiness;
6. traffic resumes.

The system MUST NOT require simultaneous deployment of all API instances.

Backward-compatible API/database transition design is mandatory for rolling updates.

---

## 10. Database Deployment Contract

PostgreSQL is a stateful critical dependency and MUST be deployed separately from stateless application instances.

Required production properties:

- encrypted transport;
- encrypted storage where provider capability exists;
- controlled credentials;
- automated backups;
- point-in-time recovery where supported and required;
- connection limits;
- monitoring;
- migration discipline;
- replication strategy appropriate to availability objectives;
- restore drills.

### 10.1 Application database identity

The runtime application role MUST have only the privileges required for runtime operations.

It MUST NOT be a PostgreSQL superuser.

It MUST NOT receive unnecessary database administration capabilities.

Where RLS is used, the runtime role MUST NOT accidentally bypass intended policy enforcement.

### 10.2 Migration ownership

Schema migrations are deployed through a controlled migration process.

Application startup MUST NOT automatically execute arbitrary destructive migrations in production.

Production migrations should be a distinct deployment step with:

- migration identifier;
- preflight checks;
- expected lock behavior;
- execution timeout policy;
- rollback/recovery procedure;
- verification.

---

## 11. Expand-and-Contract Database Changes

Breaking schema changes must use expand-and-contract when rolling deployments require compatibility.

Example:

```text
Release N
  add new column/table
  keep old path working

Release N+1
  write both / migrate data
  read new representation

Release N+2
  stop old writes
  verify no old readers

Release N+3
  remove old representation
```

Do not deploy a schema removal before all active application versions have stopped depending on it.

---

## 12. Deployment Ordering

For a normal backward-compatible release:

```text
1. validate candidate
2. create immutable artifact
3. verify artifact/provenance
4. apply additive DB migration if required
5. deploy application
6. run health checks
7. run smoke checks
8. progressively increase traffic
9. monitor
10. finalize release
```

For destructive migrations, a specific ADR and recovery plan are required.

---

## 13. Configuration Management

Configuration is classified into:

```text
static application configuration
runtime environment configuration
secrets
tenant configuration
provider configuration
regulatory configuration
feature flags
```

These MUST NOT be conflated.

### 13.1 Static configuration

Examples:

- server port;
- timeout defaults;
- feature compile-time capability.

### 13.2 Runtime configuration

Examples:

- database endpoint;
- telemetry endpoint;
- service limits.

### 13.3 Secrets

Examples:

- database passwords;
- provider API keys;
- signing keys;
- webhook secrets;
- MRA terminal secret material.

Secrets MUST come from approved secret-management mechanisms and MUST NOT be baked into container images or client binaries.

### 13.4 Tenant configuration

Tenant-owned business settings are durable business data, not environment variables.

### 13.5 Regulatory configuration

Tax/EIS configuration received from MRA is an external integration projection and MUST retain provenance and effective version.

---

## 14. Secret Management

Secrets management requirements:

- centralized secret storage where practical;
- least-privileged access;
- environment separation;
- rotation support;
- audit trail of secret access where provider supports it;
- no secret logging;
- no secret inclusion in crash reports;
- no secret in Git history;
- no secret in Docker/container layers;
- no secret in mobile/web bundles.

Secret compromise response:

```text
revoke
  -> rotate
  -> redeploy
  -> verify old secret fails
  -> investigate blast radius
  -> preserve evidence
```

---

## 15. Secret Rotation

Application design SHOULD support overlapping credentials where provider semantics permit:

```text
old secret + new secret
       |
       v
transition window
       |
       v
new secret only
       |
       v
old secret revoked
```

Hard cutovers are acceptable where necessary but require coordinated operational handling.

Cryptographic signing-key rotation MUST maintain verification of still-valid historical material for the required validation window.

---

## 16. Containerization

Container images are preferred for reproducible service packaging where the selected runtime platform supports containers.

Images SHOULD:

- use minimal runtime bases;
- run as non-root where practical;
- contain only required runtime files;
- use a read-only filesystem where practical;
- drop unnecessary Linux capabilities;
- use explicit network policy;
- avoid shell access in production;
- pin critical base image references by digest where practical;
- be scanned before promotion.

Container startup MUST not download arbitrary executable dependencies from the public internet.

---

## 17. Image Build Rules

A production image build MUST be deterministic enough to identify exactly what source and dependencies produced it.

Build inputs should include:

```text
source commit
build pipeline identity
compiler/toolchain version
Cargo.lock
dependency lock state
base image digest
build configuration
```

The resulting release should have:

```text
artifact digest
SBOM
provenance attestation
source revision
release identifier
```

SLSA is used as a supply-chain provenance reference; note that SLSA 1.1 is retired and current SLSA documentation identifies 1.2 as current, so implementation should use the current compatible provenance specification rather than hard-coding an obsolete version. 

---

## 18. Rust Toolchain Pinning

The Rust toolchain MUST be explicitly pinned for reproducible builds.

Repository policy should use a versioned toolchain declaration such as:

```text
rust-toolchain.toml
```

The exact production version is an implementation release decision, but it MUST be recorded in version control and updated deliberately.

Cargo.lock MUST be committed for application/service binaries where appropriate.

CI MUST verify lockfile consistency.

---

## 19. Dependency Reproducibility

Dependency resolution MUST be reproducible.

Required controls:

- committed lockfiles;
- dependency update review;
- vulnerability scanning;
- license policy;
- transitive dependency visibility;
- supply-chain anomaly monitoring.

A build MUST fail where a required lockfile is absent or inconsistent with the repository's policy.

---

## 20. Deployment Artifact Promotion

The same immutable release artifact MUST move through environments where practical.

Preferred flow:

```text
build once
   |
   v
scan
   |
   v
attest
   |
   v
staging
   |
   v
approve
   |
   v
production
```

Do not rebuild the source independently for production after staging validation unless the process explicitly treats the new build as a new release candidate.

---

## 21. Release Identity

Each production deployment MUST be attributable to:

- source commit;
- release version;
- artifact digest;
- deployment environment;
- deployment actor or automation identity;
- migration version;
- configuration version where relevant;
- time of deployment.

Release metadata is retained for operational forensics.

---

## 22. CI/CD Trust Boundary

CI/CD is a privileged production system.

Rules:

- untrusted pull-request code MUST NOT receive production credentials;
- deployment identities MUST be short-lived where practical;
- production deployment permissions MUST be separate from ordinary CI test permissions;
- protected branches/tags are required for production releases;
- release approvals must be auditable;
- secrets must not be printed by workflows;
- third-party actions must be pinned according to repository policy;
- artifact provenance must be verifiable.

---

## 23. Deployment Authentication

Production deployment SHOULD use workload identity or equivalent short-lived credentials instead of long-lived static deployment tokens.

Where OIDC-based workload identity is supported, the deployment identity should be restricted by:

- repository;
- branch/tag/environment;
- workflow identity;
- deployment target;
- operation permissions.

A stolen CI token must not automatically grant unrestricted cloud administration.

---

## 24. Infrastructure as Code

Infrastructure MUST be declarative and version-controlled.

The exact IaC technology is intentionally not frozen by this document.

It MUST be capable of defining:

- networking;
- compute;
- database;
- object storage;
- secret references;
- monitoring;
- access control;
- firewall/network policy;
- backups;
- DNS/edge resources where applicable.

IaC changes require review for:

- security;
- data exposure;
- cost impact;
- availability;
- state migration;
- rollback feasibility.

---

## 25. Network Architecture

Production network segmentation SHOULD distinguish:

```text
Internet-facing edge
       |
       v
API/application tier
       |
       +--> PostgreSQL
       +--> workers
       +--> internal services
       +--> secret provider
       +--> telemetry
       |
       +--> controlled external integrations
```

PostgreSQL MUST NOT be directly internet-addressable.

Administrative interfaces MUST NOT be exposed publicly unless protected by explicitly approved controls.

---

## 26. Egress Control

External communication SHOULD be allowlisted by function where operationally practical.

Examples:

```text
payment provider endpoints
MRA EIS endpoints
telemetry backend
object storage
notification provider
identity provider
```

Arbitrary outbound HTTP from business services is prohibited unless explicitly justified.

This materially reduces SSRF blast radius.

---

## 27. DNS and TLS

Production public endpoints MUST use TLS.

Certificate lifecycle must be automated where possible.

Certificate renewal MUST be observable.

Expired certificates must fail deployment/preflight rather than being discovered by customers.

Internal service TLS requirements depend on the hosting architecture but must be appropriate to the trust boundary.

---

## 28. Edge Controls

The edge layer MAY provide:

- TLS termination;
- DDoS protection;
- WAF rules;
- request-size limits;
- rate limits;
- connection limits;
- bot/abuse controls;
- static asset delivery;
- API routing.

Edge controls do not replace application authorization.

---

## 29. API Rate-Limit Deployment

Rate limiting SHOULD exist at multiple layers:

```text
edge
  -> IP/network dimensions

application
  -> identity/tenant/resource dimensions

business logic
  -> operation-specific limits
```

Limits must preserve enough capacity for critical merchant operations such as POS synchronization and payment confirmation.

---

## 30. PostgreSQL Availability

The initial deployment MAY use one primary PostgreSQL instance where business scale and availability objectives justify the tradeoff.

As requirements increase, the architecture may add:

- synchronous/asynchronous replicas;
- managed failover;
- read replicas;
- PITR;
- separate analytics infrastructure.

The deployment choice MUST follow measured availability and recovery requirements rather than an arbitrary “enterprise” label.

---

## 31. PostgreSQL Connection Management

The API and workers MUST use bounded connection pools.

Pool sizing must account for:

- PostgreSQL max connections;
- number of application instances;
- worker concurrency;
- admin/reporting workloads;
- migration tooling;
- failover behavior.

A scaling event MUST NOT multiply connection demand beyond database capacity.

---

## 32. Redis Deployment Boundary

Redis is optional and non-authoritative.

Redis MAY provide:

- cache;
- rate limiting;
- ephemeral coordination;
- short-lived locks where appropriate.

Redis MUST NOT become the only durable store for:

- payments;
- inventory;
- financial transactions;
- tax submissions;
- durable commands.

If Redis is unavailable, features depending on it MUST degrade according to explicit semantics.

---

## 33. Object Storage

Object storage may hold:

- exports;
- receipts/documents where required;
- backup artifacts where appropriate;
- customer-uploaded business files.

Required controls:

- private-by-default buckets;
- server-authorized access;
- signed URLs with bounded expiry where needed;
- malware/file validation according to file class;
- encryption;
- lifecycle/retention policies;
- access logging;
- deletion semantics consistent with legal/business requirements.

Object-store credentials MUST not be exposed to clients unless a narrow presigned upload/download capability is deliberately implemented.

---

## 34. Worker Deployment

Workers are separate deployable processes or process groups where useful.

Worker properties:

- durable job claim;
- lease/visibility timeout;
- bounded concurrency;
- retry policy;
- idempotency;
- dead-letter handling;
- graceful shutdown;
- metrics;
- structured errors.

Worker classes SHOULD be separated by operational criticality.

Example:

```text
critical financial/integration
high-priority reconciliation
normal notifications
bulk reporting
maintenance
```

Bulk jobs MUST not starve transaction-critical processing.

---

## 35. Outbox Processing

Outbox processing MUST be safe against duplicate execution.

Canonical sequence:

```text
claim
  -> execute idempotent side effect
  -> record completion
```

If a worker crashes after an external side effect and before recording completion, the retry path MUST reconcile rather than assume the side effect did not happen.

---

## 36. Payment Worker Deployment

Payment operations MUST be isolated from ordinary asynchronous jobs sufficiently to preserve capacity during incidents.

Required properties:

- bounded concurrency;
- provider-specific timeouts;
- retry classification;
- unknown-outcome handling;
- circuit breaking where appropriate;
- reconciliation scheduling;
- alerting on pending-age growth.

A provider outage must not exhaust all worker capacity.

---

## 37. MRA EIS Worker Deployment

MRA EIS integration must use a dedicated controlled execution path.

Worker state must retain enough evidence to determine:

- sale/transaction identifier;
- terminal identity;
- configuration snapshot/version;
- submission payload hash;
- attempt count;
- last error;
- external response/reference;
- tax state.

MRA failure MUST NOT corrupt the underlying sale transaction.

---

## 38. Offline Sync Deployment

Offline synchronization is a client/server protocol and must be deployed with backward-compatible schema/version support.

The server MUST support the protocol versions intentionally allowed by the release policy.

Protocol downgrade attacks MUST be rejected.

A new server release MUST NOT invalidate a valid already-installed mobile client without an explicit compatibility strategy.

---

## 39. Mobile Release Coordination

Flutter releases introduce a longer upgrade tail than backend deployments.

Therefore API changes should follow:

```text
add compatible server capability
      |
      v
release mobile client
      |
      v
observe adoption
      |
      v
retire old protocol
```

Destructive removal of API behavior requires measured client adoption and a deprecation policy.

---

## 40. Tauri Release Coordination

Desktop clients must use a controlled update channel.

Required:

- authenticated update metadata;
- signed artifacts where supported by platform/update system;
- rollback availability;
- version compatibility checks;
- protection against downgrade to vulnerable versions.

---

## 41. Deployment Strategies

Supported strategies:

### Rolling

Default for backward-compatible stateless application releases.

### Canary

Recommended for risky changes or material infrastructure changes.

### Blue/green

Appropriate when fast environment switch and duplicate capacity are worth the cost.

### Maintenance window

Appropriate for explicitly incompatible migrations that cannot be made online safely.

The chosen strategy must match the actual failure mode of the change.

---

## 42. Canary Requirements

A canary release should start with a controlled portion of traffic or an isolated test cohort.

Observe:

- HTTP errors;
- latency;
- database errors;
- authorization denials;
- payment failures;
- EIS failures;
- sync rejection rates;
- memory/CPU;
- queue depth;
- business transaction success.

A canary that only passes infrastructure health checks is insufficient.

---

## 43. Release Gates

Production deployment MUST be blocked by unresolved:

- failing critical tests;
- critical security vulnerabilities;
- cross-tenant isolation failures;
- migration validation failures;
- artifact provenance failures;
- secret exposure;
- failed restore prerequisites;
- broken payment idempotency tests;
- broken EIS contract tests when applicable;
- incompatible API/schema changes without migration plan.

---

## 44. Pre-Deployment Checklist

```text
[ ] source commit identified
[ ] release version created
[ ] dependency lock state verified
[ ] build reproducible
[ ] unit tests pass
[ ] integration tests pass
[ ] security tests pass
[ ] API contract tests pass
[ ] tenant isolation suite passes
[ ] payment tests pass
[ ] EIS tests pass where applicable
[ ] migration reviewed
[ ] backup health verified
[ ] artifact scanned
[ ] SBOM generated
[ ] provenance verified
[ ] configuration validated
[ ] secrets available without exposure
[ ] rollback plan documented
[ ] operator ownership assigned
```

---

## 45. Database Migration Gate

A production migration must answer:

1. What changes?
2. Does it acquire long locks?
3. What happens if interrupted?
4. Is it backward compatible?
5. What is the recovery procedure?
6. What is the expected runtime at production scale?
7. Does it affect RLS/authorization?
8. Does it affect financial immutability?
9. Does it affect indexes/query plans?
10. Can application versions N and N+1 coexist?

No production migration is approved without those answers.

---

## 46. Deployment Observability

Every deployment emits telemetry identifying:

```text
release_id
artifact_digest
source_revision
environment
migration_version
 deployment_started_at
 deployment_completed_at
 deployment_result
```

Deployment dashboards MUST correlate release events with:

- errors;
- latency;
- database load;
- worker queues;
- payments;
- EIS;
- sync;
- authentication;
- authorization.

---

## 47. SLO Model

Initial SLO categories:

### Availability

API availability for normal customer traffic.

### Latency

Critical POS/API paths should have explicit percentile objectives.

### Durability

Committed server-side transactions must remain recoverable according to the database recovery objectives.

### Integration freshness

Payment/EIS pending queues must remain within operational age targets.

### Sync freshness

Connected devices should converge within a defined operational target.

Exact numeric SLOs belong in the operational baseline and must be based on measured workload, provider constraints, and business requirements rather than invented universal numbers.

---

## 48. Error Budgets

An SLO error budget is consumed by user-impacting reliability failure.

When the budget is exhausted or materially threatened:

- risky feature releases may pause;
- reliability work takes priority;
- dependency incidents trigger focused remediation;
- non-critical capacity may be reduced to preserve POS paths.

Reliability policy MUST distinguish between availability loss and integrity/security loss; an integrity incident is not merely an SLO miss.

---

## 49. Backup Architecture

Required backup classes:

```text
PostgreSQL logical/physical backup as appropriate
PITR/WAL where supported and required
object-storage versioning/backup where required
configuration/IaC repositories
critical operational metadata
```

Backups MUST be:

- access controlled;
- encrypted;
- monitored;
- independently restorable;
- retention-managed.

A successful backup job without successful restoration evidence is insufficient.

---

## 50. Recovery Objectives

The final production baseline MUST specify:

- RPO: maximum tolerable data-loss window for authoritative server data;
- RTO: maximum tolerable service-recovery duration.

These must be set against business requirements and deployment cost.

Do not claim zero RPO for unsynchronized device-local data.

Offline clients inherently create a distinction between local intent and server-committed state.

---

## 51. Restore Drill

At defined intervals, execute a restore exercise:

```text
backup source
   |
   v
isolated restore
   |
   v
migration/schema validation
   |
   v
integrity checks
   |
   v
application compatibility
   |
   v
critical transaction verification
   |
   v
measured recovery time
```

Record:

- restore duration;
- backup timestamp;
- data gap;
- application version;
- migration version;
- failed assumptions;
- corrective actions.

---

## 52. Disaster Recovery Modes

### Database failure

Fail over or restore PostgreSQL according to the chosen operational architecture.

### Complete infrastructure loss

Recreate infrastructure from IaC, restore database/object state, deploy the verified artifact, then reconcile external integration state.

### Region loss

Region-level disaster recovery is an explicitly staged capability. It must not be claimed until tested.

### Credential compromise

Prefer credential rotation/revocation and redeployment rather than restoring compromised infrastructure blindly.

---

## 53. External Dependency Failure

Deployment architecture MUST support:

```text
provider unavailable
        |
        +--> local authoritative transaction remains valid
        |
        +--> integration state becomes PENDING
        |
        +--> bounded retry/reconciliation
        |
        +--> exception on permanent failure
```

Provider failure MUST NOT force the system to mutate historical financial facts merely to make integration state appear successful.

---

## 54. Rollback Strategy

Rollback is defined at multiple layers:

```text
application rollback
configuration rollback
traffic rollback
migration recovery
infrastructure rollback
mobile rollback
```

An application rollback is safe only when the database schema remains compatible with the rolled-back application.

This is why expand-and-contract is required for many production changes.

---

## 55. Rollback Decision Tree

```text
Release degradation detected
        |
        v
Is data integrity affected?
   |                |
  yes               no
   |                |
freeze unsafe       assess error budget
writes              |
   |                v
preserve evidence   rollback/canary stop
   |
   v
repair/reconcile
```

A rollback MUST NOT erase audit evidence or hide the fact that a release executed.

---

## 56. Failed Migration Recovery

A failed migration requires:

1. stop unsafe application traffic if required;
2. determine whether migration partially committed;
3. inspect schema version;
4. preserve logs;
5. determine compatible application version;
6. execute documented recovery SQL/procedure;
7. verify constraints/indexes/data integrity;
8. only then resume service.

Never improvise destructive production SQL during an incident without controlled authorization and evidence preservation.

---

## 57. Release Freeze Conditions

Automatic or manual release freeze SHOULD trigger on:

- active SEV-1 incident;
- tenant isolation incident;
- suspected secret compromise;
- critical supply-chain compromise;
- database corruption;
- uncontrolled queue growth;
- payment integrity incident;
- EIS corruption affecting statutory evidence;
- failed restore validation.

Emergency security patches remain possible under emergency-release governance.

---

## 58. Production Access

Human production access MUST follow least privilege.

Administrative access should use:

- MFA;
- individual accounts;
- audited commands/actions;
- short-lived access where practical;
- just-in-time access for high privilege;
- separation of duties for destructive operations.

Shared production administrator accounts are prohibited.

---

## 59. Break-Glass Access

Break-glass access exists only for exceptional recovery situations.

It requires:

- explicit authorization;
- strong authentication;
- reason capture;
- time-bounded activation;
- enhanced telemetry;
- post-event review.

Break-glass must not silently bypass audit.

---

## 60. Production Administrative Network

Administrative infrastructure SHOULD be reachable through controlled access paths rather than broad public exposure.

Examples include:

- private network access;
- VPN;
- identity-aware access proxy;
- bastion/access gateway where necessary.

Direct public PostgreSQL administration is prohibited.

---

## 61. Kubernetes Option

If Kubernetes is adopted, production clusters MUST address at least:

- workload security contexts;
- least-privilege RBAC;
- secret management;
- cluster-level admission/policy controls;
- network segmentation;
- API/server exposure;
- component patching;
- image provenance;
- resource requests/limits;
- Pod disruption/availability strategy;
- audit logging;
- backup/recovery of stateful workloads.

These requirements align with current OWASP Kubernetes Top 10 risk categories, including insecure workloads, overly permissive authorization, secrets-management failures, missing policy enforcement, missing network segmentation, and exposed components.

Kubernetes must not be introduced merely to appear enterprise-grade.

---

## 62. Kubernetes Stateful Boundary

PostgreSQL SHOULD be externally managed or operated by a team with demonstrated stateful Kubernetes expertise.

A first deployment should avoid unnecessarily coupling database durability to cluster lifecycle.

---

## 63. Kubernetes Resource Governance

Every workload MUST have resource requests/limits appropriate to its workload class.

A runaway report job must not starve API transaction capacity.

Use:

```text
API pool
worker pool
bulk/report pool
```

with explicit capacity boundaries where platform supports them.

---

## 64. VM Deployment Option

A VM deployment may be the correct initial choice when:

- engineering team size is small;
- workload scale is moderate;
- managed database is available separately;
- Kubernetes operational overhead is not justified.

The same application health, security, backup, deployment, and observability contracts still apply.

---

## 65. Managed Container Deployment Option

Managed containers can provide:

- immutable image deployment;
- autoscaling;
- load balancing;
- secret integration;
- health checks;
- deployment rollout.

The application MUST not depend on proprietary platform APIs for business correctness.

---

## 66. Cloud Provider Neutrality

The initial document intentionally does not freeze AWS, GCP, Azure, Cloudflare, or another vendor as the mandatory provider.

The deployment architecture instead freezes capability requirements:

```text
secure compute
managed/operable PostgreSQL
object storage
secret management
network controls
observability
backup/restore
identity
artifact registry
```

Provider selection becomes an ADR after cost, latency, data residency, support, compliance, team skill, and availability requirements are evaluated.

---

## 67. Geographic Strategy

Initial deployment may be single-region.

The platform MUST nevertheless capture region/location metadata where required for:

- compliance;
- operational telemetry;
- data residency decisions;
- disaster recovery;
- future regional expansion.

Multi-region writes MUST NOT be introduced until transaction ownership and conflict semantics justify them.

---

## 68. Data Residency

Before production, establish:

- database region;
- object-storage region;
- telemetry region;
- backup region;
- support-access geography where relevant;
- provider data-processing location where relevant.

No unsupported compliance statement should be made merely because a cloud provider exposes a regional endpoint.

---

## 69. Tenant-Noisy-Neighbor Controls

Multi-tenant deployment must prevent one tenant from consuming unlimited shared resources.

Controls may include:

- per-tenant API quotas;
- report concurrency limits;
- export quotas;
- storage quotas;
- request budgets;
- batch size limits;
- background-job fairness;
- tenant-specific rate controls.

Quotas must not weaken authorization or allow a tenant to affect another tenant's data.

---

## 70. Cost Governance

Production infrastructure must be observable for cost drivers.

Monitor:

- database compute/storage;
- API instances;
- worker instances;
- object storage;
- network egress;
- telemetry ingestion;
- backup storage;
- third-party API usage.

Cost controls must not disable core financial correctness or backups silently.

---

## 71. Autoscaling

Autoscaling signals may include:

- CPU;
- memory;
- request concurrency;
- request latency;
- queue depth;
- worker backlog.

Autoscaling PostgreSQL writes is not a substitute for query/transaction engineering.

Autoscaling must respect provider/account/database limits.

---

## 72. Capacity Planning

Capacity planning must identify bottlenecks in this order:

```text
API CPU / memory
   |
   v
connection pools
   |
   v
PostgreSQL CPU / IOPS / locks
   |
   v
worker capacity
   |
   v
external-provider quotas
```

Synthetic peak assumptions must be labeled as assumptions.

Production capacity limits should eventually be derived from measured telemetry.

---

## 73. Load Shedding

Under severe resource pressure, preserve:

1. authentication/session safety;
2. POS transactional paths;
3. inventory/payment reconciliation;
4. sync processing;
5. high-priority integrations.

Shed or delay:

- bulk exports;
- expensive analytics;
- low-priority notifications;
- maintenance jobs.

Load shedding must be explicit and observable.

---

## 74. Deployment of Reporting Jobs

Reports and exports must execute asynchronously for costly workloads.

They MUST have:

- bounded date ranges;
- row/result limits;
- tenant authorization;
- job quotas;
- execution timeout;
- storage lifecycle;
- cancellation where practical.

A deployment cannot allow an expensive report to consume all database capacity needed for POS operations.

---

## 75. Observability Dependency

The application should remain functionally available during temporary telemetry backend outage where doing so does not compromise security or integrity.

Use bounded local buffering.

Do not:

- block every transaction indefinitely waiting for telemetry;
- store unbounded local logs;
- silently discard security events that are mandatory for incident response.

For critical security events, durable audit storage remains distinct from ordinary observability pipelines.

---

## 76. Time Synchronization

All server infrastructure requires reliable time synchronization.

The platform must distinguish:

- wall-clock timestamps;
- monotonic elapsed-time measurement;
- client timestamps;
- authoritative server timestamps.

Client time MUST NOT be treated as authoritative for security decisions.

---

## 77. Clock Skew Monitoring

Monitor clock skew on:

- API nodes;
- workers;
- database host where accessible;
- CI/release systems when relevant.

Important token/signature operations must use defined clock-skew tolerances rather than arbitrary behavior.

---

## 78. Deployment of Cryptographic Material

Cryptographic keys MUST have:

- owner;
- purpose;
- algorithm;
- storage location;
- rotation policy;
- activation date;
- retirement date;
- emergency revocation procedure.

MRA terminal-specific signing secrets must be isolated to the EIS integration boundary and never injected into unrelated workloads.

---

## 79. Feature Flag Governance

Feature flags MUST be:

- typed;
- scoped;
- auditable;
- default-safe;
- removable.

Security-sensitive features should default closed when configuration is unavailable.

Feature flags must not be used as a substitute for migrations or authorization.

---

## 80. Configuration Change Governance

Production configuration changes are deployments of behavior even when no code changes.

Each high-risk change should record:

```text
what changed
who changed it
when
previous value/version
new value/version
reason
rollback
```

Sensitive values themselves must not be logged.

---

## 81. Mobile Backend Compatibility

The API contract MUST support the oldest still-supported mobile protocol according to an explicit support matrix.

Example:

```text
server release       supported client versions
N                    N-2, N-1, N
N+1                  N-1, N, N+1
```

The exact window is a release-policy decision, but it must be explicit.

---

## 82. Protocol Deprecation

Before removing a client-facing protocol version:

- measure active clients;
- communicate migration requirements where relevant;
- release compatible replacement;
- block unsupported versions intentionally;
- provide controlled recovery behavior.

Never silently interpret old protocol messages as new semantics.

---

## 83. Security of Deployment Manifests

Deployment manifests can contain sensitive operational metadata.

They MUST be scanned for:

- passwords;
- API keys;
- certificates/private keys;
- privileged tokens;
- public exposure of internal endpoints;
- unrestricted security groups/network policies.

A manifest repository must be treated as production-adjacent infrastructure code.

---

## 84. Vulnerability Management

Before production deployment, evaluate:

- application dependencies;
- base images;
- OS packages;
- infrastructure components;
- managed-service exposure;
- Kubernetes components if used.

Current OWASP Kubernetes guidance specifically identifies vulnerable/misconfigured cluster components as a major risk category. 

Critical vulnerabilities require remediation or a formally approved exception.

---

## 85. Emergency Patching

Emergency patches follow:

```text
identify
 -> triage
 -> contain
 -> patch
 -> test
 -> deploy
 -> verify
 -> retrospective
```

The process may shorten normal review steps only to the minimum justified by severity; it does not remove auditability.

---

## 86. Dependency Update Deployment

Dependency updates should be grouped by risk.

Security-critical updates SHOULD have:

- dependency advisory analysis;
- targeted regression tests;
- integration tests;
- deployment to staging;
- monitored rollout.

Large dependency upgrades that affect Rust async runtime, database drivers, auth libraries, TLS, or serialization require additional compatibility review.

---

## 87. Production Smoke Tests

Immediately after deployment verify:

```text
health endpoint
authentication
authorization
tenant access
product read
sale path in test-safe context
inventory mutation path where permitted
queue processing
payment integration status path
MRA status/configuration path where applicable
observability
```

No real customer money or statutory production transaction should be generated by generic smoke tests unless specifically designed and authorized.

---

## 88. Synthetic Monitoring

Synthetic monitors SHOULD verify customer-critical paths using controlled non-production or explicitly safe production scenarios.

Production synthetic transactions must never create fake financial records that corrupt reporting, tax obligations, or merchant accounts.

---

## 89. Operational Runbook — Bad Release

```text
1. Detect regression.
2. Identify release/artifact digest.
3. Determine whether integrity is affected.
4. Stop rollout.
5. Preserve logs/traces/audit evidence.
6. Roll back application if schema-compatible.
7. If migration involved, execute migration recovery plan.
8. Verify critical transaction paths.
9. Monitor for delayed queue/integration effects.
10. Record incident and create regression tests.
```

---

## 90. Operational Runbook — Database Failure

```text
1. Confirm database failure.
2. Protect application from unsafe writes.
3. Determine failover/restore path.
4. Verify recovery target.
5. Restore/fail over.
6. Validate schema/migrations.
7. Validate critical invariants.
8. Reconcile outbox/integration states.
9. Resume traffic progressively.
10. Verify backups and root cause.
```

---

## 91. Operational Runbook — Credential Compromise

```text
1. Identify credential and scope.
2. Revoke immediately.
3. Rotate replacement credential.
4. Restrict affected identity.
5. Preserve audit evidence.
6. Search for unauthorized use.
7. Redeploy affected workloads.
8. Verify old credential is invalid.
9. Review neighboring credentials.
10. Document corrective action.
```

---

## 92. Operational Runbook — Queue Explosion

```text
1. Identify queue class.
2. Determine oldest pending age.
3. Determine retry/error distribution.
4. Protect critical queues from bulk work.
5. Stop poison-message loops.
6. Repair provider/dependency.
7. Drain at bounded concurrency.
8. Review dead-letter items.
9. Verify downstream reconciliation.
```

---

## 93. Operational Runbook — Security Incident During Deployment

```text
1. Freeze rollout.
2. Preserve current and candidate artifacts.
3. Revoke compromised release/deployment credentials.
4. Identify affected versions.
5. Contain exposed path.
6. Rotate secrets if required.
7. Verify deployed binaries/images by digest.
8. Redeploy trusted artifact.
9. Run security regression suite.
10. Complete incident review.
```

---

## 94. Disaster Recovery Reconciliation

After infrastructure recovery, do not assume external state equals local state.

Reconcile:

```text
Sitolo payment intents
      ↕
provider state

Sitolo EIS submissions
      ↕
MRA evidence/state

outbox/jobs
      ↕
side effects

sync checkpoints
      ↕
client acknowledgements
```

The reconciliation process must be deterministic and auditable.

---

## 95. Production Data Migration Rules

Never use ad hoc export/import to “fix” production data where a migration can provide controlled semantics.

Financial corrections MUST use domain correction workflows rather than direct row mutation.

Data migrations require:

- idempotency;
- bounded batches;
- progress markers;
- observability;
- retry/restart safety;
- rollback/recovery plan.

---

## 96. Backup Encryption Keys

Backup encryption keys must have a recovery plan independent enough that a single compromised application credential cannot destroy both production and backups.

Where possible, backup deletion/retention privileges should be separate from application runtime identities.

---

## 97. Backup Deletion Protection

Production backups should use provider mechanisms such as retention locks/versioning/immutability where appropriate.

The exact control depends on the selected storage platform and the required recovery threat model.

---

## 98. Infrastructure Drift

Production infrastructure MUST be checked for configuration drift.

Material drift includes:

- firewall changes;
- public exposure;
- IAM privilege changes;
- storage policy changes;
- database configuration changes;
- workload image changes;
- encryption setting changes.

Unexpected drift should trigger investigation.

---

## 99. Deployment Audit Evidence

Retain enough evidence to reconstruct:

```text
what artifact ran
where it ran
when it ran
who/what deployed it
which migration ran
which configuration version was active
what release health looked like
what rollback occurred
```

Deployment evidence should be linked to incident and release records.

---

## 100. Production Release Record

Each release record SHOULD contain:

```yaml
release_id:
source_revision:
artifact_digest:
build_pipeline:
build_time:
compiler_version:
database_migration:
configuration_version:
security_scan_reference:
sbom_reference:
provenance_reference:
staging_validation:
approval:
production_deployment_time:
rollback_status:
```

---

## 101. Compliance Boundary

Deployment controls do not by themselves establish regulatory compliance.

Examples:

- secure storage does not itself prove tax compliance;
- a certified cloud does not itself certify Sitolo;
- successful API calls do not constitute MRA approval;
- encrypted backups do not prove retention compliance.

Compliance claims require appropriate legal/regulatory evidence.

---

## 102. MRA Production Deployment Gate

A production deployment that changes the EIS integration MUST verify, as applicable:

- current MRA API contract;
- terminal identity/configuration compatibility;
- credentials/secrets;
- configuration versioning;
- offline signing behavior;
- submission mapping;
- error classification;
- test/certification status;
- operational runbook updates.

Sitolo MUST NOT market an implementation as MRA-certified merely because staging calls succeed.

---

## 103. Payment Production Gate

A production payment deployment MUST verify:

- provider credentials;
- amount/currency mapping;
- provider account mapping;
- idempotency behavior;
- webhook verification;
- reconciliation;
- refund/chargeback behavior;
- provider outage behavior;
- support runbooks.

---

## 104. Security Deployment Gate

The release is blocked when any of the following is true:

```text
critical vulnerability
critical tenant-isolation test failure
secret exposure
unsafe admin path
unverified artifact provenance
broken authz regression
payment duplicate-effect regression
offline replay regression
backup/restore failure for release-critical change
```

Emergency exceptions must be explicit, time-bounded, owned, and reviewed.

---

## 105. Performance Deployment Gate

Performance regressions should be evaluated against workload-specific baselines.

Measure at minimum:

- p50/p95/p99 request latency;
- throughput;
- DB CPU;
- lock wait;
- connection utilization;
- queue age;
- worker throughput;
- error rate.

Do not set arbitrary “enterprise” performance claims without workload evidence.

---

## 106. Resource Exhaustion Protection

Production must place bounds on:

- request body size;
- file size;
- report date range;
- export rows;
- sync batch size;
- command size;
- queue retry rate;
- worker concurrency;
- database connections;
- memory-intensive operations.

Resource limits are security controls as well as reliability controls.

---

## 107. Logging Deployment Requirements

All production services MUST produce structured logs with:

- timestamp;
- service/component;
- severity;
- environment;
- release identifier;
- trace/span correlation where available;
- safe operation identifiers.

Do not log:

- access tokens;
- passwords;
- API keys;
- provider secrets;
- MRA terminal secret material;
- complete payment credentials;
- full sensitive payloads without a documented need.

---

## 108. Log Retention

Retention must balance:

- operational debugging;
- audit requirements;
- security investigation;
- privacy;
- storage cost.

Retention policies must be explicit and environment-specific.

---

## 109. Incident Command During Releases

For high-risk production changes, assign:

- release owner;
- operations owner;
- security contact;
- database owner where needed;
- business/product decision-maker where customer impact is possible.

The release owner must have authority to stop rollout.

---

## 110. Change Classification

Changes should be classified:

```text
LOW
routine compatible application change

MEDIUM
schema/index/provider/infra change with bounded impact

HIGH
auth, tenant isolation, payment, EIS, major database, deployment identity, networking

CRITICAL
security incident patch, data-recovery operation, tenant isolation incident remediation
```

The higher the category, the stronger the release evidence required.

---

## 111. Architecture Decision Requirements

An ADR is required when deployment changes materially affect:

- hosting model;
- data-region strategy;
- multi-region writes;
- database topology;
- identity/deployment trust;
- secret-management architecture;
- Kubernetes adoption;
- disaster-recovery targets;
- public network exposure;
- payment/EIS trust boundary;
- immutable artifact model.

---

## 112. Deployment Definition of Done

A deployment system is complete only when:

```text
[ ] environment separation exists
[ ] immutable artifacts exist
[ ] reproducible builds exist
[ ] dependency locks are enforced
[ ] artifact scanning exists
[ ] provenance exists
[ ] deployment identity is constrained
[ ] secrets are externalized
[ ] runtime least privilege exists
[ ] health checks exist
[ ] graceful shutdown exists
[ ] rolling compatibility exists
[ ] migration discipline exists
[ ] backups exist
[ ] restore drills exist
[ ] observability exists
[ ] release gates exist
[ ] rollback is tested
[ ] incident runbooks exist
[ ] production access is audited
[ ] external dependency failure is handled
[ ] tenant isolation remains intact
[ ] payment/EIS boundaries remain intact
```

---

## 113. Minimal Production Topology

The initial recommended production footprint is:

```text
                    Internet
                       |
                 DNS / Edge / WAF
                       |
                Load Balancer
                       |
             +---------+---------+
             |                   |
          Rust API           Rust API
             |                   |
             +---------+---------+
                       |
                PostgreSQL
                       |
            +----------+----------+
            |                     |
       Worker Pool          Report/Bulk Pool
            |
      +-----+------+---------+
      |            |         |
   Payments       MRA      Other
   Adapter        EIS      adapters

       + Object Storage
       + Secret Manager
       + Telemetry
       + Optional Redis
```

This is deliberately simpler than a microservice mesh.

---

## 114. Scale-Out Path

The platform may evolve:

```text
modular monolith
      |
      v
separate worker pools
      |
      v
read replicas / analytics
      |
      v
isolated integration workers
      |
      v
selective service extraction
```

Extraction should occur only when a real scaling, isolation, deployment, or ownership boundary exists.

---

## 115. Multi-Region Future State

A future multi-region design must explicitly solve:

- tenant/home-region ownership;
- write routing;
- session/device behavior;
- offline sync convergence;
- payment integration locality;
- MRA terminal locality;
- cross-region audit;
- failover sequencing;
- regulatory residency;
- split-brain prevention.

Do not deploy active-active financial writes simply because the cloud platform supports them.

---

## 116. Tenant Home Region Option

If regionalization occurs, each tenant may eventually receive a logical home region:

```text
tenant -> home region -> authoritative writes
```

Cross-region reads can be optimized independently.

This model is intentionally future architecture, not an MVP requirement.

---

## 117. Client Release Security

Mobile and desktop clients must receive only public/runtime-safe configuration.

Client packages MUST be scanned for:

- secrets;
- debug endpoints;
- test provider URLs;
- development certificates;
- unsafe logging;
- accidental source maps where inappropriate.

Backend authorization remains authoritative even when clients contain feature flags.

---

## 118. Release Signing

Production client and server artifacts SHOULD be digitally signed where the ecosystem supports it.

Verification is required before distribution.

Signing keys must be separated from ordinary build credentials.

Compromise of build infrastructure must not automatically imply unrestricted signing authority.

---

## 119. Supply-Chain Threat Model

Deployment must defend against:

```text
malicious dependency
compromised build action
stolen CI token
modified base image
poisoned artifact registry
unsigned release
workflow injection
malicious infrastructure change
secret exfiltration
```

Controls include:

- lockfiles;
- pinned actions;
- provenance;
- artifact digest verification;
- least-privileged CI;
- secret isolation;
- SBOM;
- scanning;
- protected release paths.

NIST SSDF provides the development-process baseline; the deployment implementation should integrate its secure-build, protected-environment, vulnerability-management, and release-evidence principles. 

---

## 120. SBOM Requirements

Each production release SHOULD generate an SBOM covering:

- direct dependencies;
- transitive dependencies;
- OS packages;
- runtime artifacts where applicable.

The SBOM is stored with release evidence and associated with the artifact digest.

---

## 121. Provenance Verification

The deployment platform should verify that a production artifact:

1. originates from the expected repository;
2. was built by an approved builder/workflow;
3. corresponds to the expected source revision;
4. was not unexpectedly rebuilt after approval;
5. matches the release digest.

A provenance statement that cannot be independently verified is not sufficient evidence.

---

## 122. Deployment Policy as Code

Where possible, enforce deployment policy through automation:

```text
no signed artifact -> deny
no provenance -> deny
critical scan failure -> deny
unapproved environment -> deny
unsafe migration -> deny
missing approval -> deny
```

Manual approval remains useful for high-risk changes but should not replace machine checks.

---

## 123. Breakage Containment

Every deployment should have bounded blast radius.

Examples:

- canary percentage;
- isolated worker pools;
- feature flags;
- rate limits;
- queue priority;
- tenant-scoped rollout where appropriate.

The objective is to prevent one defect from becoming a platform-wide failure.

---

## 124. Tenant-Scoped Rollout

For risky tenant-facing features, support rollout by:

```text
tenant cohort
business type
region
plan/entitlement
internal test tenant
```

Tenant-scoped rollout MUST NOT bypass tenant authorization or reveal feature state across tenants.

---

## 125. Release Verification Evidence

For every production release retain:

```text
source commit
artifact digest
SBOM
provenance
security scan results
test results
migration result
deployment logs
health verification
rollback outcome if any
```

This evidence is part of the release artifact lifecycle.

---

## 126. Definition of Production-Ready

Sitolo is not production-ready because the binary starts.

Production-ready means:

```text
SOURCE
  -> reproducibly built
  -> scanned
  -> attested
  -> immutable

INFRASTRUCTURE
  -> version controlled
  -> least privilege
  -> monitored
  -> backed up

DATABASE
  -> migrated safely
  -> recoverable
  -> integrity checked

RUNTIME
  -> healthy
  -> observable
  -> horizontally replaceable
  -> gracefully degradable

SECURITY
  -> secrets protected
  -> identity controlled
  -> tenant isolation verified
  -> deployment trust verified

OPERATIONS
  -> rollback tested
  -> restore tested
  -> runbooks owned
  -> incidents observable

BUSINESS
  -> payment semantics preserved
  -> tax/EIS evidence preserved
  -> offline continuity preserved
```

---

## 127. Final Deployment Contract

The binding rule is:

> **A production deployment is an auditable transition between known system states. The deployed artifact must be identifiable, the infrastructure must be reproducible, secrets must remain outside artifacts, database changes must be compatible and recoverable, critical workloads must remain isolated from bulk work, external side effects must be durable and idempotent, observability must make failures diagnosable, and rollback/recovery must preserve historical truth rather than rewrite it.**

Enterprise readiness is therefore demonstrated by the ability to answer, after an incident:

```text
WHAT ran?
WHERE did it run?
WHO deployed it?
WHAT changed?
WHAT data changed?
WHAT external side effects occurred?
WHAT failed?
WHAT remained authoritative?
HOW was service recovered?
HOW was integrity verified?
WHAT regression prevents recurrence?
```

---

## 128. Deployment Exit Checklist

```text
ARCHITECTURE
[ ] runtime topology approved
[ ] hosting target selected or explicitly open
[ ] Kubernetes decision recorded if relevant
[ ] region strategy recorded

SECURITY
[ ] production identities isolated
[ ] secret manager configured
[ ] deployment identity restricted
[ ] network exposure reviewed
[ ] supply-chain controls active
[ ] artifact signing/provenance verified

APPLICATION
[ ] graceful shutdown
[ ] readiness/liveness
[ ] bounded resources
[ ] backward-compatible API deployment
[ ] feature flags governed

DATABASE
[ ] backup
[ ] PITR where required
[ ] restore drill
[ ] migration process
[ ] least-privileged role
[ ] RLS/runtime privilege validation

INTEGRATIONS
[ ] payment production gate
[ ] MRA EIS gate
[ ] webhook/callback security
[ ] external failure handling

OBSERVABILITY
[ ] logs
[ ] metrics
[ ] traces
[ ] alerts
[ ] deployment correlation

OPERATIONS
[ ] rollback runbook
[ ] database recovery runbook
[ ] credential compromise runbook
[ ] queue incident runbook
[ ] incident ownership

RELEASE
[ ] immutable artifact
[ ] test evidence
[ ] SBOM
[ ] provenance
[ ] approval
[ ] deployment record
[ ] smoke validation
```

---

## 129. External References

- Kubernetes documentation — production environment guidance.
- OWASP Kubernetes Top 10 (2025).
- NIST SP 800-218 — Secure Software Development Framework 1.1.
- NIST SP 800-218 Rev. 1 initial public draft — SSDF 1.2 status/reference.
- SLSA specification documentation — current version 1.2 and provenance model.
- Sitolo `system_architecture_design.md`.
- Sitolo `security_architecture_design.md`.
- Sitolo `security_implementation_spec.md`.
- Sitolo `database_design.md`.
- Sitolo `testing_strategy.md`.
- Sitolo `observability_spec.md`.

---

## 130. Document Governance

Changes to this document require architecture review when they alter:

- authoritative state;
- production trust boundaries;
- deployment identity;
- tenant isolation;
- database recovery semantics;
- secret-management model;
- external integration guarantees;
- release/rollback safety;
- disaster-recovery objectives;
- hosting topology in a way that affects system invariants.

Routine parameter tuning does not necessarily require a new ADR, but the change must remain auditable.

---

**Document end — Sitolo Deployment Specification, Phase 0 / File 12 of 16.**
