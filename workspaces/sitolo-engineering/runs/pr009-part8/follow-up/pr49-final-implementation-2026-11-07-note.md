# PR #49 final implementation note — 2026-11-07 (append-only)

**Branch:** `feat/phase4-pr009-audit-outbox`
**Pushed head:** `d6a5b6ff7be951dc341e3a55a419975dc71de009` (local fmt/clippy/config tests PASS; Integration PASS on exact head; Policy failed on doc-reference resolution and is fixed in the follow-up commit; Rust/Security `verify`/`codeql` still pending at that head).
**Mission:** Final implementation & closure execution for PR #49.
**Status:** PARTIAL — see Blocker / Readiness / Smallest unblocking actions below.

## Harness blocker (verification)

The local OpenCode harness `shell` tool fails on every invocation before the
command runs:

```text
The argument 'NYASULU\AppData\Local\Microsoft\WindowsApps\pwsh.exe' is not
recognized as the name of a script file.
```

This is a username-with-space path quoting defect in the harness, reproduced
via direct `shell` and via `subagent`. Consequently `git status`/`rev-parse`,
`cargo fmt/clippy/test/build`, `cargo deny/audit`, and `scripts/ci/*` could not
be executed in this session. No branch change, commit, push, merge, or CI run
is claimed. All test/CI evidence below is therefore `NOT VERIFIED` by this
execution, not failed.

## Implementation applied (source edits in working tree)

### Workstream D — consumer configuration (strict, fail-closed)

- `crates/sitolo-config/src/model.rs`: added `consumer_name`,
  `consumer_sqs_queue_url`, `consumer_sqs_quarantine_queue_url` to
  `AppConfig` and `defaults::Builder` (all `None` by default), with Debug
  redaction retained (URLs/names are not secrets; secret references unchanged).
- `crates/sitolo-config/src/field.rs`: catalogue entries and `ACCEPTED_KEYS`
  for `EVENTS__CONSUMER__NAME`, `EVENTS__CONSUMER__SQS_QUEUE_URL`,
  `EVENTS__CONSUMER__SQS_QUARANTINE_QUEUE_URL` (catalogue/parser contract
  kept in sync).
- `crates/sitolo-config/src/parse.rs`: env parsing for the three keys.
- `crates/sitolo-config/src/validate.rs`: new `check_consumer` (all-or-nothing
  pairing, consumer-name grammar mirroring `ConsumerProcessor::new`,
  same-region FIFO-only URL checks mirroring `SqsFifoConsumerConfig::new`,
  distinct source/quarantine URLs) and extended worker-credential pairing to
  relay **or** consumer destinations. Added `is_sqs_fifo_queue_url` helper.
- `crates/sitolo-config/src/fingerprint.rs`: `write_consumer` section,
  exhaustive inventory update, and per-field change tests for each new field.
- `crates/sitolo-config/src/lib.rs`: `consumer_settings_are_typed_and_invalid_values_fail_closed`
  contract test (valid combination, incomplete pairing, region mismatch).
- `apps/worker/src/main.rs`: configuring any consumer destination fails
  startup with `consumer SQS destination is configured but no approved
  business effect is supplied` instead of silently ignoring queues or
  defaulting to a no-op effect. Relay-only composition is unchanged.
- `docs/environment-configuration-reference.md`, `.env.example`: documented
  the three keys and the startup-failure-until-effect-approved semantics.

The consumer processor (`apps/worker/src/consumer.rs`), SQS transport
(`apps/worker/src/sqs.rs`), durable inbox (`crates/sitolo-persistence/src/consumer_inbox.rs`),
and SNS→SQS ordering already implement the transaction sequence
`RECEIVE → VALIDATE → BEGIN → INBOX MARKER → EFFECT → COMMIT → ACKNOWLEDGE`.
No duplicate/broker component was rebuilt. The concrete business effect
remains `BLOCKED_BY_APPROVED_DECISION` (owner must name the effect);
no production no-op effect exists.

### Workstream E — migration/release governance (repository-side)

- `docs/runbooks/postgresql-migration-release.md` (new): preflight,
  manifest verification (`scripts/ci/check-schema-migrations.py`), dedicated
  migration identity, `__SCHEMA__` handling, catalog/privilege/RLS
  verification, runtime-DDL denial, and forward-fix recovery. Names no
  production owner, account, or credential.
- migrations README (canonical bundle 0001–0006 and manifest unchanged):
  pointer to the runbook added.

Production migration ownership, ledger location, runner, and credentials
remain `BLOCKED_BY_APPROVED_DECISION`.

### Workstream B / Role-IAM / Phase 3 — no code change in this session

Re-read the production route inventory (`apps/api/src/serve.rs`): five
authenticated routes (provision organization, create branch, activate branch,
revoke member, suspend member) derive authority from `SecurityContext`;
`TenancyService` methods taking caller-supplied `ActorRef` remain
reference/test entry points. Generic role-assignment/revocation routes are
correctly absent pending the Product/Security role-risk decision (§6.4);
no route was exposed. Phase 3 remains `PARTIAL` per the existing matrix.
No PostgreSQL test was added because real-DB proof requires the disposable
CI environment and could not be run or reviewed against a compiled baseline
in this harness-blocked session; the existing `membership_transaction_integration.rs`
coverage (including trigger-based audit/outbox rollback) is preserved.

## Readiness (independent)

1. Technical implementation completeness: **PARTIAL** (consumer config + migration procedure done; effect, full mutation matrix, Phase 3 durable lifecycle remain).
2. PR merge readiness: **NOT READY** (exact-head CI on the follow-up fix must be all-green; independent Stage 06 and human approval outstanding).
3. Phase 4 Part 8 closure: **NOT READY** (AUD-01 effect/acknowledgement proof and AUD-05 owner/runner remain).
4. Phase 3 completion: **PARTIAL** (unchanged).
5. Production release readiness: **NOT READY** (migration owner/ledger/credentials, AWS identifiers, effect approval all blocked).
6. Independent audit readiness: **PREPARED FOR STAGE 06, NOT PASSED** (this note is an implementation record, not independent review).

## Smallest unblocking actions

- Harness owner: fix `shell` quoting for user paths containing spaces, then
  run the command block from mission §12 on the pushed head and record
  workflow SHA equality.
- Product/Security owner: approve role-risk classes/SoD/approval rules
  (unblocks §6.4); approve the concrete consumer business effect (unblocks
  consumer startup).
- Database/platform owner: name migration execution identity, ledger, and
  runner; run this runbook in the disposable environment.
- Reviewer: independent Stage 06 re-audit of the config diff + mutation matrix,
  then human approval. PR #49 remains open and unmerged by this execution.

## Verification update — 2026-11-09 (append-only; preserves the record above)

The Harness blocker section above describes the agent session at
2026-11-07 (harness `shell` quoting defect; no commands run from that
session). It is superseded by the owner-run PowerShell 7.6.9 evidence
below. Nothing above is deleted.

**Commits (owner-executed):**

- `d6a5b6ff7be951dc341e3a55a419975dc71de009`: implementation (12 files:
  consumer config + refinements, migration runbook, docs).
- `55dd2c058b63674e8f42e0bde6d3fdfc487cdf2d`: documentation correction
  (note reference fix that resolved the Policy doc-reference failure).

**Local commands (owner PowerShell 7.6.9 at repo root):**

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: PASS.
- `cargo test -p sitolo-config --all-targets --all-features --locked`: PASS (27 + 2).
- `rustup`/full-workspace local runs: blocked by Windows tooling; deferred
  to CI per owner direction. No local PostgreSQL used.

**Exact-head CI on `55dd2c058b63674e8f42e0bde6d3fdfc487cdf2d`** (`gh pr view 49`
`headRefOid` matches all four `headSha`):

- Rust `42657265915` (`architecture`, `toolchain`, `verify`): PASS
- Integration `42657265905` (`application and contract tests with PostgreSQL`): PASS
- Security `42657266191` (`dependency-policy`, `advisories`, `secrets`,
  `offensive-regression`, `codeql` + Advanced Security CodeQL): PASS
- Policy `42657266328`: PASS

Prior-head runs on `d6a5b6f...` (Integration PASS, Security PASS, Policy
FAILURE on doc-reference, Rust completed) are historical evidence only.

**Review findings (2026-11-09) addressed after this update:**

- Missing-region validation inconsistency: `check_consumer` now requires
  the shared relay AWS region whenever consumer configuration is present;
  negative tests for missing region, malformed hostname, and invalid queue
  URLs added. Awaiting new exact-head CI on the fix commit.
- PR description head (`db071f`): to be updated to the current verified head
  by the PR owner.

**Readiness supersession:** the Readiness section above predates the
verified `55dd2c05...` green set. Current status remains NOT READY for
merge pending the missing-region fix CI, independent Stage 06 re-audit,
and human approval. PR #49 remains open and unmerged.
