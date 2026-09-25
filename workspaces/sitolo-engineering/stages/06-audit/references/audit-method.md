# Audit Method Reference

For each material requirement trace:

contract → code path → data boundary → test or other evidence

Security-sensitive findings must consider attacker-controlled identifiers, direct API use, replay, privilege escalation, tenant/branch escape, resource exhaustion, and failure recovery.

Do not close a finding because a test exists. Establish that the test proves the required behavior.
