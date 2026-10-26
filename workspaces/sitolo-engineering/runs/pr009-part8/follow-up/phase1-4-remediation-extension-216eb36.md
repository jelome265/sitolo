---
artifact_type: remediation-extension
status: exact-head-passed-with-open-blockers
run_slug: "pr009-part8"
security_relevant: yes
source_head: "216eb369cd01e107c14b8e654ce6f56ff305416b"
date: "2026-10-26"
---

# Phase 1–4 remediation extension — renew SQS consumer visibility

## Finding and change

The SQS adapter requested batches of up to ten FIFO messages with a 60-second visibility timeout, while `ConsumerProcessor::process_batch` handled messages serially. A slow first message could therefore leave later messages in the same batch visible to another worker before the first worker reached them. In addition, the consumer trusted the SQS message-group attribute for quarantine routing without verifying it matched the organization and aggregate encoded in the event.

Commit `216eb369cd01e107c14b8e654ce6f56ff305416b` requests one message per worker receive and renews its visibility lease every 30 seconds for another 60 seconds while parsing, database work, effect, or quarantine is in progress. The visibility-extension API call has a five-second cap. If renewal fails, the processing future is cancelled and its database transaction is dropped; the source message remains unacknowledged for retry. The processor computes the expected group ID through the same producer helper and quarantines mismatches before database access. A missing group ID fails without acknowledgement so configured queue redrive can handle it. Different worker instances can still process separate FIFO groups concurrently.

## Evidence

- Focused `cargo test -p sitolo-worker-bin --lib --locked`: PASS, 19 tests, including visibility renewal and rejection of mismatched/missing group attributes before database access.
- Focused strict Clippy: `cargo clippy -p sitolo-worker-bin --all-targets --all-features --locked -- -D warnings` — PASS.
- Workspace `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — PASS; workspace `cargo check --workspace --all-targets --all-features --locked` — PASS.
- Exact-head Rust [40287439503](https://github.com/jelome265/sitolo/actions/runs/40287439503), Integration [40287439518](https://github.com/jelome265/sitolo/actions/runs/40287439518), Security [40287439599](https://github.com/jelome265/sitolo/actions/runs/40287439599), and Policy [40287439504](https://github.com/jelome265/sitolo/actions/runs/40287439504) all passed on this SHA. No local PostgreSQL or AWS service was used.
- No live SQS delivery, queue redrive/DLQ configuration, IAM policy, or production consumer composition is proven by these unit and CI checks.

## Limits and disposition

This closes a transport-level lease-expiry and group-integrity defect in the consumer code. The worker executable still starts only the SNS relay; no approved consumer effect or queue configuration is composed. Queue redrive policy, resource identity, IAM and operator replay/skip remain deployment/owner work. The concrete business effect and production migration/release remain blocked by their missing owners/decisions. Full mutation and Phase 3 evidence, independent Stage 06 re-audit, human review and branch-protection evidence remain open. PR #49 remains open and unmerged.
