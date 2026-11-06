#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo fmt --all -- --check

npm --prefix gui install
npm --prefix gui run build

cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings

RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features