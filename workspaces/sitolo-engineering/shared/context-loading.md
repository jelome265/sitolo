# Context Loading Rules

This file is the shared loading policy for the engineering workspace.

1. The active stage `CONTEXT.md` is the stage control surface.
2. Load only files named by the active stage Inputs table.
3. Inputs must be understood as either **Working** (this run) or **Reference** (stable factory/context).
4. Previous outputs are working artifacts, not authoritative design sources.
5. Stable rules come from `agent.md` through the recursive governance router, plus the exact project sources routed by `docs/README.md`.
6. Routing files are catalogs. They select context; they do not become substitute sources of truth.
7. Large L3 collections must grow their own `CONTEXT.md` router rather than being pulled wholesale into every stage.
8. Keep the full stage context—entry, contract, references, and working inputs—roughly in the 2,000–8,000 token operating band. If it grows, split, tighten, or push detail down.
9. Every handoff is an explicit artifact path. The next stage consumes the declared artifact, not an inferred replacement.
10. A human check is an explicit action performed by a person. CI may verify structure, syntax, references, and executable evidence; CI completion is never human approval.
11. Missing declared input means stop and report the missing handoff. Never silently substitute another file.
12. A contract, map card, or routing entry can describe intended behavior, but implementation claims require source evidence and tests that exercise the claim.
13. A mechanical cold walk proves filesystem routing and template mechanics. A true cold-agent acceptance requires an actual fresh agent to traverse the workspace and perform a meaningful run.
