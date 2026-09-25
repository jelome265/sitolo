# Change Impact

One job: route an editor from a change to the first-order System Map cards that should be opened.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Object catalog | ../objects/_index.md | Full file | Identify the noun and universe/status |
| Object cards | ../objects/*/*.md | Relevant cards | Understand first-order impact |
| Process cards | ../processes/*.md | Relevant movement | Understand movement-level impact |

## Process

Walk the change backwards from the noun. Open only the listed first-order cards. The subject tree remains authoritative.

## Outputs

This catalog itself is the output: a concise change-to-card routing table.

## Routing

| Change | Open first |
|---|---|
| API endpoint or transport handler | ../objects/runtime/api-boundary.md; ../processes/request-processing.md |
| Authentication or session | ../objects/security/authentication.md; ../objects/security/tenancy.md; ../processes/request-processing.md |
| Authorization policy | ../objects/security/authorization.md; ../objects/security/tenancy.md; ../processes/request-processing.md |
| Database schema or RLS | ../objects/data/persistence.md; ../objects/security/tenancy.md; ../processes/transactional-mutation.md |
| Financial or inventory mutation | ../objects/domain/domain-core.md; ../objects/data/persistence.md; ../processes/transactional-mutation.md |
| Offline command or sync | ../objects/continuity/synchronization.md; ../processes/offline-sync.md |
| Outbox or worker | ../objects/reliability/events-and-audit.md; ../objects/runtime/worker-runtime.md; ../processes/outbox-dispatch.md |
| External integration | ../objects/integration/integrations.md; ../processes/outbox-dispatch.md |
| Observability | ../objects/platform/observability.md; affected process card |
| Agent workflow or documentation | ../objects/verification/test-infrastructure.md; ../../workspaces/sitolo-engineering/CONTEXT.md |

## Rule

This index answers “I am changing X, what inside the tree moves.” It does not enumerate outside consumers. Source documents and source code remain authoritative.
