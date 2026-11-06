#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check

npm --prefix gui install
npm --prefix gui run build

cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings