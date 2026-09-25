# Sitolo Engineering Workspace

This workspace takes one engineering request through a controlled sequence from selection to delivery.

## Folder Map

CLAUDE.md
CONTEXT.md
setup/
_config/
_templates/
shared/
skills/
stages/
  01-select/
  02-research/
  03-investigate/
  04-plan/
  05-implement/
  06-audit/
  07-remediate/
  08-verify/
  09-deliver/

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

Do not skip stages. A remediation-triggered re-audit is a human-controlled revisit to 06 before verification, not an automatic stage dependency.
