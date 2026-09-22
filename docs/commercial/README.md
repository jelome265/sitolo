# Sitolo Commercial Documentation

This directory contains the complete commercial corpus, physically grouped by bounded commercial concern. **No commercial document has been deleted or merged.** Each source document remains an independent artifact; only its repository path has changed.

## Governance

| Document | Role |
|---|---|
| [`Commercial Corpus Audit`](00_governance/commercial_corpus_audit.md) | Corpus integrity, authority, contradiction and evidence audit |
| [`Commercial Model Precision & Consistency Contract`](00_governance/commercial_model_precision_and_consistency_contract.md) | Canonical precision and consistency rules |
| [`Commercial Metrics Governance & Decision System`](00_governance/commercial_metrics_governance_and_decision_system.md) | Metric, experiment and commercial decision governance |

## Strategy

| Document | Role |
|---|---|
| [`Business Model Design`](01_strategy/business_model_design.md) | Overall business model |
| [`Commercial Operating Model`](01_strategy/commercial_operating_model.md) | Umbrella commercial operating model |
| [`Segment Strategy & Duka Economics`](01_strategy/segment_strategy_and_duka_economics.md) | Duka-to-enterprise segmentation and Duka economics |
| [`Vertical Module Commercial Model`](01_strategy/vertical_module_commercial_model.md) | Vertical packaging and module economics |
| [`Geographic Expansion & Country Entry Strategy`](01_strategy/geographic_expansion_and_country_entry_strategy.md) | Country-entry strategy |

## Validation

| Document | Role |
|---|---|
| [`Commercial Validation Plan`](02_validation/commercial_validation_plan.md) | CV-01–CV-12 validation program, pricing experiments and PMF gates |
| [`Offline Continuity Commercial Validation`](02_validation/offline_continuity_commercial_validation.md) | Offline continuity validation |

## Pricing & Packaging

| Document | Role |
|---|---|
| [`Buyer, User, Approver & Purchase Process`](03_pricing_packaging/buyer_user_approver_and_purchase_process.md) | Buyer/payer/approver and purchase process |
| [`Upgrade, Expansion & Tier Migration Model`](03_pricing_packaging/upgrade_expansion_and_tier_migration_model.md) | Tier migration and expansion |

## Acquisition & Distribution

| Document | Role |
|---|---|
| [`Distribution Channel & Market Access Strategy`](04_acquisition_distribution/distribution_channel_and_market_access_strategy.md) | Acquisition and distribution channels |
| [`Partner, Agent & Reseller Economics and Governance`](04_acquisition_distribution/partner_agent_economics_and_governance.md) | Partner/agent economics and governance |
| [`Unit Economics, CAC & Contribution Model`](04_acquisition_distribution/unit_economics_cac_and_contribution_model.md) | CAC, contribution and channel economics |

## Lifecycle & Service

| Document | Role |
|---|---|
| [`Onboarding & Migration Strategy`](05_lifecycle_service/onboarding_and_migration_strategy.md) | Onboarding and migration |
| [`Retention, Churn & Customer Lifecycle`](05_lifecycle_service/retention_churn_and_customer_lifecycle.md) | Retention, churn and lifecycle |
| [`Support Economics & Service Operations`](05_lifecycle_service/support_economics_and_service_operations.md) | Support operations and support economics |
| [`Trust, Adoption & Operational Continuity`](05_lifecycle_service/trust_adoption_and_operational_continuity.md) | Trust, recovery and continuity |
| [`Data Portability & Data Governance`](05_lifecycle_service/data_portability_and_data_governance.md) | Portability and governance |

## Payments & Finance

| Document | Role |
|---|---|
| [`Payment, Collections & Subscription Billing`](06_payments_finance/payment_collections_and_subscription_billing.md) | Subscription billing and collections |
| [`Mobile Money Reconciliation Business Model`](06_payments_finance/mobile_money_reconciliation_business_model.md) | Mobile-money reconciliation |
| [`Credit & Lay-by Commercial Model`](06_payments_finance/credit_and_layby_commercial_model.md) | Merchant receivables and lay-by boundaries |

## Trust, Regulatory & Fiscal

| Document | Role |
|---|---|
| [`Regulatory Perimeter & Compliance Boundary`](07_trust_compliance/regulatory_perimeter_and_compliance_boundary.md) | Regulatory boundary |
| [`Tax & Fiscal Readiness Strategy`](07_trust_compliance/tax_and_fiscal_readiness_strategy.md) | Fiscal and tax readiness |
| [`Fraud, Loss Prevention & Business Control`](07_trust_compliance/fraud_loss_prevention_and_business_control.md) | Fraud and business controls |

## Intelligence & Defensibility

| Document | Role |
|---|---|
| [`Business Intelligence, Alerts & Action Model`](08_intelligence_defensibility/business_intelligence_alerts_and_action_model.md) | Intelligence and action model |
| [`Defensibility & Moat Strategy`](08_intelligence_defensibility/defensibility_and_moat_strategy.md) | Defensibility and moat |

## Physical grouping contract

The numbered groups are navigation boundaries, not authority boundaries. A document remains authoritative only for the concepts assigned to it by the commercial corpus audit and precision/consistency contract.

The reorganization is non-destructive:

1. no commercial document was merged;
2. no commercial document was rewritten as part of the physical move;
3. no commercial content was discarded;
4. the original Git blobs were reused for the moved documents;
5. the old flat paths were replaced by their grouped paths;
6. future commercial documents must be assigned to exactly one primary group unless the governance documents explicitly authorize an exception.

## Group semantics

```text
00 GOVERNANCE
    ↓ defines authority, consistency and measurement rules

01 STRATEGY
    ↓ defines where Sitolo competes and the commercial model

02 VALIDATION
    ↓ tests whether commercial hypotheses are true

03 PRICING & PACKAGING
    ↓ defines who buys, what is packaged and how accounts migrate

04 ACQUISITION & DISTRIBUTION
    ↓ defines how customers are reached and economics are acquired

05 LIFECYCLE & SERVICE
    ↓ defines activation, retention, support and continuity

06 PAYMENTS & FINANCE
    ↓ defines monetization collection and merchant financial workflows

07 TRUST & COMPLIANCE
    ↓ defines regulatory, fiscal and control boundaries

08 INTELLIGENCE & DEFENSIBILITY
    ↓ defines decision intelligence and earned defensibility
```
