# Sitolo Engineering Workspace

This workspace takes one engineering request through a controlled sequence from selection to delivery.

## Folder Map

setup/
_config/
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
| Existing code and contract investigation | stages/03-investigate/CONTEXT.md |
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
| Execute a stage | selected CONTEXT.md, declared references, declared previous outputs | unrelated stage folders |
| Resume | current stage contract and its declared handoff files | restart from the beginning unless required |

## Stage Handoffs

Every stage writes only to its own output directory. The next stage reads the previous stage output named by its contract. A human may edit an output before the next stage runs; the next stage must use the edited file.

Stages are numbered and one-way. Do not skip a stage. Remediation returns to audit before verification.
