# Sitolo Phase 0 — Corrections Errata
**Date verified:** 2026-09-05

## 1. SLSA version — FIX
**File:** `security_architecture_design.md`, Appendix C reference #16 and §19.3, §26.2
**Current (wrong):** "SLSA specification v1.1" / `https://slsa.dev/spec/v1.1/`
**Correct:** SLSA **v1.2** is current (released 24 Nov 2025). v1.2 added the Source Track and is backwards-compatible with v1.1.
**Replace with:**
> SLSA specification v1.2 — https://slsa.dev/spec/v1.2/

(`deployment_spec.md` and `threat_model.md` already correctly cite v1.2 — no change needed there.)

---

## 2. PostgreSQL doc citation version — FIX
**File:** `security_architecture_design.md`, reference #5 and §7.2/§20.2 (RLS citations)
**Current (wrong):** `https://www.postgresql.org/docs/17/ddl-rowsecurity.html`
**Correct:** Should reference PostgreSQL **18** docs, consistent with the rest of the doc set (database_design.md, domain_model.md, system_architecture_design.md all correctly target PG18; current minor as of verification is **18.6**, released 2026-08-13).
**Replace with:**
> PostgreSQL 18 Row Security Policies — https://www.postgresql.org/docs/18/ddl-rowsecurity.html

---

## 3. MRA EIS status — FIX (materially outdated in all docs)
**Files:** `mra_eis_integration_spec.md` §3.11 (explicitly flagged as unresolved), `business_model_design.md` §3.3, and any "transition period" language elsewhere.

**Current state of docs:** describe EIS as mid-transition / uncertain deadline, citing conflicting Aug 2025 vs Jan 2026 sources.

**Verified current facts (supersede all prior transition-date language):**
- EIS piloted live 2 Aug 2025 (3-month transition).
- Mandatory go-live pushed to 1 Feb 2026, then a further pilot-extension window ran 1 Feb – 30 Apr 2026.
- **EFDs are no longer permitted after 30 April 2026** — EFD-issued invoices are not valid for VAT input from that point.
- **Nationwide EIS rollout went live 1 May 2026.**
- As of **23 June 2026, MRA confirmed 8,260+ of ~9,000 targeted VAT-registered businesses (91%+) had onboarded.**
- Large taxpayers integrate via API; mid-size taxpayers use the web portal (no dedicated hardware required).

**Action for `mra_eis_integration_spec.md` §3.11:** Replace the "date discrepancy, must not be ignored" paragraph with:
> As of September 2026, MRA's EIS transition is complete and mandatory: EFDs have been disallowed since 30 April 2026, and nationwide EIS rollout has been in effect since 1 May 2026, with >91% of targeted VAT-registered businesses onboarded as of 23 June 2026 (MRA public notice, 23 June 2026). Historical references to "transition period" or EFD/EIS coexistence in older MRA materials are superseded. Sitolo's EIS integration should assume EIS is the sole compliant invoicing path, not an optional/parallel system, for any taxpayer currently subject to the mandate.

This also means: §116/117 references implying EFD fallback or "transition" grace behavior should be treated as historical context only, not current operational assumptions. Terminal onboarding, certification, and production credentialing should be verified as live-system procedures, not pilot-program procedures.

---

## 4. Doc 1 (business_model_design.md)
Left untouched per instruction — not yet finalized, will be revisited later.

---

## 5. Rust version — NO CHANGE NEEDED
Confirmed: **Rust 1.98.1** (released 3 Sept 2026, patches a 1.98.0 vtable-miscompilation bug) is the current latest stable release as of the verification date. `implementation_plan.md` and `security_implementation_spec.md` already state this correctly — no correction required.

---

## Summary of file-level actions
| File | Action |
|---|---|
| `security_architecture_design.md` | Fix SLSA v1.1→v1.2 (3 locations), fix PG17→PG18 RLS link |
| `mra_eis_integration_spec.md` | Replace §3.11 discrepancy note with resolved current-status paragraph |
| `deployment_spec.md` | No change (already correct) |
| `threat_model.md` | No change (already correct) |
| `database_design.md`, `domain_model.md`, `system_architecture_design.md` | No change (already PG18) |
| `implementation_plan.md`, `security_implementation_spec.md` | No change (Rust 1.98.1 already correct) |
| `business_model_design.md` | Skipped per instruction |
