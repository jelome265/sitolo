# Sitolo Trust & Compliance Context

## Inputs

| Source | File/Location | Scope | Why |
|---|---|---|---|
| Regulatory perimeter | ../../docs/commercial/07_trust_compliance/regulatory_perimeter_and_compliance_boundary.md | Full file when needed | Legal/regulatory classification boundary |
| Tax/fiscal readiness | ../../docs/commercial/07_trust_compliance/tax_and_fiscal_readiness_strategy.md | Relevant sections | Fiscal obligations and readiness |
| Fraud/business control | ../../docs/commercial/07_trust_compliance/fraud_loss_prevention_and_business_control.md | Relevant sections | Business control and fraud-loss model |
| Data governance | ../../docs/commercial/05_lifecycle_service/data_portability_and_data_governance.md | Relevant sections | Privacy, portability and data governance |
| Regulatory verification | ../../docs/commercial/00_governance/current_regulatory_facts_and_verification.md | Full file when current facts matter | Dated external facts |
| Security architecture | ../../docs/security_architecture_design.md; ../../docs/security_implementation_spec.md; ../../docs/threat_model.md | Relevant sections | Security/control boundary |

## Process

1. Identify the obligation, jurisdiction and affected capability.
2. Load the dated authority or governing contract first.
3. Separate legal fact, documented policy, implementation evidence and unresolved legal characterization.
4. Record approvals, owners, verification date and re-verification trigger where applicable.
5. Route technical remediation to Engineering without weakening the governing requirement.

## Output

Compliance assessment, risk decision, control requirement, regulatory verification record or incident-routing artifact.

## Do NOT load

Do not treat commercial assumptions as legal facts. Do not load unrelated engineering implementation detail unless validating a specific control.
