# Change Impact

One job: route an editor from a change to the first-order System Map cards that should be opened.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Object catalog | ../objects/_index.md | Full file | Identify the noun and universe/status |
| Object cards | ../objects/*/*.md | Relevant cards | Understand first-order impact |
| Process cards | ../processes/*.md | Real executable movements | Understand movement-level impact |

## Process

Walk the change backwards from the affected noun or real movement. Open only the listed first-order cards, then follow `See` links into authoritative sources.

## Outputs

This catalog itself is the output: a concise change-to-card routing table.

## Routing

| Change | Open first |
|---|---|
| API endpoint or transport handler | ../objects/runtime/api-boundary.md; ../processes/http-transport-processing.md |
| Authentication or session | ../objects/security/authentication.md; ../objects/security/tenancy.md |
| Authorization policy | ../objects/security/authorization.md; ../objects/security/tenancy.md |
| Database connection, schema, or RLS | ../objects/data/persistence.md; ../objects/security/tenancy.md; ../processes/postgres-authority-initialization.md |
| Financial or inventory mutation | ../objects/domain/domain-core.md; ../objects/data/persistence.md |
| Offline command or sync | ../objects/continuity/synchronization.md |
| Outbox or worker | ../objects/reliability/events-and-audit.md; ../objects/runtime/worker-runtime.md |
| External integration | ../objects/integration/integrations.md |
| Observability | ../objects/platform/observability.md; ../objects/runtime/api-boundary.md |
| Agent workflow or documentation | ../../workspaces/sitolo-engineering/CONTEXT.md; ../objects/verification/test-infrastructure.md |

## External consumers

This index answers “I am changing X, what inside the tree moves.” It does **not** discover all consumers outside the tree. External consumers such as deployment configuration, issue trackers, scheduled jobs, agents, or other repositories may not be referenced by the subject tree. They must be identified by the owner/human during the change-impact slice and recorded on the affected card or handoff using absolute paths or explicit external identifiers.

## Rule

Source documents and source code remain authoritative. The System Map cites the subject tree; it is not a second specification. If the catalog and source disagree, source evidence controls implementation truth, while normative contracts remain governing requirements until explicitly changed.
