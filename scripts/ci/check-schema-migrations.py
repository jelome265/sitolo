#!/usr/bin/env python3
"""Validate the ordered, checksum-pinned PostgreSQL migration source."""

from __future__ import annotations

import hashlib
import re
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MIGRATIONS = ROOT / "crates/sitolo-persistence/schema/migrations"
MANIFEST = MIGRATIONS / "manifest.toml"
MIGRATION_NAME = re.compile(r"^(\d{4})_[a-z0-9_]+\.sql$")


def validate_manifest() -> list[str]:
    errors: list[str] = []
    try:
        manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        return [f"cannot read migration manifest: {error}"]

    if manifest.get("format_version") != 1:
        errors.append("manifest format_version must be 1")
    entries = manifest.get("migration")
    if not isinstance(entries, list) or not entries:
        return errors + ["manifest must contain at least one [[migration]] entry"]

    expected_versions = list(range(1, len(entries) + 1))
    versions = [entry.get("version") for entry in entries if isinstance(entry, dict)]
    if versions != expected_versions:
        errors.append("migration versions must be contiguous, ordered, and start at 1")

    listed_files: set[str] = set()
    for entry in entries:
        if not isinstance(entry, dict):
            errors.append("each migration entry must be a table")
            continue
        file_name = entry.get("file")
        digest = entry.get("sha256")
        version = entry.get("version")
        if not isinstance(file_name, str) or not isinstance(digest, str):
            errors.append(f"migration {version!r} needs string file and sha256 fields")
            continue
        match = MIGRATION_NAME.fullmatch(file_name)
        if match is None or int(match.group(1)) != version:
            errors.append(f"migration {version!r} has invalid versioned filename {file_name!r}")
            continue
        if file_name in listed_files:
            errors.append(f"duplicate migration file {file_name!r}")
            continue
        listed_files.add(file_name)
        path = MIGRATIONS / file_name
        try:
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
        except OSError as error:
            errors.append(f"cannot read migration {file_name!r}: {error}")
            continue
        if actual != digest:
            errors.append(f"SHA-256 mismatch for {file_name!r}: expected {digest}, got {actual}")

    actual_files = {path.name for path in MIGRATIONS.glob("*.sql")}
    for missing in sorted(actual_files - listed_files):
        errors.append(f"unlisted SQL migration {missing!r}")
    for absent in sorted(listed_files - actual_files):
        errors.append(f"manifest migration is missing from disk: {absent!r}")
    return errors


def main() -> int:
    errors = validate_manifest()
    if errors:
        for error in errors:
            print(f"SCHEMA MIGRATION ERROR: {error}", file=sys.stderr)
        return 1
    print("SCHEMA MIGRATION INVENTORY PASSED: ordered versions and SHA-256 checksums match")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
