# Sitolo Offensive Security Skill

**Skill ID:** `offensive-security`  
**Scope:** Authorized adversarial security testing of Sitolo application, API, database, synchronization, infrastructure and supply-chain boundaries  
**Primary owner:** Engineering Security  
**Required during:** security-relevant work where implemented controls, trust boundaries or abuse resistance must be actively challenged  
**Authority:** `docs/security_control_register.md`, `docs/security_implementation_spec.md`, `docs/security_test_harness.md`, `docs/threat_model.md`, applicable phase contract, `agent.md`, and the routed defensive security-engineering skill

---

## 1. Mission

This skill exists to actively attempt to break Sitolo's implemented security controls **inside explicitly authorized scope**, then turn every discovered weakness into reproducible evidence, remediation work, regression tests and defensive verification.

The distinction is intentional:

~~~text
Defensive security engineering
    "Can we prove the control exists and works?"

Offensive security engineering
    "Can we prove an attacker cannot bypass the security boundary?"
~~~

The required lifecycle is:

~~~text
threat model
    ↓
defensive control
    ↓
authorized attack simulation
    ↓
finding
    ↓
reproduction evidence
    ↓
remediation
    ↓
regression test
    ↓
defensive verification
    ↓
CI enforcement
~~~

A green build is not evidence that an attacker cannot bypass a control.

A scanner result is not a substitute for attempting the relevant abuse path.

An exploit attempt outside authorized scope is not permitted.

---

## 2. Authorization and Scope Gate

### Rule 1 — Authorization precedes testing

Offensive security testing MUST begin with an explicit scope decision.

The test record MUST identify:

~~~text
target repository
source revision
target component
test environment
approved interfaces
approved attacker model
approved test window where applicable
data classification
destructive-test prohibition
owner/approver where required
~~~

For repository CI, the default authorized scope is:

~~~text
Sitolo source tree
Sitolo test fixtures
Sitolo-owned disposable infrastructure
Sitolo local integration services
Sitolo CI test environments
synthetic test identities
synthetic test data
~~~

### Rule 2 — No uncontrolled external targets

The agent MUST NOT probe:

~~~text
third-party systems
production customer systems
unapproved cloud resources
real payment providers
real banks
real mobile-money accounts
random public hosts
internet targets unrelated to the authorized test
~~~

An external provider may be exercised only through an explicitly authorized sandbox, mock, fixture, or documented controlled operational environment.

### Rule 3 — Production is not an offensive test target by default

Production penetration or exploitation is prohibited unless an explicit project governance artifact authorizes the exact scope.

Even where explicitly authorized:

~~~text
no destructive payloads
no customer-data extraction
no credential theft
no persistence
no uncontrolled resource exhaustion
no irreversible state changes
no broad network scanning
~~~

The normal CI offensive suite MUST run against disposable/local infrastructure.

### Rule 4 — Scope must remain bounded

The test MUST stop when it leaves the approved boundary.

Examples:

~~~text
authorized app endpoint
    ↓
discovered third-party callback
    ↓
STOP unless separately authorized
~~~

~~~text
authorized test credential
    ↓
real production credential
    ↓
STOP
~~~

### Rule 5 — Fail closed

If authorization, environment identity, target identity, fixture safety or destructive-test controls cannot be established, the offensive run enters a blocked state.

Do not continue based on assumptions.

---

## 3. Operating Rules

### Rule 1 — Attack the implementation, not the prose

A documented security control is only a hypothesis until the executable path is challenged.

### Rule 2 — Use realistic attacker positions

Where applicable, test from:

~~~text
unauthenticated client
authenticated low-privilege user
wrong tenant user
wrong branch user
revoked device
expired session
stale session
malicious operator/support user
compromised client
malformed external callback
malicious dependency/build contributor
~~~

### Rule 3 — Prefer the smallest bypass attempt

Start with the smallest action that could defeat the control.

Escalate only when required to demonstrate impact.

### Rule 4 — Preserve reproducibility

Every successful bypass MUST have:

~~~text
preconditions
fixture identity
attacker identity
target identity
request/command
input mutation
expected control decision
observed result
protected side effect
source revision
reproduction command/test
~~~

### Rule 5 — Do not create new vulnerabilities to test existing ones

Tests MUST NOT weaken:

~~~text
authentication
authorization
tenant isolation
TLS
secret handling
RLS
rate limits
~~~

through global bypasses or permanent test hooks.

Use disposable fixture seams or explicit test-only infrastructure boundaries.

### Rule 6 — Prove the side effect

An HTTP status alone is not enough.

For a denied mutation, prove:

~~~text
request denied
AND protected state unchanged
AND no hidden side effect
AND no financial effect
AND no inventory effect
AND no outbox/effect emission where applicable
AND security/audit state is correct
~~~

---

## 4. Threat-Driven Attack Program

The test plan MUST derive attack cases from:

~~~text
threat model
security controls
trust boundaries
phase contracts
implemented code paths
known vulnerability classes
previous findings
~~~

Do not blindly run every attack against every component.

For each applicable threat, record:

~~~text
asset
attacker
trust boundary
attack precondition
attack action
security property challenged
expected result
observed result
impact
~~~

---

## 5. Required Attack Coverage

### 5.1 IDOR / BOLA

Attempt object substitution across:

~~~text
path identifiers
query identifiers
body references
nested resources
bulk arrays
export identifiers
file references
sync command references
worker payload references
~~~

Example pattern:

~~~text
principal A
authorized resource A
replace resource identifier with resource B
submit unchanged request
~~~

Expected:

~~~text
deny
no target-object disclosure
no target mutation
no hidden side effect
~~~

Do not restrict BOLA testing to routes literally containing `/{id}`.

### 5.2 Tenant isolation

Attempt:

~~~text
cross-tenant read
cross-tenant create
cross-tenant update
cross-tenant delete
cross-tenant search
cross-tenant export
cross-tenant cache access
cross-tenant audit access
cross-tenant worker execution
cross-tenant sync ingestion
~~~

Mutation variants MUST try changing:

~~~text
tenant_id
organization_id
branch_id
resource_id
parent_id
filter
cursor
bulk item scope
~~~

The trusted server-side scope must win over client-supplied tenant selectors.

### 5.3 Branch and resource scope

Within one tenant, attempt unauthorized access between:

~~~text
branch A1 → branch A2
register A1 → register A2
warehouse/location A1 → location A2
restricted resource → unrestricted resource
~~~

Changing a client-supplied `branch_id` MUST NOT broaden authority.

### 5.4 Authentication and session attacks

Attempt:

~~~text
missing credential
malformed credential
expired credential
revoked session
wrong issuer
wrong audience
invalid signature
algorithm mismatch
unknown signing key
token-type confusion
stale session
logout replay
credential-renewal replay
authentication downgrade
~~~

Expected behavior is the documented fail-closed decision.

### 5.5 RBAC / privilege escalation

Starting from the lowest relevant privilege, attempt:

~~~text
horizontal privilege escalation
vertical privilege escalation
hidden admin route access
role mutation
permission mutation
scope expansion
support impersonation
break-glass abuse
self-approval
approval substitution
~~~

Direct API invocation MUST be used where UI controls could otherwise hide the weakness.

### 5.6 Property-level authorization

When some fields are more sensitive than others, attempt mutation of:

~~~text
tax classification
cost basis
approval fields
tenant ownership
financial amount
status/state
security settings
credential references
~~~

Test:

~~~text
allowed property only
protected property only
mixed request
protected property plus forged elevated context
~~~

A caller must not gain authority merely by naming a privileged field.

### 5.7 State-machine abuse

For every high-impact state machine, attempt:

~~~text
authorized + legal transition
authorized + illegal transition
unauthorized + legal transition
unauthorized + illegal transition
repeated transition
reordered transition
stale-state transition
post-close mutation
~~~

Permission alone is not proof that the transition is legal.

### 5.8 Injection

Test applicable input boundaries for:

~~~text
SQL injection
command injection
path traversal
template injection
header injection
log injection
query/operator injection
serialization confusion
parser ambiguity
~~~

Prefer controlled, non-destructive markers and assertions over payloads that intentionally damage data or execute uncontrolled commands.

### 5.9 SSRF

Where outbound URL fetching exists, attempt controlled SSRF against:

~~~text
blocked schemes
loopback targets
link-local targets
private-network targets
metadata-like addresses where the test environment provides a safe fixture
redirect chains
DNS rebinding conditions where the harness supports them
~~~

The test MUST use disposable controlled endpoints and MUST NOT scan real internal networks.

Validate:

~~~text
scheme restrictions
host validation
redirect policy
DNS resolution policy
network egress policy
timeout
response-size limit
~~~

### 5.10 File and path traversal

Where file or path input exists, attempt:

~~~text
path traversal
absolute-path substitution
separator normalization bypass
encoded traversal
symlink-sensitive access where applicable
unexpected archive extraction paths
extension/content mismatch
oversized upload
decompression/resource abuse
~~~

Expected:

~~~text
reject or safely constrain
no unauthorized file read
no unauthorized file write
no path escape
~~~

### 5.11 Unsafe deserialization

For untrusted structured data, attempt:

~~~text
unexpected types
missing fields
duplicate fields
deep nesting
oversized collections
unknown variants
invalid enum/state values
recursive structures where relevant
ambiguous encodings
~~~

The test MUST verify bounded parsing and safe rejection.

### 5.12 API abuse and rate-limit bypass

Attempt:

~~~text
burst requests
parallel requests
credential spraying in test environment
header variation
path normalization
method variation
client-identity variation
repeated failures
pagination abuse
oversized requests
~~~

Test whether controls can be bypassed by changing non-authoritative client metadata.

Resource-exhaustion tests MUST use bounded synthetic load and explicit limits.

### 5.13 Replay and idempotency

For retryable/high-impact commands attempt:

~~~text
exact replay
same command ID + changed payload
same payload + changed command ID
concurrent duplicate
delayed duplicate
reordered delivery
post-timeout replay
~~~

Expected:

~~~text
one authoritative effect
or documented equivalent idempotent result
no double financial effect
no duplicate inventory effect
no forged duplicate audit/outbox effect
~~~

### 5.14 Business-logic abuse

Security testing MUST include domain-level abuse, not only transport vulnerabilities.

Examples:

~~~text
negative/invalid quantity
impossible price manipulation
refund above refundable amount
approval amount mismatch
cross-branch stock movement
sale finalization race
payment reference substitution
billing entitlement bypass
state transition sequencing abuse
~~~

Attack the invariant itself.

### 5.15 Secrets and configuration exposure

Attempt to detect exposure through:

~~~text
source
logs
errors
responses
fixtures
CI artifacts
environment dumps
debug endpoints
configuration output
generated files
crash output
~~~

Do not acquire or display real secrets.

Use synthetic canary values in tests.

### 5.16 Dependency and supply-chain abuse

Test repository controls for:

~~~text
unreviewed dependency source
unexpected dependency origin
advisory bypass
lockfile drift
unsafe build script surface
untrusted artifact
tampered checksum/digest
workflow permission expansion
untrusted workflow input
secret exposure to pull-request code
~~~

Supply-chain testing must remain bounded to the repository and authorized CI environment.

---

## 6. Adversarial Test Construction

Every material security control SHOULD have:

~~~text
positive/control-preserving test
+
negative/bypass attempt
+
side-effect assertion
~~~

For critical controls, prefer:

~~~text
direct negative test
cross-boundary integration test
mutation/removal test where practical
~~~

The smallest test that would fail when the security control is removed is especially valuable.

Examples:

~~~text
remove tenant predicate
    → cross-tenant read test fails

remove authorization check
    → forbidden-operation test fails

remove device revocation
    → revoked-device command test fails

remove idempotency
    → replay test fails

remove callback verification
    → forged-callback test fails
~~~

### Security-control mapping

The offensive suite MUST preserve the repository's existing control IDs:

| Control | Adversarial challenge |
|---|---|
| SC-001 | authentication/session/device bypass, replay, revocation |
| SC-002 | cross-tenant/cross-branch IDOR and scope substitution |
| SC-003 | authorization and privilege-escalation bypass |
| SC-004 | malformed input, injection, SSRF, traversal and parser abuse |
| SC-005 | secret/configuration exposure and cryptographic misuse |
| SC-006 | financial/inventory mutation, replay and invariant abuse |
| SC-007 | offline tampering, revoked-device and checkpoint/replay abuse |
| SC-008 | forged, modified and replayed external callbacks |
| SC-009 | audit/evidence suppression, corruption or attribution abuse |
| SC-010 | rate-limit, concurrency and resource-exhaustion abuse |
| SC-011 | dependency, workflow and release-integrity abuse |
| SC-012 | support/admin escalation, impersonation and break-glass abuse |

Every applicable control must have at least one executable adversarial challenge. If a control has no implemented attack surface yet, record that as not-applicable-to-current-implementation rather than claiming coverage.

---

## 7. Test Authenticity

An offensive test counts only when:

1. it reaches the real relevant implementation path;
2. the attacker context is intentionally constructed;
3. the target resource/control is real within the test environment;
4. the attack actually attempts the bypass;
5. the assertion proves the security property;
6. the protected side effect is checked;
7. the result is tied to a source revision.

These do NOT count as offensive proof by themselves:

~~~text
test name containing "security"
comment describing an attack
mock that bypasses the trust boundary
expected-status-only test
snapshot of a policy document
unused fixture
dead test helper
skipped test
test against an unrelated service
green generic build
~~~

---

## 8. Disposable Attack Environment

CI offensive tests MUST use disposable infrastructure.

Required properties:

~~~text
isolated
synthetic
reproducible
destroyable
network-bounded
credential-free of production secrets
~~~

For database security tests:

~~~text
ephemeral PostgreSQL
isolated test schema
dedicated runtime role
synthetic tenants
automatic teardown
~~~

For HTTP security tests:

~~~text
local/staging test application
synthetic identities
synthetic data
controlled callbacks
bounded request volume
~~~

Production credentials MUST NOT be mounted into the offensive test job.

---

## 9. Evidence Model

Every finding MUST produce a structured record.

Minimum schema:

~~~text
finding_id
severity
status
source_revision
target_component
test_environment
attacker_context
preconditions
attack_input
expected_boundary
observed_behavior
protected_side_effect
reproduction_test
impact
affected_controls
root_cause
remediation
regression_test
verification_revision
residual_risk
~~~

### Severity

Use:

~~~text
Critical
High
Medium
Low
~~~

Severity is an engineering risk classification, not a claim that a tool's label is automatically correct.

Review:

~~~text
exploitability
privilege required
affected data
tenant scope
financial impact
availability impact
reachability
attack complexity
existing compensating controls
~~~

### CI failure policy

By default:

~~~text
Critical finding → CI FAIL
High finding     → CI FAIL unless explicitly risk-accepted by authorized governance
Medium finding   → tracked/remediated according to phase policy; block when the applicable contract requires
Low finding      → tracked according to risk
~~~

A failing security regression test is a CI failure regardless of whether a scanner would assign a lower label, because the repository has demonstrated a broken declared invariant.

No severity downgrade may be performed solely to obtain a green build.

---

## 10. Finding Lifecycle

The lifecycle is:

~~~text
discover
→ reproduce
→ classify
→ root-cause
→ remediate
→ regression test
→ re-run offensive attack
→ re-run defensive verification
→ re-scan applicable tools
→ close
~~~

A finding is NOT closed because:

~~~text
code changed
test was removed
attack was disabled
scanner was skipped
scope was silently broadened
severity was changed without evidence
~~~

### Regression rule

Every fixed vulnerability MUST leave behind a regression test when technically applicable.

The regression test must fail if the vulnerable behavior returns.

---

## 11. Defensive Handoff

Offensive findings MUST hand back into the defensive security-engineering path.

The handoff package contains:

~~~text
attack scenario
affected security control
reproduction
root cause
minimal remediation requirement
required regression test
required scanner/retest
residual risk
~~~

The offensive skill MUST NOT silently replace the defensive control design.

The control remains governed by:

~~~text
security control register
security implementation contract
threat model
phase contract
defensive security-engineering skill
~~~

---

## 12. Scanner and Static Analysis Relationship

Offensive testing complements, rather than replaces:

~~~text
dependency/advisory scans
secret scanning
SAST
DAST
infrastructure/container scanning
artifact/provenance verification
~~~

Use scanners to discover candidate weaknesses and offensive tests to determine whether the relevant boundary can actually be bypassed.

A clean scanner does not prove absence of a business-logic or authorization flaw.

A passing offensive test does not prove absence of vulnerabilities outside its tested surface.

---

## 13. Attack Surface Matrix

The auditor SHOULD maintain:

| Surface | Attacker | Abuse class | Required oracle |
|---|---|---|---|
| Authentication | unauthenticated client | bypass/replay | auth decision + session state |
| Session | stale/revoked client | reuse | rejection + no privilege |
| Authorization | low-privilege user | escalation | decision + side effect |
| Tenant | cross-tenant user | IDOR/BOLA | no disclosure/mutation |
| Branch | wrong-branch user | scope bypass | no disclosure/mutation |
| API | malformed client | injection/abuse | safe rejection + resource bound |
| Database | compromised app path | RLS bypass | real PostgreSQL state |
| Sync | untrusted device | replay/tamper | authoritative state |
| Worker | forged job/context | confused deputy | scope + effect |
| File boundary | untrusted upload | traversal/resource abuse | filesystem boundary |
| External callback | malicious provider | forgery/replay | authenticity + no effect |
| CI/CD | malicious contributor | supply-chain abuse | workflow permission/result |
| Secrets | compromised test actor | leakage | no real secret exposure |
| Release artifact | tampered artifact | provenance bypass | digest/provenance verification |

---

## 14. Offensive Review Procedure

For a security-sensitive change:

### Phase A — Establish scope

~~~text
identify repository revision
identify authorized environment
identify attacker model
identify prohibited actions
~~~

### Phase B — Reconstruct the control

~~~text
asset
→ trust boundary
→ principal
→ scope
→ authorization
→ state/invariant
→ protected side effect
~~~

### Phase C — Attack

Attempt the cheapest realistic bypass first:

~~~text
identifier substitution
scope substitution
missing authorization
stale credential
replay
malformed input
direct API access
concurrent request
~~~

Then escalate only when required.

### Phase D — Verify impact

Determine:

~~~text
disclosure?
mutation?
financial effect?
inventory effect?
privilege increase?
availability impact?
audit/evidence corruption?
~~~

### Phase E — Capture evidence

Record a deterministic reproducer.

### Phase F — Handoff

Create or update:

~~~text
remediation
regression test
defensive verification
scanner/retest requirement
~~~

---

## 15. Safe-Failure Requirements

Offensive tooling MUST be engineered to fail safely.

Required controls include:

~~~text
maximum request count
maximum concurrency
maximum response size
maximum execution time
network allowlist
target validation
synthetic credentials
automatic teardown
temporary artifact cleanup
no persistence outside the test environment
~~~

Do not implement:

~~~text
stealth mechanisms
persistence mechanisms
credential theft
real secret extraction
destructive database wiping
ransomware-like behavior
uncontrolled denial of service
broad internet scanning
~~~

The objective is security verification, not uncontrolled exploitation.

---

## 16. No-Go Conditions

Stop the offensive run when:

- authorization cannot be established;
- target environment cannot be proven disposable/approved;
- production data or credentials are detected;
- the test would contact an uncontrolled external target;
- destructive impact cannot be bounded;
- resource limits are missing for an abuse test;
- evidence cannot be tied to the source revision;
- the attack cannot be reproduced safely;
- a critical/high finding remains unresolved according to release policy;
- a claimed fix has not been attacked again;
- a required security regression test is skipped;
- test infrastructure cleanup fails and could leave sensitive state behind.

Do not turn a no-go condition into a pass by changing the report wording.

---

## 17. Agent Conduct

When using this skill, the agent MUST:

- inspect the real implementation before designing the attack;
- identify the actual trust boundary;
- stay inside explicit authorization scope;
- prefer synthetic fixtures and disposable environments;
- test from realistic attacker privilege;
- assert protected side effects, not just HTTP responses;
- preserve deterministic reproduction;
- record the exact source revision;
- distinguish observed facts from inferred impact;
- preserve existing security controls;
- add regression coverage for confirmed findings;
- re-attack fixes;
- hand findings to the defensive security path;
- report uncertainty explicitly.

The agent MUST NOT:

- scan uncontrolled external systems;
- test production by default;
- use real customer data;
- extract real credentials or secrets;
- create persistence;
- deploy malware;
- introduce permanent bypasses;
- disable controls to make tests pass;
- fabricate exploitation evidence;
- claim a vulnerability was fixed without re-test;
- silently downgrade findings;
- treat absence of a finding as proof of absence of vulnerabilities.

---

## 18. CI Enforcement Contract

The repository's authorized offensive CI path MUST:

~~~text
run on the candidate revision
use disposable/local infrastructure
use synthetic identities/data
execute real adversarial/security regression tests
fail when required security assertions fail
preserve machine-readable test evidence
~~~

The baseline repository offensive suite should execute applicable real adversarial tests, including the existing:

~~~text
PostgreSQL RLS tenant-isolation suite
API tenancy/boundary tests
security secret-provider boundary tests
~~~

As additional attack surfaces become implemented, their offensive suites MUST be added to this gate rather than leaving them as documentation-only requirements.

The CI workflow MUST NOT run offensive tests against production systems.

---

## 19. Evidence Requirements for CI

A successful offensive CI run MUST be attributable to:

~~~text
source revision
workflow run
job
test command
environment class
fixture set
result
~~~

A failed run MUST preserve enough information to reproduce the attack without exposing secrets.

Artifacts MUST NOT contain:

~~~text
production secrets
private keys
bearer tokens
customer data
unnecessary sensitive payloads
~~~

---

## 20. Relationship to the Defensive Security Skill

The two skills are complementary.

~~~text
offensive-security
    ↓
attempt bypass
    ↓
finding
    ↓
defensive-security-engineering
    ↓
control/remediation
    ↓
offensive regression
    ↓
defensive verification
~~~

Neither skill replaces:

~~~text
security_control_register.md
security_implementation_spec.md
security_test_harness.md
threat_model.md
phase contracts
agent.md
~~~

The security-control register remains the authority for control identity and ownership.

---

## 21. Relationship to the ICM Workflow

This skill operates inside the existing ICM stages:

~~~text
01-select
02-research
03-investigate
04-plan
05-implement
06-audit
07-remediate
08-verify
09-deliver
~~~

For offensive work:

~~~text
01-select
  scope + authorization

02-research
  applicable threats/attack classes

03-investigate
  implementation + trust boundaries

04-plan
  attack cases + safety limits

05-implement
  tests/harness/fixtures

06-audit
  findings + evidence

07-remediate
  fixes + regression tests

08-verify
  re-attack + defensive verification + scanners

09-deliver
  evidence + residual risk + approval
~~~

Do not create a competing lifecycle outside ICM.

---

## 22. Definition of Done — Offensive Security Test

A targeted offensive test is done only when:

~~~text
[ ] authorization scope documented
[ ] target environment identified
[ ] attacker context defined
[ ] asset identified
[ ] trust boundary identified
[ ] security control identified
[ ] attack hypothesis defined
[ ] attack executed against real implementation
[ ] expected denial/containment defined
[ ] protected side effect checked
[ ] evidence captured
[ ] severity assessed
[ ] reproduction exists
[ ] cleanup completed
[ ] regression test exists when a finding is confirmed
[ ] defensive verification/retest requirement recorded
~~~

A security-test document without an executed attack does not satisfy this definition.

---

## 23. Final Rule

**Offensive security is controlled adversarial engineering.**

The objective is to break the system **safely, legally, reproducibly and within authorized scope** before a real attacker does.

The required result is not an impressive exploit report.

The required result is:

~~~text
attack
→ evidence
→ remediation
→ regression
→ verification
→ stronger system
~~~

That chain is the measure of offensive-security value in Sitolo.
