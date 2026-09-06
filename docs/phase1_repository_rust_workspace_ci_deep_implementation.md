# SITOLO — PHASE 1 DEEP IMPLEMENTATION
## Repository, Rust Workspace, CI, Supply Chain and Build Governance

**Status:** Binding implementation specification  
**Date:** 2026-09-05  
**Phase:** 1 of the implementation program

This document converts the frozen Sitolo architecture and contract suite into an executable repository/build contract. It is intentionally implementation-heavy: every repository boundary, dependency rule, CI trust boundary, artifact identity rule, test substrate, failure mode, and acceptance gate is defined as an engineering control rather than a suggestion.

The governing rule is:

> A repository change is complete only when source structure, dependency direction, build behavior, tests, security gates, artifact identity, and operational evidence agree.

The authority hierarchy remains: applicable law/regulator requirements → current external provider contract → approved product decision → security architecture/implementation → domain model → database/API/auth/sync/integration specifications → testing/observability/deployment → ADRs → implementation convenience.

Non-negotiable Sitolo invariants remain intact: PostgreSQL is authoritative server state; SQLite is device-local operational state; clients are never final security authority; authorization is server-enforced; tenant isolation is a security boundary; finalized financial facts are not destructively edited; inventory is ledger-oriented; external providers are untrusted boundaries; external side effects require idempotent handling; offline synchronization is domain-aware; security verification is release-blocking; production artifacts are attributable and immutable; Redis, if introduced, is non-authoritative; microservices are not introduced merely for appearance.

---

# 1. PHASE 1 OBJECTIVE

Phase 1 establishes the engineering substrate required before business-domain implementation.

The target chain is:

```text
clean checkout
  ↓
pinned toolchain
  ↓
locked dependency graph
  ↓
architectural dependency validation
  ↓
format/lint/build
  ↓
unit tests
  ↓
PostgreSQL integration tests
  ↓
security checks
  ↓
immutable artifact
  ↓
SBOM + provenance
  ↓
artifact verification
  ↓
protected promotion
```

Phase 1 is not complete because `cargo build` works once. It is complete when this chain is executable and failure is treated as failure.

---

# 2. NON-GOALS

Phase 1 does not implement:

- full identity;
- sessions;
- MFA;
- tenant management;
- catalogue;
- inventory;
- POS;
- payments;
- MRA EIS;
- reporting;
- billing;
- production business workflows.

It creates the boundaries and verification substrate those later phases require.

A placeholder must never be represented as production security or compliance functionality.

---

# 3. CANONICAL REPOSITORY TOPOLOGY

```text
sitolo/
├── .cargo/config.toml
├── .github/
│   ├── CODEOWNERS
│   ├── dependabot.yml
│   ├── pull_request_template.md
│   ├── ISSUE_TEMPLATE/
│   └── workflows/
│       ├── policy.yml
│       ├── rust.yml
│       ├── integration.yml
│       ├── security.yml
│       ├── artifact.yml
│       └── release.yml
├── apps/
│   ├── api/
│   ├── worker/
│   ├── mobile/
│   └── desktop/
├── crates/
│   ├── sitolo-domain/
│   ├── sitolo-application/
│   ├── sitolo-api/
│   ├── sitolo-auth/
│   ├── sitolo-authz/
│   ├── sitolo-tenancy/
│   ├── sitolo-persistence/
│   ├── sitolo-events/
│   ├── sitolo-audit/
│   ├── sitolo-observability/
│   ├── sitolo-config/
│   ├── sitolo-integrations/
│   ├── sitolo-sync/
│   └── sitolo-testkit/
├── migrations/
├── config/
├── docs/
├── scripts/
├── tests/
├── fuzz/
├── infra/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── deny.toml
├── .gitignore
├── .gitattributes
└── README.md
```

The tree is an ownership model. It is not a reason to create meaningless empty crates. Every package must have a coherent responsibility.

---

# 4. RUST WORKSPACE CONTRACT

The workspace exists to enforce architecture.

The intended direction is:

```text
apps
 ↓
interface/application
 ↓
domain

infrastructure
 ↓
application ports
```

The domain must not depend directly on:

- Axum;
- SQLx;
- PostgreSQL clients;
- Redis clients;
- HTTP clients;
- cloud SDKs;
- PayChangu-specific types;
- MRA transport types;
- filesystem/network implementations.

A circular dependency is an architecture failure, not an invitation to create a larger shared crate.

---

# 5. CRATE RESPONSIBILITIES

## 5.1 sitolo-domain

Owns:

```text
entities
value objects
aggregates
domain events
state machines
invariants
domain errors
business policies
```

It must remain infrastructure-independent.

## 5.2 sitolo-application

Owns:

```text
use cases
commands
queries
application orchestration
transaction boundaries
ports
```

It coordinates domain behavior; it is not an HTTP framework.

## 5.3 sitolo-api

Owns:

```text
HTTP routes
request decoding
transport validation
authentication extraction
response mapping
HTTP error representation
```

It must not become the business authority.

## 5.4 sitolo-auth

Owns authentication mechanisms and abstractions.

## 5.5 sitolo-authz

Owns permissions, scope evaluation, resource authorization and step-up requirements.

## 5.6 sitolo-tenancy

Owns tenant, organization, branch and scope context propagation.

## 5.7 sitolo-persistence

Owns SQLx/PostgreSQL implementation, repositories, transactions and mapping.

## 5.8 sitolo-events

Owns internal event contracts. Domain events, integration events, outbox records and provider events remain distinct concepts.

## 5.9 sitolo-audit

Owns durable audit semantics. Logs are not a substitute for audit records.

## 5.10 sitolo-observability

Owns structured tracing, metrics, correlation identifiers and telemetry integration.

## 5.11 sitolo-config

Owns typed configuration loading and validation.

## 5.12 sitolo-integrations

Owns external adapters, including payment and MRA EIS boundaries.

## 5.13 sitolo-sync

Owns the domain-aware offline synchronization protocol.

## 5.14 sitolo-testkit

Owns reusable fixtures and test infrastructure. Production crates must never depend on it.

---

# 6. DOMAIN PURITY ENFORCEMENT

The following are forbidden:

```text
sitolo-domain → axum
sitolo-domain → sqlx
sitolo-domain → reqwest
sitolo-domain → redis
sitolo-domain → provider SDK
```

The policy must eventually be machine-enforced using Cargo metadata inspection or equivalent architecture tests.

The purpose is not style. It prevents external mechanisms from becoming business semantics.

For example, the domain should model:

```text
PaymentAttempt
```

rather than:

```text
PayChanguResponse
```

The adapter converts provider representation into internal meaning.

---

# 7. TOOLCHAIN REPRODUCIBILITY

The repository commits `rust-toolchain.toml`.

The current project baseline is Rust **1.98.1**. The critical rule is that the project pins the approved compiler rather than floating on whatever `stable` happens to mean on a runner.

CI must verify the compiler.

Release builders must use the same declaration.

A toolchain update requires:

```text
compiler update
→ dependency compatibility review
→ fmt
→ clippy
→ tests
→ release build
→ security verification
→ artifact verification
```

Compiler drift is a release-quality problem because it can alter diagnostics, dependency compatibility, code generation and build behavior.

---

# 8. LOCKFILE GOVERNANCE

`Cargo.lock` is a tracked reproducibility boundary for application binaries.

CI and release builds use locked dependency resolution.

Required principle:

```text
committed source + committed lockfile
        ↓
approved dependency graph
```

If CI modifies the lockfile during verification, the job fails.

An intentional dependency update must:

1. modify the manifest;
2. update the lockfile intentionally;
3. inspect direct and transitive changes;
4. inspect advisories;
5. inspect license effects;
6. run tests/lint/build;
7. review the resulting graph;
8. commit the change.

A lockfile prevents one category of dependency drift. It does not make a dependency inherently trustworthy.

---

# 9. CARGO CONFIGURATION

`.cargo/config.toml` must contain only repository-approved configuration.

It must not contain:

```text
developer-specific absolute paths
credentials
private tokens
production registry passwords
```

Source replacement, private registries, custom linkers and unusual build flags alter the trust model and require review.

---

# 10. DEPENDENCY GOVERNANCE

Every dependency must have a reason.

Evaluate:

```text
security history
maintenance
license
transitive graph
compile cost
runtime footprint
API stability
MSRV/toolchain compatibility
```

Do not add a dependency because it saves a few lines if it introduces disproportionate supply-chain or maintenance risk.

Direct dependencies must be declared directly rather than relying on transitive availability.

A dependency update is itself a code change.

---

# 11. FEATURES AND PROFILES

Cargo features must not create unsupported security combinations.

Avoid:

```text
feature explosion
security checks compiled out
production behavior dependent on accidental feature selection
```

Release profiles are performance configuration, not security controls.

Never disable validation, authorization, audit generation or required telemetry merely to optimize release performance.

---

# 12. FORMAT AND LINT GATES

Baseline commands:

```text
cargo fmt --all -- --check

cargo check --workspace --all-targets --all-features --locked

cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

cargo test --workspace --all-targets --all-features --locked

cargo build --workspace --release --locked
```

The exact matrix can evolve as targets are added, but the properties must remain.

Broad lint suppression is prohibited.

A lint exception requires:

```text
specific lint
specific reason
smallest practical scope
review
```

---

# 13. UNSAFE RUST

`unsafe` is exceptional.

Every unsafe block requires:

```text
safety invariant
why safe abstraction is insufficient
minimal scope
tests
review
```

Unsafe code in authentication, persistence, synchronization, financial logic or cryptographic boundaries receives elevated scrutiny.

The project does not claim that `unsafe` is inherently forbidden; it requires evidence.

---

# 14. API AND WORKER BOUNDARIES

API:

```text
HTTP
→ validation
→ principal
→ authorization context
→ application command
→ domain
→ transaction
→ response
```

Worker:

```text
durable work
→ validation
→ application operation
→ transaction
→ external side effect if applicable
→ durable result
→ retry/DLQ classification
```

Business rules must not be duplicated between API and worker paths.

---

# 15. REPOSITORY BOOTSTRAP SEQUENCE

The implementation sequence is:

```text
P1.0 repository skeleton
P1.1 toolchain/workspace
P1.2 dependency/lock policy
P1.3 local bootstrap
P1.4 format/lint/build/test
P1.5 PostgreSQL harness
P1.6 security harness
P1.7 supply-chain controls
P1.8 artifact/SBOM/provenance
P1.9 protected release path
P1.10 exit evidence review
```

Each increment should remain reviewable and should not introduce unrelated product behavior.

---

# 16. LOCAL DEVELOPER BOOTSTRAP

A clean checkout must provide a documented path to:

```text
install/activate pinned toolchain
verify required tools
start disposable dependencies
apply migrations where applicable
run verification
```

The developer path and CI path should use the same underlying scripts where practical.

Friction is a security concern: if the safe path is excessively difficult, developers create bypasses.

---

# 17. CANONICAL VERIFICATION SCRIPT

A repository-level command should converge on:

```text
./scripts/ci/verify
```

Conceptually:

```text
verify toolchain
verify repository policy
verify lockfile
format
lint
compile
unit tests
integration tests
architecture checks
security harness
```

The script must fail non-zero when a required command fails.

Do not hide failures using:

```text
command || true
```

unless the command is explicitly non-blocking and its result is separately reported.

---

# 18. POSTGRESQL TEST HARNESS

Phase 1 must provide a disposable PostgreSQL environment.

Lifecycle:

```text
start
→ health check
→ create test database/roles
→ apply migrations
→ seed fixtures
→ run tests
→ collect diagnostics
→ destroy
```

This is mandatory for later properties involving:

```text
RLS
transactions
constraints
locking
isolation
SQLx behavior
migrations
concurrency
```

A mocked database cannot prove PostgreSQL-specific invariants.

Test credentials must be disposable.

---

# 19. DATABASE SAFETY

Tests must not accidentally target production.

Test configuration should:

- identify the test environment explicitly;
- use disposable credentials;
- use disposable database names;
- reject known production endpoints where practical;
- fail closed when required configuration is absent.

A test harness that can silently connect to production is unacceptable.

---

# 20. TEST DETERMINISM

Tests should not depend on:

```text
execution order
developer timezone
developer locale
Internet availability
arbitrary external provider state
wall-clock sleep
```

Use controlled time for:

```text
expiry
retry windows
idempotency windows
sessions
scheduled operations
```

Randomized tests should record seeds where reproduction depends on them.

A failure should be reproducible from:

```text
source revision
test name
seed/input
environment
```

---

# 21. EXTERNAL INTEGRATION TESTING

Payment and MRA EIS integrations remain adapter boundaries.

Tests should simulate:

```text
success
timeout
duplicate
replay
invalid signature
malformed response
temporary failure
permanent rejection
unknown outcome
provider unavailable
```

Ordinary tests should not depend on arbitrary Internet access.

Use:

```text
fake adapter
local controlled server
sandbox
```

where appropriate.

Production credentials never belong in ordinary CI.

---

# 22. SECURITY TEST FOUNDATION

Phase 1 establishes the execution mechanism; Phase 7 expands the complete security suite.

The initial harness must prove:

```text
security test discovery
tenant fixtures
authorization fixtures
negative assertions
database security tests
blocking CI behavior
report generation
```

Security failures must produce failed verification.

A scanner/test harness crash is not a clean result.

---

# 23. GITHUB ACTIONS TRUST MODEL

Workflow files are executable privileged configuration.

Treat:

```text
.github/workflows/*
```

as security-sensitive code.

The desired trust model is:

```text
untrusted PR
   ↓
isolated validation
   ↓
reviewed source
   ↓
trusted build
   ↓
artifact
   ↓
provenance
   ↓
verification
   ↓
protected promotion
```

Do not give production credentials to arbitrary PR execution.

---

# 24. WORKFLOW PERMISSIONS

The default should be equivalent to:

```yaml
permissions:
  contents: read
```

Elevated permissions are job-specific and justified.

Never grant broad write permissions because a future step may need them.

Release signing/deployment permissions belong only to trusted jobs and protected environments.

---

# 25. WORKFLOW EVENT SECURITY

Review carefully:

```text
pull_request
pull_request_target
workflow_run
workflow_dispatch
repository_dispatch
issue_comment
```

The critical question is:

> What code can execute, under whose identity, with what secrets and permissions?

A workflow using an elevated event must be treated as a trust-boundary change.

---

# 26. WORKFLOW INJECTION

Do not interpolate untrusted event fields directly into shell programs.

Potentially attacker-controlled values include:

```text
PR titles
branch names
commit messages
issue titles
comments
labels
```

Treat them as data.

Prefer safe environment-variable or argument passing.

The rule is:

```text
untrusted event data ≠ shell source
```

---

# 27. THIRD-PARTY ACTION PINNING

Actions are dependencies.

Security-sensitive actions should be pinned to immutable commit SHAs.

A tag such as:

```text
@v4
```

is a mutable reference, not immutable source identity.

Updates should record:

```text
old reference
new SHA
reason
security review
test evidence
```

Pinning reduces mutable-reference risk but does not eliminate the need to review the selected action.

---

# 28. CI CACHE SECURITY

Caches can become trust-boundary bridges.

Review:

```text
who writes cache
who reads cache
what executes from cache
whether privileged jobs consume it
whether keys distinguish trust contexts
```

Never cache credentials.

Never allow an untrusted PR cache to become authoritative input to a trusted signing/release job.

Cold-cache builds must remain possible.

---

# 29. SECRET SCANNING

Detect at minimum:

```text
private keys
API keys
provider credentials
database passwords
tokens
JWT signing secrets
cloud credentials
```

If a real credential is found:

```text
revoke
rotate
determine exposure
remove source exposure
scan history/artifacts
rebuild
verify old credential fails
```

Removing a secret string from the latest commit does not revoke it.

---

# 30. DEPENDENCY SECURITY

Review dependencies for:

```text
known vulnerabilities
reachability
exploitability
maintenance
license
transitive dependencies
runtime relevance
```

A critical exploitable production dependency issue is normally release-blocking unless explicitly risk-accepted under a controlled emergency process.

Scanner output is evidence for engineering classification, not permission to ignore the problem.

---

# 31. LICENSE GOVERNANCE

Dependency licensing must be machine-reportable.

Classify:

```text
runtime
build
development/test
```

A development-only dependency should not accidentally impose runtime assumptions.

New incompatible licensing must block merge until resolved.

---

# 32. ARCHITECTURE POLICY

The CI policy must eventually answer:

```text
Does domain depend on infrastructure?
Does production depend on testkit?
Does provider-specific representation leak into domain?
Does an application bypass approved ports?
```

Architecture checks should be deterministic and blocking.

Documentation alone is insufficient.

---

# 33. CODEOWNERS

Sensitive paths should have explicit ownership:

```text
.github/workflows/
migrations/
crates/sitolo-auth/
crates/sitolo-authz/
crates/sitolo-tenancy/
crates/sitolo-persistence/
crates/sitolo-sync/
crates/sitolo-integrations/
infra/
```

CODEOWNERS complements branch protection; it does not replace it.

---

# 34. PULL REQUEST CONTRACT

A substantive PR should disclose:

```text
Summary
Scope
Architecture impact
Security impact
Tenant-isolation impact
Data/migration impact
Tests
Operational impact
Rollback
Documentation
```

Dependency PRs additionally disclose:

```text
lockfile impact
advisory review
license review
transitive changes
```

Workflow PRs additionally disclose:

```text
event trust
permissions
secrets
action pinning
cache behavior
```

Database PRs additionally disclose:

```text
migration compatibility
locking/concurrency effects
forward-fix/rollback strategy
```

---

# 35. COMMIT DISCIPLINE

Prefer cohesive commits:

```text
chore(repo): establish repository policy
build(rust): establish workspace and toolchain
build(deps): enforce locked dependency resolution
ci: add required Rust verification
ci(security): enforce workflow policy
test(db): establish PostgreSQL harness
ci(release): add artifact provenance
```

Do not combine foundational repository work with unrelated product features merely to reduce PR count.

---

# 36. GENERATED FILE GOVERNANCE

Generated output requires an explicit answer to:

```text
what generates it?
from what input?
with what version?
is it reproducible?
is it committed?
how is drift detected?
```

Normally exclude:

```text
target/
coverage dumps
local logs
temporary build artifacts
```

from source control.

Lockfiles remain explicit exceptions where application reproducibility requires them.

---

# 37. BUILD SCRIPT GOVERNANCE

`build.rs` is executable build logic.

Review:

```text
environment access
filesystem access
command execution
network access
generated source
compiler flags
secret exposure
```

Do not allow compilation to silently fetch arbitrary remote executable material.

Build scripts are part of the supply chain.

---

# 38. ARTIFACT IDENTITY

A production artifact must have:

```text
source SHA
artifact digest
toolchain
target
dependency inventory
SBOM
provenance
```

Never use a mutable branch/tag as the actual deployment identity.

The deployment identity is the immutable digest.

---

# 39. TESTED ARTIFACT = DEPLOYED ARTIFACT

The mandatory promotion chain is:

```text
build A
→ test A
→ scan A
→ attest A
→ promote A
→ deploy A
```

Never:

```text
test A
→ rebuild B
→ deploy B
```

Rebuilding can change dependencies, environment-derived output, timestamps or compiler behavior.

---

# 40. SBOM

Every production artifact receives an SBOM describing the actual dependency composition.

The SBOM must correspond to the artifact digest.

Retain it with:

```text
provenance
test evidence
security evidence
release metadata
```

An SBOM without artifact identity is materially weaker than an SBOM bound to a specific artifact.

---

# 41. PROVENANCE

Provenance should establish:

```text
what source was built
where it was built
under what trusted workflow
with what relevant build inputs
```

A PR must not be able to impersonate a trusted production build identity.

The trusted build environment is therefore separated from arbitrary unreviewed execution.

---

# 42. RELEASE METADATA

A release record should answer:

```text
What is running?
From what source?
With what dependencies?
Built with what compiler?
Which tests passed?
Which security gates passed?
What digest was deployed?
```

Recommended fields:

```text
version
git SHA
build ID
toolchain
target triple
Cargo.lock hash
artifact digest
SBOM reference
provenance reference
migration version
test result
security result
```

No secrets belong in metadata.

---

# 43. CI STATUS SEMANTICS

Required checks use:

```text
PASS
FAIL
NOT_RUN
NOT_APPLICABLE
WAIVED
```

Only a valid PASS is successful.

If a scanner crashes:

```text
FAIL
```

or equivalent infrastructure-error state that blocks release.

Never interpret missing evidence as clean evidence.

---

# 44. RETRIES AND FLAKES

Retries are for transient infrastructure failure.

They are not for:

```text
compilation errors
assertion failures
security failures
architecture violations
```

A flaky test requires:

```text
identity
evidence
owner
tracking issue
review date
temporary mitigation
```

Security tests have a stricter quarantine threshold.

---

# 45. RESOURCE LIMITS

CI jobs must have bounded:

```text
execution time
artifact size
database size
fuzzing duration
parallelism
```

Resource limits protect both reliability and abuse resistance.

A malicious PR must not be able to consume unlimited CI resources.

---

# 46. NETWORK POLICY

Unit tests should not need unrestricted Internet.

Integration jobs that require network access should explicitly identify:

```text
endpoint
purpose
credential
trust level
data class
```

Production credentials are prohibited from ordinary PR execution.

---

# 47. DATABASE MIGRATION POLICY

Migrations are ordered and reviewable.

Once applied to a shared environment, a migration must not simply be rewritten.

The repository must be able to execute:

```text
empty database
→ all migrations
→ schema validation
→ integration tests
```

Later phases add compatibility and zero/low-downtime migration rules.

Phase 1 establishes the mechanism.

---

# 48. FAILURE MODE REGISTER

| Failure | Trigger | Consequence | Mandatory control |
|---|---|---|---|
| Toolchain drift | Different compiler | Non-reproducible build | Pinned toolchain |
| Lockfile drift | CI resolves new graph | Untested dependencies | Locked builds |
| Action mutation | Mutable action tag changes | Workflow compromise | SHA pinning |
| PR secret exposure | Untrusted code receives secret | Credential theft | No production secrets |
| Cache poisoning | Untrusted cache consumed by trusted job | Build compromise | Trust-separated caches |
| Artifact substitution | Different binary deployed | Invalid test evidence | Digest promotion |
| Scanner crash | Scanner exits unexpectedly | False assurance | Fail closed |
| Migration drift | Schema differs from history | Runtime inconsistency | Migration CI |
| Architecture bypass | Domain imports infrastructure | Coupling/erosion | Dependency gate |
| Provider leakage | SDK types leak into domain | Vendor coupling | Adapter boundary |
| Flaky security test | Security test intermittently passes | Unreliable control | Strict quarantine |
| Production DB test | Test points at real DB | Data corruption | Disposable DB |

Each operational implementation should additionally record owner, detection, blocking behavior and recovery.

---

# 49. OPERATIONAL RUNBOOK — CI FAILURE

```text
1. Identify failed job.
2. Identify first failing step.
3. Preserve evidence.
4. Classify product/tooling/infrastructure.
5. Reproduce locally where possible.
6. Correct root cause.
7. Re-run the complete required gate set.
8. Confirm no security control was weakened.
```

Do not repeatedly rerun a deterministic failure.

---

# 50. OPERATIONAL RUNBOOK — SECRET EXPOSURE

```text
1. Determine whether credential is real.
2. Revoke immediately.
3. Rotate replacement credential.
4. Determine exposure window.
5. Inspect repository history/artifacts.
6. Remove source exposure.
7. Rebuild affected artifacts.
8. Verify old credential fails.
9. Record incident evidence.
10. Add regression detection where appropriate.
```

---

# 51. OPERATIONAL RUNBOOK — ARTIFACT MISMATCH

If:

```text
verified digest != promoted digest
```

then:

```text
STOP
```

Investigate:

```text
registry
artifact transfer
promotion workflow
deployment configuration
digest selection
provenance
```

Do not explain the difference as a harmless rebuild.

---

# 52. OPERATIONAL RUNBOOK — WORKFLOW COMPROMISE

```text
1. Freeze production promotion.
2. Disable affected workflow path.
3. Review recent workflow changes.
4. Review permissions/events/secrets.
5. Inspect executed commands.
6. Rotate potentially exposed credentials.
7. Rebuild from trusted source.
8. Review deployment history.
9. Restore controlled promotion only after evidence review.
```

---

# 53. PERFORMANCE WITHOUT SAFETY REGRESSION

CI should be optimized in this order:

```text
parallelize independent jobs
→ safe dependency caching
→ eliminate redundant work
→ improve fixtures
→ targeted test selection
```

Never optimize by:

```text
removing security tests
making required checks advisory
replacing PostgreSQL with mocks for database invariants
disabling provenance
disabling artifact verification
```

Fast unsafe CI is a slower incident.

---

# 54. PHASE 1 PR SEQUENCE

Recommended sequence:

```text
PR-001 repository baseline
PR-002 Rust workspace/toolchain
PR-003 dependency/lock governance
PR-004 repository governance
PR-005 developer bootstrap
PR-006 CI baseline
PR-007 security scanning
PR-008 PostgreSQL harness
PR-009 security harness integration
PR-010 artifact/SBOM/provenance
PR-011 protected release workflow
```

Each PR must be independently reviewable.

---

# 55. PR-001 ACCEPTANCE

Must establish:

```text
README
.gitignore
.gitattributes
documentation index
repository policy
```

Acceptance:

```text
clean checkout
no generated garbage
documentation discoverable
```

---

# 56. PR-002 ACCEPTANCE

Must establish:

```text
Cargo.toml
Cargo.lock
rust-toolchain.toml
workspace members
minimal crates
format/lint configuration
```

Acceptance:

```text
cargo metadata
cargo check
cargo test
cargo fmt --check
cargo clippy
```

---

# 57. PR-003 ACCEPTANCE

Must establish:

```text
lockfile policy
dependency scanner
license policy
update process
```

Acceptance:

```text
unexpected lockfile drift fails
policy violation fails
dependency graph is inspectable
```

---

# 58. PR-004 ACCEPTANCE

Must establish:

```text
CODEOWNERS
PR template
workflow review policy
sensitive-path ownership
```

---

# 59. PR-005 ACCEPTANCE

A clean development environment can:

```text
activate toolchain
run verification
start disposable dependencies
run database tests
```

without undocumented manual steps.

---

# 60. PR-006 ACCEPTANCE

Required branch checks include:

```text
format
lint
compile
unit tests
locked dependency resolution
```

---

# 61. PR-007 ACCEPTANCE

Security pipeline includes:

```text
secret scanning
dependency scanning
workflow policy
basic static analysis
```

Scanner failure cannot produce a green security result.

---

# 62. PR-008 ACCEPTANCE

PostgreSQL harness can:

```text
start
migrate
seed
test
destroy
```

reliably in CI.

---

# 63. PR-009 ACCEPTANCE

Security fixtures can express:

```text
multiple tenants
multiple branches
different roles
different devices
revoked device
```

and negative assertions fail the test when isolation is violated.

---

# 64. PR-010 ACCEPTANCE

Release path produces:

```text
artifact
digest
SBOM
provenance
verification result
```

all tied to the same source identity.

---

# 65. PR-011 ACCEPTANCE

Production promotion requires:

```text
trusted artifact
protected environment
immutable artifact identity
approval policy
post-deployment verification
rollback reference
```

No rebuild after artifact approval.

---

# 66. PHASE 1 SECURITY THREAT MODEL

Primary threats:

```text
malicious PR
workflow injection
dependency compromise
action compromise
secret leakage
cache poisoning
artifact substitution
toolchain drift
lockfile drift
test bypass
scanner failure
provenance forgery
release rebuild
```

The repository itself is part of the security boundary.

A secure application can still be compromised through an insecure build pipeline.

---

# 67. EVIDENCE PACKAGE

At Phase 1 completion retain:

```text
repository tree
Cargo metadata
toolchain output
lockfile hash
format report
lint report
unit-test report
PostgreSQL integration report
security-test report
dependency report
secret-scan report
workflow-policy report
SBOM
provenance
artifact digest
artifact verification result
architecture dependency report
release workflow result
```

Evidence must identify the source revision.

A screenshot of a green CI page is not sufficient as the only evidence.

---

# 68. PHASE 1 EXIT CRITERIA

```text
[ ] clean checkout builds
[ ] toolchain pinned
[ ] workspace explicit
[ ] dependency direction documented
[ ] forbidden dependencies detectable
[ ] Cargo.lock tracked
[ ] locked build verified
[ ] rustfmt blocking
[ ] Clippy blocking
[ ] unit tests blocking
[ ] PostgreSQL harness works
[ ] migration execution works
[ ] security harness works
[ ] security failures block
[ ] dependency scanning active
[ ] secret scanning active
[ ] workflow policy active
[ ] third-party actions appropriately pinned
[ ] PR jobs have no production secrets
[ ] artifact digest generated
[ ] SBOM generated
[ ] provenance generated
[ ] artifact verification works
[ ] protected promotion exists
[ ] rollback reference exists
[ ] repository hygiene enforced
[ ] CODEOWNERS/review controls active
[ ] fail-closed CI semantics verified
[ ] evidence package retained
```

A missing control is not compensated for by a strong neighboring control.

---

# 69. RELATIONSHIP TO PHASE 2

Phase 2 implements:

```text
configuration
secrets
logging
errors
telemetry
```

Phase 1 supplies:

```text
config package boundary
observability package boundary
test environment
secret scanning
CI enforcement
build metadata
```

Phase 1 must not prematurely implement Phase 2 business/security semantics.

---

# 70. RELATIONSHIP TO PHASE 3

Phase 3 relies on:

```text
auth package
authz package
device fixtures
security harness
CI gates
```

Phase 1 prevents those modules from requiring a repository redesign.

---

# 71. RELATIONSHIP TO PHASE 5

Phase 5 requires:

```text
real PostgreSQL CI
migration execution
persistence boundary
database fixtures
```

Those capabilities originate in Phase 1.

---

# 72. RELATIONSHIP TO PHASE 7

Phase 7 expands the security suite established in Phase 1.

Phase 1 creates the mechanism.

Phase 7 creates the comprehensive control coverage.

---

# 73. RELATIONSHIP TO PHASE 20

Production certification depends on accumulated evidence.

Phase 1 establishes:

```text
test evidence
security evidence
artifact identity
SBOM
provenance
promotion evidence
```

Later phases populate these controls with business-specific assurance.

---

# 74. ANTI-PATTERNS

Explicitly rejected:

```text
microservice-first repository
giant common crate
floating compiler
missing lockfile
PR access to production secrets
mutable deployment identity
fail-open scanners
mock-only database assurance
infinite retries
permanent security-test quarantine
provider SDK types in domain
uncontrolled build-time network fetches
post-approval rebuilds
manual production schema edits
documentation-only architecture enforcement
```

Each creates a future failure mode that is cheaper to prevent now than remediate after production.

---

# 75. FINAL CONTRACT

The repository must make the safe path the normal path.

```text
SOURCE
 ↓
REVIEW
 ↓
POLICY
 ↓
BUILD
 ↓
TEST
 ↓
SECURITY
 ↓
ARTIFACT
 ↓
ATTESTATION
 ↓
VERIFICATION
 ↓
PROMOTION
 ↓
DEPLOYMENT
```

No later Sitolo implementation phase may bypass this chain.

A control that exists only in prose is not yet an engineering control.

A control that can execute but cannot fail closed is incomplete.

A control that passes but cannot produce evidence is insufficient for production certification.

Phase 1 is therefore the foundation of the entire implementation program: it establishes repository ownership, Rust architectural boundaries, dependency governance, reproducible builds, real database testing, security enforcement, supply-chain controls, immutable artifact identity, and a controlled promotion path.

**Phase 1 must be complete before Phase 2 implementation begins.**

---

# APPENDIX A — ROOT COMMAND CONTRACT

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
```

These commands are the baseline. Future modules may add additional required commands; they may not silently remove the underlying properties.

---

# APPENDIX B — WORKFLOW REVIEW CHECKLIST

```text
[ ] trigger
[ ] fork behavior
[ ] event trust
[ ] permissions
[ ] secrets
[ ] environment protection
[ ] action source
[ ] immutable pin
[ ] shell interpolation
[ ] cache scope
[ ] artifact handling
[ ] network access
[ ] timeout
[ ] retry
[ ] failure propagation
[ ] untrusted-code boundary
[ ] production authority
```

---

# APPENDIX C — DEPENDENCY REVIEW CHECKLIST

```text
[ ] genuinely required
[ ] correct crate boundary
[ ] license compatible
[ ] security history reviewed
[ ] transitive graph reviewed
[ ] maintenance acceptable
[ ] compile cost acceptable
[ ] runtime cost acceptable
[ ] toolchain compatible
[ ] lockfile intentionally changed
[ ] tests pass
[ ] SBOM updated
[ ] architecture policy passes
```

---

# APPENDIX D — PR ACCEPTANCE TEMPLATE

```text
PR:
Change:
Reason:
Affected boundaries:
New dependencies:
Lockfile changed:
Security impact:
CI impact:
Artifact impact:
Migration impact:
Tests:
Failure modes:
Rollback:
Documentation:
Reviewer:
Evidence:
```

---

# APPENDIX E — PRINCIPAL ENGINEERING RULES

```text
1. Domain meaning is independent of infrastructure.
2. Dependencies are reviewed as supply-chain inputs.
3. Lockfiles are reproducibility controls.
4. Toolchains are pinned.
5. CI is a security boundary.
6. Untrusted PRs do not receive production secrets.
7. Required checks fail closed.
8. PostgreSQL invariants require PostgreSQL tests.
9. Security tests are executable requirements.
10. Artifact identity is cryptographic, not semantic.
11. Tested artifacts are promoted; they are not rebuilt.
12. Provider representations remain behind adapters.
13. Repository architecture is machine-enforced where practical.
14. Evidence is part of completion.
15. Phase 1 controls are prerequisites for later feature phases.
```

**END — PHASE 1 DEEP IMPLEMENTATION SPECIFICATION**

## Control 01 — Repository integrity

**Requirement:** The repository must reject accidental build output, secrets, unexplained binaries and unowned generated files.

**Implementation:** Policy check + ignore rules + secret scan.

**Blocking behavior:** Merge blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 02 — Toolchain integrity

**Requirement:** Every build must identify the approved Rust compiler.

**Implementation:** rust-toolchain.toml + CI verification.

**Blocking behavior:** Release blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 03 — Dependency integrity

**Requirement:** Application dependency resolution must be reproducible.

**Implementation:** Cargo.lock + --locked + dependency review.

**Blocking behavior:** Release blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 04 — Architecture integrity

**Requirement:** Forbidden dependency directions must be detectable.

**Implementation:** cargo metadata policy/architecture test.

**Blocking behavior:** Merge blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 05 — Workflow integrity

**Requirement:** Workflow source is treated as executable privileged configuration.

**Implementation:** Review + permissions + pinned actions.

**Blocking behavior:** Merge blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 06 — Secret integrity

**Requirement:** Untrusted execution cannot receive production credentials.

**Implementation:** Environment separation + secret scanning.

**Blocking behavior:** Release blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 07 — Cache integrity

**Requirement:** Trusted jobs cannot rely on attacker-controlled cache state.

**Implementation:** Scoped cache keys + trust separation + cold-cache path.

**Blocking behavior:** Release blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 08 — Database integrity

**Requirement:** Integration tests use real PostgreSQL where PostgreSQL semantics matter.

**Implementation:** Disposable CI database.

**Blocking behavior:** Merge blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 09 — Test integrity

**Requirement:** Security failures cannot be interpreted as success.

**Implementation:** Blocking security harness.

**Blocking behavior:** Merge blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 10 — Artifact integrity

**Requirement:** The promoted artifact must be byte-identified by digest.

**Implementation:** Digest verification.

**Blocking behavior:** Release blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 11 — Provenance integrity

**Requirement:** Artifact origin must be attributable to a trusted build.

**Implementation:** Build provenance/attestation.

**Blocking behavior:** Release blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 12 — Promotion integrity

**Requirement:** Production consumes the already verified artifact.

**Implementation:** Immutable promotion.

**Blocking behavior:** Release blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?


## Control 13 — Evidence integrity

**Requirement:** Release claims must be supported by machine-readable evidence.

**Implementation:** Retained reports and metadata.

**Blocking behavior:** Certification blocking.

**Failure semantics:** A missing, skipped, crashed or unverifiable control is not treated as success.

**Review questions:**
- Can the control be bypassed by a pull request?
- Can it fail silently?
- Is its output tied to a source revision?
- Can the same evidence be reproduced?
- Does the control introduce a new trust boundary?
- Is the recovery path documented?

# Control Dossier 001 — workspace membership

## Requirement
Cargo workspace membership is an architectural boundary

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 002 — domain dependency direction

## Requirement
domain crates must remain independent of infrastructure

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 003 — application ports

## Requirement
application ports define infrastructure inversion

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 004 — API composition

## Requirement
HTTP composition must not become business authority

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 005 — worker composition

## Requirement
workers must reuse application semantics

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 006 — authentication boundary

## Requirement
authentication mechanisms remain separated from authorization

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 007 — authorization boundary

## Requirement
authorization is explicit and server enforced

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 008 — tenant context

## Requirement
tenant context is security-sensitive data

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 009 — persistence boundary

## Requirement
database implementation remains behind persistence interfaces

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 010 — event boundary

## Requirement
domain events and integration events remain distinct

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 011 — audit boundary

## Requirement
audit records are not equivalent to logs

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 012 — observability boundary

## Requirement
telemetry must not leak sensitive information

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 013 — configuration boundary

## Requirement
configuration enters through typed validated structures

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 014 — integration boundary

## Requirement
external providers remain untrusted adapters

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 015 — sync boundary

## Requirement
offline synchronization is domain-aware

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 016 — testkit boundary

## Requirement
test infrastructure cannot leak into production

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 017 — toolchain pinning

## Requirement
compiler identity is part of build identity

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 018 — lockfile integrity

## Requirement
dependency resolution must be reproducible

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 019 — dependency review

## Requirement
new dependencies are supply-chain decisions

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 020 — feature governance

## Requirement
features cannot silently disable security

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 021 — unsafe review

## Requirement
unsafe code requires explicit safety reasoning

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 022 — format enforcement

## Requirement
formatting is deterministic repository policy

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 023 — lint enforcement

## Requirement
warnings are actionable engineering defects

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 024 — compiler warnings

## Requirement
warning-free production compilation is the baseline

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 025 — build scripts

## Requirement
build scripts are executable supply-chain code

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 026 — generated code

## Requirement
generated output must have provenance

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 027 — migration execution

## Requirement
schema history must be reproducible

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 028 — migration immutability

## Requirement
applied migrations must not be casually rewritten

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 029 — database isolation

## Requirement
tests must never silently target production

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 030 — database fixtures

## Requirement
fixtures must model security boundaries

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 031 — controlled time

## Requirement
time-sensitive tests must be deterministic

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 032 — random seeds

## Requirement
property-test failures must be reproducible

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 033 — network isolation

## Requirement
ordinary tests should not depend on arbitrary Internet

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 034 — provider fakes

## Requirement
external provider failures must be simulatable

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 035 — secret scanning

## Requirement
credential-like material must be detected

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 036 — credential rotation

## Requirement
detected secrets require revocation rather than cleanup alone

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 037 — workflow permissions

## Requirement
GitHub permissions must be least privilege

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 038 — workflow events

## Requirement
event choice determines trust context

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 039 — shell injection

## Requirement
event data must not become shell source

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 040 — action pinning

## Requirement
third-party workflow code is a dependency

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 041 — cache isolation

## Requirement
cache state must not cross trust boundaries

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 042 — artifact upload

## Requirement
uploaded artifacts must have controlled provenance

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 043 — artifact download

## Requirement
downloaded release artifacts must be verified

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 044 — SBOM generation

## Requirement
software composition must be attributable to an artifact

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 045 — provenance

## Requirement
build origin must be cryptographically/evidentially attributable

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 046 — artifact digest

## Requirement
deployment identity must be immutable

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 047 — promotion

## Requirement
promotion must move the verified artifact without rebuilding

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 048 — deployment verification

## Requirement
deployment must verify the selected artifact

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 049 — rollback

## Requirement
rollback must select a known-good immutable artifact

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 050 — release metadata

## Requirement
release records must answer what is running

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 051 — CI status

## Requirement
missing evidence cannot be interpreted as success

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 052 — retry policy

## Requirement
retries must not hide deterministic failures

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 053 — flaky tests

## Requirement
flakiness requires ownership and expiration

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 054 — resource limits

## Requirement
CI must have bounded resource consumption

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 055 — job isolation

## Requirement
untrusted and trusted jobs require separation

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 056 — environment protection

## Requirement
production credentials belong to protected environments

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 057 — branch protection

## Requirement
required checks must be enforceable

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 058 — CODEOWNERS

## Requirement
sensitive paths require explicit ownership

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 059 — pull request evidence

## Requirement
PRs must disclose architecture and security impact

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 060 — commit discipline

## Requirement
commits should be cohesive and reconstructable

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 061 — repository hygiene

## Requirement
source control must exclude local artifacts

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 062 — binary policy

## Requirement
compiled binaries should not become source artifacts

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 063 — documentation authority

## Requirement
repository documentation must identify authoritative contracts

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 064 — architecture tests

## Requirement
architectural assumptions must be machine-checkable

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 065 — dependency graph inspection

## Requirement
dependency changes must be inspectable

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 066 — license policy

## Requirement
dependency licensing must be controlled

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 067 — vulnerability policy

## Requirement
advisories require severity and reachability assessment

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 068 — cold-cache builds

## Requirement
build correctness must survive cache loss

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 069 — offline development

## Requirement
local work should not require production infrastructure

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 070 — developer bootstrap

## Requirement
safe development must be straightforward

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 071 — canonical commands

## Requirement
local and CI verification must converge

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 072 — script exit codes

## Requirement
automation must propagate failure

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 073 — test reporting

## Requirement
test evidence must identify revision and environment

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 074 — security reporting

## Requirement
security evidence must be retained

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 075 — artifact reporting

## Requirement
artifact metadata must identify exact outputs

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 076 — runner trust

## Requirement
release builders must be separated from untrusted execution

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 077 — registry trust

## Requirement
package sources must be controlled

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 078 — source replacement

## Requirement
Cargo source replacement changes dependency trust

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 079 — private registries

## Requirement
private registry credentials must remain outside source

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 080 — cross compilation

## Requirement
release targets require explicit toolchain configuration

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 081 — target identity

## Requirement
artifact target triple must be recorded

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 082 — release profile

## Requirement
optimization must not remove required controls

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 083 — debug artifacts

## Requirement
debug output must not enter production

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 084 — panic policy

## Requirement
panic behavior must be understood per boundary

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 085 — error mapping

## Requirement
internal errors must not leak through HTTP

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 086 — log redaction

## Requirement
structured logs must omit or redact secrets

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 087 — telemetry cardinality

## Requirement
labels must not create uncontrolled cardinality

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 088 — correlation IDs

## Requirement
request correlation must not become sensitive data leakage

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 089 — test isolation

## Requirement
parallel tests must not share accidental mutable state

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 090 — concurrency tests

## Requirement
race behavior must be deliberately exercised

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 091 — transaction tests

## Requirement
transaction boundaries must be tested against PostgreSQL

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 092 — locking tests

## Requirement
database locking behavior requires real database tests

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 093 — RLS tests

## Requirement
row-level security requires PostgreSQL execution

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 094 — migration rollback strategy

## Requirement
rollback assumptions must be explicit

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 095 — forward fixes

## Requirement
production schema corrections should prefer controlled forward migrations

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 096 — fixture ownership

## Requirement
security fixtures must remain understandable

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 097 — test data classification

## Requirement
test data must not accidentally contain real personal data

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 098 — sandbox credentials

## Requirement
external credentials in CI must be scoped and disposable

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 099 — provider timeouts

## Requirement
timeout paths must be deterministic

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 100 — provider duplicates

## Requirement
duplicate provider responses must be testable

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 101 — provider replay

## Requirement
replay defenses must be testable

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 102 — EIS boundary

## Requirement
tax integration must not become sale authority

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 103 — payment boundary

## Requirement
payment provider state must not become internal financial authority

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 104 — outbox boundary

## Requirement
external side effects require durable intent

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 105 — idempotency

## Requirement
repeated requests must not duplicate financial side effects

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 106 — audit evidence

## Requirement
security-relevant decisions require durable evidence

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 107 — tenant isolation

## Requirement
cross-tenant access must be treated as a critical defect

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 108 — branch scope

## Requirement
resource scope must not be inferred from client claims

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 109 — device identity

## Requirement
device trust must be independently represented

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 110 — offline security

## Requirement
offline capability must not become unlimited authority

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 111 — configuration fingerprint

## Requirement
operationally relevant configuration changes must be observable

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 112 — secret defaults

## Requirement
unsafe configuration defaults must fail closed

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 113 — environment separation

## Requirement
development/staging/production must remain distinct

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 114 — release approvals

## Requirement
production promotion must have controlled authority

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 115 — emergency bypass

## Requirement
break-glass release paths require explicit evidence

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 116 — policy exceptions

## Requirement
exceptions require scope, owner and expiration

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 117 — security gate ownership

## Requirement
each blocking security gate needs an accountable owner

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 118 — CI observability

## Requirement
pipeline failures must be diagnosable without exposing secrets

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 119 — artifact retention

## Requirement
evidence retention must support incident reconstruction

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 120 — release reconstruction

## Requirement
historical builds must be reconstructable from evidence

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 121 — supply-chain incident

## Requirement
dependency compromise requires containment and rebuild

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 122 — workflow incident

## Requirement
workflow compromise requires credential and artifact review

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 123 — artifact incident

## Requirement
artifact mismatch requires immediate promotion stop

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 124 — database incident

## Requirement
migration failure requires controlled diagnosis

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 125 — test incident

## Requirement
false-green testing requires gate review

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 126 — toolchain incident

## Requirement
unexpected compiler drift requires release review

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 127 — dependency incident

## Requirement
lockfile mismatch requires graph inspection

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 128 — repository compromise

## Requirement
unexpected privileged files require investigation

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 129 — access review

## Requirement
repository permissions must align with responsibility

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 130 — automation ownership

## Requirement
automated changes must remain reviewable

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 131 — dependency update cadence

## Requirement
updates must be intentional and observable

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 132 — security exception expiry

## Requirement
temporary exceptions must not become permanent

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 133 — phase exit

## Requirement
completion requires evidence rather than confidence

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 134 — workspace membership

## Requirement
Cargo workspace membership is an architectural boundary

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 135 — domain dependency direction

## Requirement
domain crates must remain independent of infrastructure

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 136 — application ports

## Requirement
application ports define infrastructure inversion

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 137 — API composition

## Requirement
HTTP composition must not become business authority

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 138 — worker composition

## Requirement
workers must reuse application semantics

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 139 — authentication boundary

## Requirement
authentication mechanisms remain separated from authorization

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 140 — authorization boundary

## Requirement
authorization is explicit and server enforced

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 141 — tenant context

## Requirement
tenant context is security-sensitive data

## Threat model
What can go wrong, who can influence the input, and what trust boundary is crossed.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 142 — persistence boundary

## Requirement
database implementation remains behind persistence interfaces

## Implementation
What repository structure, command, policy or automation enforces the requirement.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 143 — event boundary

## Requirement
domain events and integration events remain distinct

## Failure semantics
What exact state must result when the control cannot execute or its evidence is missing.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 144 — audit boundary

## Requirement
audit records are not equivalent to logs

## Testing
What positive and negative tests demonstrate that the control works.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 145 — observability boundary

## Requirement
telemetry must not leak sensitive information

## CI enforcement
How the control becomes a blocking machine check rather than a documentation claim.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 146 — configuration boundary

## Requirement
configuration enters through typed validated structures

## Operational response
How an engineer diagnoses, contains and recovers from a violation.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 147 — integration boundary

## Requirement
external providers remain untrusted adapters

## Evidence
What immutable or attributable evidence should be retained.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 148 — sync boundary

## Requirement
offline synchronization is domain-aware

## Review criteria
What reviewers must inspect before accepting a change.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 149 — testkit boundary

## Requirement
test infrastructure cannot leak into production

## Anti-bypass
How the control can accidentally be weakened and how that weakening is detected.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.


# Control Dossier 150 — toolchain pinning

## Requirement
compiler identity is part of build identity

## Dependencies
Which earlier and later Sitolo contracts rely on this control.

The implementation must preserve the distinction between source authority, build authority, runtime authority and evidence authority. A control is not considered effective merely because a developer can execute a command locally. The repository must make the expected state explicit, CI must be able to observe it, and protected branches or release workflows must be able to block on its violation.

For Sitolo, this is particularly important because later phases introduce tenant isolation, authentication, authorization, financial invariants, inventory concurrency, offline synchronization, payment adapters, MRA EIS integration, auditability and disaster recovery. A weak repository boundary can allow a later feature to bypass a stronger domain contract. Phase 1 therefore treats repository structure and delivery automation as preventive controls rather than administrative conveniences.

## Threat and trust analysis
The relevant threat is not limited to accidental developer error. Consider a malicious pull request, a compromised dependency, a modified workflow action, a poisoned cache, a stale generated file, a mismatched lockfile, a privileged runner, or an operational shortcut taken during an incident. The analysis must identify which actor can influence the input and whether that actor is trusted to influence the resulting artifact.

The preferred trust flow is:

```text
untrusted input
→ validation
→ isolated execution
→ review
→ trusted build
→ verification
→ immutable artifact
→ protected promotion
```

Any design that moves an untrusted actor directly into the trusted-artifact path requires explicit security review.

## Implementation contract
The repository implementation should make the safe state the default. Configuration should be explicit, failures should propagate, and sensitive information should remain outside source and untrusted execution contexts. Where a rule can be represented as a machine-checkable invariant, prefer that over prose.

The implementation must not introduce hidden dependencies on a developer workstation, manually modified database, mutable remote state, undocumented environment variable or production credential. If an input is required, its existence and validity should be detectable.

## Failure semantics
Failure must be classified rather than erased. A command that cannot execute is not equivalent to a command that passed. A skipped security test is not equivalent to a successful security test. A scanner crash is not a clean scan. An artifact that cannot be verified is not a releasable artifact.

The preferred behavior is fail closed:

```text
missing evidence
→ blocked state
→ diagnostic evidence
→ controlled remediation
→ explicit re-verification
```

## Testing requirements
Tests should cover both the intended path and the negative path. At minimum, demonstrate that the control:

1. succeeds under a valid configuration;
2. fails under an invalid configuration;
3. cannot be bypassed by an ordinary contributor;
4. produces useful diagnostics;
5. remains effective from a clean checkout;
6. remains effective without a warm cache;
7. does not expose secrets while producing evidence.

Where the property depends on PostgreSQL semantics, execute it against PostgreSQL rather than a mock.

## CI enforcement
The corresponding CI gate must have a stable identity and must return a machine-readable result. The workflow should not convert failures to success through unconditional shell fallbacks, ignored exit codes or conditional skips.

Required checks must be configured at the protected-branch level, not merely displayed in the workflow UI.

## Operational response
When this control fails in development, first identify the first failing stage, preserve the relevant logs, classify the cause and reproduce it. When it fails in release, stop promotion until the cause is understood. When it may have exposed credentials or altered artifact trust, revoke affected credentials and rebuild from a trusted source.

## Evidence
Evidence should identify:

```text
source revision
workflow run
toolchain
environment
test/security status
artifact identity where applicable
```

Evidence must not itself leak secrets.

## Review criteria
Reviewers should ask whether the implementation:

- strengthens or weakens a trust boundary;
- creates a new dependency;
- changes artifact identity;
- changes privileged workflow permissions;
- changes database assumptions;
- changes security-test coverage;
- changes recovery behavior;
- creates a new undocumented manual step.

## Anti-bypass
Common bypasses include broad allowlists, skipped jobs, mutable action references, ignored exit codes, developer-only configuration, cache reuse across trust classes, and “temporary” exceptions with no expiry. Each should be detectable through policy checks or periodic review.

## Dependencies on later phases
This control becomes more important as Phase 2 introduces configuration/secrets/telemetry, Phase 3 introduces identity/device trust, Phase 4 introduces tenant/IAM, Phase 5 introduces PostgreSQL/RLS, Phase 6 introduces authorization, Phase 7 expands security tests, Phase 11 introduces payments, Phase 12 introduces synchronization, Phase 15 introduces EIS, and Phase 19/20 introduce production hardening and certification.

## Definition of done
The control is complete only when its repository implementation exists, its failure behavior is known, its tests execute, its CI gate blocks where required, and its evidence can be associated with a specific revision.
