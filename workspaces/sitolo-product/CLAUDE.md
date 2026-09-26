# Sitolo Product Workspace

Mode: routing-only
Primary owner: Product scope, actors, product behavior, capability decisions, product experience and product-level requirements.

## Entry condition

Enter only after `workspaces/CLAUDE.md` selects Product as the primary work domain. Product ownership is for decisions about what Sitolo should do, for whom, and why.

## Workflow destination

Primary destination: `CONTEXT.md` for the smallest product-decision source set and the resulting product decision, requirement or hypothesis artifact. Technical consequences exit to Engineering; other declared domain consequences exit through Cross-domain exits.

## Cross-domain exits

| Dependency | Destination |
|---|---|
| Onboarding, migration or lifecycle/service operation | ../sitolo-customer-operations/CONTEXT.md |
| Commercial validation, packaging or economics | ../sitolo-commercial/CONTEXT.md |
| Technical implementation or verification | ../sitolo-engineering/CLAUDE.md |
| Regulatory, privacy, risk or control obligation | ../sitolo-trust-compliance/CONTEXT.md |

## Routing

| Task | Go To |
|---|---|
| Product scope, capabilities and business workflow | CONTEXT.md |
| User/actor/segment semantics | CONTEXT.md → business model / segment sources |
| Onboarding and first-value experience | ../sitolo-customer-operations/CONTEXT.md |
| Commercial validation affecting product | ../sitolo-commercial/CONTEXT.md |
| Technical implementation | ../sitolo-engineering/CLAUDE.md |

## Loading

Read CONTEXT.md, then only the routed source sections.

## Authority

Product workspace documents route canonical product/business sources. They do not override technical, regulatory or security contracts.

## Do NOT load

Do not load the full technical architecture, database design, or entire commercial corpus unless the decision explicitly depends on them.
