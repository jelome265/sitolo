## Summary

<!-- One paragraph: what this change does and why. -->

## Scope

<!-- What is in and out of scope for this PR. -->

## Architecture impact

<!-- Which layers/boundaries change (API/application/domain/persistence/integrations). Dependency direction effects. -->

## Security impact

<!-- Authentication, authorization, tenant isolation, secrets, injection, SSRF, web/mobile surface, resource exhaustion. -->

## Tenant-isolation impact

<!-- Cross-tenant negative cases considered and tested. -->

## Data / migration impact

<!-- Schema changes, migration compatibility, locking/concurrency effects, forward-fix/rollback strategy. -->

## Tests

<!-- What tests were added/changed and how they were run. PostgreSQL-integrated tests where the database is part of the security boundary. -->

## Operational impact

<!-- Observability, failure modes, retries, deployment effects. -->

## Rollback

<!-- How this change is rolled back safely. -->

## Documentation

<!-- Docs updated: agent.md, docs/*, external references. -->

---

### Dependency changes?

- [ ] Lockfile intentionally changed
- [ ] Advisory review performed (`cargo audit`)
- [ ] License review performed (`cargo deny check licenses`)
- [ ] Transitive changes inspected (`cargo tree --locked`)

### Workflow changes?

- [ ] Event trust reviewed
- [ ] Permissions minimal and justified
- [ ] No new secrets exposed to untrusted contexts
- [ ] Third-party actions pinned to immutable SHAs
- [ ] Cache trust separated

### Database changes?

- [ ] Migration compatibility reviewed
- [ ] Locking/concurrency effects reviewed
- [ ] Forward-fix / rollback strategy documented