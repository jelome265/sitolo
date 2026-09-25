---
type: object
status: verified
cluster: platform
source_revision: main@e8f46c0d61f9b50efe98fb8ea6a281d6a4d3f2dc
source: crates/sitolo-observability/src/lib.rs
source_citation: crates/sitolo-observability/src/lib.rs:25
---
# Observability


## Why this shape

Operational telemetry provides bounded evidence about system behavior without becoming business or security authority.

## Shape

Primary implementation: `crates/sitolo-observability`. Canonical contract: `docs/observability_spec.md`.

## Connected to

API, workers, persistence, security controls, deployments and verification.

## If you change this

### Hits

Telemetry schema, bounded dimensions, diagnostics and operational evidence.

### Does not hit

Business state or authorization decisions.

## Surfaces

Runtime logs/metrics/traces, CI evidence and operations.

## See

crates/sitolo-observability/src/lib.rs

