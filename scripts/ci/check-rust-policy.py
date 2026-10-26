#!/usr/bin/env python3
"""Fail-closed checks for workspace lint inheritance and CI lint bypasses."""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path


ALLOW_ATTRIBUTE = re.compile(r"#\s*!?\s*\[\s*allow\s*\(")
CFG_ATTR_ALLOW = re.compile(r"#\s*!?\s*\[\s*cfg_attr\s*\([^]]*\ballow\s*\(", re.DOTALL)
EXPECT_ATTRIBUTE = re.compile(r"#\s*!?\s*\[\s*expect\s*\(")
UNSAFE_ITEM = re.compile(r"\bunsafe\s*(?:\{|fn\b|impl\b|extern\b)")
FORBIDDEN_FLAGS = re.compile(
    r"--cap-lints(?:\s+|=)allow\b|(?<!\S)-A(?:\s+|=)(?:warnings|clippy::all|rust::warnings|rust::unsafe_code)\b",
    re.IGNORECASE,
)
POLICY_TOOLS = ("cargo-deny", "cargo-audit")


def _attribute_end(source: str, start: int) -> int:
    opening = source.find("[", start)
    if opening < 0:
        return len(source)
    depth = 0
    quoted = False
    escaped = False
    for index in range(opening, len(source)):
        char = source[index]
        if quoted:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
            continue
        if char == '"':
            quoted = True
        elif char == "[":
            depth += 1
        elif char == "]":
            depth -= 1
            if depth == 0:
                return index + 1
    return len(source)


def _lint_level(value: object) -> str | None:
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        level = value.get("level")
        return level if isinstance(level, str) else None
    return None


def _canonical_expect_attribute(attribute: str) -> str:
    """Normalize layout so exception inventory entries do not depend on wrapping."""
    return " ".join(attribute.split())


def _expect_inventory_errors(
    observed: set[tuple[str, str]], inventory_data: object, inventory_path: str
) -> list[str]:
    errors: list[str] = []
    inventoried: set[tuple[str, str]] = set()
    if not isinstance(inventory_data, list):
        return [f"{inventory_path}: inventory must be a JSON array"]

    for index, entry in enumerate(inventory_data):
        if not isinstance(entry, dict) or set(entry) != {"path", "attribute"}:
            errors.append(f"{inventory_path}: entry {index} must contain only path and attribute")
            continue
        path = entry["path"]
        attribute = entry["attribute"]
        if not isinstance(path, str) or not isinstance(attribute, str):
            errors.append(f"{inventory_path}: entry {index} path and attribute must be strings")
            continue
        inventoried.add((path, _canonical_expect_attribute(attribute)))

    for path, attribute in sorted(observed - inventoried):
        errors.append(f"{path}: #[expect] is missing a path-specific entry in {inventory_path}: {attribute}")
    for path, attribute in sorted(inventoried - observed):
        errors.append(f"{inventory_path}: stale #[expect] inventory entry for {path}: {attribute}")
    return errors


def check_repository(root: Path) -> list[str]:
    errors: list[str] = []
    observed_expectations: set[tuple[str, str]] = set()
    root_manifest_path = root / "Cargo.toml"
    root_manifest = tomllib.loads(root_manifest_path.read_text(encoding="utf-8"))
    workspace = root_manifest.get("workspace", {})
    workspace_lints = workspace.get("lints", {})
    rust_lints = workspace_lints.get("rust", {})
    clippy_lints = workspace_lints.get("clippy", {})

    if _lint_level(rust_lints.get("unsafe_code")) != "deny":
        errors.append("workspace lints must keep rust.unsafe_code = deny")
    if _lint_level(clippy_lints.get("all")) != "deny":
        errors.append("workspace lints must keep clippy::all = deny")

    member_manifests: set[Path] = set()
    for pattern in workspace.get("members", []):
        for candidate in root.glob(pattern):
            manifest = candidate / "Cargo.toml" if candidate.is_dir() else candidate
            if manifest.is_file() and manifest != root_manifest_path:
                member_manifests.add(manifest)
    if not member_manifests:
        errors.append("workspace member manifests were not discovered")

    for manifest_path in sorted(member_manifests):
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        if "package" not in manifest:
            continue
        lint_config = manifest.get("lints", {})
        relative = manifest_path.relative_to(root)
        if lint_config.get("workspace") is not True:
            errors.append(f"{relative}: package must inherit workspace lints")
        overrides = sorted(set(lint_config) - {"workspace"})
        if overrides:
            errors.append(f"{relative}: package-level lint overrides are forbidden: {', '.join(overrides)}")

    source_dirs = [root / "apps", root / "crates"]
    for source_dir in source_dirs:
        if not source_dir.exists():
            continue
        for source_path in sorted(source_dir.rglob("*.rs")):
            source = source_path.read_text(encoding="utf-8")
            relative = source_path.relative_to(root)
            if ALLOW_ATTRIBUTE.search(source):
                errors.append(f"{relative}: #[allow] attributes are forbidden; use a reviewed narrow #[expect] only when justified")
            if CFG_ATTR_ALLOW.search(source):
                errors.append(f"{relative}: cfg_attr-based lint downgrades are forbidden")
            for match in EXPECT_ATTRIBUTE.finditer(source):
                end = _attribute_end(source, match.start())
                attribute = source[match.start():end]
                observed_expectations.add(
                    (relative.as_posix(), _canonical_expect_attribute(attribute))
                )
                if not re.search(r"\breason\s*=\s*\"[^\"]+\"", attribute):
                    line = source.count("\n", 0, match.start()) + 1
                    errors.append(f"{relative}:{line}: every #[expect] must include a non-empty reason")
            if "tests" not in source_path.parts and "test" not in source_path.parts and UNSAFE_ITEM.search(source):
                errors.append(f"{relative}: unsafe Rust in production source requires explicit approved policy and review")

    exception_inventory_path = root / "scripts" / "ci" / "rust-lint-exceptions.json"
    try:
        inventory_data = json.loads(exception_inventory_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"{exception_inventory_path.relative_to(root)}: cannot read exception inventory: {error}")
        inventory_data = []

    errors.extend(
        _expect_inventory_errors(
            observed_expectations,
            inventory_data,
            exception_inventory_path.relative_to(root).as_posix(),
        )
    )

    cargo_config_paths = sorted((root / ".cargo").glob("**/*")) if (root / ".cargo").exists() else []
    policy_paths = [
        *sorted((root / ".github" / "workflows").glob("*.yml")),
        *sorted((root / ".github" / "workflows").glob("*.yaml")),
        root / "scripts" / "ci" / "verify",
        root / "scripts" / "ci" / "verify.ps1",
        *sorted((root / "scripts" / "ci").glob("*.sh")),
        *sorted((root / "scripts" / "ci").glob("check-*")),
        *cargo_config_paths,
    ]
    seen: set[Path] = set()
    for policy_path in policy_paths:
        if not policy_path.is_file() or policy_path in seen:
            continue
        seen.add(policy_path)
        source = policy_path.read_text(encoding="utf-8", errors="replace")
        if FORBIDDEN_FLAGS.search(source):
            errors.append(f"{policy_path.relative_to(root)}: forbidden compiler lint downgrade flag")

    verifier = (root / "scripts" / "ci" / "verify").read_text(encoding="utf-8")
    required_tests = (
        "cargo test --workspace --all-targets --all-features --locked",
        "cargo test -p sitolo-persistence --test part8_postgres_integration_tests --all-features --locked",
        "cargo test -p sitolo-persistence --test rls_security_tests --all-features --locked",
    )
    for command in required_tests:
        if command not in verifier:
            errors.append(f"scripts/ci/verify: required unfiltered database test is missing: {command}")
    for required in ("check-rust-policy.py", "install-policy-tools.py", "cargo deny check", "cargo audit"):
        if required not in verifier:
            errors.append(f"scripts/ci/verify: required policy gate is missing: {required}")

    powershell_verifier = (root / "scripts" / "ci" / "verify.ps1").read_text(encoding="utf-8")
    for required in (
        "check-rust-policy.py",
        "install-policy-tools.py",
        "part8_postgres_integration_tests",
        "rls_security_tests",
        "cargo deny check",
        "cargo audit",
    ):
        if required not in powershell_verifier:
            errors.append(f"scripts/ci/verify.ps1: required policy gate is missing: {required}")

    workflows = {
        "rust.yml": "./scripts/ci/verify",
        "artifact.yml": "./scripts/ci/verify",
        "security.yml": "install-policy-tools.py --tool cargo-audit",
        "policy.yml": "check-rust-policy.py",
    }
    for workflow_name, required in workflows.items():
        workflow_path = root / ".github" / "workflows" / workflow_name
        workflow_text = workflow_path.read_text(encoding="utf-8")
        if required not in workflow_text:
            errors.append(f".github/workflows/{workflow_name}: required gate invocation is missing: {required}")

    policy_workflow = (root / ".github" / "workflows" / "policy.yml").read_text(encoding="utf-8")
    if 'python3 -m unittest discover -s scripts/ci -p "test_*.py"' not in policy_workflow:
        errors.append(".github/workflows/policy.yml: Rust policy regression tests must run in CI")

    integration_workflow = (root / ".github" / "workflows" / "integration.yml").read_text(encoding="utf-8")
    if "--exclude sitolo-persistence" not in integration_workflow:
        errors.append("integration.yml: the bounded persistence package exclusion must remain explicit")

    versions = tomllib.loads(
        (root / "scripts" / "ci" / "policy-tool-versions.toml").read_text(encoding="utf-8")
    ).get("tools", {})
    for tool in POLICY_TOOLS:
        if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", str(versions.get(tool, ""))):
            errors.append(f"scripts/ci/policy-tool-versions.toml: {tool} must have an exact X.Y.Z pin")

    return errors


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    errors = check_repository(root)
    if errors:
        for error in errors:
            print(f"RUST POLICY VIOLATION: {error}", file=sys.stderr)
        return 1
    print("Rust lint, unsafe-code, and required database-test policy check PASSED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
