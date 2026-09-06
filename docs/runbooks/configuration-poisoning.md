# Configuration poisoning

1. Freeze rollout and identify affected instances by artifact identity and non-secret configuration fingerprint.
2. Compare the effective configuration with the approved deployment record; do not expose secret values during comparison.
3. Revert to a known-good, validated configuration and confirm readiness only after required secrets and dependencies validate.
4. Preserve deployment identity, configuration source provenance, and access evidence for investigation.
5. Correct the trust boundary or deployment policy, then add a regression test for the rejected configuration.
