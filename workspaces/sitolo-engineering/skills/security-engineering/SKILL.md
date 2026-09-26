# Sitolo Security Engineering Skill

**Skill ID:** \`security-engineering\`  
**Scope:** Defensive application, API, data, infrastructure, supply-chain and operational security engineering  
**Primary owner:** Engineering Security  
**Required during:** security-relevant Select, Research, Investigate, Plan, Implement, Audit and Verify work  
**Authority:** \`docs/security_control_register.md\`, \`docs/security_implementation_spec.md\`, \`docs/threat_model.md\`, applicable phase contract, and the governing source hierarchy in \`agent.md\`

## 1. Mission

This skill exists to ensure that security is **implemented and demonstrated**, not merely described.

The required chain is:

~~~text
threat / obligation
    ↓
security requirement
    ↓
control design
    ↓
real implementation
    ↓
negative / adversarial test
    ↓
security scanner evidence
    ↓
remediation
    ↓
re-test / re-scan
    ↓
semantic audit
    ↓
verification evidence
~~~

A security claim without implementation evidence and a test that exercises the control is **unproven**.

A passing scanner that does not analyze the affected surface is **not evidence of security for that surface**.

A skipped, crashed, stale or unavailable scanner is **not equivalent to a pass**.

---

## 2. Security applicability

Use this skill whenever a change can affect:

- authentication, sessions, MFA, device identity or credentials;
- authorization, roles, permissions, scopes or tenant isolation;
- APIs, request parsing, serialization or trust boundaries;
- secrets, cryptography, signing or key management;
- databases, migrations, RLS, query construction or persistence integrity;
- payments, financial state, inventory or tax/e-invoicing;
- offline storage, synchronization or replay;
- external integrations, callbacks or webhooks;
- files, uploads, exports or downloads;
- workers, queues, retries or resource exhaustion;
- admin/support/break-glass operations;
- CI/CD, dependencies, build provenance or release artifacts;
- infrastructure, containers, IaC or deployment configuration;
- privacy-sensitive or regulated information.

If applicability is unclear, treat the change as security-relevant until the audit records why it is not.

---

## 3. Mandatory operating rules

### Rule 1 — Server-side authority

Clients, UI state, cached state, local flags and client-supplied role/tenant/scope values are never sufficient security authority.

### Rule 2 — Fail closed

When identity, scope, authorization, integrity, signature verification, configuration, security evidence or required scanner state cannot be established, the operation or release must enter an explicit blocked/error state rather than silently proceeding.

### Rule 3 — No security by naming

Names such as \`admin\`, \`internal\`, \`trusted\`, \`verified\`, \`secure\`, \`system\` or \`validated\` do not establish a security property.

### Rule 4 — Control must be executable

Every material security control must map to at least one executable verification mechanism where technically applicable:

~~~text
unit test
integration test
security regression test
property test
fuzz test
mutation test
static analysis
dependency/advisory scan
secret scan
dynamic/API test
operational test
~~~

The appropriate mechanism depends on the control. Do not satisfy a runtime authorization claim using only static analysis.

### Rule 5 — Negative testing is mandatory

For a security-sensitive control, test both:

~~~text
allowed case
denied / malformed / replayed / cross-scope case
~~~

The negative case must prove that the prohibited action does not cause the protected side effect.

### Rule 6 — Evidence is revision-bound

Security evidence must identify the source revision and, where applicable:

~~~text
workflow run
job
tool version
configuration/version
test fixture
artifact identity
environment
result
~~~

### Rule 7 — Remediation requires re-verification

A vulnerability or security finding is not closed because code changed.

The sequence is:

~~~text
finding
→ root cause
→ fix
→ regression test
→ scanner re-run
→ semantic re-audit
→ evidence
→ close
~~~

---

## 4. Threat-first procedure

### Step A — Define the asset

Identify what is being protected:

~~~text
identity
credential
tenant data
financial fact
inventory fact
tax fact
device authority
payment reference
provider credential
audit evidence
release artifact
availability/resource budget
~~~

### Step B — Define the attacker and trust boundary

Identify:

~~~text
unauthenticated client
authenticated low-privilege user
cross-tenant user
revoked device
malicious operator/support user
compromised dependency
malicious provider callback
malformed file/payload
CI contributor
compromised build/dependency path
external service
~~~

### Step C — Define the security property

Examples:

~~~text
confidentiality
integrity
authenticity
authorization
non-repudiable attribution
tenant isolation
availability / resource exhaustion resistance
replay resistance
secret protection
auditability
recoverability
~~~

### Step D — Select controls

Map the change to the project's security-control IDs:

~~~text
SC-001 authentication/session/device
SC-002 tenant/org/branch isolation
SC-003 authorization/fail-closed decision
SC-004 API/input/boundary security
SC-005 secrets/cryptography lifecycle
SC-006 financial/inventory integrity
SC-007 offline/synchronization security
SC-008 external callbacks/webhooks
SC-009 audit/evidence
SC-010 resource limits / availability security
SC-011 supply chain / release
SC-012 privileged support/admin access
~~~

A control may map to multiple tests. One test may support multiple controls, but the evidence must identify the relationship.

---

## 5. Real security implementation requirements

A security-sensitive implementation is not complete until the actual code path contains the control.

### Authentication

Verify, where applicable:

~~~text
credential verification
session issuance
session validation
session expiry
revocation
MFA assurance
device binding
refresh rotation
replay detection
clock handling
~~~

### Authorization

Trace the complete decision:

~~~text
principal
→ authenticated session/device state
→ membership
→ organization/branch scope
→ permission
→ resource
→ property/state
→ approval/separation-of-duties
→ allow/deny
~~~

Do not accept UI authorization, hidden fields or route naming as proof.

### Tenant isolation

For tenant-owned data, prove:

~~~text
cross-tenant SELECT denied
cross-tenant INSERT denied
cross-tenant UPDATE denied
cross-tenant DELETE denied
cross-tenant object lookup denied
cross-tenant export denied
cross-tenant cached-state access denied
~~~

Use application authorization and database defense in depth where the architecture requires both.

### Input and boundary security

For every externally influenced boundary check:

~~~text
length
size
encoding
type
range
structure
nesting
scheme/host restrictions where applicable
content type
resource budget
~~~

Do not rely on downstream parsers alone when the security contract requires an explicit boundary check.

### Secrets and cryptography

Verify:

~~~text
no secrets in source
no secrets in ordinary logs
no production key material in repository configuration
approved cryptographic libraries only
key purpose separation
rotation/revocation path
safe failure
no bespoke cryptography
~~~

### Financial/inventory integrity

For high-impact mutations prove:

~~~text
authorization
→ invariant validation
→ transaction
→ idempotency
→ durable fact
→ audit
→ required outbox/event
~~~

Security testing must include replay, concurrent requests and unauthorized mutation attempts where applicable.

### External integrations

Treat external systems as untrusted boundaries:

~~~text
authenticate provider evidence
validate response
bound timeouts/retries
preserve unknown state
deduplicate callbacks
scope provider references
protect credentials
~~~

### Offline/synchronization

Security must account for:

~~~text
revoked device
stale authorization
replay
command tampering
cross-tenant command
checkpoint abuse
schema/version mismatch
resource exhaustion
~~~

### Admin/support

Require:

~~~text
explicit privilege
explicit scope
purpose
approval where required
short-lived access
audit
revocation
post-use review
~~~

---

## 6. Mandatory vulnerability-scanning model

Security scanning is layered. No single scanner proves the absence of vulnerabilities.

### Layer A — Dependency / SCA

For Rust:

~~~text
cargo audit
cargo deny check advisories
~~~

Use \`cargo audit\` against the authoritative \`Cargo.lock\`. RustSec documents \`cargo-audit\` as a dependency vulnerability audit mechanism. \`cargo-deny\` provides advisory, license, source and dependency-policy checks. citeturn179608search2turn179608search13

If another supported language/runtime is introduced, add the ecosystem-appropriate authoritative SCA scanner before treating that surface as covered.

### Layer B — Secret scanning

Use the repository's configured secret scanner and ensure the scan covers:

~~~text
tracked files
repository history where policy requires
configuration
CI definitions
test fixtures
generated artifacts
~~~

A scanner failure, unavailable scan, authentication failure or ambiguous result is not success.

### Layer C — SAST / dataflow

Use CodeQL or the project's approved equivalent for every supported production language.

For Rust, CodeQL's query set covers security-relevant classes including SQL construction from user-controlled data, SSRF, cleartext sensitive logging/storage, disabled TLS checks, path/command injection, uncontrolled allocation and weak cryptography. citeturn179608search0turn179608search4

Use the strongest approved query suite appropriate to the repository's risk profile. Do not downgrade or suppress findings merely to make CI green.

### Layer D — Dynamic/API security

When a deployed HTTP/API surface exists, test the live or staging boundary for:

~~~text
authentication bypass
BOLA / IDOR
authorization bypass
input validation failures
rate-limit bypass
method confusion
error leakage
SSRF where applicable
file/upload abuse where applicable
resource exhaustion
~~~

Use an approved DAST/API security tool where the environment permits it.

### Layer E — Infrastructure / container / IaC

When the repository contains containers, Kubernetes, Terraform, cloud/IaC or comparable deployment artifacts, perform the approved static/security scan for those artifacts.

Do not mark infrastructure security as covered when only application code was scanned.

### Layer F — Supply-chain / artifact integrity

Verify:

~~~text
dependency lock state
source revision
artifact digest
SBOM
provenance/attestation
signed artifact where required
artifact verification
build environment identity
~~~

A source scan does not prove the deployed artifact is the scanned artifact.

---

## 7. Scanner applicability matrix

| Surface | Required baseline | Failure semantics |
|---|---|---|
| Rust dependencies | \`cargo audit\` + \`cargo deny check advisories\` | blocking |
| Secrets | configured secret scan | blocking |
| Rust source | CodeQL / approved SAST | blocking for configured security findings |
| HTTP/API runtime | DAST/API security when environment exists | blocking when the phase/control requires it |
| Container/IaC | applicable approved scan | blocking when surface exists |
| Release artifact | provenance/SBOM/artifact verification | blocking |
| Domain security controls | executable negative/security tests | blocking |
| Fuzz-sensitive parser | fuzz/property coverage where required | blocking when designated by threat model |
| Security evidence | revision-attributable evidence | blocking |

A scanner is **not applicable** only when the affected surface genuinely does not exist or the security contract explicitly excludes it. The reason must be recorded.

---

## 8. Vulnerability triage

Every discovered vulnerability or security finding is classified as:

~~~text
Confirmed vulnerability
Potential vulnerability
False positive
Accepted risk
Unresolved
Not applicable
~~~

### Severity handling

Project release policy:

~~~text
Critical → release-blocking
High     → release-blocking unless formally risk-accepted by authorized human governance
Medium   → remediation or explicit tracked disposition
Low      → tracked according to risk and exposure
~~~

Do not translate tool severity mechanically into business risk. Review:

~~~text
exploitability
reachability
privilege required
affected data
tenant impact
financial impact
availability impact
exposure
compensating controls
~~~

A “not exploitable” conclusion requires evidence.

---

## 9. Security regression test design

For each material control, create the smallest test that can fail when the control is removed.

Examples:

~~~text
remove tenant predicate
→ cross-tenant test must fail

remove authorization check
→ forbidden operation test must fail

remove idempotency key
→ duplicate command test must fail

disable callback verification
→ forged callback test must fail

ignore device revocation
→ revoked-device test must fail

remove resource limit
→ exhaustion test must fail

remove secret redaction
→ secret-leak test must fail
~~~

Mutation testing is strongly preferred for high-value security controls because it tests whether the regression suite actually detects removal of the control.

---

## 10. Test authenticity rules

A security test counts only when:

1. it executes against the relevant implementation path;
2. its setup establishes the intended trust boundary;
3. it exercises the protected operation;
4. the asserted outcome is security-relevant;
5. failure of the control causes test failure;
6. the result is attributable to a revision.

These do **not** count as security proof by themselves:

~~~text
test name containing "security"
comment saying "secure"
placeholder assertion
mock that bypasses the trust boundary
test that never executes the protected branch
snapshot of expected policy text
green generic build
skipped security test
scanner run against an unrelated artifact
~~~

---

## 11. Secure implementation sequence

Security-sensitive changes MUST follow:

~~~text
01 threat / asset / boundary
02 security requirement
03 control selection
04 implementation plan
05 secure implementation
06 negative + adversarial tests
07 SCA / secret / SAST / DAST scans as applicable
08 failure / abuse / resource tests
09 semantic security audit
10 remediation
11 re-test + re-scan
12 verification evidence
~~~

This sequence is compatible with the repository's ICM 01–09 workflow; it is a security engineering discipline inside the relevant ICM stages, not a replacement workflow.

---

## 12. Evidence package

Each security-sensitive run should produce:

~~~text
security applicability decision
threat/asset summary
control IDs
implementation references
test IDs
scanner names/versions
scan configuration
scan result
finding disposition
remediation commit
regression result
re-scan result
semantic audit
residual risk
human approval where required
~~~

Evidence must never contain live secrets, private keys, bearer tokens or unnecessary sensitive data.

---

## 13. No-go conditions

Stop the implementation/release when any applicable condition is true:

- security authority is ambiguous;
- tenant scope cannot be proven;
- authorization is client-controlled;
- critical mutation lacks negative tests;
- scanner required by the applicable surface did not run;
- scanner crashed or produced an untrusted result;
- dependency vulnerability is unresolved at a blocking severity;
- secret exposure is confirmed or cannot be ruled out;
- security regression tests are skipped;
- a high-impact security control has no executable evidence;
- a material finding was “fixed” without re-test/re-scan;
- evidence cannot be tied to the actual candidate revision.

Do not convert any of these into a pass by changing wording.

---

## 14. Security audit procedure

The auditor reconstructs:

~~~text
asset
→ attacker
→ trust boundary
→ required property
→ security control
→ implementation path
→ negative test
→ scanner evidence
→ failure behavior
→ recovery
→ residual risk
~~~

Then challenge:

~~~text
authorization
tenant isolation
input handling
cryptography/secrets
replay
race conditions
logging/evidence
resource exhaustion
external boundaries
supply chain
deployment artifact
~~~

Output:

~~~text
PASS
PARTIAL
FAIL
N/A
~~~

with evidence for every non-N/A conclusion.

---

## 15. Standards/reference posture

This skill uses external standards as engineering references, not as certification claims.

The primary reference set should be maintained through \`docs/external_standards_verification_register.md\`.

Relevant current references include:

- NIST CSF 2.0 for cybersecurity risk outcomes. citeturn138528search0turn138528search2
- NIST SP 800-218 SSDF 1.1 for secure software development practices; NIST also lists SP 800-218 Rev. 1 / SSDF 1.2 as a **draft**, so the draft must not be represented as a final standard without explicit verification. citeturn764876search0turn764876search9
- OWASP ASVS 5.0.0 as the current stable application-security verification standard. citeturn138528search5
- OWASP API Security Top 10 (2023) for API-risk awareness. citeturn138528search10turn138528search12
- CISA-led Secure by Design guidance for threat modeling and defense-in-depth principles. citeturn764876search48

External guidance informs the control design; project-specific requirements remain governed by Sitolo's source hierarchy.

---

## 16. Agent conduct

When using this skill, the agent MUST:

- inspect the actual code path before declaring a control implemented;
- inspect existing tests before adding replacements;
- preserve the existing security-control IDs;
- prefer narrowly-scoped, testable controls over broad abstractions;
- avoid weakening production security to make a test pass;
- avoid suppressing scanner findings without documented disposition;
- distinguish exploitable findings from theoretical concerns using evidence;
- preserve tenant, financial and audit invariants;
- report uncertainty explicitly;
- stop when a required security dependency or evidence artifact is missing.

The agent MUST NOT:

- claim a clean scan without executing the scan;
- treat a green CI job as proof of an untested security property;
- disable security tests to unblock delivery;
- add global bypasses for development convenience;
- create a “temporary” admin or tenant bypass;
- store real secrets in fixtures;
- fabricate scanner output;
- silently downgrade findings;
- declare a vulnerability fixed without regression and re-scan evidence.

**Security is a defensive engineering property of the system, not a documentation adjective.**
