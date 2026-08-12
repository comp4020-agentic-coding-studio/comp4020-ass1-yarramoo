#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build -p raytracer-wasm --release --target wasm32-unknown-unknown
wasm-bindgen \
  --target web \
  --out-dir raytracer-wasm/pkg \
  --out-name raytracer_wasm \
  target/wasm32-unknown-unknown/release/raytracer_wasm.wasm
