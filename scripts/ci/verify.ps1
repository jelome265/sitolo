$ErrorActionPreference = "Continue"

$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

Write-Output "==> Verify toolchain"
rustc --version
cargo --version

Write-Output "==> Verify lockfile (fails if stale)"
& cargo metadata --format-version 1 --locked 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "lockfile check failed"; exit $LASTEXITCODE }

Write-Output "==> Format check"
& cargo fmt --all -- --check 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "format check failed"; exit $LASTEXITCODE }

Write-Output "==> Lint"
& cargo clippy --workspace --all-targets --all-features --locked -- -D warnings 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "clippy failed"; exit $LASTEXITCODE }

Write-Output "==> Compile"
& cargo check --workspace --all-targets --all-features --locked 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "compile check failed"; exit $LASTEXITCODE }

Write-Output "==> Unit tests"
& cargo test --workspace --all-targets --all-features --locked 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "unit tests failed"; exit $LASTEXITCODE }

Write-Output "==> Release build"
& cargo build --workspace --release --locked 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "release build failed"; exit $LASTEXITCODE }

Write-Output "==> Production panic-path policy"
& "$ROOT\scripts\ci\check-production-panic-paths.ps1"
if ($LASTEXITCODE -ne 0) { Write-Output "production panic-path policy failed"; exit $LASTEXITCODE }

Write-Output "==> Architecture check"
& "$ROOT\scripts\ci\check-architecture.ps1"
if ($LASTEXITCODE -ne 0) { Write-Output "architecture check failed"; exit $LASTEXITCODE }

Write-Output "==> Phase 2 policy check"
& "$ROOT\scripts\ci\check-phase2-policy.ps1"
if ($LASTEXITCODE -ne 0) { Write-Output "phase 2 policy check failed"; exit $LASTEXITCODE }

Write-Output "==> Dependency and license policy"
$deny = Get-Command cargo-deny -ErrorAction SilentlyContinue
if ($deny) {
    & cargo deny check 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Output "cargo deny failed"; exit $LASTEXITCODE }
} else {
    Write-Output "    cargo-deny not installed; skipping (the CI security workflow installs it unconditionally)"
}
$audit = Get-Command cargo-audit -ErrorAction SilentlyContinue
if ($audit) {
    & cargo audit 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Output "cargo audit failed"; exit $LASTEXITCODE }
} else {
    Write-Output "    cargo-audit not installed; skipping (the CI security workflow installs it unconditionally)"
}

Write-Output "==> All verification passed"
