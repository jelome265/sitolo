# Sitolo Workspace Domain Architecture

**Status:** Current organizational ICM routing architecture
**Baseline:** main at `b633e5e296eb56e138dbf74730844c77d2f464d4`
**Purpose:** define which organizational work domains receive first-class ICM workspaces without inventing unsupported departments.

## Decision

Sitolo currently has five first-class organizational workspaces:

```text
Engineering
Product
Commercial
Customer Operations
Trust & Compliance
```

Engineering already existed as `workspaces/sitolo-engineering/`. The other four are routed workspaces for distinct decision/knowledge domains evidenced by the existing corpus.

## Evidence basis

### Engineering

Evidence: `agent.md`, `docs/system_architecture_design.md`, phase implementation contracts, CI policy and the existing nine-stage ICM pipeline.

Owns technical architecture, implementation, testing, deployment, security engineering and technical verification.

### Product

Evidence: `docs/sitolo.md`, `docs/commercial/01_strategy/business_model_design.md`, `docs/commercial/01_strategy/segment_strategy_and_duka_economics.md`, `docs/commercial/05_lifecycle_service/onboarding_and_migration_strategy.md`, and the Business Intelligence/Action model.

Owns product scope, actors, product behavior, capability decisions, product experience and product-level requirements.

### Commercial

Evidence: `docs/commercial/README.md`, commercial governance, strategy, validation, pricing, acquisition/distribution, lifecycle economics and payments/finance corpus.

Owns market/segment strategy, validation, pricing/packaging, acquisition/distribution, monetization and commercial economics.

`Strategy` is not a separate workspace because the repository explicitly places strategy inside the commercial strategy layer.

### Customer Operations

Evidence: onboarding/migration, retention/churn/lifecycle, support/service operations and trust/adoption/operational-continuity documents. The onboarding document explicitly names product, growth, onboarding, support and implementation as its primary audience.

Owns onboarding, migration, support, service operations, lifecycle and operational customer continuity.

### Trust & Compliance

Evidence: regulatory perimeter, tax/fiscal readiness, fraud/loss prevention, data portability/governance, current regulatory verification and security/compliance documents.

Owns regulatory/legal boundary analysis, privacy/data governance, risk/control requirements and compliance routing.

## Explicitly not first-class workspaces yet

### Finance

The current corpus uses “finance” primarily as a customer actor, commercial/payment domain, reconciliation concern or enterprise operating function. It does not establish a distinct Sitolo internal corporate-finance operating corpus. A Finance workspace should be created only when such an authoritative internal-finance domain exists.

### Strategy

Strategy is already a named commercial strategy layer. Creating a second strategy workspace would split one authority domain without evidence of separate ownership.

### Business Intelligence

The current intelligence corpus defines BI as a product value layer and commercial defensibility concern. It is therefore routed to Product and/or Commercial rather than made into a standalone organizational workspace.

### Payments

Payments cross Commercial, Trust & Compliance and Engineering: monetization, regulated boundary and technical integration. A standalone Payments workspace would create a fourth competing owner.

## Workspace rules

1. Workspace names describe decision ownership, not every subject mentioned in the corpus.
2. Each task has one primary workspace.
3. Cross-domain work is explicit and routed; it must not duplicate authority.
4. A workspace router is navigation, not source truth.
5. Adding a new first-class workspace requires evidence of a distinct owner, decision set, artifacts and lifecycle.

## ICM workspace conformance contract

The organizational workspaces use two deliberate modes:

- **Routing-only:** Product, Commercial, Customer Operations, and Trust & Compliance. These are organizational dispatch boundaries. They do not invent stage pipelines or Layer 4 shelves where no executable workflow is evidenced.
- **Executable:** Engineering. This workspace owns the existing nine-stage ICM pipeline and therefore carries the executable stage-contract obligations.

Every registered workspace must expose, through its routing/context surfaces:

1. an explicit entry condition and primary owner;
2. scoped inputs identifying the canonical sources required for the domain decision;
3. explicit cross-domain exits;
4. a defined primary workflow destination.

Every executable stage must additionally expose the ICM execution contract: scoped Inputs, one job in Process, Outputs, Layer 3 reference material, Layer 4 output/handoff structure, a human verification/checkpoint surface, and one-way stage dependencies. The controlled human re-entry from remediation to audit is a governance instruction, not an executable dependency edge.

Mechanical validation is performed by `scripts/ci/check-icm-workspace-contracts` and the main `scripts/ci/check-icm-workspace` verifier. The validators check reachability from the organizational router, valid cross-domain targets, workspace mode consistency, Layer 3 presence, Layer 4 handoffs, and forward-only executable-stage references.

## Relationship to the product domain model

These workspaces are an **organizational work routing model**, not a DDD bounded-context map and not a replacement for the product/domain model. Product bounded contexts remain governed by the technical/domain corpus.
