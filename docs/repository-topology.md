# Sitolo Repository Topology — Accepted Omissions Record (Audit F-019)

The Phase 1 specification sketches a canonical tree that includes
`apps/mobile`, `apps/desktop`, `migrations`, `config`, `tests`, `fuzz`, and
`infra`. Phase 1 also states that empty structure must not be created merely
to satisfy a diagram. This record makes every current omission intentional,
owned, and phased, so a missing directory is never ambiguous.

| Canonical component | Present | Disposition | Owner | Phase |
|---|---|---|---|---|
| `apps/mobile` (Flutter) | No | Accepted future structure; no executable client responsibility yet | Mobile client phase | Later phase |
| `apps/desktop` (Tauri) | No | Accepted future structure; no executable client responsibility yet | Desktop client phase | Later phase |
| `migrations/` | No | Accepted future structure; schema/migrations belong to Phase 5 | Phase 5 | Phase 5 |
| `config/` (deployment files) | No | Accepted future structure; deployment manifests arrive with hosting | Operations | Later phase |
| `tests/` (top-level) | No | Accepted future structure; crate-level and app-level suites exist (`src/`, `tests/` per package) | Platform | As needed |
| `fuzz/` | No | Accepted future structure; fuzz targets arrive with the security-test expansion | Phase 7 | Phase 7 |
| `infra/` | No | Accepted future structure; provider-specific infrastructure arrives with deployment | Operations | Later phase |

Present and owned today: `apps/api`, `apps/worker`, all `crates/sitolo-*`
packages, `docs/`, `scripts/ci/`, `.github/`, workspace manifests, lockfile,
and policy files (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`,
`deny.toml`, `.gitattributes`, `.gitignore`).

Rule: create a listed directory only when it gains an executable
responsibility in its owning phase. Do not create placeholder modules for
aesthetics.
