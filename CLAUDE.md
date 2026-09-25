# Sitolo — ICM Root Router

Sitolo uses a filesystem-routed Interpretable Context Methodology workspace for repeatable engineering work.

## Workspace Map

CLAUDE.md
CONTEXT.md
AGENTS.md
agent.md
docs/
workspaces/
  sitolo-engineering/

## Routing

| Task | Workspace | Entry |
|---|---|---|
| Software change, bug, feature, security fix, migration | workspaces/sitolo-engineering/CLAUDE.md | workspace router |
| Pipeline status | workspaces/sitolo-engineering/CONTEXT.md | status section |
| Workflow setup | workspaces/sitolo-engineering/setup/questionnaire.md | setup |

## What to Load

| Task | Load | Do NOT Load |
|---|---|---|
| Any engineering task | selected workspace router, then selected stage contract | unrelated workspaces, all stage references, all prior outputs |
| Architecture question | agent.md, docs/README.md, then only routed documents | stage outputs unless the question concerns a run |
| Pipeline status | workspace CONTEXT.md | application source and stage references |

The root router only selects a workspace. Stage procedures and project engineering rules live elsewhere.
