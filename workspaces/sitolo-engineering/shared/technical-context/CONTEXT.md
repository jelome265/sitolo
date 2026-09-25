# Technical Context Router

Stable technical Layer 3 context.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| System architecture | ../../../../docs/system_architecture_design.md | Relevant sections | Runtime and module invariants |
| Domain model | ../../../../docs/domain_model.md | Relevant sections | Bounded contexts and invariants |
| Database | ../../../../docs/database_design.md | Relevant schema, constraints, RLS, transaction sections | Persistence authority |
| API | ../../../../docs/api_contract.md | Relevant endpoint, error, idempotency sections | Public contract |
| Sync | ../../../../docs/sync_protocol.md | Relevant command and convergence sections | Offline semantics |
| Testing | ../../../../docs/testing_strategy.md | Relevant verification sections | Test evidence |
| Observability | ../../../../docs/observability_spec.md | Relevant telemetry sections | Operational evidence |
| Deployment | ../../../../docs/deployment_spec.md | Relevant release and recovery sections | Deployment and operations |

## Rule

This file routes to the canonical corpus. Do not copy technical rules into it.
