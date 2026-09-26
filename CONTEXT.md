# Sitolo Root Context

## Task Routing

| Task Type | Go To | Description |
|---|---|---|
| Organizational work | workspaces/CLAUDE.md | Mandatory primary-domain dispatch |
| Change impact | map/CLAUDE.md | System Map routing after primary domain selection, or for an explicit map-only request |

## Loading

Every organizational task starts at `workspaces/CLAUDE.md`. Select exactly one primary workspace before entering a domain-specific workflow. Do not bypass the workspace router because a task includes code, repository, infrastructure, security, payments, documentation, or implementation language.

After primary-domain selection, read only that workspace's CONTEXT.md and explicit cross-domain routes. Engineering is selected by the workspace router only when Engineering owns the primary work; it is never inferred from the presence of implementation work.

## Authority

| Resource | Location | Contains |
|---|---|---|
| Engineering governance | agent.md | Repository-wide engineering contract |
| Documentation map | docs/README.md | Canonical project knowledge |
| Workspace architecture | docs/workspace_domain_architecture.md | Organizational workspace ownership and exclusions |

Do not enter a domain workspace directly from this root context.