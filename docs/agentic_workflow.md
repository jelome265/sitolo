# Sitolo ICM Engineering Workflow

Sitolo uses the Interpretable Context Methodology as its agent workflow architecture. The workflow is filesystem-routed: one agent reads the smallest relevant context, performs one stage job, writes a plain-text handoff, and proceeds through explicit gates.

Reference: https://github.com/RinDig/Interpretable-Context-Methodology

## Instruction entry points

Claude Code uses `CLAUDE.md` for repository guidance. Codex uses `AGENTS.md` as its discovered repository instruction file. Sitolo keeps `CLAUDE.md` as the canonical ICM router and `AGENTS.md` as the Codex entry point that routes into that same canonical policy. OpenAI's current Codex documentation confirms automatic `AGENTS.md` discovery and root-to-leaf instruction loading. citeturn830571view2turn830571view3

## Product phase vs ICM stage

The ICM pipeline is engineering workflow infrastructure, not Sitolo product-phase sequencing. Sitolo's active product stream remains Phase 4 Part 8 / PR-009. A later contract such as Phase 8 Product Catalogue is target-state documentation, not a phase transition.

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

ICM separates stable reference material from run-specific artifacts and requires selective loading. Current ICM also expects stage inputs and outputs to be explicit file paths and human-editable handoffs. citeturn375393search0turn375393search2

## Business model

The business model is part of engineering context, but it is Layer 3 reference material, not stage procedure and not copied into every prompt.

Use the business-context router for product scope, actor semantics, customer workflows, segments, tiers, vertical modules, onboarding, monetization, continuity, and business/regulatory boundaries.

The canonical commercial documents remain under `docs/commercial/`. Low-level implementation work should load only the relevant source and sections.

## Pipeline

01-select → 02-research → 03-investigate → 04-plan → 05-implement → 06-audit → 07-remediate → 08-verify → 09-deliver

Every stage owns an output directory. Human edits to stage output are valid handoffs. Stage contracts contain Inputs, Process, Outputs and Human Check.

## Human gates

Consequential handoffs are intentionally visible. The person can inspect and edit the artifact before the next stage consumes it.

## System Map

`map/` follows the current ICM System Map form for a repository that later agents must edit. The subject tree remains authoritative. The map catalogs nouns, real movements and first-order change impact; it does not become a second architecture specification. Current ICM Architect explicitly defines the System Map as a form for an editable repository and requires walk-test validation. citeturn375393search2turn375393search3

## Validation

`scripts/ci/check-icm-workspace` and its PowerShell counterpart verify the workspace structure. `scripts/ci/check-doc-references` verifies Markdown reference resolution.

The repository now has restored authority paths for the formerly missing contract filenames. Remaining documentation integrity work is semantic: stale-vs-current classification, authority conflicts, implementation drift, external-version freshness, and historical routing.

## Walk-test requirement

A fresh agent must be able to enter through the root router, discover the engineering workspace, identify the current stage, load only required references and inputs, produce a Markdown handoff, and report status from filesystem state alone. That walk test is the acceptance test for the workflow structure. citeturn375393search2
