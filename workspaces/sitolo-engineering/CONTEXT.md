# Sitolo Engineering Pipeline

Run one engineering request through the stages in order. Each stage is a file-based contract and handoff.

## Pipeline

01-select → 02-research → 03-investigate → 04-plan → 05-implement → 06-audit → 07-remediate → 08-verify → 09-deliver

Research always runs as a decision gate. Remediation always runs as a decision gate. A remediation that changes the implementation returns to audit before verification.

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
