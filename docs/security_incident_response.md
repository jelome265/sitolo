# Sitolo Security Incident Response

**Status:** Current security operations contract  
**Primary owner:** Trust & Compliance  
**Technical response owner:** Engineering

## Purpose

Provide a controlled response lifecycle for security incidents. Applicable law, regulator direction, signed contracts and incident-specific professional advice remain higher authority.

## Incident classes

| Class | Examples |
|---|---|
| Identity | credential theft, session compromise, MFA abuse, device compromise |
| Authorization | tenant escape, branch escape, privilege escalation, IDOR |
| Data | unauthorized disclosure, export abuse, sensitive-data exposure |
| Integrity | financial/inventory manipulation, replay, audit tampering |
| External trust | payment/webhook/fiscal impersonation or mismatch |
| Availability/abuse | resource exhaustion, retry amplification, destructive workload |
| Supply chain | compromised dependency, CI action, artifact or deployment identity |
| Administrative | privileged/support-access misuse |

## Response lifecycle

```text
DETECT
  ↓
TRIAGE
  ↓
CONTAIN
  ↓
PRESERVE EVIDENCE
  ↓
SCOPE / BLAST RADIUS
  ↓
CORRECT / ERADICATE
  ↓
RECOVER
  ↓
VERIFY
  ↓
NOTIFY / COORDINATE WHEN REQUIRED
  ↓
REGRESSION CONTROL
  ↓
POST-INCIDENT REVIEW
```

## Immediate rules

1. Protect tenant, authorization, financial and evidence boundaries before restoring convenience.
2. Preserve relevant audit, telemetry, deployment and artifact evidence.
3. Never copy secrets into tickets, chat, incident records or evidence bundles.
4. Reconcile business truth against authoritative state, not telemetry alone.
5. Constrain or disable an affected capability when continued execution could increase integrity loss.
6. Record source revision, artifact identity and non-secret configuration fingerprint.
7. Suspected tenant isolation, authorization, financial-integrity, payment/fiscal, privileged-access or supply-chain compromise is escalated to Trust & Compliance and Engineering.

## Evidence

Preserve, where applicable:

- source revision and deployed artifact digest;
- non-secret configuration fingerprint;
- request/operation/command identifiers;
- audit evidence;
- security telemetry;
- authentication/session/device evidence;
- database transaction evidence;
- provider callback/transaction evidence;
- deployment/access evidence;
- safe reproduction artifacts.

Evidence must be access-controlled and integrity-protected.

## Recovery gate

Service restoration does not itself close an incident. Closure requires authoritative state reconciliation, credential/key actions as required, restored monitoring, validated controls, required coordination, and a documented regression control or explicit residual-risk decision.

## Post-incident control

Every material incident produces at least one of: a new/strengthened security test, a control-register update, a security contract update, a runbook update, or a formally recorded residual-risk decision.

## Related runbooks

- docs/runbooks/secret-compromise.md
- docs/runbooks/configuration-poisoning.md
- docs/runbooks/error-storm.md
- docs/runbooks/telemetry-blackout.md
