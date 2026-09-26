# Sitolo Engineering Pipeline

Mode: executable
Primary owner: Engineering technical architecture, implementation, testing, deployment, security engineering and technical verification.

## Inputs

| Source | Location | Scope | Why |
|---|---|---|---|
| Governance | shared/governance-context/CONTEXT.md | Applicable engineering rules | Repository-wide implementation authority |
| Documentation map | ../../docs/README.md | Routed documents only | Locate authoritative project contracts |
| Technical context | shared/technical-context/CONTEXT.md | Relevant architecture/domain sections | Technical ownership and constraints |
| Security context | shared/security-context/CONTEXT.md | Relevant threat/control sections | Security invariants and evidence |
| Phase context | shared/phase-context/CONTEXT.md | Current phase/contract | Current implementation boundaries |

## Security gate

Security-sensitive runs are conditionally routed through `shared/security-context/CONTEXT.md`. Stage 01 records `security_relevant: yes|no`; when yes, control IDs are selected and preserved through planning, implementation, audit and verification.

## Workflow destination

Primary destination: `stages/01-select/CONTEXT.md` for a new run; continue at the current stage CONTEXT.md for an existing run; verified work terminates at `stages/09-deliver/CONTEXT.md`.

## Cross-domain exits

| Dependency | Destination |
|---|---|
| Product requirement or behavior changes | ../sitolo-product/CONTEXT.md |
| Commercial requirement or economics changes | ../sitolo-commercial/CONTEXT.md |
| Customer lifecycle/service requirement | ../sitolo-customer-operations/CONTEXT.md |
| Regulatory, privacy, risk or control requirement | ../sitolo-trust-compliance/CONTEXT.md |

## Pipeline

01-select → 02-research → 03-investigate → 04-plan → 05-implement → 06-audit → 07-remediate → 08-verify → 09-deliver

Research always runs as a decision gate. Remediation always runs as a decision gate. If remediation changes the implementation, the human re-enters the audit stage before verification; this is a human-controlled revisit, not a stage dependency cycle.

## Task Routing

| Task | Go To |
|---|---|
| Start a change | stages/01-select/CONTEXT.md |
| Continue a run | current stage CONTEXT.md |
| Re-audit remediation | stages/06-audit/CONTEXT.md |
| Show status | stage output directories |

## Shared Resources

| Resource | Location | Contains |
|---|---|---|
| Business and product context | shared/business-context/CONTEXT.md | Product, actor, segment, tier, onboarding and commercial routing |
| Technical context | shared/technical-context/CONTEXT.md | Architecture, domain, DB, API, sync, testing and operations routing |
| Security context | shared/security-context/CONTEXT.md | Threat, controls and security test routing |
| Commercial context | shared/commercial-context/CONTEXT.md | Commercial governance and economics routing |
| Integration context | shared/integration-context/CONTEXT.md | Provider and fiscal integration routing |
| Phase context | shared/phase-context/CONTEXT.md | Current implementation contracts |
| Artifact templates | _templates/CONTEXT.md | Output shapes |
| Repository map | ../../map/CLAUDE.md | Change-impact routing |
| Governance | ../../agent.md | Engineering authority |

## Status

An output directory containing only .gitkeep is PENDING. Any other file means an artifact exists. File presence does not prove approval or quality.

## Do NOT load

Do not load unrelated stage contracts, full commercial/regulatory corpora, or full agent.md. Use the workspace and selected stage contracts to route exact inputs.
