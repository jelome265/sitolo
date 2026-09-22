# Sitolo — Commercial Metrics, Governance & Decision System

**Status:** Commercial operating-model governance document  
**Date:** 2026-09-22

---

## 1. Purpose

Commercial decisions must be based on consistent definitions.

A metric without a definition is not governance.

---

## 2. Metric Hierarchy

```NORTH-STAR BUSINESS VALUE
        ↓
RETENTION + CONTRIBUTION
        ↓
ACTIVATION + PAID USE
        ↓
ACQUISITION + SUPPORT
        ↓
OPERATIONAL EVENTS
```

Do not optimize top-of-funnel metrics at the expense of contribution.

---

## 3. Core Definitions

| Metric | Definition |
|---|---|
| lead | identifiable prospective customer |
| qualified lead | lead meeting predefined eligibility |
| trial | authorized non/low-paid evaluation state |
| activated | completed first-value condition |
| paid | valid subscription payment recognized |
| retained | remains commercially active under cohort rule |
| expansion | increased recurring commercial value |
| churn | ended commercial relationship under lifecycle definition |
| contribution | net revenue less defined variable cost |
| CAC | acquisition cost divided by acquired customer cohort |

Definitions must remain stable during an experiment.

---

## 4. Segment Dimensions

Every significant commercial metric should be sliceable by:

- tier;
- business type;
- geography;
- acquisition channel;
- onboarding mode;
- payment method;
- account age.

---

## 5. Leading vs Lagging Metrics

### Leading

- activation;
- product usage;
- first-value time;
- support issues;
- payment failures.

### Lagging

- retention;
- contribution;
- churn;
- LTV;
- expansion.

Do not claim long-term success from leading metrics alone.

---

## 6. Source of Truth

Commercial dashboards should use authoritative billing/product/support sources.

Manual spreadsheets may be used for experiments but must preserve:

- source;
- timestamp;
- cohort;
- assumptions;
- calculations.

---

## 7. Experiment Governance

Each experiment requires:

- hypothesis;
- population;
- primary metric;
- secondary metrics;
- sample plan;
- pass threshold;
- fail threshold;
- stop rule;
- decision owner;
- start/end date.

The existing Commercial Validation Plan is the governing experiment framework.

---

## 8. Segment Health

No blended dashboard may be the only dashboard.

Required minimum views:

```
DUKA
SME
GROWTH
MULTI-BRANCH
ENTERPRISE
```

---

## 9. Channel Health

Track:

- acquisition;
- activation;
- payment;
- retention;
- contribution.

A channel can be high-volume and still fail economically.

---

## 10. Payment Health

Track:

- successful collections;
- failures;
- unknown outcomes;
- grace;
- suspension;
- reactivation.

---

## 11. Support Health

Track:

- contacts;
- cost;
- severity;
- resolution;
- support-caused churn.

---

## 12. Trust Health

Track:

- recovery;
- export;
- discrepancy;
- sync;
- security incidents.

---

## 13. Decision States

Use:

- **PROCEED** — evidence clears the predefined gate;
- **HOLD** — evidence incomplete;
- **ITERATE** — hypothesis shows signal but needs refinement;
- **STOP** — predefined failure condition met.

These are experiment-management states, not rankings of customer segments.

---

## 14. Data Integrity Rules

Reject:

- changing metric definitions after seeing results;
- mixing trial and paid cohorts;
- counting reactivation as new customer without labeling;
- counting seasonal inactivity as churn;
- using gross bookings as contribution;
- using unverified market estimates as customer evidence.

---

## 15. Monthly Commercial Review

Required agenda:

1. acquisition;
2. activation;
3. paid conversion;
4. retention;
5. contribution;
6. channel economics;
7. support;
8. trust;
9. expansion;
10. open regulatory/compliance risks.

---

## 16. Evidence Register

Maintain:

- source;
- publication date;
- measurement period;
- population;
- metric definition;
- limitation.

External statistics must never lose their denominator or period.

---

## 17. Decision Log

Every material commercial decision records:

- problem;
- evidence;
- assumptions;
- alternatives considered;
- decision;
- affected segment;
- economic effect;
- implementation dependency;
- review date.

---

## 18. Final Contract

```MEASURE
→ SEGMENT
→ VERIFY
→ DECIDE
→ RECORD
→ RE-TEST
```

Commercial governance is an evidence system, not a dashboard decoration.