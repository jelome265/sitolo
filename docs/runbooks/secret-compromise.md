# Secret compromise

1. Identify the secret class, reference fingerprint, affected workload, and first known exposure time. Do not copy the secret into the incident record.
2. Freeze deployments that could overwrite evidence; scope the blast radius using audit, provider, and deployment evidence.
3. Rotate or revoke the affected credential through the approved secret authority. Validate the replacement before withdrawing an overlapping old credential.
4. Invalidate dependent sessions, provider credentials, or signing material when the relevant contract requires it.
5. Search logs, traces, response fixtures, and build artifacts for the synthetic or known exposure pattern; preserve evidence under incident controls.
6. Deploy a regression test and record the replacement validation, artifact digest, and config fingerprint.
