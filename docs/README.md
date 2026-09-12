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
