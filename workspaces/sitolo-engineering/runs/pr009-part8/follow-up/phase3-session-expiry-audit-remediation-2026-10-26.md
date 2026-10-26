---
artifact_type: remediation-extension
status: implemented-partial
run_slug: "pr009-part8"
security_relevant: yes
source_before: "0385aadd2697b9900e77eecd47ab0f71f886c66a"
implementation_source: "b7de7eb0bee7209b8526aa0017b2416efeea53fd"
verification_head: "c387047306dfa580395eccc8cdb6de23a1a8b3a6"
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

Local formatting, locked persistence all-target check, strict persistence
Clippy, PostgreSQL integration-target compilation, migration inventory,
documentation references and diff checks pass. The corrected SQL behavior ran
against the approved disposable PostgreSQL service. All exact-head checks
passed on documentation head `c387047306dfa580395eccc8cdb6de23a1a8b3a6`, which
contains implementation source `b7de7eb0bee7209b8526aa0017b2416efeea53fd`:

- Rust canonical verifier, including both session-expiry PostgreSQL tests:
  [40301735355](https://github.com/jelome265/sitolo/actions/runs/40301735355).
- PostgreSQL application/contract integration:
  [40301735469](https://github.com/jelome265/sitolo/actions/runs/40301735469).
- Security: [40301735345](https://github.com/jelome265/sitolo/actions/runs/40301735345).
- Policy: [40301735372](https://github.com/jelome265/sitolo/actions/runs/40301735372).
- CodeQL: [121503273264](https://github.com/jelome265/sitolo/runs/121503273264).

The Rust verifier log confirms
`postgres_identity_mapping_and_session_authority_fail_closed` and
`postgres_session_expiry_rolls_back_when_audit_insert_fails` passed. The policy
job confirms the System Map source revision is current.

The full Phase 3 lifecycle remains partial, and no independent Stage 06 audit
or human review is claimed.
