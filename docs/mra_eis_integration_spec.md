# Sitolo — MRA Electronic Invoicing System (EIS) Integration Specification

**Document:** 08 — `mra_eis_integration_spec.md`  
**System:** Sitolo — Business Operating System for African SMEs  
**Primary jurisdiction:** Malawi  
**Integration authority:** Malawi Revenue Authority (MRA)  
**Integration:** Electronic Invoicing System (EIS) API v1  
**Status:** Engineering specification / implementation contract  
**Research baseline:** 4 September 2026  
**Audience:** Staff engineers, backend engineers, mobile engineers, security engineers, SRE/DevOps, QA, compliance/integration engineers, technical leadership.

> This document defines how Sitolo shall integrate with MRA EIS without allowing an external regulatory dependency to corrupt Sitolo's internal financial model. It is intentionally stricter than a simple HTTP API integration guide: the integration is a regulated boundary, a cryptographic boundary, an offline-trust boundary, and an operational evidence boundary.

## 1. Executive Summary

Sitolo shall treat MRA EIS as an external tax-fiscalization authority, not as the authoritative source of Sitolo's internal sales ledger. A successful Sitolo sale must be committed according to Sitolo's own transactional invariants; tax submission to MRA is an external side effect whose lifecycle is tracked separately.

MRA's current EIS developer resources expose an API centered on terminal activation, configuration retrieval, sales submission and utilities. The live Swagger additionally exposes void/cancel receipt, credit/debit note, terminal blocking, site-product retrieval, initial inventory upload, inventory transfer/adjustment, TIN authorization-code checks, VAT-5 certificate validation, and related endpoints. The developer documentation also defines an explicit offline mechanism with terminal-specific thresholds and a terminal-specific secret used to create offline validation signatures. citeturn633109search0turn887457search2turn962055view0turn962055view2

The implementation consequences are non-negotiable:

1. The EIS integration is isolated behind a typed `EisGateway`/adapter boundary.
2. MRA-issued terminal identity and configuration are treated as external integration state, not ordinary merchant-editable configuration.
3. Configuration versions are persisted and associated with tax submissions so historical records remain reproducible even after MRA changes configuration.
4. MRA terminal secrets never cross the general application boundary; only cryptographic operations/results cross the boundary.
5. Online sales use MRA's online submission response to obtain the receipt validation artifact/URL required for the receipt QR flow.
6. Offline sales are permitted only when the activated terminal is authorized for offline operation and only within the MRA-provided thresholds.
7. Offline receipts use the documented MRA HMAC-SHA256 mechanism and terminal secret; the exact algorithm and canonicalization must be implemented exactly as MRA documents, not reinterpreted.
8. Connectivity restoration triggers bounded submission of locally retained offline transactions.
9. Timeout-after-send is treated as an ambiguity state; Sitolo must reconcile with MRA's last-online/last-offline transaction endpoints before retransmitting blindly.
10. MRA rejection never rewrites or deletes the original Sitolo sale. It creates an explicit tax exception and preserves the evidence chain.
11. Production claims of MRA compliance are forbidden until terminal/product approval, test/certification, onboarding, operational procedures, and current MRA requirements have been independently verified.

This specification deliberately distinguishes three kinds of truth:

```text
Sitolo commercial truth
    = what the merchant's system committed as the sale/business event

MRA fiscal truth
    = what MRA accepted/recorded through EIS

Integration evidence truth
    = what Sitolo can prove was generated, signed, transmitted,
      acknowledged, rejected, retried, or reconciled
```

No one of these may be silently substituted for another.

## 2. Authority and Source Hierarchy

The integration shall use this source hierarchy:

### Tier 1 — Current MRA contractual/technical sources

- Current MRA EIS API developer documentation.
- Current MRA Swagger/OpenAPI contract exposed by the EIS API.
- Current MRA taxpayer portal/developer resource center.
- Written MRA-issued integration, certification, onboarding or production instructions supplied directly to the approved integration team.

### Tier 2 — Current MRA legal/regulatory material

- Value Added Tax (Amendment) Act, 2024.
- Current Value Added Tax (Electronic Invoicing System) Regulations and related Government Notices.
- Current MRA public notices describing the transition and operational requirements.

### Tier 3 — Sitolo architecture/domain contracts

- `system_architecture_design.md`
- `domain_model.md`
- `database_design.md`
- `api_contract.md`
- `auth_authorization_spec.md`
- `sync_protocol.md`
- `payment_integration_spec.md`

### Tier 4 — Engineering defaults

OWASP/NIST/RFC conventions and standard Rust security practices may govern implementation quality, but they do not override an MRA-required request field, lifecycle, credential procedure, certification requirement, or fiscalization rule.

### Conflict rule

If MRA's production contract conflicts with an earlier Sitolo assumption, the external contract wins for the integration boundary. Sitolo's security invariants remain independently mandatory. An MRA contract change must result in a reviewed adapter change, migration/compatibility decision, test-fixture update, and ADR if architectural behavior changes.

## 3. Current MRA EIS Facts Verified During This Specification

The following points are supported by current MRA public resources reviewed for this document.

### 3.1 EIS is the current electronic tax-invoicing mechanism

MRA describes EIS as a digital system for issuing tax invoices and maintaining stock records for tax-law purposes. MRA's public material states that taxpayers are expected to acquire EIS or integrate an existing system, register on the EIS taxpayer portal, and upload stock records. MRA's January 2026 public notice states that the EFD-to-EIS transition ended on **31 January 2026**. citeturn170859search13turn170859search12

### 3.2 There is a live EIS API v1 and developer resource center

MRA provides a developer resource center and a live Swagger UI for EIS API v1. The live Swagger currently exposes onboarding, configuration, sales, utilities, stock and other endpoints. citeturn170859search4turn633109search0

### 3.3 A POS/integrating application is treated as a terminal

MRA's terminology explicitly states that a terminal can be software running on Windows, Linux, Android, macOS or another platform, and that a client application connecting to MRA EIS is considered a terminal. Receipts may be printed or sent electronically as long as they contain a QR code that can be scanned for validation. citeturn764139view2

### 3.4 Terminal activation is part of the lifecycle

The documented lifecycle is terminal acquisition → Terminal Activation Code (TAC) → terminal activation API → configuration persistence → terminal-activated confirmation. MRA says the TAC is invalidated after confirmation. Activation requests include environment data and a MRA-issued product identifier/product version; MRA states that the product identifier is issued to POS developers when their terminal software is approved for compliance, with a test product ID available during development. citeturn764139view0turn690690search3turn887457search0

### 3.5 Configuration is not static

MRA requires activated terminals to retrieve current configuration. Current configuration is divided into global, terminal, and taxpayer configuration, with version numbers. The documented response includes tax rates, terminal identity/details, offline thresholds, taxpayer TIN/VAT registration/tax office and activated tax rates/levies. MRA also says outdated configuration can cause failed API calls and indicates configuration refresh can be triggered by the `shouldDownloadLatestConfig` response flag. citeturn887457search1turn887457search3turn764139view1

### 3.6 Online sales are submitted to MRA

MRA's EIS documentation requires an API call for online sales and states that the response includes a validation URL used to generate the receipt QR code. The live Swagger exposes `POST /api/v1/sales/submit-sales-transaction`. citeturn764139view0turn690690search2turn633109search0

### 3.7 Offline sales are explicitly supported under conditions

MRA documents that terminals certified for offline operation can store transactions locally during connectivity problems and later upload them. Offline use is bounded by configured transaction-age and cumulative-amount thresholds; the terminal must enforce those thresholds and stop offline processing when either is exceeded. citeturn764139view3turn962055view2

### 3.8 Offline receipt validation has a documented cryptographic mechanism

MRA's published example uses a terminal-provided secret key, invoice-derived values, HMAC-SHA256 and a generated validation URL for offline receipts. The documentation states that the resulting offline signature is retained with the transaction and must be present when an offline transaction is later submitted. citeturn962055view0

### 3.9 MRA exposes transaction recovery/inspection endpoints

The sales API exposes endpoints for the last submitted online transaction and last submitted offline transaction. MRA specifically describes the last-online endpoint as useful for determining whether a transaction was successfully received after transmission interruptions such as power loss or application crashes. citeturn690690search0turn690690search5

### 3.10 Current MRA surface includes more than sales submission

The live Swagger currently exposes cancellation/voiding, credit/debit notes, VAT-5 certificate validation, TIN authorization-code checks, terminal blocking/unblocking checks, terminal site products, initial inventory upload, stock transfer/adjustment and related operations. Sitolo shall not assume every exposed MRA capability belongs in the first Sitolo release; the adapter shall implement only the approved capability set required for the business/legal scope. citeturn633109search0turn170859search2turn170859search6

### 3.11 Current public materials contain a date discrepancy that must not be ignored

MRA's “Understanding EIS” material describes an implementation/transition sequence beginning in August 2025, while MRA's later public notice states that the EFD-to-EIS transition period ended on 31 January 2026 following publication of the 2025 EIS regulations on 9 January 2026. These materials are not internally identical in their stated transition timeline. The engineering team must use the latest written MRA position supplied for production/certification and must never derive legal compliance deadlines from an old PDF simply because it remains publicly reachable. citeturn170859search13turn170859search12

## 4. Scope

### In scope

- Terminal acquisition/onboarding state representation.
- Terminal Activation Code handling.
- Activation request construction.
- MRA terminal secret protection.
- Terminal activated confirmation.
- Authorization token handling as supplied by the MRA activation lifecycle.
- Configuration retrieval and versioning.
- Global/taxpayer/terminal configuration snapshots.
- Tax rate/levy mapping.
- Product/service mapping required for EIS invoicing.
- Online invoice submission.
- Online response verification and receipt artifact handling.
- Offline eligibility enforcement.
- Offline threshold enforcement.
- Offline receipt HMAC/signature generation.
- Local offline submission queue.
- Online/offline transaction reconciliation with MRA.
- MRA tax acceptance/rejection state machine.
- MRA void/cancellation and credit/debit note boundary design.
- VAT-5 relief flow boundary.
- Buyer TIN and purchase authorization code boundary.
- Terminal blocking/unblocking state detection.
- Integration observability and evidence retention.
- Security controls and test requirements.
- MRA certification/release gates.

### Explicitly out of scope for this version

- Implementing tax law calculations beyond what Sitolo's tax domain and MRA configuration contract require.
- Acting as MRA's tax portal.
- Replacing MRA's taxpayer portal workflows.
- Claiming MRA certification without actual certification evidence.
- Inventing undocumented API endpoints, request signatures, retry semantics or error-code meanings.
- Replacing a merchant's accounting system.
- Treating EIS as Sitolo's source of truth for inventory quantities.

The implementation may expose additional adapter stubs for future approved MRA functions, but undocumented capabilities must remain disabled.

## 5. Architectural Position

The integration belongs in the platform integration layer, downstream of committed business state.

```text
┌────────────────────────────────────────────────────────────┐
│                     Sitolo Clients                         │
│ Flutter POS / Tauri / Admin / Approved API Clients        │
└─────────────────────────────┬──────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│                         Rust Core                           │
│                                                            │
│ Sale Domain → Inventory → Payments → Tax/EIS Projection   │
└─────────────────────────────┬──────────────────────────────┘
                              │
                       Transactional Outbox
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│                  MRA EIS Integration Worker                │
│                                                            │
│ submission scheduler / retries / reconciliation / evidence │
└─────────────────────────────┬──────────────────────────────┘
                              │
                       Typed EisGateway
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│                    MraEisV1Adapter                         │
│ activation / config / sales / offline / utilities / tax   │
└─────────────────────────────┬──────────────────────────────┘
                              │ HTTPS + MRA auth/signatures
                              ▼
                     Malawi Revenue Authority
```

### Critical boundary rule

The sale transaction must never require a successful network round-trip to MRA merely to commit the internal commercial event when Sitolo's own offline/transaction policy allows the sale. Conversely, the tax adapter must never decide whether the sale economically occurred.

### Canonical state sequence

```text
SALE_COMMITTED
      │
      ▼
TAX_SUBMISSION_PENDING
      │
      ├── ONLINE_SUBMITTING ──────┐
      │                           │
      │                           ├── ACCEPTED
      │                           ├── CONFIG_REFRESH_REQUIRED
      │                           ├── RETRYABLE_FAILURE
      │                           └── RECONCILE_REQUIRED
      │
      └── OFFLINE_TAX_ARTIFACT_READY
                    │
                    ▼
           LOCAL_OFFLINE_QUEUE
                    │
                    ▼
           MRA_OFFLINE_SUBMITTING
                    │
                    ├── ACCEPTED
                    ├── RETRYABLE_FAILURE
                    ├── REJECTED
                    └── RECONCILE_REQUIRED
```

## 6. Integration Domain Model

### 6.1 `mra_terminal`

Represents the Sitolo-side projection of an MRA terminal.

```text
mra_terminal
- id
- tenant_id
- branch_id
- terminal_id
- site_id
- activation_state
- product_id
- product_version
- platform_os_name
- platform_os_version
- platform_os_build
- hardware_identity_ref
- mra_access_key_ref
- mra_token_ref
- mra_secret_ref
- global_config_version
- taxpayer_config_version
- terminal_config_version
- offline_enabled
- max_offline_transaction_age_hours
- max_offline_cumulative_amount
- last_config_refresh_at
- last_successful_mra_contact_at
- blocked_state
- blocked_message_ref
- created_at
- activated_at
- deactivated_at
- updated_at
```

Sensitive credentials are references to the secret-management subsystem, not plaintext columns.

### 6.2 `mra_configuration_snapshot`

```text
mra_configuration_snapshot
- id
- terminal_id
- effective_at
- global_version
- taxpayer_version
- terminal_version
- config_hash
- serialized_redacted_snapshot
- created_at
```

A full secret-bearing configuration object must not be stored in ordinary application tables. Persist only the minimum fields required to reproduce tax behavior and audit the external state; secret fields are stored through the dedicated secret boundary.

### 6.3 `mra_tax_rate_snapshot`

```text
mra_tax_rate_snapshot
- snapshot_id
- tax_rate_id
- name
- charge_mode
- ordinal
- rate
- effective_from
- effective_to
```

Tax rates used in an issued transaction must be snapshotted at the time the transaction becomes immutable.

### 6.4 `mra_levy_snapshot`

```text
mra_levy_snapshot
- snapshot_id
- levy_type_id
- name
- charge_mode
- rate
- is_active
```

### 6.5 `tax_submission`

```text
tax_submission
- id
- tenant_id
- sale_id
- terminal_id
- submission_mode             -- ONLINE | OFFLINE
- lifecycle_state
- invoice_number
- request_hash
- canonical_payload_hash
- configuration_snapshot_id
- external_reference
- validation_url
- validation_qr_payload_hash
- offline_signature_ref
- first_attempt_at
- last_attempt_at
- next_attempt_at
- attempt_count
- last_http_status
- last_mra_status_code
- last_mra_remark
- last_mra_error_code
- last_mra_error_field
- accepted_at
- rejected_at
- created_at
- updated_at
```

### 6.6 `tax_submission_attempt`

Every external call is recorded as an attempt, including failures that occur before a definitive response.

```text
tax_submission_attempt
- id
- tax_submission_id
- attempt_number
- request_hash
- request_size_bytes
- response_class
- http_status
- mra_status_code
- mra_remark_redacted
- response_hash
- started_at
- completed_at
- timeout_at
- transport_error_code
- retry_decision
- worker_id
```

### 6.7 `tax_exception`

```text
tax_exception
- id
- tenant_id
- tax_submission_id
- exception_type
- severity
- status
- owner
- reason_code
- human_message
- evidence_ref
- created_at
- acknowledged_at
- resolved_at
- resolved_by
- resolution_code
```

### 6.8 `mra_offline_artifact`

Stores the minimum durable material needed to prove and submit an offline invoice.

```text
mra_offline_artifact
- id
- tax_submission_id
- invoice_number
- invoice_date_time
- item_count
- invoice_total
- vat_amount
- julian_date_value
- julian_date_encoded
- signed_parameter_hash
- offline_signature_ciphertext_ref
- validation_url_ciphertext_ref OR protected encrypted representation
- created_at
```

The secret key is never copied into this table.

## 7. Tenant and Branch Association

MRA's model is taxpayer/site/terminal oriented; Sitolo is tenant/org/branch/device oriented. The mapping must be explicit.

```text
Sitolo tenant
   │
   └── organization
        │
        ├── branch A ── site_id X ── terminal 1
        │                         └─ terminal 2
        │
        └── branch B ── site_id Y ── terminal 3
```

### Invariants

- A terminal may be associated with exactly one active Sitolo branch at a time.
- A terminal's MRA `siteId` is external authoritative data and cannot be arbitrarily edited by a cashier.
- A terminal may only submit sales whose seller/TIN and branch/site context matches its active MRA configuration.
- Moving a device between branches is not a simple UI setting; it is a controlled lifecycle operation that may require MRA reconfiguration/activation.
- Historical submissions retain the old branch/site/configuration references.
- Cross-tenant terminal reuse is forbidden unless the complete MRA lifecycle explicitly supports it and an authorized administrative process records the reassociation.

## 8. Terminal Acquisition and Activation

### 8.1 Lifecycle

```text
MRA taxpayer portal acquisition
              │
              ▼
       Terminal Activation Code
              │
              ▼
    Sitolo activation workflow
              │
              ▼
POST /api/v1/onboarding/activate-terminal
              │
              ▼
MRA returns terminal identity/config/credentials
              │
              ▼
Securely persist required state
              │
              ▼
POST /api/v1/onboarding/terminal-activated-confirmation
              │
              ▼
        TERMINAL_ACTIVE
```

MRA's documentation states that the TAC is obtained from the MRA portal after terminal acquisition, used to obtain activation configuration, and invalidated after successful terminal-activated confirmation. citeturn764139view0turn887457search0

### 8.2 Activation request contract

The documented activation example includes:

```json
{
  "terminalActivationCode": "...",
  "environment": {
    "platform": {
      "osName": "...",
      "osVersion": "...",
      "osBuild": "...",
      "macAddress": "..."
    },
    "pos": {
      "productID": "...",
      "productVersion": "..."
    }
  }
}
```

MRA documents TAC as mandatory, OS name/version as mandatory, OS build as optional, MAC address as mandatory in its example contract, and product ID/product version as mandatory. Product ID is described as a unique identifier issued to approved POS developers, with a test product ID during development. citeturn690690search3

### 8.3 Sitolo activation policy

The UI must not allow arbitrary mutation of the externally required activation fields. The activation workflow shall:

1. Verify the authenticated operator has the `MRA_TERMINAL_ACTIVATE` capability.
2. Confirm the target tenant/branch.
3. Validate product version/build metadata.
4. Accept TAC only over a secure channel.
5. Never log TAC in plaintext.
6. Call MRA activation.
7. Validate the returned terminal identity/configuration schema.
8. Store MRA-provisioned credentials in secret storage.
9. Persist a redacted activation evidence record.
10. Confirm configuration persistence succeeded.
11. Call terminal-activated confirmation.
12. Only then mark the terminal active.

### 8.4 Partial activation failure

If MRA activation succeeds but local persistence fails:

- Do not repeatedly generate activation requests automatically.
- Preserve enough evidence to determine whether MRA now considers the TAC consumed.
- Put the terminal into `ACTIVATION_RECOVERY_REQUIRED`.
- Require controlled re-entry/recovery using the official MRA process.
- Never store or print recovered secrets through ordinary application logs.

### 8.5 Duplicate activation

Activation must not be treated as an idempotent generic operation unless MRA explicitly defines it as such. Sitolo should deduplicate local operator actions, but a repeated MRA activation call requires a deliberate recovery path because the TAC lifecycle is externally stateful.

## 9. Authentication and MRA Credential Handling

MRA's current developer materials show multiple request-header concepts, including `x-signature`, `Authorization`, and `x-access-key` in the API guide index. The documented activation-confirmation request specifically uses an `x-signature` generated by HMAC-SHA512 over the TAC using the secret obtained from activation. The sample configuration/sales requests show an authorization token produced by the terminal activation lifecycle. citeturn212075search1turn212075search0turn690690search4turn690690search6

### Security requirements

- TAC: one-time bootstrap credential; treat as secret.
- MRA authorization token: secret; store encrypted or through secret management; never send to clients unless MRA explicitly requires the terminal client to own it.
- MRA secret key: highest sensitivity within the integration; isolate from application business code.
- Access key/signature material: store and derive only inside the MRA adapter boundary.
- No MRA credential in mobile source, desktop bundles, test screenshots, logs, analytics events, crash dumps, or Git history.

### Key-handling architecture

```text
                        Secret Store
                             │
              ┌──────────────┼───────────────┐
              │              │               │
          TAC ref       token ref       secret-key ref
              │              │               │
              └──────────────┼───────────────┘
                             ▼
                    MRA Credential Broker
                             │
                    typed, minimal interface
                             ▼
                      MraEisV1Adapter
```

### Mobile architecture decision

Because MRA defines the terminal as the client application, a certified architecture may require terminal-resident information for offline processing. Sitolo must not prematurely assume that all MRA credentials can be centralized server-side if the MRA certification model requires the approved POS terminal to perform signing and direct submission. The production architecture shall therefore support two modes:

```text
MODE A: Direct certified terminal
Flutter/Tauri terminal → MRA EIS

MODE B: Sitolo brokered integration
Approved terminal → Sitolo secure integration service → MRA
```

**Open decision:** Which mode MRA will certify for Sitolo's final product architecture must be confirmed in writing before production certification. The system must not be designed around an unverified assumption.

## 10. Terminal Configuration Management

MRA's configuration response is versioned across global, terminal and taxpayer scopes. The terminal is expected to refresh configuration regularly and at minimum at the start of the day and/or after restart; MRA also indicates that a `shouldDownloadLatestConfig` signal can require immediate refresh. citeturn764139view0turn887457search3

### 10.1 Stored configuration domains

```text
GlobalConfiguration
 ├── versionNo
 └── taxRates[]

TerminalConfiguration
 ├── versionNo
 ├── terminalLabel
 ├── terminal/site metadata
 └── offlineLimit

TaxpayerConfiguration
 ├── versionNo
 ├── TIN
 ├── VAT registration state
 ├── tax office
 ├── activated tax rates
 └── activated levies
```

### 10.2 Refresh triggers

Configuration refresh shall occur:

- at terminal initialization/startup;
- at minimum according to the MRA-required schedule;
- immediately when MRA instructs the terminal to refresh;
- after a submission response indicates configuration drift;
- after a controlled operator recovery from a configuration error;
- before high-risk operations when locally cached configuration is known to be stale.

### 10.3 Atomic application

A new configuration must never be partially applied.

```text
download
   ↓
validate schema
   ↓
validate version progression
   ↓
validate cryptographic/transport integrity
   ↓
build immutable snapshot
   ↓
commit snapshot atomically
   ↓
activate snapshot
```

If validation fails, retain the prior known-good configuration and mark the terminal `CONFIG_REFRESH_FAILED`. Do not silently merge pieces of two configuration versions.

### 10.4 Historical reproducibility

A sale issued under configuration versions `(G, T, P)` must retain those exact references. Later refreshes do not rewrite history.

This is necessary because MRA's sale request contract explicitly includes `globalConfigVersion`, `taxpayerConfigVersion`, and `terminalConfigVersion`. citeturn690690search6

## 11. Product and Tax Identity Mapping

MRA's current API exposes product/status and terminal-site product retrieval, while its portal guidance says taxpayers must upload stock and ensure products are mapped appropriately. The developer API documentation shows product codes are part of invoice line items and product/status checking can be used to establish mapping state. citeturn170859search2turn170859search6turn633109search0

### Sitolo product mapping model

```text
sitolo_product
    │
    └── mra_product_mapping
           ├── terminal/site scope
           ├── mra_product_code
           ├── mra_mapping_state
           ├── last_verified_at
           ├── source_config_version
           └── verification_evidence_ref
```

### Rules

- Do not let a cashier type arbitrary MRA product codes at checkout.
- Product identity is determined by controlled mapping/configuration.
- A local product can exist before MRA mapping, but sale-to-MRA submission may be blocked or converted to an explicit `PRODUCT_MAPPING_REQUIRED` exception depending on the approved tax policy.
- The internal Sitolo SKU and the MRA product code are distinct identifiers.
- MRA mapping changes do not rewrite historical line items.

### Initial inventory

MRA's current Swagger exposes an initial inventory upload process. The current endpoint documentation states that products may be uploaded in batches, staged until a final batch marker, classified as mapped/unmapped, then synchronized through the portal and approved before becoming visible in warehouse inventory. It also states this initial inventory operation is one-time. citeturn170859search6

Sitolo's inventory ledger remains authoritative for Sitolo operations. An MRA inventory projection is an integration concern, and differences must be explicit exceptions rather than silent local rewrites.

## 12. Invoice Canonicalization

The MRA request documented for sales contains the following major groups:

- `invoiceHeader`
- `invoiceLineItems`
- `invoiceSummary`

The documented header includes invoice number/time, seller TIN, optional buyer TIN/name/authorization code, site ID, three configuration versions, relief indicator, optional VAT-5 details, and payment method. Line items include product code, description, unit price, quantity, discount, net total, VAT, tax rate ID and product/service flag. The summary includes tax and levy breakdowns, total VAT, invoice total, optional offline signature and amount tendered. citeturn690690search6

### Canonical internal model

Sitolo shall construct the MRA request from immutable internal facts rather than from client-supplied retyped values.

```text
Sale aggregate
 ├── seller identity snapshot
 ├── buyer identity snapshot
 ├── branch/site snapshot
 ├── line facts
 ├── pricing facts
 ├── tax facts
 ├── payment method snapshot
 └── sale issue timestamp
           │
           ▼
     MRA mapping layer
           │
           ▼
    MRA canonical request
```

### Arithmetic invariants

Before submission, the adapter must validate at minimum:

```text
line.total = net amount after discount before VAT
line.totalVAT = tax amount for line
sum(line.total) + sum(line.totalVAT) + sum(levies) = invoiceTotal
sum(taxBreakdown.taxableAmount by rate) = taxable totals derived from lines
sum(taxBreakdown.taxAmount by rate) = totalVAT where applicable
```

Exact arithmetic must use Sitolo's canonical money model and decimal-safe calculations, not binary floating point.

### Authority rule

MRA should receive the tax values produced by the approved Sitolo tax engine/configuration snapshot. The client must not be allowed to override `totalVAT`, `taxRateId`, `levyAmount`, `invoiceTotal`, or seller TIN simply by changing local JSON/SQLite state.

## 13. Invoice Number Strategy

MRA's offline documentation describes an invoice-number construction process involving taxpayer ID, terminal/position, Julian date and a serial transaction count. The exact algorithm shown by MRA must be treated as an external fiscal-numbering contract. It must not be replaced by a generic UUID when MRA requires its documented numbering semantics. citeturn962055view0

### Requirements

- Invoice numbers must be generated by the designated tax/fiscalization component.
- The generation mechanism must be deterministic and crash-safe.
- Serial allocation must be monotonic within the MRA-required scope.
- A committed invoice number must never be reused for another financial event.
- Rollback of an internal transaction must not recycle an already exposed fiscal number.
- Gaps must be treated as evidence/audit events, not automatically “fixed.”
- Multiple terminals at the same site must not generate colliding invoice numbers.

### Open contract verification

The public documentation demonstrates the structure but does not constitute sufficient evidence for every production edge case. Before certification, verify with MRA:

- exact fiscal sequence scope;
- whether sequences are per terminal, site, taxpayer, or another partition;
- behavior on terminal reset/reinstallation;
- behavior on gaps;
- behavior after device loss;
- behavior after configuration changes;
- whether invoice numbers can be generated centrally or must be generated on each certified terminal.

## 14. Online Sale Submission

### 14.1 Normal path

```text
Sale command
   ↓
validate domain invariants
   ↓
commit sale + inventory + payment records
   ↓
write EIS outbox event
   ↓
EIS worker loads immutable sale snapshot
   ↓
ensure terminal/config current
   ↓
build canonical MRA request
   ↓
submit online
   ↓
validate response
   ↓
persist MRA evidence/state
   ↓
receipt gets validation artifact / QR representation
```

### 14.2 MRA response handling

The adapter must distinguish:

- transport failure;
- authentication failure;
- validation/rejection response;
- accepted response;
- accepted response with configuration-refresh instruction;
- ambiguous timeout;
- malformed/contract-drift response.

A successful HTTP transport response is not automatically a fiscal acceptance. The MRA status/result payload must be validated against the expected schema before advancing the tax state.

### 14.3 Receipt QR

MRA's overview says that online submission returns a validation URL and the POS uses that URL to generate the QR code shown on the receipt. Sitolo should store the validation URL or a protected equivalent sufficient to reproduce the QR payload and should make the generated QR data traceable to the tax submission record. citeturn764139view0

### 14.4 Receipt immutability

Once a receipt has been presented to the customer, its fiscal artifact must not be mutated in place. Corrections use the proper MRA/Sitolo correction mechanism.

## 15. Timeout-After-Commit and Ambiguous Outcomes

The most dangerous integration failure is:

```text
Sitolo sends invoice
       ↓
MRA receives invoice
       ↓
MRA commits invoice
       ↓
network response is lost
       ↓
Sitolo sees timeout
```

A naive retry can create a duplicate fiscal transaction.

### Required state

```text
SUBMITTING
   ↓ timeout
RECONCILE_REQUIRED
```

### Reconciliation sequence

1. Preserve the original request hash.
2. Preserve attempt metadata and timeout timestamp.
3. Query MRA's last-submitted-online-transaction endpoint or another official lookup mechanism applicable to the terminal.
4. Compare the returned transaction's stable identifiers/fiscal attributes against the intended submission.
5. If it is the same transaction, mark accepted/reconciled.
6. If MRA shows no evidence and the official integration guidance allows retry, retry with bounded policy.
7. If evidence remains ambiguous, stop automatic retries and create a tax exception.

MRA explicitly describes the last-online endpoint as useful for determining whether an invoice was successfully received after interruptions such as power failure or application crash. citeturn690690search0turn690690search5

### Idempotency limitation

Do not invent an MRA idempotency-key mechanism unless the current MRA contract explicitly documents one. Sitolo's own `submission_id`, payload hash and attempt history protect internal state; they do not create idempotency in an external system that does not support it.

## 16. Offline EIS Architecture

MRA explicitly supports offline transaction processing for terminals certified to work offline. Offline transactions are stored locally and submitted once connectivity returns. MRA provides configurable thresholds based on cumulative transaction amount and transaction age. citeturn764139view3turn962055view2

### 16.1 Two offline domains

Sitolo has two distinct offline mechanisms:

```text
Sitolo offline policy
    = can the merchant continue using Sitolo?

MRA offline policy
    = is this certified terminal allowed to issue this fiscal transaction now?
```

The stricter effective policy wins.

### 16.2 Offline eligibility

A transaction can be MRA-offline only when all of these are true:

```text
terminal active
AND terminal certified/approved for offline use
AND current configuration exists
AND offline threshold not exceeded
AND transaction age relative to MRA rule is valid
AND invoice-number allocation is valid
AND required tax mapping is locally available
AND required cryptographic material is available
AND local durable storage is healthy
```

### 16.3 Threshold enforcement

MRA's documented terminal configuration includes `maxTransactionAgeInHours` and `maxCummulativeAmount`. MRA requires the POS to stop offline transacting when a threshold is exceeded. citeturn962055view2turn887457search1

Sitolo must enforce these thresholds locally using monotonic/durable accounting, not UI estimates.

### 16.4 Cumulative amount accounting

The offline cumulative amount counter must be:

- durable;
- crash-safe;
- monotonic for the relevant fiscal window;
- reconciled after successful submission;
- resistant to client-side tampering;
- tied to the active terminal/configuration state.

A device restore must not reset the counter merely because local application storage was recreated.

### 16.5 Offline age accounting

The device timestamp alone cannot be trusted as an authoritative clock. Sitolo should track:

- transaction issue timestamp;
- local monotonic elapsed time where available;
- last known trustworthy server/MRA contact time;
- time synchronization state;
- clock-skew warnings.

The MRA-specific age rule must follow MRA's documented interpretation. A local clock rollback must never grant extra fiscal offline capacity.

## 17. Offline Receipt Signing

MRA's current developer documentation specifies a lightweight offline signing approach. It constructs a parameter string from invoice-related values, computes HMAC-SHA256 using the terminal-specific secret key received during activation, and creates a validation URL containing the resulting signature. MRA's example includes invoice number, line-item count, invoice total, VAT amount and a base64-encoded Julian date value. The documentation says the `offlineSignature` must be included when the offline transaction is subsequently submitted. citeturn962055view0turn962055view1

### 17.1 Security boundary

```text
MRA terminal secret
       │
       ▼
isolated crypto implementation
       │
       ├── offline signature
       └── validation URL artifact
       │
       ▼
receipt renderer / submission queue
```

The business/domain layer receives only the resulting artifact and a reference to the protected secret operation; it never receives the raw MRA secret.

### 17.2 Exact implementation requirement

The documented parameter ordering, encoding, base64 behavior, URL construction, HMAC algorithm and secret-key interpretation must be implemented exactly as the current MRA developer contract specifies. Do not “clean up” the algorithm for aesthetic reasons.

MRA's separate documentation also describes HMAC-SHA512 for the activation-confirmation `x-signature`, so the implementation must not accidentally reuse the offline HMAC-SHA256 function for onboarding confirmation. citeturn212075search0

### 17.3 Rust crypto interface

Conceptual interface:

```rust
pub trait MraOfflineSigner: Send + Sync {
    fn sign_invoice(
        &self,
        invoice: &MraOfflineInvoiceData,
    ) -> Result<MraOfflineSignatureArtifact, MraCryptoError>;
}
```

The production implementation must:

- avoid unnecessary plaintext key copies;
- zeroize key material where practical;
- avoid logging sensitive input/output;
- use constant-time verification primitives where verification is performed locally;
- include vector tests from the official MRA examples;
- fail closed if required key/configuration state is missing.

### 17.4 Test vectors

MRA publishes examples and an HMAC-SHA512 online test vector for its signature utility. The integration test suite must contain locally generated expected outputs based on official test material while ensuring no production secrets enter the repository. citeturn212075search2

## 18. Offline Submission Queue

MRA requires locally stored offline transactions to be uploaded when connectivity returns and states that the `offlineSignature` identifies the transaction as offline. citeturn962055view1

### Queue record

```text
mra_offline_submission_queue
- id
- tenant_id
- terminal_id
- tax_submission_id
- invoice_number
- sequence_no
- canonical_payload_hash
- offline_signature_hash
- status
- attempt_count
- available_at
- last_attempt_at
- next_attempt_at
- last_error_class
- external_reference
- created_at
- accepted_at
```

### Queue ordering

Unless MRA explicitly authorizes parallel out-of-order submission, prefer the order required by the terminal's fiscal sequence. Parallel workers must not create sequence collisions.

### Submission worker

```text
claim bounded batch
       ↓
verify terminal still active
       ↓
verify offline policy/age
       ↓
load immutable payload
       ↓
submit
       ↓
validate response
       ↓
record evidence
       ↓
ack/remove from pending state
```

A queue worker may run twice. Therefore completion must be determined from durable submission evidence, not from process memory.

## 19. Configuration Drift During Offline Operation

A difficult case occurs when a terminal becomes disconnected under configuration version V1 and reconnects after MRA has moved to V2.

### Rule

Historical offline transactions remain tied to the configuration snapshot under which they were produced. New transactions must use current permitted configuration as soon as the device can refresh.

```text
Offline sale A → config V1 → queued
Reconnect
   ↓
download config V2
   ↓
sale A submitted with original V1 snapshot/evidence
new sale B uses V2
```

Do not rewrite sale A to claim it used V2 merely because submission occurred later.

### Refresh failure

If MRA requires current configuration and Sitolo cannot obtain it:

- stop only those transaction classes that require fresh configuration;
- retain local business continuity for explicitly permitted safe operations;
- surface a clear operational state;
- do not fabricate defaults.

## 20. VAT 5 Relief Supply Boundary

The documented sales contract includes `isReliefSupply` and `vat5CertificateDetails`, and MRA provides a VAT-5 certificate validation endpoint. The documented sales guidance states that relief transactions require VAT-5 certificate validation. citeturn690690search6turn633109search0

### Sitolo flow

```text
Cashier/authorized operator selects relief
            ↓
collect certificate data
            ↓
validate certificate through MRA endpoint
            ↓
receive validated response
            ↓
calculate according to approved tax rules
            ↓
submit invoice with relief fields
```

### Security

- No user may locally mark a sale as tax-relieved merely by toggling a flag.
- Validation evidence is retained.
- Certificate identifiers are treated as regulated/business-sensitive data.
- If validation is unavailable, the system must follow the approved business continuity policy; it must not silently grant relief.

### Open legal/product decision

Exact categories and legal treatment of relief supplies must be governed by current MRA/tax rules, not by product-manager assumptions.

## 21. Buyer TIN and Purchase Authorization Code (PAC)

The MRA sales request allows an optional buyer TIN and buyer authorization code. The current MRA portal guidance states that taxpayers may protect their TIN and, where protected, B2B transactions require a valid purchase authorization code generated via the taxpayer portal. The live API exposes endpoints to check whether a TIN requires authorization and to validate an authorization code. citeturn690690search6turn170859search0turn633109search0

### Flow

```text
Buyer TIN entered
      ↓
check TIN authorization requirement
      ↓
if protected → require PAC
      ↓
validate PAC
      ↓
submit B2B invoice
```

### Rules

- Never persist a buyer's authorization code beyond the required audit/evidence period unless current law/contract requires it.
- Treat TINs as sensitive business identity data.
- Do not allow a cashier to bypass PAC validation by removing the buyer TIN from a transaction that is actually B2B.
- Buyer identity corrections follow an explicit correction workflow.

## 22. Payment Method Mapping

MRA's invoice header requires a payment method, with the public example showing values such as Cash, Card and MobileMoney. The payment method sent to MRA is a fiscal classification, not the entire Sitolo payment ledger. citeturn690690search6

### Mapping

```text
Sitolo tender(s)
 ├── CASH
 ├── CARD
 ├── MOBILE_MONEY
 ├── BANK_TRANSFER
 └── OTHER_APPROVED
          │
          ▼
MRA paymentMethod mapping
```

### Split payments

Sitolo may support multiple internal tenders for one sale, while MRA may expose a more constrained field. The exact MRA representation must be verified in the production contract. The mapping must not falsely claim a single payment type when the fiscal contract requires a different encoding.

### Financial principle

MRA tax submission does not confirm that a payment was settled. Payment state remains governed by Sitolo's payment/reconciliation model.

## 23. Voids, Returns, Credit Notes and Debit Notes

The live MRA API exposes `cancel-receipt` and `process-credit-debit-note`. The latter is described as generating a credit note or debit note from an existing invoice depending on whether the updated VAT/total is lower or higher. citeturn633109search0

### Architectural rule

Sitolo must never “edit” a historical MRA-submitted invoice in place.

```text
Original invoice
      │
      ├── void/cancel flow (where legally applicable)
      ├── credit note
      └── debit note
```

### Internal invariants

- Original sale remains immutable.
- Any correction references the original fiscal submission.
- The correction has its own audit trail and lifecycle.
- The system distinguishes commercial return/refund semantics from fiscal credit/debit note semantics.
- An MRA correction response does not rewrite historical inventory/payment events.

### Open contract verification

Current MRA public Swagger confirms endpoint availability, but the precise eligibility conditions, required fields, timing limits, role restrictions, and legal treatment for every correction case must be verified from the production integration/certification documentation before enabling the feature in merchant-facing UI.

## 24. Terminal Blocking and Unblocking

The current MRA Swagger exposes terminal blocking message retrieval and unblock-status checking. citeturn633109search0

### State model

```text
ACTIVE
  │
  ├── MRA_BLOCKED
  │       │
  │       └── awaiting unblock
  │
  └── ACTIVE
```

### Rule

A locally active terminal must not be treated as fiscally usable if MRA reports it as blocked.

Sitolo should surface:

- terminal blocked state;
- MRA blocking reason/message where permitted;
- time detected;
- last successful configuration refresh;
- operational remediation status.

### Offline consequence

The system must not assume that MRA blocking can be ignored indefinitely because the device is offline. The exact blocked-terminal behavior and local enforcement requirements must be validated during certification.

## 25. Error Taxonomy

Do not collapse all MRA failures into `500`.

### 25.1 Transport classes

```text
DNS_FAILURE
TLS_FAILURE
CONNECT_TIMEOUT
READ_TIMEOUT
RESET_BY_PEER
PROXY_FAILURE
NETWORK_UNAVAILABLE
```

### 25.2 Authentication classes

```text
INVALID_TOKEN
EXPIRED_TOKEN
INVALID_SIGNATURE
INVALID_ACCESS_KEY
UNAUTHORIZED_TERMINAL
```

### 25.3 Configuration classes

```text
STALE_CONFIGURATION
TERMINAL_NOT_ACTIVE
TERMINAL_BLOCKED
SITE_MISMATCH
TAXPAYER_MISMATCH
INVALID_TAX_CONFIGURATION
PRODUCT_MAPPING_MISSING
```

### 25.4 Fiscal validation classes

```text
INVALID_INVOICE
INVALID_TAX_RATE
INVALID_TOTAL
INVALID_VAT
INVALID_INVOICE_NUMBER
INVALID_BUYER_AUTHORIZATION
INVALID_VAT5_CERTIFICATE
INVALID_PAYMENT_METHOD
```

### 25.5 Protocol/contract classes

```text
SCHEMA_DRIFT
UNKNOWN_MRA_STATUS
MALFORMED_RESPONSE
UNEXPECTED_CONTENT_TYPE
```

### Retry matrix

| Failure | Auto-retry | Notes |
|---|---:|---|
| DNS/network timeout | Yes, bounded | Reconcile ambiguous sends before blind retry |
| 5xx-style transient transport/service failure | Usually | Follow current MRA guidance and bounded backoff |
| authentication expiry | Yes, after controlled refresh | Never hot-loop credential refresh |
| stale configuration | Yes, after config refresh | Do not resubmit with known-stale config |
| terminal blocked | No | Operational exception / await MRA action |
| invalid tax data | No | Permanent until corrected through approved workflow |
| invalid buyer authorization | No | Require valid buyer authorization |
| malformed response | No automatic repeat | Treat as integration incident/contract drift |
| unknown MRA business result | No | Reconcile and escalate |

Do not infer retryability solely from HTTP status. MRA business result codes have contractual meaning that must be mapped from current documentation/fixtures.

## 26. Retry and Backoff Strategy

### Requirements

- Exponential backoff with jitter.
- Maximum attempt budget per submission class.
- Maximum wall-clock age before human exception.
- Per-terminal concurrency limit.
- Global worker fairness.
- Dead-letter / exception state after bounded retries.
- No retry storm after MRA outage.

### Example policy

```text
attempt 1  → immediate
attempt 2  → short delay
attempt 3  → exponential delay + jitter
...
retry budget exhausted
        ↓
TAX_EXCEPTION / MANUAL_RECONCILIATION
```

The exact numbers should be configured centrally and tuned from production evidence; they are not fiscal rules and therefore must never be embedded as magic constants across the codebase.

## 27. Idempotency and Duplicate Submission Protection

Sitolo must provide strong internal duplicate protection even if MRA does not expose a formal idempotency key.

### Required uniqueness

At minimum:

```text
unique(tenant_id, tax_submission.sale_id)
unique(tenant_id, tax_submission.canonical_payload_hash, fiscal_operation_scope)
```

The second constraint must be designed carefully because two legitimate correction events can have similar payloads; the unique key must describe a true operation identity, not merely coincidental JSON equality.

### Stable operation identity

```text
sale_id
 + fiscal operation type
 + fiscal correction sequence
 = stable Sitolo tax operation ID
```

### Duplicate detector

Before creating a new external call:

1. load submission state;
2. inspect any accepted/reconciled response;
3. inspect latest attempt;
4. if ambiguous, reconcile externally;
5. only then execute a new attempt.

## 28. Canonical Hashing and Evidence

Every external submission should have a deterministic canonical representation. Hashes provide evidence of what Sitolo intended to send without storing every sensitive payload indefinitely.

### Recommended evidence chain

```text
immutable sale facts
      ↓
MRA mapping snapshot
      ↓
canonical MRA request
      ↓
SHA-256 payload hash
      ↓
external attempt
      ↓
response hash + response metadata
      ↓
accepted/rejected state
```

### Requirements

- Canonicalization must be deterministic.
- JSON property ordering should be normalized by a defined serializer rather than incidental language/runtime ordering.
- Decimal representation must be fixed.
- Timestamp format must be fixed.
- Optional-field inclusion rules must be fixed.
- Null vs omitted field semantics must be explicit.

The hash is an evidence mechanism, not an MRA signature unless MRA explicitly defines it as such.

## 29. Offline Device Security

Offline capability materially increases the trust placed in the terminal.

### Threats

- Local database modification.
- Secret extraction.
- Clock rollback.
- Replay of old commands.
- Invoice-number counter rollback.
- Deletion of pending tax submissions.
- Tampering with offline amount counters.
- Theft of a device containing unsent fiscal records.
- Rooted/jailbroken/compromised operating environments.

### Controls

- OS secure storage for key material where possible.
- Application-level authenticated encryption for sensitive local records.
- Tamper-evident record chaining for critical offline evidence.
- Durable monotonically advancing fiscal counters.
- Device identity binding.
- Device revocation on loss.
- Shortest practical credential lifetimes consistent with MRA's contract.
- Minimal local personal/business data.
- Local audit trail for privileged operations.
- Server-side duplicate/replay detection at sync time.

### Do not promise impossible properties

A compromised physical terminal may be able to falsify its local state. The security objective is to make tampering detectable, bound offline operation, protect secrets, and ensure the server/MRA reject or surface invalid submissions. Do not market “tamper-proof” as a guaranteed property without a hardware-backed certification mechanism.

## 30. Sync Protocol Interaction

MRA offline submission must remain separate from Sitolo's general client sync protocol.

```text
Sitolo Sync
    = synchronizes business commands/state

MRA EIS Queue
    = synchronizes fiscal submissions to an external authority
```

A local sale may produce both:

```text
outbox_command → Sitolo server

mra_offline_queue → MRA EIS
```

### Ordering rule

Sitolo server acceptance of a sale does not imply MRA acceptance. Conversely, MRA acceptance does not recreate or replace the Sitolo sale aggregate.

### Reconnect sequence

```text
network restored
     │
     ├── sync Sitolo commands
     │
     ├── refresh auth/device state
     │
     ├── refresh MRA configuration where required
     │
     └── drain MRA offline queue
```

The exact sequencing may be optimized, but fiscal submission must never consume a mutable draft that can diverge while queued.

## 31. Audit Requirements

Each MRA fiscal action must be auditable.

### Minimum evidence

```text
actor
tenant
branch
terminal
sale_id
invoice_number
operation_type
submission_mode
configuration versions
product mappings
request hash
attempt count
timestamps
response class
MRA status / error identifiers
external reference
result
exception/resolution
application version
schema version
```

### Sensitive fields

Do not log:

- raw access tokens;
- TACs;
- MRA secret keys;
- complete authorization headers;
- customer credentials;
- raw PAN/card data;
- unnecessary certificate secrets.

### Evidence retention

Retention must satisfy applicable legal/tax/audit requirements and be defined before production. The system should support separate retention classes for:

- fiscal transaction evidence;
- operational logs;
- security logs;
- raw provider responses;
- redacted diagnostic metadata.

Storage retention should not be inferred casually from application-log defaults.

## 32. Observability

### Metrics

```text
mra_submission_attempts_total{mode,result}
mra_submission_duration_seconds{mode}
mra_submission_retries_total{mode,reason}
mra_submission_reconciliation_total{result}
mra_rejection_total{reason}
mra_config_refresh_total{result}
mra_config_age_seconds
mra_offline_queue_depth
mra_offline_oldest_age_seconds
mra_terminal_blocked_total
mra_contract_drift_total
mra_activation_attempts_total{result}
```

No metric should contain tenant IDs, invoice numbers, TINs, device identifiers or other high-cardinality secrets unless explicitly justified and tightly bounded.

### Logs

Every integration operation should include correlation fields:

```text
trace_id
submission_id
sale_id
terminal_ref
operation
attempt
result
```

Use safe references rather than raw credentials.

### Alerts

Critical:
- broad MRA rejection spike;
- widespread authentication failure;
- secret access failure;
- terminal blocking spike;
- unbounded offline queue growth;
- duplicate/replay detection spike.

High:
- configuration refresh failures;
- reconciliation backlog;
- contract drift detection;
- large increase in retryable transport failures.

Warning:
- increasing queue age;
- stale terminal configuration;
- elevated MRA latency.

## 33. Database Constraints and Transactions

### Local atomicity

When the system commits a sale, the following may be committed in one Sitolo transaction where they are all local facts:

```text
sale
sale_lines
inventory_postings
payment_records
ledger/audit records
transactional_outbox event
```

The MRA network call must not occur inside the same DB transaction.

### Tax submission projection

A worker consumes the outbox event and creates/advances the `tax_submission` record. Its state transitions are transactionally protected.

### Constraint examples

```sql
CHECK (attempt_count >= 0)
CHECK (invoice_number <> '')
CHECK (submission_mode IN ('ONLINE','OFFLINE'))
CHECK (lifecycle_state IN (...))
```

Use foreign keys to sale/tenant/terminal records. Tenant scoping is mandatory.

### No destructive correction

Tax exceptions, rejection, and retry state updates may be mutable; the original sale/tax facts are not.

## 34. Rust Module Boundary

Recommended workspace structure:

```text
crates/
  domain-tax/
  integration-eis/
  integration-mra/
  tax-application/
  tax-persistence/
  tax-worker/
  crypto-boundary/
  evidence/
```

### Traits

```rust
pub trait EisGateway {
    async fn activate_terminal(
        &self,
        request: MraActivationRequest,
    ) -> Result<MraActivationResult, EisError>;

    async fn confirm_terminal_activation(
        &self,
        request: MraActivationConfirmation,
    ) -> Result<MraActivationConfirmationResult, EisError>;

    async fn get_latest_configuration(
        &self,
        terminal: &TerminalContext,
    ) -> Result<MraConfiguration, EisError>;

    async fn submit_sale(
        &self,
        request: MraSaleRequest,
    ) -> Result<MraSaleResponse, EisError>;

    async fn get_last_online_transaction(
        &self,
        terminal: &TerminalContext,
    ) -> Result<LastSubmittedTransaction, EisError>;

    async fn get_last_offline_transaction(
        &self,
        terminal: &TerminalContext,
    ) -> Result<LastSubmittedTransaction, EisError>;
}
```

Actual signatures should use domain-specific typed identifiers and immutable request structs.

### Adapter rule

`domain-tax` must not import HTTP libraries or MRA-specific DTOs.

Only `integration-mra` knows:

- URL paths;
- header construction;
- MRA DTO serialization;
- MRA status/error mapping;
- HMAC details;
- MRA environment URLs.

## 35. HTTP Client Requirements

Use a hardened async HTTP client.

### Mandatory controls

- HTTPS only in production.
- Explicit connect/read/write timeouts.
- Bounded response body size.
- Strict content-type validation.
- TLS certificate validation enabled.
- No insecure redirects to arbitrary hosts.
- No SSRF through merchant-configurable MRA URL fields.
- Connection pooling with bounded concurrency.
- Backpressure under MRA outage.
- Respect server responses without assuming every 2xx is success.

### Environment endpoints

MRA documentation examples currently show development endpoints such as `https://dev-eis-api.mra.mw/...`, while production endpoint selection must be obtained from the current MRA integration/certification package. The endpoint must never be hard-coded in mobile binaries as the only supported target.

Configuration must support:

```text
MRA_ENVIRONMENT = DEV | TEST | CERTIFICATION | PROD
MRA_BASE_URL
```

with an allowlist of approved hostnames per environment. Do not permit tenants to provide arbitrary MRA URLs.

## 36. Contract Versioning and API Drift

The public API documentation currently identifies **EIS API v1**. Public Swagger and documentation pages may change while retaining a v1 label, so the adapter must maintain schema fixtures and contract tests. citeturn887457search2turn633109search0

### Drift detection

CI should detect:

- required field additions/removals;
- type changes;
- enum changes;
- response shape changes;
- authentication-header changes;
- endpoint path changes;
- pagination/limit changes;
- new error-code meanings.

### Strategy

```text
MRA source/spec change
        ↓
fixture update
        ↓
contract test change
        ↓
adapter review
        ↓
compatibility decision
        ↓
controlled release
```

Never silently coerce a breaking external change into an apparently successful internal state.

## 37. Test Strategy

### 37.1 Unit tests

- money/tax arithmetic mapping;
- invoice canonicalization;
- configuration-version mapping;
- offline threshold arithmetic;
- offline HMAC vector generation;
- activation confirmation HMAC-SHA512 vector generation;
- invoice number algorithm;
- error classification;
- retry classification;
- MRA response parsing;
- QR/validation URL representation.

### 37.2 Contract tests

For each endpoint:

```text
request fixture → serializer → expected wire shape
response fixture → parser → canonical internal result
```

At minimum:

- activation success;
- activation rejection;
- activation confirmation success/failure;
- configuration refresh;
- accepted online sale;
- rejected sale;
- stale configuration response;
- last-online reconciliation;
- last-offline reconciliation;
- offline submission accepted;
- offline signature missing/rejected;
- invalid VAT-5 certificate;
- invalid PAC;
- terminal blocked.

### 37.3 Property tests

Useful properties:

```text
canonicalize(canonicalize(request)) == canonicalize(request)

same immutable sale + same config snapshot → same payload hash

offline cumulative amount never decreases inside a fiscal window

offline age never becomes younger because the clock moved backwards
```

### 37.4 Failure injection

Simulate:

- connection drops after request body upload;
- timeout after MRA acceptance;
- response body truncation;
- JSON corruption;
- stale configuration;
- secret-store outage;
- local DB crash;
- worker process death after network send but before DB commit;
- duplicate worker execution.

### 37.5 Production-like certification tests

The actual MRA certification environment/test account must be used where MRA requires it. Real taxpayer transactions must never be created as synthetic tests without explicit authorization.

## 38. Security Test Matrix

| Control | Test |
|---|---|
| tenant isolation | terminal from tenant A cannot submit sale for tenant B |
| branch isolation | terminal cannot submit against unauthorized site |
| secret protection | logs/artifacts contain no TAC/token/secret |
| role control | cashier cannot activate/deactivate terminals |
| device revocation | revoked device cannot submit fiscal operations |
| replay | same operation does not create duplicate internal submission |
| timeout ambiguity | lost response enters reconciliation, not blind retry |
| config drift | stale config is rejected/refreshed as required |
| offline threshold | cumulative threshold blocks additional offline sale |
| offline age | aged transaction is blocked/exceptioned as required |
| clock rollback | clock manipulation cannot extend offline window |
| payload tamper | local line/tax tampering is detected before submission |
| signature tamper | modified offline payload produces invalid signature/rejection |
| certificate abuse | arbitrary VAT-5 value cannot create relief sale |
| PAC abuse | invalid authorization code cannot pass B2B protection |
| contract drift | unexpected response schema fails closed |
| SSRF | tenant cannot choose arbitrary MRA endpoint |
| blocked terminal | locally active state cannot override MRA block |
| historical immutability | config refresh cannot mutate past sale tax snapshots |
| correction safety | void/credit/debit paths create linked new records |

## 39. Operational Runbook — MRA EIS Outage

### Detection

Symptoms:

- transport errors increase;
- MRA latency rises;
- online submissions enter retry/reconciliation;
- offline queue begins growing.

### Response

1. Confirm MRA/API reachability.
2. Confirm no internal DNS/TLS/proxy incident.
3. Check configuration freshness.
4. Check authentication failures separately from transport failures.
5. Freeze unsafe retry amplification if queue pressure grows.
6. Allow approved offline flow when MRA rules permit.
7. Monitor queue age and terminal thresholds.
8. Reconcile ambiguous transactions before resubmitting.
9. Escalate according to MRA support/incident process.
10. Record incident start/end and affected terminal population.

### Recovery

After MRA availability returns:

```text
health check
   ↓
auth/config refresh
   ↓
reconcile ambiguous online attempts
   ↓
drain offline queue in bounded batches
   ↓
verify acceptance/rejection counts
   ↓
resolve residual exceptions
```

No bulk “retry everything” button should exist without bounded controls.

## 40. Operational Runbook — Rejected Invoice

1. Identify `sale_id`, invoice number and terminal.
2. Read the tax-submission state.
3. Record MRA status/error identifiers.
4. Verify configuration versions.
5. Verify product/tax mapping.
6. Verify buyer TIN/PAC if applicable.
7. Verify VAT-5 evidence if relief applies.
8. Determine permanent vs retryable classification.
9. Correct only through an approved workflow.
10. Preserve the original rejection evidence.
11. Retry only when the corrected submission is a new, valid fiscal operation under the MRA contract.
12. Never mutate the original sale merely to hide rejection.

## 41. Operational Runbook — Lost/Compromised Terminal

1. Revoke Sitolo device authorization immediately.
2. Revoke or quarantine terminal credentials where the MRA process allows/required.
3. Record last synchronization and MRA-submission state.
4. Determine outstanding local offline invoices.
5. Preserve server-known evidence.
6. Follow MRA terminal replacement/re-registration procedure.
7. Do not silently rebind a replacement device to an old terminal identity.
8. Reconcile any fiscal sequence gaps.
9. Audit unusual activity before device loss.

A device replacement is a fiscal lifecycle event, not merely an app reinstall.

## 42. Production Deployment Gates

The MRA adapter may ship to production only when all required gates are green.

### Mandatory gates

- Current MRA API contract captured.
- Approved MRA product ID available for target release.
- Certification status confirmed.
- Production credentials provisioned through an approved secret process.
- Terminal onboarding tested.
- Terminal-activated confirmation tested.
- Configuration refresh tested.
- Online sale flow tested.
- Offline flow tested where certified.
- Offline HMAC vectors pass.
- Threshold enforcement tested.
- Timeout/reconciliation tested.
- Void/correction flows tested if enabled.
- VAT-5/PAC flows tested if enabled.
- Terminal blocking handling tested.
- Security regression suite passes.
- Observability dashboards/alerts enabled.
- Incident runbooks reviewed.
- Backup/recovery evidence retention verified.
- Merchant-facing compliance language legally/operationally approved.

## 43. Certification Gate — What Sitolo Must Not Claim

The implementation team must distinguish:

```text
“supports MRA API integration”
≠
“is MRA certified”
≠
“is compliant for every taxpayer/use case”
```

MRA's portal states that MRA certifies integrated POS vendors and provides API documentation to enable integration. MRA's developer materials also describe an approved product identifier issued to POS developers after compliance approval. citeturn170859search1turn690690search3

Therefore Sitolo must not market a release as MRA-compliant merely because:

- an HTTP request returned success in development;
- a QR code can be generated;
- a local demo works offline;
- an API token exists;
- a sample invoice was submitted in a developer environment.

The compliance claim requires actual MRA-approved integration evidence.

## 44. Environment Separation

Maintain strict separation:

```text
LOCAL
  │
  ├── mocked contract tests
  └── no real credentials

DEV/TEST
  │
  └── MRA-provided test identity

CERTIFICATION
  │
  └── MRA-approved certification environment/process

PRODUCTION
  │
  └── real taxpayer/terminal credentials
```

No production credential may be copied into test environments.

No development build may point at production MRA hosts by default.

Environment selection must be explicit and observable at process startup.

## 45. Data Privacy and Minimization

EIS integration may involve:

- TINs;
- buyer names;
- branch/site details;
- supplier/product information;
- transaction values;
- certificate data.

### Rules

- Collect only fields required by MRA and Sitolo operations.
- Encrypt sensitive stored data at rest.
- Restrict support access by tenant and need.
- Redact secrets in observability systems.
- Provide controlled exports of fiscal evidence.
- Avoid copying MRA responses into multiple uncontrolled storage layers.
- Establish retention/deletion policies compatible with applicable tax/legal obligations.

## 46. Performance and Scalability

### Throughput model

MRA submission throughput must be bounded per tenant/terminal and globally.

```text
sale commit path
      ↓
fast durable outbox
      ↓
worker pool
      ↓
per-terminal serialization where required
      ↓
MRA
```

Do not put MRA network latency directly on the primary sale database transaction unless the final certified MRA architecture explicitly requires synchronous fiscal acceptance and the commercial/legal design permits it.

### Backpressure

When MRA is slow:

- queue locally/server-side;
- cap in-flight requests;
- prioritize fiscal recovery over reports/exports;
- expose queue age;
- prevent memory growth by using durable queues.

### Multi-branch scaling

A tenant with 100 terminals should not create 100 unbounded concurrent workers. Use bounded pools and per-terminal concurrency rules.

## 47. DR and Backup Requirements

Backups must include enough information to reconstruct the Sitolo↔MRA evidence relationship:

- sale identifiers;
- tax submission state;
- attempt history;
- configuration snapshot references;
- payload hashes;
- external references;
- exception state;
- fiscal correction links.

Secrets should remain in the dedicated secret-management system and follow its own backup/recovery process.

### Recovery invariant

After DB restore:

```text
restored local tax state
        +
external MRA truth
        +
reconciliation evidence
        =
consistent integration state
```

A restore must never blindly replay every historical outbox event into MRA.

## 48. Support and Administrative Controls

Support staff may need to inspect MRA errors and terminal state, but support must not become an unrestricted fiscal authority.

### Support access

- Read-only by default.
- Tenant-scoped.
- Break-glass actions require explicit approval and audit.
- Credential values remain inaccessible.
- Manual “mark accepted” operations are forbidden unless a controlled reconciliation action is explicitly designed and legally appropriate.

### Safe administrative actions

- trigger configuration refresh;
- retry an eligible transient submission;
- pause a queue;
- resume a queue;
- create an exception case;
- record a human reconciliation decision;
- initiate replacement-terminal workflow.

### Unsafe actions

- editing invoice total after acceptance;
- changing seller TIN on a historical submission;
- replacing an MRA external reference manually;
- deleting a rejection;
- resetting fiscal sequence counters;
- bypassing offline thresholds.

## 49. ADRs Required for This Integration

### ADR-EIS-001 — External fiscal authority boundary

**Decision:** MRA EIS is an external authority integration, not Sitolo's primary financial ledger.

### ADR-EIS-002 — Adapter isolation

**Decision:** MRA DTOs/HTTP/cryptography stay behind `EisGateway`.

### ADR-EIS-003 — Asynchronous submission

**Decision:** Tax submission is generally an outbox/worker side effect of committed local sale state, subject to the final certified architecture.

### ADR-EIS-004 — Offline fiscal queue

**Decision:** Offline transactions use a durable queue and immutable payload evidence.

### ADR-EIS-005 — Configuration snapshots

**Decision:** MRA global/taxpayer/terminal configuration is snapshotted by version and historical transactions reference the exact versions used.

### ADR-EIS-006 — Secret isolation

**Decision:** MRA secret material is isolated from general domain/application code.

### ADR-EIS-007 — Fiscal correction immutability

**Decision:** Voids/credit/debit notes are explicit linked operations; no in-place mutation of historical fiscal records.

### ADR-EIS-008 — Terminal lifecycle

**Decision:** Terminal activation, blocking, revocation and replacement are first-class state machines.

## 50. Open Questions Requiring MRA Confirmation

These are deliberately preserved as open decisions rather than filled with assumptions:

1. What is the exact certified production architecture for a Flutter mobile terminal: direct MRA connection, Sitolo broker, or another approved model?
2. Which MRA environments are currently available for certification in September 2026, and what are the exact base URLs?
3. What exact product ID/version approval evidence is required for the Sitolo release currently under development?
4. What is the current production credential issuance/rotation procedure?
5. What are the exact semantics of all production status/error codes used by current EIS API v1?
6. What is the official idempotency/duplicate-detection guidance for `submit-sales-transaction`?
7. What is the authoritative invoice-number allocation scope and behavior across multiple POS terminals?
8. What is the exact treatment of fiscal gaps after power loss, reinstall, terminal replacement or sequence corruption?
9. What is the production rule when MRA is unreachable but the terminal's offline cumulative amount or age threshold is exhausted?
10. What is the legally approved behavior for terminal blocking discovered while offline?
11. What is the exact retention requirement for invoice, QR/validation URL, tax response and offline-signature evidence?
12. What current correction/void/credit/debit note scenarios are enabled for integrated POS vendors?
13. What are the exact B2B/PAC rules and current buyer authorization flows in production?
14. What VAT-5 workflows and fields are required for every relief category supported by Sitolo?
15. What is the exact MRA certification test suite and pass/fail evidence package?
16. What operational SLAs or support/escalation procedures apply to certified POS integrators?
17. Is terminal/site relocation permitted through software, or must the MRA lifecycle reissue/reconfigure the terminal?
18. Which MRA fields are mandatory by taxpayer class, VAT status, business type or site type?

Each open question must become a tracked integration task and must be closed with written evidence before production enablement.

## 51. Implementation Sequence

### Stage A — Contract lock

- Capture current Swagger/OpenAPI.
- Capture current MRA developer docs.
- Capture certification requirements.
- Freeze external DTO fixtures.
- Record environment/base URLs.

### Stage B — Domain/persistence

- Add terminal state model.
- Add configuration snapshots.
- Add tax submission state machine.
- Add tax attempt/evidence tables.
- Add tax exceptions.
- Add offline queue.

### Stage C — Secure credential boundary

- Integrate secret store.
- Implement activation state machine.
- Implement confirmation HMAC-SHA512.
- Implement access-token lifecycle.
- Add secret redaction tests.

### Stage D — Configuration adapter

- Implement latest configuration call.
- Validate schema.
- Apply snapshots transactionally.
- Implement drift detection.

### Stage E — Online sales

- Implement canonical request builder.
- Implement submission.
- Validate response.
- Store validation artifact.
- Implement ambiguity reconciliation.

### Stage F — Offline sales

- Implement terminal eligibility.
- Implement threshold engine.
- Implement offline invoice numbering.
- Implement HMAC-SHA256 receipt signing.
- Persist offline artifacts.
- Implement queue submission.

### Stage G — Corrections / regulated extras

- VAT-5.
- PAC.
- Void.
- Credit/debit notes.
- Terminal blocking.

### Stage H — Certification

- Execute MRA test process.
- Capture evidence.
- Remediate findings.
- Obtain production approval.

## 52. Definition of Done

MRA EIS integration is not “done” until:

```text
[ ] current MRA contract captured
[ ] production/certification environment confirmed
[ ] approved product ID/version confirmed
[ ] terminal activation implemented
[ ] activation confirmation implemented
[ ] credentials protected
[ ] configuration refresh implemented
[ ] config versions persisted
[ ] tax mappings versioned
[ ] online sale submission implemented
[ ] validation artifact/QR flow implemented
[ ] timeout reconciliation implemented
[ ] offline eligibility implemented
[ ] offline threshold enforcement implemented
[ ] offline signing implemented with official test vectors
[ ] offline queue implemented
[ ] duplicate/replay protections implemented
[ ] rejection/exception workflow implemented
[ ] correction immutability implemented
[ ] VAT-5 flow implemented if in scope
[ ] PAC flow implemented if in scope
[ ] terminal-block handling implemented
[ ] audit evidence implemented
[ ] sensitive-field redaction tested
[ ] contract tests pass
[ ] failure injection passes
[ ] real-device tests pass for target platforms
[ ] certification tests pass
[ ] DR/recovery runbook tested
[ ] compliance claim approved
```

Any unchecked item that is required for the target release is a release blocker.

## 53. Final Engineering Rules

1. **MRA is an external fiscal authority; Sitolo remains responsible for its own commercial state.**
2. **Never treat a network timeout as proof that MRA rejected or accepted an invoice. Reconcile.**
3. **Never blindly replay a fiscal submission.**
4. **Never rewrite a committed sale to repair an external integration failure.**
5. **Never expose MRA terminal secrets to general business code.**
6. **Never bypass MRA offline thresholds.**
7. **Never use stale tax configuration when current configuration is required.**
8. **Never accept client-provided tax totals as authoritative.**
9. **Never invent undocumented MRA API behavior.**
10. **Never claim certification without certification evidence.**
11. **Never let configuration refresh rewrite historical tax snapshots.**
12. **Never destroy fiscal evidence to make dashboards look clean.**
13. **Never let support staff directly manipulate historical fiscal state.**
14. **Never make tenant or branch scope an implicit property. It must be explicitly enforced.**
15. **Never equate Sitolo payment settlement with MRA fiscal acceptance.**
16. **Never equate MRA fiscal acceptance with successful payment settlement.**
17. **Never equate a valid QR image with legal compliance unless the MRA approval/certification requirements are met.**
18. **Availability may degrade before fiscal integrity, confidentiality, or auditability is compromised.**

---

## 54. Official and Primary Sources Consulted

1. MRA EIS API Swagger UI: `https://eis-api.mra.mw/swagger/index.html` — current endpoint/schema surface reviewed 4 Sep 2026. citeturn633109search0
2. MRA EIS API overview: `https://eis-api.mra.mw/docs/overview.htm` — terminal lifecycle, configuration refresh, online/offline behavior. citeturn764139view0
3. MRA EIS API v1 developer guide/index: `https://eis-api.mra.mw/docs/index.htm` — API sections, authentication/signature topics, EIS API structure. citeturn887457search2turn212075search1
4. MRA terminal terminology: `https://eis-api.mra.mw/docs/terminal.htm` — terminal/POS definition. citeturn764139view2
5. Terminal activation request: `https://eis-api.mra.mw/docs/request_1.htm` — activation request fields and product approval concept. citeturn690690search3
6. Terminal activation confirmation: `https://eis-api.mra.mw/docs/terminal_activated_confirmation.htm` and request page — TAC invalidation and HMAC-SHA512 confirmation signature. citeturn887457search0turn212075search0
7. Configuration: `https://eis-api.mra.mw/docs/get_latest_configuration.htm` / response — configuration categories and versions. citeturn887457search3turn887457search1
8. Sales: `https://eis-api.mra.mw/docs/sales.htm` / sale transaction request — online submission, reconciliation endpoints, fiscal request fields. citeturn690690search0turn690690search6
9. Offline signing: `https://eis-api.mra.mw/docs/signing_offline_receipts_print.htm` — HMAC-SHA256 offline artifact algorithm. citeturn962055view0
10. Offline thresholds: `https://eis-api.mra.mw/docs/offline_thresholds.htm` — cumulative amount and maximum transaction age. citeturn962055view2
11. Offline submission: `https://eis-api.mra.mw/docs/submitting_offline_transactions.htm` — reconnect/upload behavior and offline signature requirement. citeturn962055view1
12. MRA EIS taxpayer portal / developer resources / FAQ — integration availability, tax invoice obligations, QR validation, PAC, inventory and integrated POS guidance. citeturn170859search4turn170859search1turn170859search0
13. MRA 2026 public notice on transition from EFDs to EIS — transition deadline and integration expectation. citeturn170859search12
14. MRA EIS explanatory material — implementation context and taxpayer expectations. citeturn170859search13
15. Value Added Tax (Amendment) Act, 2024 — statutory electronic invoicing and tax-invoice provisions. citeturn170859search14

> Source freshness rule: before implementing production behavior, re-check the live MRA Swagger and the latest MRA-issued certification/legal materials. Public developer documentation explicitly carries historical 2024 copyright/version context, while the live portal/API surface can change. Production code must follow the currently approved MRA contract.
