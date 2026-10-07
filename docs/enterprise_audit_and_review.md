# Sitolo Codebase Enterprise Audit & Architecture Review

**Target system:** Sitolo — Business Operating System for African SMEs
**Audit date:** 2026-09-26
**Source baseline at this review:** PR #68 feature branch; runtime transport source @ `b2491549ebe3b72e3c4e62c44788f638f51f60f9`
**Documentation-remediation context:** `feat/agentic-workflow`
**Status:** Current-state enterprise audit
**Authority:** Current source tree plus the governing documentation hierarchy in `agent.md`

> This document is a current-state assessment. It replaces the older audit's pre-Phase-4 implementation snapshot. Historical findings from earlier audits remain useful evidence but must not be read as the current repository state.

---

## Executive Summary

Sitolo is **not production-ready**, but the repository is materially beyond the condition described by the previous enterprise audit.

The current source contains real identity/authentication primitives, organization/branch/membership domain state, tenancy and effective-scope resolution, role/permission/assignment logic, invitation state, API transport validation and tenancy handlers, PostgreSQL authority/runtime primitives, security controls, and substantial telemetry/test infrastructure.

The remaining production gap is not the absence of an architectural model. It is the incomplete connection of those foundations into a durable production transaction system.

The largest current blockers are:

1. **The HTTP architecture has been reconciled.** Governing documents specify Rust + Axum + Tokio; the current `apps/api` boundary uses Tokio for runtime/listener lifecycle, Hyper/Hyper-util for HTTP transport, and Axum/Tower for routing and request controls. The former hand-written HTTP parser has been removed.
2. **General business-domain execution is incomplete.** The domain crate currently exposes tenancy; product catalogue, inventory, sales, payments, reconciliation and other core business engines remain contract/future work.
3. **Phase 5 PostgreSQL schema/migration/RLS delivery is not the full current runtime business authority yet.** PostgreSQL connection/authority primitives exist, but the complete business schema and repository implementation remain phase-gated.
4. **Authentication helpers exist but are not yet the complete production HTTP authentication boundary.**
5. **General Phase 6 policy enforcement is not yet the final production authorization layer.** Current role/scope primitives are real, but high-risk authorization policy remains separately owned.
6. **Workers, event publication, integrations and synchronization remain substantially unimplemented.**
7. **The commercial and technical documentation corpus still contains temporal/history ambiguities that can cause agents to confuse targets, current state and historical evidence.**
8. **GitHub repository administration controls are an external deployment concern and are not proven by workflow files alone.**

Therefore the appropriate posture is:

```text
ARCHITECTURAL FOUNDATION
        ↓
REFERENCE IMPLEMENTATIONS
        ↓
PARTIAL PRODUCTION BOUNDARIES
        ↓
NOT YET A COMPLETE BUSINESS SYSTEM
```

---

# 1. Authority and Evidence Model

The repository's source hierarchy remains:

```text
1. Applicable law / regulator requirement
2. Current external provider contract
3. Approved product/business decision
4. Security architecture + security implementation contract
5. Domain model
6. Database / API / integration specifications
7. Testing / observability / deployment specifications
8. ADRs
9. Implementation convenience
```

The current source tree is used to determine **implementation state**. A normative contract is used to determine **required state**.

A contradiction is therefore classified as one of:

| Finding type | Meaning |
|---|---|
| Documentation stale | The document describes a state that the current authority no longer supports |
| Implementation gap | The contract is current, but source implementation is incomplete |
| Historical evidence | The statement is true for an earlier baseline and must not be used as current state |
| Future-phase requirement | The contract intentionally describes a later phase |
| External-fact freshness issue | A dated/versioned outside fact needs current verification |

---

# 2. Current Source Baseline

The current repository contains non-trivial implementations in:

- `sitolo-auth`: authentication/session/MFA/device/token/security-context primitives
- `sitolo-domain`: tenancy entities and lifecycle semantics
- `sitolo-tenancy`: requested-vs-trusted scope typing and effective scope resolution
- `sitolo-authz`: permissions, roles, assignments, scope grants and invitation models
- `sitolo-application`: organization/branch/membership/IAM service orchestration
- `sitolo-api`: bounded DTOs, authentication helpers, errors and tenancy HTTP transport
- `sitolo-persistence`: reference repositories plus PostgreSQL authority/runtime controls
- `sitolo-security`: cross-cutting security primitives
- `sitolo-observability`: bounded operational telemetry
- repository-level CI/policy/security verification

At the same time:

- `apps/worker/src/main.rs` remains a scaffold with no worker loop;
- `sitolo-events` has no substantive event runtime;
- `sitolo-integrations` has no substantive provider runtime;
- `sitolo-sync` has no substantive synchronization runtime;
- `sitolo-domain` does not yet contain the broader operational business engines;
- mobile/desktop clients are not present;
- the full Phase 5 application schema/migration layer is not yet the completed business persistence system.

---

# 3. Enterprise Assessment Matrix

| Dimension | Current state | Evidence / implication |
|---|---|---|
| 1. Architecture & boundaries | **Strong foundation / transport reconciled** | Modular-monolith boundaries and dependency direction exist; the HTTP boundary now follows Tokio → Hyper/Hyper-util → Axum/Tower with explicit transport/resource controls. |
| 2. Application architecture | **Partial** | Application services now orchestrate tenancy/IAM, but the complete command/query/business engine chain is unfinished. |
| 3. API design & trust boundaries | **Partial** | Bounded DTOs, validation and tenancy handlers exist. General business API surface is not implemented, and production auth wiring is incomplete. |
| 4. Authentication & authorization | **Partial foundation** | Authentication/security primitives and role/scope models exist. General policy enforcement and complete HTTP integration are still phase-gated. |
| 5. Input validation & integrity | **Partial** | Tenancy transport uses bounded bodies and `deny_unknown_fields`; domain-wide request validation still depends on future business endpoints. |
| 6. IDOR / injection / SSRF / boundary security | **Partial** | Security contracts and primitives are substantial; broad production attack-surface coverage cannot be proven until all business/integration endpoints exist. |
| 7. Errors / retries / timeouts / failure isolation | **Partial substrate** | Error families and bounded probe behavior exist; provider retry/reconciliation infrastructure remains future work. |
| 8. CPU vs I/O behavior | **Contract defined / implementation incomplete** | Async/runtime rules and password-hashing boundaries are specified; full production load evidence does not yet exist. |
| 9. Concurrency & race safety | **Partial** | Reference services use in-process locking; PostgreSQL authority primitives exist, but the full business mutation workload is not yet running through durable transactions. |
| 10. Database / transactions / migrations / RLS | **Partial / Phase 5 gate** | PostgreSQL authority/runtime code and database-specific test assets exist, while complete schema/migration/RLS application delivery remains phase-gated. |
| 11. Caching & consistency | **Not implemented as a production capability** | Redis/cache is an optional architectural acceleration, not a current source-of-truth mechanism. |
| 12. Workers / outbox / events / idempotency | **Not production-complete** | Worker binary and event crate remain scaffolds; durable asynchronous execution is still required. |
| 13. Observability | **Substrate implemented; operational coverage incomplete** | Structured telemetry, bounded names and registry/schema controls exist; end-to-end business metrics/exporters/alerts are not fully integrated. |
| 14. Testing | **Strong foundation / incomplete coverage** | Auth/config/API/PostgreSQL security tests exist; full business, offline-device, integration and recovery coverage does not yet exist. |
| 15. Configuration & secrets | **Implemented foundation** | Pinned configuration/toolchain and sensitive-value handling are implemented; production secret infrastructure remains deployment-specific. |
| 16. Deployment & release safety | **Contract-heavy / deployment controls incomplete** | Release governance and workflow contracts exist; external GitHub repository administration and production environment controls must be verified separately. |
| 17. Code quality / dependency governance | **Strong foundation** | Rust workspace policy, dependency direction, unsafe-code prohibition and lockfile/toolchain discipline are present. |
| 18. Domain/business completeness | **Early platform foundation** | Tenancy/IAM is real; catalogue/inventory/sales/payment/reconciliation/reporting/vertical engines remain largely contractual. |
| 19. Production readiness | **Not ready** | Durable business state, complete API/security enforcement, asynchronous processing, integrations, clients and end-to-end evidence are incomplete. |

---

# 4. Critical Documentation/Implementation Divergences

## 4.1 Axum/Hyper/Tokio transport reconciliation

The governing architecture is Rust + Axum + Hyper/Hyper-util + Tokio, and the transport implementation follows that boundary:

- `apps/api/src/main.rs` owns the Tokio runtime and `tokio::net::TcpListener` lifecycle;
- `apps/api/src/serve.rs` owns the Axum `Router` and request extraction;
- Hyper-util owns HTTP/1/HTTP/2 connection serving and protocol-level transport configuration;
- `TowerToHyperService` explicitly adapts Axum/Tower's service trait to Hyper's service trait;
- validated request-body, header-read and keepalive settings are applied at the transport boundary;
- a Tokio semaphore separately caps accepted connection tasks at `MAX_IN_FLIGHT_CONNECTIONS`;
- `GlobalConcurrencyLimitLayer` separately caps in-flight application requests across cloned router services;
- the previous hand-written `TcpStream` HTTP parser has been removed;
- tenancy integration tests exercise the production Axum router through an in-process adapter.

Audit disposition:

> **Transport architecture reconciled. The remaining important distinction is that TCP connection capacity and application-request capacity are separate controls and are now bounded independently.**

## 4.2 Tenancy and authorization are no longer empty

The older audit described `sitolo-tenancy` and `sitolo-authz` as empty stubs. That is no longer true.

Current source includes:

- tenant/organization/branch scope resolution;
- trusted-vs-requested identifier separation;
- permission and role catalogs;
- assignment lifecycle;
- scope grants;
- invitation state;
- application-level tenancy services.

Disposition:

> **Old audit finding is historical and must not be used as a current implementation description.**

---

## 4.3 PostgreSQL work is more advanced than the old audit stated, but not complete

Current persistence contains PostgreSQL authority/runtime code including separate administrative/runtime pools and runtime-role checks.

This is not equivalent to full Phase 5 completion.

Disposition:

> **Partial. Preserve the distinction between PostgreSQL runtime authority infrastructure and the complete application schema/migrations/RLS business persistence contract.**

---

## 4.4 Workers, events, integrations and synchronization remain genuine gaps

The current worker binary explicitly identifies itself as a Phase 1 scaffold and contains no worker loop. The events, integrations and sync crates remain contract boundaries without substantive runtime implementations.

Disposition:

> **Future-phase implementation gaps, not stale documentation.**

---

# 5. Production Readiness Gates

Before Sitolo can be called production-ready, the following chain must be executable and tested:

```text
Authenticated principal
    ↓
Session/device state
    ↓
Tenant / organization / branch scope
    ↓
Authorization policy
    ↓
Domain command
    ↓
PostgreSQL transaction
    ↓
Durable audit
    ↓
Transactional outbox
    ↓
Worker / integration side effect
    ↓
Reconciliation
    ↓
Observable / recoverable result
```

Every boundary needs negative tests, failure semantics and recovery evidence.

---

# 6. Documentation Corpus Findings

## 6.1 Historical audit drift

`docs/phase4_part5_to_phase0_enterprise_audit_remediation_plan.md` identifies Phase 4 Part 5 / PR-005 as the current repository position. It is now historical because later Phase 4 work exists.

Disposition: retain as historical evidence and remove it from current-state routing.

## 6.2 Phase contracts must not masquerade as implementation evidence

Phase 8, 9 and 10 documents define future/current implementation requirements. Imperative language inside a contract does not prove the feature exists.

Examples:

- “API exposes...” should be read as a requirement when the corresponding feature is not implemented.
- `[X]` acceptance markers are evidence only when backed by executable evidence.
- A detailed SQL design does not prove that migration/application code exists.

Disposition: annotate contracts with clear `Target`, `Implemented`, `Verified`, and `Historical` semantics where ambiguity exists.

## 6.3 External facts require dated verification

Current repository sources correctly pin Rust to 1.98.1 and PostgreSQL to the 18.x family. Rust 1.98.1 was released on 2026-09-03, and PostgreSQL 18.6 is the current PostgreSQL 18 release listed by the PostgreSQL project as of the audit date. citeturn544987search0turn544987search2turn544987search6

SLSA references should remain on v1.2 unless the official project changes the active specification.

MRA/EIS regulatory facts require the latest MRA primary-source verification before external compliance claims are published. The live MRA developer documentation currently exposes the EIS API/comparison material. citeturn689436search4

---

# 7. Documentation Remediation Priorities

| Priority | Target | Required action |
|---|---|---|
| P0 | `enterprise_audit_and_review.md` | Keep this current-state audit aligned with source evidence |
| P0 | Axum/API divergence | Explicitly resolve implementation-vs-contract status |
| P1 | `phase8_product_catalogue_implementation.md` | Separate normative future requirement from current implementation |
| P1 | `phase4_part5_to_phase0_enterprise_audit_remediation_plan.md` | Mark historical and exclude from current-state routing |
| P1 | `icm_reference_integrity.md` | Convert from missing-file record into standing semantic-integrity policy |
| P1 | `agentic_workflow.md` | Update integrity/status language after missing-reference repair |
| P1 | Commercial EIS language | Remove current-sounding transition language; retain dated history only |
| P1 | External-version references | Add dated verification metadata and official-source authority |
| P2 | Phase 5–10 contracts | Audit requirement/implementation/verification state systematically |
| P2 | System Map processes | Promote only when source evidence is sufficient |
| P2 | Final reference inventory | Require zero missing/ambiguous internal references |

---

# 8. Final Assessment

Sitolo has crossed the line from an empty architectural scaffold into a **substantial security-oriented platform foundation**, especially around identity, tenancy, IAM, authorization vocabulary, scope resolution, persistence authority primitives, transport validation and observability.

It has **not** crossed the line into a complete enterprise business operating system.

The principal work remaining is execution of the already-defined authority chain:

```text
CONTRACTS
   ↓
REAL DOMAIN ENGINES
   ↓
REAL POSTGRESQL BUSINESS PERSISTENCE
   ↓
REAL AUTHORIZATION ENFORCEMENT
   ↓
REAL ASYNC / OUTBOX
   ↓
REAL EXTERNAL INTEGRATIONS
   ↓
REAL OFFLINE CLIENTS + SYNC
   ↓
END-TO-END SECURITY / RECOVERY / LOAD EVIDENCE
```

The documentation system must reflect that distinction exactly. No document should claim a capability is implemented simply because a contract for it exists.
