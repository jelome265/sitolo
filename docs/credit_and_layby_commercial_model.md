# Sitolo — Merchant Credit & Lay-by Commercial Model

**Status:** Commercial/product boundary document  
**Date:** 2026-09-22  
**Scope:** merchant-recorded receivables and lay-by workflows

---

## 1. Purpose

“Customer owes me money” can be a core business workflow.

Sitolo should model the merchant's obligation records without confusing them with Sitolo lending.

---

## 2. Core Distinction

```
MERCHANT SELLS ON CREDIT
→ SITolo RECORDS RECEIVABLE

SITOLO FINANCES CUSTOMER
→ FINANCIAL PRODUCT
→ REGULATORY REVIEW
```

---

## 3. Credit Record

A merchant credit record should identify:

- customer;
- originating sale;
- total obligation;
- payments;
- outstanding amount;
- due date/terms where relevant;
- actor;
- status.

---

## 4. Lay-by Record

Lay-by may involve:

- reserved item;
- customer;
- agreed price;
- deposits;
- remaining amount;
- due dates;
- release/collection condition;
- cancellation/refund terms.

The exact business rules must be validated per merchant type.

---

## 5. Credit Lifecycle

```
CREATED
→ PARTIALLY PAID
→ DUE
→ OVERDUE
→ SETTLED
```

Where applicable:

- disputed;
- written off;
- cancelled.

Write-offs require authorization and audit.

---

## 6. Credit Visibility

Owner needs:

- amount outstanding;
- ageing;
- customer history;
- payments;
- overdue accounts;
- concentration.

Avoid exposing unnecessary customer personal information.

---

## 7. Credit Limits

Merchant-defined credit limits may be supported as business policy.

Do not automatically convert this into Sitolo underwriting.

---

## 8. Repayment Recording

Each repayment must identify:

- obligation;
- payment method;
- amount;
- actor;
- timestamp;
- reference;
- reconciliation state.

---

## 9. Credit + Cash

A credit sale must not be counted as cash received.

```
SALE VALUE
→ RECEIVABLE

PAYMENT LATER
→ CASH / MOBILE MONEY / BANK
```

---

## 10. Credit + Inventory

The sale affects inventory according to sale finalization rules even if the customer pays later, subject to the domain model.

---

## 11. Credit + Reconciliation

Receivables should flow into the overall business position:

```
CASH
+
MOBILE MONEY
+
BANK
+
RECEIVABLES
−
PAYABLES
=
BUSINESS POSITION
```

This is an analytical view, not an accounting standard.

---

## 12. Customer Privacy

Credit information may be sensitive.

Apply:

- minimization;
- access control;
- audit;
- limited export;
- retention rules.

---

## 13. Commercial Value

Test whether merchants value:

- overdue reminders;
- ageing;
- customer balance;
- repayment history;
- credit visibility;
- lay-by tracking.

---

## 14. Pricing Hypothesis

Basic customer balances may belong in core.

Advanced credit/lay-by controls may be:

- Growth;
- Multi-Branch;
- Enterprise;
- optional module.

Price according to incremental value and support/compliance cost.

---

## 15. Regulatory Boundary

Recording merchant receivables is not the same as issuing loans.

Before any Sitolo-funded credit product:

- legal classification;
- licensing review;
- partner requirements;
- KYC/AML requirements;
- responsible-lending controls;
- capital/funding model.

must be resolved.

---

## 16. Final Contract

```
TRACK MERCHANT CREDIT
≠
BECOME THE LENDER
```

The core feature is business control; financial-product expansion requires a separate perimeter.