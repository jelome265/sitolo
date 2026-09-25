# Sitolo ICM Engineering Workflow

Sitolo adopts the Interpretable Context Methodology (ICM) described by Jake Van Clief and David McDermott. The workflow is filesystem-routed rather than driven by a multi-agent orchestration runtime.

## Architecture

The routing layers are:

```
root CLAUDE.md
    ↓
root CONTEXT.md
    ↓
workspaces/sitolo-engineering/CLAUDE.md
    ↓
workspaces/sitolo-engineering/CONTEXT.md
    ↓
numbered stage CONTEXT.md
    ↓
declared references + previous output
    ↓
stage output
    ↓
next stage
```

The repository AGENTS.md is a Codex compatibility adapter that routes Codex into the same filesystem model. The detailed Sitolo engineering contract remains in agent.md. Project knowledge remains in docs/.

## Five Layers

1. Root or workspace router: where am I and where do I go?
2. Stage CONTEXT.md: what does this stage load and do?
3. Reference material: stable rules and domain knowledge.
4. Working artifacts: the current run's outputs and handoffs.
5. Human edit surface: each output may be reviewed and edited before the next stage consumes it.

No stage loads the entire repository by default.

## Sitolo Pipeline

The base engineering workspace uses:

```
01-select
02-research
03-investigate
04-plan
05-implement
06-audit
07-remediate
08-verify
09-deliver
```

The sequence is intentionally explicit. Research always runs, but may conclude that no external research is required. Remediation always runs, but may conclude that no remediation is required. When remediation changes code, the workflow returns to 06-audit before 08-verify.

## Stage Contracts

Every stage has a CONTEXT.md with:

- Inputs: exactly what the stage may load.
- Process: one job expressed as explicit steps.
- Outputs: the artifact written for the next stage.

Stage contracts are routing files, not repositories for long engineering policy. Long-lived rules belong in agent.md or an appropriate reference file.

## Handoffs

Each stage writes to its own output directory. The next stage reads the output named by the previous stage contract.

A human can edit an output file before the next stage runs. That edited file is the handoff. The workflow does not rely on hidden memory or an orchestration database.

## Canonical Sources

One rule has one authoritative home.

- Engineering governance: agent.md.
- Documentation routing: docs/README.md.
- Workflow configuration: workspaces/sitolo-engineering/_config/workflow-policy.md.
- Stage procedure: that stage's CONTEXT.md.
- Stable stage-specific knowledge: that stage's references or workspace skills.
- Run-specific state: stage output files.

Previous stage outputs are not style guides or policy sources.

## Human Gates

Consequential stages contain a Human Check. The agent produces the stage artifact, the human reviews or edits it, and only then should the next stage consume it.

This keeps the workflow inspectable and makes every intermediate decision an explicit edit surface.

## Mechanical Validation

The repository validates the ICM structure with:

```
./scripts/ci/check-icm-workspace
```

The canonical repository verifier also invokes this check. CI policy runs it for workflow/workspace changes.

Validation checks include required routing files, numbered stages, stage-contract length, reference length, output-directory structure, Markdown output discipline, and rejection of the old repository-global orchestration directories.

## Why This Fits Sitolo

Sitolo's engineering work is primarily sequential and reviewable:

- requirements are selected before design;
- external facts are established before implementation when necessary;
- existing code and contracts are investigated before planning;
- implementation follows an explicit plan;
- audit is independent of implementation;
- remediation can feed back into audit;
- verification produces reproducible evidence;
- delivery is separate from implementation.

The filesystem therefore provides a visible state machine without creating a second application that has to understand Sitolo's domain.

## Scope Boundary

ICM is the workflow architecture, not a replacement for domain architecture, CI, PostgreSQL, workers, external integrations, or application runtime design.

For dynamic real-time collaboration among multiple agents or high-concurrency distributed execution, a dedicated coordination system may be justified. That is outside this repository's base engineering workspace.
