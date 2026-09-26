# Sitolo ICM Engineering Workflow

Sitolo uses the Interpretable Context Methodology as its agent workflow architecture. The workflow is filesystem-routed: one agent reads the smallest relevant context, performs one stage job, writes a plain-text handoff, and proceeds through explicit gates.

Reference: https://github.com/RinDig/Interpretable-Context-Methodology

## Instruction entry points

Claude Code uses the repository root `../CLAUDE.md` for guidance. Codex uses the repository root `../AGENTS.md` as its discovered instruction file. Sitolo keeps the repository root `../CLAUDE.md` as the canonical ICM router and the repository root `../AGENTS.md` as the Codex entry point that routes into that same canonical policy. OpenAI's current Codex documentation confirms automatic discovery of the repository root `../AGENTS.md` instruction file and root-to-leaf instruction loading.

## Sitolo's ICM composition

The nine items commonly used to describe this repository are a **composition of ICM forms**, not one canonical folder tree:

| Concern | Sitolo implementation | Architectural role |
|---|---|---|
| Routing | root/workspace entry and context files, System Map routing twins | Select the next context/shelf |
| Contracts | stage context files, System Map schema/templates | Define the local job and data shape |
| Objects | `map/objects/` | Verified nouns and ownership boundaries |
| Processes | `map/processes/` | Only real executable movements |
| Effects | `map/effects/CONTEXT.md` | First-order change-impact routing |
| References | shared context, stage `references/`, governance router | Stable rules/evidence |
| Working artifacts | stage `output/` | Per-run product/state; not repository policy |
| Verification | human gates + CI verification scripts | Evidence and control, not a payload shelf |

This distinction prevents two common errors: treating verification as another content store, and treating the System Map as a second application specification. ICM's System Map is explicitly a walkable edit map over the subject tree, while the subject tree remains authoritative.

## Agent context contract

Every consequential stage should be answerable through this chain:

| Question | Required answer |
|---|---|
| What should the agent know? | Only the capability-specific requirements, current working handoffs, applicable governance, and authoritative source material |
| Where is it? | An exact path named by the active stage contract or its recursive router |
| Why is it authoritative? | The source's role is explicit; routing files never override governing contracts or source evidence |
| What is the smallest context? | The active entry + stage contract + only required L3 references and L4 working inputs, targeted toward 2,000–8,000 tokens |
| What artifact must be produced? | The exact output file declared by the stage contract, using the appropriate template/frontmatter |
| Who/what verifies it? | The stage's explicit human check plus mechanical CI evidence appropriate to the claim |
| What does the next stage consume? | The exact prior-stage artifact path declared in the next stage's Inputs table |

The chain is a control loop, not a prompt-writing trick: **scope → locate → establish authority → minimize context → produce artifact → verify evidence → hand off exact state**.

## Product phase vs ICM stage

The ICM pipeline is engineering workflow infrastructure, not Sitolo product-phase sequencing. Sitolo's active product stream remains Phase 4 Part 8 / PR-009. A later contract such as Phase 8 Product Catalogue is target-state documentation, not a phase transition.

## Two complementary forms

Sitolo composes two ICM forms:

1. Pipeline: repeated engineering runs with numbered stages and human-editable handoffs.
2. System Map: a read-oriented edit graph of the repository so later agents can locate nouns, real movements, and first-order change impact without crawling the entire tree.

## Five-layer context

ICM's five layers are **recursive**, not one global file per layer. Sitolo has a nested engineering workspace, so the effective walk is:

```text
root `../CLAUDE.md`             L0 routing
  -> root `../CONTEXT.md`       L1 routing
     -> `../workspaces/sitolo-engineering/CLAUDE.md`  nested L0
        -> `../workspaces/sitolo-engineering/CONTEXT.md` nested L1
           -> stage context contract        L2 control
              -> shared/router + references  L3 factory
                 -> stage/output artifact   L4 run product
```

The layer semantics remain:

| Layer | Role | Question |
|---|---|---|
| L0 | small entry/router | Where am I? |
| L1 | hub/context router | Where do I go? |
| L2 | stage control contract | What do I do? |
| L3 | stable references | What rules apply? |
| L4 | run artifacts | What am I working with? |

Large L3 collections recursively get their own context router. The important property is that the stage contract controls context selection instead of delegating repository-wide discovery to the model.

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

`map/` follows the current ICM System Map form for a repository that later agents must edit. The subject tree remains authoritative. The map catalogs nouns, real movements and first-order change impact; it does not become a second architecture specification. Current ICM Architect explicitly defines the System Map as a form for an editable repository and requires walk-test validation.

## Validation

`scripts/ci/check-icm-workspace` and its PowerShell counterpart verify workspace structure, context boundaries, reference scaffolding, and factory-output hygiene. `scripts/ci/check-doc-references` verifies Markdown reference resolution.

The CI cold walk is intentionally **mechanical**: it proves the routing graph, exact handoff paths, template instantiation, and System Map source navigation in an isolated copy. It does not prove that a fresh LLM can make correct engineering decisions or produce semantically meaningful run artifacts.

## Walk-test requirement

Use two distinct proofs:

1. **Mechanical walk:** CI traverses the filesystem, instantiates every stage artifact, checks exact handoffs, and validates a System Map source hop.
2. **Cold-agent walk:** a fresh agent with no prior conversation memory enters through the root router, follows the declared Inputs, performs a meaningful bounded task, writes a valid handoff, and reports status from filesystem state alone.

Only the second demonstrates agent usability; neither substitutes for product-level tests or human approval.
