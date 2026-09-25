# Sitolo — Current Regulatory Facts & Verification Register

**Status:** Current verification register
**Verification date:** 2026-09-25
**Scope:** Regulatory facts that can change product/compliance behavior
**Authority rule:** Applicable law and current regulator/provider instructions outrank every internal document.

---

## 1. Why this register exists

Regulatory dates and implementation instructions are time-sensitive. Internal product and commercial documents may retain historical transition dates for context, but those dates must not silently become current operating rules.

The register records the latest verified state used by Sitolo engineering and commercial documentation. Before external compliance claims or production certification, the relevant regulator source must be re-checked.

---

## 2. Malawi MRA Electronic Invoicing System

Current public reporting states that nationwide EIS rollout took effect on 1 May 2026 after the transition period, replacing the prior EFD operating model. A 6 July 2026 report citing an MRA confirmation dated 23 June 2026 reported that more than 8,260 of approximately 9,000 targeted VAT-registered businesses had migrated by June 2026. citeturn297771search1turn297771search8

The MRA's public developer resources currently expose EIS API documentation covering the integration boundary used by software systems. citeturn689436search4

### Engineering rule

For taxpayers currently subject to the applicable EIS requirements:

- do not model EFD fallback as the default current path;
- keep historical EFD/transition dates explicitly historical;
- treat MRA terminal/configuration/certification procedures as external regulator-controlled facts;
- verify live onboarding, certification, credentials and endpoint behavior before production integration or compliance claims.

### Evidence quality

The rollout date and June 2026 migration figure are supported by recent secondary reporting that cites MRA. The MRA developer resources are direct MRA evidence for the existence and shape of the EIS API material. This register does not treat the secondary source as a substitute for applicable law, official notices, or current MRA certification instructions.

---

## 3. Internal-document rule

A current internal document may reference this register for dated regulatory context, but it must not copy the register's facts into multiple independent sources when a link will do.

Where a current regulator instruction conflicts with this register:

```text
CURRENT LAW / REGULATOR
        ↓
UPDATE REGISTER
        ↓
UPDATE AFFECTED CONTRACTS
        ↓
RUN DOCUMENTATION AUDIT
```

Historical documents remain historical.

---

## 4. Reverification trigger

Reverify this register before:

- production EIS certification;
- external compliance claims;
- investor/customer compliance material;
- material EIS integration changes;
- any regulator announcement that changes rollout, taxpayer scope, API, certification or invoicing requirements.

Sources:
- MRA developer resources: https://developer.mra.mw/
- Recent reporting citing MRA: https://regfollower.com/malawi-mra-transitions-from-fiscal-devices-to-real-time-electronic-invoicing/
- MRA-linked April 2026 implementation reporting: https://www.bridgepathcapitalmw.com/wp-content/uploads/2026/04/BridgepathCapitalMalawiFinancialMarketUpdate10April2026.pdf
