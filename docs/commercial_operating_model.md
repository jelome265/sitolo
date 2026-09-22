# Sitolo — Commercial Operating Model

**Status:** Enterprise target commercial-control document  
**Market entry:** Malawi first  
**Scope:** Duka through enterprise  
**Commercial posture:** broad product scope; segment-specific, evidence-led execution  
**Date:** 2026-09-22  
**Authority:** commercial strategy layer; subordinate to applicable law/regulator requirements, current provider contracts, and approved product/security architecture decisions.

---

## 1. Executive Decision

Sitolo is one Business Operating System for African SMEs, with a universal product scope and a staged commercial machine.

Universal scope does **not** mean simultaneous selling to every segment.

The operating doctrine is:

```text
ONE PLATFORM
+
COMMON BUSINESS CORE
+
VERTICAL CONFIGURATION
+
PLAN ENTITLEMENTS
+
PROGRESSIVE DISCLOSURE
+
SEGMENT-SPECIFIC GTM
+
SEGMENT-SPECIFIC ECONOMICS
+
EVIDENCE-GATED SCALE
```

The commercial system must answer five questions for every segment:

1. Who experiences the problem?
2. Who decides to buy?
3. Who pays and how is payment collected?
4. What causes the customer to remain and expand?
5. Does the account produce positive contribution after all material variable costs?

Account count is not the primary economic objective. Sustainable contribution, retained business value, and repeatable distribution are.

---

## 2. Relationship to Existing Documentation

This document connects the commercial layer to the existing Sitolo corpus.

Existing documents remain authoritative within their domains:

- `business_model_design.md` — product/business model target.
- `commercial_validation_plan.md` — experiment governance and validation gates.
- `segment_strategy_and_duka_economics.md` — universal segment/tier/device strategy.
- `domain_model.md` — business semantics and invariants.
- `system_architecture_design.md` — technical architecture.
- `payment_integration_spec.md` — provider/payment implementation boundary.
- `threat_model.md` and security documents — security boundaries.
- `deployment_spec.md` — operational/release boundaries.

Commercial documents must never override legal, regulatory, security, financial-integrity, tenant-isolation, or provider-contract requirements.

When a commercial proposal requires a technical or regulated capability that is not yet approved, the capability remains a hypothesis rather than an entitlement.

---

## 3. Commercial Operating Model Map

```text
                         SITOLO
                           |
                 COMMERCIAL OPERATING MODEL
                           |
      +--------------------+--------------------+
      |                    |                    |
    DEMAND              DELIVERY             ECONOMICS
      |                    |                    |
 buyer/user            onboarding            price
 approver              support              payment
 channel               trust                 CAC
 segment               continuity            COGS
      |                    |                    |
      +--------------------+--------------------+
                           |
                       RETENTION
                           |
             +-------------+-------------+
             |             |             |
          VALUE         CONTROL       EXPANSION
             |             |             |
        insights       fraud/cash      more users
        reports        reconciliation  more branches
        reorder        audit           higher tier
                           |
                        MOAT
                           |
       network + history + workflows + trust
       + integrations + distribution + data products
```

---

## 4. Commercial Principles

### 4.1 Product universality, commercial sequencing

Sitolo may support Dukas, growing SMEs, specialists, multi-branch organizations and enterprises.

The go-to-market machine must still select bounded learning cohorts.

The distinction is:

```text
PRODUCT SCOPE = BROAD
COMMERCIAL EXECUTION = FOCUSED
```

The commercial team must never use “serve everyone” as a justification for:

- launching every acquisition channel simultaneously;
- supporting every vertical deeply at launch;
- pricing every segment from one assumption;
- running one blended PMF metric;
- accepting unprofitable low-ARPU accounts;
- hiding segment failure behind aggregate growth.

### 4.2 Complexity-based tiers

Commercial tiers represent operational complexity, not industry.

A pharmacy and a grocery store can share a plan when their operational complexity is similar, while a simple pharmacy and a large pharmacy chain should not be forced into the same commercial package.

### 4.3 Value before monetization

The first commercial event is not payment.

It is successful business operation.

The core activation sequence is:

```text
CREATE
→ CONFIGURE
→ RECORD FIRST VALUE
→ OPERATE REPEATEDLY
→ TRUST THE RECORD
→ PAY
→ EXPAND
```

### 4.4 No artificial dependency on hardware

Hardware is optional at entry.

The commercial unit is the business, not the phone, printer, scanner, or PC.

---

## 5. Commercial Actor Model

Every account must be analysed using at least these actors:

| Actor | Meaning | Example |
|---|---|---|
| Economic buyer | controls purchase decision/budget | owner, director, procurement |
| User | performs daily work | cashier, stock clerk |
| Approver | approves risk-sensitive decisions | owner, manager, IT/security |
| Administrator | configures organization | operations/admin |
| Beneficiary | receives business value | owner, management, finance |
| Payer | causes payment to Sitolo | owner, finance, procurement |
| Influencer | affects purchase without owning budget | accountant, distributor |

A single person may hold several roles in a Duka. Enterprise accounts normally separate them.

Commercial research must capture actor separation explicitly.

---

## 6. Buyer/User/Approver Matrix

| Segment | Economic buyer | Primary users | Typical approvers/influencers | Payer |
|---|---|---|---|---|
| Duka | owner/operator | owner/operator | sometimes family/partner | owner |
| Small SME | owner | owner + staff | owner/accountant | owner |
| Growth | owner/director | managers + staff | finance/accountant | business |
| Multi-Branch | owner/director/group management | branch managers + workers | finance/operations/IT | central finance |
| Enterprise | business unit/procurement | workers + managers + admins | finance, IT, security, legal/procurement | central finance/procurement |

This matrix is a starting hypothesis. Validation must record the actual decision path per account.

---

## 7. Payment Collection Principle

Sitolo pricing is not commercially complete until collection is operationally defined.

Every subscription must have:

- payer identity;
- amount;
- currency;
- billing period;
- due date;
- payment method;
- collection status;
- invoice/receipt state;
- settlement state;
- refund state where applicable;
- failure reason;
- retry policy;
- grace policy;
- suspension state;
- reactivation state;
- reconciliation evidence.

The subscription record must distinguish:

```text
BILLED
≠
PAYMENT INITIATED
≠
PAYMENT CONFIRMED
≠
FUNDS SETTLED
```

The business must never treat an ambiguous provider response as successful payment.

---

## 8. Duka Payment Model

The Duka commercial lane must support more than one collection mechanism.

Candidate methods to validate:

- mobile-money initiated payment;
- bank transfer;
- assisted/agent-collected payment;
- cash collected by an authorized commercial agent, where permitted and controllable;
- card/online payment only where commercially useful.

“Card required” is not a product invariant.

The default experience should permit the smallest merchant to pay without owning a bank card, subject to the final payment-provider and legal arrangement.

Manual/assisted collection must create a proper accounting trail and must not become an unbounded cash-handling operation.

---

## 9. Billing Lifecycle

```text
SUBSCRIPTION ACTIVE
      |
   BILL GENERATED
      |
   DUE / PAST DUE
      |
  +---+------------------+
  |                      |
PAID                   NOT PAID
  |                      |
SETTLED              GRACE PERIOD
                         |
                    +----+----+
                    |         |
                   PAID    EXPIRED
                              |
                          RESTRICT /
                          SUSPEND
                              |
                           PAY
                              |
                         REACTIVATE
```

Grace periods must preserve business continuity wherever possible while preventing indefinite unpaid access.

Restriction design must avoid corrupting transaction history. It should restrict entitlements, not delete or mutate business records.

---

## 10. Distribution Principle

The core distribution question is:

> Can Sitolo acquire and onboard merchants outside founder-led visits while maintaining acceptable CAC, activation, support burden, and contribution?

The desired model is:

```text
HEAD OFFICE
    ↓
DISTRICT / REGION
    ↓
TRADING CENTRE
    ↓
LOCAL CHANNEL PARTNER
    ↓
MERCHANT
```

Potential channels are hypotheses to test, not assumptions:

- accountants/bookkeepers;
- wholesalers/distributors;
- telecom/mobile-money agent networks;
- merchant associations;
- local technology agents;
- reseller/dealer organizations;
- referrals;
- community acquisition;
- digital self-service;
- device partners.

Each channel needs a unique attribution ID and complete economic accounting.

---

## 11. Channel Economics

For every channel:

```text
CHANNEL CAC =
marketing
+ sales labour
+ travel
+ onboarding
+ commissions
+ partner incentives
+ samples/promotions
+ payment acquisition costs
```

Do not compare channels using advertising spend alone.

Channel quality must be evaluated using:

- qualified lead rate;
- activation rate;
- paid conversion;
- 30/90/180-day retention;
- support hours per account;
- contribution margin;
- CAC payback;
- fraud/chargeback/collection losses;
- partner dependency.

---

## 12. Zero-Data Duka Onboarding

The default onboarding path must permit a merchant to start without a historical export.

Target flow:

```text
DOWNLOAD
   ↓
CREATE BUSINESS
   ↓
CHOOSE BUSINESS TYPE
   ↓
ADD 3–10 PRODUCTS
   ↓
OPTIONALLY ENTER COST / SELL PRICE
   ↓
RECORD OPENING STOCK OR START FRESH
   ↓
MAKE FIRST SALE
   ↓
SEE FIRST BUSINESS SUMMARY
```

The system must not require a merchant to understand accounting vocabulary before achieving first value.

Optional richer setup follows after first value.

---

## 13. Notebook Migration

Migration has three modes:

### Mode A — Start fresh

Historical detail is intentionally not imported.

The merchant records opening state and begins operating.

### Mode B — Opening balances

Import only the minimum operational state:

- cash;
- bank/mobile-money opening balance;
- inventory quantities/value where known;
- customer receivables;
- supplier payables;
- outstanding lay-bys/credit obligations.

### Mode C — Historical migration

For higher-value customers, optionally import historical records after validation and data-quality controls.

Historical migration must never be allowed to pollute authoritative financial history with ambiguous or unverifiable records.

---

## 14. Trust as a Commercial Requirement

Trust must be measured, not described only in marketing language.

A merchant must be able to answer:

- what happens if the phone is lost;
- whether business data survives device replacement;
- who changed a price;
- who recorded a sale;
- what happened to stock;
- what happened to money;
- what happens when connectivity disappears;
- how records are corrected;
- how data is exported;
- how account recovery works.

The trust loop is:

```text
VISIBLE HISTORY
+
AUDITABILITY
+
RECOVERABILITY
+
OFFLINE CONTINUITY
+
DATA PORTABILITY
+
CLEAR SUPPORT
=
COMMERCIAL TRUST
```

---

## 15. Offline Commercial Continuity

Offline behaviour is commercially material.

A representative validation scenario is:

```text
DAY 1 ONLINE
DAY 2 OFFLINE
DAY 3 OFFLINE
DAY 4 ONLINE
```

Test and measure:

- local durability;
- command idempotency;
- duplicate handling;
- conflict handling;
- stock outcomes;
- timestamps;
- user attribution;
- payment state;
- reconciliation state;
- reporting freshness;
- recovery duration.

The existing sync protocol remains the technical authority.

Commercial acceptance requires that the failure mode be understandable to a merchant and not silently manufacture financial truth.

---

## 16. Owner Absence / Business Continuity

Business identity must be organization-centred rather than phone-centred.

Commercially important continuity cases include:

- owner loses phone;
- owner changes phone number;
- owner is unavailable;
- manager temporarily operates business;
- owner delegates administration;
- staff continue after owner travel.

Recovery must preserve:

- organization;
- branch relationships;
- authorized users;
- business history;
- audit history;
- subscriptions;
- entitlements.

High-risk recovery must not become a social-engineering bypass.

---

## 17. Internal Fraud Value Proposition

Sitolo should create measurable control around:

- unrecorded sales;
- unauthorized discounts;
- price changes;
- voids;
- refunds;
- stock adjustments;
- fake expenses;
- unusual cash variances;
- unauthorized access;
- suspicious repeated corrections.

Near-term product value is visibility and control.

Longer-term value may include anomaly detection, but only after sufficient evidence and governance.

The commercial claim should be:

> Know what happened in the business when the owner was not present.

That claim must remain grounded in actual controls.

---

## 18. Credit / Lay-by Commercial Boundary

Customer credit should be treated as a business-operating capability, not automatically as lending.

Core business-software functions may include:

- customer account;
- obligation amount;
- originating sale;
- payments received;
- outstanding balance;
- ageing;
- responsible user;
- settlement history.

The system must distinguish:

```text
MERCHANT RECORDING CUSTOMER DEBT
≠
SITOLO PROVIDING CREDIT
```

Any Sitolo-funded lending, financing, credit underwriting or money-advancing product requires separate legal/compliance assessment.

---

## 19. Reconciliation as Retention Layer

The commercial control loop is:

```text
SALE
 ↓
PAYMENT METHOD
 ↓
SETTLEMENT / COLLECTION
 ↓
RECONCILIATION
 ↓
EXCEPTION
 ↓
ACTION
```

Long-term value increases when Sitolo can help a merchant understand:

- expected cash;
- recorded cash;
- mobile-money collections;
- bank movement;
- outstanding credit;
- unresolved exceptions.

Reconciliation must not be marketed as a promise of payment settlement unless Sitolo actually controls the relevant rail.

---

## 20. Upgrade Mechanics

Tier migration is triggered by business complexity.

Candidate signals:

| Complexity signal | Possible commercial response |
|---|---|
| more than one regular user | SME capabilities |
| growing catalogue/procurement workload | Growth capabilities |
| management reporting need | Growth |
| multiple branches | Multi-Branch |
| central finance/control | Multi-Branch / Enterprise |
| formal governance/security review | Enterprise |
| integration/custom SLA requirement | Enterprise |

The exact thresholds must be measured.

The system must avoid arbitrary feature walls that create forced upgrades without a corresponding business problem.

---

## 21. Vertical Modules

The commercial model is:

```text
CORE BUSINESS OS
+
BUSINESS-TYPE CONFIGURATION
+
OPTIONAL VERTICAL MODULES
+
PLAN ENTITLEMENTS
```

Core concepts remain shared:

- sales;
- inventory;
- purchasing;
- pricing;
- cash;
- payments;
- customers;
- reconciliation;
- reporting;
- audit.

Vertical modules add specialized workflows and invariants.

Examples:

- pharmacy;
- agro-dealer;
- wholesale;
- hospitality/food service;
- specialist retail.

Verticalization must not create independent product forks.

---

## 22. Tax/Fiscal Readiness

Tax functionality must be designed around configurable tax rules, not hard-coded assumptions.

Commercial requirements include support for differences in:

- tax registration status;
- tax-inclusive/exclusive pricing;
- tax treatment by product/service;
- invoice requirements;
- reporting obligations;
- fiscal periods;
- external tax-system integration.

The current Malawi fiscal baseline must be rechecked against the latest law, regulations and MRA notices before each release that makes compliance claims.

As of the 2026-27 Budget Policy Statement, Malawi announced an increase in the mandatory VAT registration threshold from MK25 million to MK50 million, subject to the stated legislative approval process; qualifying businesses remain subject to EIS requirements under the applicable framework. MRA separately published an EIS transition notice with a 31 January 2026 transition end date. These facts are incorporated as dated compliance signals, not permanent hard-coded product rules.

---

## 23. Regulatory Perimeter

The operating rule is:

### GREEN — ordinary business software

Examples:

- sales recording;
- inventory management;
- purchasing;
- business reporting;
- accounting-like operational records;
- merchant subscription billing;
- audit logs;
- business-data export.

### YELLOW — regulated-partner / legal-review boundary

Examples:

- initiating payments;
- merchant settlement workflows;
- storing sensitive identity/financial data at greater scale;
- credit-related integrations;
- tax-system integrations;
- regulated financial-product referrals;
- cross-border financial-data processing.

### RED — do not implement as an unlicensed Sitolo activity

Examples may include:

- accepting public deposits;
- operating a payment system/service without required authorization;
- issuing regulated payment instruments without required authorization;
- lending or microcredit activity without the required legal basis/licensing;
- acting as a regulated financial institution without the relevant authorization.

The classification is deliberately conservative. Legal counsel and the relevant regulator determine the final perimeter.

---

## 24. Data Portability

Commercial trust requires a credible exit path.

The merchant must be able to request an export of business data subject to applicable law and contract.

Exports must support:

- structured data;
- CSV where appropriate;
- machine-readable formats;
- audit/history exports;
- metadata explaining fields and time periods;
- export status and scope;
- controlled access.

Enterprise contracts should define retention, deletion, backup, restoration, residency and termination treatment explicitly.

Data portability must not weaken tenant isolation or expose another tenant's data.

---

## 25. Retention and Churn

Aggregate churn is insufficient.

Every cancellation should have a primary and optional secondary reason.

Suggested taxonomy:

- price;
- no value;
- network/connectivity;
- payment failure;
- support;
- complexity;
- staff problem;
- phone/device loss;
- business closed;
- seasonal inactivity;
- competitor;
- trust/security concern;
- migration problem;
- regulatory/compliance issue;
- other.

Churn reporting must be segmented by:

- tier;
- business type;
- geography;
- acquisition channel;
- age cohort;
- payment method;
- onboarding mode.

---

## 26. Customer State Model

At minimum:

```text
PROSPECT
→ TRIAL
→ ACTIVE
→ PAST_DUE
→ GRACE
→ SUSPENDED
→ REACTIVATED
→ CHURNED
```

Operational inactivity is not automatically churn.

A separate activity classification should distinguish:

- active;
- temporarily inactive;
- seasonally inactive;
- delinquent;
- suspended;
- closed.

This prevents seasonal businesses from corrupting retention analytics.

---

## 27. Support Economics

Support must be measured per segment.

```text
SUPPORT COST
=
tickets
+
agent time
+
chat/voice cost
+
travel
+
training
+
escalations
```

Duka economics require a high degree of self-service and low-touch support.

Enterprise economics can tolerate higher-touch service only when the account contribution covers it.

Support quality metrics:

- first response time;
- resolution time;
- repeat-contact rate;
- cost per active account;
- issue categories;
- product defect rate;
- self-service deflection;
- satisfaction signal.

---

## 28. Unit Economics

At account level:

```text
REVENUE
− PAYMENT FEES
− REFUNDS / CHARGEBACK LOSSES
− VARIABLE INFRASTRUCTURE
− SUPPORT
− ONBOARDING
− PARTNER COMMISSION
− VARIABLE COMPLIANCE / INTEGRATION COST
=
CONTRIBUTION
```

CAC is tracked separately but used for payback:

```text
CAC PAYBACK
=
CAC
/
MONTHLY CONTRIBUTION
```

A low-ARPU tier must not be scaled merely because gross revenue grows.

---

## 29. Duka Economics Gate

The Duka lane receives a hard economic gate because MK7,500/month is only a planning hypothesis.

Required evidence:

- collection method viability;
- payment success;
- support cost;
- activation;
- retention;
- CAC;
- channel commission;
- onboarding cost;
- variable infrastructure;
- net contribution;
- CAC payback.

The pass/fail threshold remains governed by `commercial_validation_plan.md`.

---

## 30. Defensibility

The moat is not assumed to be a feature.

Potential defensibility components:

```text
TRUST
+
DISTRIBUTION
+
WORKFLOW DEPTH
+
MERCHANT HISTORY
+
RECONCILIATION
+
INTEGRATIONS
+
NETWORK EFFECTS
+
SWITCHING COST
```

Each must be earned.

A copycat can reproduce screens quickly. It is harder to reproduce:

- trusted business history;
- embedded workflows;
- local distribution relationships;
- integration reliability;
- accumulated reconciliation context;
- merchant operating habits;
- low-friction support infrastructure.

The commercial objective is to make replacing Sitolo progressively harder because the customer receives increasing operational value, not because Sitolo traps data.

---

## 31. Intelligence Layer

The long-term product loop is:

```text
RECORD
  ↓
UNDERSTAND
  ↓
DETECT
  ↓
ALERT
  ↓
RECOMMEND
  ↓
ACT
```

Examples of useful derived signals:

- sales growth with margin compression;
- fast-moving item nearing stockout;
- unexplained cash variance;
- unusual void/refund activity;
- overdue supplier balance;
- declining customer repayment;
- abnormal purchase price movement.

Commercially valuable intelligence must be explainable and traceable to underlying business records.

The recommendation engine must never silently alter financial state.

---

## 32. Geographic Expansion

Expansion is evidence-gated.

The model is:

```text
MALAWI
  ↓
PROVE PRODUCT
  ↓
PROVE UNIT ECONOMICS
  ↓
PROVE DISTRIBUTION
  ↓
PROVE SUPPORT MODEL
  ↓
PROVE REGULATORY PLAYBOOK
  ↓
COUNTRY-BY-COUNTRY EXPANSION
```

Country expansion requires a documented local dossier covering:

- payments;
- tax;
- data protection;
- consumer law;
- business registration context;
- languages;
- mobile/device constraints;
- market pricing;
- distribution channels;
- support;
- FX/currency;
- data residency;
- regulatory partners.

Do not export Malawi assumptions as if they were African defaults.

---

## 33. Commercial Scorecard

Every monthly commercial review must show, by segment and channel:

### Demand

- leads;
- qualified leads;
- trial starts;
- acquisition cost.

### Activation

- first-value completion;
- time to first sale;
- first-week active rate.

### Monetization

- paid conversion;
- ARPU;
- collection success;
- failed payment rate.

### Retention

- 30/90/180-day retention;
- logo churn;
- revenue churn;
- seasonal inactivity;
- reactivation.

### Economics

- CAC;
- contribution;
- CAC payback;
- support cost/account;
- partner commissions.

### Product value

- sales recorded;
- reconciliation use;
- repeat usage;
- alerts acted upon;
- exports;
- multi-user usage.

### Trust

- account recovery;
- device replacement;
- sync failures;
- security incidents;
- support trust complaints.

---

## 34. Commercial Decision Gates

Do not scale a segment because a dashboard looks positive.

A segment must progress through:

```text
PROBLEM EVIDENCE
→ ACTIVATION
→ PAID USE
→ RETENTION
→ POSITIVE CONTRIBUTION
→ CAC PAYBACK
→ NON-FOUNDER DISTRIBUTION
→ EXPANSION
```

If a stage fails, the commercial response is to investigate the failed mechanism rather than hide it inside the blended portfolio.

---

## 35. Anti-Patterns

Reject these patterns:

1. “Every merchant is the same.”
2. One buyer persona for all tiers.
3. Card-only billing for low-ARPU merchants.
4. Founder-only sales treated as scalable distribution.
5. Revenue growth without contribution accounting.
6. Aggregate churn hiding segment failure.
7. Seasonal inactivity counted as churn.
8. Forced upgrades without business-complexity signals.
9. Vertical product forks.
10. Payment status inferred from client UI.
11. Cash collection without reconciliation/evidence.
12. Compliance claims without current legal/provider evidence.
13. Data lock-in presented as retention.
14. Features treated as moat without distribution/workflow adoption.
15. AI recommendations without traceable source data.

---

## 36. Research and Evidence Policy

Commercial assumptions are labelled as:

- **FACT** — supported by a current source or verified internal evidence.
- **HYPOTHESIS** — proposed but unvalidated.
- **EXPERIMENT** — currently being tested.
- **DECISION** — approved internal product/commercial decision.
- **DEPENDENCY** — requires external partner, legal, regulator or provider confirmation.

External market statistics must include:

- source;
- publication date;
- population/geography;
- measurement definition;
- limitation.

A dated legal or fiscal statement must not be generalized into an eternal product rule.

---

## 37. Implementation Traceability

This operating model implies future implementation requirements:

| Commercial requirement | System consequence |
|---|---|
| buyer/user separation | account actor metadata |
| subscription billing | entitlement/billing state |
| payment reconciliation | provider-independent payment events |
| channel attribution | acquisition-source identifiers |
| partner commissions | partner ledger/reporting |
| onboarding | activation instrumentation |
| trust | audit/recovery/export controls |
| offline continuity | sync/recovery telemetry |
| churn reasons | structured lifecycle events |
| seasonality | status model |
| fraud controls | audit + approval + analytics |
| tier migration | server-side entitlements |
| vertical modules | bounded module/configuration model |
| fiscal readiness | tax configuration/versioning |
| data portability | controlled export jobs |
| intelligence | derived read models and explainable signals |

These implications do not authorize implementation by themselves; each engineering change must follow the relevant technical contract.

---

## 38. Final Commercial Contract

Sitolo becomes commercially durable only when the following chain is true:

```text
THE RIGHT BUSINESS
        ↓
REACHABLE THROUGH A REPEATABLE CHANNEL
        ↓
CAN START WITH LOW FRICTION
        ↓
RECEIVES FIRST VALUE QUICKLY
        ↓
TRUSTS THE RECORD
        ↓
CAN PAY IN A LOCALLY VIABLE WAY
        ↓
REMAINS ACTIVE
        ↓
PRODUCES POSITIVE CONTRIBUTION
        ↓
EXPANDS WHEN COMPLEXITY INCREASES
        ↓
CREATES HARDER-TO-REPLICATE WORKFLOW VALUE
```

The central operating principle is:

> **Broad product scope; focused commercial execution; measured economics; explicit regulatory boundaries; earned defensibility.**

