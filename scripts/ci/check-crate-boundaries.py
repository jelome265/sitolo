#!/usr/bin/env python3
"""Fail-closed checker for declared Sitolo crate-layer boundaries."""
from __future__ import annotations

import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / "architecture" / "crate-boundaries.toml"


def main() -> int:
    try:
        policy = tomllib.loads(POLICY.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"ARCHITECTURE VIOLATION: cannot load crate boundary contract: {error}", file=sys.stderr)
        return 2

    if policy.get("version") != 1:
        print("ARCHITECTURE VIOLATION: unsupported crate boundary contract version", file=sys.stderr)
        return 2

    violations: list[str] = []
    for rule in policy.get("rules", []):
        crate = rule.get("crate")
        if not isinstance(crate, str) or not crate:
            violations.append("contract contains a rule without a crate name")
            continue

        crate_dir = ROOT / "crates" / crate
        manifest_path = crate_dir / "Cargo.toml"
        try:
            manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            violations.append(f"{crate}: cannot load Cargo.toml: {error}")
            continue

        production_dependencies = manifest.get("dependencies", {})
        for dependency in rule.get("forbidden_dependencies", []):
            if dependency in production_dependencies:
                violations.append(
                    f"{crate}/Cargo.toml: '{dependency}' is a forbidden production dependency"
                )

        source_tokens = rule.get("forbidden_source_tokens", [])
        if source_tokens:
            source_root = crate_dir / "src"
            if not source_root.is_dir():
                violations.append(f"{crate}: source directory is missing")
                continue
            reported: set[tuple[str, str]] = set()
            for source in sorted(source_root.rglob("*.rs")):
                try:
                    lines = source.read_text(encoding="utf-8").splitlines()
                except OSError as error:
                    violations.append(f"{crate}: cannot read {source.relative_to(ROOT)}: {error}")
                    continue
                for line_number, line in enumerate(lines, 1):
                    for token in source_tokens:
                        if token in line:
                            key = (str(source.relative_to(ROOT)), token)
                            if key not in reported:
                                reported.add(key)
                                violations.append(
                                    f"{key[0]}:{line_number}: forbidden infrastructure reference '{token}'"
                                )

    if violations:
        print("CRATE BOUNDARY CHECK FAILED — these are architecture violations, not lint warnings:")
        for violation in violations:
            print(f"  - {violation}")
        print("\nMove database/driver implementations behind application-owned ports; do not suppress this gate.")
        return 1

    print("Crate boundary check PASSED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
