# Sitolo — Business Intelligence, Alerts & Action Model

**Status:** Product/commercial strategy document  
**Date:** 2026-09-22  
**Purpose:** define the value layer beyond record keeping

---

## 1. Strategic Shift

The long-term product should progress:

```RECORD
→ UNDERSTAND
→ DETECT
→ ALERT
→ RECOMMEND
→ ACT
```

This is what makes Sitolo a Business Operating System rather than a digital notebook.

---

## 2. Source of Truth

Insights must derive from authoritative business records.

Possible sources:

- sales;
- inventory;
- purchases;
- prices;
- cash;
- payments;
- reconciliation;
- customers;
- suppliers.

---

## 3. Example Insight Classes

### Margin

“Sales increased while gross margin fell.”

### Stock

“Fast-moving item is approaching stockout.”

### Cash

“Recorded cash differs from expected cash.”

### Procurement

“Purchase price for an important product increased.”

### Credit

“Outstanding customer balances are ageing.”

### Operations

“Refund/void activity is unusual relative to prior periods.”

These are examples of system-generated signals, not guaranteed predictions.

---

## 4. Insight Pipeline

```
RAW DOMAIN EVENTS
→ VALIDATED READ MODEL
→ DERIVED METRIC
→ THRESHOLD / MODEL
→ EXPLANATION
→ ALERT
→ USER ACTION
```

Do not build recommendations directly on mutable client state.

---

## 5. Explainability

Every material insight should be able to answer:

- what happened;
- compared with what;
- over what time period;
- which records support it;
- why it matters;
- what action is suggested.

---

## 6. Example

```
SALES +18%
GROSS MARGIN -6%
        ↓
MARGIN COMPRESSION SIGNAL
        ↓
SHOW TOP PRODUCTS CAUSING CHANGE
        ↓
SUGGEST PRICE / COST REVIEW
```

The system may recommend review.

It should not silently change prices.

---

## 7. Alert Priority

Use:

- informational;
- useful;
- important;
- critical operational.

Avoid flooding users.

---

## 8. Owner vs Staff

### Owner

Business-level insights.

### Manager

Operational exceptions.

### Worker

Task-specific actions.

Do not expose sensitive management analytics to users without permission.

---

## 9. Alert Lifecycle

```
GENERATED
→ DELIVERED
→ VIEWED
→ ACKNOWLEDGED
→ ACTIONED / DISMISSED
```

Track the outcome.

---

## 10. Action Integration

The useful endpoint is not the alert.

It is:

```
ALERT
→ ACTION
→ OUTCOME
```

Examples:

- reorder;
- review price;
- investigate cash variance;
- approve stock adjustment;
- contact overdue customer.

---

## 11. Recommendation Safety

Recommendations cannot:

- bypass authorization;
- write financial truth without command;
- mutate inventory silently;
- override tax rules;
- bypass approval.

Human or explicitly authorized workflow remains responsible for final action.

---

## 12. Cold Start

A new merchant has little history.

Use:

- simple rule-based signals;
- business configuration;
- explicit thresholds;
- domain knowledge.

Do not pretend a predictive model is statistically reliable without data.

---

## 13. Data Quality

An insight must be suppressed or marked uncertain when:

- data is stale;
- reconciliation is unresolved;
- offline sync is incomplete;
- transaction history is too sparse.

Bad inputs should not produce confident recommendations.

---

## 14. Commercial Value Test

Measure:

- alert open rate;
- action rate;
- time to action;
- outcome improvement;
- retention;
- willingness to pay.

“AI usage” is not a value metric by itself.

---

## 15. Packaging

Potential packaging:

- basic insights in Duka/SME;
- advanced operational intelligence in Growth;
- consolidated intelligence in Multi-Branch;
- governed enterprise analytics in Enterprise.

This is a hypothesis.

---

## 16. Final Contract

The intelligence layer should answer:

> What does Sitolo know from the business record that the owner should act on?

It must be useful, explainable, permissioned and evidence-based.