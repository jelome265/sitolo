# ICM Reference Integrity

This document records source-integrity findings discovered while making the Sitolo repository walkable under ICM.

## Known missing canonical references

The current documentation corpus refers to files that are not present in the repository tree, including:

- security_architecture_design.md
- auth_authorization_spec.md
- ci_enforcement.md
- phase8_product_catalogue_implementation.md
- sitolo.md

These are not silently synthesized by the ICM workspace.

## Agent rule

When a routed document refers to one of these missing files:

1. record the missing reference as a documentation-integrity finding;
2. use the nearest existing authoritative source only when its authority and scope are explicit;
3. do not treat a substitute document as equivalent;
4. do not mark the affected requirement verified until the authority gap is resolved.

## Why this is separate

The ICM workspace routes context. It does not rewrite the Sitolo documentation corpus or invent missing architecture contracts.
