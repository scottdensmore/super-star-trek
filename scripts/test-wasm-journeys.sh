#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Optional: build and test a different output directory (default web/pkg).
PKG_DIR="${1:-${SST_WASM_PKG_DIR:-web/pkg}}"

echo "=== 1. Building WebAssembly module ==="
./scripts/build-wasm.sh "${PKG_DIR}"

echo ""
echo "=== 2. Running Automated WebAssembly User Journeys ==="
SST_WASM_PKG_DIR="${PKG_DIR}" node tests/wasm_journey_test.js

echo ""
echo "=== 3. Replaying the parity sessions through the WebAssembly build ==="
SST_WASM_PKG_DIR="${PKG_DIR}" node tests/wasm_parity_replay.js
