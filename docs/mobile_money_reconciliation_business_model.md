# Sitolo — Mobile Money Reconciliation Business Model

**Status:** Commercial operating-model document  
**Date:** 2026-09-22  
**Boundary:** business control and reconciliation; regulated payment activity remains an external/provider boundary

---

## 1. Strategic Thesis

The commercial opportunity is larger than “record mobile money.”

The useful workflow is:

```SALE
→ PAYMENT METHOD
→ MOBILE MONEY
→ SETTLEMENT
→ RECONCILIATION
→ EXCEPTION
→ ACTION
```

This can turn fragmented merchant records into a business-control loop.

---

## 2. Problem

A merchant may have:

- sales in Sitolo;
- cash outside Sitolo;
- mobile-money activity in a provider statement;
- bank activity elsewhere;
- customer credit tracked manually.

The business question becomes:

> Does recorded commercial activity reconcile to the money that should actually exist?

---

## 3. Commercial Value

Potential value:

- identify missing settlement;
- identify duplicate records;
- identify timing differences;
- quantify unresolved exceptions;
- improve daily cash visibility;
- reduce manual reconciliation work.

---

## 4. Reconciliation Sources

Conceptually:

```SITOLO SALES
SITOLO CASH
SITOLO CREDIT
PROVIDER STATEMENT
BANK STATEMENT
        ↓
RECONCILIATION ENGINE
```

Provider/bank data is external evidence.

The internal ledger is not overwritten by imported statements.

---

## 5. Matching Hierarchy

Potential matching signals:

1. provider transaction/reference;
2. merchant reference;
3. amount;
4. currency;
5. date/time window;
6. payer/payee identifier;
7. contextual metadata.

Exact matching rules remain technical/domain decisions.

---

## 6. Exception Types

- missing provider transaction;
- missing internal transaction;
- duplicate;
- amount mismatch;
- timing difference;
- fee difference;
- unknown transaction;
- reversal;
- refund;
- settlement delay.

---

## 7. Exception Workflow

```
DETECTED
→ REVIEW
→ EVIDENCE
→ CLASSIFY
→ RESOLVE / ESCALATE
→ AUDIT
```

Never mark an exception resolved without a reason/evidence.

---

## 8. Merchant Dashboard

A useful summary:

| State | Meaning |
|---|---|
| matched | evidence aligns |
| pending | expected event not final |
| exception | mismatch requiring review |
| unknown | insufficient evidence |
| reversed | prior event subsequently reversed |

---

## 9. Daily Reconciliation

Recommended operational rhythm:

```
CLOSE BUSINESS DAY
→ IMPORT / RECEIVE STATEMENT
→ MATCH
→ REVIEW EXCEPTIONS
→ CONFIRM POSITION
```

The exact close process depends on merchant maturity.

---

## 10. Multi-Branch

At multi-branch level:

```BRANCH A
BRANCH B
BRANCH C
   ↓
CENTRAL RECONCILIATION
```

Branch ownership must remain explicit.

---

## 11. Enterprise

Enterprise may require:

- multiple providers;
- multiple bank accounts;
- business-unit reconciliation;
- centralized exceptions;
- approval workflows;
- evidence exports;
- SLA reporting.

---

## 12. Pricing Hypothesis

Reconciliation may be:

- included in Growth;
- partially included in Multi-Branch;
- advanced in Enterprise;
- optional add-on where incremental cost/value supports it.

This is a pricing hypothesis.

---

## 13. Retention Hypothesis

Reconciliation may produce stronger retention because it connects daily operations to financial control.

Validate:

- weekly use;
- exception-resolution frequency;
- reduced manual work;
- customer-reported value;
- renewal impact.

Do not assume retention impact before evidence.

---

## 14. Regulatory Boundary

Sitolo should distinguish:

```
RECONCILE RECORDS
≠
HOLD FUNDS
≠
SETTLE FUNDS
≠
OPERATE PAYMENT SYSTEM
```

Any feature that moves beyond information/reconciliation into payment execution must pass the regulatory/provider gate.

---

## 15. Provider Failure

If a provider statement is unavailable:

- retain existing internal state;
- mark external feed freshness;
- permit supported internal operations;
- reconcile later;
- communicate stale data.

Never invent settlement.

---

## 16. Fees

Reconciliation should model provider fees where observable:

```
GROSS COLLECTION
−
PROVIDER FEE
=
EXPECTED NET
```

Differences are reconciliation data.

---

## 17. Commercial Metrics

- reconciliation adoption;
- statements imported;
- matched percentage;
- exception rate;
- exception age;
- resolution time;
- unresolved monetary value;
- support contacts;
- retention by adoption cohort.

---

## 18. Final Contract

The product value is:

> Convert fragmented transaction records into a traceable business money position.

It is not permission to become an unlicensed payment operator.