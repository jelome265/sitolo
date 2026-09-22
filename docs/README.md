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

The commercial corpus is physically grouped under [`commercial/`](commercial/). Grouping is non-destructive: documents remain separate and retain their individual domain boundaries. The commercial corpus audit and precision/consistency contract define authority; the directory structure defines navigation only.

| Group | Scope |
|---|---|
| [`commercial/00_governance/`](commercial/00_governance/) | Corpus authority, precision, metrics and decision governance |
| [`commercial/01_strategy/`](commercial/01_strategy/) | Business model, operating model, segmentation, verticals and country expansion |
| [`commercial/02_validation/`](commercial/02_validation/) | Commercial experiments and offline validation |
| [`commercial/03_pricing_packaging/`](commercial/03_pricing_packaging/) | Buyer/payer process, packaging and tier migration |
| [`commercial/04_acquisition_distribution/`](commercial/04_acquisition_distribution/) | Channels, partners, agents, CAC and contribution economics |
| [`commercial/05_lifecycle_service/`](commercial/05_lifecycle_service/) | Onboarding, retention, support, trust, portability and continuity |
| [`commercial/06_payments_finance/`](commercial/06_payments_finance/) | Subscription billing, mobile-money reconciliation, credit and lay-by |
| [`commercial/07_trust_compliance/`](commercial/07_trust_compliance/) | Regulatory, fiscal/tax and fraud-control boundaries |
| [`commercial/08_intelligence_defensibility/`](commercial/08_intelligence_defensibility/) | Business intelligence, actioning and defensibility |

### Commercial corpus index

See [`commercial/README.md`](commercial/README.md) for the complete grouped-document map, scope definitions, and migration policy.
