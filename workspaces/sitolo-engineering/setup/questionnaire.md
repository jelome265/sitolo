# Sitolo Engineering Workspace Setup

Run this questionnaire only to intentionally change stable workflow preferences. Per-run requirements belong in Stage 01.

Answer all questions in one pass. Defaults preserve the current Sitolo workflow.

1. What default branch should delivery target? Default: main.
2. What is the canonical local verifier? Default: ./scripts/ci/verify.
3. Should consequential stage handoffs require human review? Default: yes.
4. Should stage outputs remain human-editable Markdown artifacts? Default: yes.
5. Should remediation return to audit before verification? Default: yes.
6. Should delivery remain a separate final stage? Default: yes.

After setup, update only _config/workflow-policy.md. Do not duplicate repository engineering requirements there.
