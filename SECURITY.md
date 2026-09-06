# Sitolo — Security Policy

## Reporting a vulnerability

Please report security vulnerabilities privately. Do not open a public issue for a security problem.

To report:

1. Describe the vulnerability, including the affected area and a minimal reproduction where possible.
2. State the impact (for example, tenant isolation bypass, authorization bypass, credential exposure, financial integrity, denial of service).
3. Provide your contact details.

Vulnerabilities will be triaged against the Sitolo security model:

- Cross-tenant access: **Critical**
- Authentication / authorization bypass: **High**
- Credential or secret exposure: **High**
- Financial or inventory integrity violation: **High**
- Payment or tax (MRA EIS) integrity violation: **High**
- DoS / resource exhaustion: **Medium**
- Availability / recoverability degradation: **Medium**

A confirmed **Critical** or **High (authorization/payment/CI)** issue is release-blocking in ordinary circumstances. Do not assume silent mitigation; the issue must be resolved, risk-accepted under a controlled process, or the release held.

## Secrets

Never commit secrets: private keys, API keys, provider credentials, database passwords, tokens, JWT signing secrets, or cloud credentials. If a real credential is exposed in history or artifacts, treat it as compromised: revoke, rotate, determine blast radius, remove source exposure, rebuild affected artifacts, and verify the old credential fails.

## Coordinated disclosure

We will coordinate disclosure with affected parties and prefer a fix-then-disclose timeline appropriate to the severity.