# Sitolo — Tax & Fiscal Readiness Strategy

**Status:** Commercial/product boundary document  
**Date:** 2026-09-22  
**Market:** Malawi first

---

## 1. Purpose

Tax requirements can materially alter:

- pricing;
- invoices;
- product catalogue;
- tax-inclusive/exclusive prices;
- stock records;
- reporting;
- integrations;
- customer onboarding.

The domain must therefore be tax-aware without assuming every merchant has identical tax status.

---

## 2. Current Malawi Evidence Baseline

The Malawi Government's 2026-27 Budget Policy Statement states that the mandatory VAT registration threshold was proposed to increase from MK25 million to MK50 million annual turnover, with the policy stated as subject to legislative approval. It also states that businesses below the new threshold would not be required to operate under EIS, while businesses exceeding the threshold would be subject to VAT/EIS requirements under the stated policy.

Reference: https://www.finance.gov.mw/documents/uploads/2026-03/Doc%201_2026-27%20Budget%20Policy%20Statement_0.pdf

MRA's EIS public notice states that the transition from EFDs to EIS ended on 31 January 2026 and describes EIS as a software-based system for electronic tax invoices, stock records and real-time transmission.

Reference: https://www.mra.mw/admin/storage/download_files/1769007736_003%20TRANSITION%20FROM%20ELECTRONIC%20FISCAL%20DEVICES%20TO%20THE%20ELECTRONIC%20INVOICING%20SYSTEM.pdf

These are dated source signals. Product code must not hard-code them as permanent facts without checking the applicable law and current MRA guidance.

---

## 3. Tax Status Model

A merchant may be:

- non-VAT registered;
- VAT registered;
- subject to other tax obligations;
- exempt for specific supplies;
- in transition;
- unknown pending verification.

The product must support explicit state.

---

## 4. Price Semantics

A sale line may need to distinguish:

```BASE PRICE
+
DISCOUNT
+
TAX
=
TOTAL
```

Where relevant, store enough immutable snapshot data to reproduce the final calculation.

Do not use a single ambiguous “price” field for all fiscal contexts.

---

## 5. Tax Configuration

Tax configuration should support:

- effective date;
- jurisdiction;
- tax code;
- rate;
- inclusion/exclusion;
- product applicability;
- exemption/zero-rating classification where applicable;
- rounding policy;
- source/evidence;
- version.

---

## 6. Historical Tax Correctness

When tax rules change:

```OLD TRANSACTION
→ OLD SNAPSHOT

NEW TRANSACTION
→ NEW SNAPSHOT
```

Do not recalculate old finalized transactions using a new rule unless an explicit correction process requires it.

---

## 7. Invoice Requirements

Where a merchant is required to issue tax invoices, Sitolo must use the approved integration path.

The product should distinguish:

- internal sale;
- customer receipt;
- tax invoice;
- fiscal response;
- submission status;
- rejection;
- correction.

---

## 8. MRA EIS Boundary

EIS integration is an external regulated boundary.

The product must account for:

- onboarding/activation;
- provider/API state;
- submission;
- acknowledgement;
- rejection;
- retry;
- unknown state;
- audit evidence.

Do not represent an invoice as fiscally accepted merely because Sitolo successfully sent an HTTP request.

---

## 9. Tax Readiness for Duka

A small merchant may not have the same fiscal obligations as a formal VAT/EIS customer.

The UI should therefore avoid presenting advanced tax configuration as mandatory for everyone.

Progressive configuration:

```
BASIC BUSINESS
→ TAX STATUS WHEN RELEVANT
→ TAX CONFIGURATION
→ EIS WHEN REQUIRED
```

---

## 10. Tax Readiness for Enterprise

Enterprise customers may require:

- multiple tax registrations;
- entity-level reporting;
- branch allocation;
- central tax reports;
- integration evidence;
- audit exports.

These requirements should be modelled without changing the core sales semantics.

---

## 11. Compliance Claims

Marketing must distinguish:

- “supports EIS integration”;
- “integrated with EIS”;
- “certified/approved,” only when evidence exists;
- “tax-ready,” when the actual supported obligations are listed.

---

## 12. Tax Decision Gate

Before changing tax behaviour:

1. obtain current legal source;
2. identify effective date;
3. identify affected merchants;
4. update configuration model;
5. update tests;
6. update external integration contract if required;
7. define migration/rollout;
8. store evidence.

---

## 13. Commercial Effects of Tax

Tax changes may alter:

- displayed price;
- merchant margin perception;
- invoice value;
- Sitolo's own tax obligations;
- customer willingness to pay.

Commercial experiments must clearly specify whether price is:

- tax-inclusive;
- tax-exclusive;
- tax-treatment dependent.

---

## 14. Final Contract

Tax is configuration plus immutable historical evidence.

```text
CURRENT TAX RULE
→ APPLIED TO NEW TRANSACTION

HISTORICAL TRANSACTION
→ PRESERVES ITS ORIGINAL TAX SNAPSHOT
```