# Sitolo — Vertical Module Commercial Model

**Status:** Product/commercial architecture document  
**Date:** 2026-09-22  
**Scope:** business types without product forks

---

## 1. Core Rule

Sitolo is one platform.

```CORE BUSINESS OS
+
VERTICAL CONFIGURATION
+
VERTICAL MODULES
+
PLAN ENTITLEMENTS
```

Industry does not automatically determine tier.

Complexity determines tier.

---

## 2. Shared Core

All suitable businesses share:

- organization;
- branch;
- users;
- products;
- pricing;
- purchasing;
- inventory;
- sales;
- cash;
- payments;
- reconciliation;
- reporting;
- audit;
- exports.

---

## 3. Vertical Extension

A vertical may add:

- workflow;
- fields;
- validation;
- reports;
- specialized inventory state;
- approvals;
- domain-specific integrations.

It must not fork the core sale/inventory/payment model without documented justification.

---

## 4. Pharmacy

Potential workflows:

- batch/lot;
- expiry;
- FEFO;
- regulated-product controls;
- specialized procurement;
- dispensing-related records.

These require domain validation and, where applicable, regulatory review.

---

## 5. Agro-Dealer

Potential workflows:

- seasonal inventory;
- batch/lot;
- supplier programs;
- unit conversions;
- campaign/season tracking;
- stock aging.

Do not infer that every agro-dealer needs every extension.

---

## 6. Wholesale

Potential workflows:

- bulk pricing;
- customer-specific price lists;
- credit terms;
- delivery;
- large procurement;
- sales-order workflows.

---

## 7. Hospitality / Food Service

Potential workflows may include:

- recipe/bill-of-materials concepts;
- wastage;
- kitchen consumption;
- order routing.

These belong to a future validated module.

---

## 8. Specialist Retail

Examples:

- hardware;
- electronics;
- spare parts;
- fashion.

Potential extensions may include serials, variants or warranty records where justified.

---

## 9. Module Activation

Vertical modules should be activated by:

- business type;
- explicit merchant choice;
- plan entitlement;
- required compliance status.

Do not expose irrelevant complexity by default.

---

## 10. Pricing Model

A vertical module may be:

- included;
- add-on;
- enterprise/custom;
- partner-delivered.

Pricing must reflect incremental value and support/compliance cost.

---

## 11. Vertical + Tier Matrix

```BUSINESS TYPE
        +
OPERATIONAL COMPLEXITY
        +
COMPLIANCE REQUIREMENTS
        +
MODULE NEED
        ↓
PLAN / PACKAGE
```

A small pharmacy may remain on a simple plan with one module.

A large pharmacy chain may require Multi-Branch/Enterprise controls.

---

## 12. Module Boundaries

Each module must own:

- domain concepts;
- invariants;
- persistence boundaries;
- authorization expectations;
- events;
- reporting needs.

Existing architecture rules prohibit cross-module ownership ambiguity.

---

## 13. No Vertical Product Fork

Reject:

```
SITOLO-PHARMACY
SITOLO-AGRO
SITOLO-WHOLESALE
```

Prefer:

```
SITOLO
├── CORE
├── PHARMACY
├── AGRO
└── WHOLESALE
```

---

## 14. Commercial Validation

A vertical module requires evidence of:

- recurring problem;
- sufficient customer concentration;
- willingness to pay;
- retention benefit;
- support economics;
- compliance feasibility.

Build depth only after demand exists.

---

## 15. Final Contract

Verticalization means:

> different workflows on the same business operating system.

It does not mean creating separate products.