# Sitolo Workspace Context

## Purpose

Select the smallest organizational work domain for the task.

## Workspace registry

| ID | Workspace | Mode | Evidence basis | Status |
|---|---|---|---|---|
| engineering | sitolo-engineering/ | executable | Existing 9-stage engineering pipeline, repository governance, architecture and implementation corpus | active |
| product | sitolo-product/ | routing-only | Product specification, business model product scope, onboarding/product audience, product intelligence model | active |
| commercial | sitolo-commercial/ | routing-only | Commercial corpus, operating model, validation, pricing, acquisition, lifecycle economics | active |
| customer-operations | sitolo-customer-operations/ | routing-only | Onboarding, migration, support/service operations, trust/adoption/continuity | active |
| trust-compliance | sitolo-trust-compliance/ | routing-only | Regulatory perimeter, tax/fiscal readiness, fraud/business controls, data governance, security/compliance boundaries | active |

Current shape: 5 registered organizational workspaces; 1 executable workspace and 4 routing-only workspaces.

## Workspace contract

Every workspace declares:

1. an unambiguous entry condition and primary ownership;
2. scoped authoritative inputs in its CONTEXT.md;
3. explicit cross-domain exits;
4. a defined workflow destination.

A routing-only workspace is an organizational dispatch boundary. It must not fabricate an executable stage pipeline merely to satisfy this contract.

An executable workspace additionally owns stage contracts, Layer 3 references, Layer 4 handoffs, verification/checkpoint semantics and one-way stage dependencies.

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

Cross-domain exits are organizational routing/consultation edges and may be bidirectional. One-way dependency verification applies to executable stage handoffs, not to mutual domain consultation.

## Do NOT load

Do not load every workspace for a task. Start with one primary workspace, then follow only its declared cross-domain routes.
