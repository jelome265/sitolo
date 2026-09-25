# Sitolo - ICM Root Router

Sitolo uses a filesystem-routed Interpretable Context Methodology workspace for repeatable engineering work.

## Routing

| Task | Go To | Description |
|---|---|---|
| Engineering change | workspaces/sitolo-engineering/CLAUDE.md | Routes to the engineering pipeline |
| Pipeline status | workspaces/sitolo-engineering/CONTEXT.md | Shows stage state |
| Repository change impact | map/CLAUDE.md | Routes to the ICM System Map of the codebase |
| Architecture or corpus question | docs/README.md | Routes to authoritative project knowledge |
| Workflow integrity | scripts/ci/check-icm-workspace | Mechanical workspace validation |

## Loading

The root router only chooses a workspace or map. It must not contain stage procedures or duplicate project policy.

When entering the engineering workspace, read its CLAUDE.md, then CONTEXT.md, then the selected stage contract.

## Authority

agent.md is the detailed engineering governance contract. docs/README.md is the project documentation map. Workspaces provide workflow routing only.
