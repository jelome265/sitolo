# Sitolo — Fraud, Loss Prevention & Business Control Model

**Status:** Commercial risk/control document  
**Date:** 2026-09-22  
**Scope:** employee fraud, cash leakage, stock leakage, unauthorized actions, owner visibility

---

## 1. Commercial Thesis

Internal loss is a business-control problem.

Sitolo's potential value is not only recording transactions. It can make operational deviations visible.

The target is:

```RECORD
→ CONTROL
→ DETECT
→ INVESTIGATE
→ APPROVE
→ CORRECT
```

---

## 2. Loss Categories

### Sales leakage

- sale not recorded;
- discount abuse;
- price manipulation;
- void abuse.

### Cash leakage

- unrecorded cash;
- false expenses;
- cash variance;
- unauthorized payout.

### Stock leakage

- unexplained adjustment;
- false receiving;
- transfer manipulation;
- theft.

### Account abuse

- credential sharing;
- unauthorized access;
- privilege abuse.

### Reconciliation abuse

- false payment state;
- duplicate entries;
- concealed exceptions.

---

## 3. Control Objectives

Sitolo should make it possible to answer:

- who did it;
- what changed;
- when;
- where;
- under which role;
- whether approval was required;
- what was the original state;
- what is the compensating correction.

---

## 4. Separation of Duties

High-risk actions should be separable from ordinary operation.

Examples:

```
CASHIER
→ records sale

MANAGER
→ approves unusual void/refund

OWNER/FINANCE
→ reviews reports
```

The exact permission model remains governed by the authorization specification.

---

## 5. High-Risk Operations

Potential controls around:

- price changes;
- discounts above threshold;
- refunds;
- voids;
- stock adjustments;
- transfer approvals;
- cash withdrawals;
- reconciliation write-offs;
- user/role changes.

Do not make every event require manual approval; excessive friction destroys usability.

---

## 6. Auditability

Each material mutation should be attributable to:

- organization;
- actor;
- operation;
- resource;
- timestamp;
- before/after state where appropriate;
- reason;
- correlation/command identity.

---

## 7. Cash Variance

Expected cash:

```
OPENING CASH
+
CASH SALES
+
OTHER CASH IN
−
REFUNDS
−
PAYOUTS
=
EXPECTED CASH
```

Compare against counted cash.

Variance becomes an exception.

---

## 8. Stock Variance

```
OPENING STOCK
+
RECEIPTS
+
TRANSFERS IN
−
SALES
−
RETURNS OUT
−
TRANSFERS OUT
±
AUTHORIZED ADJUSTMENTS
=
EXPECTED STOCK
```

Physical counts should be reconciled through explicit operations.

---

## 9. Owner Notifications

Potential alerts:

- unusual void volume;
- repeated price changes;
- large cash variance;
- unusual stock adjustments;
- repeated failed approvals;
- suspicious access.

Notifications should be actionable and not noisy.

---

## 10. Anomaly Detection — Future

A future intelligence layer may identify:

- deviations from normal transaction patterns;
- repeated end-of-day adjustments;
- abnormal discounting;
- employee-level anomalies.

This is a future capability.

Do not claim fraud detection accuracy before evidence exists.

---

## 11. False Positive Control

An alert is not proof of fraud.

Use:

```
SIGNAL
→ REVIEW
→ EVIDENCE
→ DECISION
```

Never automatically accuse a worker based only on analytics.

---

## 12. Commercial Value Measurement

Test whether controls produce:

- lower unexplained variance;
- fewer disputed transactions;
- fewer unauthorized changes;
- higher owner trust;
- higher retention;
- willingness to pay.

---

## 13. Duka vs Enterprise

### Duka

Simple:

- owner visibility;
- basic audit;
- basic variance.

### SME/Growth

Add:

- roles;
- approvals;
- manager review.

### Multi-Branch

Add:

- branch comparison;
- central exception review;
- transfer controls.

### Enterprise

Add:

- advanced IAM;
- approvals;
- centralized audit;
- policy enforcement;
- analytics.

---

## 14. Loss Prevention Must Not Become Surveillance Abuse

Collect only information necessary for business control.

Avoid:

- unnecessary personal monitoring;
- hidden employee profiling;
- indiscriminate biometric collection.

Privacy rules remain authoritative.

---

## 15. Final Contract

The commercial proposition is:

> Know what happened in the business, even when the owner was not there.

That value must come from traceable controls, not accusations.