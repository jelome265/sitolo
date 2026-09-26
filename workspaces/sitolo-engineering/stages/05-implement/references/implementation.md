# Implementation Reference

Preserve:

- dependency direction;
- tenant and branch authorization;
- explicit domain transitions;
- authoritative PostgreSQL state;
- history and compensating-operation semantics;
- bounded resources;
- idempotency for retryable mutations;
- external calls outside long database transactions;
- real negative tests for security boundaries.

Prefer existing correct abstractions. Do not introduce an unrelated refactor while implementing a bounded change.
