#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

GOROOT="$(go env GOROOT)"
mkdir -p web

# Locate wasm_exec.js in Go toolchain
WASM_EXEC=""
if [ -f "$GOROOT/misc/wasm/wasm_exec.js" ]; then
    WASM_EXEC="$GOROOT/misc/wasm/wasm_exec.js"
elif [ -f "$GOROOT/lib/wasm/wasm_exec.js" ]; then
    WASM_EXEC="$GOROOT/lib/wasm/wasm_exec.js"
fi

if [ -n "$WASM_EXEC" ]; then
    cp "$WASM_EXEC" web/wasm_exec.js
    echo "Copied wasm_exec.js from $WASM_EXEC"
else
    echo "Warning: wasm_exec.js not found in GOROOT ($GOROOT)"
fi

echo "Compiling WebAssembly binary: web/sst.wasm..."
GOOS=js GOARCH=wasm go build -trimpath -ldflags="-s -w" -o web/sst.wasm ./cmd/wasm
echo "Build complete: $(ls -lh web/sst.wasm | awk '{print $5}')"
