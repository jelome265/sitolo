# Error storm

1. Check recent artifact and configuration changes, then group failures by bounded error family and operation.
2. Inspect database pool pressure/locks, provider availability, and retry amplification before changing authoritative data.
3. Protect transaction paths by applying only documented timeout, rate-limit, and retry controls.
4. Capture a reproducible safe diagnostic sample; do not log raw requests, provider bodies, or credentials.
5. Resolve the dependency or deployment fault, validate recovery, and add a regression test for the mapping or failure classification.
