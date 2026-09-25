# Sitolo — Documentation Index

Source of authority: [`agent.md`](../agent.md) at the repository root. When documents conflict, apply the source hierarchy defined there.

## Governing / program

| Document | Purpose |
|---|---|
| [`agent.md`](../agent.md) | Repository-wide engineering governance contract |
| [`ADR-001-025.md`](ADR-001-025.md) | Architecture decision records |
| [`implementation_plan.md`](implementation_plan.md) | Implementation program overview and sequencing |
| [`enterprise_audit_and_review.md`](enterprise_audit_and_review.md) | Enterprise architecture/security/readiness audit |
| [`phase0_corrections_errata.md`](phase0_corrections_errata.md) | Verified corrections and errata |
| [`agentic_workflow.md`](agentic_workflow.md) | ICM engineering workflow |
| [`icm_reference_integrity.md`](icm_reference_integrity.md) | Documentation reference-integrity findings and repair policy |

## Restored authority paths

| Document | Purpose |
|---|---|
| [`sitolo.md`](sitolo.md) | Product/domain specification baseline |
| [`business_model_design.md`](business_model_design.md) | Compatibility entry to the canonical commercial business model |
| [`security_architecture_design.md`](security_architecture_design.md) | Security architecture baseline |
| [`auth_authorization_spec.md`](auth_authorization_spec.md) | Authentication/authorization contract |
| [`ci_enforcement.md`](ci_enforcement.md) | CI/release enforcement contract |

## Design corpus

| Document | Purpose |
|---|---|
| [`system_architecture_design.md`](system_architecture_design.md) | System architecture invariants |
| [`domain_model.md`](domain_model.md) | Domain model and semantics |
| [`database_design.md`](database_design.md) | PostgreSQL schema and data design |
| [`api_contract.md`](api_contract.md) | Public API contract |
| [`sync_protocol.md`](sync_protocol.md) | Offline synchronization protocol |
| [`deployment_spec.md`](deployment_spec.md) | Deployment and operations |
| [`observability_spec.md`](observability_spec.md) | Logging, metrics, tracing |
| [`testing_strategy.md`](testing_strategy.md) | Testing strategy |

## Security

| Document | Purpose |
|---|---|
| [`threat_model.md`](threat_model.md) | Threat model |
| [`security_architecture_design.md`](security_architecture_design.md) | Security architecture |
| [`security_implementation_spec.md`](security_implementation_spec.md) | Security implementation |
| [`security_test_harness.md`](security_test_harness.md) | Security test harness |
| [`auth_authorization_spec.md`](auth_authorization_spec.md) | Identity and authorization contract |
| [`ci_enforcement.md`](ci_enforcement.md) | CI security/release enforcement |

## Phase implementation specifications

The phase documents remain implementation contracts. The restored Phase 8 catalogue contract is:

- [`phase8_product_catalogue_implementation.md`](phase8_product_catalogue_implementation.md)

Existing Phase 1–7 and Phase 9–10 contracts remain listed in their respective corpus sections and must not be replaced by this index.

## Commercial Operating Model

The commercial corpus is physically grouped under [`commercial/`](commercial/). The canonical business model is [`commercial/01_strategy/business_model_design.md`](commercial/01_strategy/business_model_design.md). The root `business_model_design.md` exists only for compatibility with older references.

See [`commercial/README.md`](commercial/README.md) for the complete grouped-document map, scope definitions, and migration policy.