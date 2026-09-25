# Sitolo — Phase 4 Part 8 PR-009 Current Status

**Status:** Current review snapshot  
**PR:** #49 — Phase 4 Part 8 audit and outbox implementation  
**Reviewed head:** `49c939983281d36b8a503b374287a809ed8b7415`  
**Base:** `main` at `e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc`  
**Review date:** 2026-09-25  
**Binding contract:** `docs/phase4_part8_audit_outbox_implementation_contract.md`  

---

## 1. Current state

PR #49 is still open. Its current head contains real PostgreSQL audit/outbox persistence code and PostgreSQL integration-test infrastructure.

However, the repository still does not have a production-complete Part 8 execution path.

The current worker binary remains explicitly a Phase 1 scaffold:

```text
apps/worker/src/main.rs
fn main() {}
```

The current PR also does not establish the complete authoritative application command path:

```text
authorized business command
    ↓
business mutation
    +
audit insert
    +
outbox insert
    ↓
same PostgreSQL transaction
    ↓
commit
    ↓
worker relay
    ↓
external side effect
    ↓
durable result
```

The existence of repositories and tests is not itself proof that this end-to-end invariant is wired.

---

## 2. Confirmed improvements in the current PR

The current head contains:

- IAM audit event types;
- PostgreSQL audit persistence;
- transactional outbox persistence boundaries;
- PostgreSQL outbox state-transition functions;
- row-lock/claim infrastructure;
- retry/quarantine concepts;
- PostgreSQL integration-test scaffolding;
- worker-role fixtures;
- expanded tenant/RLS test coverage.

The root workspace now also contains the dependency required by the PR's persistence crate on the PR branch.

---

## 3. Remaining high-impact proof gaps

The detailed execution contract records the full remediation list. The current review should keep these classes visible:

1. real business mutation + audit + outbox atomicity;
2. complete worker relay runtime;
3. safe SECURITY DEFINER ownership/search_path/EXECUTE grants;
4. stale-worker claim protection;
5. concurrency-safe aggregate ordering;
6. bounded worker parameters and retry policy;
7. explicit worker database identity;
8. transaction-correct tenant context tests;
9. isolated PostgreSQL test functions;
10. genuine duplicate-delivery/idempotent-consumer testing;
11. event registry/schema/version enforcement;
12. bounded audit metadata;
13. end-to-end worker/audit observability.

These are implementation/proof gaps, not documentation existence problems.

---

## 4. Interpretation rule

The older file `phase4_part8_pr009_remediation_implementation_instructions.md` is superseded and retained as historical review evidence.

The older `phase4_part8_pr009_remediation_implementation_instructions.md` is retained only as historical review evidence. Any PR-head-specific fact in a historical review must be rechecked against the current PR.

This document is the current PR-status pointer.

---

## 5. Closure

Part 8 is complete only when:

- the binding contract is implemented;
- a real business mutation is transactionally coupled to audit and outbox;
- worker relay exists;
- security-definer privilege boundaries are proven;
- lease ownership is race-safe;
- event ordering is concurrency-safe;
- duplicate delivery reaches an idempotent consumer;
- required telemetry exists;
- PostgreSQL integration/security tests pass;
- canonical CI passes on the exact implementation head.