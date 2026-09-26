# Sitolo - ICM Root Router

Sitolo uses filesystem-routed ICM workspaces. The root router selects the organizational domain first, then the domain workspace selects the task context.

## Routing

| Task | Go To | Description |
|---|---|---|
| Any organizational work | workspaces/CLAUDE.md | Select the smallest bounded work domain |
| Engineering change | workspaces/sitolo-engineering/CLAUDE.md | Build, audit and verify the system |
| Product decision | workspaces/sitolo-product/CLAUDE.md | Product scope, behavior, actors and requirements |
| Commercial decision | workspaces/sitolo-commercial/CLAUDE.md | Strategy, validation, pricing, acquisition and monetization |
| Customer operations | workspaces/sitolo-customer-operations/CLAUDE.md | Onboarding, support, lifecycle and service operations |
| Trust/compliance | workspaces/sitolo-trust-compliance/CLAUDE.md | Regulatory, privacy, risk, fraud and controls |
| Repository change impact | map/CLAUDE.md | Routes to the ICM System Map of the codebase |
| Canonical project knowledge | docs/README.md | Routes to authoritative project knowledge |
| Workflow integrity | scripts/ci/check-icm-workspace | Mechanical workspace validation |

## Loading

Read this file, then workspaces/CLAUDE.md, then the selected workspace CLAUDE.md and CONTEXT.md. Do not load sibling workspaces unless explicitly routed.

## Authority

agent.md remains the detailed engineering governance contract. docs/README.md and the named canonical documents remain authoritative for project knowledge. Workspace files are routing/control surfaces, not competing sources of truth.