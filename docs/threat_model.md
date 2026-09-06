# Sitolo — Enterprise Threat Model

**Document:** `threat_model.md`  
**Phase:** 0 — Architecture / Contract Freeze  
**File:** 10 of 16  
**Status:** Design baseline / security governance artifact  
**Date:** 2026-09-04  
**Primary system:** Sitolo Business Operating System for African SMEs  
**Primary deployment market:** Malawi-first, controlled African regionalization  
**Backend baseline:** Rust / Axum / Tokio / SQLx  
**Authoritative server database:** PostgreSQL 18  
**Primary mobile client:** Flutter / SQLite offline store  
**Desktop client:** Tauri  
**Threat-modeling posture:** continuous, adversary-driven, evidence-based

---

## 1. Executive Security Position

Sitolo is a transaction-processing platform, not merely a CRUD application. Its most dangerous failures are therefore not limited to classic confidentiality vulnerabilities. The highest-impact failures are situations in which an attacker, compromised employee, compromised device, faulty integration, or software defect can cause Sitolo to accept an operation that is outside the authority or economic reality of the merchant.

The security objective is consequently stronger than “protect the API.” Sitolo must preserve:

```text
TENANT ISOLATION
        +
AUTHENTICATION / SESSION INTEGRITY
        +
AUTHORIZATION / SCOPE INTEGRITY
        +
FINANCIAL INTEGRITY
        +
INVENTORY INTEGRITY
        +
OFFLINE COMMAND INTEGRITY
        +
PAYMENT / TAX INTEGRATION TRUST
        +
AUDITABILITY
        +
AVAILABILITY / RECOVERABILITY
        +
SUPPLY-CHAIN INTEGRITY
```

A security failure is considered severe when it can make the system believe something that did not legitimately happen, allow one tenant to operate on another tenant's data, allow an unauthorized user/device to perform a sensitive mutation, duplicate a financial side effect, bypass a business invariant, or destroy the evidence needed to reconstruct what occurred.

This threat model follows the OWASP threat-modeling lifecycle of understanding the system, identifying what can go wrong, choosing mitigations, and validating that the model remains adequate. OWASP explicitly treats threat modeling as an iterative activity that should start early in the SDLC and evolve with the system. ([OWASP Threat Modeling Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Threat_Modeling_Cheat_Sheet.html))

NIST CSF 2.0 is used as the risk-governance frame rather than as a replacement for technical threat analysis. CSF 2.0 provides outcomes for managing cybersecurity risk but does not prescribe a single implementation method. ([NIST CSF 2.0](https://www.nist.gov/publications/nist-cybersecurity-framework-csf-20))

OWASP ASVS 5.0.0 is the principal application-security verification baseline. The current stable ASVS release is 5.0.0; requirement identifiers should be version-qualified in evidence records because identifiers can change between major versions. ([OWASP ASVS](https://owasp.org/www-project-application-security-verification-standard/))

---

# 2. Purpose and Scope

## 2.1 Purpose

This document establishes the adversarial model that implementation, testing, operations, and architectural review must use when determining whether a Sitolo feature is safe to release.

It defines:

- assets worth protecting;
- trust boundaries;
- principals and attacker classes;
- system entry points;
- abuse cases;
- threat scenarios;
- preconditions and attack paths;
- security invariants;
- mitigations;
- detection signals;
- residual risks;
- security tests;
- recovery expectations;
- release-gate implications;
- ownership expectations;
- triggers for re-modeling.

## 2.2 In scope

The model covers:

1. mobile Flutter applications;
2. desktop Tauri application;
3. web/API clients where present;
4. public API edge;
5. authentication and session infrastructure;
6. identity, tenant, organization, branch, location, register and device relationships;
7. authorization and policy enforcement;
8. PostgreSQL authoritative state;
9. local SQLite offline state;
10. synchronization protocol;
11. sales and POS;
12. inventory and stock movements;
13. procurement and receiving;
14. cash and registers;
15. payment-provider integrations;
16. MRA EIS integration;
17. tax evidence and fiscalization artifacts;
18. workers and asynchronous jobs;
19. outbox/inbox/idempotency records;
20. reporting, exports and support tools;
21. object storage and file workflows;
22. notifications;
23. billing and entitlements;
24. administration and support access;
25. CI/CD and software supply chain;
26. cloud/network/database operational dependencies;
27. monitoring, audit and incident response;
28. backup, restore and disaster recovery.

## 2.3 Out of scope

The following are not treated as built-in Sitolo capabilities merely because they are adjacent to the platform:

- banking or custodial custody of customer funds;
- arbitrary lending decisions;
- a general-purpose accounting ledger beyond the documented merchant finance domain;
- unrestricted workflow scripting;
- a general remote shell;
- an arbitrary plugin execution environment;
- unrestricted arbitrary SQL for tenants;
- machine-learning decisions that become financial authority;
- unsupported regulatory certifications.

Any future inclusion of these capabilities requires a material threat-model update.

---

# 3. System Security Objectives

## 3.1 Confidentiality

Protect:

- tenant business data;
- customer personal data;
- supplier data;
- merchant credentials;
- API credentials;
- signing keys;
- MRA terminal secrets;
- payment integration secrets;
- device credentials;
- recovery material;
- security telemetry where sensitive;
- internal support and administrative information.

## 3.2 Integrity

Preserve the truth of:

- who performed an operation;
- which tenant and scope it belonged to;
- which device originated it;
- what business command was submitted;
- whether it was accepted exactly once;
- what inventory movement resulted;
- what financial fact resulted;
- what payment state resulted;
- what tax/EIS state resulted;
- what evidence was generated;
- which version/configuration governed the operation.

## 3.3 Availability

Maintain the ability to perform permitted business operations despite:

- intermittent mobile connectivity;
- provider outages;
- worker crashes;
- application restarts;
- transient database failures;
- partial infrastructure degradation;
- client process death;
- stale local state.

Availability must not override integrity. The system may reject or hold sensitive operations rather than silently bypassing security controls.

## 3.4 Accountability

Security-sensitive actions must produce sufficient evidence to answer:

```text
WHO?
WHAT?
WHEN?
WHERE / WHICH TENANT?
WHICH DEVICE?
WHICH RESOURCE?
WHICH AUTHORIZATION?
WHICH VERSION / CONFIG?
WHAT RESULT?
WHAT EXTERNAL SIDE EFFECT?
```

---

# 4. Architectural Security Principles

The following principles are mandatory constraints on implementation.

## 4.1 Server authority

Clients preserve operational continuity but do not determine server truth.

## 4.2 Explicit tenant context

Tenant identity is derived from authenticated membership/session/device state and never accepted as a security assertion merely because a client supplied `tenant_id`.

## 4.3 Scope is multi-dimensional

Authorization may depend on:

```text
principal
membership
role
permission
organization
branch
location
warehouse
register
resource ownership
resource state
amount
approval state
risk level
```

## 4.4 Financial immutability

Historical finalized financial facts are not overwritten to conceal errors. Corrections use explicit compensating operations.

## 4.5 External systems are trust boundaries

Provider responses are evidence, not permission to violate Sitolo invariants.

## 4.6 Offline is bounded authority

Offline capability means the client can preserve authorized operational intent within an explicitly defined capability profile. It does not permanently expand the user's authority or allow the client to bypass server validation at synchronization time.

## 4.7 Idempotency is a security invariant

Any operation that may be retried after an ambiguous outcome must have stable identity and duplicate-resistant side-effect semantics.

## 4.8 Database is defense in depth

Application authorization does not replace database constraints, foreign keys, unique keys, transaction semantics, or correctly designed row-level controls.

## 4.9 Security controls fail closed

When critical evidence is unavailable—tenant context, device status, authorization decision, idempotency state, signature validation—the system must not guess its way into a sensitive mutation.

## 4.10 No security-by-client-only design

UI hiding, local validation, disabled buttons and obfuscated client requests are never treated as authorization controls.

---

# 5. Reference Architecture and Trust Boundaries

The simplified system is:

```text
                    INTERNET / UNTRUSTED NETWORK
                              |
             +----------------+----------------+
             |                                 |
          Mobile                           Desktop/Web
          Flutter                           Tauri/Browser
             |                                 |
             +----------------+----------------+
                              |
                              v
                     API / EDGE LAYER
                              |
                     AUTHN / RATE LIMIT
                              |
                              v
                    RUST APPLICATION CORE
             +----------------+----------------+
             |                |                |
          AuthZ/Scope      Domain          Integrations
             |                |                |
             |       +--------+--------+       |
             |       |        |        |       |
             |     Sales   Inventory Payments EIS
             |                          |       |
             +--------------------------+-------+
                                        |
                          +-------------+-------------+
                          |                           |
                    PostgreSQL                   Workers/Outbox
                          |                           |
                    Authoritative                    |
                       State                    External Providers

Offline boundary:
Flutter/Tauri -> SQLite/local evidence -> Sync protocol -> Server validation

External trust boundaries:
Payment providers / MRA / notification providers / object storage / identity provider

Supply-chain boundaries:
Developer -> Git -> CI runner -> build tools -> dependencies -> artifact registry -> deployment
```

## 5.1 TB-01: Untrusted client boundary

A client may be:

- modified;
- rooted/jailbroken;
- inspected;
- replayed;
- scripted;
- running an old version;
- offline;
- disconnected;
- holding stale authorization state;
- physically stolen.

Anything presented from this boundary must be treated as untrusted until server-side controls establish trust.

## 5.2 TB-02: Authentication boundary

Credentials, MFA and session artifacts cross from authentication infrastructure into Sitolo's application security context. Token validation, issuer/audience, expiry, signing keys and revocation semantics must be explicit.

## 5.3 TB-03: Tenant authorization boundary

This separates one merchant's authority from another merchant's authority and is one of the highest-impact boundaries in the platform.

## 5.4 TB-04: Domain authority boundary

Transport-level DTOs must not become business authority. Commands cross into domain services where state transitions and invariants are evaluated.

## 5.5 TB-05: Database authority boundary

PostgreSQL holds authoritative server-side business state. Database compromise, privilege escalation, SQL injection, RLS misconfiguration or unsafe repository behavior can cross the entire application's trust model.

## 5.6 TB-06: Offline synchronization boundary

The local device is allowed to hold commands and local state snapshots. Synchronization turns local intent into a server-side decision.

## 5.7 TB-07: Payment-provider boundary

Provider availability, authentication, signature semantics, transaction state and eventual consistency are outside Sitolo's direct control.

## 5.8 TB-08: MRA EIS boundary

MRA EIS is an external fiscal authority. Terminal credentials, terminal identity, configuration and tax evidence must be isolated from unrelated application code.

## 5.9 TB-09: Administrative/support boundary

Privileged support capabilities can access data across ordinary tenant boundaries. This is a separate high-assurance security domain and must use JIT/SoD/MFA/audit controls.

## 5.10 TB-10: CI/CD boundary

Source repositories, pull requests, dependencies, build runners, deployment identities and artifact registries are security boundaries. Compromise here can produce trusted malicious software.

---

# 6. Assets and Security Impact

| ID | Asset | Primary property | Impact if compromised |
|---|---|---|---|
| A-01 | Tenant business data | C/I | Very high |
| A-02 | User identity | C/I | Very high |
| A-03 | Sessions/tokens | C/I | Critical |
| A-04 | Device credentials | C/I | High |
| A-05 | Tenant membership/roles | I | Critical |
| A-06 | Product/catalogue | I | Medium/High |
| A-07 | Inventory ledger | I | Critical |
| A-08 | Sales/finalized invoices | I/A | Critical |
| A-09 | Cash/register history | I | Critical |
| A-10 | Payment state | I | Critical |
| A-11 | Payment secrets | C/I | Critical |
| A-12 | MRA credentials/secrets | C/I | Critical |
| A-13 | Tax/EIS evidence | I/C | Critical |
| A-14 | Offline command journal | I | Critical |
| A-15 | Idempotency records | I | Critical |
| A-16 | Audit trail | I | Critical |
| A-17 | Support/JIT identities | C/I | Critical |
| A-18 | Database credentials | C/I | Critical |
| A-19 | CI deployment identity | I | Critical |
| A-20 | Build artifacts | I | Critical |
| A-21 | Backups | C/I/A | Critical |
| A-22 | Reports/exports | C | High |
| A-23 | Object storage | C/I/A | High |
| A-24 | Notifications | I/C | Medium/High |
| A-25 | Billing/entitlements | I/A | High |

Impact ratings are deliberately business-oriented. A technically small bug that permits a second execution of a refund may have a higher business impact than a conventional low-severity information-disclosure bug.

---

# 7. Principals and Threat Actors

## 7.1 Anonymous internet attacker

Capabilities:

- send arbitrary HTTP requests;
- enumerate public endpoints;
- attempt rate-limit exhaustion;
- attempt credential attacks;
- exploit application/API defects;
- manipulate file and URL inputs.

## 7.2 Authenticated low-privilege merchant user

Capabilities are legitimate but intentionally bounded. The attacker attempts:

- vertical privilege escalation;
- cross-branch access;
- cross-object manipulation;
- abuse of discounts/refunds;
- reporting/data exfiltration;
- token/device abuse.

## 7.3 Malicious tenant administrator

The tenant admin may legitimately control many resources within one tenant but must not escape tenant scope or use support-only capabilities.

## 7.4 Malicious employee / insider

Can exploit legitimate workflows such as:

- refunds;
- discounts;
- stock adjustments;
- cash movements;
- device registration;
- report exports;
- price overrides.

## 7.5 Compromised endpoint

The attacker has control over a user's device or application runtime and can alter local SQLite state, replay commands, inspect local data, manipulate network traffic through a user-controlled proxy, or use stolen credentials.

## 7.6 Compromised external provider

A provider may return malformed, stale, fraudulent, duplicated, unavailable or unexpected data. This model does not assume providers are malicious by default, but explicitly assumes external responses are untrusted inputs.

## 7.7 Supply-chain attacker

Attacks:

- dependency packages;
- CI configuration;
- build runners;
- actions/plugins;
- artifact registries;
- release tooling;
- signing credentials.

## 7.8 Privileged platform operator

Has high access but remains a security risk. Abuse or compromise could cross normal tenant isolation controls. Access must therefore be stronger than ordinary merchant access.

## 7.9 Automated bot/scanner

Attempts enumeration, credential attacks, endpoint discovery, API abuse, report/export exhaustion, webhook abuse, or vulnerable dependency exploitation.

---

# 8. Attacker Capability Model

Threat analysis assumes attackers may:

1. forge arbitrary client fields;
2. modify HTTP headers;
3. call undocumented endpoints directly;
4. ignore the official UI;
5. replay old requests;
6. reorder requests;
7. submit duplicates;
8. hold requests and deliver them later;
9. alter local SQLite state;
10. change client clocks;
11. run multiple application instances;
12. retain old application versions;
13. use stolen sessions;
14. use a revoked device credential if revocation is not enforced;
15. inspect application binaries;
16. scrape reports and exports;
17. submit malformed JSON and oversized payloads;
18. induce timeouts after server commit;
19. cause concurrent operations;
20. exploit race conditions;
21. exploit eventual consistency;
22. attempt tenant/branch ID substitution;
23. exploit database/API parser differences;
24. supply SSRF payloads;
25. abuse file uploads;
26. exploit CI pull-request execution;
27. introduce malicious dependencies;
28. induce provider callbacks or duplicates;
29. intentionally corrupt local cache or command history;
30. exploit operator mistakes.

No security control may depend on an attacker voluntarily using the official client behavior.

---

# 9. Abuse-Case Catalog

## UC-01 Cross-tenant data access

Attacker changes an object ID, tenant ID, branch ID or nested resource reference to obtain another tenant's data.

**Primary controls:** trusted tenant context, object authorization, repository scoping, database constraints/RLS where selected, negative tests.

## UC-02 Cross-tenant mutation

Attacker changes identifiers in a write command to modify another tenant's records.

**Severity:** Critical.

## UC-03 Cross-branch privilege escalation

A user permitted for Branch A submits a command for Branch B.

## UC-04 Role forgery

Client submits `role=admin`, altered claims, or privileged route metadata.

## UC-05 Approval forgery

Client fabricates manager approval or alters approval state.

## UC-06 Refund overrun

Attacker replays or duplicates refunds so cumulative refunds exceed refundable value.

## UC-07 Discount abuse

Attacker manipulates price/discount inputs to obtain unauthorized economic benefit.

## UC-08 Stock inflation

Attacker modifies local stock or adjustment commands to increase available inventory.

## UC-09 Stock double-spend

Concurrent devices sell the same scarce stock or exploit stale local state.

## UC-10 Cash manipulation

Attacker modifies cash-in/out records, opening float or closing count to conceal theft.

## UC-11 Payment spoofing

Attacker sends a fake provider callback or manipulates client-side payment state.

## UC-12 Payment replay

Same provider event or payment result produces more than one financial effect.

## UC-13 Payment amount mismatch

A valid provider transaction is applied to the wrong amount, currency, tenant, sale or merchant account.

## UC-14 Payment ambiguity exploitation

An attacker deliberately triggers timeout conditions and attempts a second payment or second fulfilment path.

## UC-15 Offline replay

Captured offline commands are resubmitted multiple times.

## UC-16 Offline privilege extension

An attacker uses stale offline capabilities after role/device revocation.

## UC-17 Checkpoint rollback

A compromised client manipulates synchronization checkpoints so previously accepted events are replayed or later events are skipped.

## UC-18 EIS forgery

Attacker fabricates tax acceptance or modifies tax state based on fake external evidence.

## UC-19 EIS replay

Same fiscal transaction is submitted repeatedly or a duplicate is interpreted as a new fiscal event.

## UC-20 MRA credential theft

MRA terminal secrets are extracted from source, logs, mobile bundles or general application memory/storage.

## UC-21 Configuration downgrade

Stale or malicious configuration is used after the authoritative MRA configuration changes.

## UC-22 Report exfiltration

User with ordinary operational access exports data from another branch/tenant or requests unbounded sensitive reports.

## UC-23 Support overreach

Support engineer uses global administrative access without JIT justification or accesses unrelated tenants.

## UC-24 Lost device abuse

Attacker acquires an enrolled device and uses cached authority or locally stored credentials.

## UC-25 Credential stuffing

Compromised credentials are tested against Sitolo accounts.

## UC-26 Session fixation/theft

Attacker reuses a session after credential reset/revocation/device loss.

## UC-27 API resource exhaustion

Attacker causes expensive reports, searches, imports, synchronization batches or file processing.

## UC-28 SSRF

Attacker causes Sitolo to fetch private infrastructure, cloud metadata or internal administration endpoints.

## UC-29 File parser abuse

Malicious archive, document, image or polyglot file attacks processing components.

## UC-30 Notification abuse

Attacker causes message floods, phishing-like content or unauthorized recipient changes.

## UC-31 CI command injection

Untrusted pull-request content reaches privileged CI commands.

## UC-32 Dependency substitution

A malicious dependency or compromised build artifact enters production.

## UC-33 Secret leakage

Secrets appear in Git, CI logs, crash reports, telemetry, bundles, images or database dumps.

## UC-34 Audit suppression

Attacker or operator modifies/removes evidence after a suspicious operation.

## UC-35 Backup compromise

An attacker obtains database backups containing tenant/business secrets.

## UC-36 Denial of service with economic impact

Attacker exhausts worker/database/API capacity, preventing merchants from selling or reconciling.

---

# 10. STRIDE-Oriented Threat Enumeration

STRIDE is used as a threat-identification lens, but business-domain abuse cases are treated as first-class threats because generic protocol threats do not capture financial and inventory failures adequately.

## 10.1 Spoofing

Examples:

- forged session;
- stolen refresh token;
- forged device identity;
- impersonated provider callback;
- fake administrator;
- fabricated offline origin.

Primary assets: identity, sessions, device identity, provider trust.

## 10.2 Tampering

Examples:

- price modification;
- inventory mutation;
- manipulated command payload;
- altered tax fields;
- altered approval state;
- database row modification;
- artifact modification.

## 10.3 Repudiation

Examples:

- user denies refund;
- worker denies external submission;
- device denies offline transaction;
- operator denies support access.

Controls must retain reliable evidence while minimizing unnecessary sensitive data.

## 10.4 Information disclosure

Examples:

- cross-tenant API response;
- report leakage;
- support overreach;
- local database extraction;
- logs containing secrets;
- backups exposed;
- provider payload over-disclosure.

## 10.5 Denial of service

Examples:

- request floods;
- database lock amplification;
- oversized sync batch;
- expensive report queries;
- webhook flood;
- export flood;
- worker queue starvation;
- dependency outage amplification.

## 10.6 Elevation of privilege

Examples:

- user -> manager;
- branch user -> organization-wide;
- tenant admin -> platform support;
- ordinary device -> privileged device;
- stale offline authority -> active privileged operation.

---

# 11. High-Risk Threat Scenarios

The following scenarios are release-blocking classes when exploitable.

## T-001 Cross-Tenant BOLA

**Attack:** attacker authenticates as Tenant A and replaces an object identifier with Tenant B's object ID.

**Failure condition:** object is returned or mutated without tenant ownership verification.

**Potential impact:** full tenant data compromise, fraud, privacy breach.

**Mitigations:**

- derive tenant from trusted security context;
- re-check resource tenant relationship;
- scope repositories by authorized tenant;
- use DB constraints/RLS as defense in depth;
- test every ID-bearing endpoint negatively.

**Detection:** authorization-denial telemetry, unusual object-not-found/forbidden ratios, cross-scope access alerts.

**Evidence:** automated BOLA suite passes against release candidate.

## T-002 Cross-Branch Resource Escape

**Attack:** valid user manipulates branch/location/register IDs.

**Failure condition:** tenant is correct but resource scope is too broad.

**Impact:** stock manipulation, cash access, restricted operational data exposure.

**Mitigations:** explicit scope resolution and branch-aware policy evaluation.

## T-003 Role/Claim Forgery

**Attack:** client submits administrator role or modifies unsigned claims.

**Mitigation:** server trusts only cryptographically verified authentication claims and server-side membership records; roles are never accepted from business payloads.

## T-004 Approval Forgery

**Attack:** attacker creates or modifies an approval artifact before executing refund/stock/cash operation.

**Mitigation:** approval identity is server-created, stateful, bound to operation and approver, with SoD and immutable evidence.

## T-005 Double Refund

**Attack:** replay refund commands concurrently.

**Mitigation:** stable command identity, refundable-balance invariant, database uniqueness/locking, idempotent result storage.

**Required property:**

```text
sum(valid refunds + reversals) <= refundable amount
```

## T-006 Inventory Double Spend

**Attack:** two devices sell one remaining unit.

**Mitigation:** authoritative server-side inventory transaction and concurrency control. Offline capability may permit local operation only under documented thresholds; reconciliation must reject or hold commands that violate authoritative invariants.

## T-007 Local SQLite Tampering

**Attack:** attacker modifies local stock, price, command state or timestamps.

**Security position:** local state is untrusted evidence and operational state, not authoritative server truth.

**Mitigation:** server revalidation, cryptographic integrity where justified, monotonic sequence/checkpoint checks, device registration/revocation, sensitive local-storage minimization.

## T-008 Offline Replay

**Attack:** attacker replays a captured `command_id` or clones a valid command with modified content.

**Mitigation:**

```text
command_id + device_id + semantic payload fingerprint
                 -> unique idempotency record
```

Reuse with different semantics is an error, not a new command.

## T-009 Stale Authorization After Revocation

**Attack:** user/device is revoked while offline then continues to execute high-risk commands.

**Mitigation:** device capabilities are bounded; high-risk commands require current server authority or policy that explicitly permits offline execution; synchronization checks revocation state.

## T-010 Payment Callback Forgery

**Attack:** attacker posts directly to webhook endpoint.

**Mitigation:** provider-specific signature verification, event identity checks, independent provider re-query where contract requires, amount/currency/account/sale correlation.

## T-011 Payment Timeout After Commit

**Attack:** provider request times out after provider or Sitolo has committed external/internal state.

**Mitigation:** durable payment intent, explicit `UNKNOWN/PENDING` semantics, provider status re-query, stable operation IDs, no unsafe blind retry.

## T-012 EIS False Acceptance

**Attack:** fake tax acknowledgement or malformed provider response is used to mark a sale tax-compliant.

**Mitigation:** authenticated transport and provider response validation, persisted external evidence, explicit tax lifecycle, no client authority.

## T-013 MRA Secret Extraction

**Attack:** secret stored in source, bundle, log, SQLite or generic application configuration.

**Mitigation:** isolate terminal credentials, server-side secret management, redaction, restricted access, secret scanning.

## T-014 EIS Configuration Staleness

**Attack:** old terminal configuration remains active while MRA expects newer configuration.

**Mitigation:** versioned external configuration projection, refresh jobs, freshness checks, historical configuration references.

## T-015 Report/Data Exfiltration

**Attack:** attacker invokes broad export endpoint or manipulates filters to escape scope.

**Mitigation:** scope-first query construction, fixed maximum ranges, quotas, asynchronous processing for large exports, sensitive-field policy, audit trail.

## T-016 Support JIT Abuse

**Attack:** compromised support account accesses unrelated tenant without customer-authorized incident.

**Mitigation:** MFA, JIT activation, explicit reason/ticket, time bound, tenant-specific scope, immutable audit, SoD.

## T-017 SSRF to Cloud Metadata

**Attack:** URL field causes server-side fetch to metadata service/internal network.

**Mitigation:** allowlisted destinations or eliminate arbitrary URL fetching, egress policy, DNS/IP validation, redirect controls, timeouts, response-size limits.

## T-018 Malicious File Upload

**Attack:** archive bomb, parser exploit, path traversal, polyglot or executable content.

**Mitigation:** file-type allowlists, size/count limits, isolated processing, storage outside execution paths, content validation, malware scanning where justified.

## T-019 Database Privilege Escalation

**Attack:** SQL injection or compromised service allows access to unrelated data or admin operations.

**Mitigation:** parameterized SQL, least-privilege runtime role, migration role separation, no unnecessary superuser/BYPASSRLS capability, monitoring.

## T-020 CI/CD Supply-Chain Compromise

**Attack:** malicious PR modifies CI workflow, dependency, build step or artifact.

**Mitigation:** untrusted PR isolation, pinned actions/dependencies where feasible, protected branches, scoped OIDC/deploy identities, reproducible/attested artifacts, SBOM and artifact verification.

SLSA currently has version 1.2 as the current specification; the threat model therefore treats older references to SLSA 1.1 as historical rather than the latest baseline. ([SLSA specification](https://slsa.dev/spec/v1.2/))

## T-021 Secret Leakage Through Telemetry

**Attack:** bearer token, MRA secret, signature, payment credential or command payload enters logs/traces.

**Mitigation:** denylist/allowlist logging design, structured redaction, secret scanning, telemetry tests, restricted debugging.

OpenTelemetry semantic conventions provide common naming for telemetry and cover traces, metrics, logs, profiles and resource attributes. Sitolo should use them where practical while maintaining an explicit prohibition on sensitive-value cardinality. ([OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/))

## T-022 Database Lock Exhaustion

**Attack:** intentionally conflicting or expensive writes cause lock contention and connection exhaustion.

**Mitigation:** short transactions, deterministic lock order, bounded retries, statement timeouts, concurrency tests, queue bulk work, monitor wait times.

PostgreSQL's own regression infrastructure includes dedicated concurrency/isolation tests, reinforcing that concurrent behavior is a distinct testing domain rather than something adequately covered by ordinary unit tests. ([PostgreSQL 18 test documentation](https://www.postgresql.org/docs/18/regress-run.html))

---

# 12. Domain-Specific Threat Analysis

## 12.1 Identity and sessions

Threats:

- credential stuffing;
- password reset abuse;
- MFA bypass;
- refresh-token theft;
- session fixation;
- token replay;
- issuer/audience confusion;
- stale session after revocation;
- device/session mismatch.

Security invariants:

```text
verified identity -> trusted security context
expired/revoked session -> no sensitive mutation
MFA-protected operation -> cannot be downgraded by client input
refresh token -> rotation/reuse policy enforced
```

## 12.2 Tenant and IAM

Threats:

- tenant ID substitution;
- role escalation;
- inherited-scope mistakes;
- stale membership cache;
- branch escape;
- support/user-plane confusion.

Required negative cases:

- same user, wrong tenant;
- same tenant, wrong branch;
- correct tenant, wrong object owner;
- correct object, insufficient role;
- revoked membership;
- expired JIT grant;
- stale cache after revocation.

## 12.3 Catalogue and pricing

Threats:

- unauthorized price overrides;
- hidden discount abuse;
- historical sale reinterpretation;
- client-calculated total accepted as authority;
- malformed unit conversions.

Mitigation: server-calculated authoritative values, versioned price/tax snapshots, permission and threshold checks.

## 12.4 Inventory

Threats:

- stock injection;
- duplicate receiving;
- negative-stock bypass;
- concurrent sale race;
- adjustment fraud;
- historical ledger editing.

Integrity model:

```text
Stock state = reproducible result of accepted stock movements
```

No UI-only stock count is authority.

## 12.5 POS / Sales

Threats:

- duplicate sale submission;
- total manipulation;
- unauthorized discounts;
- stale inventory;
- unauthorized void;
- sale reordering.

A finalized sale must have a stable identity, immutable financial facts and auditable links to inventory, payment and tax state.

## 12.6 Cash

Threats:

- opening-float manipulation;
- unauthorized cash-in/out;
- closing-count forgery;
- duplicate correction;
- concealed variance.

Controls: register state machine, permissions, SoD, variance evidence, immutable transaction history.

## 12.7 Payments

Threats:

- fake success;
- callback forgery;
- duplicate callback;
- timeout ambiguity;
- wrong amount/currency/account;
- refund race;
- provider outage.

Payment provider evidence must not bypass Sitolo state validation.

## 12.8 Offline sync

Threats:

- cloned commands;
- replay;
- altered payload;
- stale capabilities;
- checkpoint rollback;
- oversized batch;
- event omission;
- duplicated side effects;
- local SQLite tampering;
- device loss.

Core invariant:

```text
LOCAL INTENT != SERVER AUTHORITY
```

## 12.9 MRA EIS

Threats:

- terminal impersonation;
- secret extraction;
- stale configuration;
- replayed fiscal submission;
- malformed fiscal payload;
- offline receipt-signing abuse;
- false acceptance;
- blocked terminal ignored.

MRA EIS failures must not corrupt the underlying sale record.

## 12.10 Reporting and export

Threats:

- cross-tenant leakage;
- hidden sensitive fields;
- unbounded queries;
- repeated export abuse;
- export job impersonation;
- stale access continuing after role revocation.

## 12.11 Administration/support

Threats:

- privileged-account compromise;
- tenant enumeration;
- broad search;
- unauthorized impersonation;
- hidden support access.

Administrative capabilities require stronger assurance than ordinary merchant actions.

## 12.12 CI/CD

Threats:

- malicious workflow change;
- pull-request secret exfiltration;
- dependency confusion;
- compromised action;
- mutable artifact replacement;
- leaked deployment credentials;
- unsigned release.

---

# 13. Data Flow Threat Analysis

## 13.1 Login flow

```text
Client
  -> authentication service
  -> credential/MFA validation
  -> session/token
  -> Sitolo API
  -> security context
```

Threats: token theft, replay, issuer confusion, device mismatch, downgrade to weaker authentication.

## 13.2 Online sale

```text
Client
 -> API
 -> authn/authz
 -> sale validation
 -> inventory transaction
 -> payment linkage
 -> audit/outbox
 -> commit
 -> async EIS/payment processing
```

Threats: forged totals, duplicate commands, authorization bypass, inventory race, external-call-inside-transaction defects.

## 13.3 Offline sale

```text
Client
 -> local command journal
 -> local sale evidence
 -> connectivity returns
 -> sync transport
 -> authentication
 -> device validation
 -> scope authorization
 -> idempotency
 -> domain validation
 -> PostgreSQL transaction
 -> acknowledgement
```

Threats: local tampering, replay, stale role, altered timestamp, batch abuse, checkpoint rollback.

## 13.4 Payment webhook

```text
Provider
 -> HTTPS webhook
 -> edge validation
 -> signature validation
 -> event identity
 -> persistence
 -> provider verification
 -> reconciliation
```

Threats: forged callback, replay, malformed payload, event ordering, timeout race.

## 13.5 EIS submission

```text
Sale committed
 -> tax outbox
 -> MRA adapter
 -> authenticated request
 -> MRA response
 -> validate/match
 -> tax state transition
 -> audit evidence
```

Threats: false acceptance, duplicate submission, configuration mismatch, credential leakage.

## 13.6 Export

```text
Authorized user
 -> scoped query
 -> policy/limits
 -> asynchronous export job
 -> object storage
 -> signed/authorized retrieval
```

Threats: scope escape, predictable object names, stale download URLs, bulk exfiltration.

---

# 14. Security Invariants

These are testable truths. Any implementation that cannot state how an invariant is protected is incomplete.

## 14.1 Tenant invariant

```text
A request may access or mutate a tenant resource only if the authenticated
security context contains valid authority over that tenant and the specific scope.
```

## 14.2 Object invariant

```text
Possession of an object ID never grants access to the object.
```

## 14.3 Financial invariant

```text
A finalized financial fact is not overwritten to conceal a correction.
```

## 14.4 Idempotency invariant

```text
Same operation identity + same semantic request
    -> one logical side effect
```

## 14.5 Idempotency mismatch invariant

```text
Same operation identity + materially different request
    -> rejection / conflict
```

## 14.6 Payment invariant

```text
Client assertion != payment success
```

## 14.7 Tax invariant

```text
Failed external tax submission != invalid local sale
```

## 14.8 Authorization invariant

```text
No caller-supplied role/tenant/approval field can elevate authority.
```

## 14.9 Offline invariant

```text
Local acceptance does not bypass eventual authoritative validation.
```

## 14.10 Audit invariant

```text
A sensitive operation must have sufficient evidence to reconstruct actor,
resource, scope, authorization, action and result.
```

## 14.11 Secret invariant

```text
Secrets are never intentionally emitted into source, client bundles,
logs, telemetry or public artifacts.
```

## 14.12 Recovery invariant

```text
Ambiguous external outcomes are represented as unresolved state until safely reconciled.
```

---

# 15. Threat Prioritization

A practical Sitolo risk score is:

```text
Risk = Likelihood x Impact x Exposure Factor
```

where each factor is assessed on a documented scale. The arithmetic is a prioritization aid, not a claim of mathematical precision.

## 15.1 Impact dimensions

Assess:

- tenant confidentiality loss;
- tenant isolation breach;
- financial loss;
- inventory corruption;
- regulatory/tax exposure;
- operational outage;
- evidence destruction;
- safety implications in vertical modules;
- reputational damage;
- recovery complexity.

## 15.2 Likelihood dimensions

Assess:

- public reachability;
- attack complexity;
- authentication prerequisite;
- insider requirement;
- exploit repeatability;
- presence of automation;
- ease of detection.

## 15.3 Prioritization classes

### Critical

Examples:

- cross-tenant mutation;
- authentication bypass;
- privileged-session compromise;
- payment double settlement;
- destructive financial corruption;
- signing-secret compromise;
- deployment credential compromise;
- arbitrary code execution in production control plane.

### High

Examples:

- branch authorization bypass;
- refund/discount abuse;
- EIS integrity bypass;
- major data export escape;
- offline replay producing financial duplication;
- database privilege escalation;
- serious SSRF.

### Medium

Examples:

- restricted metadata disclosure;
- non-sensitive report leakage;
- bounded availability degradation;
- non-critical configuration tampering.

### Low

Only issues with genuinely constrained impact should enter this category. “Low” must not be used simply because exploitation is inconvenient.

---

# 16. Mitigation Architecture

## 16.1 Prevent

Controls include:

- typed security context;
- centralized policy checks;
- database constraints;
- transaction boundaries;
- cryptographic verification;
- allowlists;
- quotas;
- secure defaults;
- dependency pinning/policy.

## 16.2 Detect

Controls include:

- audit events;
- authorization-denial metrics;
- anomaly signals;
- provider verification failures;
- secret scanning;
- supply-chain scanning;
- integrity checks.

## 16.3 Respond

Every high-risk threat should have an operational path:

```text
DETECT
 -> CONTAIN
 -> PRESERVE EVIDENCE
 -> REVOKE / BLOCK
 -> RECOVER
 -> VALIDATE
 -> PATCH
 -> ADD REGRESSION TEST
 -> REVIEW RESIDUAL RISK
```

## 16.4 Recover

Recovery must preserve historical evidence and avoid “fixing” data through destructive edits.

---

# 17. Security Control Mapping

| Threat family | Primary control | Secondary control | Test evidence |
|---|---|---|---|
| Tenant escape | Trusted tenant context | DB scoping/RLS | Cross-tenant suite |
| BOLA | Object authorization | Query scoping | API negative tests |
| Role escalation | Server-side IAM | SoD | AuthZ matrix |
| Replay | Idempotency | Unique constraints | Replay tests |
| Financial duplication | Transaction + idempotency | Reconciliation | Concurrency suite |
| Inventory race | DB locking/transaction | Ledger invariants | Isolation tests |
| Payment forgery | Signature verification | Provider re-query | Contract tests |
| EIS forgery | Adapter validation | Evidence store | EIS tests |
| Offline abuse | Device/capability checks | Server revalidation | Sync security suite |
| Secret leakage | Secret manager | Scanners | Secret tests |
| SSRF | Destination policy | Egress controls | SSRF suite |
| File abuse | Parser isolation | Limits/scanning | File-security tests |
| DoS | Rate limits | Bulkheads/timeouts | Load tests |
| CI compromise | Branch/workflow controls | Provenance | Pipeline tests |
| Audit tampering | Append-oriented model | Access control | Audit integrity tests |
| Backup exposure | Encryption/access control | Restore tests | DR suite |

---

# 18. Security Test Requirements Derived From This Threat Model

Every threat marked Critical or High must map to at least one executable test, and the highest-risk classes must have multiple independent defenses tested.

## 18.1 Tenant isolation suite

Must test:

- every major read endpoint;
- every major mutation endpoint;
- nested resources;
- exports;
- search;
- asynchronous jobs;
- worker execution;
- support paths;
- direct database access paths.

## 18.2 Authorization mutation suite

Attempt:

- role substitution;
- branch substitution;
- register substitution;
- approval substitution;
- tenant substitution;
- stale membership;
- revoked device;
- expired grant.

## 18.3 Replay suite

Attempt to replay:

- sale commands;
- refund commands;
- payment events;
- EIS submissions;
- sync batches;
- device enrollment operations;
- approval actions.

## 18.4 Concurrency suite

Must include:

- two sales consuming last stock;
- two refunds consuming same refundable balance;
- concurrent payment processing;
- duplicate webhook processing;
- concurrent price changes where relevant;
- register-close races;
- duplicate worker claims.

## 18.5 Offline adversarial suite

Must include:

- modified command payload;
- reused command ID with changed payload;
- cloned device;
- revoked device;
- stale checkpoint;
- duplicate batch;
- out-of-order commands;
- oversized batch;
- invalid dependency graph;
- process crash mid-sync;
- SQLite corruption simulation.

## 18.6 Supply-chain suite

Must include:

- dependency vulnerability checks;
- secret scan;
- workflow-policy checks;
- dependency policy;
- SBOM generation;
- artifact provenance;
- artifact signature/verification;
- deploy identity restrictions.

---

# 19. Abuse-Resistant Resource Controls

Resource exhaustion is a security threat when it prevents merchants from operating.

## 19.1 API limits

Every endpoint must have:

- maximum body size;
- query complexity limits;
- pagination;
- timeout;
- concurrency policy;
- rate-limit classification;
- retry semantics.

## 19.2 Sync limits

Bound:

- commands per batch;
- bytes per batch;
- nested object depth;
- command history window;
- dependency graph size;
- checkpoint ranges;
- retry count;
- replay window.

## 19.3 Report limits

Limit:

- date range;
- row count;
- execution time;
- parallel jobs;
- export size;
- concurrent exports per tenant/user.

## 19.4 File limits

Bound:

- file size;
- number of files;
- archive expansion ratio;
- processing time;
- image dimensions;
- parser concurrency.

---

# 20. Security Telemetry

Telemetry must identify patterns without becoming a secondary data-leak channel.

## 20.1 High-value security metrics

```text
authentication_failures_total
authorization_denials_total
cross_scope_denials_total
mfa_failures_total
session_reuse_failures_total
device_revocations_total
idempotency_conflicts_total
replay_rejections_total
payment_signature_failures_total
payment_verification_mismatches_total
eis_submission_rejections_total
eis_retry_queue_depth
offline_sync_conflicts_total
report_export_bytes_total
rate_limit_rejections_total
secret_scan_failures_total
security_test_failures_total
restore_drill_failures_total
```

## 20.2 Cardinality rules

Never use:

```text
sale_id
user_id
raw_command_body
access_token
provider_signature
```

as unbounded metric labels.

Use trace/log correlation fields under a controlled schema rather than high-cardinality metrics.

OpenTelemetry provides standardized semantic conventions across telemetry signals; Sitolo should adopt common attribute names where applicable, but its own redaction and cardinality policy remains authoritative. ([OpenTelemetry](https://opentelemetry.io/docs/specs/semconv/))

## 20.3 Audit vs telemetry

Audit records answer:

> “What happened to this business/security object?”

Telemetry answers:

> “What operational/security pattern is occurring?”

They must not be conflated.

---

# 21. Incident Detection and Response Mapping

| Threat | Detection | Immediate containment |
|---|---|---|
| Cross-tenant access | denial spike/anomaly | disable affected route, investigate |
| Account takeover | login/MFA anomalies | revoke sessions/devices |
| Payment forgery | signature failures | quarantine events |
| Payment mismatch | reconciliation anomaly | hold fulfilment |
| EIS fraud | response mismatch | tax exception + preserve evidence |
| Offline compromise | replay/conflict anomalies | revoke device |
| Secret leak | secret scanner / log detection | revoke + rotate |
| CI compromise | workflow/artifact anomaly | freeze releases |
| Data export abuse | volume/rate alerts | suspend export capability |
| DB compromise | DB/auth logs | isolate runtime identity |
| DoS | rate/queue/DB saturation | shed non-critical work |

---

# 22. Recovery Requirements

## 22.1 Financial integrity incident

1. freeze affected flow;
2. identify affected transactions;
3. stop additional side effects;
4. preserve database/audit/provider evidence;
5. reconcile against authoritative evidence;
6. issue explicit corrective events;
7. validate inventory/payment/tax projections;
8. add regression tests;
9. document root cause.

## 22.2 Tenant isolation incident

1. disable affected path;
2. revoke relevant credentials;
3. determine blast radius;
4. preserve evidence;
5. validate tenant boundaries;
6. patch;
7. run full isolation suite;
8. review notification/legal obligations.

## 22.3 Compromised device

1. revoke device;
2. revoke device-bound credentials;
3. inspect pending sync commands;
4. assess locally exposed data;
5. preserve server-received evidence;
6. register replacement device;
7. require reauthentication;
8. investigate anomalies.

## 22.4 Supply-chain incident

1. freeze releases;
2. revoke affected identities;
3. identify artifacts/commits/dependencies;
4. establish clean build environment;
5. verify source and dependencies;
6. redeploy verified artifacts;
7. rotate exposed credentials;
8. preserve forensic evidence.

---

# 23. Residual Risk Register

The following risks cannot be eliminated purely through architecture.

## R-01 Compromised merchant endpoint

A rooted or physically compromised Android device may expose cached data or credentials. Mitigation reduces impact but cannot turn the device into a trusted enclave.

## R-02 Unsynchronized offline data loss

A device can be destroyed before local commands synchronize. Sitolo cannot honestly guarantee recovery of information that never reached an authoritative server or durable backup path.

## R-03 External provider outage

Sitolo cannot force payment or MRA availability. It can preserve intent, reconcile later, and prevent inconsistent internal state.

## R-04 Provider contract drift

External APIs can change. Contract fixtures, adapter tests and monitored schema/version changes reduce but do not eliminate this risk.

## R-05 Insider abuse

Strong SoD/JIT/audit controls reduce risk but do not remove the possibility of collusion or credential compromise.

## R-06 Database catastrophic failure

Backups/PITR/recovery testing reduce recovery risk. DR objectives must remain measurable rather than being stated as “zero data loss.”

## R-07 Supply-chain compromise

Dependency and CI controls lower probability and blast radius but cannot guarantee that every upstream ecosystem component is uncompromised.

---

# 24. Threats Introduced by Offline Capability

Offline support is a major security differentiator but also increases attack surface.

## 24.1 New attack surfaces

Offline introduces:

- durable local data;
- persistent local command logs;
- longer-lived local capabilities;
- replay windows;
- clock manipulation;
- local tampering;
- device cloning;
- process-crash recovery;
- synchronization conflict semantics.

## 24.2 Required compensating controls

```text
minimum cached data
        +
secure credential storage
        +
device identity
        +
bounded capability profile
        +
immutable command identity
        +
server-side validation
        +
replay/idempotency controls
        +
checkpoint safety
        +
revocation
        +
crash recovery
```

## 24.3 Prohibited shortcuts

Do not:

- trust device timestamp for financial authority;
- trust local role flags;
- trust local stock as final truth;
- trust local payment success;
- allow arbitrary local administration;
- accept a reused command ID with changed meaning.

---

# 25. Threats Introduced by Integrations

Every integration creates a new trust boundary.

## 25.1 General adapter requirements

An adapter must define:

- authentication mechanism;
- secret location;
- request canonicalization;
- response validation;
- timeout;
- retry classification;
- idempotency;
- duplicate behavior;
- rate limits;
- error mapping;
- observability;
- evidence retention;
- contract-test fixtures.

## 25.2 Payment-specific

Payment status must be independently reconciled when callback evidence is insufficient.

## 25.3 MRA-specific

Terminal identity, fiscal configuration, offline signing and external acceptance/rejection must remain inside the EIS boundary.

---

# 26. Software Supply Chain Threat Model

The application is not secure if its release pipeline can be trivially subverted.

## 26.1 Threat sources

- compromised developer account;
- malicious pull request;
- compromised dependency;
- compromised CI action;
- vulnerable build tool;
- runner persistence;
- leaked OIDC/deployment authority;
- mutable artifact registry;
- stolen signing key;
- unverified production artifact.

## 26.2 Security controls

Required:

- branch protection;
- review requirements;
- least-privileged CI permissions;
- PR trust separation;
- secret isolation;
- dependency scanning;
- license/policy checks;
- SBOM;
- provenance/attestation;
- artifact verification;
- protected deployment environments;
- credential rotation;
- release audit trail.

SLSA 1.2 is the current SLSA specification as of this document date, so future implementation documentation should use the current specification rather than older 1.1 terminology unless historical compatibility is intentional. ([SLSA 1.2](https://slsa.dev/spec/v1.2/))

---

# 27. Threat Modeling for CI/CD Changes

Any change to:

```text
.github/workflows/
Dockerfiles
build scripts
release scripts
package managers
Cargo manifests/lockfiles
Flutter dependencies
Tauri dependencies
artifact signing
deployment manifests
OIDC configuration
cloud IAM
```

must trigger supply-chain threat review.

A CI change is security-sensitive even if application code does not change.

---

# 28. Threat Model for Multi-Tenant Noise and DoS

One tenant must not be able to consume all shared resources.

## Required isolation dimensions

- API concurrency;
- worker concurrency;
- report jobs;
- export bandwidth;
- storage quotas;
- database connections;
- expensive query budgets;
- synchronization throughput.

Conceptual scheduling:

```text
Critical financial/integration work
        |
High-priority retries
        |
Normal operational jobs
        |
Bulk reporting/export
        |
Maintenance
```

Bulk work must not starve POS and financial processing.

---

# 29. Threat Model for Observability

Observability can itself create a security incident.

## Threats

- access tokens in logs;
- raw webhook bodies;
- MRA secrets;
- payment identifiers with excessive PII;
- full command bodies;
- sensitive trace baggage;
- debug endpoints exposing internals.

## Controls

- structured allowlisted fields;
- redaction at source;
- safe exception serialization;
- protected log access;
- retention policy;
- production debug disabled;
- telemetry security tests.

---

# 30. Threat Model for Backup and Restore

Backups are attractive targets because they combine large quantities of data with privileged access.

Threats:

- unencrypted backups;
- public object storage;
- overprivileged restore operator;
- backup tampering;
- stale secrets inside backup;
- ransomware-like deletion of both primary and backup.

Controls:

- encryption at rest;
- restricted backup identity;
- separate backup account/project where feasible;
- immutable/versioned storage where appropriate;
- restore drills;
- backup integrity monitoring;
- access audit.

Restore itself must be treated as a security-sensitive action because a restore may expose historical authorization/configuration state.

---

# 31. Threat Model for Vertical Extensions

## 31.1 Pharmacy

Potential additional threats:

- lot/expiry manipulation;
- FEFO bypass;
- controlled-product access bypass;
- prescription data leakage;
- counterfeit/recall traceability failure.

The pharmacy extension requires separate compliance validation before any regulatory claim.

## 31.2 Agro-dealer

Potential threats:

- batch/lot traceability corruption;
- regulated input access abuse;
- quantity manipulation;
- supplier identity fraud.

## 31.3 Wholesale

Potential threats:

- large-order approval bypass;
- credit exposure manipulation;
- bulk pricing leakage;
- warehouse transfer races.

Each vertical must inherit core tenant/IAM/audit/inventory controls rather than implementing parallel security models.

---

# 32. Threat Modeling of State Machines

Boolean flags frequently create exploitable impossible states. Threat analysis therefore applies to state transitions.

Example sale:

```text
DRAFT
  -> VALIDATED
  -> FINALIZED
  -> FULFILLMENT / PAYMENT / TAX states
```

Example refund:

```text
REQUESTED
  -> APPROVED
  -> PROCESSING
  -> COMPLETED
  -> FAILED / EXCEPTION
```

Example payment:

```text
CREATED
 -> PENDING
 -> SUCCEEDED
 -> FAILED
 -> UNKNOWN / RECONCILIATION_REQUIRED
```

Security testing must attempt illegal transitions directly through APIs and worker paths.

---

# 33. Threat Modeling of Events and Outbox

Events are security-relevant because they can trigger external side effects.

Threats:

- duplicate event delivery;
- missing event;
- stale event version;
- forged event payload;
- incorrect tenant context;
- outbox corruption;
- replay after worker crash.

Required properties:

```text
outbox inserted atomically with domain transaction
        |
stable event identity
        |
consumer idempotency
        |
versioned event schema
        |
explicit retry semantics
```

No consumer should treat event delivery as proof that the underlying business state exists without reading the authoritative record when required.

---

# 34. Threat Modeling of Caching

Caches can become authorization vulnerabilities when stale security decisions survive too long.

Threats:

- stale membership;
- stale role;
- stale device state;
- stale price;
- stale tax configuration;
- stale entitlement.

Rule:

```text
cache improves performance
cache does not redefine authority
```

Security-sensitive cache invalidation must be explicit.

---

# 35. Threat Modeling of Database Access

The database is a high-value target.

## Required separation

At minimum conceptually distinguish:

```text
application runtime role
migration role
administrative role
backup/restore role
read/reporting role
```

The runtime role must not receive unnecessary superuser, broad DDL or row-security bypass privileges.

PostgreSQL RLS can provide default-deny behavior, but privileged roles and table ownership require explicit review. Therefore RLS is defense in depth, not permission to abandon application authorization.

## Query threat controls

- parameterized SQL;
- bounded query patterns;
- explicit transaction boundaries;
- safe repository interfaces;
- query timeout;
- lock timeout where appropriate;
- least privilege;
- migration review.

---

# 36. Security Decision Matrix

Every feature review must answer:

| Question | Required answer |
|---|---|
| What asset is at risk? | Named asset |
| What principal can invoke it? | Explicit actor set |
| What tenant/scope applies? | Explicit scope |
| What can be replayed? | Listed operations |
| What can race? | Listed concurrent paths |
| What external system is involved? | Named boundary |
| What is authoritative? | Named source |
| What is cached/local? | Named non-authoritative state |
| What secrets are involved? | Storage + access policy |
| What evidence is produced? | Audit/telemetry records |
| How can it fail? | State/failure taxonomy |
| How is failure recovered? | Runbook |
| What proves the control works? | Automated/manual test |
| What blocks release? | Gate |

A feature lacking these answers is not security-ready.

---

# 37. Change Triggers Requiring Threat-Model Review

Threat-model review is mandatory when any of the following changes:

1. authentication mechanism;
2. MFA/recovery flow;
3. authorization semantics;
4. tenant hierarchy;
5. branch/resource scopes;
6. offline capability profile;
7. sync protocol;
8. payment provider;
9. payment state machine;
10. MRA EIS integration;
11. tax fields or fiscalization;
12. financial state transitions;
13. inventory concurrency semantics;
14. support/JIT access;
15. file upload or URL fetching;
16. report/export architecture;
17. database privilege model;
18. object storage policy;
19. logging/telemetry pipeline;
20. deployment identity;
21. CI workflow privileges;
22. dependency-management mechanism;
23. artifact-signing mechanism;
24. cloud network exposure;
25. new vertical requiring regulated data;
26. introduction of a new third-party integration;
27. any critical vulnerability; or
28. material incident revealing a missing attack path.

---

# 38. Threat Model Review Cadence

Threat modeling is continuous.

## Mandatory reviews

- architecture baseline;
- each major feature;
- pre-production security review;
- before first production launch;
- after major incident;
- after material dependency/cloud/security change;
- periodic review at least annually.

## Triggered review

A review must happen immediately when:

- a Critical vulnerability is found;
- tenant isolation is questioned;
- production secret exposure occurs;
- authentication bypass is suspected;
- external provider contract materially changes;
- regulated vertical scope expands.

---

# 39. Threat-to-Test Traceability Matrix

| Threat | Mandatory test class | Release status |
|---|---|---|
| T-001 BOLA | Cross-tenant negative tests | Block |
| T-002 Branch escape | Scope authorization tests | Block |
| T-003 Role forgery | AuthZ negative tests | Block |
| T-004 Approval forgery | SoD/approval tests | Block |
| T-005 Double refund | Concurrency + idempotency | Block |
| T-006 Stock double-spend | DB isolation/concurrency | Block |
| T-007 SQLite tampering | Sync adversarial tests | Block |
| T-008 Offline replay | Replay/idempotency | Block |
| T-009 Revoked device | Device/security sync | Block |
| T-010 Payment forgery | Provider contract tests | Block |
| T-011 Payment ambiguity | Timeout/reconciliation | Block |
| T-012 EIS forgery | Adapter contract tests | Block |
| T-013 Secret extraction | Secret scan/bundle scan | Block |
| T-014 Config staleness | Configuration lifecycle tests | Block |
| T-015 Export leakage | Scope/export tests | Block |
| T-016 Support abuse | JIT/audit tests | Block |
| T-017 SSRF | SSRF suite | Block |
| T-018 File abuse | File security suite | Block |
| T-019 DB escalation | Integration/security tests | Block |
| T-020 Supply-chain | CI/security pipeline | Block |
| T-021 Telemetry leakage | Redaction tests | Block |
| T-022 DB exhaustion | Concurrency/load tests | Gate |

---

# 40. Release Blocking Rules

A release must not proceed when any of the following is true:

```text
Critical threat has an unmitigated exploitable path
        OR
Tenant isolation tests fail
        OR
Sensitive authorization bypass exists
        OR
Financial duplication can be demonstrated
        OR
Payment verification can be bypassed
        OR
Offline replay can create duplicate financial effects
        OR
Production secrets are present in artifacts
        OR
Deployment identity is broader than approved
        OR
Critical restore path is unverified
```

Temporary exceptions require explicit risk ownership, expiration date, compensating controls and documented approval. Exceptions do not silently reduce the severity of the threat.

---

# 41. Secure-by-Design Requirements

Security must be designed into the architecture rather than appended after feature implementation.

CISA-led secure-by-design guidance recommends that manufacturers perform risk assessment to identify prevalent cyber threats and incorporate protections into product design, with defense in depth. ([CISA/partner secure-by-design guidance](https://www.cisa.gov/resources-documents/principles-and-approaches-secure-design-and-default))

For Sitolo this means:

```text
THREAT MODEL
      ↓
SECURITY INVARIANTS
      ↓
ARCHITECTURE
      ↓
IMPLEMENTATION
      ↓
AUTOMATED TEST
      ↓
TELEMETRY
      ↓
OPERATIONS
      ↓
INCIDENT LEARNING
      ↓
THREAT MODEL UPDATE
```

---

# 42. Architectural Anti-Patterns Prohibited by This Threat Model

## 42.1 “The frontend hides it”

Not a security control.

## 42.2 “The tenant ID is in the JWT”

Not sufficient without validating current membership and resource scope.

## 42.3 “The provider said it succeeded”

External evidence still requires correlation with internal intent/state.

## 42.4 “SQLite said the sale succeeded”

Local state is not final server authority.

## 42.5 “We can just retry”

Unsafe for unknown financial/external outcomes.

## 42.6 “RLS will solve authorization”

RLS can be defense in depth; application/business authorization remains necessary.

## 42.7 “Logs are harmless”

Logs often become the easiest route to secrets and personal data.

## 42.8 “A successful build means a secure release”

Build correctness does not establish provenance, authorization, dependency safety or isolation.

## 42.9 “Security testing is a later phase”

Threat-derived tests must exist alongside implementation.

## 42.10 “Enterprise means maximum complexity”

Unnecessary distributed systems add attack surface and failure modes. Complexity must have a business/security justification.

---

# 43. Evidence Requirements

Security claims must be evidence-backed.

## 43.1 Acceptable evidence

- automated test result;
- code review evidence;
- static analysis result;
- dependency scan;
- configuration validation;
- deployment attestation;
- signed artifact verification;
- penetration-test result;
- controlled manual test;
- restore-drill evidence;
- provider contract fixture/result;
- audit record inspection.

## 43.2 Unacceptable evidence

- “works on my machine”;
- UI-only demonstration;
- source-code intent without execution;
- a passing happy-path test;
- undocumented manual assertion;
- a production request that merely happened to succeed once.

OWASP ASVS is explicitly intended to provide a basis for testing technical security controls and for building secure applications; Sitolo therefore treats security requirements as verification targets rather than documentation-only aspirations. ([OWASP ASVS](https://owasp.org/www-project-application-security-verification-standard/))

---

# 44. Security Ownership Model

Every threat must have an owner at the implementation level.

| Area | Primary owner |
|---|---|
| AuthN/session | Identity/security subsystem |
| AuthZ/IAM | Security + application architecture |
| Tenant isolation | Platform/domain engineering |
| Financial invariants | Sales/finance domain owner |
| Inventory invariants | Inventory domain owner |
| Offline sync | Sync/platform owner |
| Payment integration | Payments owner |
| MRA EIS | Tax/integration owner |
| Database privilege/RLS | Data/platform owner |
| CI/CD security | Platform/SRE/security |
| Cloud/IAM | Platform/SRE |
| Audit | Security/platform |
| Incident response | Security/SRE |
| DR | SRE/platform |

Ownership does not transfer responsibility away from engineering teams. A high-risk threat must have cross-functional review when it crosses boundaries.

---

# 45. Threat Model Exit Criteria

This document is considered implementation-ready only when:

```text
[X] system boundaries defined
[X] trust boundaries defined
[X] critical assets defined
[X] threat actors defined
[X] attacker capability model defined
[X] abuse cases defined
[X] STRIDE lens applied
[X] domain-specific threats defined
[X] security invariants defined
[X] mitigations mapped
[X] detection mapped
[X] recovery mapped
[X] high-risk tests identified
[X] release-blocking threats identified
[X] residual risks recorded
[X] review triggers defined
[X] evidence requirements defined
```

Implementation is **not** considered threat-model complete until code and infrastructure produce evidence for the controls identified here.

---

# 46. Final Security Position

Sitolo's central security problem is not merely unauthorized access. It is preserving **trusted economic state across hostile clients, multiple tenants, intermittent connectivity, concurrent devices, external payment systems, tax authorities, and evolving infrastructure**.

The security architecture therefore forms a chain:

```text
UNTRUSTED INPUT
      ↓
AUTHENTICATION
      ↓
TRUSTED SECURITY CONTEXT
      ↓
TENANT / SCOPE AUTHORIZATION
      ↓
COMMAND VALIDATION
      ↓
DOMAIN INVARIANTS
      ↓
TRANSACTION / CONCURRENCY CONTROL
      ↓
IDEMPOTENCY
      ↓
AUTHORITATIVE COMMIT
      ↓
AUDIT / OUTBOX
      ↓
EXTERNAL SIDE EFFECT
      ↓
RECONCILIATION
      ↓
OBSERVABILITY
      ↓
RECOVERY
      ↓
REGRESSION TEST
```

The corresponding failure philosophy is:

```text
DO NOT GUESS
DO NOT TRUST THE CLIENT
DO NOT TRUST OBJECT IDS
DO NOT TRUST EXTERNAL RESPONSES BLINDLY
DO NOT RETRY UNKNOWN FINANCIAL OPERATIONS BLINDLY
DO NOT MODIFY HISTORY TO HIDE ERRORS
DO NOT GRANT PRIVILEGE FROM LOCAL STATE
DO NOT TREAT OFFLINE DATA AS FINAL AUTHORITY
DO NOT LET BULK WORK STARVE TRANSACTIONAL WORK
DO NOT DEPLOY UNVERIFIED ARTIFACTS
```

And the recovery philosophy is:

```text
DETECT
 -> CONTAIN
 -> PRESERVE EVIDENCE
 -> REVOKE / BLOCK
 -> RECONCILE
 -> RECOVER
 -> VERIFY
 -> TEST
 -> IMPROVE
```

This threat model is therefore not a static document intended to declare Sitolo “secure.” It is the adversarial contract against which the architecture, implementation, testing, deployment and operations documents must remain consistent.

---

# 47. References and Current Baselines

1. OWASP Threat Modeling Cheat Sheet — threat modeling as a structured, repeatable and continuously maintained process.  
   https://cheatsheetseries.owasp.org/cheatsheets/Threat_Modeling_Cheat_Sheet.html

2. OWASP Application Security Verification Standard 5.0.0 — current stable application-security verification baseline as of 2026-09-04.  
   https://owasp.org/www-project-application-security-verification-standard/

3. OWASP API Security Top 10 2023 — API-specific authorization, authentication, resource-consumption, SSRF, misconfiguration and unsafe third-party API risks.  
   https://owasp.org/API-Security/editions/2023/en/0x10-api-security-risks/

4. NIST Cybersecurity Framework 2.0 — cybersecurity risk-management framework.  
   https://www.nist.gov/publications/nist-cybersecurity-framework-csf-20

5. CISA and international partners — secure-by-design principles and approaches.  
   https://www.cisa.gov/resources-documents/principles-and-approaches-secure-design-and-default

6. PostgreSQL 18 documentation — regression, isolation/concurrency and recovery test infrastructure.  
   https://www.postgresql.org/docs/18/regress-run.html

7. OpenTelemetry Semantic Conventions — standardized semantic names across traces, metrics, logs and resources.  
   https://opentelemetry.io/docs/specs/semconv/

8. SLSA 1.2 specification — current SLSA specification baseline.  
   https://slsa.dev/spec/v1.2/

---

# 48. Relationship to Other Sitolo Documents

This file must remain consistent with:

```text
security_implementation_spec.md
        |
system_architecture_design.md
        |
domain_model.md
        |
database_design.md
        |
api_contract.md
        |
auth_authorization_spec.md
        |
sync_protocol.md
        |
payment_integration_spec.md
        |
mra_eis_integration_spec.md
        |
testing_strategy.md
```

Threat-model changes that invalidate one of these documents require explicit reconciliation rather than allowing contradictory security assumptions to coexist.

The next implementation contract is `11 — observability_spec.md`.

---

**Document end — Sitolo Enterprise Threat Model, Phase 0 / File 10 of 16.**
