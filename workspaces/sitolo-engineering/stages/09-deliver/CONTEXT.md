# 09 Deliver

One job: prepare the verified change for review and controlled integration.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Working | Run brief | ../01-select/output/[run-slug]-brief.md | Full file | Scope |
| Working | Plan | ../04-plan/output/[run-slug]-plan.md | Full file | Intended result |
| Working | Implementation | ../05-implement/output/[run-slug]-implementation.md | Full file | Change summary |
| Working | Audit | ../06-audit/output/[run-slug]-audit.md | Full file | Findings status |
| Working | Verification | ../08-verify/output/[run-slug]-verification.md | Full file | Evidence |
| Reference | Governance | ../../shared/governance-context/CONTEXT.md | Git and review rules | Delivery requirements |
| Reference | Delivery guide | references/delivery.md | Full file | Handoff format |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Confirm verification is complete and no material audit finding remains open.
2. Inspect the final diff for scope, secrets, generated artifacts, and unrelated changes.
3. Prepare a factual commit/PR summary with tests and known limitations.
4. Commit and open the review request only through the project's normal Git workflow.
5. Do not merge merely because the stage is complete.

## Human Check

Review the final diff and delivery summary. Human approval remains the final integration decision.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Delivery handoff | output/[run-slug]-delivery.md | Markdown |
