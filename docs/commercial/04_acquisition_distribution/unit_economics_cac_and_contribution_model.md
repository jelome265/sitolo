# Sitolo — Unit Economics, CAC & Contribution Model

**Status:** Commercial economics control document  
**Date:** 2026-09-22  
**Scope:** segment, channel, product and support economics

---

## 1. Purpose

Headline price is not economics.

For each customer:

```REVENUE
− VARIABLE COST
=
CONTRIBUTION
```

CAC is then evaluated against contribution.

---

## 2. Revenue

Include:

- subscription revenue;
- implementation fees;
- migration fees;
- paid modules;
- premium support;
- integration fees where applicable.

Do not use unearned future fintech revenue in base-case SaaS economics.

---

## 3. Variable Cost

Track:

- payment processing;
- infrastructure;
- storage;
- bandwidth;
- support;
- onboarding;
- partner commissions;
- refunds;
- failed-payment recovery;
- incremental compliance/integration cost.

---

## 4. Contribution

```
CONTRIBUTION
=
NET REVENUE
−
VARIABLE COST
```

Use a consistent accounting definition.

---

## 5. CAC

```CAC
=
ALLOCATED ACQUISITION SPEND
/
NEW QUALIFIED CUSTOMERS
```

The numerator includes appropriate sales, marketing, channel, travel and onboarding acquisition costs.

Do not include long-term engineering cost as CAC unless the accounting policy explicitly does so.

---

## 6. Channel CAC

Calculate separately:

- direct digital;
- referrals;
- accountants;
- distributors;
- agents;
- resellers;
- enterprise sales.

---

## 7. CAC Payback

```
CAC PAYBACK (MONTHS)
=
CAC
/
MONTHLY CONTRIBUTION
```

Use contribution, not gross subscription revenue.

---

## 8. Duka Gate

For MK7,500/month hypothesis:

```
LOW ARPU
+
PAYMENT COST
+
SUPPORT
+
ACQUISITION
=
HIGH ECONOMIC SENSITIVITY
```

The Duka segment must demonstrate viable contribution before scale.

---

## 9. SME/Growth Economics

Higher ARPU can support:

- richer support;
- assisted onboarding;
- partner acquisition;
- desktop workflows.

But higher feature depth can also raise variable infrastructure and support cost.

---

## 10. Multi-Branch Economics

Model:

```
BASE ACCOUNT
+
BRANCH COUNT
+
USERS
+
SUPPORT
+
INTEGRATIONS
+
IMPLEMENTATION
```

Pricing should avoid large subsidy for unusually complex accounts.

---

## 11. Enterprise Economics

Separate:

- annual recurring revenue;
- implementation revenue;
- implementation cost;
- support cost;
- integration cost;
- security/procurement cost;
- account management.

An apparently large enterprise contract can be unattractive if delivery cost consumes the economics.

---

## 12. Scenario Planning

Use at least:

- Duka-heavy;
- balanced;
- higher-complexity.

For each calculate:

- MRR;
- contribution;
- support load;
- channel cost;
- CAC payback.

These are planning scenarios, not forecasts.

---

## 13. Gross Margin

At platform level:

```
GROSS MARGIN
=
(NET REVENUE − COST OF SERVICE)
/
NET REVENUE
```

Define exactly which costs belong in cost of service.

---

## 14. Cohort Economics

Measure by:

- signup month;
- channel;
- tier;
- business type;
- geography;
- onboarding path;
- payment method.

Do not blend profitable and unprofitable cohorts.

---

## 15. LTV

A simple planning model:

```
LTV ≈
AVERAGE MONTHLY CONTRIBUTION
×
EXPECTED CONTRIBUTIVE LIFETIME
```

Do not claim statistically robust LTV from tiny samples.

---

## 16. Economic Danger Zones

Examples:

- ARPU falls while support rises;
- partner commission exceeds contribution;
- payment fees consume meaningful revenue;
- enterprise implementation is underpriced;
- rural travel cost dominates Duka acquisition;
- refunds or delinquency are material;
- infrastructure cost scales faster than revenue.

---

## 17. Scale Gate

Do not increase acquisition spend simply because CAC is temporarily low.

Confirm:

- retention;
- contribution;
- payback;
- support capacity;
- channel stability.

---

## 18. Final Contract

```
PRICE
→ REVENUE
→ VARIABLE COST
→ CONTRIBUTION
→ CAC
→ PAYBACK
→ SCALE
```

Account growth without contribution is not sustainable commercial validation.