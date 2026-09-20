#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== 1. Building WebAssembly binary ==="
./scripts/build-wasm.sh

echo ""
echo "=== 2. Running Automated WebAssembly User Journeys ==="
node tests/wasm_journey_test.js
