# Sitolo Workspace Context

## Purpose

Select the smallest organizational work domain for the task.

## Workspace registry

| ID | Workspace | Evidence basis | Status |
|---|---|---|---|
| engineering | sitolo-engineering/ | Existing 9-stage engineering pipeline, repository governance, architecture and implementation corpus | active |
| product | sitolo-product/ | Product specification, business model product scope, onboarding/product audience, product intelligence model | active |
| commercial | sitolo-commercial/ | Commercial corpus, operating model, validation, pricing, acquisition, lifecycle economics | active |
| customer-operations | sitolo-customer-operations/ | Onboarding, migration, support/service operations, trust/adoption/continuity | active |
| trust-compliance | sitolo-trust-compliance/ | Regulatory perimeter, tax/fiscal readiness, fraud/business controls, data governance, security/compliance boundaries | active |

## Explicit non-domains

| Concern | Routed under | Why not a separate workspace yet |
|---|---|---|
| Strategy | Commercial | Existing corpus defines strategy inside the commercial strategy layer; a standalone strategy operating corpus is not established |
| Finance | Commercial / product / trust-compliance as applicable | Existing finance material is primarily monetization, reconciliation, credit/lay-by and customer finance workflows; no distinct Sitolo corporate-finance operating corpus is established |
| Business intelligence | Product + Commercial | Defined as a product value layer and commercial/defensibility concern, not an independent organizational operating domain |
| Security engineering | Engineering + Trust & Compliance | Technical controls live in engineering; legal/compliance obligations live in trust/compliance |
| Payments | Commercial + Trust & Compliance + Engineering | Payments are simultaneously a monetization workflow, regulated boundary and technical integration; splitting them into a standalone org workspace would duplicate ownership |

## Routing rule

A workspace is justified only when the corpus shows a distinct set of decisions, actors, artifacts and ownership boundaries. Document-folder names alone do not create organizational domains.

## Cross-domain rule

One workspace owns the primary decision. Other workspaces are consulted through explicit references; they do not become competing authorities.

## Do NOT load

Do not load every workspace for a task. Start with one primary workspace, then follow only its declared cross-domain routes.
