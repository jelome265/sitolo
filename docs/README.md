# Sitolo — Documentation Index

Source of authority: [`agent.md`](../agent.md) at the repository root. When documents conflict, apply the source hierarchy defined there.

## Governing / program

| Document | Purpose |
|---|---|
| [`agent.md`](../agent.md) | Repository-wide engineering governance contract for AI coding agents and contributors |
| [`ADR-001-025.md`](ADR-001-025.md) | Architecture decision records |
| [`implementation_plan.md`](implementation_plan.md) | Implementation program overview and sequencing |
| [`enterprise_audit_and_review.md`](enterprise_audit_and_review.md) | Canonical enterprise architecture, security, reliability, and readiness audit |
| [`phase0_corrections_errata.md`](phase0_corrections_errata.md) | Verified corrections and errata for the design corpus (2026-09-05) |

## Design corpus

| Document | Purpose |
|---|---|
| [`business_model_design.md`](business_model_design.md) | Product and business model |
| [`commercial_validation_plan.md`](commercial_validation_plan.md) | Executable commercial validation, pricing experiments, CAC measurement, segment-aware PMF gates, and commercial decision rules |
| [`segment_strategy_and_duka_economics.md`](segment_strategy_and_duka_economics.md) | Universal Duka-to-enterprise market scope, segment economics, packaging, GTM, support, and progressive-complexity doctrine |
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
| [`security_architecture_design.md`](security_architecture_design.md) | Security architecture design |
| [`security_implementation_spec.md`](security_implementation_spec.md) | Security implementation specification |
| [`security_test_harness.md`](security_test_harness.md) | Security test harness design |

## External integrations

| Document | Purpose |
|---|---|
| [`payment_integration_spec.md`](payment_integration_spec.md) | Payment provider integration |
| [`mra_eis_integration_spec.md`](mra_eis_integration_spec.md) | MRA EIS tax integration |

## Phase implementation specifications

| Document | Scope |
|---|---|
| [`phase1_repository_rust_workspace_ci_deep_implementation.md`](phase1_repository_rust_workspace_ci_deep_implementation.md) | Repository, Rust workspace, CI, supply-chain and build governance |
| [`phase2_config_secrets_logging_errors_telemetry_implementation.md`](phase2_config_secrets_logging_errors_telemetry_implementation.md) | Config, secrets, logging, errors, telemetry |
| [`phase2_appconfig_fingerprint_coverage_and_configuration_contract_remediation.md`](phase2_appconfig_fingerprint_coverage_and_configuration_contract_remediation.md) | Phase 2 configuration contract and fingerprint remediation |
| [`phase2_ci_sha256_fingerprint_lowerhex_failure_remediation.md`](phase2_ci_sha256_fingerprint_lowerhex_failure_remediation.md) | Phase 2 SHA-256 compatibility remediation |
| [`phase2_postgresql_runtime_configuration_persistence_boundary_implementation.md`](phase2_postgresql_runtime_configuration_persistence_boundary_implementation.md) | Phase 2 PostgreSQL runtime boundary |
| [`postgresql-runtime-development.md`](postgresql-runtime-development.md) | Safe local PostgreSQL runtime-development path |
| [`phase3_identity_sessions_mfa_device_identity_implementation.md`](phase3_identity_sessions_mfa_device_identity_implementation.md) | Identity, sessions, MFA, device identity |
| [`phase4_tenant_organization_branch_iam_implementation.md`](phase4_tenant_organization_branch_iam_implementation.md) | Tenant, organization, branch, IAM |
| [`phase5_postgresql_schema_migrations_constraints_rls_implementation.md`](phase5_postgresql_schema_migrations_constraints_rls_implementation.md) | PostgreSQL schema, migrations, constraints, RLS |
| [`phase6_authorization_engine_policy_enforcement_implementation.md`](phase6_authorization_engine_policy_enforcement_implementation.md) | Authorization engine and policy enforcement |
| [`phase7_security_test_framework_implementation.md`](phase7_security_test_framework_implementation.md) | Security test framework |

Note: `onstreams.apk` is a reference artifact present in this directory, not part of the documentation corpus.

## Commercial Operating Model

The commercial operating model is decomposed into explicit documents so each economic, distribution, trust, lifecycle, regulatory, and defensibility boundary can be reviewed independently.

| Document | Purpose |
|---|---|
| [`commercial_operating_model.md`](commercial_operating_model.md) | Umbrella commercial operating model: buyer/user separation, payments, distribution, onboarding, trust, retention, regulation, economics, moat and expansion |
| [`buyer_user_approver_and_purchase_process.md`](buyer_user_approver_and_purchase_process.md) | Economic buyer, user, approver, payer and purchase-process model |
| [`payment_collections_and_subscription_billing.md`](payment_collections_and_subscription_billing.md) | Subscription billing, payment methods, collection, failures, grace, suspension and settlement/reconciliation |
| [`distribution_channel_and_market_access_strategy.md`](distribution_channel_and_market_access_strategy.md) | Direct, referral, accountant, distributor, agent, reseller and geographic distribution strategy |
| [`partner_agent_economics_and_governance.md`](partner_agent_economics_and_governance.md) | Partner roles, commissions, data boundaries, fraud controls and channel economics |
| [`onboarding_and_migration_strategy.md`](onboarding_and_migration_strategy.md) | Zero-data onboarding, opening state, paper migration, CSV migration and enterprise cutover |
| [`trust_adoption_and_operational_continuity.md`](trust_adoption_and_operational_continuity.md) | Business trust, auditability, device loss, recovery, privacy and operational continuity |
| [`offline_continuity_commercial_validation.md`](offline_continuity_commercial_validation.md) | Commercial validation of prolonged offline operation and convergence after reconnect |
| [`retention_churn_and_customer_lifecycle.md`](retention_churn_and_customer_lifecycle.md) | Lifecycle, churn taxonomy, cohort rules, reactivation and seasonal inactivity |
| [`support_economics_and_service_operations.md`](support_economics_and_service_operations.md) | Support tiers, support cost, severity, self-service and capacity economics |
| [`regulatory_perimeter_and_compliance_boundary.md`](regulatory_perimeter_and_compliance_boundary.md) | Green/yellow/red regulatory boundary and release/marketing controls |
| [`tax_and_fiscal_readiness_strategy.md`](tax_and_fiscal_readiness_strategy.md) | Tax-status, fiscal data, pricing semantics and EIS readiness |
| [`data_portability_and_data_governance.md`](data_portability_and_data_governance.md) | Export, retention, deletion, data dictionary and enterprise data-governance expectations |
| [`fraud_loss_prevention_and_business_control.md`](fraud_loss_prevention_and_business_control.md) | Employee fraud, cash/stock variance, approvals, audit and anomaly-control model |
| [`vertical_module_commercial_model.md`](vertical_module_commercial_model.md) | Shared core + vertical modules + plan entitlements without product forks |
| [`upgrade_expansion_and_tier_migration_model.md`](upgrade_expansion_and_tier_migration_model.md) | Complexity-based upgrade triggers, expansion, contraction, downgrade and safe migration |
| [`unit_economics_cac_and_contribution_model.md`](unit_economics_cac_and_contribution_model.md) | Revenue, variable cost, contribution, CAC, payback and segment/channel economics |
| [`defensibility_and_moat_strategy.md`](defensibility_and_moat_strategy.md) | Trust, distribution, workflow depth, historical context, integrations and intelligence as earned defensibility |
| [`geographic_expansion_and_country_entry_strategy.md`](geographic_expansion_and_country_entry_strategy.md) | Country-entry evidence, localization, regulatory/payment/tax adaptation and expansion gates |
| [`mobile_money_reconciliation_business_model.md`](mobile_money_reconciliation_business_model.md) | Mobile-money/business reconciliation as a merchant-control and retention layer |
| [`credit_and_layby_commercial_model.md`](credit_and_layby_commercial_model.md) | Merchant-recorded receivables/lay-by while preserving the lending regulatory boundary |
| [`business_intelligence_alerts_and_action_model.md`](business_intelligence_alerts_and_action_model.md) | Record → understand → detect → alert → recommend → act intelligence layer |
| [`commercial_metrics_governance_and_decision_system.md`](commercial_metrics_governance_and_decision_system.md) | Commercial metric definitions, segmentation, experiment governance and decision logging |
