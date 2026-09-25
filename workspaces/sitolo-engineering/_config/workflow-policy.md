# Sitolo Engineering Workspace Policy

These are workflow preferences, not replacements for repository governance.

## Stable Settings

- Default branch: main.
- Workspace type: sequential engineering pipeline.
- Human review: required at consequential handoffs.
- Stage outputs: plain Markdown and human-editable.
- Stage references: read only when declared by the active stage.
- Canonical verifier: ./scripts/ci/verify.
- Workspace validator: ./scripts/ci/check-icm-workspace.
- Status is derived from stage output contents.
- Remediation returns to audit before verification.
- Delivery is a separate final stage.

For engineering authority, use ../../agent.md. For project knowledge, use ../../docs/README.md.
