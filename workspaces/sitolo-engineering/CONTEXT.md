# Sitolo Engineering Pipeline

Run one engineering request through the stages in order. Each stage is a file-based contract and handoff.

## Pipeline

01-select → 02-research → 03-investigate → 04-plan → 05-implement → 06-audit → 07-remediate → 08-verify → 09-deliver
                                                                               ↑
                                                                               └── re-audit after fixes

Research is always a stage. It may conclude that external research is not required. Remediation is always a stage. It may conclude that no fix is required.

## Task Routing

| Task Type | Go To | Input |
|---|---|---|
| Start a change | stages/01-select/CONTEXT.md | User request |
| Continue current run | current stage CONTEXT.md | Existing handoff |
| Re-audit after remediation | stages/06-audit/CONTEXT.md | Latest remediation output |
| Show status | scan stage output directories | files other than .gitkeep |

## Shared Resources

| Resource | Location | Contains |
|---|---|---|
| Workspace policy | _config/workflow-policy.md | Stable workflow settings |
| Shared context | shared/ | Cross-stage material |
| Skills | skills/ | Bundled engineering knowledge |
| Project governance | ../../agent.md | Authority and engineering rules |
| Project docs map | ../../docs/README.md | Selective document routing |

## Status

An output directory containing only .gitkeep means PENDING. Any other file means an artifact exists. File presence does not prove quality or approval.
