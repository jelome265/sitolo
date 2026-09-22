# Sitolo — Regulatory Perimeter & Compliance Boundary

**Status:** Commercial/legal control document  
**Date:** 2026-09-22  
**Market baseline:** Malawi  
**Important:** This document is a product/compliance boundary hypothesis, not legal advice.

---

## 1. Purpose

Sitolo may eventually touch:

- payments;
- reconciliation;
- credit records;
- lending;
- identity;
- tax;
- payroll;
- financial integrations.

The company must know when a feature remains ordinary business software and when it enters regulated activity.

---

## 2. Decision Model

Every new financial/compliance feature must answer:

1. What service is Sitolo actually providing?
2. Who holds the money?
3. Who makes the financial decision?
4. Who bears the regulated obligation?
5. Which external system executes the regulated action?
6. Which law/regulator applies?
7. Is Sitolo an agent/participant, technical service provider, or regulated operator?
8. What licensing/authorization is required?

---

## 3. GREEN — Core Business Software

Examples:

- sales recording;
- stock management;
- purchasing;
- cashbook-style operational records;
- customer balances;
- supplier balances;
- reporting;
- workflow approvals;
- audit logs;
- data export;
- subscription management.

These may still involve privacy, consumer, tax or contractual obligations.

“Green” does not mean “unregulated in every respect.”

---

## 4. YELLOW — External/Partner/Legal Review Required

Examples:

- payment initiation;
- payment collection;
- mobile-money integration;
- bank integration;
- settlement reconciliation;
- financial-product referrals;
- credit bureau integration;
- identity verification;
- cross-border processing;
- payroll disbursement;
- advanced tax integrations.

These should use approved providers where regulated activity is involved.

---

## 5. RED — Do Not Implement as an Unlicensed Sitolo Activity

Potential examples:

- operating a regulated payment system/service without required authorization;
- issuing regulated payment instruments without required authorization;
- accepting public deposits;
- providing loans/microcredit as a principal where licensing is required;
- operating a regulated financial institution without the appropriate authorization.

The precise legal characterization belongs to counsel and the relevant authority.

---

## 6. Malawi Payment Boundary

The Malawi Payment Systems Act states that persons generally may not establish or operate a payment, clearing and settlement system/service, remittance service including electronic money transfer services, or mobile payment services without a Reserve Bank licence/authorization, subject to the Act's framework.

Reference: https://malawilii.org/akn/mw/act/2016/15/eng@2017-12-31

Commercial implication:

> Sitolo should integrate with authorized rails rather than assume that owning a software interface authorizes payment activity.

---

## 7. Financial Services Boundary

The Financial Services Act framework covers financial institutions and financial services and includes microcredit/microfinance concepts.

Reference: https://malawilii.org/akn/mw/act/2010/26/eng@2014-12-31

Commercial implication:

- record merchant/customer credit as business data where lawful;
- do not silently convert that record into Sitolo-funded lending;
- assess licensing before launching financing products.

---

## 8. Customer Credit vs Sitolo Credit

```text
MERCHANT RECORDS
CUSTOMER OWES MKX
        ↓
BUSINESS SOFTWARE

SITOLO FUNDS
CUSTOMER LOAN
        ↓
FINANCIAL PRODUCT
        ↓
LEGAL / LICENSING REVIEW
```

These are separate product categories.

---

## 9. Payment Collection

Subscription billing may use a payment provider.

The distinction is:

```SITOLO OWES COLLECTION OBLIGATION
+
AUTHORIZED PROVIDER EXECUTES PAYMENT
```

The final legal/provider structure must be confirmed before launch.

---

## 10. Data Protection Boundary

The Malawi Data Protection Authority states that it regulates processing of personal information under the Data Protection Act 2024 and provides registration services for applicable data controllers/processors.

Reference: https://www.dpa.mw/download/data-protection-act-2024/

A 2026 DPA notice states that processing personal information of more than 10,000 Malawi-resident data subjects, or information of significance to Malawi's economy, society or security, is within the “Data Controller or Processor of Significant Importance” registration framework described there.

Reference: https://www.dpa.mw/call-for-written-submissions-on-proposed-registration-fees-for-data-controllers-and-data-processors-of-significant-importance/

Legal/compliance review must confirm Sitolo's exact classification at the time of processing.

---

## 11. Regulatory Inventory

Maintain a live inventory:

| Capability | Law/regulator | Provider | Approval | Owner |
|---|---|---|---|---|
| payment | relevant payment framework | provider | required status | compliance |
| tax/EIS | tax framework | MRA | certification/approval as applicable | tax/integration |
| personal data | DPA framework | internal/external | registration where applicable | privacy |
| communications | MACRA framework where applicable | telecom | applicable licence/provider | legal |
| lending | financial-services framework | licensed partner | required status | compliance |

---

## 12. Regulatory Change Management

Every regulatory dependency requires:

- source;
- date checked;
- effective date;
- applicability;
- owner;
- evidence;
- next review date.

Do not rely on stale screenshots or old blog posts for compliance claims.

---

## 13. Release Gate

A regulated feature cannot reach production without:

- legal classification;
- provider contract;
- required authorization/certification;
- data-protection review;
- security review;
- operational runbook;
- customer disclosure;
- evidence stored with the release record.

---

## 14. Regulatory Marketing Claims

Never claim:

- “MRA approved” without actual evidence;
- “RBM licensed” unless the company/partner holds the relevant licence;
- “compliant” without defining which obligation;
- “banking” or “lending” if the product is merely recording or facilitating permitted software workflows.

Marketing must not outrun legal reality.

---

## 15. Regulatory Exit Strategy

If a capability becomes regulated unexpectedly:

```DETECT
→ FREEZE NEW SALES
→ PRESERVE EXISTING DATA
→ DISABLE UNAPPROVED ACTION
→ INFORM USERS
→ ENGAGE COUNSEL / REGULATOR / PARTNER
→ RECLASSIFY
```

---

## 16. Final Contract

The safest boundary is:

```SITOLO
=
BUSINESS SOFTWARE
+
CONTROL
+
INTEGRATION

NOT
=
UNLICENSED FINANCIAL INSTITUTION
```

When the product crosses the boundary, the operating model must change before the feature ships.