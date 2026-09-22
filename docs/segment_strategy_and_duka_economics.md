
# Sitolo — Universal Segment Strategy, Duka Economics & Market Coverage Doctrine

**Document:** docs/segment_strategy_and_duka_economics.md  
**Status:** Canonical commercial-strategy supplement  
**Date:** 2026-09-22  
**Market:** Malawi first; African regionalization only after evidence and operational readiness  
**Covered market:** Duka → micro/small retail → growing SME → specialist SME → multi-branch → enterprise

This document is the commercial clarification that follows the 22 September 2026 business-model review. It converts the Duka rationale, universal-market thesis, segmented pricing model, low-touch economics, acquisition model, product packaging rules, validation implications, and architectural implications into repository documentation.

It supplements business_model_design.md and commercial_validation_plan.md. It does not replace law, regulator requirements, provider contracts, security architecture, domain invariants, database rules, API contracts, synchronization rules, deployment controls, or other engineering source-of-truth documents.

---

# 0. Executive Decision

Sitolo must **not** permanently narrow itself to large SMEs, growing retailers, urban shops, formal businesses, or any other single customer class.

The commercial doctrine is:

> **One Sitolo. One core platform. Duka to enterprise. Different levels of operational complexity, different entitlements, different acquisition motions, different support economics, and different willingness-to-pay — but one underlying business operating system.**

The key distinction is between:

1. product-market coverage;
2. go-to-market focus;
3. commercial validation cohorts;
4. packaging and entitlements;
5. customer-acquisition motion;
6. unit economics;
7. support model.

These must not be treated as one variable.

A platform can be broad while the learning program is deliberately segmented.

The intended long-term coverage is:

~~~
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

This does not require six products.

The correct implementation is:

~~~
ONE CORE
+
BUSINESS-TYPE CONFIGURATION
+
PLAN ENTITLEMENTS
+
PROGRESSIVE DISCLOSURE
+
SEGMENT-SPECIFIC GTM
+
SEGMENT-SPECIFIC ECONOMICS
~~~

The brutal commercial truth is:

> **Serving everyone is not the hard part. Serving everyone profitably, simply, and reliably is the hard part.**

The solution is therefore not to shrink the addressable market unnecessarily. The solution is to control complexity and cost-to-serve.

---

# 1. Why the Earlier Narrow-Wedge Interpretation Is Corrected

The preceding business model used an initial commercial-wedge concept centered on inventory-heavy small and growing retail.

That concept remains useful as a **learning cohort**, but it must not be interpreted as a permanent restriction on Sitolo's product scope.

The distinction is now explicit:

| Concept | Meaning |
|---|---|
| Market scope | Customers Sitolo is intentionally capable of serving over its product lifetime |
| Validation cohort | Customers grouped for a controlled commercial experiment |
| GTM focus | The customer group receiving a particular acquisition motion |
| Packaging | Capabilities and limits exposed in a plan |
| Business type | Workflow defaults and vertical configuration |
| Unit economics | Revenue and cost structure required for a sustainable segment |

Therefore:

> **A narrow validation cohort does not imply a narrow product.**

A Duka can be in scope while not being included in the same statistical cohort as an enterprise retailer.

A growing SME can be in scope while a Duka receives a completely different price, interface, support model, and acquisition channel.

An enterprise can be in scope while not becoming the dominant roadmap driver.

---

# 2. Market Evidence: Why Small Businesses Matter

## 2.1 Malawi has a very large micro-enterprise base

The 2019 Malawi FinScope MSME Survey estimated approximately 1,600,739 MSMEs in Malawi.

The reported size composition was approximately:

- 74% micro;
- 23% small;
- 3% medium.

The World Bank's 2022 discussion of the same survey describes these businesses as largely informal micro-enterprises and identifies limitations in business planning, bookkeeping, financial management, marketing, and employee management.

This evidence does not prove Sitolo demand. It does establish why a Malawi-first strategy cannot rationally assume that the addressable market consists mainly of formal, urban, multi-staff enterprises.

References:

- FinMark Trust, Malawi data portal and FinScope MSME Survey 2019: https://www.finmark.org.za/data-portal/MWI
- Malawi Ministry of Trade, FinScope MSME 2019 full report: https://www.trade.gov.mw/index.php/downloads/category/4-docs?download=6%3Afinscope-msme-2019-survey-full-report
- World Bank, Supporting Malawi's small enterprises to spur economic growth and create more job opportunities, 30 May 2022: https://blogs.worldbank.org/en/africacan/supporting-malawis-small-enterprises-spur-economic-growth-and-create-more-job

## 2.2 Rural and geographically distributed businesses matter

The 2019 FinScope data indicate approximately 1,253,379 MSMEs in rural areas and approximately 347,360 in urban areas.

That does not mean all rural enterprises are potential Sitolo customers. It means the national business population is geographically distributed enough that a commercial model built exclusively around dense urban sales clusters risks ignoring a major part of the market.

The Duka thesis therefore includes distribution density:

~~~
NEIGHBOURHOOD
     ↓
TRADING CENTRE
     ↓
DISTRICT
     ↓
REGION
     ↓
COUNTRY
~~~

A product intended for this environment must be usable without assuming constant connectivity, desktop access, a professional accountant, or a field-sales team.

## 2.3 Informality changes the entry product

The same FinScope survey estimated that approximately 89% of Malawi's MSMEs were neither registered nor licensed.

That is highly relevant to the Duka product.

A merchant may start with:

- one owner;
- one device;
- a small catalogue;
- cash-heavy operations;
- occasional mobile-money transactions;
- handwritten records;
- limited accounting discipline;
- limited administrative time.

The product must create useful value without demanding enterprise-grade administrative ceremony before the first sale.

This is a usability requirement, not a reason to weaken security.

---

# 3. Why Dukas Belong in Sitolo

Dukas are not included because they are a charitable market.

They are included because they represent a structurally large and distributed business population and because the core Sitolo problem exists at their scale:

> **The merchant can transact every day without possessing a reliable picture of what happened economically.**

The smallest shop may know the cash physically present in the drawer.

That does not necessarily answer:

- what stock was sold;
- what stock remains;
- what the stock cost;
- whether the recorded sales equal the money received;
- whether mobile-money transactions match expected payments;
- whether a pricing mistake erased margin;
- whether stock shrank;
- whether customer credit has been collected;
- what the business approximately earned from selling inventory.

The product is therefore not merely:

> Replace the notebook.

It is:

> **Turn daily transactions into a trustworthy and understandable business record.**

---

# 4. The Duka Information-Gap Hypothesis

The commercial hypothesis can be represented as:

~~~
MANUAL / MEMORY-BASED OPERATION
        ↓
FRAGMENTED INFORMATION
        ↓
LOW VISIBILITY
        ↓
UNDETECTED LEAKAGE / UNCERTAINTY
        ↓
POOR DAILY DECISIONS
~~~

Sitolo attempts:

~~~
SALE
  ↓
STOCK MOVEMENT
  ↓
EXPECTED PAYMENT
  ↓
OBSERVED PAYMENT
  ↓
RECONCILIATION
  ↓
ESTIMATED GROSS PROFIT
  ↓
DAILY DECISION
~~~

The economic value is the reduction of information friction.

---

# 5. Manual Records Are Not the Same as Business Intelligence

The Malawi MSME evidence should be interpreted carefully.

A MAP Malawi MSME diagnostic using the 2019 FinScope evidence reported that more than 80% of target-growth groups kept financial records, that 86% of those records were maintained manually, and that 21% used computerized systems.

Those figures are **target-group-specific** and must not be generalized to every Duka.

The strategic point is broader:

~~~
DATA RECORDED
does not automatically mean
DATA UNDERSTOOD
~~~

A merchant may maintain a notebook correctly and still spend significant time reconstructing:

- sales;
- purchases;
- stock;
- expenses;
- payment differences;
- profit estimates.

Sitolo's value can therefore exist even where some manual record keeping already exists.

Reference:

- MAP Malawi MSME Diagnostic: https://genesis.imgix.net/uploads/downloads/Malawi_Diagnostic-2020-05-22.pdf

---

# 6. One Product, Multiple Operating Experiences

One universal Sitolo does not mean every customer sees the same interface.

The product should expose a complexity ladder:

~~~
                    SITOLO
                       |
       +---------------+---------------+
       |               |               |
     DUKA             SME          ENTERPRISE
       |               |               |
   SIMPLICITY      OPERATIONS      GOVERNANCE
       |               |               |
   LOW ARPU          MID ARPU       HIGHER ACV
       |               |               |
   LOW TOUCH      MIXED TOUCH       HIGH TOUCH
~~~

The underlying platform remains shared.

The customer experience is progressively disclosed.

This is a standard platform strategy:

> **Hide complexity until complexity becomes useful.**

---

# 7. One Core Domain, Not Separate Products

Sitolo should not become:

~~~
Sitolo Duka
Sitolo Retail
Sitolo Pharmacy
Sitolo Agro
Sitolo Wholesale
Sitolo Enterprise
~~~

as independent applications with divergent business logic.

Instead:

~~~
COMMON CORE
+
VERTICAL MODULES
+
PLAN ENTITLEMENTS
+
CONFIGURATION
~~~

The shared core can include:

- identity;
- tenancy;
- organizations;
- branches;
- users;
- authorization;
- catalogue;
- pricing;
- purchasing;
- receiving;
- inventory;
- sales;
- payments;
- cash;
- reconciliation;
- audit;
- reports;
- subscriptions;
- integration boundaries.

Vertical workflows can add specialized capability without duplicating the security and persistence model.

---

# 8. Business Type vs Plan

These must remain separate concepts.

## Business type

Answers:

> What kind of work does this merchant normally perform?

Examples:

- Duka;
- general retail;
- pharmacy;
- agro-dealer;
- wholesale;
- restaurant;
- service business.

Business type should influence:

- default dashboard;
- terminology;
- workflow suggestions;
- catalogue fields;
- reports;
- onboarding templates.

## Plan

Answers:

> How much operational complexity and organizational capability does this customer need?

Plan may govern:

- users;
- devices;
- branches;
- warehouses;
- advanced reports;
- approvals;
- integrations;
- support;
- storage;
- enterprise controls;
- SLA;
- implementation services.

Therefore:

~~~
BUSINESS TYPE
→ workflow defaults

PLAN
→ capability depth

ROLE
→ authorization

REGULATORY CONTEXT
→ required compliance behavior
~~~

This prevents a small pharmacy from automatically becoming an enterprise customer merely because the domain is specialized.

---

# 9. Segment Ladder

## 9.1 Duka / ultra-micro retail

Typical characteristics:

- one location;
- one owner or very small team;
- very few devices;
- limited administrative time;
- high price sensitivity;
- cash and mobile money;
- basic inventory;
- simple purchasing;
- potentially weak formal bookkeeping.

Value:

- fast selling;
- stock visibility;
- cash;
- payment recording;
- daily summary;
- estimated gross profit;
- low-stock alerts;
- offline continuity.

Commercial model:

- very low entry price hypothesis;
- self-service first;
- low-touch support;
- low acquisition cost.

## 9.2 Micro / small retail

Characteristics:

- larger catalogue;
- some staff;
- higher transaction frequency;
- suppliers;
- purchasing;
- basic customer credit;
- richer reporting.

Value:

- inventory;
- suppliers;
- purchasing;
- cash control;
- reconciliation;
- staff permissions.

## 9.3 Growing SME

Characteristics:

- several users;
- more devices;
- larger stock;
- increasing operational structure;
- stronger reporting requirements.

Value:

- purchasing control;
- reconciliation;
- expenses;
- user governance;
- approvals;
- desktop back office.

## 9.4 Specialist SME

Pharmacy, agro-dealer and similar verticals can need domain-specific workflows.

Examples:

- expiry/batch tracking;
- quarantine;
- unit conversions;
- seasonal purchasing;
- stock aging.

Specialist capability must be modular and evidence-led.

## 9.5 Multi-branch

Needs:

- centralized catalogue;
- branch stock;
- inter-branch transfers;
- consolidated reporting;
- branch permissions;
- branch reconciliation.

## 9.6 Enterprise

Needs:

- organization governance;
- segregation of duties;
- advanced authorization;
- audit evidence;
- integrations;
- API contracts;
- dedicated support;
- implementation;
- SLA.

---

# 10. Same Business Loop, Different Depth

The underlying commercial loop remains:

~~~
PROCURE
→ RECEIVE
→ STOCK
→ PRICE
→ SELL
→ COLLECT
→ RECONCILE
→ REPORT
→ DECIDE
~~~

Duka experience:

~~~
SELL
STOCK
CASH
TODAY
~~~

Growing SME:

~~~
PROCURE
RECEIVE
STOCK
SELL
PAYMENTS
RECONCILE
REPORT
~~~

Enterprise:

~~~
PROCURE
RECEIVE
STOCK
SELL
PAYMENTS
RECONCILE
REPORT

+

APPROVALS
SEGREGATION OF DUTIES
BRANCH GOVERNANCE
INTEGRATIONS
AUDIT
~~~

Same chain. Different depth.

---

# 11. Duka Value Proposition

The Duka experience should answer a compact set of practical questions.

Candidate daily summary:

~~~
TODAY

Sales
Cash expected
Mobile money expected
Estimated gross profit
Items low in stock
Unreconciled amount
Credit outstanding
~~~

The exact dashboard remains a hypothesis and must be validated.

The product must avoid turning the first screen into an ERP.

---

# 12. The Duka "Business Truth" Promise

The strongest Duka promise is not "POS."

POS is increasingly commoditized.

A stronger promise is:

> **Know what sold, what remains, what you should have collected, what was actually collected, and what the business approximately earned from those sales.**

The core chain is:

~~~
SELL
  ↓
RECORD
  ↓
DEDUCT STOCK
  ↓
EXPECT PAYMENT
  ↓
OBSERVE PAYMENT
  ↓
RECONCILE
  ↓
ESTIMATE GROSS PROFIT
  ↓
REPORT EXCEPTIONS
~~~

The product creates value by making this sequence understandable.

---

# 13. Gross Profit Must Not Be Misreported as Net Profit

Where Sitolo has sales revenue and cost of goods sold:

~~~
GROSS PROFIT
=
SALES
−
COST OF GOODS SOLD
~~~

That is not automatically net profit.

Net profit requires appropriate expense capture and accounting treatment.

Therefore a lightweight Duka plan should generally prefer:

> **Estimated gross profit**

unless sufficient information is available to support a broader profit calculation.

This is not semantic nitpicking.

A product that claims "you made profit" from incomplete inputs can destroy the trust it is supposed to create.

---

# 14. Duka Economics: The Hard Constraint

The Duka segment can work only if the entire cost structure is controlled.

The target equation is:

~~~
LOW ARPU
+
LOW CAC
+
LOW ONBOARDING COST
+
LOW SUPPORT COST
+
LOW INFRASTRUCTURE COST
+
LOW CHURN
+
HIGH REPEAT USAGE
=
POSITIVE CONTRIBUTION
~~~

The dangerous model is:

~~~
LOW PRICE
+
FIELD SALES
+
MANUAL SETUP
+
MANUAL MIGRATION
+
HIGH SUPPORT
+
HIGH CHURN
=
NEGATIVE CONTRIBUTION
~~~

Cheap pricing without cheap delivery is not a strategy.

---

# 15. Duka Acquisition Must Be Different From Enterprise Acquisition

A merchant paying a very small monthly subscription cannot economically support an acquisition model that looks like a large enterprise deployment.

Duka acquisition candidates:

- merchant referrals;
- WhatsApp/community channels;
- local agents;
- accountants where relevant;
- distributors;
- wholesalers;
- merchant associations;
- device partnerships;
- self-service digital acquisition.

Potential sequence:

~~~
DISCOVER
→ SIGN UP
→ SELF-SERVE
→ FIRST SALE
→ PAY
→ RETAIN
→ REFER
~~~

Enterprise sequence is different:

~~~
LEAD
→ DISCOVERY
→ SECURITY / PROCUREMENT
→ PILOT
→ CONTRACT
→ IMPLEMENTATION
→ RENEWAL
~~~

The same brand can support both without using the same sales process.

---

# 16. Duka Distribution Density Hypothesis

A Duka exists close to another Duka.

This creates a possible local distribution advantage:

~~~
MERCHANT
   ↓
CUSTOMER / NEIGHBOUR
   ↓
REFERRAL
   ↓
NEARBY MERCHANT
   ↓
DISTRIBUTOR / ACCOUNTANT
   ↓
MORE MERCHANTS
~~~

This is only a hypothesis.

The company must measure:

- referral rate;
- referral conversion;
- cluster density;
- partner-generated leads;
- partner CAC;
- retention by channel;
- support burden by channel.

Do not call this a network effect until the data demonstrates it.

---

# 17. Duka Onboarding Doctrine

The first useful experience should not depend on:

- desktop setup;
- consultant involvement;
- complicated migration;
- full chart-of-accounts configuration;
- enterprise permissions;
- professional accounting knowledge.

Candidate path:

~~~
CREATE BUSINESS
  ↓
SELECT BUSINESS TYPE
  ↓
ADD A FEW PRODUCTS
  ↓
SET COST / PRICE
  ↓
MAKE FIRST SALE
  ↓
SEE STOCK
  ↓
SEE TODAY'S NUMBERS
~~~

The product should optimize time-to-first-value.

---

# 18. Data Migration Economics

Migration is a hidden commercial cost.

Use different migration modes:

### Start fresh

For merchants with poor records.

### Simple import

For merchants with a small spreadsheet or catalogue.

### Assisted import

For larger SMB accounts.

### Enterprise migration

For complex data sets requiring validation, reconciliation, mapping, rollback and audit evidence.

A 5,000-MWK plan must not silently inherit a 100,000-MWK onboarding process.

---

# 19. Support Economics

Support must be measured as a variable cost.

Track:

~~~
TICKETS / ACCOUNT
MINUTES / ACCOUNT
ESCALATIONS / ACCOUNT
SUPPORT COST / ACCOUNT
AUTOMATION RATE
TIME TO RESOLUTION
REOPEN RATE
~~~

Segment contribution should include support.

For a segment s:

~~~
CONTRIBUTION_s
=
REVENUE_s
− PAYMENT COST_s
− INFRASTRUCTURE_s
− SUPPORT_s
− PARTNER COST_s
− OTHER DIRECT VARIABLE COST_s
~~~

This is the correct lens for low-ARPU segments.

---

# 20. Infrastructure Economics for a Duka-Heavy Customer Base

Many small merchants can produce significant aggregate event volume.

The system must therefore optimize:

- low marginal cost;
- compact synchronization;
- efficient reads;
- bounded queues;
- safe offline operation;
- low storage overhead;
- connection-pool efficiency;
- automation.

Track:

~~~
INFRASTRUCTURE COST
-------------------
ACTIVE MERCHANTS
~~~

and:

~~~
INFRASTRUCTURE COST
-------------------
TRANSACTION VOLUME
~~~

A system that is cheap per enterprise but expensive per tiny merchant can still be commercially wrong.

---

# 21. Duka Plan Packaging

A conceptual Duka tier could include:

- one location;
- one or very few users;
- one primary device;
- products;
- simple POS;
- stock;
- cash;
- mobile-money recording;
- daily summary;
- estimated gross profit;
- low-stock alerts;
- basic offline operation;
- basic reports.

Advanced capabilities can remain outside the low-cost package:

- many users;
- many branches;
- advanced approvals;
- complex warehouse operations;
- advanced integrations;
- enterprise exports;
- premium support.

Exact inclusions remain hypotheses.

---

# 22. Growing SME Packaging

A growing SME plan can introduce:

- multiple users;
- multiple devices;
- suppliers;
- purchasing;
- advanced inventory;
- customer credit;
- payment reconciliation;
- expenses;
- desktop back office;
- management reports;
- richer permissions.

The merchant pays for greater organizational capability.

---

# 23. Enterprise Packaging

Enterprise value is not merely "more features."

It is:

- organizational governance;
- many users;
- many branches;
- approval workflows;
- audit evidence;
- integrations;
- security review;
- implementation;
- SLA;
- dedicated support.

Enterprise pricing should therefore include implementation and support economics rather than pretending the subscription is the entire contract value.

---

# 24. Same Security Baseline Across Every Plan

There is no "cheap security tier."

Every customer requires:

- tenant isolation;
- authentication;
- authorization;
- auditability;
- secure secret handling;
- encrypted transport;
- safe local storage;
- bounded synchronization;
- idempotency;
- validation;
- failure-safe behavior.

Commercial pricing must not reduce the security boundary.

---

# 25. Same Reliability Baseline Across Every Plan

The Duka should not receive:

~~~
BEST-EFFORT STOCK
~~~

while enterprise receives:

~~~
CORRECT STOCK
~~~

The invariant is shared.

The difference is:

- depth;
- scale;
- controls;
- support;
- reporting;
- integration;
- SLA.

This is critical to product trust.

---

# 26. Progressive Disclosure

The intended UX strategy is:

~~~
COMMON DOMAIN MODEL
        ↓
COMMON API
        ↓
COMMON SECURITY
        ↓
BUSINESS TYPE CONFIGURATION
        ↓
PLAN ENTITLEMENTS
        ↓
PROGRESSIVE DISCLOSURE
        ↓
SEGMENT EXPERIENCE
~~~

Examples:

Duka:

~~~
SELL
STOCK
CASH
TODAY
~~~

Growing SME:

~~~
SELL
STOCK
PURCHASING
SUPPLIERS
CASH
REPORTS
~~~

Enterprise:

~~~
OPERATIONS
BRANCHES
USERS
APPROVALS
RECONCILIATION
AUDIT
INTEGRATIONS
REPORTING
~~~

---

# 27. Current Market Pricing Signals in Malawi

Current public pricing observed on 22 September 2026 demonstrates that locally priced business software already spans materially different commercial tiers.

Observed public prices:

~~~
GulaSync
MWK 20,000 / month
MWK 35,000 / month
MWK 75,000 / month

MalondaPlus
MWK 25,000 / month
MWK 45,000 / month
MWK 90,000 / month
MWK 130,000 / month

Phindu
MWK 65,000 / month starting
~~~

These are vendor price signals, not demand benchmarks.

They establish only that a locally denominated, multi-tier price architecture already exists in the Malawi business-software market.

Sources:

- GulaSync: https://www.gulasync.com/pricing
- MalondaPlus: https://www.malondaplus.com/malondaplus/pricing.php
- Phindu: https://www.phindu.co/pricing

---

# 28. Pricing Doctrine

Sitolo must not set price by copying a competitor and subtracting a small percentage.

The correct sequence is:

~~~
CUSTOMER VALUE
      ↓
FREQUENCY OF VALUE
      ↓
COST TO SERVE
      ↓
ACCEPTABLE CONTRIBUTION
      ↓
PRICE TEST
      ↓
OBSERVED CONVERSION
      ↓
OBSERVED RETENTION
~~~

The exact Duka price remains an experiment.

The exact growing-SME price remains an experiment.

The exact enterprise ACV remains an experiment.

---

# 29. Price Should Follow Complexity

A useful conceptual rule is:

~~~
MORE USERS
+
MORE DEVICES
+
MORE BRANCHES
+
MORE CONTROL
+
MORE INTEGRATION
+
MORE SUPPORT
=
MORE COMMERCIAL VALUE / COST
~~~

This is more rational than:

~~~
SMALL BUSINESS = CHEAP
BIG BUSINESS = EXPENSIVE
~~~

because two small businesses can have very different workflow complexity.

---

# 30. Segment Portfolio Roles

| Segment | Revenue Role | Volume Role | Acquisition Role | Expansion Role | Main Economic Risk |
|---|---|---|---|---|---|
| Duka | low-ARPU recurring | potentially very high | referral/network potential | organic growth where applicable | support and churn |
| Small retail | core recurring | high | digital + partner | plan expansion | commodity pricing |
| Growing SME | higher recurring | medium | partner + sales | users/modules/branches | onboarding cost |
| Specialist SME | premium workflow value | lower | vertical channels | specialized modules | over-customization |
| Multi-branch | high-value expansion | low | direct + partner | branches/users | operational complexity |
| Enterprise | high ACV + services | low | direct | integrations/governance | long cycle/custom work |

This is a role map, not a ranking.

---

# 31. Duka Is Not a Charity Layer

The intended model is not:

~~~
BIG COMPANIES SUBSIDIZE DUKAS FOREVER
~~~

The intended hypothesis is:

~~~
DUKA
→ LOW CAC
→ LOW COST TO SERVE
→ RECURRING REVENUE
→ HIGH RETENTION
→ REFERRALS
→ VOLUME
~~~

while:

~~~
GROWING SME
→ HIGHER ARPU
→ MORE USERS
→ MORE MODULES
→ BRANCH EXPANSION
~~~

and:

~~~
ENTERPRISE
→ HIGH ACV
→ IMPLEMENTATION
→ INTEGRATIONS
→ GOVERNANCE
→ SLA
~~~

Different segments can contribute differently.

The company does not need every Duka to become an enterprise account.

---

# 32. Do Not Fake Land-and-Expand

Expansion is useful only when observed.

Valid path:

~~~
DUKA
│
├── remains Duka
│
├── grows
│   └── upgrades
│
└── closes
~~~

A Duka that remains on a profitable low-cost plan is a valid customer.

A large retailer can expand through:

- users;
- branches;
- integrations;
- support;
- governance.

Do not include hypothetical expansion in current LTV calculations.

---

# 33. Duka Retention Must Reflect the Duka Job

Duka retention should not depend on use of enterprise modules.

Candidate activity signals:

- sale recorded;
- stock updated;
- cash recorded;
- mobile-money transaction recorded;
- daily summary viewed;
- low-stock action taken;
- purchase recorded;
- reconciliation performed.

The exact retention definition must remain experimentally validated.

The rule is:

> **Measure the customer's actual job, not feature count.**

---

# 34. Duka Activation

Candidate activation:

> A merchant completes a real sale and a meaningful stock state update within the activation window, with durable acceptance and a valid tenant/business context.

Candidate secondary signals:

- time to first sale;
- time to first stock record;
- time to first daily summary;
- time to first reconciliation.

Tutorial completion is not sufficient evidence of activation.

---

# 35. Duka Daily Habit

Potential habit loop:

~~~
OPEN SHOP
→ SELL
→ RECORD
→ CHECK STOCK
→ CHECK MONEY
→ CLOSE DAY
~~~

A daily summary can make the value visible:

~~~
TODAY
Sales: ...
Cost of goods: ...
Estimated gross profit: ...
Cash expected: ...
Mobile money expected: ...
Unreconciled: ...
Low stock: ...
~~~

The exact contents must be tested.

---

# 36. Duka Fraud / Shrinkage Value

Sitolo can identify discrepancies.

Potential controls:

~~~
EXPECTED STOCK
vs
OBSERVED STOCK
~~~

~~~
EXPECTED CASH
vs
OBSERVED CASH
~~~

~~~
EXPECTED PAYMENT
vs
OBSERVED PAYMENT
~~~

The product must not claim that a discrepancy proves theft.

It identifies an exception that needs investigation.

This creates value for owners with staff while preserving factual neutrality.

---

# 37. Staff Security Exists Even in a Tiny Shop

A Duka can have assistants or multiple devices.

The system therefore still requires:

- identity;
- role assignment;
- action attribution;
- audit;
- controlled corrections;
- device awareness.

The phrase "small business" must never become an excuse for weak account controls.

---

# 38. Inventory Is a Strong Small-Merchant Problem

Small merchants often have constrained working capital.

Important workflows include:

- purchasing;
- receiving;
- selling;
- stock adjustment with reason;
- stock count;
- low-stock visibility;
- stock value estimation.

The UI should expose only the necessary level of warehouse complexity.

---

# 39. Replenishment Opportunity

Potential workflow:

~~~
LOW STOCK
→ REPLENISHMENT NEED
→ PURCHASE
→ RECEIVE
→ STOCK UPDATE
~~~

This can reduce:

- stockouts;
- over-buying;
- forgotten purchases;
- supplier confusion.

Build and monetize deeper workflows only after evidence.

---

# 40. Customer Credit

Simple customer-credit workflows can be useful.

They should remain:

- recording tools;
- collection visibility;
- receivable controls.

They must not become:

- loan underwriting;
- consumer lending;
- disguised fintech.

Sitolo records the commercial relationship; it does not need to become the lender.

---

# 41. Why Larger Shops Still Matter

The large shop has a different problem.

Potential issues:

- branch fragmentation;
- staff permissions;
- purchasing complexity;
- stock transfers;
- supplier controls;
- payment reconciliation;
- audit;
- management reporting;
- integrations.

The small shop asks:

> "Did I actually make money today?"

The large business asks:

> "Can headquarters prove what each branch, user, channel, supplier and payment did?"

Sitolo can serve both because the operating chain is the same.

---

# 42. Mobile-First Is Commercially Rational

Current Malawi digital evidence shows broad cellular connectivity while internet usage is much lower than the number of mobile connections. SIVIO's Lilongwe-focused MSME study also reported high mobile-money access among its sampled businesses.

The SIVIO study is geographically limited and is not a national census.

These signals support testing a mobile-first operating product.

References:

- DataReportal, Digital 2026 Malawi: https://datareportal.com/reports/digital-2026-malawi
- SIVIO Institute, Digital Financial Inclusion of MSMEs in Malawi: https://connect.sivioinstitute.org/2024/04/30/report-publication-digital-financial-inclusion-in-malawi/
- FinMark Trust Malawi financial-inclusion publications: https://www.finmark.org.za/knowledge-hub/publications

---

# 43. Offline Is a Commercial Capability

In a small shop, connectivity failure can directly break the business workflow.

Without offline continuity:

~~~
NETWORK FAILS
→ SALE FAILS
→ MERCHANT BYPASSES SYSTEM
→ BUSINESS RECORD BECOMES INCOMPLETE
~~~

With safe offline continuity:

~~~
NETWORK FAILS
→ LOCAL DURABLE COMMAND
→ CONTINUE WORK
→ LATER SYNC
→ SERVER VALIDATION
→ RECONCILIATION
~~~

GSMA's January 2026 merchant-payments research states that cash continues to dominate everyday transactions particularly among micro and small merchants and identifies affordability, usability, trust and infrastructure as adoption barriers.

Reference:

- GSMA, What Will It Take to Scale Merchant Payments?, 20 January 2026: https://www.gsma.com/solutions-and-impact/connectivity-for-good/mobile-for-development/gsma_resources/what-will-it-take-to-scale-merchant-payments-harnessing-the-digital-opportunity/

Offline is therefore part of the commercial value proposition, not merely an engineering trick.

---

# 44. Africa-Specific Commercial Principle

The commercial model must not assume that an African merchant is simply a smaller American SaaS customer.

Relevant structural conditions can include:

- large informal-business populations;
- cash-heavy commerce;
- uneven digital-payment adoption;
- mobile-first access;
- intermittent networks;
- low absolute software budgets;
- variable business formality;
- family-operated businesses;
- distributed geography;
- heterogeneous regulations.

The exact conditions vary by country.

The correct strategy is therefore local evidence plus reusable platform architecture.

---

# 45. Local Payment Relevance

A locally adapted billing experience matters.

Phindu publicly presents payment options including Airtel Money and TNM Mpamba, illustrating a local market pattern in which business software can be paid for in familiar local channels.

This is only a market signal, not proof that Sitolo must copy the implementation.

Sitolo billing must remain subject to payment-provider terms, security, reconciliation, and legal requirements.

Reference:

- Phindu pricing: https://www.phindu.co/pricing

---

# 46. Specialist Verticals Must Not Fork the Platform

A pharmacy or agro-dealer may require specialist data and workflows.

The architecture should become:

~~~
COMMON CORE
+
PHARMACY MODULE
+
AGRO MODULE
+
WHOLESALE MODULE
~~~

not:

~~~
COMMON CORE
+
THREE DIVERGENT SYSTEMS
~~~

Shared:

- tenant model;
- identity;
- authorization;
- audit;
- payments;
- synchronization;
- business records.

Specialized:

- domain workflows;
- fields;
- reports;
- terminology;
- product rules.

---

# 47. Formalization Is a Journey

A Duka may become more formal over time.

Potential sequence:

~~~
BASIC BUSINESS RECORD
      ↓
BETTER STOCK CONTROL
      ↓
BETTER CASH / PAYMENT CONTROL
      ↓
BETTER MANAGEMENT RECORD
      ↓
FORMAL REPORTING
      ↓
TAX / EIS CAPABILITY
~~~

Sitolo must not claim that the software itself formalizes a business.

It can provide systems that help a business maintain better records and potentially become more administratively mature.

---

# 48. EIS Is Not the Universal Purchase Driver

MRA EIS may be highly relevant to:

- formal businesses;
- businesses becoming formal;
- businesses with explicit invoicing requirements;
- enterprise customers.

It may be irrelevant to a basic Duka at the beginning.

Therefore:

~~~
DUKA
→ OPERATING VALUE FIRST

FORMALIZING SME
→ OPERATING + COMPLIANCE VALUE

ENTERPRISE
→ OPERATING + COMPLIANCE + INTEGRATION + GOVERNANCE
~~~

All MRA certification and compliance claims must be backed by current official evidence before external publication.

---

# 49. Financial History as a Potential Moat

The potential moat is not the fact that Sitolo has POS.

The potential moat is the accumulation of:

~~~
TRUSTED BUSINESS HISTORY
+
WORKFLOW DEPTH
+
RECONCILIATION HISTORY
+
USER / BRANCH CONTROLS
+
LOCAL INTEGRATIONS
+
OFFLINE RELIABILITY
+
DISTRIBUTION
~~~

A Duka can accumulate years of business history.

A growing SME can add suppliers, staff, reports and reconciliation.

A multi-branch customer can add branch governance.

The shared core creates continuity.

The company must still protect customer data, portability, legal rights and trust.

---

# 50. Data Portability Is a Trust Requirement

Sitolo must not manufacture lock-in by:

- abusive export fees;
- deliberately opaque formats;
- refusal of reasonable exports;
- security theater around legitimate customer portability.

A merchant should believe:

> "My business record is safe with Sitolo."

not:

> "Sitolo owns my business history."

Trust is itself a retention asset.

---

# 51. Segment-Aware Commercial Validation

The 12 existing experiments remain useful.

The correction is in cohort interpretation.

Every experiment must preserve:

~~~
SEGMENT
BUSINESS TYPE
SIZE BAND
PLAN
PRICE
CHANNEL
ONBOARDING VARIANT
DEVICE MIX
MOBILE / DESKTOP MIX
WORKFLOW DEPTH
~~~

This allows:

> "This price worked for Dukas"

instead of the invalid:

> "Customers are willing to pay this price."

---

# 52. Parallel Validation Lanes

The commercial-validation program should be designed around several lanes.

## Lane A — Duka

Test:

- low-price willingness to pay;
- free vs trial vs low-cost paid;
- mobile-only activation;
- daily business-truth value;
- referral economics;
- support economics.

## Lane B — Small and growing SME

Test:

- reconciliation;
- multi-user;
- desktop value;
- inventory depth;
- purchasing;
- branch expansion;
- accountant/distributor acquisition.

## Lane C — Enterprise

Test:

- governance;
- security readiness;
- implementation economics;
- integration demand;
- SLA willingness;
- enterprise conversion.

These lanes remain related but statistically interpretable.

---

# 53. CV-01 Free vs Trial — Segment Interpretation

Free vs trial should not be treated as one universal commercial answer.

A free model could help Duka acquisition.

It could also create:

- non-paying users;
- support load;
- storage cost;
- weak urgency.

A trial may reduce indefinite free usage.

It can also reduce adoption among highly price-sensitive merchants.

Test actual behavior.

Primary metrics:

- activation;
- paid conversion;
- retention;
- contribution;
- support cost.

---

# 54. CV-02 Core Price Sensitivity — Segment Interpretation

The previously defined price cells are experimental cells.

They must not become universal tariff assumptions.

A price that is economically rational for one customer class may be irrational for another.

Preserve:

- segment;
- size band;
- price;
- channel;
- support;
- onboarding.

The outcome should be segment-specific.

---

# 55. CV-03 Reconciliation WTP — Segment Interpretation

Reconciliation can mean different things.

Duka:

- simple cash/mobile-money matching.

Growing SME:

- multiple payment channels;
- user accountability;
- more sophisticated exceptions.

Enterprise:

- branch/provider consolidation;
- auditable resolution;
- governance.

Measure willingness to pay and retention impact separately.

---

# 56. CV-04 Accountant Referrals

Accountants can be a multiplier where one professional serves multiple small businesses.

But not every Duka has an accountant.

Therefore measure:

~~~
ACCOUNTANT
→ QUALIFIED LEADS
→ ACTIVATED ACCOUNTS
→ PAID ACCOUNTS
→ RETAINED ACCOUNTS
~~~

The important number is not lead volume.

It is contribution after partner acquisition cost.

---

# 57. CV-05 Distributor Acquisition

Distributors can potentially reach dense merchant networks.

Include:

- commission;
- activation;
- onboarding;
- support;
- retention;
- channel concentration.

A distributor that generates many low-quality leads is not a sustainable channel.

---

# 58. CV-06 Business-Type Onboarding

This experiment becomes more important under the universal-product doctrine.

Hypothesis:

> Business-specific defaults can reduce time-to-value without creating maintenance and complexity costs that exceed the commercial benefit.

Test:

- Duka;
- retail;
- pharmacy;
- agro;
- wholesale;
- other relevant cohorts.

The business-type layer should remain configuration-driven.

---

# 59. CV-07 Mobile-Only Retention

Dukas are a critical cohort for this test.

Question:

> Can the lowest-complexity customer operate successfully without desktop?

If yes, the product avoids imposing unnecessary hardware or workflow costs.

Measure:

- activation;
- retention;
- task completion;
- support;
- contribution.

---

# 60. CV-08 Mobile + Desktop Value

Desktop should be a progression trigger.

Test whether introducing desktop for growing complexity improves:

- retention;
- expansion;
- reporting use;
- operational control.

Do not assume every merchant needs desktop immediately.

---

# 61. CV-09 Branch Expansion

Branch expansion is irrelevant to many Dukas.

It becomes relevant when a customer actually acquires multiple locations.

Therefore:

> Measure branch expansion among accounts with genuine expansion triggers.

Do not use branch conversion as a universal product metric.

---

# 62. CV-10 EIS Purchase Driver

EIS matters differently by formalization stage.

Do not position it as the universal Sitolo purchase driver.

Use operating value as the base proposition.

Use EIS as a verified compliance/integration capability where relevant.

---

# 63. CV-11 Premium Support

Premium support is naturally more valuable to:

- growing SMEs;
- multi-branch businesses;
- enterprise customers.

For Dukas, a high-cost support package could destroy the economics.

Test attach rate against incremental support cost.

---

# 64. CV-12 Enterprise Pilot

Enterprise pilots remain important but must not dominate the entire strategy.

Enterprise evidence should measure:

- paid pilots;
- contract value;
- implementation contribution;
- sales cycle;
- customization burden;
- renewal/expansion evidence.

The enterprise roadmap must not become a custom-development roadmap.

---

# 65. Segment-Specific PMF

There must not be one blended PMF statement that hides divergent economics.

## Duka PMF candidate

Requires evidence of:

- fast activation;
- frequent use;
- low churn;
- low support burden;
- low CAC;
- positive contribution;
- real willingness to pay;
- potential referrals.

## Growing-SME PMF candidate

Adds:

- sustained multi-user use;
- purchasing;
- reconciliation;
- reporting;
- expansion.

## Enterprise PMF candidate

Adds:

- procurement acceptance;
- security acceptance;
- implementation economics;
- recurring contracted value;
- renewal/expansion.

These are different evidence requirements.

---

# 66. Segment-Level North Star

The overall North Star candidate remains:

> **Verified business activity per active merchant.**

The definition should vary by segment.

Duka:

~~~
VERIFIED SALES
+
STOCK MOVEMENTS
+
CASH / PAYMENT RECORDS
+
RECONCILIATION
~~~

Growing SME:

~~~
ABOVE
+
PURCHASING
+
SUPPLIER ACTIVITY
+
STAFF WORKFLOWS
+
REPORTING
~~~

Enterprise:

~~~
ABOVE
+
BRANCH ACTIVITY
+
APPROVALS
+
AUDITABLE CONTROL EVENTS
+
INTEGRATION ACTIVITY
~~~

The metric should represent business value, not feature consumption.

---

# 67. Segment P&L

Eventually the company should maintain segment contribution reporting.

For segment s:

~~~
Revenue
− payment processing
− infrastructure
− support
− acquisition commissions
− onboarding
− variable operations
=
Segment Contribution
~~~

Then:

~~~
Segment Contribution
--------------------
Paying Accounts
=
Contribution / Account
~~~

This prevents:

- enterprise revenue masking broken Duka economics;
- Duka volume masking enterprise implementation losses.

---

# 68. Duka Economics Dashboard

Minimum instrumentation:

~~~
DUKA_ACCOUNTS
DUKA_PAID_ACCOUNTS
DUKA_ACTIVE_ACCOUNTS
DUKA_ARPU
DUKA_CAC
DUKA_CAC_PAYBACK
DUKA_GROSS_MARGIN
DUKA_CONTRIBUTION
DUKA_SUPPORT_MINUTES
DUKA_SUPPORT_COST
DUKA_ONBOARDING_COST
DUKA_DAY_7_ACTIVATION
DUKA_DAY_30_RETENTION
DUKA_DAY_90_RETENTION
DUKA_REFERRAL_RATE
DUKA_EXPANSION_RATE
DUKA_CHURN
DUKA_PAYMENT_FAILURE_RATE
DUKA_SYNC_FAILURE_RATE
DUKA_RECONCILIATION_ADOPTION
~~~

This instrumentation is required before claiming Duka economics are proven.

---

# 69. Segment Cohort Rules

Each cohort should preserve:

~~~
SEGMENT
BUSINESS TYPE
SIZE BAND
PLAN
PRICE CELL
CHANNEL
ONBOARDING
DEVICE TYPE
WORKFLOW DEPTH
PRODUCT VERSION
~~~

Do not mix:

- free and paid;
- Duka and enterprise;
- old and materially different product versions;
- different price cells;
- different acquisition channels,

without preserving the dimensions required for analysis.

---

# 70. Channel Economics by Segment

For channel c and segment s:

~~~
CAC(c,s)
=
Channel Spend(c,s)
/
New Paying Accounts(c,s)
~~~

And:

~~~
Contribution(c,s)
=
Revenue(c,s)
− Variable Costs(c,s)
~~~

And:

~~~
Payback(c,s)
=
CAC(c,s)
/
Monthly Contribution(c,s)
~~~

This is critical because one blended CAC can hide:

~~~
LOW-CAC DUKA
+
HIGH-CAC ENTERPRISE
=
MEANINGLESS AVERAGE
~~~

The same applies to retention and support.

---

# 71. Support Segmentation

## Duka

- self-service;
- in-app help;
- automated diagnostics;
- lightweight human escalation.

## Small/growing SME

- standard support;
- guided onboarding;
- business-hours escalation.

## Enterprise

- priority support;
- technical account management where justified;
- incident communication;
- SLA.

The service level should be tied to economics.

---

# 72. Partner Strategy

Potential partners:

- accountants;
- wholesalers;
- distributors;
- associations;
- payment ecosystem actors;
- device vendors;
- business-support organizations.

Evaluate:

~~~
LEAD VOLUME
QUALIFIED RATE
PAID CONVERSION
CAC
COMMISSION
ONBOARDING COST
RETENTION
SUPPORT COST
DEPENDENCY RISK
~~~

Partner channels are businesses in their own right.

---

# 73. Geographic Expansion Inside Malawi

A universal product supports a geographic path such as:

~~~
LILONGWE
→ OTHER URBAN CENTRES
→ DISTRICT TOWNS
→ TRADING CENTRES
→ RURAL CLUSTERS
~~~

The sequence should be driven by:

- lead density;
- channel efficiency;
- support burden;
- connectivity;
- payment availability;
- retention;
- partner leverage.

The fact that an area has many shops is not enough evidence by itself.

---

# 74. Rural Operations

Rural expansion should not assume:

- reliable broadband;
- easy field training;
- desktop computers;
- dense support teams.

Commercial design should test:

~~~
LOW BANDWIDTH
+
OFFLINE
+
LOW-END DEVICES
+
REMOTE SUPPORT
+
LOW COST TO SERVE
~~~

The rural commercial case remains a hypothesis until field evidence exists.

---

# 75. Regionalization

The one-platform model becomes more valuable when Sitolo expands beyond Malawi.

Shared core:

~~~
BUSINESS
CATALOGUE
STOCK
SALES
PAYMENTS
RECONCILIATION
CONTROL
REPORTING
~~~

Country-specific layers:

~~~
TAX
CURRENCY
PAYMENT PROVIDERS
INVOICING
IDENTITY
LOCAL REGULATORY RULES
~~~

This preserves the common product while accepting real national differences.

---

# 76. Feature Prioritization

A useful internal decision model:

~~~
PRIORITY
=
PAIN
× FREQUENCY
× ECONOMIC VALUE
× REACH
/
DELIVERY COST
~~~

Relevant modifiers can include:

- trust impact;
- security impact;
- operational risk;
- reusability.

The formula is a decision aid, not objective mathematical truth.

A low-cost feature serving 100,000 Dukas can be highly valuable.

A high-cost feature requested by one enterprise customer may not be.

---

# 77. Do Not Build Duka Feature Sprawl

Low price does not mean "everything for less."

The Duka should have:

~~~
FEWER FEATURES
+
HIGHER FREQUENCY
+
HIGHER CLARITY
+
HIGHER RELIABILITY
~~~

The smallest customer should experience a focused workflow.

---

# 78. Do Not Let Enterprise Pollute the Default Experience

Enterprise concepts should not become the default UI for everyone.

Avoid forcing Dukas to understand:

- organizational hierarchies;
- approval matrices;
- complex branch administration;
- integration configuration;
- enterprise audit exports.

The platform can support these capabilities without exposing them unnecessarily.

---

# 79. Feature Access Must Be Server-Enforced

UI hiding is not authorization.

The server must enforce entitlements for:

- users;
- branches;
- devices;
- advanced modules;
- integrations;
- storage;
- reports;
- enterprise capabilities.

The client may hide unavailable functions for usability.

The server remains the authority.

---

# 80. Product Roadmap Tagging

Major initiatives should be tagged:

~~~
DUKA
SMALL_RETAIL
GROWING_SME
SPECIALIST
MULTI_BRANCH
ENTERPRISE
CROSS_SEGMENT
~~~

Every initiative should answer:

- which segment needs it;
- what measurable problem it solves;
- what incremental cost it creates;
- what evidence supports it;
- whether it is reusable.

This makes custom enterprise work visible instead of allowing it to silently become universal product scope.

---

# 81. Duka Cost-to-Serve Engineering Requirements

To make low-ARPU pricing plausible, maximize:

- self-service;
- automated onboarding;
- automated diagnostics;
- mobile-first workflows;
- efficient synchronization;
- compact payloads;
- low support burden;
- low storage cost;
- low operational overhead.

Commercial viability and architecture are linked.

---

# 82. Enterprise Cost-to-Serve

Enterprise customers can justify:

- data migration;
- training;
- security review;
- integration;
- implementation;
- support;
- SLA.

But all costs must be included in enterprise contribution.

High revenue is not the same thing as high profit.

---

# 83. Segment-Level Margin

For segment s:

~~~
Gross Margin(s)
=
Revenue(s)
− Direct Cost(s)
------------------
Revenue(s)
~~~

For more operational accuracy:

~~~
Contribution Margin(s)
=
Revenue(s)
− Variable Cost(s)
− Support(s)
− Partner Cost(s)
--------------------------------
Revenue(s)
~~~

Duka decisions should be made from contribution, not from revenue.

---

# 84. Segment-Level LTV

Conceptually:

~~~
LTV(s)
=
ARPU(s)
× Gross Margin(s)
× Expected Lifetime(s)
~~~

But the production model should eventually incorporate:

- real retention curves;
- churn;
- support;
- payment costs;
- CAC;
- expansion;
- contraction;
- actual contribution.

Never manufacture lifetime from an arbitrary assumption.

---

# 85. Low ARPU Can Still Produce Strong Economics

A low monthly price is not automatically weak.

A Duka can be commercially attractive if:

~~~
LONG RETENTION
+
LOW SUPPORT
+
LOW CAC
+
LOW PAYMENT COST
+
REFERRAL VALUE
~~~

Likewise, high ARPU can be commercially weak if:

~~~
HIGH CAC
+
HIGH IMPLEMENTATION
+
HIGH SUPPORT
+
HIGH CUSTOMIZATION
+
SHORT CONTRACT
~~~

Revenue size is not sufficient for segment economics.

---

# 86. Universal-Scope Doctrine

The formal doctrine is:

> **Sitolo's addressable product scope extends from the smallest Duka and micro-retail merchant through growing SMEs, specialist businesses, multi-branch operators, and enterprise organizations.**

Product breadth is preserved.

Commercial focus is controlled by cohort.

Packaging is controlled by complexity.

Acquisition is controlled by economics.

Support is controlled by cost-to-serve.

Engineering remains one shared platform.

---

# 87. What "Serve Everyone" Does Not Mean

It does not mean:

- one dashboard;
- one price;
- one sales process;
- one support model;
- one onboarding flow;
- every feature shown immediately;
- every feature enabled;
- every customer requiring desktop;
- every customer requiring an implementation team.

It means:

> **One platform has a broad market scope with progressively deeper operational capability.**

---

# 88. What Does Not Change

Across all segments:

- tenant isolation;
- authorization;
- auditability;
- financial correctness;
- inventory correctness;
- idempotency;
- data integrity;
- recoverability;
- secure secrets;
- observability;
- safe synchronization.

Across segments, only the commercial and operational depth changes.

---

# 89. Anti-Commodity Test

Sitolo must not depend on:

> "We have a POS."

The stronger defensibility comes from:

- trusted business history;
- reconciliation;
- payment visibility;
- offline continuity;
- operational exceptions;
- local fit;
- branch controls;
- governance;
- accumulated workflow depth.

The product should be differentiated by the quality of the business record and the control loop, not merely by the number of buttons.

---

# 90. Duka Value Stack

~~~
LEVEL 1
RECORD SALES

LEVEL 2
KNOW STOCK

LEVEL 3
KNOW CASH

LEVEL 4
KNOW PAYMENT STATUS

LEVEL 5
KNOW ESTIMATED GROSS PROFIT

LEVEL 6
SEE EXCEPTIONS

LEVEL 7
IMPROVE DECISIONS
~~~

This is the commercial progression for the smallest customer.

---

# 91. Growing SME Value Stack

~~~
SALES
+
STOCK
+
PURCHASING
+
SUPPLIERS
+
CASH
+
PAYMENTS
+
RECONCILIATION
+
REPORTING
+
USER CONTROL
~~~

---

# 92. Enterprise Value Stack

~~~
EVERYTHING ABOVE
+
BRANCHES
+
APPROVALS
+
SEGREGATION OF DUTIES
+
AUDIT
+
API
+
INTEGRATIONS
+
SLA
+
IMPLEMENTATION
~~~

---

# 93. Strategic Flywheel

The universal platform can potentially create:

~~~
SIMPLE ENTRY
    ↓
REAL DAILY USE
    ↓
TRUSTED BUSINESS RECORD
    ↓
RETENTION
    ↓
REFERRAL
    ↓
MORE MERCHANTS
    ↓
BUSINESS GROWTH
    ↓
MORE COMPLEXITY
    ↓
PLAN EXPANSION
    ↓
HIGHER VALUE
    ↓
BETTER ECONOMICS
    ↓
BETTER PRODUCT
~~~

Not every merchant must complete the entire flywheel.

The flywheel is commercially valid only if sufficient accounts move through enough stages to create sustainable economics.

---

# 94. Strategic Risk: Duka Economics

Main Duka risks:

1. ARPU too low.
2. CAC too high.
3. Support too expensive.
4. Onboarding too expensive.
5. Churn too high.
6. Merchants record only a fraction of their activity.
7. Offline reliability is inadequate.
8. The product delivers accounting complexity rather than value.
9. Free users consume resources without conversion.
10. Referral assumptions fail.

Mitigations:

- self-service;
- progressive disclosure;
- local distribution;
- low-cost support;
- simple workflow;
- real behavioral measurement.

---

# 95. Strategic Risk: Enterprise Economics

Main enterprise risks:

1. long sales cycle;
2. large implementation effort;
3. expensive customization;
4. security review overhead;
5. support requirements;
6. roadmap capture by one customer;
7. low renewal after one-off project delivery.

Mitigations:

- implementation pricing;
- reusable modules;
- explicit customization boundary;
- contract minimums;
- enterprise contribution tracking;
- architecture discipline.

---

# 96. Strategic Risk: Universal Scope Becoming Feature Sprawl

The universal-market model becomes dangerous if interpreted as:

> Build everything for everyone immediately.

That is not the strategy.

The strategy is:

~~~
UNIVERSAL MARKET
+
CONTROLLED DELIVERY
+
CONTROLLED COHORTS
+
PROGRESSIVE COMPLEXITY
~~~

Product scope can be broad while current release scope remains narrow.

This distinction is essential.

---

# 97. Strategic Risk: Hiding Weakness in Aggregate Metrics

Do not report:

~~~
TOTAL MRR
TOTAL RETENTION
TOTAL CAC
~~~

without the ability to segment the result.

A strong enterprise cohort can hide weak Duka retention.

Large Duka volume can hide poor enterprise implementation economics.

Segment visibility is mandatory.

---

# 98. Commercial Evidence Hierarchy

Evidence should be interpreted in this order:

~~~
PAID REPEATED BEHAVIOR
↓
OBSERVED WORKFLOW CHANGE
↓
REPEATED PRODUCT USE
↓
PAID CONVERSION
↓
STATED WILLINGNESS TO PAY
↓
STATED PREFERENCE
↓
HYPOTHETICAL INTEREST
~~~

The lower the evidence, the weaker the commercial conclusion.

---

# 99. Research Limitations

Current external evidence has important boundaries.

### FinScope MSME 2019

Strong national baseline for the survey year, but not a 2026 census.

### World Bank 2022

Useful interpretation of the MSME structure and constraints, but based largely on the older survey baseline.

### MAP diagnostic

Useful for target-growth segments; not a universal Duka statistic.

### SIVIO 2024

Useful mobile-money signal, but geographically limited to sampled areas of Lilongwe.

### GSMA 2026

Current regional merchant-payment context; not a Malawi-only survey.

### Competitor pricing pages

Useful live market signals; not independent evidence of willingness to pay, market share, or product success.

### Sitolo experiments

The strongest source for Sitolo-specific commercial claims once executed correctly.

---

# 100. Commercial Decisions Now Formalized

The following are product/commercial decisions:

1. Sitolo market scope is Duka to enterprise.
2. Sitolo remains one platform and one brand.
3. Segment experiences are configuration and entitlement driven.
4. Pricing is differentiated by complexity and service.
5. Duka economics are a first-class workstream.
6. Duka acquisition should prioritize low-CAC channels.
7. Enterprise acquisition may remain high-touch.
8. Commercial validation is segment-aware.
9. Exact prices remain experiments.
10. Security and reliability baselines remain common across plans.
11. Expansion is measured, not assumed.
12. Enterprise custom work must not automatically become universal product scope.

---

# 101. Commercial Hypotheses That Remain Unproven

The following require evidence:

- exact Duka price;
- exact Duka free/trial model;
- exact Duka feature limits;
- exact Duka support level;
- Duka referral rate;
- Duka CAC;
- Duka support cost;
- Duka retention;
- Duka willingness to record all sales;
- reconciliation value for Dukas;
- accountant-channel efficiency;
- distributor-channel efficiency;
- rural acquisition economics;
- specialist-vertical willingness to pay;
- enterprise implementation economics;
- cross-segment expansion rates;
- global economics of one-platform progressive disclosure.

These are hypotheses, not established facts.

---

# 102. Product-Scale Taxonomy

Customer scale should be described using measurable dimensions, not only labels.

Potential variables:

~~~
BRANCH_COUNT
USER_COUNT
DEVICE_COUNT
SKU_COUNT
TRANSACTION_VOLUME
MONTHLY_SALES_VOLUME
PURCHASING_COMPLEXITY
PAYMENT_CHANNEL_COUNT
WAREHOUSE_COUNT
STAFF_COUNT
REGULATORY_COMPLEXITY
INTEGRATION_REQUIREMENTS
~~~

This supports flexible plans.

Two businesses with the same employee count can have radically different complexity.

---

# 103. Example Entitlement Decision

Conceptual server-side input:

~~~
PLAN
+
BUSINESS TYPE
+
ORGANIZATION
+
BRANCH COUNT
+
USER COUNT
+
FEATURE ENTITLEMENTS
+
CONTRACT
~~~

Potential result:

~~~
ALLOWED
DENIED
REQUIRES UPGRADE
REQUIRES APPROVAL
REQUIRES SUPPORT TIER
~~~

This is an entitlement system, not UI decoration.

---

# 104. Product Roadmap Allocation

Evidence-led allocation can look like:

~~~
DUKA
→ SELF-SERVICE
→ MOBILE
→ RELIABILITY
→ REFERRALS

GROWING SME
→ INVENTORY
→ RECONCILIATION
→ REPORTING
→ DESKTOP

ENTERPRISE
→ GOVERNANCE
→ API
→ INTEGRATIONS
→ SECURITY
→ SLA
~~~

Shared infrastructure is funded centrally.

Segment-specific features require evidence.

---

# 105. Commercial Doctrine for the First Year

The first-year objective should not be:

> Pick one permanent ICP and abandon everything else.

Instead:

~~~
PROVE
1.
DAILY OPERATIONAL VALUE

2.
AT LEAST ONE LOW-TOUCH
PROFITABLE SEGMENT

3.
A REPEATABLE
GROWING-SME MOTION

4.
ENTERPRISE SALES WITHOUT
ROADMAP CAPTURE

5.
ONE PLATFORM CAN SERVE
MULTIPLE COMPLEXITY LEVELS
WITHOUT DESTROYING SIMPLICITY
~~~

This is a stronger first-year standard for a universal platform.

---

# 106. Segment Portfolio States

Use:

### Explore

Insufficient evidence.

### Validate

Active commercial testing.

### Commercially viable

Contribution and retention evidence exists.

### Scale

Acquisition and cost-to-serve are repeatable.

### Maintain

Healthy but not a major current investment priority.

### Deprioritize

Current economics or evidence do not justify further investment at the current configuration.

These are operational portfolio states.

---

# 107. Country Expansion Gate

Country expansion remains gated by:

~~~
STABLE MALAWI RETENTION
+
REPEATABLE ACQUISITION
+
POSITIVE CORE SAAS CONTRIBUTION
+
MANAGEABLE SUPPORT
+
LOCAL PAYMENT REQUIREMENTS
+
LOCAL TAX REQUIREMENTS
+
VALIDATED LOCAL PRICING
+
VALIDATED LOCAL CHANNEL
~~~

Large theoretical African TAM does not override poor Malawi economics.

---

# 108. Financial Ecosystem Gate

Financial-partner expansion remains gated by:

~~~
VIABLE SAAS ECONOMICS
+
PROVEN DATA QUALITY
+
PROVEN AUDITABILITY
+
CLEAR DISCLOSURE
+
CLEAR LEGAL MODEL
+
SIGNED PARTNER CONTRACT
+
SECURITY REVIEW
+
EXPLICIT RISK OWNERSHIP
~~~

Sitolo must not make speculative fintech revenue part of the base SaaS business case.

---

# 109. Commercial Stop List

Do not build primarily because:

- a competitor has the feature;
- an investor wants a larger TAM story;
- a feature looks impressive;
- one enterprise customer demands it;
- speculative future finance revenue might exist;
- a feature increases vanity metrics.

Build when evidence supports:

~~~
CUSTOMER PAIN
+
ECONOMIC VALUE
+
RETENTION IMPACT
+
ACCEPTABLE DELIVERY COST
+
REUSABILITY
~~~

---

# 110. Final Strategic Model

The complete commercial architecture is:

~~~
                         SITOLO
                            |
                 BUSINESS OPERATING SYSTEM
                            |
       +--------------------+--------------------+
       |                    |                    |
     DUKA                 SME                ENTERPRISE
       |                    |                    |
   LOW TOUCH           MIXED TOUCH            HIGH TOUCH
       |                    |                    |
   LOW ARPU             MID ARPU              HIGH ACV
       |                    |                    |
 LOW CAC REQUIRED    BALANCED CAC          HIGH CAC TOLERANCE
       |                    |                    |
 SIMPLE UX           PROGRESSIVE UX         GOVERNED UX
       |                    |                    |
 MOBILE FIRST       MOBILE + DESKTOP       FULL OPERATING STACK
       |                    |                    |
       +--------------------+--------------------+
                            |
                    SAME TRUSTED CORE
                            |
                    SAME BUSINESS HISTORY
~~~

The business model is therefore not:

~~~
DUKA OR SME OR ENTERPRISE
~~~

It is:

~~~
DUKA
→ SME
→ COMPLEX SME
→ MULTI-BRANCH
→ ENTERPRISE

ALL ON ONE PLATFORM
~~~

with optional progression rather than forced progression.

---

# 111. Final Doctrine

The final position is:

> **Sitolo should offer one business operating system to the smallest meaningful merchant and the largest organization within its defined business domains.**

The company does not need to choose between Dukas and big shops.

It needs to choose:

- the right product complexity for each;
- the right price for each;
- the right acquisition cost for each;
- the right support burden for each;
- the right validation cohort for each.

The Duka is strategically important because the bottom of the market is large, distributed, underserved by sophisticated business software, and structurally sensitive to cost and complexity.

The enterprise is strategically important because organizational complexity creates higher willingness to pay for governance, integrations, support, and control.

Sitolo's opportunity is to connect both ends through a common platform.

The commercial equation is:

~~~
BROAD MARKET
+
SIMPLE ENTRY
+
PROGRESSIVE COMPLEXITY
+
SEGMENTED ECONOMICS
+
LOW-COST DUKA DISTRIBUTION
+
HIGH-VALUE ENTERPRISE CAPABILITY
+
COMMON TRUST MODEL
=
ONE SCALABLE BUSINESS OPERATING SYSTEM
~~~

The fundamental constraint is:

> **Never confuse broad market coverage with broad unmeasured spending.**

Serve the market.

Measure the segments.

Charge according to value and cost-to-serve.

Keep the platform unified.

Keep the trust boundary universal.

---

# 112. Acceptance Criteria

This document and the related commercial corpus are considered aligned when:

- the product scope explicitly includes Duka through enterprise;
- business_model_design.md does not imply a permanent narrow market restriction;
- commercial_validation_plan.md uses segment-aware cohorts;
- Duka economics are explicit;
- Duka acquisition is designed for low CAC;
- Duka support economics are measurable;
- enterprise implementation costs are measurable;
- one shared core is preserved;
- business type and plan are distinct;
- security and reliability baselines remain universal;
- exact prices remain hypotheses until validated;
- current external evidence is cited with limitations;
- segment-level metrics exist;
- aggregate metrics cannot hide broken segment economics;
- expansion is measured rather than assumed;
- country expansion remains evidence-gated;
- financial ecosystem expansion remains evidence-gated.

---

# 113. Canonical External References

1. FinMark Trust — Malawi data portal and FinScope MSME Survey 2019  
   https://www.finmark.org.za/data-portal/MWI

2. Malawi Ministry of Trade — FinScope MSME 2019 full report  
   https://www.trade.gov.mw/index.php/downloads/category/4-docs?download=6%3Afinscope-msme-2019-survey-full-report

3. World Bank — Supporting Malawi's small enterprises to spur economic growth and create more job opportunities  
   https://blogs.worldbank.org/en/africacan/supporting-malawis-small-enterprises-spur-economic-growth-and-create-more-job

4. MAP Malawi MSME Diagnostic  
   https://genesis.imgix.net/uploads/downloads/Malawi_Diagnostic-2020-05-22.pdf

5. SIVIO Institute — Digital Financial Inclusion of MSMEs in Malawi  
   https://connect.sivioinstitute.org/2024/04/30/report-publication-digital-financial-inclusion-in-malawi/

6. GSMA — What Will It Take to Scale Merchant Payments?  
   https://www.gsma.com/solutions-and-impact/connectivity-for-good/mobile-for-development/gsma_resources/what-will-it-take-to-scale-merchant-payments-harnessing-the-digital-opportunity/

7. IFC — Africa MSME context  
   https://www.ifc.org/en/pressroom/2024/ifc-and-c2fo-partner-to-enhance-financing-for-local-enterprises-in-africa

8. IFC — Venture Capital and the Rise of Africa's Tech Startups  
   https://www.ifc.org/en/insights-reports/2025/venture-capital-and-the-rise-of-africa-s-tech-startups

9. GulaSync pricing  
   https://www.gulasync.com/pricing

10. MalondaPlus pricing  
    https://www.malondaplus.com/malondaplus/pricing.php

11. Phindu pricing  
    https://www.phindu.co/pricing

**End of Universal Segment Strategy, Duka Economics & Market Coverage Doctrine.**
