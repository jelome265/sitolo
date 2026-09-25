# Configuration

- type: object
- status: verified
- source revision: Sitolo main at e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
- cluster: platform

## Why this shape

Configuration determines runtime behavior and security posture without becoming an alternate source of business truth.

## Shape

Primary implementation: crates/sitolo-config. Canonical behavior: docs/phase2_config_secrets_logging_errors_telemetry_implementation.md.

## Connected to

API, workers, secrets, observability and deployment.

## If you change this

### Hits

Runtime startup, validation, secrets/config boundaries, telemetry and deployment behavior.

### Does not hit

Persisted business records unless configuration changes their explicit contract.

## Surfaces

Application startup, workers, deployment and tests.

## See

docs/phase2_config_secrets_logging_errors_telemetry_implementation.md
