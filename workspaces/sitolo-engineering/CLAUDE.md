# Sitolo Engineering Workspace

Mode: executable
Primary owner: Engineering technical architecture, implementation, testing, deployment, security engineering and technical verification.

## Entry condition

This workspace is valid only after `workspaces/CLAUDE.md` has selected Engineering as the primary work domain. The presence of code, a repository, infrastructure, or an implementation request does not by itself authorize entry here.

## Workflow destination

Primary destination: `stages/01-select/CONTEXT.md` for a new engineering run, or the current stage CONTEXT.md when continuing an existing run. Delivery terminates at `stages/09-deliver/CONTEXT.md`.

## Cross-domain exits

| Dependency | Destination |
|---|---|
| Product requirement or behavior changes | ../sitolo-product/CONTEXT.md |
| Commercial requirement or economics changes | ../sitolo-commercial/CONTEXT.md |
| Customer lifecycle/service requirement | ../sitolo-customer-operations/CONTEXT.md |
| Regulatory, privacy, risk or control requirement | ../sitolo-trust-compliance/CONTEXT.md |

## Triggers

| Keyword | Action |
|---|---|
| setup | Read setup/questionnaire.md |
| status | Read CONTEXT.md and scan every stage output directory |

## Routing

| Task | Go To |
|---|---|
| New feature, bug, security fix, migration, refactor | stages/01-select/CONTEXT.md |
| Current external/regulatory/provider fact | stages/02-research/CONTEXT.md |
| Existing implementation and contracts | stages/03-investigate/CONTEXT.md |
| Implementation design | stages/04-plan/CONTEXT.md |
| Code change | stages/05-implement/CONTEXT.md |
| Contract/security review | stages/06-audit/CONTEXT.md |
| Fix confirmed findings | stages/07-remediate/CONTEXT.md |
| Verification and evidence | stages/08-verify/CONTEXT.md |
| Commit/PR handoff | stages/09-deliver/CONTEXT.md |

## What to Load

| Task | Load | Do NOT Load |
|---|---|---|
| Enter workspace | CONTEXT.md, then selected stage CONTEXT.md | other stage contracts |
| Product-affecting engineering | shared/business-context/CONTEXT.md and only the routed source sections | the full commercial corpus |
| Architecture/security change | shared/technical-context/CONTEXT.md or shared/security-context/CONTEXT.md | unrelated reference families |
| Repository change impact | ../../map/CLAUDE.md | the whole source tree |

## Stage Handoffs

Every stage writes only to its own output directory. The next stage reads the previous stage's declared output. A human may edit an output before the next stage runs.

Do not skip stages. A remediation-triggered re-audit is a human-controlled revisit to 06 before verification, not an automatic stage dependency cycle.
