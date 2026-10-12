$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'This acceptance entrypoint requires native Windows.' }
Set-Location (Split-Path $PSScriptRoot -Parent)
cargo build --locked --workspace
if ($LASTEXITCODE -ne 0) { throw 'Native build failed' }
cargo fmt --all --check
if ($LASTEXITCODE -ne 0) { throw 'Formatting failed' }
cargo clippy --locked --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'Clippy failed' }
cargo test --locked --workspace
if ($LASTEXITCODE -ne 0) { throw 'Native tests failed' }
python scripts/native_smoke.py --platform windows
if ($LASTEXITCODE -ne 0) { throw 'Native CLI smoke failed' }
