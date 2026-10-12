#!/bin/zsh
set -euo pipefail
cd "${0:A:h:h}"
[[ "$(uname -s)" == Darwin ]] || { print -u2 'This acceptance entrypoint requires native macOS.'; exit 1; }
cargo build --locked --workspace
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
python3 scripts/native_smoke.py --platform macos
