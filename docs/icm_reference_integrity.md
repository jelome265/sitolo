# ICM Reference Integrity

This document records documentation-reference defects found while making the Sitolo repository walkable under ICM.

## Defects addressed in this pass

The following filenames were referenced throughout the corpus but absent from the repository tree:

- `security_architecture_design.md`
- `auth_authorization_spec.md`
- `ci_enforcement.md`
- `phase8_product_catalogue_implementation.md`
- `sitolo.md`
- `business_model_design.md` at its former root `docs/` location

All six have now been restored. The business-model entry is deliberately a compatibility pointer to the canonical commercial document rather than a duplicate source.

## Reconstruction rule

The restored documents were derived from existing authoritative material already present in the repository: threat/security contracts, identity/IAM/authorization phases, active CI workflows/scripts, domain/API/database contracts, inventory/sales implementation contracts, and the canonical commercial corpus.

They are not claims that a historical lost document was recovered byte-for-byte. They are current canonical baselines that restore the missing authority paths without inventing requirements outside the existing corpus.

## Remaining integrity work

Reference existence is now materially better, but existence alone is not proof of consistency. A future documentation audit must still check:

1. relative-path correctness from every referring document;
2. one-home-per-fact and duplicate-source drift;
3. precedence conflicts between historical and current phase documents;
4. stale external standards/version references;
5. claims that describe implementation not present in the current source tree;
6. commercial claims that are hypotheses but are phrased as validated facts;
7. generated indexes and routing files against the actual tree.

## Agent rule

When a routed document refers to a missing path, do not synthesize a replacement silently. Record the defect, identify the nearest authoritative source, and either repair the reference or create an explicitly marked canonical compatibility/contract document with evidence.