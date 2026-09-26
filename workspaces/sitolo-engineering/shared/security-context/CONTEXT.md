# Security Context Router

Stable security Layer 3 context.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Threat model | ../../../../docs/threat_model.md | Relevant scenarios | Attacker model |
| Security implementation | ../../../../docs/security_implementation_spec.md | Full file when security-sensitive | Enforceable controls |
| Security test harness | ../../../../docs/security_test_harness.md | Relevant controls and negative tests | Proof requirements |
| Phase 1 repository security | ../../../../docs/phase1_repository_rust_workspace_ci_deep_implementation.md | Supply-chain and CI sections | Build/release trust |
| Regulatory perimeter | ../../../../docs/commercial/07_trust_compliance/regulatory_perimeter_and_compliance_boundary.md | §§5-16 when relevant | Regulatory control boundary |

| External standards | ../../../../docs/external_standards_verification_register.md | Full file when version/standard claims matter | Dated external authority |

## Mandatory security engineering skills

For security-relevant work, load and apply `../../skills/security-engineering/SKILL.md`. The defensive skill governs real security implementation, vulnerability scanning, negative/security testing, remediation and security evidence.

When the work explicitly challenges, attempts to bypass, or validates resistance of an implemented security control, also load and apply `../../skills/offensive-security/SKILL.md`. The offensive skill governs authorized adversarial testing, scope gates, exploit evidence, regression enforcement, safe failure and defensive handoff.

The offensive skill does not authorize production or third-party testing by itself and does not replace the security-control register or higher-authority contracts.

## Security applicability rule

When Stage 01 marks `security_relevant: yes`, the selected control IDs are carried through Plan, Implement, Audit and Verify. The security path is mandatory for applicable changes, while unrelated runs must not load the entire security corpus.

## Integrity note

The formerly missing `security_architecture_design.md` reference has been restored. It is a reconstructed canonical baseline, while the current source tree remains the implementation evidence. The standing reference-integrity policy is `docs/icm_reference_integrity.md`. The external standards register must be rechecked before using version-sensitive security claims.
