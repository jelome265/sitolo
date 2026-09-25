# Sitolo — Commercial Model Precision & Consistency Contract

**Status:** Canonical commercial consistency addendum
**Date:** 2026-09-22
**Scope:** Commercial documentation corpus in PR #43
**Purpose:** eliminate ambiguity between commercial strategy, validation, pricing hypotheses, unit economics, tiers, devices, channels, and PMF decisions.

---

# 1. Why This Contract Exists

PR #43 introduces a large commercial documentation corpus. The documents are directionally consistent, but precision requires an explicit authority model.

The most important correction is this:

> **A commercial strategy document may describe a proposed package; only the validation system and subsequent approved decision record can establish that the package is commercially validated.**

The five-tier architecture is a product/packaging hypothesis. Prices are planning/test values until validated. Experiment thresholds are internal gates, not market facts.

This document therefore resolves:

- document authority;
- price semantics;
- tier semantics;
- device semantics;
- experiment ownership;
- CAC ownership;
- PMF terminology;
- forecast vs scenario separation;
- evidence requirements;
- decision promotion;
- contradiction handling.

---

# 2. Source-of-Truth Hierarchy

Commercial documents MUST be interpreted in this order:

```text
LAW / REGULATION / REGULATOR
        ↓
SIGNED PROVIDER / PARTNER CONTRACT
        ↓
APPROVED BUSINESS DECISION RECORD
        ↓
COMMERCIAL VALIDATION PLAN
        ↓
BUSINESS MODEL DESIGN
        ↓
COMMERCIAL OPERATING MODEL
        ↓
SPECIALIZED COMMERCIAL DOCUMENTS
        ↓
MARKETING / PRESENTATION MATERIAL
```

Engineering/security/domain contracts remain authoritative for technical and security properties.

A lower document MUST NOT silently override a higher document.

---

# 3. Document Responsibility Matrix

| Document | Owns | Must not own |
|---|---|---|
| `../01_strategy/business_model_design.md` | business model, category, strategic scope | observed validation results |
| `commercial_validation_plan.md` | experiments, thresholds, evidence and validation decisions | production implementation details |
| `commercial_operating_model.md` | end-to-end commercial operating model | claiming hypotheses are validated |
| `unit_economics_cac_and_contribution_model.md` | economic definitions and measurement methodology | final market pricing by itself |
| `upgrade_expansion_and_tier_migration_model.md` | tier transitions and complexity triggers | unsupported fixed thresholds |
| `commercial_metrics_governance_and_decision_system.md` | metric definitions, cohorts and decision governance | redefining experiment hypotheses |
| specialized commercial docs | their named commercial boundary | changing the global tier model without an approved decision |
| the repository README, `../../README.md`, or `../README.md` | navigation and concise public summary | becoming a second commercial source of truth |

---

# 4. Tier Model — Exact Semantic Definition

Sitolo remains one product serving:

```text
DUKA → SME → GROWTH → MULTI-BRANCH → ENTERPRISE
```

These are **commercial packaging tiers**, not five separate products.

## 4.1 Duka

Definition:

- normally one operating location;
- very small operational team;
- mobile-first operation;
- low catalogue complexity;
- high price sensitivity;
- high requirement for low-cost onboarding and support.

Default device:

```text
MOBILE
```

Primary job:

```text
SELL + STOCK + MONEY + TODAY
```

## 4.2 SME

Definition:

- larger operational workload than Duka;
- multiple users may exist;
- management needs exceed frontline mobile workflows;
- purchasing and reporting become materially important.

Default devices:

```text
MOBILE + PC
```

## 4.3 Growth

Definition:

- sustained operational complexity;
- deeper inventory/procurement/reconciliation requirements;
- greater management/control requirements.

Default devices:

```text
MOBILE + PC
```

## 4.4 Multi-Branch

Definition:

> One organization operating two or more physical locations where centralized control is materially useful.

A large single-location store is not automatically Multi-Branch.

Default topology:

```text
                    HQ
                     |
                    PC
                     |
          +----------+----------+
          |          |          |
       BRANCH A   BRANCH B   BRANCH C
        MOBILE     MOBILE     MOBILE
```

## 4.5 Enterprise

Definition:

A customer with materially higher requirements for governance, authorization, audit, integration, procurement, security review, SLA, or organizational complexity.

Enterprise is **not defined by PC ownership alone**.

Default control topology:

```text
ENTERPRISE CONTROL
      PC / WEB
          |
   GOVERNANCE / ADMIN
          |
    MOBILE OPERATIONS
          |
       BRANCHES
```

---

# 5. Device Semantics

Device is a workflow property, not the commercial segmentation key.

```text
MOBILE = FRONTLINE OPERATION
PC     = MANAGEMENT / CONTROL
WEB    = ADMINISTRATION / SUPPORT / SELECTED CONTROL
```

Therefore:

- Duka being mobile-only is a default, not a prohibition on future optional devices;
- SME/Growth receiving PC capability does not mean every user needs a PC;
- Multi-Branch normally uses mobile devices at branches and PC/Web at central control;
- Enterprise may use mobile, PC and Web according to job function.

Pricing MUST NOT be primarily based on device count unless an experiment demonstrates that device-based pricing aligns with customer value and contribution.

---

# 6. Price Semantics

The following values may appear in planning documents:

| Tier | Planning value | Semantic status |
|---|---:|---|
| Duka | MK7,500/month | pricing hypothesis / planning value |
| SME | MK25,000/month | pricing hypothesis / planning value |
| Growth | MK50,000/month | pricing hypothesis / planning value |
| Multi-Branch | from MK90,000/month | pricing hypothesis / planning value |
| Enterprise | from MK200,000/month | pricing hypothesis / planning value |

These values MUST NOT be described as:

- validated willingness to pay;
- guaranteed market-clearing prices;
- forecasts;
- market averages;
- competitor-equivalent prices;
- evidence of PMF.

They may be used for:

- scenario planning;
- unit-economics modelling;
- controlled pricing experiments;
- internal planning.

A price becomes an approved commercial price only after an explicit decision record promotes it from hypothesis to approved price.

---

# 7. Pricing Authority

`commercial_validation_plan.md` owns pricing experiments.

The operating model may reference the current pricing hypothesis but MUST NOT independently change it.

Required lifecycle:

```text
PRICE HYPOTHESIS
      ↓
EXPERIMENT
      ↓
OBSERVED PAYMENT
      ↓
RETENTION
      ↓
CONTRIBUTION
      ↓
DECISION RECORD
      ↓
APPROVED PRICE
```

---

# 8. Existing 12-Experiment Program

The existing 12 experiments are already the authoritative validation program. PR #43 MUST NOT create a competing second experiment program.

The canonical IDs are:

| ID | Experiment |
|---|---|
| CV-01 | Free vs Trial |
| CV-02 | Core Price Sensitivity |
| CV-03 | Reconciliation WTP |
| CV-04 | Accountant Referrals |
| CV-05 | Distributor Acquisition |
| CV-06 | Business-Type Onboarding |
| CV-07 | Mobile-Only Retention |
| CV-08 | Mobile + Desktop Value |
| CV-09 | Branch Expansion |
| CV-10 | EIS Purchase-Driver Effect |
| CV-11 | Premium Support |
| CV-12 | Enterprise Pilot Economics |

The existing validation plan remains responsible for the detailed hypothesis, population, experiment design, sample target, thresholds, measurement window and decision procedure.

Specialized commercial documents explain **what mechanism is being tested**; they do not redefine the experiment.

---

# 9. Experiment-to-Commercial-Corpus Crosswalk

| Experiment | Commercial mechanism | Primary evidence | Economic gate |
|---|---|---|---|
| CV-01 | free vs trial | activation + paid conversion | contribution/account |
| CV-02 | core price | paid conversion by price cell | contribution + payback |
| CV-03 | reconciliation | adoption + willingness to pay | incremental contribution |
| CV-04 | accountant channel | qualified → paid → retained | channel CAC |
| CV-05 | distributor channel | qualified → paid → retained | channel CAC + contribution |
| CV-06 | business-type onboarding | activation + time-to-value | onboarding cost vs benefit |
| CV-07 | mobile-only | mobile workflow completion + retention | support/device cost |
| CV-08 | mobile + desktop | incremental workflow/value | incremental retention/expansion |
| CV-09 | branch expansion | second branch + paid expansion | expansion contribution |
| CV-10 | EIS capability | conversion difference where relevant | incremental commercial value |
| CV-11 | premium support | paid service adoption | service contribution |
| CV-12 | enterprise | pilot → contract → renewal | implementation-adjusted contribution |

---

# 10. Sample-Size Semantics

Sample sizes in the validation plan are **operational targets**.

They must not be described as universal statistical standards.

Interpretation:

```text
<30 observations/cell
→ signal only

30–59/cell
→ directional evidence; replication preferred

60+/cell
→ stronger evidence, still subject to economics and segmentation
```

A sample-size target is not a guarantee of statistical significance.

---

# 11. PMF Terminology

Do not use a binary:

```text
PMF = YES / NO
```

Use staged language:

```text
UNVALIDATED
→ SIGNAL
→ DIRECTIONAL
→ PMF CANDIDATE
→ REPEATABLE PMF EVIDENCE
```

A segment cannot become `PMF CANDIDATE` from conversion alone.

Required evidence includes:

- repeated meaningful usage;
- paid behaviour;
- sustained retention;
- viable support/infrastructure economics;
- non-founder-only acquisition;
- evidence surviving segmentation.

---

# 12. Scenario vs Forecast

Any account-mix model such as:

```text
600 Duka
220 SME
100 Growth
60 Multi-Branch
20 Enterprise
```

and resulting MRR calculations are **scenario analysis**.

They are not:

- forecasts;
- targets committed to investors;
- expected market share;
- probability-weighted outcomes.

A forecast requires an explicit forecasting methodology, historical data, assumptions and confidence/uncertainty treatment.

---

# 13. CAC Precision

CAC must always declare:

```text
TIME WINDOW
CHANNEL
SEGMENT
COHORT
NUMERATOR COSTS
DENOMINATOR CUSTOMER EVENT
```

The default denominator for commercial CAC is:

```text
NEW PAYING CUSTOMERS
```

Do not silently substitute:

- leads;
- downloads;
- registrations;
- qualified leads.

Pre-conversion sales/onboarding costs are included only where the documented accounting policy says they are acquisition costs.

---

# 14. Contribution Precision

For each cohort:

```text
NET REVENUE
− PAYMENT / BILLING COST
− DIRECT INFRASTRUCTURE
− VARIABLE SUPPORT
− VARIABLE CHANNEL COMMISSION
− OTHER DIRECT VARIABLE DELIVERY COST
=
CONTRIBUTION
```

One-time implementation revenue and cost must be separately identified for enterprise economics.

Future speculative fintech revenue MUST NOT be used to rescue negative SaaS contribution.

---

# 15. Pricing Experiment Requirements

Every pricing test must record:

```text
EXPERIMENT ID
SEGMENT
PLAN
PRICE
CURRENCY
TAX TREATMENT
BILLING PERIOD
PAYMENT METHOD
EXPOSURE COUNT
PURCHASE COUNT
PAYMENT SUCCESS
REFUNDS
DAY-30 RETENTION
DAY-90 RETENTION
VARIABLE COST
CONTRIBUTION
CAC
CAC PAYBACK
```

If tax treatment or payment fees differ between cells, the economic comparison must normalize or explicitly account for the difference.

---

# 16. Enterprise Pricing Precision

Enterprise `from` pricing means:

```text
BASE COMMERCIAL PACKAGE
+
CUSTOM SCOPING
+
APPROVED IMPLEMENTATION / INTEGRATION FEES
```

It MUST NOT mean:

```text
UNLIMITED CUSTOM WORK FOR A FIXED SUBSCRIPTION
```

Every enterprise deal should separately record:

- recurring subscription;
- implementation;
- migration;
- integration;
- premium support/SLA;
- custom development, if approved;
- direct delivery cost;
- expected renewal value.

---

# 17. Duka Economics Precision

Duka remains explicitly in commercial scope.

The low price hypothesis requires a low-cost service model.

```text
LOW ARPU
+
LOW CAC
+
LOW SUPPORT COST
+
LOW ONBOARDING COST
+
LOW PAYMENT COST
+
HIGH RETENTION
```

The Duka segment must be evaluated independently rather than blended into SME economics.

A large Duka account count does not prove viability unless contribution is positive and repeatable.

---

# 18. Upgrade Precision

An upgrade recommendation requires:

```text
OBSERVED COMPLEXITY
→ VALIDATED TRIGGER
→ RELEVANT CAPABILITY
→ EXPLICIT VALUE EXPLANATION
→ OPTIONAL UPGRADE
```

Do not classify an account as an upgrade opportunity solely because:

- it has been a customer for a long time;
- it uses many devices;
- it has high login frequency.

The trigger must map to a real business complexity or control requirement.

---

# 19. Seasonal Precision

The following are distinct:

```text
SEASONAL_INACTIVE
BUSINESS_PAUSED
PAST_DUE
SUSPENDED
CHURNED
BUSINESS_CLOSED
```

Monthly inactivity MUST NOT automatically equal churn.

Retention reports must state whether seasonal customers are included, excluded, or separately modelled.

---

# 20. Channel Precision

Every paying customer must have an attributable acquisition channel where possible.

Minimum channel taxonomy:

```text
DIRECT_DIGITAL
REFERRAL
ACCOUNTANT
DISTRIBUTOR
AGENT
RESELLER
DIRECT_ENTERPRISE
PARTNER
OTHER
UNKNOWN
```

`UNKNOWN` is a data-quality state, not a successful attribution.

---

# 21. Commercial Decision Promotion

A hypothesis may be promoted only when the evidence package contains:

```text
EXPERIMENT ID
→ PRE-REGISTERED HYPOTHESIS
→ DEFINED POPULATION
→ ACTUAL SAMPLE
→ ACTUAL PRICE / TREATMENT
→ PRIMARY METRIC
→ PASS / FAIL / CONDITIONAL RESULT
→ CAC / CONTRIBUTION WHERE APPLICABLE
→ RETENTION WINDOW
→ SEGMENT ANALYSIS
→ DECISION
→ DECISION DATE
→ OWNER
```

The decision must identify whether the result is:

```text
SCALE
MODIFY
REPEAT
HOLD
KILL
```

No commercial document may silently convert `HYPOTHESIS` into `VALIDATED`.

---

# 22. Contradiction Resolution

When two commercial documents disagree:

1. identify the conflicting statements;
2. classify each as fact, hypothesis, scenario, recommendation, or validated decision;
3. apply the authority hierarchy;
4. preserve the higher-authority statement;
5. create a decision record if the conflict requires a new commercial choice;
6. update dependent documents only after the decision exists.

Never resolve a contradiction by silently editing one document until the contradiction disappears.

---

# 23. Documentation Acceptance Gate

PR #43's commercial corpus is internally consistent only when:

```text
[ ] one universal product scope
[ ] five-tier packaging semantics are consistent
[ ] Duka remains explicitly included
[ ] Multi-Branch means multiple physical operating locations
[ ] Enterprise is governance/complexity driven, not merely device driven
[ ] mobile = frontline operation
[ ] PC/Web = control/administration where justified
[ ] prices are labelled as hypotheses/planning values until validated
[ ] the 12 existing experiments remain the only canonical validation program
[ ] specialized docs map to existing experiment IDs where applicable
[ ] CAC denominator is explicit
[ ] contribution definition is explicit
[ ] scenarios are not presented as forecasts
[ ] PMF is evidence-gated
[ ] seasonal inactivity is not silently treated as churn
[ ] enterprise implementation economics are separated from recurring revenue
[ ] pricing tests record payment/tax economics where material
[ ] contradictions have an explicit resolution path
```

---

# 24. Final Commercial Contract

Sitolo's commercial model is:

```text
ONE PRODUCT
+
BROAD MARKET SCOPE
+
SEGMENTED PACKAGING
+
SEGMENTED GTM
+
SEGMENTED ECONOMICS
+
MOBILE-FIRST FRONTLINE OPERATION
+
PC / WEB CONTROL WHEN COMPLEXITY JUSTIFIES IT
+
EVIDENCE-GATED PRICING
+
EVIDENCE-GATED PMF
+
POSITIVE CONTRIBUTION AS THE ECONOMIC FLOOR
=
COMMERCIAL SYSTEM
```

The purpose of this contract is not to add another strategy layer.

It is to prevent the existing commercial corpus from accidentally turning:

```text
HYPOTHESIS → FACT
SCENARIO → FORECAST
SIGNUP → CUSTOMER
CUSTOMER → PMF
REVENUE → CONTRIBUTION
DEVICE → SEGMENT
INACTIVITY → CHURN
```

That distinction is mandatory for future commercial decisions.