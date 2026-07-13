#!/usr/bin/env bash
set -euo pipefail

cargo component build --release --target wasm32-unknown-unknown
cargo fmt
cargo fmt --check
git diff --exit-code -- src/bindings.rs
