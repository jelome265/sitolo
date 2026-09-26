# Audit Contract

## Purpose

Stage 06 is an independent semantic review of an implemented change. Its job is to detect incorrect behavior that mechanical tests, static analysis, or CI may not exercise.

## Required trace

For every material requirement:

`contract → actual code path → data/state boundary → test or other evidence → audit disposition`

The auditor must inspect implementation evidence directly. A contract, comment, test name, coverage percentage, or green CI result is not proof by itself.

## Mandatory audit questions

1. What behavior is required?
2. What code path actually implements it?
3. Which invariants and trust boundaries must remain true?
4. Which inputs, states, failures, retries, and concurrent executions can violate those invariants?
5. Do the tests exercise the boundary that matters, including negative behavior where applicable?
6. Could the implementation be locally plausible but globally wrong?
7. Does the implementation contradict any authoritative contract or existing invariant?
8. What remains unproven or uncertain?

## Disposition

- **PASS:** requirement is traced to implementation and sufficient evidence; no material unresolved flaw is identified.
- **PARTIAL:** behavior is partly supported, or evidence is materially incomplete.
- **FAIL:** implementation violates the requirement or contains a material correctness/security defect.
- **N/A:** requirement is demonstrably outside the changed surface.

CI results belong in the evidence column. They never bypass Stage 06.

## Audit closure

Stage 06 is complete only when every material requirement has a disposition, material findings are routed to remediation, and limitations are explicitly recorded.
