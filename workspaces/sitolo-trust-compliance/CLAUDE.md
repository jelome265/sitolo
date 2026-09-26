# Sitolo Trust & Compliance Workspace

Mode: routing-only
Primary owner: Regulatory/legal boundary analysis, privacy/data governance, risk/control requirements, fraud and compliance routing.

## Entry condition

Enter only after `workspaces/CLAUDE.md` selects Trust & Compliance as the primary work domain. Applicable law, regulators and signed external contracts remain higher authorities.

## Workflow destination

Primary destination: `CONTEXT.md` for the applicable authority sources and resulting compliance assessment, risk decision, control requirement, regulatory verification record or incident-routing artifact. Technical remediation and other domain consequences exit through Cross-domain exits.

## Cross-domain exits

| Dependency | Destination |
|---|---|
| Security architecture implementation or technical control | ../sitolo-engineering/CLAUDE.md |
| Commercial implication | ../sitolo-commercial/CONTEXT.md |
| Customer service incident | ../sitolo-customer-operations/CONTEXT.md |
| Product capability affected | ../sitolo-product/CONTEXT.md |

## Routing

| Task | Go To |
|---|---|
| Regulatory perimeter | CONTEXT.md |
| Tax/fiscal readiness | CONTEXT.md |
| Fraud/loss prevention | CONTEXT.md |
| Privacy/data governance | CONTEXT.md |
| Security architecture implementation | ../sitolo-engineering/CLAUDE.md |
| Commercial implications | ../sitolo-commercial/CONTEXT.md |
| Customer service incident | ../sitolo-customer-operations/CONTEXT.md |

## Loading

Read CONTEXT.md and only the applicable authority sources.

## Authority

Applicable law, regulators and signed external contracts outrank this workspace. Security architecture and product requirements remain authoritative for their respective technical/product decisions.

## Do NOT load

Do not treat commercial assumptions as legal facts. Do not load unrelated engineering implementation detail unless validating a specific control.
