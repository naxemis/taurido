#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo fmt --all

npm --prefix gui install
npm --prefix gui run build

cargo clippy --fix --workspace --all-targets --all-features --allow-dirty --allow-staged

cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings