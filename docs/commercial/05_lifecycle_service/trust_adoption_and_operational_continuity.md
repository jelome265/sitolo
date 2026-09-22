# Sitolo — Trust, Adoption & Operational Continuity Model

**Status:** Commercial operating-model control document  
**Date:** 2026-09-22  
**Scope:** trust, reliability perception, device loss, offline continuity, recovery, data ownership, employee control

---

## 1. Commercial Thesis

For a merchant moving from paper or informal records to software, trust is part of product-market fit.

The merchant must believe:

```text
MY RECORD IS SAFE
MY MONEY RECORD IS TRACEABLE
MY STOCK IS EXPLAINABLE
MY BUSINESS SURVIVES DEVICE LOSS
MY DATA IS RECOVERABLE
MY STAFF ACTIONS ARE VISIBLE
```

---

## 2. Trust Domains

| Trust domain | Merchant question |
|---|---|
| availability | Can I use it when I need it? |
| correctness | Can I trust the stock and sales numbers? |
| continuity | What happens when network disappears? |
| recoverability | What happens if my phone is lost? |
| accountability | Can I see what staff changed? |
| privacy | Who can see my business/customer information? |
| control | Can unauthorized people change records? |
| portability | Can I get my data out? |
| support | Who helps when something breaks? |

---

## 3. Trust Is Observable

Do not rely only on satisfaction surveys.

Measure:

- sync success;
- duplicate command rate;
- failed operations;
- recovery success;
- support contacts;
- export completion;
- authorization denials;
- audit coverage;
- reconciliation exceptions;
- device replacement completion.

---

## 4. Stock Trust

A merchant needs to understand why a stock number changed.

The product should make a stock position explainable through:

```OPENING
+
RECEIPTS
+
TRANSFERS
−
SALES
−
RETURNS
±
ADJUSTMENTS
=
CURRENT POSITION
```

The existing inventory ledger and domain rules remain authoritative.

---

## 5. Money Trust

The commercial model must distinguish:

- sale;
- payment;
- cash event;
- settlement;
- reconciliation;
- correction.

Never collapse these into a single generic “money received” number.

---

## 6. Employee Accountability

The owner must be able to determine:

- who logged in;
- who recorded a sale;
- who changed a price;
- who voided;
- who refunded;
- who adjusted stock;
- who opened/closed a register;
- who approved a sensitive action.

Auditability is a commercial control feature.

---

## 7. Device Loss

Business state must survive:

- broken phone;
- stolen phone;
- replaced SIM/device;
- operating-system reset;
- migration to a new device.

Recovery flow:

```
NEW DEVICE
→ AUTHENTICATE
→ AUTHORIZE
→ RESTORE ALLOWED STATE
→ SYNC
→ VERIFY
```

Security controls must remain intact.

---

## 8. Owner Absence

The business should support controlled delegation.

Possible roles:

- owner;
- administrator;
- manager;
- cashier;
- stock user.

Owner absence does not justify bypassing normal authorization.

---

## 9. Offline Scenario

Validation scenario:

```DAY 1 ONLINE
DAY 2 OFFLINE
DAY 3 OFFLINE
DAY 4 ONLINE
```

Validate:

- local durability;
- queued commands;
- duplicate rejection;
- conflict classification;
- server validation;
- final stock;
- sales;
- payments;
- user attribution;
- timestamps;
- reports.

---

## 10. Offline User Experience

The UI should clearly communicate:

- offline status;
- what can be done;
- what is queued;
- what is confirmed;
- what is waiting for server validation.

Avoid ambiguous “success” states for operations that are only locally recorded.

---

## 11. Offline Financial Trust

The device may preserve continuity.

It must not become permanent authority over:

- final financial truth;
- cross-device conflicts;
- provider settlement;
- regulated external outcomes.

The server remains authoritative under the existing architecture.

---

## 12. Sync Failure Recovery

When sync fails:

```
LOCAL DURABLE COMMAND
→ RETRY
→ BACKOFF
→ SERVER VALIDATION
→ ACCEPT / REJECT / CONFLICT
```

The merchant should never need to manually recreate an accepted command solely because a network request timed out.

---

## 13. Backup Trust

The product should communicate the difference between:

- local device state;
- server state;
- backups.

Backups are operational recovery mechanisms, not a substitute for transactional correctness.

---

## 14. Data Export as Trust

Merchant-facing data portability should be clear.

Possible export categories:

- sales;
- inventory;
- products;
- customers;
- suppliers;
- payments;
- cash;
- reconciliation;
- audit evidence where contractually permitted.

Exports should be scoped and audited.

---

## 15. Privacy Trust

Only collect data needed for the business purpose.

Customer personal data should be minimized.

Sensitive information must not be used merely because the database can store it.

The Malawi Data Protection Authority states that it regulates processing of personal information under the Data Protection Act 2024 and provides registration services for applicable controllers/processors. Sitolo therefore treats data protection as an operating requirement, not a future add-on.

Reference: https://www.dpa.mw/download/data-protection-act-2024/

---

## 16. Security Communication

Avoid exaggerated claims.

Do not claim:

- “unhackable”;
- “100% secure”;
- “zero fraud.”

Use measurable descriptions:

- audit trail;
- least privilege;
- encrypted transport;
- device recovery;
- backup policy;
- incident process.

---

## 17. Trust Event Taxonomy

Track trust-sensitive events:

- sync failure;
- data discrepancy;
- unauthorized access;
- suspicious activity;
- recovery attempt;
- export request;
- support escalation;
- payment dispute;
- stock discrepancy.

---

## 18. Adoption Barriers

Common hypothesis categories:

- fear of losing records;
- unfamiliar terminology;
- network concern;
- device concern;
- cost concern;
- staff resistance;
- distrust of cloud systems;
- fear of tax visibility;
- fear of exposing business performance.

These are research questions, not assumptions about every merchant.

---

## 19. Adoption Intervention Matrix

| Barrier | Product/business response |
|---|---|
| unfamiliarity | task-first UX |
| network concern | bounded offline continuity |
| device loss | recoverable account model |
| stock distrust | explainable ledger/history |
| employee concern | roles + audit |
| data concern | export + privacy controls |
| support fear | clear escalation |
| cost | segment-appropriate packaging |

---

## 20. Trust Proof During Sales

For SME/enterprise prospects, demonstrate:

- tenant isolation;
- authorization;
- audit history;
- recovery;
- export;
- backup/recovery posture;
- incident handling;
- provider boundaries.

Proof should match the audience.

---

## 21. Trust and Retention

Measure whether trust failures predict churn.

Example analysis:

```
TRUST INCIDENT
→ SUPPORT CONTACT
→ LOWER USAGE
→ PAYMENT RISK
→ CHURN
```

If correlation is observed, trust incidents become retention-risk events.

---

## 22. Continuity Metrics

Track:

- successful device replacements;
- recovery time;
- offline session duration;
- offline command backlog;
- sync recovery rate;
- unresolved conflicts;
- duplicate command rate;
- user-visible discrepancy reports.

---

## 23. Final Trust Contract

```text
OPERATE
→ RECORD
→ EXPLAIN
→ RECOVER
→ EXPORT
→ TRUST
```

Trust is earned by observable control and recovery, not by marketing language.