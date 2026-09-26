# System Map Schema

The map is a navigational record library. The subject tree remains authoritative.

## Object cards

Every object card uses YAML frontmatter:

```yaml
---
type: object
status: stub | verified | stale
universe: live | leftover | ghost
cluster: <cluster>
source_revision: <commit or dated revision>
source: <repository path>
source_citation: <path:line when source is code>
---
```

Required body sections:

- one-sentence identity;
- Why this shape;
- Shape;
- Connected to;
- If you change this, with Hits / Does not hit;
- Surfaces;
- See.

`status: verified` requires a current source revision and source citation. `stub` is the safe default when executable source evidence is insufficient. `universe: ghost` means the noun/process is named or filed but not wired into the current executable path.

## Process cards

Every process card uses YAML frontmatter:

```yaml
---
type: process
status: stub | verified | stale
universe: live | leftover | ghost
source_revision: <commit or dated revision>
source: <repository path or governing document>
---
```

Process cards describe only real movements as Input → Movement → Output and must include source citations before promotion to verified.

## Generated index

`map/objects/_index.md` is generated from object-card frontmatter by `scripts/ci/generate-system-map-index`. Do not hand-edit it.

## Authority

Map cards point to source. They never become a second architecture specification.