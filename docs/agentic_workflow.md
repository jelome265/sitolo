# Sitolo ICM Engineering Workflow

Sitolo uses the Interpretable Context Methodology as its agent workflow architecture. The workflow is filesystem-routed: one agent reads the smallest relevant context, performs one stage job, writes a plain-text handoff, and proceeds through explicit gates.

Reference: https://github.com/RinDig/Interpretable-Context-Methodology

## Two complementary forms

Sitolo composes two ICM forms:

1. Pipeline: repeated engineering runs with numbered stages and human-editable handoffs.
2. System Map: a read-oriented edit graph of the repository so later agents can locate nouns, real movements, and first-order change impact without crawling the entire tree.

## Five-layer context

| Layer | Sitolo | Purpose |
|---|---|---|
| L0 | root CLAUDE.md | Where am I? |
| L1 | workspace CONTEXT.md | Where do I go? |
| L2 | stage CONTEXT.md | What do I do? |
| L3 | shared routers, references, skills | What rules apply? |
| L4 | stage output | What am I working with? |

ICM explicitly separates stable reference material from run-specific artifacts and requires selective loading.

## Business model

Yes, the business model is part of the engineering context, but it is Layer 3 reference material, not stage procedure and not copied into every prompt.

Use the business-context router for product scope, user/actor semantics, customer workflows, segments, tiers, vertical modules, onboarding, monetization, continuity, and business/regulatory boundaries.

The canonical commercial documents remain under docs/commercial/. The agent must read only the relevant document and section for the active change. A low-level implementation change that cannot alter product behavior should not load the commercial corpus.

## Pipeline

01-select → 02-research → 03-investigate → 04-plan → 05-implement → 06-audit → 07-remediate → 08-verify → 09-deliver

Every stage owns an output directory. Human edits to stage output are valid handoffs. Stage contracts contain Inputs, Process, and Outputs and remain short routing documents.

## Human gates

Consequential handoffs are intentionally visible. The person can inspect and edit the artifact before the next stage consumes it.

## System Map

map/ follows the current ICM System Map form for a repository that later agents must edit. The subject tree remains authoritative. The map catalogs nouns, real movements, and first-order change impact; it does not become a second architecture specification.

## Validation

scripts/ci/check-icm-workspace and its PowerShell counterpart verify the workspace structure. The canonical repository verifier invokes the workspace check.

Known missing references in the existing documentation corpus are recorded as integrity findings rather than fabricated into new facts.
