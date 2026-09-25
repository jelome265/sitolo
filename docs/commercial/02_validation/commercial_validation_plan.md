# Sitolo — Commercial Validation Plan

**Status:** Working commercial-validation contract  
**Prepared:** 2026-09-22  
**Market:** Malawi first; regional expansion only after evidence-based validation  
**Parent strategy:** [Business Model Design](../01_strategy/business_model_design.md)

---


# 0. Executive Decision

Sitolo has broad product-market scope with segmented commercial execution.

The long-term category is:

> **Business Operating System for African SMEs**

The addressable product scope is:

~~~text
DUKA
  ↓
MICRO / SMALL RETAIL
  ↓
GROWING SME
  ↓
SPECIALIST SME
  ↓
MULTI-BRANCH
  ↓
ENTERPRISE
~~~

The validation program must **not** turn one experimental cohort into a permanent product restriction.

The correct distinction is:

| Dimension | Commercial rule |
|---|---|
| Product scope | Duka through enterprise |
| Validation cohort | Narrow enough to measure and falsify a hypothesis |
| GTM motion | Different by segment |
| Packaging | Different by operational complexity |
| Support | Different by cost-to-serve |
| Pricing | Experimental and segment-aware |
| Product architecture | One shared platform |
| Security/reliability | Universal baseline |

The previous inventory-heavy small-and-growing-retail focus remains a valid current learning cohort for experiments where that population is the correct test population. It is not the permanent Sitolo ICP and must not be documented as a product exclusion.

The validation program exists to determine, separately and honestly, whether each commercially material segment can produce:

~~~text
REAL CUSTOMER PAIN
        ↓
REAL OPERATIONAL USE
        ↓
REAL PAYMENT
        ↓
REAL RETENTION
        ↓
POSITIVE CONTRIBUTION
        ↓
REPEATABLE ACQUISITION
        ↓
EXPANSION WHERE APPLICABLE
~~~

A merchant account is commercially meaningful when it produces real business activity and repeatedly demonstrates value. Registrations, downloads, free accounts, demos, page views and optimistic interviews are not sufficient.

This plan therefore measures:

- operational pain;
- activation;
- retention;
- payment;
- reconciliation value;
- pricing;
- CAC;
- support cost;
- contribution;
- expansion;
- segment-specific PMF.

The thresholds in this plan remain **internal validation gates and operating hypotheses, not industry benchmarks, forecasts, or universal standards**.

# 1. Authority and Relationship to the Business Model

This document operationalizes the commercial assumptions already recorded in business_model_design.md.

The relationship is:

~~~text
business_model_design.md
        ↓
commercial hypotheses
        ↓
commercial_validation_plan.md
        ↓
experiments
        ↓
observed evidence
        ↓
decision log
        ↓
pricing / packaging / channel / roadmap updates
~~~

This document must not silently redefine the product category, security architecture, tenant boundaries, financial authority, regulatory obligations, or domain invariants.

It may refine:

- ICP selection;
- pricing hypotheses;
- packaging;
- channel priorities;
- onboarding;
- trial design;
- support model.

Commercial assumptions remain hypotheses until they are supported by observed customer evidence.

No future financing, partner-finance, data-product, or transaction-fee revenue is permitted to substitute for weak SaaS economics.

---

# 2. Why Validation Is Required

Sitolo operates in a market where several local products already advertise combinations of POS, inventory, offline operation, mobile money, multi-branch capability, business management, MRA EIS integration, and local-currency pricing.

Therefore the commercial question is not:

> Can Sitolo implement these capabilities?

It is:

> Will a defined customer segment choose Sitolo, pay for it, keep using it, and expand usage because Sitolo creates measurable value that is difficult to replace?

MRA currently provides its own computer, mobile, and web EIS POS solutions free to taxpayers, while third-party POS systems require certification. EIS capability can create eligibility and convenience value, but EIS alone must not be treated as the core economic moat.

Current public competitor pricing is a market signal rather than a Sitolo pricing prescription. Phindu currently advertises a 14-day trial and a general-retail starting price of MWK 65,000/month. MalondaPlus currently advertises public monthly tiers of MWK 25,000, MWK 45,000, MWK 90,000, and MWK 130,000. GulaSync currently advertises MWK 20,000/month Starter, MWK 35,000/month Business, and MWK 75,000/month Enterprise. These are public vendor prices and must not be presented as independent evidence of market share, willingness to pay, or superior product value.

Sitolo's validation program therefore concentrates on:

1. operational pain;
2. activation;
3. retention;
4. payment;
5. reconciliation value;
6. pricing;
7. acquisition economics;
8. expansion;
9. support economics.

---


# 3. Current Validation Cohorts and Universal Market Scope

## 3.1 Product scope

Sitolo remains commercially designed to serve:

- duka / ultra-micro retail;
- micro and small retail;
- growing SMEs;
- pharmacies;
- agro-dealers;
- wholesalers;
- multi-branch operators;
- enterprise organizations.

Product breadth is intentionally separated from experiment breadth.

## 3.2 Current primary validation cohort

For experiments that require a concentrated cohort, the current primary learning cohort is:

~~~text
Business type:
  General retail / inventory-heavy SME

Typical scale:
  1–5 locations
  2–30 operational users/devices

Operational characteristics:
  meaningful daily sales
  material stock
  purchasing activity
  cash and/or mobile-money payments
  owner/manager involvement

Current systems:
  paper
  spreadsheets
  basic POS
  disconnected payment records
  mixed/manual workflows

Pain:
  stock uncertainty
  cash variance
  payment mismatch
  staff-control gaps
  difficult reporting
  reconciliation effort
~~~

This is a **current validation cohort**, not a permanent product boundary.

## 3.3 Duka validation cohort

Dukas are explicitly in commercial scope.

A Duka cohort should be tested separately where price, support, onboarding, mobile-only operation, or daily value are materially different.

Candidate Duka characteristics:

- one location;
- one or very few users;
- one primary device;
- small catalogue;
- frequent small-value transactions;
- cash and/or mobile money;
- manual or memory-driven records;
- high price sensitivity.

Candidate Duka questions:

- Will the merchant pay a low recurring price?
- Does Sitolo actually replace or augment manual records?
- Does the daily business summary create a repeat habit?
- Does mobile-only operation work?
- Can acquisition be low-cost?
- Can support remain low-cost?
- Does the segment generate positive contribution?

## 3.4 Growing-SME validation cohort

Use a separate cohort when testing:

- multiple users;
- purchasing;
- reconciliation;
- desktop value;
- deeper reporting;
- branch expansion;
- accountant/distributor channels.

## 3.5 Enterprise validation cohort

Use a separately qualified cohort for:

- enterprise sales;
- implementation;
- procurement;
- integrations;
- security review;
- SLA;
- governance.

Enterprise evidence must not dominate blended commercial metrics simply because individual contracts are larger.

## 3.6 Specialist cohorts

Pharmacy, agro-dealer, wholesale and other specialist segments can be tested when the specialized workflow is sufficiently implemented to produce meaningful evidence.

## 3.7 Segment dimensions

Every experiment must preserve:

~~~text
segment
business_type
size_band
branch_count
user_count
device_count
price
plan
channel
onboarding_variant
workflow_depth
product_version
~~~

This allows Sitolo to answer segment-specific questions without contaminating evidence.


# 4. Definitions

## 4.1 Lead

A person or organization that has expressed identifiable commercial interest.

## 4.2 Qualified lead

A lead matching the ICP or an explicitly approved secondary cohort and meeting the minimum qualification criteria:

- real operating business;
- identifiable decision maker or strong buyer influence;
- sufficient transaction activity;
- current operational problem;
- plausible willingness and ability to pay.

## 4.3 Trial

A controlled evaluation period during which the customer can experience the defined product value loop.

## 4.4 Activated merchant

A merchant that completes the defined first-value event and demonstrates repeated meaningful operational usage.

The preferred activation event is:

~~~text
real sale
+
correct stock mutation
+
payment recorded
+
business result visible
~~~

A signup, tutorial completion, or app opening is not activation.

## 4.5 Retained merchant

A merchant that continues the expected critical workflow after the defined observation window.

Retention must be measured using actual activity, not login count alone.

## 4.6 Expansion

An increase in commercial scope such as additional branches, additional operational users/devices, additional paid capabilities, higher control/reconciliation package, or enterprise integration scope.

## 4.7 CAC

Customer Acquisition Cost.

For channel c:

~~~text
CAC_c =
(all attributable acquisition spend for channel c)
/
(new paying customers attributable to channel c)
~~~

Acquisition spend includes, where applicable:

- sales compensation;
- commission;
- paid media;
- field travel directly attributable to acquisition;
- partner commission;
- onboarding labour attributable to acquisition;
- demo/support labour explicitly incurred before conversion when material.

Common overhead must not be arbitrarily loaded into CAC without a consistent allocation policy.

## 4.8 Contribution

For merchant m:

~~~text
Contribution_m =
revenue
-
direct infrastructure cost
-
payment/billing cost
-
variable support cost
-
variable acquisition/sales commission
-
other directly attributable variable delivery cost
~~~

## 4.9 Payback

CAC payback is:

~~~text
CAC Payback =
CAC /
monthly contribution margin
~~~

Track both:

- gross-profit payback;
- contribution-margin payback.

Contribution-margin payback is the more conservative operational measure.

## 4.10 PMF evidence

PMF is not a single survey score.

Sitolo will use the condition **PMF Candidate** only when:

- a defined ICP has strong repeated usage;
- paid conversion is demonstrated;
- retention is sustained;
- support and infrastructure economics remain viable;
- acquisition works outside founder-only sales;
- the product's core value proposition is confirmed by observed behaviour;
- evidence survives cohort segmentation.

---

# 5. Experimental Governance

## 5.1 One primary hypothesis

Every experiment must have exactly one primary hypothesis.

Secondary metrics may explain the result but must not be retroactively promoted to replace the original primary outcome.

## 5.2 Pre-register the experiment

Before launch record:

~~~text
experiment ID
date
owner
segment
ICP definition
hypothesis
control
treatment
sample-size target
primary metric
secondary metrics
measurement window
pass threshold
conditional threshold
fail threshold
stop rules
known confounders
decision date
~~~

## 5.3 Do not move thresholds after seeing the data

Changing a pass threshold after observing results is a new experiment and must create a new decision record.

## 5.4 Segment the data

At minimum segment by:

- business type;
- acquisition channel;
- plan;
- price;
- branch count;
- business age where available;
- customer tenure;
- onboarding path.

## 5.5 No vanity success

The following cannot be used as sole evidence of commercial success:

- impressions;
- downloads;
- signups;
- app opens;
- demo attendance;
- free-account count;
- message volume;
- page views.

## 5.6 Missing data is a failure signal

A metric that cannot be measured reliably is itself a product or operations problem.

Do not silently exclude difficult observations.

---

# 6. Sample-Size Philosophy

The sample sizes below are deliberately practical for a Malawi-first early-stage validation program.

They are **minimum operational sample targets**, not claims of universal statistical power.

Early experiments should use directional evidence and confidence intervals where sample sizes allow them.

For binary conversion metrics:

~~~text
conversion rate =
successful outcomes /
eligible participants
~~~

For pricing experiments record:

- offer price;
- exposure;
- acceptance;
- payment;
- churn;
- support burden.

For retention:

- define the activity event before launch;
- use cohort dates;
- do not backfill definitions after observing outcomes.

Where an experiment has fewer than approximately 30 observations per cell, treat the result as signal rather than proof.

Where an experiment has 30–59 observations per cell, treat the result as directional evidence requiring replication.

Where an experiment reaches 60+ observations per cell, the result can carry materially more weight, but product decisions should still incorporate economics and qualitative evidence.

---

# 7. Common Commercial Measurement Windows

Use consistent observation windows:

~~~text
Day 0
lead / signup

Day 1–7
activation

Day 14
early habit

Day 30
initial retention

Day 45
trial-to-paid conversion window

Day 60
early commercial stability

Day 90
retention / PMF signal

Month 6
medium-term retention and expansion

Month 12
cohort economics
~~~

Enterprise pilots may use different windows because procurement cycles are structurally longer.

---

# 8. Experiment 01 — Free vs Trial

## Objective

Determine whether permanently free onboarding or a time-limited full/near-full trial produces superior commercial economics.

## Hypothesis

A controlled time-limited trial will produce better paid conversion and a healthier cost-to-convert ratio than a permanently free entry path.

## Population

Qualified merchants from a defined validation cohort who have not previously used Sitolo.

The default experiment lane may use the current general-retail validation cohort, but Duka accounts must be analysed separately whenever pricing, support, onboarding, device requirements, or value proposition materially differ.

The experiment must preserve:

~~~text
SEGMENT
BUSINESS TYPE
SIZE BAND
CHANNEL
DEVICE MIX
PRICE
PLAN
~~~

## Minimum sample

**120 qualified accounts**, randomized or alternated into:

~~~text
Control: 60
Free/limited access

Treatment: 60
14–30 day trial with clearly defined value loop
~~~

If randomization is operationally impossible, use matched cohorts by:

- segment;
- business size;
- acquisition channel;
- transaction volume.

## Primary metric

Paid conversion by Day 45.

## Secondary metrics

- Day-7 activation;
- Day-30 retained activity;
- support hours per account;
- time-to-first-value;
- trial completion;
- conversion by acquisition channel;
- revenue generated from converted cohort.

## Pass threshold

Treatment passes when all are true:

1. paid conversion is at least **1.5×** the control conversion rate or produces materially higher contribution per acquired account;
2. Day-7 activation is at least **10 percentage points** higher or treatment maintains equivalent activation with meaningfully lower support cost;
3. no material increase in abuse or operational burden;
4. contribution per account over the observation period is positive.

## Conditional

- conversion advantage exists but sample is insufficient;
- activation improves but conversion does not;
- conversion improves but support economics deteriorate.

Conditional result requires a second pricing/onboarding experiment.

## Fail

- no meaningful conversion advantage;
- materially worse activation;
- free users consume materially more support/resources without conversion;
- treatment creates a negative contribution profile.

## Stop rules

Stop early if:

- severe abuse is discovered;
- the free path creates uncontrolled infrastructure/support cost;
- a regulatory requirement makes one path inappropriate.

## Required output

Decision:

~~~text
KEEP FREE
KEEP TRIAL
HYBRID
ABANDON FREE
ABANDON TRIAL
~~~

---

# 9. Experiment 02 — Core Price Sensitivity

## Objective

Determine the price region at which the primary validation cohort still converts while producing acceptable contribution.

## Hypothesis

The primary validation cohort can support a price above the low-end adoption hypothesis when the product clearly includes operational control and reconciliation value.

## Test structure

The standard core-price experiment remains:

~~~text
P1 = MWK 35,000/month
P2 = MWK 55,000/month
P3 = MWK 75,000/month
~~~

These are test prices only, not approved final prices.

They test demand for the growing-SME/core operating package. They must **not** be reused as a universal price for Dukas, Multi-Branch accounts, or Enterprise.

### Duka price lane

Duka requires a distinct low-price experiment because the unit economics and hardware assumptions are materially different.

Initial Duka planning hypothesis:

~~~text
Duka reference price = MK7,500/month
~~~

This is not a final tariff.

The Duka experiment must test a narrow set of low-price offers around the reference point and must measure:

~~~text
PAID CONVERSION
+
DAY-30 / DAY-90 RETENTION
+
CAC
+
SUPPORT COST
+
INFRASTRUCTURE COST
+
CONTRIBUTION
~~~

A Duka price should never be selected solely because it maximizes conversion.

### Tier architecture reference

The current planning tier structure is:

| Tier | Indicative price | Main device model |
|---|---:|---|
| Duka | MK7,500/month | Mobile only |
| SME | MK25,000/month | Mobile + PC |
| Growth | MK50,000/month | Mobile + PC |
| Multi-Branch | From MK90,000/month | Mobile branches + PC HQ |
| Enterprise | From MK200,000/month | Mobile operations + PC/Web control |

All five values are planning hypotheses. The commercial program must validate them individually or through appropriately designed price/packaging experiments.

## Minimum sample

**90 qualified prospects**, approximately:

~~~text
30 per price cell
~~~

A larger replication target is **150–180** total prospects after the first directional experiment.

## Primary metric

Paid conversion from qualified offer exposure.

## Secondary metrics

- gross revenue per lead;
- contribution per acquired customer;
- support burden;
- requested discounts;
- annual-payment acceptance;
- time from offer to payment;
- churn;
- feature objections;
- competitor mentions.

## Required qualitative capture

Every lost or rejected offer should be classified:

~~~text
TOO EXPENSIVE
NOT VALUABLE ENOUGH
WRONG PRODUCT
WRONG TIMING
NEEDS FEATURE
USES COMPETITOR
NO BUDGET
OTHER
~~~

## Pass threshold

A price cell passes only if:

1. conversion remains at least **15%** among qualified, genuinely exposed prospects;
2. contribution margin remains positive within the measured cohort;
3. no evidence shows that the price causes unacceptable early churn;
4. customer feedback indicates the value proposition is understood.

A stronger price cell must not be selected solely because it maximizes headline revenue.

## Conditional

- 10–14.9% conversion with positive contribution;
- 15%+ conversion but high discount dependency;
- good conversion with poor retention;
- strong retention but insufficient margin.

## Fail

- below 10% conversion with no strong qualitative signal;
- contribution remains negative;
- price triggers immediate customer rejection across the relevant ICP;
- customers consistently state a different value expectation.

## Follow-up

After the first three-cell test, run one narrower two-cell experiment around the observed demand boundary. Do not run endless micro-tests.

---

# 10. Experiment 03 — Reconciliation Willingness-to-Pay

## Objective

Determine whether reconciliation is perceived as economically valuable enough to influence purchase or expansion.

## Hypothesis

Merchants with payment complexity will pay more for reliable automated reconciliation and exception handling than for transaction recording alone.

## Population

Merchants using multiple payment channels, mobile money + cash, high transaction volume, multiple staff, multiple branches, or other workflows that create reconciliation pain.

## Minimum sample

**90 qualified merchants**, split into three commercial presentations:

~~~text
A — reconciliation included
B — reconciliation add-on priced at MWK 10,000–20,000
C — reconciliation not offered initially
~~~

The exact add-on price is an experiment variable, not a permanent price.

## Primary metric

Paid selection/attachment of reconciliation capability or upgrade to a package containing it.

## Secondary metrics

- number/value of exceptions detected;
- time saved;
- willingness-to-pay stated before price exposure;
- willingness-to-pay after observed value;
- retention;
- expansion;
- reconciliation frequency.

## Required product evidence

The experiment should show real examples such as:

~~~text
SALE
50,000
   ↓
EXPECTED PAYMENT
50,000
   ↓
OBSERVED
45,000
   ↓
EXCEPTION
5,000
~~~

The experiment is weak if customers never experience the underlying value.

## Pass threshold

Pass if:

- at least **25%** of qualified exposed merchants pay for or upgrade because of reconciliation; or
- reconciliation-enabled customers show a meaningful retention/expansion improvement of at least **10 percentage points** over comparable non-enabled accounts.

Track these two forms of value separately.

## Conditional

- high interest but low immediate payment;
- strong value evidence in one payment profile only;
- customers prefer it included rather than as an add-on.

## Fail

- customers do not identify reconciliation as meaningful;
- exceptions exist but merchants do not care;
- feature increases complexity without improving retention or willingness to pay.

---

# 11. Experiment 04 — Accountant Referral Channel

## Objective

Determine whether accountants can become a repeatable low-CAC acquisition channel.

## Hypothesis

Accountants can produce higher-quality and more-retained customers than general direct acquisition because they already hold trusted merchant relationships.

## Pilot structure

Recruit **10 accountants**.

Target:

~~~text
5 referred qualified prospects per accountant
=
50 referred prospects
~~~

Build a matched direct-acquisition cohort of approximately **50** qualified prospects.

## Primary metric

90-day retained paying customer rate.

## Secondary metrics

- activation;
- paid conversion;
- CAC;
- CAC payback;
- support burden;
- referral volume per accountant;
- merchant quality;
- annual billing adoption.

## Partner economics

Record:

~~~text
accountant commission
sales effort
onboarding effort
support effort
revenue
retention
expansion
~~~

## Pass threshold

Accountant channel passes when:

1. it produces at least **70% 90-day retained-paying rate** among converted customers;
2. channel CAC is at most **75% of founder/direct blended CAC** or produces materially superior contribution despite higher CAC;
3. at least **3 of 10 accountants** produce qualified conversions;
4. no accountant relationship creates unacceptable compliance or operational dependency.

## Conditional

- strong retention but low referral volume;
- strong volume but weak retention;
- high CAC but excellent expansion;
- only one or two accountants perform.

## Fail

- no repeatable partner productivity;
- partner CAC remains uneconomic;
- referred merchants churn materially faster;
- channel requires excessive manual servicing.

---

# 12. Experiment 05 — Distributor Acquisition

## Objective

Determine whether wholesalers/distributors can acquire retail merchants economically.

## Hypothesis

A distributor that already supplies target retailers can lower acquisition friction and create high-quality merchant referrals.

## Pilot

Engage **3 distributors**.

For each:

~~~text
target 15 qualified merchant introductions
minimum 45 prospects
~~~

Use a defined referral or reseller structure.

## Primary metric

90-day contribution per referred merchant.

## Secondary metrics

- lead-to-activation;
- activation-to-paid;
- CAC;
- partner commission;
- time to conversion;
- merchant retention;
- product adoption depth;
- branch expansion.

## Pass threshold

Pass when:

- at least **2 of 3 distributors** generate qualified paying customers;
- referred-customer CAC is no greater than direct-channel CAC;
- 90-day retention is not materially worse than direct acquisition;
- distributor economics remain positive after commissions.

## Conditional

- one partner clearly works and two do not;
- high conversion but low scale;
- strong distribution but excessive commission.

## Fail

- merchants treat Sitolo as a bundled commodity with no engagement;
- partner economics are negative;
- the distributor relationship creates channel conflict.

---

# 13. Experiment 06 — Business-Type Onboarding

## Objective

Determine whether business-type personalization improves activation without increasing operational cost.

## Hypothesis

Showing business-specific defaults, terminology, reports, and workflows improves activation and time-to-value.

## Test

**120 new qualified accounts**:

~~~text
60 — generic onboarding
60 — business-type personalized onboarding
~~~

Business types remain configuration-driven.

## Primary metric

Day-7 activation rate.

## Secondary metrics

- time-to-first-value;
- Day-30 retention;
- onboarding completion;
- support requests;
- setup time;
- configuration abandonment.

## Pass threshold

Personalized onboarding passes if:

- Day-7 activation improves by at least **10 percentage points**;
- time-to-first-value improves by at least **20%**;
- support cost does not increase by more than **15%**.

## Conditional

- activation improves but support increases;
- one vertical benefits strongly while others do not;
- personalization only improves time-to-value.

## Fail

- no meaningful activation improvement;
- onboarding becomes longer;
- users become confused by irrelevant vertical defaults.

---

# 14. Experiment 07 — Mobile-Only Retention

## Objective

Measure whether a mobile-first product can support sustained operational usage without requiring desktop.

## Hypothesis

The primary day-to-day merchant workflow can achieve healthy retention through mobile alone.

## Population

Mobile-first primary-ICP merchants.

## Minimum sample

**60 activated merchants** followed for at least 90 days.

## Primary metric

90-day retained active merchant rate based on real business activity.

## Secondary metrics

- transaction frequency;
- stock workflow use;
- payment workflow use;
- reporting use;
- support burden;
- offline usage;
- device failure frequency.

## Pass threshold

Pass if:

- at least **70%** of activated merchants remain operationally active at Day 90;
- critical workflows can be performed without requiring desktop;
- support cost remains within contribution assumptions.

## Conditional

- mobile retention is strong for micro/small merchants but weak for larger operations;
- users retain mobile but request desktop for management.

## Fail

- mobile-only usage cannot support critical workflows;
- merchants repeatedly abandon tasks because back-office functions are inaccessible.

---

# 15. Experiment 08 — Mobile + Desktop Incremental Value

## Objective

Determine whether desktop materially increases retention, expansion, or willingness to pay among growing businesses.

## Hypothesis

Desktop produces incremental control value once transaction volume, catalogue size, staff count, or branch complexity passes a threshold.

## Population

Growing merchants with at least one complexity trigger.

## Minimum sample

**120 accounts**:

~~~text
60 — mobile-first with desktop available after complexity trigger
60 — mobile-first + desktop offered immediately
~~~

The test must not force a customer to remain on an obviously inadequate setup; safety and business continuity override experiment purity.

## Primary metric

90-day retained active rate plus commercial expansion.

## Secondary metrics

- desktop weekly active use;
- bulk catalogue usage;
- reconciliation usage;
- reporting use;
- multi-user management;
- plan upgrade;
- branch creation;
- support cost.

## Pass threshold

Desktop incremental value is confirmed when:

- retention improves by at least **10 percentage points**, or
- expansion/upgrade rate improves by at least **15 percentage points**,

while contribution remains positive.

## Conditional

- desktop is used heavily but does not yet affect retention;
- desktop value appears only after a business complexity threshold.

## Fail

- desktop adds little measurable value;
- desktop significantly increases support and implementation cost without corresponding retention or expansion.

---

# 16. Experiment 09 — Branch Expansion Demand

## Objective

Determine whether Sitolo can monetize genuine business growth through additional branches.

## Hypothesis

Existing customers will pay for multi-branch capability when branch growth creates measurable management pain.

## Population

Existing single-branch customers showing expansion signals:

- new location planned;
- second physical site;
- staff separation;
- cross-location stock;
- consolidated reporting request.

## Minimum sample

**40 eligible existing customers**.

## Primary metric

Branch expansion conversion within 60 days of qualified trigger.

## Secondary metrics

- expansion MRR;
- time to deployment;
- branch adoption;
- consolidated reporting usage;
- retention after expansion;
- support cost.

## Pass threshold

Pass if:

- at least **25%** of qualified single-branch accounts with a real expansion trigger purchase or add a second branch;
- expansion contribution is positive;
- expanded accounts maintain or improve retention.

## Conditional

- strong branch demand but implementation friction;
- demand occurs mainly in one segment;
- customers prefer flat pricing over per-branch pricing.

## Fail

- branch requests are mostly aspirational;
- customers will not pay for branch functionality;
- branch expansion creates disproportionate support cost.

---

# 17. Experiment 10 — EIS as a Purchase Driver

## Objective

Determine how much EIS-related capability influences purchasing decisions versus general operational value.

## Important compliance condition

Do not run this test using an unverified claim such as “Sitolo is MRA certified” unless the exact certification status and product version are current and documented.

MRA's published materials state that third-party POS systems must be certified and that MRA provides its own EIS POS solutions free to taxpayers.

## Hypothesis

For the formal-business cohort, EIS integration/compliance convenience improves conversion, but operational control remains the stronger product reason for retention.

## Minimum sample

**100 qualified formal-business prospects**:

~~~text
50 — operational value message
50 — operational value + approved EIS capability message
~~~

## Primary metric

Qualified prospect → paid conversion.

## Secondary metrics

- stated purchase reason;
- demo completion;
- legal/compliance questions;
- time-to-conversion;
- retention;
- support burden.

## Pass threshold

EIS message passes as a purchase driver only if:

- conversion improves by at least **8 percentage points**, or
- EIS is cited as a primary decision factor by at least **20%** of converted formal-business customers.

The result does not permit EIS to replace the core value proposition.

## Conditional

- EIS affects conversion in formal businesses but not broader SMEs;
- EIS helps close deals but does not affect retention.

## Fail

- no measurable conversion impact;
- customers primarily care about operational problems instead;
- compliance positioning causes unsupported claims or implementation risk.

---

# 18. Experiment 11 — Premium Support Demand

## Objective

Determine whether higher-support commitments can be monetized without subsidizing high-touch service.

## Hypothesis

A meaningful subset of growing customers will pay for faster support, onboarding, and escalation when operational continuity matters.

## Population

Existing paid customers and qualified enterprise-adjacent prospects.

## Minimum sample

**60 accounts** offered:

~~~text
Standard support
vs
Priority support
~~~

Test a premium price, initially in the range of **10–25% of the underlying subscription**, without permanently standardizing the percentage.

## Primary metric

Priority-support attach rate among eligible customers.

## Secondary metrics

- support hours per account;
- response-time requirement;
- incident severity;
- support satisfaction;
- contribution after support labour;
- churn.

## Pass threshold

Pass if:

- at least **15%** of eligible accounts attach priority support;
- incremental support revenue exceeds incremental support cost by a meaningful margin;
- standard-service economics remain healthy.

## Conditional

- demand exists only for enterprise;
- customers want response-time commitments but not generic “priority support.”

## Fail

- attach rate is low;
- premium customers consume more support than the premium covers;
- support expectations exceed operational capability.

---

# 19. Experiment 12 — Enterprise Pilot Economics

## Objective

Determine whether enterprise contracts can be sold without turning product engineering into unprofitable custom consulting.

## Hypothesis

A standardized proof-of-value with defined implementation scope can convert higher-complexity customers at positive contribution.

## Population

Qualified enterprises, institutions, and multi-branch operators.

## Minimum sample

**12 qualified enterprise prospects**.

Target structure:

~~~text
12 opportunities
8 paid or commercially committed proofs-of-value
4 structured discovery / non-paid controls where appropriate
~~~

Prioritize prospects with:

- multiple branches;
- meaningful transaction volume;
- integration needs;
- governance requirements;
- identifiable executive buyer.

## Required pilot structure

Every pilot must define:

~~~text
baseline
scope
duration
success metrics
implementation responsibilities
data migration scope
security requirements
support model
commercial conversion path
exit date
~~~

## Primary metric

Pilot → paid contract conversion.

## Secondary metrics

- ACV;
- implementation cost;
- engineering hours;
- support hours;
- security review effort;
- procurement duration;
- expansion scope;
- contract duration.

## Pass threshold

Enterprise pilot program passes when:

1. at least **3 pilots convert to paid contracts** or achieve a documented contracted expansion path;
2. average implementation contribution is positive;
3. median sales-cycle duration is economically compatible with contract value;
4. custom requirements do not dominate the product roadmap;
5. the standard enterprise package accounts for most implementation effort.

## Conditional

- strong conversion but excessive customization;
- high ACV but very long procurement;
- good economics in one vertical only.

## Fail

- pilots consistently require bespoke engineering;
- no meaningful conversion;
- contract value does not cover sales + implementation + support + security burden.

---

# 20. Pricing-Test Framework

Pricing experiments must test the complete commercial object, not only a number.

The object is:

~~~text
PRICE
+
CAPABILITY
+
LIMITS
+
SUPPORT
+
BILLING FREQUENCY
+
IMPLEMENTATION
+
DATA/EXPORT TERMS
~~~

## 20.1 Pricing dimensions

Potential dimensions:

- monthly;
- annual;
- branch count;
- user/device count;
- reconciliation depth;
- reporting depth;
- support tier;
- API/integration;
- enterprise SLA.

## 20.2 Pricing rules

Do not:

- hide mandatory fees;
- manufacture fake discounts;
- charge customers for regulatory necessities without clear value;
- create artificial limits that damage core workflows;
- punish proper security use;
- charge for exporting customer data.

## 20.3 Annual billing test

Test annual prepayment after baseline monthly willingness-to-pay is established.

Minimum sample:

**60 paying accounts**.

Primary metric:

annual-prepayment adoption.

Secondary:

- cash collected;
- churn;
- discount cost;
- support burden;
- downgrade rate.

A proposed annual discount must be approved only when the retention/cash-flow improvement exceeds the contribution lost through discounting.

---

# 21. CAC Measurement Framework

Every acquisition channel must have an independently maintained ledger.

## 21.1 Channel IDs

Use at minimum:

~~~text
DIRECT_FOUNDER
FIELD
ACCOUNTANT
DISTRIBUTOR
HARDWARE
REFERRAL
CONTENT
DIGITAL_PAID
ENTERPRISE
OTHER_PARTNER
~~~

## 21.2 Required fields

~~~text
lead_id
account_id
channel_id
campaign_id
first_contact_at
qualified_at
demo_at
trial_start_at
activation_at
paid_at
first_invoice_amount
acquisition_cost
commission_cost
pre-conversion_support_cost
onboarding_cost
month_1_revenue
month_1_contribution
month_3_revenue
month_3_contribution
status
churn_at
churn_reason
~~~

## 21.3 CAC formula

~~~text
channel CAC =
attributable channel spend /
new paying customers
~~~

Calculate three forms:

~~~text
Blended CAC
Direct CAC
Fully-loaded channel CAC
~~~

The fully-loaded version is used for strategic decisions.

## 21.4 CAC payback

Track:

~~~text
gross-profit payback
contribution-margin payback
~~~

Internal target bands:

~~~text
≤ 6 months
strong operating signal

> 6 to 12 months
acceptable but requires discipline

> 12 months
high-risk unless retention/expansion evidence is unusually strong
~~~

These are Sitolo internal decision bands, not universal SaaS benchmarks.

---

# 22. Support Economics

Low-ARPU segments must be analysed separately.

Measure:

~~~text
tickets / merchant / month
support minutes / merchant
cost / ticket
escalation rate
repeat-incident rate
after-hours events
support cost / paid merchant
~~~

## Duka danger zone

A low-price merchant can destroy unit economics if human support remains high.

Therefore the Duka model should trend toward:

~~~text
self-service
+
guided onboarding
+
in-product education
+
low-cost support
~~~

High-touch assistance should be reserved for justified higher-value accounts.

---

# 23. Product-Market-Fit Gate System

PMF must be treated as a staged evidence system.

## Gate 0 — Instrumentation Ready

Required before commercial conclusions:

- activation event instrumented;
- payment event instrumented;
- plan/price instrumented;
- channel attribution instrumented;
- support cost captured;
- churn reason captured;
- expansion events captured;
- reconciliation usage captured.

**No PMF claim permitted without Gate 0.**

---

## Gate 1 — Problem Evidence

Minimum:

~~~text
30+ structured customer interviews
10+ observed businesses
3+ recurring high-cost workflow problems
~~~

At least one problem must be operationally measurable.

Preferred problem examples:

- payment mismatch;
- stock uncertainty;
- cash variance;
- reporting burden;
- staff-control weakness.

Interview evidence is qualitative and cannot replace paid behaviour.

---

## Gate 2 — Activation

Primary Validation Cohort target:

~~~text
≥ 40% Day-7 activation
~~~

Strong signal:

~~~text
≥ 55%
~~~

Activation must represent real business activity.

---

## Gate 3 — Paid Conversion

For qualified trial accounts:

~~~text
minimum:
≥ 15% paid conversion

strong:
≥ 25%
~~~

Conversion must be measured by cohort, not cherry-picked customers.

---

## Gate 4 — Retention

For activated primary-ICP customers:

~~~text
Day-30 retained activity:
≥ 70%

Day-90 retained activity:
≥ 60%
~~~

A stronger PMF candidate should show:

~~~text
Day-90:
≥ 70%
~~~

These are internal gates, not claims that all SaaS businesses should meet the same values.

---

## Gate 5 — Contribution Economics

Required:

~~~text
positive contribution margin by merchant
positive contribution after variable support
no structurally loss-making core segment
~~~

A segment may be intentionally negative only under an explicitly approved acquisition strategy with a documented conversion path.

Permanent subsidy without evidence is prohibited.

---

## Gate 6 — CAC Payback

Preferred:

~~~text
≤ 12 months contribution-margin payback
~~~

Strong:

~~~text
≤ 6 months
~~~

Any channel above 12 months requires documented justification.

---

## Gate 7 — Retention Depth

At least one core value-depth metric must correlate with retention.

Example:

~~~text
SALES ONLY
vs
SALES + STOCK
vs
SALES + STOCK + PAYMENTS
vs
SALES + STOCK + PAYMENTS + RECONCILIATION
~~~

The expected pattern is that deeper workflow adoption should correlate with stronger retention.

The relationship must be measured, not assumed.

---

## Gate 8 — Non-Founder Acquisition

PMF should not be declared while all customers are acquired directly by the founder.

Minimum target:

~~~text
≥ 2 repeatable acquisition channels

20+ paying customers outside founder-only sourcing
~~~

with positive channel-level contribution.

---

## Gate 9 — Expansion

PMF candidate requires evidence that an installed account can expand.

Examples:

~~~text
new branch
higher plan
reconciliation
additional users/devices
enterprise package
~~~

Minimum signal:

~~~text
≥ 15% of eligible accounts
show a paid expansion event within 12 months
~~~

---

# 24. PMF Candidate Definition

Sitolo may use the internal label **PMF Candidate** only when all of the following hold for the primary validation cohort:

~~~text
[ ] Gate 0 instrumentation complete
[ ] Gate 1 problem evidence complete
[ ] ≥ 40% Day-7 activation
[ ] ≥ 60% Day-90 retained activity
[ ] ≥ 15% qualified paid conversion
[ ] positive contribution per merchant
[ ] CAC payback ≤ 12 months for at least one repeatable channel
[ ] two repeatable acquisition channels
[ ] measurable workflow-depth / retention relationship
[ ] evidence of expansion
[ ] no critical trust/reliability blocker
~~~

The designation remains provisional until the cohort is large and stable enough for meaningful confidence.

---


# 25. PMF Is Segment-Specific

Do not use one blended PMF label that hides divergent economics.

The existing PMF gate framework remains useful, but PMF evidence must be evaluated by segment.

## 25.1 Duka PMF Candidate

A Duka PMF candidate requires evidence across:

~~~text
FAST ACTIVATION
+
REPEATED REAL-BUSINESS USE
+
ACCEPTABLE DAY-30 / DAY-90 RETENTION
+
LOW CAC
+
LOW SUPPORT COST
+
POSITIVE CONTRIBUTION
+
ACTUAL PAID BEHAVIOR
+
NO CRITICAL TRUST / RELIABILITY BLOCKER
~~~

Strong supporting evidence includes:

- merchant referral;
- continued daily/weekly business-record use;
- growing share of transactions recorded in Sitolo;
- successful offline continuity;
- repeat reconciliation;
- low correction rate caused by system defects.

## 25.2 Growing-SME PMF Candidate

Add:

- sustained multi-user usage;
- purchasing;
- inventory depth;
- reconciliation;
- management reporting;
- desktop value;
- measurable expansion.

## 25.3 Enterprise PMF Candidate

Add:

- repeatable qualified pipeline;
- procurement acceptance;
- security acceptance;
- implementation contribution;
- recurring contracted value;
- acceptable sales cycle;
- limited custom-work burden;
- renewal/expansion evidence.

## 25.4 No blended PMF hiding

A strong enterprise cohort must not turn weak Duka economics into a company-wide "PMF" conclusion.

Likewise, a large Duka population must not conceal an enterprise motion that is permanently loss-making.

A company-level PMF statement should therefore be accompanied by segment-level evidence.

# 26. Commercial Dashboard

The minimum commercial dashboard should contain:

~~~text
MRR
ARR
NEW MRR
EXPANSION MRR
CONTRACTION MRR
CHURNED MRR
ARPU
PAID MERCHANTS
QUALIFIED LEADS
ACTIVATION RATE
PAID CONVERSION
DAY-30 RETENTION
DAY-90 RETENTION
NRR
CAC
CAC PAYBACK
GROSS MARGIN
CONTRIBUTION MARGIN
SUPPORT COST / MERCHANT
RECONCILIATION ADOPTION
EXPANSION RATE
~~~

---

# 27. Trust Dashboard

Commercial metrics must not be separated from product trust.

Track:

~~~text
PAYMENT EXCEPTIONS
RECONCILIATION EXCEPTIONS
STOCK DISCREPANCIES
SYNC FAILURES
FAILED COMMANDS
DUPLICATE EVENTS
REFUND EXCEPTIONS
OFFLINE CONFLICTS
SECURITY INCIDENTS
DATA-QUALITY INCIDENTS
~~~

A revenue number is not healthy if the underlying business record is unreliable.

---

# 28. Decision Rules

## 28.1 Pass

A pass means the hypothesis is sufficiently supported to become an input to product/commercial planning.

It does not mean permanent commitment.

## 28.2 Conditional

A conditional result means:

- the signal is promising;
- evidence is incomplete or segmented;
- another experiment is required.

## 28.3 Fail

A fail means:

- the hypothesis lacks supporting evidence;
- economics are negative;
- or operational risk exceeds value.

A failed experiment is a useful outcome because it prevents capital allocation into unsupported assumptions.

---

# 29. Commercial Experiment Calendar

A practical first validation cycle:

~~~text
WEEK 1–2
instrumentation
interviews
ICP qualification
pricing instrumentation

WEEK 3–5
Experiment 01
Experiment 02
Experiment 06

WEEK 4–7
Experiment 03
Experiment 04
Experiment 05

WEEK 5–12
Experiment 07
Experiment 08
Experiment 09

WEEK 6–10
Experiment 10
Experiment 11

WEEK 8–16
Experiment 12

WEEK 12
first retention review

WEEK 16
commercial gate review
~~~

This schedule is a planning sequence, not a requirement to compress observation windows below what the metric logically requires.

---

# 30. Experiment Dependencies

Some experiments must wait for prerequisite capability.

~~~text
01 Free vs Trial
    ↓
02 Price Sensitivity
    ↓
03 Reconciliation WTP
    ↓
09 Branch Expansion

06 Business-type onboarding
    ↓
07 Mobile retention
    ↓
08 Desktop incremental value

04 Accountant channel
    ↓
05 Distributor channel

10 EIS purchase driver
    ↓
only after approved/current EIS claim

11 Premium support
    ↓
after support instrumentation

12 Enterprise economics
    ↓
after enterprise security / contract readiness
~~~

Do not sell unsupported enterprise commitments merely to populate the experiment.

---

# 31. Qualitative Research Protocol

Every experiment must include structured qualitative follow-up.

Ask:

~~~text
What were you doing before Sitolo?
What was most painful?
What caused money or stock uncertainty?
What would make you stop using Sitolo?
What do you currently pay for?
What would make Sitolo worth paying for?
Which part saves time?
Which part reduces loss?
Which report do you actually use?
What would make you recommend it?
~~~

Avoid leading questions such as:

> “Would you pay MWK 50,000 for this?”

unless the product/value context has already been established.

Prefer actual purchase behaviour.

---

# 32. Customer Interview Evidence Hierarchy

Use this hierarchy:

~~~text
1. paid repeated behaviour
2. observed workflow change
3. repeated product usage
4. paid conversion
5. stated willingness to pay
6. stated preference
7. hypothetical interest
~~~

The farther down the list, the weaker the evidence.

---

# 33. Competitive Research Discipline

Track competitors continuously, but do not build the roadmap from feature checklists.

Capture:

~~~text
competitor
target segment
price
billing
trial
offline claim
EIS claim
payment capabilities
inventory capabilities
multi-branch
support model
data portability
differentiation message
customer evidence
unknowns
~~~

Competitive vendor claims must remain attributed.

Do not repeat competitor customer counts, performance claims, certification claims, or revenue claims as facts without independent verification.

---

# 34. Channel Attribution

Every customer must have a primary acquisition source.

When multiple channels contribute, record:

~~~text
first_touch
qualified_touch
conversion_touch
partner_influence
~~~

Do not give full credit to every channel.

For early-stage operations, use a simple primary-source model first and adopt multi-touch attribution only when volume justifies the complexity.

---

# 35. Cohort Rules

Every cohort must be immutable after creation.

Required cohort dimensions:

~~~text
signup_month
first_payment_month
segment
plan
price
channel
branch_count_at_start
workflow_depth
onboarding_variant
~~~

Retention calculations must not mix:

- trial and paid cohorts;
- different ICPs;
- different pricing cells;
- product versions with materially different capabilities.

---

# 36. Pricing Anti-Patterns

Prohibited commercial patterns:

~~~text
fake anchor pricing
permanent “temporary” discounts
hidden implementation fees
mandatory paid features that are actually regulatory necessities
charges designed to discourage security controls
opaque data-export fees
pricing that makes required multi-user authorization uneconomic
~~~

The commercial model must not create incentives that undermine Sitolo's security architecture.

---

# 37. Commercial Risk Gates

A commercial experiment must stop when:

- a pricing model materially harms core activation;
- support cost exceeds revenue contribution without a strategic exception;
- customer data handling creates legal/security risk;
- a regulatory claim cannot be substantiated;
- partner concentration becomes dangerous;
- the experiment requires weakening tenant isolation;
- customer onboarding requires unsafe data handling;
- a financial workflow cannot maintain reliable state.

Revenue does not override correctness or trust.

---

# 38. Evidence Storage

Each completed experiment should produce:

~~~text
experiment_brief.md
raw_metric_export
cohort_definition
analysis
qualitative_summary
decision_record
follow_up_actions
~~~

Where possible, preserve:

- timestamps;
- experiment version;
- product version;
- offer configuration;
- pricing cell;
- channel attribution.

Do not store unnecessary personal data solely for analytics.

---

# 39. Commercial Decision Log

Every experiment decision must record:

~~~text
EXPERIMENT_ID
DATE
SEGMENT
HYPOTHESIS
SAMPLE_TARGET
SAMPLE_OBSERVED
RESULT
PRIMARY_METRIC
SECONDARY_METRICS
CAC
CONTRIBUTION
DECISION
EVIDENCE_LINKS
KNOWN_LIMITATIONS
FOLLOW_UP_EXPERIMENT
OWNER
REVIEW_DATE
~~~

The word “success” is prohibited unless the published pass criteria were met.

---

# 40. Portfolio-Level Decision Rules

The program should produce a portfolio matrix:

| Area | Evidence | Economic Result | Decision |
|---|---|---|---|
| ICP | weak / moderate / strong | n/a | narrow / keep / expand |
| Pricing | weak / moderate / strong | positive / negative | reprice / retain |
| Reconciliation | weak / moderate / strong | positive / negative | core / add-on / deprioritize |
| Channel | weak / moderate / strong | positive / negative | scale / test / stop |
| Mobile | weak / moderate / strong | positive / negative | core / revise |
| Desktop | weak / moderate / strong | positive / negative | trigger-based / default |
| Branches | weak / moderate / strong | positive / negative | expand / defer |
| EIS | weak / moderate / strong | positive / negative | baseline / selling point |
| Support | weak / moderate / strong | positive / negative | monetize / automate |
| Enterprise | weak / moderate / strong | positive / negative | scale / defer |

The portfolio must be reviewed as a system, not as isolated experiments.

---

# 41. Investment and Capital-Allocation Gate

External capital, grants, or large internal spending should be tied to evidence.

Priority should be given to:

~~~text
proven acquisition
proven retention
reliability
security
customer success
repeatable enterprise sales
~~~

Avoid capital deployment primarily into:

- speculative fintech;
- data products without governed demand;
- feature sprawl;
- unsupported country expansion;
- unprofitable field sales;
- custom enterprise work with no reuse.

---

# 42. Country Expansion Gate

Country 2 must wait for:

~~~text
stable Malawi retention
repeatable Malawi acquisition
positive core SaaS contribution
manageable support burden
documented local payment requirements
documented tax requirements
validated local pricing
validated distribution channel
country-specific operational budget
~~~

A large theoretical TAM does not override weak Malawi unit economics.

---

# 43. Financial Ecosystem Gate

No partner-finance product should move beyond discovery until:

~~~text
SaaS economics are viable
data quality is proven
auditability is proven
customer disclosure is designed
legal model is clear
partner contract is signed
security review is complete
risk ownership is explicit
~~~

Sitolo must not make speculative finance revenue part of the base-case SaaS economics.

---

# 44. Commercial Stop List

Do not build primarily because:

- a competitor has the feature;
- an investor asks for a larger TAM story;
- the feature looks impressive;
- the feature may generate speculative future revenue;
- the feature helps close one uneconomic custom deal;
- the feature increases vanity metrics.

Build when evidence shows:

~~~text
customer pain
+
economic value
+
retention impact
+
acceptable delivery cost
~~~

---


# 45. First-Year Commercial Success Definition

The first-year objective is not maximum account count and is not to choose one permanent ICP.

The objective is to establish that a single platform can support multiple complexity levels with healthy commercial economics.

Required evidence:

~~~text
1.
A measurable core operational value proposition

+

2.
At least one low-touch segment
with acceptable contribution economics

+

3.
At least one growing-SME segment
with repeatable retention / expansion evidence

+

4.
At least two repeatable acquisition channels
for commercially material segments

+

5.
A viable enterprise motion
without roadmap capture by custom work

+

6.
Segment-level CAC, retention,
support, and contribution measurement

+

7.
A credible pricing and packaging model

+

8.
Evidence that Duka → SME → enterprise
can coexist on one platform through
progressive complexity
~~~

A large inactive account base is not success.

A large free-account population with negative contribution is not success.

A high-ACV enterprise contract that requires permanent bespoke engineering is not proof of a scalable enterprise motion.

Each segment must earn increased investment through evidence.

# 46. Final Commercial Validation Contract

Sitolo's commercial operating loop is:

~~~text
HYPOTHESIS
    ↓
QUALIFIED SAMPLE
    ↓
CONTROLLED OFFER
    ↓
REAL PRODUCT USE
    ↓
PAYMENT / NON-PAYMENT
    ↓
RETENTION
    ↓
COST-TO-SERVE
    ↓
CAC / CONTRIBUTION
    ↓
DECISION
    ↓
NEXT EXPERIMENT
~~~

The governing principle is:

> **Do not confuse market interest with a business.**

A commercially validated Sitolo account is a merchant that:

- repeatedly uses the product;
- trusts the business record;
- pays for value;
- remains economically serviceable;
- expands when business complexity grows;
- and can be acquired through a channel whose economics can be repeated.

That evidence should precede aggressive regional expansion, speculative fintech, large field-sales deployment, or extensive enterprise customization.

---



# 47. Tier, Device and Subscriber Validation Framework

The commercial validation program must validate the **tier architecture**, not merely one generic subscription price.

## 47.1 Tier architecture hypothesis

The current planning hypothesis is:

| Tier | Indicative monthly price | Customer | Default device model | UI complexity |
|---|---:|---|---|---|
| Duka | MK7,500 | One-location ultra-micro merchant | Mobile only | Very simple |
| SME | MK25,000 | Small established retailer | Mobile + PC | Simple mobile / moderate PC |
| Growth | MK50,000 | Growing, more complex SME | Mobile + PC | Moderate mobile / detailed PC |
| Multi-Branch | From MK90,000 | Two or more locations | Mobile branches + PC HQ | Simple branch / high-density HQ |
| Enterprise | From MK200,000 | High-governance organization | Mobile + PC/Web | Role-specific / high-density control |

The values remain commercial hypotheses.

## 47.2 Device doctrine

The underlying hypothesis is:

~~~text
MOBILE
→ RUN / OPERATE

PC
→ CONTROL / MANAGE

WEB
→ ADMINISTER / SUPPORT
~~~

This is a workflow rule.

It is not a rigid rule that every SME must have both devices or that every enterprise branch must have a PC.

## 47.3 Duka device hypothesis

~~~
DUKA
→ EXISTING ANDROID PHONE
→ MOBILE OPERATIONS
→ NO PC REQUIREMENT
→ NO PRINTER REQUIREMENT
→ NO SCANNER REQUIREMENT
~~~

The Duka test must measure whether existing-device onboarding reduces:

- acquisition friction;
- time-to-first-sale;
- onboarding cost;
- support.

## 47.4 SME and Growth device hypothesis

~~~
MOBILE
→ FRONTLINE

PC
→ BACK OFFICE
~~~

Test whether desktop availability improves:

- retention;
- reporting usage;
- reconciliation;
- purchasing;
- expansion.

Desktop should remain optional for low-complexity workflows.

## 47.5 Multi-Branch device hypothesis

~~~
                    HQ
                    |
                   PC
                    |
       +------------+------------+
       |            |            |
    Branch A     Branch B     Branch C
      Mobile       Mobile       Mobile
~~~

The primary test is whether centralized PC control plus mobile branch operation reduces total device requirements while preserving control.

A branch PC may be justified by:

- transaction volume;
- peripherals;
- staff workflow;
- local operational requirements.

Branch count alone is insufficient justification.

## 47.6 Enterprise device hypothesis

~~~
PC / WEB
=
CONTROL PLANE

MOBILE
=
DISTRIBUTED OPERATING PLANE
~~~

Enterprise testing must determine how much control-plane work belongs in PC/Web and how much branch activity remains better served by mobile.

## 47.7 Device problem hypothesis

The device constraint is real enough to measure explicitly.

Afrobarometer's 2024/2025 Malawi survey reported 25% adult smartphone ownership and 10% household computer ownership. GSMA's August 2026 Malawi report separately estimated smartphone adoption at 33% and identified device affordability and the mobile-internet usage gap as major constraints.

The studies use different methods, so their percentages must not be mechanically combined.

The commercial conclusion to test is:

> Requiring a PC at entry can materially reduce addressable adoption for the lower end of the market.

## 47.8 Hardware policy

Default:

~~~text
SOFTWARE FIRST
+
BYOD FIRST
+
OPTIONAL HARDWARE
~~~

Potential future accessories:

- Android tablets;
- receipt printers;
- barcode scanners;
- dedicated POS devices.

Hardware partnerships remain optional.

## 47.9 Device replacement

A device test is incomplete unless replacement works:

~~~text
OLD DEVICE LOST
      ↓
NEW DEVICE
      ↓
AUTHENTICATE
      ↓
AUTHORIZE
      ↓
RESTORE PERMITTED STATE
      ↓
CONTINUE OPERATING
~~~

Business identity and data must not be tied irreversibly to one physical device.

## 47.10 Subscriber planning scenario

The following is a **portfolio planning scenario**, not a forecast:

| Tier | Illustrative paying accounts | Share |
|---|---:|---:|
| Duka | 600 | 60% |
| SME | 220 | 22% |
| Growth | 100 | 10% |
| Multi-Branch | 60 | 6% |
| Enterprise | 20 | 2% |
| **Total** | **1,000** | **100%** |

Using the planning price hypotheses:

~~~text
Duka
600 × MK7,500 = MK4.5M MRR

SME
220 × MK25,000 = MK5.5M MRR

Growth
100 × MK50,000 = MK5.0M MRR

Multi-Branch
60 × MK90,000 = MK5.4M MRR

Enterprise
20 × MK200,000 = MK4.0M MRR

TOTAL = MK24.4M illustrative MRR
~~~

This scenario is useful because it demonstrates:

> **A majority-Duka customer base can coexist with a diversified revenue portfolio.**

It does not establish:

- expected market share;
- expected year-one subscribers;
- actual conversion;
- actual retention;
- actual revenue.

## 47.11 Subscriber target governance

Hard subscriber targets must be stage-gated.

~~~
PRODUCT VALIDATION
→
PAID VALIDATION
→
SEGMENT CONTRIBUTION
→
CHANNEL REPEATABILITY
→
SCALE
~~~

Increasing account targets before proving unit economics is prohibited by this commercial doctrine.

## 47.12 Segment-level subscriber reporting

At minimum report:

~~~text
TOTAL ACCOUNTS
TOTAL PAYING ACCOUNTS
ACTIVE PAYING ACCOUNTS
DUKA ACCOUNTS
SME ACCOUNTS
GROWTH ACCOUNTS
MULTI-BRANCH ACCOUNTS
ENTERPRISE ACCOUNTS
NET NEW ACCOUNTS
CHURNED ACCOUNTS
UPGRADED ACCOUNTS
DOWNGRADED ACCOUNTS
~~~

Also report per segment:

~~~text
ARPU
CAC
DAY-30 RETENTION
DAY-90 RETENTION
SUPPORT COST
CONTRIBUTION
~~~

## 47.13 Tier transition metrics

Track:

~~~text
DUKA → SME
SME → GROWTH
GROWTH → MULTI-BRANCH
MULTI-BRANCH → ENTERPRISE
~~~

But also:

~~~text
DUKA → REMAINS DUKA
SME → REMAINS SME
GROWTH → REMAINS GROWTH
~~~

Remaining on a profitable tier is not failure.

## 47.14 Device adoption metrics

Track:

~~~text
MOBILE-ONLY ACTIVE RATE
PC-ACTIVE RATE
MOBILE + PC ACTIVE RATE
AVERAGE DEVICES / ACCOUNT
DEVICES / BRANCH
PC-REQUIRED-TO-ACTIVATE RATE
DEVICE-RELATED CHURN
DEVICE-RELATED SUPPORT
~~~

The key metric is not device count.

The key metric is whether device architecture improves business outcomes at acceptable cost.

## 47.15 UI complexity metrics

Track:

- time to first sale;
- time to complete common workflow;
- task error rate;
- support incidents related to navigation;
- feature discovery;
- number of screens visited for common tasks;
- abandonment.

The lower tier must not win merely by having fewer features.

It must win by accomplishing its critical jobs with less friction.

## 47.16 Tier economics gate

A tier is not commercially scalable until:

~~~text
REVENUE
− PAYMENT COST
− INFRASTRUCTURE
− SUPPORT
− CAC / COMMISSION
− ONBOARDING
=
POSITIVE OR STRATEGICALLY JUSTIFIED CONTRIBUTION
~~~

Any intentionally subsidized tier requires an explicit decision record showing:

- why the subsidy exists;
- what evidence is expected;
- the maximum duration;
- the conversion/retention mechanism;
- the stop condition.

## 47.17 Device and tier anti-patterns

Prohibited:

~~~text
CHEAP PLAN
→ BAD RELIABILITY

CHEAP PLAN
→ WEAK SECURITY

DUKA
→ FORCED PC

MULTI-BRANCH
→ FORCED PC AT EVERY BRANCH

ENTERPRISE
→ MOBILE DISABLED

UPGRADE
→ ACCOUNT RECREATION

HARDWARE
→ REQUIRED BEFORE FIRST VALUE
~~~

## 47.18 Final device doctrine

The official commercial interpretation is:

~~~text
DUKA
→ MOBILE FIRST / MOBILE ONLY BY DEFAULT

SME
→ MOBILE + PC

GROWTH
→ MOBILE + PC

MULTI-BRANCH
→ MOBILE BRANCHES + PC HQ

ENTERPRISE
→ MOBILE OPERATIONS + PC / WEB CONTROL
~~~

But the underlying rule remains:

> **Use the device that best performs the job.**


# 48. References and Current External Market Baseline

1. **DataReportal — Digital 2026: Malawi**
   - late-2025 data used for the 2026 planning cycle;
   - reports 14.1 million cellular connections and 4.02 million internet users;
   - explicitly distinguishes mobile connections from unique individuals.
   - https://datareportal.com/reports/digital-2026-malawi

2. **Malawi Revenue Authority — Understanding Electronic Invoicing System**
   - MRA states that its computer, mobile, and web EIS POS solutions are provided free to taxpayers;
   - third-party integrated invoicing systems incur vendor cost;
   - https://www.mra.mw/assets/upload/downloads/UNDERSTANDING_ELECTRONIC_INVOICING_SYSTEM_%28EIS%29.pdf

3. **Malawi Revenue Authority — EIS API certification**
   - third-party POS systems require certification;
   - certification can cover both provider and software/product;
   - https://eis-api.mra.mw/docs/api_compliance_certification.htm
   - https://eis-api.mra.mw/docs/certification_process.htm

4. **Phindu**
   - current public pricing and product positioning used as a market signal;
   - https://www.phindu.co/pricing
   - https://www.phindu.co/pos-system-malawi

5. **MalondaPlus**
   - current public pricing and product positioning used as a market signal;
   - https://www.malonda.ictechmw.com/malondaplus/index.php

Public competitor prices and claims are not treated as independent proof of demand, market share, or product superiority.

---

# 49. Acceptance Criteria

This validation plan is implementation-ready when the commercial operating system can answer:

~~~text
WHO IS THE ICP?
WHAT PROBLEM IS BEING TESTED?
WHAT IS THE CONTROL?
WHAT IS THE TREATMENT?
HOW MANY ACCOUNTS ARE REQUIRED?
WHAT COUNTS AS ACTIVATION?
WHAT COUNTS AS RETENTION?
WHAT PRICE WAS TESTED?
WHAT DID THE CUSTOMER ACTUALLY PAY?
WHAT DID ACQUISITION COST?
WHAT DID SUPPORT COST?
WHAT CONTRIBUTION WAS CREATED?
WHAT WAS THE PASS THRESHOLD?
WHAT WAS THE ACTUAL RESULT?
WHAT DECISION FOLLOWED?
~~~

No commercial initiative should be described as validated when these questions cannot be answered.

**End of Commercial Validation Plan.**


# 50. Commercial Operating Model Documentation Crosswalk

The Commercial Operating Model is now decomposed into dedicated documents. The existing experiment framework remains authoritative for validation thresholds and methodology; these documents define the mechanisms to validate.

| Commercial mechanism | Governing document | Existing/required validation focus |
|---|---|---|
| buyer/user/approver separation | [Buyer/User/Approver Model](buyer_user_approver_and_purchase_process.md) | decision-path mapping, approval loss, sales-cycle friction |
| collection/payment method | [Payment & Collections](payment_collections_and_subscription_billing.md) | payment-method conversion, success, fees, delinquency, recovery |
| distribution | [Distribution Strategy](distribution_channel_and_market_access_strategy.md) | channel CAC, activation, retention, contribution |
| partner economics | [Partner Economics](partner_agent_economics_and_governance.md) | commission burden, partner quality, concentration |
| onboarding/migration | [Onboarding & Migration](onboarding_and_migration_strategy.md) | time-to-first-value, migration failure, assisted-cost burden |
| trust/recovery | [Trust & Continuity](trust_adoption_and_operational_continuity.md) | recovery success, discrepancy reports, trust incidents |
| prolonged offline | [Offline Validation](offline_continuity_commercial_validation.md) | multi-day offline convergence, duplicate/conflict rate, support burden |
| retention/churn | [Retention & Churn](retention_churn_and_customer_lifecycle.md) | segmented churn, reactivation, seasonal inactivity |
| support economics | [Support Economics](support_economics_and_service_operations.md) | contacts/account, cost/account, resolution, support-caused churn |
| regulatory boundary | [Regulatory Perimeter](regulatory_perimeter_and_compliance_boundary.md) | legal/provider readiness before regulated feature release |
| tax/fiscal readiness | [Tax & Fiscal Readiness](tax_and_fiscal_readiness_strategy.md) | tax-status segmentation, EIS purchase driver, compliance workflow |
| data portability | [Data Governance](data_portability_and_data_governance.md) | export success, enterprise procurement questions, trust/churn signals |
| fraud/loss prevention | [Fraud & Business Control](fraud_loss_prevention_and_business_control.md) | variance reduction, control adoption, willingness-to-pay |
| verticalization | [Vertical Module Model](vertical_module_commercial_model.md) | module demand, support burden, incremental willingness-to-pay |
| upgrade/expansion | [Upgrade & Tier Migration](upgrade_expansion_and_tier_migration_model.md) | trigger frequency, upgrade acceptance, expansion contribution |
| unit economics | [Unit Economics](unit_economics_cac_and_contribution_model.md) | contribution, CAC, payback by tier/channel |
| defensibility | [Defensibility Strategy](defensibility_and_moat_strategy.md) | workflow depth, referral share, retention, integration dependency |
| geographic expansion | [Geographic Expansion](geographic_expansion_and_country_entry_strategy.md) | country dossier completeness and pre-entry economics |
| reconciliation | [Mobile Money Reconciliation](mobile_money_reconciliation_business_model.md) | reconciliation adoption, exception resolution, retention impact |
| merchant credit/lay-by | [Credit & Lay-by](credit_and_layby_commercial_model.md) | workflow demand and legal classification before financial-product expansion |
| intelligence/alerts | [Business Intelligence](business_intelligence_alerts_and_action_model.md) | action rate and measurable business outcome, not “AI usage” |

## 50.1 Required Experiment Discipline

Every new experiment derived from these documents must retain the existing discipline:

- one primary hypothesis;
- pre-registered population and metric;
- fixed pass/fail/conditional thresholds;
- segment-level reporting;
- contribution economics where money is involved;
- documented evidence and decision log.

No new commercial document overrides the experiment governance already defined in this plan.
