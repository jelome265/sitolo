#!/usr/bin/env python3
"""Install and verify the repository-pinned Rust policy tools."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from pathlib import Path


COMMANDS = {
    "cargo-deny": ("deny", "cargo-deny"),
    "cargo-audit": ("audit", "cargo-audit"),
}
VERSION = re.compile(r"(?<![0-9])([0-9]+\.[0-9]+\.[0-9]+)(?![0-9])")


def detected_version(package: str) -> str | None:
    cargo_command, _ = COMMANDS[package]
    result = subprocess.run(
        ["cargo", cargo_command, "--version"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    match = VERSION.search(result.stdout + result.stderr)
    return match.group(1) if match else None


def install(package: str, version: str) -> None:
    current = detected_version(package)
    if current == version:
        print(f"{package} {version} already installed")
        return

    command = ["cargo", "install", package, "--version", version, "--locked"]
    if current is not None:
        command.append("--force")
    subprocess.run(command, check=True)
    installed = detected_version(package)
    if installed != version:
        raise RuntimeError(f"{package}: expected {version}, observed {installed or 'unknown'}")
    print(f"{package} {version} installed and verified")


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    versions_path = root / "scripts" / "ci" / "policy-tool-versions.toml"
    versions = tomllib.loads(versions_path.read_text(encoding="utf-8"))["tools"]

    parser = argparse.ArgumentParser()
    parser.add_argument("--tool", choices=sorted(COMMANDS), action="append")
    args = parser.parse_args()
    selected = args.tool or sorted(COMMANDS)
    for package in selected:
        version = versions.get(package)
        if not isinstance(version, str):
            raise RuntimeError(f"{versions_path}: missing pinned version for {package}")
        install(package, version)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, subprocess.CalledProcessError, RuntimeError, tomllib.TOMLDecodeError) as error:
        print(f"policy tool installation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
