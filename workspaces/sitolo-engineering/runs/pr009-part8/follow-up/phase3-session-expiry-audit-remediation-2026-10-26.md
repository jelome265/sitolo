---
artifact_type: remediation-extension
status: implementation-correction-awaiting-exact-head-verification
run_slug: "pr009-part8"
security_relevant: yes
source_before: "0385aadd2697b9900e77eecd47ab0f71f886c66a"
date: "2026-10-26"
---

# Session expiry audit correction

## Review finding

The first expiry-state implementation persisted `idle_expired` and
`absolute_expired` but did not emit the required security audit event. That
violated Phase 3 §1, §37, and §52, which require session state transitions to
be auditable and durable security evidence to be committed with the state
change. Its exact-head CI results on `0385aadd` do not apply to this corrected
candidate.

## Corrected implementation

- Migration 0005 creates the restricted `identity_auth_events` table, owned
  by `app_identity_owner`; API and worker roles have no direct read or write
  access. The existing security-definer session function records the
  `auth.session.expired` event in the same PostgreSQL statement/transaction as
  the terminal state transition.
- The audit row uses bounded event/reason labels, opaque internal subject and
  session references, server time, outcome and security version. A unique
  session/event constraint makes the transition event single-use.
- API runtime startup and readiness now require the audit table, its expected
  owner and the absence of direct runtime privileges.
- PostgreSQL regressions assert expiry denial, matching audit evidence,
  wrong-user no-write behavior, replay without duplicate audit, rollback when
  audit insertion fails, and restricted table privileges.

## Verification status

The corrected PostgreSQL integration target compiles locally. Formatting,
locked persistence all-target check, strict persistence Clippy, migration
inventory, documentation references and diff checks pass. The new SQL behavior
has not yet run against PostgreSQL on this corrected source candidate; do not
reuse the `0385aadd` workflow results as proof. Exact-head CI is required.

The full Phase 3 lifecycle remains partial, and no independent Stage 06 audit
or human review is claimed.
