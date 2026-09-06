# SITOLO
## Business Model Design — Enterprise Target

**Positioning:** Business Operating System for African SMEs

**Primary market:** Malawi first; Africa expansion only after economic and operational validation

**Primary clients:** Mobile application + desktop application; web is a supporting acquisition/administration surface, not the primary operating product

**Document status:** Enterprise business-model target; release scope is staged; all commercial assumptions remain hypotheses until validated

**Prepared:** 2026-09-03

---

# 0. Executive Decision

Sitolo is one product, one brand and one commercial system.

It combines:

- business operations;
- point of sale;
- inventory;
- purchasing;
- payments;
- reconciliation;
- cash control;
- reporting;
- compliance workflows;
- multi-user control;
- multi-branch management;
- enterprise integrations.

The merchant-facing story is simple:

> **Run the business. Know the stock. Know the money. Control the operation.**

The deeper strategic model is:

```text
                    SITOLO
       Business Operating System
                    |
       +------------+------------+
       |                         |
     OPERATE                   CONTROL
       |                         |
   POS / STOCK             CASH / PAYMENTS
   PURCHASING              RECONCILIATION
   CUSTOMERS               FINANCIAL CONTROL
       |                         |
       +------------+------------+
                    |
                  RECORD
                    |
              BUSINESS HISTORY
                    |
          +---------+---------+
          |                   |
       COMPLIANCE         INSIGHTS
          |                   |
         EIS              REPORTS / ALERTS
          |                   |
          +---------+---------+
                    |
                ENTERPRISE
                    |
            API / BRANCH / SLA
                    |
                ECOSYSTEM
                    |
          BANKS / MFIs / INSURERS
```

The economic sequence is deliberately conservative:

```text
SELL SOFTWARE
    ↓
RETAIN MERCHANTS
    ↓
EXPAND ACCOUNTS
    ↓
SELL ENTERPRISE CAPABILITIES
    ↓
SELL INTEGRATIONS / SERVICES
    ↓
ENABLE PARTNER FINANCIAL PRODUCTS
```

SaaS revenue must be capable of sustaining the company before the business assumes future fintech or data revenue.

---

# 1. Scope of This Document

This document describes how Sitolo creates, delivers and captures economic value.

It is intentionally separate from the future system architecture design.

It defines:

- customers;
- segments;
- jobs-to-be-done;
- value propositions;
- packaging;
- pricing logic;
- revenue streams;
- unit economics;
- customer acquisition;
- retention;
- expansion;
- partnerships;
- enterprise sales;
- operations;
- market expansion;
- competitive positioning;
- regulatory boundaries;
- business risks;
- strategic metrics;
- decision gates.

It does not prescribe:

- exact databases;
- exact services;
- exact APIs;
- exact infrastructure topology;
- exact source-code layout.

Those belong in the next design document.

---

# 2. Source Baseline and Strategic Context

The existing Sitolo specification defines a mobile-first, offline-capable, multi-tenant SME retail operating system with product/catalogue, purchasing, inventory, POS, cash, payments, reconciliation, reporting, tax integration boundaries, audit, subscriptions, enterprise administration and integrations.

The business model preserves that scope while changing the primary question from:

> “What features can Sitolo contain?”

to:

> “What economic value does each layer create, and how does the company capture part of that value without destroying adoption?”

The core business loop remains:

```text
PROCURE
   ↓
RECEIVE
   ↓
STOCK
   ↓
PRICE
   ↓
SELL
   ↓
COLLECT
   ↓
RECONCILE
   ↓
REPORT
   ↓
DECIDE
```

That loop is the commercial foundation.

---

# 3. Market Evidence Used in This Design

## 3.1 Malawi digital environment

DataReportal's Digital 2026 Malawi report states that 14.1 million cellular mobile connections were active in late 2025, equivalent to 63.0% of the population, while 4.02 million people used the internet and internet penetration was 18.0%. DataReportal also warns that mobile connections are not equivalent to unique people and may exceed population because individuals can have multiple connections.

Commercial implication:

> Mobile is a more natural base for the broadest Sitolo market than a browser-only product.

## 3.2 Mobile money

Malawi has a large digital-payment ecosystem and high mobile-money relevance. The precise transaction figures should always be refreshed against the latest Reserve Bank of Malawi publication before being used in investor material, commercial proposals or external marketing.

Commercial implication:

> Payment-channel visibility and reconciliation are valuable business controls, not cosmetic features.

## 3.3 MRA Electronic Invoicing System

The Malawi Revenue Authority provides EIS API documentation and developer resources. Its documentation describes POS/API integration, terminal onboarding/activation and inventory or invoice-related synchronization. MRA also maintains a certification process for POS developers and products.

Commercial implication:

> Tax integration can increase product value for formalizing SMEs and enterprise customers, but certification/compliance claims must be backed by actual current approvals.

## 3.4 Enterprise constraints

The World Bank's Malawi Enterprise Survey provides evidence on the business environment, including finance, infrastructure, government requirements and operational constraints. It is a formal enterprise survey and should not be treated as a direct census of ultra-micro shops.

Commercial implication:

> Sitolo should solve concrete operating problems rather than assume that all SMEs have identical financial maturity.

---

# 4. Core Commercial Thesis

The core thesis is:

> **A business operating system can acquire customers cheaply through operational utility and monetize them over time through increased control and complexity.**

The merchant may initially care only about:

```text
FAST SALES
```

but the company wants the relationship to grow toward:

```text
FAST SALES
+
TRUSTED STOCK
+
PAYMENT CONTROL
+
RECONCILIATION
+
BUSINESS FINANCE
+
BRANCH MANAGEMENT
+
COMPLIANCE
+
ENTERPRISE INTEGRATION
```

The commercial flywheel is:

```text
LOW FRICTION
    ↓
ADOPTION
    ↓
DAILY USAGE
    ↓
DEPENDENCY
    ↓
EXPANSION
    ↓
REVENUE
    ↓
BETTER PRODUCT
    ↓
LOWER FRICTION
```

---

# 5. The Category

## 5.1 Public category

**Business Operating System for African SMEs.**

## 5.2 Internal strategic category

**SME operating and financial-control infrastructure.**

## 5.3 What Sitolo is not

Sitolo is not positioned as:

- a simple cash register;
- a generic accounting package;
- a heavyweight ERP by default;
- a wallet;
- a bank;
- a lender;
- an insurer;
- a generic e-commerce marketplace.

Future connections to financial services can exist without changing the core category.

---

# 6. Jobs-to-be-Done

## Functional jobs

A merchant hires Sitolo to:

- record sales;
- track stock;
- receive stock;
- manage suppliers;
- manage cash;
- record payments;
- reconcile payment channels;
- understand performance;
- control staff;
- manage branches.

## Emotional jobs

The merchant wants to feel:

- in control;
- informed;
- confident;
- less exposed to staff mistakes;
- less dependent on memory.

## Social jobs

The merchant may want to look:

- professional;
- organized;
- credible to suppliers;
- credible to accountants;
- credible to financing partners.

---

# 7. The Primary Economic Problem

The fundamental customer problem is not lack of software.

It is fragmented business truth.

A merchant can have:

```text
SALES → NOTEBOOK
STOCK → MEMORY
CASH → DRAWER
MOBILE MONEY → PHONE / SMS
SUPPLIERS → PAPER
REPORTS → EXCEL
EXPENSES → RECEIPTS
```

Sitolo converts that fragmentation into:

```text
ONE BUSINESS RECORD
```

The economic value comes from reducing information friction.

---

# 8. Fragmentation Tax

Every disconnected system produces costs.

Call this the **fragmentation tax**.

Examples:

- re-entering data;
- reconciling spreadsheets;
- searching messages;
- counting stock repeatedly;
- resolving missing transactions;
- checking staff explanations;
- manually preparing reports.

Sitolo's commercial argument is that the subscription can be cheaper than the fragmentation tax.

---

# 9. Operational Leakage Model

Merchants can lose money through:

- unrecorded sales;
- pricing errors;
- inventory shrinkage;
- cash shortages;
- payment mismatches;
- duplicate refunds;
- stock damage;
- expired goods;
- purchasing errors.

A useful internal value model is:

```text
VALUE CREATED
=
LOSS PREVENTION
+
TIME SAVED
+
DECISION QUALITY
+
COMPLIANCE VALUE
+
GROWTH ENABLEMENT
```

The value need not be identical across segments.

---

# 10. Why “Know Your Money” Is Stronger Than “Use Our POS”

A POS reports transactions.

A finance-control system answers whether the transaction's economic consequences actually line up.

Example:

```text
SALE
MK 50,000
   ↓
EXPECTED PAYMENT
MK 50,000
   ↓
PROVIDER TRANSACTION
MK 50,000
   ↓
MATCH
YES
   ↓
SETTLED
YES
```

When mismatch occurs:

```text
SALE
MK 50,000
   ↓
EXPECTED
MK 50,000
   ↓
OBSERVED
MK 45,000
   ↓
EXCEPTION
MK 5,000
```

That exception has direct economic meaning.

---

# 11. Product Surfaces

## 11.1 Mobile application

Primary operational surface.

Used for:

- POS;
- quick stock checks;
- payment capture;
- cash operations;
- notifications;
- lightweight reporting.

## 11.2 Desktop application

Primary back-office control surface.

Used for:

- bulk catalogue management;
- purchasing;
- reconciliation;
- reporting;
- multi-branch management;
- staff administration;
- exports;
- integrations.

## 11.3 Website

Supporting surface only.

Used for:

- discovery;
- pricing;
- documentation;
- downloads;
- enterprise sales;
- status/help.

---

# 12. Mobile vs Desktop Value Model

```text
MOBILE
“RUN THE BUSINESS”

DESKTOP
“CONTROL THE BUSINESS”
```

A merchant does not need both on day one.

The system should allow:

```text
MOBILE ONLY
      ↓
MOBILE + DESKTOP
      ↓
MULTI-DEVICE
      ↓
MULTI-BRANCH
```

Commercial opportunity increases naturally as business complexity increases.

---

# 13. Business-Type Personalization

The customer should choose a business type during onboarding.

```text
WHAT KIND OF BUSINESS DO YOU RUN?

DUKA
RETAIL
PHARMACY
AGRO-DEALER
WHOLESALE
RESTAURANT
SERVICE
OTHER
```

The answer determines:

- default modules;
- default dashboard;
- product attributes;
- terminology;
- recommended workflows;
- reports;
- onboarding template.

It should not create eight independent products.

---

# 14. Business-Type Configuration Model

The business type is a **configuration and recommendation layer**.

Commercial plan is the **entitlement layer**.

The distinction is:

```text
BUSINESS TYPE
“What do you need by default?”

PLAN
“How much organizational capability do you need?”
```

This prevents product fragmentation and enables natural expansion.

---

# 15. Duka Segment

Primary needs:

- fast selling;
- simple catalogue;
- stock visibility;
- cash tracking;
- mobile-money recording;
- low learning curve.

Commercial position:

- low entry price;
- low-touch acquisition;
- high potential volume;
- careful support economics.

Key risk:

ARPU may be too low for high-touch support.

---

# 16. Small Retail Segment

Primary needs:

- multi-user;
- stock;
- suppliers;
- purchasing;
- payments;
- reports.

Commercial position:

> Core recurring-revenue segment.

---

# 17. Pharmacy Segment

Primary needs can include:

- controlled product records;
- batch/lot tracking;
- expiry;
- FEFO/FIFO-oriented workflows;
- stronger staff permissions;
- recall/quarantine support;
- auditability.

Commercial position:

> Higher operational complexity can support higher willingness to pay.

The platform must not claim that software alone constitutes legal pharmacy compliance.

---

# 18. Agro-Dealer Segment

Primary needs can include:

- units and conversions;
- lots/batches;
- expiry/use-by metadata where applicable;
- seasonal purchasing;
- supplier history;
- stock aging.

Commercial position:

> Strong fit for inventory-heavy workflows and supplier relationships.

---

# 19. Wholesale Segment

Primary needs:

- bulk pricing;
- sales orders;
- large inventory;
- warehouses;
- suppliers;
- receivables;
- multi-user access;
- branch management.

Commercial position:

> Higher ARPU potential due to complexity.

---

# 20. Multi-Branch Segment

Primary needs:

- branch consolidation;
- inter-branch transfers;
- centralized product catalogue;
- branch-level staff controls;
- consolidated reconciliation;
- branch profitability.

Commercial position:

> High-value expansion and enterprise entry point.

---

# 21. Customer Segment Pyramid

```text
                   ENTERPRISE
                       /\\
                      /  \\
                 MULTI-BRANCH
                    /      \\
               WHOLESALE / SME
                 /          \\
             RETAIL / PHARMACY / AGRO
                /                \\
             MICRO / DUKA
```

The bottom gives acquisition volume.

The middle gives recurring revenue.

The top gives high contract value.

---

# 22. Land-and-Expand Model

The expected customer journey is:

```text
ENTRY
POS + INVENTORY
    ↓
CORE
PAYMENTS + REPORTS
    ↓
CONTROL
RECONCILIATION + CASH
    ↓
GROWTH
USERS + BRANCHES
    ↓
ENTERPRISE
EIS + APIs + SLA
```

Each step must create measurable value.

---

# 23. Entry Product

The entry product should answer one question:

> “Can I run my normal day-to-day business with this?”

Minimum value:

- create products;
- sell;
- track stock;
- record payment;
- continue offline where supported;
- see today's numbers.

The goal is habit formation, not maximum monetization on the first day.

---

# 24. Core Product

The Core plan should answer:

> “Can I trust Sitolo with the business every day?”

Additional value:

- multiple staff;
- expenses;
- purchasing;
- richer reports;
- reconciliation;
- stronger support.

---

# 25. Business Plan

The Business plan should answer:

> “Can Sitolo control a growing operation?”

Capabilities may include:

- advanced reconciliation;
- approvals;
- more users;
- more devices;
- purchasing controls;
- advanced analytics;
- EIS where approved;
- customer/supplier accounts.

---

# 26. Growth / Multi-Branch Plan

The Growth plan should answer:

> “Can management control several locations?”

Capabilities:

- multi-branch;
- multi-warehouse;
- centralized reporting;
- branch transfers;
- consolidated reconciliation;
- advanced permissions;
- performance comparison.

---

# 27. Enterprise Plan

The Enterprise plan should answer:

> “Can Sitolo become part of our organizational control environment?”

Capabilities may include:

- advanced RBAC;
- segregation of duties;
- enterprise audit;
- API access;
- custom integrations;
- SLA;
- dedicated support;
- implementation services.

---

# 28. Packaging Doctrine

The packaging doctrine is:

> **Sell business outcomes, not arbitrary feature restrictions.**

Good packaging:

```text
MORE COMPLEX BUSINESS
→
MORE CONTROL
→
HIGHER PLAN
```

Bad packaging:

```text
COMMON FEATURE
→
LOCKED ONLY TO EXTRACT MONEY
```

---

# 29. Suggested Pricing Hypothesis

The following ranges are planning hypotheses only.

| Plan | Indicative range | Primary purpose |
|---|---:|---|
| Entry | MK0–10k/month | Adoption |
| Core | MK20k–40k/month | Recurring base |
| Business | MK50k–100k/month | Expansion |
| Growth | MK100k–200k+/month | Multi-branch |
| Enterprise | MK150k–500k+/month | High-value accounts |
| Custom | Negotiated | Large integrations / SLA |

The exact price must be validated with actual customers.

---

# 30. Price Based on Complexity

A more robust commercial model is:

```text
BASE PLATFORM
+
BRANCHES
+
USERS / DEVICES
+
ADVANCED FINANCE
+
INTEGRATIONS
+
SUPPORT LEVEL
```

This avoids arbitrary vertical pricing.

Two pharmacies can have radically different scale.

Price should reflect operational complexity.

---

# 31. Free Plan Strategy

The free plan exists to reduce adoption friction.

Possible boundaries:

- one location;
- one primary device;
- limited staff;
- basic reporting;
- no advanced reconciliation;
- no enterprise integrations.

The free plan must be economically bounded.

---

# 32. Trial Strategy

Trial design should create a complete value loop.

```text
INSTALL
 ↓
CREATE BUSINESS
 ↓
ADD PRODUCTS
 ↓
MAKE FIRST SALE
 ↓
MAKE REPEATED SALES
 ↓
VIEW REPORT
 ↓
RECONCILE
 ↓
PAY
```

The trial should not be treated as successful merely because a user opens the app.

---

# 33. Activation Definition

A customer is activated when the business has completed meaningful operational activity.

Preferred metric:

> **Repeated real-world transactions with stock and payment records successfully captured.**

A registration without transactions is not activation.

---

# 34. Time-to-Value

Target the shortest possible time between installation and first clear business benefit.

Potential TTV:

```text
ACCOUNT
→
PRODUCT
→
SALE
→
TODAY'S REPORT
```

The first “aha” moment should happen quickly.

---

# 35. Monetization Hierarchy

The recommended order is:

```text
1. SaaS subscription
2. Account expansion
3. Enterprise plans
4. Implementation / migration
5. Premium support
6. Integrations
7. Partner revenue
8. Future financial ecosystem revenue
```

Never reverse this order merely because fintech sounds larger.

---

# 36. Revenue Stream — SaaS

SaaS is the primary revenue engine.

Advantages:

- predictable revenue;
- clear customer relationship;
- direct monetization;
- recurring cash flow;
- easier forecasting.

SaaS should fund core engineering and operations.

---

# 37. Revenue Stream — Expansion

Expansion revenue comes from:

- additional branches;
- additional users/devices where justified;
- advanced reconciliation;
- enterprise controls;
- reporting;
- compliance modules;
- APIs.

The best expansion trigger is genuine business growth.

---

# 38. Revenue Stream — Enterprise

Enterprise revenue comes from:

- annual contracts;
- implementation;
- service levels;
- integrations;
- premium support;
- larger deployment scope.

Enterprise deals must meet a minimum contribution-margin threshold.

---

# 39. Revenue Stream — Implementation

Charge for real implementation work:

- migration;
- setup;
- branch rollout;
- configuration;
- training;
- enterprise integration.

Do not disguise broken onboarding as a services opportunity.

---

# 40. Revenue Stream — Support

Premium support can be packaged as:

```text
STANDARD
→ normal support

PRIORITY
→ faster support + onboarding

ENTERPRISE
→ SLA + escalation + account management
```

Support pricing must reflect actual human cost.

---

# 41. Revenue Stream — Integrations

Integration revenue may cover:

- accounting integrations;
- custom bank integrations;
- external ERP feeds;
- warehouse integrations;
- enterprise APIs.

Standard connectors should become productized over time.

---

# 42. Revenue Stream — EIS

EIS should create value through:

- compliant electronic invoicing workflows where approved;
- integration;
- reduced manual reporting burden;
- better record continuity.

Commercial packaging:

- included in upper plans;
- add-on for formal businesses;
- enterprise implementation.

---

# 43. Revenue Stream — Financial Partners

Future partner revenue may come from:

- bank referrals;
- MFI referrals;
- insurance referrals;
- partner technology fees;
- other lawful revenue-sharing models.

This revenue should remain supplementary until proven.

---

# 44. Revenue Stream — Data Products

Potential later products can use aggregated, lawful, appropriately governed data.

Possible outputs:

- category trends;
- demand patterns;
- aggregate inventory signals;
- regional sales trends.

Rules:

- no casual resale of raw merchant data;
- respect contractual rights;
- respect data protection requirements;
- prevent re-identification;
- make disclosures and consent decisions explicit where necessary.

---

# 45. Revenue Stream — Transaction Fees

Transaction fees should not be assumed.

They become interesting only if Sitolo provides a service that creates incremental payment value, such as:

- payment initiation;
- settlement orchestration;
- regulated-partner services.

A transaction occurring inside Sitolo is not automatically sufficient justification for charging a fee.

---

# 46. The “Do Not Become a Wallet” Rule

Recording and reconciling mobile-money transactions does not require Sitolo to hold customer funds.

The strategic sequence should be:

```text
RECORD
 ↓
RECONCILE
 ↓
UNDERSTAND
 ↓
PARTNER
```

not:

```text
RECORD
 ↓
BECOME A WALLET
```

unless there is a future, explicit legal and commercial case.

---

# 47. The “Do Not Become a Lender” Rule

Credit ledger capability is distinct from lending.

```text
MERCHANT CREDIT RECORD
≠
LENDING BUSINESS
```

Future lending should use licensed partners unless Sitolo itself becomes appropriately authorized.

---

# 48. Merchant Financial Profile

A mature Sitolo account can contain a structured business profile:

- sales history;
- payment mix;
- stock behavior;
- purchasing history;
- expenses;
- receivables;
- branch performance.

This profile may be valuable to the merchant first and partners second.

---

# 49. Why Financial History Is a Moat

The value grows over time.

```text
DAY 1
ONE SALE

MONTH 3
THOUSANDS OF EVENTS

YEAR 1
LONGITUDINAL PATTERN

YEAR 3
BUSINESS HISTORY
```

Historical data becomes useful because it explains current business behavior.

The moat is therefore **trusted history**, not simply data possession.

---

# 50. Reconciliation Product Value

Reconciliation provides:

- exception detection;
- settlement visibility;
- financial confidence;
- reduced manual work;
- auditability.

The key metric is not the number of matched payments.

It is the number and value of financial exceptions successfully explained or resolved.

---

# 51. Reconciliation Revenue Packaging

```text
ENTRY
Manual payment record

CORE
Basic reconciliation

BUSINESS
Automated matching + exceptions

ENTERPRISE
Multi-channel + multi-branch + API reconciliation
```

This allows the same engine to serve both small and large customers.

---

# 52. Reconciliation Matching Hierarchy

Commercial trust requires deterministic matching wherever possible.

Preferred order:

1. Provider transaction ID.
2. Provider reference.
3. Sitolo payment intent/reference.
4. Exact fallback rules.
5. Manual review.

A financially ambiguous transaction should never be silently confirmed.

---

# 53. Cash Management Value

Cash control is separate from sales.

Expected cash can be computed from:

```text
OPENING FLOAT
+
CASH SALES
+
CASH IN
-
CASH REFUNDS
-
CASH OUT
=
EXPECTED CASH
```

Compare expected cash to counted cash.

The variance is the control event.

---

# 54. Inventory Value

Inventory is economically significant because money is trapped in stock.

Sitolo can create value through:

- stock visibility;
- reorder controls;
- receiving accuracy;
- stock movement traceability;
- expiry control;
- shrinkage detection.

---

# 55. Purchasing Value

Purchasing becomes valuable when the system connects:

```text
SALES VELOCITY
+
CURRENT STOCK
+
SUPPLIER TERMS
+
PURCHASE HISTORY
```

to better buying decisions.

This creates a natural expansion into higher plans.

---

# 56. Customer Credit Value

Customer credit records can support:

- receivable visibility;
- due dates;
- aging;
- partial payments;
- collection reminders.

This does not automatically make Sitolo a lender.

---

# 57. Supplier Relationship Value

Supplier records can support:

- purchase history;
- price history;
- payment terms;
- product relationships;
- performance.

This increases switching value for mature customers.

---

# 58. Reporting Value

Reports should answer business questions.

Examples:

- What sold today?
- Which products move fastest?
- Which products are slow?
- What is my expected cash?
- Which payment is unresolved?
- Which branch performs best?
- What do customers owe?
- How much did I spend?

Avoid analytics theater.

---

# 59. Alert Value

Alerts should trigger action.

Examples:

```text
LOW STOCK
PAYMENT EXCEPTION
CASH VARIANCE
EXPIRING STOCK
OVERDUE RECEIVABLE
FAILED SYNC
UNUSUAL REFUND ACTIVITY
```

Advanced alerts are a premium-value opportunity.

---

# 60. Audit Value

Auditability creates commercial value in more complex businesses.

Questions answered:

- Who changed the price?
- Who approved the refund?
- Who adjusted stock?
- Who closed the register?
- Who accessed the branch?

Auditability is especially important in:

- pharmacies;
- multi-user environments;
- multi-branch businesses;
- finance workflows.

---

# 61. Approval Economics

Approval controls become valuable when the cost of error exceeds the cost of review.

Examples:

- high-value refund;
- large discount;
- stock write-off;
- supplier return;
- branch transfer.

This is a legitimate enterprise pricing driver.

---

# 62. Verticalization Without Fragmentation

The product should support:

```text
ONE CORE
+
MANY VERTICAL CONFIGURATIONS
```

not:

```text
PHARMACY PRODUCT
AGRO PRODUCT
RETAIL PRODUCT
WHOLESALE PRODUCT
```

as totally separate code/business systems.

---

# 63. Configuration Example

```text
Business Type = Pharmacy

Enabled by default:
- Batch
- Expiry
- FEFO workflow
- Controlled roles
- Recall states

Available later:
- Multi-branch
- Advanced reconciliation
- EIS
- API
```

The merchant remains inside the same Sitolo ecosystem.

---

# 64. Commercial Entitlement Model

Entitlements should describe:

- enabled capabilities;
- limits;
- quotas;
- service level.

Conceptually:

```text
PLAN
 ↓
ENTITLEMENTS
 ↓
MODULES
 ↓
LIMITS
 ↓
USAGE
```

This prevents arbitrary hard-coded pricing logic.

---

# 65. Branch Expansion Economics

A new branch creates:

- more users;
- more inventory;
- more transactions;
- more reconciliation;
- more management value.

It therefore represents legitimate expansion revenue.

---

# 66. User Expansion Economics

User count can be used as a pricing signal, but pure seat pricing may discourage proper access control.

A hybrid model is preferable:

```text
BASE PLAN
includes normal user count

SCALE
prices additional organizational complexity
```

---

# 67. Device Expansion Economics

Additional devices increase operational capacity.

Examples:

- second cashier;
- warehouse device;
- branch manager phone;
- desktop workstation.

Device pricing should remain predictable.

---

# 68. API Monetization

API access is primarily an enterprise value lever.

Potential pricing:

- included in enterprise;
- metered above a threshold;
- contracted integration package.

Avoid charging tiny businesses for normal internal API activity they never see.

---

# 69. SLA Monetization

SLAs should be sold where customers genuinely require:

- defined support response;
- incident escalation;
- operational commitments.

Do not promise enterprise SLAs before operational capability exists.

---

# 70. Enterprise Professional Services

Enterprise implementations may include:

```text
DISCOVERY
→
DATA MIGRATION
→
CONFIGURATION
→
INTEGRATION
→
TRAINING
→
ROLLOUT
```

Each stage should be scoped.

---

# 71. Enterprise Customization Policy

Customization must be divided into:

### Configuration

Supported without code change.

### Productized extension

Useful to many customers.

### Custom service

Useful to one customer and paid separately.

### Unsupported change

Not accepted.

This protects product integrity.

---

# 72. Customer Acquisition Strategy

Use a portfolio of channels.

```text
DIRECT
PARTNERS
REFERRALS
FIELD SALES
CONTENT
COMMUNITY
DIGITAL
ENTERPRISE SALES
```

The correct channel is the one producing retained customers, not merely cheap leads.

---

# 73. Founder-Led Sales

Early founder-led sales should be used to discover:

- pain;
- language;
- objections;
- price sensitivity;
- workflow variation.

Do not outsource learning before understanding the customer.

---

# 74. Accountant Channel

Accountants are a high-value channel because they already serve multiple businesses.

Potential structure:

```text
ACCOUNTANT
 ↓
CLIENT REFERRAL
 ↓
SITOLO
 ↓
CLEANER RECORDS
 ↓
BETTER ACCOUNTING SERVICE
```

Both parties benefit.

---

# 75. Distributor Channel

Wholesalers and distributors can introduce Sitolo to downstream retailers.

Potential models:

- referral;
- reseller;
- bundled subscription;
- distributor-sponsored merchant plan.

The merchant relationship must remain transparent.

---

# 76. Hardware Partner Channel

Hardware resellers can bundle:

- supported printer;
- scanner;
- cash drawer;
- Sitolo subscription.

Sitolo should avoid proprietary hardware lock-in unless the economics are compelling.

---

# 77. Merchant Referral Program

A referral program can reward:

- customer referrals;
- verified activation;
- paid conversion.

Reward based on economically meaningful outcomes, not merely installs.

---

# 78. Field Sales Economics

Field sales may work well where merchant density is high.

Measure:

- visits;
- demos;
- activations;
- paid conversions;
- retention;
- sales per representative.

A field rep generating low-retention customers is not productive.

---

# 79. WhatsApp as a Commercial Channel

WhatsApp can support:

- lead generation;
- product education;
- onboarding assistance;
- customer support;
- payment reminders.

The product itself should not become dependent on WhatsApp as the system of record.

---

# 80. Content Marketing

Content should solve real merchant problems.

Examples:

- mobile-money reconciliation;
- stock counting;
- cash controls;
- profit visibility;
- preparing for EIS workflows.

Useful content can create inbound demand.

---

# 81. Sales Messaging

Preferred hierarchy:

```text
RUN THE STORE
 ↓
KNOW YOUR STOCK
 ↓
KNOW YOUR MONEY
 ↓
CONTROL YOUR TEAM
 ↓
GROW YOUR BUSINESS
```

Avoid opening with enterprise jargon.

---

# 82. Enterprise Sales Messaging

For larger customers:

```text
CONNECT OPERATIONS
+
PAYMENTS
+
INVENTORY
+
FINANCE
+
COMPLIANCE
```

Enterprise buyers care about control and integration.

---

# 83. Sales Qualification

Qualify on:

- transaction volume;
- branch count;
- staff count;
- payment channels;
- stock complexity;
- current tools;
- reporting pain;
- integration needs;
- buyer authority.

---

# 84. Customer Lifecycle

```text
PROSPECT
 ↓
TRIAL
 ↓
ACTIVATED
 ↓
PAID
 ↓
RETAINED
 ↓
EXPANDED
 ↓
ADVOCATE
```

Every stage should have a metric.

---

# 85. Retention Theory

Customers stay because Sitolo becomes useful repeatedly.

Retention drivers:

- transaction history;
- stock history;
- payment history;
- reconciliation;
- staff workflows;
- branch operations;
- reporting.

The best retention is earned dependency.

---

# 86. Churn Drivers

Likely drivers include:

- poor reliability;
- incorrect records;
- inadequate onboarding;
- price/value mismatch;
- support problems;
- missing workflows;
- competitor displacement;
- business closure.

Every churn should be classified.

---

# 87. Churn Intelligence

Track:

```text
LOGO CHURN
REVENUE CHURN
REASON
SEGMENT
PLAN
TENURE
USAGE DEPTH
SUPPORT LOAD
```

Churn is a source of product strategy.

---

# 88. Expansion Revenue

Expansion comes from:

```text
MORE USERS
MORE BRANCHES
MORE MODULES
MORE CONTROL
MORE INTEGRATIONS
```

Expansion is preferable to forcing price increases on customers who are not receiving more value.

---

# 89. Net Revenue Retention

NRR should become a key mature-company metric.

Conceptually:

```text
STARTING REVENUE
+
EXPANSION
-
CONTRACTION
-
CHURN
=
ENDING REVENUE
```

A strong NRR means the installed base itself grows revenue.

---

# 90. Unit Economics

Core definitions:

```text
ARPU
Average Revenue Per User / Merchant

CAC
Customer Acquisition Cost

COGS
Variable Cost to Serve

LTV
Lifetime Value

GROSS MARGIN
Revenue minus direct service cost
```

Track these by segment.

---

# 91. Contribution Margin per Merchant

A useful internal equation:

```text
CONTRIBUTION
=
REVENUE
-
INFRASTRUCTURE
-
BILLING COST
-
VARIABLE SUPPORT
-
VARIABLE SALES / COMMISSIONS
```

A high-ARPU customer can still be unprofitable if cost-to-serve is high.

---

# 92. Illustrative Unit Economics

Scenario only:

```text
ARPU = MK45,000/month
Gross margin = 80%
Gross profit = MK36,000/month
Lifetime = 24 months
Gross-profit LTV ≈ MK864,000
CAC = MK250,000
LTV/CAC ≈ 3.46×
```

These are illustrative calculations, not forecasts.

---

# 93. CAC Payback

Approximate formula:

```text
CAC PAYBACK
=
CAC / MONTHLY GROSS PROFIT
```

Track actual payback by acquisition channel.

---

# 94. Free-Tier Economics

The free tier must be modeled as:

```text
COST OF FREE USER
vs
PROBABILITY OF FUTURE CONVERSION
×
EXPECTED LIFETIME VALUE
```

Free is not automatically good.

---

# 95. Support Economics

Track:

- tickets per merchant;
- minutes per ticket;
- escalation rate;
- repeat incident rate;
- cost per merchant.

Support should become increasingly self-service as the business scales.

---

# 96. Infrastructure Economics

Track:

- compute per active merchant;
- database cost per merchant;
- storage growth;
- synchronization traffic;
- report-generation cost.

Architecture decisions later must preserve acceptable unit economics.

---

# 97. Pricing Elasticity

Pricing should be treated as a measurable demand curve.

Test:

- willingness to pay;
- conversion at different prices;
- churn after price changes;
- plan migration;
- annual prepayment behavior.

---

# 98. Annual Billing

Annual billing can improve cash flow and retention.

Offer a reasonable discount where it improves customer commitment without destroying margin.

Enterprise should strongly consider annual contractual terms.

---

# 99. Payment Collection for Sitolo

Potential collection methods in Malawi include:

- Airtel Money;
- TNM Mpamba;
- bank transfer;
- card where appropriate.

The exact payment mix should be validated against the payment-provider agreements available to Sitolo.

---

# 100. Billing Risk Controls

The company itself needs reconciliation.

```text
CUSTOMER INVOICE
 ↓
PAYMENT EXPECTED
 ↓
PAYMENT OBSERVED
 ↓
RECONCILED
 ↓
SUBSCRIPTION ENTITLEMENT
```

Sitolo should apply its own product philosophy to its own revenue.

---

# 101. Pricing Transparency

Customers should understand:

- monthly price;
- annual price;
- included limits;
- taxes where applicable;
- add-ons;
- cancellation terms;
- data export.

No hidden fees.

---

# 102. Data Portability

Data portability is commercially healthy.

Allow appropriate exports for:

- sales;
- inventory;
- payments;
- reports;
- customers;
- suppliers.

Trust is a stronger retention mechanism than artificial lock-in.

---

# 103. Migration Strategy

The migration pipeline should be:

```text
IMPORT
 ↓
VALIDATE
 ↓
PREVIEW
 ↓
APPROVE
 ↓
COMMIT
```

Common sources:

- Excel;
- CSV;
- existing POS;
- paper lists.

---

# 104. Migration as Acquisition Strategy

A merchant who fears losing existing records may refuse to switch.

Therefore:

> Migration capability can directly reduce CAC and improve conversion.

Offer guided migration for higher-value customers.

---

# 105. Customer Education

Education should focus on outcomes.

Examples:

- how to reconcile money;
- how to count stock;
- how to manage staff;
- how to close a register;
- how to interpret reports.

Education improves activation and reduces support cost.

---

# 106. Customer Success Model

Customer success focuses on:

- activation;
- retention;
- expansion.

Support focuses on issue resolution.

These are related but not identical functions.

---

# 107. Merchant Health

A future health model could combine:

- usage;
- sales consistency;
- reconciliation state;
- stock state;
- overdue receivables;
- support activity.

Use it to identify churn risk and expansion opportunities.

---

# 108. Enterprise Account Management

Enterprise customers should receive:

- named owner;
- scheduled reviews;
- renewal planning;
- incident escalation;
- integration roadmap.

This justifies enterprise support pricing.

---

# 109. Renewal Strategy

Before renewal, show:

- usage;
- branch coverage;
- reconciliation value;
- support performance;
- reports used;
- expansion opportunities.

Renewal should be a business review, not merely an invoice.

---

# 110. Competitive Landscape

Current market research shows increasingly sophisticated African and Malawian products combining POS, inventory, offline operation, business reporting, mobile-money workflows and accounting/business management.

Examples include vendor offerings such as MalondaPlus, Phindu, Quick-Think, Stooqo, ninoPOS and other African SMB-management tools.

The strategic conclusion is critical:

> **POS + inventory + offline is not sufficient differentiation anymore.**

Sitolo must win on the integrated business-control loop.

---

# 111. Competitive Positioning

```text
BASIC POS
“SELL”

ERP
“MANAGE EVERYTHING”

SITOLO
“RUN + CONTROL THE BUSINESS”
```

Sitolo should occupy the middle ground between overly simplistic POS and heavyweight ERP.

---

# 112. Competitive Differentiation

Key differentiators should become:

1. African SME operational fit.
2. Mobile-first daily workflows.
3. Offline-capable operation.
4. Integrated payment visibility.
5. Reconciliation as a first-class business function.
6. Progressive enterprise controls.
7. Business-type configuration without product fragmentation.
8. Local-market integrations and support.

---

# 113. Offline Differentiation

Offline capability is commercially valuable only if it is reliable.

A useful promise is:

> **Keep selling when connectivity is unavailable; synchronize when it returns.**

Do not promise that external payment providers or tax services themselves remain available offline.

---

# 114. Low-End Device Strategy as Market Strategy

The target market may include lower-cost Android hardware.

Therefore:

- fast startup;
- bounded storage;
- efficient synchronization;
- low battery impact;
- lightweight assets

can become acquisition and retention advantages.

---

# 115. Brand Strategy

Brand attributes:

```text
TRUSTED
PRACTICAL
MODERN
AFRICAN
SERIOUS
SIMPLE
```

Avoid branding that sounds like:

- charity software;
- government-only product;
- giant ERP consulting project;
- speculative fintech.

---

# 116. Brand Architecture

One brand:

> **Sitolo**

Product surfaces:

- Sitolo Mobile;
- Sitolo Desktop;
- Sitolo Enterprise;
- Sitolo API where appropriate.

No second “Merchant Finance OS” brand is required initially.

---

# 117. Future Financial Platform Spin-Out

A separate financial infrastructure product should only emerge if external demand exists for:

- reconciliation without Sitolo POS;
- API-based payment normalization;
- bank/financial-institution integrations;
- third-party POS connectors.

At least two independent signals should exist before spin-out.

---

# 118. Ecosystem Strategy

Potential ecosystem participants:

```text
MERCHANTS
ACCOUNTANTS
SUPPLIERS
BANKS
MFIs
INSURERS
PAYMENT PROVIDERS
TAX SYSTEMS
HARDWARE VENDORS
```

The ecosystem should reinforce the merchant's core workflow.

---

# 119. Partner Strategy Sequence

Recommended order:

```text
DISTRIBUTION PARTNERS
 ↓
PAYMENT / TAX INTEGRATIONS
 ↓
ACCOUNTING PARTNERS
 ↓
ENTERPRISE PARTNERS
 ↓
FINANCIAL PARTNERS
```

This order minimizes premature complexity.

---

# 120. Payment Partner Strategy

Provider integrations require:

- API documentation;
- sandbox;
- authentication;
- webhook behavior;
- settlement behavior;
- commercial agreement;
- support pathway.

Provider-specific claims must be verified before committing the roadmap.

---

# 121. Accounting Partner Strategy

The accountant can become a force multiplier.

Potential value:

```text
ACCOUNTANT
 ↓
CLIENT ONBOARDING
 ↓
CLEANER DATA
 ↓
LOWER ACCOUNTING FRICTION
 ↓
MORE SIT0LO REFERRALS
```

---

# 122. Financial Institution Strategy

The eventual pitch should focus on structured business evidence, not inflated user counts.

Potential evidence:

- consistent sales;
- reconciled payments;
- business tenure;
- inventory behavior;
- cash-flow patterns.

Any partner access must be lawful, permissioned and auditable.

---

# 123. Insurance Partner Strategy

Potential partner products:

- stock cover;
- equipment cover;
- business interruption.

Sitolo should provide distribution/context while the licensed insurer controls underwriting.

---

# 124. Supplier Ecosystem Strategy

A future supplier ecosystem can create:

- better replenishment;
- demand visibility;
- promotional offers;
- supplier-financed stock through partners.

Merchant benefit must be primary.

---

# 125. Regulatory Boundary

The business must distinguish:

```text
FEATURE
vs
CERTIFICATION
vs
LICENSE
vs
LEGAL COMPLIANCE
```

A feature does not automatically create regulatory status.

---

# 126. Data Protection Business Model

Because Sitolo processes business and potentially personal information, the business must treat privacy as a foundational trust requirement.

Key principles:

- purpose limitation;
- minimization;
- lawful processing;
- access control;
- retention management;
- incident response;
- appropriate contracts.

---

# 127. Data Monetization Ethics

Do not build a commercial model in which merchants only become valuable because their data can be sold.

Primary value remains:

> software → merchant outcomes.

Secondary value may become:

> permissioned, lawful ecosystem services.

---

# 128. Trust as an Economic Asset

Trust increases:

- retention;
- willingness to connect payment accounts;
- willingness to use EIS;
- willingness to adopt enterprise modules;
- partner confidence.

A single major trust failure can destroy years of platform value.

---

# 129. Security as Revenue Protection

Security protects:

- merchant data;
- transaction history;
- financial records;
- reputation;
- enterprise contracts.

Security is therefore part of the revenue model.

---

# 130. Reliability as Revenue Protection

For a merchant operating system:

```text
DOWNTIME
→
LOST BUSINESS
→
CUSTOMER ANGER
→
CHURN
→
REVENUE LOSS
```

Offline-first operation, recovery and observability have economic value.

---

# 131. Incident Economics

Track incidents by:

- merchants affected;
- transactions affected;
- duration;
- support cost;
- refunds;
- churn.

This lets management price reliability investments rationally.

---

# 132. Service Quality Strategy

Enterprise service quality should be measurable.

Metrics:

- response time;
- resolution time;
- incident frequency;
- availability;
- sync success.

Do not sell “premium support” without measurable service differentiation.

---

# 133. Disaster Recovery Economics

Recovery protects:

- historical data;
- customer trust;
- operating continuity;
- enterprise contracts.

RTO/RPO should eventually be translated into customer-plan economics.

---

# 134. Fraud Economics

Fraud can occur through:

- refunds;
- discounts;
- stock adjustments;
- account takeover;
- reconciliation manipulation.

Fraud controls have economic value when losses exceed control cost.

---

# 135. Segregation of Duties

Higher-value customers may require:

```text
CREATE
≠
APPROVE
```

Examples:

- cashier creates refund;
- manager approves refund.

This is a legitimate enterprise value driver.

---

# 136. Support Impersonation Risk

Support staff must not silently become merchant users.

Privileged support access must be:

- scoped;
- auditable;
- time-bound where possible;
- visible to governance.

This protects the business model from trust-destroying incidents.

---

# 137. Revenue Concentration

Track:

- top-1 customer revenue share;
- top-5;
- top-10.

High concentration can be acceptable in enterprise sales but must be consciously managed.

---

# 138. Provider Concentration

Track dependence on:

- one payment provider;
- one tax integration;
- one cloud vendor;
- one sales channel.

Concentration creates strategic fragility.

---

# 139. Government Concentration

Government integration and contracts can be useful.

The core business should still work without assuming government procurement revenue.

---

# 140. Grant Revenue Discipline

Grants can support:

- pilots;
- research;
- inclusion initiatives;
- market development.

Do not treat speculative grants as recurring revenue.

---

# 141. Country Expansion

Do not expand merely because the product “could work” elsewhere.

Expansion should require:

```text
MALAWI PRODUCT-MARKET FIT
+
REPEATABLE ACQUISITION
+
REASONABLE RETENTION
+
LOCAL PARTNERS
+
LOCALIZATION ECONOMICS
```

---

# 142. Country Localization Cost

A new country can require:

- tax integration;
- payment integration;
- legal review;
- currency;
- language;
- support;
- business registration logic;
- compliance.

Country entry is a business investment.

---

# 143. Market Selection Scorecard

Evaluate countries by:

- SME density;
- payment digitization;
- competitive intensity;
- local ARPU;
- localization cost;
- regulation;
- partner availability;
- acquisition cost;
- support cost.

---

# 144. Africa Expansion Doctrine

The platform should expand in a ring, not a scatter.

```text
MALAWI
 ↓
PROVEN MODEL
 ↓
ADJACENT MARKET
 ↓
LOCAL PAYMENT/TAX
 ↓
LOCAL DISTRIBUTION
```

The core remains common.

---

# 145. Regional Product Economics

Maintain:

```text
COMMON CORE
+
COUNTRY-SPECIFIC ADAPTERS
```

This protects development efficiency.

---

# 146. Pricing by Country

Local pricing should consider:

- local willingness to pay;
- inflation;
- FX exposure;
- competition;
- payment collection cost.

Do not simply convert MWK into USD and call it localization.

---

# 147. Revenue Forecasting

Operational forecasting should use:

```text
ACTIVE MERCHANTS
×
ARPU
×
RETENTION
+
EXPANSION
```

Top-down market size remains useful for strategy but weak for near-term forecasting.

---

# 148. Cohort Revenue

Track each merchant cohort by:

- acquisition month;
- plan;
- segment;
- retention;
- expansion;
- revenue.

Cohorts reveal whether the product is getting better.

---

# 149. Revenue Bridge

Monthly bridge:

```text
STARTING MRR
+
NEW MRR
+
EXPANSION MRR
-
CONTRACTION MRR
-
CHURNED MRR
=
ENDING MRR
```

This is the operational truth of SaaS growth.

---

# 150. Pipeline Economics

Track pipeline by:

- stage;
- segment;
- expected value;
- probability;
- expected close date;
- acquisition channel.

Do not inflate pipeline by counting unqualified leads.

---

# 151. Sales Cycle Economics

Longer sales cycles require higher expected contract value or higher strategic value.

A 6-month enterprise sales cycle is dangerous for a small contract.

---

# 152. Enterprise Minimum Deal Size

Set a minimum economic threshold based on:

```text
SALES EFFORT
+
IMPLEMENTATION
+
SUPPORT
+
SECURITY REVIEW
+
EXPECTED CONTRACT VALUE
```

If the economics fail, reject the deal.

---

# 153. Enterprise Procurement Playbook

Prepare reusable assets:

- security questionnaire;
- standard MSA;
- DPA;
- SLA;
- implementation SOW;
- pricing schedule;
- support policy.

This lowers sales cost over time.

---

# 154. Enterprise Proof-of-Value

A proof-of-value should define:

- baseline;
- target;
- duration;
- scope;
- success metric;
- paid conversion path.

Never let an enterprise pilot become indefinite free consulting.

---

# 155. Customer ROI Model

For suitable customers, estimate:

```text
TIME SAVED
+
EXCEPTIONS RESOLVED
+
STOCK LOSSES IDENTIFIED
+
REPORTING EFFORT REDUCED
```

Clearly label measured data vs estimates.

---

# 156. Product-Led vs Sales-Led Model

Sitolo should operate as a hybrid.

```text
MICRO
PRODUCT-LED / SELF-SERVICE

SME
ASSISTED

ENTERPRISE
SALES-LED
```

This matches customer economics.

---

# 157. Onboarding Automation

As volume rises, reduce human dependency:

```text
FOUNDER-LED
 ↓
ASSISTED
 ↓
GUIDED
 ↓
SELF-SERVICE
```

Enterprise remains assisted.

---

# 158. Merchant Onboarding Completion

Track completion of:

- business profile;
- location;
- products;
- opening stock;
- payment methods;
- first sale.

Drop-off at each stage indicates product friction.

---

# 159. First Value Event

Preferred first value event:

> **A real sale completes successfully, stock changes correctly, payment is recorded, and the owner can immediately see the business result.**

That is stronger than a simple app tour.

---

# 160. Product Habit Formation

A product becomes sticky when the merchant repeats a valuable workflow.

Desired habit:

```text
OPEN SHOP
→
SELL
→
CHECK
→
RECONCILE
→
CLOSE
```

---

# 161. End-of-Day Workflow

A high-value daily habit is:

```text
TODAY'S SALES
+
CASH COUNT
+
MOBILE MONEY
+
BANK
+
EXCEPTIONS
=
DAY CLOSE
```

This is an excellent retention loop.

---

# 162. Month-End Workflow

A mature merchant may use:

```text
MONTH SALES
+
COSTS
+
STOCK
+
PAYMENTS
+
EXPENSES
+
RECEIVABLES
=
MANAGEMENT REPORT
```

That creates a second retention loop.

---

# 163. Quarterly Management Value

For growing businesses:

```text
BRANCH PERFORMANCE
+
PRODUCT MARGIN
+
SUPPLIER PERFORMANCE
+
WORKING CAPITAL
```

This creates management-level dependency.

---

# 164. Business-Type Growth Paths

## Duka

```text
POS
→
Stock
→
Payments
→
Multi-user
```

## Pharmacy

```text
POS
→
Batch/Expiry
→
Controls
→
Branches
```

## Agro

```text
Inventory
→
Supplier
→
Seasonal control
→
Wholesale links
```

## Wholesale

```text
Purchasing
→
Warehouse
→
Receivables
→
Branches
→
API
```

---

# 165. Cross-Segment Platform Economics

All verticals share:

- identity;
- business hierarchy;
- users;
- payments;
- inventory foundations;
- reporting;
- billing;
- integrations.

This means each new vertical can reuse platform economics.

---

# 166. Platform Reuse Economics

The more shared capability exists, the lower the marginal cost of adding a new vertical.

```text
CORE PLATFORM COST
is paid once

VERTICAL EXTENSION COST
is incremental
```

This makes vertical expansion economically attractive.

---

# 167. Revenue Efficiency

Monitor:

```text
NEW MRR / SALES COST
EXPANSION MRR / CUSTOMER SUCCESS COST
MRR / SUPPORT FTE
MRR / ENGINEERING FTE
```

These metrics show organizational efficiency.

---

# 168. Headcount Planning

Do not add teams because the org chart looks professional.

Hire when:

```text
WORKLOAD
>
CURRENT CAPACITY
```

and the workload is economically justified.

---

# 169. Company Functional Model

Eventually:

```text
PRODUCT
ENGINEERING
SECURITY
SALES
MARKETING
CUSTOMER SUCCESS
SUPPORT
FINANCE
LEGAL / COMPLIANCE
PARTNERSHIPS
OPERATIONS
```

Early employees can cover multiple functions.

---

# 170. Founder Operating Model

In the early stage, the founder should stay close to:

- customers;
- product decisions;
- pricing;
- major incidents;
- key partnerships.

Distance from customer reality is a strategic risk.

---

# 171. Decision-Making Framework

For each major decision ask:

1. Does it increase merchant value?
2. Does it improve retention?
3. Does it improve expansion?
4. Does it create defensibility?
5. What does it cost?
6. What regulatory burden does it create?
7. What assumptions are unproven?

---

# 172. Feature Economics

Each major feature should be mapped to:

```text
ADOPTION
RETENTION
EXPANSION
MOAT
RISK
COST
```

A feature with high cost and no commercial effect should be questioned.

---

# 173. Feature Kill Policy

Retire or redesign features when:

- usage is persistently low;
- support burden is high;
- revenue effect is absent;
- strategic value is absent;
- risk is disproportionate.

Enterprise products need subtraction discipline.

---

# 174. Roadmap Gates

### Gate A
Problem confirmed.

### Gate B
Workflow works.

### Gate C
Customers repeat workflow.

### Gate D
Customers pay.

### Gate E
Unit economics are healthy.

### Gate F
Expansion is evidenced.

### Gate G
Scale is justified.

---

# 175. Product-Market Fit Evidence

Strong signals:

- merchants complain when Sitolo is unavailable;
- customers pay without founder discounting;
- branches are added;
- users are added;
- referrals occur;
- customers reject returning to manual methods.

Weak signals:

- social engagement;
- downloads;
- praise for design.

---

# 176. Customer Interview Discipline

Ask:

- What do you do today?
- What breaks?
- What does it cost?
- Who uses the process?
- What are you already paying?
- What makes you distrust software?
- What would make switching worthwhile?

Avoid hypothetical pricing questions as the only validation method.

---

# 177. Willingness-to-Pay Validation

Better questions:

> What do you currently spend solving this problem?

> What happens financially when it fails?

> Who approves software spend?

> What would have to improve for the spend to make sense?

These produce better evidence than “Would you pay?” alone.

---

# 178. Pilot Design

A good pilot has:

```text
BASELINE
+
HYPOTHESIS
+
DURATION
+
SCOPE
+
SUCCESS METRIC
+
COMMERCIAL CONVERSION
```

A free pilot without a conversion criterion is product research, not sales validation.

---

# 179. Pricing Experiments

Possible tests:

- free vs trial;
- MK25k vs MK35k Core;
- monthly vs annual;
- reconciliation bundled vs add-on;
- branch-based pricing.

Maintain customer trust throughout experiments.

---

# 180. Plan Migration Rules

Customers should be able to upgrade quickly.

Downgrade policy should be explicit.

Do not delete business data merely because a customer changes plans.

---

# 181. Failed Payment Handling

The commercial workflow can be:

```text
PAYMENT FAILED
 ↓
REMINDER
 ↓
GRACE PERIOD
 ↓
RESTRICTED MODE
 ↓
SUSPENSION
```

Keep records recoverable.

---

# 182. Refund Policy

Customer refunds should be governed by:

- reason;
- approval;
- amount;
- time window;
- audit.

Do not create a commercial refund system that is impossible to reconcile.

---

# 183. Discount Policy

Discounts should have:

- owner;
- duration;
- reason;
- ceiling;
- expiry.

Permanent founder discounts can hide pricing problems.

---

# 184. Sales Commission Economics

Commission should reward:

- paid conversion;
- quality of customer;
- retention.

Do not reward raw account creation alone.

---

# 185. Referral Economics

Referral payout should be tied to:

```text
REFERRED
→
ACTIVATED
→
PAID
```

This protects against low-quality referrals.

---

# 186. Customer Success Economics

Customer success should concentrate on high-value and high-risk customers.

Do not assign expensive human success managers to the smallest free accounts unless strategic evidence supports it.

---

# 187. Support Automation

Automate common questions:

- password recovery;
- device status;
- sync state;
- payment status;
- invoice lookup;
- report definitions.

This improves margin.

---

# 188. Operational Transparency

A merchant should see when:

- sync is pending;
- payment is unresolved;
- provider is unavailable;
- EIS submission is pending.

Clarity reduces support tickets.

---

# 189. Exception-First Design

The system should surface exceptions rather than bury them.

```text
2 UNMATCHED PAYMENTS
1 CASH VARIANCE
7 LOW-STOCK ITEMS
```

This creates obvious daily value.

---

# 190. Business Health Loop

```text
MEASURE
 ↓
FIND EXCEPTION
 ↓
ACT
 ↓
RESOLVE
 ↓
LEARN
```

This is a commercially valuable workflow.

---

# 191. Merchant Profitability

A useful high-level metric may be:

```text
REVENUE
-
COST OF GOODS
-
OPERATING EXPENSES
=
NET CONTRIBUTION / PROFIT VIEW
```

Exact accounting semantics require deliberate finance design.

---

# 192. Avoid “Fake Profit”

The product must distinguish:

- revenue;
- cash received;
- gross margin;
- expenses;
- receivables.

A merchant with high sales but weak cash collection should not be shown as automatically “healthy.”

---

# 193. Working-Capital Insight

A future finance layer can show:

- stock tied up in inventory;
- receivables;
- supplier obligations;
- payment timing.

This makes Sitolo more valuable to growing businesses.

---

# 194. Working-Capital Partner Opportunity

Once business records are trustworthy, partner products may become possible:

```text
MERCHANT DATA
 ↓
RISK / ELIGIBILITY VIEW
 ↓
LICENSED PARTNER
 ↓
WORKING CAPITAL
```

No guarantee is implied.

---

# 195. Credit Data Governance

Before sharing data:

- identify legal basis;
- identify data owner/controller roles as applicable;
- document purpose;
- establish access controls;
- log use;
- minimize fields.

---

# 196. Financial Services Revenue Governance

Any future partner revenue must define:

- who owns the customer relationship;
- who handles complaints;
- who bears fraud responsibility;
- who performs KYC/AML where required;
- what revenue share exists;
- what data is shared.

---

# 197. Partner Contract Principles

Contracts should cover:

- commercial scope;
- service levels;
- API usage;
- data;
- security;
- incident response;
- termination;
- liability;
- customer support.

---

# 198. Regulatory Change Monitoring

Maintain a regulatory watch for:

- tax/EIS;
- payments;
- data protection;
- pharmacy;
- financial products.

Regulatory changes can alter both product requirements and economics.

---

# 199. Compliance Evidence Repository

Store evidence of:

- certifications;
- contracts;
- policy approvals;
- security assessments;
- vendor due diligence.

This reduces enterprise sales friction.

---

# 200. Competitive Intelligence Cadence

Review competitors quarterly.

Capture:

- price;
- free tier;
- capabilities;
- market message;
- integration claims;
- vertical focus;
- support model.

Do not blindly copy.

---

# 201. Strategic Competitor Rule

The competitor is not only another POS vendor.

The true substitutes include:

- notebook;
- Excel;
- WhatsApp;
- calculator;
- accountant;
- manual reconciliation;
- another POS;
- ERP.

The product must beat the customer's **current workflow**, not merely another application.

---

# 202. Switching Barrier Removal

To replace an incumbent, reduce:

- migration fear;
- training cost;
- downtime risk;
- data loss risk;
- hardware cost.

Migration is a growth feature.

---

# 203. Competitive Win Conditions

Sitolo should win when customers value:

- offline continuity;
- integrated operations;
- payment control;
- better stock truth;
- easier growth;
- local support;
- predictable pricing.

---

# 204. Competitive Lose Conditions

Sitolo may lose when:

- merchant needs only a simple receipt generator;
- competitor has a materially lower price with sufficient value;
- enterprise requires integrations Sitolo does not provide;
- merchant distrusts recurring SaaS;
- customer needs a capability outside the product boundary.

These conditions should be accepted rather than solved through unlimited scope.

---

# 205. Market Positioning Matrix

| Dimension | Informal workflow | Basic POS | ERP | Sitolo |
|---|---|---|---|---|
| Setup simplicity | High | Medium | Low | High |
| Offline resilience | Human/manual | Varies | Varies | Core |
| Stock control | Low | Medium | High | High |
| Payment reconciliation | Low | Low/medium | Medium | Strategic |
| Multi-branch | Low | Medium | High | High |
| Enterprise controls | Low | Low | High | High at upper tiers |
| SME accessibility | High | High | Low | High |
| Local operating fit | High | Varies | Varies | Core strategy |

---

# 206. Channel Portfolio Model

Use a balanced mix:

```text
LOW-COST DIGITAL
+
REFERRALS
+
PARTNERS
+
FIELD
+
ENTERPRISE SALES
```

No single channel should become the only source of growth.

---

# 207. Acquisition Experiment Matrix

Measure each channel on:

- CAC;
- activation;
- conversion;
- retention;
- ARPU;
- payback.

A channel with low CAC but weak retention may be worse than a more expensive channel.

---

# 208. Sales Territory Strategy

Early geographic growth should prioritize merchant density.

```text
LOCAL CLUSTER
→
SUPPORT DENSITY
→
REFERRALS
→
NEXT CLUSTER
```

---

# 209. Merchant Community Strategy

Build communities around:

- merchant education;
- product updates;
- business tips;
- user feedback.

Do not turn the product into a social network.

---

# 210. Brand Trust Strategy

Use:

- transparent pricing;
- accurate claims;
- responsive support;
- customer proof;
- security documentation.

Avoid:

- fake testimonials;
- fake usage numbers;
- unsupported certification claims.

---

# 211. Enterprise References

Enterprise sales improve when reference customers exist.

Build references by sector:

- retail;
- pharmacy;
- wholesale;
- agro;
- multi-branch.

---

# 212. Case Study Model

A case study should capture:

```text
BEFORE
PROBLEM
IMPLEMENTATION
MEASURED OUTCOME
EXPANSION
```

Measured outcomes are more persuasive than adjectives.

---

# 213. Merchant Advisory Board

A rotating advisory board can validate:

- product;
- pricing;
- onboarding;
- support;
- roadmap.

Do not let one merchant dominate strategic decisions.

---

# 214. Commercial Metrics Tree

```text
REVENUE
  |
  +-- NEW
  +-- EXPANSION
  +-- RETENTION

CUSTOMERS
  |
  +-- ACQUISITION
  +-- ACTIVATION
  +-- RETENTION
  +-- REFERRAL

ECONOMICS
  |
  +-- ARPU
  +-- CAC
  +-- LTV
  +-- GROSS MARGIN

TRUST
  |
  +-- DATA ERRORS
  +-- RECON EXCEPTIONS
  +-- INCIDENTS
```

---

# 215. North-Star Metric

Recommended candidate:

> **Verified business activity per active merchant.**

It combines:

- real usage;
- business events;
- merchant dependency.

Do not optimize purely for transaction volume.

---

# 216. Secondary Product Metrics

Track:

- active merchants;
- transactions;
- reconciliation success;
- stock discrepancy rate;
- cash variance;
- branch coverage;
- report usage.

---

# 217. Commercial Dashboard

At minimum:

```text
MRR
ARR
NEW MRR
EXPANSION MRR
CHURN MRR
ARPU
CAC
LTV
GROSS MARGIN
ACTIVATION
RETENTION
NRR
SUPPORT COST
```

---

# 218. Trust Dashboard

```text
SYNC FAILURES
PAYMENT EXCEPTIONS
DUPLICATE EVENTS
STOCK DISCREPANCIES
REFUND EXCEPTIONS
SECURITY INCIDENTS
```

Trust should be measured as an operating metric.

---

# 219. Segment Dashboard

Break metrics down by:

- business type;
- plan;
- geography;
- acquisition channel;
- customer age;
- branch count.

Global averages can hide problems.

---

# 220. Unit Economics by Segment

Compare:

```text
DUKA
RETAIL
PHARMACY
AGRO
WHOLESALE
MULTI-BRANCH
ENTERPRISE
```

Track ARPU, cost-to-serve and retention separately.

---

# 221. Expansion Metrics

Measure:

- users per account;
- branches per account;
- modules per account;
- revenue expansion;
- plan upgrade rate.

These indicate platform depth.

---

# 222. Product Depth Metric

An internal “workflow depth” indicator could track adoption of:

```text
SALES
→
STOCK
→
PAYMENTS
→
RECONCILIATION
→
PURCHASING
→
REPORTING
→
BRANCHES
```

Deeper accounts should generally have higher retention.

---

# 223. Churn Cohort Analysis

Compare churn by:

- shallow vs deep workflow usage;
- one user vs many users;
- one branch vs multiple;
- reconciliation enabled vs not.

This can identify the real retention drivers.

---

# 224. Pricing Cohort Analysis

Track cohorts by plan price.

A higher price is only better if:

- customers still convert;
- retention remains healthy;
- support cost does not explode.

---

# 225. Customer Acquisition Payback by Channel

Each channel should have its own payback analysis.

```text
FIELD
PARTNER
REFERRAL
DIGITAL
ENTERPRISE
```

Budget should move toward productive channels.

---

# 226. Gross Margin Improvement Loop

```text
LOWER SUPPORT COST
+
LOWER INFRA COST
+
BETTER PRICING
+
LOWER BILLING COST
=
HIGHER GROSS MARGIN
```

Gross margin improvement funds growth.

---

# 227. Product Cost Discipline

Before adding a feature, estimate:

- build cost;
- support cost;
- infrastructure cost;
- security cost;
- compliance cost;
- revenue/retention value.

---

# 228. Strategic “No” List

Do not build primarily because:

- a competitor has it;
- investors might like it;
- it looks sophisticated;
- it is technically fun;
- it creates a large TAM slide.

Build because it improves the business.

---

# 229. Enterprise Architecture Handoff Principle

The future technical architecture must support this commercial structure without hard-coding business-model assumptions into infrastructure.

Commercial flexibility requires:

- plans;
- entitlements;
- business types;
- branch hierarchy;
- roles;
- integrations.

The architecture document should translate these into technical mechanisms later.

---

# 230. Business Model Dependency Graph

```text
MERCHANT ACQUISITION
        ↓
ACTIVATION
        ↓
DAILY VALUE
        ↓
TRUST
        ↓
RETENTION
        ↓
EXPANSION
        ↓
MRR
        ↓
REINVESTMENT
        ↓
BETTER PRODUCT
```

If activation fails, acquisition spend is wasted.

If trust fails, retention fails.

If expansion fails, LTV remains low.

---

# 231. Strategic Bottleneck Framework

At each stage identify the binding constraint.

```text
EARLY
Activation

GROWTH
Acquisition

SCALE
Support / reliability

ENTERPRISE
Implementation

ECOSYSTEM
Regulation / partnerships
```

The company should spend resources on the current bottleneck.

---

# 232. Business Model Stress Test

Stress-test the plan against:

- 50% slower acquisition;
- 2× support cost;
- 20% lower ARPU;
- 2× churn;
- payment-provider outage;
- EIS delay;
- enterprise sales delays.

If the business collapses under every downside case, the model is fragile.

---

# 233. Scenario Model — Conservative

```text
LOWER ARPU
LOWER CONVERSION
HIGHER SUPPORT COST
SLOWER ENTERPRISE SALES
```

The business survives only if core SaaS contribution remains positive.

---

# 234. Scenario Model — Base

```text
STEADY MERCHANT ACQUISITION
HEALTHY ACTIVATION
MODERATE UPSELL
CONTROLLED SUPPORT COST
```

This should be the operating plan.

---

# 235. Scenario Model — Expansion

```text
STRONG RETENTION
HIGHER BRANCH ADOPTION
ENTERPRISE SALES
PARTNER REVENUE
```

Treat this as upside, not baseline necessity.

---

# 236. Illustrative Revenue Scenario — 2,000 Businesses

Example mix:

```text
1,200 × MK20,000 = MK24,000,000
500 × MK50,000 = MK25,000,000
250 × MK100,000 = MK25,000,000
50 × MK250,000 = MK12,500,000

TOTAL = MK86,500,000 / month
```

Illustrative annualized revenue:

```text
MK1.038 billion
```

This is mathematics, not a forecast.

---

# 237. Illustrative Revenue Scenario — 10,000 Businesses

Model multiple plan mixes rather than one headline number.

Example:

```text
70% Entry/Core
20% Business
9% Growth
1% Enterprise
```

versus:

```text
50% Core
30% Business
15% Growth
5% Enterprise
```

The second mix may produce greater revenue with fewer accounts.

---

# 238. Revenue Quality

Prefer:

- recurring;
- diversified;
- high-retention;
- high-margin
revenue.

Do not value all revenue equally.

---

# 239. Revenue Risk Matrix

| Revenue source | Predictability | Margin potential | Complexity | Strategic value |
|---|---|---|---|---|
| SaaS | High | High | Low/medium | High |
| Expansion | High | High | Medium | High |
| Enterprise | Medium/high | Medium/high | High | High |
| Services | Medium | Medium | High | Medium |
| Partner finance | Low/medium initially | Unknown | High | High |
| Data products | Unknown | Potentially high | High | High |
| Transaction fees | Variable | Variable | High/regulatory | Potentially high |

---

# 240. Customer Concentration Strategy

The company should gradually diversify without sacrificing enterprise opportunity.

Targets should be set internally based on stage and financing strategy.

---

# 241. Payment-Provider Concentration Strategy

Maintain enough abstraction and negotiation leverage to avoid dependence on one provider.

If one provider fails:

```text
CORE SALES
should still work

ALTERNATIVE PAYMENT METHOD
should remain usable

RECONCILIATION
should mark provider exceptions clearly
```

---

# 242. Product Continuity Strategy

The product should distinguish:

```text
SYSTEM HEALTH
PAYMENT PROVIDER HEALTH
TAX SERVICE HEALTH
NETWORK HEALTH
DEVICE HEALTH
```

This avoids one outage becoming “Sitolo is down.”

---

# 243. Incident Communication

For customer trust:

- detect;
- communicate;
- mitigate;
- recover;
- explain.

Transparent incident handling is a retention asset.

---

# 244. Customer Feedback System

Collect feedback by:

- support;
- in-app prompts;
- interviews;
- advisory group;
- sales objections;
- churn interviews.

Tag feedback by business outcome.

---

# 245. Feedback Taxonomy

```text
ACQUISITION
ACTIVATION
USABILITY
RELIABILITY
FINANCE
INVENTORY
REPORTING
INTEGRATION
PRICING
SUPPORT
```

This converts noise into strategy.

---

# 246. Product Analytics Governance

Product analytics must respect privacy and contractual expectations.

Collect only data needed for:

- product improvement;
- operational reliability;
- billing;
- security;
- agreed commercial analytics.

---

# 247. Customer Consent Strategy

For any future ecosystem sharing:

```text
DISCLOSE
→
COLLECT / CONFIRM APPROPRIATE AUTHORITY
→
SHARE MINIMUM REQUIRED DATA
→
LOG ACCESS
```

Legal requirements must be validated for each use case.

---

# 248. Enterprise Data Governance

Enterprise customers may require:

- contract-specific retention;
- data export;
- access reports;
- data location commitments;
- incident notifications.

These become commercial requirements.

---

# 249. Security Review in Enterprise Sales

Prepare standard evidence for:

- authentication;
- authorization;
- encryption;
- audit;
- backups;
- incident response.

This reduces custom sales effort.

---

# 250. Commercial Risk Register

| Risk | Impact | Mitigation |
|---|---|---|
| Low WTP | High | Validate pricing early |
| High churn | High | Deepen workflow value |
| High support cost | High | Automation / segmentation |
| Competitor parity | High | Differentiate on control |
| Regulatory change | High | Governance |
| Provider dependency | High | Adapters / alternatives |
| Enterprise custom creep | High | Product boundary |
| Free-tier abuse | Medium | Limits |
| Data breach | Critical | Security investment |
| Poor data quality | High | Validation / reconciliation |

---

# 251. Strategic Risk — Commodity POS

Symptoms:

- low ARPU;
- high competition;
- low switching cost;
- feature parity.

Countermeasure:

> Deepen reconciliation, control and business-system value.

---

# 252. Strategic Risk — Giant ERP

Symptoms:

- enormous scope;
- low usability;
- long implementation;
- slow releases.

Countermeasure:

> Preserve modularity and operational simplicity.

---

# 253. Strategic Risk — Premature Fintech

Symptoms:

- regulatory burden;
- fraud exposure;
- liquidity requirements;
- distraction from SaaS.

Countermeasure:

> Partner first; own only what the business can responsibly operate.

---

# 254. Strategic Risk — Underpricing

Symptoms:

- customers love the product;
- company loses money serving them.

Countermeasure:

> Measure cost-to-serve and value-based pricing.

---

# 255. Strategic Risk — Overpricing

Symptoms:

- low conversion;
- low activation;
- heavy discounting.

Countermeasure:

> Validate price with real payment behavior.

---

# 256. Strategic Risk — Over-Serving Enterprise

Symptoms:

- custom features dominate roadmap;
- engineering becomes consulting.

Countermeasure:

> Charge separately for custom services and reject destructive requirements.

---

# 257. Strategic Risk — Data Commercialization Backlash

Symptoms:

- merchants feel surveilled;
- unclear data rights;
- partner access surprises customers.

Countermeasure:

> Make privacy and data governance part of the product's trust model.

---

# 258. Business Model Governance

Material changes require review:

- pricing;
- billing metrics;
- regulated products;
- partner sharing;
- data monetization;
- customer contract commitments.

---

# 259. Decision Rights

| Decision | Primary owner | Required review |
|---|---|---|
| Pricing | Product/Commercial | Finance |
| Enterprise discount | Commercial | Finance |
| Regulatory claim | Legal/Compliance | Product |
| Payment partner | Product | Security/Legal |
| Financial partner | Partnerships | Legal/Finance |
| Data monetization | Product | Privacy/Legal/Security |
| SLA | Operations | Engineering |
| Custom work | Product | Engineering/Commercial |

---

# 260. Evidence Discipline

Use these categories everywhere:

```text
FACT
OBSERVED
HYPOTHESIS
SCENARIO
ROADMAP
COMMITMENT
REGULATORY REQUIREMENT
```

Never present a scenario as a forecast or a roadmap item as a current feature.

---

# 261. Commercial Assumptions Register

At minimum validate:

1. Merchants will pay recurring SaaS.
2. Reconciliation increases retention or willingness to pay.
3. Business-type onboarding improves activation.
4. Desktop increases value for growing businesses.
5. Offline capability is a meaningful purchase driver.
6. EIS increases enterprise value.
7. Branch expansion increases ARPU.
8. Accountant referrals produce quality customers.
9. The product can be supported at positive contribution margin.
10. Financial partners eventually value the structured merchant record.

---

# 262. Commercial Experiment Register

Each experiment should contain:

```text
HYPOTHESIS
SEGMENT
VARIABLE
BASELINE
SAMPLE
RESULT
DECISION
```

No “successful” label without evidence.

---

# 263. 12 Initial Commercial Experiments

1. Free vs trial.
2. Core price sensitivity.
3. Reconciliation willingness-to-pay.
4. Accountant referrals.
5. Distributor acquisition.
6. Business-type onboarding.
7. Mobile-only retention.
8. Mobile+desktop retention.
9. Branch expansion demand.
10. EIS purchase-driver effect.
11. Premium support demand.
12. Enterprise pilot economics.

---

# 264. Pilot Conversion Rule

Every serious pilot should contain a conversion path.

```text
PILOT
→
MEASURE
→
REVIEW
→
PAID CONTRACT
```

If a prospect refuses any commercial discussion after a value demonstration, record the lesson.

---

# 265. Merchant Segmentation Scorecard

Score by:

- transaction volume;
- staff count;
- branch count;
- payment complexity;
- inventory complexity;
- compliance needs;
- willingness to pay;
- support burden.

---

# 266. Merchant Quality Score

A high-quality merchant tends to:

- transact frequently;
- use multiple workflows;
- pay on time;
- remain active;
- expand;
- refer others.

This score can guide sales investment.

---

# 267. Account Expansion Trigger Matrix

```text
MORE STAFF
→ user expansion

MORE LOCATIONS
→ branch plan

MORE PAYMENT CHANNELS
→ reconciliation

MORE INVENTORY
→ advanced inventory

MORE FORMALIZATION
→ EIS

MORE INTEGRATION
→ API / Enterprise
```

---

# 268. Enterprise Customer Journey

```text
DISCOVERY
 ↓
SOLUTION FIT
 ↓
SECURITY REVIEW
 ↓
PILOT
 ↓
CONTRACT
 ↓
IMPLEMENTATION
 ↓
ROLLOUT
 ↓
RENEWAL
 ↓
EXPANSION
```

---

# 269. Enterprise Expansion Strategy

After initial rollout:

```text
ONE BRANCH
→
MORE USERS
→
MORE BRANCHES
→
MORE MODULES
→
API
```

The goal is to turn successful pilots into platform standards.

---

# 270. Partner Economics

For each partner ask:

```text
REVENUE
+
DISTRIBUTION
+
MERCHANT VALUE
+
STRATEGIC VALUE
-
COMPLEXITY
-
RISK
```

A partnership without measurable benefit should not exist.

---

# 271. Partner Performance Metrics

Track:

- leads;
- activations;
- paid conversions;
- retention;
- revenue;
- support burden.

Partner logo count is not a useful KPI.

---

# 272. Reseller Economics

A reseller should receive enough margin to invest in acquisition and support, while Sitolo preserves positive contribution margin.

Use:

```text
WHOLESALE PRICE
<
CUSTOMER PRICE
```

with clear service responsibilities.

---

# 273. Enterprise Discounting

Discounting should be tied to:

- volume;
- contract length;
- deployment scale;
- prepaid value.

Do not discount randomly.

---

# 274. Commercial Contract Governance

Contracts should define:

- price;
- scope;
- service;
- support;
- data;
- security;
- termination;
- export.

---

# 275. Business Continuity for Sitolo

The company itself needs continuity for:

- payment collection;
- support;
- infrastructure;
- staffing;
- critical vendors.

Operational resilience protects revenue.

---

# 276. Critical Vendor Strategy

Critical categories:

- cloud;
- database;
- payment;
- EIS;
- communications.

Every critical dependency should have a documented failure strategy.

---

# 277. Operational Cost Review

Review monthly:

- infrastructure per merchant;
- support cost;
- payment collection cost;
- sales cost;
- implementation cost.

---

# 278. Capital Allocation

Spend primarily on:

```text
PRODUCT
RELIABILITY
SECURITY
DISTRIBUTION
CUSTOMER SUCCESS
```

Cut vanity spend before cutting customer trust.

---

# 279. Founder Capital Strategy

Early capital should preserve runway.

Priorities:

- reach paying customers;
- prove retention;
- build repeatable acquisition;
- keep fixed costs controlled.

---

# 280. Bootstrapped Viability Test

A useful test is:

> Can recurring SaaS revenue eventually fund core operating expenses without requiring speculative future financial-services income?

The business should aim for “yes.”

---

# 281. Investment Readiness

If external capital is pursued, present:

- retention;
- MRR growth;
- CAC;
- gross margin;
- expansion;
- customer concentration;
- regulatory posture.

Do not rely solely on TAM narratives.

---

# 282. Strategic Capital Use

Capital should accelerate:

- proven acquisition channels;
- product reliability;
- enterprise sales;
- regional expansion after validation.

Capital should not primarily fund speculative feature sprawl.

---

# 283. Enterprise Reference Metrics

Track:

- implementation time;
- time-to-first-value;
- branch rollout completion;
- adoption per branch;
- renewal rate;
- support incidents.

---

# 284. Vertical Expansion Economics

A new vertical is attractive if:

```text
NEW ARPU
+
NEW MARKET SIZE
+
REUSE OF CORE
>
VERTICAL BUILD COST
+
SUPPORT COST
+
REGULATORY COST
```

---

# 285. Pharmacy Expansion Gate

Before large pharmacy expansion:

- understand customer workflow;
- validate regulatory requirements;
- validate willingness to pay;
- validate support burden.

---

# 286. Agro Expansion Gate

Before scaling agro-dealer workflows:

- validate seasonality;
- validate units and batch needs;
- validate supplier process;
- validate adoption.

---

# 287. Wholesale Expansion Gate

Before prioritizing wholesale:

- validate procurement demand;
- validate warehouse complexity;
- validate credit/receivable needs;
- validate multi-branch demand.

---

# 288. Enterprise Expansion Gate

Before enterprise push:

- have reference accounts;
- standardized contracts;
- security materials;
- implementation process;
- support model.

---

# 289. Country Expansion Gate

Before entering country 2:

- Malawi retention must be stable;
- acquisition must be repeatable;
- support burden manageable;
- localization economics understood.

---

# 290. Financial Ecosystem Gate

Before launching partner finance:

- data quality proven;
- strong auditability;
- clear legal model;
- partner contract;
- security review;
- customer disclosure/consent approach.

---

# 291. Data Product Gate

Before monetizing aggregate insights:

- legal review;
- data governance;
- re-identification review;
- commercial demand;
- merchant trust assessment.

---

# 292. Separate-Product Trigger

A standalone financial infrastructure product is justified only after repeated external demand for:

- reconciliation APIs;
- payment normalization;
- third-party POS ingestion;
- enterprise finance infrastructure.

Until then, keep the capability inside Sitolo.

---

# 293. Final Strategic Moat

The moat is the combination:

```text
LOCAL FIT
+
OFFLINE RELIABILITY
+
WORKFLOW DEPTH
+
RECONCILIATION
+
BUSINESS HISTORY
+
INTEGRATIONS
+
TRUST
+
DISTRIBUTION
```

No single feature is sufficient.

---

# 294. Final Business Model Flywheel

```text
MERCHANT JOINS
      ↓
SELLS
      ↓
TRACKS STOCK
      ↓
TRACKS MONEY
      ↓
RECONCILES
      ↓
TRUSTS DATA
      ↓
DEPENDS ON WORKFLOW
      ↓
UPGRADES
      ↓
ADDS BRANCHES
      ↓
NEEDS INTEGRATIONS
      ↓
BECOMES LONG-TERM CUSTOMER
```

---

# 295. Final Business Model Architecture

```text
                         SITOLO
                           |
          +----------------+----------------+
          |                                 |
       OPERATE                            CONTROL
          |                                 |
   POS / INVENTORY                  CASH / PAYMENTS
   PURCHASING                       RECONCILIATION
   CUSTOMERS                        FINANCE CONTROLS
          |                                 |
          +----------------+----------------+
                           |
                    BUSINESS RECORD
                           |
           +---------------+---------------+
           |                               |
       COMPLIANCE                       INSIGHT
           |                               |
          EIS                        REPORTS / ALERTS
           |                               |
           +---------------+---------------+
                           |
                      ENTERPRISE
                           |
               BRANCH / API / SLA
                           |
                       PARTNERS
                           |
             BANKS / MFIs / INSURERS
```

---

# 296. Non-Negotiable Commercial Principles

1. Merchant value before monetization tricks.
2. SaaS before speculative fintech.
3. Reconciliation is strategic.
4. Offline operation is a commercial requirement.
5. Financial history must remain trustworthy.
6. Pricing must map to value and complexity.
7. Business type configures experience, not permanent entitlement.
8. Enterprise means control and service, not infinite customization.
9. Customer data is a trust asset.
10. Regulatory claims require evidence.
11. Financial partnerships require governance.
12. Market expansion follows evidence.
13. Retention is more valuable than vanity growth.
14. Every major feature needs a business reason.

---

# 297. Final “How Sitolo Makes Money” Model

```text
              CUSTOMERS
                  |
                  v
              SITOLO
                  |
       +----------+----------+
       |          |          |
      CORE      BUSINESS   ENTERPRISE
       |          |          |
      SaaS       SaaS       SaaS
       |          |          |
       +----------+----------+
                  |
              EXPANSION
                  |
      +-----------+-----------+
      |           |           |
   BRANCHES     MODULES      USERS
                  |
                  v
              SERVICES
                  |
       +----------+----------+
       |                     |
   INTEGRATIONS           SUPPORT
                  |
                  v
              ECOSYSTEM
                  |
        PARTNER FINANCE
```

Primary economics are SaaS and expansion.

Services supplement SaaS.

Financial ecosystem revenue is future upside.

---

# 298. Final “Why Customers Stay” Model

Customers should stay because:

```text
SIT0LO SAVES TIME
+
SIT0LO REDUCES ERRORS
+
SIT0LO PROTECTS BUSINESS VISIBILITY
+
SIT0LO CONTROLS CASH / PAYMENTS
+
SIT0LO HOLDS TRUSTED HISTORY
+
SIT0LO SUPPORTS GROWTH
```

Not because:

```text
SIT0LO LOCKED THE DATA
```

---

# 299. Final “Why the Combined Concept Is Strong”

Combining Sitolo with the Mobile Money Reconciliation & Merchant Finance OS concept is strategically stronger because:

- the same merchant acquisition funds both workflows;
- payments are generated by the operational workflow;
- reconciliation gains context from the originating sale;
- financial reports gain operational context;
- the merchant can expand without changing systems;
- the historical record compounds in value.

The result is:

> **One business operating system with an embedded financial-control layer.**

---

# 300. Handoff to System Architecture Design

The next document should translate this business model into a system architecture.

Architecture must support, at minimum, the business realities below:

```text
SITOLO
 |
 +-- Multi-tenant organization model
 +-- Business-type configuration
 +-- Plan / entitlement model
 +-- Mobile daily operations
 +-- Desktop back-office control
 +-- Offline-capable workflows
 +-- Sales + inventory + purchasing
 +-- Cash + payment recording
 +-- Reconciliation
 +-- Auditability
 +-- EIS integration boundary
 +-- Multi-branch
 +-- Enterprise permissions
 +-- Subscription / billing
 +-- Controlled integrations
 +-- Future partner APIs
```

The architecture should not force every future financial product into the initial deployment topology.

The correct technical goal is:

> **Strong boundaries, durable contracts, extensibility, reliability and economic efficiency.**

---

# 301. Executive Conclusion

Sitolo is a business operating system, not a POS application.

Its strongest commercial design is:

```text
MOBILE
→ run the business

DESKTOP
→ control the business

CLOUD
→ hold the shared business record

FINANCIAL LAYER
→ reconcile and explain money

ENTERPRISE LAYER
→ govern growth and complexity

ECOSYSTEM
→ connect partners when the business is ready
```

The merchant enters because Sitolo makes selling and stock easier.

The merchant stays because Sitolo makes money and operations clearer.

The merchant upgrades because the business becomes more complex.

The company grows because the same customer can progress from:

```text
DUKA
→
SMALL SME
→
MULTI-STAFF
→
MULTI-BRANCH
→
ENTERPRISE
```

The financial layer is strategically important because it transforms raw transactions into a trusted record of economic activity.

The business model should therefore remain disciplined:

> **Win with operations. Retain with control. Expand with complexity. Monetize enterprise needs. Build financial partnerships only after the software business is strong.**

That is the commercial foundation of:

# **SITOLO — BUSINESS OPERATING SYSTEM FOR AFRICAN SMEs**

---

# 302. Research Sources

### Malawi digital adoption

DataReportal — Digital 2026: Malawi

https://datareportal.com/reports/digital-2026-malawi

### MRA EIS developer resources

Malawi Revenue Authority — EIS Developer Resource Center

https://eis-portal.mra.mw/Home/DeveloperResources

### MRA EIS API introduction

https://eis-api.mra.mw/docs/introduction_1_print.htm

### MRA EIS overview

https://eis-api.mra.mw/docs/overview.htm

### MRA EIS pre-integration guide

https://eis-api.mra.mw/docs/developer_pre_integration_guide_print.htm

### MRA EIS certification

https://eis-api.mra.mw/docs/introduction_3_print.htm

### Malawi Enterprise Survey

World Bank — Malawi 2025 Enterprise Survey

https://microdata.worldbank.org/catalog/8117/variable/V225

### Malawi economic environment

World Bank — Malawi Economic Monitor

https://www.worldbank.org/en/country/malawi/publication/economic-monitor

### Airtel Africa developer portal

https://developers.airtel.africa/

### Competitive vendor research

MalondaPlus

https://www.malonda.ictechmw.com/malondaplus/index.php

Phindu

https://www.phindu.co/

Phindu POS

https://www.phindu.co/pos-system-malawi

Quick-Think Solutions

https://www.quickthinks.com/

Stooqo

https://www.stooqo.com/

ninoPOS

https://ninopos.com/

### Research use policy

Competitor pricing and capability claims are treated as vendor-reported market signals, not independent audits. Regulatory, payment-provider and certification statements must be re-verified before external use.

---

# 303. Commercial Assumption Ledger — Required Future Updates


## 303.1 Assumption ledger

| Assumption | Confidence | Validation | Decision if false |
|---|---|---|---|
| Merchants will pay recurring SaaS | High | Paid pilots | If false: reduce scope or revisit segment/pricing |
| Reconciliation increases retention | Medium | Cohort comparison | If false: bundle into core rather than premium |
| Business-type onboarding improves activation | Medium | A/B or cohort comparison | If false: simplify onboarding |
| Desktop increases ARPU for growing SMEs | Medium | Compare mobile-only vs desktop-enabled accounts | If false: keep desktop focused on enterprise |
| Offline capability reduces churn | Medium | Incident and retention analysis | If false: revisit investment emphasis |
| EIS increases enterprise conversion | Medium | Sales attribution | If false: keep as compliance capability, not pricing anchor |
| Accountant channel has favorable CAC | Low | Tracked referral cohort | If false: stop or redesign partner model |
| Multi-branch customers produce materially higher LTV | Medium | Expansion cohort | If false: revisit branch packaging |
| Support can scale through self-service | Medium | Tickets per merchant | If false: increase price or reduce low-ARPU segments |
| Financial partners will value verified business data | Low | Partner discovery interviews + pilots | If false: no impact to core SaaS strategy |


---

# 304. Commercial Decision Log Template

Every strategic commercial decision should record:

```text
DATE
DECISION
PROBLEM
OPTIONS
SELECTED OPTION
WHY
EVIDENCE
RISKS
OWNER
REVIEW DATE
REVERSAL CONDITION
```

This prevents the business from repeatedly debating the same question without accumulating institutional knowledge.

---

# 305. Pricing Decision Template

```text
SEGMENT
PLAN
PRICE
INCLUDED VALUE
EXPECTED COST TO SERVE
WILLINGNESS-TO-PAY EVIDENCE
CONVERSION RESULT
RETENTION RESULT
DECISION
```

---

# 306. Partner Decision Template

```text
PARTNER
CUSTOMER VALUE
DISTRIBUTION VALUE
REVENUE VALUE
INTEGRATION COST
REGULATORY RISK
SECURITY RISK
CONTRACT TERM
EXIT PLAN
DECISION
```

---

# 307. New-Country Decision Template

```text
COUNTRY
TARGET SEGMENT
CUSTOMER COUNT ESTIMATE
ARPU HYPOTHESIS
LOCALIZATION COST
REGULATORY COST
PAYMENT PARTNERS
TAX PARTNER
DISTRIBUTION PLAN
SUPPORT MODEL
EXPECTED PAYBACK
GO / NO-GO
```

---

# 308. New-Revenue-Stream Decision Template

Before activating any new revenue line, document:

1. Customer problem.
2. Value created.
3. Pricing model.
4. Variable cost.
5. Gross margin.
6. Operational risk.
7. Regulatory risk.
8. Security risk.
9. Partner dependence.
10. Exit criteria.

---

# 309. Enterprise Customer Scoring

Potential score dimensions:

| Dimension | Weight example |
|---|---:|
| Revenue potential | 25% |
| Strategic fit | 20% |
| Retention potential | 15% |
| Expansion potential | 15% |
| Implementation effort | 10% |
| Regulatory/security complexity | 10% |
| Reference value | 5% |

Weights are examples and should be adapted as real data appears.

---

# 310. Merchant Acquisition Scoring

Potential score dimensions:

```text
PAIN INTENSITY
TRANSACTION FREQUENCY
CURRENT SOFTWARE SPEND
PAYMENT COMPLEXITY
INVENTORY COMPLEXITY
STAFF COMPLEXITY
LIKELY RETENTION
LIKELY REFERRAL
```

Use scoring to allocate sales effort rather than as a rigid exclusion rule.

---

# 311. Support Priority Model

Support priority should consider:

```text
BUSINESS IMPACT
+
NUMBER OF USERS AFFECTED
+
REVENUE IMPACT
+
DATA INTEGRITY IMPACT
+
SECURITY IMPACT
```

A harmless UI question is different from a payment reconciliation incident.

---

# 312. Customer Health State Machine

```text
HEALTHY
 ↓
AT RISK
 ↓
CRITICAL
 ↓
RECOVERING
 ↓
HEALTHY
```

Risk indicators:

- declining activity;
- unresolved payment exceptions;
- support escalation;
- billing failure;
- branch inactivity.

---

# 313. Expansion Opportunity State Machine

```text
BASE ACCOUNT
 ↓
USAGE GROWTH
 ↓
COMPLEXITY SIGNAL
 ↓
EXPANSION PROPOSAL
 ↓
UPGRADE
 ↓
ADOPTION
```

The system should identify expansion opportunities from genuine usage signals.

---

# 314. Product Adoption Ladder

```text
LOGIN
 ↓
FIRST SALE
 ↓
REPEATED SALES
 ↓
STOCK USE
 ↓
PAYMENT USE
 ↓
RECONCILIATION
 ↓
REPORTING
 ↓
MULTI-USER
 ↓
MULTI-BRANCH
```

This ladder can be used to define customer maturity.

---

# 315. Merchant Maturity Levels

### Level 0
Manual records.

### Level 1
Digital sales.

### Level 2
Digital stock.

### Level 3
Payment control.

### Level 4
Business finance.

### Level 5
Multi-branch enterprise control.

### Level 6
Integrated ecosystem.

---

# 316. Sitolo Customer Maturity Value

Higher maturity should generally correlate with:

- more revenue;
- more retention;
- more support complexity;
- more expansion value.

This is why product packaging should grow with customer maturity.

---

# 317. Revenue-per-Workflow Model

A customer account becomes more valuable as it adopts workflows.

```text
SALES
→ baseline value

STOCK
→ retention value

PAYMENTS
→ control value

RECONCILIATION
→ premium value

BRANCHES
→ expansion value

API
→ enterprise value
```

---

# 318. Product Simplicity Rule

Customer-facing complexity must remain low even when backend capability is high.

The merchant should see:

> “My business.”

not:

> “A distributed enterprise platform with 140 modules.”

---

# 319. Enterprise Capability Without Enterprise UX

The product can support:

- advanced controls;
- many branches;
- APIs;
- audit;

without forcing these concepts onto a small merchant.

This is the purpose of progressive disclosure.

---

# 320. Progressive Disclosure Model

```text
MICRO
simple

SME
more tools

GROWTH
advanced controls

ENTERPRISE
full governance
```

---

# 321. Why This Matters Economically

Reducing UI complexity improves:

- onboarding;
- training;
- activation;
- support economics.

Increasing capabilities behind the scenes improves:

- retention;
- enterprise value;
- expansion.

Sitolo needs both.

---

# 322. Merchant Trust Contract

The product implicitly promises:

> If you record a business event, Sitolo will preserve its meaning and make its status understandable.

That is central to the brand.

---

# 323. Financial Truth Contract

The financial layer should aim to answer:

```text
WHAT HAPPENED?
WHAT SHOULD HAVE HAPPENED?
WHAT ACTUALLY HAPPENED?
WHAT IS UNRESOLVED?
WHO CHANGED IT?
```

This is the heart of merchant finance value.

---

# 324. Operational Truth Contract

Inventory should answer:

```text
WHAT CAME IN?
WHAT WENT OUT?
WHAT REMAINS?
WHAT IS DAMAGED?
WHAT IS EXPIRED?
WHAT IS UNEXPLAINED?
```

---

# 325. Management Truth Contract

Management should answer:

```text
WHAT SOLD?
WHAT MADE MONEY?
WHAT IS RUNNING OUT?
WHERE IS MONEY?
WHICH BRANCH PERFORMS?
WHAT REQUIRES ATTENTION?
```

---

# 326. Compliance Truth Contract

Compliance workflows should answer:

```text
WHAT WAS SUBMITTED?
WHEN?
WITH WHAT STATUS?
WHAT FAILED?
WHAT NEEDS ACTION?
```

---

# 327. Enterprise Truth Contract

Enterprise controls should answer:

```text
WHO CAN DO WHAT?
WHO DID WHAT?
WHAT REQUIRED APPROVAL?
WHAT CHANGED?
WHAT IS THE CURRENT POLICY?
```

---

# 328. One System, Different Questions

This is the strategic reason to keep all modules together:

```text
CASHIER
“What do I sell?”

INVENTORY
“What do we have?”

FINANCE
“Where is the money?”

MANAGER
“How are we doing?”

OWNER
“Is the business healthy?”
```

---

# 329. Enterprise Product Narrative

For a small owner:

> “Run my shop.”

For a manager:

> “Run my team.”

For a multi-branch operator:

> “Control my business.”

For enterprise:

> “Connect operations, finances and governance.”

---

# 330. Final Go-to-Market Doctrine

```text
START SIMPLE
→
PROVE VALUE
→
CHARGE
→
RETAIN
→
EXPAND
→
STANDARDIZE
→
SCALE
```

Do not invert the sequence.

---

# 331. Final Product Doctrine

```text
ONE CORE
+
VERTICAL CONFIGURATION
+
PROGRESSIVE ENTERPRISE DEPTH
```

---

# 332. Final Revenue Doctrine

```text
SaaS first
Expansion second
Enterprise third
Services fourth
Ecosystem later
```

---

# 333. Final Risk Doctrine

```text
DO NOT
trade trust for short-term revenue.

DO NOT
trade product focus for feature volume.

DO NOT
trade regulatory safety for fintech excitement.

DO NOT
trade unit economics for vanity growth.
```

---

# 334. Final Enterprise Business Model Statement

> **Sitolo is a business operating system for African SMEs that starts with the operational reality of selling and stock, grows into financial control and reconciliation, expands into enterprise governance and compliance, and eventually becomes an integration point for business and financial partners. Its primary economic engine is recurring SaaS revenue, strengthened by account expansion and enterprise contracts. Its long-term moat is the combination of local fit, offline reliability, workflow depth, trusted business history, reconciliation, integration and merchant trust.**

---

# 335. Handoff Acceptance Criteria

The business model is considered sufficient for system architecture design when the architecture team can identify:

- the customer hierarchy;
- business types;
- entitlement model;
- core workflows;
- financial boundaries;
- reconciliation requirements;
- enterprise expansion points;
- integration classes;
- billing dimensions;
- audit requirements;
- support model;
- data governance requirements;
- future ecosystem boundaries.

That is the purpose of this document.


## 303.1 Assumption ledger

| Assumption | Confidence | Validation | Decision if false |
|---|---|---|---|
| Merchants will pay recurring SaaS | High | Paid pilots | If false: reduce scope or revisit segment/pricing |
| Reconciliation increases retention | Medium | Cohort comparison | If false: bundle into core rather than premium |
| Business-type onboarding improves activation | Medium | A/B or cohort comparison | If false: simplify onboarding |
| Desktop increases ARPU for growing SMEs | Medium | Compare mobile-only vs desktop-enabled accounts | If false: keep desktop focused on enterprise |
| Offline capability reduces churn | Medium | Incident and retention analysis | If false: revisit investment emphasis |
| EIS increases enterprise conversion | Medium | Sales attribution | If false: keep as compliance capability, not pricing anchor |
| Accountant channel has favorable CAC | Low | Tracked referral cohort | If false: stop or redesign partner model |
| Multi-branch customers produce materially higher LTV | Medium | Expansion cohort | If false: revisit branch packaging |
| Support can scale through self-service | Medium | Tickets per merchant | If false: increase price or reduce low-ARPU segments |
| Financial partners will value verified business data | Low | Partner discovery interviews + pilots | If false: no impact to core SaaS strategy |


---

# 304. Commercial Decision Log Template

Every strategic commercial decision should record:

```text
DATE
DECISION
PROBLEM
OPTIONS
SELECTED OPTION
WHY
EVIDENCE
RISKS
OWNER
REVIEW DATE
REVERSAL CONDITION
```

This prevents the business from repeatedly debating the same question without accumulating institutional knowledge.

---

# 305. Pricing Decision Template

```text
SEGMENT
PLAN
PRICE
INCLUDED VALUE
EXPECTED COST TO SERVE
WILLINGNESS-TO-PAY EVIDENCE
CONVERSION RESULT
RETENTION RESULT
DECISION
```

---

# 306. Partner Decision Template

```text
PARTNER
CUSTOMER VALUE
DISTRIBUTION VALUE
REVENUE VALUE
INTEGRATION COST
REGULATORY RISK
SECURITY RISK
CONTRACT TERM
EXIT PLAN
DECISION
```

---

# 307. New-Country Decision Template

```text
COUNTRY
TARGET SEGMENT
CUSTOMER COUNT ESTIMATE
ARPU HYPOTHESIS
LOCALIZATION COST
REGULATORY COST
PAYMENT PARTNERS
TAX PARTNER
DISTRIBUTION PLAN
SUPPORT MODEL
EXPECTED PAYBACK
GO / NO-GO
```

---

# 308. New-Revenue-Stream Decision Template

Before activating any new revenue line, document:

1. Customer problem.
2. Value created.
3. Pricing model.
4. Variable cost.
5. Gross margin.
6. Operational risk.
7. Regulatory risk.
8. Security risk.
9. Partner dependence.
10. Exit criteria.

---

# 309. Enterprise Customer Scoring

Potential score dimensions:

| Dimension | Weight example |
|---|---:|
| Revenue potential | 25% |
| Strategic fit | 20% |
| Retention potential | 15% |
| Expansion potential | 15% |
| Implementation effort | 10% |
| Regulatory/security complexity | 10% |
| Reference value | 5% |

Weights are examples and should be adapted as real data appears.

---

# 310. Merchant Acquisition Scoring

Potential score dimensions:

```text
PAIN INTENSITY
TRANSACTION FREQUENCY
CURRENT SOFTWARE SPEND
PAYMENT COMPLEXITY
INVENTORY COMPLEXITY
STAFF COMPLEXITY
LIKELY RETENTION
LIKELY REFERRAL
```

Use scoring to allocate sales effort rather than as a rigid exclusion rule.

---

# 311. Support Priority Model

Support priority should consider:

```text
BUSINESS IMPACT
+
NUMBER OF USERS AFFECTED
+
REVENUE IMPACT
+
DATA INTEGRITY IMPACT
+
SECURITY IMPACT
```

A harmless UI question is different from a payment reconciliation incident.

---

# 312. Customer Health State Machine

```text
HEALTHY
 ↓
AT RISK
 ↓
CRITICAL
 ↓
RECOVERING
 ↓
HEALTHY
```

Risk indicators:

- declining activity;
- unresolved payment exceptions;
- support escalation;
- billing failure;
- branch inactivity.

---

# 313. Expansion Opportunity State Machine

```text
BASE ACCOUNT
 ↓
USAGE GROWTH
 ↓
COMPLEXITY SIGNAL
 ↓
EXPANSION PROPOSAL
 ↓
UPGRADE
 ↓
ADOPTION
```

The system should identify expansion opportunities from genuine usage signals.

---

# 314. Product Adoption Ladder

```text
LOGIN
 ↓
FIRST SALE
 ↓
REPEATED SALES
 ↓
STOCK USE
 ↓
PAYMENT USE
 ↓
RECONCILIATION
 ↓
REPORTING
 ↓
MULTI-USER
 ↓
MULTI-BRANCH
```

This ladder can be used to define customer maturity.

---

# 315. Merchant Maturity Levels

### Level 0
Manual records.

### Level 1
Digital sales.

### Level 2
Digital stock.

### Level 3
Payment control.

### Level 4
Business finance.

### Level 5
Multi-branch enterprise control.

### Level 6
Integrated ecosystem.

---

# 316. Sitolo Customer Maturity Value

Higher maturity should generally correlate with:

- more revenue;
- more retention;
- more support complexity;
- more expansion value.

This is why product packaging should grow with customer maturity.

---

# 317. Revenue-per-Workflow Model

A customer account becomes more valuable as it adopts workflows.

```text
SALES
→ baseline value

STOCK
→ retention value

PAYMENTS
→ control value

RECONCILIATION
→ premium value

BRANCHES
→ expansion value

API
→ enterprise value
```

---

# 318. Product Simplicity Rule

Customer-facing complexity must remain low even when backend capability is high.

The merchant should see:

> “My business.”

not:

> “A distributed enterprise platform with 140 modules.”

---

# 319. Enterprise Capability Without Enterprise UX

The product can support:

- advanced controls;
- many branches;
- APIs;
- audit;

without forcing these concepts onto a small merchant.

This is the purpose of progressive disclosure.

---

# 320. Progressive Disclosure Model

```text
MICRO
simple

SME
more tools

GROWTH
advanced controls

ENTERPRISE
full governance
```

---

# 321. Why This Matters Economically

Reducing UI complexity improves:

- onboarding;
- training;
- activation;
- support economics.

Increasing capabilities behind the scenes improves:

- retention;
- enterprise value;
- expansion.

Sitolo needs both.

---

# 322. Merchant Trust Contract

The product implicitly promises:

> If you record a business event, Sitolo will preserve its meaning and make its status understandable.

That is central to the brand.

---

# 323. Financial Truth Contract

The financial layer should aim to answer:

```text
WHAT HAPPENED?
WHAT SHOULD HAVE HAPPENED?
WHAT ACTUALLY HAPPENED?
WHAT IS UNRESOLVED?
WHO CHANGED IT?
```

This is the heart of merchant finance value.

---

# 324. Operational Truth Contract

Inventory should answer:

```text
WHAT CAME IN?
WHAT WENT OUT?
WHAT REMAINS?
WHAT IS DAMAGED?
WHAT IS EXPIRED?
WHAT IS UNEXPLAINED?
```

---

# 325. Management Truth Contract

Management should answer:

```text
WHAT SOLD?
WHAT MADE MONEY?
WHAT IS RUNNING OUT?
WHERE IS MONEY?
WHICH BRANCH PERFORMS?
WHAT REQUIRES ATTENTION?
```

---

# 326. Compliance Truth Contract

Compliance workflows should answer:

```text
WHAT WAS SUBMITTED?
WHEN?
WITH WHAT STATUS?
WHAT FAILED?
WHAT NEEDS ACTION?
```

---

# 327. Enterprise Truth Contract

Enterprise controls should answer:

```text
WHO CAN DO WHAT?
WHO DID WHAT?
WHAT REQUIRED APPROVAL?
WHAT CHANGED?
WHAT IS THE CURRENT POLICY?
```

---

# 328. One System, Different Questions

This is the strategic reason to keep all modules together:

```text
CASHIER
“What do I sell?”

INVENTORY
“What do we have?”

FINANCE
“Where is the money?”

MANAGER
“How are we doing?”

OWNER
“Is the business healthy?”
```

---

# 329. Enterprise Product Narrative

For a small owner:

> “Run my shop.”

For a manager:

> “Run my team.”

For a multi-branch operator:

> “Control my business.”

For enterprise:

> “Connect operations, finances and governance.”

---

# 330. Final Go-to-Market Doctrine

```text
START SIMPLE
→
PROVE VALUE
→
CHARGE
→
RETAIN
→
EXPAND
→
STANDARDIZE
→
SCALE
```

Do not invert the sequence.

---

# 331. Final Product Doctrine

```text
ONE CORE
+
VERTICAL CONFIGURATION
+
PROGRESSIVE ENTERPRISE DEPTH
```

---

# 332. Final Revenue Doctrine

```text
SaaS first
Expansion second
Enterprise third
Services fourth
Ecosystem later
```

---

# 333. Final Risk Doctrine

```text
DO NOT
trade trust for short-term revenue.

DO NOT
trade product focus for feature volume.

DO NOT
trade regulatory safety for fintech excitement.

DO NOT
trade unit economics for vanity growth.
```

---

# 334. Final Enterprise Business Model Statement

> **Sitolo is a business operating system for African SMEs that starts with the operational reality of selling and stock, grows into financial control and reconciliation, expands into enterprise governance and compliance, and eventually becomes an integration point for business and financial partners. Its primary economic engine is recurring SaaS revenue, strengthened by account expansion and enterprise contracts. Its long-term moat is the combination of local fit, offline reliability, workflow depth, trusted business history, reconciliation, integration and merchant trust.**

---

# 335. Handoff Acceptance Criteria

The business model is considered sufficient for system architecture design when the architecture team can identify:

- the customer hierarchy;
- business types;
- entitlement model;
- core workflows;
- financial boundaries;
- reconciliation requirements;
- enterprise expansion points;
- integration classes;
- billing dimensions;
- audit requirements;
- support model;
- data governance requirements;
- future ecosystem boundaries.

That is the purpose of this document.


---

# 400.1 Duka Commercial Playbook


**Complexity:** Low

**Expected support intensity:** Low

**Primary value:** Mobile-first, simple POS, stock, cash, mobile money

**Commercial role:** Retention and volume


## Acquisition

Target the merchant when the current workflow is causing measurable friction.


## Activation

Define success as repeated use of the segment's critical workflow, not account creation.


## Retention

Deepen the number of workflows connected to the business record.


## Expansion

Use real business growth signals to introduce higher-value capabilities.


## Risks

Avoid over-serving this segment with capabilities that create more cost than value.



---

# 400.2 General Retail Commercial Playbook


**Complexity:** Medium

**Expected support intensity:** Medium

**Primary value:** POS, inventory, purchasing, users, payments

**Commercial role:** Core SaaS


## Acquisition

Target the merchant when the current workflow is causing measurable friction.


## Activation

Define success as repeated use of the segment's critical workflow, not account creation.


## Retention

Deepen the number of workflows connected to the business record.


## Expansion

Use real business growth signals to introduce higher-value capabilities.


## Risks

Avoid over-serving this segment with capabilities that create more cost than value.



---

# 400.3 Pharmacy Commercial Playbook


**Complexity:** High

**Expected support intensity:** Medium/High

**Primary value:** Batch, expiry, controlled access, audit

**Commercial role:** Premium vertical


## Acquisition

Target the merchant when the current workflow is causing measurable friction.


## Activation

Define success as repeated use of the segment's critical workflow, not account creation.


## Retention

Deepen the number of workflows connected to the business record.


## Expansion

Use real business growth signals to introduce higher-value capabilities.


## Risks

Avoid over-serving this segment with capabilities that create more cost than value.



---

# 400.4 Agro-dealer Commercial Playbook


**Complexity:** Medium/High

**Expected support intensity:** Medium

**Primary value:** Lots, units, seasonality, supplier traceability

**Commercial role:** Vertical expansion


## Acquisition

Target the merchant when the current workflow is causing measurable friction.


## Activation

Define success as repeated use of the segment's critical workflow, not account creation.


## Retention

Deepen the number of workflows connected to the business record.


## Expansion

Use real business growth signals to introduce higher-value capabilities.


## Risks

Avoid over-serving this segment with capabilities that create more cost than value.



---

# 400.5 Wholesale Commercial Playbook


**Complexity:** High

**Expected support intensity:** Medium/High

**Primary value:** Warehouses, bulk pricing, purchasing, receivables

**Commercial role:** High ARPU


## Acquisition

Target the merchant when the current workflow is causing measurable friction.


## Activation

Define success as repeated use of the segment's critical workflow, not account creation.


## Retention

Deepen the number of workflows connected to the business record.


## Expansion

Use real business growth signals to introduce higher-value capabilities.


## Risks

Avoid over-serving this segment with capabilities that create more cost than value.



---

# 400.6 Multi-branch Retail Commercial Playbook


**Complexity:** Very High

**Expected support intensity:** High

**Primary value:** Branches, users, consolidation, controls

**Commercial role:** Enterprise


## Acquisition

Target the merchant when the current workflow is causing measurable friction.


## Activation

Define success as repeated use of the segment's critical workflow, not account creation.


## Retention

Deepen the number of workflows connected to the business record.


## Expansion

Use real business growth signals to introduce higher-value capabilities.


## Risks

Avoid over-serving this segment with capabilities that create more cost than value.



---

# 400.7 Enterprise / Institutional Commercial Playbook


**Complexity:** Very High

**Expected support intensity:** Very High

**Primary value:** APIs, SLA, governance, integrations

**Commercial role:** High ACV


## Acquisition

Target the merchant when the current workflow is causing measurable friction.


## Activation

Define success as repeated use of the segment's critical workflow, not account creation.


## Retention

Deepen the number of workflows connected to the business record.


## Expansion

Use real business growth signals to introduce higher-value capabilities.


## Risks

Avoid over-serving this segment with capabilities that create more cost than value.



---

# 410. Commercial Metric Dictionary


| Metric | Meaning | Measurement intent |
|---|---|---|
| MRR | Monthly recurring revenue | Core SaaS revenue at month end |
| ARR | Annualized recurring revenue | MRR × 12 for recurring revenue planning |
| ARPU | Average revenue per merchant | Recurring revenue divided by active paid merchants |
| CAC | Customer acquisition cost | Acquisition spend divided by acquired customers |
| LTV | Lifetime value | Expected gross-profit contribution over retained lifetime |
| NRR | Net revenue retention | Starting cohort revenue plus expansion less contraction/churn |
| Logo churn | Customer account churn | Customers lost over a defined period |
| Activation rate | New accounts reaching meaningful operational use | Activated accounts divided by eligible new accounts |
| Time-to-value | Time to first meaningful benefit | Elapsed time from signup to defined value event |
| Gross margin | Revenue less direct delivery cost | Primary software economics indicator |
| Support cost/merchant | Variable support cost | Support effort divided by active merchants |
| Reconciliation rate | Transactions automatically reconciled | Matched transactions divided by eligible transactions |

---

# 420. Enterprise Checklists


## Merchant Launch Readiness


- [ ] Pricing defined

- [ ] Plan entitlements defined

- [ ] Billing collection path tested

- [ ] Onboarding flow documented

- [ ] Support path available

- [ ] Data export available

- [ ] Core reliability monitored

- [ ] Regulatory claims reviewed



## Enterprise Launch Readiness


- [ ] MSA ready

- [ ] DPA ready

- [ ] SLA defined

- [ ] Security questionnaire pack ready

- [ ] Implementation process ready

- [ ] Escalation process ready

- [ ] Reference account or evidence available

- [ ] Commercial approval completed



## New Payment Provider Readiness


- [ ] Commercial agreement

- [ ] API documentation

- [ ] Sandbox

- [ ] Authentication verified

- [ ] Webhook behavior verified

- [ ] Settlement behavior understood

- [ ] Failure handling defined

- [ ] Support escalation path defined



## New Country Readiness


- [ ] Local market research

- [ ] Tax requirements mapped

- [ ] Payment providers identified

- [ ] Data requirements reviewed

- [ ] Pricing validated

- [ ] Distribution partner identified

- [ ] Support plan defined

- [ ] Country economics approved




---

# 430. Evidence Notes

1. DataReportal's Digital 2026 Malawi report is based largely on data available in late 2025 and explicitly warns readers about the distinction between cellular connections and unique people. It is used here to support a **mobile-first product design conclusion**, not to claim a specific merchant market size.

2. MRA EIS documentation establishes that a POS/API integration pathway exists and that EIS has formal developer onboarding/certification processes. Sitolo should verify live production requirements and certification status before using compliance claims commercially.

3. World Bank Enterprise Survey data is evidence about formal businesses and the general enterprise environment. It should not be extrapolated directly to every ultra-micro shop.

4. Current competitor websites are used as market-positioning and pricing signals. Their performance claims and customer counts are not treated as independently verified.

5. Future financial-partner economics are treated as hypotheses until signed commercial arrangements and actual customer demand exist.

---

# 431. Final Research Sources

- DataReportal — Digital 2026: Malawi: https://datareportal.com/reports/digital-2026-malawi
- Malawi Revenue Authority — EIS Developer Resources: https://eis-portal.mra.mw/Home/DeveloperResources
- Malawi Revenue Authority — EIS API introduction: https://eis-api.mra.mw/docs/introduction_1_print.htm
- Malawi Revenue Authority — EIS overview: https://eis-api.mra.mw/docs/overview.htm
- Malawi Revenue Authority — EIS pre-integration guide: https://eis-api.mra.mw/docs/developer_pre_integration_guide_print.htm
- Malawi Revenue Authority — EIS certification documentation: https://eis-api.mra.mw/docs/introduction_3_print.htm
- World Bank — Malawi Enterprise Survey 2025: https://microdata.worldbank.org/catalog/8117/variable/V225
- World Bank — Malawi Economic Monitor: https://www.worldbank.org/en/country/malawi/publication/economic-monitor
- Airtel Africa Developer Portal: https://developers.airtel.africa/
- MalondaPlus: https://www.malonda.ictechmw.com/malondaplus/index.php
- Phindu: https://www.phindu.co/
- Phindu POS: https://www.phindu.co/pos-system-malawi
- Quick-Think Solutions: https://www.quickthinks.com/
- Stooqo: https://www.stooqo.com/
- ninoPOS: https://ninopos.com/

---

# 432. Final Handoff Statement

The business model is intentionally complete enough that the next document can focus on **system architecture design** rather than rediscovering product economics.

The next design should derive technical boundaries from:

```text
CUSTOMER VALUE
    ↓
BUSINESS WORKFLOWS
    ↓
COMMERCIAL ENTITLEMENTS
    ↓
DATA / TRUST REQUIREMENTS
    ↓
INTEGRATION REQUIREMENTS
    ↓
SYSTEM ARCHITECTURE
```

It should not reverse that order.

**END OF BUSINESS MODEL DESIGN.**
