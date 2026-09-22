# Sitolo — Merchant Onboarding & Migration Strategy

**Status:** Commercial operating-model control document  
**Date:** 2026-09-22  
**Primary audience:** product, growth, onboarding, support, implementation  
**Segments:** Duka through enterprise

---

## 1. Purpose

The first interaction with Sitolo determines whether the merchant reaches first value.

The design must support:

- zero-data onboarding;
- opening-state setup;
- assisted onboarding;
- historical migration;
- device replacement;
- staff rollout;
- multi-branch rollout.

The default for a Duka is not “import everything.”

It is:

```START SMALL
→ RECORD REAL ACTIVITY
→ ADD DETAIL AS NEEDED
```

---

## 2. First-10-Minute Contract

The target is:

```DOWNLOAD
→ CREATE BUSINESS
→ ADD 3–10 PRODUCTS
→ RECORD OPENING STOCK
→ MAKE FIRST SALE
→ SEE FIRST SUMMARY
```

Measure elapsed time from application start to first successful value event.

---

## 3. Progressive Onboarding

### Step 1 — Identity

Create/authenticate account.

### Step 2 — Business

Capture minimum business identity.

### Step 3 — Business type

Use a small set of choices to configure workflow defaults.

### Step 4 — First catalogue

Add a few frequently sold products.

### Step 5 — Opening stock

Optional simplified stock entry.

### Step 6 — First sale

The central activation event.

### Step 7 — Value feedback

Show sales, remaining stock and/or business summary.

### Step 8 — Progressive setup

Introduce:

- suppliers;
- customers;
- staff;
- payment methods;
- reporting;
- reconciliation.

Do not front-load every setting.

---

## 4. Zero-Data Mode

A merchant must be able to start with no historical import.

Required minimum:

- business name;
- business type;
- currency;
- one user;
- at least one product before a product-based sale.

Opening values may be zero where appropriate.

The system should not force estimates merely to satisfy a form.

---

## 5. Product Entry UX

Allow rapid entry:

- name;
- selling price;
- optional cost;
- unit;
- optional barcode;
- optional initial quantity.

Advanced fields remain available without blocking the first sale.

---

## 6. Opening Inventory

Opening inventory may be entered as:

- quantity only;
- quantity + known cost;
- detailed lots/batches where relevant;
- no opening stock.

Uncertain historical data must be labelled as opening state, not reconstructed historical transactions.

---

## 7. Opening Cash

Allow an explicit opening cash balance where the product supports it.

The record should identify:

- amount;
- effective business date/time;
- actor;
- reason;
- approval requirement where applicable.

Do not fabricate prior transactions to explain an opening balance.

---

## 8. Opening Receivables

For existing customer credit:

- customer;
- outstanding amount;
- optional original date;
- source/reference;
- notes within safe data limits.

The merchant can start from a balance without recreating every historical sale.

---

## 9. Opening Payables

Similarly:

- supplier;
- outstanding amount;
- optional source date;
- reference;
- opening-state classification.

---

## 10. Historical Migration

Historical import should be a separate workflow.

Pipeline:

```SOURCE
→ VALIDATE
→ PARSE
→ PREVIEW
→ RECONCILE
→ APPROVE
→ IMPORT
→ AUDIT
```

Never import unvalidated data directly into authoritative financial history.

---

## 11. CSV Migration

CSV import must address:

- encoding;
- header mapping;
- missing values;
- duplicate rows;
- invalid amounts;
- invalid dates;
- suspicious formulas;
- unknown products;
- unknown customers;
- delimiter differences;
- large-file limits.

Existing import security rules remain authoritative.

---

## 12. Paper-to-Sitolo Migration

For notebook merchants, default to:

```PAPER
→ CURRENT OPENING STATE
→ SITOLO
```

Do not ask for years of records unless there is a business reason.

Offer optional assistance for:

- opening inventory;
- customer balances;
- supplier balances;
- cash;
- bank/mobile-money balances.

---

## 13. Assisted Migration

For SME/Growth and above, assisted migration may include:

- data preparation;
- mapping;
- validation;
- trial import;
- final cutover.

Price assisted migration when its labour materially exceeds standard onboarding capacity.

---

## 14. Enterprise Migration

Enterprise migration requires:

- source inventory;
- source-system ownership;
- data dictionary;
- mapping specification;
- sample extract;
- reconciliation rules;
- cutover plan;
- rollback plan;
- sign-off;
- post-cutover validation.

Do not promise “zero data loss” without defining exactly what data is in scope.

---

## 15. Cutover Strategy

A safe cutover is:

```DISCOVER
→ FREEZE SOURCE
→ EXPORT
→ VALIDATE
→ IMPORT
→ VERIFY
→ GO LIVE
→ MONITOR
```

Large migrations need a rollback/recovery path.

---

## 16. Migration Quality Metrics

Track:

- source rows;
- accepted rows;
- rejected rows;
- duplicate rate;
- unresolved mappings;
- monetary reconciliation difference;
- stock reconciliation difference;
- import duration;
- operator time;
- post-cutover defects.

---

## 17. Device Change During Onboarding

A merchant may begin on one device and finish on another.

The organization is the persistent business identity.

Do not make migration dependent on a device remaining online for the entire process.

---

## 18. Staff Rollout

For multiple users:

```OWNER
→ CONFIGURE
→ INVITE
→ ASSIGN ROLE
→ TRAIN
→ FIRST TASK
→ REVIEW
```

Permissions should be role-specific.

---

## 19. Multi-Branch Onboarding

Do not create every branch manually through repetitive setup when a central configuration model can safely template common settings.

Still require explicit identity and scope for every branch.

---

## 20. Onboarding Failure Recovery

When onboarding fails:

- preserve the valid business record;
- preserve entered data where safe;
- explain what failed;
- permit resume;
- do not silently duplicate organizations;
- provide support escalation.

Idempotency matters commercially.

---

## 21. First-Value Metrics

By segment:

- time to first sale;
- first-day completion;
- first-week activity;
- product-entry count;
- repeat-sale rate;
- inventory usage;
- payment setup;
- reconciliation setup.

---

## 22. Onboarding Economics

```
ONBOARDING COST
=
HUMAN TIME
+
TRAVEL
+
TRAINING
+
PARTNER COST
+
DATA PREPARATION
```

Duka onboarding must be substantially lighter than enterprise implementation.

---

## 23. Activation Failure Taxonomy

Record:

- too complicated;
- no suitable device;
- connectivity;
- payment;
- product entry;
- migration confusion;
- staff resistance;
- trust;
- support delay;
- missing feature;
- business not ready.

Do not treat every failed activation as “poor demand.”

---

## 24. Final Onboarding Contract

```text
LOWEST-COST PATH:
START FRESH

HIGHER-COMPLEXITY PATH:
OPENING STATE

ENTERPRISE PATH:
CONTROLLED MIGRATION
```

The product must let a merchant start operating before demanding historical perfection.