# Sitolo — Offline Continuity Commercial Validation

**Status:** Commercial validation companion  
**Date:** 2026-09-22  
**Technical authority:** existing synchronization and offline architecture documents

---

## 1. Purpose

Offline support is economically relevant because the smallest merchants cannot be assumed to have uninterrupted connectivity.

The test is not “does offline mode exist?”

The test is:

> Can a merchant keep operating during prolonged connectivity loss and trust the result when synchronization returns?

---

## 2. Core Scenario

```text
DAY 1 — ONLINE
   |
   + sales
   + stock changes
   + user activity

DAY 2 — OFFLINE
   |
   + sales
   + permitted local actions

DAY 3 — OFFLINE
   |
   + more activity

DAY 4 — ONLINE
   |
   + synchronize
   + reconcile
   + resolve exceptions
```

---

## 3. Required Business Outcomes

After reconnection:

- accepted sales exist once;
- inventory is consistent with accepted sales;
- payment states are not fabricated;
- user attribution survives;
- timestamps are preserved according to domain rules;
- duplicate commands are not double-applied;
- conflicts are surfaced;
- reports converge to authoritative state.

---

## 4. Merchant-Visible States

The product must distinguish:

- saved locally;
- queued;
- submitted;
- accepted;
- rejected;
- conflict;
- reconciled.

A local “saved” event is not automatically a server-confirmed business event.

---

## 5. Offline Revenue Test

Measure:

- sales attempted offline;
- sales locally committed;
- sales eventually accepted;
- rejected sales;
- duplicate events;
- time to convergence;
- user confusion.

---

## 6. Offline Inventory Test

Measure:

- inventory baseline;
- offline receipts;
- offline sales;
- offline adjustments;
- final authoritative stock;
- divergence.

Do not use a single “offline accuracy” metric without defining the denominator.

---

## 7. Offline Payment Test

A payment recorded while offline must not be represented as provider-confirmed unless there is independent authoritative evidence.

Possible states:

```RECORDED LOCALLY
→ AWAITING PROVIDER
→ CONFIRMED / FAILED / UNKNOWN
```

---

## 8. Conflict Test

Construct cases including:

- same product changed on multiple devices;
- same document replayed;
- stale client state;
- branch-scoped divergence;
- concurrent stock activity.

Measure:

- conflict classification;
- automatic resolution where safe;
- human resolution;
- time to resolution.

---

## 9. Device Loss During Offline Period

Test:

```OFFLINE DEVICE
→ DEVICE LOST
→ NEW DEVICE
→ SERVER AUTHENTICATION
→ RECOVERY
```

Determine which local-only events can and cannot be recovered.

The product must communicate this boundary clearly.

---

## 10. Commercial Metrics

A successful offline experience should improve:

- activation;
- weekly usage;
- retained active days;
- support contacts related to connectivity;
- failed transaction reports.

The feature is economically justified only if these benefits exceed its implementation and support costs.

---

## 11. Final Gate

Offline continuity passes commercial validation only when prolonged offline use:

- is understandable;
- preserves business continuity;
- converges safely;
- does not create duplicate financial effects;
- does not create false payment certainty;
- does not cause unmanageable support load.

