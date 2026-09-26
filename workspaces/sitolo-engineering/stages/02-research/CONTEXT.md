# 02 Research

One job: establish whether current external evidence is required and, when required, produce it.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Working | Run brief | ../01-select/output/[run-slug]-brief.md | Full file | Defines the research question |
| Reference | Context policy | ../../shared/context-loading.md | Full file | Loading discipline |
| Reference | Research guide | references/research-evidence.md | Full file | Evidence rules |
| Reference | Security context | ../../shared/security-context/CONTEXT.md | Conditional when security_relevant=yes | Security controls and current external standards |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Read the run brief security applicability. If `security_relevant: yes`, load `shared/security-context/CONTEXT.md` and the applicable control IDs.
2. State the exact claim or decision that may require external evidence.
3. Decide whether repository evidence is sufficient.
4. If external evidence is required, search authoritative current sources for the relevant jurisdiction, provider, standard, version, or date.
5. Record source, date/version, authority level, claim, implication, and uncertainty.
6. If external research is not required, record that conclusion and the evidence supporting it.
7. Separate documented fact from inference and recommendation.
8. Write the research memo to output/.
9. If research changes scope, return to 01-select for an edited brief.

## Human Check

Review the evidence decision, sources, and unresolved questions. Edit the memo before investigation.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Research memo | output/[run-slug]-research.md | Markdown |
