# Sitolo Workspace Router

`workspaces/` is the organizational ICM routing layer. It selects a bounded work domain; it does not define product truth or technical architecture.

## Domain routing

| Work domain | Workspace | Primary question |
|---|---|---|
| Engineering | sitolo-engineering/ | How do we build, change, audit and verify the system? |
| Product | sitolo-product/ | What should the product do, for whom, and why? |
| Commercial | sitolo-commercial/ | How do we validate demand, acquire, monetize and retain customers? |
| Customer Operations | sitolo-customer-operations/ | How do we onboard, support and operate the customer lifecycle safely? |
| Trust & Compliance | sitolo-trust-compliance/ | What legal, regulatory, privacy, risk and control obligations apply? |

## Loading

Read this router, then the selected workspace `CLAUDE.md`, then its `CONTEXT.md`. Do not load sibling workspaces unless the selected workspace routes a cross-domain dependency.

## Authority

This router is navigation only. Canonical product, commercial, security, regulatory and technical sources remain under `docs/` and applicable external authorities.
