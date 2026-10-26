$ErrorActionPreference = "Continue"

$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

Write-Output "==> ICM workspace"
& "$ROOT\scripts\ci\check-icm-workspace.ps1"
if ($LASTEXITCODE -ne 0) { Write-Output "ICM workspace check failed"; exit $LASTEXITCODE }

Write-Output "==> Verify toolchain"
rustc --version
cargo --version

Write-Output "==> Architecture check (early, fail-closed)"
& "$ROOT\scripts\ci\check-architecture.ps1"
if ($LASTEXITCODE -ne 0) { Write-Output "architecture check failed"; exit $LASTEXITCODE }

Write-Output "==> Rust lint and unsafe-code policy"
& python "$ROOT\scripts\ci\check-rust-policy.py"
if ($LASTEXITCODE -ne 0) { Write-Output "Rust policy check failed"; exit $LASTEXITCODE }

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

Write-Output "==> Phase 4 Part 8 PostgreSQL integration suite"
& cargo test -p sitolo-persistence --test part8_postgres_integration_tests --all-features --locked 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "Part 8 PostgreSQL tests failed"; exit $LASTEXITCODE }

Write-Output "==> PostgreSQL RLS security suite"
& cargo test -p sitolo-persistence --test rls_security_tests --all-features --locked 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "RLS security tests failed"; exit $LASTEXITCODE }

Write-Output "==> Release build"
& cargo build --workspace --release --locked 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "release build failed"; exit $LASTEXITCODE }

Write-Output "==> Phase 2 policy check"
& "$ROOT\scripts\ci\check-phase2-policy.ps1"
if ($LASTEXITCODE -ne 0) { Write-Output "phase 2 policy check failed"; exit $LASTEXITCODE }

Write-Output "==> Dependency and license policy"
& python "$ROOT\scripts\ci\install-policy-tools.py"
if ($LASTEXITCODE -ne 0) { Write-Output "policy tools unavailable or failed version verification"; exit $LASTEXITCODE }
& cargo deny check 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "cargo deny failed"; exit $LASTEXITCODE }
& cargo audit 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Output "cargo audit failed"; exit $LASTEXITCODE }

Write-Output "==> All verification passed"
