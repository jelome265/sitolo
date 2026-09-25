# Sitolo — External Standards Verification Register

**Status:** Current verification register
**Verification date:** 2026-09-25
**Purpose:** Keep versioned external standards distinct from internal contracts and dated evidence.

## Authority rule

Applicable law, regulator requirements, and current external provider contracts outrank this register and every internal Sitolo document.

This register records what was verified on the date above. It is not permission to treat a draft standard as a final mandatory standard.

## Verified baseline

| Source | Verified state on 2026-09-25 | Sitolo use |
|---|---|---|
| Rust | 1.98.1 is the current stable release; released 2026-09-03 | Repository toolchain baseline |
| PostgreSQL | 18.6 is the current PostgreSQL 18 maintenance release; released 2026-08-13 | Production database target line |
| SLSA | Version 1.2 is the approved specification; v1.1 is retired | Supply-chain provenance reference |
| OpenTelemetry Semantic Conventions | 1.44.0 | Telemetry naming/reference baseline |
| OWASP ASVS | 5.0.0 | Application-security verification baseline |
| NIST SP 800-218 | Version 1.1 remains the final publication; Rev. 1 / SSDF 1.2 is an initial public draft, not a final publication | Secure-development reference |

## Interpretation

### Rust

Sitolo pins the exact Rust patch release in repository configuration. Documentation should not describe an older Rust patch as the current stable release.

### PostgreSQL

Sitolo targets PostgreSQL 18.x. Exact minor versions belong in deployment/toolchain evidence and must be refreshed when a release changes.

### SLSA

Use SLSA 1.2 for current normative reference. Older 1.1 material is historical/background unless a compatibility requirement explicitly requires it.

### OpenTelemetry

Semantic conventions are versioned and individual convention groups can have different maturity states. Sitolo should pin the release used for an implementation and document any stability caveat relevant to the signal being emitted.

### OWASP ASVS

Reference ASVS requirements with the version-qualified identifier when evidence needs to remain stable across future ASVS changes.

### NIST SSDF

Do not write “SSDF 1.2 current standard” without qualification. NIST currently lists SP 800-218 Rev. 1 / SSDF 1.2 as a draft, while SP 800-218 Version 1.1 remains the final publication. Use the draft as a forward-looking reference only, unless and until NIST publishes a final revision.

## Reverification triggers

Refresh this register before:

- a toolchain major/minor/patch upgrade;
- changing the PostgreSQL major/minor support baseline;
- changing supply-chain provenance requirements;
- changing telemetry semantic-convention versions;
- changing security-verification requirements;
- relying on a newer NIST revision in a release or compliance claim;
- an external regulator/provider changing a contract or requirement.

## Primary sources

- Rust: https://blog.rust-lang.org/releases/latest/
- PostgreSQL: https://www.postgresql.org/docs/release/
- SLSA: https://slsa.dev/spec/v1.2/
- OpenTelemetry Semantic Conventions: https://opentelemetry.io/docs/specs/semconv/
- OWASP ASVS: https://owasp.org/www-project-application-security-verification-standard/
- NIST SSDF publications: https://csrc.nist.gov/projects/ssdf/publications
