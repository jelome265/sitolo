# Governance Context Router

One job: route an agent to the smallest authoritative subset of `agent.md` needed for the active engineering stage.

## Inputs

| Source | File/Location | Scope | Why |
|---|---|---|---|
| Governance authority | ../../../../agent.md | Named sections below | Canonical engineering policy |

## Routing

| Question | Load |
|---|---|
| Authority conflict or source precedence | `agent.md` §1 Governing Source Hierarchy |
| Runtime/module architecture | `agent.md` §2 Governing Sitolo Architecture |
| How an agent must reason before changing code | `agent.md` §3 Agent Operating Mode |
| Business-truth invariants | `agent.md` §4 Mission: Preserve Business Truth |
| Trust boundaries and server authority | `agent.md` §5 Clients Request; Server Decides |
| Domain ownership | `agent.md` §6 Domain Ownership Rules |

## Loading rule

Open only the section required by the current contract. Do not load the 79 KB governance document wholesale when one section answers the question.

## Authority rule

This router is navigation, not policy. `agent.md` remains authoritative. A summary here never overrides the source.

## Handoff semantics

Working artifacts prove what the current run found or produced. They do not become long-lived engineering policy merely because a later stage cites them.

## Do NOT load

Do not load unrelated `agent.md` sections, the whole documentation corpus, or unrelated stage/reference families.

## Verification

A governance claim is verified only against the referenced section of `agent.md`; this file exists solely to make that selection explicit and auditable.
