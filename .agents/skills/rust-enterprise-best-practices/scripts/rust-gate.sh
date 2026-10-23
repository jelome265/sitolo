#!/usr/bin/env bash
# rust-gate.sh — enterprise Rust gate (BP rulebook, section 25).
#
# Usage:  scripts/rust-gate.sh [workspace-root]
#
# Policy:
#   * Fails on the first failing step.
#   * A MISSING TOOL IS A FAILURE, never a skip. A skipped check is a false green.
#   * The pattern scan is a heuristic backstop (BP-* rules marked E5). It fails on any hit.
#     A hit may be acknowledged only on the same line with an inline marker that a
#     reviewer can read:   // gate-allow: <reason>
#     Markers are listed in the output so reviewers see every acknowledgement.
#   * Conditional steps (Miri, fuzz smoke, MSRV) become mandatory when their trigger exists.
#
# Environment:
#   RUST_GATE_MIRI_TOOLCHAIN  pinned nightly, e.g. nightly-2026-07-01 (mandatory when unsafe is present)
#   RUST_GATE_FUZZ_SMOKE=1    run a 60-second smoke fuzz per target when fuzz/ exists
#   RUST_GATE_FUZZ_TOOLCHAIN  pinned nightly for fuzzing (default: RUST_GATE_MIRI_TOOLCHAIN)
#
# Exit codes: 0 pass, 1 gate failure, 2 usage/environment error.

set -euo pipefail

ROOT="${1:-.}"
cd "$ROOT" || { echo "rust-gate: cannot enter $ROOT" >&2; exit 2; }

SCAN_FAILS=0
ALLOW_COUNT=0

fail() { echo "GATE FAIL: $*" >&2; exit 1; }
step() { echo; echo "== [$1] $2"; }
need() {
  command -v "$1" >/dev/null 2>&1 \
    || fail "required tool '$1' is not installed. A missing tool is a failure, not a skip."
}
run() { echo "+ $*"; "$@"; }

# ---------------------------------------------------------------------------
step 1 "toolchain pin (BP-TC-01)"
need cargo
need rustc
[ -f rust-toolchain.toml ] || fail "rust-toolchain.toml is missing (BP-TC-01)"
CHANNEL="$(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\(.*\)".*/\1/p' rust-toolchain.toml | head -n1)"
case "$CHANNEL" in
  stable|nightly|beta|"") fail "channel '$CHANNEL' is floating. Pin an exact version (BP-TC-01)." ;;
esac
ACTUAL="$(rustc --version | awk '{print $2}')"
[ "$ACTUAL" = "$CHANNEL" ] || fail "rustc $ACTUAL does not match pinned channel $CHANNEL. Install the pinned toolchain."
echo "pinned toolchain $CHANNEL OK"

# ---------------------------------------------------------------------------
step 2 "lockfile integrity (BP-CG-01)"
[ -f Cargo.lock ] || fail "Cargo.lock is missing (BP-CG-01)"
run cargo metadata --locked --format-version 1 >/dev/null

# ---------------------------------------------------------------------------
step 3 "format (BP-FM-01)"
need rustfmt
run cargo fmt --all -- --check

# ---------------------------------------------------------------------------
step 4 "clippy, warnings are errors (BP-LN-03)"
need cargo-clippy
# DATABASE_URL is unset so sqlx compile-time checks must succeed from the committed offline cache (FW-SQX-02).
run env -u DATABASE_URL cargo clippy --workspace --all-targets --locked -- -D warnings

# ---------------------------------------------------------------------------
step 5 "tests and doctests (BP-TS-01)"
if command -v cargo-nextest >/dev/null 2>&1; then
  run cargo nextest run --workspace --all-features --locked
else
  fail "cargo-nextest is required (BP-TS-01). Install it in the CI image."
fi
run cargo test --doc --workspace --locked

# ---------------------------------------------------------------------------
step 6 "docs, warnings are errors (BP-LN-07, BP-DC-03)"
RUSTDOCFLAGS="-D warnings" run cargo doc --workspace --no-deps --locked

# ---------------------------------------------------------------------------
step 7 "supply chain (BP-DP-02, BP-DP-03)"
need cargo-deny
need cargo-audit
[ -f deny.toml ] || fail "deny.toml is missing (BP-DP-02)"
run cargo deny check
run cargo audit --deny warnings

if [ -d supply-chain ]; then
  need cargo-vet
  run cargo vet --locked
else
  echo "WARNING: supply-chain/ not present; cargo-vet attestation not configured (BP-DP-07 requires it for production)." >&2
  fail "cargo-vet is not configured (BP-DP-07)."
fi

# ---------------------------------------------------------------------------
step "7b" "framework hard bans and artefact policy (FW-LN-01, FW-GATE)"
[ -f clippy.toml ] || fail "root clippy.toml with framework hard bans is missing (FW-LN-01). Copy rust-enterprise-frameworks/references/clippy.service.toml."
grep -q 'disallowed-methods' clippy.toml || fail "clippy.toml defines no disallowed-methods (FW-LN-01)."
need python3
FW_DIR="$(cd "$(dirname "$0")/../.." && pwd)/rust-enterprise-frameworks"
[ -f "$FW_DIR/scripts/check_frameworks.py" ] || fail "framework gate script not found at $FW_DIR (install the rust-enterprise-frameworks skill beside this one)."
run python3 "$FW_DIR/scripts/check_frameworks.py" \
  --lock Cargo.lock --policy "$FW_DIR/references/frameworks-policy.toml" \
  --manifest Cargo.toml --src-root .

# ---------------------------------------------------------------------------
step 8 "MSRV build (BP-TC-03)"
MSRV="$(find . -name Cargo.toml -not -path './target/*' -exec grep -hE '^[[:space:]]*rust-version[[:space:]]*=' {} + 2>/dev/null | head -n1 | sed 's/.*"\(.*\)".*/\1/' || true)"
if [ -n "${MSRV:-}" ]; then
  need rustup
  run cargo "+$MSRV" check --workspace --locked
else
  fail "no rust-version declared in any Cargo.toml (BP-TC-03)."
fi

# ---------------------------------------------------------------------------
step 9 "production source discovery"
FILES="$(find . -path ./target -prune -o -path ./fuzz -prune -o -type f -name '*.rs' -path '*/src/*' -print | sort)"
[ -n "$FILES" ] || fail "no production sources found under */src/"
NFILES="$(printf '%s\n' "$FILES" | wc -l | tr -d ' ')"
echo "scanning $NFILES production source files"

# ---------------------------------------------------------------------------
step 10 "pattern scan (E5 backstop)"

scan() {
  local desc="$1" pat="$2" hits
  hits="$(printf '%s\n' "$FILES" | xargs -r -d '\n' grep -nE "$pat" 2>/dev/null \
          | grep -v 'gate-allow:' || true)"
  if [ -n "$hits" ]; then
    echo "SCAN FAIL [$desc]"
    printf '%s\n' "$hits" | sed 's/^/  /'
    SCAN_FAILS=$((SCAN_FAILS + 1))
  fi
}

scan_allowed() {
  # Count acknowledged hits so reviewers can see every suppression.
  printf '%s\n' "$FILES" | xargs -r -d '\n' grep -nE 'gate-allow:' 2>/dev/null || true
}

scan "BP-ER-02 unwrap()"                 '\.unwrap\(\)'
scan "BP-ER-02 expect()"                 '\.expect\('
scan "BP-ER-02 panic-class macros"       '(^|[^[:alnum:]_])(panic|unreachable|todo|unimplemented)!\('
scan "BP-OB-01 print/dbg macros"         '(^|[^[:alnum:]_])(dbg|println|eprintln|print|eprint)!\('
scan "BP-PN-01 process::exit"            'process::exit\('
scan "BP-US-09 static mut"               '(^|[[:space:]])static[[:space:]]+mut[[:space:]]'
scan "BP-US-08 mem::forget"              'mem::forget\('
scan "BP-US-07 transmute"                '(^|[^[:alnum:]_])transmute'
scan "BP-TC-02 env mutation"             '(env::|[^[:alnum:]_])(set_var|remove_var)\('
scan "BP-NU-01 numeric as-cast"          '[[:space:]]as[[:space:]]+(u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)([^[:alnum:]_]|$)'
scan "SEC-INJ shell invocation"          'Command::new\("(sh|bash|zsh|cmd|powershell)"\)'
scan "BP-OW-07 Rc in source"             '(^|[^[:alnum:]_])Rc<'
scan "BP-CC-05 unbounded channel"        'unbounded_channel\('
scan "BP-CC-09 std::thread::sleep"       'std::thread::sleep\('
scan "BP-CO-04 non-DoS-resistant hasher" '(FxHashMap|FxHashSet|IdentityHasher|BuildHasherDefault)'
scan "BP-NU-03 floating money"           '(amount|price|balance|fee|total|cost|money|amt)[A-Za-z_]*[[:space:]]*:[[:space:]]*f(32|64)'
scan "SEC-CRYPTO secret equality"        '(token|secret|password|digest|hmac|mac|signature|api_key)[A-Za-z_.()]*[[:space:]]*(==|!=)'
scan "BP-ER-01 discarded result"         '(^|[[:space:]])let[[:space:]]+_[[:space:]]*='

# allow() without reason: must use #[expect(..., reason = "...")] or carry a reason.
ALLOW_HITS="$(printf '%s\n' "$FILES" | xargs -r -d '\n' grep -nE '#!?\[allow\(' 2>/dev/null \
              | grep -v 'reason' || true)"
if [ -n "$ALLOW_HITS" ]; then
  echo "SCAN FAIL [BP-LN-04 allow without reason]"
  printf '%s\n' "$ALLOW_HITS" | sed 's/^/  /'
  SCAN_FAILS=$((SCAN_FAILS + 1))
fi

# Unsafe blocks, impls and extern blocks need a SAFETY comment within the two preceding lines.
# Unsafe fns need a "# Safety" rustdoc section within the 40 preceding lines.
UNSAFE_HITS="$(for f in $FILES; do
  awk -v F="$f" '
    { L[NR] = $0 }
    /^[[:space:]]*\/\// { next }
    /unsafe[[:space:]]+(impl|\{|extern)/ {
      ok = 0
      for (i = NR - 2; i < NR; i++) if (i >= 1 && L[i] ~ /SAFETY:/) ok = 1
      if (!ok) print F ":" NR ": unsafe block/impl without // SAFETY: directly above: " $0
    }
    /unsafe[[:space:]]+fn/ {
      ok = 0
      for (i = NR - 40; i < NR; i++) if (i >= 1 && L[i] ~ /# Safety/) ok = 1
      if (!ok) print F ":" NR ": unsafe fn without # Safety rustdoc: " $0
    }
  ' "$f"
done)"
if [ -n "$UNSAFE_HITS" ]; then
  echo "SCAN FAIL [BP-US-02 unsafe documentation]"
  printf '%s\n' "$UNSAFE_HITS" | sed 's/^/  /'
  SCAN_FAILS=$((SCAN_FAILS + 1))
fi

# Debug derived on secret-named types (heuristic: derive(Debug) within 3 lines of a type whose name CONTAINS a secret word).
SECRET_DEBUG="$(for f in $FILES; do
  awk -v F="$f" '
    /#\[derive\(.*Debug/ { d = NR }
    /(struct|enum)[[:space:]]+[A-Za-z0-9_]*(Secret|Token|Password|Passwd|Credential|ApiKey|PrivateKey|SigningKey|SecretKey|Session)[A-Za-z0-9_]*/ {
      if (d > 0 && NR - d <= 3) print F ":" NR ": Debug derived on secret-named type: " $0
    }
  ' "$f"
done)"
if [ -n "$SECRET_DEBUG" ]; then
  echo "SCAN FAIL [BP-TY-09 derived Debug on secret-named type]"
  printf '%s\n' "$SECRET_DEBUG" | sed 's/^/  /'
  SCAN_FAILS=$((SCAN_FAILS + 1))
fi

ALLOW_COUNT="$(scan_allowed | wc -l | tr -d ' ')"
if [ "$ALLOW_COUNT" -gt 0 ]; then
  echo "ACKNOWLEDGED gate-allow markers ($ALLOW_COUNT) — reviewer must verify each:"
  scan_allowed | sed 's/^/  /'
fi

if [ "$SCAN_FAILS" -gt 0 ]; then
  fail "pattern scan found $SCAN_FAILS rule group(s) with violations. Fix, or add // gate-allow: <reason> on the line only when a reviewer can verify it. Hits inside #[cfg(test)] modules must be moved to tests/ or justified."
fi
echo "pattern scan clean"

# ---------------------------------------------------------------------------
step 11 "unsafe: mandatory Miri (BP-US-04)"
if grep -rqE 'unsafe[[:space:]]+(impl|fn|\{|extern)' $FILES 2>/dev/null; then
  [ -n "${RUST_GATE_MIRI_TOOLCHAIN:-}" ] \
    || fail "unsafe code present but RUST_GATE_MIRI_TOOLCHAIN is unset. Miri is mandatory for unsafe modules (BP-US-04)."
  need rustup
  run cargo "+$RUST_GATE_MIRI_TOOLCHAIN" miri setup
  MIRIFLAGS="${MIRIFLAGS:--Zmiri-strict-provenance}" run cargo "+$RUST_GATE_MIRI_TOOLCHAIN" miri test --workspace --locked
else
  echo "no unsafe code detected; Miri not required"
fi

# ---------------------------------------------------------------------------
step 12 "fuzz smoke (BP-TS-03)"
if [ -d fuzz ]; then
  if [ "${RUST_GATE_FUZZ_SMOKE:-0}" = "1" ]; then
    FT="${RUST_GATE_FUZZ_TOOLCHAIN:-${RUST_GATE_MIRI_TOOLCHAIN:-}}"
    [ -n "$FT" ] || fail "fuzz/ exists but no RUST_GATE_FUZZ_TOOLCHAIN or RUST_GATE_MIRI_TOOLCHAIN is set."
    need cargo-fuzz
    for t in $(cargo "+$FT" fuzz list); do
      run cargo "+$FT" fuzz run "$t" -- -max_total_time=60
    done
  else
    fail "fuzz/ exists but RUST_GATE_FUZZ_SMOKE is not 1. Fuzz smoke is mandatory for repositories with fuzz targets."
  fi
else
  echo "no fuzz/ directory; fuzz smoke not configured (BP-TS-03 requires fuzz targets for untrusted-input parsers: review manually)"
fi

echo
echo "RUST GATE PASSED"
