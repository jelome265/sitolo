# Sitolo — Buyer, User, Approver & Purchase Process Model

**Status:** Commercial operating-model control document  
**Date:** 2026-09-22  
**Scope:** Duka through enterprise  
**Classification:** Commercial design / validation contract

---

## 1. Purpose

A Sitolo customer is not a single persona.

The commercial system must separate:

- economic buyer;
- user;
- approver;
- payer;
- administrator;
- beneficiary;
- influencer;
- procurement owner;
- technical/security reviewer.

The separation matters because product adoption can succeed while the sale fails if the person receiving value cannot authorize spend, or if the person paying cannot see the operational value.

---

## 2. Canonical Actor Definitions

| Actor | Definition | Commercial significance |
|---|---|---|
| Economic buyer | person controlling the purchase decision or budget | must perceive measurable value |
| User | person performing work in Sitolo | drives activation and product fit |
| Approver | person allowed/required to approve purchase or high-risk actions | can block adoption |
| Payer | person/account executing payment | collection path must fit them |
| Administrator | person configuring users, plans and business settings | controls operational setup |
| Beneficiary | person/group receiving business outcome | value proof may be indirect |
| Influencer | person who materially shapes the purchase | often trusted intermediary |
| Procurement | function controlling supplier/vendor process | enterprise sales dependency |
| IT/security reviewer | evaluates technical and security risk | enterprise acceptance dependency |
| Finance reviewer | evaluates commercial/financial control | approval and renewal dependency |

One person may hold multiple roles.

The data model must not assume the roles are always distinct or always identical.

---

## 3. Segment Actor Matrix

| Segment | Buyer | Users | Approvers | Payer | Influencers |
|---|---|---|---|---|---|
| Duka | owner/operator | owner/operator | owner or partner | owner | family/peer/agent |
| Small SME | owner | owner + staff | owner | owner/business | accountant |
| Growth SME | owner/director | staff + managers | owner/finance | business | accountant, operations |
| Multi-Branch | director/owner/group management | branch teams + managers | finance/operations | central finance | accountant, IT |
| Enterprise | business/procurement | workers + managers | finance, IT/security, legal, procurement | central finance/procurement | executives, integrators |

This is the initial model, not verified market truth.

---

## 4. Buyer/User Disconnect Risks

### Risk A — User loves it, buyer does not

Example:

Cashiers use Sitolo daily, but management sees no quantified improvement.

Required response:

- quantify time saved;
- quantify stock-control improvement;
- quantify reconciliation coverage;
- show owner-level reporting;
- connect product usage to financial outcomes.

### Risk B — Buyer loves it, users reject it

Example:

Management buys a complex system that frontline workers cannot operate.

Required response:

- progressive disclosure;
- role-specific UX;
- short onboarding;
- mobile-first frontline workflows;
- task-based usability measurement.

### Risk C — Buyer and payer differ

Example:

Finance pays, owner expects benefit.

Required response:

- invoicing addressed to payer;
- business-value reporting to buyer;
- operational access for users;
- clear renewal ownership.

### Risk D — Approver blocks adoption

Example:

Security rejects integration, or finance rejects unknown recurring charges.

Required response:

- evidence package;
- security documentation;
- contract clarity;
- implementation boundaries;
- predictable billing.

---

## 5. Duka Purchase Path

Expected path:

```text
DISCOVER
→ TRY
→ SEE FIRST VALUE
→ DECIDE
→ PAY
→ USE
→ RENEW
```

The owner should be able to move through the path without a procurement department.

The commercial question is not whether the owner can understand every feature. It is whether the owner can understand:

- what problem is solved;
- what it costs;
- how to start;
- how to pay;
- what happens when offline;
- how records are protected;
- how to recover the account.

---

## 6. SME Purchase Path

```text
OWNER IDENTIFIES PROBLEM
        ↓
STAFF / MANAGER TRIAL
        ↓
VALUE REVIEW
        ↓
OWNER APPROVAL
        ↓
PAYMENT
        ↓
ROLLOUT
        ↓
RENEWAL / EXPANSION
```

Sales materials should speak separately to:

- owner economics;
- manager control;
- staff usability.

---

## 7. Multi-Branch Purchase Path

```text
CENTRAL MANAGEMENT
        ↓
BRANCH NEED DISCOVERY
        ↓
OPERATIONAL / FINANCE REVIEW
        ↓
PILOT ONE OR FEW BRANCHES
        ↓
CONTROL / RECONCILIATION PROOF
        ↓
ROLLOUT
        ↓
CENTRAL BILLING
```

The pilot must prove cross-branch visibility without requiring premature full deployment.

---

## 8. Enterprise Purchase Path

```text
BUSINESS SPONSOR
        ↓
DISCOVERY
        ↓
SECURITY / IT REVIEW
        ↓
FINANCE / PROCUREMENT
        ↓
LEGAL / CONTRACT
        ↓
PILOT / UAT
        ↓
IMPLEMENTATION
        ↓
PRODUCTION
        ↓
RENEWAL / EXPANSION
```

Enterprise sales must therefore produce more than feature demonstrations.

Required evidence may include:

- architecture;
- security controls;
- data handling;
- tenancy model;
- auditability;
- integration contracts;
- support/SLA model;
- recovery expectations;
- pricing and implementation terms.

---

## 9. Buyer Discovery Questions

Every commercial interview should establish:

1. Who decides whether Sitolo is purchased?
2. Who uses it daily?
3. Who pays?
4. Who approves recurring spend?
5. Who controls access?
6. Who owns finance/reconciliation?
7. Who would stop the purchase?
8. What evidence is required before payment?
9. Who renews the contract?
10. Who decides to add branches/users/modules?

Unknown answers are data gaps.

---

## 10. Approval Friction Map

Record every material commercial blocker:

| Blocker | Example | Owner |
|---|---|---|
| budget | price not approved | buyer |
| trust | concern about data/control | product/security |
| payment | no suitable collection method | billing |
| procurement | vendor process unavailable | sales |
| technical | integration/security requirement | engineering |
| legal | contract clause | legal |
| operational | staff training burden | onboarding |
| regulatory | required approval unclear | compliance |
| hardware | device limitation | product/channel |

Do not classify all lost deals as “price.”

---

## 11. Decision Authority

Commercial communications must make clear who can:

- start a trial;
- purchase;
- change plan;
- add users;
- add branches;
- authorize integrations;
- request export;
- cancel;
- reactivate.

The platform's authorization model remains authoritative for actual system actions.

Commercial actor data does not bypass security controls.

---

## 12. Expansion Decision Makers

Expansion events often change the buyer.

Example:

```text
DUKA
owner = buyer + user

SME
owner = buyer
manager = user/controller

MULTI-BRANCH
director = buyer
finance = payer/reviewer
branch managers = operators

ENTERPRISE
procurement = commercial gate
finance = payer
IT/security = technical gate
management = sponsor
workers = users
```

Expansion instrumentation must therefore capture role changes.

---

## 13. Renewal Ownership

Renewal must not depend on the original salesperson remembering the account.

Store:

- renewal owner;
- payer;
- invoice contact;
- operational sponsor;
- technical contact where applicable;
- renewal date;
- plan;
- current contribution;
- support load;
- usage;
- open risks;
- expansion opportunities.

For low-touch Duka accounts this should be system-driven.

---

## 14. Commercial Metrics

Per account:

- buyer identified;
- payer identified;
- user count;
- approver count;
- time to purchase;
- purchase friction events;
- payment method;
- activation after purchase;
- renewal rate;
- expansion rate.

Per segment:

- buyer/user mismatch rate;
- approval loss rate;
- median sales cycle;
- procurement cycle time;
- conversion;
- retention;
- expansion.

---

## 15. Anti-Patterns

Reject:

- “owner is always the buyer”;
- “user is always payer”;
- “enterprise just needs a bigger feature list”;
- treating security review as a late-stage surprise;
- treating procurement delay as product churn;
- collecting no role data in commercial CRM;
- granting commercial roles more technical authority than the security model permits.

---

## 16. Validation Gate

The model is validated only when a meaningful sample of paid opportunities has documented:

- buyer;
- users;
- approver;
- payer;
- decision path;
- purchase blocker;
- payment mechanism;
- renewal owner.

Until then, actor relationships remain hypotheses.

---

## 17. Final Contract

```text
THE PERSON WHO USES SITOLO
≠
THE PERSON WHO BUYS SITOLO
≠
THE PERSON WHO PAYS SITOLO
≠
THE PERSON WHO APPROVES SITOLO
```

The commercial system must know the difference.