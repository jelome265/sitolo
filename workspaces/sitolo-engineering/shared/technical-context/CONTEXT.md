# Technical Context Router

Stable technical Layer 3 context.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| System architecture | ../../../../docs/system_architecture_design.md | Relevant sections | Runtime and module invariants |
| Crate ownership contract | ../../../../architecture/crate-boundaries.toml | Applicable crate/layer rules | Executable dependency-direction contract |
| Architecture verifier | ../../../../scripts/ci/check-architecture and check-architecture.ps1 | Full check | Same boundary gate on Bash and PowerShell |
| Domain model | ../../../../docs/domain_model.md | Relevant sections | Bounded contexts and invariants |
| Database | ../../../../docs/database_design.md | Relevant schema, constraints, RLS, transaction sections | Persistence authority |
| API | ../../../../docs/api_contract.md | Relevant endpoint, error, idempotency sections | Public contract |
| Sync | ../../../../docs/sync_protocol.md | Relevant command and convergence sections | Offline semantics |
| Testing | ../../../../docs/testing_strategy.md | Relevant verification sections | Test evidence |
| Observability | ../../../../docs/observability_spec.md | Relevant telemetry sections | Operational evidence |
| Deployment | ../../../../docs/deployment_spec.md | Relevant release and recovery sections | Deployment and operations |
| External standards | ../../../../docs/external_standards_verification_register.md | Relevant versioned standard | Version/authority freshness |

## Rule

This file routes to the canonical corpus. Do not copy technical rules into it.

For any change that adds a crate, moves code between crates, or adds an infrastructure
dependency, read the crate ownership contract and run the architecture verifier. Treat
a boundary violation as a Stage 05 implementation defect, not a Stage 08 paperwork task.
