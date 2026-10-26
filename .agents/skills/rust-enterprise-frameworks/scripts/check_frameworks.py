#!/usr/bin/env python3
"""Framework artefact gate (FW-GATE rules).

Usage:
    check_frameworks.py --lock Cargo.lock --policy frameworks-policy.toml \
                        --manifest Cargo.toml [--src-root .] [--sqlx-dir .sqlx]

Checks, all fail-closed:
  1. Every banned crate is absent from Cargo.lock (any version, transitive included).
  2. Every approved crate present in Cargo.lock: every version satisfies the policy
     (major for >=1.0, minor for 0.x, and floor version).
  3. Each approved crate appears at most once in Cargo.lock.
  4. The repository rust-version (workspace.package or package) is >= repo.min_rust_version.
  5. If sqlx is present and its compile-time query macros are used under --src-root,
     an offline query cache directory with at least one JSON file must exist.

Exit codes: 0 pass, 1 policy violation(s), 2 usage or parse error.
Requires Python 3.11+ (tomllib). Missing inputs are errors, never skips.
"""
import argparse
import pathlib
import re
import sys

try:
    import tomllib
except ImportError:  # pragma: no cover
    print("check_frameworks: Python 3.11+ is required (tomllib)", file=sys.stderr)
    sys.exit(2)

SQLX_MACRO = re.compile(
    r"\b(?:sqlx::)?(query|query_as|query_scalar|query_file|query_file_as|query_file_scalar)!\s*\("
)


def parse_version(text):
    """Strict x.y.z parse. Pre-release or build metadata is rejected for policy purposes."""
    m = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", text.strip())
    if not m:
        raise ValueError(f"unparseable or pre-release version: {text!r}")
    return tuple(int(x) for x in m.groups())


def parse_floor(text):
    parts = [int(x) for x in re.findall(r"\d+", text)]
    while len(parts) < 3:
        parts.append(0)
    return tuple(parts[:3])


def load_toml(path):
    p = pathlib.Path(path)
    if not p.is_file():
        print(f"check_frameworks: missing required file: {path}", file=sys.stderr)
        sys.exit(2)
    with p.open("rb") as fh:
        return tomllib.load(fh)


def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument("--lock", required=True)
    ap.add_argument("--policy", required=True)
    ap.add_argument("--manifest", required=True)
    ap.add_argument("--src-root", default=None)
    ap.add_argument("--sqlx-dir", default=".sqlx")
    args = ap.parse_args(argv)

    lock = load_toml(args.lock)
    policy = load_toml(args.policy)
    manifest = load_toml(args.manifest)

    packages = lock.get("package", [])
    if not packages:
        print("check_frameworks: Cargo.lock has no packages; refusing to pass", file=sys.stderr)
        return 2
    by_name = {}
    for pkg in packages:
        by_name.setdefault(pkg["name"], []).append(pkg["version"])

    violations = []

    # 1. banned crates
    for ban in policy.get("banned", {}).get("crates", []):
        if ban["name"] in by_name:
            violations.append(
                f"BANNED crate '{ban['name']}' present ({', '.join(by_name[ban['name']])}): {ban['reason']}"
            )

    # 2 and 3. approved crates
    checked = []
    for entry in policy.get("approved", []):
        name = entry["crate"]
        versions = by_name.get(name, [])
        if not versions:
            continue
        checked.append(f"{name} {', '.join(sorted(set(versions)))}")
        if len(set(versions)) > 1:
            violations.append(
                f"DUPLICATE '{name}': {sorted(set(versions))}. Unify versions (FW-GATE-03)."
            )
        floor = parse_floor(entry["floor"])
        for v in set(versions):
            try:
                vt = parse_version(v)
            except ValueError as exc:
                violations.append(f"'{name}' {exc}")
                continue
            if "minor" in entry:
                if vt[0] != 0 or vt[1] != entry["minor"]:
                    violations.append(
                        f"'{name}' {v} is outside approved series 0.{entry['minor']}.x "
                        f"(area: {entry['area']})"
                    )
            elif vt[0] != entry["major"]:
                violations.append(
                    f"'{name}' {v} is outside approved major {entry['major']} (area: {entry['area']})"
                )
            if vt < floor:
                violations.append(
                    f"'{name}' {v} is below the policy floor {entry['floor']} (FW-GATE-02)"
                )

    # 4. repository MSRV
    pkg_section = manifest.get("workspace", {}).get("package", {}) or manifest.get("package", {})
    repo_msrv = pkg_section.get("rust-version")
    min_msrv = policy["repo"]["min_rust_version"]
    if not repo_msrv:
        violations.append("rust-version is not declared in the root manifest (BP-TC-03)")
    else:
        try:
            if parse_floor(repo_msrv) < parse_floor(min_msrv):
                violations.append(
                    f"rust-version {repo_msrv} is below the framework floor {min_msrv} "
                    f"required by the approved stack (FW-GATE-04)"
                )
        except ValueError as exc:
            violations.append(f"rust-version unparseable: {exc}")

    # 5. sqlx offline cache
    if "sqlx" in by_name and args.src_root:
        root = pathlib.Path(args.src_root)
        macro_hits = []
        for path in root.rglob("*.rs"):
            if any(part in {"target", "fuzz"} for part in path.parts):
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            if SQLX_MACRO.search(text):
                macro_hits.append(str(path))
        if macro_hits:
            cache = root / args.sqlx_dir
            json_files = list(cache.glob("*.json")) if cache.is_dir() else []
            if not json_files:
                violations.append(
                    f"sqlx compile-time macros used in {len(macro_hits)} file(s) but no offline "
                    f"cache at '{args.sqlx_dir}/' (FW-SQX-02). Run `cargo sqlx prepare` and commit it."
                )
            print(f"sqlx macros found in {len(macro_hits)} file(s); offline cache files: {len(json_files)}")

    print("approved frameworks in lock:")
    for line in checked:
        print(f"  {line}")
    if violations:
        print(f"\nFRAMEWORK GATE FAIL: {len(violations)} violation(s)")
        for v in violations:
            print(f"  - {v}")
        return 1
    print("FRAMEWORK GATE PASSED")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
