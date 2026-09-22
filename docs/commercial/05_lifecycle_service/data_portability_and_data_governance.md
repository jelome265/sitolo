# Sitolo — Data Portability & Data Governance Model

**Status:** Commercial trust / enterprise governance document  
**Date:** 2026-09-22

---

## 1. Purpose

Data portability serves three functions:

1. merchant trust;
2. enterprise procurement;
3. safe lifecycle management.

The commercial promise is:

> The business can obtain an understandable copy of its records.

---

## 2. Ownership Principle

Contractual ownership, legal rights and technical custody must be distinguished.

```BUSINESS RECORD
→ STORED BY SITOLO
→ CONTROLLED UNDER CONTRACT / LAW
→ EXPORTABLE ACCORDING TO RIGHTS
```

Do not claim ownership rights beyond the contract and law.

---

## 3. Export Domains

Potential export categories:

- organization;
- branches;
- users/memberships where permitted;
- products/SKUs;
- prices;
- suppliers;
- purchases;
- inventory movements;
- sales;
- returns;
- payments;
- cash events;
- reconciliation;
- customers;
- audit records subject to policy;
- subscription/billing records.

---

## 4. Export Formats

Use formats based on purpose:

| Format | Use |
|---|---|
| CSV | spreadsheet/report migration |
| JSON | machine-readable structured export |
| PDF | human-readable reports/evidence |
| archive package | controlled full-account export |

A format must be documented.

---

## 5. Export Scope

Exports should support:

- date range;
- branch;
- data class;
- full account;
- selected resources where safe.

The system must enforce authorization before generating an export.

---

## 6. Export Security

Exports are high-risk data movement.

Existing architecture already requires:

- authorization;
- rate limits;
- asynchronous generation where appropriate;
- short-lived delivery;
- audit event;
- result count;
- filters;
- actor identity.

Commercial UX must not weaken these controls.

---

## 7. Export Lifecycle

```REQUESTED
→ AUTHORIZED
→ QUEUED
→ GENERATING
→ READY
→ DOWNLOADED
→ EXPIRED
```

Failures should remain auditable.

---

## 8. Data Dictionary

Every structured export should include or reference:

- field name;
- meaning;
- type;
- units;
- time semantics;
- identifier semantics;
- nullable behaviour;
- version.

This makes migration possible without reverse-engineering the database.

---

## 9. Historical Integrity

Exported records should not alter the authoritative source.

A customer export is a read operation.

Corrections happen through domain-approved commands, not by editing export files and re-uploading blindly.

---

## 10. Termination

On cancellation/termination:

- retain records only as required/justified;
- execute deletion according to legal/contractual retention;
- preserve required audit/evidence;
- support permitted export;
- document completion.

---

## 11. Backup vs Customer Export

These are different:

```
BACKUP
=
SITOLO RECOVERY CONTROL

EXPORT
=
CUSTOMER DATA PORTABILITY
```

A backup is not a customer-readable export.

---

## 12. Data Residency

Enterprise contracts should address:

- hosting geography;
- backup geography;
- subprocessors;
- cross-border transfers;
- legal access;
- deletion.

The actual deployment geography remains an approved architecture decision.

---

## 13. Data Protection

The Malawi Data Protection Authority states that it regulates processing of personal information under the Data Protection Act 2024.

Reference: https://www.dpa.mw/download/data-protection-act-2024/

Export functionality must therefore consider personal-data minimization, authorization, access logs and lawful handling.

---

## 14. Enterprise Requirements

Expect questions about:

- retention;
- deletion;
- backups;
- restoration;
- access control;
- audit evidence;
- data residency;
- subprocessors;
- export format;
- contract termination.

These belong in the procurement/security package.

---

## 15. Portability as Retention Strategy

Do not use lock-in as the primary retention mechanism.

The intended sequence is:

```
EXPORTABLE DATA
+
TRUST
+
DEEP WORKFLOW VALUE
→
VOLUNTARY RETENTION
```

The customer stays because leaving loses operational convenience/value, not because leaving destroys access to legitimate records.

---

## 16. Portability Metrics

Track:

- export requests;
- successful exports;
- failed exports;
- export size;
- generation time;
- support contacts;
- post-export cancellation;
- export-related security events.

A spike in export requests may be an early churn or trust signal.

---

## 17. Final Contract

Data portability is a product feature and a governance requirement.

```CUSTOMER DATA
→ CONTROLLED
→ AUDITABLE
→ EXPORTABLE
→ RETAINED ONLY AS JUSTIFIED
```