# Proposed IAM Lifecycle Event Mapping

**Status:** **PROPOSED — NOT APPROVED**  
**Binding catalogue:** `docs/phase4_part8_audit_outbox_implementation_contract.md` §6.2 and `crates/sitolo-audit/src/iam_event.rs`.

## Decision

Treat all five undocumented state-changing paths as mandatory durable audit evidence and paired outbox intent. Add distinct event names instead of reusing `MembershipInvited`, `MembershipActivated` or an event with different semantics. Each new event starts at schema version **1**.

This proposal does not add code variants, change the binding contract or close the findings by itself.

## Mapping matrix

| Operation | Actual transition | Proposed event | Version | Actor | Atomicity |
|---|---|---|---:|---|---|
| `mark_member_pending` | Membership `INVITED → PENDING_ACCEPTANCE` | `iam.membership.pending_acceptance` | 1 | Request principal from server SecurityContext; if system-driven, named system actor | State + audit + outbox in one transaction |
| `expire_invitation` | Membership/enrollment `PENDING_ACCEPTANCE → EXPIRED` | `iam.membership.expired` | 1 | `system:invitation-expiry` for automated expiry; actual principal for an authenticated operator | State + audit + outbox in one transaction |
| `create_invitation` | Invitation `∅ → ISSUED`, linked membership created as `INVITED` | `iam.invitation.issued` | 1 | Authenticated issuer from server SecurityContext | Membership + invitation + audit + outbox atomic |
| `revoke_invitation` | Invitation `ISSUED → WITHDRAWN`; linked membership released if changed | `iam.invitation.withdrawn` | 1 | Authenticated issuer from server SecurityContext | Every changed row + required audit/outbox atomic |
| `expire_invitations` sweep | Each invitation `ISSUED → EXPIRED`; linked membership expires if its state changes | `iam.invitation.expired` per invitation; also `iam.membership.expired` only for an actual linked-membership transition | 1 each | `system:invitation-expiry-sweeper` | Per-invitation transition(s) + matching audit/outbox rows atomic |

**No mandatory-audit exclusion is proposed.** A batch-run summary, including a run that expires zero rows, belongs in operational telemetry; it does not replace the audit event for each committed business transition.

## Proposed v1 payload fields

The top-level outbox envelope retains its existing event ID/name/version, timestamp, organization/branch scope, aggregate identity/sequence and payload fields. Delivery bookkeeping (claim token, retry count, status, lock/publish fields) must not enter the published payload.

| Event | Minimal payload fields | Never include |
|---|---|---|
| `iam.membership.pending_acceptance` | `membership_id`, `from_state`, `to_state`, `state_version` | token/hash, credential, raw request |
| `iam.membership.expired` | `membership_id`, `from_state`, `to_state`, `state_version`, bounded `expiry_reason` | token/hash, password, contact/email |
| `iam.invitation.issued` | `invitation_id`, `membership_id`, `expires_at`, `state_version` | raw token, token hash, contact/email, provider response |
| `iam.invitation.withdrawn` | `invitation_id`, `membership_id`, `from_state`, `to_state`, `state_version`, bounded reason if supplied | raw token, token hash, contact/email |
| `iam.invitation.expired` | `invitation_id`, `membership_id`, `from_state`, `to_state`, `expires_at`, `state_version` | raw token, token hash, contact/email |

The top-level organization scope is mandatory. Consumers must not treat an event as authorization; they must use their own service identity and authorized read/write paths for further information.

## Actor provenance

- Request-driven transitions take a verified principal/security context from the HTTP boundary; actor and membership references are derived from server state, never a body field.
- Several current methods do not accept a trusted actor. Their signatures/composition must be refactored before production wiring.
- Background expiry uses a stable system subject (`system:invitation-expiry-sweeper`) and explicit operation/source metadata, never a fabricated human. If actor kind must be explicit in the durable schema, extend the schema intentionally under the binding contract.
- `iam.invitation.issued` deliberately differs from `iam.membership.invited`: direct member invitation and token-bound invitation issuance are different transitions.

## Transaction and idempotency requirements

1. Each state mutation, mandatory audit row and outbox row commit in the same authoritative PostgreSQL transaction.
2. If invitation and membership both change, emit events only for rows actually changed, atomically with both changes.
3. Sweep bounded work with concurrency-safe row claiming/locking; duplicate sweepers must not emit duplicate semantic transitions.
4. Re-running a completed sweep emits no duplicate event. Audit/outbox failure rolls back the state change.
5. Retry a relay delivery with the same event ID. Idempotent commands do not emit a second semantic event unless the approved contract defines a new transition.
6. Test concurrent sweeps, invitation accept/revoke/expiry races, duplicate commands, cross-tenant calls, audit failure and outbox failure.
7. Register every accepted variant in `IamEventName::ALL`, `as_str()`, catalogue/schema checks and destination compatibility tests.

**Owner decision required:** approve these five event names, v1 payloads and system-actor conventions, or replace a row with an explicit exclusion and reason. Until then, AUD-03/AUD-04 remain open. Update the binding contract before modifying the enum/source. See [the audit](../audit.md).
