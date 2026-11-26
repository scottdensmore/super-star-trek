#!/usr/bin/env bash
# Builds the Rust WebAssembly edition (crates/sst-wasm) and writes the
# wasm-bindgen "no-modules" output that web/index.html loads:
#   <out-dir>/sst_wasm.js       glue script, defines the global wasm_bindgen()
#   <out-dir>/sst_wasm_bg.wasm  the module
# Usage: scripts/build-wasm.sh [out-dir]   (default: web/pkg, gitignored)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${1:-${OUT_DIR:-web/pkg}}"
TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"
WASM_IN="${TARGET_DIR}/wasm32-unknown-unknown/release/sst_wasm.wasm"

# wasm-bindgen-cli must match the wasm-bindgen crate version exactly.
WB_VERSION="$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/"/, "", $3); print $3; exit }' Cargo.lock)"
if [ -z "${WB_VERSION}" ]; then
    echo "error: wasm-bindgen not found in Cargo.lock" >&2
    exit 1
fi
if ! command -v wasm-bindgen >/dev/null 2>&1; then
    echo "error: wasm-bindgen CLI not found. Install it with:" >&2
    echo "  cargo install wasm-bindgen-cli --version ${WB_VERSION} --locked" >&2
    exit 1
fi
CLI_VERSION="$(wasm-bindgen --version | awk '{ print $2 }')"
if [ "${CLI_VERSION}" != "${WB_VERSION}" ]; then
    echo "error: wasm-bindgen CLI ${CLI_VERSION} does not match the crate version ${WB_VERSION}. Run:" >&2
    echo "  cargo install wasm-bindgen-cli --version ${WB_VERSION} --locked --force" >&2
    exit 1
fi
if command -v rustup >/dev/null 2>&1 && ! rustup target list --installed | grep -qx wasm32-unknown-unknown; then
    echo "error: the wasm32-unknown-unknown target is missing. Run: rustup target add wasm32-unknown-unknown" >&2
    exit 1
fi

echo "Compiling crates/sst-wasm for wasm32-unknown-unknown (release)..."
cargo build --target wasm32-unknown-unknown --release -p sst-wasm

echo "Generating wasm-bindgen ${WB_VERSION} glue into ${OUT_DIR}..."
mkdir -p "${OUT_DIR}"
rm -f "${OUT_DIR}/sst_wasm.js" "${OUT_DIR}/sst_wasm_bg.wasm"
wasm-bindgen --target no-modules --no-typescript --out-dir "${OUT_DIR}" "${WASM_IN}"

# Optional size optimisation when binaryen is installed.
if command -v wasm-opt >/dev/null 2>&1; then
    echo "Optimizing with wasm-opt -Oz..."
    wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
        "${OUT_DIR}/sst_wasm_bg.wasm" -o "${OUT_DIR}/sst_wasm_bg.wasm"
fi

echo "Build complete:"
ls -lh "${OUT_DIR}/sst_wasm.js" "${OUT_DIR}/sst_wasm_bg.wasm"
