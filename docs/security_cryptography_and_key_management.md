# Sitolo Cryptography & Key Management Standard

**Status:** Current security control standard  
**Primary owner:** Engineering for technical implementation; Trust & Compliance for regulatory/control obligations

## Purpose

Define the lifecycle and trust boundary for cryptographic material without creating a second security policy. Version-sensitive algorithms and parameters remain in the applicable implementation contracts and external standards register.

## Principles

1. Use established, reviewed cryptographic primitives; never invent cryptography.
2. Treat keys and secret-bearing cryptographic material as distinct protected assets.
3. Separate signing and verification capabilities where the threat model permits.
4. Production key material never belongs in source control or ordinary configuration.
5. Secret/key references may be configuration; raw material must not enter general config objects, logs, telemetry or domain entities.
6. Trust-sensitive key changes default to restart/redeployment unless dynamic rotation has explicit compatibility, rollback and audit semantics.
7. Algorithm or parameter changes require migration/compatibility and rollback analysis.
8. Suspected key compromise is a security incident.

## Key classes

| Class | Examples | Required controls |
|---|---|---|
| Authentication/signing | token/signing keys | restricted storage/consumers, rotation, verification evidence |
| Device/session | device credentials, session material | narrow scope, expiry, revocation, secure storage |
| Integration | payment/webhook/MRA credentials | provider isolation, rotation, audit |
| Data protection | encryption/wrapping keys | approved key authority, versioning, recovery plan |
| Build/release | signing/provenance identities | protected CI identity and least privilege |

## Lifecycle

```text
Generate / provision
      ↓
Store in approved secret/key authority
      ↓
Distribute by reference to narrow consumer
      ↓
Use for bounded purpose
      ↓
Monitor / audit
      ↓
Rotate
      ↓
Migrate / overlap where required
      ↓
Revoke / retire
```

## Rotation

A rotation plan must specify key/version identity, consumers, overlap semantics if supported, new-key validation, cutover, old-key revocation, rollback, and completion evidence.

Rotation must not silently invalidate durable audit or evidence verification.

## Compromise

```text
Identify key class
→ contain affected capability
→ revoke / rotate
→ invalidate dependent credentials/sessions where required
→ verify replacement
→ inspect affected evidence/artifacts
→ add regression control
```

Operational response uses `docs/runbooks/secret-compromise.md`.

## Algorithm governance

Exact algorithms and parameters are recorded in the applicable implementation contract and checked against the current `external_standards_verification_register.md`. This standard intentionally does not duplicate version-sensitive parameters.

## Verification

Cryptography-related changes must identify security control IDs, implementation evidence, applicable negative tests, lifecycle proof when changed, and the relevant CI/security gates.
