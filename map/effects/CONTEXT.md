# Change Impact

First-order routing only.

| Change | Open first |
|---|---|
| API endpoint or handler | objects/API boundary; processes/request-processing |
| Authentication or session | objects/Authentication; objects/Tenancy; processes/request-processing |
| Authorization policy | objects/Authorization; objects/Tenancy; processes/request-processing |
| Database schema or RLS | objects/Persistence; objects/Tenancy; processes/transactional-mutation |
| Financial or inventory mutation | objects/Domain core; objects/Persistence; processes/transactional-mutation |
| Offline command or sync | objects/Synchronization; processes/offline-sync |
| Outbox or worker | objects/Events and audit; objects/Worker runtime; processes/outbox-dispatch |
| External integration | objects/Integrations; relevant process card |
| Observability | objects/Observability; affected process card |
| Agent workflow or documentation | workspaces/sitolo-engineering; docs/agentic_workflow.md |

This index does not replace authoritative contracts.
