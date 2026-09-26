# Sitolo — Documentation Index

Source of authority: [`agent.md`](../agent.md) at the repository root. When documents conflict, apply the source hierarchy defined there.

## Governing / current-state documents

| Document | Purpose |
|---|---|
| [`agent.md`](../agent.md) | Repository-wide engineering governance contract |
| [`enterprise_audit_and_review.md`](enterprise_audit_and_review.md) | **Current-state** enterprise architecture, security, reliability and readiness assessment |
| [`documentation_semantic_remediation_plan.md`](documentation_semantic_remediation_plan.md) | Current semantic documentation audit/remediation plan |
| [`icm_reference_integrity.md`](icm_reference_integrity.md) | Standing Markdown reference-integrity and authority-classification policy |
| [`agentic_workflow.md`](agentic_workflow.md) | Filesystem-routed ICM engineering workflow |
| [`ADR-001-025.md`](ADR-001-025.md) | Architecture decision records |
| [`implementation_plan.md`](implementation_plan.md) | Implementation program, dependency order and release posture |
| [`phase_contract_coverage_register.md`](phase_contract_coverage_register.md) | **Authoritative phase 0–20 contract coverage/reconciliation register** |
| [`current_implementation_status.md`](current_implementation_status.md) | Current product phase and active engineering-stream pointer |

## Historical / compatibility documents

| Document | Status |
|---|---|
| [`phase0_corrections_errata.md`](phase0_corrections_errata.md) | Historical dated correction record; re-verify external facts before using them as current |
| [`phase4_part5_to_phase0_enterprise_audit_remediation_plan.md`](phase4_part5_to_phase0_enterprise_audit_remediation_plan.md) | Historical Phase 4 Part 5 remediation artifact |
| [`phase0-2_implementation_audit_and_remediation.md`](phase0-2_implementation_audit_and_remediation.md) | Historical Phase 0–2 / Phase 3-readiness audit |
| [`phase4_part8_pr009_remediation_implementation_instructions.md`](phase4_part8_pr009_remediation_implementation_instructions.md) | Superseded PR-009 review; historical evidence only |
| [`phase4_part8_pr009_current_status.md`](phase4_part8_pr009_current_status.md) | Current PR-009 status pointer and closure gates |
| [`business_model_design.md`](business_model_design.md) | Compatibility entry only; canonical business model lives in `commercial/01_strategy/business_model_design.md` |

## Core design corpus

| Document | Purpose |
|---|---|
| [`sitolo.md`](sitolo.md) | Product/domain specification baseline |
| [`system_architecture_design.md`](system_architecture_design.md) | Target technical architecture and implementation blueprint |
| [`domain_model.md`](domain_model.md) | Domain semantics, aggregates, invariants and state transitions |
| [`database_design.md`](database_design.md) | PostgreSQL schema/data design |
| [`api_contract.md`](api_contract.md) | API target contract |
| [`auth_authorization_spec.md`](auth_authorization_spec.md) | Authentication/authorization target contract |
| [`sync_protocol.md`](sync_protocol.md) | Offline synchronization target contract |
| [`deployment_spec.md`](deployment_spec.md) | Deployment and operational target contract |
| [`observability_spec.md`](observability_spec.md) | Telemetry and observability target contract |
| [`testing_strategy.md`](testing_strategy.md) | Verification strategy |

## Security

| Document | Purpose |
|---|---|
| [`../workspaces/sitolo-engineering/skills/security-engineering/SKILL.md`](../workspaces/sitolo-engineering/skills/security-engineering/SKILL.md) | Defensive security engineering, real security tests, vulnerability scanning and evidence |
| [`../workspaces/sitolo-engineering/skills/offensive-security/SKILL.md`](../workspaces/sitolo-engineering/skills/offensive-security/SKILL.md) | Authorized adversarial security testing, exploit evidence, remediation and regression |
| [`threat_model.md`](threat_model.md) | Threat model and security governance |
| [`security_architecture_design.md`](security_architecture_design.md) | Security architecture baseline |
| [`security_implementation_spec.md`](security_implementation_spec.md) | Implementation-level security controls |
| [`security_test_harness.md`](security_test_harness.md) | Security verification harness |
| [`ci_enforcement.md`](ci_enforcement.md) | CI/release enforcement contract |
| [`security_control_register.md`](security_control_register.md) | Security control traceability, ownership, implementation and evidence |
| [`security_incident_response.md`](security_incident_response.md) | Security incident response and recovery contract |
| [`security_exception_register.md`](security_exception_register.md) | Security exception and risk-acceptance governance |
| [`security_cryptography_and_key_management.md`](security_cryptography_and_key_management.md) | Cryptography and key lifecycle control standard |

## Phase implementation contracts

Phase documents are normative requirements and implementation sequencing artifacts. They are **not proof that the described capability exists**.

| Document | Scope |
|---|---|
| [`phase1_repository_rust_workspace_ci_deep_implementation.md`](phase1_repository_rust_workspace_ci_deep_implementation.md) | Repository/workspace/CI foundation |
| [`phase2_config_secrets_logging_errors_telemetry_implementation.md`](phase2_config_secrets_logging_errors_telemetry_implementation.md) | Runtime config/secrets/errors/telemetry |
| [`phase3_identity_sessions_mfa_device_identity_implementation.md`](phase3_identity_sessions_mfa_device_identity_implementation.md) | Identity/session/MFA/device identity |
| [`phase4_tenant_organization_branch_iam_implementation.md`](phase4_tenant_organization_branch_iam_implementation.md) | Tenant/org/branch/IAM |
| [`phase5_postgresql_schema_migrations_constraints_rls_implementation.md`](phase5_postgresql_schema_migrations_constraints_rls_implementation.md) | PostgreSQL schema/migrations/constraints/RLS |
| [`phase6_authorization_engine_policy_enforcement_implementation.md`](phase6_authorization_engine_policy_enforcement_implementation.md) | Authorization engine/policy enforcement |
| [`phase7_security_test_framework_implementation.md`](phase7_security_test_framework_implementation.md) | Security test framework target/implementation contract |
| [`phase8_product_catalogue_implementation.md`](phase8_product_catalogue_implementation.md) | Product catalogue target-state contract |
| [`phase9_inventory_ledger_implementation.md`](phase9_inventory_ledger_implementation.md) | Inventory ledger target/implementation contract |
| [`phase10_pos_sales_implementation.md`](phase10_pos_sales_implementation.md) | POS/sales target/implementation contract |
| [`phase11_payments_reconciliation_implementation.md`](phase11_payments_reconciliation_implementation.md) | Payments/reconciliation implementation contract |
| [`phase12_offline_synchronization_implementation.md`](phase12_offline_synchronization_implementation.md) | Offline synchronization implementation contract |
| [`phase13_procurement_suppliers_implementation.md`](phase13_procurement_suppliers_implementation.md) | Procurement/suppliers implementation contract |
| [`phase14_returns_refunds_cash_implementation.md`](phase14_returns_refunds_cash_implementation.md) | Returns/refunds/cash implementation contract |
| [`phase15_mra_eis_implementation.md`](phase15_mra_eis_implementation.md) | MRA EIS implementation/certification contract |
| [`phase16_reporting_exports_implementation.md`](phase16_reporting_exports_implementation.md) | Reporting/exports implementation contract |
| [`phase17_billing_entitlements_implementation.md`](phase17_billing_entitlements_implementation.md) | Billing/entitlements implementation contract |
| [`phase18_admin_support_implementation.md`](phase18_admin_support_implementation.md) | Admin/support implementation contract |
| [`phase19_hardening_performance_dr_implementation.md`](phase19_hardening_performance_dr_implementation.md) | Hardening/performance/disaster-recovery implementation contract |
| [`phase20_production_certification_implementation.md`](phase20_production_certification_implementation.md) | Production certification implementation contract |

Completed PR-specific contracts for Phase 4 Parts 6 and 7 are historical implementation records; active work is represented by the current phase/PR status documents and the Part 8 binding contract. Additional Phase 2 and Phase 4 remediation/execution contracts remain in this directory. Their status must be read from metadata and current implementation evidence, not inferred from filename or existence. PR-specific review documents are snapshots and must be checked against the current PR head.

## Active Phase 4 Part 8 work

| Document | Purpose |
|---|---|
| [`phase4_part8_audit_outbox_implementation_contract.md`](phase4_part8_audit_outbox_implementation_contract.md) | Binding Part 8 audit/outbox target contract |
| [`phase4_part8_pr009_current_status.md`](phase4_part8_pr009_current_status.md) | Current PR-009 implementation/status snapshot |
| [`phase4_part8_pr009_remediation_implementation_instructions.md`](phase4_part8_pr009_remediation_implementation_instructions.md) | Superseded historical PR-009 review |
| [`phase4_part8_pr009_current_status.md`](phase4_part8_pr009_current_status.md) | Current PR-009 implementation/status snapshot |

## External standards

| Document | Purpose |
|---|---|
| [`external_standards_verification_register.md`](external_standards_verification_register.md) | Dated version/authority verification for external engineering standards |

## External integrations

| Document | Purpose |
|---|---|
| [`payment_integration_spec.md`](payment_integration_spec.md) | Payment-provider integration contract |
| [`mra_eis_integration_spec.md`](mra_eis_integration_spec.md) | MRA EIS integration contract |

## Organizational ICM workspaces

The organizational workspace routing is defined by [`workspace_domain_architecture.md`](workspace_domain_architecture.md). The workspace tree is a work-domain router, not a duplicate product/domain model.

| Workspace | Scope |
|---|---|
| [`../workspaces/sitolo-engineering/`](../workspaces/sitolo-engineering/) | Engineering implementation, audit and verification |
| [`../workspaces/sitolo-product/`](../workspaces/sitolo-product/) | Product scope, actors, behavior and product requirements |
| [`../workspaces/sitolo-commercial/`](../workspaces/sitolo-commercial/) | Strategy, validation, pricing, acquisition, monetization and commercial economics |
| [`../workspaces/sitolo-customer-operations/`](../workspaces/sitolo-customer-operations/) | Onboarding, support, lifecycle and service operations |
| [`../workspaces/sitolo-trust-compliance/`](../workspaces/sitolo-trust-compliance/) | Regulatory, privacy, risk, fraud and compliance routing |

## Commercial operating model

The commercial corpus is physically grouped under [`commercial/`](commercial/). Grouping is navigation, not authority; the canonical source remains each document's declared role and the commercial governance contracts.

| Group | Scope |
|---|---|
| [`commercial/00_governance/`](commercial/00_governance/) | Corpus authority, precision, metrics and decision governance |
| [`commercial/01_strategy/`](commercial/01_strategy/) | Business model, operating model, segmentation, verticals and country expansion |
| [`commercial/02_validation/`](commercial/02_validation/) | Commercial experiments and validation |
| [`commercial/03_pricing_packaging/`](commercial/03_pricing_packaging/) | Buyer, packaging and tier migration |
| [`commercial/04_acquisition_distribution/`](commercial/04_acquisition_distribution/) | Channels, partners, CAC and contribution economics |
| [`commercial/05_lifecycle_service/`](commercial/05_lifecycle_service/) | Onboarding, retention, support, trust and continuity |
| [`commercial/06_payments_finance/`](commercial/06_payments_finance/) | Subscription billing, mobile-money reconciliation, credit and lay-by |
| [`commercial/07_trust_compliance/`](commercial/07_trust_compliance/) | Regulatory, fiscal/tax and fraud-control boundaries |
| [`commercial/08_intelligence_defensibility/`](commercial/08_intelligence_defensibility/) | Business intelligence, actioning and defensibility |

See [`commercial/README.md`](commercial/README.md) for the grouped-document map.

## Repository snapshots and runbooks

`release-governance-evidence.md` is a dated repository-administration snapshot and must be refreshed before production certification.

`repository-topology.md` records intentional omissions from the target repository structure; absence of a listed directory is not itself a defect.

Runbooks live under [`runbooks/`](runbooks/) and describe operational failure response.

## Interpretation rule

A document can be:

- **Current**: describes the current repository or current policy.
- **Target/Contract**: specifies required future or phase behavior.
- **Historical**: records an earlier state or dated evidence.
- **Compatibility**: preserves a legacy path without becoming a second source.

The documentation audit must keep those states explicit.