# Sitolo — Retention, Churn & Customer Lifecycle Model

**Status:** Commercial operating-model control document  
**Date:** 2026-09-22  
**Purpose:** distinguish product failure, payment failure, seasonal inactivity, business closure and true churn

---

## 1. Principle

Retention is not one number.

Sitolo must understand:

```WHO LEFT
+
WHY
+
WHEN
+
FROM WHICH SEGMENT
+
THROUGH WHICH CHANNEL
+
AFTER WHAT EXPERIENCE
```

Aggregate churn can hide a broken Duka cohort or an unhealthy enterprise cohort.

---

## 2. Lifecycle

```LEAD
→ QUALIFIED
→ TRIAL
→ ACTIVATED
→ PAID
→ ACTIVE
→ AT RISK
→ PAST DUE
→ GRACE
→ SUSPENDED
→ REACTIVATED
→ CHURNED
```

Business-state classification is separate from subscription-state classification.

---

## 3. Activity State

A customer may be:

- active;
- temporarily inactive;
- seasonally inactive;
- technically blocked;
- payment delinquent;
- suspended;
- business closed.

“Not recently active” is not sufficient evidence of churn.

---

## 4. Churn Definition

Define churn operationally before measuring it.

Candidate definition:

> An organization is churned when its commercial relationship has ended according to subscription state/contract rules and the account is not merely delinquent, seasonally inactive, temporarily inactive, or under a recovery process.

The exact grace/recovery window must be fixed before reporting.

---

## 5. Churn Taxonomy

Primary reason:

- price;
- no perceived value;
- complexity;
- connectivity;
- payment failure;
- support;
- device loss;
- staff problem;
- migration;
- trust/security;
- competitor;
- seasonal/business inactivity;
- business closure;
- regulatory/compliance;
- other.

Secondary reason is optional.

---

## 6. Churn Evidence

Use multiple signals:

- cancellation request;
- payment termination;
- support conversation;
- account status;
- usage decline;
- business closure evidence.

Avoid deriving human intent from analytics alone.

---

## 7. Voluntary vs Involuntary Churn

### Voluntary

Customer actively cancels.

### Involuntary

Access ends because of:

- payment failure;
- fraud/security action;
- contract expiration without renewal;
- account closure.

These should not be blended without explanation.

---

## 8. Seasonal Businesses

Agriculture-linked and other seasonal businesses may have periods of low activity.

Model:

```NO ACTIVITY
≠
NO BUSINESS
≠
CHURN
```

The account may be seasonally inactive while still economically active as a relationship.

---

## 9. Seasonal State

Capture:

- expected active months;
- expected low months;
- business type;
- historical activity pattern;
- subscription status.

Do not infer seasonality from one quiet month.

---

## 10. Seasonal Retention Measurement

For seasonal cohorts:

- compare like periods;
- use annualized retention where appropriate;
- distinguish reactivation from new acquisition;
- do not penalize intentional seasonal closure as normal churn.

---

## 11. Duka Retention

Key signals:

- days active per week;
- sales recorded;
- inventory usage;
- business summary views;
- payment continuity;
- support incidents.

Duka retention should be tied to operating habit, not dashboard login count.

---

## 12. SME/Growth Retention

Add:

- multi-user activity;
- purchasing;
- reconciliation;
- reports;
- staff adoption;
- management review.

---

## 13. Multi-Branch Retention

Key signals:

- active branches;
- central reconciliation;
- management usage;
- branch utilization;
- centralized control.

A branch-level failure must not be hidden inside account-level retention.

---

## 14. Enterprise Retention

Measure:

- production scope;
- users/branches;
- integration utilization;
- SLA performance;
- support incidents;
- procurement/renewal;
- expansion/contraction.

---

## 15. Cohort Rules

Segment by:

- acquisition channel;
- geographic cluster;
- business type;
- tier;
- payment method;
- onboarding mode;
- account age.

Never compare fundamentally different cohorts without marking the difference.

---

## 16. Early-Warning Risk

Potential risk signals:

- declining usage;
- repeated support complaints;
- payment failures;
- unresolved reconciliation exceptions;
- stock discrepancies;
- trust incidents;
- inactive users;
- plan mismatch.

These are triggers for investigation, not automatic churn labels.

---

## 17. Save / Recovery Motions

Potential interventions:

- payment recovery;
- training;
- simplified setup;
- support escalation;
- device recovery;
- plan adjustment;
- migration help.

Record the intervention.

---

## 18. Reactivation

A reactivated customer retains the original organization identity.

Measure:

- time inactive;
- reactivation cause;
- reactivation cost;
- post-reactivation retention.

A reactivated account is not the same economic event as a new logo.

---

## 19. Expansion / Contraction

Track:

```
STARTING ARR
+
EXPANSION
−
CONTRACTION
−
CHURN
=
ENDING ARR
```

For lower-ARPU plans, use MRR or monthly equivalent consistently.

---

## 20. Churn Reporting

Minimum monthly dashboard:

- logo churn;
- revenue churn;
- voluntary churn;
- involuntary churn;
- churn by tier;
- churn by channel;
- churn reason;
- seasonal inactivity;
- business closure;
- reactivation.

---

## 21. Churn Investigation

Every materially elevated cohort should be analysed through:

```ACQUISITION
→ ONBOARDING
→ FIRST VALUE
→ PAYMENT
→ USAGE
→ SUPPORT
→ VALUE
→ CHURN
```

The aim is to locate the failed mechanism, not merely count the loss.

---

## 22. Anti-Patterns

Reject:

- calling all inactive accounts churned;
- using aggregate churn as PMF proof;
- ignoring payment failure;
- ignoring seasonal business patterns;
- using login frequency as the sole health metric;
- hiding churn in plan migrations;
- counting reactivation as new acquisition.

---

## 23. Final Contract

Retention means the customer continues receiving business value and maintains the commercial relationship.

```ACTIVE
≠
LOGIN
≠
PAID
≠
RETAINED
```

The lifecycle model must preserve those distinctions.