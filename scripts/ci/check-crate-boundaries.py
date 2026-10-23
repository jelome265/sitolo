#!/usr/bin/env python3
"""Fail-closed checker for the declared Sitolo crate-layer contract.

The contract in ``architecture/crate-boundaries.toml`` is exhaustive: every
workspace package must be classified exactly once, and every declared
production dependency edge must respect the layer policy.

Dependency edges are read from ``cargo metadata`` rather than by parsing
manifests, so workspace inheritance, renamed dependencies and path crates are
resolved by Cargo itself. The source-token scan is a supplementary guard for
infrastructure types reached through a re-export, which a manifest check cannot
observe.

Exit codes:
    0  contract satisfied
    1  architecture violation(s)
    2  the contract or the tooling could not be evaluated (fail closed)
"""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / "architecture" / "crate-boundaries.toml"


def fail_closed(message: str) -> int:
    print(f"ARCHITECTURE CONTRACT ERROR: {message}", file=sys.stderr)
    return 2


def load_policy() -> dict:
    return tomllib.loads(POLICY.read_text(encoding="utf-8"))


def workspace_packages() -> list[dict]:
    """Return the workspace packages as reported by Cargo."""
    cargo = shutil.which("cargo")
    if cargo is None:
        raise RuntimeError("cargo is not on PATH")
    completed = subprocess.run(
        [cargo, "metadata", "--no-deps", "--format-version", "1", "--locked"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            "cargo metadata failed: " + (completed.stderr.strip() or "no diagnostics")
        )
    metadata = json.loads(completed.stdout)
    workspace_members = set(metadata.get("workspace_members", []))
    return [
        package
        for package in metadata.get("packages", [])
        if package.get("id") in workspace_members
    ]


def production_dependencies(package: dict) -> set[str]:
    """Direct normal dependencies only; dev/build edges are out of scope."""
    return {
        dependency["name"]
        for dependency in package.get("dependencies", [])
        if dependency.get("kind") is None
    }


def source_files(package: dict) -> list[Path]:
    source_root = Path(package["manifest_path"]).parent / "src"
    if not source_root.is_dir():
        return []
    return sorted(source_root.rglob("*.rs"))


def check() -> int:
    try:
        policy = load_policy()
    except (OSError, tomllib.TOMLDecodeError) as error:
        return fail_closed(f"cannot load {POLICY.relative_to(ROOT)}: {error}")

    if policy.get("version") != 1:
        return fail_closed("unsupported crate boundary contract version")

    rules = policy.get("rules", [])
    if not isinstance(rules, list):
        return fail_closed("`rules` must be an array of tables")

    try:
        packages = workspace_packages()
    except (RuntimeError, json.JSONDecodeError) as error:
        return fail_closed(str(error))

    violations: list[str] = []

    rules_by_crate: dict[str, dict] = {}
    for rule in rules:
        crate = rule.get("crate")
        if not isinstance(crate, str) or not crate:
            violations.append("contract contains a rule without a crate name")
            continue
        if crate in rules_by_crate:
            violations.append(f"contract declares '{crate}' more than once")
            continue
        if not isinstance(rule.get("role"), str) or not rule["role"]:
            violations.append(f"{crate}: rule is missing a `role`")
        rules_by_crate[crate] = rule

    package_names = {package["name"] for package in packages}

    for crate in sorted(set(rules_by_crate) - package_names):
        violations.append(f"contract classifies '{crate}', which is not a workspace package")

    for package in sorted(packages, key=lambda item: item["name"]):
        crate = package["name"]
        rule = rules_by_crate.get(crate)
        if rule is None:
            violations.append(
                f"{crate}: no architecture rule; classify the crate in "
                "architecture/crate-boundaries.toml before adding it"
            )
            continue

        dependencies = production_dependencies(package)
        for dependency in rule.get("forbidden_dependencies", []):
            if dependency in dependencies:
                violations.append(
                    f"{crate}: '{dependency}' is a forbidden production dependency"
                )

        tokens = rule.get("forbidden_source_tokens", [])
        if not tokens:
            continue
        files = source_files(package)
        if not files:
            violations.append(f"{crate}: source directory is missing")
            continue
        for source in files:
            try:
                lines = source.read_text(encoding="utf-8").splitlines()
            except OSError as error:
                violations.append(
                    f"{crate}: cannot read {source.relative_to(ROOT)}: {error}"
                )
                continue
            for line_number, line in enumerate(lines, 1):
                if line.lstrip().startswith("//"):
                    continue
                for token in tokens:
                    if token in line:
                        violations.append(
                            f"{source.relative_to(ROOT)}:{line_number}: "
                            f"forbidden infrastructure reference '{token}'"
                        )

    if violations:
        print("CRATE BOUNDARY CHECK FAILED - these are architecture violations, not lint warnings:")
        for violation in violations:
            print(f"  - {violation}")
        print(
            "\nMove database/driver implementations behind application-owned ports; "
            "do not suppress this gate."
        )
        return 1

    print(f"Crate boundary check PASSED ({len(packages)} workspace packages classified)")
    return 0


if __name__ == "__main__":
    raise SystemExit(check())
