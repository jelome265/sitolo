# Correctness Audit Checks

Apply the checks relevant to the changed behavior and domain.

## Logic and invariants

- Does every state transition preserve the domain invariants?
- Are boundary values, empty/null cases, invalid combinations, and impossible states handled deliberately?
- Can a valid-looking local result violate a higher-level invariant?

## Data and transactions

- Are writes performed against the authoritative source of truth?
- Are transaction boundaries correct for the invariant being protected?
- Can partial failure leave durable state inconsistent?
- Are reads observing the intended consistency level?

## Concurrency and retries

- What happens under duplicate, concurrent, delayed, reordered, or retried execution?
- Could two valid operations race into an invalid state?
- Is retry behavior idempotent or explicitly compensating?

## Control flow and failures

- Are authorization, validation, persistence, and external-effect ordering correct?
- Do errors preserve required invariants rather than merely returning a safe-looking status?
- Are timeout, cancellation, restart, and dependency-failure paths correct?

## Compatibility and contracts

- Does the implementation satisfy the current API/domain contract rather than only the new test cases?
- Does it preserve dependency direction and existing ownership boundaries?
- Are migrations backward/forward compatible where required?

## Observability

- Can the important success/failure state be distinguished in operational evidence?
- Does telemetry remain diagnostic without becoming business truth?
- Is durable audit evidence emitted where the governing contract requires it?

## Evidence rule

Passing tests demonstrate only the behavior they execute. Audit the untested logical space explicitly.
