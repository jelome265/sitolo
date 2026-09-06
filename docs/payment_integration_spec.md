# SITOLO — Payment Integration & Reconciliation Specification

**Document:** `payment_integration_spec.md`  
**Phase:** 0 — Architecture / Contracts / ADR Freeze  
**Sequence:** File 07 of 16  
**Status:** Implementation-governing payment and reconciliation specification  
**Product:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first; controlled African regional expansion  
**Backend authority:** Rust + Axum + Tokio  
**Authoritative persistence:** PostgreSQL  
**Offline operational store:** SQLite  
**Primary mobile surface:** Flutter / Android-first  
**Desktop:** Tauri  
**Web/admin:** TypeScript where materially useful  
**Architecture:** Modular monolith first; workers and provider adapters; selective service extraction only when independently justified

> This document defines how Sitolo represents, initiates, observes, verifies, reconciles, audits and corrects payment state. Sitolo is not a bank, wallet, card network, or payment processor by default. It is a merchant operating system that records sales, creates payment intents where appropriate, receives external payment evidence, reconciles provider state with merchant transactions, and maintains authoritative internal business state.

---

# 0. Executive Decision

The payment architecture is:

```text
                         SITOLO PAYMENT SYSTEM

                         CUSTOMER / MERCHANT
                                |
                                v
                         Flutter / Tauri / Web
                                |
                                | intent
                                v
                         Rust Payment Domain
                                |
                    +-----------+-----------+
                    |                       |
                    v                       v
             PostgreSQL             Provider Adapter
             authoritative              |
                    |                    +---- PayChangu
                    |                    +---- Future Provider
                    |                    +---- Bank
                    |                    +---- Direct Mobile Money
                    |
                    v
                Outbox
                    |
                    v
              Async workers
                    |
                    +---- provider API
                    |
                    <---- webhook / polling / statement
                    |
                    v
             verification
                    |
                    v
             reconciliation
                    |
                    v
           payment state transition
                    |
                    v
             sale settlement
```

The central rule is:

> **The client never proves payment. The payment provider's authenticated evidence is not automatically enough either; Sitolo verifies that evidence against the expected payment intent and internal transaction state before granting business value.**

Current PayChangu documentation explicitly requires webhook request validation through the `Signature` header and recommends re-querying the provider API to verify webhook details before confirming an order. citeturn456905search1

---

# 1. Payment Is a Trust Boundary

Payment is not a normal CRUD domain.

The following inputs are untrusted until verified:

```text
client payment status
client transaction reference
client amount
client currency
client provider
provider callback
provider status
provider metadata
merchant-entered reference
mobile number
redirect/browser result
```

The server must reconstruct the financial decision from authoritative data.

Example:

```text
UNTRUSTED CLIENT
    |
    | "payment_status=success"
    | "amount=10000"
    | "provider_ref=ABC"
    v
RUST
    |
    +--> lookup payment intent
    +--> compare expected amount
    +--> compare expected currency
    +--> verify authenticated provider evidence
    +--> re-query provider where contract requires
    +--> check duplicate provider event/transaction
    +--> check legal state transition
    +--> reconcile
    v
AUTHORITATIVE PAYMENT STATE
```

---

# 2. Scope

This document covers:

1. payment domain terminology;
2. payment methods;
3. payment intent lifecycle;
4. payment attempt lifecycle;
5. provider abstraction;
6. provider credentials;
7. provider authentication;
8. initialization;
9. webhook processing;
10. webhook authentication;
11. webhook replay protection;
12. provider verification;
13. polling fallback;
14. reconciliation;
15. settlement;
16. refunds;
17. chargebacks where supported;
18. payment allocation;
19. partial payments;
20. split payments;
21. overpayments;
22. underpayments;
23. duplicate payments;
24. ambiguous payments;
25. payment failure;
26. offline cash;
27. offline mobile-money constraints;
28. client-side payment security;
29. idempotency;
30. concurrency;
31. financial invariants;
32. audit;
33. observability;
34. security tests;
35. failure handling;
36. provider outages;
37. provider contract changes;
38. credentials and secret rotation;
39. support/reconciliation operations;
40. release gates.

This document does not establish the legal status of Sitolo as a payment service. Any regulated payment activity outside ordinary merchant collection/orchestration requires separate legal, regulatory and provider-contract review.

---

# 3. Product Position

Sitolo's existing product model defines payment categories:

```text
CASH
MOBILE_MONEY
BANK_TRANSFER
CARD
CREDIT_ACCOUNT
OTHER
```

Provider-specific behavior belongs behind an adapter boundary.

The commercial role of Sitolo is:

```text
SALE
  ↓
PAYMENT EXPECTATION
  ↓
PAYMENT INTENT
  ↓
EXTERNAL PROVIDER
  ↓
PAYMENT EVIDENCE
  ↓
RECONCILIATION
  ↓
INTERNAL SETTLEMENT
```

The provider remains responsible for the external movement of funds.

Sitolo remains responsible for whether the merchant's sale is considered paid within Sitolo's own business state.

---

# 4. Source-of-Truth Hierarchy

Payment decisions use:

```text
1. Applicable law/regulation
2. Current signed provider contract/API behavior
3. Authenticated provider evidence
4. Sitolo payment intent
5. Sitolo sale state
6. Sitolo reconciliation rules
7. Client UI state
```

Client UI is lowest-authority evidence.

---

# 5. Core Payment Concepts

## 5.1 Sale

The commercial transaction.

## 5.2 Payment Intent

The expected payment associated with a business transaction.

Typical fields:

```text
payment_intent_id
organization_id
sale_id
amount
currency
payment_method
provider
merchant_reference
state
created_at
expires_at
```

## 5.3 Payment Attempt

One attempt to satisfy an intent.

One intent may have:

```text
attempt 1 -> failed
attempt 2 -> pending
attempt 3 -> succeeded
```

depending on provider semantics.

## 5.4 Provider Transaction

The provider-side financial transaction identifier.

## 5.5 Provider Event

A callback/event generated by the provider.

## 5.6 Payment Allocation

How a confirmed payment is applied to one or more internal obligations.

## 5.7 Reconciliation Case

A durable investigation state when internal and external evidence do not match cleanly.

---

# 6. Payment State Machine

Payment state is separate from sale state.

Recommended baseline:

```text
CREATED
   |
   v
PENDING
   |
   +----> SUCCEEDED
   |
   +----> FAILED
   |
   +----> EXPIRED
   |
   +----> CANCELLED
   |
   +----> UNKNOWN
   |
   +----> REQUIRES_RECONCILIATION
```

Potential post-success states:

```text
SUCCEEDED
   |
   +----> REFUND_PENDING
   |          |
   |          +----> PARTIALLY_REFUNDED
   |          |
   |          +----> REFUNDED
   |
   +----> CHARGEBACK_PENDING
              |
              +----> CHARGEBACK_CONFIRMED
```

Exact provider capabilities determine which transitions exist.

---

# 7. State Transition Rules

Valid transition:

```text
PENDING -> SUCCEEDED
```

Invalid examples:

```text
FAILED -> SUCCEEDED
```

unless the provider explicitly models a later reattempt as a new attempt or an authorized correction.

Never use arbitrary state overwrites:

```sql
UPDATE payments SET status = 'success'
```

without checking the previous state and evidence.

---

# 8. Sale vs Payment State

A sale can exist while payment is:

```text
UNPAID
PARTIALLY_PAID
PAID
PAYMENT_PENDING
PAYMENT_FAILED
PAYMENT_EXCEPTION
```

Do not collapse these into one boolean:

```text
paid = true
```

because real payment workflows are asynchronous.

---

# 9. Full Payment Domain Diagram

```text
SALE
 |
 +------------------------+
 |                        |
 v                        v
PAYMENT INTENT          CASH PAYMENT
 |                        |
 v                        v
PAYMENT ATTEMPTS       CASH EVENT
 |
 v
PROVIDER
 |
 +-------> WEBHOOK
 |
 +-------> VERIFY API
 |
 +-------> STATEMENT/POLL
             |
             v
       EXTERNAL EVIDENCE
             |
             v
        RECONCILIATION
             |
             +---- MATCH
             |
             +---- MISMATCH
             |
             +---- UNKNOWN
             |
             v
       INTERNAL PAYMENT
         STATE CHANGE
             |
             v
        PAYMENT ALLOCATION
             |
             v
       SALE SETTLEMENT
```

---

# 10. Payment Provider Adapter Boundary

Rust should expose a provider-neutral contract.

Conceptually:

```rust
trait PaymentProvider {
    async fn create_payment(
        &self,
        request: ProviderPaymentRequest,
    ) -> Result<ProviderPaymentResponse, ProviderError>;

    async fn verify_payment(
        &self,
        reference: ProviderTransactionReference,
    ) -> Result<ProviderPaymentStatus, ProviderError>;

    async fn process_webhook(
        &self,
        request: ProviderWebhookRequest,
    ) -> Result<NormalizedProviderEvent, ProviderError>;
}
```

The real interface should be strongly typed and split further if provider capabilities differ.

Do not create one giant interface requiring every provider to implement unsupported operations.

---

# 11. Capability-Based Provider Interface

Providers differ.

One provider may support:

```text
charge
verify
webhook
refund
```

while another may only support:

```text
charge
webhook
```

Model capabilities explicitly.

Example:

```text
ProviderCapabilities
├── supports_charge
├── supports_verify
├── supports_webhooks
├── supports_refunds
├── supports_partial_refunds
├── supports_payouts
├── supports_statements
└── supports_mobile_money
```

Do not fake unsupported provider capabilities.

---

# 12. PayChangu as a Provider Adapter

The current research confirms PayChangu offers APIs for card, bank and mobile-money collection and that mobile-money collection can produce a webhook after the customer authorizes the payment. citeturn456905search3turn456905search6

The Sitolo architecture should therefore treat PayChangu as:

```text
PayChanguAdapter
    |
    +--> initialize collection
    +--> verify transaction
    +--> receive webhook
    +--> normalize provider state
```

The exact endpoint/payload contract must be generated from the current provider documentation during implementation, not copied from a stale example.

---

# 13. No Hard-Coded Provider Logic in Core Domain

Bad:

```text
if provider == "PAYCHANGU" {
   ...
}
```

throughout sales code.

Good:

```text
PaymentService
    |
    v
ProviderPort
    |
    +--> PayChanguAdapter
```

Core business logic operates on normalized payment concepts.

---

# 14. Provider Credentials

Provider secrets are:

```text
production secrets
```

They belong in managed secret storage.

Never put them in:

```text
Flutter
Tauri
browser
Git
mobile configuration
client .env
logs
database rows visible to merchants
```

PayChangu's current API documentation states its endpoints require API-key authentication, reinforcing that those credentials must remain server-side. citeturn456905search5

---

# 15. Payment Intent Creation

A payment intent is created from authoritative sale state.

Example:

```text
sale.total = 15,000 MWK

CreatePaymentIntent:
  amount = 15,000
  currency = MWK
  sale_id = S123
```

Client-supplied amount is not authoritative.

The server calculates or retrieves:

```text
expected_amount
expected_currency
```

from the authoritative sale/financial model.

---

# 16. Intent Idempotency

Creating a payment intent must be idempotent.

Repeated request:

```text
CreatePaymentIntent(sale=S123)
```

must not create uncontrolled duplicate provider transactions.

Use:

```text
idempotency_key
```

plus a business uniqueness rule appropriate to provider semantics.

---

# 17. Attempt Model

One intent may have multiple attempts, but every attempt is explicit.

```text
PaymentIntent P1
    |
    +-- Attempt A1 -> FAILED
    +-- Attempt A2 -> PENDING
    +-- Attempt A3 -> SUCCEEDED
```

The domain must prevent an invalid attempt from overwriting a later successful attempt.

---

# 18. Provider Request

Provider requests should include only required information.

Do not send:

- unnecessary PII;
- internal secrets;
- irrelevant customer data;
- full internal object payloads.

Minimize external disclosure.

---

# 19. Provider Reference

Every external transaction must have an internal mapping:

```text
payment_intent_id
        ↕
provider
        ↕
provider_transaction_id
```

The provider transaction ID must be unique within the provider context.

Recommended constraint:

```text
UNIQUE(provider_id, provider_transaction_id)
```

---

# 20. Merchant Reference

Sitolo should generate a stable merchant reference.

It should be:

- unique where required;
- deterministic enough for support;
- safe to expose;
- unrelated to secrets;
- linked to the payment intent.

Example:

```text
STL-20260904-7K2M8P
```

Avoid exposing database primary keys if a separate reference better suits merchant UX.

---

# 21. Webhook Architecture

```text
Provider
   |
   | HTTPS POST
   v
Edge
   |
   v
Webhook endpoint
   |
   +--> size limit
   +--> authentication/signature
   +--> canonical/raw body handling
   +--> schema validation
   +--> timestamp/replay check
   +--> event identity check
   |
   v
Persist/claim event
   |
   v
Async processing
   |
   v
Provider verification
   |
   v
Reconciliation
```

---

# 22. Webhook Authentication

PayChangu's current documentation states webhook requests include a `Signature` header and instructs merchants to validate it. citeturn456905search1

Sitolo must:

1. capture the provider-specified signed representation;
2. retrieve the correct secret/key;
3. verify signature before trusting event semantics;
4. reject invalid signatures;
5. prevent timing-sensitive comparisons from leaking secret information where applicable;
6. log only safe metadata.

If a future provider offers no cryptographic webhook authentication, its callback must be treated as a lower-trust signal and independently reconciled.

---

# 23. Exact Signed Payload

Never assume:

```text
parsed JSON -> reserialize -> verify
```

is equivalent to:

```text
exact raw request body -> verify
```

Use the provider's exact signing specification.

For protocols using standardized HTTP message signatures, RFC 9421 defines canonical signature-input and signature verification concepts. Sitolo should adopt the provider's actual protocol rather than force an unrelated signature format. citeturn456905search0

---

# 24. Webhook Size Limit

Webhook endpoints must enforce:

```text
max body bytes
max header sizes
max nesting depth
```

before expensive parsing where practical.

---

# 25. Webhook Event Identity

Every provider event should have a stable deduplication identity if the provider supplies one.

Store:

```text
provider
provider_event_id
received_at
signature_verified
processing_state
```

with uniqueness where contractually safe:

```text
UNIQUE(provider, provider_event_id)
```

If no event ID exists, use a provider-safe derived identity only after careful contract analysis.

---

# 26. Webhook Duplicate

If provider sends:

```text
EVENT-123
EVENT-123
EVENT-123
```

Sitolo must produce:

```text
one financial effect
```

The duplicate callback can return a successful HTTP acknowledgement after deduplication.

---

# 27. Webhook Replay

A valid old webhook is still potentially dangerous.

Where provider supplies:

```text
timestamp
nonce
event_id
```

verify replay constraints.

Additionally:

```text
already-processed event
```

must remain harmless.

---

# 28. Webhook Processing

Recommended pattern:

```text
receive
  ↓
authenticate
  ↓
persist/deduplicate
  ↓
acknowledge quickly
  ↓
worker retrieves event
  ↓
provider verification
  ↓
reconcile
```

Do not perform long provider calls during the inbound webhook HTTP request if that would cause provider retries or resource exhaustion.

---

# 29. Do Not Trust Webhook Alone

PayChangu explicitly recommends re-querying/verifying a transaction after a webhook and keeping a background verification strategy if webhook delivery fails. citeturn456905search1

Therefore the baseline Sitolo model is:

```text
Webhook
   =
notification/evidence

Provider verification
   =
stronger external evidence

Sitolo reconciliation
   =
business decision
```

---

# 30. Verification API

Where the provider supports verification:

```text
provider_transaction_id
```

is submitted to the provider's verification endpoint.

The adapter normalizes:

```text
provider status
provider amount
provider currency
provider reference
provider timestamp
```

into an internal representation.

---

# 31. Never Trust Provider Status Alone

Even a verified provider response must be compared against:

```text
payment_intent.expected_amount
payment_intent.currency
payment_intent.provider
sale state
organization
merchant account
```

Example:

```text
Expected:
10,000 MWK

Provider:
1,000 MWK

Result:
MISMATCH
```

Do not mark the sale paid.

---

# 32. Amount Integrity

Payment settlement invariant:

```text
confirmed_payment_amount
must match
eligible_payment_amount
```

according to explicit partial-payment policy.

Never trust:

```text
client_amount
```

as proof.

---

# 33. Currency Integrity

A payment in:

```text
USD
```

cannot automatically settle:

```text
MWK
```

without explicit FX semantics.

The default behavior is mismatch/reconciliation.

---

# 34. Provider Integrity

A payment intent created for:

```text
Provider A
```

must not be completed using evidence from:

```text
Provider B
```

unless an explicit reconciliation workflow allows that.

---

# 35. Merchant Account Integrity

Provider credentials/configuration may point to a merchant account.

The verified transaction must belong to the expected merchant/provider account context where the provider exposes that information.

This prevents accepting evidence from an attacker-controlled payment account.

---

# 36. Payment Intent Expiration

Some intents may expire.

Example:

```text
CREATED
  |
  30 min
  v
EXPIRED
```

A late provider callback must be reconciled against actual provider state, not automatically ignored or accepted.

---

# 37. Late Success

Scenario:

```text
Intent expired
Provider later says SUCCESS
```

Result:

```text
do not silently discard money evidence
```

Create:

```text
REQUIRES_RECONCILIATION
```

if direct settlement is no longer valid.

---

# 38. Payment Allocation

A confirmed payment can be allocated to:

```text
one sale
```

or, if explicitly supported:

```text
multiple eligible obligations
```

Allocation must be deterministic.

Never let a client choose arbitrary allocation against unrelated tenants/sales.

---

# 39. Partial Payment

Support explicitly if required:

```text
sale total = 10,000

payment A = 4,000
payment B = 6,000

remaining = 0
```

The aggregate payment state becomes:

```text
PAID
```

only when allocated confirmed amounts satisfy the sale's settlement rule.

---

# 40. Overpayment

Example:

```text
sale = 10,000
provider payment = 12,000
```

Possible policies:

```text
reject
unallocated credit
refund
manual reconciliation
```

The policy must be explicit.

Do not silently increase the sale total to match payment.

---

# 41. Underpayment

Example:

```text
sale = 10,000
payment = 9,000
```

State remains:

```text
PARTIALLY_PAID
```

unless the business policy supports a different resolution.

---

# 42. Split Payment

If supported:

```text
10,000 sale

3,000 cash
7,000 mobile money
```

Payment allocations are explicit.

Each payment method retains its own evidence.

---

# 43. Cash Payment

Cash does not have a provider callback.

It uses:

```text
CashEvent
+
RegisterSession
+
actor
+
amount
+
reconciliation
```

Cash is locally observable but still subject to:

- authorization;
- register scope;
- duplicate protection;
- audit;
- closing reconciliation.

---

# 44. Offline Cash

Offline cash sales are a natural fit for Sitolo's continuity model.

Local:

```text
sale
+
cash event
```

are durably stored.

Server reconciliation later validates:

```text
actor
register
sale
amount
state
```

---

# 45. Offline Mobile Money

Mobile-money operation must be much more carefully bounded.

The device can:

```text
create payment intent
```

or record an attempted payment.

It cannot independently declare:

```text
provider payment = SUCCESS
```

without authoritative provider evidence.

---

# 46. Provider Polling

Webhook delivery can fail.

The system needs a background reconciliation worker.

Potential policy:

```text
PENDING < 1h
    poll periodically

PENDING > threshold
    slower polling

very old
    reconciliation case
```

The exact intervals must come from provider SLA/limits and operational evidence.

---

# 47. Polling Limits

Do not poll every payment every second.

Use:

```text
backoff
+
jitter
+
maximum attempt rate
+
provider quotas
```

Provider documentation should determine actual polling limits.

---

# 48. Provider Outage

If provider is unavailable:

```text
payment = PENDING_PROVIDER_UNAVAILABLE
```

or equivalent internal state.

Do not mark:

```text
FAILED
```

simply because the provider timed out.

A timeout means:

```text
unknown result
```

unless provider semantics explicitly establish failure.

---

# 49. Timeout-After-Provider-Commit

Scenario:

```text
Sitolo -> provider
provider commits transaction
response lost
Sitolo times out
```

The application must not initiate an uncontrolled duplicate payment.

Retry using:

```text
same provider idempotency key
```

where supported, and/or retrieve/verify using the provider reference.

---

# 50. Provider Idempotency

When provider supports idempotency keys:

```text
Sitolo idempotency key
```

should be stable for the same payment attempt.

Never generate a new key for every HTTP retry.

---

# 51. Provider Without Idempotency

If a provider does not support idempotency:

```text
initialize once
+
persist provider reference
+
verify before reinitialization
```

Before creating another attempt after timeout, determine whether the first attempt already exists.

If uncertainty remains:

```text
reconciliation
```

may be safer than duplicate initiation.

---

# 52. Duplicate Charge Prevention

Critical invariant:

```text
one logical payment attempt
cannot produce
two intended external charges
```

unless explicitly represented as two separate attempts.

---

# 53. Payment Attempt Concurrency

Two workers must not independently settle the same intent.

Use database locking or atomic state transitions.

Conceptually:

```sql
UPDATE payment_intent
SET state = 'PROCESSING'
WHERE payment_intent_id = $1
  AND state = 'PENDING';
```

Require exactly one affected row before proceeding.

---

# 54. Payment State Concurrency

All state transitions should follow:

```text
BEGIN
  lock/re-read intent
  verify current state
  verify provider evidence
  calculate resulting state
  write state/evidence
  write audit/outbox
COMMIT
```

---

# 55. Duplicate Webhook + User Confirmation

Potential race:

```text
Webhook worker
        +
Merchant presses "Confirm"
```

Only one authoritative transition should succeed.

The other sees current state and becomes harmless/no-op/appropriate conflict.

---

# 56. User “Mark as Paid” Feature

A generic:

```text
Mark as paid
```

button is dangerous.

If needed for:

```text
cash payment
```

it must be an explicit cash transaction.

For provider payments:

```text
manual confirmation
```

should not override external provider evidence unless a formally governed reconciliation/admin workflow exists.

---

# 57. Frontend Payment Security

The frontend may display:

```text
Payment successful
```

based on a provider UI result for UX.

It must still call the server.

Server:

```text
revalidates payment
```

before:

```text
fulfillment
sale settlement
stock release
tax status
```

---

# 58. Browser Redirects

A browser/hosted-payment redirect is not authoritative.

Example:

```text
redirect?status=success
```

is only a user-agent signal.

Server verifies provider state.

---

# 59. Mobile App Callbacks

Similarly:

```text
mobile callback says paid
```

does not settle payment.

It triggers:

```text
server verification
```

---

# 60. Payment Webhook Endpoint Classification

Webhook routes are:

```text
PUBLIC NETWORK REACHABILITY
+
AUTHENTICATED PROVIDER MESSAGE
```

They are not normal merchant-authenticated API endpoints.

Authentication model is provider-specific.

---

# 61. Webhook CORS

CORS is irrelevant to server-to-server webhook trust.

Do not use:

```text
CORS allowlist
```

as webhook authentication.

---

# 62. Webhook IP Allowlisting

IP allowlists can be an additional control if the provider documents stable source ranges.

They are not sufficient cryptographic authenticity by themselves.

---

# 63. Webhook TLS

Require HTTPS.

Do not allow plaintext provider callback endpoints.

---

# 64. Provider Secret Rotation

Each provider credential has:

```text
active
previous
disabled
```

or equivalent rotation states.

Rotation should support overlap when provider allows it.

---

# 65. Credential Compromise

Runbook:

```text
detect
  ↓
revoke
  ↓
rotate
  ↓
update secret store
  ↓
restart/reload adapter
  ↓
verify old credential fails
  ↓
review transactions
  ↓
audit
```

---

# 66. Payment Data Classification

Examples:

```text
provider API key
    SECRET

provider secret/signing key
    SECRET

transaction ID
    CONFIDENTIAL

customer mobile number
    SENSITIVE PII

amount
    CONFIDENTIAL BUSINESS DATA

internal payment intent ID
    INTERNAL/CONFIDENTIAL
```

Exact classification follows data-governance policy.

---

# 67. Logging Rules

Never log:

- provider API keys;
- webhook signing secrets;
- authentication headers;
- full card information;
- unnecessary customer PII;
- raw authorization data.

Safe:

```text
payment_intent_id
provider
provider_transaction_id
status
reason_code
request_id
```

subject to access controls.

---

# 68. Audit Events

Payment audit events should distinguish:

```text
PAYMENT_INTENT_CREATED
PAYMENT_ATTEMPT_CREATED
PAYMENT_PROVIDER_REQUESTED
PAYMENT_WEBHOOK_RECEIVED
PAYMENT_WEBHOOK_REJECTED
PAYMENT_VERIFICATION_REQUESTED
PAYMENT_VERIFICATION_MISMATCH
PAYMENT_CONFIRMED
PAYMENT_REJECTED
PAYMENT_RECONCILIATION_OPENED
PAYMENT_RECONCILIATION_RESOLVED
REFUND_REQUESTED
REFUND_APPROVED
REFUND_COMPLETED
```

Audit records should not replace the financial ledger.

---

# 69. Payment Ledger vs Payment State

A payment status projection:

```text
SUCCEEDED
```

is not the same as a financial ledger entry.

Where Sitolo's accounting/financial model requires it:

```text
payment evidence
    →
financial event
```

is explicit and auditable.

---

# 70. Settlement Model

Settlement means:

```text
provider-confirmed money
```

has been accepted into Sitolo's defined internal payment state.

Do not conflate:

```text
payment initiated
```

with:

```text
funds settled
```

---

# 71. Provider Settlement Timing

Some providers can report:

```text
payment success
```

before actual merchant settlement.

Sitolo should preserve:

```text
payment transaction state
+
settlement/reconciliation state
```

separately if provider semantics require it.

---

# 72. Reconciliation Architecture

```text
                     RECONCILIATION

Internal Sale
     |
     v
Payment Intent
     |
     +---------------------+
                           |
                           v
                   Provider Evidence
                           |
             +-------------+-------------+
             |             |             |
             v             v             v
          EXACT          PARTIAL      UNKNOWN
           MATCH          MATCH
             |             |             |
             v             v             v
         SETTLE        PARTIAL       EXCEPTION
```

---

# 73. Exact Match

Example:

```text
sale = 15,000 MWK
provider = 15,000 MWK
same expected provider
same merchant context
same transaction mapping
```

Result:

```text
MATCHED
```

---

# 74. Amount Mismatch

Example:

```text
expected = 15,000
provider = 14,000
```

Result:

```text
REQUIRES_RECONCILIATION
```

Do not round away material mismatches.

---

# 75. Provider Reference Mismatch

If:

```text
webhook transaction ID
```

does not correspond to the expected intent:

```text
unknown/mismatch
```

requiring controlled reconciliation.

---

# 76. Customer Reference Collision

Two customers must never share a provider transaction identity in the same provider context.

Database constraints protect against duplicate association.

---

# 77. Reconciliation Priority

Existing Sitolo design establishes:

```text
1. Provider transaction ID
2. Provider reference
3. Sitolo payment intent/client reference
4. Controlled exact-match fallback
5. Manual review
```

Do not let fuzzy matching silently settle money.

---

# 78. Fuzzy Matching

Fuzzy matching may produce:

```text
candidate
```

not:

```text
confirmed payment
```

Example:

```text
same amount
same time
same customer phone
```

is not necessarily sufficient evidence.

---

# 79. Reconciliation Case

A case should contain:

```text
case_id
tenant_id
payment_intent_id?
provider
provider_transaction_id?
expected_amount
observed_amount
expected_currency
observed_currency
candidate references
reason
state
opened_at
assigned_to
resolved_by
resolution
evidence
```

---

# 80. Reconciliation State

```text
OPEN
  ↓
INVESTIGATING
  ↓
MATCHED
  ↓
RESOLVED

or

OPEN
  ↓
REJECTED
  ↓
CLOSED
```

---

# 81. Manual Reconciliation

Manual resolution requires:

- permission;
- reason;
- evidence;
- actor;
- timestamp;
- audit;
- ideally approval for high-value cases.

The operator must not directly edit historical payment rows to hide the discrepancy.

---

# 82. Manual Settlement

A manual settlement adjustment should create a new explicit business/audit event.

Never:

```sql
UPDATE payment
SET status = 'SUCCESS'
```

without evidence and an appropriate correction/audit record.

---

# 83. Refund Domain

Refund is a distinct financial operation.

It must reference:

```text
original payment
original sale
eligible amount
requested amount
actor
reason
provider
```

---

# 84. Refund Invariant

```text
total_refunded
<=
eligible_refundable_amount
```

This must be enforced transactionally.

---

# 85. Concurrent Refunds

Two refund requests race:

```text
eligible = 10,000

A -> refund 8,000
B -> refund 8,000
```

Only a valid combined amount may succeed.

Use:

```text
transaction
+
lock/recheck
+
remaining eligibility
```

---

# 86. Partial Refunds

Where provider supports them:

```text
original payment 10,000
refund 3,000
remaining eligible 7,000
```

Every refund has an independent identity.

---

# 87. Provider Refund Failure

If provider refund fails:

```text
refund state = FAILED
```

not:

```text
sale silently modified
```

The merchant financial state remains explicit.

---

# 88. Refund Timeout

Provider may commit then timeout.

Use:

```text
same provider idempotency reference
+
verification
```

before retrying.

---

# 89. Chargebacks

If supported by provider:

```text
CHARGEBACK_PENDING
CHARGEBACK_CONFIRMED
```

should be separate from ordinary refunds.

A chargeback is external evidence and may require reconciliation rather than a simple refund operation.

---

# 90. Credit Accounts

If Sitolo supports merchant credit accounts:

```text
CREDIT_ACCOUNT
```

must be a separate controlled financial domain.

Do not silently turn ordinary customer balances into a lending product.

Existing Sitolo architecture explicitly defers lending/credit-scoring as separate products absent formal architecture review.

---

# 91. Cash Reconciliation

For a register:

```text
Expected Cash
=
Opening Float
+ Cash Sales
+ Cash In
- Cash Refunds
- Cash Out
```

Counted cash is then compared.

Variance becomes an explicit record.

---

# 92. Mobile Money Reconciliation

Merchant reconciliation should compare:

```text
Sitolo recorded payment
vs
provider transactions
```

over a defined time window.

A reconciliation job may ingest provider statements/API results where contracts permit.

---

# 93. Reconciliation Jobs

Workers should support:

```text
scan pending payments
verify stale payments
ingest provider events
match provider transactions
open exceptions
notify authorized operators
```

Workers must be idempotent.

---

# 94. Reconciliation Retry

Do not retry forever.

Use:

```text
attempt count
last attempt
next attempt
backoff
final exception threshold
```

---

# 95. Reconciliation Queue Isolation

Payment reconciliation should have resource isolation from:

```text
notifications
reports
bulk exports
```

A report storm must not prevent payment verification.

---

# 96. Provider API Rate Limits

Respect provider-specific limits.

The adapter should implement:

```text
rate limiter
backoff
concurrency cap
```

where needed.

---

# 97. Provider API Timeouts

Every outbound provider call requires:

```text
connect timeout
request deadline
read timeout
```

The adapter must never hang a Tokio worker indefinitely.

---

# 98. Retry Policy

Retry only when:

```text
provider behavior is known retryable
+
operation is idempotent or independently safe
```

Do not retry arbitrary 4xx.

---

# 99. Circuit Breaker

If a provider is repeatedly failing:

```text
CLOSED
   ↓ failures
OPEN
   ↓ cooldown
HALF_OPEN
   ↓ success
CLOSED
```

This prevents provider failures from exhausting Sitolo resources.

---

# 100. Bulkhead

Separate concurrency pools for:

```text
payment initiation
payment verification
refunds
reconciliation
```

where load warrants it.

---

# 101. Provider Error Normalization

Normalize provider outcomes into stable Sitolo categories:

```text
AUTHENTICATION_FAILURE
INVALID_REQUEST
RATE_LIMITED
TEMPORARY_UNAVAILABLE
TIMEOUT
NOT_FOUND
ALREADY_PROCESSED
INSUFFICIENT_FUNDS
DECLINED
UNKNOWN
```

Preserve provider-specific diagnostic codes separately.

---

# 102. Provider Response Validation

Do not deserialize provider JSON and immediately trust it.

Validate:

```text
required fields
types
ranges
provider identity
reference format
status vocabulary
amount
currency
timestamps
```

Unknown status should become:

```text
UNKNOWN_PROVIDER_STATE
```

not automatic success.

---

# 103. Provider Schema Drift

External payloads change.

Maintain:

```text
adapter version
provider schema version where available
contract tests
fixtures
```

---

# 104. Provider Webhook Fixture Corpus

Keep redacted test fixtures for:

```text
success
failure
duplicate
replay
partial
refund
invalid signature
unknown event
malformed payload
new provider field
missing provider field
```

Do not store live secrets or real customer data.

---

# 105. Provider Contract Testing

Every adapter requires:

```text
initialization test
verification test
webhook test
failure test
retry test
idempotency test
amount mismatch test
currency mismatch test
duplicate test
```

---

# 106. Payment Security Threat Model

Threats:

```text
fake success callback
modified amount
modified currency
provider impersonation
webhook replay
duplicate webhook
payment duplication
refund abuse
stolen API key
provider account mismatch
race-condition settlement
client-only paid state
overpayment exploit
underpayment exploit
reconciliation poisoning
rate-limit exhaustion
provider outage
```

---

# 107. Payment Security Controls

| Threat | Control |
|---|---|
| Fake success | provider authentication + verification |
| Amount tampering | server-authoritative intent |
| Currency tampering | explicit comparison |
| Replay | event ID + uniqueness + replay controls |
| Duplicate charge | provider idempotency + internal attempts |
| Refund abuse | eligible amount invariant |
| API key theft | managed secret storage |
| Account mismatch | merchant/provider-context validation |
| Race | transactional state transitions |
| Client spoofing | server authority |
| Overpayment | explicit reconciliation |
| Provider outage | pending state + worker retry |
| Flood | quotas/rate limits |
| Unknown provider state | fail closed |

---

# 108. Payment Invariants

At minimum:

```text
payment intent belongs to one tenant
payment intent references valid sale
expected amount is server authoritative
expected currency is server authoritative
provider transaction identity is unique
provider event identity is unique where available
one successful attempt cannot be settled twice
refund <= refundable amount
client cannot directly set success
invalid webhook signature cannot alter payment state
payment state transitions are legal
external timeout does not imply failure
duplicate provider events are harmless
```

---

# 109. Tenant Isolation

Payment records are tenant-owned.

A payment ID from Tenant A must never retrieve:

```text
transaction details
customer details
provider references
amounts
```

for Tenant B.

Cross-tenant negative tests are mandatory.

---

# 110. Provider Reference Enumeration

Do not expose provider references unnecessarily.

Attackers should not be able to enumerate payment IDs and infer:

- payment amounts;
- customer identities;
- transaction status.

---

# 111. Payment API Authorization

Examples:

```text
CreatePaymentIntent
    SELL / PAYMENT_CREATE

ViewPayment
    PAYMENT_READ

RefundPayment
    PAYMENT_REFUND

ResolveReconciliation
    PAYMENT_RECONCILE

ChangeProviderConfig
    PAYMENT_PROVIDER_ADMIN
```

High-risk actions require stronger authorization/approval.

---

# 112. Payment Provider Configuration

Provider configuration belongs to an organization/business entity and may vary by branch where contracts support it.

Configuration changes require:

```text
authorization
audit
step-up
secret validation
```

---

# 113. Payout Credentials

If Sitolo eventually supports payout operations, they require a separate high-risk domain and must not reuse ordinary payment-collection permissions.

Payout destination changes should require:

```text
step-up
verification
cooldown/approval as risk dictates
```

---

# 114. Provider Webhook Secrets

Store:

```text
provider
credential_id
secret version
active status
rotation metadata
```

Do not embed secrets in code.

---

# 115. Secrets Rotation Without Downtime

Where provider supports multiple active secrets:

```text
new secret
   ↓
deploy
   ↓
accept old+new verification
   ↓
rotate provider
   ↓
disable old
```

If provider supports only one secret, use a controlled maintenance rotation.

---

# 116. Payment Observability

Metrics:

```text
payment_intents_created
payment_attempts_created
payment_success_rate
payment_failure_rate
payment_pending_count
payment_unknown_count
webhook_count
webhook_signature_failures
webhook_duplicates
verification_requests
verification_mismatches
reconciliation_cases_opened
reconciliation_age
refund_count
refund_failure_rate
provider_latency
provider_timeout_rate
provider_rate_limits
```

---

# 117. Payment Tracing

A payment trace should connect:

```text
request_id
sale_id
payment_intent_id
attempt_id
provider_transaction_id
webhook_event_id
verification_request
reconciliation_case
outbox_job
```

---

# 118. Payment Audit Reconstruction

A support/operator should be able to answer:

```text
What did the merchant sell?
What payment did Sitolo expect?
Which provider did it use?
What did the provider report?
Was the webhook authenticated?
Was the provider re-queried?
What amount/currency was observed?
Why was the payment accepted/rejected?
When was the sale considered paid?
Was there a refund?
Who initiated it?
```

without reading raw secrets or customer-sensitive payloads.

---

# 119. Payment Error Model

Stable internal error categories:

```text
PAYMENT_INVALID_REQUEST
PAYMENT_NOT_FOUND
PAYMENT_ALREADY_SETTLED
PAYMENT_PROVIDER_UNAVAILABLE
PAYMENT_PROVIDER_TIMEOUT
PAYMENT_PROVIDER_REJECTED
PAYMENT_VERIFICATION_FAILED
PAYMENT_AMOUNT_MISMATCH
PAYMENT_CURRENCY_MISMATCH
PAYMENT_PROVIDER_MISMATCH
PAYMENT_REQUIRES_RECONCILIATION
PAYMENT_REFUND_NOT_ELIGIBLE
PAYMENT_REFUND_LIMIT_EXCEEDED
PAYMENT_WEBHOOK_INVALID
PAYMENT_WEBHOOK_REPLAY
PAYMENT_IDEMPOTENCY_CONFLICT
```

---

# 120. Frontend State Model

Frontend can represent:

```text
CREATING
AWAITING_CUSTOMER
PENDING
VERIFYING
SUCCESS
FAILED
REQUIRES_REVIEW
```

But final authority comes from server state.

---

# 121. No Optimistic Paid State

A UI may optimistically show:

```text
Processing…
```

It should not show:

```text
PAID
```

until server authority is established.

---

# 122. Payment UI Under Network Failure

If provider/payment API request times out:

```text
Do NOT:
  retry uncontrolled
  show paid
  create new intent blindly

DO:
  show processing/unknown
  query server
  allow controlled retry/verification
```

---

# 123. Offline Sale + Payment UX

When offline:

```text
Cash:
  can be recorded according to offline policy

Mobile money:
  intent/attempt can be recorded
  authoritative settlement remains pending until verified
```

The app should explain the difference to the merchant.

---

# 124. No Secret Provider SDK in Client

If a provider SDK requires:

```text
secret API key
```

it belongs server-side.

The client can use publishable/public credentials only where provider contract explicitly permits.

---

# 125. Payment Callback Validation

Every callback endpoint:

```text
authenticate provider
validate schema
deduplicate
verify transaction
compare intent
transition state
audit
```

No shortcut.

---

# 126. Webhook Acknowledgement

Return a provider-appropriate success response once the event has been safely accepted/deduplicated according to the integration contract.

Long reconciliation work should be asynchronous when possible.

---

# 127. Webhook Retry Semantics

Assume providers retry failed webhooks.

Therefore:

```text
5xx
```

may lead to duplicates.

Deduplication must be independent of successful previous HTTP responses.

---

# 128. Webhook Poison Events

Unknown/malformed events should be:

```text
accepted only as safe evidence where possible
```

or:

```text
rejected/quarantined
```

without modifying payment truth.

---

# 129. Provider Verification Failure

If webhook is validly authenticated but verification API fails:

```text
do not settle
```

Record:

```text
VERIFICATION_PENDING
```

and retry with bounded policy.

---

# 130. Provider Verification Mismatch

If provider webhook says:

```text
success 10,000 MWK
```

but verification says:

```text
failed
```

result:

```text
REQUIRES_RECONCILIATION
```

Do not choose whichever result is convenient.

---

# 131. Reconciliation Confidence

Possible internal confidence:

```text
EXACT
STRONG
WEAK
AMBIGUOUS
UNRESOLVED
```

Only sufficiently strong evidence may settle automatically.

---

# 132. No Silent Matching

Never use:

```text
same amount + same day
```

as an automatic payment match when stronger evidence exists or ambiguity remains.

---

# 133. Statement Reconciliation

Where providers expose merchant statements:

```text
download/import
    ↓
validate
    ↓
deduplicate
    ↓
normalize
    ↓
match
    ↓
exceptions
```

Statement imports require the same tenant and authorization controls as all other bulk imports.

---

# 134. Reconciliation Import Security

Treat statement files as hostile.

Apply:

```text
file size limits
schema validation
virus scanning where applicable
CSV/spreadsheet injection defenses
tenant ownership controls
```

---

# 135. Payment Data Retention

Retention depends on:

```text
financial evidence
provider contract
tax requirements
privacy law
merchant agreement
```

Do not set arbitrary short TTLs for records required to explain historical payments.

---

# 136. Sensitive Payment Exports

Payment exports require:

```text
PAYMENT_EXPORT
```

permission.

Large exports should be asynchronous, audited and protected with expiring downloads.

---

# 137. Support Access

Support agents should be able to inspect:

```text
status
references
error codes
timestamps
reconciliation state
```

but not necessarily:

```text
raw payment credentials
```

or unrestricted customer data.

---

# 138. Break-Glass Payment Access

Emergency payment intervention requires:

```text
MFA
specific privilege
reason
approval where required
full audit
time limit
```

---

# 139. No Direct Database Payment Fixes

Production operators must not manually update:

```text
payment.status = success
```

as the normal remediation mechanism.

Use a governed reconciliation/correction command.

---

# 140. Payment Correction Events

Corrections should produce:

```text
original state
+
correction event
+
reason
+
actor
+
evidence
```

The original financial fact remains preserved.

---

# 141. Payment and Inventory

Payment confirmation may affect:

```text
sale settlement
inventory reservation release
order fulfillment
```

but payment/provider calls must not occur inside the core database transaction.

---

# 142. External Call Separation

Correct:

```text
DB transaction
   -> create payment intent
   -> commit
   -> worker
   -> provider
```

Not:

```text
BEGIN
  call provider
  wait
  update DB
COMMIT
```

This prevents long transaction locks and uncertain rollback behavior.

---

# 143. Outbox

Creating an intent or settlement-triggered event should write an outbox event transactionally.

Example:

```text
PAYMENT_VERIFICATION_REQUIRED
PAYMENT_CONFIRMED
REFUND_REQUESTED
```

Workers consume them idempotently.

---

# 144. Inbound Provider Events

Webhook events can be treated similarly to an inbox:

```text
persist provider event
+
deduplicate
+
process
```

An inbox/event table should protect against duplicate provider delivery.

---

# 145. Payment Event Sourcing Boundary

Sitolo does not need to become a full event-sourced payment system.

Use:

```text
authoritative PostgreSQL state
+
immutable financial evidence
+
audit/events
```

where appropriate.

Do not implement event sourcing merely because payments involve events.

---

# 146. Database Constraints

Important constraints include:

```text
UNIQUE(provider, provider_transaction_id)

UNIQUE(provider, provider_event_id)

UNIQUE(command_id)

FK(payment_intent -> sale)

CHECK(amount >= 0)

CHECK(refunded_amount >= 0)
```

Additional constraints depend on actual schema.

---

# 147. Payment Transaction Boundary

Example successful verification:

```text
BEGIN

lock payment intent
lock relevant payment attempt
read current intent
validate provider evidence
validate amount
validate currency
validate merchant/provider context
check current state
update payment state
record evidence
record allocation
record audit
record outbox
record idempotency

COMMIT
```

---

# 148. Double Settlement Defense

Use multiple layers:

```text
provider transaction uniqueness
+
payment attempt state
+
payment intent state
+
financial allocation invariant
+
transaction locking
+
idempotency
```

---

# 149. Race: Two Webhooks

```text
Webhook A -> success
Webhook B -> success
```

Both must not create two settlements.

Only one transitions the authoritative state.

The other becomes duplicate/no-op.

---

# 150. Race: Success + Refund

```text
payment success
refund request
```

must have legal sequencing.

Do not allow refund against:

```text
still-pending payment
```

unless provider/domain explicitly supports that workflow.

---

# 151. Race: Two Refunds

Use transactional remaining eligibility:

```text
eligible_remaining = original - completed/refunding amounts
```

lock/recheck before accepting.

---

# 152. Race: Two Payment Attempts

The system must know whether simultaneous attempts are allowed.

If allowed:

```text
attempt A
attempt B
```

must both remain explicit until one/both settle under policy.

If only one attempt may be active:

```text
database state constraint
```

enforces it.

---

# 153. Payment Idempotency Table

A logical record should preserve:

```text
tenant
actor
idempotency_key
operation
request_fingerprint
resource_id
result
status
created_at
expires_at
```

Do not use Redis as the sole source of financial idempotency.

---

# 154. Provider Idempotency vs Sitolo Idempotency

They solve different layers.

```text
Sitolo idempotency:
    protect internal command/retry semantics

Provider idempotency:
    protect external charge creation
```

Both can be needed.

---

# 155. Idempotency-Key Scope

An idempotency key must be scoped to:

```text
tenant
principal/device context where relevant
operation
```

to prevent cross-context collisions.

---

# 156. Idempotency Fingerprint

If the same idempotency key is reused with:

```text
different amount
```

return:

```text
IDEMPOTENCY_CONFLICT
```

Do not silently use the first or second payload.

---

# 157. Payment Reconciliation Scheduler

Conceptual:

```text
every N seconds/minutes
  |
  +--> pending intents
  +--> failed webhooks
  +--> unknown payments
  +--> stale refunds
  +--> open provider exceptions
```

The actual schedule follows provider rate limits and SLOs.

---

# 158. Payment Queue Prioritization

Prioritize:

```text
old unknown payments
near-expiry payment intents
refund verification
recent webhook retries
normal reconciliation
```

according to business risk.

---

# 159. Provider API Unavailability

When provider is down:

```text
do not:
  mark all payments failed
  create duplicates
  block cash sales unnecessarily

do:
  maintain explicit pending state
  use bounded retry
  preserve evidence
  expose degraded provider state
```

---

# 160. Merchant UX During Provider Outage

The mobile app should communicate:

```text
Mobile money provider unavailable
```

rather than:

```text
Payment failed
```

when the result is actually unknown.

This prevents merchants from mistakenly collecting money twice.

---

# 161. Duplicate Customer Payment Scenario

Customer pays once.

Merchant sees timeout.

Merchant asks customer to pay again.

Both provider transactions succeed.

Sitolo must represent:

```text
payment 1 = matched
payment 2 = overpayment/unallocated/reconciliation
```

not silently merge them into one payment.

---

# 162. Double Refund Scenario

Two support users request refunds simultaneously.

The database prevents refundable amount from going below zero.

---

# 163. Fake Payment Screenshot

A customer shows:

```text
payment screenshot
```

That is not authoritative evidence.

Merchant UI should require actual provider-supported verification flow.

---

# 164. Fake Client Callback

Attacker sends:

```http
POST /payment/success
```

with:

```json
{"amount":999999,"status":"success"}
```

Result:

```text
no settlement
```

unless valid provider evidence exists.

---

# 165. Forged Webhook

Attacker reproduces a valid-looking JSON body without valid provider signature.

Result:

```text
reject
```

and security event.

---

# 166. Replay Webhook

Previously accepted event sent again.

Result:

```text
no additional financial effect
```

---

# 167. Provider Transaction Swap

Attacker changes provider reference from:

```text
their payment
```

to:

```text
another transaction
```

Server checks expected intent and merchant context.

Mismatch -> reconciliation/rejection.

---

# 168. Amount Tampering

Attacker changes:

```text
10,000
```

to:

```text
1
```

or:

```text
100,000
```

The authoritative intent/verification comparison detects mismatch.

---

# 169. Currency Tampering

Attacker changes:

```text
MWK
```

to:

```text
USD
```

Mismatch.

---

# 170. Provider Account Tampering

Attacker attempts to make evidence from a different provider merchant account settle the sale.

Server verifies provider account context where available.

---

# 171. Payment Enumeration

Do not reveal payment state by allowing arbitrary unauthenticated:

```text
GET /payments/{provider_tx}
```

queries.

All access is authenticated/scoped.

Webhook endpoints are the exception because they use provider-specific authentication.

---

# 172. Public Provider References

Provider transaction IDs should not be treated as secret, but should not be unnecessarily exposed.

Use internal payment IDs/reference IDs for ordinary merchant APIs.

---

# 173. API Rate Limits

Recommended classes:

```text
payment initiation
payment verification
refund
reconciliation
webhook
provider configuration
```

with provider-aware limits.

---

# 174. Webhook Rate Limiting

Webhook rate limiting must be careful not to cause valid provider retries to be rejected indefinitely.

Use:

```text
bounded concurrency
provider-specific quotas
event deduplication
```

and return correct HTTP behavior for provider retry semantics.

---

# 175. Payment DDoS

Attackers can generate expensive verification requests.

Never allow:

```text
client -> verify arbitrary provider transaction
```

without:

```text
authorization
ownership
rate limit
```

---

# 176. Provider Polling Abuse

A merchant user must not be able to force unlimited provider API calls.

Verification endpoints require:

```text
ownership
cooldown
quota
```

where appropriate.

---

# 177. Refund Abuse Detection

Signals:

```text
many refunds
high refund ratio
refund shortly after sale
repeated same amount
refunds by same cashier
refunds near authorization threshold
```

These feed security analytics.

---

# 178. Payment Fraud Detection

The initial product should use deterministic rules before introducing complex ML.

Examples:

```text
provider mismatch
amount mismatch
unusual refund velocity
duplicate customer payment
rapid payment/reversal loops
```

Advanced fraud scoring is a later extension.

---

# 179. Reconciliation SLA

Define operational objectives:

```text
new pending payment:
    verify within target

unknown payment:
    review within target

high-value mismatch:
    escalate faster
```

Exact targets should be set after provider/customer evidence.

---

# 180. Payment SLOs

Candidate metrics:

```text
P95 payment intent creation
P95 provider response latency
P95 verification latency
P99 webhook processing latency
pending-payment age
reconciliation resolution age
provider error percentage
```

---

# 181. Provider Health

Maintain:

```text
provider status
latency
failure rate
webhook delay
verification success
rate-limit responses
```

This supports safe degraded UX.

---

# 182. Provider Health vs Business Truth

A provider's overall health signal must never automatically settle or fail individual payments.

It is operational evidence only.

---

# 183. Payment Failure Matrix

| Situation | Internal result |
|---|---|
| Invalid client payment fields | reject |
| Invalid provider signature | reject |
| Duplicate provider event | no-op |
| Provider timeout | pending/unknown |
| Verification success | eligible for settlement |
| Verification amount mismatch | reconciliation |
| Verification currency mismatch | reconciliation |
| Provider transaction missing | pending/exception |
| Provider says failed | failed |
| Provider says success but internal intent missing | reconciliation |
| Provider account mismatch | reject/reconciliation |
| Refund over limit | reject |
| Double refund race | one succeeds; other rejects/conflicts |

---

# 184. Payment Test Pyramid

```text
                Manual provider test
                       ▲
                 Contract tests
                       ▲
                 API integration
                       ▲
               PostgreSQL tests
                       ▲
             Property/concurrency
                       ▲
                 Pure domain
```

---

# 185. Mandatory Payment Security Tests

At minimum:

```text
[ ] fake success rejected
[ ] fake amount rejected
[ ] fake currency rejected
[ ] fake provider rejected
[ ] invalid webhook signature rejected
[ ] webhook replay harmless
[ ] duplicate webhook harmless
[ ] duplicate provider transaction harmless
[ ] response-loss retry safe
[ ] provider timeout handled safely
[ ] refund cannot exceed eligible amount
[ ] concurrent refunds safe
[ ] concurrent settlement safe
[ ] cross-tenant payment access denied
[ ] frontend paid flag ignored
[ ] provider account mismatch rejected
[ ] provider verification mismatch creates exception
[ ] secrets absent from logs
[ ] provider credentials absent from clients
[ ] polling rate limits enforced
[ ] webhook resource bounds enforced
```

---

# 186. Property Tests

Generate:

```text
sale amount
payments
refund sequences
duplicate events
retry sequences
```

and prove:

```text
paid_amount <= eligible_amount
refunded_amount <= refundable_amount
duplicate event has no extra effect
```

---

# 187. Concurrency Tests

Run:

```text
100 concurrent webhook deliveries
```

for the same event.

Expected:

```text
one financial effect
```

Run:

```text
100 concurrent refund attempts
```

against one payment.

Expected:

```text
total refunds never exceed eligibility
```

---

# 188. Failure Injection

Test:

```text
provider succeeds then connection drops
provider times out
webhook delayed
webhook duplicated
verification unavailable
database restarts
worker dies after DB commit
worker dies before provider call
```

---

# 189. Worker Crash Semantics

Scenario:

```text
worker reads outbox
provider call succeeds
worker crashes before marking outbox complete
```

Retry must not produce duplicate provider effects.

Use provider idempotency or verification before retry.

---

# 190. Outbox Processing

Outbox state:

```text
PENDING
PROCESSING
DELIVERED
RETRY_WAIT
DEAD_LETTER
```

Do not hold DB transaction open while calling provider.

---

# 191. Outbox Leasing

If multiple workers process events:

```text
lease owner
lease expiration
attempt count
next_attempt_at
```

prevent duplicate processing while still allowing recovery after worker death.

---

# 192. Webhook Inbox

Likewise:

```text
event received
    ↓
deduplicate
    ↓
persist
    ↓
process
```

The inbound event record remains evidence.

---

# 193. Provider Reconciliation After Restore

After database restore:

```text
pending payment intents
provider state
webhook backlog
outbox
```

must be reconciled before declaring recovery complete.

---

# 194. Key Rotation After Restore

Provider secrets must come from current secret infrastructure, not stale database backups.

Do not store active provider API secrets as business records merely to make restores convenient.

---

# 195. Payment Configuration Versioning

Provider configuration should be versioned.

Example:

```text
PayChangu config v4
```

A payment attempt can retain:

```text
provider_config_version = 4
```

for forensic traceability.

---

# 196. Branch-Specific Provider Config

If permitted:

```text
Organization
  |
  +-- Branch A -> Provider account X
  +-- Branch B -> Provider account Y
```

Every payment intent resolves the correct provider configuration server-side.

---

# 197. No Client Provider Selection

Client may request:

```text
MOBILE_MONEY
```

but the server determines:

```text
which configured provider
```

according to merchant configuration and policy.

---

# 198. Payment Method Eligibility

Payment method is subject to:

```text
organization config
branch config
device capability
user role
currency
provider availability
sale context
```

---

# 199. Payment Provider Down + Alternative Method

If mobile-money provider is unavailable:

```text
cash
bank
another configured method
```

may remain available.

The UI should not conflate provider availability with Sitolo availability.

---

# 200. Provider Configuration Validation

When enabling a provider:

```text
credential test
webhook test
merchant account validation
supported currency validation
capability validation
```

where provider contract allows.

---

# 201. Webhook Endpoint Rotation

Changing webhook endpoint:

```text
old endpoint
new endpoint
```

should support controlled migration.

Track endpoint version if necessary.

---

# 202. Provider Contract Drift

When provider changes:

```text
API
webhook schema
signature
status vocabulary
rate limits
```

require:

```text
contract review
adapter update
test fixture update
staging verification
release approval
```

---

# 203. No Provider SDK Blind Trust

SDK responses still pass through domain validation.

An SDK can have bugs or be compromised.

Treat SDK output as provider-derived input, not automatically valid business truth.

---

# 204. Dependency Security

Payment adapters are security-critical.

Run:

```text
cargo audit
dependency policy
SAST
secret scan
```

for payment-related dependencies.

---

# 205. Crypto

Do not implement:

```text
custom HMAC
custom signing
custom encryption
```

Use maintained cryptographic libraries and provider-specified algorithms.

---

# 206. Signature Verification

Signature verification must fail closed.

Cases:

```text
missing signature -> reject
malformed signature -> reject
invalid signature -> reject
wrong key -> reject
unsupported algorithm -> reject
expired signature where required -> reject
```

---

# 207. Timing Safety

Use constant-time comparison where applicable to authentication tags or signatures.

Do not hand-roll comparison logic.

---

# 208. Request Body Handling

If provider signs raw body:

```text
capture exact bytes
verify
then parse
```

Do not allow middleware to transform the body before verification unless the provider's contract supports the transformation.

---

# 209. Webhook Header Handling

Do not trust arbitrary proxy-added headers.

Determine the actual application trust model.

Never let a public client supply:

```text
X-Provider-Verified: true
```

and treat it as provider authentication.

---

# 210. Internal Event Trust

Do not assume:

```text
worker event
```

is trusted merely because it originated internally.

Validate event schema and state transitions.

---

# 211. Service Identity

Payment workers use separate service identities where appropriate.

They receive only:

```text
payment operations
provider credentials
required tables/queues
```

not:

```text
tenant admin
user management
full database DDL
```

---

# 212. Least Privilege Database Access

Payment worker role should not have arbitrary:

```text
DROP
ALTER
CREATE ROLE
```

permissions.

---

# 213. Provider Credential Isolation

Each provider's credential namespace should be separate.

Example:

```text
secret/payment/paychangu/collection
secret/payment/paychangu/refund
secret/payment/providerB/collection
```

when operationally appropriate.

---

# 214. Merchant Credential Rotation

Merchant/provider credentials may rotate without changing financial history.

Historical transaction records keep:

```text
provider
transaction ID
configuration version
```

not the secret.

---

# 215. Payment Data Recovery

After disaster recovery:

```text
payments
provider references
webhook events
reconciliation cases
audit
outbox
```

must remain mutually reconstructable.

---

# 216. Privacy

Payment metadata can contain PII.

Minimize:

```text
mobile numbers
customer names
provider payloads
```

to what the business actually needs.

---

# 217. Customer Mobile Number

If needed for mobile-money initiation:

```text
validate format
normalize
protect in logs
protect exports
encrypt at rest according to data policy
```

Do not expose full numbers unnecessarily.

---

# 218. Customer Consent/Use

Any customer contact information sent to external providers must have a business/legal basis under the applicable product and regulatory model.

The exact legal requirement requires jurisdiction-specific review.

---

# 219. Payment Data Export

Exports may include:

```text
sale
payment
provider reference
amount
currency
status
```

but not provider secrets.

---

# 220. Payment Reporting

Reports should derive from authoritative payment/sale records.

Never make an analytics table the sole payment truth.

---

# 221. Reconciliation Reporting

Operational report:

```text
Pending payments
Unmatched provider transactions
Amount mismatches
Duplicate events
Refund exceptions
```

should be separate from ordinary sales reporting.

---

# 222. Merchant Dispute Handling

A merchant should be able to locate:

```text
sale
payment intent
provider reference
payment state
reconciliation case
```

without having database access.

---

# 223. Support Diagnostics

Support should see enough evidence to diagnose:

```text
payment stuck pending
```

without receiving:

```text
secret
```

or unnecessary private customer data.

---

# 224. Customer-Facing Receipt

Receipt should show:

```text
payment method
amount
appropriate reference
status
```

but not secret provider information.

---

# 225. Receipt State

For pending mobile-money:

```text
PAYMENT PENDING
```

not:

```text
PAID
```

until authoritative settlement.

---

# 226. Duplicate Customer Payment UX

If customer pays twice:

```text
duplicate/overpayment detected
```

should be explicit.

Do not automatically refund unless the domain/provider policy allows and the business has chosen it.

---

# 227. Payment Cancellation

Cancellation is a business state transition.

It is not equivalent to:

```text
delete payment intent
```

Historical evidence remains.

---

# 228. Expired Intent

Expired payment intent may remain queryable for audit but cannot automatically settle under an incompatible new business state.

---

# 229. Merchant Retry UX

When a payment is unknown:

```text
Verify payment
```

should be preferred over:

```text
Pay again
```

until the first attempt's state is known.

---

# 230. Safe Retry Hierarchy

```text
1. Query Sitolo payment state
2. If pending, verify provider
3. If provider unknown, reconcile
4. Only create another attempt when safe
```

---

# 231. Payment State Cache

Payment state may be cached for UI performance but:

```text
cache != authority
```

Critical confirmation queries use authoritative data.

---

# 232. Redis

Redis may accelerate:

```text
rate limits
short-lived coordination
cache
```

but must not become authoritative payment truth or the sole idempotency store.

---

# 233. Payment Database Indexes

Critical access patterns:

```text
payment_intent_id
sale_id
provider + provider_transaction_id
provider + provider_event_id
state + next_verification_at
tenant + created_at
tenant + status
```

Exact indexes should be validated through query plans.

---

# 234. Payment Partitioning

Do not partition payment tables automatically.

Use partitioning only when actual data volume/retention/query patterns justify it.

Potential candidate:

```text
provider_events
audit/payment evidence
```

rather than the core payment intent table initially.

---

# 235. Financial Immutability

Finalized payment facts cannot be silently rewritten.

Corrections use explicit events such as:

```text
PAYMENT_REVERSAL
REFUND
ADJUSTMENT
RECONCILIATION_CORRECTION
```

with appropriate business semantics.

---

# 236. No Negative Payment Amounts

Normal payment amounts must satisfy domain constraints.

Refunds should be separate entities/events rather than:

```text
payment.amount = -1000
```

unless an explicit ledger model defines that representation.

---

# 237. Payment Calculation Precision

Use exact monetary representation.

Never:

```text
float
```

for financial truth.

---

# 238. Currency Configuration

Currency is explicit in every payment.

Default launch currency can be configured for Malawi, but the domain must not hard-code assumptions into every table so regionalization remains possible.

---

# 239. FX

Foreign exchange is not implicit.

If later supported:

```text
source currency
target currency
rate
rate source
rate timestamp
rounding
```

must be explicit.

---

# 240. Provider Currency Restrictions

Provider adapters validate supported currencies before initiation.

---

# 241. Payment Authorization Policy

Authorization is evaluated before initiation:

```text
user
+
tenant
+
branch
+
register
+
payment method
+
amount
```

High-value or exceptional operations may require approval/step-up.

---

# 242. Payment Provider Admin

Changing:

```text
provider credential
provider account
webhook secret
enabled methods
```

is privileged configuration.

It should require:

```text
administrator permission
MFA/step-up
audit
```

---

# 243. Payment Method Feature Flags

A provider can be disabled immediately if:

```text
security incident
provider outage
contract issue
```

The kill switch must fail safe.

Existing payment state remains intact.

---

# 244. Emergency Provider Disable

Disabling a provider means:

```text
new payment initiation blocked
```

not:

```text
historical payments deleted
```

Pending payments move to controlled reconciliation/verification workflows.

---

# 245. Provider Decommissioning

When a provider is removed:

```text
new intents blocked
old transactions retained
historical reconciliation maintained
credentials revoked
webhook disabled
```

---

# 246. Payment Incident Response

If provider credentials leak:

```text
revoke
rotate
disable affected provider
inspect transaction history
inspect webhook events
inspect admin access
compare provider statements
```

---

# 247. Fake Webhook Incident

If forged webhook accepted:

```text
freeze affected settlement path
identify affected transactions
reverse/reconcile financial effects
preserve evidence
rotate webhook secret if needed
patch verification
add regression tests
```

---

# 248. Duplicate Charge Incident

If duplicate external charges occur:

```text
identify provider IDs
map to attempts
determine cause
stop retry source
reconcile
refund/credit according to policy
add idempotency regression test
```

---

# 249. Payment Provider Dispute

Provider says:

```text
SUCCESS
```

Sitolo says:

```text
FAILED
```

Do not silently choose one.

Open:

```text
RECONCILIATION_CASE
```

and preserve both evidence sets.

---

# 250. Reconciliation Evidence

Evidence should include:

```text
provider raw payload
```

only where retention/security policy permits, otherwise:

```text
secure reference
normalized fields
hash
provider timestamp
verification response reference
```

Sensitive raw payloads should be protected and access-controlled.

---

# 251. Raw Provider Payload Storage

Do not blindly store every provider payload forever.

Store only what is needed for:

- dispute;
- audit;
- debugging;
- compliance;
- replay.

Redact secrets.

---

# 252. Webhook Body Hash

A body hash can support evidence/correlation.

It is not a substitute for provider signature verification.

---

# 253. Payment Integration Development Environment

Use:

```text
provider sandbox/test account
```

where available.

Never use production payment credentials in local development.

PayChangu's documentation indicates test transactions can be used after account setup; exact current sandbox behavior must be confirmed from the provider's current environment/account. citeturn456905search6

---

# 254. Synthetic Test Data

Use synthetic:

```text
phone numbers
customer names
payment references
transaction IDs
```

unless provider sandbox requires documented test values.

---

# 255. Contract Test Recording

Do not commit production credentials or sensitive production payloads.

Recorded provider responses must be redacted.

---

# 256. Provider Test Matrix

```text
charge success
charge decline
charge timeout
charge duplicate
charge invalid
verification success
verification failed
verification mismatch
webhook success
webhook duplicate
webhook replay
webhook bad signature
refund success
refund failure
provider 5xx
rate limit
unknown status
```

---

# 257. API Contract Tests

Provider adapter tests verify the normalized interface:

```text
provider input
    ↓
adapter
    ↓
normalized domain result
```

This isolates the rest of Sitolo from provider-specific schema changes.

---

# 258. Provider-Specific Quirks

Provider quirks stay inside the adapter.

Do not leak:

```text
PayChangu-only status codes
```

through core domain types unless business semantics genuinely require them.

---

# 259. Normalized Payment Status

Internal status vocabulary remains stable.

Example:

```text
PENDING
SUCCEEDED
FAILED
EXPIRED
CANCELLED
UNKNOWN
```

Provider-specific raw state can be retained separately.

---

# 260. Adapter Error Boundary

Provider errors should not bubble raw through the merchant API.

Bad:

```json
{
  "provider_error": "...internal provider details..."
}
```

Good:

```json
{
  "code": "PAYMENT_PROVIDER_UNAVAILABLE",
  "message": "The payment provider is temporarily unavailable.",
  "request_id": "..."
}
```

---

# 261. Provider Webhook Raw Data

Merchant-facing APIs do not return raw webhook payloads.

Support sees controlled diagnostic projections.

---

# 262. Payment API Route Inventory

Suggested resource families:

```text
/payments/intents
/payments/intents/{id}
/payments/intents/{id}/verify
/payments/intents/{id}/attempts
/payments/{id}
/payments/{id}/refunds
/payment-webhooks/{provider}
/payment-reconciliation/cases
/payment-reconciliation/cases/{id}
/payment-providers
```

Exact paths remain governed by `api_contract.md`.

---

# 263. Payment Endpoint Security Classification

| Route | Class |
|---|---|
| create intent | authenticated-sensitive |
| view payment | authenticated-sensitive |
| verify payment | authenticated-sensitive |
| refund | privileged-sensitive |
| reconciliation | privileged |
| provider config | admin |
| webhook | provider-authenticated public |
| provider health | restricted/internal |

---

# 264. Payment Request Validation

Validate:

```text
sale_id
payment_method
requested amount if present
currency
provider
customer/payment destination
idempotency key
```

Then authoritative server state supersedes client values where appropriate.

---

# 265. Payment Resource Authorization

A caller must prove:

```text
can operate on sale
```

before accessing its payment.

Do not authorize payment access from:

```text
payment ID alone
```

---

# 266. Payment Ownership

Payment ownership is derived through:

```text
payment
→ payment_intent
→ sale
→ organization
```

and should be backed by database/query scope.

---

# 267. Payment Bulk Operations

Do not allow arbitrary:

```text
bulk verify all provider refs
```

without strict permission/quotas.

Bulk reconciliation should use server-controlled datasets.

---

# 268. Payment CSV/Statement Import

Treat as a high-risk mass operation.

Pipeline:

```text
upload
→ scan
→ parse
→ validate
→ preview
→ approve
→ import
→ reconcile
```

No direct mutation from uploaded rows.

---

# 269. CSV Injection

Export/import pipelines must prevent spreadsheet formulas from becoming executable when opened in spreadsheet applications.

---

# 270. Payment Search

Search filters are typed and bounded.

No arbitrary provider query language.

---

# 271. Payment Pagination

Use bounded pagination.

Never return:

```text
all tenant payments
```

in one request.

---

# 272. Payment Exports

Large exports are asynchronous.

Require:

```text
permission
scope
quota
audit
expiring URL
```

---

# 273. Payment Notifications

Notification:

```text
payment received
```

must be triggered by authoritative payment state.

Do not send customer “payment successful” messages from an unverified webhook.

---

# 274. Payment + Tax

A paid sale may still have:

```text
tax submission pending
```

These states remain separate.

---

# 275. Payment + Inventory

Inventory release/finalization policy must be explicit.

Do not let payment retries create duplicate inventory mutations.

---

# 276. Payment + Sale Correction

A sale reversal does not necessarily mean provider refund.

Those are separate operations:

```text
sale reversal
payment refund
```

and may need coordinated workflows.

---

# 277. Payment + Return

A returned item does not automatically mean refund is complete.

Return:

```text
goods/business event
```

Refund:

```text
money movement
```

must remain distinct.

---

# 278. Refund and Inventory Coordination

Example:

```text
customer returns item
        ↓
return accepted
        ↓
inventory movement
        ↓
refund eligibility
        ↓
refund provider
```

The exact ordering is domain/provider-dependent.

---

# 279. Refund Before Physical Return

If business permits:

```text
refund before physical stock receipt
```

that must be an explicit controlled policy.

Never infer.

---

# 280. Refund Provider Failure After Return

The system may have:

```text
goods returned
refund pending
```

This is why return and refund are separate aggregates/states.

---

# 281. Payment Configuration Audit

Every provider configuration change logs:

```text
actor
tenant
provider
field category
old/new version
reason
timestamp
```

Never log secret values.

---

# 282. Payment Security Definition of Done

Payment integration is complete only when:

```text
[ ] provider contract verified
[ ] adapter boundary exists
[ ] credentials server-side
[ ] initialization idempotent
[ ] provider verification implemented
[ ] webhook authentication implemented
[ ] webhook replay protection implemented
[ ] event deduplication implemented
[ ] amount validation implemented
[ ] currency validation implemented
[ ] provider-context validation implemented
[ ] timeout handling implemented
[ ] retry policy implemented
[ ] circuit breaker implemented where justified
[ ] refund invariant implemented
[ ] reconciliation implemented
[ ] manual exception workflow implemented
[ ] audit implemented
[ ] observability implemented
[ ] cross-tenant tests pass
[ ] concurrency tests pass
[ ] fake-payment tests pass
[ ] duplicate-event tests pass
[ ] secret scanning passes
[ ] provider credentials absent from client
[ ] CI security gates pass
```

---

# 283. Production Certification Gate

Before production payment enablement:

```text
provider sandbox integration passes
+
contract fixtures pass
+
signature tests pass
+
replay tests pass
+
timeout-after-commit passes
+
duplicate-charge scenario passes
+
refund concurrency passes
+
reconciliation scenario passes
+
audit reconstruction passes
+
secret scan passes
+
rollback tested
```

---

# 284. Provider Readiness Gate

A provider is not production-ready until:

```text
documented API
+
authentication
+
transaction verification
+
webhook security
+
idempotency behavior
+
failure semantics
+
rate limits
+
reconciliation source
```

are sufficiently understood.

If a provider cannot provide enough evidence to deterministically identify payment state, Sitolo should not pretend its integration is fully automated.

---

# 285. PayChangu-Specific Current Baseline

Current PayChangu documentation indicates:

- API endpoints use API-key authentication. citeturn456905search5
- Mobile-money collection uses an API charge, customer authorization, a webhook, and transaction verification. citeturn456905search3
- Webhooks carry a `Signature` header that should be validated. citeturn456905search1
- PayChangu recommends re-querying/verification before confirming an order and recommends a background verification/polling strategy for missed webhook notifications. citeturn456905search1

These are external-provider facts and are therefore intentionally kept outside the core domain contract wherever possible.

---

# 286. Important Provider Caveat

Provider documentation can change.

The implementation must verify the exact current:

```text
endpoint
request schema
response schema
signature algorithm
signature input
status vocabulary
idempotency support
refund semantics
rate limits
sandbox behavior
settlement semantics
```

at the time the adapter is implemented.

This document deliberately does not invent unsupported provider capabilities.

---

# 287. Build vs Buy

Sitolo should build:

```text
payment domain state
payment intent semantics
reconciliation rules
merchant allocation
financial invariants
tenant authorization
audit semantics
provider abstraction
fraud/business-abuse controls
```

Reuse:

```text
TLS
HTTP
cryptography
provider APIs
secret management
observability
database
```

---

# 288. What Sitolo Must Never Build

Do not build a custom:

```text
card network
payment rail
cryptographic protocol
banking ledger
```

unless future business/regulatory strategy explicitly changes and architecture review approves it.

---

# 289. Regulatory Boundary

Sitolo's payment role must be reviewed against Malawi's current regulatory requirements and actual contractual relationship with providers before production expansion.

This document is an engineering specification, not a legal opinion.

---

# 290. Future Providers

Future providers can implement:

```text
PaymentProvider
```

without changing:

```text
Sale
PaymentIntent
PaymentAllocation
ReconciliationCase
```

unless their semantics expose a genuinely new domain concept.

---

# 291. Direct Airtel/TNM Integrations

If Sitolo later integrates directly with mobile-money operators rather than through an aggregator, each direct integration is a separate trust boundary.

Do not assume:

```text
PayChangu semantics == direct operator semantics
```

because webhooks, references, settlement and API capabilities can differ.

---

# 292. Provider Aggregator vs Direct Rail

Aggregator advantage:

```text
one integration
multiple rails
```

Tradeoffs:

```text
dependency on aggregator
additional reconciliation layer
provider-specific coverage
fees/contracts
```

Direct rail advantage:

```text
potentially more control
```

Tradeoffs:

```text
more integrations
more credentials
more operational burden
```

The choice belongs in the payment business/integration ADRs.

---

# 293. Merchant Economics

Payment architecture must account for:

```text
provider fees
transaction limits
settlement timing
refund fees
failed transaction behavior
```

but the domain should not hard-code provider pricing into core payment state.

---

# 294. Provider Fees

Represent separately from customer payment amount when applicable:

```text
gross
provider fee
net settlement
currency
```

Historical fee evidence must not modify sale price.

---

# 295. Net Settlement

A provider could report:

```text
customer paid 10,000
provider fee 200
merchant settlement 9,800
```

Sitolo must distinguish:

```text
sale amount = 10,000
```

from:

```text
provider settlement = 9,800
```

when provider semantics require it.

---

# 296. Settlement Reconciliation

Reconciliation can therefore involve:

```text
transaction amount
provider fee
net settlement
settlement batch
```

not only success/failure.

---

# 297. Merchant Statement Matching

If statements provide:

```text
settlement batch ID
```

use that to connect:

```text
payment transactions
→ settlement
```

where the provider contract supports it.

---

# 298. Settlement Delays

Store:

```text
payment confirmed_at
settlement expected_at
settled_at
```

where meaningful.

---

# 299. Provider Partial Settlement

If provider settles multiple transactions in one batch:

```text
batch
  ├── tx1
  ├── tx2
  └── tx3
```

the reconciliation model must preserve those relationships.

---

# 300. Payment Completion Definition

A payment is **Sitolo-confirmed** only when the authoritative internal payment state says so.

It is **externally verified** only when adequate provider evidence exists.

It is **settled** only when the business/provider model says funds reached the appropriate merchant settlement state.

These are separate concepts.

---

# 301. Final Payment State Model

Conceptually:

```text
CUSTOMER INTENT
      |
      v
PAYMENT ATTEMPT
      |
      v
PROVIDER TRANSACTION
      |
      +--> PENDING
      |
      +--> SUCCESS
      |
      +--> FAILURE
      |
      +--> UNKNOWN
      |
      v
VERIFICATION
      |
      v
RECONCILIATION
      |
      v
SITOLO PAYMENT STATE
      |
      v
SALE SETTLEMENT
      |
      v
OPTIONAL SETTLEMENT RECONCILIATION
```

---

# 302. Final Payment Security Contract

> **Sitolo never treats client state as proof of payment. A payment intent is created from authoritative sale state. Every provider interaction occurs through a dedicated adapter and server-side credentials. Provider callbacks are authenticated, bounded, deduplicated and independently verified where supported. Amount, currency, provider, merchant account and transaction identity are validated against the expected payment intent. Network ambiguity is represented as pending/unknown rather than blindly failed or successful. External calls are outside core database transactions. Idempotency, database uniqueness, state-machine validation and concurrency controls prevent duplicate financial effects. Refunds cannot exceed eligible amounts. Ambiguous provider/internal states become explicit reconciliation cases. Financial corrections remain auditable. Payment secrets never enter clients, logs or source control.**

---

# 303. Final Reference Diagram

```text
                         SITOLO PAYMENT ARCHITECTURE

                         ┌─────────────────┐
                         │ Flutter/Tauri   │
                         │ Web/Admin       │
                         └────────┬────────┘
                                  │
                                  │ payment intent
                                  ▼
                         ┌─────────────────┐
                         │ Rust API        │
                         │ Auth/AuthZ      │
                         └────────┬────────┘
                                  │
                                  ▼
                         ┌─────────────────┐
                         │ Payment Domain  │
                         └───────┬─────────┘
                                 │
                    ┌────────────┴────────────┐
                    │                         │
                    ▼                         ▼
             ┌─────────────┐          ┌─────────────┐
             │ PostgreSQL  │          │ Outbox      │
             │ Authority   │          │             │
             └─────────────┘          └──────┬──────┘
                                             │
                                             ▼
                                      ┌──────────────┐
                                      │ Rust Worker  │
                                      └──────┬───────┘
                                             │
                                             ▼
                                      ┌──────────────┐
                                      │ Provider     │
                                      │ Adapter      │
                                      └──────┬───────┘
                                             │
                         ┌───────────────────┼───────────────────┐
                         │                   │                   │
                         ▼                   ▼                   ▼
                       PayChangu          Bank             Future Rail
                         │
                         │ webhook
                         ▼
                  ┌─────────────────┐
                  │ Webhook Gateway │
                  └────────┬────────┘
                           │
                           ▼
                    authenticate
                           │
                           ▼
                      deduplicate
                           │
                           ▼
                    persist event
                           │
                           ▼
                       verify API
                           │
                           ▼
                    reconcile intent
                           │
                           ▼
                  ┌──────────────────┐
                  │ Payment State    │
                  └─────────┬────────┘
                            │
                            ▼
                     Sale Settlement
                            │
                            ▼
                     Audit + Reports
```

---

# 304. Final Implementation Gate

Payment integration must not be enabled for production until:

```text
[ ] exact provider contract verified
[ ] provider credentials managed securely
[ ] payment intent implemented
[ ] attempt identity implemented
[ ] provider adapter implemented
[ ] provider verification implemented
[ ] webhook endpoint implemented
[ ] signature verification tested
[ ] webhook replay tested
[ ] event uniqueness enforced
[ ] amount mismatch tested
[ ] currency mismatch tested
[ ] merchant-account mismatch tested
[ ] timeout-after-commit tested
[ ] provider idempotency tested
[ ] internal idempotency tested
[ ] duplicate settlement tested
[ ] refund concurrency tested
[ ] reconciliation implemented
[ ] ambiguous state implemented
[ ] provider polling/recovery implemented where required
[ ] audit implemented
[ ] metrics/tracing implemented
[ ] cross-tenant tests pass
[ ] frontend payment-tampering tests pass
[ ] secret scanner passes
[ ] provider secrets absent from artifacts
[ ] CI gates are blocking
[ ] incident runbook exists
[ ] sandbox certification passes
[ ] rollback tested
```

---

# 305. Next-Document Contract

`mra_eis_integration_spec.md`, `testing_strategy.md`, `observability_spec.md`, `deployment_spec.md`, and `implementation_plan.md` must consume this document's:

```text
payment state model
provider adapter boundary
webhook security
idempotency model
reconciliation model
failure semantics
audit requirements
test obligations
```

They must not invent competing payment semantics.

---

# 306. Closing Position

Sitolo's payment architecture should remain intentionally boring at the core:

```text
AUTHORITATIVE SALE
       +
PAYMENT INTENT
       +
EXTERNAL PROVIDER
       +
VERIFIED EVIDENCE
       +
RECONCILIATION
       +
IMMUTABLE AUDIT
```

The difficult engineering is in the boundaries:

```text
timeout
replay
duplicate callback
duplicate charge
provider outage
amount mismatch
currency mismatch
refund race
stolen credentials
fake client success
ambiguous external state
```

Those are not edge cases.

For a Business Operating System controlling merchant sales and money, **those failure modes are the payment architecture**.
