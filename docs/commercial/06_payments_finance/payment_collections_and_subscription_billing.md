# Sitolo — Payment, Collections & Subscription Billing Model

**Status:** Commercial operating-model control document  
**Date:** 2026-09-22  
**Market:** Malawi first  
**Technical dependency:** `payment_integration_spec.md`, `domain_model.md`, security and financial-integrity contracts

---

## 1. Purpose

Sitolo pricing is incomplete until the company can reliably collect it.

This document defines the commercial lifecycle for subscription billing and collection without turning the Sitolo core into an unlicensed financial service.

It covers:

- who collects;
- who pays;
- payment methods;
- recurring billing;
- assisted/manual collection;
- payment failure;
- grace periods;
- suspension;
- refunds;
- reconciliation;
- provider failure;
- settlement uncertainty;
- collection economics.

---

## 2. Regulatory Boundary

Malawi's Payment Systems Act places payment-system/service, mobile payment and certain electronic money-transfer activity inside a Reserve Bank licensing/authorization perimeter. Sitolo therefore must use appropriately authorized providers/partners for regulated payment activity and must not treat “API access” as permission to operate a regulated payment service. The Act also distinguishes payment instructions, system operators, participants and settlement.

Source: Malawi Payment Systems Act, Chapter 74:01 — MalawiLII: https://malawilii.org/akn/mw/act/2016/15/eng@2017-12-31

This document is a commercial control model, not legal advice.

---

## 3. Core Principle

```text
BILLING
≠
PAYMENT INITIATION
≠
PAYMENT CONFIRMATION
≠
SETTLEMENT
```

The subscription ledger must preserve those states independently.

---

## 4. Commercial Payment Actors

| Actor | Responsibility |
|---|---|
| Merchant organization | owes subscription |
| Payer | executes payment |
| Sitolo billing system | creates invoice/billing obligation |
| Payment provider | processes payment through its authorized rail |
| Settlement institution/provider | moves/settles funds |
| Sitolo reconciliation | matches payment/settlement to billing |
| Support | resolves exceptions |
| Finance | reconciles books and collections |

Sitolo must not rely on the merchant to prove a payment that a provider can independently verify.

---

## 5. Duka Collection Requirement

The Duka plan is a planning hypothesis at MK7,500/month.

The collection experience should be validated without assuming a bank card.

Candidate methods:

- mobile-money collection through an authorized provider/partner;
- bank transfer;
- assisted payment;
- card/online payment when economically useful.

The final mix depends on provider capabilities, fees, settlement, legal structure and observed merchant behaviour.

---

## 6. Billing State Machine

```text
DRAFT
  ↓
ISSUED
  ↓
DUE
  ↓
PAYMENT_PENDING
  ↓
+------------------------------+
|                              |
CONFIRMED                   FAILED
  |                            |
SETTLEMENT_PENDING         RETRY / GRACE
  |                            |
SETTLED                    +---+---+
  |                        |       |
  |                       PAID   EXPIRED
  |                                |
ACTIVE / RENEWED               SUSPENDED
                                   |
                                REACTIVATE
```

Provider-specific status values must be normalized.

Unknown provider states must remain unknown.

---

## 7. Payment Intent Model

A payment intent should carry:

- billing obligation ID;
- organization ID;
- amount;
- currency;
- payment method;
- provider;
- provider reference;
- idempotency key;
- created timestamp;
- expiry;
- current normalized state;
- failure class;
- reconciliation state.

The client cannot finalize payment success.

---

## 8. Recurring Billing

Recurring billing should be server-controlled.

At the billing boundary:

1. generate the next billing obligation;
2. determine whether automatic collection is authorized;
3. initiate provider transaction where appropriate;
4. wait for authoritative confirmation;
5. record the result;
6. reconcile settlement;
7. advance the subscription only from valid evidence.

Never extend paid entitlement from client-side UI state.

---

## 9. Manual / Assisted Collection

Manual collection exists for markets where automated payment may not cover every customer.

A manual receipt must include:

- organization;
- amount;
- payer;
- collection channel;
- collector/agent identity;
- date/time;
- reference;
- evidence;
- verification state;
- approval where required.

Cash collection should be tightly controlled.

The platform should avoid creating a parallel informal cash economy inside Sitolo.

---

## 10. Cash Collection Risk

Cash creates:

- theft exposure;
- reconciliation workload;
- falsified receipts;
- delayed remittance;
- disputes;
- operational cost.

Therefore:

```text
CASH COLLECTION
→ CONTROLLED EXCEPTION
NOT
→ DEFAULT AUTOMATED BILLING RAIL
```

Any agent cash model requires a separate operating playbook.

---

## 11. Payment Failure Taxonomy

Separate:

- insufficient funds;
- customer declined;
- provider timeout;
- provider outage;
- network failure;
- invalid payer credentials;
- expired authorization;
- duplicate attempt;
- amount mismatch;
- unknown outcome;
- settlement delay;
- suspected fraud;
- provider reversal.

Failure reasons must be structured, not free-text only.

---

## 12. Retry Policy

Retries must be:

- bounded;
- idempotent;
- observable;
- provider-compatible;
- backoff-controlled.

Recommended conceptual sequence:

```text
IMMEDIATE FAILURE
→ bounded retry
→ exponential backoff
→ grace state
→ human/self-service recovery
```

Do not retry blindly on ambiguous outcomes.

A provider timeout may mean the transaction succeeded remotely.

---

## 13. Unknown Outcome

The most dangerous state is:

```text
SITOLO DOES NOT KNOW
WHETHER THE PAYMENT SUCCEEDED
```

Correct response:

- retain pending state;
- reconcile provider status;
- use webhook/query mechanisms;
- prevent duplicate financial effect;
- display safe user messaging;
- never assume success.

---

## 14. Grace Period

Grace is a commercial policy, not a technical accident.

Define:

- start condition;
- duration;
- allowed operations;
- blocked premium operations;
- reminders;
- retry schedule;
- expiry action;
- reactivation rule.

Do not delete business data on subscription expiry.

---

## 15. Grace-Period Product Design

A low-ARPU merchant can be damaged if billing failure suddenly destroys access to operational history.

Preferred model:

```text
PAST DUE
→ CONTINUITY + COLLECTION
→ EXPIRED
→ ENTITLEMENT RESTRICTION
```

Potential restrictions may include:

- plan-limited features;
- new-user/branch additions;
- advanced reporting;
- integrations.

Critical historical access should be handled according to approved retention and contract rules.

---

## 16. Subscription Suspension

Suspension must change entitlements, not financial history.

Preserve:

- sales;
- inventory;
- payments;
- reconciliation;
- audit events;
- invoices;
- exports;
- organizational identity.

Reactivation must not create a new organization.

---

## 17. Refunds

Refunds require an explicit reason and reference.

Possible causes:

- duplicate charge;
- service failure;
- commercial exception;
- contract termination;
- incorrect amount.

A refund must update the billing/revenue record through compensating entries.

Never silently rewrite historical billing events.

---

## 18. Provider Settlement

```text
CUSTOMER PAYS
  ↓
PROVIDER CONFIRMS
  ↓
SETTLEMENT
  ↓
SITOLO RECONCILES
```

Provider confirmation does not automatically equal settled cash.

Where provider settlement reports are available, reconciliation should compare:

- provider transaction;
- billing obligation;
- provider fee;
- expected net;
- actual settled amount;
- settlement date.

---

## 19. Payment Provider Adapter Boundary

The commercial system depends on a provider abstraction.

Provider adapters must isolate:

- API contracts;
- provider status codes;
- webhooks;
- retries;
- signatures;
- secrets;
- settlement reports;
- fee schedules.

Provider specifics belong in `payment_integration_spec.md`.

---

## 20. Provider Failure

When a provider is unavailable:

- existing subscriptions remain in their current state;
- new payment attempts may be queued or safely failed;
- customer messaging must distinguish provider outage from customer failure;
- no duplicate attempt should be created from repeated taps;
- reconciliation resumes when the provider recovers.

---

## 21. Collection Economics

For each payment method:

```text
NET COLLECTION VALUE
=
SUBSCRIPTION PRICE
−
PAYMENT FEE
−
FX / TRANSFER COST
−
FAILED COLLECTION COST
−
SUPPORT COST
−
RECONCILIATION COST
```

A method that increases conversion but consumes excessive variable cost may still be uneconomic.

---

## 22. Duka Payment Gate

The MK7,500 hypothesis passes only when tested evidence supports:

- acceptable payment conversion;
- acceptable payment success rate;
- acceptable fee burden;
- acceptable retry burden;
- low support cost;
- acceptable delinquency;
- positive contribution.

---

## 23. Non-Card Principle

A bank card is not a Sitolo product invariant.

The commercial architecture must support payment methods appropriate to the local market and the actual authorized provider ecosystem.

“Card required” should be treated as a deliberate commercial decision, not a default assumption.

---

## 24. Payment Reconciliation Dashboard

Finance should see:

- billed;
- due;
- collected;
- pending;
- failed;
- in grace;
- suspended;
- refunded;
- provider fees;
- settled;
- unmatched.

The dashboard must distinguish transaction count from monetary value.

---

## 25. Collection Metrics

By segment and method:

- invoice-to-payment conversion;
- first-attempt success;
- eventual success;
- median collection time;
- failure rate;
- grace rate;
- suspension rate;
- reactivation rate;
- payment fee as % of revenue;
- manual collection share;
- support cost per payer.

---

## 26. Anti-Patterns

Reject:

- card-only assumption;
- trusting client-side success;
- treating provider timeout as failure without reconciliation;
- charging the same customer twice;
- silent subscription expiry;
- deleting data after suspension;
- cash collection without evidence;
- mixing subscription revenue and settlement status;
- provider-specific states leaking into domain semantics.

---

## 27. Implementation Dependencies

Any runtime implementation must follow:

- `payment_integration_spec.md`;
- `domain_model.md`;
- `api_contract.md`;
- security implementation;
- audit rules;
- entitlement/billing rules.

This document does not authorize production payment processing by itself.

---

## 28. Final Contract

```text
BILL → COLLECT → CONFIRM → SETTLE → RECONCILE
```

Every transition must be observable and independently auditable.